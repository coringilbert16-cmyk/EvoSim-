//! Geometry-only approach targets for exact chemical connection snapping.
//!
//! These helpers describe a physically reachable rigid-body destination. They
//! do not move materials, create bonds, or override collision geometry.
use crate::connection_faces::ConnectionFace;
use crate::structure::Placement;

const LENGTH_RELATIVE_EPSILON: f64 = 1e-9;
const VECTOR_EPSILON: f64 = 1e-12;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FaceSpan {
    /// Fractional interval on face A, measured from its stable start endpoint.
    pub a_start: f64,
    pub a_end: f64,
    /// Fractional interval on face B, measured from its stable start endpoint.
    pub b_start: f64,
    pub b_end: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FaceInterval {
    pub start: f64,
    pub end: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConnectionApproachTarget {
    pub face_a_index: usize,
    pub face_b_index: usize,
    pub span: FaceSpan,
    /// Exact rigid placement of B relative to the current placement of A.
    pub placement_b: Placement,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConnectionApproachError {
    /// Maximum endpoint displacement from the exact target pose.
    pub endpoint_error: f64,
    /// Absolute angular difference from the target orientation, in radians.
    pub rotation_error: f64,
}

fn finite_placement(placement: Placement) -> bool {
    placement.x.is_finite() && placement.y.is_finite() && placement.rotation_radians.is_finite()
}

fn rotate(point: (f64, f64), angle: f64) -> (f64, f64) {
    let (sin, cos) = angle.sin_cos();
    (point.0 * cos - point.1 * sin, point.0 * sin + point.1 * cos)
}

fn world_point(point: (f64, f64), placement: Placement) -> (f64, f64) {
    let rotated = rotate(point, placement.rotation_radians);
    (placement.x + rotated.0, placement.y + rotated.1)
}

fn span_point(face: ConnectionFace, fraction: f64) -> (f64, f64) {
    (
        face.start.0 + (face.end.0 - face.start.0) * fraction,
        face.start.1 + (face.end.1 - face.start.1) * fraction,
    )
}

fn lengths_match(a: f64, b: f64) -> bool {
    (a - b).abs() <= LENGTH_RELATIVE_EPSILON * a.max(b).max(1.0)
}

/// Check whether a proposed paired span is free on both connection faces.
///
/// Occupancy is expressed as normalized intervals on each face. Touching at an
/// interval endpoint is allowed; positive-length overlap is not.
pub fn face_span_is_unoccupied(
    span: FaceSpan,
    occupied_a: &[FaceInterval],
    occupied_b: &[FaceInterval],
) -> bool {
    fn available(start: f64, end: f64, occupied: &[FaceInterval]) -> bool {
        if !start.is_finite()
            || !end.is_finite()
            || start < 0.0
            || end > 1.0
            || end <= start
        {
            return false;
        }
        occupied.iter().all(|interval| {
            interval.start.is_finite()
                && interval.end.is_finite()
                && interval.start >= 0.0
                && interval.end <= 1.0
                && interval.end > interval.start
                && (end <= interval.start + VECTOR_EPSILON
                    || start >= interval.end - VECTOR_EPSILON)
        })
    }

    available(span.a_start, span.a_end, occupied_a)
        && available(span.b_start, span.b_end, occupied_b)
}

/// Enumerate permitted full-face or half-face pairings.
///
/// Equal-length faces pair in full. A face exactly half the length of the
/// other may occupy either half of the longer face. Other ratios are rejected.
pub fn compatible_face_spans(a: ConnectionFace, b: ConnectionFace) -> Vec<FaceSpan> {
    if !a.length.is_finite()
        || !b.length.is_finite()
        || a.length <= VECTOR_EPSILON
        || b.length <= VECTOR_EPSILON
    {
        return Vec::new();
    }
    if lengths_match(a.length, b.length) {
        return vec![FaceSpan {
            a_start: 0.0,
            a_end: 1.0,
            b_start: 0.0,
            b_end: 1.0,
        }];
    }
    if lengths_match(a.length, b.length * 2.0) {
        return [0.0, 0.5]
            .into_iter()
            .map(|start| FaceSpan {
                a_start: start,
                a_end: start + 0.5,
                b_start: 0.0,
                b_end: 1.0,
            })
            .collect();
    }
    if lengths_match(b.length, a.length * 2.0) {
        return [0.0, 0.5]
            .into_iter()
            .map(|start| FaceSpan {
                a_start: 0.0,
                a_end: 1.0,
                b_start: start,
                b_end: start + 0.5,
            })
            .collect();
    }
    Vec::new()
}

/// Build every exact snapped target allowed by the two faces' length ratio.
pub fn connection_approach_targets(
    face_a: ConnectionFace,
    placement_a: Placement,
    face_b: ConnectionFace,
) -> Vec<ConnectionApproachTarget> {
    compatible_face_spans(face_a, face_b)
        .into_iter()
        .filter_map(|span| {
            snapped_target_placement(face_a, placement_a, face_b, span).map(|placement_b| {
                ConnectionApproachTarget {
                    face_a_index: face_a.face_index,
                    face_b_index: face_b.face_index,
                    span,
                    placement_b,
                }
            })
        })
        .collect()
}

/// Calculate B's exact rigid target pose for one compatible face-span pairing.
///
/// The selected face segments meet endpoint-to-endpoint and their outward
/// normals oppose. B is not moved by this function; callers must reach this
/// target through continuous, collision-valid physical movement.
pub fn snapped_target_placement(
    face_a: ConnectionFace,
    placement_a: Placement,
    face_b: ConnectionFace,
    span: FaceSpan,
) -> Option<Placement> {
    if !finite_placement(placement_a)
        || !face_a.length.is_finite()
        || !face_b.length.is_finite()
        || !face_a.normal.0.is_finite()
        || !face_a.normal.1.is_finite()
        || !face_b.normal.0.is_finite()
        || !face_b.normal.1.is_finite()
        || [span.a_start, span.a_end, span.b_start, span.b_end]
            .iter()
            .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
    {
        return None;
    }
    let allowed_span = compatible_face_spans(face_a, face_b)
        .into_iter()
        .any(|allowed| {
            (allowed.a_start - span.a_start).abs() <= LENGTH_RELATIVE_EPSILON
                && (allowed.a_end - span.a_end).abs() <= LENGTH_RELATIVE_EPSILON
                && (allowed.b_start - span.b_start).abs() <= LENGTH_RELATIVE_EPSILON
                && (allowed.b_end - span.b_end).abs() <= LENGTH_RELATIVE_EPSILON
        });
    if !allowed_span {
        return None;
    }
    let a_start_local = span_point(face_a, span.a_start);
    let a_end_local = span_point(face_a, span.a_end);
    let b_start_local = span_point(face_b, span.b_start);
    let b_end_local = span_point(face_b, span.b_end);
    let a_edge = (
        a_end_local.0 - a_start_local.0,
        a_end_local.1 - a_start_local.1,
    );
    let b_edge = (
        b_end_local.0 - b_start_local.0,
        b_end_local.1 - b_start_local.1,
    );
    let a_length = a_edge.0.hypot(a_edge.1);
    let b_length = b_edge.0.hypot(b_edge.1);
    if !a_length.is_finite()
        || !b_length.is_finite()
        || a_length <= VECTOR_EPSILON
        || !lengths_match(a_length, b_length)
    {
        return None;
    }

    // Reverse B's selected edge so its end maps to A's start and its start
    // maps to A's end.
    let angle_a = a_edge.1.atan2(a_edge.0);
    let angle_b = b_edge.1.atan2(b_edge.0);
    let target_rotation =
        (angle_a + std::f64::consts::PI - angle_b).rem_euclid(std::f64::consts::TAU);
    let target_b_end = rotate(b_end_local, target_rotation);
    let a_start_world = world_point(a_start_local, placement_a);
    let target = Placement {
        x: a_start_world.0 - target_b_end.0,
        y: a_start_world.1 - target_b_end.1,
        rotation_radians: target_rotation,
    };
    if !finite_placement(target) {
        return None;
    }

    let normal_a = rotate(face_a.normal, placement_a.rotation_radians);
    let normal_b = rotate(face_b.normal, target.rotation_radians);
    let normal_dot = normal_a.0 * normal_b.0 + normal_a.1 * normal_b.1;
    if !normal_dot.is_finite() || normal_dot > -1.0 + 1e-8 {
        return None;
    }
    Some(target)
}

/// Measure how far B's selected face segment is from its exact snapped pose.
pub fn approach_error(
    face_b: ConnectionFace,
    current_b: Placement,
    target: ConnectionApproachTarget,
) -> Option<ConnectionApproachError> {
    if face_b.face_index != target.face_b_index
        || !finite_placement(current_b)
        || !finite_placement(target.placement_b)
    {
        return None;
    }
    let current_start = world_point(span_point(face_b, target.span.b_start), current_b);
    let current_end = world_point(span_point(face_b, target.span.b_end), current_b);
    let target_start = world_point(span_point(face_b, target.span.b_start), target.placement_b);
    let target_end = world_point(span_point(face_b, target.span.b_end), target.placement_b);
    let start_error = (current_start.0 - target_start.0).hypot(current_start.1 - target_start.1);
    let end_error = (current_end.0 - target_end.0).hypot(current_end.1 - target_end.1);
    let rotation_delta = (current_b.rotation_radians - target.placement_b.rotation_radians
        + std::f64::consts::PI)
        .rem_euclid(std::f64::consts::TAU)
        - std::f64::consts::PI;
    let error = ConnectionApproachError {
        endpoint_error: start_error.max(end_error),
        rotation_error: rotation_delta.abs(),
    };
    (error.endpoint_error.is_finite() && error.rotation_error.is_finite()).then_some(error)
}

/// Re-evaluate approach error against A's current pose.
///
/// Call this with current placements during movement; this keeps the target
/// attached to A rather than leaving it at an earlier world-space location.
pub fn approach_error_to_face_pair(
    face_a: ConnectionFace,
    current_a: Placement,
    face_b: ConnectionFace,
    current_b: Placement,
    span: FaceSpan,
) -> Option<ConnectionApproachError> {
    let target_placement_b = snapped_target_placement(face_a, current_a, face_b, span)?;
    let target = ConnectionApproachTarget {
        face_a_index: face_a.face_index,
        face_b_index: face_b.face_index,
        span,
        placement_b: target_placement_b,
    };
    approach_error(face_b, current_b, target)
}

/// Relative progress from a recorded initial endpoint error to the current
/// endpoint error. Progress may decrease if physics moves away from the target.
pub fn approach_progress(initial_error: f64, current_error: f64) -> Option<f64> {
    if !initial_error.is_finite()
        || !current_error.is_finite()
        || initial_error < 0.0
        || current_error < 0.0
    {
        return None;
    }
    if initial_error <= VECTOR_EPSILON {
        return Some(1.0);
    }
    Some((1.0 - current_error / initial_error).clamp(0.0, 1.0))
}

/// Test exact-target proximity using a caller-supplied geometric tolerance.
pub fn is_at_snapped_target(
    error: ConnectionApproachError,
    endpoint_tolerance: f64,
    angular_tolerance_radians: f64,
) -> bool {
    endpoint_tolerance.is_finite()
        && endpoint_tolerance >= 0.0
        && angular_tolerance_radians.is_finite()
        && angular_tolerance_radians >= 0.0
        && error.endpoint_error.is_finite()
        && error.rotation_error.is_finite()
        && error.endpoint_error <= endpoint_tolerance
        && error.rotation_error <= angular_tolerance_radians
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection_faces::connection_faces;
    use crate::resources::{Form, Shape};

    fn placement(x: f64, y: f64, rotation_radians: f64) -> Placement {
        Placement {
            x,
            y,
            rotation_radians,
        }
    }

    fn rectangle(width: f64, height: f64) -> Shape {
        Shape {
            form: Form::Rectangle { width, height },
        }
    }

    #[test]
    fn equal_faces_produce_exact_opposed_endpoint_target() {
        let faces_a = connection_faces(&rectangle(1.0, 1.0), "Carbon");
        let faces_b = connection_faces(&rectangle(1.0, 1.0), "Carbon");
        let a = faces_a[0];
        let b = faces_b[0];
        let span = compatible_face_spans(a, b)[0];
        let target = snapped_target_placement(a, placement(4.0, 5.0, 0.0), b, span).unwrap();
        let target_record = ConnectionApproachTarget {
            face_a_index: a.face_index,
            face_b_index: b.face_index,
            span,
            placement_b: target,
        };
        let error = approach_error(b, target, target_record).unwrap();
        assert!(is_at_snapped_target(error, 1e-10, 1e-10));
        let a_start = a.world_endpoint(
            crate::connection_faces::FaceEnd::Start,
            placement(4.0, 5.0, 0.0),
        );
        let b_end = b.world_endpoint(crate::connection_faces::FaceEnd::End, target);
        assert!((a_start.0 - b_end.0).hypot(a_start.1 - b_end.1) <= 1e-10);
    }

    #[test]
    fn half_face_can_occupy_either_half_of_a_full_face() {
        let full = connection_faces(&rectangle(1.0, 1.0), "Carbon")[0];
        let half_faces = connection_faces(&rectangle(0.5, 0.5), "Carbon");
        let half = half_faces[0];
        let spans = compatible_face_spans(full, half);
        assert_eq!(spans.len(), 2);
        assert_eq!((spans[0].a_start, spans[0].a_end), (0.0, 0.5));
        assert_eq!((spans[1].a_start, spans[1].a_end), (0.5, 1.0));
        for span in spans {
            assert!(snapped_target_placement(full, placement(0.0, 0.0, 0.0), half, span).is_some());
        }
    }

    #[test]
    fn smaller_face_can_occupy_either_half_of_a_larger_b_face() {
        let small = connection_faces(&rectangle(0.5, 0.5), "Carbon")[0];
        let large = connection_faces(&rectangle(1.0, 1.0), "Carbon")[0];
        let spans = compatible_face_spans(small, large);
        assert_eq!(spans.len(), 2);
        assert_eq!((spans[0].b_start, spans[0].b_end), (0.0, 0.5));
        assert_eq!((spans[1].b_start, spans[1].b_end), (0.5, 1.0));
        assert!(spans.into_iter().all(|span| {
            snapped_target_placement(small, placement(0.0, 0.0, 0.0), large, span).is_some()
        }));
    }

    #[test]
    fn target_builder_returns_only_valid_face_pairings() {
        let full = connection_faces(&rectangle(1.0, 1.0), "Carbon")[0];
        let half = connection_faces(&rectangle(0.5, 0.5), "Carbon")[0];
        let targets = connection_approach_targets(full, placement(2.0, 3.0, 0.0), half);
        assert_eq!(targets.len(), 2);
        assert!(targets.iter().all(|target| {
            target.face_a_index == full.face_index && target.face_b_index == half.face_index
        }));
    }

    #[test]
    fn partial_face_occupancy_allows_only_the_unoccupied_half() {
        let full = connection_faces(&rectangle(1.0, 1.0), "Carbon")[0];
        let half = connection_faces(&rectangle(0.5, 0.5), "Carbon")[0];
        let spans = compatible_face_spans(full, half);
        let occupied = [FaceInterval {
            start: 0.0,
            end: 0.5,
        }];
        assert!(!face_span_is_unoccupied(spans[0], &occupied, &[]));
        assert!(face_span_is_unoccupied(spans[1], &occupied, &[]));
    }

    #[test]
    fn touching_occupancy_intervals_do_not_overlap() {
        let span = FaceSpan {
            a_start: 0.5,
            a_end: 1.0,
            b_start: 0.0,
            b_end: 1.0,
        };
        let occupied = [FaceInterval {
            start: 0.0,
            end: 0.5,
        }];
        assert!(face_span_is_unoccupied(span, &occupied, &[]));
    }

    #[test]
    fn unsupported_face_length_ratios_are_rejected() {
        let full = connection_faces(&rectangle(1.0, 1.0), "Carbon")[0];
        let other = connection_faces(&rectangle(0.3, 0.3), "Carbon")[0];
        assert!(compatible_face_spans(full, other).is_empty());
    }

    #[test]
    fn approach_error_tracks_a_moving_anchor_without_stale_world_target() {
        let face_a = connection_faces(&rectangle(1.0, 1.0), "Carbon")[0];
        let face_b = connection_faces(&rectangle(1.0, 1.0), "Carbon")[0];
        let span = compatible_face_spans(face_a, face_b)[0];
        let initial_a = placement(0.0, 0.0, 0.0);
        let initial_target = snapped_target_placement(face_a, initial_a, face_b, span).unwrap();
        let moved_a = placement(7.0, -3.0, 0.4);
        let moved_target = snapped_target_placement(face_a, moved_a, face_b, span).unwrap();
        let error = approach_error_to_face_pair(face_a, moved_a, face_b, moved_target, span)
            .unwrap();
        assert!(is_at_snapped_target(error, 1e-10, 1e-10));
        assert_ne!(initial_target, moved_target);
    }

    #[test]
    fn approach_progress_tracks_motion_toward_and_away_from_target() {
        assert_eq!(approach_progress(10.0, 2.5), Some(0.75));
        assert_eq!(approach_progress(10.0, 12.0), Some(0.0));
        assert_eq!(approach_progress(0.0, 0.0), Some(1.0));
    }

    #[test]
    fn invalid_pose_and_non_finite_progress_fail_closed() {
        let full = connection_faces(&rectangle(1.0, 1.0), "Carbon")[0];
        let span = compatible_face_spans(full, full)[0];
        assert!(
            snapped_target_placement(full, placement(f64::NAN, 0.0, 0.0), full, span).is_none()
        );
        assert_eq!(approach_progress(f64::NAN, 1.0), None);
    }
}
