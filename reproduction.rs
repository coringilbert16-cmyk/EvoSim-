//! Physical budding/reproduction boundary.
//!
//! Reproduction transfers an actual connected structural subgraph from the
//! parent into a new Offspring. No structural cloning or reproduction-only
//! material pool is used.

use rand_chacha::ChaCha8Rng;
use std::collections::{HashMap, HashSet};

use crate::state::{Environment, Organism};
use crate::structure::OrganismStructure;

/// Return a parent/offspring structural split without mutating the source.
///
/// The selected units must form a connected subgraph and at least one bond
/// must cross the proposed cut. Every unit belongs to exactly one result, and
/// only bonds internal to each result are retained. This is deliberately a
/// pure structural primitive; reproduction policy is layered on top later.
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

    // The transferred region must be physically connected through existing
    // bonds; we do not manufacture a connection merely to make a bud.
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

    // The cut must actually detach the selected region from the remainder.
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

/// Reproduction is intentionally not wired to mutation of organism state yet.
/// The structural primitive above must be validated before the energy and
/// lifecycle commit path is introduced.
pub(crate) fn try_form_bud(
    _parent: &mut Organism,
    _environment: &Environment,
    _child_id: String,
    _rng: &mut ChaCha8Rng,
) -> Option<Organism> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
