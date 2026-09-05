use crate::resources::{merge_parts, ConnectionPoint, Material, Shape};
use crate::structural_material::StructuralMaterial;
use crate::structure::Placement;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StructuralBlueprint {
    pub elements: Vec<BlueprintElement>,
    pub connections: Vec<BlueprintConnection>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BlueprintElement {
    pub material: StructuralMaterial,
    pub geometry: BlueprintGeometry,
    pub placement: Placement,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BlueprintGeometry {
    pub constituents: Vec<ConstituentGeometry>,
    #[serde(default)]
    pub connection_regions: Vec<ConnectionRegion>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConstituentGeometry {
    pub part_index: usize,
    pub shape: Shape,
    pub placement: Placement,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct ConnectionRegion {
    pub point: ConnectionPoint,
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

    pub fn total_material_amount(&self) -> f64 {
        self.elements
            .iter()
            .map(|element| element.material.total_amount())
            .sum()
    }

    /// Return the exact unbonded material composition required to realize every
    /// element in this blueprint. This is a blueprint accounting helper; it is
    /// not used as an upfront reproductive resource requirement.
    pub fn required_material(&self) -> Material {
        let parts: Vec<(String, f64)> = self
            .elements
            .iter()
            .flat_map(|element| element.material.constituents().iter().cloned())
            .collect();
        Material {
            parts: merge_parts(&parts),
            bonded: false,
        }
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
        self.geometry.validate(self.material.constituents().len())
    }
}

impl BlueprintGeometry {
    pub fn single(shape: Shape) -> Self {
        Self {
            constituents: vec![crate::structural_blueprint::ConstituentGeometry {
                part_index: 0,
                shape,
                placement: Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            }],
            connection_regions: Vec::new(),
        }
    }

    pub fn validate(&self, constituent_count: usize) -> Result<(), String> {
        if self.constituents.is_empty() {
            return Err("geometry must contain at least one constituent".into());
        }
        for constituent in &self.constituents {
            if constituent.part_index >= constituent_count {
                return Err("constituent part index is out of range".into());
            }
        }
        Ok(())
    }
}

impl BlueprintConnection {
    pub fn validate(&self, blueprint: &StructuralBlueprint) -> Result<(), String> {
        if self.element_a >= blueprint.elements.len() || self.element_b >= blueprint.elements.len() {
            return Err("connection element index is out of range".into());
        }
        if self.element_a == self.element_b {
            return Err("connection cannot join an element to itself".into());
        }
        let points_a = blueprint.elements[self.element_a].geometry.connection_regions.len();
        let points_b = blueprint.elements[self.element_b].geometry.connection_regions.len();
        if self.point_a >= points_a || self.point_b >= points_b {
            return Err("connection point index is out of range".into());
        }
        if blueprint.connections.iter().any(|existing| {
            existing != self
                && ((existing.element_a == self.element_a
                    && existing.point_a == self.point_a
                    && existing.element_b == self.element_b
                    && existing.point_b == self.point_b)
                    || (existing.element_a == self.element_b
                        && existing.point_a == self.point_b
                        && existing.element_b == self.element_a
                        && existing.point_b == self.point_a))
        }) {
            return Err("duplicate connection".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn element(resource: &str) -> BlueprintElement {
        let catalog = crate::resources::default_catalog();
        let shape = catalog.iter().find(|r| r.name == resource).unwrap().shape.clone();
        BlueprintElement {
            material: StructuralMaterial::single(resource),
            geometry: BlueprintGeometry::single(shape),
            placement: Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 },
        }
    }

    #[test]
    fn single_element_blueprint_is_valid() {
        assert!(StructuralBlueprint::new(vec![element("Carbon")], Vec::new()).is_valid());
    }

    #[test]
    fn blueprint_does_not_require_six_elements() {
        assert!(StructuralBlueprint::new(vec![element("Carbon")], Vec::new()).is_valid());
    }

    #[test]
    fn required_material_is_aggregated() {
        let blueprint = StructuralBlueprint::new(vec![element("Carbon"), element("Carbon")], vec![BlueprintConnection { element_a: 0, point_a: 0, element_b: 1, point_b: 0 }]);
        let required = blueprint.required_material();
        assert_eq!(required.parts, vec![("Carbon".into(), 2.0)]);
        assert!(!required.bonded);
    }
}
