//! Geometry-only approach targets for exact chemical connection snapping.
//!
//! These helpers describe a physically reachable rigid-body destination. They
//! do not move materials, create bonds, or override collision geometry.
use crate::connection_faces::ConnectionFace;
use crate::material_geometry::{placed_forms_penetrate, PlacedMaterialPart};
use crate::structure::Placement;

const LENGTH_RELATIVE_EPSILON: f64 = 1e-9;
const VECTOR_EPSILON: f64 = 1e-12;

const SWEEP_MAX_DEPTH: u8 = 12;
const SWEEP_DISTANCE_EPSILON: f64 = 1e-10;

/// Compute the shortest continuous rigid transform from the current pose
/// to the exact snapped target. The returned delta is a requested movement,
/// not an instruction to teleport or commit the target pose.
pub fn connection_approach_delta(
    current: Placement,
    target: Placement,
    vertical_period: Option<f64>,
) -> Option<(f64, f64, f64)> {
    if !finite_placement(current)
        || !finite_placement(target)
        || vertical_period.is_some_and(|period| !period.is_finite() || period <= 0.0)
    {
        return None;
    }
    let delta_y = if let Some(period) = vertical_period {
        (target.y - current.y + period * 0.5).rem_euclid(period) - period * 0.5
    } else {
        target.y - current.y
    };
    let delta_rotation = (target.rotation_radians - current.rotation_radians
        + std::f64::consts::PI)
        .rem_euclid(std::f64::consts::TAU)
        - std::f64::consts::PI;
    let delta = (target.x - current.x, delta_y, delta_rotation);
    [delta.0, delta.1, delta.2]
        .iter()
        .all(|value| value.is_finite())
        .then_some(delta)
}

/// Plan one collision-valid movement step toward a selected exact connection.
///
/// This function only plans a physically reachable partial transform. It
/// never mutates material, changes bonds, or applies a final snap correction.
pub fn plan_connection_approach_step(
    moving_parts: &[PlacedMaterialPart],
    current_anchor: Placement,
    target_anchor: Placement,
    obstacles: &[PlacedMaterialPart],
    vertical_period: Option<f64>,
) -> Option<ConnectionApproachStep> {
    let requested = connection_approach_delta(current_anchor, target_anchor, vertical_period)?;
    let fraction = collision_free_rigid_approach_fraction(
        moving_parts,
        current_anchor,
        (requested.0, requested.1),
        requested.2,
        obstacles,
        vertical_period,
    )?;
    let translation = (requested.0 * fraction, requested.1 * fraction);
    let rotation_delta = requested.2 * fraction;
    let destination_anchor = Placement {
        x: current_anchor.x + translation.0,
        y: vertical_period.map_or(current_anchor.y + translation.1, |period| {
            (current_anchor.y + translation.1).rem_euclid(period)
        }),
        rotation_radians: (current_anchor.rotation_radians + rotation_delta)
            .rem_euclid(std::f64::consts::TAU),
    };
    finite_placement(destination_anchor).then_some(ConnectionApproachStep {
        fraction,
        destination_anchor,
        translation,
        rotation_delta,
    })
}

/// Validate a straight-line translation against stationary geometry.
///
/// The broad phase uses the exact swept center segment and each pair's
/// bounding radii. Intervals that cannot be proven clear are recursively
/// subdivided; unresolved intervals fail closed, so a fast body cannot tunnel
/// through a thin obstacle between endpoint checks. The narrow phase uses the
/// same material penetration predicate as the rest of the geometry system.
///
/// This is intentionally translation-only. Rotating approach paths require a
/// separate swept rotational bound and must not be approximated as translation.
pub fn translation_path_is_collision_free(
    moving_parts: &[PlacedMaterialPart],
    obstacles: &[PlacedMaterialPart],
    delta_x: f64,
    delta_y: f64,
    vertical_period: Option<f64>,
) -> bool {
    if moving_parts.is_empty()
        || !delta_x.is_finite()
        || !delta_y.is_finite()
        || vertical_period
            .is_some_and(|period| !period.is_finite() || period <= 0.0 || delta_y.abs() > period)
        || moving_parts.iter().chain(obstacles).any(|part| {
            !finite_placement(part.placement)
                || !part.form.bounding_radius().is_finite()
                || part.form.bounding_radius() < 0.0
        })
    {
        return false;
    }

    moving_parts.iter().all(|moving| {
        obstacles.iter().all(|obstacle| {
            let period = vertical_period.unwrap_or(0.0);
            let images = if period > 0.0 {
                [-period, 0.0, period]
            } else {
                [0.0, 0.0, 0.0]
            };
            images.iter().all(|image_y| {
                let mut image = obstacle.clone();
                image.placement.y += *image_y;
                translation_pair_is_clear(moving, &image, (delta_x, delta_y), 0.0, 1.0, 0)
            })
        })
    })
}

