use super::*;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct MutationBatchStats {
    pub attempts: usize,
    pub accepted: usize,
    pub rejected: usize,
    pub duplicates: usize,
    pub geometry_rejections: usize,
    /// Candidates that passed physical geometry validation and were analyzed for cavity qualification.
    pub cavity_checks_performed: usize,
    pub cavity_rejections: usize,
    pub viability_rejections: usize,
    pub rejection_reasons: BTreeMap<String, usize>,
    pub stopped_at_candidate_limit: bool,
    pub stopped_at_storage_limit: bool,
}

/// Generate swap-first mutation trials. Each resource substitution owns its
/// fit search: try the existing pose, then alternate rotations and nearby
/// translations of that same replacement before moving to the next swap.
/// Bond validity and cavity qualification remain authoritative downstream.
pub(super) fn one_step_mutations(
    parent: &KnownQualifyingCavity,
    catalog: &[BaseResource],
) -> Vec<(String, OrganismStructure)> {
    let angle_step = std::f64::consts::PI / 12.0;
    let mut out = Vec::new();
    let original_count = parent.structure.units.len();

    // Resource substitutions are the primary mutation. Search the changed
    // constituent's fit before trying another resource or constituent.
    for unit_index in 0..original_count {
        let current_name = parent.structure.units[unit_index]
            .material
            .parts
            .first()
            .map(|part| part.0.as_str())
            .unwrap_or("");
        for resource in catalog
            .iter()
            .filter(|resource| resource.name != current_name)
        {
            let poses = [
                ("existing pose", 0.0, 0.0, 0.0),
                ("rotate +15deg", 0.0, 0.0, angle_step),
                ("rotate -15deg", 0.0, 0.0, -angle_step),
                ("rotate +30deg", 0.0, 0.0, angle_step * 2.0),
                ("rotate -30deg", 0.0, 0.0, -angle_step * 2.0),
                ("move +x 0.25", 0.25, 0.0, 0.0),
                ("move -x 0.25", -0.25, 0.0, 0.0),
                ("move +y 0.25", 0.0, 0.25, 0.0),
                ("move -y 0.25", 0.0, -0.25, 0.0),
                ("move +x 0.5", 0.5, 0.0, 0.0),
                ("move -x 0.5", -0.5, 0.0, 0.0),
                ("move +y 0.5", 0.0, 0.5, 0.0),
                ("move -y 0.5", 0.0, -0.5, 0.0),
                ("move +xy 0.25", 0.25, 0.25, 0.0),
                ("move +x-y 0.25", 0.25, -0.25, 0.0),
                ("move -x+y 0.25", -0.25, 0.25, 0.0),
                ("move -xy 0.25", -0.25, -0.25, 0.0),
            ];
            for (pose_label, dx, dy, rotation_delta) in poses {
                let mut trial = parent.structure.clone();
                trial.units[unit_index].material =
                    crate::resources::Material::free_base(resource.name.clone(), 1.0);
                trial.units[unit_index].geometry = None;
                trial.units[unit_index].placement.x += dx;
                trial.units[unit_index].placement.y += dy;
                trial.units[unit_index].placement.rotation_radians += rotation_delta;
                out.push((
                    format!(
                        "replace constituent at index {unit_index} with {} ({pose_label})",
                        resource.name
                    ),
                    trial,
                ));
            }
        }
    }

    // Translation is itself a cavity mutation. Move each unchanged piece in
    // cardinal and diagonal directions at 0.25-unit increments, out to 1.5
    // units. Generate only these eight rays, not the full 13x13 square grid.
    // This keeps the mutation set broad without exploding CI work.
    for unit_index in 0..original_count {
        for step in 1..=6 {
            let distance = step as f64 * 0.25;
            for (direction, dx, dy) in [
                ("+x", distance, 0.0),
                ("-x", -distance, 0.0),
                ("+y", 0.0, distance),
                ("-y", 0.0, -distance),
                ("+xy", distance, distance),
                ("+x-y", distance, -distance),
                ("-x+y", -distance, distance),
                ("-xy", -distance, -distance),
            ] {
                let mut trial = parent.structure.clone();
                trial.units[unit_index].placement.x += dx;
                trial.units[unit_index].placement.y += dy;
                out.push((
                    format!(
                        "translate constituent at index {unit_index} (move {direction} {distance:.2})"
                    ),
                    trial,
                ));
            }
        }

        // Rotation is independent of translation and must also trigger live
        // contact/bond reconstruction before geometry and cavity validation.
        for step in 1..=24 {
            let angle_degrees = step * 15;
            let mut trial = parent.structure.clone();
            trial.units[unit_index].placement.rotation_radians += angle_step * step as f64;
            out.push((
                format!("rotate constituent at index {unit_index} by {angle_degrees}deg"),
                trial,
            ));
        }
    }

    // Destructive removal is deliberately last. Bond-only bookkeeping edits
    // are not mutations: formation identity is determined by material and
    // realized geometry, and cavity analysis decides whether it qualifies.
    for unit_index in 0..original_count {
        let unit_id = parent.structure.units[unit_index].physical_id;
        let mut removed = parent.structure.clone();
        removed.remove_units_by_physical_ids(&[unit_id]);
        out.push((format!("remove constituent at index {unit_index}"), removed));
    }

    out
}

