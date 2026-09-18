#![expect(dead_code, reason = "Staged developmental-field API retained for solver integration")]

//! Continuous inherited developmental-field blueprint.
//!
//! This module stores developmental tendencies rather than an exact body plan.
//! It contains no constituent instances, coordinates, bonds, rotations,
//! silhouette, or guaranteed topology.

use crate::resources::{BaseResource, Material};
use crate::structural_blueprint::{
    BlueprintConnection, BlueprintElement, BlueprintPlacement, StructuralBlueprint,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MaterialPreferenceField {
    pub resource_name: String,
    pub center_preference: f64,
    pub radial_falloff: f64,
}

impl MaterialPreferenceField {
    pub fn evaluate(&self, x: f64, y: f64) -> f64 {
        if !x.is_finite() || !y.is_finite() {
            return 0.0;
        }
        let radius_squared = x.mul_add(x, y * y);
        (self.center_preference * (-self.radial_falloff.max(0.0) * radius_squared).exp())
            .clamp(0.0, 1.0)
    }

    fn validate(&self) -> Result<(), String> {
        if self.resource_name.trim().is_empty()
            || !self.center_preference.is_finite()
            || !self.radial_falloff.is_finite()
            || self.radial_falloff < 0.0
        {
            return Err("material preference field is invalid".into());
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StructuralDensityField {
    pub center_preference: f64,
    pub radial_falloff: f64,
}

impl StructuralDensityField {
    pub fn evaluate(&self, x: f64, y: f64) -> f64 {
        if !x.is_finite() || !y.is_finite() {
            return 0.0;
        }
        let radius_squared = x.mul_add(x, y * y);
        (self.center_preference * (-self.radial_falloff.max(0.0) * radius_squared).exp())
            .clamp(0.0, 1.0)
    }

    fn validate(&self) -> Result<(), String> {
        if !self.center_preference.is_finite()
            || !self.radial_falloff.is_finite()
            || self.radial_falloff < 0.0
        {
            return Err("structural density field is invalid".into());
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ConnectivityField {
    pub strength: f64,
}

impl ConnectivityField {
    pub fn evaluate(&self, _x: f64, _y: f64) -> f64 {
        self.strength.clamp(0.0, 1.0)
    }

    fn validate(&self) -> Result<(), String> {
        if !self.strength.is_finite() || !(0.0..=1.0).contains(&self.strength) {
            return Err("connectivity field is invalid".into());
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DevelopmentalFieldBlueprint {
    pub material_preferences: Vec<MaterialPreferenceField>,
    pub structural_density: StructuralDensityField,
    pub connectivity: ConnectivityField,
}

impl DevelopmentalFieldBlueprint {
    /// Converts the inherited size preference into the normalized developmental
    /// scale consumed by the construction solver. The value is intentionally
    /// dimensionless; physical mass is determined only after realization.
    pub fn preferred_developmental_scale(size_preference: f64) -> f64 {
        size_preference.clamp(0.0, 1.0)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.material_preferences.is_empty() {
            return Err("developmental blueprint requires material preferences".into());
        }
        for field in &self.material_preferences {
            field.validate()?;
        }
        self.structural_density.validate()?;
        self.connectivity.validate()?;
        Ok(())
    }

    pub fn material_preference(&self, resource_name: &str, x: f64, y: f64) -> f64 {
        self.material_preferences
            .iter()
            .find(|field| field.resource_name == resource_name)
            .map_or(0.0, |field| field.evaluate(x, y))
    }

    pub fn density_preference(&self, x: f64, y: f64) -> f64 {
        self.structural_density.evaluate(x, y)
    }

    pub fn connectivity_preference(&self, x: f64, y: f64) -> f64 {
        self.connectivity.evaluate(x, y)
    }

    /// Produces a discrete construction candidate from continuous developmental
    /// preferences. The returned StructuralBlueprint is a transient solver
    /// artifact; it is never stored in the genome.
    pub fn construction_candidate(
        &self,
        catalog: &[BaseResource],
        developmental_scale: f64,
        juvenile_scale: f64,
    ) -> Result<StructuralBlueprint, String> {
        self.validate()?;
        if catalog.is_empty() {
            return Err("developmental construction requires a resource catalog".into());
        }
        let scale = (developmental_scale.clamp(0.0, 1.0)
            * juvenile_scale.clamp(0.40, 1.0))
            .max(0.40);
        let density = self.density_preference(0.0, 0.0);
        let count = (4.0 + (density * 4.0).round()) as usize;
        let radius = 1.677_217_5 * scale;
        let mut elements = Vec::with_capacity(count);
        for i in 0..count {
            let angle = i as f64 * std::f64::consts::TAU / count as f64;
            let x = radius * angle.cos();
            let y = radius * angle.sin();
            let material_name = catalog
                .iter()
                .max_by(|a, b| {
                    self.material_preference(&a.name, x / scale, y / scale)
                        .partial_cmp(&self.material_preference(&b.name, x / scale, y / scale))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|resource| resource.name.clone())
                .ok_or("no resource candidate")?;
            elements.push(BlueprintElement {
                material: Material::free_base(&material_name, 1.0),
                placement: BlueprintPlacement {
                    x,
                    y,
                    rotation_radians: angle + std::f64::consts::FRAC_PI_2,
                },
            });
        }
        let connections = (0..count)
            .map(|i| BlueprintConnection {
                element_a: i.min((i + 1) % count),
                element_b: i.max((i + 1) % count),
            })
            .collect();
        Ok(StructuralBlueprint::with_anchor_elements(elements, connections, vec![0]))
    }
}

pub fn default_developmental_blueprint() -> DevelopmentalFieldBlueprint {
    DevelopmentalFieldBlueprint {
        material_preferences: vec![
            ("Carbon", 1.0),
            ("Nitrogen", 0.0),
            ("Phosphorus", 0.0),
            ("Sulfur", 0.0),
            ("Hydrogen", 0.0),
            ("Methane", 0.0),
            ("Water", 0.0),
        ]
        .into_iter()
        .map(|(resource_name, center_preference)| MaterialPreferenceField {
            resource_name: resource_name.into(),
            center_preference,
            radial_falloff: 0.0,
        })
        .collect(),
        structural_density: StructuralDensityField {
            center_preference: 0.5,
            radial_falloff: 0.0,
        },
        connectivity: ConnectivityField { strength: 0.0 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn developmental_fields_are_continuous_evaluators() {
        let blueprint = default_developmental_blueprint();
        assert!(blueprint.validate().is_ok());
        assert!((blueprint.density_preference(0.0, 0.0) - 0.5).abs() < f64::EPSILON);
        assert!((blueprint.density_preference(2.0, 0.0) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn spatial_falloff_changes_preference_without_authored_coordinates() {
        let field = StructuralDensityField {
            center_preference: 1.0,
            radial_falloff: 0.5,
        };
        assert!(field.evaluate(0.0, 0.0) > field.evaluate(2.0, 0.0));
    }

    #[test]
    fn size_preference_maps_to_developmental_scale_without_mass_authority() {
        assert_eq!(
            DevelopmentalFieldBlueprint::preferred_developmental_scale(0.0),
            0.0
        );
        assert_eq!(
            DevelopmentalFieldBlueprint::preferred_developmental_scale(0.5),
            0.5
        );
        assert_eq!(
            DevelopmentalFieldBlueprint::preferred_developmental_scale(1.0),
            1.0
        );
    }

    #[test]
    fn construction_candidate_is_derived_from_fields_and_juvenile_scale() {
        let blueprint = default_developmental_blueprint();
        let catalog = crate::resources::default_catalog();
        let adult = blueprint.construction_candidate(&catalog, 0.5, 1.0).unwrap();
        let juvenile = blueprint.construction_candidate(&catalog, 0.5, 0.40).unwrap();
        assert!(adult.is_valid() && juvenile.is_valid());
        let adult_extent = adult
            .elements
            .iter()
            .map(|e| e.placement.x.abs().max(e.placement.y.abs()))
            .fold(0.0, f64::max);
        let juvenile_extent = juvenile
            .elements
            .iter()
            .map(|e| e.placement.x.abs().max(e.placement.y.abs()))
            .fold(0.0, f64::max);
        assert!(juvenile_extent < adult_extent);
    }

    #[test]
    fn connectivity_can_remain_inactive() {
        let blueprint = default_developmental_blueprint();
        assert_eq!(blueprint.connectivity_preference(0.0, 0.0), 0.0);
    }
}
