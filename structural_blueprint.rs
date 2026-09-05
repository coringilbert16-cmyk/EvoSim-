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

use crate::resources::{BaseResource, Material};
use crate::structure::{OrganismStructure, Placement, StructuralUnit};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StructuralBlueprint {
    pub elements: Vec<BlueprintElement>,
    pub connections: Vec<BlueprintConnection>,
}

/// One inherited physical-material element and its intended placement.
///
/// A blueprint element is one structural unit. Its material must therefore
/// contain exactly one free constituent; structural relationships between
/// units are represented by `BlueprintConnection` entries.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BlueprintElement {
    pub material: Material,
    pub placement: Placement,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlueprintConnection {
    pub element_a: usize,
    pub point_a: usize,
    pub element_b: usize,
    pub point_b: usize,
}

impl StructuralBlueprint {
    pub fn new(elements: Vec<BlueprintElement>, connections: Vec<BlueprintConnection>) -> Self {
        Self { elements, connections }
    }

    pub fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.elements.is_empty() {
            return Err("blueprint must contain at least one element".into());
        }
        for (index, element) in self.elements.iter().enumerate() {
            element.validate().map_err(|error| format!("element {index}: {error}"))?;
        }
        for (index, connection) in self.connections.iter().enumerate() {
            connection.validate(self).map_err(|error| format!("connection {index}: {error}"))?;
        }
        if self.elements.len() > 1 && !self.is_connected() {
            return Err("multi-element blueprint must be connected".into());
        }
        Ok(())
    }

    /// Realize inherited intent into the runtime structural representation.
    ///
    /// Blueprint elements become real structural units. Connections are added
    /// only when their catalog-derived connection sites physically coincide
    /// with opposing normals. The blueprint never becomes a second runtime
    /// body representation.
    pub fn realize(&self, catalog: &[BaseResource]) -> Result<OrganismStructure, String> {
        self.validate()?;
        let mut structure = OrganismStructure::new();

        for element in &self.elements {
            let (resource_name, amount) = element
                .material
                .parts
                .first()
                .cloned()
                .ok_or_else(|| "blueprint element has no material constituent".to_string())?;
            if (amount - 1.0).abs() > f64::EPSILON {
                return Err("blueprint structural-unit material must have amount 1.0".into());
            }
            structure.add_unit(StructuralUnit::new(resource_name, element.placement));
        }

        for connection in &self.connections {
            let unit_a = &structure.units[connection.element_a];
            let unit_b = &structure.units[connection.element_b];
            let site_a = structure
                .connection_site(
                    crate::structure::ConnectionSiteRef {
                        unit_index: connection.element_a,
                        point_index: connection.point_a,
                    },
                    catalog,
                )
                .ok_or_else(|| format!("connection {connection:?} references an invalid first site"))?;
            let site_b = structure
                .connection_site(
                    crate::structure::ConnectionSiteRef {
                        unit_index: connection.element_b,
                        point_index: connection.point_b,
                    },
                    catalog,
                )
                .ok_or_else(|| format!("connection {connection:?} references an invalid second site"))?;

            if !crate::contact::connection_points_contact(site_a, unit_a, site_b, unit_b, 1e-9, 1.0 - 1e-9) {
                return Err(format!("connection {connection:?} does not realize as physical contact"));
            }

            let props_a = unit_a
                .properties(catalog)
                .ok_or_else(|| "missing catalog properties for first connection endpoint".to_string())?;
            let props_b = unit_b
                .properties(catalog)
                .ok_or_else(|| "missing catalog properties for second connection endpoint".to_string())?;
            let strength = crate::combine::bond_strength(*props_a, *props_b);
            if !strength.is_finite() || !(0.0..=1.0).contains(&strength) {
                return Err("connection produced invalid intrinsic bond strength".into());
            }

            structure.add_bond(crate::structure::Bond {
                unit_a: connection.element_a,
                point_a: connection.point_a,
                unit_b: connection.element_b,
                point_b: connection.point_b,
                strength,
                bond_energy: 0.0,
            });
        }

        Ok(structure)
    }

    pub fn is_connected(&self) -> bool {
        if self.elements.is_empty() { return false; }
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

    pub fn total_material_amount(&self) -> f64 {
        self.elements.iter().map(|element| element.material.total_amount()).sum()
    }

    pub fn structural_mass(&self, catalog: &[BaseResource]) -> f64 {
        self.elements.iter().map(|element| element.material.mass(catalog)).sum()
    }
}

impl BlueprintElement {
    pub fn validate(&self) -> Result<(), String> {
        if !self.material.is_valid() {
            return Err("material is invalid".into());
        }
        if self.material.parts.len() != 1 || self.material.has_internal_structure() {
            return Err("structural-unit material must contain exactly one unstructured constituent".into());
        }
        let (_, amount) = &self.material.parts[0];
        if !amount.is_finite() || *amount <= 0.0 {
            return Err("structural-unit material amount must be positive and finite".into());
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
        Placement { x, y, rotation_radians: 0.0 }
    }

    fn element(name: &str, x: f64, y: f64) -> BlueprintElement {
        BlueprintElement { material: Material::free_base(name, 1.0), placement: placement(x, y) }
    }

    #[test]
    fn single_element_blueprint_is_valid() {
        let blueprint = StructuralBlueprint::new(vec![element("Carbon", 0.0, 0.0)], Vec::new());
        assert!(blueprint.is_valid());
        assert_eq!(blueprint.total_material_amount(), 1.0);
    }

    #[test]
    fn blueprint_uses_authoritative_material_identity() {
        let blueprint = StructuralBlueprint::new(vec![element("Carbon", 0.0, 0.0)], Vec::new());
        assert!(blueprint.is_valid());
        assert!(!blueprint.elements[0].material.has_internal_structure());
        assert_eq!(blueprint.elements[0].material.parts.len(), 1);
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
            vec![BlueprintConnection { element_a: 0, point_a: 0, element_b: 1, point_b: 0 }],
        );
        assert!(blueprint.is_valid());
    }

    #[test]
    fn invalid_connection_endpoint_is_rejected() {
        let blueprint = StructuralBlueprint::new(
            vec![element("Carbon", 0.0, 0.0)],
            vec![BlueprintConnection { element_a: 0, point_a: 0, element_b: 1, point_b: 0 }],
        );
        assert!(!blueprint.is_valid());
    }

    #[test]
    fn realization_creates_runtime_structure_from_blueprint() {
        let catalog = crate::resources::default_catalog();
        let blueprint = StructuralBlueprint::new(
            vec![element("Carbon", 0.0, 0.0), element("Carbon", 0.877382, 0.0)],
            vec![BlueprintConnection { element_a: 0, point_a: 0, element_b: 1, point_b: 3 }],
        );
        let structure = blueprint.realize(&catalog).expect("blueprint should realize");
        assert_eq!(structure.units.len(), 2);
        assert_eq!(structure.bonds.len(), 1);
        assert_eq!(structure.units[0].placement, placement(0.0, 0.0));
        assert_eq!(structure.units[1].placement, placement(0.877382, 0.0));
        assert!(structure.is_valid_bond(&structure.bonds[0], &catalog));
    }

    #[test]
    fn realization_rejects_noncontacting_intended_connection() {
        let catalog = crate::resources::default_catalog();
        let blueprint = StructuralBlueprint::new(
            vec![element("Carbon", 0.0, 0.0), element("Carbon", 10.0, 0.0)],
            vec![BlueprintConnection { element_a: 0, point_a: 0, element_b: 1, point_b: 3 }],
        );
        assert!(blueprint.realize(&catalog).is_err());
    }
}
