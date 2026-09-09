//! Exact two-dimensional interface geometry.

use crate::material_geometry::PlacedMaterialPart;
use crate::resources::Form;

const GEOMETRIC_EPSILON: f64 = 1e-12;

pub fn boundary_length(part: &PlacedMaterialPart) -> f64 {
    match &part.form {
        Form::Circle { radius } if radius.is_finite() && *radius > 0.0 => {
            std::f64::consts::TAU * radius
        }
        Form::Circle { .. } => 0.0,
        _form => world_polygon_vertices(part)
            .map(|vertices| polygon_perimeter(&vertices))
            .unwrap_or(0.0),
    }
}

/// Return the finite one-dimensional boundary shared by two rigid forms.
/// Interpenetrating polygon placements are not valid finite interfaces.
pub fn shared_boundary_length(
    a: &PlacedMaterialPart,
    b: &PlacedMaterialPart,
    tolerance: f64,
) -> f64 {
    if !tolerance.is_finite() || tolerance < 0.0 { return 0.0; }
    match (&a.form, &b.form) {
        (Form::Circle { radius: ar }, Form::Circle { radius: br }) => {
            if !ar.is_finite() || !br.is_finite() || *ar <= 0.0 || *br <= 0.0 { return 0.0; }
            let centers_coincident = (a.placement.x - b.placement.x).hypot(a.placement.y - b.placement.y)
                <= tolerance.max(GEOMETRIC_EPSILON);
            if centers_coincident && (ar - br).abs() <= tolerance.max(GEOMETRIC_EPSILON) {
                return std::f64::consts::TAU * ar;
            }
            0.0
        }
        (Form::Circle { .. }, _) | (_, Form::Circle { .. }) => 0.0,
        _ => polygon_shared_boundary_length(a, b, tolerance),
    }
}

fn world_polygon_vertices(part: &PlacedMaterialPart) -> Option<Vec<(f64, f64)>> {
    let vertices = part.form.polygon_vertices()?;
    let (sin, cos) = part.placement.rotation_radians.sin_cos();
    Some(vertices.into_iter().map(|(x, y)| (
        part.placement.x + x * cos - y * sin,
        part.placement.y + x * sin + y * cos,
    )).collect())
}

fn polygon_perimeter(vertices: &[(f64, f64)]) -> f64 {
    if vertices.len() < 2 { return 0.0; }
    vertices.iter().enumerate().map(|(i, &(x1, y1))| {
        let (x2, y2) = vertices[(i + 1) % vertices.len()];
        (x2 - x1).hypot(y2 - y1)
    }).sum()
}

fn polygon_shared_boundary_length(a: &PlacedMaterialPart, b: &PlacedMaterialPart, tolerance: f64) -> f64 {
    let (Some(a_vertices), Some(b_vertices)) = (world_polygon_vertices(a), world_polygon_vertices(b)) else { return 0.0 };
    if polygons_interpenetrate(&a_vertices, &b_vertices, tolerance) { return 0.0; }
    let mut total = 0.0;
    for (ai, &a_start) in a_vertices.iter().enumerate() {
        let a_end = a_vertices[(ai + 1) % a_vertices.len()];
        if segment_length(a_start, a_end) <= GEOMETRIC_EPSILON { continue; }
        for (bi, &b_start) in b_vertices.iter().enumerate() {
            let b_end = b_vertices[(bi + 1) % b_vertices.len()];
            if segment_length(b_start, b_end) <= GEOMETRIC_EPSILON { continue; }
            total += collinear_segment_overlap_length(a_start, a_end, b_start, b_end, tolerance);
        }
    }
    total
}

fn polygons_interpenetrate(a: &[(f64, f64)], b: &[(f64, f64)], tolerance: f64) -> bool {
    polygons_have_transverse_boundary_crossing(a, b, tolerance)
        || polygon_has_strictly_interior_boundary_point(a, b, tolerance)
        || polygon_has_strictly_interior_boundary_point(b, a, tolerance)
}

fn polygons_have_transverse_boundary_crossing(a: &[(f64, f64)], b: &[(f64, f64)], tolerance: f64) -> bool {
    for (ai, &a_start) in a.iter().enumerate() {
        let a_end = a[(ai + 1) % a.len()];
        if segment_length(a_start, a_end) <= GEOMETRIC_EPSILON { continue; }
        for (bi, &b_start) in b.iter().enumerate() {
            let b_end = b[(bi + 1) % b.len()];
            if segment_length(b_start, b_end) <= GEOMETRIC_EPSILON { continue; }
            if segments_cross_transversely(a_start, a_end, b_start, b_end, tolerance) { return true; }
        }
    }
    false
}

