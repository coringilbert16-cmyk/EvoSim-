use crate::structure::OrganismStructure;
use serde::{Deserialize, Serialize};

/// Identifies the six StructuralUnits that make up an organism's essential
/// core. The indices refer to the organism structure; no duplicate material
/// or geometry is stored here.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoreIntegrity {
    pub unit_indices: [usize; 6],
}

impl CoreIntegrity {
    pub fn new(unit_indices: [usize; 6]) -> Option<Self> {
        let mut sorted = unit_indices;
        sorted.sort_unstable();
        if sorted.windows(2).any(|pair| pair[0] == pair[1]) {
            return None;
        }
        Some(Self { unit_indices })
    }

    /// The core is intact only while all six designated units exist, remain in
    /// one connected structural component, and each core unit retains at
    /// least two bonds to other core units. Extra bonds to the future membrane
    /// are allowed.
    pub fn is_intact(&self, structure: &OrganismStructure) -> bool {
        if self.unit_indices.iter().any(|&index| index >= structure.units.len()) {
            return false;
        }

        let mut sorted = self.unit_indices;
        sorted.sort_unstable();
        if sorted.windows(2).any(|pair| pair[0] == pair[1]) {
            return false;
        }

        let core = &self.unit_indices;
        for &unit in core {
            let core_bond_count = structure
                .bonds
                .iter()
                .filter(|bond| {
                    let other = if bond.unit_a == unit {
                        Some(bond.unit_b)
                    } else if bond.unit_b == unit {
                        Some(bond.unit_a)
                    } else {
                        None
                    };
                    other.map_or(false, |other| core.contains(&other))
                })
                .count();
            if core_bond_count < 2 {
                return false;
            }
        }

        let components = structure.connected_components();
        components.iter().any(|component| {
            core.iter().all(|unit| component.contains(unit))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structure::{Bond, OrganismStructure, Placement, StructuralUnit};

    fn structure_with_six_core_units() -> OrganismStructure {
        let mut structure = OrganismStructure::new();
        for i in 0..6 {
            structure.add_unit(StructuralUnit::new(
                "Carbon",
                Placement {
                    x: i as f64,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            ));
        }
        for i in 0..6 {
            structure.add_bond(Bond {
                unit_a: i,
                point_a: 0,
                unit_b: (i + 1) % 6,
                point_b: 1,
                strength: 0.8,
                bond_energy: 1.0,
            });
        }
        structure
    }

    #[test]
    fn six_unique_units_create_valid_core_integrity() {
        assert!(CoreIntegrity::new([0, 1, 2, 3, 4, 5]).is_some());
        assert!(CoreIntegrity::new([0, 1, 2, 3, 4, 4]).is_none());
    }

    #[test]
    fn intact_closed_core_passes() {
        let structure = structure_with_six_core_units();
        let integrity = CoreIntegrity::new([0, 1, 2, 3, 4, 5]).unwrap();
        assert!(integrity.is_intact(&structure));
    }

    #[test]
    fn breaking_one_core_bond_fails_integrity() {
        let mut structure = structure_with_six_core_units();
        structure.break_bond(0);
        let integrity = CoreIntegrity::new([0, 1, 2, 3, 4, 5]).unwrap();
        assert!(!integrity.is_intact(&structure));
    }

    #[test]
    fn missing_core_unit_fails_integrity() {
        let mut structure = structure_with_six_core_units();
        structure.units.pop();
        let integrity = CoreIntegrity::new([0, 1, 2, 3, 4, 5]).unwrap();
        assert!(!integrity.is_intact(&structure));
    }

    #[test]
    fn external_bond_does_not_break_core_integrity() {
        let mut structure = structure_with_six_core_units();
        let external = structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 10.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        structure.add_bond(Bond {
            unit_a: 0,
            point_a: 2,
            unit_b: external,
            point_b: 0,
            strength: 0.4,
            bond_energy: 0.5,
        });
        let integrity = CoreIntegrity::new([0, 1, 2, 3, 4, 5]).unwrap();
        assert!(integrity.is_intact(&structure));
    }
}
