#![expect(
    dead_code,
    reason = "Staged developmental-field API retained for solver integration"
)]

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

    /// Searches a bounded family of physically realizable construction candidates.
    ///
    /// The developmental scale is a preference over the feasible candidates,
    /// not a direct multiplier on physical geometry. Juvenile development biases
    /// that preference toward the smallest feasible realization. The returned
    /// StructuralBlueprint remains a transient solver artifact.
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

        let preference = developmental_scale.clamp(0.0, 1.0);
        let juvenile_bias = juvenile_scale.clamp(0.0, 1.0);
        let effective_preference = preference * juvenile_bias;

        let mut feasible = Vec::<(usize, StructuralBlueprint, bool)>::new();
        for count in 3..=16 {
            let Ok(candidate) = self.candidate_for_count(catalog, count) else {
                continue;
            };
            let Ok(structure) = candidate.realize(catalog) else {
                continue;
            };
            let qualifies = crate::cavity::analyze_genome_cavity(&structure, catalog)
                .ok()
                .flatten()
                .is_some_and(|cavity| cavity.qualifies());
            feasible.push((count, candidate, qualifies));
        }

        if feasible.is_empty() {
            return Err(
                "developmental construction found no physically realizable candidate".into(),
            );
        }

        // Genome-capable candidates are preferred when the search can produce
        // them, but cavity qualification is still derived from realized physics.
        let qualifying = feasible
            .iter()
            .filter(|(_, _, qualifies)| *qualifies)
            .count();
        if qualifying > 0 {
            feasible.retain(|(_, _, qualifies)| *qualifies);
        }

        let max_index = feasible.len().saturating_sub(1);
        let selected_index = (effective_preference * max_index as f64).round() as usize;
        Ok(feasible
            .into_iter()
            .nth(selected_index.min(max_index))
            .expect("selected feasible developmental candidate"))
        .map(|(_, candidate, _)| candidate)
    }

    fn candidate_for_count(
        &self,
        catalog: &[BaseResource],
        count: usize,
    ) -> Result<StructuralBlueprint, String> {
        if count < 3 {
            return Err("construction candidate requires at least three elements".into());
        }

        // First choose material tendencies on a normalized developmental ring.
        // No physical coordinates are stored in the developmental field.
        let mut materials = Vec::with_capacity(count);
        for i in 0..count {
            let angle = i as f64 * std::f64::consts::TAU / count as f64;
            let x = angle.cos();
            let y = angle.sin();
            let material = catalog
                .iter()
                .max_by(|a, b| {
                    self.material_preference(&a.name, x, y)
                        .partial_cmp(&self.material_preference(&b.name, x, y))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .ok_or("no resource candidate")?;
            materials.push(material);
        }

        // A candidate ring is scaled from the actual geometry of its selected
        // materials so adjacent connection points can meet. If the material
        // cannot participate in rigid polygonal contact, this candidate is
        // rejected rather than inventing a geometric approximation.
        let circumradius = materials
            .iter()
            .map(|resource| {
                resource
                    .shape
                    .form
                    .polygon_vertices()
                    .map(|vertices| {
                        vertices
                            .into_iter()
                            .map(|(x, y)| x.hypot(y))
                            .fold(0.0, f64::max)
                    })
                    .unwrap_or(0.0)
            })
            .fold(0.0, f64::max);
        if !circumradius.is_finite() || circumradius <= 0.0 {
            return Err("candidate materials have no rigid polygonal geometry".into());
        }

        let radius = circumradius / (std::f64::consts::PI / count as f64).sin();
        let mut elements = Vec::with_capacity(count);
        for (i, material_resource) in materials.iter().enumerate() {
            let angle = i as f64 * std::f64::consts::TAU / count as f64;
            let normalized_x = angle.cos();
            let normalized_y = angle.sin();
            elements.push(BlueprintElement {
                material: Material::free_base(&material_resource.name, 1.0),
                placement: BlueprintPlacement {
                    x: radius * normalized_x,
                    y: radius * normalized_y,
                    rotation_radians: angle + std::f64::consts::PI,
                },
            });
        }

        let connections = (0..count)
            .map(|i| BlueprintConnection {
                element_a: i.min((i + 1) % count),
                element_b: i.max((i + 1) % count),
            })
            .collect();
        StructuralBlueprint::with_anchor_elements(elements, connections, vec![0]).validate()?;
        Ok(StructuralBlueprint::with_anchor_elements(
            elements,
            connections,
            vec![0],
        ))
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
        .map(
            |(resource_name, center_preference)| MaterialPreferenceField {
                resource_name: resource_name.into(),
                center_preference,
                radial_falloff: 0.0,
            },
        )
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
        let adult = blueprint
            .construction_candidate(&catalog, 0.5, 1.0)
            .unwrap();
        let juvenile = blueprint
            .construction_candidate(&catalog, 0.5, 0.40)
            .unwrap();
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
