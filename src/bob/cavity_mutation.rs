use super::*;
use std::collections::{BTreeMap, HashMap, HashSet};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use rand::seq::SliceRandom;

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
    pub stopped_at_search_exhaustion: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct MutationFrontier {
    active_parent_id: Option<String>,
    next_indices: HashMap<String, usize>,
    exhausted_parent_ids: HashSet<String>,
    validated_record_count: usize,
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
            // Explore the complete 15-degree rotation cycle, then every
            // 0.25-unit translation on a square grid out to 1.5 units.
            let mut poses: Vec<(String, f64, f64, f64)> =
                vec![("existing pose".to_string(), 0.0, 0.0, 0.0)];
            for step in 1..24 {
                poses.push((
                    format!("rotate +{}deg", step * 15),
                    0.0,
                    0.0,
                    angle_step * step as f64,
                ));
            }
            for x_step in -6i32..=6 {
                for y_step in -6i32..=6 {
                    if x_step == 0 && y_step == 0 {
                        continue;
                    }
                    let dx = x_step as f64 * 0.25;
                    let dy = y_step as f64 * 0.25;
                    poses.push((
                        format!("translate dx={dx:.2} dy={dy:.2}"),
                        dx,
                        dy,
                        0.0,
                    ));
                }
            }
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

    // After trying resource substitutions, move each unchanged constituent
    // independently. Translation is itself a cavity mutation: a boundary
    // piece can move outward, reconnect at different corners, and change the
    // measured enclosed area without changing its material.
    for unit_index in 0..original_count {
        for x_step in -6i32..=6 {
            for y_step in -6i32..=6 {
                if x_step == 0 && y_step == 0 {
                    continue;
                }
                let dx = x_step as f64 * 0.25;
                let dy = y_step as f64 * 0.25;
                let label = format!("move dx={dx:.2} dy={dy:.2}");
                let mut trial = parent.structure.clone();
                trial.units[unit_index].placement.x += dx;
                trial.units[unit_index].placement.y += dy;
                out.push((
                    format!("translate constituent at index {unit_index} ({label})"),
                    trial,
                ));
            }
        }
        // Rotations of unchanged constituents are separate mutations.
        for step in 1..24 {
            let mut trial = parent.structure.clone();
            trial.units[unit_index].placement.rotation_radians += angle_step * step as f64;
            out.push((
                format!("rotate constituent at index {unit_index} by {}deg", step * 15),
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

/// Explore a bounded batch from randomly selected known-good parents. A newly
/// accepted formation is persisted before the next call, so it joins the parent
/// pool immediately. Live geometry and cavity validation remain authoritative.
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
    let frontier_path = path.with_extension("frontier.json");
    let mut frontier: MutationFrontier = fs::read_to_string(&frontier_path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();

    let mut known_signatures = std::collections::HashSet::new();
    let mut known_ids: std::collections::HashSet<String> =
        records.iter().map(|record| record.id.clone()).collect();
    for (record_index, record) in records.iter().enumerate() {
        if record_index >= frontier.validated_record_count {
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
        }
        known_signatures.insert(geometry_signature(&record.structure)?);
    }
    let mut stats = MutationBatchStats::default();
    let mut appended = Vec::<u8>::new();
    let mut stored_count = records.len();
    let stored_bytes = fs::metadata(path)
        .map_err(|e| format!("inspect cavity store size: {e}"))?
        .len();
    let known_parent_ids: HashSet<&str> = records.iter().map(|record| record.id.as_str()).collect();
    if frontier.active_parent_id.as_deref().is_some_and(|id| !known_parent_ids.contains(id)) {
        frontier.active_parent_id = None;
    }

    let selected_parent_id = frontier.active_parent_id.clone().or_else(|| {
        let mut eligible: Vec<&KnownQualifyingCavity> = records
            .iter()
            .filter(|record| !frontier.exhausted_parent_ids.contains(&record.id))
            .collect();
        eligible.shuffle(&mut rand::thread_rng());
        eligible.first().map(|record| record.id.clone())
    });

    let Some(selected_parent_id) = selected_parent_id else {
        stats.stopped_at_search_exhaustion = true;
        fs::write(&frontier_path, serde_json::to_vec_pretty(&frontier)
            .map_err(|e| format!("serialize cavity mutation frontier: {e}"))?)
            .map_err(|e| format!("save cavity mutation frontier: {e}"))?;
        return Ok(stats);
    };
    let parent_index = records.iter().position(|record| record.id == selected_parent_id)
        .ok_or_else(|| format!("active cavity parent {selected_parent_id} is missing"))?;
    let parent = &records[parent_index];
    let mutations = one_step_mutations(parent, catalog);
    let mut next_index = frontier.next_indices.get(&parent.id).copied().unwrap_or(0);

    while next_index < mutations.len() && stats.attempts < MAX_MUTATION_ATTEMPTS_PER_RUN {
        let (change, mut structure) = mutations[next_index].clone();
        next_index += 1;
        frontier.next_indices.insert(parent.id.clone(), next_index);
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
            let reason = if error.contains("overlap") { "geometry_overlap" }
                else if error.contains("bond") { "geometry_bond" }
                else { "geometry_shape" };
            *stats.rejection_reasons.entry(reason.to_string()).or_default() += 1;
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
        ).is_err() {
            stats.rejected += 1;
            stats.viability_rejections += 1;
            *stats.rejection_reasons.entry("viability_rejected".to_string()).or_default() += 1;
            continue;
        }
        if stored_count >= MAX_QUALIFYING_CANDIDATES {
            stats.stopped_at_candidate_limit = true;
            break;
        }
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
            break;
        }
        appended.extend_from_slice(&bytes);
        stored_count += 1;
        known_ids.insert(id);
        known_signatures.insert(signature);
        stats.accepted += 1;
        // A discovery ends this parent's turn. Its remaining mutations stay
        // resumable, but the next call chooses a fresh random parent.
        frontier.active_parent_id = None;
        break;
    }

    if next_index >= mutations.len() {
        frontier.exhausted_parent_ids.insert(parent.id.clone());
        frontier.active_parent_id = None;
    } else if stats.accepted == 0
        && !stats.stopped_at_storage_limit
        && !stats.stopped_at_candidate_limit
    {
        // Continue this parent's unfinished mutation list on the next batch.
        frontier.active_parent_id = Some(parent.id.clone());
    }
    if stats.stopped_at_storage_limit {
        // Retry this exact mutation if the storage budget is raised later.
        frontier
            .next_indices
            .insert(parent.id.clone(), next_index.saturating_sub(1));
        frontier.exhausted_parent_ids.remove(&parent.id);
        frontier.active_parent_id = None;
    }
    frontier.validated_record_count = stored_count;
    if !appended.is_empty() {
        append_records_atomically(path, &appended)?;
    }
    fs::write(&frontier_path, serde_json::to_vec_pretty(&frontier)
        .map_err(|e| format!("serialize cavity mutation frontier: {e}"))?)
        .map_err(|e| format!("save cavity mutation frontier: {e}"))?;
    Ok(stats)
}

/// Keep running bounded mutation batches until the 5,000-record target is
/// reached, storage is exhausted, or every persisted parent frontier is exhausted.
pub(crate) fn mutate_until_full(
    path: impl AsRef<Path>,
    catalog: &[BaseResource],
) -> Result<MutationBatchStats, String> {
    let path = path.as_ref();
    let mut total = MutationBatchStats::default();
    loop {
        let batch = mutate_once(path, catalog)?;
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
        total.stopped_at_storage_limit |= batch.stopped_at_storage_limit;
        total.stopped_at_search_exhaustion |= batch.stopped_at_search_exhaustion;

        if total.stopped_at_candidate_limit
            || total.stopped_at_storage_limit
            || total.stopped_at_search_exhaustion
        {
            break;
        }
    }
    Ok(total)
}
