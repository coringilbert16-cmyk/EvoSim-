use super::*;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

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
    pub stopped_at_attempt_limit: bool,
    pub stopped_at_storage_limit: bool,
    pub stopped_at_search_exhaustion: bool,
    pub stopped_at_deadline: bool,
}

/// Stable parent-specific rotation of constituent priority. This keeps the
/// mutation phases ordered but prevents each parent from starting at index 0.
fn constituent_priority_order(parent_id: &str, constituent_count: usize) -> Vec<usize> {
    let mut order: Vec<usize> = (0..constituent_count).collect();
    if constituent_count > 1 {
        let offset = parent_id.bytes().fold(0usize, |hash, byte| {
            hash.wrapping_mul(31).wrapping_add(byte as usize)
        }) % constituent_count;
        order.rotate_left(offset);
    }
    order
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
    let unit_order = constituent_priority_order(&parent.id, original_count);

    // Resource substitutions are the primary mutation. For each swap and
    // each 15-degree rotation, test cardinal translations against the actual
    // rotated geometry. Keep at most two directions that are clear at both
    // 0.25 and 0.5 units, giving up to 24 × 2 × 2 = 96 poses per exchange.
    // Overlapping paths are omitted rather than queued for later rejection.
    for &unit_index in &unit_order {
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
            let mut base_swap = parent.structure.clone();
            base_swap.units[unit_index].material =
                crate::resources::Material::free_base(resource.name.clone(), 1.0);
            base_swap.units[unit_index].geometry = None;

            let cardinal_directions = [
                ("+x", 1.0, 0.0),
                ("-x", -1.0, 0.0),
                ("+y", 0.0, 1.0),
                ("-y", 0.0, -1.0),
            ];
            for step in 1..=24 {
                let angle_degrees = step * 15;
                let rotation_delta = angle_step * step as f64;
                let mut rotated_swap = base_swap.clone();
                rotated_swap.units[unit_index].placement.rotation_radians += rotation_delta;

                let mut clear_directions = Vec::new();
                for (direction, dx, dy) in cardinal_directions {
                    let clear_at_both_distances = [0.25_f64, 0.5_f64].into_iter().all(|distance| {
                        let mut probe = rotated_swap.clone();
                        probe.units[unit_index].placement.x += dx * distance;
                        probe.units[unit_index].placement.y += dy * distance;
                        validate_candidate_non_overlap(&probe, catalog).is_ok()
                    });
                    if clear_at_both_distances {
                        clear_directions.push((direction, dx, dy));
                    }
                }

                // Preserve deterministic direction ordering and never invent
                // an overlapping path. Fewer than two clear directions simply
                // means this rotation contributes fewer than four poses.
                for &(direction, dx, dy) in clear_directions.iter().take(2) {
                    for distance in [0.25_f64, 0.5_f64] {
                        let mut trial = rotated_swap.clone();
                        trial.units[unit_index].placement.x += dx * distance;
                        trial.units[unit_index].placement.y += dy * distance;
                        out.push((
                            format!(
                                "replace constituent at index {unit_index} with {} (rotate {angle_degrees}deg, move {direction} {distance:.2})",
                                resource.name
                            ),
                            trial,
                        ));
                    }
                }
            }
        }
    }

    // Translation is itself a cavity mutation. Move each unchanged piece in
    // cardinal and diagonal directions at 0.25-unit increments, out to 1.5
    // units. Generate only these eight rays, not the full 13x13 square grid.
    // This keeps the mutation set broad without exploding CI work.
    for &unit_index in &unit_order {
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
                let distance_label = if distance.fract() == 0.0 {
                    format!("{distance:.1}")
                } else {
                    format!("{distance:.2}")
                };
                let label = format!("move {direction} {distance_label}");
                out.push((
                    format!("translate constituent at index {unit_index} ({label})"),
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
    for &unit_index in &unit_order {
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
const MAX_CAVITY_MUTATION_RUN: Duration = Duration::from_secs(300);

pub(crate) fn mutate_once(
    path: impl AsRef<Path>,
    catalog: &[BaseResource],
) -> Result<MutationBatchStats, String> {
    mutate_once_until(
        path,
        catalog,
        Instant::now() + MAX_CAVITY_MUTATION_RUN,
        true,
    )
}

fn mutate_once_until(
    path: impl AsRef<Path>,
    catalog: &[BaseResource],
    deadline: Instant,
    validate_existing: bool,
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
    let mut records = load_records(path)?;
    if records.is_empty() {
        return Err("known qualifying cavity store has no seed candidates".to_string());
    }

    // The original 16-piece seed included four non-wall corner units. When an
    // older store is opened after the corner-free seed change, discard its
    // descendants and atomically restart from the updated, validated 12-piece
    // seed rather than silently continuing to train from the obsolete parent.
    let outdated_seed = records
        .iter()
        .find(|record| record.id == NITROGEN_RING_SEED_ID)
        .is_some_and(|record| record.constituent_count != 12);
    let cursor_path = path.with_extension("cursor");
    if outdated_seed {
        let structure = crate::initial_organism_constructor::build_nitrogen_ring_seed(catalog)?;
        validate_candidate_geometry(&structure, catalog)?;
        let cavity =
            crate::cavity::analyze_genome_cavity(&structure, catalog)?.ok_or_else(|| {
                "updated corner-free nitrogen seed has no sealed qualifying cavity".to_string()
            })?;
        if !cavity.qualifies() {
            return Err(
                "updated corner-free nitrogen seed failed cavity qualification".to_string(),
            );
        }
        crate::organism_viability::validate_realized_organism(
            &structure,
            catalog,
            crate::organism_viability::OrganismViabilityRequirements {
                require_sealed_genome_cavity: true,
                require_extra_structure: false,
            },
        )?;
        let seed = KnownQualifyingCavity {
            schema_version: SCHEMA_VERSION,
            id: NITROGEN_RING_SEED_ID.to_string(),
            parent_id: None,
            change: "initial validated corner-free 12-piece nitrogen cavity ring".to_string(),
            constituent_count: structure.units.len(),
            cavity_area: cavity.area,
            minimum_cavity_area: cavity.minimum_area,
            structure,
        };
        let mut bytes = serde_json::to_vec(&seed)
            .map_err(|error| format!("serialize updated nitrogen-ring seed: {error}"))?;
        bytes.push(b'\n');
        replace_records_atomically(path, &bytes)?;
        fs::write(&cursor_path, "0").map_err(|error| {
            format!("reset cavity mutation cursor after seed migration: {error}")
        })?;
        records = vec![seed];
    }

    if records.len() >= MAX_QUALIFYING_CANDIDATES {
        return Ok(MutationBatchStats {
            stopped_at_candidate_limit: true,
            ..MutationBatchStats::default()
        });
    }
    let cursor_text = fs::read_to_string(&cursor_path)
        .ok()
        .map(|text| text.trim().to_string())
        .unwrap_or_default();
    let (start, start_mutation) = match cursor_text.split_once(':') {
        Some((parent, mutation)) => (
            parent.parse::<usize>().unwrap_or(0),
            mutation.parse::<usize>().unwrap_or(0),
        ),
        None => (cursor_text.parse::<usize>().unwrap_or(0), 0),
    };
    let start = start % records.len();

    let mut known_signatures = std::collections::HashSet::new();
    let mut known_ids: std::collections::HashSet<String> =
        records.iter().map(|record| record.id.clone()).collect();
    for record in &records {
        if Instant::now() >= deadline {
            return Ok(MutationBatchStats {
                stopped_at_deadline: true,
                ..MutationBatchStats::default()
            });
        }
        if validate_existing {
            validate_candidate_geometry(&record.structure, catalog).map_err(|error| {
                format!(
                    "stored cavity candidate {} fails live geometry validation: {error}",
                    record.id
                )
            })?;
            let cavity = crate::cavity::analyze_genome_cavity(&record.structure, catalog)?
                .ok_or_else(|| {
                    format!("stored cavity candidate {} no longer qualifies", record.id)
                })?;
            if !cavity.qualifies()
                || (cavity.area - record.cavity_area).abs() > 1e-7
                || (cavity.minimum_area - record.minimum_cavity_area).abs() > 1e-7
            {
                return Err(format!(
                    "stored cavity candidate {} has stale or invalid cavity metadata",
                    record.id
                ));
            }
        }
        known_signatures.insert(geometry_signature(&record.structure)?);
    }
    let mut stats = MutationBatchStats::default();
    let mut appended = Vec::<u8>::new();
    let mut stored_count = records.len();
    let stored_bytes = fs::metadata(path)
        .map_err(|e| format!("inspect cavity store size: {e}"))?
        .len();
    let mut next_cursor_parent = start;
    let mut next_cursor_mutation = start_mutation;

    'parents: for offset in 0..records.len() {
        let parent_index = (start + offset) % records.len();
        let parent = &records[parent_index];
        let mutations = one_step_mutations(parent, catalog);
        for (mutation_index, (change, mut structure)) in mutations.into_iter().enumerate() {
            if parent_index == start && mutation_index < start_mutation {
                continue;
            }
            next_cursor_parent = parent_index;
            next_cursor_mutation = mutation_index;
            if stats.attempts >= MAX_MUTATION_ATTEMPTS_PER_RUN {
                stats.stopped_at_attempt_limit = true;
                break 'parents;
            }
            if Instant::now() >= deadline {
                next_cursor_mutation = mutation_index;
                stats.stopped_at_deadline = true;
                break 'parents;
            }
            stats.attempts += 1;
            next_cursor_mutation = mutation_index + 1;
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
            // Use the same strict penetration predicate as full validation
            // before expensive live contact reconstruction. Tangent edge/corner
            // contacts remain legal and full bond validation still follows.
            if let Err(error) = validate_candidate_non_overlap(&structure, catalog) {
                stats.rejected += 1;
                stats.geometry_rejections += 1;
                *stats
                    .rejection_reasons
                    .entry("geometry_overlap".to_string())
                    .or_default() += 1;
                let _ = error;
                continue;
            }
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
            if Instant::now() >= deadline {
                next_cursor_mutation = mutation_index;
                stats.stopped_at_deadline = true;
                break 'parents;
            }
            if let Err(error) = validate_candidate_shape_and_bonds(&structure, catalog) {
                stats.rejected += 1;
                stats.geometry_rejections += 1;
                let reason = if error.contains("bond") {
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
            if Instant::now() >= deadline {
                next_cursor_mutation = mutation_index;
                stats.stopped_at_deadline = true;
                break 'parents;
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
            if Instant::now() >= deadline {
                next_cursor_mutation = mutation_index;
                stats.stopped_at_deadline = true;
                break 'parents;
            }
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
            if Instant::now() >= deadline {
                next_cursor_mutation = mutation_index;
                stats.stopped_at_deadline = true;
                break 'parents;
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
        next_cursor_parent = (parent_index + 1) % records.len();
        next_cursor_mutation = 0;
    }

    if Instant::now() >= deadline {
        stats.stopped_at_deadline = true;
    }
    if !appended.is_empty() {
        append_records_atomically(path, &appended)?;
    }
    fs::write(
        cursor_path,
        format!("{next_cursor_parent}:{next_cursor_mutation}"),
    )
    .map_err(|e| format!("save cavity mutation cursor: {e}"))?;
    Ok(stats)
}

/// Continue bounded batches toward the 5,000-candidate cap, sharing one
/// five-minute deadline across the entire run. No individual batch resets it.
pub(crate) fn mutate_until_full(
    path: impl AsRef<Path>,
    catalog: &[BaseResource],
) -> Result<MutationBatchStats, String> {
    let path = path.as_ref();
    let deadline = Instant::now() + MAX_CAVITY_MUTATION_RUN;
    let mut total = MutationBatchStats::default();
    let mut validate_existing = true;

    loop {
        if Instant::now() >= deadline {
            total.stopped_at_deadline = true;
            break;
        }
        let batch = mutate_once_until(path, catalog, deadline, validate_existing)?;
        validate_existing = false;
        total.attempts += batch.attempts;
        total.accepted += batch.accepted;
        total.rejected += batch.rejected;
        total.duplicates += batch.duplicates;
        total.geometry_rejections += batch.geometry_rejections;
        total.cavity_checks_performed += batch.cavity_checks_performed;
        total.cavity_rejections += batch.cavity_rejections;
        total.viability_rejections += batch.viability_rejections;
        for (reason, count) in batch.rejection_reasons {
            *total.rejection_reasons.entry(reason).or_default() += count;
        }
        total.stopped_at_candidate_limit |= batch.stopped_at_candidate_limit;
        total.stopped_at_attempt_limit |= batch.stopped_at_attempt_limit;
        total.stopped_at_storage_limit |= batch.stopped_at_storage_limit;
        total.stopped_at_search_exhaustion |= batch.stopped_at_search_exhaustion;
        total.stopped_at_deadline |= batch.stopped_at_deadline;

        let stored_count = load_records(path)?.len();
        if stored_count >= MAX_QUALIFYING_CANDIDATES {
            total.stopped_at_candidate_limit = true;
            break;
        }
        if total.stopped_at_storage_limit
            || total.stopped_at_deadline
            || total.stopped_at_candidate_limit
        {
            break;
        }
        if batch.attempts == 0
            || (batch.accepted == 0 && batch.attempts < MAX_MUTATION_ATTEMPTS_PER_RUN)
        {
            total.stopped_at_search_exhaustion = true;
            break;
        }
    }
    Ok(total)
}

#[cfg(test)]
mod mutation_priority_tests {
    use super::{constituent_priority_order, mutate_once_until};
    use std::collections::HashSet;
    use std::time::{Duration, Instant};

    #[test]
    fn expired_deadline_stops_before_mutation_attempts() {
        let path = std::env::temp_dir().join(format!(
            "evosim-cavity-deadline-{}.jsonl",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let catalog = crate::resources::default_catalog();
        let stats = mutate_once_until(
            &path,
            &catalog,
            Instant::now() - Duration::from_secs(1),
            true,
        )
        .expect("expired deadline should be reported as a bounded stop");
        assert!(stats.stopped_at_deadline);
        assert_eq!(stats.attempts, 0);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("cursor"));
    }

    #[test]
    fn constituent_priority_is_stable_and_varies_between_parents() {
        let ids = [
            "nitrogen-ring-seed-v1",
            "cavity-alpha",
            "cavity-beta",
            "cavity-gamma",
            "cavity-delta",
            "cavity-epsilon",
            "cavity-zeta",
            "cavity-eta",
        ];
        let first_indices: HashSet<usize> = ids
            .iter()
            .map(|id| {
                let first_order = constituent_priority_order(id, 16);
                assert_eq!(
                    first_order,
                    constituent_priority_order(id, 16),
                    "the same parent must regenerate the same mutation order"
                );
                first_order[0]
            })
            .collect();
        assert!(
            first_indices.len() > 1,
            "fresh parents must not all start at constituent index zero"
        );
        for id in ids {
            let order = constituent_priority_order(id, 16);
            assert_eq!(order.len(), 16);
            assert_eq!(order.iter().copied().collect::<HashSet<_>>().len(), 16);
        }
    }
}
