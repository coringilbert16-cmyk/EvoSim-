//! Exact two-dimensional interface geometry.

use crate::material_geometry::PlacedMaterialPart;
use crate::resources::Form;

const GEOMETRIC_EPSILON: f64 = 1e-12;

pub fn boundary_length(part: &PlacedMaterialPart) -> f64 {
    match &part.form {
        Form::Circle { radius } if radius.is_finite() && *radius > 0.0 => {
            std::f64::consts::TAU * radius
        }
        Form::Circle { .. } | Form::Fluid { .. } => 0.0,
        form => world_polygon_vertices(part)
            .map(|vertices| polygon_perimeter(&vertices))
            .unwrap_or(0.0),
    }
}

/// Return the finite one-dimensional boundary shared by two rigid forms.
///
/// Transverse boundary crossings invalidate the pair as a finite interface:
/// an interpenetrating placement must not acquire an interface merely because
/// some other polygon edges happen to be collinear. Point contact, tangency,
/// and containment contribute zero length.
pub fn shared_boundary_length(
    a: &PlacedMaterialPart,
    b: &PlacedMaterialPart,
    tolerance: f64,
) -> f64 {
    if !tolerance.is_finite() || tolerance < 0.0 {
        return 0.0;
    }

    match (&a.form, &b.form) {
        (Form::Circle { radius: ar }, Form::Circle { radius: br }) => {
            if !ar.is_finite() || !br.is_finite() || *ar <= 0.0 || *br <= 0.0 {
                return 0.0;
            }
            let centers_coincident =
                (a.placement.x - b.placement.x).hypot(a.placement.y - b.placement.y)
                    <= tolerance.max(GEOMETRIC_EPSILON);
            if centers_coincident && (ar - br).abs() <= tolerance.max(GEOMETRIC_EPSILON) {
                return std::f64::consts::TAU * ar;
            }
            0.0
        }
        (Form::Fluid { .. }, _) | (_, Form::Fluid { .. }) => 0.0,
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

fn polygon_shared_boundary_length(
    a: &PlacedMaterialPart,
    b: &PlacedMaterialPart,
    tolerance: f64,
) -> f64 {
    let (Some(a_vertices), Some(b_vertices)) =
        (world_polygon_vertices(a), world_polygon_vertices(b)) else { return 0.0; };

    if polygons_have_transverse_boundary_crossing(&a_vertices, &b_vertices, tolerance) {
        return 0.0;
    }

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

fn polygons_have_transverse_boundary_crossing(
    a: &[(f64, f64)],
    b: &[(f64, f64)],
    tolerance: f64,
) -> bool {
    for (ai, &a_start) in a.iter().enumerate() {
        let a_end = a[(ai + 1) % a.len()];
        if segment_length(a_start, a_end) <= GEOMETRIC_EPSILON { continue; }
        for (bi, &b_start) in b.iter().enumerate() {
            let b_end = b[(bi + 1) % b.len()];
            if segment_length(b_start, b_end) <= GEOMETRIC_EPSILON { continue; }
            if segments_cross_transversely(a_start, a_end, b_start, b_end, tolerance) {
                return true;
            }
        }
    }
    false
}

fn segments_cross_transversely(
    a_start: (f64, f64), a_end: (f64, f64),
    b_start: (f64, f64), b_end: (f64, f64), tolerance: f64,
) -> bool {
    let ax = a_end.0 - a_start.0;
    let ay = a_end.1 - a_start.1;
    let bx = b_end.0 - b_start.0;
    let by = b_end.1 - b_start.1;
    let scale = ax.hypot(ay).max(1.0) * bx.hypot(by).max(1.0);
    let eps = tolerance.max(GEOMETRIC_EPSILON) * scale;
    if cross(ax, ay, bx, by).abs() <= eps { return false; }

    let c1 = cross(ax, ay, b_start.0 - a_start.0, b_start.1 - a_start.1);
    let c2 = cross(ax, ay, b_end.0 - a_start.0, b_end.1 - a_start.1);
    let c3 = cross(bx, by, a_start.0 - b_start.0, a_start.1 - b_start.1);
    let c4 = cross(bx, by, a_end.0 - b_start.0, a_end.1 - b_start.1);

    ((c1 > eps && c2 < -eps) || (c1 < -eps && c2 > eps))
        && ((c3 > eps && c4 < -eps) || (c3 < -eps && c4 > eps))
}

fn segment_length(a: (f64, f64), b: (f64, f64)) -> f64 {
    (b.0 - a.0).hypot(b.1 - a.1)
}

fn cross(ax: f64, ay: f64, bx: f64, by: f64) -> f64 { ax * by - ay * bx }

fn collinear_segment_overlap_length(
    a_start: (f64, f64), a_end: (f64, f64),
    b_start: (f64, f64), b_end: (f64, f64), tolerance: f64,
) -> f64 {
    let ax = a_end.0 - a_start.0;
    let ay = a_end.1 - a_start.1;
    let bx = b_start.0 - a_start.0;
    let by = b_start.1 - a_start.1;
    let cx = b_end.0 - a_start.0;
    let cy = b_end.1 - a_start.1;
    let scale = ax.hypot(ay).max(1.0);
    let eps = tolerance.max(GEOMETRIC_EPSILON) * scale;
    if cross(ax, ay, bx, by).abs() > eps || cross(ax, ay, cx, cy).abs() > eps { return 0.0; }
    let length = ax.hypot(ay);
    if length <= GEOMETRIC_EPSILON { return 0.0; }
    let ux = ax / length;
    let uy = ay / length;
    let b0 = bx * ux + by * uy;
    let b1 = cx * ux + cy * uy;
    let overlap = (length.min(b0.max(b1)) - 0.0_f64.max(b0.min(b1))).max(0.0);
    if overlap <= tolerance.max(GEOMETRIC_EPSILON) { 0.0 } else { overlap }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::Form;
    use crate::structure::Placement;

    fn part(form: Form, x: f64, y: f64, rotation_radians: f64) -> PlacedMaterialPart {
        PlacedMaterialPart { part_index: 0, form, placement: Placement { x, y, rotation_radians } }
    }

    #[test]
    fn circle_boundary_length_is_exact_circumference() {
        let circle = part(Form::Circle { radius: 2.0 }, 0.0, 0.0, 0.0);
        assert!((boundary_length(&circle) - 4.0 * std::f64::consts::PI).abs() < 1e-12);
    }

    #[test]
    fn rectangle_boundary_length_is_exact_perimeter() {
        let rectangle = part(Form::Rectangle { width: 4.0, height: 2.0 }, 0.0, 0.0, 0.0);
        assert!((boundary_length(&rectangle) - 12.0).abs() < 1e-12);
    }

    #[test]
    fn tangent_circles_have_zero_shared_boundary_length() {
        let a = part(Form::Circle { radius: 1.0 }, 0.0, 0.0, 0.0);
        let b = part(Form::Circle { radius: 1.0 }, 2.0, 0.0, 0.0);
        assert_eq!(shared_boundary_length(&a, &b, 0.0), 0.0);
    }

    #[test]
    fn coincident_circles_share_their_entire_circumference() {
        let a = part(Form::Circle { radius: 2.0 }, 5.0, -3.0, 0.0);
        let b = part(Form::Circle { radius: 2.0 }, 5.0, -3.0, 0.0);
        assert!((shared_boundary_length(&a, &b, 0.0) - 4.0 * std::f64::consts::PI).abs() < 1e-12);
    }

    #[test]
    fn crossing_polygons_have_zero_shared_boundary_length() {
        let a = part(Form::Rectangle { width: 4.0, height: 1.0 }, 0.0, 0.0, 0.0);
        let b = part(Form::Rectangle { width: 1.0, height: 4.0 }, 0.0, 0.0, 0.0);
        assert_eq!(shared_boundary_length(&a, &b, 0.0), 0.0);
    }

    #[test]
    fn overlapping_polygons_with_crossings_do_not_create_a_false_interface() {
        let a = part(Form::Rectangle { width: 4.0, height: 1.0 }, 0.0, 0.0, 0.0);
        let b = part(Form::Rectangle { width: 4.0, height: 1.0 }, 0.5, 0.0, 0.0);
        assert_eq!(shared_boundary_length(&a, &b, 0.0), 0.0);
    }

    #[test]
    fn identical_rectangles_share_the_full_perimeter() {
        let a = part(Form::Rectangle { width: 4.0, height: 2.0 }, 0.0, 0.0, 0.0);
        let b = part(Form::Rectangle { width: 4.0, height: 2.0 }, 0.0, 0.0, 0.0);
        assert!((shared_boundary_length(&a, &b, 0.0) - 12.0).abs() < 1e-12);
    }

    #[test]
    fn partially_shared_collinear_edges_return_exact_overlap() {
        let a = part(Form::Rectangle { width: 4.0, height: 2.0 }, 0.0, 0.0, 0.0);
        let b = part(Form::Rectangle { width: 2.0, height: 2.0 }, 3.0, 0.0, 0.0);
        assert!((shared_boundary_length(&a, &b, 0.0) - 2.0).abs() < 1e-12);
    }

    #[test]
    fn rotated_identical_polygons_share_the_same_boundary_when_rotation_matches() {
        let angle = std::f64::consts::FRAC_PI_4;
        let a = part(Form::Rectangle { width: 4.0, height: 2.0 }, 10.0, 20.0, angle);
        let b = part(Form::Rectangle { width: 4.0, height: 2.0 }, 10.0, 20.0, angle);
        assert!((shared_boundary_length(&a, &b, 0.0) - 12.0).abs() < 1e-10);
    }

    #[test]
    fn fluid_has_no_boundary_length_without_authoritative_geometry() {
        let fluid = part(Form::Fluid { nominal_area: 100.0 }, 0.0, 0.0, 0.0);
        assert_eq!(boundary_length(&fluid), 0.0);
        assert_eq!(shared_boundary_length(&fluid, &fluid, 0.0), 0.0);
    }
}
