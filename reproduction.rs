//! Physical budding/reproduction boundary.
//!
//! Reproduction transfers an actual connected structural core from the parent
//! into a new Offspring. The core is the parent's required reproductive
//! investment; juvenile mass is a later developmental threshold reached by
//! the offspring through continued growth.

use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::collections::{HashMap, HashSet};

use crate::core_integrity::CoreIntegrity;
use crate::state::{DevelopmentStage, Environment, Organism, Position, ResourceSense};
use crate::structure::OrganismStructure;

const CORE_UNIT_COUNT: usize = 6;

/// Derive an organism's spatial anchor from its actual structural geometry.
fn structural_anchor_position(structure: &OrganismStructure) -> Option<(f64, f64)> {
    if structure.units.is_empty() {
        return None;
    }

    let count = structure.units.len() as f64;
    let x = structure
        .units
        .iter()
        .map(|unit| unit.placement.x)
        .sum::<f64>()
        / count;
    let y = structure
        .units
        .iter()
        .map(|unit| unit.placement.y)
        .sum::<f64>()
        / count;

    (x.is_finite() && y.is_finite()).then_some((x, y))
}

/// Return a parent/offspring structural split without mutating the source.
fn split_structure(
    structure: &OrganismStructure,
    selected_units: &[usize],
) -> Option<(OrganismStructure, OrganismStructure)> {
    if selected_units.is_empty() || selected_units.len() >= structure.units.len() {
        return None;
    }

    let mut selected = selected_units.to_vec();
    selected.sort_unstable();
    selected.dedup();
    if selected.len() != selected_units.len()
        || selected.iter().any(|&index| index >= structure.units.len())
    {
        return None;
    }

    let selected_set: HashSet<usize> = selected.iter().copied().collect();
    let mut visited = HashSet::new();
    let mut stack = vec![selected[0]];
    while let Some(unit) = stack.pop() {
        if !visited.insert(unit) {
            continue;
        }
        for bond in &structure.bonds {
            let neighbor = if bond.unit_a == unit && selected_set.contains(&bond.unit_b) {
                Some(bond.unit_b)
            } else if bond.unit_b == unit && selected_set.contains(&bond.unit_a) {
                Some(bond.unit_a)
            } else {
                None
            };
            if let Some(neighbor) = neighbor {
                stack.push(neighbor);
            }
        }
    }
    if visited.len() != selected.len() {
        return None;
    }

    if !structure.bonds.iter().any(|bond| {
        selected_set.contains(&bond.unit_a) != selected_set.contains(&bond.unit_b)
    }) {
        return None;
    }

    fn remap_region(
        source: &OrganismStructure,
        selected: &HashSet<usize>,
        include_selected: bool,
    ) -> Option<OrganismStructure> {
        let indices: Vec<usize> = (0..source.units.len())
            .filter(|index| selected.contains(index) == include_selected)
            .collect();
        if indices.is_empty() {
            return None;
        }

        let mut map = HashMap::new();
        let mut result = OrganismStructure::new();
        for old_index in &indices {
            let new_index = result.add_unit(source.units[*old_index].clone());
            map.insert(*old_index, new_index);
        }

        for bond in &source.bonds {
            if selected.contains(&bond.unit_a) == include_selected
                && selected.contains(&bond.unit_b) == include_selected
            {
                let mut remapped = *bond;
                remapped.unit_a = *map.get(&bond.unit_a)?;
                remapped.unit_b = *map.get(&bond.unit_b)?;
                result.add_bond(remapped);
            }
        }
        Some(result)
    }

    let parent = remap_region(structure, &selected_set, false)?;
    let offspring = remap_region(structure, &selected_set, true)?;
    Some((parent, offspring))
}

/// Sum the stored energy of bonds that must be severed to separate the bud.
///
/// A crossing bond does not survive in either resulting structure, so its
/// stored bond energy is released rather than silently destroyed.
fn crossing_bond_energy(structure: &OrganismStructure, selected_units: &[usize]) -> f64 {
    let selected: HashSet<usize> = selected_units.iter().copied().collect();
    structure
        .bonds
        .iter()
        .filter(|bond| selected.contains(&bond.unit_a) != selected.contains(&bond.unit_b))
        .map(|bond| bond.bond_energy)
        .sum()
}

fn is_intact_core(structure: &OrganismStructure, units: &[usize]) -> bool {
    if units.len() != CORE_UNIT_COUNT {
        return false;
    }
    let Ok(unit_indices) = <[usize; CORE_UNIT_COUNT]>::try_from(units) else {
        return false;
    };
    CoreIntegrity::new(unit_indices)
        .map(|core| core.is_intact(structure))
        .unwrap_or(false)
}

