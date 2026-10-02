use crate::resources::{Form, Shape};
use crate::structure::Placement;

fn vertices(shape: &Shape) -> Option<Vec<(f64, f64)>> {
    shape.form.polygon_vertices()
}

fn edge_angle(a: (f64, f64), b: (f64, f64)) -> Option<f64> {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    if dx.hypot(dy) <= f64::EPSILON {
        None
    } else {
        Some(dy.atan2(dx))
    }
}

/// Outward unit normal of the actual polygon boundary at a vertex, derived only from its incident edges.
pub fn corner_normal(shape: &Shape, vertex: usize) -> Option<(f64, f64)> {
    let vertices = vertices(shape)?;
    if vertices.len() < 3 || vertex >= vertices.len() {
        return None;
    }
    let here = vertices[vertex];
    let prev = vertices[(vertex + vertices.len() - 1) % vertices.len()];
    let next = vertices[(vertex + 1) % vertices.len()];
    let signed_area = vertices
        .iter()
        .enumerate()
        .map(|(i, &(x0, y0))| {
            let (x1, y1) = vertices[(i + 1) % vertices.len()];
            x0 * y1 - y0 * x1
        })
        .sum::<f64>();
    let ccw = signed_area >= 0.0;
    let normals = |a: (f64, f64), b: (f64, f64)| {
        let dx = b.0 - a.0;
        let dy = b.1 - a.1;
        let len = dx.hypot(dy);
        if len <= f64::EPSILON {
            None
        } else if ccw {
            Some((dy / len, -dx / len))
        } else {
            Some((-dy / len, dx / len))
        }
    };
    let (nx0, ny0) = normals(prev, here)?;
    let (nx1, ny1) = normals(here, next)?;
    let len = (nx0 + nx1).hypot(ny0 + ny1);
    if len <= f64::EPSILON {
        None
    } else {
        Some(((nx0 + nx1) / len, (ny0 + ny1) / len))
    }
}

/// Outward direction of a rigid line endpoint.
pub fn line_endpoint_normal(shape: &Shape, endpoint: usize) -> Option<(f64, f64)> {
    let Form::Line { .. } = shape.form else {
        return None;
    };
    match endpoint {
        0 => Some((-1.0, 0.0)),
        1 => Some((1.0, 0.0)),
        _ => None,
    }
}

