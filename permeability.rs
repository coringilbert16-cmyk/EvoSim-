//! Physical inputs for permeability.
//!
//! This module deliberately does not assign a magic permeability coefficient.
//! It exposes only quantities that are already determined by material
//! composition and immutable resource properties. Geometry will provide the
//! other side of the permeability calculation.

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
}
