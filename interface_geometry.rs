//! Exact two-dimensional interface geometry.
//!
//! This module measures *shared boundary*, not volume overlap. Boundary
//! contact is therefore a prerequisite but is not itself a finite interface:
//! a point contact has zero interface length. The result is intentionally
//! independent of material composition and permeability; it is pure geometry.
//!
//! Supported rigid forms are the current circle and polygonal forms. Fluids
//! have no authoritative boundary geometry and therefore produce no finite
//! interface length.

use crate::material_geometry::PlacedMaterialPart;
use crate::resources::Form;

const GEOMETRIC_EPSILON: f64 = 1e-12;

/// Exact boundary length of one rigid placed form.
///
/// Circles use circumference. Polygonal forms use the sum of their edge
/// lengths. A fluid has no authoritative spatial boundary yet and returns
/// zero rather than inventing one from `nominal_area`.
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

/// Exact length of the boundary shared by two rigid placed forms.
///
/// This deliberately differs from `placed_forms_overlap`: crossing shapes,
/// tangent shapes, and containment may have physical contact while sharing
/// zero boundary length. The returned value is therefore zero unless the two
/// boundaries coincide along a finite one-dimensional segment (or are the
/// same circle).
///
/// `tolerance` is only a geometric comparison tolerance; it does not create a
/// physical interface of its own.
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
    Some(
        vertices
            .into_iter()
            .map(|(x, y)| {
                (
                    part.placement.x + x * cos - y * sin,
                    part.placement.y + x * sin + y * cos,
                )
            })
            .collect(),
    )
}

fn polygon_perimeter(vertices: &[(f64, f64)]) -> f64 {
    if vertices.len() < 2 {
        return 0.0;
    }
    vertices
        .iter()
        .enumerate()
        .map(|(index, &(x1, y1))| {
            let (x2, y2) = vertices[(index + 1) % vertices.len()];
            (x2 - x1).hypot(y2 - y1)
        })
        .sum()
}

fn polygon_shared_boundary_length(
    a: &PlacedMaterialPart,
    b: &PlacedMaterialPart,
    tolerance: f64,
) -> f64 {
    let (Some(a_vertices), Some(b_vertices)) = (world_polygon_vertices(a), world_polygon_vertices(b))
    else {
        return 0.0;
    };

    let mut total = 0.0;
    for (a_index, &a_start) in a_vertices.iter().enumerate() {
        let a_end = a_vertices[(a_index + 1) % a_vertices.len()];
        if segment_length(a_start, a_end) <= GEOMETRIC_EPSILON {
            continue;
        }

        for (b_index, &b_start) in b_vertices.iter().enumerate() {
            let b_end = b_vertices[(b_index + 1) % b_vertices.len()];
            if segment_length(b_start, b_end) <= GEOMETRIC_EPSILON {
                continue;
            }

            total += collinear_segment_overlap_length(
                a_start,
                a_end,
                b_start,
                b_end,
                tolerance,
            );
        }
    }

    total
}

fn segment_length(a: (f64, f64), b: (f64, f64)) -> f64 {
    (b.0 - a.0).hypot(b.1 - a.1)
}

fn cross(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    ax * by - ay * bx
}

