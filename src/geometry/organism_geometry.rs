//! Organism-to-physical-geometry adapters.
//!
//! Organism code may expose its realized structure as the shared physical-body
//! geometry type. Environmental chemistry consumes that shared type directly
//! and never constructs or queries an OrganismStructure.

use crate::resources::BaseResource;
use crate::structure::OrganismStructure;

pub use crate::physical_body_geometry::{PhysicalBodyGeometry as OrganismBodyGeometry, PlacedForm};
pub(crate) use crate::physical_body_geometry::source_path_blocked_for_candidates;

impl crate::physical_body_geometry::PhysicalBodyGeometry {
    pub fn from_structure(structure: &OrganismStructure, catalog: &[BaseResource]) -> Option<Self> {
        if structure.units.is_empty() {
            return None;
        }

        let structural_indices = structure.structural_unit_indices(catalog);
        if structural_indices.is_empty() {
            return None;
        }
        let mut parts = Vec::with_capacity(structural_indices.len());
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        );

        for i in structural_indices {
            let unit = &structure.units[i];
            let geometry = unit.geometry.as_ref()?;
            let shape = geometry.shape();
            if !shape.is_valid()
                || !unit.placement.x.is_finite()
                || !unit.placement.y.is_finite()
                || !unit.placement.rotation_radians.is_finite()
            {
                return None;
            }

            let r = shape.form.bounding_radius();
            min_x = min_x.min(unit.placement.x - r);
            max_x = max_x.max(unit.placement.x + r);
            min_y = min_y.min(unit.placement.y - r);
            max_y = max_y.max(unit.placement.y + r);
            parts.push(PlacedForm {
                unit_index: i,
                form: shape.form.clone(),
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

    pub(crate) fn from_all_realized_units(structure: &OrganismStructure) -> Option<Self> {
        if structure.units.is_empty() {
            return None;
        }

        let mut parts = Vec::with_capacity(structure.units.len());
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        );

        for (unit_index, unit) in structure.units.iter().enumerate() {
            // StructuralUnit is intentionally one physical constituent. Its
            // constructor and deserializer enforce that invariant; composite
            // materials belong in the physical-material representation, which
            // carries one placement and realized shape per constituent. Reject
            // malformed in-memory state rather than inventing constituent poses.
            if unit.material.parts.len() != 1 {
                return None;
            }
            let geometry = unit.geometry.as_ref()?;
            let shape = geometry.shape();
            let placement = unit.placement;
            if !shape.is_valid()
                || !placement.x.is_finite()
                || !placement.y.is_finite()
                || !placement.rotation_radians.is_finite()
            {
                return None;
            }

            let radius = shape.form.bounding_radius();
            if !radius.is_finite() || radius < 0.0 {
                return None;
            }
            min_x = min_x.min(placement.x - radius);
            max_x = max_x.max(placement.x + radius);
            min_y = min_y.min(placement.y - radius);
            max_y = max_y.max(placement.y + radius);
            parts.push(PlacedForm {
                unit_index,
                form: shape.form.clone(),
                x: placement.x,
                y: placement.y,
                rotation_radians: placement.rotation_radians,
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
}
