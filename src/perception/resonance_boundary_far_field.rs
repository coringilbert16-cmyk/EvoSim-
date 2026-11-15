//! Bounded Taylor treatment of one ordered pair's exact receiver-boundary samples.
//!
//! Each input point is one exact production interval midpoint. Its projected
//! extent and signed frequency-resolved cross-energy coefficient are folded
//! into the weight. This approximates only the smooth two-distance kernel over
//! a cluster of those already-established samples. Visibility and interval
//! generation remain exact and outside this module.
//!
//! The fixed-sample result includes scalar potential and source/receiver
//! center-gradient estimates with independent remainder bounds. A separate
//! generalized-coordinate path differentiates the exact discrete sum through
//! caller-supplied sample-position and signed-weight Jacobians, with a
//! conservative magnitude envelope. Geometry Jacobians and complete visibility
//! certification remain caller obligations; this module must not feed live movement.

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct BoundaryKernelVector {
    pub(crate) x: f64,
    pub(crate) y: f64,
}

impl BoundaryKernelVector {
    fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y
    }

    fn norm(self) -> f64 {
        self.x.hypot(self.y)
    }
}

/// One exact receiver-boundary sample for one ordered pair and one frequency.
/// Weight includes the signed spectral cross coefficient, both mass scales,
/// and the exact point-source projected extent for this interval.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WeightedBoundaryKernelSample {
    pub(crate) position: BoundaryKernelVector,
    pub(crate) weight: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct BoundaryPotentialError {
    pub(crate) potential: f64,
    pub(crate) source_force: f64,
    pub(crate) receiver_force: f64,
}

/// Certified per-sample envelope between two poses inside one smooth
/// visibility/topology region. The caller must preserve sample ordering and
/// prove these bounds for the complete pose cell.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct BoundarySampleMotionBound {
    pub(crate) position_displacement: f64,
    pub(crate) absolute_weight_change: f64,
}

/// Change bound caused only by moving the exact boundary samples and changing
/// their signed weights. It does not include the chain-rule force/torque from
/// differentiating sample position or weight with respect to body pose.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct BoundaryGeometryError {
    pub(crate) potential: f64,
    pub(crate) source_force: f64,
    pub(crate) receiver_force: f64,
}

/// One sample's generalized pose derivative and certified pose-cell envelopes.
/// Derivatives are with respect to one scalar coordinate (for example x, y,
/// or angle). The caller supplies geometry derivatives from the exact
/// visibility/sample construction; this module does not infer them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[allow(dead_code)]
pub(crate) struct BoundarySamplePoseDerivative {
    pub(crate) position_derivative: BoundaryKernelVector,
    pub(crate) weight_derivative: f64,
    pub(crate) position_derivative_bound: f64,
    pub(crate) absolute_weight_derivative_bound: f64,
    pub(crate) position_displacement_bound: f64,
    pub(crate) absolute_weight_change_bound: f64,
}

/// Derivative of both body reference positions and all matched boundary
/// samples with respect to one generalized pose coordinate.
#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub(crate) struct BoundaryPoseCoordinateDerivative {
    pub(crate) source_center_derivative: BoundaryKernelVector,
    pub(crate) receiver_center_derivative: BoundaryKernelVector,
    pub(crate) source_center_derivative_bound: f64,
    pub(crate) receiver_center_derivative_bound: f64,
    pub(crate) samples: Vec<BoundarySamplePoseDerivative>,
}

/// Exact discrete derivative at the reference pose and an absolute bound
/// valid throughout the caller-certified smooth pose cell.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[allow(dead_code)]
pub(crate) struct BoundaryPoseDerivativeEvaluation {
    pub(crate) derivative: f64,
    pub(crate) absolute_bound: f64,
    pub(crate) sample_position_derivative: f64,
    pub(crate) sample_weight_derivative: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BoundaryPotentialEvaluation {
    pub(crate) potential: f64,
    /// Derivatives of the same approximate scalar potential with respect to
    /// the source and receiver reference positions.
    pub(crate) source_gradient: BoundaryKernelVector,
    pub(crate) receiver_gradient: BoundaryKernelVector,
    pub(crate) error: BoundaryPotentialError,
    pub(crate) center: BoundaryKernelVector,
    pub(crate) radius: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BoundaryKernelExpansion {
    source_center: BoundaryKernelVector,
    receiver_center: BoundaryKernelVector,
    sample_center: BoundaryKernelVector,
    samples: Vec<WeightedBoundaryKernelSample>,
    total_weight: f64,
    absolute_weight: f64,
    first_moment: BoundaryKernelVector,
    radius: f64,
}

/// Exact derivative of a rigidly attached point for one generalized pose
/// coordinate. Angular velocity is measured about the supplied rigid-body
/// pivot; the caller chooses the body's reference point and motion authority.
#[allow(dead_code)]
pub(crate) fn rigid_pose_point_derivative(
    point: BoundaryKernelVector,
    pivot: BoundaryKernelVector,
    translation_derivative: BoundaryKernelVector,
    angular_derivative: f64,
) -> Option<BoundaryKernelVector> {
    let values = [
        point.x,
        point.y,
        pivot.x,
        pivot.y,
        translation_derivative.x,
        translation_derivative.y,
        angular_derivative,
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return None;
    }
    let offset = BoundaryKernelVector {
        x: point.x - pivot.x,
        y: point.y - pivot.y,
    };
    let derivative = BoundaryKernelVector {
        x: translation_derivative.x - angular_derivative * offset.y,
        y: translation_derivative.y + angular_derivative * offset.x,
    };
    (derivative.x.is_finite() && derivative.y.is_finite()).then_some(derivative)
}

/// One source-dependent tangent point on a rigid circle and its exact
/// generalized-coordinate derivative. tangent_side must be +1 or -1 and
/// source/circle separation must remain strictly greater than the radius.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct CircleTangentPointPoseDerivative {
    pub(crate) point: BoundaryKernelVector,
    pub(crate) derivative: BoundaryKernelVector,
}

#[allow(dead_code)]
pub(crate) fn circle_tangent_point_pose_derivative(
    center: BoundaryKernelVector,
    radius: f64,
    source: BoundaryKernelVector,
    center_derivative: BoundaryKernelVector,
    source_derivative: BoundaryKernelVector,
    tangent_side: f64,
) -> Option<CircleTangentPointPoseDerivative> {
    if !radius.is_finite()
        || radius <= 0.0
        || !tangent_side.is_finite()
        || (tangent_side.abs() - 1.0).abs() > 1.0e-12
    {
        return None;
    }
    let delta = BoundaryKernelVector {
        x: source.x - center.x,
        y: source.y - center.y,
    };
    let delta_derivative = BoundaryKernelVector {
        x: source_derivative.x - center_derivative.x,
        y: source_derivative.y - center_derivative.y,
    };
    let distance = delta.norm();
    if !distance.is_finite()
        || distance <= radius + f64::EPSILON
        || [
            center.x,
            center.y,
            source.x,
            source.y,
            center_derivative.x,
            center_derivative.y,
            source_derivative.x,
            source_derivative.y,
        ]
        .iter()
        .any(|value| !value.is_finite())
    {
        return None;
    }
    let unit = BoundaryKernelVector {
        x: delta.x / distance,
        y: delta.y / distance,
    };
    let perpendicular = BoundaryKernelVector {
        x: -unit.y,
        y: unit.x,
    };
    let distance_derivative = unit.dot(delta_derivative);
    let unit_derivative = BoundaryKernelVector {
        x: (delta_derivative.x - unit.x * distance_derivative) / distance,
        y: (delta_derivative.y - unit.y * distance_derivative) / distance,
    };
    let ratio = radius / distance;
    let transverse = (1.0 - ratio * ratio).sqrt();
    if !transverse.is_finite() || transverse <= f64::EPSILON {
        return None;
    }
    let ratio_derivative = -ratio * distance_derivative / distance;
    let transverse_derivative = -ratio * ratio_derivative / transverse;
    let perpendicular_derivative = BoundaryKernelVector {
        x: -unit_derivative.y,
        y: unit_derivative.x,
    };
    let point = BoundaryKernelVector {
        x: center.x + radius * (ratio * unit.x + tangent_side * transverse * perpendicular.x),
        y: center.y + radius * (ratio * unit.y + tangent_side * transverse * perpendicular.y),
    };
    let derivative = BoundaryKernelVector {
        x: center_derivative.x
            + radius
                * (ratio_derivative * unit.x
                    + ratio * unit_derivative.x
                    + tangent_side
                        * (transverse_derivative * perpendicular.x
                            + transverse * perpendicular_derivative.x)),
        y: center_derivative.y
            + radius
                * (ratio_derivative * unit.y
                    + ratio * unit_derivative.y
                    + tangent_side
                        * (transverse_derivative * perpendicular.y
                            + transverse * perpendicular_derivative.y)),
    };
    [point.x, point.y, derivative.x, derivative.y]
        .iter()
        .all(|value| value.is_finite())
        .then_some(CircleTangentPointPoseDerivative { point, derivative })
}

/// Conservative displacement of a point attached to a rigid body when its
/// reference point translates by at most `translation_bound` and its
/// orientation changes by at most `rotation_bound_radians`. The point is
/// at most `maximum_radius` from the rotation pivot. This bounds only the
/// rigid transport contribution; source-ray intersection changes and
/// visibility/topology changes require separate certificates.
pub(crate) fn rigid_pose_point_displacement_bound(
    maximum_radius: f64,
    translation_bound: f64,
    rotation_bound_radians: f64,
) -> Option<f64> {
    if !maximum_radius.is_finite()
        || maximum_radius < 0.0
        || !translation_bound.is_finite()
        || translation_bound < 0.0
        || !rotation_bound_radians.is_finite()
        || rotation_bound_radians < 0.0
    {
        return None;
    }
    let angular_displacement =
        2.0 * maximum_radius * (0.5 * rotation_bound_radians.min(std::f64::consts::PI)).sin();
    let bound = translation_bound + angular_displacement;
    bound.is_finite().then_some(bound)
}

/// Pose envelope for one source ray intersecting one receiver edge. The
/// critical point is the blocker feature defining the ray; its displacement
/// bound must include its own rigid motion and any source-dependent tangent
/// construction. This helper certifies only this one intersection, not the
/// ordering of multiple cuts or the complete visibility topology.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct RayEdgeIntersectionMotion {
    pub(crate) source_displacement: f64,
    pub(crate) critical_point_displacement: f64,
    pub(crate) edge_start_displacement: f64,
    pub(crate) edge_end_displacement: f64,
}

/// Exact first derivative of one strict ray/edge cut for one generalized
/// pose coordinate. The critical-point derivative is supplied by the caller;
/// for a polygon vertex it is the vertex's rigid-pose derivative.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct RayEdgeIntersectionPoseDerivative {
    pub(crate) edge_parameter_derivative: f64,
    pub(crate) point_derivative: BoundaryKernelVector,
}

/// Differentiate the existing source-ray/receiver-edge intersection
/// construction while its strict interior/beyond-edge classification holds.
#[allow(dead_code)]
pub(crate) fn ray_edge_intersection_pose_derivative(
    source: BoundaryKernelVector,
    critical_point: BoundaryKernelVector,
    edge_start: BoundaryKernelVector,
    edge_end: BoundaryKernelVector,
    source_derivative: BoundaryKernelVector,
    critical_point_derivative: BoundaryKernelVector,
    edge_start_derivative: BoundaryKernelVector,
    edge_end_derivative: BoundaryKernelVector,
) -> Option<RayEdgeIntersectionPoseDerivative> {
    let points = [
        source,
        critical_point,
        edge_start,
        edge_end,
        source_derivative,
        critical_point_derivative,
        edge_start_derivative,
        edge_end_derivative,
    ];
    if points
        .iter()
        .any(|point| !point.x.is_finite() || !point.y.is_finite())
    {
        return None;
    }
    let subtract = |left: BoundaryKernelVector, right: BoundaryKernelVector| BoundaryKernelVector {
        x: left.x - right.x,
        y: left.y - right.y,
    };
    let cross = |left: BoundaryKernelVector, right: BoundaryKernelVector| {
        left.x * right.y - left.y * right.x
    };
    let q = subtract(edge_start, source);
    let r = subtract(critical_point, source);
    let edge = subtract(edge_end, edge_start);
    let q_derivative = subtract(edge_start_derivative, source_derivative);
    let r_derivative = subtract(critical_point_derivative, source_derivative);
    let edge_derivative = subtract(edge_end_derivative, edge_start_derivative);
    let denominator = cross(edge, r);
    let numerator = -cross(q, r);
    if !denominator.is_finite() || !numerator.is_finite() || denominator.abs() <= f64::EPSILON {
        return None;
    }
    let t = numerator / denominator;
    let u = -cross(q, edge) / denominator;
    if !t.is_finite()
        || !u.is_finite()
        || !(1.0e-10..1.0 - 1.0e-10).contains(&t)
        || u <= 1.0 + 1.0e-10
    {
        return None;
    }
    let numerator_derivative = -cross(q_derivative, r) - cross(q, r_derivative);
    let denominator_derivative = cross(edge_derivative, r) + cross(edge, r_derivative);
    let t_derivative = (numerator_derivative * denominator - numerator * denominator_derivative)
        / (denominator * denominator);
    let point_derivative = BoundaryKernelVector {
        x: edge_start_derivative.x + t_derivative * edge.x + t * edge_derivative.x,
        y: edge_start_derivative.y + t_derivative * edge.y + t * edge_derivative.y,
    };
    [t_derivative, point_derivative.x, point_derivative.y]
        .iter()
        .all(|value| value.is_finite())
        .then_some(RayEdgeIntersectionPoseDerivative {
            edge_parameter_derivative: t_derivative,
            point_derivative,
        })
}

/// Exact first derivative of one strict straight/straight boundary cut.
/// Both segment parameters must remain strictly interior; parallel, endpoint,
/// and topology-transition cases fail closed to exact evaluation/subdivision.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct StraightBoundaryIntersectionPoseDerivative {
    pub(crate) target_parameter_derivative: f64,
    pub(crate) blocker_parameter_derivative: f64,
    pub(crate) point_derivative: BoundaryKernelVector,
}

/// Differentiate A + t(B-A) = C + u(D-C) for one generalized pose
/// coordinate. The caller supplies derivatives of all four rigid endpoints.
#[allow(dead_code)]
pub(crate) fn straight_boundary_intersection_pose_derivative(
    target_start: BoundaryKernelVector,
    target_end: BoundaryKernelVector,
    blocker_start: BoundaryKernelVector,
    blocker_end: BoundaryKernelVector,
    target_start_derivative: BoundaryKernelVector,
    target_end_derivative: BoundaryKernelVector,
    blocker_start_derivative: BoundaryKernelVector,
    blocker_end_derivative: BoundaryKernelVector,
) -> Option<StraightBoundaryIntersectionPoseDerivative> {
    let points = [
        target_start,
        target_end,
        blocker_start,
        blocker_end,
        target_start_derivative,
        target_end_derivative,
        blocker_start_derivative,
        blocker_end_derivative,
    ];
    if points
        .iter()
        .any(|point| !point.x.is_finite() || !point.y.is_finite())
    {
        return None;
    }
    let subtract = |left: BoundaryKernelVector, right: BoundaryKernelVector| BoundaryKernelVector {
        x: left.x - right.x,
        y: left.y - right.y,
    };
    let cross = |left: BoundaryKernelVector, right: BoundaryKernelVector| {
        left.x * right.y - left.y * right.x
    };
    let target_edge = subtract(target_end, target_start);
    let blocker_edge = subtract(blocker_end, blocker_start);
    let offset = subtract(blocker_start, target_start);
    let target_edge_derivative = subtract(target_end_derivative, target_start_derivative);
    let blocker_edge_derivative = subtract(blocker_end_derivative, blocker_start_derivative);
    let offset_derivative = subtract(blocker_start_derivative, target_start_derivative);
    let denominator = cross(target_edge, blocker_edge);
    let denominator_scale = target_edge.norm() * blocker_edge.norm();
    if !denominator.is_finite()
        || !denominator_scale.is_finite()
        || denominator_scale <= f64::MIN_POSITIVE
        || denominator.abs() <= 64.0 * f64::EPSILON * denominator_scale
    {
        return None;
    }
    let target_numerator = cross(offset, blocker_edge);
    let blocker_numerator = cross(offset, target_edge);
    let target_parameter = target_numerator / denominator;
    let blocker_parameter = blocker_numerator / denominator;
    const PARAMETER_MARGIN: f64 = 1.0e-10;
    if !target_parameter.is_finite()
        || !blocker_parameter.is_finite()
        || !(PARAMETER_MARGIN..1.0 - PARAMETER_MARGIN).contains(&target_parameter)
        || !(PARAMETER_MARGIN..1.0 - PARAMETER_MARGIN).contains(&blocker_parameter)
    {
        return None;
    }
    let denominator_derivative =
        cross(target_edge_derivative, blocker_edge) + cross(target_edge, blocker_edge_derivative);
    let target_numerator_derivative =
        cross(offset_derivative, blocker_edge) + cross(offset, blocker_edge_derivative);
    let blocker_numerator_derivative =
        cross(offset_derivative, target_edge) + cross(offset, target_edge_derivative);
    let denominator_squared = denominator * denominator;
    let target_parameter_derivative = (target_numerator_derivative * denominator
        - target_numerator * denominator_derivative)
        / denominator_squared;
    let blocker_parameter_derivative = (blocker_numerator_derivative * denominator
        - blocker_numerator * denominator_derivative)
        / denominator_squared;
    let point_derivative = BoundaryKernelVector {
        x: target_start_derivative.x
            + target_parameter_derivative * target_edge.x
            + target_parameter * target_edge_derivative.x,
        y: target_start_derivative.y
            + target_parameter_derivative * target_edge.y
            + target_parameter * target_edge_derivative.y,
    };
    [
        target_parameter_derivative,
        blocker_parameter_derivative,
        point_derivative.x,
        point_derivative.y,
    ]
    .iter()
    .all(|value| value.is_finite())
    .then_some(StraightBoundaryIntersectionPoseDerivative {
        target_parameter_derivative,
        blocker_parameter_derivative,
        point_derivative,
    })
}

/// Compose rigid-body pose rates into the four endpoint rates of a
/// straight/straight visibility cut. Endpoint positions are the world-space
/// realization of the exact edge identities retained by the visibility event;
/// each edge may belong to a different rigid body.
#[allow(dead_code)]
pub(crate) fn rigid_straight_boundary_intersection_pose_derivative(
    target_start: BoundaryKernelVector,
    target_end: BoundaryKernelVector,
    blocker_start: BoundaryKernelVector,
    blocker_end: BoundaryKernelVector,
    target_pivot: BoundaryKernelVector,
    blocker_pivot: BoundaryKernelVector,
    target_translation_derivative: BoundaryKernelVector,
    target_angular_derivative: f64,
    blocker_translation_derivative: BoundaryKernelVector,
    blocker_angular_derivative: f64,
) -> Option<StraightBoundaryIntersectionPoseDerivative> {
    let target_start_derivative = rigid_pose_point_derivative(
        target_start,
        target_pivot,
        target_translation_derivative,
        target_angular_derivative,
    )?;
    let target_end_derivative = rigid_pose_point_derivative(
        target_end,
        target_pivot,
        target_translation_derivative,
        target_angular_derivative,
    )?;
    let blocker_start_derivative = rigid_pose_point_derivative(
        blocker_start,
        blocker_pivot,
        blocker_translation_derivative,
        blocker_angular_derivative,
    )?;
    let blocker_end_derivative = rigid_pose_point_derivative(
        blocker_end,
        blocker_pivot,
        blocker_translation_derivative,
        blocker_angular_derivative,
    )?;
    straight_boundary_intersection_pose_derivative(
        target_start,
        target_end,
        blocker_start,
        blocker_end,
        target_start_derivative,
        target_end_derivative,
        blocker_start_derivative,
        blocker_end_derivative,
    )
}

/// Current rigid pose for one body that owns a straight visibility feature.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct RigidBoundaryPose {
    pub(crate) pivot: BoundaryKernelVector,
    pub(crate) rotation_radians: f64,
}

impl RigidBoundaryPose {
    fn transform_local_point(self, local: BoundaryKernelVector) -> Option<BoundaryKernelVector> {
        if ![
            self.pivot.x,
            self.pivot.y,
            self.rotation_radians,
            local.x,
            local.y,
        ]
        .iter()
        .all(|value| value.is_finite())
        {
            return None;
        }
        let (sin, cos) = self.rotation_radians.sin_cos();
        let world = BoundaryKernelVector {
            x: self.pivot.x + local.x * cos - local.y * sin,
            y: self.pivot.y + local.x * sin + local.y * cos,
        };
        (world.x.is_finite() && world.y.is_finite()).then_some(world)
    }
}

/// A straight/straight visibility event resolved against its two owning poses.
/// The intersection parameters are retained from the exact visibility event;
/// this resolver verifies that they still describe the same strict-interior
/// intersection before differentiating it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ResolvedStraightBoundaryIntersection {
    pub(crate) target_start: BoundaryKernelVector,
    pub(crate) target_end: BoundaryKernelVector,
    pub(crate) blocker_start: BoundaryKernelVector,
    pub(crate) blocker_end: BoundaryKernelVector,
    pub(crate) intersection: BoundaryKernelVector,
    pub(crate) target_parameter: f64,
    pub(crate) blocker_parameter: f64,
    pub(crate) derivative: StraightBoundaryIntersectionPoseDerivative,
}

