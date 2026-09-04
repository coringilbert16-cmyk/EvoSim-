use crate::resources::{ConnectionPoint, Form, Shape};
use crate::structural_material::StructuralMaterial;
use crate::structure::Placement;
use serde::{Deserialize, Serialize};

/// An inherited description of an organism's physical structure.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct StructuralBlueprint {
    /// Material-bearing structural elements in the inherited body plan.
    pub elements: Vec<BlueprintElement>,
    /// Topological connections between those elements.
    pub connections: Vec<BlueprintConnection>,
}

/// One material-bearing element of an inherited structural blueprint.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BlueprintElement {
    /// Complete material composition, including internal chemical bonds.
    pub material: StructuralMaterial,
    /// Physical geometry occupied by the element in its local coordinates.
    /// This is inherited along with the material; it is not regenerated from
    /// a resource-name lookup at runtime.
    pub geometry: BlueprintGeometry,
    /// Position and orientation relative to the blueprint origin.
    pub placement: Placement,
}

/// Geometry for a structural element.
///
/// A composite material may contain multiple constituents, each with its own
/// local geometry. The envelope is the union-level geometry used for broad
/// physical contact tests.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BlueprintGeometry {
    pub constituents: Vec<ConstituentGeometry>,
    pub envelope: Shape,
    /// Physical boundary regions from which bonds may originate. A region is
    /// not a finite-capacity site; geometry alone determines whether another
    /// bond can coexist there.
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

/// A topological connection between two blueprint elements.
///
/// Connection-point capacity is deliberately not represented. Multiple bonds
/// may originate from the same physical region when geometry permits.
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

    /// Validate identity and numeric geometry without imposing a fixed anatomy.
    pub fn is_valid(&self) -> bool {
        if self.elements.is_empty() {
            return false;
        }
        if !self.elements.iter().all(BlueprintElement::is_valid) {
            return false;
        }
        self.connections.iter().all(|connection| {
            connection.element_a < self.elements.len()
                && connection.element_b < self.elements.len()
                && connection.element_a != connection.element_b
                && connection.point_a
                    < self.elements[connection.element_a].geometry.connection_regions.len()
                && connection.point_b
                    < self.elements[connection.element_b].geometry.connection_regions.len()
        })
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
    pub fn is_valid(&self) -> bool {
        if !self.material.is_valid()
            || !self.placement.x.is_finite()
            || !self.placement.y.is_finite()
            || !self.placement.rotation_radians.is_finite()
        {
            return false;
        }
        self.geometry.is_valid(self.material.constituents().len())
    }
}

impl BlueprintGeometry {
    pub fn is_valid(&self, constituent_count: usize) -> bool {
        self.envelope.is_valid()
            && !self.constituents.is_empty()
            && self.constituents.iter().all(|constituent| {
                constituent.part_index < constituent_count
                    && constituent.shape.is_valid()
                    && constituent.placement.x.is_finite()
                    && constituent.placement.y.is_finite()
                    && constituent.placement.rotation_radians.is_finite()
            })
            && self.connection_regions.iter().all(|region| region.point.is_valid())
    }

