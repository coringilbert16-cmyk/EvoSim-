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

/// Exact circular free-boundary descriptor in a constituent-local coordinate frame.
///
/// The fluid region represented here is the circular segment bounded by the
/// arc and its endpoint chord. The chord is the solid-contact interval; it is
/// not a rigid bond and must not be exposed as a fluid-fluid bond. Angles are
/// in radians, and the sweep follows the positive angular direction from the
/// start angle. This type is deliberately separate from the legacy polygon
/// boundary so consumers cannot accidentally mistake an arc for sampled edges.
#[allow(dead_code)] // Staged until exact arc queries are wired into physical geometry.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct CircularArcBoundary {
    pub center_x: f64,
    pub center_y: f64,
    pub radius: f64,
    pub start_angle_radians: f64,
    pub sweep_radians: f64,
}

#[allow(dead_code)] // Keep the exact primitive available to the integration sprint.
impl CircularArcBoundary {
    /// Validate the descriptor and its finite, non-degenerate circular segment.
    pub fn is_valid(&self) -> bool {
        self.center_x.is_finite()
            && self.center_y.is_finite()
            && self.radius.is_finite()
            && self.radius > 0.0
            && self.start_angle_radians.is_finite()
            && self.sweep_radians.is_finite()
            && self.sweep_radians > 0.0
            && self.sweep_radians < TAU
            && self
                .segment_area()
                .is_some_and(|area| area.is_finite() && area > EPS)
    }

    /// Exact arc start point in the descriptor's local frame.
    pub fn start_point(&self) -> (f64, f64) {
        (
            self.center_x + self.radius * self.start_angle_radians.cos(),
            self.center_y + self.radius * self.start_angle_radians.sin(),
        )
    }

    /// Exact arc end point in the descriptor's local frame.
    pub fn end_point(&self) -> (f64, f64) {
        let angle = self.start_angle_radians + self.sweep_radians;
        (
            self.center_x + self.radius * angle.cos(),
            self.center_y + self.radius * angle.sin(),
        )
    }

    /// Exact free-interface length, without polygonal sampling.
    pub fn arc_length(&self) -> Option<f64> {
        self.is_finite_arc()
            .then_some(self.radius * self.sweep_radians)
    }

    /// Exact chord/contact length joining the arc endpoints.
    pub fn chord_length(&self) -> Option<f64> {
        self.is_finite_arc()
            .then_some(2.0 * self.radius * (self.sweep_radians * 0.5).sin())
    }

    /// Exact area of the circular segment closed by the endpoint chord.
    pub fn segment_area(&self) -> Option<f64> {
        if !self.center_x.is_finite()
            || !self.center_y.is_finite()
            || !self.radius.is_finite()
            || self.radius <= 0.0
            || !self.start_angle_radians.is_finite()
            || !self.sweep_radians.is_finite()
            || self.sweep_radians <= 0.0
            || self.sweep_radians >= TAU
        {
            return None;
        }
        let area =
            0.5 * self.radius * self.radius * (self.sweep_radians - self.sweep_radians.sin());
        (area.is_finite() && area > 0.0).then_some(area)
    }

    /// Rigidly transform this local descriptor into another coordinate frame.
    /// Translation and rotation preserve radius, sweep, chord length, and area.
    pub fn transformed(
        &self,
        translate_x: f64,
        translate_y: f64,
        rotation_radians: f64,
    ) -> Option<Self> {
        if !self.is_valid()
            || !translate_x.is_finite()
            || !translate_y.is_finite()
            || !rotation_radians.is_finite()
        {
            return None;
        }
        let (sin, cos) = rotation_radians.sin_cos();
        Some(Self {
            center_x: translate_x + self.center_x * cos - self.center_y * sin,
            center_y: translate_y + self.center_x * sin + self.center_y * cos,
            radius: self.radius,
            start_angle_radians: (self.start_angle_radians + rotation_radians).rem_euclid(TAU),
            sweep_radians: self.sweep_radians,
        })
    }

