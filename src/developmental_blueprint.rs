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

mod realization;

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

    /// Returns a transient physical construction candidate derived from the same
    /// developmental field at the requested realization scale.
    ///
    /// The field never stores exact structure. The discrete blueprint returned here
    /// is only a solver artifact and is validated through the physical realization
    /// machinery before it is accepted.
    pub fn construction_candidate(
        &self,
        catalog: &[BaseResource],
        preferred_mass: f64,
        juvenile: bool,
    ) -> Result<StructuralBlueprint, String> {
        self.validate()?;
        if catalog.is_empty() || !preferred_mass.is_finite() || preferred_mass <= 0.0 {
            return Err(
                "developmental construction requires a finite positive preferred mass".into(),
            );
        }

        // Juveniles and adults use the same field. Juvenile construction begins
        // from the confirmed-good seed realization; development then evaluates
        // the same field at the reduced realization scale rather than selecting
        // a separate inherited body plan.
        let realization_fraction = if juvenile { 0.40_f64.powi(2) } else { 1.0 };
        // 0.40 is the approved juvenile linear spatial realization; its comparable 2-D mass fraction is approximately 0.16.
        let target_mass = preferred_mass * realization_fraction;
        let mut candidate = self.confirmed_juvenile_candidate(catalog)?;
        let mut current_mass = candidate.structural_mass(catalog);
        if !current_mass.is_finite() || current_mass <= 0.0 {
            return Err(
                "confirmed juvenile construction baseline has invalid structural mass".into(),
            );
        }
        if current_mass + 1e-9 >= target_mass {
            return Ok(candidate);
        }

        // Search outward from the already-realized candidate. Positions and bonds
        // below are generated by the solver from physical geometry and field
        // preference; they are not inherited coordinates or topology.
        let mut budget = 128usize;
        if let Some(realized) =
            self.find_growth_path(&candidate, catalog, target_mass, &mut budget)?
        {
            candidate = realized;
            current_mass = candidate.structural_mass(catalog);
        }

        // Preferred mass is soft developmental intent. Physical constraints may
        // leave the realized adult below that preference; the physical structure
        // remains authoritative and adulthood is evaluated from its realization.
        Ok(candidate)
    }

    fn confirmed_seed_candidate(
        &self,
        catalog: &[BaseResource],
    ) -> Result<StructuralBlueprint, String> {
        // The confirmed-good juvenile construction baseline is retained as a physical regression
        // baseline. It is not exposed as a genome count, topology, or body-plan
        // authority. Development may grow from this realization using the same
        // developmental field used for adulthood.
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
                    let ring_radius = radius / (std::f64::consts::PI / 4.0).sin();
                    (0..4)
                        .map(|i| {
                            let angle = i as f64 * std::f64::consts::TAU / 4.0;
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
            let mut connections = (0..4)
                .map(|i| BlueprintConnection {
                    element_a: i,
                    element_b: (i + 1) % 4,
                })
                .collect::<Vec<_>>();
            let first = ring[0];
            let outward_length = match &resource.shape.form {
                crate::resources::Form::Rectangle { height, .. } => *height,
                _ => resource.shape.form.bounding_radius().max(1e-6),
            };
            let norm = first.x.hypot(first.y).max(1e-9);
            let direction = (first.x / norm, first.y / norm);
            let mut previous = first;
            for _ in 0..8 {
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
            if qualifies {
                return Ok(candidate);
            }
        }
        Err("confirmed developmental seed could not be physically realized".into())
    }

    fn find_growth_path(
        &self,
        current: &StructuralBlueprint,
        catalog: &[BaseResource],
        target_mass: f64,
        budget: &mut usize,
    ) -> Result<Option<StructuralBlueprint>, String> {
        let current_mass = current.structural_mass(catalog);
        if current_mass + 1e-9 >= target_mass {
            return Ok(Some(current.clone()));
        }
        if *budget == 0 {
            return Ok(None);
        }
        let candidates = self.growth_candidates(current, catalog, target_mass)?;
        for candidate in candidates.into_iter().take(4) {
            if *budget == 0 {
                break;
            }
            *budget -= 1;
            if let Some(realized) =
                self.find_growth_path(&candidate, catalog, target_mass, budget)?
            {
                return Ok(Some(realized));
            }
        }
        Ok(None)
    }

    fn growth_candidates(
        &self,
        current: &StructuralBlueprint,
        catalog: &[BaseResource],
        target_mass: f64,
    ) -> Result<Vec<(StructuralBlueprint, f64)>, String> {
        let mut candidates = Vec::new();
        let base_count = current.elements.len();
        let preferred_length = self
            .preferred_length(catalog, target_mass.max(1e-9))
            .max(1e-6);
        const DIRECTION_SAMPLES: usize = 16;
        for parent in 0..base_count {
            let p = current.elements[parent].placement;
            for step in 0..DIRECTION_SAMPLES {
                let angle = step as f64 * std::f64::consts::TAU / DIRECTION_SAMPLES as f64;
                let radius = catalog
                    .iter()
                    .find(|r| r.name == current.elements[parent].material.parts[0].0)
                    .map(|r| r.shape.form.bounding_radius().max(1e-6) * 1.5)
                    .unwrap_or(1.0);
                let placement = BlueprintPlacement {
                    x: p.x + angle.cos() * radius,
                    y: p.y + angle.sin() * radius,
                    rotation_radians: angle,
                };
                let resource = self.select_material(catalog, placement.x, placement.y)?;
                let mut candidate = current.clone();
                let child = candidate.elements.len();
                candidate.elements.push(BlueprintElement {
                    material: Material::free_base(&resource.name, 1.0),
                    placement,
                });
                candidate.connections.push(BlueprintConnection {
                    element_a: parent,
                    element_b: child,
                });
                if !candidate.is_valid() {
                    continue;
                }
                let Ok(structure) = candidate.realize(catalog) else {
                    continue;
                };
                let mass = structure.structural_mass(catalog);
                // EXPERIMENTAL solver weights. These tune candidate preference but
                // do not establish additional biological authorities.
                const MATERIAL_WEIGHT: f64 = 1.0;
                const DENSITY_WEIGHT: f64 = 1.0;
                const CONNECTIVITY_WEIGHT: f64 = 0.25;
                let field_score = MATERIAL_WEIGHT
                    * self.material_preference_scaled(
                        &resource.name,
                        placement.x,
                        placement.y,
                        preferred_length,
                    )
                    + DENSITY_WEIGHT
                        * self.density_preference_scaled(
                            placement.x,
                            placement.y,
                            preferred_length,
                        )
                    + CONNECTIVITY_WEIGHT
                        * self.connectivity_preference_scaled(
                            placement.x,
                            placement.y,
                            preferred_length,
                        );
                candidates.push((field_score, candidate, mass));
            }
        }
        candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        Ok(candidates
            .into_iter()
            .map(|(_, candidate, mass)| (candidate, mass))
            .collect())
    }

    pub fn preferred_developmental_length(
        &self,
        catalog: &[BaseResource],
        preferred_mass: f64,
    ) -> f64 {
        self.preferred_length(catalog, preferred_mass)
    }

    fn preferred_length(&self, catalog: &[BaseResource], preferred_mass: f64) -> f64 {
        let seed = self.confirmed_seed_candidate(catalog).ok();
        let seed_mass = seed
            .as_ref()
            .map(|candidate| candidate.structural_mass(catalog))
            .filter(|mass| mass.is_finite() && *mass > 0.0)
            .unwrap_or(preferred_mass.max(1.0));
        let seed_length = seed
            .as_ref()
            .map(|candidate| {
                candidate
                    .elements
                    .iter()
                    .map(|element| element.placement.x.hypot(element.placement.y))
                    .fold(0.0, f64::max)
                    .max(1e-6)
            })
            .unwrap_or(1.0);
        // Approved calibration relationship: L_preferred = L_seed * sqrt(M_preferred / M_seed).
        // The seed realization is only a scale calibration reference, never an inherited body plan.
        seed_length * (preferred_mass / seed_mass).max(0.0).sqrt()
    }

    fn select_material<'a>(
        &self,
        catalog: &'a [BaseResource],
        x: f64,
        y: f64,
    ) -> Result<&'a BaseResource, String> {
        catalog
            .iter()
            .max_by(|a, b| {
                self.material_preference(&a.name, x, y)
                    .partial_cmp(&self.material_preference(&b.name, x, y))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .ok_or_else(|| "catalog contains no material candidates".into())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn developmental_fields_are_continuous_evaluators() {
        let blueprint = default_developmental_blueprint();
        assert!(blueprint.validate().is_ok());
        assert!((blueprint.density_preference(0.0, 0.0) - 0.5).abs() < f64::EPSILON);
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
    fn construction_candidate_is_derived_from_fields_and_juvenile_scale() {
        let blueprint = default_developmental_blueprint();
        let catalog = crate::resources::default_catalog();
        let adult = blueprint
            .construction_candidate(&catalog, 30.0, false)
            .unwrap();
        let juvenile = blueprint
            .construction_candidate(&catalog, 30.0, true)
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
