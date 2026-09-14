//! Connection relationships over physical boundaries.
//! Continuous boundaries intentionally have no socket indices.
use crate::math::directional_compatibility;
use crate::resources::{ConnectionPoint, ConnectionSites};
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConnectionRegion { Corner(WorldConnectionPoint), Boundary { center_x: f64, center_y: f64, radius: f64 }, Fluid { center_x: f64, center_y: f64, effective_radius: f64 } }
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldConnectionPoint { pub x: f64, pub y: f64, pub normal_x: f64, pub normal_y: f64 }
pub fn transform_connection_point(point: ConnectionPoint, origin_x: f64, origin_y: f64, rotation_radians: f64) -> WorldConnectionPoint {
    let (s, c) = rotation_radians.sin_cos();
    WorldConnectionPoint { x: origin_x + point.x * c - point.y * s, y: origin_y + point.x * s + point.y * c, normal_x: 0.0, normal_y: 0.0 }
}
pub fn transform_connection_regions(sites: &ConnectionSites, origin_x: f64, origin_y: f64, rotation_radians: f64, fluid_effective_radius: f64) -> Vec<ConnectionRegion> {
    match sites {
        ConnectionSites::Corners(points) | ConnectionSites::Endpoints(points) => points.iter().copied().map(|p| ConnectionRegion::Corner(transform_connection_point(p, origin_x, origin_y, rotation_radians))).collect(),
        ConnectionSites::Circumference { radius } => vec![ConnectionRegion::Boundary { center_x: origin_x, center_y: origin_y, radius: radius.max(0.0) }],
        ConnectionSites::Undetermined => vec![ConnectionRegion::Fluid { center_x: origin_x, center_y: origin_y, effective_radius: fluid_effective_radius.max(0.0) }],
    }
}
pub fn point_distance(a: WorldConnectionPoint, b: WorldConnectionPoint) -> f64 { (a.x - b.x).hypot(a.y - b.y) }
/// A point feature has no unique normal. Returning maximum compatibility means
/// facing does not constrain a corner/endpoint; physical contact and overlap do.
pub fn facing_compatibility(a: WorldConnectionPoint, b: WorldConnectionPoint) -> f64 {
    let al = a.normal_x.hypot(a.normal_y);
    let bl = b.normal_x.hypot(b.normal_y);
    if al <= f64::EPSILON || bl <= f64::EPSILON { 1.0 } else { directional_compatibility(a.normal_x, a.normal_y, -b.normal_x, -b.normal_y) }
}
pub fn within_contact_tolerance(a: WorldConnectionPoint, b: WorldConnectionPoint, tolerance: f64) -> bool { point_distance(a, b) <= tolerance.max(0.0) }
impl ConnectionRegion {
    pub fn representative_point(self) -> Option<WorldConnectionPoint> { match self { Self::Corner(p) => Some(p), Self::Boundary { .. } | Self::Fluid { .. } => None } }
    pub fn center(self) -> (f64, f64) { match self { Self::Corner(p) => (p.x, p.y), Self::Boundary { center_x, center_y, .. } | Self::Fluid { center_x, center_y, .. } => (center_x, center_y) } }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn cp(x: f64, y: f64) -> ConnectionPoint { ConnectionPoint { x, y, direction_radians: 0.0 } }
    #[test]
    fn transform_rotates_only_the_physical_point() {
        let r = transform_connection_point(cp(1.0, 0.0), 10.0, 20.0, std::f64::consts::FRAC_PI_2);
        assert!((r.x - 10.0).abs() < 1e-12);
        assert!((r.y - 21.0).abs() < 1e-12);
        assert_eq!((r.normal_x, r.normal_y), (0.0, 0.0));
    }
    #[test]
    fn distance_is_euclidean() {
        let a = transform_connection_point(cp(0.0, 0.0), 0.0, 0.0, 0.0);
        let b = transform_connection_point(cp(0.0, 0.0), 3.0, 4.0, 0.0);
        assert!((point_distance(a, b) - 5.0).abs() < 1e-12)
    }
    #[test]
    fn point_features_have_no_invented_facing_normal() {
        let a = transform_connection_point(cp(0.0, 0.0), 0.0, 0.0, 0.0);
        assert_eq!(facing_compatibility(a, a), 1.0);
    }
    #[test]
    fn continuous_regions_have_no_fake_socket_identity() {
        let b = ConnectionRegion::Boundary { center_x: 1.0, center_y: 2.0, radius: 3.0 };
        let f = ConnectionRegion::Fluid { center_x: 4.0, center_y: 5.0, effective_radius: 6.0 };
        assert!(b.representative_point().is_none());
        assert!(f.representative_point().is_none());
        assert_eq!(b.center(), (1.0, 2.0));
        assert_eq!(f.center(), (4.0, 5.0));
    }
    #[test]
    fn continuous_sites_map_to_regions() {
        let b = transform_connection_regions(&ConnectionSites::Circumference { radius: 2.0 }, 3.0, 4.0, 0.0, 0.0);
        assert_eq!(b, vec![ConnectionRegion::Boundary { center_x: 3.0, center_y: 4.0, radius: 2.0 }]);
        let f = transform_connection_regions(&ConnectionSites::Undetermined, 3.0, 4.0, 0.0, 5.0);
        assert_eq!(f, vec![ConnectionRegion::Fluid { center_x: 3.0, center_y: 4.0, effective_radius: 5.0 }]);
        let e = transform_connection_regions(&ConnectionSites::Endpoints(vec![cp(-1.0, 0.0), cp(1.0, 0.0)]), 0.0, 0.0, 0.0, 0.0);
        assert_eq!(e.len(), 2);
    }
    #[test]
    fn negative_tolerance_does_not_create_contact() {
        let a = transform_connection_point(cp(0.0, 0.0), 0.0, 0.0, 0.0);
        let b = transform_connection_point(cp(0.0, 0.0), 1.0, 0.0, 0.0);
        assert!(!within_contact_tolerance(a, b, -1.0));
    }
}
