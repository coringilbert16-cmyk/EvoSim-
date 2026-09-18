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

        // These fields rank physically feasible developmental realizations.
        // They never become direct coordinate or piece-count authorities.
        let preference = developmental_scale.clamp(0.0, 1.0);
        let juvenile_bias = juvenile_scale.clamp(0.0, 1.0);
        let effective_preference = (preference * juvenile_bias).clamp(0.0, 1.0);

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

        // A genome-capable realization is required where one exists for this
        // developmental request, but qualification remains a consequence of
        // the realized structure rather than an authored core.
        let qualifying: Vec<_> = feasible
            .iter()
            .filter(|(_, _, qualifies)| *qualifies)
            .map(|(count, _, _)| *count)
            .collect();

        let (min_count, max_count) =
            if let (Some(min), Some(max)) = (qualifying.iter().min(), qualifying.iter().max()) {
                (*min, *max)
            } else {
                let min = feasible.iter().map(|(count, _, _)| *count).min().unwrap();
                let max = feasible.iter().map(|(count, _, _)| *count).max().unwrap();
                (min, max)
            };

        let target_count =
            min_count as f64 + effective_preference * (max_count.saturating_sub(min_count) as f64);

        let selected_index = feasible
            .iter()
            .enumerate()
            .filter(|(_, (_, _, qualifies))| qualifying.is_empty() || *qualifies)
            .min_by(|(_, a), (_, b)| {
                let da = (a.0 as f64 - target_count).abs();
                let db = (b.0 as f64 - target_count).abs();
                da.partial_cmp(&db)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.0.cmp(&b.0))
            })
            .map(|(index, _)| index)
            .expect("selected feasible developmental candidate");

        Ok(feasible
            .into_iter()
            .nth(selected_index)
            .expect("selected feasible developmental candidate")
            .1)
    }

    fn candidate_for_count(
        &self,
        catalog: &[BaseResource],
        count: usize,
    ) -> Result<StructuralBlueprint, String> {
        // The minimum viable realization must contain a closed physical region
        // plus material outside that region. Those are validation outcomes, not
        // inherited topology. This candidate generator therefore searches the
        // smallest bounded graph that can express both conditions: a cycle
        // capable of producing a cavity and one locally connected growth
        // extension. The actual geometry is taken from the selected resources.
        const MIN_TOTAL_ELEMENTS: usize = 13;
        const CAVITY_CYCLE_ELEMENTS: usize = 12;
        if count < MIN_TOTAL_ELEMENTS {
            return Err(
                "candidate is too small to express a qualifying cavity and external structure"
                    .into(),
            );
        }

        let cycle_count = CAVITY_CYCLE_ELEMENTS;
        let mut materials = Vec::with_capacity(count);

        // Material preference is sampled at normalized developmental locations
        // only to choose constituent tendencies. These normalized coordinates
        // are not persisted as physical coordinates.
        for i in 0..cycle_count {
            let angle = i as f64 * std::f64::consts::TAU / cycle_count as f64;
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

        // Remaining constituents are selected from the inherited material
        // tendency along a local developmental extension. Their count changes
        // developmental extent, while physical spacing is determined below
        // from the selected resource geometry.
        for extension_index in 0..(count - cycle_count) {
            let material = catalog
                .iter()
                .max_by(|a, b| {
                    self.material_preference(&a.name, 1.0 + extension_index as f64, 0.0)
                        .partial_cmp(&self.material_preference(
                            &b.name,
                            1.0 + extension_index as f64,
                            0.0,
                        ))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .ok_or("no resource candidate")?;
            materials.push(material);
        }

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

        // Build the cavity candidate from physical connection spacing rather
        // than an authored angular ring. Axial hex-lattice steps are generated
        // from the selected constituent extent; the graph closes naturally
        // around the interior region.
        let step = 2.0 * circumradius;
        let lattice_directions = [(1i32, 0i32), (0, 1), (-1, 1), (-1, 0), (0, -1), (1, -1)];
        let mut axial = (0i32, -2i32);
        let mut cycle_positions = Vec::with_capacity(cycle_count);
        for &(dq, dr) in &lattice_directions {
            for _ in 0..2 {
                let q = axial.0;
                let r = axial.1;
                cycle_positions.push((
                    step * (q as f64 + 0.5 * r as f64),
                    step * (3.0_f64.sqrt() * 0.5 * r as f64),
                ));
                axial.0 += dq;
                axial.1 += dr;
            }
        }
        if cycle_positions.len() != cycle_count {
            return Err("physical cavity candidate did not close its lattice search".into());
        }

        let mut elements = Vec::with_capacity(count);
        for (material_resource, (x, y)) in materials[..cycle_count]
            .iter()
            .zip(cycle_positions.iter().copied())
        {
            elements.push(BlueprintElement {
                material: Material::free_base(&material_resource.name, 1.0),
                placement: BlueprintPlacement {
                    x,
                    y,
                    rotation_radians: 0.0,
                },
            });
        }

        let first_radius = materials
            .first()
            .and_then(|resource| {
                resource.shape.form.polygon_vertices().map(|vertices| {
                    vertices
                        .into_iter()
                        .map(|(x, y)| x.hypot(y))
                        .fold(0.0, f64::max)
                })
            })
            .unwrap_or(0.0);
        if first_radius <= 0.0 {
            return Err("growth extension requires rigid polygonal geometry".into());
        }
        let first_position = cycle_positions
            .first()
            .copied()
            .ok_or("physical cavity candidate has no boundary anchor")?;
        let first_distance = first_position.0.hypot(first_position.1);
        if first_distance <= 0.0 {
            return Err("physical cavity boundary anchor is degenerate".into());
        }
        let outward = (
            first_position.0 / first_distance,
            first_position.1 / first_distance,
        );

        // Extend from the cavity boundary by physically sized constituents.
        // Each extension is admitted as a local contact candidate; no
        // size_preference-derived coordinate scale is used.
        let mut previous_center = first_position;
        let mut previous_radius = first_radius;
        for extension_index in 0..(count - cycle_count) {
            let extension_material = materials[cycle_count + extension_index];
            let extension_radius = extension_material
                .shape
                .form
                .polygon_vertices()
                .map(|vertices| {
                    vertices
                        .into_iter()
                        .map(|(x, y)| x.hypot(y))
                        .fold(0.0, f64::max)
                })
                .unwrap_or(0.0);
            if extension_radius <= 0.0 {
                return Err("growth extension requires rigid polygonal geometry".into());
            }
            let extension_distance = previous_radius + extension_radius;
            let extension_center = (
                previous_center.0 + outward.0 * extension_distance,
                previous_center.1 + outward.1 * extension_distance,
            );
            elements.push(BlueprintElement {
                material: Material::free_base(&extension_material.name, 1.0),
                placement: BlueprintPlacement {
                    x: extension_center.0,
                    y: extension_center.1,
                    rotation_radians: std::f64::consts::PI,
                },
            });
            previous_center = extension_center;
            previous_radius = extension_radius;
        }

        let mut connections = Vec::with_capacity(count);
        for i in 0..cycle_count {
            let next = (i + 1) % cycle_count;
            connections.push(BlueprintConnection {
                element_a: i.min(next),
                element_b: i.max(next),
            });
        }
        for extension_index in 0..(count - cycle_count) {
            let extension = cycle_count + extension_index;
            let parent = if extension_index == 0 {
                0
            } else {
                extension - 1
            };
            connections.push(BlueprintConnection {
                element_a: parent.min(extension),
                element_b: parent.max(extension),
            });
        }

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