/// Find connected six-unit subsets that satisfy the actual core-integrity
/// invariant. This deliberately does not use juvenile mass: juvenile mass is
/// the offspring's later developmental threshold, not the birth requirement.
fn core_candidates(structure: &OrganismStructure) -> Vec<Vec<usize>> {
    if structure.units.len() < CORE_UNIT_COUNT {
        return Vec::new();
    }

    fn neighbors(structure: &OrganismStructure, unit: usize) -> Vec<usize> {
        let mut result = Vec::new();
        for bond in &structure.bonds {
            let neighbor = if bond.unit_a == unit {
                Some(bond.unit_b)
            } else if bond.unit_b == unit {
                Some(bond.unit_a)
            } else {
                None
            };
            if let Some(neighbor) = neighbor {
                result.push(neighbor);
            }
        }
        result.sort_unstable();
        result.dedup();
        result
    }

    fn search(
        structure: &OrganismStructure,
        start: usize,
        selected: &mut Vec<usize>,
        selected_set: &mut HashSet<usize>,
        frontier: &mut HashSet<usize>,
        candidates: &mut HashSet<Vec<usize>>,
    ) {
        if selected.len() == CORE_UNIT_COUNT {
            let mut candidate = selected.clone();
            candidate.sort_unstable();
            if is_intact_core(structure, &candidate) {
                candidates.insert(candidate);
            }
            return;
        }

        let choices: Vec<usize> = frontier
            .iter()
            .copied()
            .filter(|&unit| unit >= start && !selected_set.contains(&unit))
            .collect();

        for unit in choices {
            selected.push(unit);
            selected_set.insert(unit);
            let was_frontier = frontier.remove(&unit);
            let mut added = Vec::new();
            for neighbor in neighbors(structure, unit) {
                if neighbor >= start && !selected_set.contains(&neighbor) && frontier.insert(neighbor) {
                    added.push(neighbor);
                }
            }

            search(structure, start, selected, selected_set, frontier, candidates);

            for neighbor in added {
                frontier.remove(&neighbor);
            }
            if was_frontier {
                frontier.insert(unit);
            }
            selected_set.remove(&unit);
            selected.pop();
        }
    }

    let mut candidates = HashSet::new();
    for start in 0..structure.units.len() {
        let mut selected = vec![start];
        let mut selected_set = HashSet::from([start]);
        let mut frontier = HashSet::new();
        for bond in &structure.bonds {
            let neighbor = if bond.unit_a == start {
                Some(bond.unit_b)
            } else if bond.unit_b == start {
                Some(bond.unit_a)
            } else {
                None
            };
            if let Some(neighbor) = neighbor {
                if neighbor >= start {
                    frontier.insert(neighbor);
                }
            }
        }
        search(
            structure,
            start,
            &mut selected,
            &mut selected_set,
            &mut frontier,
            &mut candidates,
        );
    }

    candidates.into_iter().collect()
}

/// Select a six-unit intact core for the offspring while preserving an intact
/// six-unit core in the parent. The offspring may be below juvenile mass at
/// birth and must grow after this transfer.
fn select_bud_units(
    parent: &Organism,
    rng: &mut ChaCha8Rng,
) -> Option<Vec<usize>> {
    let cores = core_candidates(&parent.structure);
    if cores.len() < 2 {
        return None;
    }

    let mut candidates = Vec::new();
    for offspring_core in &cores {
        let offspring_set: HashSet<usize> = offspring_core.iter().copied().collect();
        if cores.iter().any(|parent_core| {
            parent_core
                .iter()
                .all(|unit| !offspring_set.contains(unit))
        }) {
            candidates.push(offspring_core.clone());
        }
    }

    if candidates.is_empty() {
        return None;
    }

    Some(candidates[rng.gen_range(0..candidates.len())].clone())
}

fn remapped_indices_after_split(
    structure: &OrganismStructure,
    selected_units: &[usize],
    preserved_units: &[usize],
) -> Option<Vec<usize>> {
    let selected: HashSet<usize> = selected_units.iter().copied().collect();
    preserved_units
        .iter()
        .map(|&unit| {
            (unit < structure.units.len() && !selected.contains(&unit))
                .then(|| (0..unit).filter(|index| !selected.contains(index)).count())
        })
        .collect()
}

