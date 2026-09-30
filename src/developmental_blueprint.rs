#![expect(
    dead_code,
    reason = "Staged developmental-field API retained for solver integration"
)]

//! Continuous inherited developmental-field blueprint.
//!
//! This module stores developmental tendencies rather than an exact body plan.
//! It contains no constituent instances, coordinates, bonds, rotations,
//! silhouette, or guaranteed topology.

use crate::resources::BaseResource;
use rand::Rng;
use serde::{Deserialize, Serialize};

#[path = "developmental_realization.rs"]
mod realization;
pub(crate) use realization::{default_developmental_blueprint, developmental_point};

pub(crate) const CANDIDATE_MATERIAL_WEIGHT: f64 = 1.0; // EXPERIMENTAL: initial solver weight.
pub(crate) const CANDIDATE_DENSITY_WEIGHT: f64 = 1.0; // EXPERIMENTAL: initial solver weight.
pub(crate) const CANDIDATE_CONNECTIVITY_WEIGHT: f64 = 0.25; // EXPERIMENTAL: initial solver weight.

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RadialInfluence {
    pub center_x: f64,
    pub center_y: f64,
    pub radial_falloff: f64,
    pub strength: f64,
}

impl RadialInfluence {
    fn validate(&self) -> bool {
        self.center_x.is_finite()
            && self.center_y.is_finite()
            && self.radial_falloff.is_finite()
            && self.radial_falloff > 0.0
            && self.strength.is_finite()
            && (0.0..=1.0).contains(&self.strength)
    }

    fn evaluate(&self, x: f64, y: f64, scale: f64) -> f64 {
        if !self.validate() || !scale.is_finite() || scale <= 0.0 {
            return 0.0;
        }
        let dx = x - self.center_x * scale;
        let dy = y - self.center_y * scale;
        let falloff = self.radial_falloff / (scale * scale);
        (self.strength * (-falloff * dx.mul_add(dx, dy * dy)).exp()).clamp(0.0, 1.0)
    }

