//! Rigid geometry helpers for realized physical connection points.
//! Continuous boundaries intentionally have no socket indices.
use crate::math::directional_compatibility;
use crate::resources::Shape;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldConnectionPoint {
    pub x: f64,
    pub y: f64,
    pub normal_x: f64,
    pub normal_y: f64,
}

/// Transform a point whose normal has already been derived from physical geometry.
///
/// This is the low-level rigid-body transform. It deliberately does not derive or
/// modify a normal: callers must supply the normal produced by the shape geometry.
pub fn transform_derived_point(
    x: f64,
    y: f64,
    normal_x: f64,
    normal_y: f64,
    origin_x: f64,
    origin_y: f64,
    rotation_radians: f64,
) -> WorldConnectionPoint {
    let (s, c) = rotation_radians.sin_cos();
    WorldConnectionPoint {
        x: origin_x + x * c - y * s,
        y: origin_y + x * s + y * c,
        normal_x: normal_x * c - normal_y * s,
        normal_y: normal_x * s + normal_y * c,
    }
}

/// Derive and transform a rigid polygon vertex using the actual shape boundary.
pub fn transform_polygon_vertex(
    shape: &Shape,
    vertex: usize,
    origin_x: f64,
    origin_y: f64,
    rotation_radians: f64,
) -> Option<WorldConnectionPoint> {
    let vertices = shape.form.polygon_vertices()?;
    let (x, y) = *vertices.get(vertex)?;
    let (normal_x, normal_y) = crate::rigid_boundary::corner_normal(shape, vertex)?;
    Some(transform_derived_point(
        x,
        y,
        normal_x,
        normal_y,
        origin_x,
        origin_y,
        rotation_radians,
    ))
}

/// Derive and transform a rigid line endpoint using the actual line geometry.
pub fn transform_line_endpoint(
    shape: &Shape,
    endpoint: usize,
    origin_x: f64,
    origin_y: f64,
    rotation_radians: f64,
) -> Option<WorldConnectionPoint> {
    let crate::resources::Form::Line { length } = shape.form else {
        return None;
    };
    let x = if endpoint == 0 {
        -length / 2.0
    } else if endpoint == 1 {
        length / 2.0
    } else {
        return None;
    };
    let (normal_x, normal_y) = crate::rigid_boundary::line_endpoint_normal(shape, endpoint)?;
    Some(transform_derived_point(
        x,
        0.0,
        normal_x,
        normal_y,
        origin_x,
        origin_y,
        rotation_radians,
    ))
}

/// Return the physical world-space connection point for a rigid boundary vertex.
///
/// This function is deliberately keyed by the realized shape and vertex index,
/// not by connection metadata. It is the preferred endpoint API for rigid shapes.
pub fn rigid_endpoint_world_point(
    shape: &Shape,
    vertex: usize,
    origin_x: f64,
    origin_y: f64,
    rotation_radians: f64,
) -> Option<WorldConnectionPoint> {
    if matches!(shape.form, crate::resources::Form::Line { .. }) {
        transform_line_endpoint(shape, vertex, origin_x, origin_y, rotation_radians)
    } else {
        transform_polygon_vertex(shape, vertex, origin_x, origin_y, rotation_radians)
    }
}

/// Geometry-derived facing for two rigid endpoints.
///
/// The normals come exclusively from the realized shape boundaries. No authored
/// connection direction, bond angle, socket capacity, or radial approximation is
/// consulted here.
pub fn rigid_endpoint_facing(
    shape_a: &Shape,
    vertex_a: usize,
    origin_a_x: f64,
    origin_a_y: f64,
    rotation_a: f64,
    shape_b: &Shape,
    vertex_b: usize,
    origin_b_x: f64,
    origin_b_y: f64,
    rotation_b: f64,
) -> Option<f64> {
    let a = rigid_endpoint_world_point(shape_a, vertex_a, origin_a_x, origin_a_y, rotation_a)?;
    let b = rigid_endpoint_world_point(shape_b, vertex_b, origin_b_x, origin_b_y, rotation_b)?;
    Some(facing_compatibility(a, b))
}

pub fn point_distance(a: WorldConnectionPoint, b: WorldConnectionPoint) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

pub fn facing_compatibility(a: WorldConnectionPoint, b: WorldConnectionPoint) -> f64 {
    directional_compatibility(a.normal_x, a.normal_y, -b.normal_x, -b.normal_y)
}

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
    use crate::resources::{Form, Shape};
    use std::f64::consts::FRAC_PI_2;

    fn square() -> Shape {
        Shape {
            form: Form::Rectangle {
                width: 2.0,
                height: 2.0,
            },
        }
    }

    #[test]
    fn derived_polygon_transform_uses_incident_boundary_geometry() {
        let r = transform_polygon_vertex(&square(), 0, 10.0, 20.0, 0.0).unwrap();
        assert!((r.x - (-1.0 + 10.0)).abs() < 1e-12);
        assert!((r.y - (-1.0 + 20.0)).abs() < 1e-12);
        assert!((r.normal_x + 2.0_f64.sqrt() / 2.0).abs() < 1e-12);
        assert!((r.normal_y + 2.0_f64.sqrt() / 2.0).abs() < 1e-12);
    }

    #[test]
    fn derived_polygon_transform_applies_only_rigid_rotation_and_translation() {
        let r = transform_polygon_vertex(&square(), 0, 10.0, 20.0, FRAC_PI_2).unwrap();
        assert!((r.x - 11.0).abs() < 1e-12);
        assert!((r.y - 19.0).abs() < 1e-12);
        assert!((r.normal_x - 2.0_f64.sqrt() / 2.0).abs() < 1e-12);
        assert!((r.normal_y + 2.0_f64.sqrt() / 2.0).abs() < 1e-12);
    }

    #[test]
    fn derived_line_transform_uses_physical_endpoint_direction() {
        let shape = Shape {
            form: Form::Line { length: 2.0 },
        };
        let r = transform_line_endpoint(&shape, 1, 0.0, 0.0, 0.0).unwrap();
        assert_eq!((r.x, r.y), (1.0, 0.0));
        assert_eq!((r.normal_x, r.normal_y), (1.0, 0.0));
    }

    #[test]
    fn rigid_endpoint_facing_uses_shape_geometry() {
        let a = square();
        let b = square();
        let facing = rigid_endpoint_facing(&a, 0, 0.0, 0.0, 0.0, &b, 2, 2.0, 0.0, 0.0).unwrap();
        assert!((facing - 1.0).abs() < 1e-12);
    }

    #[test]
    fn negative_tolerance_does_not_create_contact() {
        let a = WorldConnectionPoint {
            x: 0.0,
            y: 0.0,
            normal_x: 1.0,
            normal_y: 0.0,
        };
        let b = WorldConnectionPoint {
            x: 1.0,
            y: 0.0,
            normal_x: -1.0,
            normal_y: 0.0,
        };
        assert!(!within_contact_tolerance(a, b, -1.0));
    }
}
