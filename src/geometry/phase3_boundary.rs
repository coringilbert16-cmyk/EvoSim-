//! Checked exposed-boundary length for the strict Phase 3 geometry path.
//!
//! Inputs are already-discovered, straight, exposed intervals. This module
//! validates their local geometry and rejects positive-length duplicate
//! coverage; it does not perform visibility discovery or polygon union work.

use crate::source_exposure::ExposedBoundaryInterval;

const ROUNDING_ULPS: f64 = 16.0;
const UNIT_TOLERANCE: f64 = 1.0e-9;

/// Exposed boundary length and estimated numerical uncertainty, both in L.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ExposedBoundaryLength {
    pub(crate) world_units: f64,
    pub(crate) uncertainty: f64,
}

fn cross(a: (f64, f64), b: (f64, f64)) -> f64 {
    a.0 * b.1 - a.1 * b.0
}

fn interval_length(interval: ExposedBoundaryInterval) -> Option<f64> {
    let length = (interval.end.0 - interval.start.0)
        .hypot(interval.end.1 - interval.start.1);
    (length.is_finite() && length > 0.0).then_some(length)
}

/// True when two intervals cover the same collinear segment for positive length.
/// Shared endpoints are valid and do not count as duplicate coverage.
fn has_positive_collinear_overlap(
    first: ExposedBoundaryInterval,
    second: ExposedBoundaryInterval,
    tolerance: f64,
) -> Option<bool> {
    let first_length = interval_length(first)?;
    let second_length = interval_length(second)?;
    let first_direction = (
        (first.end.0 - first.start.0) / first_length,
        (first.end.1 - first.start.1) / first_length,
    );
    let second_direction = (
        (second.end.0 - second.start.0) / second_length,
        (second.end.1 - second.start.1) / second_length,
    );
    let offset_start = (
        second.start.0 - first.start.0,
        second.start.1 - first.start.1,
    );
    let offset_end = (
        second.end.0 - first.start.0,
        second.end.1 - first.start.1,
    );
    if [
        first_direction.0,
        first_direction.1,
        second_direction.0,
        second_direction.1,
        offset_start.0,
        offset_start.1,
        offset_end.0,
        offset_end.1,
    ]
    .iter()
    .any(|value| !value.is_finite())
    {
        return None;
    }

    let parallel_tolerance =
        tolerance / first_length.max(second_length).max(tolerance);
    if cross(first_direction, second_direction).abs() > parallel_tolerance
        || cross(first_direction, offset_start).abs() > tolerance
        || cross(first_direction, offset_end).abs() > tolerance
    {
        return Some(false);
    }

    let second_start = offset_start.0 * first_direction.0
        + offset_start.1 * first_direction.1;
    let second_end = offset_end.0 * first_direction.0
        + offset_end.1 * first_direction.1;
    let overlap = first_length.min(second_start.max(second_end))
        - 0.0_f64.max(second_start.min(second_end));
    Some(overlap > tolerance)
}