    /// Whether an angle lies on this arc's positive sweep.
    pub fn contains_angle(&self, angle_radians: f64) -> bool {
        if !self.is_finite_arc() || !angle_radians.is_finite() {
            return false;
        }
        let offset = (angle_radians - self.start_angle_radians).rem_euclid(TAU);
        offset <= self.sweep_radians + EPS
    }

    /// Exact nearest point on the finite arc to a query point.
    ///
    /// If the radial projection falls outside the arc sweep, the nearer
    /// endpoint is returned. No tessellation or vertex search is involved.
    pub fn closest_point(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        if !self.is_finite_arc() || !x.is_finite() || !y.is_finite() {
            return None;
        }
        let dx = x - self.center_x;
        let dy = y - self.center_y;
        if dx.hypot(dy) > f64::EPSILON {
            let angle = dy.atan2(dx);
            if self.contains_angle(angle) {
                let (sin, cos) = angle.sin_cos();
                return Some((
                    self.center_x + self.radius * cos,
                    self.center_y + self.radius * sin,
                ));
            }
        }
        let start = self.start_point();
        let end = self.end_point();
        let start_distance = (x - start.0).hypot(y - start.1);
        let end_distance = (x - end.0).hypot(y - end.1);
        Some(if start_distance <= end_distance {
            start
        } else {
            end
        })
    }

    /// Exact first non-negative ray intersection with this finite arc.
    ///
    /// Direction need not be normalized. The result is (ray_parameter, x, y);
    /// a circle intersection outside the recorded sweep is not a boundary hit.
    pub fn ray_intersection(
        &self,
        origin_x: f64,
        origin_y: f64,
        direction_x: f64,
        direction_y: f64,
    ) -> Option<(f64, f64, f64)> {
        if !self.is_finite_arc()
            || !origin_x.is_finite()
            || !origin_y.is_finite()
            || !direction_x.is_finite()
            || !direction_y.is_finite()
        {
            return None;
        }
        let a = direction_x * direction_x + direction_y * direction_y;
        if a <= f64::EPSILON {
            return None;
        }
        let ox = origin_x - self.center_x;
        let oy = origin_y - self.center_y;
        let b = 2.0 * (ox * direction_x + oy * direction_y);
        let c = ox * ox + oy * oy - self.radius * self.radius;
        let discriminant = b * b - 4.0 * a * c;
        if !discriminant.is_finite() || discriminant < -EPS {
            return None;
        }
        let root = discriminant.max(0.0).sqrt();
        let mut candidates = [(-b - root) / (2.0 * a), (-b + root) / (2.0 * a)];
        candidates
            .sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
        for t in candidates {
            if t < -EPS {
                continue;
            }
            let t = t.max(0.0);
            let x = origin_x + t * direction_x;
            let y = origin_y + t * direction_y;
            let angle = (y - self.center_y).atan2(x - self.center_x);
            if self.contains_angle(angle) {
                return Some((t, x, y));
            }
        }
        None
    }