/// Validate a rigid body's continuous translation and rotation against
/// stationary geometry. Each part follows the same anchor transform, preserving
/// its local offset and orientation throughout the path.
pub fn rigid_approach_path_is_collision_free(
    moving_parts: &[PlacedMaterialPart],
    anchor_start: Placement,
    translation: (f64, f64),
    rotation_delta: f64,
    obstacles: &[PlacedMaterialPart],
    vertical_period: Option<f64>,
) -> bool {
    if moving_parts.is_empty()
        || !finite_placement(anchor_start)
        || !translation.0.is_finite()
        || !translation.1.is_finite()
        || !rotation_delta.is_finite()
        || rotation_delta.abs() > std::f64::consts::PI
        || vertical_period.is_some_and(|period| {
            !period.is_finite() || period <= 0.0 || translation.1.abs() > period
        })
        || moving_parts.iter().chain(obstacles).any(|part| {
            !finite_placement(part.placement)
                || !part.form.bounding_radius().is_finite()
                || part.form.bounding_radius() < 0.0
        })
    {
        return false;
    }

    let period = vertical_period.unwrap_or(0.0);
    moving_parts.iter().all(|moving| {
        let relative_y = if period > 0.0 {
            (moving.placement.y - anchor_start.y + period * 0.5).rem_euclid(period) - period * 0.5
        } else {
            moving.placement.y - anchor_start.y
        };
        let relative = (moving.placement.x - anchor_start.x, relative_y);
        let local_offset = rotate(relative, -anchor_start.rotation_radians);
        let local_rotation = moving.placement.rotation_radians - anchor_start.rotation_radians;
        obstacles.iter().all(|obstacle| {
            let images = if period > 0.0 {
                [-period, 0.0, period]
            } else {
                [0.0, 0.0, 0.0]
            };
            images.iter().all(|image_y| {
                let mut image = obstacle.clone();
                image.placement.y += *image_y;
                rigid_approach_pair_is_clear(
                    moving,
                    image,
                    anchor_start,
                    translation,
                    rotation_delta,
                    local_offset,
                    local_rotation,
                    0.0,
                    1.0,
                    0,
                )
            })
        })
    })
}

/// Find the furthest collision-free fraction of a rigid translation/rotation.
///
/// This is the rotational counterpart to collision_free_translation_fraction.
/// It never applies a partial move; it returns the maximum verified fraction
/// for the caller to stage and commit.
pub fn collision_free_rigid_approach_fraction(
    moving_parts: &[PlacedMaterialPart],
    anchor_start: Placement,
    translation: (f64, f64),
    rotation_delta: f64,
    obstacles: &[PlacedMaterialPart],
    vertical_period: Option<f64>,
) -> Option<f64> {
    if !rigid_approach_path_is_collision_free(
        moving_parts,
        anchor_start,
        (0.0, 0.0),
        0.0,
        obstacles,
        vertical_period,
    ) {
        return None;
    }
    if rigid_approach_path_is_collision_free(
        moving_parts,
        anchor_start,
        translation,
        rotation_delta,
        obstacles,
        vertical_period,
    ) {
        return Some(1.0);
    }

    let mut clear = 0.0;
    let mut blocked = 1.0;
    for _ in 0..48 {
        let midpoint = (clear + blocked) * 0.5;
        if rigid_approach_path_is_collision_free(
            moving_parts,
            anchor_start,
            (translation.0 * midpoint, translation.1 * midpoint),
            rotation_delta * midpoint,
            obstacles,
            vertical_period,
        ) {
            clear = midpoint;
        } else {
            blocked = midpoint;
        }
    }
    Some(clear)
}

