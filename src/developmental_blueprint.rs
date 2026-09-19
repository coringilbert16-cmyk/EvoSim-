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

/// Current developmental realization floor for a juvenile derived from the adult blueprint.
pub const CANONICAL_JUVENILE_COUNT: usize = 12;

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

    /// Returns the canonical juvenile realization when juvenile is true.
    ///
    /// The known-good seed construction is the juvenile baseline. It is not
    /// discovered by shrinking an adult target or by searching nearby sizes.
    /// Adult development uses the same transient construction architecture and
    /// expands outward from this baseline according to inherited size preference.
    pub fn construction_candidate(
        &self,
        catalog: &[BaseResource],
        developmental_scale: f64,
        juvenile: bool,
    ) -> Result<StructuralBlueprint, String> {
        self.validate()?;
        if catalog.is_empty() {
            return Err("developmental construction requires a resource catalog".into());
        }

        if juvenile {
            return self.candidate_for_count(catalog, CANONICAL_JUVENILE_COUNT);
        }

        const MAX_DEVELOPMENTAL_COUNT: usize = 16;
        let preference = developmental_scale.clamp(0.0, 1.0);
        let target_count = CANONICAL_JUVENILE_COUNT as f64
            + preference * (MAX_DEVELOPMENTAL_COUNT - CANONICAL_JUVENILE_COUNT) as f64;
        let count = target_count.round() as usize;

        self.candidate_for_count(catalog, count)
    }

    fn candidate_for_count(
        &self,
        catalog: &[BaseResource],
        count: usize,
    ) -> Result<StructuralBlueprint, String> {
        if count < 5 {
            return Err(
                "candidate requires room for a qualifying cavity and extra structure".into(),
            );
        }

        // Physical feasibility is evaluated before preference. Rectangular
        // resources are searched first because their realized geometry can
        // naturally enclose the minimum genome cavity.
        let mut resources = catalog
            .iter()
            .filter(|resource| {
                matches!(
                    resource.shape.form,
                    crate::resources::Form::Rectangle { .. }
                )
            })
            .collect::<Vec<_>>();
        if resources.is_empty() {
            resources = catalog.iter().collect();
        }
        resources.sort_by(|a, b| {
            self.material_preference(&b.name, 0.0, 0.0)
                .partial_cmp(&self.material_preference(&a.name, 0.0, 0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let cycle_count = 4usize;
        for resource in resources {
            let ring = match &resource.shape.form {
                crate::resources::Form::Rectangle { width, height } => {
                    let radius = (width + height) * 0.5;
                    vec![
                        BlueprintPlacement {
                            x: 0.0,
                            y: radius,
                            rotation_radians: 0.0,
                        },
                        BlueprintPlacement {
                            x: radius,
                            y: 0.0,
                            rotation_radians: std::f64::consts::FRAC_PI_2,
                        },
                        BlueprintPlacement {
                            x: 0.0,
                            y: -radius,
                            rotation_radians: 0.0,
                        },
                        BlueprintPlacement {
                            x: -radius,
                            y: 0.0,
                            rotation_radians: std::f64::consts::FRAC_PI_2,
                        },
                    ]
                }
                _ => {
                    let Some(vertices) = resource.shape.form.polygon_vertices() else {
                        continue;
                    };
                    let radius = vertices
                        .iter()
                        .map(|(x, y)| x.hypot(*y))
                        .fold(0.0, f64::max);
                    if radius <= 0.0 {
                        continue;
                    }
                    let ring_radius = radius / (std::f64::consts::PI / cycle_count as f64).sin();
                    (0..cycle_count)
                        .map(|i| {
                            let angle = i as f64 * std::f64::consts::TAU / cycle_count as f64;
                            BlueprintPlacement {
                                x: ring_radius * angle.cos(),
                                y: ring_radius * angle.sin(),
                                rotation_radians: angle + std::f64::consts::FRAC_PI_2,
                            }
                        })
                        .collect()
                }
            };

            let mut elements = ring
                .iter()
                .copied()
                .map(|placement| BlueprintElement {
                    material: Material::free_base(&resource.name, 1.0),
                    placement,
                })
                .collect::<Vec<_>>();

            let mut connections = Vec::with_capacity(count);
            for i in 0..cycle_count {
                let next = (i + 1) % cycle_count;
                connections.push(BlueprintConnection {
                    element_a: i.min(next),
                    element_b: i.max(next),
                });
            }

            let first = ring[0];
            let outward_length = match &resource.shape.form {
                crate::resources::Form::Rectangle { height, .. } => *height,
                _ => resource.shape.form.bounding_radius().max(1e-6),
            };
            let outward = (first.x, first.y);
            let norm = outward.0.hypot(outward.1).max(1e-9);
            let direction = (outward.0 / norm, outward.1 / norm);
            let mut previous = first;

            for _ in 0..(count - cycle_count) {
                let placement = BlueprintPlacement {
                    x: previous.x + direction.0 * outward_length.max(1e-6),
                    y: previous.y + direction.1 * outward_length.max(1e-6),
                    rotation_radians: first.rotation_radians,
                };
                let parent = elements.len() - 1;
                elements.push(BlueprintElement {
                    material: Material::free_base(&resource.name, 1.0),
                    placement,
                });
                connections.push(BlueprintConnection {
                    element_a: parent,
                    element_b: parent + 1,
                });
                previous = placement;
            }

            let candidate =
                StructuralBlueprint::with_anchor_elements(elements, connections, vec![0]);
            if !candidate.is_valid() {
                continue;
            }
            let Ok(structure) = candidate.realize(catalog) else {
                continue;
            };
            let qualifies = crate::cavity::analyze_genome_cavity(&structure, catalog)
                .ok()
                .flatten()
                .is_some_and(|cavity| cavity.qualifies());
            if qualifies && structure.units.len() == count {
                return Ok(candidate);
            }
        }

        Err("candidate search found no physically viable developmental realization".into())
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
            .construction_candidate(&catalog, 0.5, false)
            .unwrap();
        let juvenile = blueprint
            .construction_candidate(&catalog, 0.5, true)
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
