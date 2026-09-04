use crate::resources::{ConnectionPoint, Form, Shape};
use crate::structural_material::StructuralMaterial;
use crate::structure::Placement;
use serde::{Deserialize, Serialize};

/// An inherited description of an organism's physical structure.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
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
    /// Regions have no numerical bond capacity. Geometry determines whether
    /// multiple bonds can physically coexist.
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

    /// Validate the inherited body plan before it is instantiated. This is
    /// deliberately generic: no six-unit core, role tags, or finite bond
    /// capacity are imposed here.
    pub fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }

    /// Validate structural identity, geometry, topology, and physical contact.
    /// This is the authority used before an inherited blueprint may become a
    /// living physical structure.
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
        self.elements.iter().map(|element| element.material.total_amount()).sum()
    }

    pub fn structural_mass(&self, catalog: &[crate::resources::BaseResource]) -> f64 {
        self.elements.iter().map(|element| element.material.mass(catalog)).sum()
    }
}

impl BlueprintElement {
    pub fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }

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
    pub fn is_valid(&self, constituent_count: usize) -> bool {
        self.validate(constituent_count).is_ok()
    }

    pub fn validate(&self, constituent_count: usize) -> Result<(), String> {
        if !self.envelope.is_valid() {
            return Err("envelope geometry is invalid".into());
        }
        if self.constituents.is_empty() {
            return Err("geometry must contain at least one constituent".into());
        }
        for (index, constituent) in self.constituents.iter().enumerate() {
            if constituent.part_index >= constituent_count {
                return Err(format!("constituent {index} references missing material part"));
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
            if !self.point_within_envelope(region.point) {
                return Err(format!("connection region {index} lies outside the element envelope"));
            }
        }
        Ok(())
    }

    fn point_within_envelope(&self, point: ConnectionPoint) -> bool {
        match self.envelope.form {
            Form::Circle { radius } => point.x * point.x + point.y * point.y <= radius * radius + 1e-9,
            Form::Rectangle { width, height } => {
                point.x.abs() <= width / 2.0 + 1e-9 && point.y.abs() <= height / 2.0 + 1e-9
            }
            Form::RegularPolygon { radius, .. } => {
                point.x * point.x + point.y * point.y <= radius * radius + 1e-9
            }
            Form::Polygon { ref vertices } => {
                if vertices.is_empty() {
                    return false;
                }
                point_in_polygon(point.x, point.y, vertices)
            }
            Form::Fluid { nominal_area } => {
                let radius = (nominal_area / std::f64::consts::PI).max(0.0).sqrt();
                point.x * point.x + point.y * point.y <= radius * radius + 1e-9
            }
        }
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
                placement: Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 },
            }],
            envelope: shape,
            connection_regions,
        }
    }

    pub fn from_constituents(constituents: Vec<ConstituentGeometry>) -> Option<Self> {
        if constituents.is_empty() {
            return None;
        }
        let radius = constituents
            .iter()
            .map(|c| {
                let local = c.shape.form.bounding_radius();
                (c.placement.x * c.placement.x + c.placement.y * c.placement.y).sqrt() + local
            })
            .fold(0.0_f64, f64::max);
        if !radius.is_finite() || radius <= 0.0 {
            return None;
        }
        Some(Self {
            constituents,
            envelope: Shape { form: Form::Circle { radius } },
            connection_regions: Vec::new(),
        })
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
        let region_a = element_a.geometry.connection_regions.get(self.point_a)
            .ok_or_else(|| "endpoint A references a missing connection region".to_string())?;
        let region_b = element_b.geometry.connection_regions.get(self.point_b)
            .ok_or_else(|| "endpoint B references a missing connection region".to_string())?;
        let world_a = transform_point(region_a.point, element_a.placement);
        let world_b = transform_point(region_b.point, element_b.placement);
        let distance = ((world_a.x - world_b.x).powi(2) + (world_a.y - world_b.y).powi(2)).sqrt();
        let contact_tolerance = 0.25;
        if !distance.is_finite() || distance > contact_tolerance {
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

fn point_in_polygon(x: f64, y: f64, vertices: &[(f64, f64)]) -> bool {
    let mut inside = false;
    let mut j = vertices.len() - 1;
    for i in 0..vertices.len() {
        let (xi, yi) = vertices[i];
        let (xj, yj) = vertices[j];
        let intersects = ((yi > y) != (yj > y))
            && (x < (xj - xi) * (y - yi) / (yj - yi) + xi);
        if intersects {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::{default_catalog, Shape};

    fn placement(x: f64, y: f64) -> Placement {
        Placement { x, y, rotation_radians: 0.0 }
    }
    fn single_element(name: &str, shape: Shape, x: f64) -> BlueprintElement {
        BlueprintElement { material: StructuralMaterial::single(name), geometry: BlueprintGeometry::single(shape), placement: placement(x, 0.0) }
    }

    #[test]
    fn blueprint_represents_material_topology_and_geometry() {
        let catalog = default_catalog();
        let mut a = single_element("Carbon", catalog[0].shape.clone(), 0.0);
        let mut b = single_element("Methane", catalog[1].shape.clone(), 1.0);
        a.geometry.connection_regions.push(ConnectionRegion { point: ConnectionPoint { x: 0.0, y: 0.4, direction_radians: 0.0 } });
        b.geometry.connection_regions.push(ConnectionRegion { point: ConnectionPoint { x: 0.0, y: -0.4, direction_radians: std::f64::consts::PI } });
        let blueprint = StructuralBlueprint::new(vec![a, b], vec![BlueprintConnection { element_a: 0, point_a: 6, element_b: 1, point_b: 3 }]);
        assert!(blueprint.is_valid());
        assert_eq!(blueprint.total_material_amount(), 2.0);
        assert_eq!(blueprint.structural_mass(&catalog), 2.0);
    }

    #[test]
    fn blueprint_does_not_require_six_elements() {
        let blueprint = StructuralBlueprint::new(vec![single_element("Carbon", default_catalog()[0].shape.clone(), 0.0)], Vec::new());
        assert!(blueprint.is_valid());
    }

    #[test]
    fn disconnected_multi_element_blueprint_is_rejected() {
        let blueprint = StructuralBlueprint::new(vec![single_element("Carbon", default_catalog()[0].shape.clone(), 0.0), single_element("Methane", default_catalog()[1].shape.clone(), 10.0)], Vec::new());
        assert!(!blueprint.is_valid());
    }

    #[test]
    fn invalid_connection_reference_is_rejected() {
        let blueprint = StructuralBlueprint::new(vec![single_element("Carbon", default_catalog()[0].shape.clone(), 0.0)], vec![BlueprintConnection { element_a: 0, point_a: 0, element_b: 1, point_b: 0 }]);
        assert!(!blueprint.is_valid());
    }

    #[test]
    fn invalid_connection_region_reference_is_rejected() {
        let blueprint = StructuralBlueprint::new(vec![single_element("Carbon", default_catalog()[0].shape.clone(), 0.0), single_element("Carbon", default_catalog()[0].shape.clone(), 1.0)], vec![BlueprintConnection { element_a: 0, point_a: 99, element_b: 1, point_b: 0 }]);
        assert!(!blueprint.is_valid());
    }

    #[test]
    fn impossible_contact_is_rejected() {
        let mut a = single_element("Carbon", default_catalog()[0].shape.clone(), 0.0);
        let mut b = single_element("Methane", default_catalog()[1].shape.clone(), 10.0);
        a.geometry.connection_regions.push(ConnectionRegion { point: ConnectionPoint { x: 0.0, y: 0.4, direction_radians: 0.0 } });
        b.geometry.connection_regions.push(ConnectionRegion { point: ConnectionPoint { x: 0.0, y: -0.4, direction_radians: std::f64::consts::PI } });
        let blueprint = StructuralBlueprint::new(vec![a, b], vec![BlueprintConnection { element_a: 0, point_a: 6, element_b: 1, point_b: 3 }]);
        assert!(!blueprint.is_valid());
    }

    #[test]
    fn multiple_connections_can_reference_the_same_region() {
        let mut a = single_element("Carbon", default_catalog()[0].shape.clone(), 0.0);
        let mut b = single_element("Methane", default_catalog()[1].shape.clone(), 1.0);
        let mut c = single_element("Hydrogen", default_catalog()[2].shape.clone(), -1.0);
        a.geometry.connection_regions.push(ConnectionRegion { point: ConnectionPoint { x: 0.0, y: 0.4, direction_radians: 0.0 } });
        b.geometry.connection_regions.push(ConnectionRegion { point: ConnectionPoint { x: 0.0, y: -0.4, direction_radians: std::f64::consts::PI } });
        c.geometry.connection_regions.push(ConnectionRegion { point: ConnectionPoint { x: 0.0, y: -0.4, direction_radians: std::f64::consts::PI } });
        let blueprint = StructuralBlueprint::new(vec![a, b, c], vec![
            BlueprintConnection { element_a: 0, point_a: 6, element_b: 1, point_b: 3 },
            BlueprintConnection { element_a: 0, point_a: 6, element_b: 2, point_b: 1 },
        ]);
        assert!(blueprint.is_valid());
    }

    #[test]
    fn blueprint_round_trip_preserves_identity() {
        let blueprint = StructuralBlueprint::new(vec![single_element("Carbon", default_catalog()[0].shape.clone(), 0.0)], Vec::new());
        let restored: StructuralBlueprint = serde_json::from_str(&serde_json::to_string(&blueprint).unwrap()).unwrap();
        assert_eq!(restored, blueprint);
    }

    #[test]
    fn composite_geometry_preserves_constituent_placement() {
        let constituents = vec![
            ConstituentGeometry { part_index: 0, shape: default_catalog()[0].shape.clone(), placement: placement(-0.3, 0.0) },
            ConstituentGeometry { part_index: 1, shape: default_catalog()[1].shape.clone(), placement: placement(0.3, 0.0) },
        ];
        let geometry = BlueprintGeometry::from_constituents(constituents.clone()).unwrap();
        assert_eq!(geometry.constituents, constituents);
        assert!(geometry.is_valid(2));
    }
}
