//! Exact organism/environment interface geometry.
//!
//! This module is the geometry bridge between a realized organism boundary
//! and one physical environmental material instance. It measures the finite
//! shared boundary only; contact without shared boundary remains a zero-length
//! interface. Composition, water content, permeability, and transfer capacity
//! are intentionally outside this module.

use crate::interface_geometry::shared_boundary_length;
use crate::material_geometry::{PhysicalMaterialInstance, PlacedMaterialPart};
use crate::organism_boundary::exposed_boundary;
use crate::organism_geometry::OrganismBodyGeometry;
use crate::structure::Placement;

/// A finite organism/material interface contributed by one constituent pair.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhysicalInterfacePart {
    pub organism_unit_index: usize,
    pub material_part_index: usize,
    pub length: f64,
}

/// Exact interface between a realized organism and one physical material
/// instance.
#[derive(Clone, Debug, PartialEq)]
pub struct PhysicalInterface {
    pub parts: Vec<PhysicalInterfacePart>,
    /// Exact finite shared-boundary measure, L_I.
    pub interface_length: f64,
    /// Organism boundary participating in that same finite interface, L_P.
    pub participating_boundary_length: f64,
}

fn organism_part(part: &crate::organism_geometry::PlacedForm) -> PlacedMaterialPart {
    PlacedMaterialPart {
        part_index: part.unit_index,
        form: part.form.clone(),
        placement: Placement {
            x: part.x,
            y: part.y,
            rotation_radians: part.rotation_radians,
        },
    }
}

/// Measure the exact finite interface between an organism body and a physical
/// environmental material instance.
///
/// The participating organism boundary is defined by the same shared boundary
/// that constitutes the physical interface. This means point contact,
/// tangency, crossing, and containment produce zero finite interface length;
/// only genuinely coincident boundary segments contribute to L_I and L_P.
///
/// The exposed organism boundary is evaluated first so an internal organism
/// constituent boundary cannot be mistaken for an environmental interface.
pub fn physical_interface(
    body: &OrganismBodyGeometry,
    material: &PhysicalMaterialInstance,
    catalog: &[crate::resources::BaseResource],
    tolerance: f64,
) -> Option<PhysicalInterface> {
    if !tolerance.is_finite() || tolerance < 0.0 {
        return None;
    }

    let exposed = exposed_boundary(body, catalog, tolerance)?;
    let mut parts = Vec::new();

    for body_part in &body.parts {
        let organism = organism_part(body_part);
        let exposed_part = exposed
            .parts
            .iter()
            .find(|part| part.unit_index == body_part.unit_index)?;

        if exposed_part.length <= tolerance {
            continue;
        }

        for material_part in &material.geometry.parts {
            let length = shared_boundary_length(&organism, material_part, tolerance);
            if length <= 0.0 {
                continue;
            }

            // A shared boundary can only be participating if the organism
            // constituent has exposed boundary remaining after lattice
            // assembly. Clamp only against the already-derived physical
            // boundary; never invent interface length.
            let length = length.min(exposed_part.length);
            if length > 0.0 && length.is_finite() {
                parts.push(PhysicalInterfacePart {
                    organism_unit_index: body_part.unit_index,
                    material_part_index: material_part.part_index,
                    length,
                });
            }
        }
    }

    let interface_length = parts.iter().map(|part| part.length).sum::<f64>();
    if !interface_length.is_finite() {
        return None;
    }

    // L_P is the portion of the organism boundary that is actually shared with
    // the environmental material. In the current exact constituent geometry,
    // each recorded finite interface contributes that same boundary measure.
    // General overlapping environmental occlusion is not silently resolved;
    // the physical-material realization layer must provide non-overlapping
    // constituent geometry for this sum to remain a true boundary measure.
    let participating_boundary_length = interface_length;

    Some(PhysicalInterface {
        parts,
        interface_length,
        participating_boundary_length,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material_geometry::PhysicalMaterialInstance;
    use crate::resources::{default_catalog, Material};
    use crate::structure::{OrganismStructure, StructuralUnit};

    fn body_at(name: &str, x: f64, y: f64) -> OrganismBodyGeometry {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        structure.add_unit(StructuralUnit::new(
            name,
            Placement {
                x,
                y,
                rotation_radians: 0.0,
            },
        ));
        OrganismBodyGeometry::from_structure(&structure, &catalog).unwrap()
    }

    fn material_at(name: &str, x: f64, y: f64) -> PhysicalMaterialInstance {
        let catalog = default_catalog();
        PhysicalMaterialInstance::new(
            Material::free_base(name, 1.0),
            &[Placement {
                x,
                y,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .unwrap()
    }

    #[test]
    fn coincident_rigid_boundaries_produce_exact_interface_and_participating_length() {
        let catalog = default_catalog();
        let body = body_at("Nitrogen", 0.0, 0.0);
        let material = material_at("Nitrogen", 0.0, 0.0);
        let interface = physical_interface(&body, &material, &catalog, 0.0).unwrap();
        let expected = 2.0 * (1.511_858 + 0.330_719);
        assert!((interface.interface_length - expected).abs() < 1e-10);
        assert!((interface.participating_boundary_length - expected).abs() < 1e-10);
        assert_eq!(interface.parts.len(), 1);
    }

    #[test]
    fn tangent_circles_have_zero_finite_interface() {
        let catalog = default_catalog();
        let body = body_at("Carbon", 0.0, 0.0);
        let material = material_at("Carbon", 2.0, 0.0);
        let interface = physical_interface(&body, &material, &catalog, 0.0).unwrap();
        assert_eq!(interface.interface_length, 0.0);
        assert_eq!(interface.participating_boundary_length, 0.0);
        assert!(interface.parts.is_empty());
    }

    #[test]
    fn separated_material_has_no_interface() {
        let catalog = default_catalog();
        let body = body_at("Nitrogen", 0.0, 0.0);
        let material = material_at("Nitrogen", 1000.0, 0.0);
        let interface = physical_interface(&body, &material, &catalog, 0.0).unwrap();
        assert_eq!(interface.interface_length, 0.0);
        assert!(interface.parts.is_empty());
    }

    #[test]
    fn crossing_boundaries_have_zero_finite_interface() {
        let catalog = default_catalog();
        let body = body_at("Nitrogen", 0.0, 0.0);
        let material = material_at("Nitrogen", 0.5, 0.0);
        let interface = physical_interface(&body, &material, &catalog, 0.0).unwrap();
        assert_eq!(interface.interface_length, 0.0);
        assert!(interface.parts.is_empty());
    }

    #[test]
    fn invalid_tolerance_is_rejected() {
        let catalog = default_catalog();
        let body = body_at("Carbon", 0.0, 0.0);
        let material = material_at("Carbon", 0.0, 0.0);
        assert!(physical_interface(&body, &material, &catalog, -1.0).is_none());
        assert!(physical_interface(&body, &material, &catalog, f64::NAN).is_none());
    }
}