/// Rebuild connections for a swapped constituent from current physical
/// contacts. No bond count or old endpoint layout is treated as formation
/// identity; the ordinary constructor bond transaction remains authoritative.
pub(super) fn rebuild_replacement_connections(
    structure: &mut OrganismStructure,
    changed_index: usize,
    catalog: &[BaseResource],
) -> bool {
    let Some(changed_id) = structure
        .units
        .get(changed_index)
        .map(|unit| unit.physical_id)
    else {
        return false;
    };
    structure.bonds.retain(|bond| {
        bond.endpoint_a.constituent_id != changed_id && bond.endpoint_b.constituent_id != changed_id
    });
    let remaining_bond_count = structure.bonds.len();

    let mut ledger = crate::state::EnergyLedger::default();
    let mut energy = f64::MAX / 4.0;
    let neighbor_indices = (0..structure.units.len())
        .filter(|&index| index != changed_index)
        .collect::<Vec<_>>();
    for neighbor_index in neighbor_indices {
        // A non-contact is an expected failed fit, not a mutation-wide error.
        // Each trial is isolated, so a partial repair cannot leak to another.
        let _ = crate::initial_organism_constructor::bond_units(
            structure,
            changed_index,
            neighbor_index,
            catalog,
            &mut ledger,
            &mut energy,
        );
    }
    structure.bonds.len() > remaining_bond_count
}

