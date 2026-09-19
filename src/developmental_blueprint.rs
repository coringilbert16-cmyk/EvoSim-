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

impl DevelopmentalFieldBlueprint {
    pub fn realization(
        &self,
        structure: &crate::structure::OrganismStructure,
        catalog: &[BaseResource],
        developmental_origin: (f64, f64),
        developmental_orientation_radians: f64,
        preferred_mass: f64,
    ) -> DevelopmentalRealization {
        let preferred_length = self.preferred_length(catalog, preferred_mass.max(1e-9));
        let material_available = self.material_available_value(preferred_length);
        let density_available = self.density_available_value(preferred_length);

        let material_realized = if material_available > 0.0 {
            let mut realized = 0.0;
            for unit in &structure.units {
                let Some(shape) = unit.shape(catalog) else {
                    continue;
                };
                let total_amount = unit
                    .material
                    .parts
                    .iter()
                    .map(|(_, amount)| *amount)
                    .sum::<f64>();
                if !total_amount.is_finite() || total_amount <= 0.0 {
                    continue;
                }
                realized += self.integrate_shape_field(
                    &shape.form,
                    unit.placement,
                    developmental_origin,
                    developmental_orientation_radians,
                    |x, y| {
                        unit.material
                            .parts
                            .iter()
                            .map(|(name, amount)| {
                                (amount / total_amount) * self.material_preference_scaled(name, x, y, preferred_length)
                            })
                            .sum::<f64>()
                    },
                );
            }
            Some((realized / material_available).clamp(0.0, 1.0))
        } else {
            None
        };

        let density_realized = if density_available > 0.0 {
            let mut realized = 0.0;
            for unit in &structure.units {
                let Some(shape) = unit.shape(catalog) else {
                    continue;
                };
                realized += self.integrate_shape_field(
                    &shape.form,
                    unit.placement,
                    developmental_origin,
                    developmental_orientation_radians,
                    |x, y| self.density_preference_scaled(x, y, preferred_length),
                );
            }
            Some((realized / density_available).clamp(0.0, 1.0))
        } else {
            None
        };

        let connectivity = self.connectivity_realization(
            structure,
            catalog,
            developmental_origin,
            developmental_orientation_radians,
            preferred_length,
        );

        let mut sum = 0.0;
        let mut active = 0usize;
        for value in [material_realized, density_realized, connectivity] {
            if let Some(value) = value {
                if value.is_finite() {
                    sum += value;
                    active += 1;
                }
            }
        }

        DevelopmentalRealization {
            material: material_realized,
            density: density_realized,
            connectivity,
            overall: if active == 0 {
                0.0
            } else {
                (sum / active as f64).clamp(0.0, 1.0)
            },
        }
    }

    fn material_available_value(&self, preferred_length: f64) -> f64 {
        self.material_preferences
            .iter()
            .map(|field| {
                let primary = field.primary_influence().integral(preferred_length);
                let additional = field
                    .additional_influences
                    .iter()
                    .map(|influence| influence.integral(preferred_length))
                    .sum::<f64>();
                let normalization = field.center_preference.max(0.0)
                    + field
                        .additional_influences
                        .iter()
                        .map(|i| i.strength)
                        .sum::<f64>();
                if normalization <= 0.0 {
                    0.0
                } else {
                    (primary + additional) / normalization
                }
            })
            .sum()
    }

    fn density_available_value(&self, preferred_length: f64) -> f64 {
        let primary = RadialInfluence {
            center_x: self.structural_density.center_x,
            center_y: self.structural_density.center_y,
            radial_falloff: self.structural_density.radial_falloff,
            strength: self.structural_density.center_preference.max(0.0),
        };
        let normalization = primary.strength
            + self
                .structural_density
                .additional_influences
                .iter()
                .map(|i| i.strength)
                .sum::<f64>();
        if normalization <= 0.0 {
            0.0
        } else {
            (primary.integral(preferred_length)
                + self
                    .structural_density
                    .additional_influences
                    .iter()
                    .map(|i| i.integral(preferred_length))
                    .sum::<f64>())
                / normalization
        }
    }

