//! Self-contained geometry helpers for structural connection points.
//!
//! This module deliberately does not create bonds, determine bond strength,
//! or perform energy calculations. It only transforms the project's existing
//! `ConnectionPoint` representation into world space and evaluates geometry.

use crate::math::directional_compatibility;
use crate::resources::ConnectionPoint;

/// A connection point after applying a structural-unit placement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldConnectionPoint {
    pub x: f64,
    pub y: f64,
    pub normal_x: f64,
    pub normal_y: f64,
}

/// Transform an authored `ConnectionPoint` from unit-local space into
/// organism/world space.
///
/// `rotation_radians` rotates both the point and its outward-facing normal;
/// translation then places the unit in world space. The immutable catalog
/// connection point itself is never modified.
pub fn transform_connection_point(
    point: ConnectionPoint,
    origin_x: f64,
    origin_y: f64,
    rotation_radians: f64,
) -> WorldConnectionPoint {
    let (s, c) = rotation_radians.sin_cos();
    let (nx, ny) = (point.direction_radians.cos(), point.direction_radians.sin());

    WorldConnectionPoint {
        x: origin_x + point.x * c - point.y * s,
        y: origin_y + point.x * s + point.y * c,
        normal_x: nx * c - ny * s,
        normal_y: nx * s + ny * c,
    }
}

/// Euclidean distance between two world-space connection points.
pub fn point_distance(a: WorldConnectionPoint, b: WorldConnectionPoint) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

/// Compatibility of two surfaces facing one another.
///
/// Because connection-point directions are outward-facing normals, the second
/// normal is reversed before comparison. Directly facing surfaces therefore
/// score `1`, perpendicular surfaces `0`, and surfaces facing the same way
/// score `-1`.
pub fn facing_compatibility(a: WorldConnectionPoint, b: WorldConnectionPoint) -> f64 {
    directional_compatibility(a.normal_x, a.normal_y, -b.normal_x, -b.normal_y)
}

/// Whether two connection points are within a supplied geometric tolerance.
pub fn within_contact_tolerance(
    a: WorldConnectionPoint,
    b: WorldConnectionPoint,
    tolerance: f64,
) -> bool {
    point_distance(a, b) <= tolerance.max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::{FRAC_PI_2, PI};

    fn cp(x: f64, y: f64, direction_radians: f64) -> ConnectionPoint {
        ConnectionPoint { x, y, direction_radians }
    }

    #[test]
    fn transform_rotates_point_and_direction_and_applies_translation() {
        let result = transform_connection_point(cp(1.0, 0.0, 0.0), 10.0, 20.0, FRAC_PI_2);
        assert!((result.x - 10.0).abs() < 1e-12);
        assert!((result.y - 21.0).abs() < 1e-12);
        assert!(result.normal_x.abs() < 1e-12);
        assert!((result.normal_y - 1.0).abs() < 1e-12);
    }

    #[test]
    fn distance_is_euclidean() {
        let a = transform_connection_point(cp(0.0, 0.0, 0.0), 0.0, 0.0, 0.0);
        let b = transform_connection_point(cp(0.0, 0.0, 0.0), 3.0, 4.0, 0.0);
        assert!((point_distance(a, b) - 5.0).abs() < 1e-12);
    }

    #[test]
    fn directly_facing_normals_have_maximum_compatibility() {
        let a = transform_connection_point(cp(0.0, 0.0, 0.0), 0.0, 0.0, 0.0);
        let b = transform_connection_point(cp(0.0, 0.0, PI), 1.0, 0.0, 0.0);
        assert!((facing_compatibility(a, b) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn perpendicular_surfaces_have_zero_compatibility() {
        let a = transform_connection_point(cp(0.0, 0.0, 0.0), 0.0, 0.0, 0.0);
        let b = transform_connection_point(cp(0.0, 0.0, FRAC_PI_2), 1.0, 0.0, 0.0);
        assert!(facing_compatibility(a, b).abs() < 1e-12);
    }

    #[test]
    fn tolerance_is_respected_and_negative_tolerance_means_exact_contact_only() {
        let a = transform_connection_point(cp(0.0, 0.0, 0.0), 0.0, 0.0, 0.0);
        let b = transform_connection_point(cp(0.0, 0.0, 0.0), 1.0, 0.0, 0.0);
        assert!(within_contact_tolerance(a, b, 1.0));
        assert!(!within_contact_tolerance(a, b, 0.99));
        assert!(!within_contact_tolerance(a, b, -1.0));
    }
}
