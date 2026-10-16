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

/// Choose the deepest already-persisted lineage leaf when migrating from the
/// earlier breadth-first cursor. Ties prefer the most recently stored leaf.
fn deepest_lineage_leaf(records: &[KnownQualifyingCavity]) -> usize {
    let mut depths = vec![0usize; records.len()];
    let mut parent_indices = std::collections::HashSet::new();
    let index_by_id: std::collections::HashMap<&str, usize> = records
        .iter()
        .enumerate()
        .map(|(index, record)| (record.id.as_str(), index))
        .collect();

    for (index, record) in records.iter().enumerate() {
        if let Some(parent_id) = record.parent_id.as_deref() {
            if let Some(&parent_index) = index_by_id.get(parent_id) {
                depths[index] = depths[parent_index].saturating_add(1);
                parent_indices.insert(parent_index);
            }
        }
    }

    (0..records.len())
        .filter(|index| !parent_indices.contains(index))
        .max_by_key(|&index| (depths[index], index))
        .unwrap_or(0)
}

/// Extract the constituent index from the last accepted mutation so the
/// lineage can continue evolving that same piece before trying unrelated edits.
fn changed_constituent_index(change: &str) -> Option<usize> {
    [
        "replace constituent at index ",
        "translate constituent at index ",
        "rotate constituent at index ",
    ]
    .iter()
    .find_map(|prefix| {
        change
            .strip_prefix(prefix)
            .and_then(|rest| rest.split_whitespace().next())
            .and_then(|index| index.parse::<usize>().ok())
    })
}

/// Append independently validated cardinal translations for one constituent.
fn append_translations(
    out: &mut Vec<(String, OrganismStructure)>,
    parent: &KnownQualifyingCavity,
    catalog: &[BaseResource],
    unit_index: usize,
) {
    for (direction, dx, dy) in [
        ("+x", 1.0, 0.0),
        ("-x", -1.0, 0.0),
        ("+y", 0.0, 1.0),
        ("-y", 0.0, -1.0),
    ] {
        for distance in [0.25_f64, 0.5_f64] {
            let mut trial = parent.structure.clone();
            trial.units[unit_index].placement.x += dx * distance;
            trial.units[unit_index].placement.y += dy * distance;
            // Test each distance separately: an overlapping 0.5 move must not
            // suppress a clear 0.25 move in the same direction.
            if validate_candidate_non_overlap(&trial, catalog).is_ok() {
                out.push((
                    format!(
                        "translate constituent at index {unit_index} {direction} {distance:.2}"
                    ),
                    trial,
                ));
            }
        }
    }
}

/// Append independently validated in-place rotations for one constituent.
fn append_rotations(
    out: &mut Vec<(String, OrganismStructure)>,
    parent: &KnownQualifyingCavity,
    catalog: &[BaseResource],
    unit_index: usize,
) {
    let angle_step = std::f64::consts::PI / 12.0;
    for step in 1..=24 {
        let angle_degrees = step * 15;
        let mut trial = parent.structure.clone();
        trial.units[unit_index].placement.rotation_radians += angle_step * step as f64;
        if validate_candidate_non_overlap(&trial, catalog).is_ok() {
            out.push((
                format!("rotate constituent at index {unit_index} by {angle_degrees}deg"),
                trial,
            ));
        }
    }
}