    fn connectivity_realization(
        &self,
        structure: &crate::structure::OrganismStructure,
        catalog: &[BaseResource],
        origin: (f64, f64),
        orientation: f64,
        preferred_length: f64,
    ) -> Option<f64> {
        let mut actual_value = 0.0;
        let mut available_value = 0.0;
        let mut actual_pairs = Vec::new();

        for bond in &structure.bonds {
            let Some(a) = structure.unit_index(bond.endpoint_a.constituent_id) else {
                continue;
            };
            let Some(b) = structure.unit_index(bond.endpoint_b.constituent_id) else {
                continue;
            };
            let Some(wa) = bond
                .endpoint_a
                .location
                .world_point(&structure.units[a], catalog)
            else {
                continue;
            };
            let Some(wb) = bond
                .endpoint_b
                .location
                .world_point(&structure.units[b], catalog)
            else {
                continue;
            };
            let la = developmental_point(wa.x, wa.y, origin, orientation);
            let lb = developmental_point(wb.x, wb.y, origin, orientation);
            actual_value += self.connectivity_score(
                la,
                lb,
                structure,
                a,
                b,
                bond.endpoint_a.location,
                bond.endpoint_b.location,
                catalog,
                preferred_length,
            );
            actual_pairs.push((a, b));
        }

        for a in 0..structure.units.len() {
            for b in (a + 1)..structure.units.len() {
                if actual_pairs
                    .iter()
                    .any(|(x, y)| (*x == a && *y == b) || (*x == b && *y == a))
                {
                    continue;
                }
                for candidate in
                    crate::contact::connection_pair_candidates(structure, a, b, catalog)
                        .into_iter()
                        .filter(|candidate| candidate.available_a && candidate.available_b)
                {
                    let Some(wa) = candidate
                        .endpoint_a
                        .world_point(&structure.units[a], catalog)
                    else {
                        continue;
                    };
                    let Some(wb) = candidate
                        .endpoint_b
                        .world_point(&structure.units[b], catalog)
                    else {
                        continue;
                    };
                    let la = developmental_point(wa.x, wa.y, origin, orientation);
                    let lb = developmental_point(wb.x, wb.y, origin, orientation);
                    available_value += self.connectivity_score(
                        la,
                        lb,
                        structure,
                        a,
                        b,
                        candidate.endpoint_a,
                        candidate.endpoint_b,
                        catalog,
                        preferred_length,
                    );
                }
            }
        }

        available_value += actual_value;
        if available_value <= 0.0 {
            None
        } else {
            Some((actual_value / available_value).clamp(0.0, 1.0))
        }
    }

    fn connectivity_score(
        &self,
        a: (f64, f64),
        b: (f64, f64),
        structure: &crate::structure::OrganismStructure,
        unit_a: usize,
        unit_b: usize,
        endpoint_a: crate::structure::ConnectionEndpoint,
        endpoint_b: crate::structure::ConnectionEndpoint,
        catalog: &[BaseResource],
        preferred_length: f64,
    ) -> f64 {
        let ka = self.connectivity_preference_scaled(a.0, a.1, preferred_length);
        let kb = self.connectivity_preference_scaled(b.0, b.1, preferred_length);
        let qa = endpoint_opportunity_count(structure, unit_a, endpoint_a, catalog);
        let qb = endpoint_opportunity_count(structure, unit_b, endpoint_b, catalog);
        let qreal_a = endpoint_realized_count(structure, unit_a, endpoint_a);
        let qreal_b = endpoint_realized_count(structure, unit_b, endpoint_b);
        let n = 0.5 * (qreal_a as f64 / qa.max(1) as f64 + qreal_b as f64 / qb.max(1) as f64);
        const LAMBDA: f64 = 0.25; // EXPERIMENTAL: initial connectivity neighborhood coefficient.
        let lambda = LAMBDA;
        ((ka + kb) * 0.5 + lambda * n).max(0.0)
    }

    fn integrate_shape_field<F>(
        &self,
        form: &crate::resources::Form,
        placement: crate::structure::Placement,
        origin: (f64, f64),
        orientation: f64,
        field: F,
    ) -> f64
    where
        F: Fn(f64, f64) -> f64,
    {
        const NODES: [f64; 8] = [
            -0.9602898564975363,
            -0.7966664774136267,
            -0.525532409916329,
            -0.1834346424956498,
            0.1834346424956498,
            0.525532409916329,
            0.7966664774136267,
            0.9602898564975363,
        ];
        const WEIGHTS: [f64; 8] = [
            0.1012285362903763,
            0.2223810344533745,
            0.3137066458778873,
            0.362683783378362,
            0.362683783378362,
            0.3137066458778873,
            0.2223810344533745,
            0.1012285362903763,
        ];
        let radius = form.bounding_radius().max(0.0);
        if !radius.is_finite() || radius <= 0.0 {
            return 0.0;
        }
        let min_x = placement.x - radius;
        let max_x = placement.x + radius;
        let min_y = placement.y - radius;
        let max_y = placement.y + radius;
        let hx = (max_x - min_x) * 0.5;
        let hy = (max_y - min_y) * 0.5;
        let cx = (max_x + min_x) * 0.5;
        let cy = (max_y + min_y) * 0.5;
        let mut total = 0.0;
        for i in 0..8 {
            for j in 0..8 {
                let world_x = cx + hx * NODES[i];
                let world_y = cy + hy * NODES[j];
                if !point_in_form(form, placement, world_x, world_y) {
                    continue;
                }
                let (x, y) = developmental_point(world_x, world_y, origin, orientation);
                total += WEIGHTS[i] * WEIGHTS[j] * field(x, y);
            }
        }
        total * hx * hy
    }
}