/// Resolve the local edge endpoints retained by a straight-boundary visibility
/// event using the current rigid poses, then compose both bodies' translation
/// and angular rates into the exact intersection derivative. No coordinate
/// nearest-neighbour lookup is performed. Ambiguous, stale, endpoint, parallel,
/// or unsupported events return None and must use exact/subdivision fallback.
#[allow(dead_code)]
pub(crate) fn resolve_straight_boundary_intersection_event(
    event: &crate::source_exposure::VisibilityCutEvent,
    expected_target_body: usize,
    expected_target_unit: usize,
    expected_blocker_body: usize,
    expected_blocker_unit: usize,
    target_pose: RigidBoundaryPose,
    blocker_pose: RigidBoundaryPose,
    target_translation_derivative: BoundaryKernelVector,
    target_angular_derivative: f64,
    blocker_translation_derivative: BoundaryKernelVector,
    blocker_angular_derivative: f64,
) -> Option<ResolvedStraightBoundaryIntersection> {
    let (
        blocker_body,
        blocker_unit,
        target_body,
        target_unit,
        target_start_local,
        target_end_local,
        blocker_start_local,
        blocker_end_local,
        target_parameter,
        blocker_parameter,
    ) = match event {
        crate::source_exposure::VisibilityCutEvent::StraightBoundaryIntersection {
            blocker_body_index,
            blocker_unit_index,
            target_body_index,
            target_unit_index,
            target_edge_start_local,
            target_edge_end_local,
            blocker_edge_start_local,
            blocker_edge_end_local,
            target_parameter,
            blocker_parameter,
            ..
        } => (
            *blocker_body_index,
            *blocker_unit_index,
            *target_body_index,
            *target_unit_index,
            *target_edge_start_local,
            *target_edge_end_local,
            *blocker_edge_start_local,
            *blocker_edge_end_local,
            *target_parameter,
            *blocker_parameter,
        ),
        _ => return None,
    };
    if target_body != expected_target_body
        || target_unit != expected_target_unit
        || blocker_body != expected_blocker_body
        || blocker_unit != expected_blocker_unit
        || !target_parameter.is_finite()
        || !blocker_parameter.is_finite()
        || !(1.0e-10..1.0 - 1.0e-10).contains(&target_parameter)
        || !(1.0e-10..1.0 - 1.0e-10).contains(&blocker_parameter)
    {
        return None;
    }

    let target_start = target_pose.transform_local_point(BoundaryKernelVector {
        x: target_start_local.0,
        y: target_start_local.1,
    })?;
    let target_end = target_pose.transform_local_point(BoundaryKernelVector {
        x: target_end_local.0,
        y: target_end_local.1,
    })?;
    let blocker_start = blocker_pose.transform_local_point(BoundaryKernelVector {
        x: blocker_start_local.0,
        y: blocker_start_local.1,
    })?;
    let blocker_end = blocker_pose.transform_local_point(BoundaryKernelVector {
        x: blocker_end_local.0,
        y: blocker_end_local.1,
    })?;

    let target_edge = BoundaryKernelVector {
        x: target_end.x - target_start.x,
        y: target_end.y - target_start.y,
    };
    let blocker_edge = BoundaryKernelVector {
        x: blocker_end.x - blocker_start.x,
        y: blocker_end.y - blocker_start.y,
    };
    let target_point = BoundaryKernelVector {
        x: target_start.x + target_parameter * target_edge.x,
        y: target_start.y + target_parameter * target_edge.y,
    };
    let blocker_point = BoundaryKernelVector {
        x: blocker_start.x + blocker_parameter * blocker_edge.x,
        y: blocker_start.y + blocker_parameter * blocker_edge.y,
    };
    let scale = [
        target_start.x.abs(),
        target_start.y.abs(),
        target_end.x.abs(),
        target_end.y.abs(),
        blocker_start.x.abs(),
        blocker_start.y.abs(),
        blocker_end.x.abs(),
        blocker_end.y.abs(),
        1.0,
    ]
    .into_iter()
    .fold(0.0_f64, f64::max);
    let consistency_tolerance = 1.0e-8 * scale;
    if (target_point.x - blocker_point.x).hypot(target_point.y - blocker_point.y)
        > consistency_tolerance
    {
        return None;
    }

    let derivative = rigid_straight_boundary_intersection_pose_derivative(
        target_start,
        target_end,
        blocker_start,
        blocker_end,
        target_pose.pivot,
        blocker_pose.pivot,
        target_translation_derivative,
        target_angular_derivative,
        blocker_translation_derivative,
        blocker_angular_derivative,
    )?;
    Some(ResolvedStraightBoundaryIntersection {
        target_start,
        target_end,
        blocker_start,
        blocker_end,
        intersection: BoundaryKernelVector {
            x: 0.5 * (target_point.x + blocker_point.x),
            y: 0.5 * (target_point.y + blocker_point.y),
        },
        target_parameter,
        blocker_parameter,
        derivative,
    })
}

/// Derivative of one strict-interior intersection between a moving straight
/// target edge and a moving circular blocker arc.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct CircleStraightIntersectionPoseDerivative {
    pub(crate) target_parameter_derivative: f64,
    pub(crate) local_arc_angle_derivative: f64,
    pub(crate) point_derivative: BoundaryKernelVector,
}

/// Differentiate |A + t(B-A) - C|² = R² for one generalized pose
/// coordinate. Tangencies and non-finite geometry fail closed because the
/// implicit derivative denominator vanishes at a tangent.
#[allow(dead_code)]
pub(crate) fn circle_straight_intersection_pose_derivative(
    target_start: BoundaryKernelVector,
    target_end: BoundaryKernelVector,
    circle_center: BoundaryKernelVector,
    radius: f64,
    target_parameter: f64,
    target_start_derivative: BoundaryKernelVector,
    target_end_derivative: BoundaryKernelVector,
    circle_center_derivative: BoundaryKernelVector,
    blocker_angular_derivative: f64,
) -> Option<CircleStraightIntersectionPoseDerivative> {
    if !radius.is_finite()
        || radius <= 0.0
        || !target_parameter.is_finite()
        || !(1.0e-10..1.0 - 1.0e-10).contains(&target_parameter)
        || !blocker_angular_derivative.is_finite()
    {
        return None;
    }
    let values = [
        target_start.x,
        target_start.y,
        target_end.x,
        target_end.y,
        circle_center.x,
        circle_center.y,
        target_start_derivative.x,
        target_start_derivative.y,
        target_end_derivative.x,
        target_end_derivative.y,
        circle_center_derivative.x,
        circle_center_derivative.y,
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return None;
    }
    let edge = BoundaryKernelVector {
        x: target_end.x - target_start.x,
        y: target_end.y - target_start.y,
    };
    let edge_derivative = BoundaryKernelVector {
        x: target_end_derivative.x - target_start_derivative.x,
        y: target_end_derivative.y - target_start_derivative.y,
    };
    let point = BoundaryKernelVector {
        x: target_start.x + target_parameter * edge.x,
        y: target_start.y + target_parameter * edge.y,
    };
    let radial = BoundaryKernelVector {
        x: point.x - circle_center.x,
        y: point.y - circle_center.y,
    };
    let radial_length = radial.norm();
    let edge_length = edge.norm();
    if !radial_length.is_finite()
        || !edge_length.is_finite()
        || edge_length <= f64::EPSILON
        || (radial_length - radius).abs() > 1.0e-8 * (radius + radial_length + 1.0)
    {
        return None;
    }
    let implicit_denominator = radial.dot(edge);
    let denominator_scale = radius * edge_length + 1.0;
    if !implicit_denominator.is_finite()
        || implicit_denominator.abs() <= 1.0e-10 * denominator_scale
    {
        return None;
    }
    let fixed_parameter_point_derivative = BoundaryKernelVector {
        x: target_start_derivative.x - circle_center_derivative.x
            + target_parameter * edge_derivative.x,
        y: target_start_derivative.y - circle_center_derivative.y
            + target_parameter * edge_derivative.y,
    };
    let target_parameter_derivative =
        -radial.dot(fixed_parameter_point_derivative) / implicit_denominator;
    let point_derivative = BoundaryKernelVector {
        x: target_start_derivative.x
            + target_parameter_derivative * edge.x
            + target_parameter * edge_derivative.x,
        y: target_start_derivative.y
            + target_parameter_derivative * edge.y
            + target_parameter * edge_derivative.y,
    };
    let relative_point_derivative = BoundaryKernelVector {
        x: point_derivative.x - circle_center_derivative.x,
        y: point_derivative.y - circle_center_derivative.y,
    };
    let world_angle_derivative = (radial.x * relative_point_derivative.y
        - radial.y * relative_point_derivative.x)
        / (radius * radius);
    let local_arc_angle_derivative = world_angle_derivative - blocker_angular_derivative;
    [
        target_parameter_derivative,
        point_derivative.x,
        point_derivative.y,
        local_arc_angle_derivative,
    ]
    .iter()
    .all(|value| value.is_finite())
    .then_some(CircleStraightIntersectionPoseDerivative {
        target_parameter_derivative,
        local_arc_angle_derivative,
        point_derivative,
    })
}

/// Resolve a retained circle/straight event from its local target-edge and
/// blocker-arc provenance. The arc's local angular span is checked before the
/// exact implicit derivative is returned; stale or tangent events fall back.
#[allow(dead_code)]
pub(crate) fn resolve_circle_straight_intersection_event(
    event: &crate::source_exposure::VisibilityCutEvent,
    expected_target_body: usize,
    expected_target_unit: usize,
    expected_blocker_body: usize,
    expected_blocker_unit: usize,
    target_pose: RigidBoundaryPose,
    blocker_pose: RigidBoundaryPose,
    target_translation_derivative: BoundaryKernelVector,
    target_angular_derivative: f64,
    blocker_translation_derivative: BoundaryKernelVector,
    blocker_angular_derivative: f64,
) -> Option<(
    BoundaryKernelVector,
    CircleStraightIntersectionPoseDerivative,
)> {
    let (
        blocker_body,
        blocker_unit,
        target_body,
        target_unit,
        target_start_local,
        target_end_local,
        circle_center_local,
        radius,
        arc_start,
        arc_sweep,
        target_parameter,
        arc_angle_local,
    ) = match event {
        crate::source_exposure::VisibilityCutEvent::CircleStraightIntersection {
            blocker_body_index,
            blocker_unit_index,
            target_body_index,
            target_unit_index,
            target_edge_start_local,
            target_edge_end_local,
            blocker_arc_center_local,
            blocker_arc_radius,
            blocker_arc_start_local,
            blocker_arc_sweep,
            target_parameter,
            arc_angle_local,
            ..
        } => (
            *blocker_body_index,
            *blocker_unit_index,
            *target_body_index,
            *target_unit_index,
            *target_edge_start_local,
            *target_edge_end_local,
            *blocker_arc_center_local,
            *blocker_arc_radius,
            *blocker_arc_start_local,
            *blocker_arc_sweep,
            *target_parameter,
            *arc_angle_local,
        ),
        _ => return None,
    };
    if blocker_body != expected_blocker_body
        || blocker_unit != expected_blocker_unit
        || target_body != expected_target_body
        || target_unit != expected_target_unit
        || !arc_start.is_finite()
        || !arc_sweep.is_finite()
        || arc_sweep <= 0.0
        || arc_sweep > std::f64::consts::TAU
        || !arc_angle_local.is_finite()
    {
        return None;
    }
    let target_start = target_pose.transform_local_point(BoundaryKernelVector {
        x: target_start_local.0,
        y: target_start_local.1,
    })?;
    let target_end = target_pose.transform_local_point(BoundaryKernelVector {
        x: target_end_local.0,
        y: target_end_local.1,
    })?;
    let circle_center = blocker_pose.transform_local_point(BoundaryKernelVector {
        x: circle_center_local.0,
        y: circle_center_local.1,
    })?;
    let target_edge = BoundaryKernelVector {
        x: target_end.x - target_start.x,
        y: target_end.y - target_start.y,
    };
    let point = BoundaryKernelVector {
        x: target_start.x + target_parameter * target_edge.x,
        y: target_start.y + target_parameter * target_edge.y,
    };
    let radial = BoundaryKernelVector {
        x: point.x - circle_center.x,
        y: point.y - circle_center.y,
    };
    let actual_radius = radial.norm();
    if !actual_radius.is_finite()
        || (actual_radius - radius).abs() > 1.0e-8 * (actual_radius + radius + 1.0)
    {
        return None;
    }
    let world_angle = radial.y.atan2(radial.x);
    let actual_local_angle =
        (world_angle - blocker_pose.rotation_radians).rem_euclid(std::f64::consts::TAU);
    let angle_difference = (actual_local_angle - arc_angle_local + std::f64::consts::PI)
        .rem_euclid(std::f64::consts::TAU)
        - std::f64::consts::PI;
    let arc_offset = (actual_local_angle - arc_start).rem_euclid(std::f64::consts::TAU);
    if angle_difference.abs() > 1.0e-8 || arc_offset > arc_sweep + 1.0e-8 {
        return None;
    }

    let target_start_derivative = rigid_pose_point_derivative(
        target_start,
        target_pose.pivot,
        target_translation_derivative,
        target_angular_derivative,
    )?;
    let target_end_derivative = rigid_pose_point_derivative(
        target_end,
        target_pose.pivot,
        target_translation_derivative,
        target_angular_derivative,
    )?;
    let circle_center_derivative = rigid_pose_point_derivative(
        circle_center,
        blocker_pose.pivot,
        blocker_translation_derivative,
        blocker_angular_derivative,
    )?;
    let derivative = circle_straight_intersection_pose_derivative(
        target_start,
        target_end,
        circle_center,
        radius,
        target_parameter,
        target_start_derivative,
        target_end_derivative,
        circle_center_derivative,
        blocker_angular_derivative,
    )?;
    Some((point, derivative))
}

/// Displacement and generalized-coordinate derivative bounds for one
/// rigidly attached circular-arc endpoint.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct RigidArcEndpointMotionBound {
    pub(crate) displacement: f64,
    pub(crate) derivative: f64,
}

/// Bound a curved endpoint's world displacement and derivative from the
/// owning rigid body's translation/rotation envelopes. The endpoint's lever
/// arm is measured from the body's rigid pivot, so angular motion is included
/// without approximating the arc as a straight chord.
#[allow(dead_code)]
pub(crate) fn rigid_circle_arc_endpoint_motion_bound(
    local_center: BoundaryKernelVector,
    radius: f64,
    local_angle: f64,
    translation_derivative: BoundaryKernelVector,
    angular_derivative: f64,
    translation_displacement: f64,
    angular_displacement: f64,
) -> Option<RigidArcEndpointMotionBound> {
    let values = [
        local_center.x,
        local_center.y,
        radius,
        local_angle,
        translation_derivative.x,
        translation_derivative.y,
        angular_derivative,
        translation_displacement,
        angular_displacement,
    ];
    if values.iter().any(|value| !value.is_finite())
        || radius <= 0.0
        || translation_displacement < 0.0
        || angular_displacement < 0.0
    {
        return None;
    }
    let lever = BoundaryKernelVector {
        x: local_center.x + radius * local_angle.cos(),
        y: local_center.y + radius * local_angle.sin(),
    };
    let lever_length = lever.norm();
    let derivative = translation_derivative.norm() + angular_derivative.abs() * lever_length;
    let displacement = translation_displacement + angular_displacement * lever_length;
    let roundoff_guard = 128.0 * f64::EPSILON * (derivative + displacement + lever_length + 1.0);
    (lever_length.is_finite()
        && derivative.is_finite()
        && displacement.is_finite()
        && roundoff_guard.is_finite())
    .then_some(RigidArcEndpointMotionBound {
        displacement: displacement + roundoff_guard,
        derivative: derivative + roundoff_guard,
    })
}

/// Conservative pose-cell movement envelope for a circle/straight cut.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct CircleStraightIntersectionMotionBound {
    pub(crate) target_parameter_change: f64,
    pub(crate) local_arc_angle_change: f64,
}

/// Bound target-edge parameter and local arc-angle movement while a rigid
/// line and circle move inside endpoint/center displacement disks. The signed
/// line-to-center distance must remain separated from the circle radius, which
/// proves the intersection stays away from tangency. Arc endpoints must also
/// remain separated from the moving intersection angle.
#[allow(dead_code)]
pub(crate) fn circle_straight_intersection_motion_bound(
    target_start: BoundaryKernelVector,
    target_end: BoundaryKernelVector,
    circle_center: BoundaryKernelVector,
    radius: f64,
    target_parameter: f64,
    arc_start_local: f64,
    arc_sweep: f64,
    arc_angle_local: f64,
    target_start_displacement: f64,
    target_end_displacement: f64,
    circle_center_displacement: f64,
    blocker_angular_displacement: f64,
) -> Option<CircleStraightIntersectionMotionBound> {
    let displacements = [
        target_start_displacement,
        target_end_displacement,
        circle_center_displacement,
        blocker_angular_displacement,
    ];
    if displacements
        .iter()
        .any(|value| !value.is_finite() || *value < 0.0)
        || !radius.is_finite()
        || radius <= 0.0
        || !target_parameter.is_finite()
        || !(1.0e-10..1.0 - 1.0e-10).contains(&target_parameter)
        || !arc_start_local.is_finite()
        || !arc_sweep.is_finite()
        || arc_sweep <= 0.0
        || arc_sweep > std::f64::consts::TAU
        || !arc_angle_local.is_finite()
        || [
            target_start.x,
            target_start.y,
            target_end.x,
            target_end.y,
            circle_center.x,
            circle_center.y,
        ]
        .iter()
        .any(|value| !value.is_finite())
    {
        return None;
    }
    let edge = BoundaryKernelVector {
        x: target_end.x - target_start.x,
        y: target_end.y - target_start.y,
    };
    let offset = BoundaryKernelVector {
        x: circle_center.x - target_start.x,
        y: circle_center.y - target_start.y,
    };
    let edge_length = edge.norm();
    let offset_length = offset.norm();
    let intersection = BoundaryKernelVector {
        x: target_start.x + target_parameter * edge.x,
        y: target_start.y + target_parameter * edge.y,
    };
    let radial = BoundaryKernelVector {
        x: intersection.x - circle_center.x,
        y: intersection.y - circle_center.y,
    };
    let radial_length = radial.norm();
    if !edge_length.is_finite()
        || !offset_length.is_finite()
        || !radial_length.is_finite()
        || edge_length <= f64::EPSILON
        || (radial_length - radius).abs() > 1.0e-8 * (radial_length + radius + 1.0)
    {
        return None;
    }
    let cross = |left: BoundaryKernelVector, right: BoundaryKernelVector| {
        left.x * right.y - left.y * right.x
    };
    let edge_displacement = target_start_displacement + target_end_displacement;
    let offset_displacement = target_start_displacement + circle_center_displacement;
    let minimum_edge_length = edge_length - edge_displacement;
    if !minimum_edge_length.is_finite() || minimum_edge_length <= f64::EPSILON {
        return None;
    }
    let signed_distance_numerator = cross(edge, offset);
    let signed_distance = signed_distance_numerator / edge_length;
    let numerator_change = edge_length * offset_displacement
        + offset_length * edge_displacement
        + edge_displacement * offset_displacement;
    let signed_distance_change = numerator_change / minimum_edge_length
        + signed_distance_numerator.abs() * edge_displacement / (edge_length * minimum_edge_length);
    let maximum_signed_distance = signed_distance.abs() + signed_distance_change;
    if !signed_distance.is_finite()
        || !signed_distance_change.is_finite()
        || !maximum_signed_distance.is_finite()
        || maximum_signed_distance >= radius
    {
        return None;
    }
    let minimum_implicit_denominator = minimum_edge_length
        * (radius * radius - maximum_signed_distance * maximum_signed_distance).sqrt();
    if !minimum_implicit_denominator.is_finite() || minimum_implicit_denominator <= f64::EPSILON {
        return None;
    }
    let target_parameter_change = radius
        * (target_start_displacement + circle_center_displacement + edge_displacement)
        / minimum_implicit_denominator;
    let target_parameter_guard =
        128.0 * f64::EPSILON * (target_parameter_change + target_parameter.abs() + 1.0);
    let target_parameter_change = target_parameter_change + target_parameter_guard;
    const PARAMETER_MARGIN: f64 = 1.0e-10;
    if !target_parameter_change.is_finite()
        || target_parameter_change >= target_parameter - PARAMETER_MARGIN
        || target_parameter_change >= 1.0 - PARAMETER_MARGIN - target_parameter
    {
        return None;
    }

    let point_displacement = target_start_displacement
        + target_parameter_change * edge_length
        + target_parameter * edge_displacement
        + target_parameter_change * edge_displacement;
    let local_arc_angle_change =
        (point_displacement + circle_center_displacement) / radius + blocker_angular_displacement;
    let arc_offset = (arc_angle_local - arc_start_local).rem_euclid(std::f64::consts::TAU);
    if !point_displacement.is_finite()
        || !local_arc_angle_change.is_finite()
        || (arc_sweep < std::f64::consts::TAU - 1.0e-10
            && (local_arc_angle_change >= arc_offset
                || local_arc_angle_change >= arc_sweep - arc_offset))
    {
        return None;
    }
    Some(CircleStraightIntersectionMotionBound {
        target_parameter_change,
        local_arc_angle_change,
    })
}

