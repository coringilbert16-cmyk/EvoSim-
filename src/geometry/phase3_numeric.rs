//! Scale-aware floating-point tolerances for Phase 3 geometry predicates.
//!
//! Tolerances are derived from the coordinates and characteristic lengths
//! participating in each predicate. No unit-sized floor is applied, so
//! geometries remain scale invariant near the origin. Coordinate magnitude is
//! included because subtraction after a large translation loses precision.

const ROUNDING_ULPS: f64 = 16.0;

pub(crate) fn linear_tolerance(points: &[(f64, f64)], characteristic_length: f64) -> f64 {
    let coordinate_scale = points
        .iter()
        .flat_map(|point| [point.0.abs(), point.1.abs()])
        .fold(characteristic_length.abs(), f64::max);
    coordinate_scale * f64::EPSILON * ROUNDING_ULPS
}

pub(crate) fn cross_tolerance(first: (f64, f64), second: (f64, f64), linear_tolerance: f64) -> f64 {
    let first_length = first.0.hypot(first.1);
    let second_length = second.0.hypot(second.1);
    linear_tolerance * (first_length + second_length)
        + f64::EPSILON * ROUNDING_ULPS * first_length * second_length
}

/// Parameter-space tolerance corresponding to a world-space linear tolerance.
pub(crate) fn parameter_tolerance(linear_tolerance: f64, segment_length: f64) -> f64 {
    if !segment_length.is_finite() || segment_length <= 0.0 {
        return 0.25;
    }
    (linear_tolerance / segment_length).min(0.25)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tolerance_scales_with_geometry_without_a_unit_floor() {
        let unit = linear_tolerance(&[(0.0, 0.0), (1.0, 0.0)], 1.0);
        let tiny = linear_tolerance(&[(0.0, 0.0), (1e-9, 0.0)], 1e-9);
        let huge = linear_tolerance(&[(0.0, 0.0), (1e9, 0.0)], 1e9);
        assert!((tiny / unit - 1e-9).abs() <= 1e-24);
        assert!((huge / unit - 1e9).abs() <= 1e-6);
    }

    #[test]
    fn translated_coordinates_account_for_lost_subtraction_precision() {
        let local = linear_tolerance(&[(0.0, 0.0), (1.0, 0.0)], 1.0);
        let translated = linear_tolerance(&[(1e12, -1e12), (1e12 + 1.0, -1e12)], 1.0);
        assert!(translated > local);
        assert!(translated < 1e-2);
    }

    #[test]
    fn parameter_tolerance_tracks_segment_length() {
        let linear = 1e-12;
        assert!((parameter_tolerance(linear, 1.0) - 1e-12).abs() < 1e-24);
        assert!((parameter_tolerance(linear, 1e6) - 1e-18).abs() < 1e-30);
    }
}
