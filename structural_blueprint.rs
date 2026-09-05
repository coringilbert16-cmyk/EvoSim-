//! Inherited structural blueprint.
//!
//! A blueprint describes what an organism is inherited to build: physical
//! material composition, constituent placement, and intended structural
//! connections. It does not duplicate runtime geometry, store a bonding flag,
//! or assert that the blueprint itself is a realized physical body.
//!
//! Runtime physical geometry is derived later from the realized
//! `OrganismStructure`, its authoritative `Material` values, and the immutable
//! resource catalog. This keeps inherited intent separate from current
//! physical state.

use crate::resources::Material;
use crate::structure::Placement;
use serde::{Deserialize, Serialize};

/// Inherited construction plan for an organism's structural body.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StructuralBlueprint {
    pub elements: Vec<BlueprintElement>,
    pub connections: Vec<BlueprintConnection>,
}

/// One inherited physical-material element and its intended placement.
///
/// `Material` is the sole authority for the element's composition and internal
/// structure. No parallel structural-material wrapper is permitted here.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BlueprintElement {
    pub material: Material,
    pub placement: Placement,
}

/// An intended structural connection between two blueprint elements.
///
/// The point indices refer to the discrete connection sites exposed by the
/// eventual realized structural units. The blueprint records topology here;
/// physical contact is validated when the structure is actually realized.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlueprintConnection {
    pub element_a: usize,
    pub point_a: usize,
    pub element_b: usize,
    pub point_b: usize,
}

impl StructuralBlueprint {
    pub fn new(elements: Vec<BlueprintElement>, connections: Vec<BlueprintConnection>) -> Self {
        Self {
            elements,
            connections,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }

    /// Validate inherited construction intent without pretending that the
    /// blueprint is itself the realized physical body.
    pub fn validate(&self) -> Result<(), String> {
        if self.elements.is_empty() {
            return Err("blueprint must contain at least one element".into());
        }

        for (index, element) in self.elements.iter().enumerate() {
            element
                .validate()
                .map_err(|error| format!("element {index}: {error}"))?;
        }

        for (index, connection) in self.connections.iter().enumerate() {
            connection
                .validate(self)
                .map_err(|error| format!("connection {index}: {error}"))?;
        }

        if self.elements.len() > 1 && !self.is_connected() {
            return Err("multi-element blueprint must be connected".into());
        }

        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        if self.elements.is_empty() {
            return false;
        }

        let mut visited = vec![false; self.elements.len()];
        let mut stack = vec![0usize];
        visited[0] = true;

        while let Some(current) = stack.pop() {
            for connection in &self.connections {
                let next = if connection.element_a == current {
                    connection.element_b
                } else if connection.element_b == current {
                    connection.element_a
                } else {
                    continue;
                };

                if next < visited.len() && !visited[next] {
                    visited[next] = true;
                    stack.push(next);
                }
            }
        }

        visited.into_iter().all(|seen| seen)
    }

    /// Sum the material amounts requested by the blueprint without changing
    /// the structural identity of any individual element.
    pub fn total_material_amount(&self) -> f64 {
        self.elements
            .iter()
            .map(|element| element.material.total_amount())
            .sum()
    }

    pub fn structural_mass(&self, catalog: &[crate::resources::BaseResource]) -> f64 {
        self.elements
            .iter()
            .map(|element| element.material.mass(catalog))
            .sum()
    }
}

impl BlueprintElement {
    pub fn validate(&self) -> Result<(), String> {
        if !self.material.is_valid() {
            return Err("material is invalid".into());
        }

        if !self.placement.x.is_finite()
            || !self.placement.y.is_finite()
            || !self.placement.rotation_radians.is_finite()
        {
            return Err("placement must be finite".into());
        }

        Ok(())
    }
}

impl BlueprintConnection {
    fn validate(&self, blueprint: &StructuralBlueprint) -> Result<(), String> {
        if self.element_a >= blueprint.elements.len() || self.element_b >= blueprint.elements.len() {
            return Err("references a missing element".into());
        }
        if self.element_a == self.element_b {
            return Err("self-connections are not permitted".into());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placement(x: f64, y: f64) -> Placement {
        Placement {
            x,
            y,
            rotation_radians: 0.0,
        }
    }

    fn element(name: &str, x: f64, y: f64) -> BlueprintElement {
        BlueprintElement {
            material: Material::free_base(name, 1.0),
            placement: placement(x, y),
        }
    }

    #[test]
    fn single_element_blueprint_is_valid() {
        let blueprint = StructuralBlueprint::new(vec![element("Carbon", 0.0, 0.0)], Vec::new());
        assert!(blueprint.is_valid());
        assert_eq!(blueprint.total_material_amount(), 1.0);
    }

    #[test]
    fn blueprint_uses_authoritative_material_identity() {
        let material = Material {
            parts: vec![("Carbon".into(), 1.0), ("Methane".into(), 1.0)],
            internal_bonds: vec![crate::resources::InternalBond {
                part_a: 0,
                part_b: 1,
            }],
        };
        let blueprint = StructuralBlueprint::new(
            vec![BlueprintElement {
                material,
                placement: placement(0.0, 0.0),
            }],
            Vec::new(),
        );

        assert!(blueprint.is_valid());
        assert!(blueprint.elements[0].material.has_internal_structure());
        assert_eq!(blueprint.elements[0].material.parts.len(), 2);
    }

    #[test]
    fn disconnected_multi_element_blueprint_is_rejected() {
        let blueprint = StructuralBlueprint::new(
            vec![element("Carbon", 0.0, 0.0), element("Methane", 10.0, 0.0)],
            Vec::new(),
        );
        assert!(!blueprint.is_valid());
    }

    #[test]
    fn connected_multi_element_blueprint_is_valid() {
        let blueprint = StructuralBlueprint::new(
            vec![element("Carbon", 0.0, 0.0), element("Methane", 1.0, 0.0)],
            vec![BlueprintConnection {
                element_a: 0,
                point_a: 0,
                element_b: 1,
                point_b: 0,
            }],
        );
        assert!(blueprint.is_valid());
    }

    #[test]
    fn invalid_connection_endpoint_is_rejected() {
        let blueprint = StructuralBlueprint::new(
            vec![element("Carbon", 0.0, 0.0)],
            vec![BlueprintConnection {
                element_a: 0,
                point_a: 0,
                element_b: 1,
                point_b: 0,
            }],
        );
        assert!(!blueprint.is_valid());
    }
}
