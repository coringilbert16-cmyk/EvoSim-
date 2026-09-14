use crate::resources::{Form, Shape};
use crate::structure::Placement;
fn vertices(shape: &Shape) -> Option<Vec<(f64, f64)>> { shape.form.polygon_vertices() }
fn edge_angle(a: (f64, f64), b: (f64, f64)) -> Option<f64> { let dx = b.0 - a.0; let dy = b.1 - a.1; if dx.hypot(dy) <= f64::EPSILON { None } else { Some(dy.atan2(dx)) } }
pub fn corner_alignment_rotations(candidate: &Shape, candidate_vertex: usize, target: &Shape, target_vertex: usize, target_rotation: f64) -> Vec<f64> {
    let (Some(cv), Some(tv)) = (vertices(candidate), vertices(target)) else { return Vec::new(); };
    if cv.len() < 3 || tv.len() < 3 || candidate_vertex >= cv.len() || target_vertex >= tv.len() { return Vec::new(); }
    let ci = candidate_vertex; let ti = target_vertex;
    let c_prev = cv[(ci + cv.len() - 1) % cv.len()]; let c_here = cv[ci]; let c_next = cv[(ci + 1) % cv.len()];
    let t_prev = tv[(ti + tv.len() - 1) % tv.len()]; let t_here = tv[ti]; let t_next = tv[(ti + 1) % tv.len()];
    let mut out = Vec::new();
    for ca in [edge_angle(c_prev, c_here), edge_angle(c_here, c_next)].into_iter().flatten() {
        for ta in [edge_angle(t_prev, t_here), edge_angle(t_here, t_next)].into_iter().flatten() {
            for relative in [ta - ca, ta + std::f64::consts::PI - ca] {
                let rotation = target_rotation + relative;
                if !out.iter().any(|r: &f64| (r - rotation).abs() <= 1e-10) { out.push(rotation); }
            }
        }
    }
    out
}
pub fn line_endpoint_alignment_rotations(candidate_endpoint: usize, target_endpoint: usize, target_rotation: f64) -> Vec<f64> {
    if candidate_endpoint > 1 || target_endpoint > 1 { return Vec::new(); }
    let candidate_interior = if candidate_endpoint == 0 { 0.0 } else { std::f64::consts::PI };
    let target_interior = target_rotation + if target_endpoint == 0 { 0.0 } else { std::f64::consts::PI };
    vec![target_interior + std::f64::consts::PI - candidate_interior, target_interior - candidate_interior]
}
pub fn world_vertex(shape: &Shape, vertex: usize, placement: Placement) -> Option<(f64, f64)> { let vertices = vertices(shape)?; let (x, y) = *vertices.get(vertex)?; let (s, c) = placement.rotation_radians.sin_cos(); Some((placement.x + x * c - y * s, placement.y + x * s + y * c)) }
pub fn has_polygon_boundary(shape: &Shape) -> bool { matches!(shape.form, Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. }) }
#[cfg(test)]
mod tests {
    use super::*; use std::f64::consts::PI;
    fn square() -> Shape { Shape { form: Form::Rectangle { width: 2.0, height: 2.0 } } }
    fn l_shape() -> Shape { Shape { form: Form::Polygon { vertices: vec![(-1.0,-2.0),(1.0,-2.0),(1.0,2.0),(0.0,2.0),(0.0,0.0),(-1.0,0.0)] } } }
    #[test] fn corner_alignment_uses_incident_edges_not_authored_normals() { let rotations = corner_alignment_rotations(&square(), 0, &square(), 2, 0.0); assert!(rotations.iter().any(|r| (r - 0.0).abs() < 1e-10)); assert!(rotations.iter().any(|r| (r - PI / 2.0).abs() < 1e-10)); }
    #[test] fn line_endpoint_alignment_is_rigid() { let rotations = line_endpoint_alignment_rotations(0, 1, 0.0); assert_eq!(rotations.len(), 2); assert!(rotations.iter().any(|r| (r - 0.0).abs() < 1e-12)); assert!(rotations.iter().any(|r| (r - PI).abs() < 1e-12)); }
    #[test] fn l_shape_preserves_its_concave_boundary() { let v = l_shape().form.polygon_vertices().unwrap(); assert_eq!(v.len(), 6); assert_eq!(v[0], (-1.0,-2.0)); assert_eq!(v[1], (1.0,-2.0)); assert_eq!(v[2], (1.0,2.0)); assert_eq!(v[3], (0.0,2.0)); assert_eq!(v[4], (0.0,0.0)); assert_eq!(v[5], (-1.0,0.0)); }
    #[test] fn world_vertex_applies_only_rigid_transform() { let p = world_vertex(&square(), 0, Placement { x: 10.0, y: 20.0, rotation_radians: PI / 2.0 }).unwrap(); assert!((p.0 - 1.0).abs() < 1e-12); assert!((p.1 - 19.0).abs() < 1e-12); }
}