/// Exact pose derivative of a rigidly attached circular-arc endpoint. The
/// local angle and center are intrinsic to the owning constituent.
#[allow(dead_code)]
pub(crate) fn rigid_circle_arc_endpoint_pose_derivative(
    local_center: BoundaryKernelVector,
    radius: f64,
    local_angle: f64,
    pose: RigidBoundaryPose,
    translation_derivative: BoundaryKernelVector,
    angular_derivative: f64,
) -> Option<(BoundaryKernelVector, BoundaryKernelVector)> {
    if !radius.is_finite() || radius <= 0.0 || !local_angle.is_finite() {
        return None;
    }
    let local_point = BoundaryKernelVector {
        x: local_center.x + radius * local_angle.cos(),
        y: local_center.y + radius * local_angle.sin(),
    };
    let world_point = pose.transform_local_point(local_point)?;
    let derivative = rigid_pose_point_derivative(
        world_point,
        pose.pivot,
        translation_derivative,
        angular_derivative,
    )?;
    Some((world_point, derivative))
}

/// A visibility endpoint resolved to one world point and its exact
/// generalized-coordinate derivative.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ResolvedBoundaryEndpointPoseDerivative {
    pub(crate) point: BoundaryKernelVector,
    pub(crate) derivative: BoundaryKernelVector,
}

/// Consume the typed endpoint provenance emitted by the exact visibility
/// solver. Target vertices and unique straight/circle intersection events are
/// differentiated through their owning rigid poses; unresolved or unsupported
/// cuts remain on exact evaluation/subdivision.
#[allow(dead_code)]
pub(crate) fn resolve_boundary_endpoint_pose_derivative(
    provenance: &crate::source_exposure::BoundaryEndpointProvenance,
    expected_target_body: usize,
    expected_target_unit: usize,
    expected_blocker_body: usize,
    expected_blocker_unit: usize,
    target_pose: RigidBoundaryPose,
    blocker_pose: RigidBoundaryPose,
    target_translation_derivative: BoundaryKernelVector,
    target_angular_derivative: f64,
    blocker_translation_derivative: BoundaryKernelVector,
    blocker_angular_derivative: f64,
) -> Option<ResolvedBoundaryEndpointPoseDerivative> {
    match provenance {
        crate::source_exposure::BoundaryEndpointProvenance::RigidTargetVertex {
            body_index,
            unit_index,
            local_point,
            ..
        } if *body_index == expected_target_body && *unit_index == expected_target_unit => {
            let point = target_pose.transform_local_point(BoundaryKernelVector {
                x: local_point.0,
                y: local_point.1,
            })?;
            let derivative = rigid_pose_point_derivative(
                point,
                target_pose.pivot,
                target_translation_derivative,
                target_angular_derivative,
            )?;
            Some(ResolvedBoundaryEndpointPoseDerivative { point, derivative })
        }
        crate::source_exposure::BoundaryEndpointProvenance::RigidVisibilityIntersection {
            parameter,
            event,
        } => {
            let event_parameter = match event {
                crate::source_exposure::VisibilityCutEvent::StraightBoundaryIntersection {
                    target_parameter,
                    ..
                }
                | crate::source_exposure::VisibilityCutEvent::CircleStraightIntersection {
                    target_parameter,
                    ..
                } => *target_parameter,
                _ => return None,
            };
            if !parameter.is_finite() || (event_parameter - *parameter).abs() > 1.0e-10 {
                return None;
            }
            match event {
                crate::source_exposure::VisibilityCutEvent::StraightBoundaryIntersection {
                    ..
                } => {
                    let resolved = resolve_straight_boundary_intersection_event(
                        event,
                        expected_target_body,
                        expected_target_unit,
                        expected_blocker_body,
                        expected_blocker_unit,
                        target_pose,
                        blocker_pose,
                        target_translation_derivative,
                        target_angular_derivative,
                        blocker_translation_derivative,
                        blocker_angular_derivative,
                    )?;
                    Some(ResolvedBoundaryEndpointPoseDerivative {
                        point: resolved.intersection,
                        derivative: resolved.derivative.point_derivative,
                    })
                }
                crate::source_exposure::VisibilityCutEvent::CircleStraightIntersection {
                    ..
                } => {
                    let (point, derivative) = resolve_circle_straight_intersection_event(
                        event,
                        expected_target_body,
                        expected_target_unit,
                        expected_blocker_body,
                        expected_blocker_unit,
                        target_pose,
                        blocker_pose,
                        target_translation_derivative,
                        target_angular_derivative,
                        blocker_translation_derivative,
                        blocker_angular_derivative,
                    )?;
                    Some(ResolvedBoundaryEndpointPoseDerivative {
                        point,
                        derivative: derivative.point_derivative,
                    })
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// Conservative absolute movement bounds for the two parameters of one
/// straight/straight intersection over a pose cell.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct StraightIntersectionParameterMotionBound {
    pub(crate) target_parameter_change: f64,
    pub(crate) blocker_parameter_change: f64,
}

/// Bound how far the target-edge and blocker-edge intersection parameters can
/// move when each endpoint remains inside its supplied displacement disk.
/// The denominator envelope must stay strictly nonzero, and both parameter
/// envelopes must remain strictly inside (0, 1); otherwise subdivide or use
/// exact visibility evaluation.
#[allow(dead_code)]
pub(crate) fn straight_intersection_parameter_motion_bound(
    target_start: BoundaryKernelVector,
    target_end: BoundaryKernelVector,
    blocker_start: BoundaryKernelVector,
    blocker_end: BoundaryKernelVector,
    target_start_displacement: f64,
    target_end_displacement: f64,
    blocker_start_displacement: f64,
    blocker_end_displacement: f64,
) -> Option<StraightIntersectionParameterMotionBound> {
    let motions = [
        target_start_displacement,
        target_end_displacement,
        blocker_start_displacement,
        blocker_end_displacement,
    ];
    if motions
        .iter()
        .any(|value| !value.is_finite() || *value < 0.0)
        || [
            target_start.x,
            target_start.y,
            target_end.x,
            target_end.y,
            blocker_start.x,
            blocker_start.y,
            blocker_end.x,
            blocker_end.y,
        ]
        .iter()
        .any(|value| !value.is_finite())
    {
        return None;
    }
    let target_edge = BoundaryKernelVector {
        x: target_end.x - target_start.x,
        y: target_end.y - target_start.y,
    };
    let blocker_edge = BoundaryKernelVector {
        x: blocker_end.x - blocker_start.x,
        y: blocker_end.y - blocker_start.y,
    };
    let offset = BoundaryKernelVector {
        x: blocker_start.x - target_start.x,
        y: blocker_start.y - target_start.y,
    };
    let target_length = target_edge.norm();
    let blocker_length = blocker_edge.norm();
    let offset_length = offset.norm();
    if !target_length.is_finite()
        || !blocker_length.is_finite()
        || !offset_length.is_finite()
        || target_length <= f64::EPSILON
        || blocker_length <= f64::EPSILON
    {
        return None;
    }
    let cross = |left: BoundaryKernelVector, right: BoundaryKernelVector| {
        left.x * right.y - left.y * right.x
    };
    let denominator = cross(target_edge, blocker_edge);
    let denominator_change = target_length
        * (blocker_start_displacement + blocker_end_displacement)
        + blocker_length * (target_start_displacement + target_end_displacement)
        + (target_start_displacement + target_end_displacement)
            * (blocker_start_displacement + blocker_end_displacement);
    let denominator_minimum = denominator.abs() - denominator_change;
    let denominator_scale = target_length * blocker_length + denominator_change + 1.0;
    if !denominator_minimum.is_finite()
        || denominator_minimum <= 128.0 * f64::EPSILON * denominator_scale
    {
        return None;
    }

    let target_numerator = cross(offset, blocker_edge);
    let blocker_numerator = cross(offset, target_edge);
    let target_parameter = target_numerator / denominator;
    let blocker_parameter = blocker_numerator / denominator;
    const PARAMETER_MARGIN: f64 = 1.0e-10;
    if !target_parameter.is_finite()
        || !blocker_parameter.is_finite()
        || !(PARAMETER_MARGIN..1.0 - PARAMETER_MARGIN).contains(&target_parameter)
        || !(PARAMETER_MARGIN..1.0 - PARAMETER_MARGIN).contains(&blocker_parameter)
    {
        return None;
    }

    let offset_change = target_start_displacement + blocker_start_displacement;
    let blocker_edge_change = blocker_start_displacement + blocker_end_displacement;
    let target_edge_change = target_start_displacement + target_end_displacement;
    let target_numerator_change = offset_length * blocker_edge_change
        + blocker_length * offset_change
        + offset_change * blocker_edge_change;
    let blocker_numerator_change = offset_length * target_edge_change
        + target_length * offset_change
        + offset_change * target_edge_change;
    let target_parameter_change = (target_numerator_change
        + target_parameter.abs() * denominator_change)
        / denominator_minimum;
    let blocker_parameter_change = (blocker_numerator_change
        + blocker_parameter.abs() * denominator_change)
        / denominator_minimum;
    let target_guard =
        128.0 * f64::EPSILON * (target_parameter_change + target_parameter.abs() + 1.0);
    let blocker_guard =
        128.0 * f64::EPSILON * (blocker_parameter_change + blocker_parameter.abs() + 1.0);
    let target_parameter_change = target_parameter_change + target_guard;
    let blocker_parameter_change = blocker_parameter_change + blocker_guard;
    if !target_parameter_change.is_finite()
        || !blocker_parameter_change.is_finite()
        || target_parameter_change >= target_parameter - PARAMETER_MARGIN
        || target_parameter_change >= 1.0 - PARAMETER_MARGIN - target_parameter
        || blocker_parameter_change >= blocker_parameter - PARAMETER_MARGIN
        || blocker_parameter_change >= 1.0 - PARAMETER_MARGIN - blocker_parameter
    {
        return None;
    }
    Some(StraightIntersectionParameterMotionBound {
        target_parameter_change,
        blocker_parameter_change,
    })
}

/// Midpoint and exact point-source projected-extent derivatives for one
/// already-corresponding straight exposed interval. The interval normal is
/// transported from its edge orientation; edge-on transitions fail closed.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct StraightIntervalPoseDerivative {
    pub(crate) midpoint_derivative: BoundaryKernelVector,
    pub(crate) projected_extent: f64,
    pub(crate) projected_extent_derivative: f64,
}

/// Conservative magnitude bound for the straight-interval projected-extent
/// derivative over a smooth pose cell. Source/endpoint displacement envelopes
/// preserve front-facing separation; derivative bounds apply to the same
/// generalized coordinate throughout the cell.
#[allow(dead_code)]
pub(crate) fn straight_interval_projected_extent_derivative_bound(
    source: BoundaryKernelVector,
    edge_start: BoundaryKernelVector,
    edge_end: BoundaryKernelVector,
    outward_normal: BoundaryKernelVector,
    source_displacement: f64,
    edge_start_displacement: f64,
    edge_end_displacement: f64,
    source_derivative_bound: f64,
    edge_start_derivative_bound: f64,
    edge_end_derivative_bound: f64,
) -> Option<f64> {
    let values = [
        source.x,
        source.y,
        edge_start.x,
        edge_start.y,
        edge_end.x,
        edge_end.y,
        outward_normal.x,
        outward_normal.y,
        source_displacement,
        edge_start_displacement,
        edge_end_displacement,
        source_derivative_bound,
        edge_start_derivative_bound,
        edge_end_derivative_bound,
    ];
    if values.iter().any(|value| !value.is_finite()) || values[8..].iter().any(|value| *value < 0.0)
    {
        return None;
    }
    let edge = BoundaryKernelVector {
        x: edge_end.x - edge_start.x,
        y: edge_end.y - edge_start.y,
    };
    let length = edge.norm();
    let normal_length = outward_normal.norm();
    if !length.is_finite() || length <= f64::EPSILON || (normal_length - 1.0).abs() > 1.0e-9 {
        return None;
    }
    let tangent = BoundaryKernelVector {
        x: edge.x / length,
        y: edge.y / length,
    };
    let perpendicular = BoundaryKernelVector {
        x: -tangent.y,
        y: tangent.x,
    };
    let orientation = outward_normal.dot(perpendicular);
    if !orientation.is_finite() || orientation.abs() < 1.0 - 1.0e-9 {
        return None;
    }

    let edge_displacement = edge_start_displacement + edge_end_displacement;
    let minimum_length = length - edge_displacement;
    if !minimum_length.is_finite() || minimum_length <= f64::EPSILON {
        return None;
    }
    let relative_displacement = source_displacement + edge_start_displacement;
    let source_offset = BoundaryKernelVector {
        x: source.x - edge_start.x,
        y: source.y - edge_start.y,
    };
    let radius_max = source_offset.norm() + relative_displacement;
    let normal_distance = source_offset.dot(outward_normal);
    if !radius_max.is_finite() || !normal_distance.is_finite() || normal_distance <= 0.0 {
        return None;
    }
    let normal_change = (2.0 * edge_displacement / minimum_length).min(2.0);
    let minimum_normal_distance =
        normal_distance - relative_displacement - radius_max * normal_change;
    if !minimum_normal_distance.is_finite() || minimum_normal_distance <= f64::EPSILON {
        return None;
    }
    let along = source_offset.dot(tangent);
    let maximum_absolute_along = along.abs() + relative_displacement + radius_max * normal_change;
    let maximum_length = length + edge_displacement;
    let normal_partial_bound = (maximum_absolute_along / minimum_normal_distance).asinh()
        + ((maximum_length + maximum_absolute_along) / minimum_normal_distance).asinh()
        + 2.0;

    let edge_derivative_bound = edge_start_derivative_bound + edge_end_derivative_bound;
    let relative_derivative_bound = source_derivative_bound + edge_start_derivative_bound;
    let tangent_derivative_bound = 2.0 * edge_derivative_bound / minimum_length;
    let along_derivative_bound = relative_derivative_bound + radius_max * tangent_derivative_bound;
    let normal_derivative_bound = along_derivative_bound;
    let result = edge_derivative_bound
        + 2.0 * along_derivative_bound
        + normal_partial_bound * normal_derivative_bound;
    let roundoff_guard = 128.0 * f64::EPSILON * (result + radius_max + maximum_length + 1.0);
    (result.is_finite() && roundoff_guard.is_finite()).then_some(result + roundoff_guard)
}

/// Compose straight-interval midpoint and projected-extent derivatives
/// into the sample-coordinate input consumed by the discrete chain-rule
/// evaluator. The signed spectral coefficient is treated as pose-invariant.
#[allow(dead_code)]
pub(crate) fn straight_interval_sample_pose_derivative(
    source: BoundaryKernelVector,
    edge_start: BoundaryKernelVector,
    edge_end: BoundaryKernelVector,
    outward_normal: BoundaryKernelVector,
    source_derivative: BoundaryKernelVector,
    edge_start_derivative: BoundaryKernelVector,
    edge_end_derivative: BoundaryKernelVector,
    source_displacement: f64,
    edge_start_displacement: f64,
    edge_end_displacement: f64,
    source_derivative_bound: f64,
    edge_start_derivative_bound: f64,
    edge_end_derivative_bound: f64,
    coordinate_displacement: f64,
    signed_weight_scale: f64,
) -> Option<BoundarySamplePoseDerivative> {
    if !coordinate_displacement.is_finite()
        || coordinate_displacement < 0.0
        || !signed_weight_scale.is_finite()
    {
        return None;
    }
    let geometry = straight_interval_pose_derivative(
        source,
        edge_start,
        edge_end,
        outward_normal,
        source_derivative,
        edge_start_derivative,
        edge_end_derivative,
    )?;
    let extent_derivative_bound = straight_interval_projected_extent_derivative_bound(
        source,
        edge_start,
        edge_end,
        outward_normal,
        source_displacement,
        edge_start_displacement,
        edge_end_displacement,
        source_derivative_bound,
        edge_start_derivative_bound,
        edge_end_derivative_bound,
    )?;
    let position_derivative_bound = 0.5 * (edge_start_derivative_bound + edge_end_derivative_bound);
    let position_displacement_bound = 0.5 * (edge_start_displacement + edge_end_displacement);
    let absolute_weight_derivative_bound = signed_weight_scale.abs() * extent_derivative_bound;
    let absolute_weight_change_bound = absolute_weight_derivative_bound * coordinate_displacement;
    let result = BoundarySamplePoseDerivative {
        position_derivative: geometry.midpoint_derivative,
        weight_derivative: signed_weight_scale * geometry.projected_extent_derivative,
        position_derivative_bound,
        absolute_weight_derivative_bound,
        position_displacement_bound,
        absolute_weight_change_bound,
    };
    [
        result.position_derivative.x,
        result.position_derivative.y,
        result.weight_derivative,
        result.position_derivative_bound,
        result.absolute_weight_derivative_bound,
        result.position_displacement_bound,
        result.absolute_weight_change_bound,
    ]
    .iter()
    .all(|value| value.is_finite())
    .then_some(result)
}

/// Differentiate the closed-form point-source projected extent with respect
/// to source and both edge endpoints. This includes rigid edge translation
/// and rotation; it does not discover or differentiate visibility cuts.
#[allow(dead_code)]
pub(crate) fn straight_interval_pose_derivative(
    source: BoundaryKernelVector,
    edge_start: BoundaryKernelVector,
    edge_end: BoundaryKernelVector,
    outward_normal: BoundaryKernelVector,
    source_derivative: BoundaryKernelVector,
    edge_start_derivative: BoundaryKernelVector,
    edge_end_derivative: BoundaryKernelVector,
) -> Option<StraightIntervalPoseDerivative> {
    let points = [
        source,
        edge_start,
        edge_end,
        outward_normal,
        source_derivative,
        edge_start_derivative,
        edge_end_derivative,
    ];
    if points
        .iter()
        .any(|point| !point.x.is_finite() || !point.y.is_finite())
    {
        return None;
    }
    let edge = BoundaryKernelVector {
        x: edge_end.x - edge_start.x,
        y: edge_end.y - edge_start.y,
    };
    let edge_rate = BoundaryKernelVector {
        x: edge_end_derivative.x - edge_start_derivative.x,
        y: edge_end_derivative.y - edge_start_derivative.y,
    };
    let length = edge.norm();
    let normal_length = outward_normal.norm();
    if !length.is_finite() || length <= f64::EPSILON || (normal_length - 1.0).abs() > 1.0e-9 {
        return None;
    }
    let tangent = BoundaryKernelVector {
        x: edge.x / length,
        y: edge.y / length,
    };
    let perpendicular = BoundaryKernelVector {
        x: -tangent.y,
        y: tangent.x,
    };
    let orientation = outward_normal.dot(perpendicular);
    if !orientation.is_finite() || orientation.abs() < 1.0 - 1.0e-9 {
        return None;
    }
    let length_rate = tangent.dot(edge_rate);
    let tangent_rate = BoundaryKernelVector {
        x: (edge_rate.x - tangent.x * length_rate) / length,
        y: (edge_rate.y - tangent.y * length_rate) / length,
    };
    let normal_rate = BoundaryKernelVector {
        x: -orientation.signum() * tangent_rate.y,
        y: orientation.signum() * tangent_rate.x,
    };
    let source_offset = BoundaryKernelVector {
        x: source.x - edge_start.x,
        y: source.y - edge_start.y,
    };
    let relative_rate = BoundaryKernelVector {
        x: source_derivative.x - edge_start_derivative.x,
        y: source_derivative.y - edge_start_derivative.y,
    };
    let along = source_offset.dot(tangent);
    let normal_distance = source_offset.dot(outward_normal);
    let along_rate = relative_rate.dot(tangent) + source_offset.dot(tangent_rate);
    let normal_rate_distance = relative_rate.dot(outward_normal) + source_offset.dot(normal_rate);
    if !along.is_finite() || !normal_distance.is_finite() {
        return None;
    }
    let midpoint_derivative = BoundaryKernelVector {
        x: 0.5 * (edge_start_derivative.x + edge_end_derivative.x),
        y: 0.5 * (edge_start_derivative.y + edge_end_derivative.y),
    };
    if normal_distance < -1.0e-12 {
        return Some(StraightIntervalPoseDerivative {
            midpoint_derivative,
            projected_extent: 0.0,
            projected_extent_derivative: 0.0,
        });
    }
    if normal_distance <= 1.0e-12 {
        return None;
    }

    let remaining = length - along;
    let first = normal_distance.hypot(along);
    let second = normal_distance.hypot(remaining);
    if !first.is_finite() || !second.is_finite() || first <= 0.0 || second <= 0.0 {
        return None;
    }
    let projected_extent = normal_distance
        * ((remaining / normal_distance).asinh() + (along / normal_distance).asinh());
    let derivative_length = normal_distance / second;
    let derivative_along = normal_distance / first - normal_distance / second;
    let derivative_normal = (remaining / normal_distance).asinh()
        + (along / normal_distance).asinh()
        - remaining / second
        - along / first;
    let projected_extent_derivative = derivative_length * length_rate
        + derivative_along * along_rate
        + derivative_normal * normal_rate_distance;
    [
        projected_extent,
        projected_extent_derivative,
        midpoint_derivative.x,
        midpoint_derivative.y,
    ]
    .iter()
    .all(|value| value.is_finite())
    .then_some(StraightIntervalPoseDerivative {
        midpoint_derivative,
        projected_extent,
        projected_extent_derivative,
    })
}

/// Bound movement of a ray/edge cut while it remains a strict interior
/// intersection and the critical point remains beyond the receiver edge.
///
/// The exact construction is s + u(v-s) = a + t(b-a). A denominator interval
/// excluding zero and strict t/u margins are required; near-parallel rays,
/// endpoint events, and cut-order transitions fail closed.
#[allow(dead_code)]
pub(crate) fn ray_edge_intersection_displacement_bound(
    source: BoundaryKernelVector,
    critical_point: BoundaryKernelVector,
    edge_start: BoundaryKernelVector,
    edge_end: BoundaryKernelVector,
    motion: RayEdgeIntersectionMotion,
) -> Option<f64> {
    let points = [source, critical_point, edge_start, edge_end];
    if points
        .iter()
        .any(|point| !point.x.is_finite() || !point.y.is_finite())
    {
        return None;
    }
    let bounds = [
        motion.source_displacement,
        motion.critical_point_displacement,
        motion.edge_start_displacement,
        motion.edge_end_displacement,
    ];
    if bounds
        .iter()
        .any(|bound| !bound.is_finite() || *bound < 0.0)
    {
        return None;
    }

    let q = BoundaryKernelVector {
        x: edge_start.x - source.x,
        y: edge_start.y - source.y,
    };
    let r = BoundaryKernelVector {
        x: critical_point.x - source.x,
        y: critical_point.y - source.y,
    };
    let e = BoundaryKernelVector {
        x: edge_end.x - edge_start.x,
        y: edge_end.y - edge_start.y,
    };
    let cross = |left: BoundaryKernelVector, right: BoundaryKernelVector| {
        left.x * right.y - left.y * right.x
    };
    let denominator = cross(e, r);
    let numerator_t = -cross(q, r);
    let numerator_u = -cross(q, e);
    if !denominator.is_finite() || !numerator_t.is_finite() || !numerator_u.is_finite() {
        return None;
    }
    let t = numerator_t / denominator;
    let u = numerator_u / denominator;
    if !t.is_finite()
        || !u.is_finite()
        || !(1.0e-10..1.0 - 1.0e-10).contains(&t)
        || u <= 1.0 + 1.0e-10
    {
        return None;
    }

    let delta_q = motion.edge_start_displacement + motion.source_displacement;
    let delta_r = motion.critical_point_displacement + motion.source_displacement;
    let delta_e = motion.edge_start_displacement + motion.edge_end_displacement;
    let q_norm = q.norm();
    let r_norm = r.norm();
    let e_norm = e.norm();
    let denominator_change = delta_e * r_norm + e_norm * delta_r + delta_e * delta_r;
    let denominator_min = denominator.abs() - denominator_change;
    if !denominator_min.is_finite() || denominator_min <= f64::EPSILON {
        return None;
    }

    let numerator_t_change = delta_q * r_norm + q_norm * delta_r + delta_q * delta_r;
    let numerator_u_change = delta_q * e_norm + q_norm * delta_e + delta_q * delta_e;
    let t_change = (numerator_t_change + t.abs() * denominator_change) / denominator_min;
    let u_change = (numerator_u_change + u.abs() * denominator_change) / denominator_min;
    if !t_change.is_finite()
        || !u_change.is_finite()
        || t_change >= t.min(1.0 - t)
        || u_change >= u - 1.0
    {
        return None;
    }

    // p = a + t e. Bound all products including simultaneous changes in t
    // and the edge vector.
    let point_change =
        motion.edge_start_displacement + t.abs() * delta_e + e_norm * t_change + delta_e * t_change;
    let roundoff_guard = 64.0 * f64::EPSILON * (point_change + q_norm + r_norm + e_norm + 1.0);
    let result = point_change + roundoff_guard;
    result.is_finite().then_some(result)
}

/// Certify that a supplied ordered list of visibility-cut parameters cannot
/// cross or reach an edge endpoint anywhere inside their individual motion
/// envelopes. This does not prove that the list contains every geometric cut.
#[allow(dead_code)]
pub(crate) fn certify_cut_parameter_order(cuts: &[(f64, f64)]) -> Option<()> {
    let mut previous_upper = 1.0e-10;
    for &(parameter, displacement) in cuts {
        if !parameter.is_finite()
            || !displacement.is_finite()
            || displacement < 0.0
            || parameter - displacement <= previous_upper
            || parameter + displacement >= 1.0 - 1.0e-10
        {
            return None;
        }
        previous_upper = parameter + displacement;
    }
    Some(())
}

/// Combined error certificate for one ordered pair over a pose cell whose
/// visibility topology and exact sample correspondence have been established
/// by the caller. The cut-order check verifies supplied cut envelopes; it does
/// not prove that every geometric cut was enumerated or blocker labels stayed
/// unchanged.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct BoundaryPoseCellError {
    pub(crate) potential: f64,
    pub(crate) source_force: f64,
    pub(crate) receiver_force: f64,
}

/// Composition of the sample-cluster Taylor remainder and the pose-driven
/// sample/weight-change envelope. Forces are center-translation derivatives
/// only; pose-chain-rule terms and torque are not included.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BoundaryPoseCellEvaluation {
    pub(crate) evaluation: BoundaryPotentialEvaluation,
    pub(crate) error: BoundaryPoseCellError,
}

/// Complete generalized-coordinate result for one smooth ordered-pair pose cell.
/// Each entry corresponds, in caller-supplied order, to one body translation or
/// rotation coordinate. Angular entries are torque derivatives; translational
/// entries are force derivatives. The caller must supply every participating
/// body's x, y, and angular coordinate, including occluders.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BoundaryPoseCellGeneralizedEvaluation {
    pub(crate) boundary: BoundaryPoseCellEvaluation,
    pub(crate) coordinates: Vec<BoundaryPoseDerivativeEvaluation>,
}