    fn integral(&self, scale: f64) -> f64 {
        if !self.validate() || !scale.is_finite() || scale <= 0.0 {
            return 0.0;
        }
        self.strength * std::f64::consts::PI * scale * scale / self.radial_falloff
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MaterialPreferenceField {
    pub resource_name: String,
    pub center_preference: f64,
    pub radial_falloff: f64,
    #[serde(default)]
    pub center_x: f64,
    #[serde(default)]
    pub center_y: f64,
    /// Additional influences in the initial four-influence representation.
    /// Influence count is experimental; the four-influence starting point is approved.
    #[serde(default)]
    pub additional_influences: Vec<RadialInfluence>,
}

impl MaterialPreferenceField {
    pub fn evaluate(&self, x: f64, y: f64) -> f64 {
        self.evaluate_scaled(x, y, 1.0)
    }

    fn evaluate_scaled(&self, x: f64, y: f64, scale: f64) -> f64 {
        let mut total = if self.center_preference.is_finite() {
            self.primary_influence().evaluate(x, y, scale)
        } else {
            0.0
        };
        total += self
            .additional_influences
            .iter()
            .map(|i| i.evaluate(x, y, scale))
            .sum::<f64>();
        let strength = self.center_preference.max(0.0)
            + self
                .additional_influences
                .iter()
                .map(|i| i.strength)
                .sum::<f64>();
        if strength <= 0.0 {
            0.0
        } else {
            (total / strength).clamp(0.0, 1.0)
        }
    }

    fn primary_influence(&self) -> RadialInfluence {
        RadialInfluence {
            center_x: self.center_x,
            center_y: self.center_y,
            radial_falloff: self.radial_falloff,
            strength: self.center_preference.max(0.0),
        }
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
    #[serde(default)]
    pub center_x: f64,
    #[serde(default)]
    pub center_y: f64,
    /// Additional influences in the initial four-influence representation.
    /// Influence count is experimental; the four-influence starting point is approved.
    #[serde(default)]
    pub additional_influences: Vec<RadialInfluence>,
}

impl StructuralDensityField {
    pub fn evaluate(&self, x: f64, y: f64) -> f64 {
        self.evaluate_scaled(x, y, 1.0)
    }

    fn evaluate_scaled(&self, x: f64, y: f64, scale: f64) -> f64 {
        let primary = RadialInfluence {
            center_x: self.center_x,
            center_y: self.center_y,
            radial_falloff: self.radial_falloff,
            strength: self.center_preference.max(0.0),
        };
        let mut total = primary.evaluate(x, y, scale);
        total += self
            .additional_influences
            .iter()
            .map(|i| i.evaluate(x, y, scale))
            .sum::<f64>();
        let strength = primary.strength
            + self
                .additional_influences
                .iter()
                .map(|i| i.strength)
                .sum::<f64>();
        if strength <= 0.0 {
            0.0
        } else {
            (total / strength).clamp(0.0, 1.0)
        }
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
    #[serde(default)]
    pub center_x: f64,
    #[serde(default)]
    pub center_y: f64,
    #[serde(default)]
    pub radial_falloff: f64,
    /// Additional influences in the initial four-influence representation.
    /// Influence count is experimental; the four-influence starting point is approved.
    #[serde(default)]
    pub additional_influences: Vec<RadialInfluence>,
}

impl ConnectivityField {
    pub fn evaluate(&self, x: f64, y: f64) -> f64 {
        self.evaluate_scaled(x, y, 1.0)
    }

    fn evaluate_scaled(&self, x: f64, y: f64, scale: f64) -> f64 {
        let primary = RadialInfluence {
            center_x: self.center_x,
            center_y: self.center_y,
            radial_falloff: self.radial_falloff,
            strength: self.strength.max(0.0),
        };
        let mut total = primary.evaluate(x, y, scale);
        total += self
            .additional_influences
            .iter()
            .map(|i| i.evaluate(x, y, scale))
            .sum::<f64>();
        let strength = primary.strength
            + self
                .additional_influences
                .iter()
                .map(|i| i.strength)
                .sum::<f64>();
        if strength <= 0.0 {
            0.0
        } else {
            (total / strength).clamp(0.0, 1.0)
        }
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
        self.material_preference_scaled(resource_name, x, y, 1.0)
    }

    pub fn material_preference_scaled(
        &self,
        resource_name: &str,
        x: f64,
        y: f64,
        scale: f64,
    ) -> f64 {
        self.material_preferences
            .iter()
            .filter(|field| field.resource_name == resource_name)
            .map(|field| field.evaluate_scaled(x, y, scale))
            .sum::<f64>()
            .clamp(0.0, 1.0)
    }

    pub fn density_preference(&self, x: f64, y: f64) -> f64 {
        self.density_preference_scaled(x, y, 1.0)
    }

    pub fn density_preference_scaled(&self, x: f64, y: f64, scale: f64) -> f64 {
        self.structural_density.evaluate_scaled(x, y, scale)
    }

    pub fn connectivity_preference(&self, x: f64, y: f64) -> f64 {
        self.connectivity_preference_scaled(x, y, 1.0)
    }

    pub fn connectivity_preference_scaled(&self, x: f64, y: f64, scale: f64) -> f64 {
        self.connectivity.evaluate_scaled(x, y, scale)
    }

    /// Mutate the inherited developmental tendencies. The blueprint is part of
    /// the genome, so its spatial material, density, and connectivity tendencies
    /// evolve alongside ordinary behavioral traits. The realized body remains the
    /// authority: mutation changes future construction pressure, not existing geometry.
    pub(crate) fn mutate(&mut self, rng: &mut impl Rng) {
        const PROBABILITY: f64 = 0.001;
        const SIGMA: f64 = 0.05;

        for field in &mut self.material_preferences {
            mutate_if_selected(rng, PROBABILITY, || {
                field.center_preference = (field.center_preference + gaussian_unit(rng) * SIGMA)
                    .clamp(0.0, 1.0);
            });
            mutate_if_selected(rng, PROBABILITY, || {
                field.radial_falloff = (field.radial_falloff + gaussian_unit(rng) * SIGMA).max(1e-6);
            });
            mutate_if_selected(rng, PROBABILITY, || {
                field.center_x += gaussian_unit(rng) * SIGMA;
            });
            mutate_if_selected(rng, PROBABILITY, || {
                field.center_y += gaussian_unit(rng) * SIGMA;
            });
            for influence in &mut field.additional_influences {
                mutate_influence(influence, rng, PROBABILITY, SIGMA);
            }
        }

        mutate_if_selected(rng, PROBABILITY, || {
            self.structural_density.center_preference =
                (self.structural_density.center_preference + gaussian_unit(rng) * SIGMA)
                    .clamp(0.0, 1.0);
        });
        mutate_if_selected(rng, PROBABILITY, || {
            self.structural_density.radial_falloff =
                (self.structural_density.radial_falloff + gaussian_unit(rng) * SIGMA).max(1e-6);
        });
        mutate_if_selected(rng, PROBABILITY, || {
            self.structural_density.center_x += gaussian_unit(rng) * SIGMA;
        });
        mutate_if_selected(rng, PROBABILITY, || {
            self.structural_density.center_y += gaussian_unit(rng) * SIGMA;
        });
        for influence in &mut self.structural_density.additional_influences {
            mutate_influence(influence, rng, PROBABILITY, SIGMA);
        }

        mutate_if_selected(rng, PROBABILITY, || {
            self.connectivity.strength =
                (self.connectivity.strength + gaussian_unit(rng) * SIGMA).clamp(0.0, 1.0);
        });
        mutate_if_selected(rng, PROBABILITY, || {
            self.connectivity.radial_falloff =
                (self.connectivity.radial_falloff + gaussian_unit(rng) * SIGMA).max(1e-6);
        });
        mutate_if_selected(rng, PROBABILITY, || {
            self.connectivity.center_x += gaussian_unit(rng) * SIGMA;
        });
        mutate_if_selected(rng, PROBABILITY, || {
            self.connectivity.center_y += gaussian_unit(rng) * SIGMA;
        });
        for influence in &mut self.connectivity.additional_influences {
            mutate_influence(influence, rng, PROBABILITY, SIGMA);
        }
    }

    pub fn preferred_developmental_length(
        &self,
        preferred_mass: f64,
        seed_mass: f64,
        seed_length: f64,
    ) -> f64 {
        if !preferred_mass.is_finite()
            || preferred_mass <= 0.0
            || !seed_mass.is_finite()
            || seed_mass <= 0.0
            || !seed_length.is_finite()
            || seed_length <= 0.0
        {
            return 0.0;
        }
        // Approved calibration relationship:
        // L_preferred = L_seed * sqrt(M_preferred / M_seed).
        // Seed values are physical calibration references, never inherited
        // structural or topological authority.
        seed_length * (preferred_mass / seed_mass).sqrt()
    }
}

/// Normalized developmental realization measured from continuous fields and
/// the authoritative physical graph. Quadrature is numerical infrastructure only.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DevelopmentalRealization {
    pub material: Option<f64>,
    pub density: Option<f64>,
    pub connectivity: Option<f64>,
    pub overall: f64,
}


fn mutate_if_selected(rng: &mut impl Rng, probability: f64, mut mutation: impl FnMut()) {
    if rng.gen::<f64>() < probability {
        mutation();
    }
}

fn mutate_influence(influence: &mut RadialInfluence, rng: &mut impl Rng, probability: f64, sigma: f64) {
    mutate_if_selected(rng, probability, || {
        influence.center_x += gaussian_unit(rng) * sigma;
    });
    mutate_if_selected(rng, probability, || {
        influence.center_y += gaussian_unit(rng) * sigma;
    });
    mutate_if_selected(rng, probability, || {
        influence.radial_falloff = (influence.radial_falloff + gaussian_unit(rng) * sigma).max(1e-6);
    });
    mutate_if_selected(rng, probability, || {
        influence.strength = (influence.strength + gaussian_unit(rng) * sigma).clamp(0.0, 1.0);
    });
}

fn gaussian_unit(rng: &mut impl Rng) -> f64 {
    let u1 = rng.gen_range(f64::MIN_POSITIVE..1.0);
    let u2 = rng.gen_range(0.0..1.0);
    (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn developmental_fields_are_continuous_evaluators() {
        let blueprint = default_developmental_blueprint();
        assert!(blueprint.validate().is_ok());
        assert!((blueprint.density_preference(0.0, 0.0) - 1.0).abs() < f64::EPSILON);
        assert!(blueprint.density_preference(2.0, 0.0) < 0.5);
    }

    #[test]
    fn spatial_falloff_changes_preference_without_authored_coordinates() {
        let field = StructuralDensityField {
            center_preference: 1.0,
            radial_falloff: 0.5,
            center_x: 0.0,
            center_y: 0.0,
            additional_influences: Vec::new(),
        };
        assert!(field.evaluate(0.0, 0.0) > field.evaluate(2.0, 0.0));
    }

    #[test]
    fn connectivity_can_remain_inactive() {
        let blueprint = default_developmental_blueprint();
        assert_eq!(blueprint.connectivity_preference(0.0, 0.0), 0.0);
    }
}
