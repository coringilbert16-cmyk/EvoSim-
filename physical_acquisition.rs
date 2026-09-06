//! Physical ACQUIRE targeting and interface evaluation.
//!
//! ACQUIRE must target persistent physical environmental material, not an
//! abstract field-cell center. This module performs the non-mutating physical
//! part of that pipeline:
//!
//! perception/target candidate -> organism body -> broad phase -> exact
//! contact/interface -> composition-derived permeability.
//!
//! It deliberately does not transfer material. A transfer amount cannot be
//! invented here: interaction capacity is a separate physical rule that must
//! determine how much material can cross an interface.

use crate::organism_geometry::OrganismBodyGeometry;
use crate::physical_environment::PhysicalEnvironment;
use crate::physical_interface::{physical_interface, PhysicalInterface};
use crate::physical_spatial_index::PhysicalSpatialIndex;
use crate::permeability::permeability;
use crate::resources::BaseResource;
use crate::state::Organism;

/// A physically reachable environmental object with an exact interface.
#[derive(Clone, Debug, PartialEq)]
pub struct PhysicalAcquireCandidate {
    pub object_index: usize,
    pub interface: PhysicalInterface,
    pub permeability: f64,
}

/// Find environmental material that the organism can physically ACQUIRE from.
///
/// The spatial index is only a broad phase. Exact organism/material geometry
/// remains authoritative. Candidates with no finite shared interface or zero
/// permeability are not acquisition targets.
pub fn candidates(
    organism: &Organism,
    environment: &PhysicalEnvironment,
    catalog: &[BaseResource],
    spatial_index: &PhysicalSpatialIndex,
    broad_phase_radius: f64,
    tolerance: f64,
) -> Option<Vec<PhysicalAcquireCandidate>> {
    if !broad_phase_radius.is_finite() || broad_phase_radius < 0.0 {
        return None;
    }

    let body = OrganismBodyGeometry::from_structure(&organism.structure, catalog)?;
    let anchor = organism.occupied_cells.first()?;
    let candidates = spatial_index.candidate_indices(anchor.x, anchor.y, broad_phase_radius);
    let mut out = Vec::new();

    for object_index in candidates {
        let Some(material) = environment.get(object_index) else {
            continue;
        };
        let Some(interface) = physical_interface(&body, material, catalog, tolerance) else {
            continue;
        };
        if interface.interface_length <= 0.0 || interface.participating_boundary_length <= 0.0 {
            continue;
        }

        let Some(permeability) = permeability(
            &material.material,
            catalog,
            interface.interface_length,
            interface.participating_boundary_length,
        ) else {
            continue;
        };
        if permeability <= 0.0 {
            continue;
        }

        out.push(PhysicalAcquireCandidate {
            object_index,
            interface,
            permeability,
        });
    }

    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physical_environment::PhysicalEnvironment;
    use crate::resources::{default_catalog, Material};
    use crate::state::{DevelopmentStage, EnergyLedger, Organism, Position, ResourceSense};
    use crate::structure::{OrganismStructure, Placement, StructuralUnit};

    fn organism_at(name: &str, x: f64, y: f64) -> Organism {
        let mut structure = OrganismStructure::new();
        structure.add_unit(StructuralUnit::new(
            name,
            Placement {
                x,
                y,
                rotation_radians: 0.0,
            },
        ));
        Organism {
            id: "test".into(),
            occupied_cells: vec![Position { x, y }],
            genome: crate::genome::initial_genome(),
            resource_sense: ResourceSense {
                sensed_resources: Vec::new(),
                direction_x: 0.0,
                direction_y: 0.0,
                direction_strength: 0.0,
            },
            memory: Vec::new(),
            decision_history: crate::decision::DecisionHistory::default(),
            usable_energy: 0.0,
            stress: 0.0,
            stored_material: Material::free_base("Carbon", 0.0),
            structure,
            development_stage: DevelopmentStage::Juvenile,
            age: 0,
            reproductive_readiness: 0.0,
            active_transformation_id: None,
            reproductive_construction: None,
        }
    }

    #[test]
    fn candidates_require_exact_physical_interface() {
        let catalog = default_catalog();
        let organism = organism_at("Nitrogen", 0.0, 0.0);
        let mut environment = PhysicalEnvironment::new();
        environment
            .realize(
                Material::free_base("Water", 1.0),
                &[Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .unwrap();

        let mut index = PhysicalSpatialIndex::new(100.0, 100.0, 10.0);
        index.rebuild(&environment);
        let found = candidates(&organism, &environment, &catalog, &index, 5.0, 0.0).unwrap();

        assert_eq!(found.len(), 1);
        assert!(found[0].interface.interface_length > 0.0);
        assert!((found[0].permeability - 1.0).abs() < 1e-12);
    }

    #[test]
    fn tangent_contact_is_not_an_acquire_interface() {
        let catalog = default_catalog();
        let organism = organism_at("Carbon", 0.0, 0.0);
        let mut environment = PhysicalEnvironment::new();
        environment
            .realize(
                Material::free_base("Water", 1.0),
                &[Placement {
                    x: 2.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .unwrap();

        let mut index = PhysicalSpatialIndex::new(100.0, 100.0, 10.0);
        index.rebuild(&environment);
        let found = candidates(&organism, &environment, &catalog, &index, 5.0, 0.0).unwrap();
        assert!(found.is_empty());
    }

    #[test]
    fn dry_material_has_no_permeable_acquire_interface() {
        let catalog = default_catalog();
        let organism = organism_at("Nitrogen", 0.0, 0.0);
        let mut environment = PhysicalEnvironment::new();
        environment
            .realize(
                Material::free_base("Carbon", 1.0),
                &[Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .unwrap();

        let mut index = PhysicalSpatialIndex::new(100.0, 100.0, 10.0);
        index.rebuild(&environment);
        let found = candidates(&organism, &environment, &catalog, &index, 5.0, 0.0).unwrap();
        assert!(found.is_empty());
    }

    #[test]
    fn broad_phase_only_accelerates_and_does_not_create_contact() {
        let catalog = default_catalog();
        let organism = organism_at("Nitrogen", 0.0, 0.0);
        let mut environment = PhysicalEnvironment::new();
        environment
            .realize(
                Material::free_base("Water", 1.0),
                &[Placement {
                    x: 4.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                }],
                &catalog,
            )
            .unwrap();

        let mut index = PhysicalSpatialIndex::new(100.0, 100.0, 10.0);
        index.rebuild(&environment);
        let found = candidates(&organism, &environment, &catalog, &index, 10.0, 0.0).unwrap();
        assert!(found.is_empty());
    }
}