fn collinear_segment_overlap_length(
    a_start: (f64, f64),
    a_end: (f64, f64),
    b_start: (f64, f64),
    b_end: (f64, f64),
    tolerance: f64,
) -> f64 {
    let ax = a_end.0 - a_start.0;
    let ay = a_end.1 - a_start.1;
    let bx = b_start.0 - a_start.0;
    let by = b_start.1 - a_start.1;
    let cx = b_end.0 - a_start.0;
    let cy = b_end.1 - a_start.1;

    let scale = ax.hypot(ay).max(1.0);
    let collinearity_tolerance = tolerance.max(GEOMETRIC_EPSILON) * scale;
    if cross(ax, ay, bx, by).abs() > collinearity_tolerance
        || cross(ax, ay, cx, cy).abs() > collinearity_tolerance
    {
        return 0.0;
    }

    let length = ax.hypot(ay);
    if length <= GEOMETRIC_EPSILON {
        return 0.0;
    }

    // Project both B endpoints onto A's unit direction. The overlap of the
    // resulting one-dimensional intervals is the shared physical edge length.
    let ux = ax / length;
    let uy = ay / length;
    let b0 = bx * ux + by * uy;
    let b1 = cx * ux + cy * uy;
    let b_min = b0.min(b1);
    let b_max = b0.max(b1);
    let overlap = (length.min(b_max) - 0.0_f64.max(b_min)).max(0.0);

    // A tolerance may make nearly coincident endpoints compare equal, but it
    // must never manufacture a positive-length interface from a point.
    if overlap <= tolerance.max(GEOMETRIC_EPSILON) {
        0.0
    } else {
        overlap
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::Form;
    use crate::structure::Placement;

    fn part(form: Form, x: f64, y: f64, rotation_radians: f64) -> PlacedMaterialPart {
        PlacedMaterialPart {
            part_index: 0,
            form,
            placement: Placement {
                x,
                y,
                rotation_radians,
            },
        }
    }

    #[test]
    fn circle_boundary_length_is_exact_circumference() {
        let circle = part(Form::Circle { radius: 2.0 }, 0.0, 0.0, 0.0);
        assert!((boundary_length(&circle) - 4.0 * std::f64::consts::PI).abs() < 1e-12);
    }

    #[test]
    fn rectangle_boundary_length_is_exact_perimeter() {
        let rectangle = part(
            Form::Rectangle {
                width: 4.0,
                height: 2.0,
            },
            0.0,
            0.0,
            0.0,
        );
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
        let expected = 4.0 * std::f64::consts::PI;
        assert!((shared_boundary_length(&a, &b, 0.0) - expected).abs() < 1e-12);
    }

    #[test]
    fn crossing_polygons_have_zero_shared_boundary_length() {
        let a = part(
            Form::Rectangle {
                width: 4.0,
                height: 1.0,
            },
            0.0,
            0.0,
            0.0,
        );
        let b = part(
            Form::Rectangle {
                width: 1.0,
                height: 4.0,
            },
            0.0,
            0.0,
            0.0,
        );
        assert_eq!(shared_boundary_length(&a, &b, 0.0), 0.0);
    }

    #[test]
    fn identical_rectangles_share_the_full_perimeter() {
        let a = part(
            Form::Rectangle {
                width: 4.0,
                height: 2.0,
            },
            0.0,
            0.0,
            0.0,
        );
        let b = part(
            Form::Rectangle {
                width: 4.0,
                height: 2.0,
            },
            0.0,
            0.0,
            0.0,
        );
        assert!((shared_boundary_length(&a, &b, 0.0) - 12.0).abs() < 1e-12);
    }

    #[test]
    fn partially_shared_collinear_edges_return_exact_overlap() {
        let a = part(
            Form::Rectangle {
                width: 4.0,
                height: 2.0,
            },
            0.0,
            0.0,
            0.0,
        );
        let b = part(
            Form::Rectangle {
                width: 2.0,
                height: 2.0,
            },
            3.0,
            0.0,
            0.0,
        );
        assert!((shared_boundary_length(&a, &b, 0.0) - 2.0).abs() < 1e-12);
    }

    #[test]
    fn rotated_identical_polygons_share_the_same_boundary_when_rotation_matches() {
        let angle = std::f64::consts::FRAC_PI_4;
        let a = part(
            Form::Rectangle {
                width: 4.0,
                height: 2.0,
            },
            10.0,
            20.0,
            angle,
        );
        let b = part(
            Form::Rectangle {
                width: 4.0,
                height: 2.0,
            },
            10.0,
            20.0,
            angle,
        );
        assert!((shared_boundary_length(&a, &b, 0.0) - 12.0).abs() < 1e-10);
    }

    #[test]
    fn fluid_has_no_boundary_length_without_authoritative_geometry() {
        let fluid = part(Form::Fluid { nominal_area: 100.0 }, 0.0, 0.0, 0.0);
        assert_eq!(boundary_length(&fluid), 0.0);
        assert_eq!(shared_boundary_length(&fluid, &fluid, 0.0), 0.0);
    }
}