/// Validate and measure a set of already-exposed straight boundary intervals.
///
/// Endpoints and normals must be finite; each interval must have positive
/// length, a unit outward normal perpendicular to its edge, and no other
/// interval may duplicate a positive length of its coverage. Endpoint-only
/// contact is allowed. The returned uncertainty has units L.
pub(crate) fn checked_exposed_boundary_length(
    intervals: &[ExposedBoundaryInterval],
) -> Option<ExposedBoundaryLength> {
    if intervals.is_empty() {
        return None;
    }

    let mut coordinate_scale: f64 = 0.0;
    for interval in intervals {
        for value in [
            interval.start.0,
            interval.start.1,
            interval.end.0,
            interval.end.1,
            interval.outward_normal.0,
            interval.outward_normal.1,
        ] {
            if !value.is_finite() {
                return None;
            }
        }
        coordinate_scale = coordinate_scale
            .max(interval.start.0.abs())
            .max(interval.start.1.abs())
            .max(interval.end.0.abs())
            .max(interval.end.1.abs());
    }

    let mut lengths = Vec::with_capacity(intervals.len());
    let mut characteristic_length: f64 = 0.0;
    for interval in intervals {
        let length = interval_length(*interval)?;
        let normal_length = interval
            .outward_normal
            .0
            .hypot(interval.outward_normal.1);
        if !normal_length.is_finite() || (normal_length - 1.0).abs() > UNIT_TOLERANCE {
            return None;
        }
        let tangent = (
            (interval.end.0 - interval.start.0) / length,
            (interval.end.1 - interval.start.1) / length,
        );
        let normal_dot_tangent = interval.outward_normal.0 * tangent.0
            + interval.outward_normal.1 * tangent.1;
        if !normal_dot_tangent.is_finite()
            || normal_dot_tangent.abs() > UNIT_TOLERANCE
        {
            return None;
        }
        characteristic_length = characteristic_length.max(length);
        lengths.push(length);
    }

    let linear_tolerance =
        coordinate_scale.max(characteristic_length) * f64::EPSILON * ROUNDING_ULPS;
    if !linear_tolerance.is_finite() {
        return None;
    }
    if lengths.iter().any(|length| *length <= linear_tolerance) {
        return None;
    }

    for i in 0..intervals.len() {
        for j in (i + 1)..intervals.len() {
            if has_positive_collinear_overlap(intervals[i], intervals[j], linear_tolerance)? {
                return None;
            }
        }
    }

    // Kahan-compensated summation keeps the result stable across interval order.
    let mut sum = 0.0;
    let mut compensation = 0.0;
    for length in lengths {
        let adjusted = length - compensation;
        let next = sum + adjusted;
        compensation = (next - sum) - adjusted;
        sum = next;
    }
    let uncertainty = linear_tolerance * intervals.len() as f64
        + f64::EPSILON * ROUNDING_ULPS * sum.abs();
    if !sum.is_finite() || !uncertainty.is_finite() || sum <= uncertainty {
        return None;
    }

    Some(ExposedBoundaryLength {
        world_units: sum,
        uncertainty,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interval(
        start: (f64, f64),
        end: (f64, f64),
        normal: (f64, f64),
        unit_index: usize,
    ) -> ExposedBoundaryInterval {
        ExposedBoundaryInterval {
            body_index: 0,
            unit_index,
            start,
            end,
            outward_normal: normal,
        }
    }

    fn rectangle() -> Vec<ExposedBoundaryInterval> {
        vec![
            interval((0.0, 0.0), (4.0, 0.0), (0.0, -1.0), 0),
            interval((4.0, 0.0), (4.0, 2.0), (1.0, 0.0), 0),
            interval((4.0, 2.0), (0.0, 2.0), (0.0, 1.0), 0),
            interval((0.0, 2.0), (0.0, 0.0), (-1.0, 0.0), 0),
        ]
    }

    #[test]
    fn rectangle_boundary_length_is_analytic() {
        let measured = checked_exposed_boundary_length(&rectangle()).unwrap();
        assert!((measured.world_units - 12.0).abs() < 1e-12);
        assert!(measured.uncertainty > 0.0);
    }

    #[test]
    fn polygon_boundary_length_sums_non_axis_aligned_edges() {
        let intervals = vec![
            interval((0.0, 0.0), (3.0, 0.0), (0.0, -1.0), 0),
            interval((3.0, 0.0), (3.0, 4.0), (1.0, 0.0), 0),
            interval((3.0, 4.0), (0.0, 0.0), (-0.8, 0.6), 0),
        ];
        let measured = checked_exposed_boundary_length(&intervals).unwrap();
        assert!((measured.world_units - 12.0).abs() < 1e-12);
    }

    #[test]
    fn shared_endpoints_are_allowed_but_positive_overlap_is_rejected() {
        let touching = vec![
            interval((0.0, 0.0), (1.0, 0.0), (0.0, -1.0), 0),
            interval((1.0, 0.0), (2.0, 0.0), (0.0, -1.0), 1),
        ];
        assert_eq!(
            checked_exposed_boundary_length(&touching).unwrap().world_units,
            2.0
        );

        let overlapping = vec![
            interval((0.0, 0.0), (2.0, 0.0), (0.0, -1.0), 0),
            interval((1.0, 0.0), (3.0, 0.0), (0.0, -1.0), 1),
        ];
        assert!(checked_exposed_boundary_length(&overlapping).is_none());
    }

    #[test]
    fn reversed_duplicate_and_conflicting_normal_coverage_are_rejected() {
        let reversed = vec![
            interval((0.0, 0.0), (2.0, 0.0), (0.0, -1.0), 0),
            interval((2.0, 0.0), (0.0, 0.0), (0.0, 1.0), 1),
        ];
        assert!(checked_exposed_boundary_length(&reversed).is_none());

        let conflicting = vec![
            interval((0.0, 0.0), (2.0, 0.0), (0.0, -1.0), 0),
            interval((0.0, 0.0), (2.0, 0.0), (0.0, 1.0), 1),
        ];
        assert!(checked_exposed_boundary_length(&conflicting).is_none());
    }

    #[test]
    fn malformed_intervals_fail_closed() {
        assert!(checked_exposed_boundary_length(&[]).is_none());
        assert!(checked_exposed_boundary_length(&[
            interval((0.0, 0.0), (0.0, 0.0), (0.0, 1.0), 0)
        ]).is_none());
        assert!(checked_exposed_boundary_length(&[
            interval((0.0, 0.0), (1.0, 0.0), (0.0, 2.0), 0)
        ]).is_none());
        assert!(checked_exposed_boundary_length(&[
            interval((0.0, 0.0), (1.0, 0.0), (1.0, 0.0), 0)
        ]).is_none());
        assert!(checked_exposed_boundary_length(&[
            interval((0.0, 0.0), (f64::INFINITY, 0.0), (0.0, 1.0), 0)
        ]).is_none());
    }

    #[test]
    fn length_scales_linearly_and_survives_large_translation() {
        let mut small = rectangle();
        for item in &mut small {
            item.start.0 *= 1e-6;
            item.start.1 *= 1e-6;
            item.end.0 *= 1e-6;
            item.end.1 *= 1e-6;
        }
        let small_length = checked_exposed_boundary_length(&small).unwrap();
        assert!((small_length.world_units - 12e-6).abs() < 1e-18);

        let mut translated = rectangle();
        for item in &mut translated {
            item.start.0 += 1e12;
            item.start.1 -= 1e12;
            item.end.0 += 1e12;
            item.end.1 -= 1e12;
        }
        let translated_length = checked_exposed_boundary_length(&translated).unwrap();
        assert!((translated_length.world_units - 12.0).abs() < 1e-3);
    }
}
