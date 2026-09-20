//! Physical reproduction lifecycle. Offspring construction uses the shared
//! construction runtime so every bond admission passes through COMBINE.
use crate::energy_ledger::EnergyLedgerAuthority;
use crate::juvenile_requirements::{validate_realized_juvenile, JuvenileViabilityRequirements};
use crate::material_storage::MaterialStorage;
use crate::resources::{BaseResource, Material};
use crate::state::{
    DevelopmentStage, EnergyLedger, Organism, ReproductiveConstruction, ResourceSense,
};
use crate::structure::OrganismStructure;
use rand_chacha::ChaCha8Rng;
use std::collections::{HashMap, HashSet};

fn assemble_blueprint_material(
    remaining: &mut MaterialStorage,
    target: &Material,
) -> Option<Material> {
    if !target.is_valid() || target.parts.is_empty() {
        return None;
    }
    let mut trial = remaining.clone();
    for (name, amount) in &target.parts {
        if !amount.is_finite() || *amount <= 0.0 || amount.fract().abs() > f64::EPSILON {
            return None;
        }
        for _ in 0..(*amount as usize) {
            trial.take_one_unstructured_named(name)?;
        }
    }
    *remaining = trial;
    Some(target.clone())
}

fn frontier(
    blueprint: &crate::structural_blueprint::StructuralBlueprint,
    realized: &HashMap<usize, Vec<usize>>,
    allowed: &HashSet<usize>,
) -> Vec<usize> {
    allowed
        .iter()
        .copied()
        .filter(|index| {
            !realized.contains_key(index)
                && blueprint.connections.iter().any(|c| {
                    (c.element_a == *index && realized.contains_key(&c.element_b))
                        || (c.element_b == *index && realized.contains_key(&c.element_a))
                })
        })
        .collect()
}
fn realized_mapping(elements: &[usize], groups: &[Vec<usize>]) -> HashMap<usize, Vec<usize>> {
    elements
        .iter()
        .copied()
        .zip(groups.iter().cloned())
        .collect()
}
fn all_indices(blueprint: &crate::structural_blueprint::StructuralBlueprint) -> HashSet<usize> {
    (0..blueprint.elements.len()).collect()
}

fn add_blueprint_element(
    structure: &mut OrganismStructure,
    realized: &HashMap<usize, Vec<usize>>,
    blueprint_index: usize,
    blueprint: &crate::structural_blueprint::StructuralBlueprint,
    catalog: &[BaseResource],
    ledger: &mut EnergyLedger,
    energy: &mut f64,
) -> Option<(Vec<usize>, f64)> {
    let element = &blueprint.elements[blueprint_index];
    let external = blueprint
        .connections
        .iter()
        .filter_map(|connection| {
            let other = if connection.element_a == blueprint_index {
                connection.element_b
            } else if connection.element_b == blueprint_index {
                connection.element_a
            } else {
                return None;
            };
            realized.get(&other).cloned()
        })
        .collect::<Vec<_>>();
    crate::construction_runtime::realize_material_with_context(
        structure, element, catalog, ledger, energy, &external,
    )
    .ok()
}

fn construct_any_frontier_element(
    stored_material: &MaterialStorage,
    structure: &mut OrganismStructure,
    realized: &HashMap<usize, Vec<usize>>,
    allowed: &HashSet<usize>,
    blueprint: &crate::structural_blueprint::StructuralBlueprint,
    catalog: &[BaseResource],
    ledger: &mut EnergyLedger,
    energy: &mut f64,
) -> Option<(usize, MaterialStorage, f64, Vec<usize>)> {
    let mut candidates = frontier(blueprint, realized, allowed);
    candidates.sort_unstable();
    for blueprint_index in candidates {
        let mut remaining = stored_material.clone();
        if assemble_blueprint_material(
            &mut remaining,
            &blueprint.elements[blueprint_index].material,
        )
        .is_none()
        {
            continue;
        }
        let mut candidate_structure = structure.clone();
        let mut candidate_ledger = *ledger;
        let mut candidate_energy = *energy;
        let Some((indices, stress)) = add_blueprint_element(
            &mut candidate_structure,
            realized,
            blueprint_index,
            blueprint,
            catalog,
            &mut candidate_ledger,
            &mut candidate_energy,
        ) else {
            continue;
        };
        *structure = candidate_structure;
        *ledger = candidate_ledger;
        *energy = candidate_energy;
        return Some((blueprint_index, remaining, stress, indices));
    }
    None
}