/// Validate supplied visibility-cut ordering and compose all currently
/// available scalar-potential and center-gradient error terms for one
/// source-specific ordered pair.
///
/// The caller must additionally establish complete cut enumeration, stable
/// blocker classification on every open interval, and one-to-one sample
/// correspondence. Each sample displacement must include rigid transport,
/// moving ray/edge intersections, and interval geometry. Each weight-change
/// bound must include projected-extent and signed spectral-coefficient
/// changes. Missing or invalid local certificates fail closed.
///
/// This is not a complete rigid-pose force/torque certificate.
#[allow(dead_code)]
pub(crate) fn certify_boundary_pose_cell(
    expansion: &BoundaryKernelExpansion,
    ordered_visibility_cuts: &[(f64, f64)],
    sample_motion: &[BoundarySampleMotionBound],
) -> Option<BoundaryPoseCellEvaluation> {
    certify_cut_parameter_order(ordered_visibility_cuts)?;
    let evaluation = expansion.evaluate()?;
    let geometry = expansion.bound_sample_geometry_change(sample_motion)?;
    let error = BoundaryPoseCellError {
        potential: evaluation.error.potential + geometry.potential,
        source_force: evaluation.error.source_force + geometry.source_force,
        receiver_force: evaluation.error.receiver_force + geometry.receiver_force,
    };
    [error.potential, error.source_force, error.receiver_force]
        .iter()
        .all(|bound| bound.is_finite())
        .then_some(BoundaryPoseCellEvaluation { evaluation, error })
}

/// Extend the scalar pose-cell certificate with a discrete visibility-topology
/// check. The caller supplies signatures from a certified cover of the
/// proposed pose cell; candidate-body and open-interval visibility changes
/// invalidate the comparison. Equality is necessary but not sufficient: this
/// helper does not prove candidate completeness or exclude unobserved event
/// births/deaths between samples. Those require independent conservative
/// geometric bounds; otherwise subdivide or use exact visibility.
#[allow(dead_code)]
pub(crate) fn certify_boundary_pose_cell_with_visibility_signature(
    expansion: &BoundaryKernelExpansion,
    baseline_signature: &crate::source_exposure::VisibilityCutSignature,
    sampled_signatures: &[crate::source_exposure::VisibilityCutSignature],
    ordered_visibility_cuts: &[(f64, f64)],
    sample_motion: &[BoundarySampleMotionBound],
) -> Option<BoundaryPoseCellEvaluation> {
    baseline_signature.certify_unchanged_across(sampled_signatures)?;
    certify_boundary_pose_cell(expansion, ordered_visibility_cuts, sample_motion)
}

/// Certify a boundary-pose expansion against the same swept blocker inventory
/// used to produce its exact visibility signatures. This joins three separate
/// obligations: the inventory must still cover the requested target pose, the
/// signature must use that inventory's exact candidate set, and every cut must
/// have an independently derived whole-cell event bound. The candidate
/// inventory alone does not prove that all event classes were enumerated or
/// exclude unobserved event births/deaths; callers must establish that separately.
/// Unsupported or incomplete topology/event proofs fail closed to exact visibility/subdivision.
#[allow(dead_code)]
pub(crate) fn certify_boundary_pose_cell_with_blocker_inventory(
    expansion: &BoundaryKernelExpansion,
    baseline_signature: &crate::source_exposure::VisibilityCutSignature,
    sampled_signatures: &[crate::source_exposure::VisibilityCutSignature],
    ordered_visibility_cuts: &[(f64, f64)],
    sample_motion: &[BoundarySampleMotionBound],
    event_bounds: &[crate::source_exposure::VisibilityEventPoseCellBound],
    inventory: &crate::source_exposure::VisibilityPoseCellCandidateInventory,
    source: (f64, f64),
    bodies: &[crate::organism_geometry::OrganismBodyGeometry],
    target_body_indices: &[usize],
    target_body_index: usize,
) -> Option<BoundaryPoseCellEvaluation> {
    inventory.certifies_target_signature(
        source,
        bodies,
        target_body_indices,
        target_body_index,
        baseline_signature,
    )?;
    baseline_signature.certify_pose_cell(event_bounds)?;
    baseline_signature.certify_unchanged_across(sampled_signatures)?;
    certify_boundary_pose_cell(expansion, ordered_visibility_cuts, sample_motion)
}

/// Certify the exact discrete ordered-pair derivative for every supplied
/// generalized body coordinate. This composes sample-position and signed-weight
/// chain rules with source/receiver center motion, and returns an independent
/// absolute derivative bound for each coordinate. Rotational coordinates retain
/// the full boundary-motion/projected-weight torque term.
///
/// The caller must enumerate x, y, and angle for every body participating as
/// source, receiver, or occluder. Missing coordinates cannot be inferred by
/// this local kernel and must be prevented by the caller's body/DOF inventory.
#[allow(dead_code)]
pub(crate) fn certify_boundary_pose_cell_generalized(
    expansion: &BoundaryKernelExpansion,
    ordered_visibility_cuts: &[(f64, f64)],
    sample_motion: &[BoundarySampleMotionBound],
    coordinates: &[BoundaryPoseCoordinateDerivative],
) -> Option<BoundaryPoseCellGeneralizedEvaluation> {
    if coordinates.is_empty() {
        return None;
    }
    let boundary = certify_boundary_pose_cell(expansion, ordered_visibility_cuts, sample_motion)?;
    let coordinates = coordinates
        .iter()
        .map(|coordinate| expansion.evaluate_pose_coordinate_derivative(coordinate))
        .collect::<Option<Vec<_>>>()?;
    Some(BoundaryPoseCellGeneralizedEvaluation {
        boundary,
        coordinates,
    })
}

impl BoundaryKernelExpansion {
    /// Retain exact sample correspondence for this ordered pair. The
    /// expansion disk is centered at sample_center and contains every midpoint.
    pub(crate) fn new(
        source_center: BoundaryKernelVector,
        receiver_center: BoundaryKernelVector,
        sample_center: BoundaryKernelVector,
        samples: Vec<WeightedBoundaryKernelSample>,
    ) -> Option<Self> {
        let finite = |point: BoundaryKernelVector| point.x.is_finite() && point.y.is_finite();
        if !finite(source_center)
            || !finite(receiver_center)
            || !finite(sample_center)
            || samples.is_empty()
        {
            return None;
        }

        let mut total_weight = 0.0;
        let mut absolute_weight = 0.0;
        let mut first_moment = BoundaryKernelVector::default();
        let mut radius: f64 = 0.0;
        for sample in &samples {
            if !finite(sample.position) || !sample.weight.is_finite() {
                return None;
            }
            let offset = BoundaryKernelVector {
                x: sample.position.x - sample_center.x,
                y: sample.position.y - sample_center.y,
            };
            total_weight += sample.weight;
            absolute_weight += sample.weight.abs();
            first_moment.x += sample.weight * offset.x;
            first_moment.y += sample.weight * offset.y;
            radius = radius.max(offset.norm());
        }
        if !total_weight.is_finite()
            || !absolute_weight.is_finite()
            || !first_moment.x.is_finite()
            || !first_moment.y.is_finite()
            || !radius.is_finite()
        {
            return None;
        }

        Some(Self {
            source_center,
            receiver_center,
            sample_center,
            samples,
            total_weight,
            absolute_weight,
            first_moment,
            radius,
        })
    }

    /// First-order Taylor sum of each signed weight times the product of the
    /// two existing distance attenuations. The expansion disk must remain
    /// strictly separated from both body reference positions.
    pub(crate) fn evaluate(&self) -> Option<BoundaryPotentialEvaluation> {
        let center = self.sample_center;
        let source_delta = BoundaryKernelVector {
            x: center.x - self.source_center.x,
            y: center.y - self.source_center.y,
        };
        let receiver_delta = BoundaryKernelVector {
            x: center.x - self.receiver_center.x,
            y: center.y - self.receiver_center.y,
        };
        let source_distance = source_delta.norm();
        let receiver_distance = receiver_delta.norm();
        let source_min = source_distance - self.radius;
        let receiver_min = receiver_distance - self.radius;
        if !source_min.is_finite()
            || !receiver_min.is_finite()
            || source_min <= f64::EPSILON
            || receiver_min <= f64::EPSILON
        {
            return None;
        }

        let (source_value, source_gradient) =
            radial_kernel_value_gradient(source_delta, source_distance);
        let (receiver_value, receiver_gradient) =
            radial_kernel_value_gradient(receiver_delta, receiver_distance);
        let kernel = source_value * receiver_value;
        let gradient = BoundaryKernelVector {
            x: source_gradient.x * receiver_value + source_value * receiver_gradient.x,
            y: source_gradient.y * receiver_value + source_value * receiver_gradient.y,
        };
        let potential = self.total_weight * kernel + gradient.dot(self.first_moment);

        // Differentiate the same first-order sample-cluster expansion with
        // respect to each body center, holding exact sample positions/weights
        // fixed. H_f M_1 is evaluated as a radial Hessian-vector product.
        let source_hessian_moment =
            radial_hessian_apply(source_delta, source_distance, self.first_moment);
        let receiver_hessian_moment =
            radial_hessian_apply(receiver_delta, receiver_distance, self.first_moment);
        let source_gradient_moment = source_gradient.dot(self.first_moment);
        let receiver_gradient_moment = receiver_gradient.dot(self.first_moment);
        let source_center_gradient = BoundaryKernelVector {
            x: -self.total_weight * source_gradient.x * receiver_value
                - receiver_value * source_hessian_moment.x
                - source_gradient.x * receiver_gradient_moment,
            y: -self.total_weight * source_gradient.y * receiver_value
                - receiver_value * source_hessian_moment.y
                - source_gradient.y * receiver_gradient_moment,
        };
        let receiver_center_gradient = BoundaryKernelVector {
            x: -self.total_weight * receiver_gradient.x * source_value
                - source_value * receiver_hessian_moment.x
                - receiver_gradient.x * source_gradient_moment,
            y: -self.total_weight * receiver_gradient.y * source_value
                - source_value * receiver_hessian_moment.y
                - receiver_gradient.y * source_gradient_moment,
        };

        // Bounds for the fixed-sample center-gradient remainders. For
        // h_a(p)=d[g(p)]/d(c_a)=-grad(f_a) f_b, bound its spatial Hessian by
        // T_a F_b + 3 H_a G_b + 3 G_a H_b. The factor 3 deliberately
        // overbounds the mixed tensor contractions in two dimensions.
        let source_value_bound = 1.0 / (1.0 + source_min);
        let receiver_value_bound = 1.0 / (1.0 + receiver_min);
        let source_gradient_bound = 1.0 / (1.0 + source_min).powi(2);
        let receiver_gradient_bound = 1.0 / (1.0 + receiver_min).powi(2);
        let source_hessian_bound = radial_hessian_bound(source_min);
        let receiver_hessian_bound = radial_hessian_bound(receiver_min);
        let source_third_bound = radial_third_bound(source_min);
        let receiver_third_bound = radial_third_bound(receiver_min);
        let product_hessian_bound = receiver_value_bound * source_hessian_bound
            + source_value_bound * receiver_hessian_bound
            + 2.0 * source_gradient_bound * receiver_gradient_bound;
        let source_gradient_remainder = source_third_bound * receiver_value_bound
            + 3.0 * source_hessian_bound * receiver_gradient_bound
            + 3.0 * source_gradient_bound * receiver_hessian_bound;
        let receiver_gradient_remainder = receiver_third_bound * source_value_bound
            + 3.0 * receiver_hessian_bound * source_gradient_bound
            + 3.0 * receiver_gradient_bound * source_hessian_bound;
        let scale = 0.5 * self.absolute_weight * self.radius * self.radius;
        let potential_analytic_error = scale * product_hessian_bound;
        let source_analytic_error = scale * source_gradient_remainder;
        let receiver_analytic_error = scale * receiver_gradient_remainder;
        let roundoff_guard = 64.0
            * f64::EPSILON
            * (potential.abs()
                + source_center_gradient.norm()
                + receiver_center_gradient.norm()
                + self.absolute_weight
                + 1.0);
        let error = BoundaryPotentialError {
            potential: potential_analytic_error + roundoff_guard,
            source_force: source_analytic_error + roundoff_guard,
            receiver_force: receiver_analytic_error + roundoff_guard,
        };
        if !potential.is_finite()
            || !source_center_gradient.x.is_finite()
            || !source_center_gradient.y.is_finite()
            || !receiver_center_gradient.x.is_finite()
            || !receiver_center_gradient.y.is_finite()
            || !error.potential.is_finite()
            || !error.source_force.is_finite()
            || !error.receiver_force.is_finite()
        {
            return None;
        }

        Some(BoundaryPotentialEvaluation {
            potential,
            source_gradient: source_center_gradient,
            receiver_gradient: receiver_center_gradient,
            error,
            center,
            radius: self.radius,
        })
    }

    /// Bound the change in the exact discrete potential and its fixed-sample
    /// source/receiver center gradients under certified sample motion.
    ///
    /// For every sample, the caller supplies eta >= |p' - p| and
    /// omega >= |w' - w|. The closed disk of radius eta around p must remain
    /// separated from both body centers. Sample correspondence and visibility
    /// topology must be unchanged throughout the pose cell; otherwise callers
    /// must reject this path and use exact evaluation.
    pub(crate) fn bound_sample_geometry_change(
        &self,
        motion: &[BoundarySampleMotionBound],
    ) -> Option<BoundaryGeometryError> {
        if motion.len() != self.samples.len() {
            return None;
        }

        let mut potential_error = 0.0;
        let mut source_force_error = 0.0;
        let mut receiver_force_error = 0.0;
        for (sample, bound) in self.samples.iter().zip(motion) {
            let eta = bound.position_displacement;
            let omega = bound.absolute_weight_change;
            if !eta.is_finite() || eta < 0.0 || !omega.is_finite() || omega < 0.0 {
                return None;
            }
            let source_distance = BoundaryKernelVector {
                x: sample.position.x - self.source_center.x,
                y: sample.position.y - self.source_center.y,
            }
            .norm();
            let receiver_distance = BoundaryKernelVector {
                x: sample.position.x - self.receiver_center.x,
                y: sample.position.y - self.receiver_center.y,
            }
            .norm();
            let source_min = source_distance - eta;
            let receiver_min = receiver_distance - eta;
            if !source_min.is_finite()
                || !receiver_min.is_finite()
                || source_min <= f64::EPSILON
                || receiver_min <= f64::EPSILON
            {
                return None;
            }

            let source_value = 1.0 / (1.0 + source_min);
            let receiver_value = 1.0 / (1.0 + receiver_min);
            let source_gradient = 1.0 / (1.0 + source_min).powi(2);
            let receiver_gradient = 1.0 / (1.0 + receiver_min).powi(2);
            let source_hessian = radial_hessian_bound(source_min);
            let receiver_hessian = radial_hessian_bound(receiver_min);

            // g = f_a f_b; h_a = -grad(f_a) f_b and
            // h_b = -grad(f_b) f_a. Their spatial Lipschitz bounds follow
            // from the product rule and radial Hessian operator bounds.
            let kernel_bound = source_value * receiver_value;
            let kernel_lipschitz =
                source_gradient * receiver_value + source_value * receiver_gradient;
            let source_gradient_bound = source_gradient * receiver_value;
            let receiver_gradient_bound = receiver_gradient * source_value;
            let source_gradient_lipschitz =
                source_hessian * receiver_value + source_gradient * receiver_gradient;
            let receiver_gradient_lipschitz =
                receiver_hessian * source_value + receiver_gradient * source_gradient;

            let weight = sample.weight.abs();
            potential_error += omega * kernel_bound + weight * kernel_lipschitz * eta;
            source_force_error +=
                omega * source_gradient_bound + weight * source_gradient_lipschitz * eta;
            receiver_force_error +=
                omega * receiver_gradient_bound + weight * receiver_gradient_lipschitz * eta;
        }

        let roundoff_guard = 64.0
            * f64::EPSILON
            * (potential_error + source_force_error + receiver_force_error + 1.0);
        let result = BoundaryGeometryError {
            potential: potential_error + roundoff_guard,
            source_force: source_force_error + roundoff_guard,
            receiver_force: receiver_force_error + roundoff_guard,
        };
        [result.potential, result.source_force, result.receiver_force]
            .iter()
            .all(|value| value.is_finite())
            .then_some(result)
    }

