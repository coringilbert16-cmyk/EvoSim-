//! Permeability derived from material composition and exact physical interface.
//!
//! Permeability is not a stored material property and has no magic coefficient.
//! It is derived from the approved physical model:
//!
//! W = Water mass / total material mass
//! G = clamp(L_I / L_P, 0, 1)
//! P = W * G
//!
//! Composition supplies W. Exact interface geometry supplies L_I and L_P.
//! No field-cell, movement, reach, or transfer-rate concept belongs here.

use crate::resources::{BaseResource, Material};

/// Return the fraction of a material's physical mass contributed by Water.
///
/// Water content is therefore derived from composition and immutable resource
/// mass properties. It is not stored on Material and cannot drift separately
/// from the material itself.
pub fn water_mass_fraction(material: &Material, catalog: &[BaseResource]) -> f64 {
    if !material.is_valid() {
        return 0.0;
    }

    let mut total_mass = 0.0;
    let mut water_mass = 0.0;

    for (name, amount) in &material.parts {
        let Some(resource) = catalog.iter().find(|resource| resource.name == *name) else {
            return 0.0;
        };

        let mass = resource.properties.mass * *amount;
        total_mass += mass;
        if name == "Water" {
            water_mass += mass;
        }
    }

    if total_mass <= 0.0 || !total_mass.is_finite() || !water_mass.is_finite() {
        return 0.0;
    }

    (water_mass / total_mass).clamp(0.0, 1.0)
}

/// Calculate permeability from material water content and exact interface geometry.
///
/// `interface_length` is L_I: the exact physical shared-boundary measure.
/// `participating_boundary_length` is L_P: the boundary measure against which
/// that interface participates. A zero participating boundary produces no
/// permeability rather than an invented infinite ratio.
pub fn permeability(
    material: &Material,
    catalog: &[BaseResource],
    interface_length: f64,
    participating_boundary_length: f64,
) -> Option<f64> {
    if !material.is_valid()
        || !interface_length.is_finite()
        || !participating_boundary_length.is_finite()
        || interface_length < 0.0
        || participating_boundary_length < 0.0
    {
        return None;
    }

    if participating_boundary_length == 0.0 {
        return Some(0.0);
    }

    let water_fraction = water_mass_fraction(material, catalog);
    let geometry_factor =
        (interface_length / participating_boundary_length).clamp(0.0, 1.0);
    let result = water_fraction * geometry_factor;

    if result.is_finite() {
        Some(result.clamp(0.0, 1.0))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;

    #[test]
    fn pure_water_has_full_water_mass_fraction() {
        let catalog = default_catalog();
        let material = Material::free_base("Water", 2.0);
        assert!((water_mass_fraction(&material, &catalog) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn water_fraction_is_mass_weighted() {
        let catalog = default_catalog();
        let water_mass = catalog
            .iter()
            .find(|resource| resource.name == "Water")
            .unwrap()
            .properties
            .mass;
        let carbon_mass = catalog
            .iter()
            .find(|resource| resource.name == "Carbon")
            .unwrap()
            .properties
            .mass;
        let material = Material {
            parts: vec![("Water".to_string(), 1.0), ("Carbon".to_string(), 1.0)],
            internal_bonds: Vec::new(),
        };
        let expected = water_mass / (water_mass + carbon_mass);
        assert!((water_mass_fraction(&material, &catalog) - expected).abs() < 1e-12);
    }

    #[test]
    fn water_fraction_is_zero_without_water() {
        let catalog = default_catalog();
        let material = Material::free_base("Carbon", 2.0);
        assert_eq!(water_mass_fraction(&material, &catalog), 0.0);
    }

    #[test]
    fn invalid_material_does_not_produce_water_content() {
        let catalog = default_catalog();
        let material = Material {
            parts: vec![("Water".to_string(), 1.0)],
            internal_bonds: vec![crate::resources::InternalBond {
                part_a: 0,
                part_b: 1,
            }],
        };
        assert_eq!(water_mass_fraction(&material, &catalog), 0.0);
    }

    #[test]
    fn no_interface_produces_zero_permeability() {
        let catalog = default_catalog();
        let material = Material::free_base("Water", 1.0);
        assert_eq!(permeability(&material, &catalog, 0.0, 5.0), Some(0.0));
    }

    #[test]
    fn pure_water_permeability_equals_geometry_factor() {
        let catalog = default_catalog();
        let material = Material::free_base("Water", 1.0);
        assert!((permeability(&material, &catalog, 2.0, 4.0).unwrap() - 0.5).abs() < 1e-12);
    }

    #[test]
    fn full_participating_boundary_gives_full_geometry_factor() {
        let catalog = default_catalog();
        let material = Material::free_base("Water", 1.0);
        assert_eq!(permeability(&material, &catalog, 4.0, 4.0), Some(1.0));
    }

    #[test]
    fn interface_ratio_is_clamped_to_one() {
        let catalog = default_catalog();
        let material = Material::free_base("Water", 1.0);
        assert_eq!(permeability(&material, &catalog, 8.0, 4.0), Some(1.0));
    }

    #[test]
    fn zero_participating_boundary_has_zero_permeability() {
        let catalog = default_catalog();
        let material = Material::free_base("Water", 1.0);
        assert_eq!(permeability(&material, &catalog, 0.0, 0.0), Some(0.0));
    }

    #[test]
    fn non_water_material_reduces_permeability_by_water_fraction() {
        let catalog = default_catalog();
        let water_mass = catalog
            .iter()
            .find(|resource| resource.name == "Water")
            .unwrap()
            .properties
            .mass;
        let carbon_mass = catalog
            .iter()
            .find(|resource| resource.name == "Carbon")
            .unwrap()
            .properties
            .mass;
        let material = Material {
            parts: vec![("Water".to_string(), 1.0), ("Carbon".to_string(), 1.0)],
            internal_bonds: Vec::new(),
        };
        let expected_water_fraction = water_mass / (water_mass + carbon_mass);
        let expected = expected_water_fraction * 0.5;
        assert!((permeability(&material, &catalog, 2.0, 4.0).unwrap() - expected).abs() < 1e-12);
    }

    #[test]
    fn invalid_geometry_is_rejected() {
        let catalog = default_catalog();
        let material = Material::free_base("Water", 1.0);
        assert!(permeability(&material, &catalog, -1.0, 4.0).is_none());
        assert!(permeability(&material, &catalog, 1.0, -1.0).is_none());
        assert!(permeability(&material, &catalog, f64::NAN, 4.0).is_none());
        assert!(permeability(&material, &catalog, 1.0, f64::INFINITY).is_none());
    }
}
