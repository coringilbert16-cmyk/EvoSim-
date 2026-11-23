//! Checked single-polygon area for the strict Phase 3 geometry path.
//!
//! This module is deliberately independent of legacy Form::is_valid(): that
//! validator retains historical absolute tolerances for non-Phase-3 callers.

use crate::resources::Form;

const ROUNDING_ULPS: f64 = 16.0;

/// Area and its estimated numerical uncertainty, both in squared world units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PolygonArea {
    pub(crate) square_units: f64,
    pub(crate) uncertainty: f64,
}

fn cross(a: (f64, f64), b: (f64, f64)) -> f64 {
    a.0 * b.1 - a.1 * b.0
}

fn orientation_sign(value: f64, tolerance: f64) -> i8 {
    if value > tolerance {
        1
    } else if value < -tolerance {
        -1
    } else {
        0
    }
}

fn segments_intersect(
    a: (f64, f64),
    b: (f64, f64),
    c: (f64, f64),
    d: (f64, f64),
    linear_tolerance: f64,
) -> bool {
    let ab = (b.0 - a.0, b.1 - a.1);
    let cd = (d.0 - c.0, d.1 - c.1);
    let ac = (c.0 - a.0, c.1 - a.1);
    let ad = (d.0 - a.0, d.1 - a.1);
    let ca = (a.0 - c.0, a.1 - c.1);
    let cb = (b.0 - c.0, b.1 - c.1);
    let ab_tol = linear_tolerance * ab.0.hypot(ab.1)
        + f64::EPSILON * ROUNDING_ULPS * ab.0.hypot(ab.1) * ac.0.hypot(ac.1).max(ad.0.hypot(ad.1));
    let cd_tol = linear_tolerance * cd.0.hypot(cd.1)
        + f64::EPSILON * ROUNDING_ULPS * cd.0.hypot(cd.1) * ca.0.hypot(ca.1).max(cb.0.hypot(cb.1));
    let o1 = orientation_sign(cross(ab, ac), ab_tol);
    let o2 = orientation_sign(cross(ab, ad), ab_tol);
    let o3 = orientation_sign(cross(cd, ca), cd_tol);
    let o4 = orientation_sign(cross(cd, cb), cd_tol);

    if o1 * o2 < 0 && o3 * o4 < 0 {
        return true;
    }

    fn on_segment(a: (f64, f64), b: (f64, f64), p: (f64, f64), tol: f64) -> bool {
        p.0 >= a.0.min(b.0) - tol
            && p.0 <= a.0.max(b.0) + tol
            && p.1 >= a.1.min(b.1) - tol
            && p.1 <= a.1.max(b.1) + tol
    }

    (o1 == 0 && on_segment(a, b, c, linear_tolerance))
        || (o2 == 0 && on_segment(a, b, d, linear_tolerance))
        || (o3 == 0 && on_segment(c, d, a, linear_tolerance))
        || (o4 == 0 && on_segment(c, d, b, linear_tolerance))
}