    /// Exact intersections between this finite circular arc and a segment.
    ///
    /// Returns zero, one, or two distinct points. Segment endpoints are included;
    /// intersections on the circle but outside this arc's angular sweep are not.
    pub fn segment_intersections(
        &self,
        ax: f64,
        ay: f64,
        bx: f64,
        by: f64,
    ) -> Option<Vec<(f64, f64)>> {
        if !self.is_finite_arc()
            || !ax.is_finite()
            || !ay.is_finite()
            || !bx.is_finite()
            || !by.is_finite()
        {
            return None;
        }
        let dx = bx - ax;
        let dy = by - ay;
        let ox = ax - self.center_x;
        let oy = ay - self.center_y;
        let a = dx * dx + dy * dy;
        if a <= f64::EPSILON {
            let radial_error = ox.hypot(oy) - self.radius;
            let angle = oy.atan2(ox);
            return Some(if radial_error.abs() <= EPS && self.contains_angle(angle) {
                vec![(ax, ay)]
            } else {
                Vec::new()
            });
        }
        let b = 2.0 * (ox * dx + oy * dy);
        let c = ox * ox + oy * oy - self.radius * self.radius;
        let discriminant = b * b - 4.0 * a * c;
        if !discriminant.is_finite() || discriminant < -EPS {
            return Some(Vec::new());
        }
        let root = discriminant.max(0.0).sqrt();
        let mut parameters = [(-b - root) / (2.0 * a), (-b + root) / (2.0 * a)];
        parameters
            .sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
        let mut intersections = Vec::with_capacity(2);
        for t in parameters {
            if !(-EPS..=1.0 + EPS).contains(&t) {
                continue;
            }
            let t = t.clamp(0.0, 1.0);
            let x = ax + t * dx;
            let y = ay + t * dy;
            let angle = (y - self.center_y).atan2(x - self.center_x);
            if self.contains_angle(angle)
                && intersections
                    .last()
                    .is_none_or(|(px, py): &(f64, f64)| (x - px).hypot(y - py) > EPS)
            {
                intersections.push((x, y));
            }
        }
        Some(intersections)
    }

    /// Exact outward normal at a point identified by arc parameter angle.
    pub fn normal_at_angle(&self, angle_radians: f64) -> Option<(f64, f64)> {
        if !self.is_finite_arc() || !angle_radians.is_finite() {
            return None;
        }
        Some((angle_radians.cos(), angle_radians.sin()))
    }