fn polygon_has_strictly_interior_boundary_point(subject: &[(f64, f64)], container: &[(f64, f64)], tolerance: f64) -> bool {
    subject.iter().enumerate().any(|(index, &start)| {
        let end = subject[(index + 1) % subject.len()];
        if segment_length(start, end) <= GEOMETRIC_EPSILON { return false; }
        if point_is_strictly_inside_polygon(start, container, tolerance)
            || point_is_strictly_inside_polygon(end, container, tolerance) { return true; }
        let midpoint = ((start.0 + end.0) * 0.5, (start.1 + end.1) * 0.5);
        point_is_strictly_inside_polygon(midpoint, container, tolerance)
    })
}

fn point_is_strictly_inside_polygon(point: (f64, f64), polygon: &[(f64, f64)], tolerance: f64) -> bool {
    if polygon.len() < 3 { return false; }
    for (i, &start) in polygon.iter().enumerate() {
        let end = polygon[(i + 1) % polygon.len()];
        if point_to_segment_distance(point, start, end) <= tolerance.max(GEOMETRIC_EPSILON) { return false; }
    }
    let mut inside = false;
    for (i, &(x1, y1)) in polygon.iter().enumerate() {
        let (x2, y2) = polygon[(i + 1) % polygon.len()];
        if (y1 > point.1) != (y2 > point.1) {
            let x_at_y = x1 + (point.1 - y1) * (x2 - x1) / (y2 - y1);
            if point.0 < x_at_y { inside = !inside; }
        }
    }
    inside
}

fn point_to_segment_distance(point: (f64, f64), start: (f64, f64), end: (f64, f64)) -> f64 {
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;
    let length_squared = dx * dx + dy * dy;
    if length_squared <= GEOMETRIC_EPSILON * GEOMETRIC_EPSILON { return (point.0 - start.0).hypot(point.1 - start.1); }
    let t = (((point.0 - start.0) * dx + (point.1 - start.1) * dy) / length_squared).clamp(0.0, 1.0);
    let projection = (start.0 + t * dx, start.1 + t * dy);
    (point.0 - projection.0).hypot(point.1 - projection.1)
}

fn segments_cross_transversely(a_start: (f64, f64), a_end: (f64, f64), b_start: (f64, f64), b_end: (f64, f64), tolerance: f64) -> bool {
    let ax = a_end.0 - a_start.0; let ay = a_end.1 - a_start.1;
    let bx = b_end.0 - b_start.0; let by = b_end.1 - b_start.1;
    let scale = ax.hypot(ay).max(1.0) * bx.hypot(by).max(1.0);
    let eps = tolerance.max(GEOMETRIC_EPSILON) * scale;
    if cross(ax, ay, bx, by).abs() <= eps { return false; }
    let c1 = cross(ax, ay, b_start.0 - a_start.0, b_start.1 - a_start.1);
    let c2 = cross(ax, ay, b_end.0 - a_start.0, b_end.1 - a_start.1);
    let c3 = cross(bx, by, a_start.0 - b_start.0, a_start.1 - b_start.1);
    let c4 = cross(bx, by, a_end.0 - b_start.0, a_end.1 - b_start.1);
    ((c1 > eps && c2 < -eps) || (c1 < -eps && c2 > eps)) && ((c3 > eps && c4 < -eps) || (c3 < -eps && c4 > eps))
}

fn segment_length(a: (f64, f64), b: (f64, f64)) -> f64 { (b.0 - a.0).hypot(b.1 - a.1) }
fn cross(ax: f64, ay: f64, bx: f64, by: f64) -> f64 { ax * by - ay * bx }

fn collinear_segment_overlap_length(a_start: (f64, f64), a_end: (f64, f64), b_start: (f64, f64), b_end: (f64, f64), tolerance: f64) -> f64 {
    let ax = a_end.0 - a_start.0; let ay = a_end.1 - a_start.1;
    let bx = b_start.0 - a_start.0; let by = b_start.1 - a_start.1;
    let cx = b_end.0 - a_start.0; let cy = b_end.1 - a_start.1;
    let scale = ax.hypot(ay).max(1.0);
    let eps = tolerance.max(GEOMETRIC_EPSILON) * scale;
    if cross(ax, ay, bx, by).abs() > eps || cross(ax, ay, cx, cy).abs() > eps { return 0.0; }
    let length = ax.hypot(ay);
    if length <= GEOMETRIC_EPSILON { return 0.0; }
    let ux = ax / length; let uy = ay / length;
    let b0 = bx * ux + by * uy; let b1 = cx * ux + cy * uy;
    let overlap = (length.min(b0.max(b1)) - 0.0_f64.max(b0.min(b1))).max(0.0);
    if overlap <= tolerance.max(GEOMETRIC_EPSILON) { 0.0 } else { overlap }
}
