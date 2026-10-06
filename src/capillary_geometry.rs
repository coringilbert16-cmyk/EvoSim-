//! Capillary equilibrium geometry for deformable fluids.
//!
//! This module uses the zero-gravity Young–Laplace model rather than sampling
//! placements of a circular fluid primitive. In 2-D, a constant-pressure,
//! constant-surface-tension free interface has constant curvature, so the free
//! boundary is a circular arc. A solid/fluid contact is constrained by the
//! Young contact angle.
//!
//! The solver here deliberately handles the first exact case: a finite amount
//! of fluid resting against a locally flat rigid boundary. It returns the
//! equilibrium arc analytically. More complicated rigid boundaries can be
//! assembled from the same contact conditions once their active boundary
//! features are known.
//!
//! No arbitrary position or angle sampling is used.

use std::f64::consts::{PI, TAU};

const EPS: f64 = 1e-12;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CapillaryContactFamily {
    /// Conserved 2-D fluid area.
    pub area: f64,
    /// Interior contact angle, in radians, in (0, pi).
    pub contact_angle_radians: f64,
    /// Radius of the constant-curvature free interface.
    pub curvature_radius: f64,
    /// Central angle swept by the free interface.
    pub free_arc_angle_radians: f64,
    /// Length of the solid/fluid contact interval.
    pub contact_length: f64,
    /// Area enclosed by the solid contact segment and free arc.
    pub enclosed_area: f64,
}

impl CapillaryContactFamily {
    /// Solve the zero-gravity 2-D Young–Laplace equilibrium for a fluid
    /// contacting a flat solid boundary.
    ///
    /// For contact angle theta, the free arc subtends
    /// alpha = 2(pi - theta). Its circular-segment area is
    ///
    ///     A = R^2 / 2 * (alpha - sin(alpha)).
    ///
    /// Thus R follows directly from conserved area. No geometric search is
    /// required.
    pub fn solve(area: f64, contact_angle_radians: f64) -> Option<Self> {
        if !area.is_finite()
            || area <= 0.0
            || !contact_angle_radians.is_finite()
            || !(0.0 < contact_angle_radians && contact_angle_radians < PI)
        {
            return None;
        }

        let alpha = 2.0 * (PI - contact_angle_radians);
        let shape_factor = 0.5 * (alpha - alpha.sin());
        if !shape_factor.is_finite() || shape_factor <= EPS {
            return None;
        }

        let radius = (area / shape_factor).sqrt();
        let contact_length = 2.0 * radius * (alpha * 0.5).sin();

        Some(Self {
            area,
            contact_angle_radians,
            curvature_radius: radius,
            free_arc_angle_radians: alpha,
            contact_length,
            enclosed_area: shape_factor * radius * radius,
        })
    }

    /// Half-width of the symmetric contact interval.
    pub fn half_contact_length(&self) -> f64 {
        self.contact_length * 0.5
    }

    /// The curvature magnitude kappa = 1/R for the circular free interface.
    pub fn curvature(&self) -> f64 {
        1.0 / self.curvature_radius
    }

    /// Pressure jump for a supplied surface tension gamma:
    ///
    ///     Delta P = gamma * kappa
    ///
    /// Surface tension is intentionally supplied by the caller because it is
    /// an interfacial property, not a universal property of a resource alone.
    pub fn pressure_jump(&self, surface_tension: f64) -> Option<f64> {
        if !surface_tension.is_finite() || surface_tension < 0.0 {
            return None;
        }
        Some(surface_tension * self.curvature())
    }
}

/// A finite interval along a rigid edge at which the symmetric capillary
/// solution may translate without changing its internal equilibrium.
///
/// This is the finite representation of the continuous placement family.
/// The interval is not sampled: its endpoints are part of the geometry
/// constraint and a later rigid-boundary solver can intersect it with the
/// actual exposed edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactTranslationInterval {
    pub edge_start_parameter: f64,
    pub edge_end_parameter: f64,
}

impl ContactTranslationInterval {
    pub fn from_edge_length(edge_length: f64, family: CapillaryContactFamily) -> Option<Self> {
        if !edge_length.is_finite() || edge_length < family.contact_length - EPS {
            return None;
        }

        let half = family.contact_length * 0.5;
        Some(Self {
            edge_start_parameter: half,
            edge_end_parameter: edge_length - half,
        })
    }

    pub fn length(&self) -> f64 {
        (self.edge_end_parameter - self.edge_start_parameter).max(0.0)
    }
}

