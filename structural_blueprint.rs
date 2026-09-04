use crate::resources::{ConnectionPoint, Shape};
use crate::structural_material::StructuralMaterial;
use crate::structure::Placement;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StructuralBlueprint {
    pub elements: Vec<BlueprintElement>,
    pub connections: Vec<BlueprintConnection>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BlueprintElement {
    pub material: StructuralMaterial,
    pub geometry: BlueprintGeometry,
    pub placement: Placement,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BlueprintGeometry {
    pub constituents: Vec<ConstituentGeometry>,
    pub envelope: Shape,
    #[serde(default)]
    pub connection_regions: Vec<ConnectionRegion>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ConstituentGeometry {
    pub part_index: usize,
    pub shape: Shape,
    pub placement: Placement,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
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

        self.geometry.validate(self.material.constituents().len())
    }
}

impl BlueprintGeometry {
    pub fn validate(&self, constituent_count: usize) -> Result<(), String> {
        if !self.envelope.is_valid() {
            return Err("envelope geometry is invalid".into());
        }
        if self.constituents.is_empty() {
            return Err("geometry must contain at least one constituent".into());
        }

        for (index, constituent) in self.constituents.iter().enumerate() {
            if constituent.part_index >= constituent_count {
                return Err(format!(
                    "constituent {index} references missing material part"
                ));
            }
            if !constituent.shape.is_valid() {
                return Err(format!("constituent {index} shape is invalid"));
            }
            if !constituent.placement.x.is_finite()
                || !constituent.placement.y.is_finite()
                || !constituent.placement.rotation_radians.is_finite()
            {
                return Err(format!("constituent {index} placement is not finite"));
            }
        }

        for (index, region) in self.connection_regions.iter().enumerate() {
            if !region.point.is_valid() {
                return Err(format!("connection region {index} is invalid"));
            }
        }

        Ok(())
    }

    pub fn single(shape: Shape) -> Self {
        let connection_regions = match shape.connection_sites() {
            crate::resources::ConnectionSites::Corners(points) => points
                .into_iter()
                .map(|point| ConnectionRegion { point })
                .collect(),
            crate::resources::ConnectionSites::Circumference { .. }
            | crate::resources::ConnectionSites::Undetermined => Vec::new(),
        };

        Self {
            constituents: vec![ConstituentGeometry {
                part_index: 0,
                shape: shape.clone(),
                placement: Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            }],
            envelope: shape,
            connection_regions,
        }
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

        let element_a = &blueprint.elements[self.element_a];
        let element_b = &blueprint.elements[self.element_b];
        let region_a = element_a
            .geometry
            .connection_regions
            .get(self.point_a)
            .ok_or_else(|| "endpoint A references a missing connection region".to_string())?;
        let region_b = element_b
            .geometry
            .connection_regions
            .get(self.point_b)
            .ok_or_else(|| "endpoint B references a missing connection region".to_string())?;

        let world_a = transform_point(region_a.point, element_a.placement);
        let world_b = transform_point(region_b.point, element_b.placement);
        let distance = ((world_a.x - world_b.x).powi(2) + (world_a.y - world_b.y).powi(2)).sqrt();

        if !distance.is_finite() || distance > 0.25 {
            return Err(format!("endpoints are not in physical contact (distance {distance:.4})"));
        }

        Ok(())
    }
}

fn transform_point(point: ConnectionPoint, placement: Placement) -> ConnectionPoint {
    let (sin, cos) = placement.rotation_radians.sin_cos();
    ConnectionPoint {
        x: placement.x + point.x * cos - point.y * sin,
        y: placement.y + point.x * sin + point.y * cos,
        direction_radians: point.direction_radians + placement.rotation_radians,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;

    fn placement(x: f64, y: f64) -> Placement {
        Placement { x, y, rotation_radians: 0.0 }
    }

    #[test]
    fn single_element_blueprint_is_valid() {
        let catalog = default_catalog();
        let element = BlueprintElement {
            material: StructuralMaterial::single("Carbon"),
            geometry: BlueprintGeometry::single(catalog[0].shape.clone()),
            placement: placement(0.0, 0.0),
        };
        let blueprint = StructuralBlueprint::new(vec![element], Vec::new());
        assert!(blueprint.is_valid());
        assert_eq!(blueprint.total_material_amount(), 1.0);
    }

    #[test]
    fn blueprint_does_not_require_six_elements() {
        let catalog = default_catalog();
        let element = BlueprintElement {
            material: StructuralMaterial::single("Carbon"),
            geometry: BlueprintGeometry::single(catalog[0].shape.clone()),
            placement: placement(0.0, 0.0),
        };
        assert!(StructuralBlueprint::new(vec![element], Vec::new()).is_valid());
    }

    #[test]
    fn disconnected_multi_element_blueprint_is_rejected() {
        let catalog = default_catalog();
        let a = BlueprintElement {
            material: StructuralMaterial::single("Carbon"),
            geometry: BlueprintGeometry::single(catalog[0].shape.clone()),
            placement: placement(0.0, 0.0),
        };
        let b = BlueprintElement {
            material: StructuralMaterial::single("Methane"),
            geometry: BlueprintGeometry::single(catalog[1].shape.clone()),
            placement: placement(10.0, 0.0),
        };
        assert!(!StructuralBlueprint::new(vec![a, b], Vec::new()).is_valid());
    }
}
