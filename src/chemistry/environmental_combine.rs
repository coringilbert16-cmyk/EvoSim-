//! Bounded, field-owned environmental COMBINE.
//!
//! Environmental material pairs use the same live contact evaluator, chemical
//! activation accumulation, and energy-ledger transaction as organism COMBINE.
//! The field owns both source objects and the resulting realized compound.
use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::contact::{ConnectionCompatibilityCache, ConnectionPairCandidate};
use crate::physical_material::{PhysicalMaterial, PhysicalMaterialBond};
use crate::resources::{BaseResource, InternalBond, Material};
use crate::state::{EnergyLedger, Environment};
use crate::structure::{OrganismStructure, Placement};

const ENVIRONMENT_PREFIX: &str = "environment|combine:";
const FIELD_SCAN_ANCHOR_LIMIT: usize = 64;
const FIELD_TARGET_CELL_INSPECTION_LIMIT: usize = 64;
const FIELD_COMPONENT_PAIR_LIMIT: usize = 128;
const BOUNDS_PADDING: f64 = 2.1;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EnvironmentalCombineOperation {
    pub(crate) cell_a: usize,
    pub(crate) material_a: usize,
    pub(crate) source_a: PhysicalMaterial,
    pub(crate) cell_b: usize,
    pub(crate) material_b: usize,
    pub(crate) source_b: PhysicalMaterial,
    /// Indices relative to the respective restored source objects.
    pub(crate) unit_a: usize,
    pub(crate) unit_b: usize,
    pub(crate) candidate: ConnectionPairCandidate,
    pub(crate) investment: f64,
    pub(crate) accumulation_key: String,
    pub(crate) accumulated_reaction: f64,
}

#[derive(Clone, Copy)]
struct SelectedContact {
    unit_a: usize,
    unit_b: usize,
    candidate: ConnectionPairCandidate,
    investment: f64,
    required_energy: f64,
}

fn origin() -> Placement {
    Placement {
        x: 0.0,
        y: 0.0,
        rotation_radians: 0.0,
    }
}

fn material_signature(material: &PhysicalMaterial) -> String {
    // The realized payload, rather than composition alone, distinguishes moved,
    // reshaped, or differently bonded instances that happen to share a recipe.
    format!("{material:?}")
}

fn source_pair_key(
    cell_a: usize,
    material_a: usize,
    source_a: &PhysicalMaterial,
    cell_b: usize,
    material_b: usize,
    source_b: &PhysicalMaterial,
    unit_a: usize,
    unit_b: usize,
    candidate: ConnectionPairCandidate,
) -> String {
    format!(
        "{ENVIRONMENT_PREFIX}{cell_a}:{material_a}:{}:{unit_a}|{cell_b}:{material_b}:{}:{unit_b}|{:?}|{:?}",
        material_signature(source_a),
        material_signature(source_b),
        candidate.endpoint_a,
        candidate.endpoint_b,
    )
}

fn dissipate_unupdated_accumulations(
    accumulation: &mut BTreeMap<String, f64>,
    updated_keys: &BTreeSet<String>,
) {
    accumulation.retain(|key, value| {
        if !key.starts_with(ENVIRONMENT_PREFIX) || updated_keys.contains(key) {
            return true;
        }
        *value *= 1.0 - crate::chemistry::CHEMICAL_DISSIPATION;
        value.is_finite() && *value > 0.0
    });
}

fn restored_source(
    source: &PhysicalMaterial,
    environment: &Environment,
) -> Option<(OrganismStructure, Vec<usize>)> {
    if !source.is_realized()
        || source.material.is_empty()
        || !source.material.is_valid_for_catalog(&environment.catalog)
    {
        return None;
    }
    let mut structure = OrganismStructure::new();
    let indices = crate::material_restoration::restore_material(
        &mut structure,
        source,
        origin(),
        &environment.catalog,
    )?;
    Some((structure, indices))
}

