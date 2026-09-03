//! Physical budding/reproduction boundary.
//!
//! Reproduction transfers an actual connected structural subgraph from the
//! parent into a new Offspring. No structural cloning or reproduction-only
//! material pool is used.

use rand::Rng;
use rand_chacha::ChaCha8Rng;
use std::collections::{HashMap, HashSet};

use crate::state::{DevelopmentStage, Environment, Organism, Position, ResourceSense};
use crate::structure::OrganismStructure;

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

/// Select a connected bud whose structural mass reaches the parent's genetic
/// juvenile threshold while leaving at least one structural unit behind.
fn select_bud_units(
    parent: &Organism,
    environment: &Environment,
    rng: &mut ChaCha8Rng,
) -> Option<Vec<usize>> {
    let unit_count = parent.structure.units.len();
    if unit_count < 2 {
        return None;
    }

    let target_mass = parent.genome.juvenile_mass();
    let mut candidates = Vec::new();

    for start in 0..unit_count {
        let mut selected = Vec::new();
        let mut selected_set = HashSet::new();
        let mut frontier = vec![start];

        while let Some(unit) = frontier.pop() {
            if !selected_set.insert(unit) {
                continue;
            }
            selected.push(unit);

            let mass: f64 = selected
                .iter()
                .filter_map(|&index| {
                    parent
                        .structure
                        .units[index]
                        .properties(&environment.catalog)
                        .map(|properties| properties.mass)
                })
                .sum();
            if mass + f64::EPSILON >= target_mass {
                break;
            }

            for bond in &parent.structure.bonds {
                let neighbor = if bond.unit_a == unit {
                    bond.unit_b
                } else if bond.unit_b == unit {
                    bond.unit_a
                } else {
                    continue;
                };
                if !selected_set.contains(&neighbor) {
                    frontier.push(neighbor);
                }
            }
        }

        if selected.len() < unit_count {
            let mass: f64 = selected
                .iter()
                .filter_map(|&index| {
                    parent
                        .structure
                        .units[index]
                        .properties(&environment.catalog)
                        .map(|properties| properties.mass)
                })
                .sum();
            if mass + f64::EPSILON >= target_mass {
                candidates.push((mass - target_mass, selected));
            }
        }
    }

    if candidates.is_empty() {
        return None;
    }

    candidates.sort_by(|a, b| a.0.total_cmp(&b.0));
    let best_excess = candidates[0].0;
    let tied: Vec<_> = candidates
        .into_iter()
        .take_while(|candidate| (candidate.0 - best_excess).abs() <= 1e-9)
        .collect();
    Some(tied[rng.gen_range(0..tied.len())].1.clone())
}

