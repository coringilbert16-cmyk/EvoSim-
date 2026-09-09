//! Physical reproduction lifecycle.

use rand_chacha::ChaCha8Rng;
use std::collections::{HashMap, HashSet};

use crate::material_storage::MaterialStorage;
use crate::resources::{BaseResource, Material};
use crate::state::{DevelopmentStage, Organism, ReproductiveConstruction, ResourceSense};
use crate::structure::OrganismStructure;

const JUVENILE_MATURE_MASS_FRACTION: f64 = 0.40;

fn assemble_blueprint_material(remaining: &mut MaterialStorage, target: &Material) -> Option<Material> {
    let mut inputs = Vec::with_capacity(target.parts.len());
    for (name, amount) in &target.parts {
        if (*amount - 1.0).abs() > f64::EPSILON {
            return None;
        }
        inputs.push(remaining.take_one_unstructured_named(name)?);
    }
    let assembled = if inputs.len() == 1 { inputs.into_iter().next()? } else { crate::resources::combine_materials(&inputs) };
    if assembled == *target { Some(assembled) } else { None }
}

fn frontier(blueprint: &crate::structural_blueprint::StructuralBlueprint, realized: &HashSet<usize>, allowed: &HashSet<usize>) -> Vec<usize> {
    allowed.iter().copied().filter(|index| !realized.contains(index)).filter(|index| {
        blueprint.connections.iter().any(|c| (c.element_a == *index && realized.contains(&c.element_b)) || (c.element_b == *index && realized.contains(&c.element_a)))
    }).collect()
}

fn juvenile_target_set(blueprint: &crate::structural_blueprint::StructuralBlueprint, catalog: &[BaseResource]) -> Option<HashSet<usize>> {
    if !blueprint.is_valid() { return None; }
    let core = blueprint.core_elements.iter().copied().collect::<HashSet<_>>();
    let target_mass = blueprint.structural_mass(catalog) * JUVENILE_MATURE_MASS_FRACTION;
    let mut selected = core.clone();
    let mut current_mass = selected.iter().map(|&i| blueprint.elements[i].material.mass(catalog)).sum::<f64>();
    while current_mass + f64::EPSILON < target_mass {
        let next = frontier(blueprint, &selected, &all_indices(blueprint)).into_iter().min_by_key(|&i| i)?;
        selected.insert(next);
        current_mass += blueprint.elements[next].material.mass(catalog);
    }
    Some(selected)
}

fn all_indices(blueprint: &crate::structural_blueprint::StructuralBlueprint) -> HashSet<usize> { (0..blueprint.elements.len()).collect() }

fn add_blueprint_element(structure: &mut OrganismStructure, realized_units: &HashMap<usize, usize>, blueprint_index: usize, material: Material, blueprint: &crate::structural_blueprint::StructuralBlueprint, catalog: &[BaseResource], energy: &mut f64) -> Option<(usize, f64)> {
    let element = &blueprint.elements[blueprint_index];
    let unit = crate::structure::StructuralUnit::from_material(material, element.placement)?;
    let mut candidate_structure = structure.clone();
    let new_index = candidate_structure.add_unit(unit);
    let mut added_stress = 0.0;
    let mut trial_energy = *energy;
    let mut cache = crate::contact::ConnectionCompatibilityCache::new();
    for connection in &blueprint.connections {
        let other_blueprint_index = if connection.element_a == blueprint_index { connection.element_b } else if connection.element_b == blueprint_index { connection.element_a } else { continue };
        let Some(&other_structure_index) = realized_units.get(&other_blueprint_index) else { continue };
        let (new_point, other_point) = if connection.element_a == blueprint_index { (connection.point_a, connection.point_b) } else { (connection.point_b, connection.point_a) };
        let candidate = crate::contact::connection_pair_candidates_cached(&candidate_structure, new_index, other_structure_index, catalog, &mut cache).into_iter().find(|c| c.point_a == new_point && c.point_b == other_point && c.distance <= 1.0 && c.available_a && c.available_b)?;
        let pa = candidate_structure.units[new_index].properties(catalog)?;
        let pb = candidate_structure.units[other_structure_index].properties(catalog)?;
        let evaluation = crate::combine::evaluate_formation(candidate, pa.cohesion, pb.cohesion);
        if !evaluation.threshold.is_finite() || evaluation.threshold < 0.0 { return None; }
        let attempt = crate::combine_runtime::form_bond(&mut candidate_structure, new_index, new_point, other_structure_index, other_point, catalog, &mut cache, evaluation.threshold, 0.0, &mut trial_energy)?;
        added_stress += attempt.work_cost;
    }
    *structure = candidate_structure;
    *energy = trial_energy;
    Some((new_index, added_stress))
}