fn select_best_contact(
    structure: &OrganismStructure,
    indices_a: &[usize],
    indices_b: &[usize],
    catalog: &[BaseResource],
    cache: &mut ConnectionCompatibilityCache,
    remaining_work: &mut usize,
) -> Option<SelectedContact> {
    let mut best: Option<SelectedContact> = None;
    'pairs: for &unit_a in indices_a {
        for &unit_b in indices_b {
            if *remaining_work == 0 {
                break 'pairs;
            }
            *remaining_work = (*remaining_work).saturating_sub(1);
            let candidates = crate::contact::connection_pair_candidates_cached(
                structure, unit_a, unit_b, catalog, cache,
            );
            for candidate in candidates {
                let Some((evaluation, _work, investment, required_energy)) =
                    crate::combine_runtime::selected_candidate_evaluation(
                        structure, unit_a, unit_b, candidate, catalog,
                    )
                else {
                    continue;
                };
                let proposed = SelectedContact {
                    unit_a,
                    unit_b,
                    candidate: evaluation.candidate,
                    investment,
                    required_energy,
                };
                let replace = best.map_or(true, |current| {
                    proposed
                        .required_energy
                        .partial_cmp(&current.required_energy)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| {
                            proposed
                                .candidate
                                .distance
                                .partial_cmp(&current.candidate.distance)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                        .then_with(|| {
                            format!(
                                "{:?}|{:?}",
                                proposed.candidate.endpoint_a, proposed.candidate.endpoint_b
                            )
                            .cmp(&format!(
                                "{:?}|{:?}",
                                current.candidate.endpoint_a, current.candidate.endpoint_b
                            ))
                        })
                        .is_lt()
                });
                if replace {
                    best = Some(proposed);
                }
            }
        }
    }
    best
}

fn chemical_step(
    environment: &Environment,
    chemistry_library: &mut crate::chemistry_library::ChemistryLibrary,
    structure: &OrganismStructure,
    selected: SelectedContact,
    source_a: &PhysicalMaterial,
    source_b: &PhysicalMaterial,
    cell_a: usize,
    material_a: usize,
    cell_b: usize,
    material_b: usize,
    accumulation: &mut BTreeMap<String, f64>,
    updated_keys: &mut BTreeSet<String>,
    operations: &mut Vec<EnvironmentalCombineOperation>,
) {
    let Some(a) = structure
        .units
        .get(selected.unit_a)
        .and_then(|unit| unit.properties(&environment.catalog))
    else {
        return;
    };
    let Some(b) = structure
        .units
        .get(selected.unit_b)
        .and_then(|unit| unit.properties(&environment.catalog))
    else {
        return;
    };
    let (Some(position_a), Some(position_b)) = (a.chemical_position, b.chemical_position) else {
        return;
    };
    let Some(material_name_a) = structure.units[selected.unit_a]
        .material
        .parts
        .first()
        .map(|part| part.0.as_str())
    else {
        return;
    };
    let Some(material_name_b) = structure.units[selected.unit_b]
        .material
        .parts
        .first()
        .map(|part| part.0.as_str())
    else {
        return;
    };
    let Some(interface) = crate::geometry_reference_library::resolve_live_contact_candidate(
        material_name_a,
        &structure.units[selected.unit_a],
        material_name_b,
        &structure.units[selected.unit_b],
        selected.candidate,
        &environment.catalog,
    ) else {
        return;
    };
    let cache_key = crate::chemistry_library::ChemistryKey::from_live_geometry(
        material_name_a,
        material_name_b,
        &interface,
        position_a,
        position_b,
    );
    let static_potential = match chemistry_library.get(&cache_key) {
        Some(record) => record.static_potential,
        None => {
            let Some(calculated) = crate::chemistry::interaction_potential(
                position_a,
                position_b,
                crate::chemistry::CHEMICAL_K,
                crate::chemistry::CHEMICAL_D_MAX,
            ) else {
                return;
            };
            match chemistry_library.get_or_insert_static_potential(cache_key, calculated) {
                Ok(Some(value)) => value,
                Ok(None) | Err(_) => calculated,
            }
        }
    };
    let Some(contact) = crate::chemistry::contact_factor(
        selected.candidate.distance,
        crate::chemistry::CHEMICAL_CONTACT_RADIUS,
    ) else {
        return;
    };
    let Some(engagement) = crate::chemistry::interface_engagement(selected.candidate.facing) else {
        return;
    };
    let key = source_pair_key(
        cell_a,
        material_a,
        source_a,
        cell_b,
        material_b,
        source_b,
        selected.unit_a,
        selected.unit_b,
        selected.candidate,
    );
    let previous = accumulation.get(&key).copied().unwrap_or(0.0);
    let Some(next) = crate::chemistry::accumulate_reaction(
        previous,
        static_potential,
        contact,
        engagement,
        crate::chemistry::CHEMICAL_DISSIPATION,
    ) else {
        return;
    };
    if next <= 0.0 {
        updated_keys.insert(key.clone());
        accumulation.remove(&key);
        return;
    }
    let strength = crate::combine::bond_strength(a, b);
    let Some(barrier) = crate::chemistry::activation_barrier_from_bond_strength(strength) else {
        return;
    };
    let Some(activation_signal) = crate::chemistry::normalized_chemistry_to_activation_signal(next)
    else {
        return;
    };
    updated_keys.insert(key.clone());
    if crate::chemistry::activated(activation_signal, barrier) {
        accumulation.remove(&key);
        operations.push(EnvironmentalCombineOperation {
            cell_a,
            material_a,
            source_a: source_a.clone(),
            cell_b,
            material_b,
            source_b: source_b.clone(),
            unit_a: selected.unit_a,
            unit_b: match selected.unit_b.checked_sub(source_a.material.parts.len()) {
                Some(index) => index,
                None => return,
            },
            candidate: selected.candidate,
            investment: selected.investment,
            accumulation_key: key,
            accumulated_reaction: next,
        });
    } else {
        accumulation.insert(key, next);
    }
}