/// Convert the existing scalar cohesion values into an explicit *effective*
/// contact angle for the capillary model.
///
/// This is deliberately an adapter, not a claim that cohesion alone is the
/// full thermodynamic Young-equation input. Young's equation depends on
/// interfacial energies. Until EvoSim has explicit interface energies, the
/// existing cohesion contrast provides a deterministic provisional wetting
/// parameter without adding another evolved property.
///
/// Low fluid cohesion relative to the contacted solid produces stronger
/// wetting (smaller angle); equal cohesion gives 90 degrees.
pub fn effective_contact_angle(fluid_cohesion: f64, solid_cohesion: f64) -> Option<f64> {
    if !fluid_cohesion.is_finite() || !solid_cohesion.is_finite() {
        return None;
    }

    let contrast = (solid_cohesion - fluid_cohesion).clamp(-1.0, 1.0);
    Some((PI * 0.5 - contrast * PI * 0.25).clamp(EPS, PI - EPS))
}

/// Exact capillary solution for the current EvoSim water/solid pair.
///
/// The amount of water is represented by conserved area. The contacted
/// resource supplies the effective wetting parameter through the existing
/// cohesion field.
pub fn solve_water_against_solid(
    water_area: f64,
    water_cohesion: f64,
    solid_cohesion: f64,
) -> Option<CapillaryContactFamily> {
    let angle = effective_contact_angle(water_cohesion, solid_cohesion)?;
    CapillaryContactFamily::solve(water_area, angle)
}

/// Normalize an arc parameter to [0, 2pi).
pub fn normalize_arc_parameter(angle: f64) -> Option<f64> {
    if !angle.is_finite() {
        return None;
    }
    Some(angle.rem_euclid(TAU))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semicircular_solution_is_exact_at_ninety_degrees() {
        let family = CapillaryContactFamily::solve(0.5, PI * 0.5).unwrap();
        assert!((family.free_arc_angle_radians - PI).abs() < 1e-12);
        assert!((family.curvature_radius - (0.5 / (PI * 0.5)).sqrt()).abs() < 1e-12);
        assert!((family.enclosed_area - 0.5).abs() < 1e-12);
        assert!((family.contact_length - 2.0 * family.curvature_radius).abs() < 1e-12);
    }

    #[test]
    fn conserved_area_survives_the_equilibrium_solution() {
        for angle in [PI / 6.0, PI / 3.0, PI / 2.0, 2.0 * PI / 3.0, 5.0 * PI / 6.0] {
            let family = CapillaryContactFamily::solve(0.5, angle).unwrap();
            assert!((family.enclosed_area - family.area).abs() < 1e-10);
            assert!(family.curvature_radius.is_finite());
            assert!(family.contact_length.is_finite());
        }
    }

    #[test]
    fn impossible_contact_interval_is_rejected_without_search() {
        let family = CapillaryContactFamily::solve(0.5, PI / 2.0).unwrap();
        assert!(ContactTranslationInterval::from_edge_length(
            family.contact_length - 1e-6,
            family
        )
        .is_none());
    }

    #[test]
    fn sufficient_edge_yields_a_continuous_translation_interval() {
        let family = CapillaryContactFamily::solve(0.5, PI / 2.0).unwrap();
        let interval =
            ContactTranslationInterval::from_edge_length(family.contact_length + 2.0, family)
                .unwrap();
        assert!((interval.length() - 2.0).abs() < 1e-10);
    }

    #[test]
    fn water_cohesion_adapter_is_deterministic() {
        let angle = effective_contact_angle(0.0, 0.95).unwrap();
        assert!(angle < PI * 0.5);
        assert!((effective_contact_angle(0.5, 0.5).unwrap() - PI * 0.5).abs() < 1e-12);
    }

    #[test]
    fn water_against_carbon_has_a_finite_exact_solution() {
        let family = solve_water_against_solid(0.5, 0.0, 0.95).unwrap();
        assert!(family.curvature_radius.is_finite());
        assert!((family.enclosed_area - 0.5).abs() < 1e-10);
    }

    #[test]
    fn invalid_angles_and_areas_are_rejected() {
        assert!(CapillaryContactFamily::solve(0.0, PI / 2.0).is_none());
        assert!(CapillaryContactFamily::solve(1.0, 0.0).is_none());
        assert!(CapillaryContactFamily::solve(1.0, PI).is_none());
        assert!(CapillaryContactFamily::solve(f64::NAN, PI / 2.0).is_none());
    }
}