    fn is_finite_arc(&self) -> bool {
        self.center_x.is_finite()
            && self.center_y.is_finite()
            && self.radius.is_finite()
            && self.radius > 0.0
            && self.start_angle_radians.is_finite()
            && self.sweep_radians.is_finite()
            && self.sweep_radians > 0.0
            && self.sweep_radians < TAU
    }
}

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
        if !edge_length.is_finite() || edge_length <= EPS {
            return None;
        }
        if edge_length < family.contact_length - EPS {
            return None;
        }

        // Exposed-edge intervals elsewhere in the geometry library use the
        // normalized edge parameter t in [0, 1]. Keep this representation in
        // the same coordinate system; returning physical lengths here would
        // silently mix units when intersecting with an exposed interval.
        let half_fraction = (family.contact_length * 0.5) / edge_length;
        Some(Self {
            edge_start_parameter: half_fraction.clamp(0.0, 1.0),
            edge_end_parameter: (1.0 - half_fraction).clamp(0.0, 1.0),
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
    fn circular_arc_descriptor_preserves_exact_segment_area_and_contact_length() {
        let family = CapillaryContactFamily::solve(0.5, PI * 0.5).unwrap();
        let arc = CircularArcBoundary {
            center_x: 0.0,
            center_y: -family.curvature_radius,
            radius: family.curvature_radius,
            start_angle_radians: 0.0,
            sweep_radians: family.free_arc_angle_radians,
        };
        assert!(arc.is_valid());
        assert!((arc.segment_area().unwrap() - family.area).abs() < 1e-12);
        assert!((arc.chord_length().unwrap() - family.contact_length).abs() < 1e-12);
        assert!((arc.arc_length().unwrap() - family.curvature_radius * PI).abs() < 1e-12);
        assert!((arc.start_point().0 - family.curvature_radius).abs() < 1e-12);
        assert!((arc.end_point().0 + family.curvature_radius).abs() < 1e-12);
    }

    #[test]
    fn circular_arc_rigid_transform_preserves_area_and_rebases_endpoints() {
        let arc = CircularArcBoundary {
            center_x: 1.0,
            center_y: 2.0,
            radius: 3.0,
            start_angle_radians: 0.0,
            sweep_radians: PI,
        };
        let moved = arc.transformed(10.0, -4.0, PI * 0.5).unwrap();
        assert!((moved.center_x - 8.0).abs() < 1e-12);
        assert!((moved.center_y + 3.0).abs() < 1e-12);
        assert!((moved.segment_area().unwrap() - arc.segment_area().unwrap()).abs() < 1e-12);
        assert!((moved.chord_length().unwrap() - arc.chord_length().unwrap()).abs() < 1e-12);
        assert!((moved.start_point().0 - 8.0).abs() < 1e-12);
        assert!(moved.start_point().1.abs() < 1e-12);
    }

    #[test]
    fn circular_arc_exact_ray_and_closest_point_queries_respect_sweep() {
        let arc = CircularArcBoundary {
            center_x: 0.0,
            center_y: -1.0,
            radius: 1.0,
            start_angle_radians: 0.0,
            sweep_radians: PI,
        };
        let hit = arc.ray_intersection(-2.0, 0.0, 1.0, 0.0).unwrap();
        assert!((hit.0 - 2.0).abs() < 1e-12);
        assert!(hit.1.abs() < 1e-12);
        assert!(hit.2.abs() < 1e-12);
        let closest = arc.closest_point(0.0, 2.0).unwrap();
        assert!(closest.0.abs() < 1e-12);
        assert!(closest.1.abs() < 1e-12);
        assert!(arc.ray_intersection(-2.0, -2.0, 1.0, 0.0).is_none());
    }

    #[test]
    fn circular_arc_segment_intersections_are_exact_and_sweep_limited() {
        let arc = CircularArcBoundary {
            center_x: 0.0,
            center_y: 0.0,
            radius: 1.0,
            start_angle_radians: 0.0,
            sweep_radians: PI,
        };
        let hits = arc.segment_intersections(-2.0, 0.0, 2.0, 0.0).unwrap();
        assert_eq!(hits.len(), 2);
        assert!((hits[0].0 + 1.0).abs() < 1e-12);
        assert!(hits[0].1.abs() < 1e-12);
        assert!((hits[1].0 - 1.0).abs() < 1e-12);
        assert!(hits[1].1.abs() < 1e-12);

        let tangent = arc.segment_intersections(-2.0, 1.0, 2.0, 1.0).unwrap();
        assert_eq!(tangent.len(), 1);
        assert!(tangent[0].0.abs() < 1e-12);
        assert!((tangent[0].1 - 1.0).abs() < 1e-12);

        let outside_sweep = CircularArcBoundary {
            start_angle_radians: PI,
            sweep_radians: PI * 0.5,
            ..arc
        };
        assert!(outside_sweep
            .segment_intersections(-2.0, 0.0, 2.0, 0.0)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn circular_arc_descriptor_rejects_invalid_or_degenerate_parameters() {
        let mut arc = CircularArcBoundary {
            center_x: 0.0,
            center_y: 0.0,
            radius: 1.0,
            start_angle_radians: 0.0,
            sweep_radians: PI,
        };
        assert!(arc.is_valid());
        arc.radius = f64::NAN;
        assert!(!arc.is_valid());
        arc.radius = 1.0;
        arc.sweep_radians = TAU;
        assert!(!arc.is_valid());
        arc.sweep_radians = 0.0;
        assert!(arc.segment_area().is_none());
    }

    #[test]
    fn circular_arc_normal_is_radial_and_exact() {
        let arc = CircularArcBoundary {
            center_x: 2.0,
            center_y: -3.0,
            radius: 4.0,
            start_angle_radians: 0.0,
            sweep_radians: PI,
        };
        let normal = arc.normal_at_angle(PI / 3.0).unwrap();
        assert!((normal.0 - 0.5).abs() < 1e-12);
        assert!((normal.1 - 3.0_f64.sqrt() / 2.0).abs() < 1e-12);
    }

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
        assert!(
            ContactTranslationInterval::from_edge_length(family.contact_length - 1e-6, family)
                .is_none()
        );
    }

    #[test]
    fn sufficient_edge_yields_a_continuous_translation_interval() {
        let family = CapillaryContactFamily::solve(0.5, PI / 2.0).unwrap();
        let interval =
            ContactTranslationInterval::from_edge_length(family.contact_length + 2.0, family)
                .unwrap();
        assert!((interval.length() - (2.0 / (family.contact_length + 2.0))).abs() < 1e-10);
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