    /// Differentiate the exact discrete boundary-sample potential through
    /// one generalized pose coordinate. This includes both body-center
    /// derivatives and the sample-position/projected-weight chain rule.
    ///
    /// The returned derivative is exact for the supplied reference-pose
    /// samples and derivatives. The absolute bound is conservative over the
    /// supplied smooth pose cell, using each sample's displacement,
    /// derivative, and signed-weight envelopes. Callers must establish
    /// complete visibility topology and exact sample correspondence.
    #[allow(dead_code)]
    pub(crate) fn evaluate_pose_coordinate_derivative(
        &self,
        coordinate: &BoundaryPoseCoordinateDerivative,
    ) -> Option<BoundaryPoseDerivativeEvaluation> {
        if coordinate.samples.len() != self.samples.len()
            || !coordinate.source_center_derivative.x.is_finite()
            || !coordinate.source_center_derivative.y.is_finite()
            || !coordinate.receiver_center_derivative.x.is_finite()
            || !coordinate.receiver_center_derivative.y.is_finite()
            || !coordinate.source_center_derivative_bound.is_finite()
            || coordinate.source_center_derivative_bound < 0.0
            || !coordinate.receiver_center_derivative_bound.is_finite()
            || coordinate.receiver_center_derivative_bound < 0.0
            || coordinate.source_center_derivative.norm()
                > coordinate.source_center_derivative_bound + 64.0 * f64::EPSILON
            || coordinate.receiver_center_derivative.norm()
                > coordinate.receiver_center_derivative_bound + 64.0 * f64::EPSILON
        {
            return None;
        }

        let mut derivative = 0.0;
        let mut absolute_bound = 0.0;
        let mut sample_position_derivative = 0.0;
        let mut sample_weight_derivative = 0.0;
        for (sample, pose) in self.samples.iter().zip(&coordinate.samples) {
            let values = [
                pose.position_derivative.x,
                pose.position_derivative.y,
                pose.weight_derivative,
                pose.position_derivative_bound,
                pose.absolute_weight_derivative_bound,
                pose.position_displacement_bound,
                pose.absolute_weight_change_bound,
            ];
            if values.iter().any(|value| !value.is_finite())
                || pose.position_derivative_bound < 0.0
                || pose.absolute_weight_derivative_bound < 0.0
                || pose.position_displacement_bound < 0.0
                || pose.absolute_weight_change_bound < 0.0
                || pose.position_derivative.norm()
                    > pose.position_derivative_bound + 64.0 * f64::EPSILON
                || pose.weight_derivative.abs()
                    > pose.absolute_weight_derivative_bound + 64.0 * f64::EPSILON
            {
                return None;
            }

            let source_delta = BoundaryKernelVector {
                x: sample.position.x - self.source_center.x,
                y: sample.position.y - self.source_center.y,
            };
            let receiver_delta = BoundaryKernelVector {
                x: sample.position.x - self.receiver_center.x,
                y: sample.position.y - self.receiver_center.y,
            };
            let source_distance = source_delta.norm();
            let receiver_distance = receiver_delta.norm();
            let source_min = source_distance - pose.position_displacement_bound;
            let receiver_min = receiver_distance - pose.position_displacement_bound;
            if !source_min.is_finite()
                || !receiver_min.is_finite()
                || source_min <= f64::EPSILON
                || receiver_min <= f64::EPSILON
            {
                return None;
            }

            let (source_value, source_gradient) =
                radial_kernel_value_gradient(source_delta, source_distance);
            let (receiver_value, receiver_gradient) =
                radial_kernel_value_gradient(receiver_delta, receiver_distance);
            let kernel = source_value * receiver_value;
            let sample_gradient = BoundaryKernelVector {
                x: source_gradient.x * receiver_value + receiver_gradient.x * source_value,
                y: source_gradient.y * receiver_value + receiver_gradient.y * source_value,
            };
            let source_center_gradient = BoundaryKernelVector {
                x: -sample.weight * source_gradient.x * receiver_value,
                y: -sample.weight * source_gradient.y * receiver_value,
            };
            let receiver_center_gradient = BoundaryKernelVector {
                x: -sample.weight * receiver_gradient.x * source_value,
                y: -sample.weight * receiver_gradient.y * source_value,
            };
            let position_term = sample.weight * sample_gradient.dot(pose.position_derivative);
            let weight_term = kernel * pose.weight_derivative;
            derivative += source_center_gradient.dot(coordinate.source_center_derivative)
                + receiver_center_gradient.dot(coordinate.receiver_center_derivative)
                + position_term
                + weight_term;
            sample_position_derivative += position_term;
            sample_weight_derivative += weight_term;

            let source_value_bound = 1.0 / (1.0 + source_min);
            let receiver_value_bound = 1.0 / (1.0 + receiver_min);
            let source_gradient_bound = 1.0 / (1.0 + source_min).powi(2);
            let receiver_gradient_bound = 1.0 / (1.0 + receiver_min).powi(2);
            let kernel_bound = source_value_bound * receiver_value_bound;
            let sample_gradient_bound = source_gradient_bound * receiver_value_bound
                + receiver_gradient_bound * source_value_bound;
            let maximum_absolute_weight = sample.weight.abs() + pose.absolute_weight_change_bound;
            absolute_bound += maximum_absolute_weight
                * (source_gradient_bound
                    * receiver_value_bound
                    * coordinate.source_center_derivative_bound
                    + receiver_gradient_bound
                        * source_value_bound
                        * coordinate.receiver_center_derivative_bound
                    + sample_gradient_bound * pose.position_derivative_bound)
                + kernel_bound * pose.absolute_weight_derivative_bound;
        }

        let roundoff_guard =
            128.0 * f64::EPSILON * (derivative.abs() + absolute_bound + self.absolute_weight + 1.0);
        let result = BoundaryPoseDerivativeEvaluation {
            derivative,
            absolute_bound: absolute_bound + roundoff_guard,
            sample_position_derivative,
            sample_weight_derivative,
        };
        [
            result.derivative,
            result.absolute_bound,
            result.sample_position_derivative,
            result.sample_weight_derivative,
        ]
        .iter()
        .all(|value| value.is_finite())
        .then_some(result)
    }

    /// Exact discrete sum, retained as the differential oracle.
    #[cfg(test)]
    fn exact_potential(&self) -> f64 {
        self.samples
            .iter()
            .map(|sample| {
                let source_distance = BoundaryKernelVector {
                    x: sample.position.x - self.source_center.x,
                    y: sample.position.y - self.source_center.y,
                }
                .norm();
                let receiver_distance = BoundaryKernelVector {
                    x: sample.position.x - self.receiver_center.x,
                    y: sample.position.y - self.receiver_center.y,
                }
                .norm();
                sample.weight / ((1.0 + source_distance) * (1.0 + receiver_distance))
            })
            .sum()
    }

    /// Exact fixed-sample derivatives with respect to both body centers.
    #[cfg(test)]
    fn exact_center_gradients(&self) -> (BoundaryKernelVector, BoundaryKernelVector) {
        let mut source = BoundaryKernelVector::default();
        let mut receiver = BoundaryKernelVector::default();
        for sample in &self.samples {
            let source_delta = BoundaryKernelVector {
                x: sample.position.x - self.source_center.x,
                y: sample.position.y - self.source_center.y,
            };
            let receiver_delta = BoundaryKernelVector {
                x: sample.position.x - self.receiver_center.x,
                y: sample.position.y - self.receiver_center.y,
            };
            let source_distance = source_delta.norm();
            let receiver_distance = receiver_delta.norm();
            let (source_value, source_gradient) =
                radial_kernel_value_gradient(source_delta, source_distance);
            let (receiver_value, receiver_gradient) =
                radial_kernel_value_gradient(receiver_delta, receiver_distance);
            source.x -= sample.weight * source_gradient.x * receiver_value;
            source.y -= sample.weight * source_gradient.y * receiver_value;
            receiver.x -= sample.weight * receiver_gradient.x * source_value;
            receiver.y -= sample.weight * receiver_gradient.y * source_value;
        }
        (source, receiver)
    }
}

fn radial_kernel_value_gradient(
    delta: BoundaryKernelVector,
    distance: f64,
) -> (f64, BoundaryKernelVector) {
    let value = 1.0 / (1.0 + distance);
    let scale = -1.0 / (distance * (1.0 + distance).powi(2));
    (
        value,
        BoundaryKernelVector {
            x: scale * delta.x,
            y: scale * delta.y,
        },
    )
}

fn radial_hessian_bound(minimum_distance: f64) -> f64 {
    let first = -1.0 / (1.0 + minimum_distance).powi(2);
    let second = 2.0 / (1.0 + minimum_distance).powi(3);
    second.abs().max((first / minimum_distance).abs())
}

fn radial_third_bound(minimum_distance: f64) -> f64 {
    let d = minimum_distance;
    6.0 / (1.0 + d).powi(4)
        + 3.0 * (2.0 / (d * (1.0 + d).powi(3)) + 1.0 / (d * d * (1.0 + d).powi(2)))
}

