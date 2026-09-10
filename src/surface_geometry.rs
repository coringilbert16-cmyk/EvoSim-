//! Continuous physical-boundary queries.
//!
//! This module deliberately works from the currently realized `Form` rather
//! than from resource-level connection sockets. A boundary query returns a
//! physical location and outward normal; it does not allocate a numbered
//! connection slot. This is the seam used later by deformable and fluid
//! constituents.

use crate::resources::{Form, Shape};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundaryPoint {
    pub x: f64,
    pub y: f64,
    pub normal_x: f64,
    pub normal_y: f64,
}

fn normalized(x: f64, y: f64) -> Option<(f64, f64)> {
    let length = x.hypot(y);
    if !length.is_finite() || length <= f64::EPSILON {
        None
    } else {
        Some((x / length, y / length))
    }
}

fn polygon_boundary_toward(vertices: &[(f64, f64)], target_x: f64, target_y: f64) -> Option<BoundaryPoint> {
    if vertices.len() < 3 {
        return None;
    }

    let (ux, uy) = normalized(target_x, target_y)?;
    let mut best: Option<(f64, BoundaryPoint)> = None;

    for index in 0..vertices.len() {
        let (ax, ay) = vertices[index];
        let (bx, by) = vertices[(index + 1) % vertices.len()];
        let ex = bx - ax;
        let ey = by - ay;
        let denominator = ux * ey - uy * ex;
        if denominator.abs() <= 1e-12 {
            continue;
        }

        // Center-to-boundary ray: t * u = a + s * (b-a).
        let t = (ax * ey - ay * ex) / denominator;
        let s = (ax * uy - ay * ux) / denominator;
        if t <= 1e-12 || !(-1e-10..=1.0000000001).contains(&s) {
            continue;
        }

        let edge_length = ex.hypot(ey);
        if edge_length <= f64::EPSILON {
            continue;
        }

        // The catalog's polygon vertices are authored counter-clockwise.
        // The outward normal of an edge is therefore its right-hand normal.
        let nx = ey / edge_length;
        let ny = -ex / edge_length;
        let point = BoundaryPoint {
            x: t * ux,
            y: t * uy,
            normal_x: nx,
            normal_y: ny,
        };

        // Select the first intersection along the ray, never an optimized
        // geometric arrangement. Ties are resolved by catalog edge order.
        if best.map_or(true, |(best_t, _)| t < best_t) {
            best = Some((t, point));
        }
    }

    best.map(|(_, point)| point)
}

/// Find the first physical boundary point reached from the shape center in
/// the requested local direction.
///
/// The query is intentionally directional rather than socket-based. A caller
/// may ask again with a different direction, allowing the same continuous
/// boundary to accept arbitrarily many physically separated relationships.
pub fn boundary_point_toward(shape: &Shape, target_x: f64, target_y: f64) -> Option<BoundaryPoint> {
    let form = &shape.form;
    match form {
        Form::Circle { radius } => {
            let (ux, uy) = normalized(target_x, target_y)?;
            Some(BoundaryPoint {
                x: radius * ux,
                y: radius * uy,
                normal_x: ux,
                normal_y: uy,
            })
        }
        Form::Rectangle { .. }
        | Form::RegularPolygon { .. }
        | Form::Polygon { .. } => {
            polygon_boundary_toward(form.polygon_vertices()?.as_slice(), target_x, target_y)
        }
        Form::Fluid { .. } => None,
    }
}

/// Return the two physical end locations of a future line form without
/// pretending that a line has polygon corners or a continuous circumference.
///
/// This helper stays independent of `Form::Line` until that form is introduced
/// into the immutable resource geometry model.
pub fn segment_endpoints(x0: f64, y0: f64, x1: f64, y1: f64) -> Option<(BoundaryPoint, BoundaryPoint)> {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let (nx, ny) = normalized(dx, dy)?;
    Some((
        BoundaryPoint {
            x: x0,
            y: y0,
            normal_x: -nx,
            normal_y: -ny,
        },
        BoundaryPoint {
            x: x1,
            y: y1,
            normal_x: nx,
            normal_y: ny,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn circle() -> Shape {
        Shape {
            form: Form::Circle { radius: 2.0 },
        }
    }

    #[test]
    fn circle_boundary_is_derived_from_requested_direction() {
        let point = boundary_point_toward(&circle(), 3.0, 4.0).unwrap();
        assert!((point.x - 1.2).abs() < 1e-12);
        assert!((point.y - 1.6).abs() < 1e-12);
        assert!((point.normal_x - 0.6).abs() < 1e-12);
        assert!((point.normal_y - 0.8).abs() < 1e-12);
    }

    #[test]
    fn polygon_boundary_uses_first_ray_intersection() {
        let shape = Shape {
            form: Form::Rectangle {
                width: 2.0,
                height: 4.0,
            },
        };
        let point = boundary_point_toward(&shape, 3.0, 1.0).unwrap();
        assert!((point.x - 1.0).abs() < 1e-12);
        assert!((point.y - (1.0 / 3.0)).abs() < 1e-12);
    }

    #[test]
    fn zero_direction_does_not_invent_a_boundary_location() {
        assert!(boundary_point_toward(&circle(), 0.0, 0.0).is_none());
    }

    #[test]
    fn fluid_placeholder_has_no_boundary_until_realized_geometry_exists() {
        let shape = Shape {
            form: Form::Fluid { nominal_area: 0.5 },
        };
        assert!(boundary_point_toward(&shape, 1.0, 0.0).is_none());
    }

    #[test]
    fn segment_endpoints_are_distinct_physical_locations() {
        let (a, b) = segment_endpoints(-1.0, 0.0, 1.0, 0.0).unwrap();
        assert_eq!(a.x, -1.0);
        assert_eq!(b.x, 1.0);
        assert_eq!((a.normal_x, a.normal_y), (-1.0, 0.0));
        assert_eq!((b.normal_x, b.normal_y), (1.0, 0.0));
    }
}