/// Form an Offspring by transferring real parent structure and a fraction of
/// the parent's usable energy. The parent is mutated only after the complete
/// split has been validated, so failed reproduction leaves it unchanged.
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

    let selected_units = select_bud_units(parent, environment, rng)?;
    let (remaining_structure, offspring_structure) =
        split_structure(&parent.structure, &selected_units)?;
    let anchor = structural_anchor_position(&offspring_structure)?;

    let investment = parent.genome.reproductive_investment();
    let transferred_energy = (parent.usable_energy * investment).clamp(0.0, parent.usable_energy);

    let mut child_genome = parent.genome.clone();
    child_genome.mutate(rng);

    parent.structure = remaining_structure;
    parent.usable_energy -= transferred_energy;
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
                10.0,
            ),
            vents: Vec::new(),
        }
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
            },
            memory: Vec::new(),
            decision_history: crate::decision::DecisionHistory::default(),
            usable_energy: 10.0,
            stress: 0.0,
            stored_unbonded: crate::resources::Material {
                parts: Vec::new(),
                bonded: false,
            },
            structure: structure(),
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
    fn split_reindexes_internal_bonds() {
        let source = structure();
        let (parent, offspring) = split_structure(&source, &[2, 3]).expect("valid split");
        assert!(parent
            .bonds
            .iter()
            .all(|bond| bond.unit_a < parent.units.len() && bond.unit_b < parent.units.len()));
        assert!(offspring
            .bonds
            .iter()
            .all(|bond| bond.unit_a < offspring.units.len() && bond.unit_b < offspring.units.len()));
        assert_eq!(parent.connected_components(), vec![vec![0, 1]]);
        assert_eq!(offspring.connected_components(), vec![vec![0, 1]]);
    }

    #[test]
    fn split_preserves_unit_properties_and_geometry() {
        let source = structure();
        let (parent, offspring) = split_structure(&source, &[2, 3]).expect("valid split");
        let catalog = default_catalog();
        let source_mass: f64 = source
            .units
            .iter()
            .map(|unit| unit.properties(&catalog).unwrap().mass)
            .sum();
        let result_mass: f64 = parent
            .units
            .iter()
            .chain(offspring.units.iter())
            .map(|unit| unit.properties(&catalog).unwrap().mass)
            .sum();
        assert_eq!(source_mass, result_mass);
        assert_eq!(offspring.units[0].placement.x, 2.0);
        assert_eq!(offspring.units[1].placement.x, 3.0);
    }

    #[test]
    fn split_rejects_disconnected_selection_without_mutating_source() {
        let source = structure();
        let before = source.clone();
        assert!(split_structure(&source, &[0, 3]).is_none());
        assert_eq!(source.units.len(), before.units.len());
        assert_eq!(source.bonds.len(), before.bonds.len());
    }

    #[test]
    fn split_rejects_duplicate_or_invalid_indices() {
        let source = structure();
        assert!(split_structure(&source, &[1, 1]).is_none());
        assert!(split_structure(&source, &[1, 99]).is_none());
        assert!(split_structure(&source, &[]).is_none());
        assert!(split_structure(&source, &[0, 1, 2, 3]).is_none());
    }

    #[test]
    fn budding_transfers_real_structure_and_energy() {
        let mut parent = adult_parent();
        let before_mass = parent
            .structure
            .units
            .iter()
            .map(|unit| unit.properties(&default_catalog()).unwrap().mass)
            .sum::<f64>();
        let before_energy = parent.usable_energy;
        let env = environment();
        let mut rng = ChaCha8Rng::seed_from_u64(11);

        let child = try_form_bud(&mut parent, &env, "child".into(), &mut rng).expect("bud forms");

        let after_mass = parent
            .structure
            .units
            .iter()
            .chain(child.structure.units.iter())
            .map(|unit| unit.properties(&env.catalog).unwrap().mass)
            .sum::<f64>();
        assert_eq!(before_mass, after_mass);
        assert!((parent.usable_energy + child.usable_energy - before_energy).abs() < 1e-12);
        assert!(matches!(child.development_stage, DevelopmentStage::Offspring));
        assert_eq!(child.occupied_cells.len(), 1);
        assert_eq!(child.occupied_cells[0].x, 2.5);
        assert_eq!(child.occupied_cells[0].y, 0.0);
        assert_eq!(parent.reproductive_readiness, 0.0);
    }

    #[test]
    fn budding_does_not_copy_parent_anchor() {
        let mut parent = adult_parent();
        let env = environment();
        let mut rng = ChaCha8Rng::seed_from_u64(11);
        let child = try_form_bud(&mut parent, &env, "child".into(), &mut rng).expect("bud forms");
        assert_ne!(child.occupied_cells[0].x, 99.0);
        assert_ne!(child.occupied_cells[0].y, 99.0);
    }

    #[test]
    fn failed_budding_leaves_parent_unchanged() {
        let mut parent = adult_parent();
        parent.genome.traits.iter_mut().find(|t| t.name == "juvenile_mass").unwrap().value = 40.0;
        let before = parent.clone();
        let env = environment();
        let mut rng = ChaCha8Rng::seed_from_u64(11);
        assert!(try_form_bud(&mut parent, &env, "child".into(), &mut rng).is_none());
        assert_eq!(parent.structure.units.len(), before.structure.units.len());
        assert_eq!(parent.usable_energy, before.usable_energy);
        assert_eq!(parent.reproductive_readiness, before.reproductive_readiness);
    }
}
