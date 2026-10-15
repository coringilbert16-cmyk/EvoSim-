use super::*;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct MutationBatchStats {
    pub attempts: usize,
    pub accepted: usize,
    pub rejected: usize,
    pub duplicates: usize,
    pub stopped_at_candidate_limit: bool,
    pub stopped_at_storage_limit: bool,
}

/// Generate a bounded set of one-operation variants. Every operator preserves
/// or reduces constituent count; replacement swaps one resource in place.
pub(super) fn one_step_mutations(
    parent: &KnownQualifyingCavity,
    catalog: &[BaseResource],
) -> Vec<(String, OrganismStructure)> {
    let angle_step = std::f64::consts::PI / 12.0;
    let mut out = Vec::new();
    let original_count = parent.structure.units.len();
    for unit_index in 0..original_count {
        let unit_id = parent.structure.units[unit_index].physical_id;
        let mut removed = parent.structure.clone();
        removed.remove_units_by_physical_ids(&[unit_id]);
        out.push((format!("remove constituent at index {unit_index}"), removed));

        for (label, dx, dy) in [
            ("move +x", 0.05, 0.0),
            ("move -x", -0.05, 0.0),
            ("move +y", 0.0, 0.05),
            ("move -y", 0.0, -0.05),
        ] {
            let mut moved = parent.structure.clone();
            moved.units[unit_index].placement.x += dx;
            moved.units[unit_index].placement.y += dy;
            out.push((format!("{label} constituent at index {unit_index}"), moved));
        }
        for (label, delta) in [
            ("rotate +15deg", angle_step),
            ("rotate -15deg", -angle_step),
        ] {
            let mut rotated = parent.structure.clone();
            rotated.units[unit_index].placement.rotation_radians += delta;
            out.push((
                format!("{label} constituent at index {unit_index}"),
                rotated,
            ));
        }

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
            let mut replaced = parent.structure.clone();
            replaced.units[unit_index].material =
                crate::resources::Material::free_base(resource.name.clone(), 1.0);
            replaced.units[unit_index].geometry = None;
            out.push((
                format!(
                    "replace constituent at index {unit_index} with {}",
                    resource.name
                ),
                replaced,
            ));
        }
    }
    for bond_index in 0..parent.structure.bonds.len() {
        let mut changed = parent.structure.clone();
        changed.break_bond(bond_index);
        out.push((format!("remove bond at index {bond_index}"), changed));
    }
    out
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
    let cursor_path = path.with_extension("cursor");
    let start = fs::read_to_string(&cursor_path)
        .ok()
        .and_then(|text| text.trim().parse::<usize>().ok())
        .unwrap_or(0)
        % records.len();

    let mut known_signatures = std::collections::HashSet::new();
    for record in &records {
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
        for (change, structure) in mutations {
            if stats.attempts >= MAX_MUTATION_ATTEMPTS_PER_RUN {
                break 'parents;
            }
            stats.attempts += 1;
            if structure.units.len() > parent.constituent_count {
                stats.rejected += 1;
                continue;
            }
            let signature = geometry_signature(&structure)?;
            if known_signatures.contains(&signature) {
                stats.duplicates += 1;
                continue;
            }
            if validate_candidate_geometry(&structure, catalog).is_err() {
                stats.rejected += 1;
                continue;
            }
            let cavity = match crate::cavity::analyze_genome_cavity(&structure, catalog) {
                Ok(Some(cavity)) if cavity.qualifies() => cavity,
                _ => {
                    stats.rejected += 1;
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
                continue;
            }
            if stored_count >= MAX_QUALIFYING_CANDIDATES {
                stats.stopped_at_candidate_limit = true;
                break 'parents;
            }
            let id = format!("cavity-{}", &signature[..24]);
            let record = KnownQualifyingCavity {
                schema_version: SCHEMA_VERSION,
                id,
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
            known_signatures.insert(signature);
            stats.accepted += 1;
        }
    }

    if !appended.is_empty() {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| format!("open cavity store for mutation append: {e}"))?;
        file.write_all(&appended)
            .map_err(|e| format!("append qualifying mutations: {e}"))?;
        file.sync_data()
            .map_err(|e| format!("sync qualifying mutations: {e}"))?;
    }
    let next_cursor = (start + parents_processed.max(1)) % records.len();
    fs::write(cursor_path, next_cursor.to_string())
        .map_err(|e| format!("save cavity mutation cursor: {e}"))?;
    Ok(stats)
}