pub(crate) fn accumulate_environmental(
    environment: &Environment,
    accumulation: &mut BTreeMap<String, f64>,
    chemistry_library: &mut crate::chemistry_library::ChemistryLibrary,
    tick: u64,
    remaining_discovery_work: &mut usize,
) -> Vec<EnvironmentalCombineOperation> {
    if *remaining_discovery_work == 0 {
        dissipate_unupdated_accumulations(accumulation, &BTreeSet::new());
        return Vec::new();
    }
    let field_cell_count = environment.field.cells.len();
    if field_cell_count == 0 {
        dissipate_unupdated_accumulations(accumulation, &BTreeSet::new());
        return Vec::new();
    }

    let mut anchor_work = (*remaining_discovery_work)
        .min(FIELD_SCAN_ANCHOR_LIMIT)
        .min(crate::chemical_reaction::MAX_REACTION_CANDIDATE_INSPECTIONS_PER_TICK);
    let cell_scan_count = field_cell_count.min(FIELD_SCAN_ANCHOR_LIMIT);
    let mut operations = Vec::new();
    let mut updated_keys = BTreeSet::new();
    let mut cache = ConnectionCompatibilityCache::new();
    let cell_start = tick as usize % field_cell_count;

    'anchors: for cell_offset in 0..cell_scan_count {
        if anchor_work == 0 || *remaining_discovery_work == 0 {
            break;
        }
        let cell_a = (cell_start + cell_offset) % field_cell_count;
        let material_count = environment.field.cells[cell_a].physical_materials.len();
        if material_count == 0 {
            continue;
        }
        let material_start = (tick as usize + cell_a) % material_count;
        for material_offset in 0..material_count {
            if anchor_work == 0 || *remaining_discovery_work == 0 {
                break 'anchors;
            }
            anchor_work -= 1;
            let material_a = (material_start + material_offset) % material_count;
            let source_a = environment.field.cells[cell_a].physical_materials[material_a].clone();
            let Some((source_structure, indices_a)) = restored_source(&source_a, environment) else {
                continue;
            };
            let Some(body) = crate::organism_geometry::OrganismBodyGeometry::from_structure(
                &source_structure,
                &environment.catalog,
            ) else {
                continue;
            };
            let target_cells = environment.field.cells_intersecting_bounds(
                body.min_x - BOUNDS_PADDING,
                body.max_x + BOUNDS_PADDING,
                body.min_y - BOUNDS_PADDING,
                body.max_y + BOUNDS_PADDING,
            );
            if target_cells.is_empty() {
                continue;
            }
            let target_start = tick as usize % target_cells.len();
            let target_cell_scan_count = target_cells.len().min(FIELD_TARGET_CELL_INSPECTION_LIMIT);
            for target_offset in 0..target_cell_scan_count {
                if *remaining_discovery_work == 0 {
                    break 'anchors;
                }
                let cell_b = target_cells[(target_start + target_offset) % target_cells.len()];
                let target_count = environment.field.cells[cell_b].physical_materials.len();
                if target_count == 0 {
                    continue;
                }
                let target_material_start = (tick as usize + cell_b + material_a) % target_count;
                for target_offset in 0..target_count {
                    if *remaining_discovery_work == 0 {
                        break 'anchors;
                    }
                    *remaining_discovery_work = (*remaining_discovery_work).saturating_sub(1);
                    let material_b = (target_material_start + target_offset) % target_count;
                    if (cell_b, material_b) <= (cell_a, material_a) {
                        continue;
                    }
                    let source_b =
                        environment.field.cells[cell_b].physical_materials[material_b].clone();
                    if !source_b.is_realized() || source_b.material.is_empty() {
                        continue;
                    }
                    let mut trial_structure = source_structure.clone();
                    let Some(indices_b) = crate::material_restoration::restore_material(
                        &mut trial_structure,
                        &source_b,
                        origin(),
                        &environment.catalog,
                    ) else {
                        continue;
                    };
                    if indices_b.iter().any(|index| {
                        crate::construction_runtime::placed_unit_overlaps(
                            &trial_structure,
                            &trial_structure.units[*index],
                            &indices_b,
                            &environment.catalog,
                        )
                    }) {
                        continue;
                    }
                    let mut component_work =
                        (*remaining_discovery_work).min(FIELD_COMPONENT_PAIR_LIMIT);
                    let component_work_initial = component_work;
                    let selected = select_best_contact(
                        &trial_structure,
                        &indices_a,
                        &indices_b,
                        &environment.catalog,
                        &mut cache,
                        &mut component_work,
                    );
                    let component_work_used = component_work_initial.saturating_sub(component_work);
                    *remaining_discovery_work =
                        (*remaining_discovery_work).saturating_sub(component_work_used);
                    let Some(selected) = selected else {
                        continue;
                    };
                    chemical_step(
                        environment,
                        chemistry_library,
                        &trial_structure,
                        selected,
                        &source_a,
                        &source_b,
                        cell_a,
                        material_a,
                        cell_b,
                        material_b,
                        accumulation,
                        &mut updated_keys,
                        &mut operations,
                    );
                }
            }
        }
    }

    dissipate_unupdated_accumulations(accumulation, &updated_keys);
    operations
}

