//! Physical budding/reproduction boundary.
//!
//! Reproduction transfers an actual connected structural subgraph from the
//! parent into a new Offspring. No structural cloning or reproduction-only
//! material pool is used.

use rand_chacha::ChaCha8Rng;

use crate::decision::DecisionHistory;
use crate::resources::Material;
use crate::state::{DevelopmentStage, Environment, Organism, Position, ResourceSense};
use crate::structure::OrganismStructure;

const EPSILON: f64 = 1e-12;
const READINESS_THRESHOLD: f64 = 1.0;

fn structural_mass(organism: &Organism, environment: &Environment) -> f64 {
    organism
        .structure
        .units
        .iter()
        .filter_map(|unit| unit.properties(&environment.catalog).map(|properties| properties.mass))
        .sum()
}

/// Return a parent/offspring structural split without mutating the parent.
///
/// The selected unit indices form a connected component of the graph induced
/// by the selected units. The caller can commit the split only after all
/// validation succeeds, which keeps reproduction failure-atomic.
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
    if selected.len() != selected_units.len() || selected.iter().any(|&i| i >= structure.units.len()) {
        return None;
    }

    // A budding region must be one connected physical subgraph.
    let selected_set: std::collections::HashSet<usize> = selected.iter().copied().collect();
    let mut visited = std::collections::HashSet::new();
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

    // The selected region must actually bud away from the parent: at least one
    // bond crosses the proposed cut. Internal bonds stay with their region.
    let crosses_cut = structure.bonds.iter().any(|bond| {
        selected_set.contains(&bond.unit_a) != selected_set.contains(&bond.unit_b)
    });
    if !crosses_cut {
        return None;
    }

    fn remap_region(
        source: &OrganismStructure,
        selected: &std::collections::HashSet<usize>,
        include_selected: bool,
    ) -> Option<OrganismStructure> {
        let old_indices: Vec<usize> = (0..source.units.len())
            .filter(|index| selected.contains(index) == include_selected)
            .collect();
        if old_indices.is_empty() {
            return None;
        }
        let mut map = std::collections::HashMap::new();
        let mut result = OrganismStructure::new();
        for old_index in &old_indices {
            map.insert(*old_index, result.add_unit(source.units[*old_index].clone()));
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

    let offspring = remap_region(structure, &selected_set, true)?;
    let parent = remap_region(structure, &selected_set, false)?;
    Some((parent, offspring))
}

/// Attempt one physical reproductive investment from a parent that has full
/// reproductive readiness.
///
/// This first constructs a candidate physical split without changing the
/// parent. The public reproduction path will commit only after the split and
/// energy transfer have both been validated.
pub(crate) fn try_form_bud(
    parent: &mut Organism,
    environment: &Environment,
    child_id: String,
    rng: &mut ChaCha8Rng,
) -> Option<Organism> {
    if parent.reproductive_readiness + EPSILON < READINESS_THRESHOLD {
        return None;
    }

    if parent.structure.units.len() < 2 {
        return None;
    }

    // Step 1 deliberately remains a structural primitive. Reproduction must
    // choose a real connected substructure rather than clone the whole parent.
    // The higher-level selection/investment policy is added only after this
    // primitive has independently established conservation and validity.
    let _ = (environment, child_id, rng, structural_mass, split_structure);
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genome::initial_genome;
    use crate::resources::Material;
    use crate::state::{Environment, Position, ResourceSense};
    use crate::structure::{Bond, Placement, StructuralUnit};

    fn environment() -> Environment {
        Environment {
            width: 1000.0,
            height: 1000.0,
            catalog: crate::resources::default_catalog(),
            field: crate::environment::ActiveMaterialField::new(1000.0, 1000.0, 10.0),
            reservoir: crate::environment::DeepReservoir::new(1000.0, 1000.0, 100.0),
            vents: Vec::new(),
        }
    }

    fn structure() -> OrganismStructure {
        let mut s = OrganismStructure::new();
        for x in 0..4 {
            s.add_unit(StructuralUnit::new(
                "Carbon",
                Placement {
                    x: x as f64,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            ));
        }
        s.add_bond(Bond { unit_a: 0, point_a: 0, unit_b: 1, point_b: 0, strength: 0.5, bond_energy: 1.0 });
        s.add_bond(Bond { unit_a: 1, point_a: 1, unit_b: 2, point_b: 0, strength: 0.5, bond_energy: 1.0 });
        s.add_bond(Bond { unit_a: 2, point_a: 1, unit_b: 3, point_b: 0, strength: 0.5, bond_energy: 1.0 });
        s
    }

    #[test]
    fn split_transfers_real_units_without_duplication() {
        let source = structure();
        let (parent, offspring) = split_structure(&source, &[2, 3]).expect("valid connected bud");
        assert_eq!(source.units.len(), parent.units.len() + offspring.units.len());
        assert_eq!(parent.units.len(), 2);
        assert_eq!(offspring.units.len(), 2);
        assert_eq!(parent.bonds.len(), 1);
        assert_eq!(offspring.bonds.len(), 1);
        assert_eq!(parent.connected_components(), vec![vec![0, 1]]);
        assert_eq!(offspring.connected_components(), vec![vec![0, 1]]);
    }

    #[test]
    fn split_preserves_internal_bonds_and_removes_cut_bond() {
        let source = structure();
        let (parent, offspring) = split_structure(&source, &[2, 3]).expect("valid connected bud");
        assert_eq!(parent.bonds.len(), 1);
        assert_eq!(offspring.bonds.len(), 1);
        assert!(parent.bonds.iter().all(|bond| bond.unit_a < 2 && bond.unit_b < 2));
        assert!(offspring.bonds.iter().all(|bond| bond.unit_a < 2 && bond.unit_b < 2));
    }

    #[test]
    fn split_rejects_disconnected_selection_and_leaves_source_unchanged() {
        let source = structure();
        let before = source.clone();
        assert!(split_structure(&source, &[0, 3]).is_none());
        assert_eq!(source.units.len(), before.units.len());
        assert_eq!(source.bonds.len(), before.bonds.len());
    }

    #[test]
    fn reproduction_is_not_yet_committed_through_the_public_path() {
        let environment = environment();
        let mut parent = Organism {
            id: "parent".into(),
            occupied_cells: vec![Position { x: 0.0, y: 0.0 }],
            genome: initial_genome(),
            resource_sense: ResourceSense { sensed_resources: Vec::new(), direction_x: 0.0, direction_y: 0.0, direction_strength: 0.0 },
            memory: Vec::new(),
            decision_history: DecisionHistory::default(),
            usable_energy: 100.0,
            stress: 0.0,
            stored_unbonded: Material { parts: Vec::new(), bonded: false },
            structure: structure(),
            development_stage: DevelopmentStage::Adult,
            age: 100,
            reproductive_readiness: 1.0,
            active_transformation_id: None,
        };
        let original = parent.structure.clone();
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        assert!(try_form_bud(&mut parent, &environment, "child".into(), &mut rng).is_none());
        assert_eq!(parent.structure.units.len(), original.units.len());
        assert_eq!(parent.structure.bonds.len(), original.bonds.len());
        assert_eq!(parent.usable_energy, 100.0);
        assert_eq!(parent.reproductive_readiness, 1.0);
    }
}