/// Validate a supported convex polygon and return its stable area measure.
///
/// Coordinates are translated to a local origin before cross-product
/// accumulation. Concave, self-intersecting, degenerate, and unresolved input
/// fails closed. The returned uncertainty has units L².
pub(crate) fn checked_polygon_area(form: &Form) -> Option<PolygonArea> {
    if !matches!(
        form,
        Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. }
    ) {
        return None;
    }
    if let Form::Rectangle { width, height } = form {
        if !width.is_finite() || !height.is_finite() || *width <= 0.0 || *height <= 0.0 {
            return None;
        }
    }
    if let Form::RegularPolygon { sides, radius } = form {
        if *sides < 3 || !radius.is_finite() || *radius <= 0.0 {
            return None;
        }
    }

    let vertices = form.polygon_vertices()?;
    let count = vertices.len();
    if count < 3
        || vertices
            .iter()
            .any(|(x, y)| !x.is_finite() || !y.is_finite())
    {
        return None;
    }

    let origin = vertices[0];
    let local: Vec<(f64, f64)> = vertices
        .iter()
        .map(|&(x, y)| (x - origin.0, y - origin.1))
        .collect();
    if local.iter().any(|(x, y)| !x.is_finite() || !y.is_finite()) {
        return None;
    }

    let characteristic_length = (0..count)
        .map(|i| {
            let a = local[i];
            let b = local[(i + 1) % count];
            (b.0 - a.0).hypot(b.1 - a.1)
        })
        .fold(0.0_f64, f64::max);
    if !characteristic_length.is_finite() || characteristic_length <= 0.0 {
        return None;
    }

    let coordinate_scale = vertices
        .iter()
        .flat_map(|(x, y)| [x.abs(), y.abs()])
        .fold(characteristic_length, f64::max);
    let linear_tolerance = coordinate_scale * f64::EPSILON * ROUNDING_ULPS;
    if !linear_tolerance.is_finite() {
        return None;
    }

    let mut perimeter = 0.0;
    for i in 0..count {
        let a = local[i];
        let b = local[(i + 1) % count];
        let length = (b.0 - a.0).hypot(b.1 - a.1);
        if !length.is_finite() || length <= linear_tolerance {
            return None;
        }
        perimeter += length;
    }
    if !perimeter.is_finite() {
        return None;
    }

    // Non-adjacent edges may not cross, touch, or overlap.
    for i in 0..count {
        let i_next = (i + 1) % count;
        for j in (i + 1)..count {
            let j_next = (j + 1) % count;
            if i == j || i_next == j || j_next == i {
                continue;
            }
            if segments_intersect(
                local[i],
                local[i_next],
                local[j],
                local[j_next],
                linear_tolerance,
            ) {
                return None;
            }
        }
    }

    // First implementation slice is intentionally convex-only.
    let mut turn_sign = 0_i8;
    for i in 0..count {
        let a = local[i];
        let b = local[(i + 1) % count];
        let c = local[(i + 2) % count];
        let ab = (b.0 - a.0, b.1 - a.1);
        let bc = (c.0 - b.0, c.1 - b.1);
        let cross_value = cross(ab, bc);
        let cross_tolerance = linear_tolerance * (ab.0.hypot(ab.1) + bc.0.hypot(bc.1))
            + f64::EPSILON * ROUNDING_ULPS * ab.0.hypot(ab.1) * bc.0.hypot(bc.1);
        if !cross_value.is_finite() || !cross_tolerance.is_finite() {
            return None;
        }
        let sign = orientation_sign(cross_value, cross_tolerance);
        if sign == 0 || (turn_sign != 0 && sign != turn_sign) {
            return None;
        }
        turn_sign = sign;
    }

    // Kahan-compensated translated-origin shoelace accumulation.
    let mut sum = 0.0;
    let mut compensation = 0.0;
    let mut absolute_products = 0.0;
    for i in 0..count {
        let term = cross(local[i], local[(i + 1) % count]);
        if !term.is_finite() {
            return None;
        }
        absolute_products += term.abs();
        let adjusted = term - compensation;
        let next = sum + adjusted;
        compensation = (next - sum) - adjusted;
        sum = next;
    }
    let area = sum.abs() * 0.5;
    let uncertainty =
        (linear_tolerance * perimeter + f64::EPSILON * ROUNDING_ULPS * absolute_products) * 0.5;
    if !area.is_finite() || !uncertainty.is_finite() || area <= uncertainty || area <= 0.0 {
        return None;
    }

    Some(PolygonArea {
        square_units: area,
        uncertainty,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn polygon(vertices: &[(f64, f64)]) -> Form {
        Form::Polygon {
            vertices: vertices.to_vec(),
        }
    }

    #[test]
    fn rectangle_area_is_analytic_and_dimensionally_squared() {
        let area = checked_polygon_area(&Form::Rectangle {
            width: 4.0,
            height: 2.0,
        })
        .unwrap();
        assert!((area.square_units - 8.0).abs() < 1e-12);
        assert!(area.uncertainty > 0.0);
    }

    #[test]
    fn regular_polygon_area_matches_analytic_value() {
        let area = checked_polygon_area(&Form::RegularPolygon {
            sides: 6,
            radius: 2.0,
        })
        .unwrap();
        let expected = 3.0 * 3.0_f64.sqrt() * 2.0;
        assert!((area.square_units - expected).abs() < 1e-12);
    }

    #[test]
    fn winding_and_large_translation_do_not_change_area() {
        let vertices = [(-2.0, -1.0), (2.0, -1.0), (2.0, 1.0), (-2.0, 1.0)];
        let forward = checked_polygon_area(&polygon(&vertices)).unwrap();
        let reverse =
            checked_polygon_area(&polygon(&vertices.into_iter().rev().collect::<Vec<_>>()))
                .unwrap();
        let translated = polygon(
            &vertices
                .iter()
                .map(|(x, y)| (x + 1.0e12, y - 1.0e12))
                .collect::<Vec<_>>(),
        );
        let shifted = checked_polygon_area(&translated).unwrap();
        assert!((forward.square_units - 8.0).abs() < 1e-12);
        assert!((reverse.square_units - forward.square_units).abs() < 1e-12);
        assert!((shifted.square_units - forward.square_units).abs() < 1e-3);
    }

    #[test]
    fn area_scales_with_square_of_geometry_scale() {
        let small = checked_polygon_area(&Form::Rectangle {
            width: 1e-6,
            height: 2e-6,
        })
        .unwrap();
        let large = checked_polygon_area(&Form::Rectangle {
            width: 1e6,
            height: 2e6,
        })
        .unwrap();
        assert!((small.square_units / 2e-12 - 1.0).abs() < 1e-12);
        assert!((large.square_units / 2e12 - 1.0).abs() < 1e-12);
    }

    #[test]
    fn concave_self_intersecting_degenerate_and_non_finite_polygons_fail_closed() {
        assert!(checked_polygon_area(&polygon(&[
            (0.0, 0.0),
            (2.0, 0.0),
            (1.0, 0.5),
            (2.0, 2.0),
            (0.0, 2.0)
        ]))
        .is_none());
        assert!(
            checked_polygon_area(&polygon(&[(0.0, 0.0), (2.0, 2.0), (0.0, 2.0), (2.0, 0.0)]))
                .is_none()
        );
        assert!(checked_polygon_area(&polygon(&[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)])).is_none());
        assert!(
            checked_polygon_area(&polygon(&[(0.0, 0.0), (1.0, 0.0), (f64::NAN, 1.0)])).is_none()
        );
    }
}
