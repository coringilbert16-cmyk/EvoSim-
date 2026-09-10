//! Atomic inherited construction blueprint.
//!
//! This module is the design-side representation of an organism's physical
//! construction plan. Every structural constituent has its own identity and
//! every prescribed relationship names both participating constituents and
//! their connection locations.

use serde::{Deserialize, Serialize};

use crate::structural_blueprint::StructuralBlueprint;
use crate::structure::ConnectionEndpoint;

/// A transform in blueprint space. World-space position and orientation are
/// supplied by the organism anchor at realization time.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct BlueprintTransform {
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub rotation_radians: f64,
}

impl BlueprintTransform {
    pub fn is_valid(&self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.rotation_radians.is_finite()
    }
}

/// One atomic resource constituent in an inherited structural design.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BlueprintAtom {
    /// Resource identity is atomic: one blueprint node represents one unit of
    /// one resource type. Compound Material objects do not belong in this
    /// structural layer.
    pub resource: String,
    /// Position and orientation relative to the blueprint anchor.
    pub transform: BlueprintTransform,
}

impl BlueprintAtom {
    pub fn is_valid(&self) -> bool {
        !self.resource.is_empty() && self.transform.is_valid()
    }
}

/// One exact structural relationship prescribed by the genome.
///
/// The topology is inherited. Construction is responsible only for fulfilling
/// it with physical material; it must never select a different constituent or
/// connection location to make the design work.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct BlueprintBond {
    pub atom_a: usize,
    pub endpoint_a: ConnectionEndpoint,
    pub atom_b: usize,
    pub endpoint_b: ConnectionEndpoint,
    /// Number of physical bonds required at this prescribed relationship.
    #[serde(default = "default_required_bonds")]
    pub required_bonds: u16,
}

fn default_required_bonds() -> u16 {
    1
}

impl BlueprintBond {
    pub fn is_valid(&self, atom_count: usize) -> bool {
        self.atom_a < atom_count
            && self.atom_b < atom_count
            && self.atom_a != self.atom_b
            && self.required_bonds > 0
    }

    pub fn canonical(self) -> Self {
        if self.atom_a <= self.atom_b {
            self
        } else {
            Self {
                atom_a: self.atom_b,
                endpoint_a: self.endpoint_b,
                atom_b: self.atom_a,
                endpoint_b: self.endpoint_a,
                required_bonds: self.required_bonds,
            }
        }
    }
}

/// Atomic construction plan owned by the genome.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AtomicBlueprint {
    pub anchor: BlueprintTransform,
    pub atoms: Vec<BlueprintAtom>,
    pub core_atoms: Vec<usize>,
    pub bonds: Vec<BlueprintBond>,
}

impl AtomicBlueprint {
    pub fn validate(&self) -> Result<(), String> {
        if !self.anchor.is_valid() {
            return Err("blueprint anchor is not finite".into());
        }
        if self.atoms.is_empty() {
            return Err("atomic blueprint must contain at least one atom".into());
        }
        for (i, atom) in self.atoms.iter().enumerate() {
            if !atom.is_valid() {
                return Err(format!("atomic blueprint atom {i} is invalid"));
            }
        }
        if self.core_atoms.is_empty() {
            return Err("atomic blueprint must define a core".into());
        }
        let mut core_seen = vec![false; self.atoms.len()];
        for &atom in &self.core_atoms {
            if atom >= self.atoms.len() {
                return Err("atomic blueprint core references a missing atom".into());
            }
            if core_seen[atom] {
                return Err("atomic blueprint core contains a duplicate atom".into());
            }
            core_seen[atom] = true;
        }
        for (i, bond) in self.bonds.iter().enumerate() {
            if !bond.is_valid(self.atoms.len()) {
                return Err(format!("atomic blueprint bond {i} is invalid"));
            }
            let canonical = bond.canonical();
            if self.bonds[..i]
                .iter()
                .map(|previous| previous.canonical())
                .any(|previous| previous == canonical)
            {
                return Err("atomic blueprint contains duplicate bonds".into());
            }
        }
        Ok(())
    }

    /// Refuse to infer missing endpoint topology from the legacy element graph.
    /// The old format does not contain enough information to recover which
    /// constituent and connection point an external connection intended.
    pub fn from_legacy(_legacy: &StructuralBlueprint) -> Result<Self, String> {
        Err(
            "legacy structural blueprint cannot be flattened deterministically: "
                .to_string()
                + "external connections do not identify constituent endpoints",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn carbon(x: f64) -> BlueprintAtom {
        BlueprintAtom {
            resource: "Carbon".into(),
            transform: BlueprintTransform {
                x,
                y: 0.0,
                rotation_radians: 0.0,
            },
        }
    }

    #[test]
    fn atomic_blueprint_requires_explicit_constituent_endpoints() {
        let blueprint = AtomicBlueprint {
            anchor: BlueprintTransform {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
            atoms: vec![carbon(0.0), carbon(1.0)],
            core_atoms: vec![0],
            bonds: vec![BlueprintBond {
                atom_a: 0,
                endpoint_a: ConnectionEndpoint::Corner { point_index: 0 },
                atom_b: 1,
                endpoint_b: ConnectionEndpoint::Corner { point_index: 3 },
                required_bonds: 1,
            }],
        };
        assert!(blueprint.validate().is_ok());
    }

    #[test]
    fn atomic_blueprint_rejects_duplicate_relationships() {
        let blueprint = AtomicBlueprint {
            anchor: BlueprintTransform {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
            atoms: vec![carbon(0.0), carbon(1.0)],
            core_atoms: vec![0],
            bonds: vec![
                BlueprintBond {
                    atom_a: 0,
                    endpoint_a: ConnectionEndpoint::Corner { point_index: 0 },
                    atom_b: 1,
                    endpoint_b: ConnectionEndpoint::Corner { point_index: 3 },
                    required_bonds: 1,
                },
                BlueprintBond {
                    atom_a: 1,
                    endpoint_a: ConnectionEndpoint::Corner { point_index: 3 },
                    atom_b: 0,
                    endpoint_b: ConnectionEndpoint::Corner { point_index: 0 },
                    required_bonds: 1,
                },
            ],
        };
        assert!(blueprint.validate().is_err());
    }

    #[test]
    fn legacy_flattening_never_guesses_external_topology() {
        let legacy = crate::genome::initial_genome().structural_blueprint;
        let error = AtomicBlueprint::from_legacy(&legacy).unwrap_err();
        assert!(error.contains("does not identify constituent endpoints"));
    }
}