pub(crate) fn begin_reproduction(
    parent: &mut Organism,
    rng: &mut ChaCha8Rng,
    catalog: &[BaseResource],
    ledger: &mut EnergyLedger,
) -> bool {
    if !matches!(parent.development_stage, DevelopmentStage::Adult)
        || parent.reproductive_construction.is_some()
    {
        return false;
    }
    let mut child_genome = parent.genome.clone();
    child_genome.mutate(rng);
    let blueprint = match crate::juvenile::confirmed_seed_baseline(catalog) {
        Ok(target) => target,
        Err(_) => return false,
    };
    if !blueprint.is_valid()
        || blueprint.anchor_elements.is_empty()
        || !child_genome.juvenile_energy_reserve.is_finite()
        || child_genome.juvenile_energy_reserve <= 0.0
    {
        return false;
    }
    let target_set = all_indices(&blueprint);
    let anchors = blueprint
        .anchor_elements
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    let mut remaining = parent.stored_material.clone();
    let Some(reserved_material) = remaining.take_matching(&child_genome.juvenile_reserve) else {
        return false;
    };
    let mut committed_material = MaterialStorage::default();
    if !committed_material.store(reserved_material.clone()) {
        return false;
    }
    let mut structure = OrganismStructure::new();
    let mut realized = HashMap::new();
    let mut realized_elements = Vec::new();
    let mut realized_groups = Vec::new();
    let mut initial_stress = 0.0;
    let mut trial_ledger = *ledger;
    let mut trial_energy = parent.usable_energy;
    while realized.len() < anchors.len() {
        let result = construct_any_frontier_element(
            &remaining,
            &mut structure,
            &realized,
            &anchors,
            &blueprint,
            catalog,
            &mut trial_ledger,
            &mut trial_energy,
        )
        .or_else(|| {
            if !realized.is_empty() {
                return None;
            }
            let mut candidates = anchors.iter().copied().collect::<Vec<_>>();
            candidates.sort_unstable();
            for candidate in candidates {
                let mut trial_remaining = remaining.clone();
                if assemble_blueprint_material(
                    &mut trial_remaining,
                    &blueprint.elements[candidate].material,
                )
                .is_none()
                {
                    continue;
                }
                let mut trial_structure = structure.clone();
                let mut candidate_ledger = trial_ledger;
                let mut candidate_energy = trial_energy;
                let empty = HashMap::new();
                let Some((new_indices, stress)) = add_blueprint_element(
                    &mut trial_structure,
                    &empty,
                    candidate,
                    &blueprint,
                    catalog,
                    &mut candidate_ledger,
                    &mut candidate_energy,
                ) else {
                    continue;
                };
                structure = trial_structure;
                trial_ledger = candidate_ledger;
                trial_energy = candidate_energy;
                return Some((candidate, trial_remaining, stress, new_indices));
            }
            None
        });
        let Some((index, next_remaining, stress, indices)) = result else {
            return false;
        };
        remaining = next_remaining;
        realized.insert(index, indices.clone());
        realized_elements.push(index);
        realized_groups.push(indices);
        initial_stress += stress;
    }
    parent.stored_material = remaining;
    parent.usable_energy = trial_energy;
    *ledger = trial_ledger;
    parent.add_transaction_stress(initial_stress);
    parent.reproductive_construction = Some(ReproductiveConstruction {
        committed_material,
        developing_structure: structure,
        child_genome,
        target_elements: target_set.into_iter().collect(),
        realized_elements,
        realized_constituent_groups: realized_groups,
        pending_stress: 0.0,
    });
    true
}

pub(crate) fn advance_construction(
    stored_material: &mut MaterialStorage,
    construction: &mut ReproductiveConstruction,
    catalog: &[BaseResource],
    ledger: &mut EnergyLedger,
    energy: &mut f64,
) -> Option<f64> {
    let blueprint = crate::juvenile::confirmed_seed_baseline(catalog).ok()?;
    let target = construction
        .target_elements
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    let realized = realized_mapping(
        &construction.realized_elements,
        &construction.realized_constituent_groups,
    );
    if realized.len() >= target.len() {
        return None;
    }
    let (index, remaining, stress, indices) = construct_any_frontier_element(
        stored_material,
        &mut construction.developing_structure,
        &realized,
        &target,
        &blueprint,
        catalog,
        ledger,
        energy,
    )?;
    *stored_material = remaining;
    construction.realized_elements.push(index);
    construction.realized_constituent_groups.push(indices);
    Some(stress)
}

