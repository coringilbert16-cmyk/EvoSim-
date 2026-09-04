//! Derived physical geometry of the organism's current structural units.
//!
//! This module derives geometry from the actual structural instances and the
//! immutable resource catalog. It does not designate a membrane, envelope, or
//! other aggregate shape as the organism boundary.

use crate::resources::{BaseResource, Shape};
use crate::structure::{OrganismStructure, Placement};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PlacedStructuralShape {
    pub unit_index: usize,
    pub shape: Shape,
    pub placement: Placement,
}

pub(crate) fn placed_structural_shapes(
    structure: &OrganismStructure,
    catalog: &[BaseResource],
) -> Vec<PlacedStructuralShape> {
    structure
        .units
        .iter()
        .enumerate()
        .filter_map(|(unit_index, unit)| {
            let resource = catalog.iter().find(|resource| resource.name == unit.resource_name)?;
            Some(PlacedStructuralShape {
                unit_index,
                shape: resource.shape.clone(),
                placement: unit.placement,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;
    use crate::structure::StructuralUnit;

    fn placement(x: f64, y: f64, rotation_radians: f64) -> Placement {
        Placement {
            x,
            y,
            rotation_radians,
        }
    }

    #[test]
    fn derived_geometry_uses_current_structural_units() {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        structure.add_unit(StructuralUnit::new(
            "Carbon",
            placement(5.0, 7.0, 0.25),
        ));

        let shapes = placed_structural_shapes(&structure, &catalog);
        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].unit_index, 0);
        assert_eq!(shapes[0].shape, catalog[0].shape);
        assert_eq!(shapes[0].placement, placement(5.0, 7.0, 0.25));
    }

    #[test]
    fn each_structural_unit_contributes_its_actual_resource_shape() {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        structure.add_unit(StructuralUnit::new(
            "Carbon",
            placement(0.0, 0.0, 0.0),
        ));
        structure.add_unit(StructuralUnit::new(
            "Methane",
            placement(3.0, 4.0, 1.0),
        ));

        let shapes = placed_structural_shapes(&structure, &catalog);
        assert_eq!(shapes.len(), 2);
        assert_eq!(shapes[0].shape, catalog[0].shape);
        assert_eq!(shapes[1].shape, catalog[1].shape);
        assert_eq!(shapes[1].placement, placement(3.0, 4.0, 1.0));
    }

    #[test]
    fn unknown_resource_has_no_physical_geometry() {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        structure.add_unit(StructuralUnit::new(
            "Unknown",
            placement(0.0, 0.0, 0.0),
        ));

        assert!(placed_structural_shapes(&structure, &catalog).is_empty());
    }

    #[test]
    fn empty_structure_has_no_physical_geometry() {
        let catalog = default_catalog();
        assert!(placed_structural_shapes(&OrganismStructure::new(), &catalog).is_empty());
    }
}
