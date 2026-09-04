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
    /// Position and orientation relative to the blueprint origin.
    pub placement: Placement,
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
        if !self.elements.iter().all(|element| {
            element.material.is_valid()
                && element.placement.x.is_finite()
                && element.placement.y.is_finite()
                && element.placement.rotation_radians.is_finite()
        }) {
            return false;
        }
        self.connections.iter().all(|connection| {
            connection.element_a < self.elements.len()
                && connection.element_b < self.elements.len()
                && connection.element_a != connection.element_b
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;

    fn placement(x: f64, y: f64) -> Placement {
        Placement { x, y, rotation_radians: 0.0 }
    }

    #[test]
    fn blueprint_represents_material_and_topology() {
        let blueprint = StructuralBlueprint::new(
            vec![
                BlueprintElement { material: StructuralMaterial::single("Carbon"), placement: placement(0.0, 0.0) },
                BlueprintElement { material: StructuralMaterial::single("Methane"), placement: placement(1.0, 0.0) },
            ],
            vec![BlueprintConnection { element_a: 0, point_a: 0, element_b: 1, point_b: 0 }],
        );
        assert!(blueprint.is_valid());
        assert_eq!(blueprint.total_material_amount(), 2.0);
        assert_eq!(blueprint.structural_mass(&default_catalog()), 2.0);
    }

    #[test]
    fn blueprint_does_not_require_six_elements() {
        let blueprint = StructuralBlueprint::new(
            vec![BlueprintElement { material: StructuralMaterial::single("Carbon"), placement: placement(0.0, 0.0) }],
            Vec::new(),
        );
        assert!(blueprint.is_valid());
    }

    #[test]
    fn invalid_connection_reference_is_rejected() {
        let blueprint = StructuralBlueprint::new(
            vec![BlueprintElement { material: StructuralMaterial::single("Carbon"), placement: placement(0.0, 0.0) }],
            vec![BlueprintConnection { element_a: 0, point_a: 0, element_b: 1, point_b: 0 }],
        );
        assert!(!blueprint.is_valid());
    }

    #[test]
    fn blueprint_round_trip_preserves_identity() {
        let blueprint = StructuralBlueprint::new(
            vec![BlueprintElement { material: StructuralMaterial::single("Carbon"), placement: placement(2.0, -1.0) }],
            Vec::new(),
        );
        let restored: StructuralBlueprint = serde_json::from_str(&serde_json::to_string(&blueprint).unwrap()).unwrap();
        assert_eq!(restored, blueprint);
    }
}
