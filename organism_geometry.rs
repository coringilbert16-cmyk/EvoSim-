//! Authoritative physical geometry derived from an organism's actual structure.
//!
//! This module does not invent an organism radius. The body is the collection
//! of the geometries of its structural units at their current placements.
//! Bounding envelopes are acceleration data only; physical identity remains
//! the constituent unit geometry.

use crate::resources::{BaseResource, Form};
use crate::structure::OrganismStructure;

#[derive(Clone, Debug, PartialEq)]
pub struct PlacedForm {
    pub unit_index: usize,
    pub form: Form,
    pub x: f64,
    pub y: f64,
    pub rotation_radians: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OrganismBodyGeometry {
    pub parts: Vec<PlacedForm>,
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
}

impl OrganismBodyGeometry {
    /// Derive the physical body directly from the organism's structural units.
    ///
    /// A missing catalog resource or invalid geometry invalidates the whole
    /// derived body rather than silently substituting a generic radius.
    pub fn from_structure(
        structure: &OrganismStructure,
        catalog: &[BaseResource],
    ) -> Option<Self> {
        if structure.units.is_empty() {
            return None;
        }

        let mut parts = Vec::with_capacity(structure.units.len());
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for (unit_index, unit) in structure.units.iter().enumerate() {
            let resource = catalog.iter().find(|resource| resource.name == unit.resource_name)?;
            if !resource.shape.is_valid()
                || !unit.placement.x.is_finite()
                || !unit.placement.y.is_finite()
                || !unit.placement.rotation_radians.is_finite()
            {
                return None;
            }

            let radius = resource.shape.form.bounding_radius();
            min_x = min_x.min(unit.placement.x - radius);
            max_x = max_x.max(unit.placement.x + radius);
            min_y = min_y.min(unit.placement.y - radius);
            max_y = max_y.max(unit.placement.y + radius);
            parts.push(PlacedForm {
                unit_index,
                form: resource.shape.form.clone(),
                x: unit.placement.x,
                y: unit.placement.y,
                rotation_radians: unit.placement.rotation_radians,
            });
        }

        Some(Self {
            parts,
            min_x,
            max_x,
            min_y,
            max_y,
        })
    }

    /// Conservative broad-phase test. This deliberately does not establish
    /// physical contact; callers must perform shape-level testing afterward.
    pub fn bounding_box_contains(&self, x: f64, y: f64) -> bool {
        x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
    }

    pub fn bounding_radius_about(&self, x: f64, y: f64) -> f64 {
        self.parts
            .iter()
            .map(|part| (part.x - x).hypot(part.y - y) + part.form.bounding_radius())
            .fold(0.0, f64::max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;
    use crate::structure::{OrganismStructure, Placement, StructuralUnit};

    #[test]
    fn body_geometry_is_derived_from_actual_structural_units() {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement { x: 10.0, y: 20.0, rotation_radians: 0.0 },
        ));
        structure.add_unit(StructuralUnit::new(
            "Hydrogen",
            Placement { x: 30.0, y: 20.0, rotation_radians: 0.0 },
        ));

        let body = OrganismBodyGeometry::from_structure(&structure, &catalog).unwrap();
        assert_eq!(body.parts.len(), 2);
        assert_eq!(body.parts[0].unit_index, 0);
        assert_eq!(body.parts[1].unit_index, 1);
        assert!(body.max_x > body.min_x);
    }

    #[test]
    fn body_geometry_does_not_use_a_single_magic_radius_as_identity() {
        let catalog = default_catalog();
        let mut one = OrganismStructure::new();
        one.add_unit(StructuralUnit::new(
            "Carbon",
            Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 },
        ));
        let first = OrganismBodyGeometry::from_structure(&one, &catalog).unwrap();

        let mut two = one.clone();
        two.add_unit(StructuralUnit::new(
            "Carbon",
            Placement { x: 100.0, y: 0.0, rotation_radians: 0.0 },
        ));
        let second = OrganismBodyGeometry::from_structure(&two, &catalog).unwrap();

        assert!(second.max_x - second.min_x > first.max_x - first.min_x);
        assert!(second.bounding_radius_about(0.0, 0.0) > first.bounding_radius_about(0.0, 0.0));
    }

    #[test]
    fn empty_structure_has_no_physical_body() {
        let body = OrganismBodyGeometry::from_structure(&OrganismStructure::new(), &default_catalog());
        assert!(body.is_none());
    }

    #[test]
    fn invalid_catalog_geometry_rejects_body_derivation() {
        let mut catalog = default_catalog();
        catalog[0].shape.form = Form::Circle { radius: -1.0 };
        let mut structure = OrganismStructure::new();
        structure.add_unit(StructuralUnit::new(
            catalog[0].name.clone(),
            Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 },
        ));
        assert!(OrganismBodyGeometry::from_structure(&structure, &catalog).is_none());
    }
}