fn gaussian_plane_integral(amplitude: f64, radial_falloff: f64) -> Option<f64> {
    if !amplitude.is_finite()
        || amplitude <= 0.0
        || !radial_falloff.is_finite()
        || radial_falloff <= 0.0
    {
        None
    } else {
        Some(amplitude * std::f64::consts::PI / radial_falloff)
    }
}

pub(crate) fn developmental_point(
    x: f64,
    y: f64,
    origin: (f64, f64),
    orientation: f64,
) -> (f64, f64) {
    let dx = x - origin.0;
    let dy = y - origin.1;
    let (s, c) = orientation.sin_cos();
    (dx * c + dy * s, -dx * s + dy * c)
}

fn point_in_form(
    form: &crate::resources::Form,
    placement: crate::structure::Placement,
    x: f64,
    y: f64,
) -> bool {
    match form {
        crate::resources::Form::Circle { radius } => {
            (x - placement.x).hypot(y - placement.y) <= *radius
        }
        crate::resources::Form::Rectangle { .. }
        | crate::resources::Form::RegularPolygon { .. }
        | crate::resources::Form::Polygon { .. } => {
            let Some(local_vertices) = form.polygon_vertices() else {
                return false;
            };
            let (s, c) = placement.rotation_radians.sin_cos();
            let local_x = (x - placement.x) * c + (y - placement.y) * s;
            let local_y = -(x - placement.x) * s + (y - placement.y) * c;
            let mut inside = false;
            for i in 0..local_vertices.len() {
                let a = local_vertices[i];
                let b = local_vertices[(i + 1) % local_vertices.len()];
                if (a.1 > local_y) != (b.1 > local_y)
                    && local_x < (b.0 - a.0) * (local_y - a.1) / (b.1 - a.1) + a.0
                {
                    inside = !inside;
                }
            }
            inside
        }
        crate::resources::Form::Line { .. } | crate::resources::Form::Fluid { .. } => false,
    }
}

fn endpoint_realized_count(
    structure: &crate::structure::OrganismStructure,
    unit: usize,
    endpoint: crate::structure::ConnectionEndpoint,
) -> usize {
    let Some(id) = structure.physical_id(unit) else {
        return 0;
    };
    structure
        .bonds
        .iter()
        .filter(|bond| bond.touches(id, endpoint))
        .count()
}

fn endpoint_opportunity_count(
    structure: &crate::structure::OrganismStructure,
    unit: usize,
    endpoint: crate::structure::ConnectionEndpoint,
    catalog: &[BaseResource],
) -> usize {
    let mut count = endpoint_realized_count(structure, unit, endpoint);
    for other in 0..structure.units.len() {
        if other == unit {
            continue;
        }
        if crate::contact::connection_pair_candidates(structure, unit, other, catalog)
            .into_iter()
            .any(|candidate| {
                candidate.available_a
                    && candidate.available_b
                    && candidate.endpoint_a.same_location(endpoint)
            })
        {
            count += 1;
        }
    }
    count.max(1)
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
                radial_falloff: 2.0, // EXPERIMENTAL: alpha=0.5 initial Gaussian width.
                center_x: 0.0,       // EXPERIMENTAL: initial influence center.
                center_y: 0.0,       // EXPERIMENTAL: initial influence center.
                additional_influences: vec![
                    RadialInfluence {
                        center_x: 0.0,
                        center_y: 0.0,
                        radial_falloff: 2.0,
                        strength: center_preference,
                    };
                    3
                ],
            },
        )
        .collect(),
        structural_density: StructuralDensityField {
            center_preference: 0.5,
            radial_falloff: 2.0, // EXPERIMENTAL: alpha=0.5 initial Gaussian width.
            center_x: 0.0,       // EXPERIMENTAL: initial influence center.
            center_y: 0.0,       // EXPERIMENTAL: initial influence center.
            additional_influences: vec![
                RadialInfluence {
                    center_x: 0.0,
                    center_y: 0.0,
                    radial_falloff: 2.0,
                    strength: 0.5,
                };
                3
            ],
        },
        connectivity: ConnectivityField {
            strength: 0.0,
            center_x: 0.0,       // EXPERIMENTAL: initial influence center.
            center_y: 0.0,       // EXPERIMENTAL: initial influence center.
            radial_falloff: 2.0, // EXPERIMENTAL: alpha=0.5 initial Gaussian width.
            additional_influences: vec![
                RadialInfluence {
                    center_x: 0.0,
                    center_y: 0.0,
                    radial_falloff: 2.0,
                    strength: 0.0,
                };
                3
            ],
        },
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
