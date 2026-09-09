//! Geometry helpers and physical connection-region representation.

use crate::math::directional_compatibility;
use crate::resources::ConnectionPoint;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConnectionRegion {
    Corner(WorldConnectionPoint),
    Boundary { center_x: f64, center_y: f64, radius: f64 },
    Fluid { center_x: f64, center_y: f64, effective_radius: f64 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldConnectionPoint {
    pub x: f64,
    pub y: f64,
    pub normal_x: f64,
    pub normal_y: f64,
}

pub fn transform_connection_point(point: ConnectionPoint, origin_x: f64, origin_y: f64, rotation_radians: f64) -> WorldConnectionPoint {
    let (s, c) = rotation_radians.sin_cos();
    let (nx, ny) = (point.direction_radians.cos(), point.direction_radians.sin());
    WorldConnectionPoint { x: origin_x + point.x * c - point.y * s, y: origin_y + point.x * s + point.y * c, normal_x: nx * c - ny * s, normal_y: nx * s + ny * c }
}

pub fn point_distance(a: WorldConnectionPoint, b: WorldConnectionPoint) -> f64 { (a.x - b.x).hypot(a.y - b.y) }

pub fn facing_compatibility(a: WorldConnectionPoint, b: WorldConnectionPoint) -> f64 { directional_compatibility(a.normal_x, a.normal_y, -b.normal_x, -b.normal_y) }

pub fn within_contact_tolerance(a: WorldConnectionPoint, b: WorldConnectionPoint, tolerance: f64) -> bool { point_distance(a, b) <= tolerance.max(0.0) }

impl ConnectionRegion {
    pub fn representative_point(self) -> Option<WorldConnectionPoint> {
        match self { Self::Corner(point) => Some(point), Self::Boundary { .. } | Self::Fluid { .. } => None }
    }
    pub fn center(self) -> (f64, f64) {
        match self { Self::Corner(point) => (point.x, point.y), Self::Boundary { center_x, center_y, .. } | Self::Fluid { center_x, center_y, .. } => (center_x, center_y) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::{FRAC_PI_2, PI};
    fn cp(x: f64, y: f64, direction_radians: f64) -> ConnectionPoint { ConnectionPoint { x, y, direction_radians } }
    #[test] fn transform_rotates_point_and_direction_and_applies_translation() { let result = transform_connection_point(cp(1.0, 0.0, 0.0), 10.0, 20.0, FRAC_PI_2); assert!((result.x - 10.0).abs() < 1e-12); assert!((result.y - 21.0).abs() < 1e-12); assert!(result.normal_x.abs() < 1e-12); assert!((result.normal_y - 1.0).abs() < 1e-12); }
    #[test] fn distance_is_euclidean() { let a = transform_connection_point(cp(0.0, 0.0, 0.0), 0.0, 0.0, 0.0); let b = transform_connection_point(cp(0.0, 0.0, 0.0), 3.0, 4.0, 0.0); assert!((point_distance(a, b) - 5.0).abs() < 1e-12); }
    #[test] fn directly_facing_normals_have_maximum_compatibility() { let a = transform_connection_point(cp(0.0, 0.0, 0.0), 0.0, 0.0, 0.0); let b = transform_connection_point(cp(0.0, 0.0, PI), 1.0, 0.0, 0.0); assert!((facing_compatibility(a, b) - 1.0).abs() < 1e-12); }
    #[test] fn perpendicular_surfaces_have_zero_compatibility() { let a = transform_connection_point(cp(0.0, 0.0, 0.0), 0.0, 0.0, 0.0); let b = transform_connection_point(cp(0.0, 0.0, FRAC_PI_2), 1.0, 0.0, 0.0); assert!(facing_compatibility(a, b).abs() < 1e-12); }
    #[test] fn continuous_regions_have_no_fake_socket_identity() { let boundary = ConnectionRegion::Boundary { center_x: 1.0, center_y: 2.0, radius: 3.0 }; let fluid = ConnectionRegion::Fluid { center_x: 4.0, center_y: 5.0, effective_radius: 6.0 }; assert!(boundary.representative_point().is_none()); assert!(fluid.representative_point().is_none()); assert_eq!(boundary.center(), (1.0, 2.0)); assert_eq!(fluid.center(), (4.0, 5.0)); }
    #[test] fn negative_tolerance_means_exact_contact_only() { let a = transform_connection_point(cp(0.0, 0.0, 0.0), 0.0, 0.0, 0.0); let b = transform_connection_point(cp(0.0, 0.0, 0.0), 1.0, 0.0, 0.0); assert!(!within_contact_tolerance(a, b, -1.0)); }
}