/// Generate single-change mutation trials in deliberate stages:
/// (1) resource replacement at the existing pose, (2) independent translations,
/// (3) independent rotations, and (4) removal as the last resort. No trial
/// combines a replacement with a move or rotation. After a successful mutation,
/// first continue with the same constituent on the next lineage step.
pub(super) fn one_step_mutations(
    parent: &KnownQualifyingCavity,
    catalog: &[BaseResource],
) -> Vec<(String, OrganismStructure)> {
    let mut out = Vec::new();
    let original_count = parent.structure.units.len();
    let preferred_index = changed_constituent_index(&parent.change)
        .filter(|&index| index < original_count);

    // A child continues with its last changed piece first: after a replacement,
    // try moving that replacement at its existing orientation, then rotating it.
    if let Some(unit_index) = preferred_index {
        append_translations(&mut out, parent, catalog, unit_index);
        append_rotations(&mut out, parent, catalog, unit_index);
    }

    // Stage 1: change one resource, leaving its pose untouched. Do not immediately
    // replace the same constituent again before testing its geometric consequences.
    for unit_index in 0..original_count {
        if Some(unit_index) == preferred_index {
            continue;
        }
        let current_name = parent.structure.units[unit_index]
            .material
            .parts
            .first()
            .map(|part| part.0.as_str())
            .unwrap_or("");
        for resource in catalog.iter().filter(|resource| resource.name != current_name) {
            let mut trial = parent.structure.clone();
            trial.units[unit_index].material =
                crate::resources::Material::free_base(resource.name.clone(), 1.0);
            trial.units[unit_index].geometry = None;
            out.push((
                format!(
                    "replace constituent at index {unit_index} with {} (existing pose)",
                    resource.name
                ),
                trial,
            ));
        }
    }

    // Stage 2: translations of all other constituents.
    for unit_index in 0..original_count {
        if Some(unit_index) != preferred_index {
            append_translations(&mut out, parent, catalog, unit_index);
        }
    }

    // Stage 3: rotations of all other constituents.
    for unit_index in 0..original_count {
        if Some(unit_index) != preferred_index {
            append_rotations(&mut out, parent, catalog, unit_index);
        }
    }

    // Stage 4: destructive removal is deliberately last. Adding constituents
    // is not a mutation in this phase.
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
    // New cursors explicitly track one lineage. A legacy rotating-parent cursor
    // migrates to the deepest existing leaf rather than returning to the seed.
    let (start, start_mutation) = if let Some(rest) = cursor_text.strip_prefix("lineage:") {
        let (parent, mutation) = rest.split_once(':').unwrap_or((rest, "0"));
        (
            parent.parse::<usize>().unwrap_or_else(|_| deepest_lineage_leaf(&records))
                .min(records.len() - 1),
            mutation.parse::<usize>().unwrap_or(0),
        )
    } else {
        (deepest_lineage_leaf(&records), 0)
    };

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

    // Only mutate the current lineage leaf. When one mutation qualifies, stop
    // this batch and resume from that newly stored child on the next batch.
    'parents: {
        let parent_index = start;
        let parent = &records[parent_index];
        let mutations = one_step_mutations(parent, catalog);
        let mutation_count = mutations.len();
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

            // The accepted child becomes the sole parent for the next step.
            // Stop before trying sibling mutations from the previous parent.
            next_cursor_parent = stored_count - 1;
            next_cursor_mutation = 0;
            break 'parents;
        }
        next_cursor_parent = parent_index;
        next_cursor_mutation = mutation_count;
    }

    if Instant::now() >= deadline {
        stats.stopped_at_deadline = true;
    }
    if !appended.is_empty() {
        append_records_atomically(path, &appended)?;
    }
    fs::write(
        cursor_path,
        format!("lineage:{next_cursor_parent}:{next_cursor_mutation}"),
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
mod lineage_tests {
    use super::{deepest_lineage_leaf, mutate_once_until};
    use crate::bob::known_qualifying_cavities::{KnownQualifyingCavity, SCHEMA_VERSION};
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
    fn legacy_frontier_migrates_to_deepest_lineage_leaf() {
        let catalog = crate::resources::default_catalog();
        let structure = crate::initial_organism_constructor::build_nitrogen_ring_seed(&catalog)
            .expect("seed structure should build");
        let record = |id: &str, parent_id: Option<&str>| KnownQualifyingCavity {
            schema_version: SCHEMA_VERSION,
            id: id.to_string(),
            parent_id: parent_id.map(str::to_string),
            change: "test lineage".to_string(),
            constituent_count: structure.units.len(),
            cavity_area: 9.0,
            minimum_cavity_area: 3.0,
            structure: structure.clone(),
        };
        let records = vec![
            record("seed", None),
            record("branch-a", Some("seed")),
            record("branch-b", Some("branch-a")),
            record("branch-c", Some("seed")),
        ];
        assert_eq!(deepest_lineage_leaf(&records), 2);
    }
}