fn radial_hessian_apply(
    delta: BoundaryKernelVector,
    distance: f64,
    vector: BoundaryKernelVector,
) -> BoundaryKernelVector {
    let radial_unit = BoundaryKernelVector {
        x: delta.x / distance,
        y: delta.y / distance,
    };
    let first = -1.0 / (1.0 + distance).powi(2);
    let second = 2.0 / (1.0 + distance).powi(3);
    let tangential = first / distance;
    let radial_component = radial_unit.dot(vector);
    BoundaryKernelVector {
        x: tangential * vector.x + (second - tangential) * radial_unit.x * radial_component,
        y: tangential * vector.y + (second - tangential) * radial_unit.y * radial_component,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rigid_point_pose_derivative_matches_finite_difference() {
        let point = BoundaryKernelVector { x: 3.0, y: -2.0 };
        let pivot = BoundaryKernelVector { x: 0.5, y: 1.0 };
        let translation = BoundaryKernelVector { x: 0.2, y: -0.1 };
        let angular = 0.7;
        let derivative = rigid_pose_point_derivative(point, pivot, translation, angular).unwrap();
        let transform = |q: f64| {
            let offset = BoundaryKernelVector {
                x: point.x - pivot.x,
                y: point.y - pivot.y,
            };
            let angle = q * angular;
            let (sin, cos) = angle.sin_cos();
            BoundaryKernelVector {
                x: pivot.x + q * translation.x + offset.x * cos - offset.y * sin,
                y: pivot.y + q * translation.y + offset.x * sin + offset.y * cos,
            }
        };
        let step = 1.0e-6;
        let plus = transform(step);
        let minus = transform(-step);
        assert!(((plus.x - minus.x) / (2.0 * step) - derivative.x).abs() < 1.0e-8);
        assert!(((plus.y - minus.y) / (2.0 * step) - derivative.y).abs() < 1.0e-8);
    }

    #[test]
    fn circle_tangent_pose_derivative_matches_finite_difference() {
        let center = BoundaryKernelVector { x: 1.0, y: -0.5 };
        let source = BoundaryKernelVector { x: 5.0, y: 2.0 };
        let center_derivative = BoundaryKernelVector { x: -0.1, y: 0.2 };
        let source_derivative = BoundaryKernelVector { x: 0.3, y: -0.15 };
        let radius = 1.25;
        for side in [-1.0, 1.0] {
            let base = circle_tangent_point_pose_derivative(
                center,
                radius,
                source,
                center_derivative,
                source_derivative,
                side,
            )
            .expect("external circle tangent should be differentiable");
            let tangent = |q: f64| {
                circle_tangent_point_pose_derivative(
                    BoundaryKernelVector {
                        x: center.x + q * center_derivative.x,
                        y: center.y + q * center_derivative.y,
                    },
                    radius,
                    BoundaryKernelVector {
                        x: source.x + q * source_derivative.x,
                        y: source.y + q * source_derivative.y,
                    },
                    BoundaryKernelVector::default(),
                    BoundaryKernelVector::default(),
                    side,
                )
                .expect("small pose perturbation preserves external tangency")
                .point
            };
            let step = 1.0e-6;
            let plus = tangent(step);
            let minus = tangent(-step);
            assert!(((plus.x - minus.x) / (2.0 * step) - base.derivative.x).abs() < 1.0e-8);
            assert!(((plus.y - minus.y) / (2.0 * step) - base.derivative.y).abs() < 1.0e-8);
        }
    }

    #[test]
    fn rigid_pose_displacement_bound_contains_transformed_boundary_points() {
        let translation_bound = 0.35;
        let rotation_bound = 0.8;
        let maximum_radius: f64 = 3.2;
        let bound =
            rigid_pose_point_displacement_bound(maximum_radius, translation_bound, rotation_bound)
                .expect("finite rigid-pose envelope");
        for index in 0..128 {
            let radius = maximum_radius * (index as f64 / 127.0);
            let angle = index as f64 * 0.173;
            let point = (radius * angle.cos(), radius * angle.sin());
            let translation_angle = index as f64 * 0.071;
            let translation = (
                translation_bound * translation_angle.cos(),
                translation_bound * translation_angle.sin(),
            );
            let rotation = rotation_bound * (index as f64 / 127.0);
            let moved = (
                point.0 * rotation.cos() - point.1 * rotation.sin() + translation.0,
                point.0 * rotation.sin() + point.1 * rotation.cos() + translation.1,
            );
            let displacement = (moved.0 - point.0).hypot(moved.1 - point.1);
            assert!(displacement <= bound + 1e-12);
        }
        let full_turn_bound =
            rigid_pose_point_displacement_bound(maximum_radius, 0.0, 4.0 * std::f64::consts::PI)
                .expect("large angular envelope");
        assert!((full_turn_bound - 2.0 * maximum_radius).abs() < 1e-12);
    }

    #[test]
    fn rigid_pose_displacement_bound_rejects_invalid_inputs() {
        assert!(rigid_pose_point_displacement_bound(-1.0, 0.0, 0.0).is_none());
        assert!(rigid_pose_point_displacement_bound(1.0, f64::NAN, 0.0).is_none());
        assert!(rigid_pose_point_displacement_bound(1.0, 0.0, -0.1).is_none());
    }
    #[test]
    fn ray_edge_intersection_motion_bound_contains_perturbed_cuts() {
        let source = BoundaryKernelVector { x: 0.0, y: 0.0 };
        let critical = BoundaryKernelVector { x: 1.0, y: 0.5 };
        let start = BoundaryKernelVector { x: 2.0, y: -2.0 };
        let end = BoundaryKernelVector { x: 2.0, y: 2.0 };
        let motion = RayEdgeIntersectionMotion {
            source_displacement: 0.01,
            critical_point_displacement: 0.015,
            edge_start_displacement: 0.005,
            edge_end_displacement: 0.005,
        };
        let bound = ray_edge_intersection_displacement_bound(source, critical, start, end, motion)
            .expect("strict ray-edge intersection should have a finite motion bound");

        let intersection = |s: BoundaryKernelVector,
                            v: BoundaryKernelVector,
                            a: BoundaryKernelVector,
                            b: BoundaryKernelVector| {
            let q = BoundaryKernelVector {
                x: a.x - s.x,
                y: a.y - s.y,
            };
            let r = BoundaryKernelVector {
                x: v.x - s.x,
                y: v.y - s.y,
            };
            let e = BoundaryKernelVector {
                x: b.x - a.x,
                y: b.y - a.y,
            };
            let cross = |left: BoundaryKernelVector, right: BoundaryKernelVector| {
                left.x * right.y - left.y * right.x
            };
            let denominator = cross(e, r);
            let t = -cross(q, r) / denominator;
            let u = -cross(q, e) / denominator;
            assert!(t > 0.0 && t < 1.0 && u > 1.0);
            BoundaryKernelVector {
                x: a.x + t * e.x,
                y: a.y + t * e.y,
            }
        };
        let original = intersection(source, critical, start, end);
        let mut state = 0x4d59_5df4_d0f3_3173_u64;
        let mut next_direction = || {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let angle = ((state >> 32) as u32) as f64 / u32::MAX as f64 * std::f64::consts::TAU;
            (angle.cos(), angle.sin())
        };

        for _ in 0..256 {
            let (sx, sy) = next_direction();
            let (vx, vy) = next_direction();
            let (ax, ay) = next_direction();
            let (bx, by) = next_direction();
            let moved_source = BoundaryKernelVector {
                x: source.x + sx * motion.source_displacement * 0.8,
                y: source.y + sy * motion.source_displacement * 0.8,
            };
            let moved_critical = BoundaryKernelVector {
                x: critical.x + vx * motion.critical_point_displacement * 0.8,
                y: critical.y + vy * motion.critical_point_displacement * 0.8,
            };
            let moved_start = BoundaryKernelVector {
                x: start.x + ax * motion.edge_start_displacement * 0.8,
                y: start.y + ay * motion.edge_start_displacement * 0.8,
            };
            let moved_end = BoundaryKernelVector {
                x: end.x + bx * motion.edge_end_displacement * 0.8,
                y: end.y + by * motion.edge_end_displacement * 0.8,
            };
            let moved = intersection(moved_source, moved_critical, moved_start, moved_end);
            assert!(
                (moved.x - original.x).hypot(moved.y - original.y) <= bound,
                "perturbed cut exceeded certified displacement"
            );
        }
    }

    #[test]
    fn ray_edge_intersection_motion_bound_rejects_singular_or_transitioning_cuts() {
        let source = BoundaryKernelVector { x: 0.0, y: 0.0 };
        let critical = BoundaryKernelVector { x: 1.0, y: 0.5 };
        let start = BoundaryKernelVector { x: 2.0, y: -2.0 };
        let end = BoundaryKernelVector { x: 2.0, y: 2.0 };
        let valid = RayEdgeIntersectionMotion {
            source_displacement: 0.01,
            critical_point_displacement: 0.015,
            edge_start_displacement: 0.005,
            edge_end_displacement: 0.005,
        };
        assert!(ray_edge_intersection_displacement_bound(
            source,
            critical,
            start,
            end,
            RayEdgeIntersectionMotion {
                source_displacement: f64::NAN,
                ..valid
            },
        )
        .is_none());
        assert!(ray_edge_intersection_displacement_bound(
            source,
            critical,
            start,
            end,
            RayEdgeIntersectionMotion {
                source_displacement: 10.0,
                ..valid
            },
        )
        .is_none());
        assert!(
            ray_edge_intersection_displacement_bound(source, source, start, end, valid,).is_none()
        );
    }

    #[test]
    fn straight_interval_sample_pose_derivative_composes_midpoint_and_weight() {
        let source = BoundaryKernelVector { x: 0.3, y: 2.0 };
        let start = BoundaryKernelVector { x: -1.0, y: 0.0 };
        let end = BoundaryKernelVector { x: 1.0, y: 0.0 };
        let normal = BoundaryKernelVector { x: 0.0, y: 1.0 };
        let ds = BoundaryKernelVector { x: 0.07, y: -0.03 };
        let da = BoundaryKernelVector { x: -0.02, y: 0.04 };
        let db = BoundaryKernelVector { x: 0.03, y: 0.01 };
        let sample_pose = straight_interval_sample_pose_derivative(
            source, start, end, normal, ds, da, db, 0.01, 0.01, 0.01, 0.4, 0.2, 0.3, 0.05, -0.75,
        )
        .expect("straight interval should produce a complete sample pose derivative");
        assert_eq!(
            sample_pose.position_derivative,
            BoundaryKernelVector {
                x: 0.5 * (da.x + db.x),
                y: 0.5 * (da.y + db.y),
            }
        );
        assert!(
            sample_pose.absolute_weight_derivative_bound >= sample_pose.weight_derivative.abs()
        );
        assert!(sample_pose.absolute_weight_change_bound > 0.0);
        assert!(sample_pose.position_derivative_bound >= sample_pose.position_derivative.norm());
    }

    #[test]
    fn straight_interval_projected_extent_derivative_bound_contains_pose_samples() {
        let source = BoundaryKernelVector { x: 0.3, y: 2.0 };
        let start = BoundaryKernelVector { x: -1.0, y: 0.0 };
        let end = BoundaryKernelVector { x: 1.0, y: 0.0 };
        let normal = BoundaryKernelVector { x: 0.0, y: 1.0 };
        let source_displacement = 0.05;
        let start_displacement = 0.02;
        let end_displacement = 0.03;
        let source_derivative_bound = 0.4;
        let start_derivative_bound = 0.2;
        let end_derivative_bound = 0.3;
        let bound = straight_interval_projected_extent_derivative_bound(
            source,
            start,
            end,
            normal,
            source_displacement,
            start_displacement,
            end_displacement,
            source_derivative_bound,
            start_derivative_bound,
            end_derivative_bound,
        )
        .expect("front-facing interval should have a finite derivative bound");
        let ds = BoundaryKernelVector { x: 0.3, y: 0.2 };
        let da = BoundaryKernelVector { x: 0.1, y: 0.1 };
        let db = BoundaryKernelVector { x: 0.2, y: 0.1 };
        for index in 0..96 {
            let angle = index as f64 * 0.618_033_988_75;
            let moved_source = BoundaryKernelVector {
                x: source.x + source_displacement * 0.8 * angle.cos(),
                y: source.y + source_displacement * 0.8 * angle.sin(),
            };
            let moved_start = BoundaryKernelVector {
                x: start.x + start_displacement * 0.8 * (angle + 0.7).cos(),
                y: start.y + start_displacement * 0.8 * (angle + 0.7).sin(),
            };
            let moved_end = BoundaryKernelVector {
                x: end.x + end_displacement * 0.8 * (angle + 1.3).cos(),
                y: end.y + end_displacement * 0.8 * (angle + 1.3).sin(),
            };
            let edge = BoundaryKernelVector {
                x: moved_end.x - moved_start.x,
                y: moved_end.y - moved_start.y,
            };
            let length = edge.norm();
            let moved_normal = BoundaryKernelVector {
                x: -edge.y / length,
                y: edge.x / length,
            };
            let derivative = straight_interval_pose_derivative(
                moved_source,
                moved_start,
                moved_end,
                moved_normal,
                ds,
                da,
                db,
            )
            .expect("bounded perturbation should remain in the smooth front-facing cell");
            assert!(
                derivative.projected_extent_derivative.abs() <= bound,
                "derivative {} exceeded bound {bound}",
                derivative.projected_extent_derivative
            );
        }
    }

    #[test]
    fn ray_edge_pose_derivative_matches_finite_difference() {
        let source = BoundaryKernelVector { x: 0.0, y: 0.0 };
        let critical = BoundaryKernelVector { x: 1.0, y: 0.5 };
        let start = BoundaryKernelVector { x: 2.0, y: -2.0 };
        let end = BoundaryKernelVector { x: 2.0, y: 2.0 };
        let ds = BoundaryKernelVector { x: 0.1, y: -0.04 };
        let dv = BoundaryKernelVector { x: -0.02, y: 0.03 };
        let da = BoundaryKernelVector { x: 0.01, y: 0.02 };
        let db = BoundaryKernelVector { x: -0.01, y: 0.01 };
        let derivative =
            ray_edge_intersection_pose_derivative(source, critical, start, end, ds, dv, da, db)
                .expect("strict cut should have a derivative");
        let intersect = |q: f64| {
            let s = BoundaryKernelVector {
                x: source.x + q * ds.x,
                y: source.y + q * ds.y,
            };
            let v = BoundaryKernelVector {
                x: critical.x + q * dv.x,
                y: critical.y + q * dv.y,
            };
            let a = BoundaryKernelVector {
                x: start.x + q * da.x,
                y: start.y + q * da.y,
            };
            let b = BoundaryKernelVector {
                x: end.x + q * db.x,
                y: end.y + q * db.y,
            };
            let cross = |l: BoundaryKernelVector, r: BoundaryKernelVector| l.x * r.y - l.y * r.x;
            let sub = |l: BoundaryKernelVector, r: BoundaryKernelVector| BoundaryKernelVector {
                x: l.x - r.x,
                y: l.y - r.y,
            };
            let e = sub(b, a);
            let r = sub(v, s);
            let qv = sub(a, s);
            let t = -cross(qv, r) / cross(e, r);
            BoundaryKernelVector {
                x: a.x + t * e.x,
                y: a.y + t * e.y,
            }
        };
        let step = 1.0e-6;
        let plus = intersect(step);
        let minus = intersect(-step);
        let finite_difference = BoundaryKernelVector {
            x: (plus.x - minus.x) / (2.0 * step),
            y: (plus.y - minus.y) / (2.0 * step),
        };
        assert!((finite_difference.x - derivative.point_derivative.x).abs() < 1.0e-8);
        assert!((finite_difference.y - derivative.point_derivative.y).abs() < 1.0e-8);
    }

    #[test]
    fn straight_boundary_intersection_pose_derivative_matches_finite_difference() {
        let target_start = BoundaryKernelVector { x: 0.0, y: 0.0 };
        let target_end = BoundaryKernelVector { x: 2.0, y: 0.0 };
        let blocker_start = BoundaryKernelVector { x: 1.0, y: -1.0 };
        let blocker_end = BoundaryKernelVector { x: 1.0, y: 1.0 };
        let target_start_rate = BoundaryKernelVector { x: 0.03, y: -0.02 };
        let target_end_rate = BoundaryKernelVector { x: -0.01, y: 0.04 };
        let blocker_start_rate = BoundaryKernelVector { x: 0.02, y: 0.01 };
        let blocker_end_rate = BoundaryKernelVector { x: -0.04, y: -0.03 };
        let derivative = straight_boundary_intersection_pose_derivative(
            target_start,
            target_end,
            blocker_start,
            blocker_end,
            target_start_rate,
            target_end_rate,
            blocker_start_rate,
            blocker_end_rate,
        )
        .expect("strict interior straight-boundary intersection should differentiate");
        let intersection = |coordinate: f64| {
            let moved =
                |point: BoundaryKernelVector, rate: BoundaryKernelVector| BoundaryKernelVector {
                    x: point.x + coordinate * rate.x,
                    y: point.y + coordinate * rate.y,
                };
            let a = moved(target_start, target_start_rate);
            let b = moved(target_end, target_end_rate);
            let c = moved(blocker_start, blocker_start_rate);
            let d = moved(blocker_end, blocker_end_rate);
            let edge = BoundaryKernelVector {
                x: b.x - a.x,
                y: b.y - a.y,
            };
            let blocker = BoundaryKernelVector {
                x: d.x - c.x,
                y: d.y - c.y,
            };
            let offset = BoundaryKernelVector {
                x: c.x - a.x,
                y: c.y - a.y,
            };
            let cross = |left: BoundaryKernelVector, right: BoundaryKernelVector| {
                left.x * right.y - left.y * right.x
            };
            let target_parameter = cross(offset, blocker) / cross(edge, blocker);
            let blocker_parameter = cross(offset, edge) / cross(edge, blocker);
            let point = BoundaryKernelVector {
                x: a.x + target_parameter * edge.x,
                y: a.y + target_parameter * edge.y,
            };
            (target_parameter, blocker_parameter, point)
        };
        let step = 1.0e-6;
        let plus = intersection(step);
        let minus = intersection(-step);
        let target_rate = (plus.0 - minus.0) / (2.0 * step);
        let blocker_rate = (plus.1 - minus.1) / (2.0 * step);
        let point_rate = BoundaryKernelVector {
            x: (plus.2.x - minus.2.x) / (2.0 * step),
            y: (plus.2.y - minus.2.y) / (2.0 * step),
        };
        assert!((target_rate - derivative.target_parameter_derivative).abs() < 1.0e-8);
        assert!((blocker_rate - derivative.blocker_parameter_derivative).abs() < 1.0e-8);
        assert!((point_rate.x - derivative.point_derivative.x).abs() < 1.0e-8);
        assert!((point_rate.y - derivative.point_derivative.y).abs() < 1.0e-8);
    }

    #[test]
    fn rigid_straight_boundary_intersection_pose_derivative_matches_finite_difference() {
        let target_start = BoundaryKernelVector { x: 0.0, y: 0.0 };
        let target_end = BoundaryKernelVector { x: 2.0, y: 0.0 };
        let blocker_start = BoundaryKernelVector { x: 1.0, y: -1.0 };
        let blocker_end = BoundaryKernelVector { x: 1.0, y: 1.0 };
        let target_pivot = BoundaryKernelVector { x: 0.5, y: -0.2 };
        let blocker_pivot = BoundaryKernelVector { x: 1.4, y: 0.3 };
        let target_translation = BoundaryKernelVector { x: 0.07, y: -0.03 };
        let blocker_translation = BoundaryKernelVector { x: -0.02, y: 0.05 };
        let target_angular: f64 = 0.11;
        let blocker_angular: f64 = -0.08;
        let derivative = rigid_straight_boundary_intersection_pose_derivative(
            target_start,
            target_end,
            blocker_start,
            blocker_end,
            target_pivot,
            blocker_pivot,
            target_translation,
            target_angular,
            blocker_translation,
            blocker_angular,
        )
        .expect("rigid edge intersection should remain differentiable");
        let moved = |point: BoundaryKernelVector,
                     pivot: BoundaryKernelVector,
                     translation: BoundaryKernelVector,
                     angular: f64,
                     q: f64| {
            let offset = BoundaryKernelVector {
                x: point.x - pivot.x,
                y: point.y - pivot.y,
            };
            let (sin, cos) = (q * angular).sin_cos();
            BoundaryKernelVector {
                x: pivot.x + q * translation.x + offset.x * cos - offset.y * sin,
                y: pivot.y + q * translation.y + offset.x * sin + offset.y * cos,
            }
        };
        let intersect = |q: f64| {
            let a = moved(
                target_start,
                target_pivot,
                target_translation,
                target_angular,
                q,
            );
            let b = moved(
                target_end,
                target_pivot,
                target_translation,
                target_angular,
                q,
            );
            let c = moved(
                blocker_start,
                blocker_pivot,
                blocker_translation,
                blocker_angular,
                q,
            );
            let d = moved(
                blocker_end,
                blocker_pivot,
                blocker_translation,
                blocker_angular,
                q,
            );
            let edge = BoundaryKernelVector {
                x: b.x - a.x,
                y: b.y - a.y,
            };
            let blocker = BoundaryKernelVector {
                x: d.x - c.x,
                y: d.y - c.y,
            };
            let offset = BoundaryKernelVector {
                x: c.x - a.x,
                y: c.y - a.y,
            };
            let cross = |left: BoundaryKernelVector, right: BoundaryKernelVector| {
                left.x * right.y - left.y * right.x
            };
            let t = cross(offset, blocker) / cross(edge, blocker);
            let u = cross(offset, edge) / cross(edge, blocker);
            let point = BoundaryKernelVector {
                x: a.x + t * edge.x,
                y: a.y + t * edge.y,
            };
            (t, u, point)
        };
        let step = 1.0e-6;
        let plus = intersect(step);
        let minus = intersect(-step);
        assert!(
            ((plus.0 - minus.0) / (2.0 * step) - derivative.target_parameter_derivative).abs()
                < 1.0e-8
        );
        assert!(
            ((plus.1 - minus.1) / (2.0 * step) - derivative.blocker_parameter_derivative).abs()
                < 1.0e-8
        );
        assert!(
            ((plus.2.x - minus.2.x) / (2.0 * step) - derivative.point_derivative.x).abs() < 1.0e-8
        );
        assert!(
            ((plus.2.y - minus.2.y) / (2.0 * step) - derivative.point_derivative.y).abs() < 1.0e-8
        );
    }

    #[test]
    fn straight_boundary_intersection_derivative_rejects_parallel_and_endpoint_events() {
        let point = |x, y| BoundaryKernelVector { x, y };
        let zero = BoundaryKernelVector::default();
        assert!(straight_boundary_intersection_pose_derivative(
            point(0.0, 0.0),
            point(2.0, 0.0),
            point(0.0, 1.0),
            point(2.0, 1.0),
            zero,
            zero,
            zero,
            zero,
        )
        .is_none());
        assert!(straight_boundary_intersection_pose_derivative(
            point(0.0, 0.0),
            point(2.0, 0.0),
            point(0.0, -1.0),
            point(0.0, 1.0),
            zero,
            zero,
            zero,
            zero,
        )
        .is_none());
    }

    #[test]
    fn straight_interval_pose_derivative_matches_finite_differences() {
        let source = BoundaryKernelVector { x: 0.3, y: 2.0 };
        let start = BoundaryKernelVector { x: -1.0, y: 0.0 };
        let end = BoundaryKernelVector { x: 1.0, y: 0.0 };
        let normal = BoundaryKernelVector { x: 0.0, y: 1.0 };
        let ds = BoundaryKernelVector { x: 0.07, y: -0.03 };
        let da = BoundaryKernelVector { x: -0.02, y: 0.04 };
        let db = BoundaryKernelVector { x: 0.03, y: 0.01 };
        let derivative = straight_interval_pose_derivative(source, start, end, normal, ds, da, db)
            .expect("smooth front-facing interval should differentiate");
        let extent = |q: f64| {
            let s = BoundaryKernelVector {
                x: source.x + q * ds.x,
                y: source.y + q * ds.y,
            };
            let a = BoundaryKernelVector {
                x: start.x + q * da.x,
                y: start.y + q * da.y,
            };
            let b = BoundaryKernelVector {
                x: end.x + q * db.x,
                y: end.y + q * db.y,
            };
            let e = BoundaryKernelVector {
                x: b.x - a.x,
                y: b.y - a.y,
            };
            let length = e.norm();
            let n = BoundaryKernelVector {
                x: -e.y / length,
                y: e.x / length,
            };
            let r = BoundaryKernelVector {
                x: s.x - a.x,
                y: s.y - a.y,
            };
            let along = r.x * e.x / length + r.y * e.y / length;
            let normal_distance = r.dot(n);
            normal_distance
                * (((length - along) / normal_distance).asinh() + (along / normal_distance).asinh())
        };
        let step = 1.0e-6;
        let finite_difference = (extent(step) - extent(-step)) / (2.0 * step);
        assert!((finite_difference - derivative.projected_extent_derivative).abs() < 1.0e-8);
        let midpoint_fd = BoundaryKernelVector {
            x: 0.5 * (da.x + db.x),
            y: 0.5 * (da.y + db.y),
        };
        assert!((midpoint_fd.x - derivative.midpoint_derivative.x).abs() < 1.0e-12);
        assert!((midpoint_fd.y - derivative.midpoint_derivative.y).abs() < 1.0e-12);
    }

    #[test]
    fn cut_parameter_order_certificate_rejects_overlap_and_edge_events() {
        assert!(certify_cut_parameter_order(&[(0.2, 0.01), (0.5, 0.02)]).is_some());
        assert!(certify_cut_parameter_order(&[(0.2, 0.1), (0.3, 0.1)]).is_none());
        assert!(certify_cut_parameter_order(&[(0.2, 0.01), (0.99, 0.02)]).is_none());
        assert!(certify_cut_parameter_order(&[(f64::NAN, 0.0)]).is_none());
        assert!(certify_cut_parameter_order(&[(0.2, -0.01)]).is_none());
    }

    #[test]
    fn generalized_pose_cell_certificate_covers_each_supplied_force_and_torque_coordinate() {
        let position = BoundaryKernelVector { x: 4.0, y: 1.5 };
        let expansion = BoundaryKernelExpansion::new(
            BoundaryKernelVector { x: 0.0, y: 0.0 },
            BoundaryKernelVector { x: 9.0, y: 0.5 },
            position,
            vec![sample(position.x, position.y, 1.7)],
        )
        .expect("valid one-sample expansion");
        let coordinate =
            |source_dx, source_dy, sample_dx, sample_dy| BoundaryPoseCoordinateDerivative {
                source_center_derivative: BoundaryKernelVector {
                    x: source_dx,
                    y: source_dy,
                },
                receiver_center_derivative: BoundaryKernelVector::default(),
                source_center_derivative_bound: source_dx.hypot(source_dy),
                receiver_center_derivative_bound: 0.0,
                samples: vec![BoundarySamplePoseDerivative {
                    position_derivative: BoundaryKernelVector {
                        x: sample_dx,
                        y: sample_dy,
                    },
                    weight_derivative: 0.0,
                    position_derivative_bound: sample_dx.hypot(sample_dy),
                    absolute_weight_derivative_bound: 0.0,
                    position_displacement_bound: 0.001,
                    absolute_weight_change_bound: 0.001,
                }],
            };
        let coordinates = vec![
            coordinate(1.0, 0.0, 0.0, 0.0),
            coordinate(0.0, 0.0, -1.5, 4.0),
        ];
        let result = certify_boundary_pose_cell_generalized(
            &expansion,
            &[],
            &[BoundarySampleMotionBound {
                position_displacement: 0.001,
                absolute_weight_change: 0.001,
            }],
            &coordinates,
        )
        .expect("complete supplied coordinate set should certify");
        assert_eq!(result.coordinates.len(), coordinates.len());
        assert!(result.coordinates.iter().all(|entry| {
            entry.absolute_bound.is_finite() && entry.absolute_bound >= entry.derivative.abs()
        }));
        assert!(certify_boundary_pose_cell_generalized(
            &expansion,
            &[],
            &[BoundarySampleMotionBound {
                position_displacement: 0.001,
                absolute_weight_change: 0.001,
            }],
            &[],
        )
        .is_none());
    }

    #[test]
    fn pose_coordinate_derivative_includes_sample_motion_weight_motion_and_torque_terms() {
        let source_center = BoundaryKernelVector { x: 0.0, y: 0.0 };
        let receiver_center = BoundaryKernelVector { x: 9.0, y: 0.5 };
        let sample_position = BoundaryKernelVector { x: 4.0, y: 1.5 };
        let expansion = BoundaryKernelExpansion::new(
            source_center,
            receiver_center,
            sample_position,
            vec![sample(sample_position.x, sample_position.y, 1.7)],
        )
        .expect("valid one-sample expansion");
        let coordinate = BoundaryPoseCoordinateDerivative {
            source_center_derivative: BoundaryKernelVector { x: 0.4, y: -0.2 },
            receiver_center_derivative: BoundaryKernelVector { x: 0.0, y: 0.0 },
            source_center_derivative_bound: 0.5,
            receiver_center_derivative_bound: 0.0,
            samples: vec![BoundarySamplePoseDerivative {
                position_derivative: BoundaryKernelVector { x: -0.3, y: 0.8 },
                weight_derivative: 0.12,
                position_derivative_bound: 1.0,
                absolute_weight_derivative_bound: 0.2,
                position_displacement_bound: 0.01,
                absolute_weight_change_bound: 0.01,
            }],
        };
        let evaluation = expansion
            .evaluate_pose_coordinate_derivative(&coordinate)
            .expect("smooth pose coordinate should be certifiable");
        let step = 1.0e-6;
        let moved = |direction: f64| {
            let moved_source = BoundaryKernelVector {
                x: source_center.x + direction * step * coordinate.source_center_derivative.x,
                y: source_center.y + direction * step * coordinate.source_center_derivative.y,
            };
            let moved_sample = BoundaryKernelVector {
                x: sample_position.x
                    + direction * step * coordinate.samples[0].position_derivative.x,
                y: sample_position.y
                    + direction * step * coordinate.samples[0].position_derivative.y,
            };
            BoundaryKernelExpansion::new(
                moved_source,
                receiver_center,
                moved_sample,
                vec![sample(
                    moved_sample.x,
                    moved_sample.y,
                    1.7 + direction * step * coordinate.samples[0].weight_derivative,
                )],
            )
            .expect("perturbed sample remains valid")
            .exact_potential()
        };
        let finite_difference = (moved(1.0) - moved(-1.0)) / (2.0 * step);
        assert!(
            (evaluation.derivative - finite_difference).abs() < 1.0e-8,
            "analytic={} finite_difference={finite_difference}",
            evaluation.derivative
        );
        assert!(evaluation.absolute_bound >= evaluation.derivative.abs());
        assert!(evaluation.sample_position_derivative.abs() > 0.0);
        assert!(evaluation.sample_weight_derivative.abs() > 0.0);
    }

    #[test]
    fn angular_pose_derivative_includes_boundary_motion_and_projected_extent_torque() {
        let source = BoundaryKernelVector { x: 0.3, y: 2.0 };
        let receiver_center = BoundaryKernelVector { x: 0.0, y: 0.0 };
        let start = BoundaryKernelVector { x: -1.0, y: -3.0 };
        let end = BoundaryKernelVector { x: 1.0, y: -3.0 };
        let normal = BoundaryKernelVector { x: 0.0, y: 1.0 };
        let start_derivative = BoundaryKernelVector { x: 3.0, y: -1.0 };
        let end_derivative = BoundaryKernelVector { x: 3.0, y: 1.0 };
        let geometry = straight_interval_pose_derivative(
            source,
            start,
            end,
            normal,
            BoundaryKernelVector::default(),
            start_derivative,
            end_derivative,
        )
        .expect("rotating straight interval should have a smooth derivative");
        let midpoint = BoundaryKernelVector {
            x: 0.5 * (start.x + end.x),
            y: 0.5 * (start.y + end.y),
        };
        let coefficient = 0.7;
        let expansion = BoundaryKernelExpansion::new(
            source,
            receiver_center,
            midpoint,
            vec![sample(
                midpoint.x,
                midpoint.y,
                coefficient * geometry.projected_extent,
            )],
        )
        .expect("rotating interval sample should form a valid cluster");
        let coordinate = BoundaryPoseCoordinateDerivative {
            source_center_derivative: BoundaryKernelVector::default(),
            receiver_center_derivative: BoundaryKernelVector::default(),
            source_center_derivative_bound: 0.0,
            receiver_center_derivative_bound: 0.0,
            samples: vec![BoundarySamplePoseDerivative {
                position_derivative: geometry.midpoint_derivative,
                weight_derivative: coefficient * geometry.projected_extent_derivative,
                position_derivative_bound: 4.0,
                absolute_weight_derivative_bound: 2.0,
                position_displacement_bound: 0.001,
                absolute_weight_change_bound: 0.001,
            }],
        };
        let evaluation = expansion
            .evaluate_pose_coordinate_derivative(&coordinate)
            .expect("angular coordinate should have a certified magnitude bound");

        let potential_at_angle = |angle: f64| {
            let (sin, cos) = angle.sin_cos();
            let rotate = |point: BoundaryKernelVector| BoundaryKernelVector {
                x: point.x * cos - point.y * sin,
                y: point.x * sin + point.y * cos,
            };
            let moved_start = rotate(start);
            let moved_end = rotate(end);
            let moved_normal = rotate(normal);
            let moved_midpoint = rotate(midpoint);
            let moved_geometry = straight_interval_pose_derivative(
                source,
                moved_start,
                moved_end,
                moved_normal,
                BoundaryKernelVector::default(),
                BoundaryKernelVector::default(),
                BoundaryKernelVector::default(),
            )
            .expect("small angular perturbation remains smooth");
            BoundaryKernelExpansion::new(
                source,
                receiver_center,
                moved_midpoint,
                vec![sample(
                    moved_midpoint.x,
                    moved_midpoint.y,
                    coefficient * moved_geometry.projected_extent,
                )],
            )
            .expect("perturbed interval remains valid")
            .exact_potential()
        };
        let step = 1.0e-6;
        let finite_difference =
            (potential_at_angle(step) - potential_at_angle(-step)) / (2.0 * step);
        assert!(
            (evaluation.derivative - finite_difference).abs() < 1.0e-8,
            "analytic torque={} finite-difference torque={finite_difference}",
            evaluation.derivative
        );
        assert!(evaluation.absolute_bound >= evaluation.derivative.abs());
    }

    #[test]
    fn pose_coordinate_derivative_rejects_incomplete_or_understated_envelopes() {
        let expansion = BoundaryKernelExpansion::new(
            BoundaryKernelVector { x: 0.0, y: 0.0 },
            BoundaryKernelVector { x: 9.0, y: 0.0 },
            BoundaryKernelVector { x: 4.0, y: 1.0 },
            vec![sample(4.0, 1.0, 1.0)],
        )
        .expect("valid sample");
        let mut coordinate = BoundaryPoseCoordinateDerivative {
            source_center_derivative: BoundaryKernelVector { x: 1.0, y: 0.0 },
            receiver_center_derivative: BoundaryKernelVector::default(),
            source_center_derivative_bound: 1.0,
            receiver_center_derivative_bound: 0.0,
            samples: vec![BoundarySamplePoseDerivative {
                position_derivative: BoundaryKernelVector { x: 0.0, y: 1.0 },
                weight_derivative: 0.0,
                position_derivative_bound: 1.0,
                absolute_weight_derivative_bound: 0.0,
                position_displacement_bound: 0.01,
                absolute_weight_change_bound: 0.0,
            }],
        };
        assert!(expansion
            .evaluate_pose_coordinate_derivative(&coordinate)
            .is_some());
        coordinate.samples.clear();
        assert!(expansion
            .evaluate_pose_coordinate_derivative(&coordinate)
            .is_none());
        coordinate.samples = vec![BoundarySamplePoseDerivative {
            position_derivative: BoundaryKernelVector { x: 0.0, y: 1.1 },
            ..BoundarySamplePoseDerivative::default()
        }];
        assert!(expansion
            .evaluate_pose_coordinate_derivative(&coordinate)
            .is_none());
    }

    fn sample(x: f64, y: f64, weight: f64) -> WeightedBoundaryKernelSample {
        WeightedBoundaryKernelSample {
            position: BoundaryKernelVector { x, y },
            weight,
        }
    }

    #[test]
    fn source_specific_boundary_cluster_bound_contains_exact_discrete_error() {
        let samples = vec![
            sample(5.0, 1.0, 0.8),
            sample(5.4, 1.3, -0.25),
            sample(4.7, 1.8, 0.35),
            sample(5.2, 2.1, -0.15),
        ];
        let expansion = BoundaryKernelExpansion::new(
            BoundaryKernelVector { x: 0.0, y: 0.0 },
            BoundaryKernelVector { x: 10.0, y: 0.5 },
            BoundaryKernelVector { x: 5.075, y: 1.55 },
            samples,
        )
        .expect("finite exact pair samples should form a cluster");
        let approximate = expansion
            .evaluate()
            .expect("well-separated sample cluster should admit a bounded expansion");
        let exact = expansion.exact_potential();
        assert!(
            (approximate.potential - exact).abs() <= approximate.error.potential,
            "exact={exact} approximate={} bound={}",
            approximate.potential,
            approximate.error.potential
        );
    }

    #[test]
    fn cancellation_does_not_shrink_the_absolute_remainder_bound() {
        let samples = vec![
            sample(5.0, 1.0, 1.0e6),
            sample(5.1, 1.0, -1.0e6),
            sample(5.0, 1.1, 1.0),
        ];
        let expansion = BoundaryKernelExpansion::new(
            BoundaryKernelVector { x: 0.0, y: 0.0 },
            BoundaryKernelVector { x: 10.0, y: 0.0 },
            BoundaryKernelVector { x: 5.0, y: 1.0 },
            samples,
        )
        .expect("finite signed samples should form a cluster");
        let approximate = expansion.evaluate().expect("cluster is separated");
        let exact = expansion.exact_potential();
        assert!((approximate.potential - exact).abs() <= approximate.error.potential);
        assert!(approximate.error.potential > 0.0);
    }

    #[test]
    fn cluster_touching_either_body_fails_closed() {
        let source_touching = BoundaryKernelExpansion::new(
            BoundaryKernelVector { x: 0.5, y: 0.0 },
            BoundaryKernelVector { x: 10.0, y: 0.0 },
            BoundaryKernelVector { x: 1.0, y: 0.0 },
            vec![sample(0.5, 0.0, 1.0)],
        )
        .expect("valid sample input");
        assert!(source_touching.evaluate().is_none());

        let receiver_touching = BoundaryKernelExpansion::new(
            BoundaryKernelVector { x: 0.0, y: 0.0 },
            BoundaryKernelVector { x: 9.5, y: 0.0 },
            BoundaryKernelVector { x: 9.0, y: 0.0 },
            vec![sample(9.5, 0.0, 1.0)],
        )
        .expect("valid sample input");
        assert!(receiver_touching.evaluate().is_none());
    }

    #[test]
    fn fixed_sample_center_gradient_bounds_contain_exact_derivatives() {
        let samples = vec![
            sample(5.0, 1.0, 0.8),
            sample(5.4, 1.3, -0.25),
            sample(4.7, 1.8, 0.35),
            sample(5.2, 2.1, -0.15),
        ];
        let expansion = BoundaryKernelExpansion::new(
            BoundaryKernelVector { x: 0.0, y: 0.0 },
            BoundaryKernelVector { x: 10.0, y: 0.5 },
            BoundaryKernelVector { x: 5.075, y: 1.55 },
            samples,
        )
        .expect("finite exact pair samples should form a cluster");
        let approximate = expansion.evaluate().expect("cluster is separated");
        let (exact_source, exact_receiver) = expansion.exact_center_gradients();
        let source_error = BoundaryKernelVector {
            x: approximate.source_gradient.x - exact_source.x,
            y: approximate.source_gradient.y - exact_source.y,
        }
        .norm();
        let receiver_error = BoundaryKernelVector {
            x: approximate.receiver_gradient.x - exact_receiver.x,
            y: approximate.receiver_gradient.y - exact_receiver.y,
        }
        .norm();
        assert!(
            source_error <= approximate.error.source_force,
            "source error={source_error} bound={}",
            approximate.error.source_force
        );
        assert!(
            receiver_error <= approximate.error.receiver_force,
            "receiver error={receiver_error} bound={}",
            approximate.error.receiver_force
        );
    }

    #[test]
    fn sample_geometry_change_bounds_contain_exact_potential_and_center_gradient_changes() {
        let samples = vec![
            sample(5.0, 1.0, 0.8),
            sample(5.4, 1.3, -0.25),
            sample(4.7, 1.8, 0.35),
            sample(5.2, 2.1, -0.15),
        ];
        let expansion = BoundaryKernelExpansion::new(
            BoundaryKernelVector { x: 0.0, y: 0.0 },
            BoundaryKernelVector { x: 10.0, y: 0.5 },
            BoundaryKernelVector { x: 5.075, y: 1.55 },
            samples.clone(),
        )
        .expect("finite exact pair samples should form a cluster");
        let motion = vec![
            BoundarySampleMotionBound {
                position_displacement: 0.03,
                absolute_weight_change: 0.02,
            },
            BoundarySampleMotionBound {
                position_displacement: 0.02,
                absolute_weight_change: 0.01,
            },
            BoundarySampleMotionBound {
                position_displacement: 0.04,
                absolute_weight_change: 0.015,
            },
            BoundarySampleMotionBound {
                position_displacement: 0.01,
                absolute_weight_change: 0.005,
            },
        ];
        let bound = expansion
            .bound_sample_geometry_change(&motion)
            .expect("all sample-motion disks remain separated");

        let moved = samples
            .iter()
            .zip(&motion)
            .enumerate()
            .map(|(index, (sample, change))| {
                let direction = if index % 2 == 0 { 1.0 } else { -1.0 };
                WeightedBoundaryKernelSample {
                    position: BoundaryKernelVector {
                        x: sample.position.x + direction * change.position_displacement * 0.7,
                        y: sample.position.y - direction * change.position_displacement * 0.7,
                    },
                    weight: sample.weight + direction * change.absolute_weight_change * 0.8,
                }
            })
            .collect();
        let moved_expansion = BoundaryKernelExpansion::new(
            expansion.source_center,
            expansion.receiver_center,
            expansion.sample_center,
            moved,
        )
        .expect("moved samples remain finite");
        let potential_change =
            (moved_expansion.exact_potential() - expansion.exact_potential()).abs();
        let (base_source, base_receiver) = expansion.exact_center_gradients();
        let (moved_source, moved_receiver) = moved_expansion.exact_center_gradients();
        let source_change = BoundaryKernelVector {
            x: moved_source.x - base_source.x,
            y: moved_source.y - base_source.y,
        }
        .norm();
        let receiver_change = BoundaryKernelVector {
            x: moved_receiver.x - base_receiver.x,
            y: moved_receiver.y - base_receiver.y,
        }
        .norm();

        assert!(potential_change <= bound.potential);
        assert!(source_change <= bound.source_force);
        assert!(receiver_change <= bound.receiver_force);
    }

    #[test]
    fn sample_geometry_change_bound_rejects_missing_or_singular_envelopes() {
        let expansion = BoundaryKernelExpansion::new(
            BoundaryKernelVector { x: 0.0, y: 0.0 },
            BoundaryKernelVector { x: 10.0, y: 0.0 },
            BoundaryKernelVector { x: 5.0, y: 1.0 },
            vec![sample(5.0, 1.0, 1.0)],
        )
        .expect("valid sample input");
        assert!(expansion.bound_sample_geometry_change(&[]).is_none());
        assert!(expansion
            .bound_sample_geometry_change(&[BoundarySampleMotionBound {
                position_displacement: f64::NAN,
                absolute_weight_change: 0.0,
            }])
            .is_none());
        assert!(expansion
            .bound_sample_geometry_change(&[BoundarySampleMotionBound {
                position_displacement: 6.0,
                absolute_weight_change: 0.0,
            }])
            .is_none());
    }

    #[test]
    fn randomized_fixed_sample_center_gradient_bounds_contain_exact_derivatives() {
        let mut state = 0x9e37_79b9_u64;
        let mut next = || {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((state >> 33) as u32) as f64 / u32::MAX as f64
        };
        for iteration in 0..256 {
            let source = BoundaryKernelVector { x: -2.0, y: -1.0 };
            let receiver = BoundaryKernelVector { x: 12.0, y: 2.0 };
            let center = BoundaryKernelVector {
                x: 5.0 + next(),
                y: 0.5 + next(),
            };
            let mut samples = Vec::new();
            for _ in 0..8 {
                samples.push(sample(
                    center.x + (next() - 0.5) * 0.8,
                    center.y + (next() - 0.5) * 0.8,
                    (next() - 0.5) * 4.0,
                ));
            }
            let expansion = BoundaryKernelExpansion::new(source, receiver, center, samples)
                .expect("finite randomized samples should form a cluster");
            let approximate = expansion.evaluate().expect("cluster remains separated");
            let (exact_source, exact_receiver) = expansion.exact_center_gradients();
            let source_error = BoundaryKernelVector {
                x: approximate.source_gradient.x - exact_source.x,
                y: approximate.source_gradient.y - exact_source.y,
            }
            .norm();
            let receiver_error = BoundaryKernelVector {
                x: approximate.receiver_gradient.x - exact_receiver.x,
                y: approximate.receiver_gradient.y - exact_receiver.y,
            }
            .norm();
            assert!(
                source_error <= approximate.error.source_force,
                "source error={source_error} bound={} iteration={}",
                approximate.error.source_force,
                iteration
            );
            assert!(
                receiver_error <= approximate.error.receiver_force,
                "receiver error={receiver_error} bound={} iteration={}",
                approximate.error.receiver_force,
                iteration
            );
        }
    }

    #[test]
    fn pose_cell_certificate_composes_cluster_and_geometry_errors() {
        let samples = vec![
            sample(5.0, 1.0, 0.8),
            sample(5.4, 1.3, -0.25),
            sample(4.7, 1.8, 0.35),
            sample(5.2, 2.1, -0.15),
        ];
        let expansion = BoundaryKernelExpansion::new(
            BoundaryKernelVector { x: 0.0, y: 0.0 },
            BoundaryKernelVector { x: 10.0, y: 0.5 },
            BoundaryKernelVector { x: 5.075, y: 1.55 },
            samples,
        )
        .expect("finite exact pair samples");
        let motion = vec![
            BoundarySampleMotionBound {
                position_displacement: 0.03,
                absolute_weight_change: 0.02,
            },
            BoundarySampleMotionBound {
                position_displacement: 0.02,
                absolute_weight_change: 0.01,
            },
            BoundarySampleMotionBound {
                position_displacement: 0.04,
                absolute_weight_change: 0.015,
            },
            BoundarySampleMotionBound {
                position_displacement: 0.01,
                absolute_weight_change: 0.005,
            },
        ];
        let cluster = expansion.evaluate().unwrap();
        let geometry = expansion.bound_sample_geometry_change(&motion).unwrap();
        let certified =
            certify_boundary_pose_cell(&expansion, &[(0.2, 0.01), (0.6, 0.02)], &motion)
                .expect("stable supplied cut order and sample-motion envelope");
        assert_eq!(certified.evaluation, cluster);
        assert!(
            (certified.error.potential - cluster.error.potential - geometry.potential).abs()
                < 1e-12
        );
        assert!(
            (certified.error.source_force - cluster.error.source_force - geometry.source_force)
                .abs()
                < 1e-12
        );
        assert!(
            (certified.error.receiver_force
                - cluster.error.receiver_force
                - geometry.receiver_force)
                .abs()
                < 1e-12
        );
    }

    #[test]
    fn pose_cell_certificate_rejects_cut_or_sample_correspondence_transitions() {
        let expansion = BoundaryKernelExpansion::new(
            BoundaryKernelVector { x: 0.0, y: 0.0 },
            BoundaryKernelVector { x: 10.0, y: 0.0 },
            BoundaryKernelVector { x: 5.0, y: 1.0 },
            vec![sample(5.0, 1.0, 1.0)],
        )
        .expect("valid sample input");
        let motion = [BoundarySampleMotionBound {
            position_displacement: 0.01,
            absolute_weight_change: 0.01,
        }];
        assert!(
            certify_boundary_pose_cell(&expansion, &[(0.2, 0.1), (0.3, 0.1)], &motion).is_none()
        );
        assert!(certify_boundary_pose_cell(&expansion, &[(0.2, 0.01)], &[]).is_none());
        assert!(certify_boundary_pose_cell(
            &expansion,
            &[(0.2, 0.01)],
            &[BoundarySampleMotionBound {
                position_displacement: 6.0,
                absolute_weight_change: 0.0,
            }],
        )
        .is_none());
    }

    fn zero_radius_cluster_is_exact_up_to_roundoff_guard() {
        let expansion = BoundaryKernelExpansion::new(
            BoundaryKernelVector { x: 0.0, y: 0.0 },
            BoundaryKernelVector { x: 10.0, y: 0.0 },
            BoundaryKernelVector { x: 5.0, y: 1.0 },
            vec![sample(5.0, 1.0, -2.0)],
        )
        .expect("single exact sample should form a cluster");
        let approximate = expansion.evaluate().expect("sample is separated");
        assert!(
            (approximate.potential - expansion.exact_potential()).abs()
                <= approximate.error.potential
        );
        assert_eq!(approximate.radius, 0.0);
    }
}

#[cfg(test)]
mod provenance_resolver_tests {
    use super::*;

    fn exact_intersection(
        target_start: BoundaryKernelVector,
        target_end: BoundaryKernelVector,
        blocker_start: BoundaryKernelVector,
        blocker_end: BoundaryKernelVector,
    ) -> BoundaryKernelVector {
        let a = BoundaryKernelVector {
            x: target_end.x - target_start.x,
            y: target_end.y - target_start.y,
        };
        let b = BoundaryKernelVector {
            x: blocker_end.x - blocker_start.x,
            y: blocker_end.y - blocker_start.y,
        };
        let offset = BoundaryKernelVector {
            x: blocker_start.x - target_start.x,
            y: blocker_start.y - target_start.y,
        };
        let cross = |left: BoundaryKernelVector, right: BoundaryKernelVector| {
            left.x * right.y - left.y * right.x
        };
        let denominator = cross(a, b);
        let parameter = cross(offset, b) / denominator;
        BoundaryKernelVector {
            x: target_start.x + parameter * a.x,
            y: target_start.y + parameter * a.y,
        }
    }

    fn pose_point(pose: RigidBoundaryPose, local: BoundaryKernelVector) -> BoundaryKernelVector {
        pose.transform_local_point(local).unwrap()
    }

    #[test]
    fn provenance_resolver_composes_two_local_edges_and_matches_finite_difference() {
        let event = crate::source_exposure::VisibilityCutEvent::StraightBoundaryIntersection {
            blocker_body_index: 7,
            blocker_unit_index: 3,
            target_body_index: 2,
            target_unit_index: 5,
            target_edge_index: 1,
            blocker_edge_index: 4,
            target_edge_start_local: (-1.0, 0.0),
            target_edge_end_local: (1.0, 0.0),
            blocker_edge_start_local: (0.0, -1.0),
            blocker_edge_end_local: (0.0, 1.0),
            target_parameter: 0.5,
            blocker_parameter: 0.5,
        };
        let target_pose = RigidBoundaryPose {
            pivot: BoundaryKernelVector { x: 2.0, y: 3.0 },
            rotation_radians: 0.3,
        };
        let blocker_pose = RigidBoundaryPose {
            pivot: BoundaryKernelVector { x: 2.0, y: 3.0 },
            rotation_radians: -0.2,
        };
        let target_translation = BoundaryKernelVector { x: 0.2, y: -0.1 };
        let blocker_translation = BoundaryKernelVector { x: -0.05, y: 0.15 };
        let target_angular = 0.3;
        let blocker_angular = -0.2;
        let resolved = resolve_straight_boundary_intersection_event(
            &event,
            2,
            5,
            7,
            3,
            target_pose,
            blocker_pose,
            target_translation,
            target_angular,
            blocker_translation,
            blocker_angular,
        )
        .expect("stored feature identities and local endpoints resolve");

        let step = 1.0e-6;
        let perturbed_intersection = |direction: f64| {
            let target = RigidBoundaryPose {
                pivot: BoundaryKernelVector {
                    x: target_pose.pivot.x + direction * step * target_translation.x,
                    y: target_pose.pivot.y + direction * step * target_translation.y,
                },
                rotation_radians: target_pose.rotation_radians + direction * step * target_angular,
            };
            let blocker = RigidBoundaryPose {
                pivot: BoundaryKernelVector {
                    x: blocker_pose.pivot.x + direction * step * blocker_translation.x,
                    y: blocker_pose.pivot.y + direction * step * blocker_translation.y,
                },
                rotation_radians: blocker_pose.rotation_radians
                    + direction * step * blocker_angular,
            };
            exact_intersection(
                pose_point(target, BoundaryKernelVector { x: -1.0, y: 0.0 }),
                pose_point(target, BoundaryKernelVector { x: 1.0, y: 0.0 }),
                pose_point(blocker, BoundaryKernelVector { x: 0.0, y: -1.0 }),
                pose_point(blocker, BoundaryKernelVector { x: 0.0, y: 1.0 }),
            )
        };
        let plus = perturbed_intersection(1.0);
        let minus = perturbed_intersection(-1.0);
        let finite_difference = BoundaryKernelVector {
            x: (plus.x - minus.x) / (2.0 * step),
            y: (plus.y - minus.y) / (2.0 * step),
        };
        assert!((resolved.derivative.point_derivative.x - finite_difference.x).abs() < 1.0e-8);
        assert!((resolved.derivative.point_derivative.y - finite_difference.y).abs() < 1.0e-8);
    }

    #[test]
    fn provenance_resolver_rejects_stale_identity_and_unsupported_event() {
        let event = crate::source_exposure::VisibilityCutEvent::StraightBoundaryIntersection {
            blocker_body_index: 1,
            blocker_unit_index: 2,
            target_body_index: 3,
            target_unit_index: 4,
            target_edge_index: 0,
            blocker_edge_index: 0,
            target_edge_start_local: (-1.0, 0.0),
            target_edge_end_local: (1.0, 0.0),
            blocker_edge_start_local: (0.0, -1.0),
            blocker_edge_end_local: (0.0, 1.0),
            target_parameter: 0.5,
            blocker_parameter: 0.5,
        };
        let pose = RigidBoundaryPose::default();
        let zero = BoundaryKernelVector::default();
        assert!(resolve_straight_boundary_intersection_event(
            &event, 99, 4, 1, 2, pose, pose, zero, 0.0, zero, 0.0
        )
        .is_none());
        assert!(resolve_straight_boundary_intersection_event(
            &crate::source_exposure::VisibilityCutEvent::Unresolved {
                blocker_body_index: Some(1),
                blocker_unit_index: Some(2),
            },
            3,
            4,
            1,
            2,
            pose,
            pose,
            zero,
            0.0,
            zero,
            0.0,
        )
        .is_none());
    }
}

#[cfg(test)]
mod intersection_parameter_bound_tests {
    use super::*;

    fn cross(left: BoundaryKernelVector, right: BoundaryKernelVector) -> f64 {
        left.x * right.y - left.y * right.x
    }

    fn parameters(
        a: BoundaryKernelVector,
        b: BoundaryKernelVector,
        c: BoundaryKernelVector,
        d: BoundaryKernelVector,
    ) -> (f64, f64) {
        let edge = BoundaryKernelVector {
            x: b.x - a.x,
            y: b.y - a.y,
        };
        let blocker = BoundaryKernelVector {
            x: d.x - c.x,
            y: d.y - c.y,
        };
        let offset = BoundaryKernelVector {
            x: c.x - a.x,
            y: c.y - a.y,
        };
        let denominator = cross(edge, blocker);
        (
            cross(offset, blocker) / denominator,
            cross(offset, edge) / denominator,
        )
    }

    #[test]
    fn intersection_parameter_motion_bounds_contain_endpoint_perturbations() {
        let a = BoundaryKernelVector { x: 0.0, y: 0.0 };
        let b = BoundaryKernelVector { x: 2.0, y: 0.0 };
        let c = BoundaryKernelVector { x: 1.0, y: -1.0 };
        let d = BoundaryKernelVector { x: 1.0, y: 1.0 };
        let radius = 0.01;
        let bound = straight_intersection_parameter_motion_bound(
            a, b, c, d, radius, radius, radius, radius,
        )
        .expect("well-separated intersection has a finite parameter envelope");
        let directions = [
            (1.0, 0.0),
            (-1.0, 0.0),
            (0.0, 1.0),
            (0.0, -1.0),
            (0.6, 0.8),
            (-0.8, 0.6),
            (0.8, -0.6),
            (-0.6, -0.8),
        ];
        for (ia, da) in directions.iter().enumerate() {
            for (ib, db) in directions.iter().enumerate() {
                for (ic, dc) in directions.iter().enumerate() {
                    let dd = directions[(ia + ib + ic) % directions.len()];
                    let moved = |point: BoundaryKernelVector, direction: &(f64, f64)| {
                        BoundaryKernelVector {
                            x: point.x + radius * direction.0,
                            y: point.y + radius * direction.1,
                        }
                    };
                    let (target, blocker) =
                        parameters(moved(a, da), moved(b, db), moved(c, dc), moved(d, &dd));
                    assert!((target - 0.5).abs() <= bound.target_parameter_change);
                    assert!((blocker - 0.5).abs() <= bound.blocker_parameter_change);
                }
            }
        }
    }

    #[test]
    fn intersection_parameter_motion_bound_fails_closed_near_parallel_or_endpoint() {
        let point = |x, y| BoundaryKernelVector { x, y };
        assert!(straight_intersection_parameter_motion_bound(
            point(0.0, 0.0),
            point(2.0, 0.0),
            point(0.0, 1.0),
            point(2.0, 1.0),
            0.0,
            0.0,
            0.0,
            0.0,
        )
        .is_none());
        assert!(straight_intersection_parameter_motion_bound(
            point(0.0, 0.0),
            point(2.0, 0.0),
            point(2.0, -1.0),
            point(2.0, 1.0),
            0.0,
            0.0,
            0.0,
            0.0,
        )
        .is_none());
        assert!(straight_intersection_parameter_motion_bound(
            point(0.0, 0.0),
            point(2.0, 0.0),
            point(1.0, -1.0),
            point(1.0, 1.0),
            0.6,
            0.6,
            0.6,
            0.6,
        )
        .is_none());
    }
}

#[cfg(test)]
mod circle_straight_provenance_tests {
    use super::*;

    fn circle_line_intersection(
        start: BoundaryKernelVector,
        end: BoundaryKernelVector,
        center: BoundaryKernelVector,
        radius: f64,
        reference_parameter: f64,
    ) -> (f64, BoundaryKernelVector) {
        let edge = BoundaryKernelVector {
            x: end.x - start.x,
            y: end.y - start.y,
        };
        let offset = BoundaryKernelVector {
            x: start.x - center.x,
            y: start.y - center.y,
        };
        let a = edge.dot(edge);
        let b = 2.0 * offset.dot(edge);
        let c = offset.dot(offset) - radius * radius;
        let root = (b * b - 4.0 * a * c).sqrt();
        let candidates = [(-b - root) / (2.0 * a), (-b + root) / (2.0 * a)];
        let parameter = candidates
            .into_iter()
            .min_by(|left, right| {
                (left - reference_parameter)
                    .abs()
                    .total_cmp(&(right - reference_parameter).abs())
            })
            .unwrap();
        (
            parameter,
            BoundaryKernelVector {
                x: start.x + parameter * edge.x,
                y: start.y + parameter * edge.y,
            },
        )
    }

    #[test]
    fn circle_straight_provenance_derivative_matches_two_body_finite_difference() {
        let radius = 1.0;
        let target_parameter = (2.0 + 0.75_f64.sqrt()) / 4.0;
        let arc_angle_local = (-0.5_f64)
            .atan2(0.75_f64.sqrt())
            .rem_euclid(std::f64::consts::TAU);
        let event = crate::source_exposure::VisibilityCutEvent::CircleStraightIntersection {
            blocker_body_index: 8,
            blocker_unit_index: 4,
            target_body_index: 1,
            target_unit_index: 6,
            target_edge_index: 2,
            arc_index: 0,
            target_edge_start_local: (-2.0, 0.0),
            target_edge_end_local: (2.0, 0.0),
            blocker_arc_center_local: (0.0, 0.0),
            blocker_arc_radius: radius,
            blocker_arc_start_local: 0.0,
            blocker_arc_sweep: std::f64::consts::TAU,
            target_parameter,
            arc_angle_local,
        };
        let target_pose = RigidBoundaryPose {
            pivot: BoundaryKernelVector { x: 0.0, y: 0.0 },
            rotation_radians: 0.0,
        };
        let blocker_pose = RigidBoundaryPose {
            pivot: BoundaryKernelVector { x: 0.0, y: 0.5 },
            rotation_radians: 0.0,
        };
        let target_translation = BoundaryKernelVector { x: 0.1, y: 0.05 };
        let blocker_translation = BoundaryKernelVector { x: -0.02, y: 0.04 };
        let target_angular = 0.03;
        let blocker_angular = -0.04;
        let (point, derivative) = resolve_circle_straight_intersection_event(
            &event,
            1,
            6,
            8,
            4,
            target_pose,
            blocker_pose,
            target_translation,
            target_angular,
            blocker_translation,
            blocker_angular,
        )
        .expect("local edge and arc provenance resolve a strict crossing");

        let step = 1.0e-6;
        let perturbed = |direction: f64| {
            let target = RigidBoundaryPose {
                pivot: BoundaryKernelVector {
                    x: target_pose.pivot.x + direction * step * target_translation.x,
                    y: target_pose.pivot.y + direction * step * target_translation.y,
                },
                rotation_radians: target_pose.rotation_radians + direction * step * target_angular,
            };
            let blocker = RigidBoundaryPose {
                pivot: BoundaryKernelVector {
                    x: blocker_pose.pivot.x + direction * step * blocker_translation.x,
                    y: blocker_pose.pivot.y + direction * step * blocker_translation.y,
                },
                rotation_radians: blocker_pose.rotation_radians
                    + direction * step * blocker_angular,
            };
            let start = target
                .transform_local_point(BoundaryKernelVector { x: -2.0, y: 0.0 })
                .unwrap();
            let end = target
                .transform_local_point(BoundaryKernelVector { x: 2.0, y: 0.0 })
                .unwrap();
            let center = blocker
                .transform_local_point(BoundaryKernelVector::default())
                .unwrap();
            let (parameter, intersection) =
                circle_line_intersection(start, end, center, radius, target_parameter);
            let radial = BoundaryKernelVector {
                x: intersection.x - center.x,
                y: intersection.y - center.y,
            };
            let local_angle = (radial.y.atan2(radial.x) - blocker.rotation_radians)
                .rem_euclid(std::f64::consts::TAU);
            (parameter, intersection, local_angle)
        };
        let plus = perturbed(1.0);
        let minus = perturbed(-1.0);
        let point_rate = BoundaryKernelVector {
            x: (plus.1.x - minus.1.x) / (2.0 * step),
            y: (plus.1.y - minus.1.y) / (2.0 * step),
        };
        let parameter_rate = (plus.0 - minus.0) / (2.0 * step);
        let angle_delta = |left: f64, right: f64| {
            (left - right + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU)
                - std::f64::consts::PI
        };
        let angle_rate = angle_delta(plus.2, minus.2) / (2.0 * step);
        assert!((point.x - (-2.0 + target_parameter * 4.0)).abs() < 1.0e-12);
        assert!(point.y.abs() < 1.0e-12);
        assert!((derivative.point_derivative.x - point_rate.x).abs() < 1.0e-7);
        assert!((derivative.point_derivative.y - point_rate.y).abs() < 1.0e-7);
        assert!((derivative.target_parameter_derivative - parameter_rate).abs() < 1.0e-7);
        assert!((derivative.local_arc_angle_derivative - angle_rate).abs() < 1.0e-7);
    }

    #[test]
    fn rigid_curved_endpoint_derivative_matches_finite_difference() {
        let local_center = BoundaryKernelVector { x: 0.2, y: -0.1 };
        let radius = 1.4;
        let angle = 1.1;
        let pose = RigidBoundaryPose {
            pivot: BoundaryKernelVector { x: 2.0, y: -3.0 },
            rotation_radians: 0.4,
        };
        let translation = BoundaryKernelVector { x: -0.1, y: 0.3 };
        let angular = 0.2;
        let (point, derivative) = rigid_circle_arc_endpoint_pose_derivative(
            local_center,
            radius,
            angle,
            pose,
            translation,
            angular,
        )
        .unwrap();
        let step = 1.0e-6;
        let moved = |direction: f64| {
            let next = RigidBoundaryPose {
                pivot: BoundaryKernelVector {
                    x: pose.pivot.x + direction * step * translation.x,
                    y: pose.pivot.y + direction * step * translation.y,
                },
                rotation_radians: pose.rotation_radians + direction * step * angular,
            };
            next.transform_local_point(BoundaryKernelVector {
                x: local_center.x + radius * angle.cos(),
                y: local_center.y + radius * angle.sin(),
            })
            .unwrap()
        };
        let plus = moved(1.0);
        let minus = moved(-1.0);
        assert!(point.x.is_finite() && point.y.is_finite());
        assert!(((plus.x - minus.x) / (2.0 * step) - derivative.x).abs() < 1.0e-8);
        assert!(((plus.y - minus.y) / (2.0 * step) - derivative.y).abs() < 1.0e-8);
    }
}

#[cfg(test)]
mod circle_straight_motion_bound_tests {
    use super::*;

    fn root_near(
        start: BoundaryKernelVector,
        end: BoundaryKernelVector,
        center: BoundaryKernelVector,
        radius: f64,
        reference: f64,
    ) -> (f64, f64) {
        let edge = BoundaryKernelVector {
            x: end.x - start.x,
            y: end.y - start.y,
        };
        let offset = BoundaryKernelVector {
            x: start.x - center.x,
            y: start.y - center.y,
        };
        let a = edge.dot(edge);
        let b = 2.0 * offset.dot(edge);
        let c = offset.dot(offset) - radius * radius;
        let root = (b * b - 4.0 * a * c).sqrt();
        let candidates = [(-b - root) / (2.0 * a), (-b + root) / (2.0 * a)];
        let t = candidates
            .into_iter()
            .min_by(|left, right| {
                (left - reference)
                    .abs()
                    .total_cmp(&(right - reference).abs())
            })
            .unwrap();
        let point = BoundaryKernelVector {
            x: start.x + t * edge.x,
            y: start.y + t * edge.y,
        };
        let angle = (point.y - center.y).atan2(point.x - center.x);
        (t, angle)
    }

    #[test]
    fn circle_straight_pose_cell_bound_contains_small_endpoint_and_pose_changes() {
        let radius = 1.0;
        let start = BoundaryKernelVector { x: -2.0, y: 0.0 };
        let end = BoundaryKernelVector { x: 2.0, y: 0.0 };
        let center = BoundaryKernelVector { x: 0.0, y: 0.5 };
        let target_parameter = (2.0 + 0.75_f64.sqrt()) / 4.0;
        let arc_angle = (-0.5_f64)
            .atan2(0.75_f64.sqrt())
            .rem_euclid(std::f64::consts::TAU);
        let displacement = 0.001;
        let angular_displacement = 0.001;
        let bound = circle_straight_intersection_motion_bound(
            start,
            end,
            center,
            radius,
            target_parameter,
            0.0,
            std::f64::consts::TAU,
            arc_angle,
            displacement,
            displacement,
            displacement,
            angular_displacement,
        )
        .expect("small pose cell stays away from tangency and arc endpoints");
        let directions = [
            (1.0, 0.0),
            (-1.0, 0.0),
            (0.0, 1.0),
            (0.0, -1.0),
            (0.6, 0.8),
            (-0.8, 0.6),
            (0.8, -0.6),
            (-0.6, -0.8),
        ];
        let angle_delta = |left: f64, right: f64| {
            (left - right + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU)
                - std::f64::consts::PI
        };
        for (ia, da) in directions.iter().enumerate() {
            for (ib, db) in directions.iter().enumerate() {
                let dc = directions[(ia + ib) % directions.len()];
                let moved =
                    |point: BoundaryKernelVector, direction: &(f64, f64)| BoundaryKernelVector {
                        x: point.x + displacement * direction.0,
                        y: point.y + displacement * direction.1,
                    };
                let moved_start = moved(start, da);
                let moved_end = moved(end, db);
                let moved_center = moved(center, &dc);
                let (parameter, world_angle) = root_near(
                    moved_start,
                    moved_end,
                    moved_center,
                    radius,
                    target_parameter,
                );
                let blocker_rotation = if (ia + ib) % 2 == 0 {
                    angular_displacement
                } else {
                    -angular_displacement
                };
                let local_angle =
                    (world_angle - blocker_rotation).rem_euclid(std::f64::consts::TAU);
                assert!((parameter - target_parameter).abs() <= bound.target_parameter_change);
                assert!(angle_delta(local_angle, arc_angle).abs() <= bound.local_arc_angle_change);
            }
        }
    }

    #[test]
    fn circle_straight_motion_bound_rejects_tangency_and_arc_endpoint_crossing() {
        let point = |x, y| BoundaryKernelVector { x, y };
        let radius = 1.0;
        let tangent_center = point(0.0, 0.999);
        let tangent_parameter = (2.0 + (radius * radius - 0.999_f64.powi(2)).sqrt()) / 4.0;
        assert!(circle_straight_intersection_motion_bound(
            point(-2.0, 0.0),
            point(2.0, 0.0),
            tangent_center,
            radius,
            tangent_parameter,
            0.0,
            std::f64::consts::TAU,
            0.0,
            0.01,
            0.01,
            0.01,
            0.0,
        )
        .is_none());

        let parameter = (2.0 + 0.75_f64.sqrt()) / 4.0;
        let angle = (-0.5_f64)
            .atan2(0.75_f64.sqrt())
            .rem_euclid(std::f64::consts::TAU);
        assert!(circle_straight_intersection_motion_bound(
            point(-2.0, 0.0),
            point(2.0, 0.0),
            point(0.0, 0.5),
            radius,
            parameter,
            0.0,
            angle + 0.005,
            angle,
            0.001,
            0.001,
            0.001,
            0.001,
        )
        .is_none());
    }
}

#[cfg(test)]
mod curved_endpoint_motion_bound_tests {
    use super::*;

    #[test]
    fn rigid_curved_endpoint_motion_bound_contains_pose_motion_and_derivative() {
        let local_center = BoundaryKernelVector { x: 0.2, y: -0.1 };
        let radius = 1.4;
        let angle = 1.1;
        let translation = BoundaryKernelVector { x: -0.1, y: 0.3 };
        let angular = 0.2;
        let translation_displacement = 0.02;
        let angular_displacement = 0.03;
        let bound = rigid_circle_arc_endpoint_motion_bound(
            local_center,
            radius,
            angle,
            translation,
            angular,
            translation_displacement,
            angular_displacement,
        )
        .expect("finite rigid endpoint envelope");
        let pose = RigidBoundaryPose {
            pivot: BoundaryKernelVector { x: 2.0, y: -3.0 },
            rotation_radians: 0.4,
        };
        let (point, derivative) = rigid_circle_arc_endpoint_pose_derivative(
            local_center,
            radius,
            angle,
            pose,
            translation,
            angular,
        )
        .unwrap();
        assert!(derivative.norm() <= bound.derivative);

        let scale = 0.05;
        let moved_pose = RigidBoundaryPose {
            pivot: BoundaryKernelVector {
                x: pose.pivot.x + scale * translation.x,
                y: pose.pivot.y + scale * translation.y,
            },
            rotation_radians: pose.rotation_radians + scale * angular,
        };
        let moved_point = moved_pose
            .transform_local_point(BoundaryKernelVector {
                x: local_center.x + radius * angle.cos(),
                y: local_center.y + radius * angle.sin(),
            })
            .unwrap();
        assert!((moved_point.x - point.x).hypot(moved_point.y - point.y) <= bound.displacement);
    }

    #[test]
    fn rigid_curved_endpoint_motion_bound_rejects_invalid_envelopes() {
        let zero = BoundaryKernelVector::default();
        assert!(
            rigid_circle_arc_endpoint_motion_bound(zero, 1.0, 0.0, zero, 0.0, -0.1, 0.0).is_none()
        );
        assert!(
            rigid_circle_arc_endpoint_motion_bound(zero, 1.0, 0.0, zero, 0.0, 0.0, f64::NAN)
                .is_none()
        );
    }
}

#[cfg(test)]
mod endpoint_provenance_dispatch_tests {
    use super::*;

    #[test]
    fn typed_intersection_endpoint_dispatches_to_its_rigid_pose_resolver() {
        let event = crate::source_exposure::VisibilityCutEvent::StraightBoundaryIntersection {
            blocker_body_index: 7,
            blocker_unit_index: 3,
            target_body_index: 2,
            target_unit_index: 5,
            target_edge_index: 1,
            blocker_edge_index: 4,
            target_edge_start_local: (-1.0, 0.0),
            target_edge_end_local: (1.0, 0.0),
            blocker_edge_start_local: (0.0, -1.0),
            blocker_edge_end_local: (0.0, 1.0),
            target_parameter: 0.5,
            blocker_parameter: 0.5,
        };
        let provenance =
            crate::source_exposure::BoundaryEndpointProvenance::RigidVisibilityIntersection {
                parameter: 0.5,
                event,
            };
        let target_pose = RigidBoundaryPose {
            pivot: BoundaryKernelVector { x: 2.0, y: 3.0 },
            rotation_radians: 0.3,
        };
        let blocker_pose = RigidBoundaryPose {
            pivot: BoundaryKernelVector { x: 2.0, y: 3.0 },
            rotation_radians: -0.2,
        };
        let target_translation = BoundaryKernelVector { x: 0.2, y: -0.1 };
        let blocker_translation = BoundaryKernelVector { x: -0.05, y: 0.15 };
        let resolved = resolve_boundary_endpoint_pose_derivative(
            &provenance,
            2,
            5,
            7,
            3,
            target_pose,
            blocker_pose,
            target_translation,
            0.3,
            blocker_translation,
            -0.2,
        )
        .expect("typed endpoint provenance should dispatch to the straight resolver");
        assert!((resolved.point.x - 2.0).abs() < 1.0e-12);
        assert!((resolved.point.y - 3.0).abs() < 1.0e-12);
        assert!(resolved.derivative.x.is_finite() && resolved.derivative.y.is_finite());
    }

    #[test]
    fn unresolved_or_mismatched_endpoint_provenance_stays_unresolved() {
        let pose = RigidBoundaryPose::default();
        let zero = BoundaryKernelVector::default();
        let unresolved =
            crate::source_exposure::BoundaryEndpointProvenance::UnresolvedVisibilityCut {
                parameter: 0.5,
                events: Vec::new(),
            };
        assert!(resolve_boundary_endpoint_pose_derivative(
            &unresolved,
            0,
            0,
            1,
            1,
            pose,
            pose,
            zero,
            0.0,
            zero,
            0.0
        )
        .is_none());
    }
}