fn construct_any_frontier_element(stored_material: &MaterialStorage, structure: &mut OrganismStructure, realized_units: &HashMap<usize, usize>, realized: &HashSet<usize>, allowed: &HashSet<usize>, blueprint: &crate::structural_blueprint::StructuralBlueprint, catalog: &[BaseResource], energy: &mut f64) -> Option<(usize, MaterialStorage, f64)> {
    let mut candidates = frontier(blueprint, realized, allowed);
    candidates.sort_unstable();
    for blueprint_index in candidates {
        let mut remaining = stored_material.clone();
        let Some(material) = assemble_blueprint_material(&mut remaining, &blueprint.elements[blueprint_index].material) else { continue; };
        let mut candidate_structure = structure.clone();
        let mut candidate_energy = *energy;
        if let Some((_, stress)) = add_blueprint_element(&mut candidate_structure, realized_units, blueprint_index, material, blueprint, catalog, &mut candidate_energy) {
            *structure = candidate_structure;
            *energy = candidate_energy;
            return Some((blueprint_index, remaining, stress));
        }
    }
    None
}

pub(crate) fn begin_reproduction(parent: &mut Organism, rng: &mut ChaCha8Rng, catalog: &[BaseResource]) -> bool {
    if !matches!(parent.development_stage, DevelopmentStage::Adult) || parent.reproductive_readiness < 1.0 - f64::EPSILON || parent.reproductive_construction.is_some() { return false; }
    let mut child_genome = parent.genome.clone();
    child_genome.mutate(rng);
    let blueprint = &child_genome.structural_blueprint;
    let Some(target_set) = juvenile_target_set(blueprint, catalog) else { return false; };
    let core = blueprint.core_elements.iter().copied().collect::<HashSet<_>>();
    let mut remaining = parent.stored_material.clone();
    let mut structure = OrganismStructure::new();
    let mut realized = HashSet::new();
    let mut realized_units = HashMap::new();
    let mut realized_order = Vec::new();
    let mut initial_stress = 0.0;
    let mut available_energy = parent.usable_energy;
    while realized.len() < core.len() {
        let built = construct_any_frontier_element(&remaining, &mut structure, &realized_units, &realized, &core, blueprint, catalog, &mut available_energy).or_else(|| {
            for &candidate in &core {
                let mut trial_remaining = remaining.clone();
                let Some(material) = assemble_blueprint_material(&mut trial_remaining, &blueprint.elements[candidate].material) else { continue; };
                let mut trial_structure = structure.clone();
                let mut trial_energy = available_energy;
                if let Some((_, stress)) = add_blueprint_element(&mut trial_structure, &HashMap::new(), candidate, material, blueprint, catalog, &mut trial_energy) {
                    structure = trial_structure;
                    available_energy = trial_energy;
                    return Some((candidate, trial_remaining, stress));
                }
            }
            None
        });
        let Some((index, next_remaining, stress)) = built else { return false; };
        let Some(structure_index) = structure.units.len().checked_sub(1) else { return false; };
        remaining = next_remaining;
        realized_units.insert(index, structure_index);
        realized.insert(index);
        realized_order.push(index);
        initial_stress += stress;
    }
    parent.stored_material = remaining;
    parent.usable_energy = available_energy;
    parent.reproductive_readiness = 0.0;
    parent.add_transaction_stress(initial_stress);
    parent.reproductive_construction = Some(ReproductiveConstruction { committed_material: MaterialStorage::default(), developing_structure: structure, child_genome, target_elements: target_set.into_iter().collect(), realized_elements: realized_order, pending_stress: 0.0 });
    true
}

pub(crate) fn advance_construction(stored_material: &mut MaterialStorage, construction: &mut ReproductiveConstruction, catalog: &[BaseResource], energy: &mut f64) -> Option<f64> {
    let blueprint = &construction.child_genome.structural_blueprint;
    let target = construction.target_elements.iter().copied().collect::<HashSet<_>>();
    let realized = construction.realized_elements.iter().copied().collect::<HashSet<_>>();
    if realized.len() >= target.len() { return Some(0.0); }
    let realized_units = construction.realized_elements.iter().enumerate().map(|(structure_index, &blueprint_index)| (blueprint_index, structure_index)).collect::<HashMap<_, _>>();
    let mut next_structure = construction.developing_structure.clone();
    let mut next_energy = *energy;
    let Some((index, remaining, stress)) = construct_any_frontier_element(stored_material, &mut next_structure, &realized_units, &realized, &target, blueprint, catalog, &mut next_energy) else { return None; };
    construction.developing_structure = next_structure;
    *stored_material = remaining;
    *energy = next_energy;
    construction.realized_elements.push(index);
    Some(stress)
}

pub(crate) fn finish_reproduction(parent: &mut Organism) -> Option<Organism> {
    let construction = parent.reproductive_construction.take()?;
    let mature_mass = construction.developing_structure.structural_mass(&construction.child_genome.structural_blueprint);
    let target_mass = construction.child_genome.structural_blueprint.structural_mass(&construction.child_genome.structural_blueprint) * JUVENILE_MATURE_MASS_FRACTION;
    if mature_mass + f64::EPSILON < target_mass { parent.reproductive_construction = Some(construction); return None; }
    let mut child = Organism::new_from_genome(construction.child_genome, construction.developing_structure, parent.position, ResourceSense::default());
    child.development_stage = DevelopmentStage::Juvenile;
    child.reproductive_readiness = 0.0;
    Some(child)
}