fn rigid_approach_pair_is_clear(
    moving: &PlacedMaterialPart,
    obstacle: PlacedMaterialPart,
    anchor_start: Placement,
    translation: (f64, f64),
    rotation_delta: f64,
    local_offset: (f64, f64),
    local_rotation: f64,
    start: f64,
    end: f64,
    depth: u8,
) -> bool {
    let pose_at = |progress: f64| {
        let angle = anchor_start.rotation_radians + rotation_delta * progress;
        let offset = rotate(local_offset, angle);
        Placement {
            x: anchor_start.x + translation.0 * progress + offset.0,
            y: anchor_start.y + translation.1 * progress + offset.1,
            rotation_radians: angle + local_rotation,
        }
    };
    let start_pose = pose_at(start);
    let end_pose = pose_at(end);
    let segment_x = end_pose.x - start_pose.x;
    let segment_y = end_pose.y - start_pose.y;
    let segment_length_sq = segment_x * segment_x + segment_y * segment_y;
    let projection = if segment_length_sq > f64::EPSILON {
        (((obstacle.placement.x - start_pose.x) * segment_x
            + (obstacle.placement.y - start_pose.y) * segment_y)
            / segment_length_sq)
            .clamp(0.0, 1.0)
    } else {
        0.0
    };
    let closest_x = start_pose.x + projection * segment_x;
    let closest_y = start_pose.y + projection * segment_y;
    let angle_span = (rotation_delta * (end - start)).abs();
    if angle_span > std::f64::consts::PI {
        if depth >= SWEEP_MAX_DEPTH {
            return false;
        }
        let midpoint = (start + end) * 0.5;
        return rigid_approach_pair_is_clear(
            moving,
            obstacle.clone(),
            anchor_start,
            translation,
            rotation_delta,
            local_offset,
            local_rotation,
            start,
            midpoint,
            depth + 1,
        ) && rigid_approach_pair_is_clear(
            moving,
            obstacle,
            anchor_start,
            translation,
            rotation_delta,
            local_offset,
            local_rotation,
            midpoint,
            end,
            depth + 1,
        );
    }
    let arc_radius = local_offset.0.hypot(local_offset.1);
    let arc_deviation = arc_radius * (1.0 - (angle_span * 0.5).cos());
    let radius_sum =
        moving.form.bounding_radius() + obstacle.form.bounding_radius() + arc_deviation;

    if (closest_x - obstacle.placement.x).hypot(closest_y - obstacle.placement.y)
        >= radius_sum - SWEEP_DISTANCE_EPSILON
    {
        return true;
    }

    let midpoint = (start + end) * 0.5;
    let mut pose = moving.clone();
    pose.placement = pose_at(midpoint);
    if placed_forms_penetrate(&pose, &obstacle, 0.0) {
        return false;
    }
    if depth >= SWEEP_MAX_DEPTH {
        return false;
    }
    rigid_approach_pair_is_clear(
        moving,
        obstacle.clone(),
        anchor_start,
        translation,
        rotation_delta,
        local_offset,
        local_rotation,
        start,
        midpoint,
        depth + 1,
    ) && rigid_approach_pair_is_clear(
        moving,
        obstacle,
        anchor_start,
        translation,
        rotation_delta,
        local_offset,
        local_rotation,
        midpoint,
        end,
        depth + 1,
    )
}

/// Find the furthest collision-free fraction of a requested translation.
///
/// Returns `None` when the initial pose already penetrates an obstacle
/// or the inputs are invalid. A result below 1.0 means the requested movement
/// is blocked before its endpoint; callers must let normal physics resolve the
/// remaining contact rather than snapping through it.
pub fn collision_free_translation_fraction(
    moving_parts: &[PlacedMaterialPart],
    obstacles: &[PlacedMaterialPart],
    delta_x: f64,
    delta_y: f64,
    vertical_period: Option<f64>,
) -> Option<f64> {
    if !translation_path_is_collision_free(moving_parts, obstacles, 0.0, 0.0, vertical_period) {
        return None;
    }
    if translation_path_is_collision_free(
        moving_parts,
        obstacles,
        delta_x,
        delta_y,
        vertical_period,
    ) {
        return Some(1.0);
    }

    let mut clear = 0.0;
    let mut blocked = 1.0;
    for _ in 0..48 {
        let midpoint = (clear + blocked) * 0.5;
        if translation_path_is_collision_free(
            moving_parts,
            obstacles,
            delta_x * midpoint,
            delta_y * midpoint,
            vertical_period,
        ) {
            clear = midpoint;
        } else {
            blocked = midpoint;
        }
    }
    Some(clear)
}