/// Explore one bounded batch from the persistent cavity frontier. Validation
/// and duplicate signatures are checked before a candidate is appended. A small
/// cursor rotates the starting parent between runs so early records do not starve
/// later candidates once the library grows.
pub(crate) fn mutate_once(
    path: impl AsRef<Path>,
    catalog: &[BaseResource],
) -> Result<MutationBatchStats, String> {
    let path = path.as_ref();
    if !path.exists()
        || fs::metadata(path)
            .map_err(|e| format!("inspect cavity store: {e}"))?
            .len()
            == 0
    {
        seed_nitrogen_ring(path, catalog)?;
    }
    let _store_lock = StoreLock::acquire(path)?;
    let records = load_records(path)?;
    if records.is_empty() {
        return Err("known qualifying cavity store has no seed candidates".to_string());
    }
    if records.len() >= MAX_QUALIFYING_CANDIDATES {
        return Ok(MutationBatchStats {
            stopped_at_candidate_limit: true,
            ..MutationBatchStats::default()
        });
    }
    let cursor_path = path.with_extension("cursor");
    let start = fs::read_to_string(&cursor_path)
        .ok()
        .and_then(|text| text.trim().parse::<usize>().ok())
        .unwrap_or(0)
        % records.len();

    let mut known_signatures = std::collections::HashSet::new();
    let mut known_ids: std::collections::HashSet<String> =
        records.iter().map(|record| record.id.clone()).collect();
    for record in &records {
        validate_candidate_geometry(&record.structure, catalog).map_err(|error| {
            format!(
                "stored cavity candidate {} fails live geometry validation: {error}",
                record.id
            )
        })?;
        let cavity = crate::cavity::analyze_genome_cavity(&record.structure, catalog)?
            .ok_or_else(|| format!("stored cavity candidate {} no longer qualifies", record.id))?;
        if !cavity.qualifies()
            || (cavity.area - record.cavity_area).abs() > 1e-7
            || (cavity.minimum_area - record.minimum_cavity_area).abs() > 1e-7
        {
            return Err(format!(
                "stored cavity candidate {} has stale or invalid cavity metadata",
                record.id
            ));
        }
        known_signatures.insert(geometry_signature(&record.structure)?);
    }
    let mut stats = MutationBatchStats::default();
    let mut appended = Vec::<u8>::new();
    let mut stored_count = records.len();
    let stored_bytes = fs::metadata(path)
        .map_err(|e| format!("inspect cavity store size: {e}"))?
        .len();
    let mut parents_processed = 0usize;

    'parents: for offset in 0..records.len() {
        let parent_index = (start + offset) % records.len();
        let parent = &records[parent_index];
        parents_processed += 1;
        let mutations = one_step_mutations(parent, catalog);
        for (change, mut structure) in mutations {
            if stats.attempts >= MAX_MUTATION_ATTEMPTS_PER_RUN {
                break 'parents;
            }
            stats.attempts += 1;
            if change.starts_with("replace constituent")
                || change.starts_with("translate constituent")
                || change.starts_with("rotate constituent")
            {
                let prefix = if change.starts_with("replace constituent") {
                    "replace constituent at index "
                } else if change.starts_with("translate constituent") {
                    "translate constituent at index "
                } else {
                    "rotate constituent at index "
                };
                if let Some(unit_index) = change
                    .strip_prefix(prefix)
                    .and_then(|rest| rest.split_whitespace().next())
                    .and_then(|index| index.parse::<usize>().ok())
                {
                    if !rebuild_replacement_connections(&mut structure, unit_index, catalog) {
                        stats.rejected += 1;
                        stats.geometry_rejections += 1;
                        *stats
                            .rejection_reasons
                            .entry("replacement_has_no_repaired_contact".to_string())
                            .or_default() += 1;
                        continue;
                    }
                }
            }
            if structure.units.len() > parent.constituent_count {
                stats.rejected += 1;
                *stats
                    .rejection_reasons
                    .entry("constituent_count_increased".to_string())
                    .or_default() += 1;
                continue;
            }
            let signature = geometry_signature(&structure)?;
            if known_signatures.contains(&signature) {
                stats.duplicates += 1;
                continue;
            }
            if let Err(error) = validate_candidate_geometry(&structure, catalog) {
                stats.rejected += 1;
                stats.geometry_rejections += 1;
                let reason = if error.contains("overlap") {
                    "geometry_overlap"
                } else if error.contains("bond") {
                    "geometry_bond"
                } else {
                    "geometry_shape"
                };
                *stats
                    .rejection_reasons
                    .entry(reason.to_string())
                    .or_default() += 1;
                continue;
            }
            stats.cavity_checks_performed += 1;
            let cavity = match crate::cavity::analyze_genome_cavity(&structure, catalog) {
                Ok(Some(cavity)) if cavity.qualifies() => cavity,
                Ok(_) => {
                    stats.rejected += 1;
                    stats.cavity_rejections += 1;
                    *stats
                        .rejection_reasons
                        .entry("cavity_not_qualifying".to_string())
                        .or_default() += 1;
                    continue;
                }
                Err(error) => {
                    stats.rejected += 1;
                    stats.cavity_rejections += 1;
                    *stats
                        .rejection_reasons
                        .entry(format!("cavity_analysis_error: {error}"))
                        .or_default() += 1;
                    continue;
                }
            };
            if crate::organism_viability::validate_realized_organism(
                &structure,
                catalog,
                crate::organism_viability::OrganismViabilityRequirements {
                    require_sealed_genome_cavity: true,
                    require_extra_structure: false,
                },
            )
            .is_err()
            {
                stats.rejected += 1;
                stats.viability_rejections += 1;
                *stats
                    .rejection_reasons
                    .entry("viability_rejected".to_string())
                    .or_default() += 1;
                continue;
            }
            if stored_count >= MAX_QUALIFYING_CANDIDATES {
                stats.stopped_at_candidate_limit = true;
                break 'parents;
            }
            // Use the complete digest and guard IDs explicitly before appending.
            // This keeps the store valid even if a future ID scheme changes.
            let id = format!("cavity-{signature}");
            if known_ids.contains(&id) {
                stats.duplicates += 1;
                continue;
            }
            let record = KnownQualifyingCavity {
                schema_version: SCHEMA_VERSION,
                id: id.clone(),
                parent_id: Some(parent.id.clone()),
                change,
                constituent_count: structure.units.len(),
                cavity_area: cavity.area,
                minimum_cavity_area: cavity.minimum_area,
                structure,
            };
            let mut bytes = serde_json::to_vec(&record)
                .map_err(|e| format!("serialize qualifying mutation: {e}"))?;
            bytes.push(b'\n');
            if stored_bytes
                .saturating_add(appended.len() as u64)
                .saturating_add(bytes.len() as u64)
                > MAX_STORE_BYTES
            {
                stats.stopped_at_storage_limit = true;
                break 'parents;
            }
            appended.extend_from_slice(&bytes);
            stored_count += 1;
            known_ids.insert(id);
            known_signatures.insert(signature);
            stats.accepted += 1;
        }
    }

    if !appended.is_empty() {
        append_records_atomically(path, &appended)?;
    }
    let next_cursor = (start + parents_processed.max(1)) % records.len();
    fs::write(cursor_path, next_cursor.to_string())
        .map_err(|e| format!("save cavity mutation cursor: {e}"))?;
    Ok(stats)
}
