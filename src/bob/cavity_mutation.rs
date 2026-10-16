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
    /// Attempts grouped by mutation family, so a low overall success rate can be diagnosed.
    pub mutation_attempts_by_operation: BTreeMap<String, usize>,
    /// Terminal outcome counts keyed as "<operation>:<outcome>".
    pub mutation_outcomes_by_operation: BTreeMap<String, usize>,
    /// Replacement attempts grouped by target resource name.
    pub mutation_attempts_by_resource: BTreeMap<String, usize>,
    /// Replacement outcomes keyed as "<resource>:<outcome>".
    pub mutation_outcomes_by_resource: BTreeMap<String, usize>,
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
    .copied()
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
        for distance in [0.25_f64, 0.5_f64, 1.0_f64] {
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

/// Place an exchanged piece by aligning one of its edges between vertices of
/// the original piece's bonded neighbors. This targets real contact constraints
/// instead of relying only on arbitrary translation increments.
fn append_contact_guided_replacements(
    out: &mut Vec<(String, OrganismStructure)>,
    parent: &KnownQualifyingCavity,
    replaced: &OrganismStructure,
    catalog: &[BaseResource],
    unit_index: usize,
    resource_name: &str,
) {
    let Some(changed_id) = parent
        .structure
        .units
        .get(unit_index)
        .map(|unit| unit.physical_id)
    else {
        return;
    };

    let mut neighbor_ids = std::collections::HashSet::new();
    for bond in &parent.structure.bonds {
        if bond.endpoint_a.constituent_id == changed_id {
            neighbor_ids.insert(bond.endpoint_b.constituent_id);
        } else if bond.endpoint_b.constituent_id == changed_id {
            neighbor_ids.insert(bond.endpoint_a.constituent_id);
        }
    }
    let neighbors = parent
        .structure
        .units
        .iter()
        .enumerate()
        .filter(|(_, unit)| neighbor_ids.contains(&unit.physical_id))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if neighbors.len() < 2 {
        return;
    }

    let Some(candidate_shape) = replaced.units[unit_index].shape(catalog) else {
        return;
    };
    let Some(candidate_vertices) = candidate_shape.form.polygon_vertices() else {
        return;
    };
    if candidate_vertices.len() < 3 {
        return;
    }

    for left_position in 0..neighbors.len() {
        let left_index = neighbors[left_position];
        let Some(left_unit) = parent.structure.units.get(left_index) else {
            continue;
        };
        let Some(left_shape) = left_unit.shape(catalog) else {
            continue;
        };
        let Some(left_local_vertices) = left_shape.form.polygon_vertices() else {
            continue;
        };
        let (left_sin, left_cos) = left_unit.placement.rotation_radians.sin_cos();
        let left_vertices = left_local_vertices
            .into_iter()
            .map(|(x, y)| {
                (
                    left_unit.placement.x + x * left_cos - y * left_sin,
                    left_unit.placement.y + x * left_sin + y * left_cos,
                )
            })
            .collect::<Vec<_>>();

        for &right_index in &neighbors[(left_position + 1)..] {
            let Some(right_unit) = parent.structure.units.get(right_index) else {
                continue;
            };
            let Some(right_shape) = right_unit.shape(catalog) else {
                continue;
            };
            let Some(right_local_vertices) = right_shape.form.polygon_vertices() else {
                continue;
            };
            let (right_sin, right_cos) = right_unit.placement.rotation_radians.sin_cos();
            let right_vertices = right_local_vertices
                .into_iter()
                .map(|(x, y)| {
                    (
                        right_unit.placement.x + x * right_cos - y * right_sin,
                        right_unit.placement.y + x * right_sin + y * right_cos,
                    )
                })
                .collect::<Vec<_>>();

            for &left_point in &left_vertices {
                for &right_point in &right_vertices {
                    let target_dx = right_point.0 - left_point.0;
                    let target_dy = right_point.1 - left_point.1;
                    let target_length = target_dx.hypot(target_dy);
                    if target_length <= 1e-7 {
                        continue;
                    }

                    for edge_index in 0..candidate_vertices.len() {
                        let local_start = candidate_vertices[edge_index];
                        let local_end =
                            candidate_vertices[(edge_index + 1) % candidate_vertices.len()];
                        let local_dx = local_end.0 - local_start.0;
                        let local_dy = local_end.1 - local_start.1;
                        if (local_dx.hypot(local_dy) - target_length).abs() > 1e-5 {
                            continue;
                        }
                        let local_angle = local_dy.atan2(local_dx);
                        for (target_start, target_end) in
                            [(left_point, right_point), (right_point, left_point)]
                        {
                            let target_angle = (target_end.1 - target_start.1)
                                .atan2(target_end.0 - target_start.0);
                            let rotation = target_angle - local_angle;
                            let (sin, cos) = rotation.sin_cos();
                            let rotated_start = (
                                local_start.0 * cos - local_start.1 * sin,
                                local_start.0 * sin + local_start.1 * cos,
                            );
                            let mut trial = replaced.clone();
                            trial.units[unit_index].placement.rotation_radians = rotation;
                            trial.units[unit_index].placement.x = target_start.0 - rotated_start.0;
                            trial.units[unit_index].placement.y = target_start.1 - rotated_start.1;

                            if validate_candidate_non_overlap(&trial, catalog).is_ok() {
                                out.push((
                                    format!(
                                        "replace constituent at index {unit_index} with {resource_name} then align edge {edge_index} between neighbors"
                                    ),
                                    trial,
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Generate a resource exchange together with bounded local pose alternatives.
/// Replacements are searched before unrelated translations/rotations so a new
/// material gets a fair chance to form a valid, distinct cavity wall.
fn append_replacement_variants(
    out: &mut Vec<(String, OrganismStructure)>,
    parent: &KnownQualifyingCavity,
    catalog: &[BaseResource],
    unit_index: usize,
    resource_name: &str,
) {
    let mut replaced = parent.structure.clone();
    replaced.units[unit_index].material =
        crate::resources::Material::free_base(resource_name.to_string(), 1.0);
    replaced.units[unit_index].geometry = None;

    out.push((
        format!("replace constituent at index {unit_index} with {resource_name} (existing pose)"),
        replaced.clone(),
    ));
    append_contact_guided_replacements(out, parent, &replaced, catalog, unit_index, resource_name);

    // First search orientation-only alternatives at the existing center.
    let angle_step = std::f64::consts::PI / 12.0;
    for step in 1..=24 {
        let mut trial = replaced.clone();
        trial.units[unit_index].placement.rotation_radians += angle_step * step as f64;
        if validate_candidate_non_overlap(&trial, catalog).is_ok() {
            out.push((
                format!(
                    "replace constituent at index {unit_index} with {resource_name} then rotate {}deg",
                    step * 15
                ),
                trial,
            ));
        }
    }

    // Then search cardinal translations at the exchanged piece's orientation.
    for (direction, dx, dy) in [
        ("+x", 1.0, 0.0),
        ("-x", -1.0, 0.0),
        ("+y", 0.0, 1.0),
        ("-y", 0.0, -1.0),
    ] {
        for distance in [0.25_f64, 0.5_f64, 1.0_f64] {
            let mut trial = replaced.clone();
            trial.units[unit_index].placement.x += dx * distance;
            trial.units[unit_index].placement.y += dy * distance;
            if validate_candidate_non_overlap(&trial, catalog).is_ok() {
                out.push((
                    format!(
                        "replace constituent at index {unit_index} with {resource_name} then translate {direction} {distance:.2}"
                    ),
                    trial,
                ));
            }
        }
    }

    // Finally combine a representative set of rotations and translations.
    // This avoids multiplying every possible pose while testing useful local fits.
    for angle_step_count in [1usize, 3, 6] {
        for (direction, dx, dy) in [
            ("+x", 1.0, 0.0),
            ("-x", -1.0, 0.0),
            ("+y", 0.0, 1.0),
            ("-y", 0.0, -1.0),
        ] {
            for distance in [0.5_f64, 1.0_f64] {
                let mut trial = replaced.clone();
                trial.units[unit_index].placement.rotation_radians +=
                    angle_step * angle_step_count as f64;
                trial.units[unit_index].placement.x += dx * distance;
                trial.units[unit_index].placement.y += dy * distance;
                if validate_candidate_non_overlap(&trial, catalog).is_ok() {
                    out.push((
                        format!(
                            "replace constituent at index {unit_index} with {resource_name} then rotate {}deg and translate {direction} {distance:.2}",
                            angle_step_count * 15
                        ),
                        trial,
                    ));
                }
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

fn mutation_operation(change: &str) -> &'static str {
    if change.starts_with("replace constituent") {
        "replace"
    } else if change.starts_with("translate constituent") {
        "translate"
    } else if change.starts_with("rotate constituent") {
        "rotate"
    } else if change.starts_with("remove constituent") {
        "remove"
    } else {
        "other"
    }
}

fn replacement_resource(change: &str) -> Option<&str> {
    change
        .strip_prefix("replace constituent at index ")?
        .split_once(" with ")?
        .1
        .split(" then ")
        .next()?
        .split(" (")
        .next()
}

fn record_mutation_attempt(stats: &mut MutationBatchStats, change: &str, operation: &str) {
    *stats
        .mutation_attempts_by_operation
        .entry(operation.to_string())
        .or_default() += 1;
    if let Some(resource) = replacement_resource(change) {
        *stats
            .mutation_attempts_by_resource
            .entry(resource.to_string())
            .or_default() += 1;
    }
}

fn record_mutation_outcome(
    stats: &mut MutationBatchStats,
    change: &str,
    operation: &str,
    outcome: &str,
) {
    *stats
        .mutation_outcomes_by_operation
        .entry(format!("{operation}:{outcome}"))
        .or_default() += 1;
    if let Some(resource) = replacement_resource(change) {
        *stats
            .mutation_outcomes_by_resource
            .entry(format!("{resource}:{outcome}"))
            .or_default() += 1;
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
    let preferred_index =
        changed_constituent_index(&parent.change).filter(|&index| index < original_count);

    // Stage 1: try every single-resource replacement at its unchanged pose first.
    // Put the lineage's recently changed constituent first, but do not skip it:
    // otherwise repeated translations/rotations can starve all substitutions.
    let mut replacement_indices = Vec::with_capacity(original_count);
    if let Some(index) = preferred_index {
        replacement_indices.push(index);
    }
    replacement_indices.extend((0..original_count).filter(|index| Some(*index) != preferred_index));
    for unit_index in replacement_indices {
        let current_name = parent.structure.units[unit_index]
            .material
            .parts
            .first()
            .map(|part| part.0.as_str())
            .unwrap_or("");
        // The cavity validator requires polygonal exposed boundary segments.
        // Circle-shaped Water cannot own such a segment, so skip impossible
        // cavity-wall exchanges while keeping Water in Bob's general geometry library.
        for resource in catalog.iter().filter(|resource| {
            resource.name != current_name && resource.shape.form.polygon_vertices().is_some()
        }) {
            append_replacement_variants(&mut out, parent, catalog, unit_index, &resource.name);
        }
    }

    // Stage 2: independent translations. Continue locally with the recently
    // changed piece first, but only after all resource substitutions were tried.
    if let Some(unit_index) = preferred_index {
        append_translations(&mut out, parent, catalog, unit_index);
    }
    for unit_index in 0..original_count {
        if Some(unit_index) != preferred_index {
            append_translations(&mut out, parent, catalog, unit_index);
        }
    }

    // Stage 3: independent in-place rotations, again preferring the lineage piece.
    if let Some(unit_index) = preferred_index {
        append_rotations(&mut out, parent, catalog, unit_index);
    }
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

/// Behavioral descriptors allocate search effort; they never define physical
/// identity or replace the authoritative cavity validator.
fn diversity_features(record: &KnownQualifyingCavity) -> Vec<String> {
    let area_bucket = (record.cavity_area / 0.25).round() as i64;
    let min_x = record
        .structure
        .units
        .iter()
        .map(|unit| unit.placement.x)
        .fold(f64::INFINITY, f64::min);
    let max_x = record
        .structure
        .units
        .iter()
        .map(|unit| unit.placement.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = record
        .structure
        .units
        .iter()
        .map(|unit| unit.placement.y)
        .fold(f64::INFINITY, f64::min);
    let max_y = record
        .structure
        .units
        .iter()
        .map(|unit| unit.placement.y)
        .fold(f64::NEG_INFINITY, f64::max);
    let width = (max_x - min_x).max(1e-9);
    let height = (max_y - min_y).max(1e-9);
    let aspect_bucket = ((width.max(height) / width.min(height)) / 0.25).round() as i64;

    let mut composition = std::collections::BTreeMap::<String, usize>::new();
    for unit in &record.structure.units {
        if let Some((name, _)) = unit.material.parts.first() {
            *composition.entry(name.clone()).or_default() += 1;
        }
    }
    let composition = composition
        .into_iter()
        .map(|(name, count)| format!("{name}:{count}"))
        .collect::<Vec<_>>()
        .join(",");

    vec![
        format!("area:{area_bucket}"),
        format!("pieces:{}", record.constituent_count),
        format!("aspect:{aspect_bucket}"),
        format!("composition:{composition}"),
    ]
}

fn select_diverse_parent(
    records: &[KnownQualifyingCavity],
    next_mutation_by_parent: &std::collections::HashMap<String, usize>,
    last_parent_id: Option<&str>,
) -> Option<usize> {
    let eligible = records
        .iter()
        .enumerate()
        .filter(|(_, record)| next_mutation_by_parent.get(&record.id).copied() != Some(usize::MAX))
        .collect::<Vec<_>>();
    if eligible.is_empty() {
        return None;
    }

    // Parent-before-child persistence makes lineage depth cheap to derive.
    // Exhaust shallower generations before following their descendants, so a
    // newly found child cannot monopolize the search while siblings wait.
    let index_by_id = records
        .iter()
        .enumerate()
        .map(|(index, record)| (record.id.as_str(), index))
        .collect::<std::collections::HashMap<_, _>>();
    let mut depths = vec![0usize; records.len()];
    for (index, record) in records.iter().enumerate() {
        if let Some(parent_id) = record.parent_id.as_deref() {
            if let Some(&parent_index) = index_by_id.get(parent_id) {
                if parent_index < index {
                    depths[index] = depths[parent_index].saturating_add(1);
                }
            }
        }
    }

    // Count descriptors across the whole archive, including exhausted parents.
    // Otherwise a well-explored family would falsely appear rare at the frontier.
    let mut frequencies = std::collections::HashMap::<String, usize>::new();
    let mut features_by_index = std::collections::HashMap::<usize, Vec<String>>::new();
    for (index, record) in records.iter().enumerate() {
        let features = diversity_features(record);
        for feature in &features {
            *frequencies.entry(feature.clone()).or_default() += 1;
        }
        if next_mutation_by_parent.get(&record.id).copied() != Some(usize::MAX) {
            features_by_index.insert(index, features);
        }
    }

    let last_index =
        last_parent_id.and_then(|id| records.iter().position(|record| record.id == id));
    eligible
        .into_iter()
        .max_by(|(index_a, record_a), (index_b, record_b)| {
            let depth_order = depths[*index_b].cmp(&depths[*index_a]);
            if depth_order != std::cmp::Ordering::Equal {
                return depth_order;
            }

            let offset_a = next_mutation_by_parent
                .get(&record_a.id)
                .copied()
                .unwrap_or(0);
            let offset_b = next_mutation_by_parent
                .get(&record_b.id)
                .copied()
                .unwrap_or(0);
            let offset_order = offset_b.cmp(&offset_a);
            if offset_order != std::cmp::Ordering::Equal {
                return offset_order;
            }

            let score = |index: usize| {
                features_by_index[&index]
                    .iter()
                    .map(|feature| 1.0 / (*frequencies.get(feature).unwrap_or(&1) as f64))
                    .sum::<f64>()
            };
            let score_order = score(*index_a).total_cmp(&score(*index_b));
            if score_order != std::cmp::Ordering::Equal {
                return score_order;
            }

            let distance = |index: usize| {
                last_index.map_or(index, |last| {
                    (index + records.len() - last - 1) % records.len()
                })
            };
            distance(*index_b).cmp(&distance(*index_a))
        })
        .map(|(index, _)| index)
}

/// Explore one bounded batch from the persistent cavity frontier. Validation
/// and duplicate signatures are checked before a candidate is appended. A small
/// cursor rotates the starting parent between runs so early records do not starve
/// later candidates once the library grows.
const MAX_CAVITY_MUTATION_RUN: Duration = Duration::from_secs(240);

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

#[derive(Default)]
struct MutationSearchCache {
    known_signatures: std::collections::HashSet<String>,
    known_signature_record_count: usize,
    parent_mutations: Option<(String, Vec<(String, OrganismStructure)>)>,
}

fn mutate_once_until(
    path: impl AsRef<Path>,
    catalog: &[BaseResource],
    deadline: Instant,
    validate_existing: bool,
) -> Result<MutationBatchStats, String> {
    let mut cache = MutationSearchCache::default();
    mutate_once_until_cached(path, catalog, deadline, validate_existing, &mut cache)
}

fn mutate_once_until_cached(
    path: impl AsRef<Path>,
    catalog: &[BaseResource],
    deadline: Instant,
    validate_existing: bool,
    cache: &mut MutationSearchCache,
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
    let frontier_path = path.with_extension("frontier");
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
        let _ = fs::remove_file(&frontier_path);
        records = vec![seed];
        cache.known_signatures.clear();
        cache.known_signature_record_count = 0;
        cache.parent_mutations = None;
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
    let mut next_mutation_by_parent: std::collections::HashMap<String, usize> =
        fs::read_to_string(&frontier_path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
    let mut last_parent_id = cursor_text.strip_prefix("diversity:").map(str::to_string);
    if next_mutation_by_parent.is_empty() {
        if let Some(rest) = cursor_text.strip_prefix("lineage:") {
            let (parent, mutation) = rest.split_once(':').unwrap_or((rest, "0"));
            if let Ok(index) = parent.parse::<usize>() {
                if let Some(record) = records.get(index) {
                    next_mutation_by_parent
                        .insert(record.id.clone(), mutation.parse::<usize>().unwrap_or(0));
                    last_parent_id = Some(record.id.clone());
                }
            }
        } else if !cursor_text.is_empty() && !cursor_text.starts_with("diversity:") {
            if let Some(record) = records.get(deepest_lineage_leaf(&records)) {
                next_mutation_by_parent.insert(record.id.clone(), 0);
                last_parent_id = Some(record.id.clone());
            }
        }
    }

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
    }

    if cache.known_signature_record_count != records.len() {
        cache.parent_mutations = None;
        cache.known_signatures.clear();
        for record in &records {
            if Instant::now() >= deadline {
                return Ok(MutationBatchStats {
                    stopped_at_deadline: true,
                    ..MutationBatchStats::default()
                });
            }
            cache
                .known_signatures
                .insert(geometry_signature(&record.structure, catalog)?);
        }
        cache.known_signature_record_count = records.len();
    }

    let mut stats = MutationBatchStats::default();
    let mut appended = Vec::<u8>::new();
    let stored_count = records.len();
    let stored_bytes = fs::metadata(path)
        .map_err(|e| format!("inspect cavity store size: {e}"))?
        .len();
    let selected_parent = select_diverse_parent(
        &records,
        &next_mutation_by_parent,
        last_parent_id.as_deref(),
    );
    let mut selected_parent_id = last_parent_id.clone();
    let mut selected_parent_exhausted = false;
    'parents: {
        let Some(parent_index) = selected_parent else {
            stats.stopped_at_search_exhaustion = true;
            break 'parents;
        };
        let parent = &records[parent_index];
        selected_parent_id = Some(parent.id.clone());
        if cache
            .parent_mutations
            .as_ref()
            .is_none_or(|(cached_parent_id, _)| cached_parent_id != &parent.id)
        {
            cache.parent_mutations = Some((parent.id.clone(), one_step_mutations(parent, catalog)));
        }
        let mutations = &cache
            .parent_mutations
            .as_ref()
            .expect("selected parent's mutation list is cached")
            .1;
        let mutation_count = mutations.len();
        let start_mutation = next_mutation_by_parent
            .get(&parent.id)
            .copied()
            .unwrap_or(0)
            .min(mutation_count);
        for (mutation_index, (change, candidate_structure)) in mutations.iter().enumerate() {
            if mutation_index < start_mutation {
                continue;
            }
            if stats.attempts >= MAX_MUTATION_ATTEMPTS_PER_RUN {
                stats.stopped_at_attempt_limit = true;
                break 'parents;
            }
            if Instant::now() >= deadline {
                stats.stopped_at_deadline = true;
                break 'parents;
            }
            stats.attempts += 1;
            next_mutation_by_parent.insert(parent.id.clone(), mutation_index + 1);
            let mut structure = candidate_structure.clone();
            let operation = mutation_operation(change).to_string();
            record_mutation_attempt(&mut stats, change, &operation);
            if structure.units.len() > parent.constituent_count {
                record_mutation_outcome(
                    &mut stats,
                    change,
                    &operation,
                    "constituent_count_increased",
                );
                stats.rejected += 1;
                *stats
                    .rejection_reasons
                    .entry("constituent_count_increased".to_string())
                    .or_default() += 1;
                continue;
            }
            let signature = geometry_signature(&structure, catalog)?;
            if cache.known_signatures.contains(&signature) {
                stats.duplicates += 1;
                record_mutation_outcome(&mut stats, change, &operation, "duplicate");
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
                record_mutation_outcome(&mut stats, change, &operation, "geometry_overlap");
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
                            .entry("mutation_has_no_repaired_contact".to_string())
                            .or_default() += 1;
                        record_mutation_outcome(
                            &mut stats,
                            change,
                            &operation,
                            "no_repaired_contact",
                        );
                        continue;
                    }
                }
            }
            if Instant::now() >= deadline {
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
                record_mutation_outcome(&mut stats, change, &operation, "shape_or_bond_invalid");
                continue;
            }
            if Instant::now() >= deadline {
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
                    record_mutation_outcome(
                        &mut stats,
                        change,
                        &operation,
                        "cavity_not_qualifying",
                    );
                    continue;
                }
                Err(error) => {
                    stats.rejected += 1;
                    stats.cavity_rejections += 1;
                    *stats
                        .rejection_reasons
                        .entry(format!("cavity_analysis_error: {error}"))
                        .or_default() += 1;
                    record_mutation_outcome(
                        &mut stats,
                        change,
                        &operation,
                        "cavity_analysis_error",
                    );
                    continue;
                }
            };
            if Instant::now() >= deadline {
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
                record_mutation_outcome(&mut stats, change, &operation, "viability_rejected");
                continue;
            }
            if Instant::now() >= deadline {
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
                record_mutation_outcome(&mut stats, change, &operation, "duplicate_id");
                continue;
            }
            let record = KnownQualifyingCavity {
                schema_version: SCHEMA_VERSION,
                id: id.clone(),
                parent_id: Some(parent.id.clone()),
                change: change.to_string(),
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
                record_mutation_outcome(&mut stats, change, &operation, "storage_limit");
                break 'parents;
            }
            appended.extend_from_slice(&bytes);
            known_ids.insert(id);
            cache.known_signatures.insert(signature);
            stats.accepted += 1;
            record_mutation_outcome(&mut stats, change, &operation, "accepted");

            // The new child enters the frontier next batch; continue balancing
            // exploration across stored parents instead of following one lineage.
            next_mutation_by_parent.insert(parent.id.clone(), mutation_index + 1);
            selected_parent_exhausted = mutation_index + 1 >= mutation_count;
            break 'parents;
        }
        if !stats.stopped_at_deadline
            && !stats.stopped_at_attempt_limit
            && !stats.stopped_at_candidate_limit
            && !stats.stopped_at_storage_limit
        {
            next_mutation_by_parent.insert(parent.id.clone(), usize::MAX);
            selected_parent_exhausted = true;
        }
    }

    if Instant::now() >= deadline {
        stats.stopped_at_deadline = true;
    }
    if selected_parent_exhausted {
        if let Some(id) = selected_parent_id.as_deref() {
            next_mutation_by_parent.insert(id.to_string(), usize::MAX);
        }
    }
    if !appended.is_empty() {
        append_records_atomically(path, &appended)?;
        cache.known_signature_record_count = records.len() + stats.accepted;
    }
    let frontier_json = serde_json::to_string(&next_mutation_by_parent)
        .map_err(|e| format!("serialize cavity exploration frontier: {e}"))?;
    fs::write(&frontier_path, frontier_json)
        .map_err(|e| format!("save cavity exploration frontier: {e}"))?;
    if let Some(id) = selected_parent_id {
        fs::write(cursor_path, format!("diversity:{id}"))
            .map_err(|e| format!("save cavity mutation cursor: {e}"))?;
    }
    if selected_parent.is_none() {
        stats.stopped_at_search_exhaustion = true;
    }
    Ok(stats)
}

/// Continue bounded batches toward the 5,000-candidate cap, sharing one
/// four-minute deadline across the entire run. No individual batch resets it.
pub(crate) fn mutate_until_full(
    path: impl AsRef<Path>,
    catalog: &[BaseResource],
) -> Result<MutationBatchStats, String> {
    let path = path.as_ref();
    let deadline = Instant::now() + MAX_CAVITY_MUTATION_RUN;
    let mut total = MutationBatchStats::default();
    let mut validate_existing = true;
    let mut cache = MutationSearchCache::default();

    loop {
        if Instant::now() >= deadline {
            total.stopped_at_deadline = true;
            break;
        }
        let batch =
            mutate_once_until_cached(path, catalog, deadline, validate_existing, &mut cache)?;
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
        for (operation, count) in batch.mutation_attempts_by_operation {
            *total
                .mutation_attempts_by_operation
                .entry(operation)
                .or_default() += count;
        }
        for (outcome, count) in batch.mutation_outcomes_by_operation {
            *total
                .mutation_outcomes_by_operation
                .entry(outcome)
                .or_default() += count;
        }
        for (resource, count) in batch.mutation_attempts_by_resource {
            *total
                .mutation_attempts_by_resource
                .entry(resource)
                .or_default() += count;
        }
        for (outcome, count) in batch.mutation_outcomes_by_resource {
            *total
                .mutation_outcomes_by_resource
                .entry(outcome)
                .or_default() += count;
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
        if batch.stopped_at_search_exhaustion {
            total.stopped_at_search_exhaustion = true;
            break;
        }
    }
    Ok(total)
}

#[cfg(test)]
mod lineage_tests {
    use super::super::{KnownQualifyingCavity, SCHEMA_VERSION};
    use super::{
        deepest_lineage_leaf, mutate_once_until, one_step_mutations, select_diverse_parent,
    };
    use std::time::{Duration, Instant};

    #[test]
    fn diversity_selection_prefers_an_underrepresented_cavity_area_family() {
        let catalog = crate::resources::default_catalog();
        let structure = crate::initial_organism_constructor::build_nitrogen_ring_seed(&catalog)
            .expect("seed structure should build");
        let record = |id: &str, area: f64| KnownQualifyingCavity {
            schema_version: SCHEMA_VERSION,
            id: id.to_string(),
            parent_id: None,
            change: "test family".to_string(),
            constituent_count: structure.units.len(),
            cavity_area: area,
            minimum_cavity_area: 3.0,
            structure: structure.clone(),
        };
        let records = vec![
            record("area-a", 9.0),
            record("area-b", 9.0),
            record("area-c", 9.5),
        ];
        let frontier = std::collections::HashMap::new();
        assert_eq!(
            select_diverse_parent(&records, &frontier, None),
            Some(2),
            "a less represented measured-area family should be explored first"
        );
    }

    #[test]
    fn diversity_selection_skips_exhausted_parents_and_keeps_fractional_areas_distinct() {
        let catalog = crate::resources::default_catalog();
        let structure = crate::initial_organism_constructor::build_nitrogen_ring_seed(&catalog)
            .expect("seed structure should build");
        let record = |id: &str, area: f64| KnownQualifyingCavity {
            schema_version: SCHEMA_VERSION,
            id: id.to_string(),
            parent_id: None,
            change: "test family".to_string(),
            constituent_count: structure.units.len(),
            cavity_area: area,
            minimum_cavity_area: 3.0,
            structure: structure.clone(),
        };
        let records = vec![
            record("area-a", 9.0),
            record("area-b", 9.0),
            record("area-c", 9.125),
        ];
        let mut frontier = std::collections::HashMap::new();
        frontier.insert("area-a".to_string(), usize::MAX);
        frontier.insert("area-b".to_string(), usize::MAX);
        assert_eq!(
            select_diverse_parent(&records, &frontier, Some("area-b")),
            Some(2),
            "exhausted parents must not block an underexplored fractional-area family"
        );
    }

    #[test]
    fn diversity_frontier_prefers_shallower_generation_before_descendants() {
        let catalog = crate::resources::default_catalog();
        let structure = crate::initial_organism_constructor::build_nitrogen_ring_seed(&catalog)
            .expect("seed structure should build");
        let record = |id: &str, area: f64| KnownQualifyingCavity {
            schema_version: SCHEMA_VERSION,
            id: id.to_string(),
            parent_id: None,
            change: "test generation".to_string(),
            constituent_count: structure.units.len(),
            cavity_area: area,
            minimum_cavity_area: 3.0,
            structure: structure.clone(),
        };
        let seed = record("seed", 9.0);
        let mut child = record("child", 9.5);
        child.parent_id = Some("seed".to_string());
        let mut grandchild = record("grandchild", 10.0);
        grandchild.parent_id = Some("child".to_string());
        let records = vec![seed, child, grandchild];
        let mut frontier = std::collections::HashMap::new();
        frontier.insert("seed".to_string(), usize::MAX);
        frontier.insert("child".to_string(), 1);
        frontier.insert("grandchild".to_string(), 0);

        assert_eq!(
            select_diverse_parent(&records, &frontier, Some("grandchild")),
            Some(1),
            "Bob should explore the shallower sibling generation before chasing descendants"
        );
    }

    #[test]
    fn resource_exchange_searches_placements_and_orientations_of_the_new_piece() {
        let catalog = crate::resources::default_catalog();
        let structure = crate::initial_organism_constructor::build_nitrogen_ring_seed(&catalog)
            .expect("seed structure should build");
        let parent = KnownQualifyingCavity {
            schema_version: SCHEMA_VERSION,
            id: "seed".to_string(),
            parent_id: None,
            change: "initial validated".to_string(),
            constituent_count: structure.units.len(),
            cavity_area: 9.0,
            minimum_cavity_area: 3.0,
            structure,
        };

        let mutations = one_step_mutations(&parent, &catalog);
        let methane_exchange = mutations
            .iter()
            .filter(|(change, _)| change.starts_with("replace constituent at index 0 with Methane"))
            .collect::<Vec<_>>();

        assert!(
            methane_exchange
                .iter()
                .any(|(change, _)| change.ends_with("(existing pose)")),
            "Bob should test the exchanged resource at its original pose first"
        );
        assert!(
            methane_exchange
                .iter()
                .any(|(change, _)| change.contains("then align edge")),
            "Bob should generate contact-guided poses for the exchanged resource"
        );
        assert!(
            methane_exchange
                .iter()
                .any(|(change, _)| change.contains("then rotate")),
            "Bob should test orientations of the exchanged resource"
        );
        assert!(
            methane_exchange
                .iter()
                .any(|(change, _)| change.contains("then translate")),
            "Bob should test placements of the exchanged resource"
        );
        assert!(
            methane_exchange
                .iter()
                .any(|(change, _)| change.contains("and translate")),
            "Bob should test bounded combined pose changes"
        );
        assert!(methane_exchange
            .iter()
            .all(|(_, candidate)| candidate.units.len() == parent.constituent_count));
    }

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
        let _ = std::fs::remove_file(path.with_extension("frontier"));
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