fn normalize_angle(angle: f64) -> f64 {
    let mut normalized =
        (angle + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI;
    if (normalized + std::f64::consts::PI).abs() <= 1e-12 {
        normalized = std::f64::consts::PI;
    }
    normalized
}

pub const FACE_LENGTH_TOLERANCE: f64 = 0.5;

/// Return the rigid polygon edges as local endpoint pairs.
pub fn polygon_edges(shape: &Shape) -> Vec<((f64, f64), (f64, f64))> {
    match &shape.form {
        Form::Line { length } => vec![((-*length * 0.5, 0.0), (*length * 0.5, 0.0))],
        _ => {
            let Some(vertices) = vertices(shape) else {
                return Vec::new();
            };
            if vertices.len() < 2 {
                return Vec::new();
            }
            (0..vertices.len())
                .map(|i| (vertices[i], vertices[(i + 1) % vertices.len()]))
                .collect()
        }
    }
}

/// Build a whole-material placement that puts one candidate face against one
/// existing face. The candidate's stored relative placement is included.
pub fn surface_alignment_placement(
    existing_shape: &Shape,
    existing_placement: Placement,
    existing_edge_index: usize,
    candidate_shape: &Shape,
    candidate_relative_placement: Placement,
    candidate_edge_index: usize,
    tangent_offset: f64,
) -> Option<Placement> {
    let existing_edges = polygon_edges(existing_shape);
    let candidate_edges = polygon_edges(candidate_shape);
    let (ea, eb) = *existing_edges.get(existing_edge_index)?;
    let (ca, cb) = *candidate_edges.get(candidate_edge_index)?;

    let existing_length = (eb.0 - ea.0).hypot(eb.1 - ea.1);
    let candidate_length = (cb.0 - ca.0).hypot(cb.1 - ca.1);
    if existing_length <= f64::EPSILON
        || candidate_length <= f64::EPSILON
        || (existing_length - candidate_length).abs() > FACE_LENGTH_TOLERANCE + 1e-12
    {
        return None;
    }

    let existing_angle = (eb.1 - ea.1).atan2(eb.0 - ea.0) + existing_placement.rotation_radians;
    let candidate_local_angle = (cb.1 - ca.1).atan2(cb.0 - ca.0);
    let candidate_unit_angle = existing_angle + std::f64::consts::PI - candidate_local_angle;
    let origin_rotation = candidate_unit_angle - candidate_relative_placement.rotation_radians;

    let existing_mid = ((ea.0 + eb.0) * 0.5, (ea.1 + eb.1) * 0.5);
    let candidate_mid = ((ca.0 + cb.0) * 0.5, (ca.1 + cb.1) * 0.5);
    let (rs, rc) = candidate_relative_placement.rotation_radians.sin_cos();
    let relative_mid = (
        candidate_relative_placement.x + candidate_mid.0 * rc - candidate_mid.1 * rs,
        candidate_relative_placement.y + candidate_mid.0 * rs + candidate_mid.1 * rc,
    );

    let (os, oc) = origin_rotation.sin_cos();
    let rotated_relative_mid = (
        relative_mid.0 * oc - relative_mid.1 * os,
        relative_mid.0 * os + relative_mid.1 * oc,
    );

    let (ts, tc) = existing_angle.sin_cos();
    let target_mid = (
        existing_mid.0 + tc * tangent_offset,
        existing_mid.1 + ts * tangent_offset,
    );

    Some(Placement {
        x: target_mid.0 - rotated_relative_mid.0,
        y: target_mid.1 - rotated_relative_mid.1,
        rotation_radians: normalize_angle(origin_rotation),
    })
}

/// Enumerate physically meaningful face-to-face placements. A face pair is
/// admissible when its lengths differ by no more than the shared tolerance.
pub fn surface_alignment_placements(
    existing_shape: &Shape,
    existing_placement: Placement,
    candidate_shape: &Shape,
    candidate_relative_placement: Placement,
) -> Vec<Placement> {
    let existing_edges = polygon_edges(existing_shape);
    let candidate_edges = polygon_edges(candidate_shape);
    let mut out = Vec::new();

    for existing_index in 0..existing_edges.len() {
        let (ea, eb) = existing_edges[existing_index];
        let existing_length = (eb.0 - ea.0).hypot(eb.1 - ea.1);
        if existing_length <= f64::EPSILON {
            continue;
        }
        for candidate_index in 0..candidate_edges.len() {
            let (ca, cb) = candidate_edges[candidate_index];
            let candidate_length = (cb.0 - ca.0).hypot(cb.1 - ca.1);
            if candidate_length <= f64::EPSILON
                || (existing_length - candidate_length).abs() > FACE_LENGTH_TOLERANCE + 1e-12
            {
                continue;
            }

            let delta = (existing_length - candidate_length) * 0.5;
            for offset in [0.0, delta, -delta] {
                if let Some(placement) = surface_alignment_placement(
                    existing_shape,
                    existing_placement,
                    existing_index,
                    candidate_shape,
                    candidate_relative_placement,
                    candidate_index,
                    offset,
                ) {
                    if !out.iter().any(|p: &Placement| {
                        (p.x - placement.x).abs() <= 1e-10
                            && (p.y - placement.y).abs() <= 1e-10
                            && normalize_angle(p.rotation_radians - placement.rotation_radians)
                                .abs()
                                <= 1e-10
                    }) {
                        out.push(placement);
                    }
                }
            }
        }
    }
    out
}

pub fn world_vertex(shape: &Shape, vertex: usize, placement: Placement) -> Option<(f64, f64)> {
    let vertices = vertices(shape)?;
    let (x, y) = *vertices.get(vertex)?;
    let (s, c) = placement.rotation_radians.sin_cos();
    Some((placement.x + x * c - y * s, placement.y + x * s + y * c))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;
    use std::f64::consts::PI;
    fn square() -> Shape {
        Shape {
            form: Form::Rectangle {
                width: 2.0,
                height: 2.0,
            },
        }
    }
    fn l_shape() -> Shape {
        Shape {
            form: Form::Polygon {
                vertices: vec![
                    (-1.0, -2.0),
                    (1.0, -2.0),
                    (1.0, 2.0),
                    (0.0, 2.0),
                    (0.0, 0.0),
                    (-1.0, 0.0),
                ],
            },
        }
    }
    #[test]
    fn l_shape_preserves_its_concave_boundary() {
        let v = l_shape().form.polygon_vertices().unwrap();
        assert_eq!(v.len(), 6);
        assert_eq!(v[4], (0.0, 0.0));
    }
    #[test]
    fn l_inner_corner_normal_comes_from_incident_edges() {
        let (nx, ny) = corner_normal(&l_shape(), 4).unwrap();
        assert!((nx + 2.0_f64.sqrt() / 2.0).abs() < 1e-10);
        assert!((ny - 2.0_f64.sqrt() / 2.0).abs() < 1e-10);
    }
    #[test]
    fn catalog_phosphorus_l_has_a_real_interior_corner() {
        let phosphorus = default_catalog()
            .into_iter()
            .find(|resource| resource.name == "Phosphorus")
            .expect("default catalog must contain Phosphorus");
        let vertices = phosphorus.shape.form.polygon_vertices().unwrap();
        assert_eq!(vertices.len(), 6);
        assert_eq!(vertices[4], (0.25, 0.0));
        let (nx, ny) = corner_normal(&phosphorus.shape, 4).unwrap();
        assert!(nx < 0.0);
        assert!(ny > 0.0);
    }
    #[test]
    fn square_corner_normal_is_physical_bisector() {
        let (nx, ny) = corner_normal(&square(), 0).unwrap();
        assert!((nx + 2.0_f64.sqrt() / 2.0).abs() < 1e-10);
        assert!((ny + 2.0_f64.sqrt() / 2.0).abs() < 1e-10);
    }
    #[test]
    fn surface_alignment_requires_matching_face_length_within_tolerance() {
        let long = Shape {
            form: Form::Line { length: 1.0 },
        };
        let short = Shape {
            form: Form::Line { length: 0.5 },
        };
        let too_short = Shape {
            form: Form::Line { length: 0.49 },
        };
        let origin = Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        };

        assert!(surface_alignment_placement(&long, origin, 0, &short, origin, 0, 0.0).is_some());
        assert!(
            surface_alignment_placement(&long, origin, 0, &too_short, origin, 0, 0.0).is_none()
        );
    }

    #[test]
    fn surface_alignment_places_equal_faces_opposite_each_other() {
        let square = Shape {
            form: Form::Rectangle {
                width: 1.0,
                height: 1.0,
            },
        };
        let origin = Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        };
        let placements = surface_alignment_placements(&square, origin, &square, origin);
        assert!(!placements.is_empty());

        let p = placements[0];
        let vertices = square.form.polygon_vertices().unwrap();
        let (a, b) = (vertices[0], vertices[1]);
        let edge_angle = (b.1 - a.1).atan2(b.0 - a.0);
        let candidate_edge_angle = edge_angle + p.rotation_radians;
        assert!(
            (normalize_angle(candidate_edge_angle - edge_angle) - PI).abs() < 1e-10
                || (normalize_angle(candidate_edge_angle - edge_angle) + PI).abs() < 1e-10
        );
    }

    #[test]
    fn world_vertex_applies_only_rigid_transform() {
        let p = world_vertex(
            &square(),
            0,
            Placement {
                x: 10.0,
                y: 20.0,
                rotation_radians: PI / 2.0,
            },
        )
        .unwrap();
        assert!((p.0 - 11.0).abs() < 1e-12);
        assert!((p.1 - 19.0).abs() < 1e-12);
    }
}
