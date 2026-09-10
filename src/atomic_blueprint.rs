//! Atomic inherited construction blueprint.
//!
//! This module is the design-side representation of an organism's physical
//! construction plan.  Unlike the legacy element/material representation,
//! every structural constituent has its own identity and every prescribed
//! structural relationship names both participating constituents and their
//! connection locations.

use serde::{Deserialize, Serialize};

use crate::resources::Material;
use crate::structural_blueprint::StructuralBlueprint;
use crate::structure::ConnectionEndpoint;

/// A transform in blueprint space.  World-space position and orientation are
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
        self.x.is_finite()
            && self.y.is_finite()
            && self.rotation_radians.is_finite()
    }
}

/// One atomic material constituent in an inherited structural design.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BlueprintAtom {
    /// Exactly one constituent is represented by an atomic blueprint node.
    pub material: Material,
    /// Position and orientation relative to the blueprint anchor.
    pub transform: BlueprintTransform,
}

impl BlueprintAtom {
    pub fn is_valid(&self) -> bool {
        self.material.parts.len() == 1
            && self.material.parts[0].1 == 1.0
            && self.material.internal_bonds.is_empty()
            && self.transform.is_valid()
    }

    pub fn resource_name(&self) -> Option<&str> {
        self.material.parts.get(0).map(|(name, _)| name.as_str())
    }
}

/// One exact structural relationship prescribed by the genome.
///
/// The topology is inherited. Construction is responsible only for fulfilling
/// it with physical material; it must never select a different constituent or
/// connection location to make the design work.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlueprintBond {
    pub atom_a: usize,
    pub endpoint_a: ConnectionEndpoint,
    pub atom_b: usize,
    pub endpoint_b: ConnectionEndpoint,
    /// Number of physical bonds required at this prescribed relationship.
    /// A value of one is the normal case; values greater than one are retained
    /// because the physical bond system permits repeated bonds at a contact.
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
        let mut seen = std::collections::HashSet::new();
        for bond in &self.bonds {
            if !bond.is_valid(self.atoms.len()) {
                return Err("atomic blueprint contains an invalid bond".into());
            }
            let bond = bond.canonical();
            if !seen.insert((bond.atom_a, bond.endpoint_a, bond.atom_b, bond.endpoint_b)) {
                return Err("atomic blueprint contains duplicate bonds".into());
            }
        }
        Ok(())
    }

    /// Explicitly refuses to infer missing endpoint topology from the legacy
    /// element graph.  The old format does not contain enough information to
    /// recover which constituent and connection point an external connection
    /// intended, so guessing here would recreate the exact ambiguity this
    /// representation is designed to eliminate.
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
    use crate::resources::Material;
    use crate::structure::ConnectionEndpoint;

    fn carbon() -> BlueprintAtom {
        BlueprintAtom {
            material: Material::free_base("Carbon", 1.0),
            transform: BlueprintTransform {
                x: 0.0,
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
            atoms: vec![carbon(), BlueprintAtom { transform: BlueprintTransform { x: 1.0, y: 0.0, rotation_radians: 0.0 }, ..carbon() }],
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
    fn legacy_flattening_never_guesses_external_topology() {
        let legacy = crate::genome::initial_genome().structural_blueprint;
        let error = AtomicBlueprint::from_legacy(&legacy).unwrap_err();
        assert!(error.contains("does not identify constituent endpoints"));
    }
}