pub(crate) fn schedule_environmental_operations(
    mut operations: Vec<EnvironmentalCombineOperation>,
    tick: u64,
    remaining_work: &mut usize,
    accumulation: &mut BTreeMap<String, f64>,
) -> Vec<EnvironmentalCombineOperation> {
    if operations.is_empty() || *remaining_work == 0 {
        for operation in operations {
            accumulation
                .entry(operation.accumulation_key)
                .and_modify(|current| *current = (*current).max(operation.accumulated_reaction))
                .or_insert(operation.accumulated_reaction);
        }
        return Vec::new();
    }

    operations.sort_by(|left, right| left.accumulation_key.cmp(&right.accumulation_key));
    let rotate = tick as usize % operations.len();
    operations.rotate_left(rotate);
    let mut used_cells = HashSet::new();
    let mut ready = Vec::new();
    for operation in operations {
        let cells_available =
            !used_cells.contains(&operation.cell_a) && !used_cells.contains(&operation.cell_b);
        if cells_available && *remaining_work > 0 {
            used_cells.insert(operation.cell_a);
            used_cells.insert(operation.cell_b);
            *remaining_work -= 1;
            ready.push(operation);
        } else {
            accumulation
                .entry(operation.accumulation_key)
                .and_modify(|current| *current = (*current).max(operation.accumulated_reaction))
                .or_insert(operation.accumulated_reaction);
        }
    }
    ready.sort_by(|left, right| left.accumulation_key.cmp(&right.accumulation_key));
    ready
}

fn physical_material_from_structure(
    structure: &OrganismStructure,
    catalog: &[BaseResource],
) -> Option<PhysicalMaterial> {
    if structure.units.is_empty() {
        return None;
    }
    let mut parts = Vec::with_capacity(structure.units.len());
    let mut placements = Vec::with_capacity(structure.units.len());
    let mut shapes = Vec::with_capacity(structure.units.len());
    for unit in &structure.units {
        if unit.material.parts.len() != 1 || !unit.material.internal_bonds.is_empty() {
            return None;
        }
        let (name, amount) = unit.material.parts.first()?.clone();
        if !amount.is_finite() || amount <= 0.0 || amount.fract().abs() > 1e-9 {
            return None;
        }
        parts.push((name, amount));
        placements.push(unit.placement);
        shapes.push(unit.shape(catalog)?.clone());
    }

    let mut internal_bonds = Vec::with_capacity(structure.bonds.len());
    let mut internal_connections = Vec::with_capacity(structure.bonds.len());
    for bond in &structure.bonds {
        if !bond.bond_energy.is_finite() || bond.bond_energy < 0.0 {
            return None;
        }
        let part_a = structure.unit_index(bond.endpoint_a.constituent_id)?;
        let part_b = structure.unit_index(bond.endpoint_b.constituent_id)?;
        internal_bonds.push(InternalBond { part_a, part_b });
        internal_connections.push(PhysicalMaterialBond {
            part_a,
            endpoint_a: bond.endpoint_a.location,
            part_b,
            endpoint_b: bond.endpoint_b.location,
            bond_energy: bond.bond_energy,
        });
    }
    let material = Material {
        parts,
        internal_bonds,
    };
    if !material.is_valid_for_catalog(catalog) {
        return None;
    }
    let physical = PhysicalMaterial {
        material,
        placements: Some(placements),
        internal_connections: Some(internal_connections),
        owner_relative_origin: None,
        realized_shapes: Some(shapes),
    };
    physical.is_realized().then_some(physical)
}