    /// A minimal geometry for a single base-resource element.
    pub fn single(shape: Shape) -> Self {
        Self {
            constituents: vec![ConstituentGeometry {
                part_index: 0,
                shape: shape.clone(),
                placement: Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 },
            }],
            envelope: shape,
            connection_regions: Vec::new(),
        }
    }

    /// Build a geometry envelope for a set of constituent shapes.
    ///
    /// The initial implementation uses the maximum constituent bounding
    /// radius as a conservative circular envelope. Exact polygon/overlap
    /// geometry remains a later refinement; importantly, the envelope is now
    /// explicit inherited state rather than an implicit resource-name lookup.
    pub fn from_constituents(constituents: Vec<ConstituentGeometry>) -> Option<Self> {
        if constituents.is_empty() {
            return None;
        }
        let radius = constituents
            .iter()
            .map(|c| {
                let local = c.shape.bounding_radius();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::{default_catalog, Shape};

    fn placement(x: f64, y: f64) -> Placement {
        Placement { x, y, rotation_radians: 0.0 }
    }

    fn single_element(name: &str, shape: Shape, x: f64) -> BlueprintElement {
        BlueprintElement {
            material: StructuralMaterial::single(name),
            geometry: BlueprintGeometry::single(shape),
            placement: placement(x, 0.0),
        }
    }

    #[test]
    fn blueprint_represents_material_topology_and_geometry() {
        let carbon_shape = default_catalog()[0].shape.clone();
        let methane_shape = default_catalog()[1].shape.clone();
        let mut a = single_element("Carbon", carbon_shape, 0.0);
        let mut b = single_element("Methane", methane_shape, 1.0);
        a.geometry.connection_regions.push(ConnectionRegion {
            point: ConnectionPoint { x: 0.0, y: 0.4, direction_radians: 0.0 },
        });
        b.geometry.connection_regions.push(ConnectionRegion {
            point: ConnectionPoint { x: 0.0, y: -0.4, direction_radians: std::f64::consts::PI },
        });
        let blueprint = StructuralBlueprint::new(
            vec![a, b],
            vec![BlueprintConnection { element_a: 0, point_a: 0, element_b: 1, point_b: 0 }],
        );
        assert!(blueprint.is_valid());
        assert_eq!(blueprint.total_material_amount(), 2.0);
        assert_eq!(blueprint.structural_mass(&default_catalog()), 2.0);
    }

    #[test]
    fn blueprint_does_not_require_six_elements() {
        let blueprint = StructuralBlueprint::new(
            vec![single_element("Carbon", default_catalog()[0].shape.clone(), 0.0)],
            Vec::new(),
        );
        assert!(blueprint.is_valid());
    }

    #[test]
    fn invalid_connection_reference_is_rejected() {
        let blueprint = StructuralBlueprint::new(
            vec![single_element("Carbon", default_catalog()[0].shape.clone(), 0.0)],
            vec![BlueprintConnection { element_a: 0, point_a: 0, element_b: 1, point_b: 0 }],
        );
        assert!(!blueprint.is_valid());
    }

    #[test]
    fn invalid_connection_region_reference_is_rejected() {
        let blueprint = StructuralBlueprint::new(
            vec![single_element("Carbon", default_catalog()[0].shape.clone(), 0.0), single_element("Carbon", default_catalog()[0].shape.clone(), 1.0)],
            vec![BlueprintConnection { element_a: 0, point_a: 0, element_b: 1, point_b: 0 }],
        );
        assert!(!blueprint.is_valid());
    }

    #[test]
    fn multiple_connections_can_reference_the_same_region() {
        let mut a = single_element("Carbon", default_catalog()[0].shape.clone(), 0.0);
        let mut b = single_element("Methane", default_catalog()[1].shape.clone(), 1.0);
        let mut c = single_element("Hydrogen", default_catalog()[2].shape.clone(), -1.0);
        let region = ConnectionRegion { point: ConnectionPoint { x: 0.0, y: 0.4, direction_radians: 0.0 } };
        a.geometry.connection_regions.push(region);
        b.geometry.connection_regions.push(region);
        c.geometry.connection_regions.push(region);
        let blueprint = StructuralBlueprint::new(
            vec![a, b, c],
            vec![
                BlueprintConnection { element_a: 0, point_a: 0, element_b: 1, point_b: 0 },
                BlueprintConnection { element_a: 0, point_a: 0, element_b: 2, point_b: 0 },
            ],
        );
        assert!(blueprint.is_valid());
    }

    #[test]
    fn blueprint_round_trip_preserves_identity() {
        let blueprint = StructuralBlueprint::new(
            vec![single_element("Carbon", default_catalog()[0].shape.clone(), 2.0)],
            Vec::new(),
        );
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