fn translation_pair_is_clear(
    moving: &PlacedMaterialPart,
    obstacle: &PlacedMaterialPart,
    displacement: (f64, f64),
    start: f64,
    end: f64,
    depth: u8,
) -> bool {
    let start_x = moving.placement.x + displacement.0 * start;
    let start_y = moving.placement.y + displacement.1 * start;
    let end_x = moving.placement.x + displacement.0 * end;
    let end_y = moving.placement.y + displacement.1 * end;

    let segment_x = end_x - start_x;
    let segment_y = end_y - start_y;
    let segment_length_sq = segment_x * segment_x + segment_y * segment_y;
    let projection = if segment_length_sq > f64::EPSILON {
        (((obstacle.placement.x - start_x) * segment_x
            + (obstacle.placement.y - start_y) * segment_y)
            / segment_length_sq)
            .clamp(0.0, 1.0)
    } else {
        0.0
    };
    let closest_x = start_x + projection * segment_x;
    let closest_y = start_y + projection * segment_y;
    let radius_sum = moving.form.bounding_radius() + obstacle.form.bounding_radius();

    // If the entire swept center segment clears the enclosing circles, the
    // actual forms necessarily clear one another over this interval.
    if (closest_x - obstacle.placement.x).hypot(closest_y - obstacle.placement.y)
        >= radius_sum - SWEEP_DISTANCE_EPSILON
    {
        return true;
    }

    let midpoint = (start + end) * 0.5;
    let mut pose = moving.clone();
    pose.placement.x += displacement.0 * midpoint;
    pose.placement.y += displacement.1 * midpoint;
    if placed_forms_penetrate(&pose, obstacle, 0.0) {
        return false;
    }

    if depth >= SWEEP_MAX_DEPTH {
        // The conservative broad phase still cannot prove this tiny interval
        // clear. Reject rather than permit a possible between-sample collision.
        return false;
    }

    translation_pair_is_clear(moving, obstacle, displacement, start, midpoint, depth + 1)
        && translation_pair_is_clear(moving, obstacle, displacement, midpoint, end, depth + 1)
}

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
pub struct ConnectionApproachStep {
    /// Fraction of the requested transform that is verified collision-free.
    pub fraction: f64,
    /// Resulting anchor pose after applying only the verified movement.
    pub destination_anchor: Placement,
    /// Translation actually permitted by the path validator.
    pub translation: (f64, f64),
    /// Rotation actually permitted by the path validator, in radians.
    pub rotation_delta: f64,
}

pub struct ConnectionApproachInput<'a> {
    pub face_a: ConnectionFace,
    pub placement_a: Placement,
    pub face_b: ConnectionFace,
    pub placement_b: Placement,
    pub span: FaceSpan,
    pub initial_error: ConnectionApproachError,
    pub occupied_a: &'a [FaceInterval],
    pub occupied_b: &'a [FaceInterval],
    pub moving_parts: &'a [PlacedMaterialPart],
    pub obstacles: &'a [PlacedMaterialPart],
    /// Requested displacement from the live resonance/physics force.
    pub requested_translation: (f64, f64),
    pub requested_rotation_delta: f64,
    pub chemistry_signal: f64,
    pub endpoint_tolerance: f64,
    pub angular_tolerance_radians: f64,
    pub vertical_period: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConnectionApproachEvaluation {
    pub target: ConnectionApproachTarget,
    pub error: ConnectionApproachError,
    pub progress: f64,
    pub activation_signal: f64,
    pub movement_fraction: f64,
    pub at_snapped_target: bool,
}

