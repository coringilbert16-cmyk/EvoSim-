//! Derived physical geometry of the organism's current structure.
//!
//! This module is deliberately derived from `OrganismStructure` rather than
//! from a separately designated membrane or outer-boundary object. It exposes
//! the actual placed constituent geometry that exists right now. Higher-level
//! containment queries can build on this representation without inventing an
//! enclosing shape.

use crate::resources::Shape;
use crate::structure::{OrganismStructure, Placement};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PlacedStructuralShape {
    pub unit_index: usize,
    pub part_index: usize,
    pub shape: Shape,
    pub placement: Placement,
}

pub(crate) fn placed_structural_shapes(structure: &OrganismStructure) -> Vec<PlacedStructuralShape> {
    structure
        .units
        .iter()
        .enumerate()
        .flat_map(|(unit_index, unit)| {
            unit.geometry.constituents.iter().map(move |constituent| PlacedStructuralShape {
                unit_index,
                part_index: constituent.part_index,
                shape: constituent.shape.clone(),
                placement: compose_placements(unit.placement, constituent.placement),
            })
        })
        .collect()
}

fn compose_placements(parent: Placement, child: Placement) -> Placement {
    let (sin, cos) = parent.rotation_radians.sin_cos();
    Placement {
        x: parent.x + child.x * cos - child.y * sin,
        y: parent.y + child.x * sin + child.y * cos,
        rotation_radians: parent.rotation_radians + child.rotation_radians,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;
    use crate::structure::StructuralUnit;

    fn placement(x: f64, y: f64, rotation_radians: f64) -> Placement {
        Placement { x, y, rotation_radians }
    }

    #[test]
    fn derived_geometry_uses_current_structural_units() {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        structure.add_unit(StructuralUnit::from_blueprint(
            crate::structural_material::StructuralMaterial::single("Carbon"),
            crate::structural_blueprint::BlueprintGeometry::single(catalog[0].shape.clone()),
            placement(5.0, 7.0, 0.0),
        ));

        let shapes = placed_structural_shapes(&structure);
        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].unit_index, 0);
        assert_eq!(shapes[0].part_index, 0);
        assert_eq!(shapes[0].shape, catalog[0].shape);
        assert_eq!(shapes[0].placement, placement(5.0, 7.0, 0.0));
    }

    #[test]
    fn constituent_placement_is_composed_with_unit_placement() {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        let geometry = crate::structural_blueprint::BlueprintGeometry {
            constituents: vec![crate::structural_blueprint::ConstituentGeometry {
                part_index: 0,
                shape: catalog[0].shape.clone(),
                placement: placement(2.0, 0.0, std::f64::consts::FRAC_PI_2),
            }],
            envelope: catalog[0].shape.clone(),
            connection_regions: Vec::new(),
        };
        structure.add_unit(StructuralUnit::from_blueprint(
            crate::structural_material::StructuralMaterial::single("Carbon"),
            geometry,
            placement(10.0, 20.0, std::f64::consts::FRAC_PI_2),
        ));

        let shapes = placed_structural_shapes(&structure);
        assert_eq!(shapes.len(), 1);
        assert!((shapes[0].placement.x - 10.0).abs() < 1e-12);
        assert!((shapes[0].placement.y - 22.0).abs() < 1e-12);
        assert!((shapes[0].placement.rotation_radians - std::f64::consts::PI).abs() < 1e-12);
    }

    #[test]
    fn empty_structure_has_no_physical_geometry() {
        assert!(placed_structural_shapes(&OrganismStructure::new()).is_empty());
    }
}