pub(crate) fn resolve_environmental(
    operation: EnvironmentalCombineOperation,
    environment: &mut Environment,
    ledger: &mut EnergyLedger,
) -> bool {
    let Some(source_a) = environment
        .field
        .cells
        .get(operation.cell_a)
        .and_then(|cell| cell.physical_materials.get(operation.material_a))
    else {
        return false;
    };
    let Some(source_b) = environment
        .field
        .cells
        .get(operation.cell_b)
        .and_then(|cell| cell.physical_materials.get(operation.material_b))
    else {
        return false;
    };
    if source_a != &operation.source_a || source_b != &operation.source_b {
        return false;
    }

    let Some((mut trial_structure, indices_a)) = restored_source(&operation.source_a, environment)
    else {
        return false;
    };
    let Some(indices_b) = crate::material_restoration::restore_material(
        &mut trial_structure,
        &operation.source_b,
        origin(),
        &environment.catalog,
    ) else {
        return false;
    };
    if indices_b.iter().any(|index| {
        crate::construction_runtime::placed_unit_overlaps(
            &trial_structure,
            &trial_structure.units[*index],
            &indices_b,
            &environment.catalog,
        )
    }) {
        return false;
    }
    let Some(&unit_a) = indices_a.get(operation.unit_a) else {
        return false;
    };
    let Some(&unit_b) = indices_b.get(operation.unit_b) else {
        return false;
    };

    let mut staged_ledger = *ledger;
    let mut staged_usable_energy = environment.chemistry_usable_energy;
    let mut cache = ConnectionCompatibilityCache::new();
    if crate::combine_runtime::form_selected_bond_diagnostic(
        &mut trial_structure,
        unit_a,
        unit_b,
        operation.candidate,
        operation.investment,
        &environment.catalog,
        &mut cache,
        &mut staged_ledger,
        &mut staged_usable_energy,
    )
    .is_err()
    {
        return false;
    }
    if !crate::combine_runtime::coalesce_fluid_pair_after_bond(
        &mut trial_structure,
        unit_a,
        unit_b,
        &environment.catalog,
    ) {
        return false;
    }
    let Some(product) = physical_material_from_structure(&trial_structure, &environment.catalog)
    else {
        return false;
    };

    // Keep the ledger's world-wide usable-energy total synchronized with the
    // environment-owned holder. settle_transaction updates the holder and flow
    // counters, but deliberately leaves this aggregate total to the owner.
    let usable_delta = staged_usable_energy - environment.chemistry_usable_energy;
    let next_total_usable = staged_ledger.total_usable_energy_held + usable_delta;
    let structural_delta =
        staged_ledger.total_structural_energy_delta - ledger.total_structural_energy_delta;
    let next_total_structural = staged_ledger.total_structural_energy_held + structural_delta;
    if !usable_delta.is_finite()
        || !next_total_usable.is_finite()
        || next_total_usable < 0.0
        || !structural_delta.is_finite()
        || !next_total_structural.is_finite()
    {
        return false;
    }
    staged_ledger.total_usable_energy_held = next_total_usable;
    staged_ledger.total_structural_energy_held = next_total_structural;

    // All fallible checks are complete. Remove in descending index order for a
    // same-cell pair, insert the product at the first source slot, and invalidate
    // every touched cell exactly once.
    if operation.cell_a == operation.cell_b {
        if operation.material_a >= operation.material_b {
            return false;
        }
        let cell = &mut environment.field.cells[operation.cell_a];
        cell.physical_materials.remove(operation.material_b);
        cell.physical_materials.remove(operation.material_a);
        cell.physical_materials
            .insert(operation.material_a, product);
        environment.field.mark_changed_at_index(operation.cell_a);
    } else {
        environment.field.cells[operation.cell_a]
            .physical_materials
            .remove(operation.material_a);
        environment.field.cells[operation.cell_b]
            .physical_materials
            .remove(operation.material_b);
        environment.field.cells[operation.cell_a]
            .physical_materials
            .insert(operation.material_a, product);
        environment.field.mark_changed_at_index(operation.cell_a);
        environment.field.mark_changed_at_index(operation.cell_b);
    }
    environment.chemistry_usable_energy = staged_usable_energy;
    *ledger = staged_ledger;
    true
}