impl ConnectionApproachEvaluation {
    /// Bond formation requires both the exact snapped geometry and the existing
    /// chemistry-defined activation barrier. Neither condition substitutes for
    /// the other.
    pub fn reaction_is_eligible(self, chemistry_barrier: f64) -> bool {
        self.at_snapped_target
            && crate::chemistry::activated(self.activation_signal, chemistry_barrier)
    }
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
        if !start.is_finite() || !end.is_finite() || start < 0.0 || end > 1.0 || end <= start {
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

/// Select the closest valid face-pair target, with stable face-index tie breaks.
///
/// This is geometry-only candidate selection. Chemical compatibility and
/// activation remain separate authorities, and callers must still validate
/// the continuous movement path before applying any physical displacement.
pub fn best_connection_approach_target(
    shape_a: &crate::resources::Shape,
    name_a: &str,
    placement_a: Placement,
    shape_b: &crate::resources::Shape,
    name_b: &str,
    placement_b: Placement,
    occupied_a: &[FaceInterval],
    occupied_b: &[FaceInterval],
) -> Option<(ConnectionApproachTarget, ConnectionApproachError)> {
    let faces_a = crate::connection_faces::connection_faces(shape_a, name_a);
    let faces_b = crate::connection_faces::connection_faces(shape_b, name_b);
    let mut best: Option<(f64, ConnectionApproachTarget, ConnectionApproachError)> = None;

    for face_a in faces_a {
        for face_b in &faces_b {
            for span in compatible_face_spans(face_a, *face_b) {
                if !face_span_is_unoccupied(span, occupied_a, occupied_b) {
                    continue;
                }
                let Some(target_placement) =
                    snapped_target_placement(face_a, placement_a, *face_b, span)
                else {
                    continue;
                };
                let target = ConnectionApproachTarget {
                    face_a_index: face_a.face_index,
                    face_b_index: face_b.face_index,
                    span,
                    placement_b: target_placement,
                };
                let Some(error) = approach_error(*face_b, placement_b, target) else {
                    continue;
                };
                let paired_length = face_a.length * (span.a_end - span.a_start);
                let score = error
                    .endpoint_error
                    .max(error.rotation_error * paired_length * 0.5);
                if !score.is_finite() {
                    continue;
                }
                let replace = best.as_ref().map_or(true, |(best_score, best_target, _)| {
                    score.total_cmp(best_score).is_lt()
                        || (score.total_cmp(best_score).is_eq()
                            && (target.face_a_index, target.face_b_index)
                                < (best_target.face_a_index, best_target.face_b_index))
                });
                if replace {
                    best = Some((score, target, error));
                }
            }
        }
    }
    best.map(|(_, target, error)| (target, error))
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
    // B's local edge must oppose A's world-space edge. Include A's
    // current rotation so the target remains correct for a moving/rotated anchor.
    let target_rotation = (angle_a + placement_a.rotation_radians + std::f64::consts::PI - angle_b)
        .rem_euclid(std::f64::consts::TAU);
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

/// Evaluate one face-pair approach as a single geometry/chemistry/physics state.
///
/// The caller supplies the requested movement from the existing resonance
/// force. This function validates that movement's swept path, measures progress
/// toward the exact snapped target, and gates the existing chemistry signal by
/// that same progress. It does not apply movement or create a bond.
pub fn evaluate_connection_approach(
    input: ConnectionApproachInput<'_>,
) -> Option<ConnectionApproachEvaluation> {
    if !face_span_is_unoccupied(input.span, input.occupied_a, input.occupied_b) {
        return None;
    }
    let target_placement_b =
        snapped_target_placement(input.face_a, input.placement_a, input.face_b, input.span)?;
    let target = ConnectionApproachTarget {
        face_a_index: input.face_a.face_index,
        face_b_index: input.face_b.face_index,
        span: input.span,
        placement_b: target_placement_b,
    };
    let error = approach_error(input.face_b, input.placement_b, target)?;
    let paired_face_length = input.face_a.length * (input.span.a_end - input.span.a_start);
    let progress = connection_approach_progress(input.initial_error, error, paired_face_length)?;
    let activation_signal = activation_signal_at_approach(input.chemistry_signal, progress)?;
    let movement_fraction = collision_free_rigid_approach_fraction(
        input.moving_parts,
        input.placement_b,
        input.requested_translation,
        input.requested_rotation_delta,
        input.obstacles,
        input.vertical_period,
    )?;
    Some(ConnectionApproachEvaluation {
        target,
        error,
        progress,
        activation_signal,
        movement_fraction,
        at_snapped_target: is_at_snapped_target(
            error,
            input.endpoint_tolerance,
            input.angular_tolerance_radians,
        ),
    })
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

/// Combine endpoint and orientation error into one face-pair pose error.
///
/// Angular error is converted to the displacement at half the paired face
/// length. Taking the maximum requires both endpoint alignment and orientation
/// to improve; a good center/endpoint fit cannot hide a poor angular fit.
pub fn connection_approach_progress(
    initial: ConnectionApproachError,
    current: ConnectionApproachError,
    paired_face_length: f64,
) -> Option<f64> {
    if !paired_face_length.is_finite() || paired_face_length <= 0.0 {
        return None;
    }
    let pose_error = |error: ConnectionApproachError| {
        if !error.endpoint_error.is_finite()
            || error.endpoint_error < 0.0
            || !error.rotation_error.is_finite()
            || error.rotation_error < 0.0
        {
            return None;
        }
        Some(
            error
                .endpoint_error
                .max(error.rotation_error * paired_face_length * 0.5),
        )
    };
    approach_progress(pose_error(initial)?, pose_error(current)?)
}

/// Scale the existing chemistry activation signal by verified face-pair
/// approach progress. The chemistry-defined barrier is unchanged; this only
/// prevents proximity or resonance from advancing activation independently of
/// progress toward a valid snapped geometry.
pub fn activation_signal_at_approach(chemistry_signal: f64, approach_progress: f64) -> Option<f64> {
    if !chemistry_signal.is_finite()
        || chemistry_signal < 0.0
        || !approach_progress.is_finite()
        || !(0.0..=1.0).contains(&approach_progress)
    {
        return None;
    }
    let signal = chemistry_signal * approach_progress;
    signal.is_finite().then_some(signal)
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
        return Some(if current_error <= VECTOR_EPSILON {
            1.0
        } else {
            0.0
        });
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
        let error =
            approach_error_to_face_pair(face_a, moved_a, face_b, moved_target, span).unwrap();
        assert!(is_at_snapped_target(error, 1e-10, 1e-10));
        assert_ne!(initial_target, moved_target);
    }

    #[test]
    fn activation_signal_advances_only_with_connection_approach_progress() {
        assert_eq!(activation_signal_at_approach(0.8, 0.0), Some(0.0));
        assert_eq!(activation_signal_at_approach(0.8, 0.5), Some(0.4));
        assert_eq!(activation_signal_at_approach(0.8, 1.0), Some(0.8));
        assert!(activation_signal_at_approach(0.8, 1.01).is_none());
        assert!(activation_signal_at_approach(f64::NAN, 0.5).is_none());
    }

    #[test]
    fn best_approach_target_skips_occupied_faces() {
        let shape = rectangle(1.0, 1.0);
        let placement_a = placement(0.0, 0.0, 0.0);
        let placement_b = placement(0.0, 0.0, 0.0);
        let occupied = [FaceInterval {
            start: 0.0,
            end: 1.0,
        }];
        let (target, _) = best_connection_approach_target(
            &shape,
            "Carbon",
            placement_a,
            &shape,
            "Carbon",
            placement_b,
            &occupied,
            &[],
        )
        .unwrap();
        assert_ne!(target.face_a_index, 0);
    }

    #[test]
    fn best_approach_target_fails_closed_when_no_face_pair_is_available() {
        let shape = rectangle(1.0, 1.0);
        let occupied = [FaceInterval {
            start: 0.0,
            end: 1.0,
        }];
        assert!(best_connection_approach_target(
            &shape,
            "Carbon",
            placement(0.0, 0.0, 0.0),
            &shape,
            "Carbon",
            placement(0.0, 0.0, 0.0),
            &occupied,
            &occupied,
        )
        .is_none());
    }

    #[test]
    fn approach_evaluation_couples_geometry_activation_and_safe_movement() {
        let face_a = connection_faces(&rectangle(1.0, 1.0), "Carbon")[0];
        let face_b = connection_faces(&rectangle(1.0, 1.0), "Carbon")[0];
        let placement_a = placement(0.0, 0.0, 0.0);
        let span = compatible_face_spans(face_a, face_b)[0];
        let target_b = snapped_target_placement(face_a, placement_a, face_b, span).unwrap();
        let current_b = placement(target_b.x + 1.0, target_b.y, target_b.rotation_radians);
        let target_record = ConnectionApproachTarget {
            face_a_index: face_a.face_index,
            face_b_index: face_b.face_index,
            span,
            placement_b: target_b,
        };
        let current_error = approach_error(face_b, current_b, target_record).unwrap();
        let initial_error = ConnectionApproachError {
            endpoint_error: 2.0,
            rotation_error: 0.0,
        };
        assert_eq!(current_error.endpoint_error, 1.0);
        let moving = PlacedMaterialPart {
            part_index: 0,
            form: Form::Rectangle {
                width: 1.0,
                height: 1.0,
            },
            placement: current_b,
        };
        let input = ConnectionApproachInput {
            face_a,
            placement_a,
            face_b,
            placement_b: current_b,
            span,
            initial_error,
            occupied_a: &[],
            occupied_b: &[],
            moving_parts: &[moving],
            obstacles: &[],
            requested_translation: (-0.25, 0.0),
            requested_rotation_delta: 0.0,
            chemistry_signal: 0.8,
            endpoint_tolerance: 1e-8,
            angular_tolerance_radians: 1e-8,
            vertical_period: None,
        };
        let evaluation = evaluate_connection_approach(input).unwrap();
        assert_eq!(evaluation.progress, 0.5);
        assert_eq!(evaluation.activation_signal, 0.4);
        assert_eq!(evaluation.movement_fraction, 1.0);
        assert!(!evaluation.at_snapped_target);
    }

    #[test]
    fn reaction_eligibility_requires_exact_snap_and_existing_activation_barrier() {
        let base = ConnectionApproachEvaluation {
            target: ConnectionApproachTarget {
                face_a_index: 0,
                face_b_index: 1,
                span: FaceSpan {
                    a_start: 0.0,
                    a_end: 1.0,
                    b_start: 0.0,
                    b_end: 1.0,
                },
                placement_b: placement(0.0, 0.0, 0.0),
            },
            error: ConnectionApproachError {
                endpoint_error: 0.0,
                rotation_error: 0.0,
            },
            progress: 1.0,
            activation_signal: 0.8,
            movement_fraction: 1.0,
            at_snapped_target: true,
        };
        assert!(base.reaction_is_eligible(0.5));
        assert!(!base.reaction_is_eligible(0.9));
        assert!(!ConnectionApproachEvaluation {
            at_snapped_target: false,
            ..base
        }
        .reaction_is_eligible(0.5));
    }

    #[test]
    fn approach_evaluation_rejects_occupied_faces_before_force_or_activation() {
        let face_a = connection_faces(&rectangle(1.0, 1.0), "Carbon")[0];
        let face_b = connection_faces(&rectangle(1.0, 1.0), "Carbon")[0];
        let placement_a = placement(0.0, 0.0, 0.0);
        let span = compatible_face_spans(face_a, face_b)[0];
        let target_b = snapped_target_placement(face_a, placement_a, face_b, span).unwrap();
        let target = ConnectionApproachTarget {
            face_a_index: face_a.face_index,
            face_b_index: face_b.face_index,
            span,
            placement_b: target_b,
        };
        let error = approach_error(face_b, target_b, target).unwrap();
        let moving = PlacedMaterialPart {
            part_index: 0,
            form: Form::Rectangle {
                width: 1.0,
                height: 1.0,
            },
            placement: target_b,
        };
        let occupied = [FaceInterval {
            start: 0.0,
            end: 1.0,
        }];
        let input = ConnectionApproachInput {
            face_a,
            placement_a,
            face_b,
            placement_b: target_b,
            span,
            initial_error: error,
            occupied_a: &occupied,
            occupied_b: &[],
            moving_parts: &[moving],
            obstacles: &[],
            requested_translation: (0.0, 0.0),
            requested_rotation_delta: 0.0,
            chemistry_signal: 1.0,
            endpoint_tolerance: 1e-8,
            angular_tolerance_radians: 1e-8,
            vertical_period: None,
        };
        assert!(evaluate_connection_approach(input).is_none());
    }

    #[test]
    fn connection_progress_requires_endpoint_and_orientation_alignment() {
        let initial = ConnectionApproachError {
            endpoint_error: 4.0,
            rotation_error: 0.0,
        };
        let improved_endpoint_but_worse_orientation = ConnectionApproachError {
            endpoint_error: 1.0,
            rotation_error: 1.5,
        };
        assert_eq!(
            connection_approach_progress(initial, improved_endpoint_but_worse_orientation, 4.0),
            Some(0.25),
        );

        let aligned = ConnectionApproachError {
            endpoint_error: 1.0,
            rotation_error: 0.25,
        };
        assert_eq!(
            connection_approach_progress(initial, aligned, 4.0),
            Some(0.75),
        );
    }

    #[test]
    fn approach_progress_tracks_motion_toward_and_away_from_target() {
        assert_eq!(approach_progress(10.0, 2.5), Some(0.75));
        assert_eq!(approach_progress(10.0, 12.0), Some(0.0));
        assert_eq!(approach_progress(0.0, 0.0), Some(1.0));
        assert_eq!(approach_progress(0.0, 0.5), Some(0.0));
    }

    #[test]
    fn approach_step_reaches_target_only_when_the_path_is_clear() {
        let moving = PlacedMaterialPart {
            part_index: 0,
            form: Form::Circle { radius: 0.1 },
            placement: placement(0.0, 0.0, 0.0),
        };
        let target = placement(2.0, 0.0, 0.0);
        let step = plan_connection_approach_step(
            &[moving.clone()],
            placement(0.0, 0.0, 0.0),
            target,
            &[],
            None,
        )
        .unwrap();
        assert_eq!(step.fraction, 1.0);
        assert_eq!(step.destination_anchor, target);

        let obstacle = PlacedMaterialPart {
            part_index: 0,
            form: Form::Circle { radius: 0.2 },
            placement: placement(1.0, 0.0, 0.0),
        };
        let blocked_step = plan_connection_approach_step(
            &[moving],
            placement(0.0, 0.0, 0.0),
            target,
            &[obstacle],
            None,
        )
        .unwrap();
        assert!(blocked_step.fraction > 0.0 && blocked_step.fraction < 1.0);
        assert!(blocked_step.destination_anchor.x < target.x);
    }

    #[test]
    fn approach_delta_uses_shortest_rotation_and_vertical_wrap() {
        let current = placement(2.0, 9.0, 3.0);
        let target = placement(5.0, 1.0, -3.0);
        let (dx, dy, rotation) = connection_approach_delta(current, target, Some(10.0)).unwrap();
        assert_eq!(dx, 3.0);
        assert_eq!(dy, 2.0);
        assert!((rotation - (2.0 * std::f64::consts::PI - 6.0)).abs() < 1e-12);
    }

    #[test]
    fn approach_delta_rejects_invalid_poses_and_periods() {
        assert!(connection_approach_delta(
            placement(f64::NAN, 0.0, 0.0),
            placement(0.0, 0.0, 0.0),
            None,
        )
        .is_none());
        assert!(connection_approach_delta(
            placement(0.0, 0.0, 0.0),
            placement(0.0, 0.0, 0.0),
            Some(0.0),
        )
        .is_none());
    }

    #[test]
    fn swept_translation_rejects_tunneling_through_thin_obstacle() {
        let moving = PlacedMaterialPart {
            part_index: 0,
            form: Form::Circle { radius: 0.1 },
            placement: placement(0.0, 0.0, 0.0),
        };
        let obstacle = PlacedMaterialPart {
            part_index: 0,
            form: Form::Rectangle {
                width: 0.05,
                height: 2.0,
            },
            placement: placement(5.0, 0.0, 0.0),
        };
        assert!(!translation_path_is_collision_free(
            &[moving],
            &[obstacle],
            10.0,
            0.0,
            None,
        ));
    }

    #[test]
    fn swept_translation_allows_a_path_that_clears_obstacle_geometry() {
        let moving = PlacedMaterialPart {
            part_index: 0,
            form: Form::Circle { radius: 0.1 },
            placement: placement(0.0, 0.0, 0.0),
        };
        let obstacle = PlacedMaterialPart {
            part_index: 0,
            form: Form::Rectangle {
                width: 0.2,
                height: 2.0,
            },
            placement: placement(5.0, 0.0, 0.0),
        };
        assert!(translation_path_is_collision_free(
            &[moving],
            &[obstacle],
            0.0,
            2.0,
            None,
        ));
    }

    #[test]
    fn rigid_approach_detects_rotation_sweeping_a_part_through_obstacle() {
        let moving = PlacedMaterialPart {
            part_index: 0,
            form: Form::Circle { radius: 0.1 },
            placement: placement(2.0, 0.0, 0.0),
        };
        let obstacle = PlacedMaterialPart {
            part_index: 0,
            form: Form::Circle { radius: 0.2 },
            placement: placement(0.0, 2.0, 0.0),
        };
        assert!(!rigid_approach_path_is_collision_free(
            &[moving],
            placement(0.0, 0.0, 0.0),
            (0.0, 0.0),
            std::f64::consts::FRAC_PI_2,
            &[obstacle],
            None,
        ));
    }

    #[test]
    fn rigid_approach_fraction_stops_before_swept_rotation_collision() {
        let moving = PlacedMaterialPart {
            part_index: 0,
            form: Form::Circle { radius: 0.1 },
            placement: placement(2.0, 0.0, 0.0),
        };
        let obstacle = PlacedMaterialPart {
            part_index: 0,
            form: Form::Circle { radius: 0.2 },
            placement: placement(0.0, 2.0, 0.0),
        };
        let fraction = collision_free_rigid_approach_fraction(
            &[moving],
            placement(0.0, 0.0, 0.0),
            (0.0, 0.0),
            std::f64::consts::FRAC_PI_2,
            &[obstacle],
            None,
        )
        .unwrap();
        assert!(fraction > 0.0 && fraction < 1.0);
    }

    #[test]
    fn rigid_approach_allows_rotation_when_the_swept_arc_is_clear() {
        let moving = PlacedMaterialPart {
            part_index: 0,
            form: Form::Circle { radius: 0.1 },
            placement: placement(2.0, 0.0, 0.0),
        };
        let obstacle = PlacedMaterialPart {
            part_index: 0,
            form: Form::Circle { radius: 0.2 },
            placement: placement(-2.0, 0.0, 0.0),
        };
        assert!(rigid_approach_path_is_collision_free(
            &[moving],
            placement(0.0, 0.0, 0.0),
            (0.0, 0.0),
            std::f64::consts::FRAC_PI_2,
            &[obstacle],
            None,
        ));
    }

    #[test]
    fn collision_free_fraction_stops_before_obstacle_without_snapping_through() {
        let moving = PlacedMaterialPart {
            part_index: 0,
            form: Form::Circle { radius: 0.1 },
            placement: placement(0.0, 0.0, 0.0),
        };
        let obstacle = PlacedMaterialPart {
            part_index: 0,
            form: Form::Rectangle {
                width: 0.05,
                height: 2.0,
            },
            placement: placement(5.0, 0.0, 0.0),
        };
        let fraction =
            collision_free_translation_fraction(&[moving], &[obstacle], 10.0, 0.0, None).unwrap();
        assert!(fraction > 0.0 && fraction < 0.5);
    }

    #[test]
    fn swept_translation_rejects_steps_spanning_multiple_vertical_periods() {
        let moving = PlacedMaterialPart {
            part_index: 0,
            form: Form::Circle { radius: 0.1 },
            placement: placement(5.0, 1.0, 0.0),
        };
        assert!(!translation_path_is_collision_free(
            &[moving],
            &[],
            0.0,
            21.0,
            Some(10.0),
        ));
    }

    #[test]
    fn swept_translation_checks_vertical_periodic_images() {
        let moving = PlacedMaterialPart {
            part_index: 0,
            form: Form::Circle { radius: 0.2 },
            placement: placement(5.0, 9.0, 0.0),
        };
        let obstacle = PlacedMaterialPart {
            part_index: 0,
            form: Form::Circle { radius: 0.2 },
            placement: placement(5.0, 1.0, 0.0),
        };
        assert!(!translation_path_is_collision_free(
            &[moving],
            &[obstacle],
            0.0,
            3.0,
            Some(10.0),
        ));
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
