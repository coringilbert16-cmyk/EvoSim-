//! Physical organism/environment boundary contact.
//!
//! This module bridges the two physical geometry models without changing
//! either material identity or ecological storage. A contact is identified by
//! the actual constituent pair whose shapes touch or overlap. No field-cell
//! center, organism radius, or transfer quantity is used as a proxy for
//! physical contact.

use crate::material_geometry::{placed_forms_overlap, PhysicalMaterialInstance, PlacedMaterialPart};
use crate::organism_geometry::OrganismBodyGeometry;

/// A constituent-level physical interface between an organism body and an
/// environmental material instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundaryContact {
    pub organism_unit_index: usize,
    pub material_part_index: usize,
}

/// Find every organism/material constituent pair whose actual rigid geometry
/// is in contact.
///
/// This is deliberately a contact primitive, not a permeability or transfer
/// calculation. A contact proves only that an interface exists. How much
/// material can cross that interface belongs to the later permeability and
/// interaction-capacity layers.
pub fn boundary_contacts(
    body: &OrganismBodyGeometry,
    material: &PhysicalMaterialInstance,
    tolerance: f64,
) -> Vec<BoundaryContact> {
    let mut contacts = Vec::new();

    for body_part in &body.parts {
        let organism_part = PlacedMaterialPart {
            part_index: body_part.unit_index,
            form: body_part.form.clone(),
            placement: crate::structure::Placement {
                x: body_part.x,
                y: body_part.y,
                rotation_radians: body_part.rotation_radians,
            },
        };

        for material_part in &material.geometry.parts {
            if placed_forms_overlap(&organism_part, material_part, tolerance) {
                contacts.push(BoundaryContact {
                    organism_unit_index: body_part.unit_index,
                    material_part_index: material_part.part_index,
                });
            }
        }
    }

    contacts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material_geometry::PhysicalMaterialInstance;
    use crate::resources::{default_catalog, Material};
    use crate::structure::{OrganismStructure, Placement, StructuralUnit};

    #[test]
    fn contact_is_derived_from_actual_constituent_geometry() {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 },
        ));
        let body = OrganismBodyGeometry::from_structure(&structure, &catalog).unwrap();

        let environmental = PhysicalMaterialInstance::new(
            Material::free_base("Hydrogen", 1.0),
            &[Placement { x: 0.5, y: 0.0, rotation_radians: 0.0 }],
            &catalog,
        )
        .unwrap();

        assert_eq!(
            boundary_contacts(&body, &environmental, 0.0),
            vec![BoundaryContact {
                organism_unit_index: 0,
                material_part_index: 0,
            }]
        );
    }

    #[test]
    fn separated_geometry_has_no_boundary_contact() {
        let catalog = default_catalog();
        let mut structure = OrganismStructure::new();
        structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 },
        ));
        let body = OrganismBodyGeometry::from_structure(&structure, &catalog).unwrap();

        let environmental = PhysicalMaterialInstance::new(
            Material::free_base("Hydrogen", 1.0),
            &[Placement { x: 1000.0, y: 0.0, rotation_radians: 0.0 }],
            &catalog,
        )
        .unwrap();

        assert!(boundary_contacts(&body, &environmental, 0.0).is_empty());
    }
}