pub(crate) fn finish_reproduction(
    parent: &mut Organism,
    child_id: String,
    catalog: &[BaseResource],
    ledger: &mut EnergyLedger,
) -> Option<Organism> {
    let construction = parent.reproductive_construction.take()?;
    let target = construction
        .target_elements
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    let realized = construction
        .realized_elements
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    if realized != target {
        parent.reproductive_construction = Some(construction);
        return None;
    }
    let reserve = construction
        .committed_material
        .iter_materials()
        .find(|m| **m == construction.child_genome.juvenile_reserve)
        .cloned();
    let Some(reserve) = reserve else {
        parent.reproductive_construction = Some(construction);
        return None;
    };
    if validate_realized_juvenile(
        &construction.developing_structure,
        catalog,
        JuvenileViabilityRequirements::default(),
    )
    .is_err()
    {
        parent.reproductive_construction = Some(construction);
        return None;
    }
    let parent_position = match parent.occupied_cells.first().cloned() {
        Some(p) => p,
        None => {
            parent.reproductive_construction = Some(construction);
            return None;
        }
    };
    let parent_radius =
        crate::organism_geometry::OrganismBodyGeometry::from_structure(&parent.structure, catalog)
            .map(|g| g.bounding_radius_about(0.0, 0.0))
            .unwrap_or(1.0);
    let child_radius = crate::organism_geometry::OrganismBodyGeometry::from_structure(
        &construction.developing_structure,
        catalog,
    )
    .map(|g| g.bounding_radius_about(0.0, 0.0))
    .unwrap_or(1.0);
    let child_position = crate::state::Position {
        x: parent_position.x + parent_radius.max(1.0) + child_radius.max(1.0) + 1.0,
        y: parent_position.y,
    };
    let mut child_energy = 0.0;
    let reserve_energy = construction.child_genome.juvenile_energy_reserve;
    if !ledger.transfer(&mut parent.usable_energy, &mut child_energy, reserve_energy) {
        parent.reproductive_construction = Some(construction);
        return None;
    }
    let mut stored_material = MaterialStorage::default();
    if !stored_material.store(reserve) {
        parent.reproductive_construction = Some(construction);
        return None;
    }
    Some(Organism {
        id: child_id,
        developmental_origin: child_position.clone(),
        developmental_orientation_radians: 0.0,
        occupied_cells: vec![child_position],
        genome: construction.child_genome,
        resource_sense: ResourceSense {
            sensed_resources: Vec::new(),
            direction_x: 0.0,
            direction_y: 0.0,
            direction_strength: 0.0,
        },
        memory: Vec::new(),
        decision_history: crate::decision::DecisionHistory::default(),
        usable_energy: child_energy,
        stress: 0.0,
        stress_threshold: crate::state::INITIAL_STRESS_THRESHOLD,
        stored_material,
        development_stage: DevelopmentStage::Juvenile,
        active_transformation_id: None,
        reproductive_construction: None,
        structure: construction.developing_structure,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::initial_genome;
    use crate::resources::default_catalog;
    #[test]
    fn reproduction_starts_from_the_confirmed_seed_baseline() {
        let target = crate::juvenile::confirmed_seed_baseline(&default_catalog()).unwrap();
        let indices = all_indices(&target);
        assert_eq!(indices.len(), target.elements.len());
        assert!(target.anchor_elements.iter().all(|i| indices.contains(i)));
    }
    #[test]
    fn realized_mapping_preserves_constituent_groups() {
        let m = realized_mapping(&[17, 3, 42], &[vec![0, 1], vec![2], vec![3, 4]]);
        assert_eq!(m.get(&17), Some(&vec![0, 1]));
        assert_eq!(m.get(&3), Some(&vec![2]));
        assert_eq!(m.get(&42), Some(&vec![3, 4]));
    }
    #[test]
    fn reserve_requirement_is_genome_defined() {
        let mut s = MaterialStorage::default();
        let g = initial_genome();
        assert!(s.store(g.juvenile_reserve.clone()));
        assert!(s.take_matching(&g.juvenile_reserve).is_some());
        assert!(g.juvenile_energy_reserve > 0.0);
    }
    #[test]
    fn multi_part_material_assembly_is_transactional() {
        let mut s = MaterialStorage::default();
        s.store(Material::free_base("Carbon", 1.0));
        s.store(Material::free_base("Hydrogen", 1.0));
        let target = Material {
            parts: vec![("Carbon".into(), 1.0), ("Nitrogen".into(), 1.0)],
            internal_bonds: Vec::new(),
        };
        assert!(assemble_blueprint_material(&mut s, &target).is_none());
        assert_eq!(s.count_unstructured(), 2);
    }
}
