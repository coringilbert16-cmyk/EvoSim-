//! Exposed boundary derived from the realized organism lattice.
//!
//! The organism body is the union of its realized structural constituents.
//! This module derives the boundary measure of that union; it does not use a
//! magic organism radius or the broad-phase bounding box as physical geometry.
//!
//! The current rigid lattice forms are represented by constituent boundaries.
//! For the non-overlapping lattice geometry used by the structural model, the
//! exposed boundary is the sum of constituent boundary measures minus the
//! shared boundary between adjacent constituents. Shared boundary is counted
//! twice in the constituent sum and therefore removed twice.

use crate::interface_geometry::shared_boundary_length;
use crate::material_geometry::PlacedMaterialPart;
use crate::organism_geometry::OrganismBodyGeometry;
use crate::resources::{BaseResource, Form};
use crate::structure::Placement;

/// A boundary constituent retained as an exposed portion of the realized body.
///
/// The current representation stores the source constituent and its exact
/// remaining measure. Later interface geometry can use the same constituent
/// identity to calculate the portion participating in environmental contact.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExposedBoundaryPart {
    pub unit_index: usize,
    pub length: f64,
}

/// Exact exposed-boundary result for the current rigid constituent model.
#[derive(Clone, Debug, PartialEq)]
pub struct ExposedBoundary {
    pub parts: Vec<ExposedBoundaryPart>,
    pub total_length: f64,
}

fn placed_part(body_part: &crate::organism_geometry::PlacedForm) -> PlacedMaterialPart {
    PlacedMaterialPart {
        part_index: body_part.unit_index,
        form: body_part.form.clone(),
        placement: Placement {
            x: body_part.x,
            y: body_part.y,
            rotation_radians: body_part.rotation_radians,
        },
    }
}

/// Derive the exposed boundary of the realized body.
///
/// Fluids do not contribute because they have no authoritative spatial
/// boundary. For rigid constituents, each constituent begins with its exact
/// perimeter. A shared boundary between two constituents is internal to the
/// union and is therefore removed from both exposed contributions.
pub fn exposed_boundary(
    body: &OrganismBodyGeometry,
    _catalog: &[BaseResource],
    tolerance: f64,
) -> Option<ExposedBoundary> {
    if !tolerance.is_finite() || tolerance < 0.0 {
        return None;
    }

    let mut parts: Vec<ExposedBoundaryPart> = body
        .parts
        .iter()
        .map(|part| {
            let placed = placed_part(part);
            ExposedBoundaryPart {
                unit_index: part.unit_index,
                length: crate::interface_geometry::boundary_length(&placed),
            }
        })
        .collect();

    // Each shared boundary belongs to both constituent perimeters, but neither
    // side is exposed on the outside of the union. Remove it from both.
    for i in 0..body.parts.len() {
        for j in (i + 1)..body.parts.len() {
            let a = placed_part(&body.parts[i]);
            let b = placed_part(&body.parts[j]);
            let shared = shared_boundary_length(&a, &b, tolerance);
            if shared > 0.0 {
                parts[i].length -= shared;
                parts[j].length -= shared;
            }
        }
    }

    if parts.iter().any(|part| part.length < -tolerance) {
        // The simple union decomposition is not sufficient for overlapping
        // constituents. Reject rather than invent a boundary measurement.
        return None;
    }

    for part in &mut parts {
        if part.length.abs() <= tolerance {
            part.length = 0.0;
        } else {
            part.length = part.length.max(0.0);
        }
    }

    let total_length = parts.iter().map(|part| part.length).sum();
    if !total_length.is_finite() {
        return None;
    }

    Some(ExposedBoundary { parts, total_length })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organism_geometry::OrganismBodyGeometry;
    use crate::resources::default_catalog;
    use crate::structure::{OrganismStructure, StructuralUnit};

    fn body(units: &[(&str, f64, f64)]) -> OrganismBodyGeometry {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        for &(name, x, y) in units {
            structure.add_unit(StructuralUnit::new(
                name,
                Placement {
                    x,
                    y,
                    rotation_radians: 0.0,
                },
            ));
        }
        OrganismBodyGeometry::from_structure(&structure, &catalog).unwrap()
    }

    #[test]
    fn single_rigid_constituent_exposes_its_exact_boundary() {
        let catalog = default_catalog();
        let body = body(&[("Nitrogen", 0.0, 0.0)]);
        let boundary = exposed_boundary(&body, &catalog, 0.0).unwrap();
        let expected = 2.0 * (1.511_858 + 0.330_719);
        assert!((boundary.total_length - expected).abs() < 1e-12);
        assert_eq!(boundary.parts.len(), 1);
    }

    #[test]
    fn coincident_polygon_edge_becomes_internal_boundary() {
        let catalog = default_catalog();
        let body = body(&[("Nitrogen", 0.0, 0.0), ("Nitrogen", 1.511_858, 0.0)]);
        let boundary = exposed_boundary(&body, &catalog, 0.0).unwrap();
        let single = 2.0 * (1.511_858 + 0.330_719);
        let expected = 2.0 * single - 2.0 * 0.330_719;
        assert!((boundary.total_length - expected).abs() < 1e-10);
        assert!(boundary.parts.iter().all(|part| part.length > 0.0));
    }

    #[test]
    fn point_contact_does_not_remove_boundary_length() {
        let catalog = default_catalog();
        let body = body(&[("Carbon", 0.0, 0.0), ("Carbon", 0.877_382, 0.0)]);
        let boundary = exposed_boundary(&body, &catalog, 0.0).unwrap();
        let single = std::f64::consts::TAU * 0.0;
        assert!(boundary.total_length > single);
    }

    #[test]
    fn fluid_constituent_contributes_no_authoritative_boundary() {
        let catalog = default_catalog();
        let body = body(&[("Water", 0.0, 0.0)]);
        let boundary = exposed_boundary(&body, &catalog, 0.0).unwrap();
        assert_eq!(boundary.total_length, 0.0);
        assert_eq!(boundary.parts[0].length, 0.0);
    }

    #[test]
    fn invalid_tolerance_is_rejected() {
        let catalog = default_catalog();
        let body = body(&[("Carbon", 0.0, 0.0)]);
        assert!(exposed_boundary(&body, &catalog, -1.0).is_none());
        assert!(exposed_boundary(&body, &catalog, f64::NAN).is_none());
    }
}