/// Form an Offspring by transferring real parent core structure and a fraction
/// of the parent's usable energy. The parent is mutated only after the
/// complete split has been validated, so failed reproduction leaves it
/// unchanged.
pub(crate) fn try_form_bud(
    parent: &mut Organism,
    environment: &Environment,
    child_id: String,
    rng: &mut ChaCha8Rng,
) -> Option<Organism> {
    if !matches!(parent.development_stage, DevelopmentStage::Adult) {
        return None;
    }
    if parent.reproductive_readiness < 1.0 - f64::EPSILON {
        return None;
    }

    let parent_cores = core_candidates(&parent.structure);
    if parent_cores.len() < 2 {
        return None;
    }

    let selected_units = select_bud_units(parent, rng)?;
    let preserved_core = parent_cores.iter().find(|core| {
        let selected: HashSet<usize> = selected_units.iter().copied().collect();
        core.iter().all(|unit| !selected.contains(unit))
    })?.clone();

    let (remaining_structure, offspring_structure) =
        split_structure(&parent.structure, &selected_units)?;

    if !is_intact_core(&offspring_structure, &(0..CORE_UNIT_COUNT).collect::<Vec<_>>()) {
        return None;
    }
    let remapped_parent_core =
        remapped_indices_after_split(&parent.structure, &selected_units, &preserved_core)?;
    if !is_intact_core(&remaining_structure, &remapped_parent_core) {
        return None;
    }

    let anchor = structural_anchor_position(&offspring_structure)?;
    let released_bond_energy = crossing_bond_energy(&parent.structure, &selected_units);

    let investment = parent.genome.reproductive_investment();
    let available_energy = parent.usable_energy + released_bond_energy;
    let transferred_energy = (available_energy * investment).clamp(0.0, available_energy);

    let mut child_genome = parent.genome.clone();
    child_genome.mutate(rng);

    parent.structure = remaining_structure;
    parent.usable_energy = available_energy - transferred_energy;
    parent.reproductive_readiness = 0.0;

    Some(Organism {
        id: child_id,
        occupied_cells: vec![Position { x: anchor.0, y: anchor.1 }],
        genome: child_genome,
        resource_sense: ResourceSense {
            sensed_resources: Vec::new(),
            direction_x: 0.0,
            direction_y: 0.0,
            direction_strength: 0.0,
        },
        memory: Vec::new(),
        decision_history: crate::decision::DecisionHistory::default(),
        usable_energy: transferred_energy,
        stress: 0.0,
        stored_unbonded: crate::resources::Material {
            parts: Vec::new(),
            bonded: false,
        },
        structure: offspring_structure,
        development_stage: DevelopmentStage::Offspring,
        age: 0,
        reproductive_readiness: 0.0,
        active_transformation_id: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::initial_genome;
    use crate::resources::default_catalog;
    use crate::structure::{Bond, Placement, StructuralUnit};

    fn structure() -> OrganismStructure {
        let mut structure = OrganismStructure::new();
        for x in 0..4 {
            structure.add_unit(StructuralUnit::new(
                "Carbon",
                Placement {
                    x: x as f64,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            ));
        }
        structure.add_bond(Bond {
            unit_a: 0,
            point_a: 0,
            unit_b: 1,
            point_b: 0,
            strength: 0.5,
            bond_energy: 1.0,
        });
        structure.add_bond(Bond {
            unit_a: 1,
            point_a: 1,
            unit_b: 2,
            point_b: 0,
            strength: 0.5,
            bond_energy: 1.0,
        });
        structure.add_bond(Bond {
            unit_a: 2,
            point_a: 1,
            unit_b: 3,
            point_b: 0,
            strength: 0.5,
            bond_energy: 1.0,
        });
        structure
    }

    fn environment() -> Environment {
        Environment {
            width: 100.0,
            height: 100.0,
            catalog: default_catalog(),
            field: crate::environment::ActiveMaterialField::new(100.0, 100.0, 10.0),
            reservoir: crate::environment::DeepReservoir::new_matching_field(
                &crate::environment::ActiveMaterialField::new(100.0, 100.0, 10.0),
                10,
            ),
            vents: Vec::new(),
        }
    }

    fn core_structure() -> OrganismStructure {
        let mut structure = OrganismStructure::new();
        for y in [0.0, 3.0] {
            for x in 0..6 {
                structure.add_unit(StructuralUnit::new(
                    "Carbon",
                    Placement {
                        x: x as f64,
                        y,
                        rotation_radians: 0.0,
                    },
                ));
            }
        }
        for offset in [0usize, 6usize] {
            for i in 0..6 {
                structure.add_bond(Bond {
                    unit_a: offset + i,
                    point_a: 0,
                    unit_b: offset + (i + 1) % 6,
                    point_b: 1,
                    strength: 0.5,
                    bond_energy: 1.0,
                });
            }
        }
        structure.add_bond(Bond {
            unit_a: 0,
            point_a: 2,
            unit_b: 6,
            point_b: 2,
            strength: 0.5,
            bond_energy: 1.0,
        });
        structure
    }

    fn adult_parent() -> Organism {
        Organism {
            id: "parent".into(),
            occupied_cells: vec![Position { x: 99.0, y: 99.0 }],
            genome: initial_genome(),
            resource_sense: ResourceSense {
                sensed_resources: Vec::new(),
                direction_x: 0.0,
                direction_y: 0.0,
                direction_strength: 0.0,
                sensed_resources: Vec::new(),
            },
            memory: Vec::new(),
            decision_history: crate::decision::DecisionHistory::default(),
            usable_energy: 10.0,
            stress: 0.0,
            stored_unbonded: crate::resources::Material {
                parts: Vec::new(),
                bonded: false,
            },
            structure: core_structure(),
            development_stage: DevelopmentStage::Adult,
            age: 10,
            reproductive_readiness: 1.0,
            active_transformation_id: None,
        }
    }

    #[test]
    fn structural_anchor_is_centroid_of_unit_origins() {
        let source = structure();
        assert_eq!(structural_anchor_position(&source), Some((1.5, 0.0)));
    }

    #[test]
    fn split_anchor_comes_from_transferred_geometry() {
        let source = structure();
        let (parent, offspring) = split_structure(&source, &[2, 3]).expect("valid split");
        assert_eq!(structural_anchor_position(&parent), Some((0.5, 0.0)));
        assert_eq!(structural_anchor_position(&offspring), Some((2.5, 0.0)));
    }

    #[test]
    fn empty_structure_has_no_anchor() {
        assert_eq!(
            structural_anchor_position(&OrganismStructure::new()),
            None
        );
    }

    #[test]
    fn split_transfers_each_unit_exactly_once() {
        let source = structure();
        let (parent, offspring) = split_structure(&source, &[2, 3]).expect("valid split");
        assert_eq!(parent.units.len() + offspring.units.len(), source.units.len());
        assert_eq!(parent.units.len(), 2);
        assert_eq!(offspring.units.len(), 2);
        assert_eq!(parent.bonds.len(), 1);
        assert_eq!(offspring.bonds.len(), 1);
    }

    #[test]
    fn budding_transfers_real_core_and_energy_before_juvenile_growth() {
        let environment = environment();
        let mut parent = adult_parent();
        let original_mass = parent.structural_mass(&environment.catalog);
        let original_usable_energy = parent.usable_energy;
        let original_bond_energy: f64 = parent
            .structure
            .bonds
            .iter()
            .map(|bond| bond.bond_energy)
            .sum();
        let mut rng = ChaCha8Rng::seed_from_u64(11);

        let child = try_form_bud(&mut parent, &environment, "child".into(), &mut rng)
            .expect("adult parent should form a bud");

        let combined_mass = parent.structural_mass(&environment.catalog)
            + child.structural_mass(&environment.catalog);
        let combined_energy = parent.usable_energy
            + child.usable_energy
            + parent
                .structure
                .bonds
                .iter()
                .map(|bond| bond.bond_energy)
                .sum::<f64>()
            + child
                .structure
                .bonds
                .iter()
                .map(|bond| bond.bond_energy)
                .sum::<f64>();
        assert!((combined_mass - original_mass).abs() <= f64::EPSILON);
        assert!((combined_energy - (original_usable_energy + original_bond_energy)).abs() <= f64::EPSILON);
        assert!(matches!(child.development_stage, DevelopmentStage::Offspring));
        assert!(child.structural_mass(&environment.catalog) < child.genome.juvenile_mass());
        assert_eq!(parent.reproductive_readiness, 0.0);
    }

    #[test]
    fn budding_does_not_copy_parent_anchor() {
        let environment = environment();
        let mut parent = adult_parent();
        let parent_anchor = parent.occupied_cells[0].clone();
        let mut rng = ChaCha8Rng::seed_from_u64(11);
        let child = try_form_bud(&mut parent, &environment, "child".into(), &mut rng)
            .expect("adult parent should form a bud");
        assert_ne!(child.occupied_cells[0], parent_anchor);
        assert_eq!(
            child.occupied_cells[0],
            Position {
                x: child
                    .structure
                    .units
                    .iter()
                    .map(|unit| unit.placement.x)
                    .sum::<f64>()
                    / child.structure.units.len() as f64,
                y: child
                    .structure
                    .units
                    .iter()
                    .map(|unit| unit.placement.y)
                    .sum::<f64>()
                    / child.structure.units.len() as f64,
            }
        );
    }

    #[test]
    fn failed_budding_leaves_parent_unchanged_without_two_cores() {
        let environment = environment();
        let mut parent = adult_parent();
        parent.structure = structure();
        let original = parent.clone();
        let mut rng = ChaCha8Rng::seed_from_u64(11);
        assert!(try_form_bud(&mut parent, &environment, "child".into(), &mut rng).is_none());
        assert_eq!(parent.structure.units.len(), original.structure.units.len());
        assert_eq!(parent.structure.bonds.len(), original.structure.bonds.len());
        assert_eq!(parent.usable_energy, original.usable_energy);
        assert_eq!(parent.reproductive_readiness, original.reproductive_readiness);
    }
}