pub(crate) fn resolve_environmental_preserving_accumulation(
    operation: EnvironmentalCombineOperation,
    environment: &mut Environment,
    ledger: &mut EnergyLedger,
    accumulation: &mut BTreeMap<String, f64>,
) -> bool {
    let key = operation.accumulation_key.clone();
    let accumulated_reaction = operation.accumulated_reaction;
    let success = resolve_environmental(operation, environment, ledger);
    if !success {
        accumulation.insert(key, accumulated_reaction);
    }
    success
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environmental_pair_keys_include_instance_realization_and_endpoints() {
        let catalog = crate::resources::default_catalog();
        let a = PhysicalMaterial::realized(
            Material::free_base("Carbon", 1.0),
            vec![Placement {
                x: 1.0,
                y: 2.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .unwrap();
        let b = PhysicalMaterial::realized(
            Material::free_base("Hydrogen", 1.0),
            vec![Placement {
                x: 2.0,
                y: 2.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .unwrap();
        let candidate = ConnectionPairCandidate {
            endpoint_a: crate::structure::ConnectionEndpoint::Corner { point_index: 0 },
            endpoint_b: crate::structure::ConnectionEndpoint::Corner { point_index: 0 },
            distance: 0.0,
            facing: 1.0,
            load_a: 0.0,
            load_b: 0.0,
            available_a: true,
            available_b: true,
        };
        let original = source_pair_key(0, 0, &a, 0, 1, &b, 0, 0, candidate);
        assert_ne!(
            original,
            source_pair_key(0, 0, &a, 0, 1, &b, 1, 0, candidate)
        );
        let moved = PhysicalMaterial::realized(
            Material::free_base("Carbon", 1.0),
            vec![Placement {
                x: 3.0,
                y: 2.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .unwrap();
        assert_ne!(
            original,
            source_pair_key(0, 0, &moved, 0, 1, &b, 0, 0, candidate)
        );
    }

    #[test]
    fn environmental_source_restoration_preserves_world_placements() {
        let simulation = crate::state::Simulation::new(941, 10.0);
        let source = PhysicalMaterial::realized(
            Material::free_base("Carbon", 1.0),
            vec![Placement {
                x: 37.0,
                y: 42.0,
                rotation_radians: 0.25,
            }],
            &simulation.environment.catalog,
        )
        .unwrap();
        let (structure, _) = restored_source(&source, &simulation.environment).unwrap();
        assert_eq!(
            structure.units[0].placement,
            source.placements.as_ref().unwrap()[0]
        );
    }

    #[test]
    fn environmental_pair_combine_commits_a_realized_product() {
        let mut simulation = crate::state::Simulation::new(942, 10.0);
        let catalog = simulation.environment.catalog.clone();
        for cell in &mut simulation.environment.field.cells {
            cell.physical_materials.clear();
        }
        let cell = simulation
            .environment
            .field
            .index_for_position(10.0, 10.0)
            .unwrap();

        let mut selected_pair = None;
        'search: for resource_a in &catalog {
            if resource_a.physical_state == crate::resources::PhysicalState::Fluid {
                continue;
            }
            for resource_b in &catalog {
                if resource_b.physical_state == crate::resources::PhysicalState::Fluid {
                    continue;
                }
                for step in 1..=60 {
                    let distance = step as f64 * 0.05;
                    let source_a = PhysicalMaterial::realized(
                        Material::free_base(resource_a.name.clone(), 1.0),
                        vec![Placement {
                            x: 10.0,
                            y: 10.0,
                            rotation_radians: 0.0,
                        }],
                        &catalog,
                    );
                    let source_b = PhysicalMaterial::realized(
                        Material::free_base(resource_b.name.clone(), 1.0),
                        vec![Placement {
                            x: 10.0 + distance,
                            y: 10.0,
                            rotation_radians: 0.0,
                        }],
                        &catalog,
                    );
                    let (Some(source_a), Some(source_b)) = (source_a, source_b) else {
                        continue;
                    };
                    let Some((mut structure, indices_a)) =
                        restored_source(&source_a, &simulation.environment)
                    else {
                        continue;
                    };
                    let Some(indices_b) = crate::material_restoration::restore_material(
                        &mut structure,
                        &source_b,
                        origin(),
                        &catalog,
                    ) else {
                        continue;
                    };
                    if indices_b.iter().any(|index| {
                        crate::construction_runtime::placed_unit_overlaps(
                            &structure,
                            &structure.units[*index],
                            &indices_b,
                            &catalog,
                        )
                    }) {
                        continue;
                    }
                    let mut cache = ConnectionCompatibilityCache::new();
                    let mut work = FIELD_COMPONENT_PAIR_LIMIT;
                    if let Some(selected) = select_best_contact(
                        &structure, &indices_a, &indices_b, &catalog, &mut cache, &mut work,
                    ) {
                        selected_pair = Some((source_a, source_b, selected));
                        break 'search;
                    }
                }
            }
        }

        let (source_a, source_b, selected) =
            selected_pair.expect("catalog should contain a physically eligible material pair");
        assert!(simulation.environment.field.deposit_physical_at_index(
            cell,
            source_a.clone(),
            &catalog,
        ));
        assert!(simulation.environment.field.deposit_physical_at_index(
            cell,
            source_b.clone(),
            &catalog,
        ));
        simulation.environment.chemistry_usable_energy = 1_000.0;
        let baseline = simulation.energy_inventory_snapshot();
        simulation.energy_ledger = crate::state::EnergyLedger::default();
        simulation.energy_ledger.initial_usable_energy_held = baseline.current_usable_energy;
        simulation.energy_ledger.total_usable_energy_held = baseline.current_usable_energy;
        simulation.energy_ledger.initial_resonant_energy_held = baseline.current_resonant_energy;
        simulation.energy_ledger.total_resonant_energy_held = baseline.current_resonant_energy;
        simulation.energy_ledger.initial_structural_energy_held =
            baseline.current_structural_energy;
        simulation.energy_ledger.total_structural_energy_held = baseline.current_structural_energy;

        let key = source_pair_key(
            cell,
            0,
            &source_a,
            cell,
            1,
            &source_b,
            selected.unit_a,
            selected.unit_b,
            selected.candidate,
        );
        let operation = EnvironmentalCombineOperation {
            cell_a: cell,
            material_a: 0,
            source_a,
            cell_b: cell,
            material_b: 1,
            source_b,
            unit_a: selected.unit_a,
            unit_b: selected.unit_b - 1,
            candidate: selected.candidate,
            investment: selected.investment,
            accumulation_key: key,
            accumulated_reaction: 1.0,
        };
        assert!(resolve_environmental(
            operation,
            &mut simulation.environment,
            &mut simulation.energy_ledger,
        ));

        let product = &simulation.environment.field.cells[cell].physical_materials;
        assert_eq!(product.len(), 1);
        let connections = product[0]
            .internal_connections
            .as_ref()
            .expect("combined product must retain its bond graph");
        assert_eq!(connections.len(), 1);
        assert!(connections[0].bond_energy.is_finite());
        assert!(connections[0].bond_energy > 0.0);
        let inventory = simulation.energy_inventory_snapshot();
        assert!(
            (simulation.energy_ledger.total_usable_energy_held - inventory.current_usable_energy)
                .abs()
                <= 1e-6,
            "COMBINE must keep the ledger usable-energy total aligned with its live holders"
        );
        assert!(
            (simulation.energy_ledger.total_structural_energy_held
                - inventory.current_structural_energy)
                .abs()
                <= 1e-6,
            "COMBINE must keep the ledger structural-energy total aligned with its live bonds"
        );
        let residual = inventory.residual();
        assert!(
            inventory.is_balanced_without_creation(1e-6),
            concat!(
                "environmental COMBINE must conserve the complete energy inventory; ",
                "before={baseline:?}, after={inventory:?}, residual={residual:?}; ",
                "deltas: usable={:.12}, resonant={:.12}, structural={:.12}, ",
                "external_input={:.12}, escaped_tone={:.12}, work={:.12}, heat={:.12}; ",
                "ledger totals: usable={:.12}, structural={:.12}, work={:.12}, heat={:.12}"
            ),
            inventory.current_usable_energy - baseline.current_usable_energy,
            inventory.current_resonant_energy - baseline.current_resonant_energy,
            inventory.current_structural_energy - baseline.current_structural_energy,
            inventory.external_energy_input - baseline.external_energy_input,
            inventory.world_tone_energy_escaped - baseline.world_tone_energy_escaped,
            inventory.work_expended - baseline.work_expended,
            inventory.heat_dissipated - baseline.heat_dissipated,
            simulation.energy_ledger.total_usable_energy_held,
            simulation.energy_ledger.total_structural_energy_held,
            simulation.energy_ledger.total_work_expended,
            simulation.energy_ledger.total_heat_dissipated
        );

        // Carry the exact COMBINE product into the environmental BREAK path.
        // This proves the allocated bond energy survives the field-owned round
        // trip and is settled once when the product is split again.
        let combined = product[0].clone();
        let bond = combined
            .internal_connections
            .as_ref()
            .and_then(|connections| connections.first())
            .expect("combined product must expose its allocated bond")
            .clone();
        let break_operation = crate::chemical_reaction::EnvironmentalChemicalBreakOperation {
            cell_index: cell,
            material_index: 0,
            bond_index: 0,
            source_material: combined,
            bond,
            activation_signal: 1.0,
            disruption_cost: 0.5,
            reaction_frequency_hz: 440.0,
            accumulation_key: "environmental-combine-round-trip-break".into(),
            accumulated_reaction: 1.0,
        };
        assert!(crate::chemical_reaction::resolve_environmental(
            break_operation,
            &mut simulation.environment,
            &mut simulation.energy_ledger,
        ));
        let split = &simulation.environment.field.cells[cell].physical_materials;
        assert_eq!(split.len(), 2, "environmental BREAK must restore both source objects");
        assert!(
            split.iter().all(|piece| piece
                .internal_connections
                .as_ref()
                .is_some_and(Vec::is_empty)),
            "split sources must no longer retain the realized bond"
        );
        let after_break = simulation.energy_inventory_snapshot();
        assert!(
            (simulation.energy_ledger.total_usable_energy_held
                - after_break.current_usable_energy)
                .abs()
                <= 1e-6,
            "BREAK must keep the ledger usable-energy total aligned with its live holders"
        );
        assert!(
            (simulation.energy_ledger.total_structural_energy_held
                - after_break.current_structural_energy)
                .abs()
                <= 1e-6,
            "BREAK must keep the ledger structural-energy total aligned with its live bonds"
        );
        assert!(
            after_break.is_balanced_without_creation(1e-6),
            "COMBINE followed by environmental BREAK must conserve the complete inventory; inventory={after_break:?}, residual={:?}",
            after_break.residual()
        );
    }

    #[test]
    fn environmental_product_preserves_allocated_internal_bond_energy() {
        let catalog = crate::resources::default_catalog();
        let mut structure = OrganismStructure::new();
        let carbon = crate::structure::StructuralUnit::from_material(
            Material::free_base("Carbon", 1.0),
            Placement {
                x: 1.0,
                y: 2.0,
                rotation_radians: 0.0,
            },
        )
        .unwrap();
        let hydrogen = crate::structure::StructuralUnit::from_material(
            Material::free_base("Hydrogen", 1.0),
            Placement {
                x: 2.0,
                y: 2.0,
                rotation_radians: 0.0,
            },
        )
        .unwrap();
        let a = structure.add_unit(carbon);
        let b = structure.add_unit(hydrogen);
        let id_a = structure.physical_id(a).unwrap();
        let id_b = structure.physical_id(b).unwrap();
        let candidate = crate::contact::connection_pair_candidates(&structure, a, b, &catalog)
            .into_iter()
            .find(|candidate| candidate.available_a && candidate.available_b)
            .expect("carbon-hydrogen pair should expose a compatible contact");
        let bond = crate::structure::Bond {
            endpoint_a: crate::structure::BondEndpoint::new(id_a, candidate.endpoint_a),
            endpoint_b: crate::structure::BondEndpoint::new(id_b, candidate.endpoint_b),
            strength: 0.5,
            bond_energy: 2.5,
        };
        assert!(
            crate::contact::try_add_bond(&mut structure, bond, &catalog).is_ok(),
            "valid realized contact should accept the test bond"
        );
        let Some(physical) = physical_material_from_structure(&structure, &catalog) else {
            panic!("newly formed physical compound must retain its bond");
        };
        assert_eq!(
            physical
                .internal_connections
                .as_ref()
                .unwrap()
                .iter()
                .map(|bond| bond.bond_energy)
                .sum::<f64>(),
            2.5
        );
        let mut restored = OrganismStructure::new();
        let restored_indices = crate::material_restoration::restore_material(
            &mut restored,
            &physical,
            origin(),
            &catalog,
        )
        .expect("stored product must restore its physical bond graph");
        assert_eq!(restored_indices.len(), 2);
        assert_eq!(restored.bonds.len(), 1);
        assert_eq!(restored.bonds[0].bond_energy, 2.5);
        assert_eq!(
            crate::energy_ledger::structural_potential(&restored),
            Some(-2.5),
            "restoration must preserve the same signed structural potential"
        );
    }
}
