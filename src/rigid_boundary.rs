use crate::resources::{default_catalog, Form, Shape};
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

pub fn corner_alignment_rotations(
    candidate: &Shape,
    candidate_vertex: usize,
    target: &Shape,
    target_vertex: usize,
    target_rotation: f64,
) -> Vec<f64> {
    let (Some(cv), Some(tv)) = (vertices(candidate), vertices(target)) else {
        return Vec::new();
    };
    if cv.len() < 3 || tv.len() < 3 || candidate_vertex >= cv.len() || target_vertex >= tv.len() {
        return Vec::new();
    }
    let ci = candidate_vertex;
    let ti = target_vertex;
    let c_prev = cv[(ci + cv.len() - 1) % cv.len()];
    let c_here = cv[ci];
    let c_next = cv[(ci + 1) % cv.len()];
    let t_prev = tv[(ti + tv.len() - 1) % tv.len()];
    let t_here = tv[ti];
    let t_next = tv[(ti + 1) % tv.len()];
    let mut out = Vec::new();
    for ca in [edge_angle(c_prev, c_here), edge_angle(c_here, c_next)]
        .into_iter()
        .flatten()
    {
        for ta in [edge_angle(t_prev, t_here), edge_angle(t_here, t_next)]
            .into_iter()
            .flatten()
        {
            for relative in [ta - ca, ta + std::f64::consts::PI - ca] {
                let rotation = normalize_angle(target_rotation + relative);
                if !out.iter().any(|r: &f64| (r - rotation).abs() <= 1e-10) {
                    out.push(rotation);
                }
            }
        }
    }
    out.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    out
}

pub fn line_endpoint_alignment_rotations(
    candidate_endpoint: usize,
    target_endpoint: usize,
    target_rotation: f64,
) -> Vec<f64> {
    if candidate_endpoint > 1 || target_endpoint > 1 {
        return Vec::new();
    }
    let candidate_interior = if candidate_endpoint == 0 {
        0.0
    } else {
        std::f64::consts::PI
    };
    let target_interior = target_rotation
        + if target_endpoint == 0 {
            0.0
        } else {
            std::f64::consts::PI
        };
    let mut rotations = vec![
        target_interior + std::f64::consts::PI - candidate_interior,
        target_interior - candidate_interior,
    ];
    for rotation in &mut rotations {
        *rotation = normalize_angle(*rotation);
    }
    rotations.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    rotations.dedup_by(|a, b| (*a - *b).abs() <= 1e-10);
    rotations
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
    let nx = nx0 + nx1;
    let ny = ny0 + ny1;
    let len = nx.hypot(ny);
    if len <= f64::EPSILON {
        None
    } else {
        Some((nx / len, ny / len))
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

pub fn world_vertex(shape: &Shape, vertex: usize, placement: Placement) -> Option<(f64, f64)> {
    let vertices = vertices(shape)?;
    let (x, y) = *vertices.get(vertex)?;
    let (s, c) = placement.rotation_radians.sin_cos();
    Some((placement.x + x * c - y * s, placement.y + x * s + y * c))
}

pub fn has_polygon_boundary(shape: &Shape) -> bool {
    matches!(
        shape.form,
        Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn corner_alignment_uses_incident_edges_not_authored_normals() {
        let rotations = corner_alignment_rotations(&square(), 0, &square(), 2, 0.0);
        assert!(rotations.iter().any(|r| (r - PI / 2.0).abs() < 1e-10));
        assert!(rotations.iter().any(|r| r.abs() < 1e-10));
    }
    #[test]
    fn line_endpoint_alignment_is_rigid() {
        let rotations = line_endpoint_alignment_rotations(0, 1, 0.0);
        assert_eq!(rotations.len(), 2);
        assert!(rotations.iter().any(|r| r.abs() < 1e-12));
        assert!(rotations.iter().any(|r| (r - PI).abs() < 1e-12));
    }
    #[test]
    fn l_shape_preserves_its_concave_boundary() {
        let v = l_shape().form.polygon_vertices().unwrap();
        assert_eq!(v.len(), 6);
        assert_eq!(v[0], (-1.0, -2.0));
        assert_eq!(v[1], (1.0, -2.0));
        assert_eq!(v[2], (1.0, 2.0));
        assert_eq!(v[3], (0.0, 2.0));
        assert_eq!(v[4], (0.0, 0.0));
        assert_eq!(v[5], (-1.0, 0.0));
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
        assert_eq!(vertices[3], (0.0, 0.0));
        let (nx, ny) = corner_normal(&phosphorus.shape, 3).unwrap();
        assert!((nx + 2.0_f64.sqrt() / 2.0).abs() < 1e-10);
        assert!((ny - 2.0_f64.sqrt() / 2.0).abs() < 1e-10);
    }
    #[test]
    fn square_corner_normal_is_physical_bisector() {
        let (nx, ny) = corner_normal(&square(), 0).unwrap();
        assert!((nx + 2.0_f64.sqrt() / 2.0).abs() < 1e-10);
        assert!((ny + 2.0_f64.sqrt() / 2.0).abs() < 1e-10);
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
