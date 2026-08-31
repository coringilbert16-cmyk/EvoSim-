//! Self-contained geometric helpers for structural connection evaluation.
//!
//! This module deliberately does not modify organism state, create bonds, or
//! decide bond strength. It only answers geometric questions using the
//! existing connection-point positions and instance placement.

use crate::math::directional_compatibility;

/// A connection point expressed in the local coordinates of its structural
/// unit, together with its outward-facing local normal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LocalConnectionPoint {
    pub x: f64,
    pub y: f64,
    pub normal_x: f64,
    pub normal_y: f64,
}

/// A connection point after transforming it into organism/world space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldConnectionPoint {
    pub x: f64,
    pub y: f64,
    pub normal_x: f64,
    pub normal_y: f64,
}

/// Rotate a local point and translate it according to an instance placement.
pub fn transform_point(
    point: LocalConnectionPoint,
    origin_x: f64,
    origin_y: f64,
    rotation_radians: f64,
) -> WorldConnectionPoint {
    let (s, c) = rotation_radians.sin_cos();
    WorldConnectionPoint {
        x: origin_x + point.x * c - point.y * s,
        y: origin_y + point.x * s + point.y * c,
        normal_x: point.normal_x * c - point.normal_y * s,
        normal_y: point.normal_x * s + point.normal_y * c,
    }
}

/// Euclidean distance between two world-space connection points.
pub fn point_distance(a: WorldConnectionPoint, b: WorldConnectionPoint) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

/// Compatibility of two facing surface normals.
///
/// The second normal is reversed because two surfaces facing one another
/// have opposing outward normals. Thus two directly facing surfaces score 1,
/// perpendicular surfaces score 0, and similarly-oriented outward normals
/// score -1.
pub fn facing_compatibility(a: WorldConnectionPoint, b: WorldConnectionPoint) -> f64 {
    directional_compatibility(a.normal_x, a.normal_y, -b.normal_x, -b.normal_y)
}

/// True when two connection points are within the supplied contact tolerance.
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

    fn p(x: f64, y: f64, nx: f64, ny: f64) -> LocalConnectionPoint {
        LocalConnectionPoint { x, y, normal_x: nx, normal_y: ny }
    }

    #[test]
    fn transform_applies_translation_and_rotation_to_point_and_normal() {
        let result = transform_point(p(1.0, 0.0, 1.0, 0.0), 10.0, 20.0, std::f64::consts::FRAC_PI_2);
        assert!((result.x - 10.0).abs() < 1e-12);
        assert!((result.y - 21.0).abs() < 1e-12);
        assert!(result.normal_x.abs() < 1e-12);
        assert!((result.normal_y - 1.0).abs() < 1e-12);
    }

    #[test]
    fn distance_is_euclidean() {
        let a = transform_point(p(0.0, 0.0, 1.0, 0.0), 0.0, 0.0, 0.0);
        let b = transform_point(p(0.0, 0.0, -1.0, 0.0), 3.0, 4.0, 0.0);
        assert!((point_distance(a, b) - 5.0).abs() < 1e-12);
    }

    #[test]
    fn_directly_facing_normals_have_maximum_compatibility() {
        let a = transform_point(p(0.0, 0.0, 1.0, 0.0), 0.0, 0.0, 0.0);
        let b = transform_point(p(0.0, 0.0, -1.0, 0.0), 1.0, 0.0, 0.0);
        assert!((facing_compatibility(a, b) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn perpendicular_surfaces_have_zero_facing_compatibility() {
        let a = transform_point(p(0.0, 0.0, 1.0, 0.0), 0.0, 0.0, 0.0);
        let b = transform_point(p(0.0, 0.0, 0.0, 1.0), 1.0, 0.0, 0.0);
        assert!(facing_compatibility(a, b).abs() < 1e-12);
    }

    #[test]
    fn tolerance_is_clamped_to_zero() {
        let a = transform_point(p(0.0, 0.0, 1.0, 0.0), 0.0, 0.0, 0.0);
        let b = transform_point(p(0.0, 0.0, -1.0, 0.0), 0.0, 0.0, 0.0);
        assert!(within_contact_tolerance(a, b, -1.0));
    }
}
