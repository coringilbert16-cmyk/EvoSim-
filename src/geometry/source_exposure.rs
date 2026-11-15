//! Geometry-only measurements for directional environmental input.
//!
//! This module intentionally does not allocate energy. Its inputs are boundary
//! intervals that have already been classified as exposed and unoccluded.
//! The visibility/interval-discovery stage must provide disjoint intervals so
//! that overlapping material and shadows cannot be counted twice.

/// A straight, already-exposed boundary interval in world coordinates.
///
/// The normal is the outward unit normal for this interval. Curved boundaries
/// must be split or integrated by their exact arc representation before using
/// this straight-interval primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ExposedBoundaryInterval {
    /// Index into the body slice passed to visible_polygonal_intervals.
    pub body_index: usize,
    /// Realized constituent index within that body.
    pub unit_index: usize,
    pub start: (f64, f64),
    pub end: (f64, f64),
    pub outward_normal: (f64, f64),
}

/// Provenance for one endpoint of an exposed straight interval. Only endpoints
/// that match a rigid target-feature vertex are currently certified; all cuts
/// created by occluders or intersections remain explicitly unresolved.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BoundaryEndpointProvenance {
    RigidTargetVertex {
        body_index: usize,
        unit_index: usize,
        vertex_index: usize,
        local_point: (f64, f64),
        edge_parameter: f64,
    },
    /// A unique rigid straight/straight or circle/straight cut whose local
    /// feature provenance is retained for a pose-aware resolver.
    RigidVisibilityIntersection {
        parameter: f64,
        event: VisibilityCutEvent,
    },
    UnresolvedVisibilityCut {
        parameter: f64,
        events: Vec<VisibilityCutEvent>,
    },
}

/// Construction event that introduced a visibility partition endpoint.
/// Unsupported event classes remain explicit so pose certificates can fail closed.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum VisibilityCutEvent {
    TargetVertex {
        body_index: usize,
        unit_index: usize,
        vertex_index: usize,
        local_point: (f64, f64),
    },
    RayThroughRigidFeature {
        blocker_body_index: usize,
        blocker_unit_index: usize,
        blocker_feature_index: usize,
        target_body_index: usize,
        target_unit_index: usize,
        target_edge_index: usize,
        local_point: (f64, f64),
    },
    RayThroughCircleTangent {
        blocker_body_index: usize,
        blocker_unit_index: usize,
        target_body_index: usize,
        target_unit_index: usize,
        target_edge_index: usize,
        tangent_side: i8,
        local_center: (f64, f64),
        radius: f64,
    },
    StraightBoundaryIntersection {
        blocker_body_index: usize,
        blocker_unit_index: usize,
        target_body_index: usize,
        target_unit_index: usize,
        target_edge_index: usize,
        blocker_edge_index: usize,
        target_edge_start_local: (f64, f64),
        target_edge_end_local: (f64, f64),
        blocker_edge_start_local: (f64, f64),
        blocker_edge_end_local: (f64, f64),
        target_parameter: f64,
        blocker_parameter: f64,
    },
    CircleStraightIntersection {
        blocker_body_index: usize,
        blocker_unit_index: usize,
        target_body_index: usize,
        target_unit_index: usize,
        target_edge_index: usize,
        arc_index: usize,
        target_edge_start_local: (f64, f64),
        target_edge_end_local: (f64, f64),
        blocker_arc_center_local: (f64, f64),
        blocker_arc_radius: f64,
        blocker_arc_start_local: f64,
        blocker_arc_sweep: f64,
        target_parameter: f64,
        arc_angle_local: f64,
    },
    Unresolved {
        blocker_body_index: Option<usize>,
        blocker_unit_index: Option<usize>,
    },
}

/// Stable identity of a visibility event. Geometric coordinates and cut
/// parameters are intentionally excluded so the same physical feature remains
/// identifiable as bodies move within a certified pose cell.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum VisibilityEventIdentity {
    TargetVertex {
        body: usize,
        unit: usize,
        vertex: usize,
    },
    RayThroughRigidFeature {
        blocker_body: usize,
        blocker_unit: usize,
        blocker_feature: usize,
        target_body: usize,
        target_unit: usize,
        target_edge: usize,
    },
    RayThroughCircleTangent {
        blocker_body: usize,
        blocker_unit: usize,
        target_body: usize,
        target_unit: usize,
        target_edge: usize,
        tangent_side: i8,
    },
    StraightBoundaryIntersection {
        blocker_body: usize,
        blocker_unit: usize,
        blocker_edge: usize,
        target_body: usize,
        target_unit: usize,
        target_edge: usize,
    },
    CircleStraightIntersection {
        blocker_body: usize,
        blocker_unit: usize,
        blocker_arc: usize,
        target_body: usize,
        target_unit: usize,
        target_edge: usize,
    },
}

impl VisibilityCutEvent {
    pub(crate) fn stable_identity(&self) -> Option<VisibilityEventIdentity> {
        match self {
            Self::TargetVertex {
                body_index,
                unit_index,
                vertex_index,
                ..
            } => Some(VisibilityEventIdentity::TargetVertex {
                body: *body_index,
                unit: *unit_index,
                vertex: *vertex_index,
            }),
            Self::RayThroughRigidFeature {
                blocker_body_index,
                blocker_unit_index,
                blocker_feature_index,
                target_body_index,
                target_unit_index,
                target_edge_index,
                ..
            } => Some(VisibilityEventIdentity::RayThroughRigidFeature {
                blocker_body: *blocker_body_index,
                blocker_unit: *blocker_unit_index,
                blocker_feature: *blocker_feature_index,
                target_body: *target_body_index,
                target_unit: *target_unit_index,
                target_edge: *target_edge_index,
            }),
            Self::RayThroughCircleTangent {
                blocker_body_index,
                blocker_unit_index,
                target_body_index,
                target_unit_index,
                target_edge_index,
                tangent_side,
                ..
            } => Some(VisibilityEventIdentity::RayThroughCircleTangent {
                blocker_body: *blocker_body_index,
                blocker_unit: *blocker_unit_index,
                target_body: *target_body_index,
                target_unit: *target_unit_index,
                target_edge: *target_edge_index,
                tangent_side: *tangent_side,
            }),
            Self::StraightBoundaryIntersection {
                blocker_body_index,
                blocker_unit_index,
                target_body_index,
                target_unit_index,
                target_edge_index,
                blocker_edge_index,
                ..
            } => Some(VisibilityEventIdentity::StraightBoundaryIntersection {
                blocker_body: *blocker_body_index,
                blocker_unit: *blocker_unit_index,
                blocker_edge: *blocker_edge_index,
                target_body: *target_body_index,
                target_unit: *target_unit_index,
                target_edge: *target_edge_index,
            }),
            Self::CircleStraightIntersection {
                blocker_body_index,
                blocker_unit_index,
                target_body_index,
                target_unit_index,
                target_edge_index,
                arc_index,
                ..
            } => Some(VisibilityEventIdentity::CircleStraightIntersection {
                blocker_body: *blocker_body_index,
                blocker_unit: *blocker_unit_index,
                blocker_arc: *arc_index,
                target_body: *target_body_index,
                target_unit: *target_unit_index,
                target_edge: *target_edge_index,
            }),
            Self::Unresolved { .. } => None,
        }
    }
}

/// Discrete visibility topology for one target edge. It records the ordered
/// stable event identities, conservative candidate-body set, each open
/// interval's occluding body identities, and its visibility classification. It is valid only when every cut has
/// one supported, uniquely identified event and the event order is strict.
/// Coincident, unresolved, or reordered events require exact visibility or
/// subdivision rather than a pose derivative.
#[derive(Clone, Debug)]
pub(crate) struct VisibilityCutSignature {
    candidate_bodies: Vec<usize>,
    interval_visibility: Vec<bool>,
    interval_blocker_bodies: Vec<Vec<(usize, usize)>>,
    ordered_events: Vec<VisibilityEventIdentity>,
    ordered_parameters: Vec<f64>,
}

/// Caller-derived conservative bounds for one event over a complete
/// proposed pose cell. existence_margin is the smallest positive geometric
/// margin that keeps this event valid (for example, a nonzero intersection
/// determinant, an interior parameter margin, or a positive tangency
/// discriminant); its change bound must cover the whole cell.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct VisibilityEventPoseCellBound {
    pub(crate) identity: VisibilityEventIdentity,
    pub(crate) parameter_change_bound: f64,
    pub(crate) existence_margin: f64,
    pub(crate) existence_margin_change_bound: f64,
}

fn visibility_part(
    bodies: &[crate::organism_geometry::OrganismBodyGeometry],
    body_index: usize,
    unit_index: usize,
) -> Option<&crate::organism_geometry::PlacedForm> {
    let mut parts = bodies
        .get(body_index)?
        .parts
        .iter()
        .filter(|part| part.unit_index == unit_index);
    let part = parts.next()?;
    parts.next().is_none().then_some(part)
}

fn local_straight_feature_endpoints(
    part: &crate::organism_geometry::PlacedForm,
    edge_index: usize,
) -> Option<((f64, f64), (f64, f64))> {
    if let Some(vertices) = part.form.polygon_vertices() {
        if vertices.len() < 2 {
            return None;
        }
        return Some((
            *vertices.get(edge_index)?,
            *vertices.get((edge_index + 1) % vertices.len())?,
        ));
    }
    if let crate::resources::Form::Line { length } = &part.form {
        if edge_index != 0 || !length.is_finite() || *length <= 0.0 {
            return None;
        }
        let half = *length * 0.5;
        return Some(((-half, 0.0), (half, 0.0)));
    }
    None
}

fn visibility_local_points_match(left: (f64, f64), right: (f64, f64)) -> bool {
    let scale = left
        .0
        .abs()
        .max(left.1.abs())
        .max(right.0.abs())
        .max(right.1.abs())
        .max(1.0);
    (left.0 - right.0).abs() <= 1.0e-10 * scale && (left.1 - right.1).abs() <= 1.0e-10 * scale
}

fn visibility_world_point(
    part: &crate::organism_geometry::PlacedForm,
    local: (f64, f64),
) -> Option<(f64, f64)> {
    if ![part.x, part.y, part.rotation_radians, local.0, local.1]
        .iter()
        .all(|value| value.is_finite())
    {
        return None;
    }
    let (sin, cos) = part.rotation_radians.sin_cos();
    let point = (
        part.x + local.0 * cos - local.1 * sin,
        part.y + local.0 * sin + local.1 * cos,
    );
    (point.0.is_finite() && point.1.is_finite()).then_some(point)
}

/// Derive a whole-pose-cell bound for a straight/straight visibility event.
/// Endpoint motion is bounded by each owning body's rigid-point displacement;
/// determinant and blocker-segment margins prove the intersection cannot
/// become parallel, leave the blocker segment, or disappear inside the cell.
#[allow(dead_code)]
pub(crate) fn bound_straight_boundary_intersection_event_over_pose_cell(
    event: &VisibilityCutEvent,
    bodies: &[crate::organism_geometry::OrganismBodyGeometry],
    body_motion: &[VisibilityBodyMotionBound],
) -> Option<VisibilityEventPoseCellBound> {
    let (
        blocker_body_index,
        blocker_unit_index,
        target_body_index,
        target_unit_index,
        target_edge_index,
        blocker_edge_index,
        target_start_local,
        target_end_local,
        blocker_start_local,
        blocker_end_local,
        expected_target_parameter,
        expected_blocker_parameter,
    ) = match event {
        VisibilityCutEvent::StraightBoundaryIntersection {
            blocker_body_index,
            blocker_unit_index,
            target_body_index,
            target_unit_index,
            target_edge_index,
            blocker_edge_index,
            target_edge_start_local,
            target_edge_end_local,
            blocker_edge_start_local,
            blocker_edge_end_local,
            target_parameter,
            blocker_parameter,
        } => (
            *blocker_body_index,
            *blocker_unit_index,
            *target_body_index,
            *target_unit_index,
            *target_edge_index,
            *blocker_edge_index,
            *target_edge_start_local,
            *target_edge_end_local,
            *blocker_edge_start_local,
            *blocker_edge_end_local,
            *target_parameter,
            *blocker_parameter,
        ),
        _ => return None,
    };
    if body_motion.len() != bodies.len() {
        return None;
    }
    let target_part = visibility_part(bodies, target_body_index, target_unit_index)?;
    let blocker_part = visibility_part(bodies, blocker_body_index, blocker_unit_index)?;
    let target_edge = local_straight_feature_endpoints(target_part, target_edge_index)?;
    let blocker_edge = local_straight_feature_endpoints(blocker_part, blocker_edge_index)?;
    if !visibility_local_points_match(target_edge.0, target_start_local)
        || !visibility_local_points_match(target_edge.1, target_end_local)
        || !visibility_local_points_match(blocker_edge.0, blocker_start_local)
        || !visibility_local_points_match(blocker_edge.1, blocker_end_local)
    {
        return None;
    }
    let a = visibility_world_point(target_part, target_start_local)?;
    let b = visibility_world_point(target_part, target_end_local)?;
    let c = visibility_world_point(blocker_part, blocker_start_local)?;
    let d = visibility_world_point(blocker_part, blocker_end_local)?;
    let subtract = |left: (f64, f64), right: (f64, f64)| (left.0 - right.0, left.1 - right.1);
    let cross = |left: (f64, f64), right: (f64, f64)| left.0 * right.1 - left.1 * right.0;
    let norm = |vector: (f64, f64)| vector.0.hypot(vector.1);
    let target_vector = subtract(b, a);
    let blocker_vector = subtract(d, c);
    let offset = subtract(c, a);
    let denominator = cross(target_vector, blocker_vector);
    let denominator_abs = denominator.abs();
    let target_length = norm(target_vector);
    let blocker_length = norm(blocker_vector);
    let denominator_scale = target_length * blocker_length;
    if !denominator_abs.is_finite()
        || !denominator_scale.is_finite()
        || denominator_scale <= f64::MIN_POSITIVE
        || denominator_abs <= 64.0 * f64::EPSILON * denominator_scale
    {
        return None;
    }
    let target_numerator = cross(offset, blocker_vector);
    let blocker_numerator = cross(offset, target_vector);
    let target_parameter = target_numerator / denominator;
    let blocker_parameter = blocker_numerator / denominator;
    if !expected_target_parameter.is_finite()
        || !expected_blocker_parameter.is_finite()
        || !target_parameter.is_finite()
        || !blocker_parameter.is_finite()
        || target_parameter <= 0.0
        || target_parameter >= 1.0
        || blocker_parameter <= 0.0
        || blocker_parameter >= 1.0
        || (target_parameter - expected_target_parameter).abs() > 1.0e-8
        || (blocker_parameter - expected_blocker_parameter).abs() > 1.0e-8
    {
        return None;
    }
    for body_index in [target_body_index, blocker_body_index] {
        let motion = *body_motion.get(body_index)?;
        let required_radius =
            maximum_geometry_radius_about_pivot(bodies.get(body_index)?, motion.rotation_pivot)?;
        if motion.maximum_point_displacement()?.is_finite()
            && motion.radius_bound + 1.0e-10 * motion.radius_bound.max(1.0) < required_radius
        {
            return None;
        }
    }
    let target_displacement = body_motion[target_body_index].maximum_point_displacement()?;
    let blocker_displacement = body_motion[blocker_body_index].maximum_point_displacement()?;
    let target_edge_change = 2.0 * target_displacement;
    let blocker_edge_change = 2.0 * blocker_displacement;
    let offset_change = target_displacement + blocker_displacement;
    let denominator_change = target_edge_change * blocker_length
        + target_length * blocker_edge_change
        + target_edge_change * blocker_edge_change;
    let target_numerator_change = offset_change * blocker_length
        + norm(offset) * blocker_edge_change
        + offset_change * blocker_edge_change;
    let blocker_numerator_change = offset_change * target_length
        + norm(offset) * target_edge_change
        + offset_change * target_edge_change;
    let denominator_lower_bound = denominator_abs - denominator_change;
    if ![
        denominator_change,
        target_numerator_change,
        blocker_numerator_change,
        denominator_lower_bound,
    ]
    .iter()
    .all(|value| value.is_finite())
        || denominator_lower_bound <= 0.0
    {
        return None;
    }
    let denominator_product = denominator_abs * denominator_lower_bound;
    if !denominator_product.is_finite() || denominator_product <= 0.0 {
        return None;
    }
    let target_parameter_change = (target_numerator_change * denominator_abs
        + target_numerator.abs() * denominator_change)
        / denominator_product;
    let blocker_parameter_change = (blocker_numerator_change * denominator_abs
        + blocker_numerator.abs() * denominator_change)
        / denominator_product;
    let determinant_margin = denominator_abs / denominator_scale;
    let determinant_change = denominator_change / denominator_scale;
    let blocker_parameter_margin = blocker_parameter.min(1.0 - blocker_parameter);
    let existence_margin = determinant_margin.min(blocker_parameter_margin);
    let existence_change = determinant_change.max(blocker_parameter_change);
    if ![
        target_parameter_change,
        blocker_parameter_change,
        determinant_margin,
        determinant_change,
        blocker_parameter_margin,
        existence_margin,
        existence_change,
    ]
    .iter()
    .all(|value| value.is_finite())
        || target_parameter_change < 0.0
        || blocker_parameter_change < 0.0
        || existence_margin <= existence_change
    {
        return None;
    }
    Some(VisibilityEventPoseCellBound {
        identity: event.stable_identity()?,
        parameter_change_bound: target_parameter_change,
        existence_margin,
        existence_margin_change_bound: existence_change,
    })
}

impl PartialEq for VisibilityCutSignature {
    fn eq(&self, other: &Self) -> bool {
        self.candidate_bodies == other.candidate_bodies
            && self.interval_visibility == other.interval_visibility
            && self.interval_blocker_bodies == other.interval_blocker_bodies
            && self.ordered_events == other.ordered_events
    }
}

impl VisibilityCutSignature {
    /// Prove that each event remains valid and that its parameter interval
    /// stays strictly ordered throughout the pose cell. The caller must supply
    /// independently conservative bounds for every event; sampled equality
    /// alone is not a continuous-cell proof.
    #[allow(dead_code)]
    pub(crate) fn certify_pose_cell(
        &self,
        event_bounds: &[VisibilityEventPoseCellBound],
    ) -> Option<()> {
        if event_bounds.len() != self.ordered_events.len()
            || self.ordered_parameters.len() != self.ordered_events.len()
        {
            return None;
        }
        let mut parameter_intervals = Vec::with_capacity(event_bounds.len());
        for ((identity, parameter), bound) in self
            .ordered_events
            .iter()
            .zip(&self.ordered_parameters)
            .zip(event_bounds)
        {
            if *identity != bound.identity
                || !parameter.is_finite()
                || !bound.parameter_change_bound.is_finite()
                || bound.parameter_change_bound < 0.0
                || !bound.existence_margin.is_finite()
                || bound.existence_margin <= 0.0
                || !bound.existence_margin_change_bound.is_finite()
                || bound.existence_margin_change_bound < 0.0
                || bound.existence_margin <= bound.existence_margin_change_bound
            {
                return None;
            }
            let is_target_vertex = matches!(identity, VisibilityEventIdentity::TargetVertex { .. });
            if is_target_vertex {
                if bound.parameter_change_bound != 0.0 || (*parameter != 0.0 && *parameter != 1.0) {
                    return None;
                }
                parameter_intervals.push((*parameter, *parameter));
            } else {
                let lower = *parameter - bound.parameter_change_bound;
                let upper = *parameter + bound.parameter_change_bound;
                if !lower.is_finite() || !upper.is_finite() || lower <= 0.0 || upper >= 1.0 {
                    return None;
                }
                parameter_intervals.push((lower, upper));
            }
        }
        if parameter_intervals
            .windows(2)
            .any(|pair| pair[0].1 >= pair[1].0)
        {
            return None;
        }
        Some(())
    }

    #[allow(dead_code)]
    pub(crate) fn from_cuts(cuts: &[VisibilityCutRecord]) -> Option<Self> {
        use std::collections::BTreeSet;

        if cuts.len() < 2 {
            return None;
        }
        let mut previous_parameter = f64::NEG_INFINITY;
        let mut identities = BTreeSet::new();
        let mut ordered_events = Vec::with_capacity(cuts.len());
        for cut in cuts {
            if cut.unresolved
                || cut.events.len() != 1
                || !cut.parameter.is_finite()
                || cut.parameter <= previous_parameter
            {
                return None;
            }
            previous_parameter = cut.parameter;
            let identity = cut.events[0].stable_identity()?;
            if !identities.insert(identity) {
                return None;
            }
            ordered_events.push(identity);
        }
        Some(Self {
            candidate_bodies: Vec::new(),
            interval_visibility: Vec::new(),
            interval_blocker_bodies: Vec::new(),
            ordered_parameters: cuts.iter().map(|cut| cut.parameter).collect(),
            ordered_events,
        })
    }

    /// Build a complete partition signature from the conservative candidate
    /// body set and the visibility classification of every open cut interval.
    #[allow(dead_code)]
    pub(crate) fn from_partition(
        cuts: &[VisibilityCutRecord],
        candidate_bodies: &[usize],
        interval_blocker_bodies: &[Vec<(usize, usize)>],
    ) -> Option<Self> {
        if candidate_bodies.windows(2).any(|pair| pair[0] >= pair[1])
            || interval_blocker_bodies.len() != cuts.len().checked_sub(1)?
            || interval_blocker_bodies.iter().any(|blockers| {
                blockers.windows(2).any(|pair| pair[0] >= pair[1])
                    || blockers
                        .iter()
                        .any(|(body, _)| candidate_bodies.binary_search(body).is_err())
            })
        {
            return None;
        }
        let mut signature = Self::from_cuts(cuts)?;
        signature.candidate_bodies = candidate_bodies.to_vec();
        signature.interval_visibility = interval_blocker_bodies
            .iter()
            .map(|blockers| !blockers.is_empty())
            .collect();
        signature.interval_blocker_bodies = interval_blocker_bodies.to_vec();
        Some(signature)
    }

    /// Require every sampled pose in a proposed cell to retain this exact
    /// discrete event order. A birth, death, identity change, or reordering
    /// invalidates the smooth-pose certificate.
    #[allow(dead_code)]
    pub(crate) fn certify_unchanged_across(&self, samples: &[Self]) -> Option<()> {
        if samples.is_empty() || samples.iter().any(|sample| sample != self) {
            return None;
        }
        Some(())
    }
}

/// A partition location retaining every event that generated the cut.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct VisibilityCutRecord {
    pub(crate) parameter: f64,
    pub(crate) events: Vec<VisibilityCutEvent>,
    pub(crate) unresolved: bool,
}

impl VisibilityCutRecord {
    fn new(parameter: f64, event: VisibilityCutEvent) -> Self {
        let unresolved = matches!(event, VisibilityCutEvent::Unresolved { .. });
        Self {
            parameter,
            events: vec![event],
            unresolved,
        }
    }

    fn merge_coincident(&mut self, other: Self) {
        for event in other.events {
            if !self.events.contains(&event) {
                self.events.push(event);
            }
        }
        self.unresolved |= other.unresolved;
        if self.events.len() > 1 {
            self.unresolved = true;
        }
    }
}

/// Merge coincident cuts with the legacy tolerance. The first sorted parameter
/// remains the representative, preserving the existing interval coordinates.
fn merge_visibility_cuts(mut cuts: Vec<VisibilityCutRecord>) -> Vec<VisibilityCutRecord> {
    cuts.sort_by(|left, right| left.parameter.total_cmp(&right.parameter));
    let mut merged: Vec<VisibilityCutRecord> = Vec::with_capacity(cuts.len());
    for cut in cuts {
        if let Some(previous) = merged.last_mut() {
            if (previous.parameter - cut.parameter).abs() <= 1.0e-10 {
                previous.merge_coincident(cut);
                continue;
            }
        }
        merged.push(cut);
    }
    merged
}

/// Convert an exact world-space boundary point back into the local frame
/// of the uniquely identified placed constituent. Ambiguous unit identities
/// fail closed instead of selecting an arbitrary matching part.
fn localize_boundary_point(
    bodies: &[crate::organism_geometry::OrganismBodyGeometry],
    body_index: usize,
    unit_index: usize,
    point: (f64, f64),
) -> Option<(f64, f64)> {
    let body = bodies.get(body_index)?;
    let mut matching_parts = body
        .parts
        .iter()
        .filter(|part| part.unit_index == unit_index);
    let part = matching_parts.next()?;
    if matching_parts.next().is_some()
        || !part.x.is_finite()
        || !part.y.is_finite()
        || !part.rotation_radians.is_finite()
        || !point.0.is_finite()
        || !point.1.is_finite()
    {
        return None;
    }
    let (sin, cos) = part.rotation_radians.sin_cos();
    let dx = point.0 - part.x;
    let dy = point.1 - part.y;
    let local = (dx * cos + dy * sin, -dx * sin + dy * cos);
    (local.0.is_finite() && local.1.is_finite()).then_some(local)
}

fn endpoint_provenance_from_cut(
    cut: &VisibilityCutRecord,
    target_body_index: usize,
    target_unit_index: usize,
) -> BoundaryEndpointProvenance {
    if cut.unresolved || cut.events.len() != 1 {
        return BoundaryEndpointProvenance::UnresolvedVisibilityCut {
            parameter: cut.parameter,
            events: cut.events.clone(),
        };
    }
    match cut.events.first() {
        Some(VisibilityCutEvent::TargetVertex {
            body_index,
            unit_index,
            vertex_index,
            local_point,
        }) if *body_index == target_body_index && *unit_index == target_unit_index => {
            BoundaryEndpointProvenance::RigidTargetVertex {
                body_index: *body_index,
                unit_index: *unit_index,
                vertex_index: *vertex_index,
                local_point: *local_point,
                edge_parameter: cut.parameter,
            }
        }
        Some(
            event @ VisibilityCutEvent::StraightBoundaryIntersection {
                target_body_index: body_index,
                target_unit_index: unit_index,
                target_parameter,
                ..
            },
        )
        | Some(
            event @ VisibilityCutEvent::CircleStraightIntersection {
                target_body_index: body_index,
                target_unit_index: unit_index,
                target_parameter,
                ..
            },
        ) if *body_index == target_body_index
            && *unit_index == target_unit_index
            && (*target_parameter - cut.parameter).abs() <= 1.0e-10 =>
        {
            BoundaryEndpointProvenance::RigidVisibilityIntersection {
                parameter: cut.parameter,
                event: event.clone(),
            }
        }
        Some(
            event @ VisibilityCutEvent::RayThroughRigidFeature {
                target_body_index: body_index,
                target_unit_index: unit_index,
                ..
            },
        )
        | Some(
            event @ VisibilityCutEvent::RayThroughCircleTangent {
                target_body_index: body_index,
                target_unit_index: unit_index,
                ..
            },
        ) if *body_index == target_body_index && *unit_index == target_unit_index => {
            BoundaryEndpointProvenance::RigidVisibilityIntersection {
                parameter: cut.parameter,
                event: event.clone(),
            }
        }
        _ => BoundaryEndpointProvenance::UnresolvedVisibilityCut {
            parameter: cut.parameter,
            events: cut.events.clone(),
        },
    }
}

/// Exact exposed interval paired with conservative endpoint provenance.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ExposedBoundaryIntervalWithProvenance {
    pub(crate) interval: ExposedBoundaryInterval,
    pub(crate) start_provenance: BoundaryEndpointProvenance,
    pub(crate) end_provenance: BoundaryEndpointProvenance,
    /// Stable ordered feature identity for the complete target-edge partition.
    /// `None` means a coincident, unresolved, or otherwise unsupported event
    /// prevents differentiating this visibility topology.
    #[allow(dead_code)]
    pub(crate) visibility_signature: Option<VisibilityCutSignature>,
}

/// Return the interval's contribution to projected extent for a directional
/// source. toward_source is the unit direction from the interval toward the
/// source. The result is length * max(0, n dot s), the 2-D analogue of
/// projected surface area. It is zero for edge-on or back-facing intervals.
///
/// This does not discover visibility, split partially shadowed edges, integrate
/// curved boundaries, or credit energy. Callers must supply a finite unit
/// source direction and non-overlapping intervals; invalid input returns None.
#[allow(dead_code)] // Used when exact exposed intervals are available.
pub(crate) fn projected_extent(
    interval: ExposedBoundaryInterval,
    toward_source: (f64, f64),
) -> Option<f64> {
    let values = [
        interval.start.0,
        interval.start.1,
        interval.end.0,
        interval.end.1,
        interval.outward_normal.0,
        interval.outward_normal.1,
        toward_source.0,
        toward_source.1,
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return None;
    }

    let normal_length = interval.outward_normal.0.hypot(interval.outward_normal.1);
    let direction_length = toward_source.0.hypot(toward_source.1);
    if (normal_length - 1.0).abs() > 1e-9 || (direction_length - 1.0).abs() > 1e-9 {
        return None;
    }

    let length = (interval.end.0 - interval.start.0).hypot(interval.end.1 - interval.start.1);
    if !length.is_finite() {
        return None;
    }

    let facing = (interval.outward_normal.0 * toward_source.0
        + interval.outward_normal.1 * toward_source.1)
        .max(0.0);
    let extent = length * facing;
    extent.is_finite().then_some(extent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocker_feature_ordinals_are_distinct_and_repeatable_per_constituent() {
        let points = vec![
            (1, 7, 0.0, 0.0),
            (1, 7, 1.0, 0.0),
            (0, 3, 2.0, 0.0),
            (1, 9, 3.0, 0.0),
            (1, 7, 4.0, 0.0),
        ];
        let first = index_critical_points_by_body(2, points.clone()).unwrap();
        let second = index_critical_points_by_body(2, points).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            first[1]
                .iter()
                .filter(|(unit, _, _, _)| *unit == 7)
                .map(|(_, feature, _, _)| *feature)
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert_eq!(first[1][2].0, 9);
        assert_eq!(first[0][0].1, 0);
        assert!(index_critical_points_by_body(1, vec![(1, 0, 0.0, 0.0)]).is_none());
    }

    #[test]
    fn target_edge_blocker_inventory_retains_ordered_body_and_feature_identities() {
        let critical = vec![
            vec![(2, 0, 1.0, 2.0)],
            vec![(7, 0, 3.0, 4.0), (7, 1, 5.0, 6.0)],
        ];
        let edges = vec![
            vec![(2, 0, (0.0, 0.0), (1.0, 0.0))],
            vec![(7, 3, (2.0, 0.0), (2.0, 1.0))],
        ];
        let arc = WorldCircleArc {
            body_index: 1,
            unit_index: 7,
            center: (3.0, 4.0),
            radius: 1.5,
            start: 0.25,
            sweep: 1.0,
            center_local: (0.0, 0.0),
            start_local: 0.25,
        };
        let arcs = vec![vec![], vec![(7, 0, arc)]];
        let inventory =
            build_target_edge_blocker_inventory(&[0, 1], &critical, &edges, &arcs).unwrap();
        assert_eq!(inventory.candidate_bodies.len(), 2);
        assert_eq!(inventory.candidate_bodies[1].body_index, 1);
        assert_eq!(
            inventory.candidate_bodies[1]
                .critical_features
                .iter()
                .map(|(unit, feature, _, _)| (*unit, *feature))
                .collect::<Vec<_>>(),
            vec![(7, 0), (7, 1)]
        );
        assert_eq!(
            inventory.candidate_bodies[1]
                .straight_edges
                .iter()
                .map(|(unit, edge, _, _)| (*unit, *edge))
                .collect::<Vec<_>>(),
            vec![(7, 3)]
        );
        assert_eq!(
            inventory.candidate_bodies[1]
                .circular_arcs
                .iter()
                .map(|(unit, arc, _)| (*unit, *arc))
                .collect::<Vec<_>>(),
            vec![(7, 0)]
        );
        assert!(build_target_edge_blocker_inventory(&[1, 0], &critical, &edges, &arcs).is_none());
        assert!(build_target_edge_blocker_inventory(&[2], &critical, &edges, &arcs).is_none());

        let duplicate_feature = vec![vec![(2, 0, 1.0, 2.0), (2, 0, 3.0, 4.0)]];
        assert!(
            build_target_edge_blocker_inventory(&[0], &duplicate_feature, &edges, &arcs).is_none()
        );
        let duplicate_arc = vec![vec![], vec![(7, 0, arc), (7, 0, arc)]];
        assert!(
            build_target_edge_blocker_inventory(&[1], &critical, &edges, &duplicate_arc).is_none()
        );
        let mismatched_arc = WorldCircleArc {
            body_index: 0,
            ..arc
        };
        let mismatched_arcs = vec![vec![], vec![(7, 0, mismatched_arc)]];
        assert!(
            build_target_edge_blocker_inventory(&[1], &critical, &edges, &mismatched_arcs)
                .is_none()
        );
    }

    #[test]
    fn pose_cell_visibility_broad_phase_includes_swept_blockers() {
        let target_bounds = (5.0, 6.0, -0.5, 0.5);
        let body_bounds = [target_bounds, (7.0, 8.0, -0.5, 0.5)];
        let still = VisibilityBodyMotionBound::default();
        let still_motion = [still, still];

        let current = candidate_body_indices_for_pose_cell(
            (0.0, 0.0),
            0.0,
            target_bounds,
            still,
            0.0,
            &body_bounds,
            &still_motion,
        )
        .expect("finite stationary bounds should produce candidates");
        assert_eq!(current, vec![0]);

        let blocker_moves = [
            still,
            VisibilityBodyMotionBound {
                translation_bound: 1.0,
                ..still
            },
        ];
        let swept = candidate_body_indices_for_pose_cell(
            (0.0, 0.0),
            0.0,
            target_bounds,
            still,
            0.0,
            &body_bounds,
            &blocker_moves,
        )
        .expect("swept blocker bounds should produce candidates");
        assert_eq!(swept, vec![0, 1]);

        let source_moves = candidate_body_indices_for_pose_cell(
            (0.0, 0.0),
            1.0,
            target_bounds,
            still,
            0.0,
            &body_bounds,
            &still_motion,
        )
        .expect("source motion must expand relative blocker bounds");
        assert_eq!(source_moves, vec![0, 1]);

        let blocker_rotates = [
            still,
            VisibilityBodyMotionBound {
                angular_bound_radians: std::f64::consts::FRAC_PI_2,
                radius_bound: 2.0,
                ..still
            },
        ];
        let rotated = candidate_body_indices_for_pose_cell(
            (0.0, 0.0),
            0.0,
            target_bounds,
            still,
            0.0,
            &body_bounds,
            &blocker_rotates,
        )
        .expect("rotational sweep bounds should produce candidates");
        assert_eq!(rotated, vec![0, 1]);
    }

    #[test]
    fn moving_and_rotating_blockers_remain_in_inventory_across_pose_cell_extremes() {
        use crate::organism_geometry::{OrganismBodyGeometry, PlacedForm};
        use crate::resources::Form;

        let geometry = |form: Form, x: f64, y: f64, rotation_radians: f64| {
            let radius = form.bounding_radius();
            OrganismBodyGeometry {
                parts: vec![PlacedForm {
                    unit_index: 0,
                    form,
                    x,
                    y,
                    rotation_radians,
                }],
                min_x: x - radius,
                max_x: x + radius,
                min_y: y - radius,
                max_y: y + radius,
            }
        };
        let source = (-5.0, 0.0);
        let reference = vec![
            geometry(
                Form::Rectangle {
                    width: 2.0,
                    height: 0.2,
                },
                0.0,
                1.0,
                0.0,
            ),
            geometry(
                Form::Rectangle {
                    width: 1.0,
                    height: 1.0,
                },
                5.0,
                0.0,
                0.0,
            ),
        ];
        let motion = [
            VisibilityBodyMotionBound {
                translation_bound: 0.5,
                angular_bound_radians: std::f64::consts::FRAC_PI_2,
                rotation_pivot: (0.0, 1.0),
                radius_bound: 1.1,
            },
            VisibilityBodyMotionBound {
                translation_bound: 0.2,
                angular_bound_radians: 0.2,
                rotation_pivot: (5.0, 0.0),
                radius_bound: 0.75,
            },
        ];
        let inventory =
            build_visibility_pose_cell_candidate_inventory(source, &reference, &[1], 0.25, &motion)
                .expect("complete translation/rotation envelopes should build");
        assert!(
            inventory
                .candidates_for_target(1)
                .expect("target inventory")
                .contains(&0),
            "the blocker must be retained because its swept rigid pose crosses the source-target corridor"
        );
        let reference_intervals = visible_polygonal_intervals_for_targets_with_pose_cell_inventory(
            source,
            &reference,
            &[1],
            0.25,
            &motion,
            &inventory,
        )
        .expect("reference pose should use the built blocker inventory");
        let reference_signature = reference_intervals
            .iter()
            .find_map(|interval| interval.visibility_signature.as_ref())
            .expect("reference boundary should retain its ordered visibility signature");
        inventory
            .certifies_target_signature(source, &reference, &[1], 1, reference_signature)
            .expect("boundary signature must be tied to the same certified blocker inventory");

        let blocker_translations = [
            (0.0, 0.0),
            (0.35, 0.0),
            (-0.35, 0.0),
            (0.0, 0.35),
            (0.0, -0.35),
        ];
        let blocker_rotations = [
            -std::f64::consts::FRAC_PI_2,
            0.0,
            std::f64::consts::FRAC_PI_2,
        ];
        let target_translations = [(-0.1, 0.0), (0.0, 0.0), (0.1, 0.0)];
        let target_rotations = [-0.2, 0.0, 0.2];
        let source_offsets = [-0.25, 0.0, 0.25];

        // Exercise the cross-product of translation and rotation limits rather
        // than a few correlated poses. Every sample remains inside the declared
        // rigid-body pose cells; the swept inventory must match exact visibility.
        for (blocker_dx, blocker_dy) in blocker_translations {
            for blocker_rotation in blocker_rotations {
                for (target_dx, target_dy) in target_translations {
                    for target_rotation in target_rotations {
                        for source_dx in source_offsets {
                            let pose_source = (source.0 + source_dx, source.1);
                            let bodies = vec![
                                geometry(
                                    Form::Rectangle {
                                        width: 2.0,
                                        height: 0.2,
                                    },
                                    blocker_dx,
                                    1.0 + blocker_dy,
                                    blocker_rotation,
                                ),
                                geometry(
                                    Form::Rectangle {
                                        width: 1.0,
                                        height: 1.0,
                                    },
                                    5.0 + target_dx,
                                    target_dy,
                                    target_rotation,
                                ),
                            ];
                            let indexed =
                                visible_polygonal_intervals_for_targets_with_pose_cell_inventory(
                                    pose_source,
                                    &bodies,
                                    &[1],
                                    0.25,
                                    &motion,
                                    &inventory,
                                )
                                .expect(
                                    "every sampled pose inside the declared cell must be covered",
                                );
                            let exact =
                                visible_polygonal_intervals_for_targets_with_cut_provenance(
                                    pose_source,
                                    &bodies,
                                    &[1],
                                )
                                .expect("exact current-pose visibility must remain available");
                            for signature in indexed
                                .iter()
                                .filter_map(|interval| interval.visibility_signature.as_ref())
                            {
                                inventory
                                    .certifies_target_signature(
                                        pose_source,
                                        &bodies,
                                        &[1],
                                        1,
                                        signature,
                                    )
                                    .expect("every moving/rotating pose signature must bind to the same swept inventory");
                            }
                            assert_eq!(indexed.len(), exact.len());
                            for (indexed_interval, exact_interval) in indexed.iter().zip(&exact) {
                                assert_eq!(indexed_interval.interval, exact_interval.interval);
                                assert_eq!(
                                    indexed_interval.start_provenance,
                                    exact_interval.start_provenance
                                );
                                assert_eq!(
                                    indexed_interval.end_provenance,
                                    exact_interval.end_provenance
                                );
                            }
                        }
                    }
                }
            }
        }

        let outside_translation = vec![
            geometry(
                Form::Rectangle {
                    width: 2.0,
                    height: 0.2,
                },
                0.51,
                1.0,
                std::f64::consts::FRAC_PI_2,
            ),
            reference[1].clone(),
        ];
        assert!(
            visible_polygonal_intervals_for_targets_with_pose_cell_inventory(
                source,
                &outside_translation,
                &[1],
                0.25,
                &motion,
                &inventory,
            )
            .is_none(),
            "rotation allowance must not mask translation beyond the declared pose cell"
        );
    }

    #[test]
    fn certified_inventory_reaches_generalized_boundary_pose_evaluator() {
        use crate::organism_geometry::{OrganismBodyGeometry, PlacedForm};
        use crate::resources::Form;

        let form = Form::Rectangle {
            width: 1.0,
            height: 1.0,
        };
        let radius = form.bounding_radius();
        let bodies = vec![OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index: 0,
                form,
                x: 5.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            min_x: 5.0 - radius,
            max_x: 5.0 + radius,
            min_y: -radius,
            max_y: radius,
        }];
        let source = (-5.0, 0.0);
        let motion = [VisibilityBodyMotionBound {
            translation_bound: 0.0,
            angular_bound_radians: 0.0,
            rotation_pivot: (5.0, 0.0),
            radius_bound: radius,
        }];
        let inventory =
            build_visibility_pose_cell_candidate_inventory(source, &bodies, &[0], 0.0, &motion)
                .expect("stationary target inventory should be valid");
        let intervals = visible_polygonal_intervals_for_targets_with_pose_cell_inventory(
            source,
            &bodies,
            &[0],
            0.0,
            &motion,
            &inventory,
        )
        .expect("target visibility should use the certified inventory");
        let signature = intervals
            .iter()
            .find_map(|interval| interval.visibility_signature.as_ref())
            .expect("visible target boundary should have a stable signature");
        let event_bounds = signature
            .ordered_events
            .iter()
            .cloned()
            .map(|identity| VisibilityEventPoseCellBound {
                identity,
                parameter_change_bound: 0.0,
                existence_margin: 1.0,
                existence_margin_change_bound: 0.0,
            })
            .collect::<Vec<_>>();
        let ordered_cuts = signature
            .ordered_parameters
            .iter()
            .filter(|parameter| **parameter > 1.0e-10 && **parameter < 1.0 - 1.0e-10)
            .map(|parameter| (*parameter, 0.0))
            .collect::<Vec<_>>();
        let point = crate::resonance_boundary_far_field::BoundaryKernelVector { x: 5.0, y: 0.5 };
        let expansion = crate::resonance_boundary_far_field::BoundaryKernelExpansion::new(
            crate::resonance_boundary_far_field::BoundaryKernelVector { x: 0.0, y: 0.0 },
            crate::resonance_boundary_far_field::BoundaryKernelVector { x: 5.0, y: 0.0 },
            point,
            vec![
                crate::resonance_boundary_far_field::WeightedBoundaryKernelSample {
                    position: point,
                    weight: 1.0,
                },
            ],
        )
        .expect("finite boundary sample should form an expansion");
        let coordinate = crate::resonance_boundary_far_field::BoundaryPoseCoordinateDerivative {
            source_center_derivative:
                crate::resonance_boundary_far_field::BoundaryKernelVector::default(),
            receiver_center_derivative:
                crate::resonance_boundary_far_field::BoundaryKernelVector::default(),
            source_center_derivative_bound: 0.0,
            receiver_center_derivative_bound: 0.0,
            samples: vec![
                crate::resonance_boundary_far_field::BoundarySamplePoseDerivative::default(),
            ],
        };
        crate::resonance_boundary_far_field::certify_boundary_pose_cell_generalized_with_blocker_inventory(
            &expansion,
            signature,
            std::slice::from_ref(signature),
            &ordered_cuts,
            &[crate::resonance_boundary_far_field::BoundarySampleMotionBound::default()],
            &event_bounds,
            &[coordinate],
            &inventory,
            source,
            &bodies,
            &[0],
            0,
        )
        .expect("complete stationary visibility proof should reach generalized pose evaluation");
    }

    #[test]
    fn pose_cell_visibility_broad_phase_rejects_incomplete_or_invalid_motion_inventory() {
        let bounds = [(5.0, 6.0, -0.5, 0.5)];
        let still = VisibilityBodyMotionBound::default();
        assert!(candidate_body_indices_for_pose_cell(
            (0.0, 0.0),
            0.0,
            bounds[0],
            still,
            0.0,
            &bounds,
            &[],
        )
        .is_none());
        assert!(candidate_body_indices_for_pose_cell(
            (0.0, 0.0),
            0.0,
            bounds[0],
            still,
            0.0,
            &bounds,
            &[VisibilityBodyMotionBound {
                translation_bound: f64::NAN,
                ..still
            }],
        )
        .is_none());
    }

    #[test]
    fn visibility_cut_signature_tracks_stable_feature_order_not_coordinates() {
        let event = |x| VisibilityCutEvent::RayThroughRigidFeature {
            blocker_body_index: 2,
            blocker_unit_index: 7,
            blocker_feature_index: 3,
            target_body_index: 0,
            target_unit_index: 1,
            target_edge_index: 4,
            local_point: (x, 0.25),
        };
        let first = vec![
            VisibilityCutRecord::new(
                0.0,
                VisibilityCutEvent::TargetVertex {
                    body_index: 0,
                    unit_index: 1,
                    vertex_index: 0,
                    local_point: (0.0, 0.0),
                },
            ),
            VisibilityCutRecord::new(0.4, event(0.5)),
            VisibilityCutRecord::new(
                1.0,
                VisibilityCutEvent::TargetVertex {
                    body_index: 0,
                    unit_index: 1,
                    vertex_index: 1,
                    local_point: (1.0, 0.0),
                },
            ),
        ];
        let moved = vec![
            VisibilityCutRecord::new(
                0.0,
                VisibilityCutEvent::TargetVertex {
                    body_index: 0,
                    unit_index: 1,
                    vertex_index: 0,
                    local_point: (0.0, 0.0),
                },
            ),
            VisibilityCutRecord::new(0.45, event(0.7)),
            VisibilityCutRecord::new(
                1.0,
                VisibilityCutEvent::TargetVertex {
                    body_index: 0,
                    unit_index: 1,
                    vertex_index: 1,
                    local_point: (1.0, 0.0),
                },
            ),
        ];
        assert_eq!(
            VisibilityCutSignature::from_cuts(&first),
            VisibilityCutSignature::from_cuts(&moved)
        );

        let reordered = vec![first[0].clone(), first[2].clone(), first[1].clone()];
        assert!(VisibilityCutSignature::from_cuts(&reordered).is_none());
    }

    #[test]
    fn visibility_signature_requires_stable_identity_and_partition_across_pose_samples() {
        let make_signature =
            |blocker_feature_index, candidates: &[usize], blockers: &[Vec<(usize, usize)>]| {
                let cuts = vec![
                    VisibilityCutRecord::new(
                        0.0,
                        VisibilityCutEvent::TargetVertex {
                            body_index: 0,
                            unit_index: 1,
                            vertex_index: 0,
                            local_point: (0.0, 0.0),
                        },
                    ),
                    VisibilityCutRecord::new(
                        0.5,
                        VisibilityCutEvent::RayThroughRigidFeature {
                            blocker_body_index: 2,
                            blocker_unit_index: 7,
                            blocker_feature_index,
                            target_body_index: 0,
                            target_unit_index: 1,
                            target_edge_index: 4,
                            local_point: (0.5, 0.25),
                        },
                    ),
                    VisibilityCutRecord::new(
                        1.0,
                        VisibilityCutEvent::TargetVertex {
                            body_index: 0,
                            unit_index: 1,
                            vertex_index: 1,
                            local_point: (1.0, 0.0),
                        },
                    ),
                ];
                VisibilityCutSignature::from_partition(&cuts, candidates, blockers).unwrap()
            };
        let baseline = make_signature(3, &[0, 2], &[vec![], vec![(2, 7)]]);
        let make_event_bounds = |parameter_change_bound, existence_margin, margin_change_bound| {
            baseline
                .ordered_events
                .iter()
                .map(|identity| VisibilityEventPoseCellBound {
                    identity: *identity,
                    parameter_change_bound: if matches!(
                        identity,
                        VisibilityEventIdentity::TargetVertex { .. }
                    ) {
                        0.0
                    } else {
                        parameter_change_bound
                    },
                    existence_margin,
                    existence_margin_change_bound: margin_change_bound,
                })
                .collect::<Vec<_>>()
        };
        assert!(baseline
            .certify_pose_cell(&make_event_bounds(0.05, 0.2, 0.1))
            .is_some());
        assert!(baseline
            .certify_pose_cell(&make_event_bounds(0.05, 0.05, 0.1))
            .is_none());
        assert!(baseline
            .certify_pose_cell(&make_event_bounds(0.6, 0.2, 0.1))
            .is_none());
        assert!(baseline
            .certify_unchanged_across(&[
                make_signature(3, &[0, 2], &[vec![], vec![(2, 7)]]),
                make_signature(3, &[0, 2], &[vec![], vec![(2, 7)]]),
            ])
            .is_some());
        assert!(baseline
            .certify_unchanged_across(&[make_signature(4, &[0, 2], &[vec![], vec![(2, 7)]])])
            .is_none());
        assert!(baseline
            .certify_unchanged_across(&[make_signature(3, &[0, 2, 3], &[vec![], vec![(2, 7)]])])
            .is_none());
        assert!(baseline
            .certify_unchanged_across(&[make_signature(3, &[0, 2], &[vec![], vec![(0, 1)]])])
            .is_none());
        assert!(baseline
            .certify_unchanged_across(&[make_signature(3, &[0, 2], &[vec![], vec![]])])
            .is_none());
        assert!(VisibilityCutSignature::from_partition(
            &[
                VisibilityCutRecord::new(
                    0.0,
                    VisibilityCutEvent::TargetVertex {
                        body_index: 0,
                        unit_index: 1,
                        vertex_index: 0,
                        local_point: (0.0, 0.0),
                    },
                ),
                VisibilityCutRecord::new(
                    1.0,
                    VisibilityCutEvent::TargetVertex {
                        body_index: 0,
                        unit_index: 1,
                        vertex_index: 1,
                        local_point: (1.0, 0.0),
                    },
                ),
            ],
            &[0, 2],
            &[vec![(3, 8)]],
        )
        .is_none());
        assert!(baseline.certify_unchanged_across(&[]).is_none());
    }

    #[test]
    fn straight_intersection_event_derives_conservative_pose_cell_bounds() {
        let target = crate::organism_geometry::OrganismBodyGeometry {
            parts: vec![crate::physical_body_geometry::PlacedForm {
                unit_index: 0,
                form: crate::resources::Form::Rectangle {
                    width: 2.0,
                    height: 2.0,
                },
                x: 5.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            min_x: 4.0,
            max_x: 6.0,
            min_y: -1.0,
            max_y: 1.0,
        };
        let target_vertices = crate::resources::Form::Rectangle {
            width: 2.0,
            height: 2.0,
        }
        .polygon_vertices()
        .unwrap();
        let target_edge_index = (0..target_vertices.len())
            .find(|&index| {
                let start = target_vertices[index];
                let end = target_vertices[(index + 1) % target_vertices.len()];
                (start.0 + 1.0).abs() < 1.0e-12
                    && (end.0 + 1.0).abs() < 1.0e-12
                    && (start.1 - end.1).abs() > 1.0e-12
            })
            .expect("rectangle has a left edge");
        let target_start_local = target_vertices[target_edge_index];
        let target_end_local = target_vertices[(target_edge_index + 1) % target_vertices.len()];
        let blocker = crate::organism_geometry::OrganismBodyGeometry {
            parts: vec![crate::physical_body_geometry::PlacedForm {
                unit_index: 7,
                form: crate::resources::Form::Line { length: 2.0 },
                x: 4.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            min_x: 3.0,
            max_x: 5.0,
            min_y: 0.0,
            max_y: 0.0,
        };
        let bodies = [target, blocker];
        let event = VisibilityCutEvent::StraightBoundaryIntersection {
            blocker_body_index: 1,
            blocker_unit_index: 7,
            target_body_index: 0,
            target_unit_index: 0,
            target_edge_index,
            blocker_edge_index: 0,
            target_edge_start_local: target_start_local,
            target_edge_end_local: target_end_local,
            blocker_edge_start_local: (-1.0, 0.0),
            blocker_edge_end_local: (1.0, 0.0),
            target_parameter: 0.5,
            blocker_parameter: 0.5,
        };
        let cuts = vec![
            VisibilityCutRecord::new(
                0.0,
                VisibilityCutEvent::TargetVertex {
                    body_index: 0,
                    unit_index: 0,
                    vertex_index: target_edge_index,
                    local_point: target_start_local,
                },
            ),
            VisibilityCutRecord::new(0.5, event.clone()),
            VisibilityCutRecord::new(
                1.0,
                VisibilityCutEvent::TargetVertex {
                    body_index: 0,
                    unit_index: 0,
                    vertex_index: (target_edge_index + 1) % target_vertices.len(),
                    local_point: target_end_local,
                },
            ),
        ];
        let signature =
            VisibilityCutSignature::from_partition(&cuts, &[0, 1], &[vec![], vec![]]).unwrap();
        let motion = [
            VisibilityBodyMotionBound {
                translation_bound: 0.01,
                angular_bound_radians: 0.01,
                rotation_pivot: (5.0, 0.0),
                radius_bound: 2.0,
            },
            VisibilityBodyMotionBound {
                translation_bound: 0.01,
                angular_bound_radians: 0.01,
                rotation_pivot: (4.0, 0.0),
                radius_bound: 2.0,
            },
        ];
        let event_bound =
            bound_straight_boundary_intersection_event_over_pose_cell(&event, &bodies, &motion)
                .expect("small rigid-pose cell preserves the straight intersection");
        let bounds = signature
            .ordered_events
            .iter()
            .map(|identity| {
                if *identity == event_bound.identity {
                    event_bound
                } else {
                    VisibilityEventPoseCellBound {
                        identity: *identity,
                        parameter_change_bound: 0.0,
                        existence_margin: 1.0,
                        existence_margin_change_bound: 0.0,
                    }
                }
            })
            .collect::<Vec<_>>();
        assert!(signature.certify_pose_cell(&bounds).is_some());

        let large_motion = [
            VisibilityBodyMotionBound {
                translation_bound: 1.0,
                ..motion[0]
            },
            VisibilityBodyMotionBound {
                translation_bound: 1.0,
                ..motion[1]
            },
        ];
        assert!(bound_straight_boundary_intersection_event_over_pose_cell(
            &event,
            &bodies,
            &large_motion,
        )
        .is_none());
    }

    #[test]
    fn visibility_cut_signature_rejects_unresolved_and_duplicate_feature_events() {
        let unresolved = vec![
            VisibilityCutRecord::new(
                0.0,
                VisibilityCutEvent::TargetVertex {
                    body_index: 0,
                    unit_index: 1,
                    vertex_index: 0,
                    local_point: (0.0, 0.0),
                },
            ),
            VisibilityCutRecord::new(
                0.5,
                VisibilityCutEvent::Unresolved {
                    blocker_body_index: Some(2),
                    blocker_unit_index: Some(7),
                },
            ),
            VisibilityCutRecord::new(
                1.0,
                VisibilityCutEvent::TargetVertex {
                    body_index: 0,
                    unit_index: 1,
                    vertex_index: 1,
                    local_point: (1.0, 0.0),
                },
            ),
        ];
        assert!(VisibilityCutSignature::from_cuts(&unresolved).is_none());

        let duplicate = vec![
            VisibilityCutRecord::new(
                0.0,
                VisibilityCutEvent::TargetVertex {
                    body_index: 0,
                    unit_index: 1,
                    vertex_index: 0,
                    local_point: (0.0, 0.0),
                },
            ),
            VisibilityCutRecord::new(
                0.4,
                VisibilityCutEvent::RayThroughRigidFeature {
                    blocker_body_index: 2,
                    blocker_unit_index: 7,
                    blocker_feature_index: 3,
                    target_body_index: 0,
                    target_unit_index: 1,
                    target_edge_index: 4,
                    local_point: (0.5, 0.25),
                },
            ),
            VisibilityCutRecord::new(
                0.7,
                VisibilityCutEvent::RayThroughRigidFeature {
                    blocker_body_index: 2,
                    blocker_unit_index: 7,
                    blocker_feature_index: 3,
                    target_body_index: 0,
                    target_unit_index: 1,
                    target_edge_index: 4,
                    local_point: (0.6, 0.25),
                },
            ),
            VisibilityCutRecord::new(
                1.0,
                VisibilityCutEvent::TargetVertex {
                    body_index: 0,
                    unit_index: 1,
                    vertex_index: 1,
                    local_point: (1.0, 0.0),
                },
            ),
        ];
        assert!(VisibilityCutSignature::from_cuts(&duplicate).is_none());
    }

    #[test]
    fn coincident_visibility_cuts_preserve_distinct_events_and_fail_closed() {
        let first = VisibilityCutRecord::new(
            0.25,
            VisibilityCutEvent::RayThroughRigidFeature {
                blocker_body_index: 2,
                blocker_unit_index: 7,
                blocker_feature_index: 2,
                target_body_index: 0,
                target_unit_index: 3,
                target_edge_index: 1,
                local_point: (0.5, -0.25),
            },
        );
        let second = VisibilityCutRecord::new(
            0.25 + 5.0e-11,
            VisibilityCutEvent::StraightBoundaryIntersection {
                blocker_body_index: 3,
                blocker_unit_index: 9,
                target_body_index: 0,
                target_unit_index: 2,
                target_edge_index: 4,
                blocker_edge_index: 6,
                target_edge_start_local: (-1.0, 0.0),
                target_edge_end_local: (1.0, 0.0),
                blocker_edge_start_local: (0.0, -1.0),
                blocker_edge_end_local: (0.0, 1.0),
                target_parameter: 0.25,
                blocker_parameter: 0.75,
            },
        );
        let merged = merge_visibility_cuts(vec![second, first]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].events.len(), 2);
        assert!(merged[0].unresolved);
    }

    #[test]
    fn rigid_intersection_endpoint_retains_resolvable_feature_identity() {
        let event = VisibilityCutEvent::StraightBoundaryIntersection {
            blocker_body_index: 3,
            blocker_unit_index: 9,
            target_body_index: 0,
            target_unit_index: 2,
            target_edge_index: 4,
            blocker_edge_index: 6,
            target_edge_start_local: (-1.0, 0.0),
            target_edge_end_local: (1.0, 0.0),
            blocker_edge_start_local: (0.0, -1.0),
            blocker_edge_end_local: (0.0, 1.0),
            target_parameter: 0.25,
            blocker_parameter: 0.75,
        };
        let cut = VisibilityCutRecord::new(0.25, event.clone());
        assert_eq!(
            endpoint_provenance_from_cut(&cut, 0, 2),
            BoundaryEndpointProvenance::RigidVisibilityIntersection {
                parameter: 0.25,
                event,
            }
        );
    }

    #[test]
    fn ray_visibility_endpoints_retain_rigid_feature_provenance() {
        let feature = VisibilityCutEvent::RayThroughRigidFeature {
            blocker_body_index: 3,
            blocker_unit_index: 9,
            blocker_feature_index: 2,
            target_body_index: 0,
            target_unit_index: 2,
            target_edge_index: 4,
            local_point: (0.5, -0.25),
        };
        let feature_cut = VisibilityCutRecord::new(0.4, feature.clone());
        assert_eq!(
            endpoint_provenance_from_cut(&feature_cut, 0, 2),
            BoundaryEndpointProvenance::RigidVisibilityIntersection {
                parameter: 0.4,
                event: feature,
            }
        );

        let tangent = VisibilityCutEvent::RayThroughCircleTangent {
            blocker_body_index: 3,
            blocker_unit_index: 9,
            target_body_index: 0,
            target_unit_index: 2,
            target_edge_index: 4,
            tangent_side: 1,
            local_center: (0.0, 0.0),
            radius: 0.75,
        };
        let tangent_cut = VisibilityCutRecord::new(0.6, tangent.clone());
        assert_eq!(
            endpoint_provenance_from_cut(&tangent_cut, 0, 2),
            BoundaryEndpointProvenance::RigidVisibilityIntersection {
                parameter: 0.6,
                event: tangent,
            }
        );
    }

    #[test]
    fn duplicate_visibility_events_are_deduplicated() {
        let event = VisibilityCutEvent::Unresolved {
            blocker_body_index: Some(4),
            blocker_unit_index: Some(11),
        };
        let merged = merge_visibility_cuts(vec![
            VisibilityCutRecord::new(0.5, event.clone()),
            VisibilityCutRecord::new(0.5, event),
        ]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].events.len(), 1);
        assert!(merged[0].unresolved);
    }

    fn interval(normal: (f64, f64)) -> ExposedBoundaryInterval {
        ExposedBoundaryInterval {
            body_index: 0,
            unit_index: 0,
            start: (0.0, 0.0),
            end: (2.0, 0.0),
            outward_normal: normal,
        }
    }

    #[test]
    fn directly_facing_edge_contributes_its_full_length() {
        assert_eq!(
            projected_extent(interval((0.0, 1.0)), (0.0, 1.0)),
            Some(2.0)
        );
    }

    #[test]
    fn edge_on_and_back_facing_edges_contribute_zero() {
        assert_eq!(
            projected_extent(interval((1.0, 0.0)), (0.0, 1.0)),
            Some(0.0)
        );
        assert_eq!(
            projected_extent(interval((0.0, -1.0)), (0.0, 1.0)),
            Some(0.0)
        );
    }

    #[test]
    fn oblique_edge_uses_cosine_projection_not_an_arbitrary_weight() {
        let diagonal = std::f64::consts::FRAC_1_SQRT_2;
        let oblique = ExposedBoundaryInterval {
            body_index: 0,
            unit_index: 0,
            start: (0.0, 0.0),
            end: (2.0 * diagonal, -2.0 * diagonal),
            outward_normal: (diagonal, diagonal),
        };
        let extent = projected_extent(oblique, (diagonal, diagonal)).unwrap();
        assert!((extent - 2.0).abs() < 1e-12);

        let half_projection = projected_extent(oblique, (0.0, 1.0)).unwrap();
        assert!((half_projection - std::f64::consts::SQRT_2).abs() < 1e-12);
    }

    #[test]
    fn rejects_non_unit_normals_and_source_directions() {
        assert_eq!(projected_extent(interval((0.0, 2.0)), (0.0, 1.0)), None);
        assert_eq!(projected_extent(interval((0.0, 1.0)), (0.0, 0.0)), None);
    }

    #[test]
    fn straight_projected_extent_gradient_matches_finite_differences() {
        let boundary = ExposedBoundaryInterval {
            body_index: 0,
            unit_index: 0,
            start: (-1.0, 0.0),
            end: (1.0, 0.0),
            outward_normal: (0.0, 1.0),
        };
        let source = (0.3, 2.0);
        let (_, gradient) = point_source_projected_extent_with_gradient(boundary, source).unwrap();
        let step = 1e-6;
        let dx = (point_source_projected_extent(boundary, (source.0 + step, source.1)).unwrap()
            - point_source_projected_extent(boundary, (source.0 - step, source.1)).unwrap())
            / (2.0 * step);
        let dy = (point_source_projected_extent(boundary, (source.0, source.1 + step)).unwrap()
            - point_source_projected_extent(boundary, (source.0, source.1 - step)).unwrap())
            / (2.0 * step);
        assert!((gradient.0 - dx).abs() <= 1e-8);
        assert!((gradient.1 - dy).abs() <= 1e-8);
    }

    #[test]
    fn straight_projected_extent_gradient_rejects_edge_on_transition() {
        let boundary = interval((0.0, 1.0));
        assert!(point_source_projected_extent_with_gradient(boundary, (0.5, 0.0)).is_none());
    }

    #[test]
    fn point_source_projection_integrates_varying_direction_along_a_straight_edge() {
        let interval = ExposedBoundaryInterval {
            body_index: 0,
            unit_index: 0,
            start: (-1.0, 0.0),
            end: (1.0, 0.0),
            outward_normal: (0.0, 1.0),
        };
        let extent = point_source_projected_extent(interval, (0.0, 10.0)).unwrap();
        let expected = 20.0 * 0.1_f64.asinh();
        assert!((extent - expected).abs() < 1e-12);
        assert!(
            extent < 2.0,
            "point-source projection must not assume parallel rays"
        );
    }
}

/// Project a straight boundary interval using the finite point source's
/// direction at every point along the interval. Because the outward normal is
/// constant along a straight edge, the integral has a closed-form asinh result.
#[allow(dead_code)]
pub(crate) fn point_source_projected_extent(
    interval: ExposedBoundaryInterval,
    source: (f64, f64),
) -> Option<f64> {
    let values = [
        interval.start.0,
        interval.start.1,
        interval.end.0,
        interval.end.1,
        interval.outward_normal.0,
        interval.outward_normal.1,
        source.0,
        source.1,
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return None;
    }
    let normal_length = interval.outward_normal.0.hypot(interval.outward_normal.1);
    if (normal_length - 1.0).abs() > 1e-9 {
        return None;
    }
    let dx = interval.end.0 - interval.start.0;
    let dy = interval.end.1 - interval.start.1;
    let length = dx.hypot(dy);
    if !length.is_finite() || length <= 1e-12 {
        return None;
    }
    let tangent = (dx / length, dy / length);
    let source_offset = (source.0 - interval.start.0, source.1 - interval.start.1);
    let along = source_offset.0 * tangent.0 + source_offset.1 * tangent.1;
    let normal_distance =
        source_offset.0 * interval.outward_normal.0 + source_offset.1 * interval.outward_normal.1;
    if !along.is_finite() || !normal_distance.is_finite() {
        return None;
    }
    if normal_distance <= 0.0 {
        return Some(0.0);
    }
    let extent = normal_distance * ((length - along) / normal_distance).asinh()
        + normal_distance * (along / normal_distance).asinh();
    (extent.is_finite() && extent >= 0.0).then_some(extent)
}

/// Return the exact straight-interval projected extent and its gradient
/// with respect to the source position, assuming the visible interval and its
/// outward normal remain unchanged. At an edge-on transition the derivative
/// is not smooth, so this function fails closed and the caller must use the
/// exact visibility-aware finite-difference path.
pub(crate) fn point_source_projected_extent_with_gradient(
    interval: ExposedBoundaryInterval,
    source: (f64, f64),
) -> Option<(f64, (f64, f64))> {
    let extent = point_source_projected_extent(interval, source)?;
    let dx = interval.end.0 - interval.start.0;
    let dy = interval.end.1 - interval.start.1;
    let length = dx.hypot(dy);
    let tangent = (dx / length, dy / length);
    let source_offset = (source.0 - interval.start.0, source.1 - interval.start.1);
    let along = source_offset.0 * tangent.0 + source_offset.1 * tangent.1;
    let normal_distance =
        source_offset.0 * interval.outward_normal.0 + source_offset.1 * interval.outward_normal.1;
    if !normal_distance.is_finite() || normal_distance.abs() <= 1e-12 {
        return None;
    }
    if normal_distance < 0.0 {
        return Some((extent, (0.0, 0.0)));
    }

    let remaining = length - along;
    let first = (normal_distance * normal_distance + along * along).sqrt();
    let second = (normal_distance * normal_distance + remaining * remaining).sqrt();
    if !first.is_finite() || !second.is_finite() || first <= 0.0 || second <= 0.0 {
        return None;
    }
    let derivative_along = normal_distance / first - normal_distance / second;
    let derivative_normal = (remaining / normal_distance).asinh()
        + (along / normal_distance).asinh()
        - remaining / second
        - along / first;
    let gradient = (
        derivative_along * tangent.0 + derivative_normal * interval.outward_normal.0,
        derivative_along * tangent.1 + derivative_normal * interval.outward_normal.1,
    );
    [extent, gradient.0, gradient.1]
        .iter()
        .all(|value| value.is_finite())
        .then_some((extent, gradient))
}

/// Bound the change in straight-edge projected extent under certified
/// source and endpoint displacement. The edge normal is assumed to remain the
/// consistently oriented unit normal of the moving edge; edge collapse and a
/// source/edge collision fail closed.
#[allow(dead_code)]
pub(crate) fn point_source_projected_extent_change_bound(
    interval: ExposedBoundaryInterval,
    source: (f64, f64),
    source_displacement: f64,
    start_displacement: f64,
    end_displacement: f64,
) -> Option<f64> {
    let values = [
        interval.start.0,
        interval.start.1,
        interval.end.0,
        interval.end.1,
        interval.outward_normal.0,
        interval.outward_normal.1,
        source.0,
        source.1,
        source_displacement,
        start_displacement,
        end_displacement,
    ];
    if values.iter().any(|value| !value.is_finite())
        || source_displacement < 0.0
        || start_displacement < 0.0
        || end_displacement < 0.0
    {
        return None;
    }

    let edge = (
        interval.end.0 - interval.start.0,
        interval.end.1 - interval.start.1,
    );
    let length = edge.0.hypot(edge.1);
    let normal_length = interval.outward_normal.0.hypot(interval.outward_normal.1);
    if !length.is_finite()
        || length <= f64::EPSILON
        || (normal_length - 1.0).abs() > 1e-9
        || (interval.outward_normal.0 * edge.0 + interval.outward_normal.1 * edge.1).abs()
            > 1e-9 * length.max(1.0)
    {
        return None;
    }

    let edge_displacement = start_displacement + end_displacement;
    let minimum_length = length - edge_displacement;
    if !edge_displacement.is_finite()
        || !minimum_length.is_finite()
        || minimum_length <= f64::EPSILON
    {
        return None;
    }

    let source_offset = (source.0 - interval.start.0, source.1 - interval.start.1);
    let parameter =
        ((source_offset.0 * edge.0 + source_offset.1 * edge.1) / (length * length)).clamp(0.0, 1.0);
    let closest = (
        interval.start.0 + parameter * edge.0,
        interval.start.1 + parameter * edge.1,
    );
    let source_distance = (source.0 - closest.0).hypot(source.1 - closest.1);
    let maximum_point_displacement = source_displacement + start_displacement.max(end_displacement);
    let minimum_distance = source_distance - maximum_point_displacement;
    if !source_distance.is_finite()
        || !maximum_point_displacement.is_finite()
        || !minimum_distance.is_finite()
        || minimum_distance <= f64::EPSILON
    {
        return None;
    }

    let normal_change = (2.0 * edge_displacement / minimum_length).min(2.0);
    let direction_change = (2.0 * maximum_point_displacement / minimum_distance).min(2.0);
    let facing_change = (normal_change + direction_change).min(1.0);

    // E = L * integral_0^1 max(0, n dot direction(s)) ds. The facing
    // integrand is in [0,1] and is Lipschitz in both unit vectors.
    let extent_change = edge_displacement + length * facing_change;
    let roundoff_guard = 64.0 * f64::EPSILON * (extent_change + length + source_distance + 1.0);
    let result = extent_change + roundoff_guard;
    result.is_finite().then_some(result)
}

fn polygon_is_convex(vertices: &[(f64, f64)]) -> bool {
    if vertices.len() < 3 {
        return false;
    }

    // The visibility solver excludes the target constituent from its own
    // occlusion test. That is correct for convex material, but a concave or
    // self-intersecting target can shadow itself and would be over-credited.
    // Require every vertex to lie on one side of each edge; ambiguous shapes
    // fail closed instead of being approximated.
    for index in 0..vertices.len() {
        let a = vertices[index];
        let b = vertices[(index + 1) % vertices.len()];
        let mut side = 0.0_f64;
        for (other_index, &point) in vertices.iter().enumerate() {
            if other_index == index || other_index == (index + 1) % vertices.len() {
                continue;
            }
            let cross = (b.0 - a.0) * (point.1 - a.1) - (b.1 - a.1) * (point.0 - a.0);
            if !cross.is_finite() {
                return false;
            }
            if cross != 0.0 {
                if side != 0.0 && cross.signum() != side.signum() {
                    return false;
                }
                side = cross;
            }
        }
        if side == 0.0 {
            return false;
        }
    }
    true
}

/// Derive visible straight boundary intervals for polygonal and polygon-boundary
/// fluid targets. Curved and line constituents remain in the occluder snapshot:
/// tangencies, endpoints, and boundary intersections split straight targets
/// before exact realized-geometry occlusion classifies each open interval.
/// Curved receiver intervals are returned separately by the arc query.
fn angle_interval_for_bounds(
    source: (f64, f64),
    bounds: (f64, f64, f64, f64),
    reference_angle: f64,
) -> Option<(f64, f64)> {
    if ![
        source.0,
        source.1,
        bounds.0,
        bounds.1,
        bounds.2,
        bounds.3,
        reference_angle,
    ]
    .iter()
    .all(|value| value.is_finite())
        || bounds.0 > bounds.1
        || bounds.2 > bounds.3
    {
        return None;
    }
    if source.0 >= bounds.0 && source.0 <= bounds.1 && source.1 >= bounds.2 && source.1 <= bounds.3
    {
        return Some((-std::f64::consts::PI, std::f64::consts::PI));
    }
    let corners = [
        (bounds.0, bounds.2),
        (bounds.0, bounds.3),
        (bounds.1, bounds.2),
        (bounds.1, bounds.3),
    ];
    let mut minimum = f64::INFINITY;
    let mut maximum = f64::NEG_INFINITY;
    for (x, y) in corners {
        let dx = x - source.0;
        let dy = y - source.1;
        if dx.hypot(dy) <= f64::EPSILON {
            return Some((-std::f64::consts::PI, std::f64::consts::PI));
        }
        let angle = dy.atan2(dx);
        let relative = (angle - reference_angle + std::f64::consts::PI)
            .rem_euclid(std::f64::consts::TAU)
            - std::f64::consts::PI;
        minimum = minimum.min(relative);
        maximum = maximum.max(relative);
    }
    if maximum - minimum > std::f64::consts::PI {
        Some((-std::f64::consts::PI, std::f64::consts::PI))
    } else {
        Some((minimum, maximum))
    }
}

fn reference_angle_for_bodies(
    source: (f64, f64),
    bodies: &[crate::organism_geometry::OrganismBodyGeometry],
) -> Option<f64> {
    let Some(body) = bodies.first() else {
        return Some(0.0);
    };
    let center = (
        (body.min_x + body.max_x) * 0.5,
        (body.min_y + body.max_y) * 0.5,
    );
    if !center.0.is_finite() || !center.1.is_finite() {
        return None;
    }
    Some(
        if (center.0 - source.0).hypot(center.1 - source.1) <= f64::EPSILON {
            0.0
        } else {
            (center.1 - source.1).atan2(center.0 - source.0)
        },
    )
}

fn body_angular_intervals(
    source: (f64, f64),
    bodies: &[crate::organism_geometry::OrganismBodyGeometry],
    reference_angle: f64,
) -> Option<Vec<(f64, f64)>> {
    bodies
        .iter()
        .map(|body| {
            angle_interval_for_bounds(
                source,
                (body.min_x, body.max_x, body.min_y, body.max_y),
                reference_angle,
            )
        })
        .collect()
}

fn body_minimum_distances(
    source: (f64, f64),
    bodies: &[crate::organism_geometry::OrganismBodyGeometry],
) -> Option<Vec<f64>> {
    bodies
        .iter()
        .map(|body| {
            let dx = if source.0 < body.min_x {
                body.min_x - source.0
            } else if source.0 > body.max_x {
                source.0 - body.max_x
            } else {
                0.0
            };
            let dy = if source.1 < body.min_y {
                body.min_y - source.1
            } else if source.1 > body.max_y {
                source.1 - body.max_y
            } else {
                0.0
            };
            let distance = dx.hypot(dy);
            distance.is_finite().then_some(distance)
        })
        .collect()
}

fn candidate_body_indices_for_bounds(
    source: (f64, f64),
    target_bounds: (f64, f64, f64, f64),
    reference_angle: f64,
    body_intervals: &[(f64, f64)],
    body_min_distances: &[f64],
) -> Option<Vec<usize>> {
    if body_intervals.len() != body_min_distances.len() {
        return None;
    }
    let target_interval = angle_interval_for_bounds(source, target_bounds, reference_angle)?;
    // The farthest corner of the target AABB is an upper bound on every point
    // along the target edge/arc. A body whose nearest AABB point is farther
    // from the source cannot block that target, even if their angles overlap.
    let target_max_distance = [
        (target_bounds.0, target_bounds.2),
        (target_bounds.0, target_bounds.3),
        (target_bounds.1, target_bounds.2),
        (target_bounds.1, target_bounds.3),
    ]
    .into_iter()
    .map(|(x, y)| (x - source.0).hypot(y - source.1))
    .fold(0.0_f64, f64::max);
    if !target_max_distance.is_finite() {
        return None;
    }
    let mut candidates = Vec::new();
    for (body_index, (interval, minimum_distance)) in body_intervals
        .iter()
        .copied()
        .zip(body_min_distances.iter().copied())
        .enumerate()
    {
        let angular_overlap =
            interval.0 <= target_interval.1 + 1e-10 && target_interval.0 <= interval.1 + 1e-10;
        let could_be_in_front = minimum_distance <= target_max_distance + 1e-9;
        if angular_overlap && could_be_in_front {
            candidates.push(body_index);
        }
    }
    Some(candidates)
}

/// Conservative rigid-pose motion envelope used to expand visibility
/// broad-phase bounds over an entire proposed pose cell. The radius_bound is the
/// maximum distance from the body's rotation pivot to any point on its shape.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct VisibilityBodyMotionBound {
    pub(crate) translation_bound: f64,
    pub(crate) angular_bound_radians: f64,
    /// Reference-pose rotation pivot in world coordinates.
    pub(crate) rotation_pivot: (f64, f64),
    /// Conservative radius about rotation_pivot, checked against realized forms.
    pub(crate) radius_bound: f64,
}

/// Source and per-body movement envelopes for one proposed visibility pose cell.
pub(crate) struct VisibilityPoseCellMotionEnvelope<'a> {
    pub(crate) source_translation_bound: f64,
    pub(crate) body_motion: &'a [VisibilityBodyMotionBound],
}

impl VisibilityBodyMotionBound {
    fn maximum_point_displacement(self) -> Option<f64> {
        if !self.translation_bound.is_finite()
            || self.translation_bound < 0.0
            || !self.angular_bound_radians.is_finite()
            || self.angular_bound_radians < 0.0
            || !self.rotation_pivot.0.is_finite()
            || !self.rotation_pivot.1.is_finite()
            || !self.radius_bound.is_finite()
            || self.radius_bound < 0.0
        {
            return None;
        }
        let half_angle = self.angular_bound_radians.min(std::f64::consts::PI) * 0.5;
        let rotational = 2.0 * self.radius_bound * half_angle.sin();
        let displacement = self.translation_bound + rotational;
        displacement.is_finite().then_some(displacement)
    }
}

/// One constituent's reference pose within a visibility pose cell.
type VisibilityReferencePartPose = (usize, crate::resources::Form, f64, f64, f64);
type VisibilityReferenceBodyPoses = Vec<VisibilityReferencePartPose>;

/// Candidate blocker superset for every target body throughout one rigid-pose cell.
/// Build once at the reference pose, then reuse for all pose samples in that cell.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct VisibilityPoseCellCandidateInventory {
    source_reference: (f64, f64),
    source_translation_bound: f64,
    body_motion: Vec<VisibilityBodyMotionBound>,
    reference_part_poses: Vec<VisibilityReferenceBodyPoses>,
    target_candidates: Vec<Option<Vec<usize>>>,
}

impl VisibilityPoseCellCandidateInventory {
    fn validate_pose(
        &self,
        source: (f64, f64),
        bodies: &[crate::organism_geometry::OrganismBodyGeometry],
        target_body_indices: &[usize],
    ) -> Option<()> {
        if bodies.len() != self.body_motion.len()
            || self.reference_part_poses.len() != bodies.len()
            || !source.0.is_finite()
            || !source.1.is_finite()
        {
            return None;
        }
        let source_displacement =
            (source.0 - self.source_reference.0).hypot(source.1 - self.source_reference.1);
        if !source_displacement.is_finite()
            || source_displacement
                > self.source_translation_bound + 1.0e-10 * self.source_translation_bound.max(1.0)
        {
            return None;
        }
        for ((body, reference_parts), motion) in bodies
            .iter()
            .zip(&self.reference_part_poses)
            .zip(&self.body_motion)
        {
            if body.parts.len() != reference_parts.len() {
                return None;
            }
            if body.parts.is_empty() {
                return None;
            }
            let maximum_displacement = motion.maximum_point_displacement()?;
            let translation_tolerance = 1.0e-10 * motion.translation_bound.max(1.0);
            let angular_tolerance = 1.0e-10 * motion.angular_bound_radians.max(1.0);
            let mut inferred_translation: Option<(f64, f64)> = None;
            let mut inferred_rotation: Option<f64> = None;
            for (part, (unit_index, form, x, y, rotation)) in body.parts.iter().zip(reference_parts)
            {
                if part.unit_index != *unit_index
                    || &part.form != form
                    || !part.x.is_finite()
                    || !part.y.is_finite()
                    || !part.rotation_radians.is_finite()
                {
                    return None;
                }
                let displacement = (part.x - *x).hypot(part.y - *y);
                let angular_displacement = (part.rotation_radians - *rotation
                    + std::f64::consts::PI)
                    .rem_euclid(std::f64::consts::TAU)
                    - std::f64::consts::PI;
                if !displacement.is_finite()
                    || !angular_displacement.is_finite()
                    || angular_displacement.abs() > motion.angular_bound_radians + angular_tolerance
                    || displacement > maximum_displacement + translation_tolerance
                {
                    return None;
                }

                let (sin, cos) = angular_displacement.sin_cos();
                let reference_offset = (*x - motion.rotation_pivot.0, *y - motion.rotation_pivot.1);
                let rotated_reference = (
                    motion.rotation_pivot.0 + reference_offset.0 * cos - reference_offset.1 * sin,
                    motion.rotation_pivot.1 + reference_offset.0 * sin + reference_offset.1 * cos,
                );
                let translation = (part.x - rotated_reference.0, part.y - rotated_reference.1);
                if !translation.0.is_finite() || !translation.1.is_finite() {
                    return None;
                }
                if let Some((expected_x, expected_y)) = inferred_translation {
                    if (translation.0 - expected_x).hypot(translation.1 - expected_y)
                        > translation_tolerance
                    {
                        return None;
                    }
                } else {
                    inferred_translation = Some(translation);
                }
                if let Some(expected_rotation) = inferred_rotation {
                    let rotation_difference = (angular_displacement - expected_rotation
                        + std::f64::consts::PI)
                        .rem_euclid(std::f64::consts::TAU)
                        - std::f64::consts::PI;
                    if rotation_difference.abs() > angular_tolerance {
                        return None;
                    }
                } else {
                    inferred_rotation = Some(angular_displacement);
                }
            }
            let translation = inferred_translation?;
            if translation.0.hypot(translation.1) > motion.translation_bound + translation_tolerance
            {
                return None;
            }
        }
        for &target in target_body_indices {
            self.target_candidates.get(target)?.as_ref()?;
        }
        Some(())
    }

    fn candidates_for_target(&self, target_body_index: usize) -> Option<&[usize]> {
        self.target_candidates.get(target_body_index)?.as_deref()
    }

    /// Verify that a boundary signature was evaluated with this exact
    /// conservative blocker superset and that every reported blocker belongs
    /// to that inventory. The boundary evaluator still requires independent
    /// whole-cell event bounds before accepting a smooth pose derivative.
    pub(crate) fn certifies_target_signature(
        &self,
        source: (f64, f64),
        bodies: &[crate::organism_geometry::OrganismBodyGeometry],
        target_body_indices: &[usize],
        target_body_index: usize,
        signature: &VisibilityCutSignature,
    ) -> Option<()> {
        if !target_body_indices.contains(&target_body_index) {
            return None;
        }
        self.validate_pose(source, bodies, target_body_indices)?;
        let candidates = self.candidates_for_target(target_body_index)?;
        if signature.candidate_bodies.as_slice() != candidates
            || signature.interval_blocker_bodies.iter().any(|blockers| {
                blockers
                    .iter()
                    .any(|(body_index, _)| candidates.binary_search(body_index).is_err())
            })
        {
            return None;
        }
        Some(())
    }
}

fn maximum_geometry_radius_about_pivot(
    body: &crate::organism_geometry::OrganismBodyGeometry,
    pivot: (f64, f64),
) -> Option<f64> {
    if !pivot.0.is_finite() || !pivot.1.is_finite() {
        return None;
    }
    let mut maximum = 0.0_f64;
    for part in &body.parts {
        if !part.x.is_finite() || !part.y.is_finite() {
            return None;
        }
        let shape_radius = part.form.bounding_radius();
        if !shape_radius.is_finite() || shape_radius < 0.0 {
            return None;
        }
        let radius = (part.x - pivot.0).hypot(part.y - pivot.1) + shape_radius;
        if !radius.is_finite() {
            return None;
        }
        maximum = maximum.max(radius);
    }
    Some(maximum)
}

/// Build one conservative candidate superset per target body. The target's
/// complete body bounds are used rather than individual edges, so every edge
/// and curved boundary on that target shares the same inventory.
pub(crate) fn build_visibility_pose_cell_candidate_inventory(
    source: (f64, f64),
    bodies: &[crate::organism_geometry::OrganismBodyGeometry],
    target_body_indices: &[usize],
    source_translation_bound: f64,
    body_motion: &[VisibilityBodyMotionBound],
) -> Option<VisibilityPoseCellCandidateInventory> {
    if !source.0.is_finite()
        || !source.1.is_finite()
        || !source_translation_bound.is_finite()
        || source_translation_bound < 0.0
        || body_motion.len() != bodies.len()
    {
        return None;
    }
    for (body, motion) in bodies.iter().zip(body_motion) {
        let required_radius = maximum_geometry_radius_about_pivot(body, motion.rotation_pivot)?;
        if motion.maximum_point_displacement()?.is_finite()
            && motion.radius_bound + 1.0e-10 * motion.radius_bound.max(1.0) < required_radius
        {
            return None;
        }
    }
    let reference_angle = reference_angle_for_bodies(source, bodies)?;
    let body_bounds = bodies
        .iter()
        .map(|body| (body.min_x, body.max_x, body.min_y, body.max_y))
        .collect::<Vec<_>>();
    let mut reference_part_poses = Vec::with_capacity(bodies.len());
    for body in bodies {
        let mut units = Vec::with_capacity(body.parts.len());
        let mut reference_parts = Vec::with_capacity(body.parts.len());
        for part in &body.parts {
            if !part.x.is_finite()
                || !part.y.is_finite()
                || !part.rotation_radians.is_finite()
                || units.contains(&part.unit_index)
            {
                return None;
            }
            units.push(part.unit_index);
            reference_parts.push((
                part.unit_index,
                part.form.clone(),
                part.x,
                part.y,
                part.rotation_radians,
            ));
        }
        reference_part_poses.push(reference_parts);
    }
    let mut target_candidates = vec![None; bodies.len()];
    for &target_body_index in target_body_indices {
        if target_candidates.get(target_body_index)?.is_some() {
            continue;
        }
        let mut candidates = candidate_body_indices_for_pose_cell(
            source,
            source_translation_bound,
            *body_bounds.get(target_body_index)?,
            *body_motion.get(target_body_index)?,
            reference_angle,
            &body_bounds,
            body_motion,
        )?;
        // Include the target itself so the inventory identity is independent
        // of broad-phase tie handling; exact occlusion skips its own features.
        if !candidates.contains(&target_body_index) {
            candidates.push(target_body_index);
            candidates.sort_unstable();
        }
        target_candidates[target_body_index] = Some(candidates);
    }
    Some(VisibilityPoseCellCandidateInventory {
        source_reference: source,
        source_translation_bound,
        body_motion: body_motion.to_vec(),
        reference_part_poses,
        target_candidates,
    })
}

fn expand_visibility_bounds(
    bounds: (f64, f64, f64, f64),
    expansion: f64,
) -> Option<(f64, f64, f64, f64)> {
    let (min_x, max_x, min_y, max_y) = bounds;
    if ![min_x, max_x, min_y, max_y, expansion]
        .iter()
        .all(|value| value.is_finite())
        || min_x > max_x
        || min_y > max_y
        || expansion < 0.0
    {
        return None;
    }
    let expanded = (
        min_x - expansion,
        max_x + expansion,
        min_y - expansion,
        max_y + expansion,
    );
    [expanded.0, expanded.1, expanded.2, expanded.3]
        .iter()
        .all(|value| value.is_finite())
        .then_some(expanded)
}

/// Return every body whose conservative swept AABB may occlude a target edge
/// anywhere in the supplied rigid-pose cell. Source translation is included
/// in every relative bound; target and blocker translation/rotation are
/// independently included. Missing or invalid motion envelopes fail closed.
#[allow(dead_code)]
pub(crate) fn candidate_body_indices_for_pose_cell(
    source: (f64, f64),
    source_translation_bound: f64,
    target_bounds: (f64, f64, f64, f64),
    target_motion: VisibilityBodyMotionBound,
    reference_angle: f64,
    body_bounds: &[(f64, f64, f64, f64)],
    body_motion: &[VisibilityBodyMotionBound],
) -> Option<Vec<usize>> {
    if !source.0.is_finite()
        || !source.1.is_finite()
        || !source_translation_bound.is_finite()
        || source_translation_bound < 0.0
        || body_bounds.len() != body_motion.len()
    {
        return None;
    }
    let target_expansion = source_translation_bound + target_motion.maximum_point_displacement()?;
    let swept_target_bounds = expand_visibility_bounds(target_bounds, target_expansion)?;
    let mut swept_body_intervals = Vec::with_capacity(body_bounds.len());
    let mut swept_body_distances = Vec::with_capacity(body_bounds.len());
    for (bounds, motion) in body_bounds.iter().copied().zip(body_motion.iter().copied()) {
        let expansion = source_translation_bound + motion.maximum_point_displacement()?;
        let swept_bounds = expand_visibility_bounds(bounds, expansion)?;
        swept_body_intervals.push(angle_interval_for_bounds(
            source,
            swept_bounds,
            reference_angle,
        )?);
        let dx = if source.0 < swept_bounds.0 {
            swept_bounds.0 - source.0
        } else if source.0 > swept_bounds.1 {
            source.0 - swept_bounds.1
        } else {
            0.0
        };
        let dy = if source.1 < swept_bounds.2 {
            swept_bounds.2 - source.1
        } else if source.1 > swept_bounds.3 {
            source.1 - swept_bounds.3
        } else {
            0.0
        };
        let distance = dx.hypot(dy);
        if !distance.is_finite() {
            return None;
        }
        swept_body_distances.push(distance);
    }
    candidate_body_indices_for_bounds(
        source,
        swept_target_bounds,
        reference_angle,
        &swept_body_intervals,
        &swept_body_distances,
    )
}

fn add_circle_tangent_points_tagged(
    center: (f64, f64),
    radius: f64,
    source: (f64, f64),
    body_index: usize,
    unit_index: usize,
    critical_points: &mut Vec<(usize, usize, f64, f64)>,
) {
    let mut points = Vec::with_capacity(2);
    add_circle_tangent_points(center, radius, source, &mut points);
    critical_points.extend(
        points
            .into_iter()
            .map(|(x, y)| (body_index, unit_index, x, y)),
    );
}

type IndexedCriticalPointsByBody = Vec<Vec<(usize, usize, f64, f64)>>;
type StraightEdgeFeature = (usize, usize, (f64, f64), (f64, f64));
type IndexedStraightEdgesByBody = Vec<Vec<StraightEdgeFeature>>;
type IndexedCircleArcsByBody = Vec<Vec<(usize, usize, WorldCircleArc)>>;

#[derive(Clone, Debug, PartialEq)]
struct VisibilityBlockerBodyInventory {
    body_index: usize,
    critical_features: Vec<(usize, usize, f64, f64)>,
    straight_edges: Vec<StraightEdgeFeature>,
    circular_arcs: Vec<(usize, usize, WorldCircleArc)>,
}

#[derive(Clone, Debug, PartialEq)]
struct TargetEdgeBlockerInventory {
    candidate_bodies: Vec<VisibilityBlockerBodyInventory>,
}

fn build_target_edge_blocker_inventory(
    candidate_body_indices: &[usize],
    critical_points_by_body: &IndexedCriticalPointsByBody,
    straight_edges_by_body: &IndexedStraightEdgesByBody,
    circle_arcs_by_body: &IndexedCircleArcsByBody,
) -> Option<TargetEdgeBlockerInventory> {
    let mut candidate_bodies = Vec::with_capacity(candidate_body_indices.len());
    let mut previous_body = None;
    for &body_index in candidate_body_indices {
        if previous_body.is_some_and(|previous| previous >= body_index) {
            return None;
        }
        previous_body = Some(body_index);
        let critical_features = critical_points_by_body.get(body_index)?.clone();
        let straight_edges = straight_edges_by_body.get(body_index)?.clone();
        let circular_arcs = circle_arcs_by_body.get(body_index)?.clone();
        let mut feature_ids = Vec::with_capacity(critical_features.len());
        for &(unit_index, feature_index, x, y) in &critical_features {
            if !x.is_finite()
                || !y.is_finite()
                || feature_ids.contains(&(unit_index, feature_index))
            {
                return None;
            }
            feature_ids.push((unit_index, feature_index));
        }
        let mut edge_ids = Vec::with_capacity(straight_edges.len());
        for &(unit_index, edge_index, start, end) in &straight_edges {
            if !start.0.is_finite()
                || !start.1.is_finite()
                || !end.0.is_finite()
                || !end.1.is_finite()
                || edge_ids.contains(&(unit_index, edge_index))
            {
                return None;
            }
            edge_ids.push((unit_index, edge_index));
        }
        let mut arc_ids = Vec::with_capacity(circular_arcs.len());
        for &(unit_index, arc_index, arc) in &circular_arcs {
            if arc.body_index != body_index
                || arc.unit_index != unit_index
                || !arc.center.0.is_finite()
                || !arc.center.1.is_finite()
                || !arc.radius.is_finite()
                || arc.radius <= 0.0
                || !arc.start.is_finite()
                || !arc.sweep.is_finite()
                || arc.sweep <= 0.0
                || arc.sweep > std::f64::consts::TAU
                || arc_ids.contains(&(unit_index, arc_index))
            {
                return None;
            }
            arc_ids.push((unit_index, arc_index));
        }
        candidate_bodies.push(VisibilityBlockerBodyInventory {
            body_index,
            critical_features,
            straight_edges,
            circular_arcs,
        });
    }
    Some(TargetEdgeBlockerInventory { candidate_bodies })
}

fn index_critical_points_by_body(
    body_count: usize,
    critical_points: Vec<(usize, usize, f64, f64)>,
) -> Option<IndexedCriticalPointsByBody> {
    let mut feature_counts: Vec<Vec<(usize, usize)>> =
        (0..body_count).map(|_| Vec::new()).collect();
    let mut indexed: Vec<Vec<(usize, usize, f64, f64)>> =
        (0..body_count).map(|_| Vec::new()).collect();
    for (body_index, unit_index, x, y) in critical_points {
        let unit_counts = feature_counts.get_mut(body_index)?;
        let feature_count = if let Some((_, count)) = unit_counts
            .iter_mut()
            .find(|(existing_unit, _)| *existing_unit == unit_index)
        {
            count
        } else {
            unit_counts.push((unit_index, 0));
            &mut unit_counts.last_mut()?.1
        };
        let feature_index = *feature_count;
        *feature_count = feature_count.checked_add(1)?;
        indexed[body_index].push((unit_index, feature_index, x, y));
    }
    Some(indexed)
}

pub(crate) fn visible_polygonal_intervals(
    source: (f64, f64),
    bodies: &[crate::organism_geometry::OrganismBodyGeometry],
) -> Option<Vec<ExposedBoundaryInterval>> {
    let targets: Vec<_> = (0..bodies.len()).collect();
    visible_polygonal_intervals_for_targets(source, bodies, &targets)
}

/// Compute visible intervals only for selected receiver bodies while retaining
/// every body as a potential occluder. This avoids solving exposure for thousands
/// of environmental particles when only organism energy holders receive input.
fn visible_polygonal_intervals_for_targets_with_cut_provenance_and_motion(
    source: (f64, f64),
    bodies: &[crate::organism_geometry::OrganismBodyGeometry],
    target_body_indices: &[usize],
    motion_envelope: Option<&VisibilityPoseCellMotionEnvelope<'_>>,
    candidate_inventory: Option<&VisibilityPoseCellCandidateInventory>,
) -> Option<Vec<ExposedBoundaryIntervalWithProvenance>> {
    use crate::capillary_geometry::FluidBoundaryPiece;
    use crate::resources::Form;

    if !source.0.is_finite() || !source.1.is_finite() {
        return None;
    }
    if let Some(inventory) = candidate_inventory {
        inventory.validate_pose(source, bodies, target_body_indices)?;
        let envelope = motion_envelope?;
        if envelope.source_translation_bound != inventory.source_translation_bound
            || envelope.body_motion != inventory.body_motion.as_slice()
        {
            return None;
        }
    }
    if let Some(envelope) = motion_envelope {
        if !envelope.source_translation_bound.is_finite()
            || envelope.source_translation_bound < 0.0
            || envelope.body_motion.len() != bodies.len()
        {
            return None;
        }
        for motion in envelope.body_motion {
            motion.maximum_point_displacement()?;
        }
    }
    let mut target_body_mask = vec![false; bodies.len()];
    for &index in target_body_indices {
        *target_body_mask.get_mut(index)? = true;
    }
    let reference_angle = reference_angle_for_bodies(source, bodies)?;
    let body_bounds = motion_envelope.map(|_| {
        bodies
            .iter()
            .map(|body| (body.min_x, body.max_x, body.min_y, body.max_y))
            .collect::<Vec<_>>()
    });
    if motion_envelope.is_some_and(|envelope| envelope.body_motion.len() != bodies.len()) {
        return None;
    }
    let (body_intervals, body_min_distances) = if motion_envelope.is_none() {
        (
            body_angular_intervals(source, bodies, reference_angle)?,
            body_minimum_distances(source, bodies)?,
        )
    } else {
        (Vec::new(), Vec::new())
    };

    let mut straight_targets = Vec::new();
    let mut straight_edges = Vec::new();
    let mut critical_points: Vec<(usize, usize, f64, f64)> = Vec::new();

    for (body_index, body) in bodies.iter().enumerate() {
        let mut seen_unit_indices = Vec::with_capacity(body.parts.len());
        for part in &body.parts {
            if seen_unit_indices.contains(&part.unit_index)
                || !part.x.is_finite()
                || !part.y.is_finite()
                || !part.rotation_radians.is_finite()
            {
                return None;
            }
            seen_unit_indices.push(part.unit_index);

            let straight_local = match &part.form {
                Form::Fluid {
                    nominal_area,
                    boundary: Some(vertices),
                    arc_boundary: None,
                    multi_contact_boundary: None,
                } => {
                    if !nominal_area.is_finite() || *nominal_area <= 0.0 || vertices.len() < 3 {
                        return None;
                    }
                    Some(vertices.clone())
                }
                Form::Fluid {
                    nominal_area,
                    boundary: None,
                    arc_boundary: Some(arc),
                    multi_contact_boundary: None,
                } => {
                    if !nominal_area.is_finite()
                        || *nominal_area <= 0.0
                        || !arc.is_valid()
                        || (arc.segment_area()? - *nominal_area).abs()
                            > 1e-10 * nominal_area.abs().max(1.0)
                    {
                        return None;
                    }
                    None
                }
                Form::Fluid {
                    nominal_area,
                    boundary: None,
                    arc_boundary: None,
                    multi_contact_boundary: Some(multi),
                } => {
                    if !nominal_area.is_finite()
                        || *nominal_area <= 0.0
                        || !multi.is_valid()
                        || (multi.nominal_area - *nominal_area).abs()
                            > 1e-10 * nominal_area.abs().max(1.0)
                    {
                        return None;
                    }
                    None
                }
                Form::Fluid {
                    nominal_area,
                    boundary: None,
                    arc_boundary: None,
                    multi_contact_boundary: None,
                } => {
                    if !nominal_area.is_finite() || *nominal_area <= 0.0 {
                        return None;
                    }
                    None
                }
                Form::Fluid { .. } => return None,
                Form::Circle { radius } => {
                    if !radius.is_finite() || *radius <= 0.0 {
                        return None;
                    }
                    None
                }
                Form::Line { length } => {
                    if !length.is_finite() || *length <= 0.0 {
                        return None;
                    }
                    None
                }
                _ => part.form.polygon_vertices(),
            };

            if let Some(local_vertices) = straight_local {
                if local_vertices.len() < 3
                    || local_vertices
                        .iter()
                        .any(|(x, y)| !x.is_finite() || !y.is_finite())
                {
                    return None;
                }
                let (sin, cos) = part.rotation_radians.sin_cos();
                let vertices = local_vertices
                    .into_iter()
                    .map(|(x, y)| (part.x + x * cos - y * sin, part.y + x * sin + y * cos))
                    .collect::<Vec<_>>();
                if vertices
                    .iter()
                    .any(|(x, y)| !x.is_finite() || !y.is_finite())
                {
                    return None;
                }
                critical_points.extend(
                    vertices
                        .iter()
                        .copied()
                        .map(|(x, y)| (body_index, part.unit_index, x, y)),
                );
                for index in 0..vertices.len() {
                    straight_edges.push((
                        body_index,
                        part.unit_index,
                        index,
                        vertices[index],
                        vertices[(index + 1) % vertices.len()],
                    ));
                }
                if target_body_mask[body_index] {
                    straight_targets.push((body_index, part.unit_index, vertices));
                }
                continue;
            }

            match &part.form {
                Form::Line { length } => {
                    if !length.is_finite() || *length <= 0.0 {
                        return None;
                    }
                    let (sin, cos) = part.rotation_radians.sin_cos();
                    let half = *length * 0.5;
                    let start = (part.x - half * cos, part.y - half * sin);
                    let end = (part.x + half * cos, part.y + half * sin);
                    critical_points.extend([
                        (body_index, part.unit_index, start.0, start.1),
                        (body_index, part.unit_index, end.0, end.1),
                    ]);
                    straight_edges.push((body_index, part.unit_index, 0, start, end));
                }
                Form::Circle { radius } => {
                    add_circle_tangent_points_tagged(
                        (part.x, part.y),
                        *radius,
                        source,
                        body_index,
                        part.unit_index,
                        &mut critical_points,
                    );
                }
                Form::Fluid {
                    nominal_area,
                    boundary,
                    arc_boundary,
                    multi_contact_boundary,
                } => {
                    if let Some(vertices) = boundary {
                        let (sin, cos) = part.rotation_radians.sin_cos();
                        let world = vertices
                            .iter()
                            .map(|&(x, y)| (part.x + x * cos - y * sin, part.y + x * sin + y * cos))
                            .collect::<Vec<_>>();
                        if world.iter().any(|(x, y)| !x.is_finite() || !y.is_finite()) {
                            return None;
                        }
                        critical_points.extend(
                            world
                                .iter()
                                .copied()
                                .map(|(x, y)| (body_index, part.unit_index, x, y)),
                        );
                        for index in 0..world.len() {
                            straight_edges.push((
                                body_index,
                                part.unit_index,
                                index,
                                world[index],
                                world[(index + 1) % world.len()],
                            ));
                        }
                    }
                    if let Some(arc) = arc_boundary {
                        let world = arc.transformed(part.x, part.y, part.rotation_radians)?;
                        critical_points.extend(
                            [world.start_point(), world.end_point()]
                                .map(|(x, y)| (body_index, part.unit_index, x, y)),
                        );
                        add_circle_tangent_points_tagged(
                            (world.center_x, world.center_y),
                            world.radius,
                            source,
                            body_index,
                            part.unit_index,
                            &mut critical_points,
                        );
                        straight_edges.push((
                            body_index,
                            part.unit_index,
                            0,
                            world.start_point(),
                            world.end_point(),
                        ));
                    }
                    if let Some(multi) = multi_contact_boundary {
                        for (piece_index, piece) in multi.pieces.iter().enumerate() {
                            match piece {
                                FluidBoundaryPiece::SolidContact {
                                    start_x,
                                    start_y,
                                    end_x,
                                    end_y,
                                    ..
                                } => {
                                    let (sin, cos) = part.rotation_radians.sin_cos();
                                    let transform = |x: f64, y: f64| {
                                        (part.x + x * cos - y * sin, part.y + x * sin + y * cos)
                                    };
                                    let start = transform(*start_x, *start_y);
                                    let end = transform(*end_x, *end_y);
                                    critical_points.extend([
                                        (body_index, part.unit_index, start.0, start.1),
                                        (body_index, part.unit_index, end.0, end.1),
                                    ]);
                                    straight_edges.push((
                                        body_index,
                                        part.unit_index,
                                        piece_index,
                                        start,
                                        end,
                                    ));
                                }
                                FluidBoundaryPiece::FreeArc(arc) => {
                                    let world =
                                        arc.transformed(part.x, part.y, part.rotation_radians)?;
                                    critical_points.extend(
                                        [world.start_point(), world.end_point()]
                                            .map(|(x, y)| (body_index, part.unit_index, x, y)),
                                    );
                                    add_circle_tangent_points_tagged(
                                        (world.center_x, world.center_y),
                                        world.radius,
                                        source,
                                        body_index,
                                        part.unit_index,
                                        &mut critical_points,
                                    );
                                }
                            }
                        }
                    }
                    if boundary.is_none()
                        && arc_boundary.is_none()
                        && multi_contact_boundary.is_none()
                    {
                        let radius = (nominal_area / std::f64::consts::PI).sqrt();
                        add_circle_tangent_points_tagged(
                            (part.x, part.y),
                            radius,
                            source,
                            body_index,
                            part.unit_index,
                            &mut critical_points,
                        );
                    }
                }
                _ => {}
            }
        }
    }

    if critical_points
        .iter()
        .any(|(_, _, x, y)| !x.is_finite() || !y.is_finite())
        || straight_edges.iter().any(|(_, _, _, a, b)| {
            !a.0.is_finite() || !a.1.is_finite() || !b.0.is_finite() || !b.1.is_finite()
        })
    {
        return None;
    }

    let critical_points_by_body = index_critical_points_by_body(bodies.len(), critical_points)?;
    type StraightEdge = (usize, usize, (f64, f64), (f64, f64));
    let mut straight_edges_by_body: Vec<Vec<StraightEdge>> =
        (0..bodies.len()).map(|_| Vec::new()).collect();
    for (body_index, unit_index, edge_index, start, end) in straight_edges {
        straight_edges_by_body[body_index].push((unit_index, edge_index, start, end));
    }

    let mut circle_arcs_by_body: IndexedCircleArcsByBody =
        (0..bodies.len()).map(|_| Vec::new()).collect();
    for (body_index, body) in bodies.iter().enumerate() {
        for part in &body.parts {
            for (arc_index, arc) in realized_circle_arcs(body_index, part)?
                .into_iter()
                .enumerate()
            {
                circle_arcs_by_body[body_index].push((part.unit_index, arc_index, arc));
            }
        }
    }

    let mut result = Vec::new();
    for (body_index, unit_index, vertices) in straight_targets {
        if !polygon_is_convex(&vertices) {
            return None;
        }
        let area2: f64 = vertices
            .iter()
            .enumerate()
            .map(|(index, &(x1, y1))| {
                let (x2, y2) = vertices[(index + 1) % vertices.len()];
                x1 * y2 - x2 * y1
            })
            .sum();
        if !area2.is_finite() || area2.abs() <= 1e-12 {
            return None;
        }
        let winding = area2.signum();

        for edge_index in 0..vertices.len() {
            let a = vertices[edge_index];
            let b = vertices[(edge_index + 1) % vertices.len()];
            let ex = b.0 - a.0;
            let ey = b.1 - a.1;
            let edge_length = ex.hypot(ey);
            if !edge_length.is_finite() || edge_length <= 1e-12 {
                continue;
            }
            let normal = (winding * ey / edge_length, -winding * ex / edge_length);
            let midpoint = ((a.0 + b.0) * 0.5, (a.1 + b.1) * 0.5);
            if normal.0 * (source.0 - midpoint.0) + normal.1 * (source.1 - midpoint.1) <= 1e-10 {
                continue;
            }

            let target_edge_bounds = (a.0.min(b.0), a.0.max(b.0), a.1.min(b.1), a.1.max(b.1));
            let candidate_body_indices = if let Some(inventory) = candidate_inventory {
                inventory.candidates_for_target(body_index)?.to_vec()
            } else if let Some(envelope) = motion_envelope {
                candidate_body_indices_for_pose_cell(
                    source,
                    envelope.source_translation_bound,
                    target_edge_bounds,
                    *envelope.body_motion.get(body_index)?,
                    reference_angle,
                    body_bounds.as_deref()?,
                    envelope.body_motion,
                )?
            } else {
                candidate_body_indices_for_bounds(
                    source,
                    target_edge_bounds,
                    reference_angle,
                    &body_intervals,
                    &body_min_distances,
                )?
            };
            let blocker_inventory = build_target_edge_blocker_inventory(
                &candidate_body_indices,
                &critical_points_by_body,
                &straight_edges_by_body,
                &circle_arcs_by_body,
            )?;
            let target_part = bodies[body_index]
                .parts
                .iter()
                .find(|part| part.unit_index == unit_index)?;
            let (target_sin, target_cos) = target_part.rotation_radians.sin_cos();
            let local_target_point = |point: (f64, f64)| {
                let dx = point.0 - target_part.x;
                let dy = point.1 - target_part.y;
                (
                    dx * target_cos + dy * target_sin,
                    -dx * target_sin + dy * target_cos,
                )
            };
            let mut cuts = vec![
                VisibilityCutRecord::new(
                    0.0,
                    VisibilityCutEvent::TargetVertex {
                        body_index,
                        unit_index,
                        vertex_index: edge_index,
                        local_point: local_target_point(a),
                    },
                ),
                VisibilityCutRecord::new(
                    1.0,
                    VisibilityCutEvent::TargetVertex {
                        body_index,
                        unit_index,
                        vertex_index: (edge_index + 1) % vertices.len(),
                        local_point: local_target_point(b),
                    },
                ),
            ];
            for blocker_body in &blocker_inventory.candidate_bodies {
                let candidate_body = blocker_body.body_index;
                for &(blocker_unit_index, blocker_feature_index, vx, vy) in
                    &blocker_body.critical_features
                {
                    let dx = vx - source.0;
                    let dy = vy - source.1;
                    let denominator = ex * dy - ey * dx;
                    if denominator.abs() <= 1e-12 {
                        continue;
                    }
                    let ax = a.0 - source.0;
                    let ay = a.1 - source.1;
                    let t = -(ax * dy - ay * dx) / denominator;
                    let u = -(ax * ey - ay * ex) / denominator;
                    if t > 1e-10 && t < 1.0 - 1e-10 && u > 1.0 + 1e-10 {
                        let blocker_part = bodies[candidate_body]
                            .parts
                            .iter()
                            .find(|part| part.unit_index == blocker_unit_index)?;
                        let (sin, cos) = blocker_part.rotation_radians.sin_cos();
                        let offset = (vx - blocker_part.x, vy - blocker_part.y);
                        let local_point = (
                            offset.0 * cos + offset.1 * sin,
                            -offset.0 * sin + offset.1 * cos,
                        );
                        let event = match &blocker_part.form {
                            crate::resources::Form::Circle { radius } => {
                                let source_offset =
                                    (source.0 - blocker_part.x, source.1 - blocker_part.y);
                                let local_source = (
                                    source_offset.0 * cos + source_offset.1 * sin,
                                    -source_offset.0 * sin + source_offset.1 * cos,
                                );
                                let cross =
                                    local_source.0 * local_point.1 - local_source.1 * local_point.0;
                                VisibilityCutEvent::RayThroughCircleTangent {
                                    blocker_body_index: candidate_body,
                                    blocker_unit_index,
                                    target_body_index: body_index,
                                    target_unit_index: unit_index,
                                    target_edge_index: edge_index,
                                    tangent_side: if cross < 0.0 { -1 } else { 1 },
                                    local_center: (0.0, 0.0),
                                    radius: *radius,
                                }
                            }
                            _ => VisibilityCutEvent::RayThroughRigidFeature {
                                blocker_body_index: candidate_body,
                                blocker_unit_index,
                                blocker_feature_index,
                                target_body_index: body_index,
                                target_unit_index: unit_index,
                                target_edge_index: edge_index,
                                local_point,
                            },
                        };
                        cuts.push(VisibilityCutRecord::new(t, event));
                    }
                }
            }

            // Directly split at straight-boundary intersections, including
            // contact chords. Collinear overlaps are split at both endpoints.
            for blocker_body in &blocker_inventory.candidate_bodies {
                let block_body = blocker_body.body_index;
                for &(block_unit, block_edge_index, c, d) in &blocker_body.straight_edges {
                    if block_body == body_index && block_unit == unit_index {
                        continue;
                    }
                    let sx = d.0 - c.0;
                    let sy = d.1 - c.1;
                    let qx = c.0 - a.0;
                    let qy = c.1 - a.1;
                    let denominator = ex * sy - ey * sx;
                    if denominator.abs() <= 1e-12 {
                        if (qx * ey - qy * ex).abs() <= 1e-9 {
                            let length_squared = ex * ex + ey * ey;
                            for (point, blocker_parameter) in [(c, 0.0), (d, 1.0)] {
                                let t =
                                    ((point.0 - a.0) * ex + (point.1 - a.1) * ey) / length_squared;
                                if t > 1e-10 && t < 1.0 - 1e-10 {
                                    cuts.push(VisibilityCutRecord::new(
                                        t,
                                        VisibilityCutEvent::StraightBoundaryIntersection {
                                            blocker_body_index: block_body,
                                            blocker_unit_index: block_unit,
                                            target_body_index: body_index,
                                            target_unit_index: unit_index,
                                            target_edge_index: edge_index,
                                            blocker_edge_index: block_edge_index,
                                            target_edge_start_local: localize_boundary_point(
                                                bodies, body_index, unit_index, a,
                                            )?,
                                            target_edge_end_local: localize_boundary_point(
                                                bodies, body_index, unit_index, b,
                                            )?,
                                            blocker_edge_start_local: localize_boundary_point(
                                                bodies, block_body, block_unit, c,
                                            )?,
                                            blocker_edge_end_local: localize_boundary_point(
                                                bodies, block_body, block_unit, d,
                                            )?,
                                            target_parameter: t,
                                            blocker_parameter,
                                        },
                                    ));
                                }
                            }
                        }
                        continue;
                    }
                    let t = (qx * sy - qy * sx) / denominator;
                    let u = (qx * ey - qy * ex) / denominator;
                    if t > 1e-10 && t < 1.0 - 1e-10 && (-1e-10..=1.0 + 1e-10).contains(&u) {
                        cuts.push(VisibilityCutRecord::new(
                            t,
                            VisibilityCutEvent::StraightBoundaryIntersection {
                                blocker_body_index: block_body,
                                blocker_unit_index: block_unit,
                                target_body_index: body_index,
                                target_unit_index: unit_index,
                                target_edge_index: edge_index,
                                blocker_edge_index: block_edge_index,
                                target_edge_start_local: localize_boundary_point(
                                    bodies, body_index, unit_index, a,
                                )?,
                                target_edge_end_local: localize_boundary_point(
                                    bodies, body_index, unit_index, b,
                                )?,
                                blocker_edge_start_local: localize_boundary_point(
                                    bodies, block_body, block_unit, c,
                                )?,
                                blocker_edge_end_local: localize_boundary_point(
                                    bodies, block_body, block_unit, d,
                                )?,
                                target_parameter: t,
                                blocker_parameter: u,
                            },
                        ));
                    }
                }
            }

            // Curved-boundary intersections retain the blocker constituent and
            // arc parameter. The analytic arc Jacobian is still a separate
            // certificate obligation, so these events remain unresolved later.
            for blocker_body in &blocker_inventory.candidate_bodies {
                for &(blocker_unit_index, arc_index, arc) in &blocker_body.circular_arcs {
                    if blocker_body.body_index == body_index && blocker_unit_index == unit_index {
                        continue;
                    }
                    let mut intersections = Vec::with_capacity(2);
                    add_circle_segment_intersections(
                        arc.center,
                        arc.radius,
                        a,
                        b,
                        &mut intersections,
                    )?;
                    for point in intersections {
                        let angle = (point.1 - arc.center.1).atan2(point.0 - arc.center.0);
                        if !arc_has_angle(arc, angle) {
                            continue;
                        }
                        let t = ((point.0 - a.0) * ex + (point.1 - a.1) * ey)
                            / (edge_length * edge_length);
                        if t > 1e-10 && t < 1.0 - 1e-10 {
                            let blocker_part = bodies[blocker_body.body_index]
                                .parts
                                .iter()
                                .find(|part| part.unit_index == blocker_unit_index)?;
                            cuts.push(VisibilityCutRecord::new(
                                t,
                                VisibilityCutEvent::CircleStraightIntersection {
                                    blocker_body_index: blocker_body.body_index,
                                    blocker_unit_index,
                                    target_body_index: body_index,
                                    target_unit_index: unit_index,
                                    target_edge_index: edge_index,
                                    arc_index,
                                    target_edge_start_local: localize_boundary_point(
                                        bodies, body_index, unit_index, a,
                                    )?,
                                    target_edge_end_local: localize_boundary_point(
                                        bodies, body_index, unit_index, b,
                                    )?,
                                    blocker_arc_center_local: arc.center_local,
                                    blocker_arc_radius: arc.radius,
                                    blocker_arc_start_local: arc.start_local,
                                    blocker_arc_sweep: arc.sweep,
                                    target_parameter: t,
                                    arc_angle_local: (angle - blocker_part.rotation_radians)
                                        .rem_euclid(std::f64::consts::TAU),
                                },
                            ));
                        }
                    }
                }
            }

            let cuts = merge_visibility_cuts(cuts);
            let mut interval_blocker_bodies = Vec::with_capacity(cuts.len().saturating_sub(1));
            for pair in cuts.windows(2) {
                let (t0, t1) = (pair[0].parameter, pair[1].parameter);
                if t1 - t0 <= 1e-10 {
                    return None;
                }
                let t = (t0 + t1) * 0.5;
                let point = (a.0 + ex * t, a.1 + ey * t);
                interval_blocker_bodies.push(
                    crate::physical_body_geometry::source_path_blocker_unit_identities_for_candidates(
                        source,
                        point,
                        bodies,
                        body_index,
                        unit_index,
                        &candidate_body_indices,
                    )?,
                );
            }
            let visibility_signature = VisibilityCutSignature::from_partition(
                &cuts,
                &candidate_body_indices,
                &interval_blocker_bodies,
            );
            for (pair, blockers) in cuts.windows(2).zip(interval_blocker_bodies.iter()) {
                let blocked = !blockers.is_empty();
                let (t0, t1) = (pair[0].parameter, pair[1].parameter);
                if t1 - t0 <= 1e-10 || blocked {
                    continue;
                }
                let interval = ExposedBoundaryInterval {
                    body_index,
                    unit_index,
                    start: (a.0 + ex * t0, a.1 + ey * t0),
                    end: (a.0 + ex * t1, a.1 + ey * t1),
                    outward_normal: normal,
                };
                result.push(ExposedBoundaryIntervalWithProvenance {
                    interval,
                    start_provenance: endpoint_provenance_from_cut(
                        &pair[0], body_index, unit_index,
                    ),
                    end_provenance: endpoint_provenance_from_cut(&pair[1], body_index, unit_index),
                    visibility_signature: visibility_signature.clone(),
                });
            }
        }
    }
    Some(result)
}

/// Exact current-pose visibility with stable event provenance.
pub(crate) fn visible_polygonal_intervals_for_targets_with_cut_provenance(
    source: (f64, f64),
    bodies: &[crate::organism_geometry::OrganismBodyGeometry],
    target_body_indices: &[usize],
) -> Option<Vec<ExposedBoundaryIntervalWithProvenance>> {
    visible_polygonal_intervals_for_targets_with_cut_provenance_and_motion(
        source,
        bodies,
        target_body_indices,
        None,
        None,
    )
}

/// Visibility evaluated with a conservative swept candidate inventory for a
/// complete proposed pose cell. Call this at each certified pose sample using
/// envelopes that cover the entire same cell; compare returned signatures and
/// fall back/subdivide on any change.
pub(crate) fn visible_polygonal_intervals_for_targets_with_pose_cell_provenance(
    source: (f64, f64),
    bodies: &[crate::organism_geometry::OrganismBodyGeometry],
    target_body_indices: &[usize],
    source_translation_bound: f64,
    body_motion: &[VisibilityBodyMotionBound],
) -> Option<Vec<ExposedBoundaryIntervalWithProvenance>> {
    let envelope = VisibilityPoseCellMotionEnvelope {
        source_translation_bound,
        body_motion,
    };
    let inventory = build_visibility_pose_cell_candidate_inventory(
        source,
        bodies,
        target_body_indices,
        source_translation_bound,
        body_motion,
    )?;
    visible_polygonal_intervals_for_targets_with_cut_provenance_and_motion(
        source,
        bodies,
        target_body_indices,
        Some(&envelope),
        Some(&inventory),
    )
}

/// Re-evaluate exact visibility at another pose sample using the same
/// conservative candidate superset built at the reference pose. The caller
/// must ensure every sampled pose remains inside that inventory's certified
/// motion envelope and preserves body/unit identity.
pub(crate) fn visible_polygonal_intervals_for_targets_with_pose_cell_inventory(
    source: (f64, f64),
    bodies: &[crate::organism_geometry::OrganismBodyGeometry],
    target_body_indices: &[usize],
    source_translation_bound: f64,
    body_motion: &[VisibilityBodyMotionBound],
    inventory: &VisibilityPoseCellCandidateInventory,
) -> Option<Vec<ExposedBoundaryIntervalWithProvenance>> {
    let envelope = VisibilityPoseCellMotionEnvelope {
        source_translation_bound,
        body_motion,
    };
    visible_polygonal_intervals_for_targets_with_cut_provenance_and_motion(
        source,
        bodies,
        target_body_indices,
        Some(&envelope),
        Some(inventory),
    )
}

/// Preserve the established interval-only API for existing geometry consumers.
pub(crate) fn visible_polygonal_intervals_for_targets(
    source: (f64, f64),
    bodies: &[crate::organism_geometry::OrganismBodyGeometry],
    target_body_indices: &[usize],
) -> Option<Vec<ExposedBoundaryInterval>> {
    Some(
        visible_polygonal_intervals_for_targets_with_cut_provenance(
            source,
            bodies,
            target_body_indices,
        )?
        .into_iter()
        .map(|entry| entry.interval)
        .collect(),
    )
}

/// Return exact intervals with endpoint provenance carried from the cut events
/// that created each interval. No coordinate-based provenance reconstruction
/// is performed; unsupported visibility cuts remain unresolved.
#[allow(dead_code)]
pub(crate) fn visible_polygonal_intervals_for_targets_with_provenance(
    source: (f64, f64),
    bodies: &[crate::organism_geometry::OrganismBodyGeometry],
    target_body_indices: &[usize],
) -> Option<Vec<ExposedBoundaryIntervalWithProvenance>> {
    visible_polygonal_intervals_for_targets_with_cut_provenance(source, bodies, target_body_indices)
}

#[cfg(test)]
mod visibility_tests {
    use super::*;
    use crate::organism_geometry::{OrganismBodyGeometry, PlacedForm};
    use crate::resources::Form;

    fn rectangle_body(
        unit_index: usize,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> OrganismBodyGeometry {
        OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index,
                form: Form::Rectangle { width, height },
                x,
                y,
                rotation_radians: 0.0,
            }],
            min_x: x - width * 0.5,
            max_x: x + width * 0.5,
            min_y: y - height * 0.5,
            max_y: y + height * 0.5,
        }
    }

    #[test]
    fn visibility_provenance_identifies_unoccluded_rigid_target_vertices() {
        let source = (-5.0, 0.0);
        let target = rectangle_body(0, 5.0, 0.0, 2.0, 2.0);
        let intervals =
            visible_polygonal_intervals_for_targets_with_provenance(source, &[target], &[0])
                .expect("valid target should produce provenance-aware intervals");
        assert!(!intervals.is_empty());
        for interval in intervals {
            assert!(matches!(
                &interval.start_provenance,
                BoundaryEndpointProvenance::RigidTargetVertex {
                    body_index: 0,
                    unit_index: 0,
                    ..
                }
            ));
            assert!(matches!(
                &interval.end_provenance,
                BoundaryEndpointProvenance::RigidTargetVertex {
                    body_index: 0,
                    unit_index: 0,
                    ..
                }
            ));
        }
    }

    #[test]
    fn visibility_provenance_fails_closed_for_occlusion_generated_cuts() {
        let source = (-5.0, 0.0);
        let target = rectangle_body(0, 5.0, 0.0, 2.0, 2.0);
        let blocker = rectangle_body(1, 2.0, 0.0, 1.0, 1.0);
        let intervals = visible_polygonal_intervals_for_targets_with_provenance(
            source,
            &[target, blocker],
            &[0],
        )
        .expect("valid target and blocker should produce exact intervals");
        assert!(!intervals.is_empty());
        assert!(
            intervals.iter().any(|interval| {
                matches!(
                    &interval.start_provenance,
                    BoundaryEndpointProvenance::UnresolvedVisibilityCut { .. }
                ) || matches!(
                    &interval.end_provenance,
                    BoundaryEndpointProvenance::UnresolvedVisibilityCut { .. }
                )
            }),
            "occluder-generated cuts must not be mislabeled as rigid target vertices"
        );
        assert!(
            intervals.iter().any(|interval| {
                matches!(
                    &interval.start_provenance,
                    BoundaryEndpointProvenance::RigidTargetVertex { .. }
                ) || matches!(
                    &interval.end_provenance,
                    BoundaryEndpointProvenance::RigidTargetVertex { .. }
                )
            }),
            "partial occlusion must preserve the unaffected target-vertex endpoint"
        );
    }

    #[test]
    fn near_silhouette_ray_cut_stays_unresolved_beside_target_vertex() {
        let source = (-5.0, 0.0);
        let target = rectangle_body(0, 5.0, 0.0, 2.0, 2.0);
        // Keep the blocker angularly narrower than the target-vertex ray.
        // Its upper corners then create a near-silhouette cut while the tiny
        // interval between that cut and the target vertex remains exposed.
        let blocker_right_x = 2.0 + 5.0e-6;
        let near_vertex_target_y = 1.0 - 6.16e-6;
        let blocker_top_y = near_vertex_target_y * (blocker_right_x + 5.0) / 9.0;
        let blocker = rectangle_body(1, 2.0, blocker_top_y - 0.25, 1.0e-5, 0.5);
        let intervals = visible_polygonal_intervals_for_targets_with_provenance(
            source,
            &[target, blocker],
            &[0],
        )
        .expect("near-silhouette geometry should retain exact visibility intervals");
        let near_silhouette_cut = intervals.iter().any(|interval| {
            [&interval.start_provenance, &interval.end_provenance]
                .into_iter()
                .zip([interval.interval.start, interval.interval.end])
                .any(|(provenance, point)| {
                    let near_target_vertex = (point.0 - 4.0).hypot(point.1 - 1.0) < 1.0e-5;
                    let has_blocker_ray = matches!(
                        provenance,
                        BoundaryEndpointProvenance::UnresolvedVisibilityCut { events, .. }
                            if events.iter().any(|event| matches!(
                                event,
                                VisibilityCutEvent::RayThroughRigidFeature {
                                    blocker_body_index: 1,
                                    blocker_unit_index: 1,
                                    target_body_index: 0,
                                    target_unit_index: 0,
                                    target_edge_index,
                                    blocker_feature_index,
                                    ..
                                } if *target_edge_index < 4 && *blocker_feature_index < 4
                            ))
                    );
                    near_target_vertex && has_blocker_ray
                })
        });
        assert!(
            near_silhouette_cut,
            "a ray cut close to a target silhouette must retain blocker provenance and remain unresolved"
        );
    }

    #[test]
    fn straight_intersection_provenance_retains_both_edge_identities() {
        let source = (-5.0, 0.0);
        let target = rectangle_body(0, 5.0, 0.0, 2.0, 2.0);
        let touching_blocker = rectangle_body(1, 4.5, 0.0, 1.0, 0.5);
        let intervals = visible_polygonal_intervals_for_targets_with_provenance(
            source,
            &[target, touching_blocker],
            &[0],
        )
        .expect("touching straight boundaries should retain exact intervals");
        let has_identified_intersection = |event: &VisibilityCutEvent| {
            matches!(
                event,
                VisibilityCutEvent::StraightBoundaryIntersection {
                    blocker_body_index: 1,
                    blocker_unit_index: 1,
                    target_body_index: 0,
                    target_unit_index: 0,
                    target_edge_index,
                    blocker_edge_index,
                    target_edge_start_local,
                    target_edge_end_local,
                    blocker_edge_start_local,
                    blocker_edge_end_local,
                    target_parameter,
                    blocker_parameter,
                } if *target_edge_index < 4
                    && *blocker_edge_index < 4
                    && [
                        target_edge_start_local.0,
                        target_edge_start_local.1,
                        target_edge_end_local.0,
                        target_edge_end_local.1,
                        blocker_edge_start_local.0,
                        blocker_edge_start_local.1,
                        blocker_edge_end_local.0,
                        blocker_edge_end_local.1,
                    ]
                    .iter()
                    .all(|value| value.is_finite())
                    && target_parameter.is_finite()
                    && blocker_parameter.is_finite()
            )
        };
        let found = intervals.iter().any(|interval| {
            [&interval.start_provenance, &interval.end_provenance]
                .into_iter()
                .any(|provenance| match provenance {
                    BoundaryEndpointProvenance::UnresolvedVisibilityCut { events, .. } => {
                        events.iter().any(&has_identified_intersection)
                    }
                    BoundaryEndpointProvenance::RigidVisibilityIntersection { event, .. } => {
                        has_identified_intersection(event)
                    }
                    BoundaryEndpointProvenance::RigidTargetVertex { .. } => false,
                })
        });
        assert!(
            found,
            "the cut must retain target and blocker edge identities and both local parameters"
        );
    }

    #[test]
    fn coincident_duplicate_blockers_keep_the_cut_provenance_unresolved() {
        let source = (-5.0, 0.0);
        let target = rectangle_body(0, 5.0, 0.0, 2.0, 2.0);
        let first_blocker = rectangle_body(1, 2.0, 0.0, 1.0, 0.5);
        let second_blocker = rectangle_body(2, 2.0, 0.0, 1.0, 0.5);
        let intervals = visible_polygonal_intervals_for_targets_with_provenance(
            source,
            &[target, first_blocker, second_blocker],
            &[0],
        )
        .expect("coincident blockers should retain valid exact visibility intervals");
        assert!(!intervals.is_empty());
        assert!(
            intervals.iter().any(|interval| {
                matches!(
                    &interval.start_provenance,
                    BoundaryEndpointProvenance::UnresolvedVisibilityCut { .. }
                ) || matches!(
                    &interval.end_provenance,
                    BoundaryEndpointProvenance::UnresolvedVisibilityCut { .. }
                )
            }),
            "coincident cuts from distinct blockers must not be assigned a unique derivative"
        );
    }

    #[test]
    fn candidate_filter_excludes_angularly_overlapping_bodies_behind_target() {
        let source = (-10.0, 0.0);
        let bodies = [
            rectangle_body(0, 0.0, 0.0, 1.0, 1.0),
            rectangle_body(1, -3.0, 0.0, 1.0, 1.0),
            rectangle_body(2, 5.0, 0.0, 1.0, 1.0),
        ];
        let reference_angle = reference_angle_for_bodies(source, &bodies).unwrap();
        let intervals = body_angular_intervals(source, &bodies, reference_angle).unwrap();
        let minimum_distances = body_minimum_distances(source, &bodies).unwrap();

        let candidates = candidate_body_indices_for_bounds(
            source,
            (-0.5, 0.5, -0.5, 0.5),
            reference_angle,
            &intervals,
            &minimum_distances,
        )
        .unwrap();

        assert!(candidates.contains(&0), "the target body remains eligible");
        assert!(
            candidates.contains(&1),
            "a body between the source and target remains a possible blocker"
        );
        assert!(
            !candidates.contains(&2),
            "a farther body cannot block a nearer target, even when angular bounds overlap"
        );
    }

    #[test]
    fn polygonal_exposure_rejects_duplicate_constituent_identity() {
        let body = OrganismBodyGeometry {
            parts: vec![
                PlacedForm {
                    unit_index: 4,
                    form: Form::Rectangle {
                        width: 2.0,
                        height: 2.0,
                    },
                    x: 5.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
                PlacedForm {
                    unit_index: 4,
                    form: Form::Rectangle {
                        width: 1.0,
                        height: 1.0,
                    },
                    x: 8.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            ],
            min_x: 4.0,
            max_x: 8.5,
            min_y: -1.0,
            max_y: 1.0,
        };
        assert!(
            visible_polygonal_intervals((-5.0, 0.0), &[body]).is_none(),
            "duplicate unit identities make occlusion exclusion and exposure attribution ambiguous"
        );
    }

    #[test]
    fn polygonal_exposure_fails_closed_for_concave_target_that_can_self_occlude() {
        let concave = OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index: 0,
                form: Form::Polygon {
                    vertices: vec![
                        (-2.0, -1.0),
                        (2.0, -1.0),
                        (2.0, 1.0),
                        (0.0, 0.0),
                        (-2.0, 1.0),
                    ],
                },
                x: 5.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            min_x: 3.0,
            max_x: 7.0,
            min_y: -1.0,
            max_y: 1.0,
        };
        assert!(
            visible_polygonal_intervals((-5.0, 0.0), &[concave]).is_none(),
            "concave targets require self-occlusion handling before exposure can be credited"
        );
    }

    #[test]
    fn polygonal_exposure_fails_closed_on_non_finite_realized_geometry() {
        let target = rectangle_body(0, f64::INFINITY, 0.0, 2.0, 2.0);
        assert!(visible_polygonal_intervals((-5.0, 0.0), &[target]).is_none());
    }

    #[test]
    fn polygonal_exposure_splits_a_partially_shadowed_target_edge() {
        let target = rectangle_body(0, 5.0, 0.0, 2.0, 2.0);
        let blocker = rectangle_body(0, 2.5, 0.0, 0.5, 0.5);
        let intervals = visible_polygonal_intervals((-5.0, 0.0), &[target, blocker])
            .expect("rectangles have exact polygonal boundaries");

        // The target's left edge faces the source. A smaller blocker shadows
        // its middle, so both visible shoulders must remain and total exposure
        // must be strictly between zero and the full edge length.
        let left_edge: Vec<_> = intervals
            .iter()
            .filter(|interval| {
                (interval.start.0 - 4.0).abs() < 1e-8 && (interval.end.0 - 4.0).abs() < 1e-8
            })
            .collect();
        assert!(
            left_edge.len() >= 2,
            "expected separate visible shoulders: {left_edge:?}"
        );
        let visible_length: f64 = left_edge
            .iter()
            .map(|interval| {
                (interval.end.0 - interval.start.0).hypot(interval.end.1 - interval.start.1)
            })
            .sum();
        assert!(
            visible_length > 0.0,
            "some of the target edge should remain exposed"
        );
        assert!(
            visible_length < 2.0 - 1e-6,
            "the blocker should shadow part of the edge"
        );
    }

    #[test]
    fn pose_cell_visibility_provenance_matches_static_geometry_at_zero_motion() {
        let bodies = [rectangle_body(0, 5.0, 0.0, 2.0, 2.0)];
        let target_indices = [0];
        let motion = [VisibilityBodyMotionBound {
            rotation_pivot: (5.0, 0.0),
            radius_bound: 2.0,
            ..VisibilityBodyMotionBound::default()
        }];

        let static_intervals = visible_polygonal_intervals_for_targets_with_cut_provenance(
            (-5.0, 0.0),
            &bodies,
            &target_indices,
        )
        .expect("static rectangle visibility should be exact");
        let pose_cell_intervals =
            visible_polygonal_intervals_for_targets_with_pose_cell_provenance(
                (-5.0, 0.0),
                &bodies,
                &target_indices,
                0.0,
                &motion,
            )
            .expect("zero-motion pose-cell visibility should be exact");

        let static_geometry = static_intervals
            .iter()
            .map(|entry| entry.interval)
            .collect::<Vec<_>>();
        let pose_cell_geometry = pose_cell_intervals
            .iter()
            .map(|entry| entry.interval)
            .collect::<Vec<_>>();
        assert_eq!(pose_cell_geometry, static_geometry);
        assert!(pose_cell_intervals
            .iter()
            .any(|entry| entry.visibility_signature.is_some()));
    }

    #[test]
    fn pose_cell_inventory_reuses_swept_blocker_that_moves_from_behind_to_front() {
        let reference_bodies = [
            rectangle_body(0, 5.0, 0.0, 2.0, 2.0),
            rectangle_body(1, 8.0, 0.0, 0.5, 0.5),
        ];
        let target_indices = [0];
        let motion = [
            VisibilityBodyMotionBound {
                rotation_pivot: (5.0, 0.0),
                radius_bound: 2.0,
                ..VisibilityBodyMotionBound::default()
            },
            VisibilityBodyMotionBound {
                translation_bound: 6.0,
                rotation_pivot: (8.0, 0.0),
                radius_bound: 1.0,
                ..VisibilityBodyMotionBound::default()
            },
        ];
        let inventory = build_visibility_pose_cell_candidate_inventory(
            (-5.0, 0.0),
            &reference_bodies,
            &target_indices,
            0.5,
            &motion,
        )
        .expect("the swept blocker inventory should be certifiable");
        assert!(inventory
            .candidates_for_target(0)
            .expect("target inventory")
            .contains(&1));

        let sampled_bodies = [
            rectangle_body(0, 5.0, 0.0, 2.0, 2.0),
            rectangle_body(1, 3.0, 0.0, 0.5, 0.5),
        ];
        let reused = visible_polygonal_intervals_for_targets_with_pose_cell_inventory(
            (-4.5, 0.0),
            &sampled_bodies,
            &target_indices,
            0.5,
            &motion,
            &inventory,
        )
        .expect("the same swept inventory must cover the moved source and blocker");
        let exact = visible_polygonal_intervals_for_targets_with_cut_provenance(
            (-4.5, 0.0),
            &sampled_bodies,
            &target_indices,
        )
        .expect("exact sampled-pose visibility");
        assert_eq!(
            reused
                .iter()
                .map(|entry| entry.interval)
                .collect::<Vec<_>>(),
            exact.iter().map(|entry| entry.interval).collect::<Vec<_>>()
        );
        assert!(reused.iter().any(|entry| {
            entry
                .visibility_signature
                .as_ref()
                .is_some_and(|signature| {
                    signature
                        .interval_blocker_bodies
                        .iter()
                        .any(|blockers| blockers.contains(&(1, 1)))
                })
        }));

        let undersized_radius = [
            VisibilityBodyMotionBound {
                rotation_pivot: (5.0, 0.0),
                radius_bound: 0.25,
                ..VisibilityBodyMotionBound::default()
            },
            motion[1],
        ];
        assert!(build_visibility_pose_cell_candidate_inventory(
            (-5.0, 0.0),
            &reference_bodies,
            &target_indices,
            0.5,
            &undersized_radius,
        )
        .is_none());

        let outside_envelope = [
            rectangle_body(0, 5.0, 0.0, 2.0, 2.0),
            rectangle_body(1, 1.0, 0.0, 0.5, 0.5),
        ];
        assert!(
            visible_polygonal_intervals_for_targets_with_pose_cell_inventory(
                (-4.5, 0.0),
                &outside_envelope,
                &target_indices,
                0.5,
                &motion,
                &inventory,
            )
            .is_none()
        );

        let mut changed_form = sampled_bodies.clone();
        changed_form[1].parts[0].form = Form::Circle { radius: 0.25 };
        assert!(
            visible_polygonal_intervals_for_targets_with_pose_cell_inventory(
                (-4.5, 0.0),
                &changed_form,
                &target_indices,
                0.5,
                &motion,
                &inventory,
            )
            .is_none()
        );
    }

    #[test]
    fn fully_shadowed_target_edge_has_no_exposed_interval() {
        let target = rectangle_body(0, 5.0, 0.0, 2.0, 2.0);
        let blocker = rectangle_body(0, 2.5, 0.0, 0.5, 4.0);
        let intervals = visible_polygonal_intervals((-5.0, 0.0), &[target, blocker])
            .expect("rectangles have exact polygonal boundaries");

        let left_edge: Vec<_> = intervals
            .iter()
            .filter(|interval| {
                (interval.start.0 - 4.0).abs() < 1e-8 && (interval.end.0 - 4.0).abs() < 1e-8
            })
            .collect();
        assert!(
            left_edge.is_empty(),
            "a fully occluded target edge must not receive exposure: {left_edge:?}"
        );
    }

    #[test]
    fn line_boundary_intersection_splits_a_straight_receiver_interval() {
        let target = rectangle_body(0, 5.0, 0.0, 2.0, 2.0);
        let line = OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index: 1,
                form: Form::Line { length: 4.0 },
                x: 4.0,
                y: 0.0,
                rotation_radians: std::f64::consts::FRAC_PI_4,
            }],
            min_x: 2.0,
            max_x: 6.0,
            min_y: -2.0,
            max_y: 2.0,
        };
        let intervals = visible_polygonal_intervals((-5.0, 0.0), &[target, line])
            .expect("finite line boundaries must participate in interval partitioning");
        let left_edge: Vec<_> = intervals
            .iter()
            .filter(|interval| {
                (interval.start.0 - 4.0).abs() < 1e-8 && (interval.end.0 - 4.0).abs() < 1e-8
            })
            .collect();
        assert!(
            left_edge.iter().any(|interval| {
                interval.start.1.abs() < 1e-8 || interval.end.1.abs() < 1e-8
            }),
            "the exact line/receiver intersection at y=0 must be a partition boundary: {left_edge:?}"
        );
        assert!(
            left_edge
                .iter()
                .all(|interval| { (interval.start.1 + interval.end.1) * 0.5 >= -1e-8 }),
            "the line lies before the receiver for y<0 and behind it for y>0: {left_edge:?}"
        );
    }

    #[test]
    fn polygonal_exposure_keeps_the_full_edge_when_unobstructed() {
        let target = rectangle_body(0, 5.0, 0.0, 2.0, 2.0);
        let intervals = visible_polygonal_intervals((-5.0, 0.0), &[target])
            .expect("rectangle has exact polygonal boundaries");
        let left_edge: Vec<_> = intervals
            .iter()
            .filter(|interval| {
                (interval.start.0 - 4.0).abs() < 1e-8 && (interval.end.0 - 4.0).abs() < 1e-8
            })
            .collect();
        let visible_length: f64 = left_edge
            .iter()
            .map(|interval| {
                (interval.end.0 - interval.start.0).hypot(interval.end.1 - interval.start.1)
            })
            .sum();
        assert!((visible_length - 2.0).abs() < 1e-8);
        assert_eq!(
            left_edge.len(),
            1,
            "unobstructed edge should remain one interval"
        );
    }

    #[test]
    fn straight_boundary_query_skips_curved_targets_without_polygonization() {
        let circle = OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index: 0,
                form: Form::Circle { radius: 1.0 },
                x: 5.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            min_x: 4.0,
            max_x: 6.0,
            min_y: -1.0,
            max_y: 1.0,
        };
        assert!(
            visible_polygonal_intervals((-5.0, 0.0), &[circle])
                .expect("curved targets are handled by the companion arc query")
                .is_empty(),
            "straight query must not invent polygon edges for a circle"
        );

        let family = crate::capillary_geometry::CapillaryContactFamily::solve(
            0.5,
            std::f64::consts::FRAC_PI_2,
        )
        .expect("valid capillary family");
        let arc = crate::capillary_geometry::CircularArcBoundary::from_flat_contact(family)
            .expect("valid exact circular arc");
        let fluid = OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index: 0,
                form: Form::Fluid {
                    nominal_area: 0.5,
                    boundary: None,
                    arc_boundary: Some(arc),
                    multi_contact_boundary: None,
                },
                x: 5.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            min_x: 4.0,
            max_x: 6.0,
            min_y: -1.0,
            max_y: 1.0,
        };
        assert!(
            visible_polygonal_intervals((-5.0, 0.0), &[fluid])
                .expect("exact fluid arcs are handled by the companion arc query")
                .is_empty(),
            "straight query must not invent polygon edges for an exact fluid arc"
        );
    }
}

/// Exact projected extent of a fully exposed circular free-boundary arc.
///
/// The arc is represented analytically; no polygonal tessellation is used.
/// Its outward normal is radial from the circle center. The integral clips
/// back-facing portions to zero, matching the straight-boundary primitive.
/// This measures projection only: callers must still discover visible arc
/// intervals and subtract occluded portions before using the result.
pub(crate) fn projected_arc_extent(
    arc: crate::capillary_geometry::CircularArcBoundary,
    toward_source: (f64, f64),
) -> Option<f64> {
    if !arc.is_valid() || !toward_source.0.is_finite() || !toward_source.1.is_finite() {
        return None;
    }
    let direction_length = toward_source.0.hypot(toward_source.1);
    if (direction_length - 1.0).abs() > 1e-9 {
        return None;
    }

    // Normalize first so valid but very large finite angle values cannot
    // overflow the integer bounds used to enumerate sign changes.
    let start = arc.start_angle_radians.rem_euclid(std::f64::consts::TAU);
    let end = start + arc.sweep_radians;
    let source_angle = toward_source.1.atan2(toward_source.0);
    let mut cuts = vec![start, end];

    // cos(theta - source_angle) changes sign at these exact angles.
    let first = ((start - source_angle - std::f64::consts::FRAC_PI_2) / std::f64::consts::PI)
        .floor() as i64
        - 1;
    let last = ((end - source_angle - std::f64::consts::FRAC_PI_2) / std::f64::consts::PI).ceil()
        as i64
        + 1;
    for k in first..=last {
        let angle = source_angle + std::f64::consts::FRAC_PI_2 + k as f64 * std::f64::consts::PI;
        if angle > start && angle < end {
            cuts.push(angle);
        }
    }
    cuts.sort_by(f64::total_cmp);

    let mut integral = 0.0;
    for pair in cuts.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if (((a + b) * 0.5) - source_angle).cos() > 0.0 {
            // Integral cos(theta - alpha) dtheta = sin(theta - alpha).
            integral += (b - source_angle).sin() - (a - source_angle).sin();
        }
    }
    let extent = arc.radius * integral.max(0.0);
    extent.is_finite().then_some(extent)
}

/// A visible sub-arc of a realized circular boundary in world coordinates.
/// Each returned arc is disjoint from the others on its source constituent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ExposedCircularArcInterval {
    pub body_index: usize,
    pub unit_index: usize,
    pub arc: crate::capillary_geometry::CircularArcBoundary,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WorldCircleArc {
    body_index: usize,
    unit_index: usize,
    center: (f64, f64),
    radius: f64,
    start: f64,
    sweep: f64,
    center_local: (f64, f64),
    start_local: f64,
}

/// Return the realized circular boundary pieces for a constituent. Compound
/// fluid pieces are kept separate so their mutual occlusion can be classified.
fn push_world_circle_arc(
    arcs: &mut Vec<WorldCircleArc>,
    body_index: usize,
    part: &crate::organism_geometry::PlacedForm,
    center_local: (f64, f64),
    radius: f64,
    start: f64,
    sweep: f64,
) -> Option<()> {
    let (sin, cos) = part.rotation_radians.sin_cos();
    let center = (
        part.x + center_local.0 * cos - center_local.1 * sin,
        part.y + center_local.0 * sin + center_local.1 * cos,
    );
    let normalized_start = (start.rem_euclid(std::f64::consts::TAU)
        + part.rotation_radians.rem_euclid(std::f64::consts::TAU))
    .rem_euclid(std::f64::consts::TAU);
    if !center.0.is_finite()
        || !center.1.is_finite()
        || !center_local.0.is_finite()
        || !center_local.1.is_finite()
        || !radius.is_finite()
        || radius <= 0.0
        || !start.is_finite()
        || !normalized_start.is_finite()
        || !sweep.is_finite()
        || sweep <= 0.0
        || sweep > std::f64::consts::TAU
    {
        return None;
    }
    arcs.push(WorldCircleArc {
        body_index,
        unit_index: part.unit_index,
        center,
        radius,
        start: normalized_start,
        sweep,
        center_local,
        start_local: start.rem_euclid(std::f64::consts::TAU),
    });
    Some(())
}

fn realized_circle_arcs(
    body_index: usize,
    part: &crate::organism_geometry::PlacedForm,
) -> Option<Vec<WorldCircleArc>> {
    use crate::resources::Form;
    let mut arcs = Vec::new();
    match &part.form {
        Form::Circle { radius } => {
            push_world_circle_arc(
                &mut arcs,
                body_index,
                part,
                (0.0, 0.0),
                *radius,
                0.0,
                std::f64::consts::TAU,
            )?;
        }
        Form::Fluid {
            nominal_area,
            boundary,
            arc_boundary,
            multi_contact_boundary,
        } => {
            if !nominal_area.is_finite() || *nominal_area <= 0.0 {
                return None;
            }
            if (boundary.is_some() && (arc_boundary.is_some() || multi_contact_boundary.is_some()))
                || (arc_boundary.is_some() && multi_contact_boundary.is_some())
            {
                return None;
            }
            if let Some(multi) = multi_contact_boundary {
                if !multi.is_valid()
                    || (multi.nominal_area - *nominal_area).abs()
                        > 1e-10 * nominal_area.abs().max(1.0)
                {
                    return None;
                }
                for piece in &multi.pieces {
                    if let crate::capillary_geometry::FluidBoundaryPiece::FreeArc(arc) = piece {
                        if !arc.is_valid() {
                            return None;
                        }
                        push_world_circle_arc(
                            &mut arcs,
                            body_index,
                            part,
                            (arc.center_x, arc.center_y),
                            arc.radius,
                            arc.start_angle_radians,
                            arc.sweep_radians,
                        )?;
                    }
                }
                return Some(arcs);
            }
            if let Some(vertices) = boundary {
                // A polygonal fluid boundary is handled by the polygon solver,
                // not by this circular-arc path.
                if vertices.len() >= 3
                    && vertices.iter().all(|(x, y)| x.is_finite() && y.is_finite())
                {
                    return Some(Vec::new());
                }
                return None;
            }
            if let Some(arc) = arc_boundary {
                if !arc.is_valid()
                    || (arc.segment_area()? - *nominal_area).abs()
                        > 1e-10 * nominal_area.abs().max(1.0)
                {
                    return None;
                }
                push_world_circle_arc(
                    &mut arcs,
                    body_index,
                    part,
                    (arc.center_x, arc.center_y),
                    arc.radius,
                    arc.start_angle_radians,
                    arc.sweep_radians,
                )?;
            } else {
                let radius = (nominal_area / std::f64::consts::PI).sqrt();
                push_world_circle_arc(
                    &mut arcs,
                    body_index,
                    part,
                    (0.0, 0.0),
                    radius,
                    0.0,
                    std::f64::consts::TAU,
                )?;
            }
        }
        _ => return Some(Vec::new()),
    }
    Some(arcs)
}

fn arc_has_angle(arc: WorldCircleArc, angle: f64) -> bool {
    (angle - arc.start).rem_euclid(std::f64::consts::TAU) <= arc.sweep + 1e-10
}

fn add_circle_tangent_points(
    center: (f64, f64),
    radius: f64,
    source: (f64, f64),
    points: &mut Vec<(f64, f64)>,
) {
    let dx = source.0 - center.0;
    let dy = source.1 - center.1;
    let distance = dx.hypot(dy);
    if !distance.is_finite() || !radius.is_finite() || radius <= 0.0 || distance <= radius {
        return;
    }
    let base = dy.atan2(dx);
    let offset = (radius / distance).clamp(-1.0, 1.0).acos();
    for angle in [base - offset, base + offset] {
        points.push((
            center.0 + radius * angle.cos(),
            center.1 + radius * angle.sin(),
        ));
    }
}

fn add_circle_segment_intersections(
    center: (f64, f64),
    radius: f64,
    start: (f64, f64),
    end: (f64, f64),
    points: &mut Vec<(f64, f64)>,
) -> Option<()> {
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;
    let fx = start.0 - center.0;
    let fy = start.1 - center.1;
    let a = dx * dx + dy * dy;
    if !a.is_finite() || a <= f64::EPSILON {
        return Some(());
    }
    let b = 2.0 * (fx * dx + fy * dy);
    let c = fx * fx + fy * fy - radius * radius;
    let discriminant = b * b - 4.0 * a * c;
    if !b.is_finite() || !c.is_finite() || !discriminant.is_finite() {
        return None;
    }
    if discriminant < -1e-10 {
        return Some(());
    }
    let root = discriminant.max(0.0).sqrt();
    for t in [(-b - root) / (2.0 * a), (-b + root) / (2.0 * a)] {
        if (-1e-10..=1.0 + 1e-10).contains(&t) {
            points.push((start.0 + t * dx, start.1 + t * dy));
        }
    }
    Some(())
}

fn add_circle_circle_intersections(
    first: (f64, f64),
    first_radius: f64,
    second: (f64, f64),
    second_radius: f64,
    points: &mut Vec<(f64, f64)>,
) -> Option<()> {
    let dx = second.0 - first.0;
    let dy = second.1 - first.1;
    let distance = dx.hypot(dy);
    if !distance.is_finite()
        || !first_radius.is_finite()
        || !second_radius.is_finite()
        || first_radius <= 0.0
        || second_radius <= 0.0
    {
        return None;
    }
    if distance <= 1e-12
        || distance > first_radius + second_radius + 1e-10
        || distance < (first_radius - second_radius).abs() - 1e-10
    {
        return Some(());
    }
    let along = (first_radius * first_radius - second_radius * second_radius + distance * distance)
        / (2.0 * distance);
    let height_squared = first_radius * first_radius - along * along;
    if !along.is_finite() || !height_squared.is_finite() {
        return None;
    }
    if height_squared < -1e-9 {
        return Some(());
    }
    let height = height_squared.max(0.0).sqrt();
    let ux = dx / distance;
    let uy = dy / distance;
    let base = (first.0 + along * ux, first.1 + along * uy);
    points.push((base.0 - height * uy, base.1 + height * ux));
    if height > 1e-10 {
        points.push((base.0 + height * uy, base.1 - height * ux));
    }
    Some(())
}

/// Exact visibility partitioning for circle and circular-fluid boundaries under
/// a finite point source. Visibility changes only along rays through straight-boundary
/// vertices and intersections, circular-arc endpoints and intersections, or circle
/// tangencies. Those critical rays cut the target arc; exact geometry queries
/// classify each resulting interval.
/// Compound boundaries are tested against all realized pieces; invalid geometry returns None.
#[allow(dead_code)]
pub(crate) fn visible_circular_arc_intervals(
    source: (f64, f64),
    bodies: &[crate::organism_geometry::OrganismBodyGeometry],
) -> Option<Vec<ExposedCircularArcInterval>> {
    let targets: Vec<_> = (0..bodies.len()).collect();
    visible_circular_arc_intervals_for_targets(source, bodies, &targets)
}

/// Compute visible arc intervals only for selected receiver bodies; all arcs
/// still participate in blocker intersections and occlusion classification.
pub(crate) fn visible_circular_arc_intervals_for_targets(
    source: (f64, f64),
    bodies: &[crate::organism_geometry::OrganismBodyGeometry],
    target_body_indices: &[usize],
) -> Option<Vec<ExposedCircularArcInterval>> {
    use crate::capillary_geometry::FluidBoundaryPiece;
    use crate::resources::Form;
    if !source.0.is_finite() || !source.1.is_finite() {
        return None;
    }
    let mut target_body_mask = vec![false; bodies.len()];
    for &index in target_body_indices {
        *target_body_mask.get_mut(index)? = true;
    }
    let reference_angle = reference_angle_for_bodies(source, bodies)?;
    let body_intervals = body_angular_intervals(source, bodies, reference_angle)?;
    let body_min_distances = body_minimum_distances(source, bodies)?;

    let mut all_arcs = Vec::new();
    let mut critical_points = Vec::new();
    for (body_index, body) in bodies.iter().enumerate() {
        let mut seen = Vec::with_capacity(body.parts.len());
        for part in &body.parts {
            if seen.contains(&part.unit_index)
                || !part.x.is_finite()
                || !part.y.is_finite()
                || !part.rotation_radians.is_finite()
            {
                return None;
            }
            seen.push(part.unit_index);
            if let Some(vertices) = part.form.polygon_vertices() {
                let (sin, cos) = part.rotation_radians.sin_cos();
                for (x, y) in vertices {
                    critical_points.push((part.x + x * cos - y * sin, part.y + x * sin + y * cos));
                }
            }
            match &part.form {
                Form::Circle { radius } => {
                    add_circle_tangent_points(
                        (part.x, part.y),
                        *radius,
                        source,
                        &mut critical_points,
                    );
                }
                Form::Line { length } => {
                    if !length.is_finite() || *length <= 0.0 {
                        return None;
                    }
                    let (sin, cos) = part.rotation_radians.sin_cos();
                    let half = *length * 0.5;
                    critical_points.extend([
                        (part.x - half * cos, part.y - half * sin),
                        (part.x + half * cos, part.y + half * sin),
                    ]);
                }
                Form::Fluid {
                    nominal_area,
                    boundary,
                    arc_boundary,
                    multi_contact_boundary,
                } => {
                    if let Some(verts) = boundary {
                        let (sin, cos) = part.rotation_radians.sin_cos();
                        for &(x, y) in verts {
                            critical_points
                                .push((part.x + x * cos - y * sin, part.y + x * sin + y * cos));
                        }
                    }
                    if let Some(arc) = arc_boundary {
                        if !arc.is_valid() {
                            return None;
                        }
                        let world = arc.transformed(part.x, part.y, part.rotation_radians)?;
                        critical_points.push(world.start_point());
                        critical_points.push(world.end_point());
                        add_circle_tangent_points(
                            (world.center_x, world.center_y),
                            world.radius,
                            source,
                            &mut critical_points,
                        );
                    }
                    if let Some(multi) = multi_contact_boundary {
                        for piece in &multi.pieces {
                            match piece {
                                FluidBoundaryPiece::SolidContact {
                                    start_x,
                                    start_y,
                                    end_x,
                                    end_y,
                                    ..
                                } => {
                                    let (sin, cos) = part.rotation_radians.sin_cos();
                                    for (x, y) in [(*start_x, *start_y), (*end_x, *end_y)] {
                                        critical_points.push((
                                            part.x + x * cos - y * sin,
                                            part.y + x * sin + y * cos,
                                        ));
                                    }
                                }
                                FluidBoundaryPiece::FreeArc(arc) => {
                                    let world =
                                        arc.transformed(part.x, part.y, part.rotation_radians)?;
                                    critical_points.push(world.start_point());
                                    critical_points.push(world.end_point());
                                    add_circle_tangent_points(
                                        (world.center_x, world.center_y),
                                        world.radius,
                                        source,
                                        &mut critical_points,
                                    );
                                }
                            }
                        }
                    }
                    if boundary.is_none()
                        && arc_boundary.is_none()
                        && multi_contact_boundary.is_none()
                    {
                        if !nominal_area.is_finite() || *nominal_area <= 0.0 {
                            return None;
                        }
                        let radius = (*nominal_area / std::f64::consts::PI).sqrt();
                        add_circle_tangent_points(
                            (part.x, part.y),
                            radius,
                            source,
                            &mut critical_points,
                        );
                    }
                }
                _ => {}
            }
            all_arcs.extend(realized_circle_arcs(body_index, part)?);
        }
    }

    // Visibility changes where a target circle crosses a straight boundary
    // or another circular boundary, as well as at blocker silhouettes. Add
    // those exact intersections as critical rays before classifying intervals.
    let mut straight_edges = Vec::new();
    for body in bodies {
        for part in &body.parts {
            let (sin, cos) = part.rotation_radians.sin_cos();
            let mut local_edges = Vec::new();
            if let Some(vertices) = part.form.polygon_vertices() {
                for index in 0..vertices.len() {
                    local_edges.push((vertices[index], vertices[(index + 1) % vertices.len()]));
                }
            }
            match &part.form {
                Form::Line { length } => {
                    local_edges.push(((-length * 0.5, 0.0), (*length * 0.5, 0.0)));
                }
                Form::Fluid {
                    boundary,
                    arc_boundary,
                    multi_contact_boundary,
                    ..
                } => {
                    if let Some(vertices) = boundary {
                        for index in 0..vertices.len() {
                            local_edges
                                .push((vertices[index], vertices[(index + 1) % vertices.len()]));
                        }
                    }
                    if let Some(arc) = arc_boundary {
                        local_edges.push((arc.start_point(), arc.end_point()));
                    }
                    if let Some(multi) = multi_contact_boundary {
                        for piece in &multi.pieces {
                            if let FluidBoundaryPiece::SolidContact {
                                start_x,
                                start_y,
                                end_x,
                                end_y,
                                ..
                            } = piece
                            {
                                local_edges.push(((*start_x, *start_y), (*end_x, *end_y)));
                            }
                        }
                    }
                }
                _ => {}
            }
            for (a, b) in local_edges {
                let world = |point: (f64, f64)| {
                    (
                        part.x + point.0 * cos - point.1 * sin,
                        part.y + point.0 * sin + point.1 * cos,
                    )
                };
                straight_edges.push((world(a), world(b)));
            }
        }
    }
    for target in &all_arcs {
        if !target_body_mask[target.body_index] {
            continue;
        }
        for &(start, end) in &straight_edges {
            add_circle_segment_intersections(
                target.center,
                target.radius,
                start,
                end,
                &mut critical_points,
            )?;
        }
    }
    for (first_index, first) in all_arcs.iter().copied().enumerate() {
        if !target_body_mask[first.body_index] {
            continue;
        }
        for (second_index, second) in all_arcs.iter().copied().enumerate() {
            if second_index == first_index {
                continue;
            }
            if target_body_mask[second.body_index] && second_index < first_index {
                continue;
            }
            let mut intersections = Vec::with_capacity(2);
            add_circle_circle_intersections(
                first.center,
                first.radius,
                second.center,
                second.radius,
                &mut intersections,
            )?;
            for point in intersections {
                let first_angle = (point.1 - first.center.1).atan2(point.0 - first.center.0);
                let second_angle = (point.1 - second.center.1).atan2(point.0 - second.center.0);
                if arc_has_angle(first, first_angle) && arc_has_angle(second, second_angle) {
                    critical_points.push(point);
                }
            }
        }
    }

    if critical_points
        .iter()
        .any(|(x, y)| !x.is_finite() || !y.is_finite())
        || straight_edges.iter().any(|(a, b)| {
            !a.0.is_finite() || !a.1.is_finite() || !b.0.is_finite() || !b.1.is_finite()
        })
    {
        return None;
    }

    let mut output = Vec::new();
    for target in all_arcs {
        if !target_body_mask[target.body_index] {
            continue;
        }
        // If the source is inside the target material, its boundary cannot be
        // safely interpreted as an externally illuminated surface.
        if let Some(body) = bodies.get(target.body_index) {
            if body.contains_point(source.0, source.1) {
                continue;
            }
        }

        let candidate_body_indices = candidate_body_indices_for_bounds(
            source,
            (
                target.center.0 - target.radius,
                target.center.0 + target.radius,
                target.center.1 - target.radius,
                target.center.1 + target.radius,
            ),
            reference_angle,
            &body_intervals,
            &body_min_distances,
        )?;
        let mut cuts = vec![target.start, target.start + target.sweep];
        if target.sweep >= std::f64::consts::TAU - 1e-10 {
            cuts.push(target.start + target.sweep * 0.5);
        }

        // Tangencies on the target circle are exact transitions between
        // front-facing and back-facing boundary normals for a point source.
        let mut target_tangencies = Vec::new();
        add_circle_tangent_points(target.center, target.radius, source, &mut target_tangencies);
        for point in target_tangencies {
            let angle = (point.1 - target.center.1).atan2(point.0 - target.center.0);
            if arc_has_angle(target, angle) {
                let unwrapped =
                    target.start + (angle - target.start).rem_euclid(std::f64::consts::TAU);
                if unwrapped > target.start + 1e-10
                    && unwrapped < target.start + target.sweep - 1e-10
                {
                    cuts.push(unwrapped);
                }
            }
        }

        for point in &critical_points {
            let dx = point.0 - source.0;
            let dy = point.1 - source.1;
            if dx.hypot(dy) <= 1e-12 {
                continue;
            }
            let direction = (dx, dy);
            let ox = source.0 - target.center.0;
            let oy = source.1 - target.center.1;
            let a = direction.0 * direction.0 + direction.1 * direction.1;
            let b = 2.0 * (ox * direction.0 + oy * direction.1);
            let c = ox * ox + oy * oy - target.radius * target.radius;
            let discriminant = b * b - 4.0 * a * c;
            if !discriminant.is_finite() {
                return None;
            }
            if discriminant < -1e-10 {
                continue;
            }
            let root = discriminant.max(0.0).sqrt();
            for t in [(-b - root) / (2.0 * a), (-b + root) / (2.0 * a)] {
                if t <= 1e-10 {
                    continue;
                }
                let x = source.0 + t * direction.0;
                let y = source.1 + t * direction.1;
                let angle = (y - target.center.1).atan2(x - target.center.0);
                if arc_has_angle(target, angle) {
                    let unwrapped =
                        target.start + (angle - target.start).rem_euclid(std::f64::consts::TAU);
                    if unwrapped > target.start + 1e-10
                        && unwrapped < target.start + target.sweep - 1e-10
                    {
                        cuts.push(unwrapped);
                    }
                }
            }
        }
        cuts.sort_by(f64::total_cmp);
        cuts.dedup_by(|a, b| (*a - *b).abs() <= 1e-10);

        for pair in cuts.windows(2) {
            let (start, end) = (pair[0], pair[1]);
            if end - start <= 1e-10 || end - start >= std::f64::consts::TAU {
                continue;
            }
            let angle = (start + end) * 0.5;
            let point = (
                target.center.0 + target.radius * angle.cos(),
                target.center.1 + target.radius * angle.sin(),
            );
            let normal = (angle.cos(), angle.sin());
            let toward = (source.0 - point.0, source.1 - point.1);
            let distance = toward.0.hypot(toward.1);
            if !distance.is_finite()
                || distance <= 1e-12
                || normal.0 * toward.0 + normal.1 * toward.1 <= 1e-10
            {
                continue;
            }
            // Include the target constituent in this query: its own other
            // boundary pieces must be allowed to occlude a concave/compound
            // fluid arc. The target endpoint itself is excluded by the strict
            // before-target intersection checks in the exact geometry queries.
            if crate::organism_geometry::source_path_blocked_for_candidates(
                source,
                point,
                bodies,
                usize::MAX,
                usize::MAX,
                &candidate_body_indices,
            ) {
                continue;
            }
            output.push(ExposedCircularArcInterval {
                body_index: target.body_index,
                unit_index: target.unit_index,
                arc: crate::capillary_geometry::CircularArcBoundary {
                    center_x: target.center.0,
                    center_y: target.center.1,
                    radius: target.radius,
                    start_angle_radians: start.rem_euclid(std::f64::consts::TAU),
                    sweep_radians: end - start,
                },
            });
        }
    }
    Some(output)
}

/// Exact projected extent of a circular boundary interval under parallel rays.
///
/// `toward_source` is the single unit direction shared by every point in the
/// frame. Integrating the positive outward-normal projection gives the
/// interval's exposed width without point-source direction variation or
/// numerical quadrature.
pub(crate) fn parallel_projected_arc_extent(
    arc: crate::capillary_geometry::CircularArcBoundary,
    toward_source: (f64, f64),
) -> Option<f64> {
    if !arc.center_x.is_finite()
        || !arc.center_y.is_finite()
        || !arc.radius.is_finite()
        || arc.radius <= 0.0
        || !arc.start_angle_radians.is_finite()
        || !arc.sweep_radians.is_finite()
        || arc.sweep_radians <= 0.0
        || arc.sweep_radians > std::f64::consts::TAU
        || !toward_source.0.is_finite()
        || !toward_source.1.is_finite()
    {
        return None;
    }
    let direction_length = toward_source.0.hypot(toward_source.1);
    if (direction_length - 1.0).abs() > 1e-9 {
        return None;
    }

    let direction_angle = toward_source.1.atan2(toward_source.0);
    let start = arc.start_angle_radians.rem_euclid(std::f64::consts::TAU);
    let end = start + arc.sweep_radians;
    let first_zero = ((start - direction_angle - std::f64::consts::FRAC_PI_2)
        / std::f64::consts::PI)
        .floor() as i64;
    let last_zero = ((end - direction_angle - std::f64::consts::FRAC_PI_2) / std::f64::consts::PI)
        .ceil() as i64;
    let mut cuts = vec![start, end];
    for index in first_zero..=last_zero {
        let angle =
            direction_angle + std::f64::consts::FRAC_PI_2 + index as f64 * std::f64::consts::PI;
        if angle > start + 1e-12 && angle < end - 1e-12 {
            cuts.push(angle);
        }
    }
    cuts.sort_by(f64::total_cmp);

    let mut extent = 0.0;
    for pair in cuts.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if ((a + b) * 0.5 - direction_angle).cos() <= 0.0 {
            continue;
        }
        extent += arc.radius * ((b - direction_angle).sin() - (a - direction_angle).sin());
    }
    (extent.is_finite() && extent >= -1e-10).then_some(extent.max(0.0))
}

#[cfg(test)]
mod parallel_arc_projection_tests {
    use super::*;

    fn arc(start: f64, sweep: f64) -> crate::capillary_geometry::CircularArcBoundary {
        crate::capillary_geometry::CircularArcBoundary {
            center_x: 0.0,
            center_y: 0.0,
            radius: 2.0,
            start_angle_radians: start,
            sweep_radians: sweep,
        }
    }

    #[test]
    fn parallel_arc_projection_is_exact_for_front_and_back_semicircles() {
        let front = arc(-std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
        let back = arc(std::f64::consts::FRAC_PI_2, std::f64::consts::PI);
        assert!((parallel_projected_arc_extent(front, (1.0, 0.0)).unwrap() - 4.0).abs() < 1e-12);
        assert!(parallel_projected_arc_extent(back, (1.0, 0.0)).unwrap() < 1e-12);
    }

    #[test]
    fn parallel_arc_projection_is_independent_of_center_and_start_angle_wrapping() {
        let mut wrapped = arc(
            8.0 * std::f64::consts::TAU + 0.0,
            std::f64::consts::FRAC_PI_2,
        );
        wrapped.center_x = 1.0e6;
        wrapped.center_y = -1.0e6;
        let expected = 2.0;
        assert!(
            (parallel_projected_arc_extent(wrapped, (1.0, 0.0)).unwrap() - expected).abs() < 1e-12
        );
    }

    #[test]
    fn parallel_arc_projection_rejects_non_unit_direction() {
        assert_eq!(
            parallel_projected_arc_extent(arc(0.0, 1.0), (2.0, 0.0)),
            None
        );
    }
}

/// Projected extent for a visible circular interval under a finite point source.
/// The pointwise source direction varies along the arc, so this uses adaptive
/// Simpson integration rather than substituting a single direction for the arc.
pub(crate) fn point_source_projected_arc_extent(
    arc: crate::capillary_geometry::CircularArcBoundary,
    source: (f64, f64),
) -> Option<f64> {
    // Visible sub-arcs can be arbitrarily short; CircularArcBoundary::is_valid
    // also enforces a minimum enclosed segment area, which is inappropriate
    // for a visibility interval cut from a larger valid boundary.
    if !arc.center_x.is_finite()
        || !arc.center_y.is_finite()
        || !arc.radius.is_finite()
        || arc.radius <= 0.0
        || !arc.start_angle_radians.is_finite()
        || !arc.sweep_radians.is_finite()
        || arc.sweep_radians <= 0.0
        || arc.sweep_radians > std::f64::consts::TAU
        || !source.0.is_finite()
        || !source.1.is_finite()
    {
        return None;
    }
    let integrand = |angle: f64| {
        let x = arc.center_x + arc.radius * angle.cos();
        let y = arc.center_y + arc.radius * angle.sin();
        let toward_x = source.0 - x;
        let toward_y = source.1 - y;
        let distance = toward_x.hypot(toward_y);
        if !distance.is_finite() || distance <= 1e-12 {
            return None;
        }
        Some(arc.radius * ((angle.cos() * toward_x + angle.sin() * toward_y) / distance).max(0.0))
    };
    fn simpson(a: f64, b: f64, fa: f64, fm: f64, fb: f64) -> Option<f64> {
        let value = (b - a) * (fa + 4.0 * fm + fb) / 6.0;
        value.is_finite().then_some(value)
    }
    #[allow(clippy::too_many_arguments)]
    fn integrate<F: Fn(f64) -> Option<f64>>(
        f: &F,
        a: f64,
        b: f64,
        fa: f64,
        fm: f64,
        fb: f64,
        whole: f64,
        tolerance: f64,
        depth: u8,
    ) -> Option<f64> {
        let mid = (a + b) * 0.5;
        let left_mid = (a + mid) * 0.5;
        let right_mid = (mid + b) * 0.5;
        let flm = f(left_mid)?;
        let frm = f(right_mid)?;
        let left = simpson(a, mid, fa, flm, fm)?;
        let right = simpson(mid, b, fm, frm, fb)?;
        let delta = left + right - whole;
        if depth == 0 || delta.abs() <= 15.0 * tolerance {
            return Some(left + right + delta / 15.0);
        }
        Some(
            integrate(f, a, mid, fa, flm, fm, left, tolerance * 0.5, depth - 1)?
                + integrate(f, mid, b, fm, frm, fb, right, tolerance * 0.5, depth - 1)?,
        )
    }
    let a = arc.start_angle_radians;
    let b = a + arc.sweep_radians;
    let mid = (a + b) * 0.5;
    let fa = integrand(a)?;
    let fm = integrand(mid)?;
    let fb = integrand(b)?;
    let whole = simpson(a, b, fa, fm, fb)?;
    let extent = integrate(
        &integrand,
        a,
        b,
        fa,
        fm,
        fb,
        whole,
        1e-10 * whole.abs().max(1.0),
        20,
    )?;
    (extent.is_finite() && extent >= -1e-10).then_some(extent.max(0.0))
}

#[cfg(test)]
mod point_source_curve_visibility_tests {
    use super::{point_source_projected_arc_extent, visible_circular_arc_intervals};
    use crate::organism_geometry::{OrganismBodyGeometry, PlacedForm};
    use crate::resources::Form;

    fn body(form: Form, x: f64, y: f64) -> OrganismBodyGeometry {
        let radius = match &form {
            Form::Circle { radius } => *radius,
            Form::Rectangle { width, height } => width.hypot(*height) * 0.5,
            _ => 10.0,
        };
        OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index: 0,
                form,
                x,
                y,
                rotation_radians: 0.0,
            }],
            min_x: x - radius,
            max_x: x + radius,
            min_y: y - radius,
            max_y: y + radius,
        }
    }

    #[test]
    fn curved_visibility_rejects_inconsistent_fluid_area_descriptors() {
        use crate::capillary_geometry::{
            CircularArcBoundary, FluidBoundaryPiece, MultiContactFluidBoundary,
        };

        let family = crate::capillary_geometry::CapillaryContactFamily::solve(
            0.5,
            std::f64::consts::FRAC_PI_2,
        )
        .expect("valid capillary family");
        let arc = CircularArcBoundary::from_flat_contact(family).expect("valid exact arc");
        let inconsistent_arc = body(
            Form::Fluid {
                nominal_area: 0.75,
                boundary: None,
                arc_boundary: Some(arc),
                multi_contact_boundary: None,
            },
            5.0,
            0.0,
        );
        assert!(
            visible_circular_arc_intervals((-5.0, 0.0), &[inconsistent_arc]).is_none(),
            "a curved receiver must not disagree with the area represented by its exact arc"
        );

        let start = arc.start_point();
        let end = arc.end_point();
        let boundary = MultiContactFluidBoundary {
            nominal_area: family.area,
            pieces: vec![
                FluidBoundaryPiece::FreeArc(arc),
                FluidBoundaryPiece::SolidContact {
                    start_x: end.0,
                    start_y: end.1,
                    end_x: start.0,
                    end_y: start.1,
                    contact_angle_radians: family.contact_angle_radians,
                },
            ],
        };
        let inconsistent_multi = body(
            Form::Fluid {
                nominal_area: family.area + 0.25,
                boundary: None,
                arc_boundary: None,
                multi_contact_boundary: Some(boundary),
            },
            5.0,
            0.0,
        );
        assert!(
            visible_circular_arc_intervals((-5.0, 0.0), &[inconsistent_multi]).is_none(),
            "compound fluid nominal area must agree with its validated boundary"
        );
    }

    #[test]
    fn point_source_circle_exposure_is_exactly_partitioned_without_a_blocker() {
        let target = body(Form::Circle { radius: 1.0 }, 5.0, 0.0);
        let intervals = visible_circular_arc_intervals((-5.0, 0.0), &[target])
            .expect("circle boundary and point-source visibility are supported");
        assert!(!intervals.is_empty());
        let sweep: f64 = intervals
            .iter()
            .map(|interval| interval.arc.sweep_radians)
            .sum();
        assert!(sweep > 0.0 && sweep < std::f64::consts::PI);
        let extent: f64 = intervals
            .iter()
            .map(|interval| point_source_projected_arc_extent(interval.arc, (-5.0, 0.0)).unwrap())
            .sum();
        assert!(extent.is_finite() && extent > 0.0);
    }

    #[test]
    fn point_source_circle_boundary_splits_around_a_polygonal_shadow() {
        let target = body(Form::Circle { radius: 1.0 }, 5.0, 0.0);
        let blocker = body(
            Form::Rectangle {
                width: 0.5,
                height: 0.25,
            },
            0.0,
            0.0,
        );
        let intervals = visible_circular_arc_intervals((-5.0, 0.0), &[target, blocker])
            .expect("polygon blockers and circular targets are supported");
        let target_intervals: Vec<_> = intervals
            .iter()
            .filter(|interval| interval.body_index == 0 && interval.unit_index == 0)
            .collect();
        assert!(
            !target_intervals.is_empty(),
            "unshadowed shoulders remain exposed"
        );
        let sweep: f64 = target_intervals
            .iter()
            .map(|interval| interval.arc.sweep_radians)
            .sum();
        assert!(sweep > 0.0 && sweep < std::f64::consts::PI);
        assert!(
            target_intervals.len() >= 2,
            "shadow transition rays must split the visible circular boundary"
        );
    }

    #[test]
    fn point_source_circle_shadow_from_finite_line_splits_at_line_endpoints() {
        let blocker = OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index: 0,
                form: Form::Line { length: 1.0 },
                x: 2.5,
                y: 0.0,
                rotation_radians: std::f64::consts::FRAC_PI_2,
            }],
            min_x: 2.5,
            max_x: 2.5,
            min_y: -0.5,
            max_y: 0.5,
        };
        let target = body(Form::Circle { radius: 1.0 }, 5.0, 0.0);
        let intervals = visible_circular_arc_intervals((-5.0, 0.0), &[blocker, target])
            .expect("finite line blockers must partition circular receivers at both endpoints");
        let visible_target: Vec<_> = intervals
            .iter()
            .filter(|interval| interval.body_index == 1)
            .collect();
        let visible_sweep: f64 = visible_target
            .iter()
            .map(|interval| interval.arc.sweep_radians)
            .sum();
        let unoccluded = visible_circular_arc_intervals(
            (-5.0, 0.0),
            &[body(Form::Circle { radius: 1.0 }, 5.0, 0.0)],
        )
        .expect("isolated circle visibility is supported");
        let unoccluded_sweep: f64 = unoccluded
            .iter()
            .map(|interval| interval.arc.sweep_radians)
            .sum();
        assert!(
            visible_target.len() >= 2,
            "line endpoint rays must split the arc"
        );

        // Each finite endpoint defines an exact ray from the sun. Verify that
        // both rays intersect the receiver at a returned arc boundary; merely
        // returning multiple intervals would not prove the split is correct.
        let source = (-5.0_f64, 0.0_f64);
        let center = (5.0_f64, 0.0_f64);
        let endpoint_angles = [-0.5_f64, 0.5_f64].map(|endpoint_y| {
            let direction = (2.5 - source.0, endpoint_y - source.1);
            let offset = (source.0 - center.0, source.1 - center.1);
            let a = direction.0 * direction.0 + direction.1 * direction.1;
            let b = 2.0 * (offset.0 * direction.0 + offset.1 * direction.1);
            let c = offset.0 * offset.0 + offset.1 * offset.1 - 1.0;
            let discriminant = b * b - 4.0 * a * c;
            let t = (-b - discriminant.sqrt()) / (2.0 * a);
            let point = (source.0 + t * direction.0, source.1 + t * direction.1);
            (point.1 - center.1)
                .atan2(point.0 - center.0)
                .rem_euclid(std::f64::consts::TAU)
        });
        let returned_boundaries: Vec<_> = visible_target
            .iter()
            .flat_map(|interval| {
                [
                    interval
                        .arc
                        .start_angle_radians
                        .rem_euclid(std::f64::consts::TAU),
                    (interval.arc.start_angle_radians + interval.arc.sweep_radians)
                        .rem_euclid(std::f64::consts::TAU),
                ]
            })
            .collect();
        for expected in endpoint_angles {
            assert!(
                returned_boundaries.iter().any(|actual| {
                    let delta = (actual - expected).abs();
                    delta.min(std::f64::consts::TAU - delta) < 1e-8
                }),
                "expected a visible-arc boundary at the exact line-endpoint ray angle {expected}; got {returned_boundaries:?}"
            );
        }
        assert!(visible_sweep > 0.0 && visible_sweep < unoccluded_sweep);
    }

    #[test]
    fn point_source_circle_visibility_splits_at_circle_circle_intersections() {
        let blocker = body(Form::Circle { radius: 0.25 }, 4.1, 0.0);
        let target = body(Form::Circle { radius: 1.0 }, 5.0, 0.0);
        let intervals = visible_circular_arc_intervals((-5.0, 0.0), &[blocker, target])
            .expect("intersecting circular boundaries are classified analytically");
        let target_intervals: Vec<_> = intervals
            .iter()
            .filter(|interval| interval.body_index == 1)
            .collect();
        assert!(!target_intervals.is_empty());
        let has_shared_boundary_endpoint = target_intervals.iter().any(|interval| {
            let start = interval.arc.start_point();
            let end = interval.arc.end_point();
            [start, end].iter().any(|point| {
                ((point.0 - 4.1).powi(2) + point.1.powi(2) - 0.25_f64.powi(2)).abs() < 1e-7
            })
        });
        assert!(
            has_shared_boundary_endpoint,
            "visible interval should terminate at a target/blocker circle intersection"
        );
    }

    #[test]
    fn point_source_circle_visibility_adds_intersections_with_straight_boundaries() {
        let target = body(Form::Circle { radius: 1.0 }, 5.0, 0.0);
        let blocker = body(
            Form::Rectangle {
                width: 1.0,
                height: 0.25,
            },
            4.0,
            0.0,
        );
        let intervals = visible_circular_arc_intervals((-5.0, 0.0), &[target, blocker])
            .expect("circle and polygon boundaries are supported");
        let visible: Vec<_> = intervals
            .iter()
            .filter(|interval| interval.body_index == 0)
            .collect();
        assert!(!visible.is_empty());
        assert!(visible.iter().all(|interval| interval.arc.is_valid()));
        // The blocker touches/overlaps the receiver's left boundary, so exact
        // circle/segment intersections must be among the partition cuts.
        let total_sweep: f64 = visible
            .iter()
            .map(|interval| interval.arc.sweep_radians)
            .sum();
        assert!(total_sweep > 0.0 && total_sweep < std::f64::consts::PI);
    }

    #[test]
    fn legacy_circular_fluid_casts_tangent_defined_shadows_on_a_circle() {
        let blocker = body(
            Form::Fluid {
                nominal_area: std::f64::consts::PI * 0.0625,
                boundary: None,
                arc_boundary: None,
                multi_contact_boundary: None,
            },
            0.0,
            0.0,
        );
        let target = body(Form::Circle { radius: 1.0 }, 5.0, 0.0);
        let intervals = visible_circular_arc_intervals((-5.0, 0.0), &[blocker, target])
            .expect("legacy circular fluid is an exact circular occluder");
        let visible_target_sweep: f64 = intervals
            .iter()
            .filter(|interval| interval.body_index == 1)
            .map(|interval| interval.arc.sweep_radians)
            .sum();
        let unoccluded = visible_circular_arc_intervals(
            (-5.0, 0.0),
            &[body(Form::Circle { radius: 1.0 }, 5.0, 0.0)],
        )
        .expect("isolated circle visibility is supported");
        let unoccluded_sweep: f64 = unoccluded
            .iter()
            .map(|interval| interval.arc.sweep_radians)
            .sum();
        assert!(visible_target_sweep > 0.0);
        assert!(visible_target_sweep < unoccluded_sweep);
    }

    #[test]
    fn single_convex_fluid_arc_uses_its_exact_boundary_under_point_source() {
        let family = crate::capillary_geometry::CapillaryContactFamily::solve(
            0.5,
            std::f64::consts::FRAC_PI_2,
        )
        .expect("valid capillary family");
        let arc = crate::capillary_geometry::CircularArcBoundary::from_flat_contact(family)
            .expect("valid exact arc");
        let target = body(
            Form::Fluid {
                nominal_area: 0.5,
                boundary: None,
                arc_boundary: Some(arc),
                multi_contact_boundary: None,
            },
            5.0,
            0.0,
        );
        let intervals = visible_circular_arc_intervals((-5.0, 0.0), &[target])
            .expect("single convex circular-segment fluid boundary is supported");
        assert!(!intervals.is_empty());
        assert!(intervals.iter().all(|interval| interval.arc.is_valid()));
        let extent: f64 = intervals
            .iter()
            .map(|interval| point_source_projected_arc_extent(interval.arc, (-5.0, 0.0)).unwrap())
            .sum();
        assert!(extent.is_finite() && extent > 0.0);
    }

    #[test]
    fn multi_contact_fluid_arc_is_visible_with_its_contact_chord_checked_for_self_shadow() {
        use crate::capillary_geometry::{FluidBoundaryPiece, MultiContactFluidBoundary};

        let family = crate::capillary_geometry::CapillaryContactFamily::solve(
            0.5,
            std::f64::consts::FRAC_PI_2,
        )
        .expect("valid capillary family");
        let arc = crate::capillary_geometry::CircularArcBoundary::from_flat_contact(family)
            .expect("valid exact arc");
        let start = arc.start_point();
        let end = arc.end_point();
        let boundary = MultiContactFluidBoundary {
            nominal_area: family.area,
            pieces: vec![
                FluidBoundaryPiece::FreeArc(arc),
                FluidBoundaryPiece::SolidContact {
                    start_x: end.0,
                    start_y: end.1,
                    end_x: start.0,
                    end_y: start.1,
                    contact_angle_radians: family.contact_angle_radians,
                },
            ],
        };
        assert!(
            boundary.is_valid(),
            "fixture must be a closed exact boundary"
        );
        let target = body(
            Form::Fluid {
                nominal_area: family.area,
                boundary: None,
                arc_boundary: None,
                multi_contact_boundary: Some(boundary),
            },
            5.0,
            0.0,
        );
        let intervals = visible_circular_arc_intervals((5.0, 10.0), &[target])
            .expect("compound fluid arcs are supported as exact curved targets");
        assert!(!intervals.is_empty());
        assert!(intervals.iter().all(|interval| interval.arc.is_valid()));
    }
}

#[cfg(test)]
mod exact_arc_projection_tests {
    use super::projected_arc_extent;
    use crate::capillary_geometry::CircularArcBoundary;

    #[test]
    fn exact_arc_projection_matches_analytic_semicircle_and_quarter_arc() {
        let semicircle = CircularArcBoundary {
            center_x: 0.0,
            center_y: 0.0,
            radius: 2.0,
            start_angle_radians: -std::f64::consts::FRAC_PI_2,
            sweep_radians: std::f64::consts::PI,
        };
        // For a right-facing semicircle, integral of max(cos(theta), 0) is 2.
        assert!((projected_arc_extent(semicircle, (1.0, 0.0)).unwrap() - 4.0).abs() < 1e-12);

        let quarter = CircularArcBoundary {
            start_angle_radians: 0.0,
            sweep_radians: std::f64::consts::FRAC_PI_2,
            ..semicircle
        };
        // Integral of cos(theta) from 0 to pi/2 is 1.
        assert!((projected_arc_extent(quarter, (1.0, 0.0)).unwrap() - 2.0).abs() < 1e-12);
    }

    #[test]
    fn exact_arc_projection_normalizes_large_finite_start_angles() {
        let arc = CircularArcBoundary {
            center_x: 0.0,
            center_y: 0.0,
            radius: 2.0,
            start_angle_radians: 1.0e100,
            sweep_radians: std::f64::consts::FRAC_PI_2,
        };
        let extent = projected_arc_extent(arc, (1.0, 0.0))
            .expect("finite large start angles must not overflow projection bounds");
        assert!(extent.is_finite() && extent >= 0.0);
    }

    #[test]
    fn exact_arc_projection_clips_back_facing_arc_and_rejects_invalid_direction() {
        let arc = CircularArcBoundary {
            center_x: 0.0,
            center_y: 0.0,
            radius: 3.0,
            start_angle_radians: std::f64::consts::FRAC_PI_2,
            sweep_radians: std::f64::consts::FRAC_PI_2,
        };
        assert_eq!(projected_arc_extent(arc, (1.0, 0.0)), Some(0.0));
        assert_eq!(projected_arc_extent(arc, (0.0, 0.0)), None);
        assert_eq!(projected_arc_extent(arc, (0.0, 2.0)), None);
    }
}

/// Quantize derived visibility coordinates to the approved 0.01 world-unit
/// grid. Geometry definitions themselves remain full precision.
fn round_geometry_to_cent(value: f64) -> Option<f64> {
    if !value.is_finite() {
        return None;
    }
    let rounded = (value * 100.0).round() / 100.0;
    rounded.is_finite().then_some(rounded)
}

/// Sorted shared lane partition for a single parallel-ray frame.
///
/// Straight-boundary events retain their calculated coordinates. Callers round
/// curved-boundary event lanes with `round_geometry_to_cent` before insertion;
/// the partition itself does not silently change straight geometry.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ParallelLanePartition {
    lanes: Vec<f64>,
}

impl ParallelLanePartition {
    /// Build the shared partition from straight and curved critical events.
    /// Curved-event lane coordinates are quantized to 0.01 world units as
    /// approved; straight-edge coordinates retain full precision. All events
    /// then share one sorted partition, preventing either class from being
    /// classified against a different lane set.
    #[allow(dead_code)]
    pub(crate) fn from_boundary_events(
        straight_lanes: Vec<f64>,
        curved_lanes: Vec<f64>,
    ) -> Option<Self> {
        let mut lanes = straight_lanes;
        for lane in curved_lanes {
            lanes.push(round_geometry_to_cent(lane)?);
        }
        Self::new(lanes)
    }

    /// Sort and deduplicate finite lane coordinates. A small epsilon removes
    /// numerical duplicates from the same geometric event without imposing a
    /// coarse grid on straight boundaries.
    #[allow(dead_code)]
    pub(crate) fn new(mut lanes: Vec<f64>) -> Option<Self> {
        if lanes.iter().any(|lane| !lane.is_finite()) {
            return None;
        }
        lanes.sort_by(f64::total_cmp);
        let mut unique: Vec<f64> = Vec::with_capacity(lanes.len());
        for lane in lanes {
            if unique
                .last()
                .map_or(true, |previous| (lane - *previous).abs() > 1e-9)
            {
                unique.push(lane);
            }
        }
        Some(Self { lanes: unique })
    }

    #[allow(dead_code)]
    pub(crate) fn lanes(&self) -> &[f64] {
        &self.lanes
    }

    /// Return open lane intervals and one representative midpoint per interval.
    /// Intervals with zero or non-finite width are excluded.
    #[allow(dead_code)]
    pub(crate) fn intervals(&self) -> Vec<(f64, f64, f64)> {
        self.lanes
            .windows(2)
            .filter_map(|pair| {
                let width = pair[1] - pair[0];
                let midpoint = pair[0] + width * 0.5;
                (width.is_finite()
                    && width > 1e-9
                    && midpoint.is_finite()
                    && midpoint > pair[0]
                    && midpoint < pair[1])
                    .then_some((pair[0], pair[1], midpoint))
            })
            .collect()
    }
}

/// Coordinate frame for a single shared parallel-ray direction.
///
/// `toward_source` points from the scene toward the source. Rays travel in the
/// opposite direction, but the perpendicular lane coordinate is unchanged by
/// that sign choice. Construct once per capture frame and reuse for every
/// receiver and occluder.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ParallelRayFrame {
    toward_source: (f64, f64),
    perpendicular: (f64, f64),
}

impl ParallelRayFrame {
    /// Normalize a finite nonzero direction. Invalid directions fail closed.
    #[allow(dead_code)] // Consumed by the parallel visibility partitioner.
    pub(crate) fn new(toward_source: (f64, f64)) -> Option<Self> {
        if !toward_source.0.is_finite() || !toward_source.1.is_finite() {
            return None;
        }
        let length = toward_source.0.hypot(toward_source.1);
        if !length.is_finite() || length <= 1e-12 {
            return None;
        }
        let direction = (toward_source.0 / length, toward_source.1 / length);
        Some(Self {
            toward_source: direction,
            perpendicular: (-direction.1, direction.0),
        })
    }

    /// Lane coordinate shared by all points on one parallel ray.
    #[allow(dead_code)]
    pub(crate) fn lane(self, point: (f64, f64)) -> Option<f64> {
        if !point.0.is_finite() || !point.1.is_finite() {
            return None;
        }
        let lane = point.0 * self.perpendicular.0 + point.1 * self.perpendicular.1;
        lane.is_finite().then_some(lane)
    }

    /// Signed depth toward the source. Smaller/larger ordering must be
    /// interpreted consistently by the visibility classifier.
    #[allow(dead_code)]
    pub(crate) fn depth(self, point: (f64, f64)) -> Option<f64> {
        if !point.0.is_finite() || !point.1.is_finite() {
            return None;
        }
        let depth = point.0 * self.toward_source.0 + point.1 * self.toward_source.1;
        depth.is_finite().then_some(depth)
    }

    /// Return the two tangent lane coordinates for a circular boundary.
    ///
    /// These are the exact extrema of a circle under parallel projection. Only
    /// the derived lane events are quantized; center and radius remain unchanged.
    #[allow(dead_code)]
    pub(crate) fn circle_critical_lanes(
        self,
        center: (f64, f64),
        radius: f64,
    ) -> Option<(f64, f64)> {
        if !center.0.is_finite() || !center.1.is_finite() || !radius.is_finite() || radius < 0.0 {
            return None;
        }
        let center_lane = self.lane(center)?;
        let near = round_geometry_to_cent(center_lane - radius)?;
        let far = round_geometry_to_cent(center_lane + radius)?;
        Some((near.min(far), near.max(far)))
    }

    /// Find the lane coordinate where two finite straight segments intersect.
    ///
    /// These intersection lanes are critical events: the front-to-back order of
    /// two boundaries can change only where their segments cross. A proper miss
    /// returns Some(None); invalid coordinates and collinear overlap fail closed.
    #[allow(dead_code)]
    pub(crate) fn segment_intersection_lane(
        self,
        a: (f64, f64),
        b: (f64, f64),
        c: (f64, f64),
        d: (f64, f64),
    ) -> Option<Option<f64>> {
        let points = [a, b, c, d];
        if points
            .iter()
            .any(|point| !point.0.is_finite() || !point.1.is_finite())
        {
            return None;
        }
        let r = (b.0 - a.0, b.1 - a.1);
        let q = (d.0 - c.0, d.1 - c.1);
        let offset = (c.0 - a.0, c.1 - a.1);
        let cross = |left: (f64, f64), right: (f64, f64)| left.0 * right.1 - left.1 * right.0;
        let denominator = cross(r, q);
        let numerator_t = cross(offset, q);
        let numerator_u = cross(offset, r);
        if !denominator.is_finite() || !numerator_t.is_finite() || !numerator_u.is_finite() {
            return None;
        }
        if denominator.abs() <= 1e-12 {
            // Parallel non-collinear segments never exchange depth order. For
            // collinear segments, reject only a positive-length overlap; merely
            // sharing a line is common for disjoint polygon edges and must not
            // disable the entire scene.
            if numerator_t.abs() > 1e-12 {
                return Some(None);
            }
            let length_squared = r.0 * r.0 + r.1 * r.1;
            if !length_squared.is_finite() || length_squared <= 1e-24 {
                return None;
            }
            let t0 = (offset.0 * r.0 + offset.1 * r.1) / length_squared;
            let t1 = t0 + (q.0 * r.0 + q.1 * r.1) / length_squared;
            if !t0.is_finite() || !t1.is_finite() {
                return None;
            }
            let overlap_start = t0.min(t1).max(0.0);
            let overlap_end = t0.max(t1).min(1.0);
            if overlap_end - overlap_start > 1e-12 {
                return None;
            }
            if (overlap_end - overlap_start).abs() <= 1e-12
                && (-1e-12..=1.0 + 1e-12).contains(&overlap_start)
            {
                let point = (
                    a.0 + overlap_start.clamp(0.0, 1.0) * r.0,
                    a.1 + overlap_start.clamp(0.0, 1.0) * r.1,
                );
                return self.lane(point).map(Some);
            }
            return Some(None);
        }
        let t = numerator_t / denominator;
        let u = numerator_u / denominator;
        if !t.is_finite() || !u.is_finite() {
            return None;
        }
        if !(0.0..=1.0).contains(&t) || !(0.0..=1.0).contains(&u) {
            return Some(None);
        }
        let point = (a.0 + t * r.0, a.1 + t * r.1);
        self.lane(point).map(Some)
    }

    /// Intersect one parallel lane with a full circle and return its two hit depths.
    ///
    /// The nested option distinguishes a valid miss from invalid input. Arc
    /// receivers can filter these exact hits by their angular interval; no
    /// curve tessellation is needed. Tangency returns the same depth twice.
    #[allow(dead_code)] // Consumed by the parallel visibility partitioner.
    pub(crate) fn circle_hit_depths(
        self,
        lane: f64,
        center: (f64, f64),
        radius: f64,
    ) -> Option<Option<(f64, f64)>> {
        if !lane.is_finite()
            || !center.0.is_finite()
            || !center.1.is_finite()
            || !radius.is_finite()
            || radius < 0.0
        {
            return None;
        }
        let center_lane = self.lane(center)?;
        let center_depth = self.depth(center)?;
        let lane_offset = lane - center_lane;
        if !lane_offset.is_finite() {
            return None;
        }
        let radius_squared = radius * radius;
        let offset_squared = lane_offset * lane_offset;
        if !radius_squared.is_finite() || !offset_squared.is_finite() {
            return None;
        }
        let discriminant = radius_squared - offset_squared;
        if discriminant < 0.0 {
            return Some(None);
        }
        let half_chord = discriminant.sqrt();
        let near = center_depth - half_chord;
        let far = center_depth + half_chord;
        if !near.is_finite() || !far.is_finite() {
            return None;
        }
        // Curved-boundary visibility intentionally uses a 0.01 world-unit
        // depth grid. Quantize only the derived hit depths, not the stored
        // circle geometry, so material dimensions and energy properties remain
        // untouched while the lane partition stays simple and deterministic.
        let near = round_geometry_to_cent(near)?;
        let far = round_geometry_to_cent(far)?;
        Some(Some((near.min(far), near.max(far))))
    }

    /// Intersect a lane with a straight segment and return the hit depth.
    ///
    /// The nested option distinguishes a valid miss (`Some(None)`) from
    /// invalid or coincident geometry (`None`). A segment lying along the ray
    /// is ambiguous for nearest-boundary attribution and therefore fails closed.
    #[allow(dead_code)]
    pub(crate) fn segment_hit_depth(
        self,
        lane: f64,
        start: (f64, f64),
        end: (f64, f64),
    ) -> Option<Option<f64>> {
        if !lane.is_finite()
            || !start.0.is_finite()
            || !start.1.is_finite()
            || !end.0.is_finite()
            || !end.1.is_finite()
        {
            return None;
        }
        let lane_start = self.lane(start)?;
        let lane_end = self.lane(end)?;
        let depth_start = self.depth(start)?;
        let depth_end = self.depth(end)?;
        let lane_delta = lane_end - lane_start;
        if !lane_delta.is_finite() {
            return None;
        }
        if lane_delta.abs() <= 1e-12 {
            // A segment parallel to the ray either misses this lane or overlaps
            // it for a continuum of depths; the latter cannot be attributed as
            // one unique hit.
            return if (lane - lane_start).abs() <= 1e-12 {
                None
            } else {
                Some(None)
            };
        }
        let t = (lane - lane_start) / lane_delta;
        if !t.is_finite() {
            return None;
        }
        if !(0.0..=1.0).contains(&t) {
            return Some(None);
        }
        let depth = depth_start + (depth_end - depth_start) * t;
        depth.is_finite().then_some(Some(depth))
    }
}

#[cfg(test)]
mod parallel_ray_frame_tests {
    use super::ParallelRayFrame;

    #[test]
    fn circle_tangent_lanes_are_quantized_without_mutating_geometry() {
        let frame = ParallelRayFrame::new((1.0, 0.0)).unwrap();
        assert_eq!(
            frame.circle_critical_lanes((4.0, 1.234), 0.456),
            Some((0.78, 1.69))
        );
    }

    #[test]
    fn circle_tangent_lanes_reject_invalid_geometry() {
        let frame = ParallelRayFrame::new((1.0, 0.0)).unwrap();
        assert_eq!(frame.circle_critical_lanes((0.0, 0.0), -1.0), None);
        assert_eq!(frame.circle_critical_lanes((f64::NAN, 0.0), 1.0), None);
    }

    #[test]
    fn crossing_segments_produce_the_shared_critical_lane() {
        let frame = ParallelRayFrame::new((1.0, 0.0)).unwrap();
        let lane = frame
            .segment_intersection_lane((0.0, 0.0), (2.0, 2.0), (0.0, 2.0), (2.0, 0.0))
            .unwrap()
            .unwrap();
        assert!((lane - 1.0).abs() < 1e-12);
    }

    #[test]
    fn disjoint_parallel_segments_have_no_intersection_event() {
        let frame = ParallelRayFrame::new((1.0, 0.0)).unwrap();
        assert_eq!(
            frame.segment_intersection_lane((0.0, 0.0), (1.0, 0.0), (0.0, 2.0), (1.0, 2.0)),
            Some(None)
        );
    }

    #[test]
    fn collinear_overlap_fails_closed() {
        let frame = ParallelRayFrame::new((1.0, 0.0)).unwrap();
        assert_eq!(
            frame.segment_intersection_lane((0.0, 0.0), (2.0, 0.0), (1.0, 0.0), (3.0, 0.0)),
            None
        );
    }

    #[test]
    fn intersection_outside_finite_segments_is_not_a_critical_event() {
        let frame = ParallelRayFrame::new((1.0, 0.0)).unwrap();
        assert_eq!(
            frame.segment_intersection_lane((0.0, 0.0), (1.0, 1.0), (2.0, 0.0), (3.0, 1.0)),
            Some(None)
        );
    }

    #[test]
    fn lane_is_invariant_under_translation_along_parallel_rays() {
        let frame = ParallelRayFrame::new((0.0, 4.0)).unwrap();
        let point = (3.0, 7.0);
        let translated = (point.0, point.1 + 125.0);
        assert!((frame.lane(point).unwrap() - frame.lane(translated).unwrap()).abs() < 1e-12);
    }

    #[test]
    fn segment_hit_depth_changes_but_lane_does_not_under_ray_translation() {
        let frame = ParallelRayFrame::new((1.0, 0.0)).unwrap();
        let start = (2.0, -1.0);
        let end = (2.0, 1.0);
        let lane = frame.lane((0.0, 0.0)).unwrap();
        let depth = frame.segment_hit_depth(lane, start, end).unwrap().unwrap();
        let shift = (25.0, 0.0);
        let shifted_start = (start.0 + shift.0, start.1 + shift.1);
        let shifted_end = (end.0 + shift.0, end.1 + shift.1);
        assert_eq!(frame.lane(start), frame.lane(shifted_start));
        let shifted_depth = frame
            .segment_hit_depth(lane, shifted_start, shifted_end)
            .unwrap()
            .unwrap();
        assert!((shifted_depth - depth - 25.0).abs() < 1e-12);
    }

    #[test]
    fn segment_lane_intersection_distinguishes_miss_and_ambiguous_overlap() {
        let frame = ParallelRayFrame::new((1.0, 0.0)).unwrap();
        assert_eq!(
            frame.segment_hit_depth(5.0, (2.0, 0.0), (3.0, 0.0)),
            Some(None)
        );
        assert_eq!(frame.segment_hit_depth(0.0, (2.0, 0.0), (3.0, 0.0)), None);
    }

    #[test]
    fn rejects_invalid_direction_and_non_finite_coordinates() {
        assert!(ParallelRayFrame::new((0.0, 0.0)).is_none());
        assert!(ParallelRayFrame::new((f64::NAN, 1.0)).is_none());
        let frame = ParallelRayFrame::new((1.0, 0.0)).unwrap();
        assert!(frame.lane((f64::INFINITY, 0.0)).is_none());
        assert!(frame
            .segment_hit_depth(0.0, (0.0, 0.0), (f64::NAN, 1.0))
            .is_none());
    }
}

#[cfg(test)]
mod parallel_circle_lane_tests {
    use super::ParallelRayFrame;

    #[test]
    fn circle_lane_intersection_handles_two_hits_tangent_and_miss() {
        let frame = ParallelRayFrame::new((1.0, 0.0)).unwrap();
        assert_eq!(
            frame.circle_hit_depths(0.0, (5.0, 0.0), 2.0),
            Some(Some((3.0, 7.0)))
        );
        assert_eq!(
            frame.circle_hit_depths(2.0, (5.0, 0.0), 2.0),
            Some(Some((5.0, 5.0)))
        );
        assert_eq!(frame.circle_hit_depths(2.1, (5.0, 0.0), 2.0), Some(None));
    }

    #[test]
    fn circle_lane_hits_are_invariant_in_lane_under_parallel_translation() {
        let frame = ParallelRayFrame::new((1.0, 0.0)).unwrap();
        let original = frame
            .circle_hit_depths(1.0, (5.0, 0.0), 2.0)
            .unwrap()
            .unwrap();
        let translated = frame
            .circle_hit_depths(1.0, (105.0, 0.0), 2.0)
            .unwrap()
            .unwrap();
        assert!((translated.0 - original.0 - 100.0).abs() < 1e-12);
        assert!((translated.1 - original.1 - 100.0).abs() < 1e-12);
    }

    #[test]
    fn circle_lane_intersection_rejects_invalid_geometry() {
        let frame = ParallelRayFrame::new((1.0, 0.0)).unwrap();
        assert!(frame.circle_hit_depths(0.0, (0.0, 0.0), -1.0).is_none());
        assert!(frame.circle_hit_depths(f64::NAN, (0.0, 0.0), 1.0).is_none());
    }
}

#[cfg(test)]
mod curved_visibility_precision_tests {
    use super::{round_geometry_to_cent, ParallelRayFrame};

    #[test]
    fn curved_boundary_hit_depths_round_to_nearest_hundredth() {
        assert_eq!(round_geometry_to_cent(1.234), Some(1.23));
        assert_eq!(round_geometry_to_cent(1.235), Some(1.24));
        assert_eq!(round_geometry_to_cent(-1.235), Some(-1.24));

        let frame = ParallelRayFrame::new((1.0, 0.0)).unwrap();
        let hits = frame
            .circle_hit_depths(0.0, (5.123, 0.0), 2.0)
            .unwrap()
            .unwrap();
        assert_eq!(hits, (3.12, 7.12));
    }

    #[test]
    fn curved_visibility_precision_rejects_non_finite_values() {
        assert_eq!(round_geometry_to_cent(f64::INFINITY), None);
        assert_eq!(round_geometry_to_cent(f64::NAN), None);
    }
}

#[cfg(test)]
mod parallel_lane_partition_tests {
    use super::ParallelLanePartition;

    #[test]
    fn partition_sorts_and_deduplicates_critical_lanes() {
        let partition = ParallelLanePartition::new(vec![2.0, 0.0, 1.0, 1.0 + 1e-10]).unwrap();
        assert_eq!(partition.lanes(), &[0.0, 1.0, 2.0]);
        assert_eq!(
            partition.intervals(),
            vec![(0.0, 1.0, 0.5), (1.0, 2.0, 1.5)]
        );
    }

    #[test]
    fn partition_rejects_non_finite_events_and_skips_degenerate_intervals() {
        assert!(ParallelLanePartition::new(vec![0.0, f64::NAN]).is_none());
        let partition = ParallelLanePartition::new(vec![0.0, 1e-10, 1.0]).unwrap();
        assert_eq!(partition.intervals(), vec![(0.0, 1.0, 0.5)]);
    }

    #[test]
    fn curved_events_round_before_joining_unrounded_straight_events() {
        let partition =
            ParallelLanePartition::from_boundary_events(vec![0.2345, 0.3], vec![0.1236, 0.876])
                .unwrap();
        assert_eq!(partition.lanes(), &[0.12, 0.2345, 0.3, 0.88]);
    }

    #[test]
    fn quantized_curved_events_merge_when_they_fall_in_the_same_cent() {
        let partition =
            ParallelLanePartition::from_boundary_events(vec![0.0, 1.0], vec![0.401, 0.404, 0.406])
                .unwrap();
        assert_eq!(partition.lanes(), &[0.0, 0.4, 0.41, 1.0]);
        assert!(partition
            .intervals()
            .iter()
            .all(|(_, _, midpoint)| midpoint.is_finite()));
    }

    #[test]
    fn curved_event_quantization_rejects_non_finite_lanes() {
        assert!(
            ParallelLanePartition::from_boundary_events(vec![0.0], vec![f64::INFINITY],).is_none()
        );
    }
}

/// Classify straight polygonal boundaries against one shared parallel-ray field.
///
/// Every polygon vertex and every finite segment crossing contributes a lane
/// event. Between consecutive events, boundary order cannot change, so the
/// midpoint ray identifies the unique nearest physical boundary for that lane
/// interval. Only winning, source-facing intervals belonging to requested
/// receiver bodies are returned. Curved forms deliberately fail closed here;
/// they must be added to this same partition/classification path, not handled
/// by a second independently credited exposure calculation.
pub(crate) fn visible_parallel_polygonal_intervals_for_targets(
    toward_source: (f64, f64),
    bodies: &[crate::organism_geometry::OrganismBodyGeometry],
    target_body_indices: &[usize],
) -> Option<Vec<ExposedBoundaryInterval>> {
    use crate::resources::Form;

    let frame = ParallelRayFrame::new(toward_source)?;
    let mut target_mask = vec![false; bodies.len()];
    for &index in target_body_indices {
        *target_mask.get_mut(index)? = true;
    }

    #[derive(Clone, Copy)]
    struct Edge {
        body_index: usize,
        unit_index: usize,
        start: (f64, f64),
        end: (f64, f64),
        lane_start: f64,
        lane_end: f64,
        depth_start: f64,
        depth_end: f64,
        outward_normal: Option<(f64, f64)>,
    }

    let mut edges = Vec::<Edge>::new();
    let mut lanes = Vec::<f64>::new();
    for (body_index, body) in bodies.iter().enumerate() {
        let mut seen_units = Vec::with_capacity(body.parts.len());
        for part in &body.parts {
            if seen_units.contains(&part.unit_index)
                || !part.x.is_finite()
                || !part.y.is_finite()
                || !part.rotation_radians.is_finite()
                || !part.form.is_valid()
            {
                return None;
            }
            seen_units.push(part.unit_index);
            let (sin, cos) = part.rotation_radians.sin_cos();
            let local_vertices = match &part.form {
                Form::Line { length } => {
                    let half = *length * 0.5;
                    vec![(-half, 0.0), (half, 0.0)]
                }
                Form::Fluid {
                    boundary: Some(vertices),
                    arc_boundary: None,
                    multi_contact_boundary: None,
                    ..
                } => vertices.clone(),
                Form::Fluid { .. } => return None,
                _ => part.form.polygon_vertices()?,
            };
            if local_vertices.len() < 2 {
                return None;
            }
            let vertices: Vec<_> = local_vertices
                .into_iter()
                .map(|(x, y)| (part.x + x * cos - y * sin, part.y + x * sin + y * cos))
                .collect();
            if vertices
                .iter()
                .any(|p| !p.0.is_finite() || !p.1.is_finite())
            {
                return None;
            }

            let is_line = matches!(&part.form, Form::Line { .. });
            let winding = if is_line {
                0.0
            } else {
                if vertices.len() < 3 {
                    return None;
                }
                let area_twice: f64 = vertices
                    .iter()
                    .enumerate()
                    .map(|(i, &(x, y))| {
                        let (nx, ny) = vertices[(i + 1) % vertices.len()];
                        x * ny - nx * y
                    })
                    .sum();
                if !area_twice.is_finite() || area_twice.abs() <= 1e-12 {
                    return None;
                }
                area_twice.signum()
            };
            let edge_count = if is_line { 1 } else { vertices.len() };
            for edge_index in 0..edge_count {
                let start = vertices[edge_index];
                let end = vertices[(edge_index + 1) % vertices.len()];
                let length = (end.0 - start.0).hypot(end.1 - start.1);
                if !length.is_finite() || length <= 1e-12 {
                    return None;
                }
                let lane_start = frame.lane(start)?;
                let lane_end = frame.lane(end)?;
                let depth_start = frame.depth(start)?;
                let depth_end = frame.depth(end)?;
                lanes.extend([lane_start, lane_end]);
                let outward_normal = if is_line {
                    None
                } else {
                    Some((
                        winding * (end.1 - start.1) / length,
                        -winding * (end.0 - start.0) / length,
                    ))
                };
                edges.push(Edge {
                    body_index,
                    unit_index: part.unit_index,
                    start,
                    end,
                    lane_start,
                    lane_end,
                    depth_start,
                    depth_end,
                    outward_normal,
                });
            }
        }
    }

    // Crossings are the only places where two straight boundaries can exchange
    // front-to-back order. Collinear overlaps are ambiguous and fail closed.
    for left in 0..edges.len() {
        for right in (left + 1)..edges.len() {
            let event = frame.segment_intersection_lane(
                edges[left].start,
                edges[left].end,
                edges[right].start,
                edges[right].end,
            )?;
            if let Some(lane) = event {
                lanes.push(lane);
            }
        }
    }

    let partition = ParallelLanePartition::new(lanes)?;
    let mut visible = Vec::new();
    // Emit intervals by walking the shared lane partition again. The winning
    // edge is constant within each open interval by construction.
    for pair in partition.lanes().windows(2) {
        let lo = pair[0];
        let hi = pair[1];
        if hi - lo <= 1e-9 {
            continue;
        }
        let lane = lo + (hi - lo) * 0.5;
        let mut nearest: Option<(usize, f64)> = None;
        let mut tied = false;
        for (edge_index, edge) in edges.iter().enumerate() {
            let lane_delta = edge.lane_end - edge.lane_start;
            if lane_delta.abs() <= 1e-12 {
                continue;
            }
            if lane <= edge.lane_start.min(edge.lane_end)
                || lane >= edge.lane_start.max(edge.lane_end)
            {
                continue;
            }
            let t = (lane - edge.lane_start) / lane_delta;
            let depth = edge.depth_start + (edge.depth_end - edge.depth_start) * t;
            if !depth.is_finite() {
                return None;
            }
            match nearest {
                None => {
                    nearest = Some((edge_index, depth));
                    tied = false;
                }
                Some((_, best)) if depth > best + 1e-9 => {
                    nearest = Some((edge_index, depth));
                    tied = false;
                }
                Some((_, best)) if (depth - best).abs() <= 1e-9 => tied = true,
                _ => {}
            }
        }
        if tied {
            continue;
        }
        let Some((edge_index, _)) = nearest else {
            continue;
        };
        let edge = edges[edge_index];
        if !target_mask[edge.body_index] {
            continue;
        }
        let Some(normal) = edge.outward_normal else {
            continue;
        };
        if normal.0 * frame.toward_source.0 + normal.1 * frame.toward_source.1 <= 1e-10 {
            continue;
        }
        let delta = edge.lane_end - edge.lane_start;
        if delta.abs() <= 1e-12 {
            continue;
        }
        let t0 = (lo - edge.lane_start) / delta;
        let t1 = (hi - edge.lane_start) / delta;
        if t0 < -1e-9 || t0 > 1.0 + 1e-9 || t1 < -1e-9 || t1 > 1.0 + 1e-9 {
            return None;
        }
        let point_at = |t: f64| {
            (
                edge.start.0 + (edge.end.0 - edge.start.0) * t,
                edge.start.1 + (edge.end.1 - edge.start.1) * t,
            )
        };
        visible.push(ExposedBoundaryInterval {
            body_index: edge.body_index,
            unit_index: edge.unit_index,
            start: point_at(t0.clamp(0.0, 1.0)),
            end: point_at(t1.clamp(0.0, 1.0)),
            outward_normal: normal,
        });
    }
    Some(visible)
}

/// Compute the exact parallel-projection width credited to each target constituent.
/// Straight/curved boundaries share one lane partition and one nearest-surface test;
/// curved event lanes and circle hit depths are quantized to the approved 0.01 grid.
type ParallelExposureWidths = Vec<Vec<(usize, f64)>>;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ParallelRayInteractionPath {
    /// Width of this lane interval; energy is intensity multiplied by this width.
    pub(crate) projected_width: f64,
    /// Material constituents encountered in source-to-downstream order.
    pub(crate) hits: Vec<(usize, usize)>,
}

pub(crate) fn parallel_projected_widths_by_body_for_targets(
    toward_source: (f64, f64),
    bodies: &[crate::organism_geometry::OrganismBodyGeometry],
    target_body_indices: &[usize],
) -> Option<ParallelExposureWidths> {
    Some(parallel_exposure_with_paths_for_targets(toward_source, bodies, target_body_indices)?.0)
}

// Dense worlds use boundary-event lane intervals and midpoint depth ordering
// instead of the quadratic exact intersection-event pass. This preserves one
// energy-conserving ordered path per represented lane interval while allowing
// small/local geometry to retain exact crossing splits.
const EXACT_INTERSECTION_BODY_LIMIT: usize = 256;

pub(crate) fn parallel_exposure_with_paths_for_targets(
    toward_source: (f64, f64),
    bodies: &[crate::organism_geometry::OrganismBodyGeometry],
    target_body_indices: &[usize],
) -> Option<(ParallelExposureWidths, Vec<ParallelRayInteractionPath>)> {
    parallel_exposure_with_exact_intersections(
        toward_source,
        bodies,
        target_body_indices,
        bodies.len() <= EXACT_INTERSECTION_BODY_LIMIT,
    )
}

fn parallel_exposure_with_exact_intersections(
    toward_source: (f64, f64),
    bodies: &[crate::organism_geometry::OrganismBodyGeometry],
    target_body_indices: &[usize],
    exact_intersections: bool,
) -> Option<(ParallelExposureWidths, Vec<ParallelRayInteractionPath>)> {
    use crate::capillary_geometry::FluidBoundaryPiece;
    use crate::resources::Form;

    let frame = ParallelRayFrame::new(toward_source)?;
    let mut target_mask = vec![false; bodies.len()];
    for &index in target_body_indices {
        *target_mask.get_mut(index)? = true;
    }

    #[derive(Clone, Copy)]
    struct Segment {
        body_index: usize,
        unit_index: usize,
        start: (f64, f64),
        end: (f64, f64),
        lane_start: f64,
        lane_end: f64,
        depth_start: f64,
        depth_end: f64,
        normal: Option<(f64, f64)>,
        creditable: bool,
    }
    #[derive(Clone, Copy)]
    enum Surface {
        Segment(usize),
        Arc(usize, f64),
    }

    type LocalEdge = ((f64, f64), (f64, f64));
    let mut segments = Vec::<Segment>::new();
    let mut arcs = Vec::<WorldCircleArc>::new();
    let mut straight_lanes = Vec::<f64>::new();
    let mut curved_lanes = Vec::<f64>::new();
    let mut widths: Vec<Vec<(usize, f64)>> = bodies
        .iter()
        .map(|body| {
            body.parts
                .iter()
                .map(|part| (part.unit_index, 0.0))
                .collect()
        })
        .collect();

    for (body_index, body) in bodies.iter().enumerate() {
        let mut seen_units = Vec::with_capacity(body.parts.len());
        for part in &body.parts {
            if seen_units.contains(&part.unit_index)
                || !part.x.is_finite()
                || !part.y.is_finite()
                || !part.rotation_radians.is_finite()
                || !part.form.is_valid()
            {
                return None;
            }
            seen_units.push(part.unit_index);
            let (sin, cos) = part.rotation_radians.sin_cos();
            let world = |point: (f64, f64)| {
                (
                    part.x + point.0 * cos - point.1 * sin,
                    part.y + point.0 * sin + point.1 * cos,
                )
            };

            let (local_edges, closed_surface): (Vec<LocalEdge>, bool) = match &part.form {
                Form::Line { length } => {
                    if !length.is_finite() || *length <= 0.0 {
                        return None;
                    }
                    let half = *length * 0.5;
                    (vec![((-half, 0.0), (half, 0.0))], false)
                }
                Form::Circle { .. } => (Vec::new(), false),
                Form::Fluid {
                    boundary: Some(vertices),
                    arc_boundary: None,
                    multi_contact_boundary: None,
                    ..
                } => {
                    if vertices.len() < 3 {
                        return None;
                    }
                    let edges = (0..vertices.len())
                        .map(|index| (vertices[index], vertices[(index + 1) % vertices.len()]))
                        .collect();
                    (edges, true)
                }
                Form::Fluid {
                    boundary: None,
                    arc_boundary: Some(arc),
                    multi_contact_boundary: None,
                    ..
                } => {
                    if !arc.is_valid() {
                        return None;
                    }
                    (vec![(arc.start_point(), arc.end_point())], false)
                }
                Form::Fluid {
                    boundary: None,
                    arc_boundary: None,
                    multi_contact_boundary: Some(multi),
                    ..
                } => {
                    let edges = multi
                        .pieces
                        .iter()
                        .filter_map(|piece| match piece {
                            FluidBoundaryPiece::SolidContact {
                                start_x,
                                start_y,
                                end_x,
                                end_y,
                                ..
                            } => Some(((*start_x, *start_y), (*end_x, *end_y))),
                            FluidBoundaryPiece::FreeArc(_) => None,
                        })
                        .collect();
                    (edges, false)
                }
                Form::Fluid {
                    boundary: None,
                    arc_boundary: None,
                    multi_contact_boundary: None,
                    ..
                } => (Vec::new(), false),
                Form::Fluid { .. } => return None,
                _ => {
                    let vertices = part.form.polygon_vertices()?;
                    if vertices.len() < 3 {
                        return None;
                    }
                    let edges = (0..vertices.len())
                        .map(|index| (vertices[index], vertices[(index + 1) % vertices.len()]))
                        .collect();
                    (edges, true)
                }
            };

            let vertices_for_winding = if closed_surface {
                let vertices = (match &part.form {
                    Form::Fluid {
                        boundary: Some(vertices),
                        ..
                    } => Some(vertices.clone()),
                    _ => part.form.polygon_vertices(),
                })?;
                let vertices: Vec<_> = vertices.into_iter().map(world).collect();
                let area_twice: f64 = vertices
                    .iter()
                    .enumerate()
                    .map(|(index, &(x, y))| {
                        let (nx, ny) = vertices[(index + 1) % vertices.len()];
                        x * ny - nx * y
                    })
                    .sum();
                if !area_twice.is_finite() || area_twice.abs() <= 1e-12 {
                    return None;
                }
                Some(area_twice.signum())
            } else {
                None
            };

            for (local_start, local_end) in local_edges {
                let start = world(local_start);
                let end = world(local_end);
                let length = (end.0 - start.0).hypot(end.1 - start.1);
                if !length.is_finite() || length <= 1e-12 {
                    return None;
                }
                let lane_start = frame.lane(start)?;
                let lane_end = frame.lane(end)?;
                let depth_start = frame.depth(start)?;
                let depth_end = frame.depth(end)?;
                straight_lanes.extend([lane_start, lane_end]);
                let normal = vertices_for_winding.map(|winding| {
                    (
                        winding * (end.1 - start.1) / length,
                        -winding * (end.0 - start.0) / length,
                    )
                });
                segments.push(Segment {
                    body_index,
                    unit_index: part.unit_index,
                    start,
                    end,
                    lane_start,
                    lane_end,
                    depth_start,
                    depth_end,
                    normal,
                    creditable: closed_surface,
                });
            }

            arcs.extend(realized_circle_arcs(body_index, part)?);
        }
    }

    let mut arc_lane_ranges = Vec::with_capacity(arcs.len());
    for arc in &arcs {
        let range = frame.circle_critical_lanes(arc.center, arc.radius)?;
        arc_lane_ranges.push(range);
        curved_lanes.extend([range.0, range.1]);
        for angle in [arc.start, arc.start + arc.sweep] {
            let point = (
                arc.center.0 + arc.radius * angle.cos(),
                arc.center.1 + arc.radius * angle.sin(),
            );
            curved_lanes.push(round_geometry_to_cent(frame.lane(point)?)?);
        }
    }

    // Sweep by lane extent instead of comparing every pair of boundaries.
    // Only intersections involving a receiver boundary can change whether that
    // receiver is exposed; blocker/blocker crossings cannot reveal a hidden
    // receiver without a receiver/blocker crossing of their own.
    let mut segment_order: Vec<usize> = (0..segments.len()).collect();
    segment_order.sort_by(|left, right| {
        let left = segments[*left].lane_start.min(segments[*left].lane_end);
        let right = segments[*right].lane_start.min(segments[*right].lane_end);
        left.total_cmp(&right)
    });
    if exact_intersections {
        for (position, &left_index) in segment_order.iter().enumerate() {
            let a = segments[left_index];
            let a_max = a.lane_start.max(a.lane_end);
            for &right_index in &segment_order[position + 1..] {
                let b = segments[right_index];
                let b_min = b.lane_start.min(b.lane_end);
                if b_min > a_max + 1e-9 {
                    break;
                }
                // Projected-lane overlap is necessary but not sufficient for an
                // intersection. Disjoint depth ranges prove these finite segments
                // cannot cross, avoiding an exact predicate for separated bodies.
                let a_depth_min = a.depth_start.min(a.depth_end);
                let a_depth_max = a.depth_start.max(a.depth_end);
                let b_depth_min = b.depth_start.min(b.depth_end);
                let b_depth_max = b.depth_start.max(b.depth_end);
                if a_depth_max < b_depth_min - 1e-9 || b_depth_max < a_depth_min - 1e-9 {
                    continue;
                }
                if !target_mask[a.body_index] && !target_mask[b.body_index] {
                    continue;
                }
                match frame.segment_intersection_lane(a.start, a.end, b.start, b.end) {
                    Some(Some(lane)) => straight_lanes.push(lane),
                    Some(None) => {}
                    None => {
                        // Shared collinear faces are common between touching pieces.
                        // Their overlapping lanes are resolved as a depth tie below.
                        let r = (a.end.0 - a.start.0, a.end.1 - a.start.1);
                        let q = (b.end.0 - b.start.0, b.end.1 - b.start.1);
                        let offset = (b.start.0 - a.start.0, b.start.1 - a.start.1);
                        let cross = |x: (f64, f64), y: (f64, f64)| x.0 * y.1 - x.1 * y.0;
                        let scale = r.0.hypot(r.1) * q.0.hypot(q.1);
                        if !scale.is_finite()
                            || cross(r, q).abs() > 1e-10 * scale.max(1.0)
                            || cross(offset, r).abs() > 1e-10 * r.0.hypot(r.1).max(1.0)
                        {
                            return None;
                        }
                    }
                }
            }
        }

        for (arc_index, arc) in arcs.iter().enumerate() {
            let (arc_min, arc_max) = arc_lane_ranges[arc_index];
            let upper = segment_order.partition_point(|segment_index| {
                segments[*segment_index]
                    .lane_start
                    .min(segments[*segment_index].lane_end)
                    <= arc_max + 1e-9
            });
            for &segment_index in &segment_order[..upper] {
                let segment = segments[segment_index];
                let segment_max = segment.lane_start.max(segment.lane_end);
                if segment_max < arc_min - 1e-9
                    || (!target_mask[arc.body_index] && !target_mask[segment.body_index])
                {
                    continue;
                }
                let segment_depth_min = segment.depth_start.min(segment.depth_end);
                let segment_depth_max = segment.depth_start.max(segment.depth_end);
                let arc_depth = frame.depth(arc.center)?;
                if segment_depth_max < arc_depth - arc.radius - 1e-9
                    || arc_depth + arc.radius < segment_depth_min - 1e-9
                {
                    continue;
                }
                let mut intersections = Vec::new();
                add_circle_segment_intersections(
                    arc.center,
                    arc.radius,
                    segment.start,
                    segment.end,
                    &mut intersections,
                )?;
                for point in intersections {
                    let angle = (point.1 - arc.center.1).atan2(point.0 - arc.center.0);
                    if arc_has_angle(*arc, angle) {
                        curved_lanes.push(round_geometry_to_cent(frame.lane(point)?)?);
                    }
                }
            }
        }

        let mut arc_order: Vec<usize> = (0..arcs.len()).collect();
        arc_order.sort_by(|left, right| {
            arc_lane_ranges[*left]
                .0
                .total_cmp(&arc_lane_ranges[*right].0)
        });
        for (position, &first_index) in arc_order.iter().enumerate() {
            let first = arcs[first_index];
            let first_range = arc_lane_ranges[first_index];
            for &second_index in &arc_order[position + 1..] {
                let second_range = arc_lane_ranges[second_index];
                if second_range.0 > first_range.1 + 1e-9 {
                    break;
                }
                let second = arcs[second_index];
                if !target_mask[first.body_index] && !target_mask[second.body_index] {
                    continue;
                }
                if (first.center.0 - second.center.0).abs() <= 1e-10
                    && (first.center.1 - second.center.1).abs() <= 1e-10
                    && (first.radius - second.radius).abs() <= 1e-10
                {
                    continue;
                }
                let mut intersections = Vec::with_capacity(2);
                add_circle_circle_intersections(
                    first.center,
                    first.radius,
                    second.center,
                    second.radius,
                    &mut intersections,
                )?;
                for point in intersections {
                    let first_angle = (point.1 - first.center.1).atan2(point.0 - first.center.0);
                    let second_angle = (point.1 - second.center.1).atan2(point.0 - second.center.0);
                    if arc_has_angle(first, first_angle) && arc_has_angle(second, second_angle) {
                        curved_lanes.push(round_geometry_to_cent(frame.lane(point)?)?);
                    }
                }
            }
        }
    } else {
        // Dense worlds use coarse lane intervals over occupied projected spans.
        // Preserve the exact bounds of sub-step surfaces so thin receivers are
        // not silently omitted, while avoiding thousands of redundant lane
        // partitions from every large body's vertices.
        const LANE_STEP: f64 = 0.25;
        let mut ranges: Vec<_> = segments
            .iter()
            .map(|segment| {
                (
                    segment.lane_start.min(segment.lane_end),
                    segment.lane_start.max(segment.lane_end),
                )
            })
            .chain(arc_lane_ranges.iter().copied())
            .collect();
        ranges.sort_by(|left, right| left.0.total_cmp(&right.0));

        let mut occupied_ranges: Vec<(f64, f64)> = Vec::new();
        for (lo, hi) in ranges {
            if let Some(last) = occupied_ranges.last_mut() {
                if lo <= last.1 + 1e-9 {
                    last.1 = last.1.max(hi);
                    continue;
                }
            }
            occupied_ranges.push((lo, hi));
        }

        let mut coarse_lanes = Vec::new();
        for (lo, hi) in occupied_ranges {
            coarse_lanes.push(round_geometry_to_cent(lo)?);
            let mut lane = lo + LANE_STEP;
            while lane < hi - 1e-9 {
                coarse_lanes.push(round_geometry_to_cent(lane)?);
                lane += LANE_STEP;
            }
            coarse_lanes.push(round_geometry_to_cent(hi)?);
        }
        // Preserve narrow projected surfaces even when their full extent falls
        // between regular grid samples. The approximation may blur larger
        // crossings, but does not intentionally discard thin material bounds.
        for segment in &segments {
            let lo = segment.lane_start.min(segment.lane_end);
            let hi = segment.lane_start.max(segment.lane_end);
            if hi - lo > 1e-9 && hi - lo <= LANE_STEP {
                coarse_lanes.extend([round_geometry_to_cent(lo)?, round_geometry_to_cent(hi)?]);
            }
        }
        for (lo, hi) in &arc_lane_ranges {
            if hi - lo <= LANE_STEP {
                coarse_lanes.extend([round_geometry_to_cent(*lo)?, round_geometry_to_cent(*hi)?]);
            }
        }
        straight_lanes = coarse_lanes;
        curved_lanes.clear();
    }

    let partition = ParallelLanePartition::from_boundary_events(straight_lanes, curved_lanes)?;
    // The lane partition is sorted. Sweep surface extents alongside it so each
    // lane tests only boundaries whose projected ranges can actually intersect
    // that lane, rather than rescanning every segment and arc for every interval.
    let mut active_segments = Vec::new();
    let mut segment_cursor = 0;
    let mut active_arcs = Vec::new();
    let mut arc_cursor = 0;
    let mut arc_start_order: Vec<usize> = (0..arcs.len()).collect();
    arc_start_order.sort_by(|left, right| {
        arc_lane_ranges[*left]
            .0
            .total_cmp(&arc_lane_ranges[*right].0)
    });
    let intervals = partition.intervals();
    let mut interaction_paths = Vec::new();
    for (lo, hi, lane) in intervals {
        while segment_cursor < segment_order.len() {
            let index = segment_order[segment_cursor];
            if segments[index].lane_start.min(segments[index].lane_end) >= lane {
                break;
            }
            active_segments.push(index);
            segment_cursor += 1;
        }
        active_segments
            .retain(|index| segments[*index].lane_start.max(segments[*index].lane_end) > lane);
        while arc_cursor < arc_start_order.len() {
            let index = arc_start_order[arc_cursor];
            if arc_lane_ranges[index].0 >= lane {
                break;
            }
            active_arcs.push(index);
            arc_cursor += 1;
        }
        active_arcs.retain(|index| arc_lane_ranges[*index].1 > lane);

        let mut nearest: Option<(f64, Surface)> = None;
        let mut lane_hits = Vec::new();
        let mut tied = false;
        let mut consider = |depth: f64, surface: Surface| -> Option<()> {
            if !depth.is_finite() {
                return None;
            }
            lane_hits.push((depth, surface));
            match nearest {
                None => {
                    nearest = Some((depth, surface));
                    tied = false;
                }
                Some((best, _)) if depth > best + 1e-9 => {
                    nearest = Some((depth, surface));
                    tied = false;
                }
                Some((best, _)) if (depth - best).abs() <= 1e-9 => tied = true,
                _ => {}
            }
            Some(())
        };

        for &index in &active_segments {
            let segment = segments[index];
            let delta = segment.lane_end - segment.lane_start;
            if delta.abs() <= 1e-12
                || lane <= segment.lane_start.min(segment.lane_end)
                || lane >= segment.lane_start.max(segment.lane_end)
            {
                continue;
            }
            let t = (lane - segment.lane_start) / delta;
            let depth = segment.depth_start + (segment.depth_end - segment.depth_start) * t;
            consider(depth, Surface::Segment(index))?;
        }

        for &index in &active_arcs {
            let arc = arcs[index];
            let Some((near, far)) = frame.circle_hit_depths(lane, arc.center, arc.radius)? else {
                continue;
            };
            for depth in [near, far] {
                let point = (
                    frame.toward_source.0 * depth + frame.perpendicular.0 * lane,
                    frame.toward_source.1 * depth + frame.perpendicular.1 * lane,
                );
                let angle = (point.1 - arc.center.1).atan2(point.0 - arc.center.0);
                if arc_has_angle(arc, angle) {
                    consider(depth, Surface::Arc(index, depth))?;
                }
            }
        }

        if tied {
            continue;
        }
        let Some((_, surface)) = nearest else {
            continue;
        };

        // Preserve the full ordered interaction path for this lane. Each
        // constituent is visited once, at its first source-facing boundary.
        lane_hits.sort_by(|left, right| right.0.total_cmp(&left.0));
        let mut path_hits = Vec::new();
        for (_, candidate) in &lane_hits {
            let (candidate_body, candidate_unit, candidate_facing) = match *candidate {
                Surface::Segment(index) => {
                    let segment = segments[index];
                    let facing = segment.creditable
                        && segment.normal.is_some_and(|normal| {
                            normal.0 * frame.toward_source.0 + normal.1 * frame.toward_source.1
                                > 1e-10
                        });
                    (segment.body_index, segment.unit_index, facing)
                }
                Surface::Arc(index, depth) => {
                    let arc = arcs[index];
                    let point = (
                        frame.toward_source.0 * depth + frame.perpendicular.0 * lane,
                        frame.toward_source.1 * depth + frame.perpendicular.1 * lane,
                    );
                    let facing = ((point.0 - arc.center.0) * frame.toward_source.0
                        + (point.1 - arc.center.1) * frame.toward_source.1)
                        / arc.radius
                        > 1e-10;
                    (arc.body_index, arc.unit_index, facing)
                }
            };
            if candidate_facing
                && !path_hits
                    .iter()
                    .any(|&(body, unit)| body == candidate_body && unit == candidate_unit)
            {
                path_hits.push((candidate_body, candidate_unit));
            }
        }
        let nearest_identity = match surface {
            Surface::Segment(index) => (segments[index].body_index, segments[index].unit_index),
            Surface::Arc(index, _) => (arcs[index].body_index, arcs[index].unit_index),
        };
        // Unsupported or ambiguous front surfaces stop this lane rather than
        // allowing energy to jump through geometry the material model cannot settle.
        if path_hits.first().copied() == Some(nearest_identity)
            && path_hits.iter().any(|(body, _)| target_mask[*body])
        {
            interaction_paths.push(ParallelRayInteractionPath {
                projected_width: hi - lo,
                hits: path_hits,
            });
        }

        let (body_index, unit_index, facing) = match surface {
            Surface::Segment(index) => {
                let segment = segments[index];
                let facing = segment.creditable
                    && segment.normal.is_some_and(|normal| {
                        normal.0 * frame.toward_source.0 + normal.1 * frame.toward_source.1 > 1e-10
                    });
                (segment.body_index, segment.unit_index, facing)
            }
            Surface::Arc(index, depth) => {
                let arc = arcs[index];
                let point = (
                    frame.toward_source.0 * depth + frame.perpendicular.0 * lane,
                    frame.toward_source.1 * depth + frame.perpendicular.1 * lane,
                );
                let facing = ((point.0 - arc.center.0) * frame.toward_source.0
                    + (point.1 - arc.center.1) * frame.toward_source.1)
                    / arc.radius
                    > 1e-10;
                (arc.body_index, arc.unit_index, facing)
            }
        };
        if !facing || !target_mask[body_index] {
            continue;
        }
        let (_, width) = widths
            .get_mut(body_index)?
            .iter_mut()
            .find(|(candidate, _)| *candidate == unit_index)?;
        *width += hi - lo;
    }

    if widths
        .iter()
        .flatten()
        .any(|(_, width)| !width.is_finite() || *width < 0.0)
    {
        return None;
    }
    Some((widths, interaction_paths))
}

#[cfg(test)]
mod parallel_mixed_boundary_visibility_tests {
    use super::{
        parallel_exposure_with_exact_intersections, parallel_exposure_with_paths_for_targets,
        parallel_projected_widths_by_body_for_targets, ParallelRayInteractionPath,
    };
    use crate::organism_geometry::{OrganismBodyGeometry, PlacedForm};
    use crate::resources::Form;

    fn circle(x: f64, y: f64, radius: f64) -> OrganismBodyGeometry {
        OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index: 0,
                form: Form::Circle { radius },
                x,
                y,
                rotation_radians: 0.0,
            }],
            min_x: x - radius,
            max_x: x + radius,
            min_y: y - radius,
            max_y: y + radius,
        }
    }

    fn rectangle(x: f64, y: f64, width: f64, height: f64) -> OrganismBodyGeometry {
        OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index: 0,
                form: Form::Rectangle { width, height },
                x,
                y,
                rotation_radians: 0.0,
            }],
            min_x: x - width * 0.5,
            max_x: x + width * 0.5,
            min_y: y - height * 0.5,
            max_y: y + height * 0.5,
        }
    }

    #[test]
    fn ordered_interaction_paths_reconcile_direct_exposure_and_retain_downstream_hits() {
        let bodies = [rectangle(4.0, 0.0, 2.0, 2.0), rectangle(8.0, 0.0, 2.0, 2.0)];
        let (widths, paths) =
            parallel_exposure_with_paths_for_targets((1.0, 0.0), &bodies, &[0, 1])
                .expect("valid aligned bodies should produce ordered ray paths");
        assert!(!paths.is_empty());
        assert!(paths.iter().any(|path| path.hits.len() >= 2));

        let mut first_hit_widths = vec![0.0_f64; bodies.len()];
        for path in &paths {
            let (first_body, _) = *path.hits.first().expect("path has a first hit");
            first_hit_widths[first_body] += path.projected_width;
        }
        for (body_index, body_widths) in widths.iter().enumerate() {
            let direct_width: f64 = body_widths.iter().map(|(_, width)| *width).sum();
            assert!(
                (first_hit_widths[body_index] - direct_width).abs() < 1e-7,
                "path first-hit width must equal direct exposure for body {body_index}"
            );
        }
    }

    #[test]
    fn fast_lane_sweep_bounds_small_world_visibility_error_to_lane_step() {
        let bodies = [
            rectangle(4.0, 0.0, 2.0, 2.0),
            circle(6.0, 0.35, 0.9),
            rectangle(8.0, -0.2, 1.5, 1.4),
        ];
        let targets = [0, 1, 2];
        let (exact_widths, exact_paths) =
            parallel_exposure_with_exact_intersections((1.0, 0.0), &bodies, &targets, true)
                .expect("exact small-world oracle should produce valid paths");
        let (fast_widths, fast_paths) =
            parallel_exposure_with_exact_intersections((1.0, 0.0), &bodies, &targets, false)
                .expect("fast lane sweep should produce valid paths");

        assert_eq!(exact_widths.len(), fast_widths.len());
        for (body_index, (exact, fast)) in exact_widths.iter().zip(&fast_widths).enumerate() {
            let exact_width: f64 = exact.iter().map(|(_, width)| *width).sum();
            let fast_width: f64 = fast.iter().map(|(_, width)| *width).sum();
            // The dense-world path intentionally samples at 0.25 lane
            // intervals. Narrow visibility slivers below that resolution may
            // move between receivers, but the absolute per-body error must stay
            // within one lane step; the source-width budget is checked below.
            assert!(
                (exact_width - fast_width).abs() <= 0.25,
                "fast lane sweep exceeds one lane step of projected-exposure error for body {body_index}: exact={exact_width}, fast={fast_width}"
            );
        }

        let total_path_width = |paths: &[ParallelRayInteractionPath]| {
            paths.iter().map(|path| path.projected_width).sum::<f64>()
        };
        assert!(
            (total_path_width(&exact_paths) - total_path_width(&fast_paths)).abs() <= 0.02,
            "fast lane sweep must conserve approximately the same source-width budget as the exact oracle"
        );
        for paths in [&exact_paths, &fast_paths] {
            assert!(paths.iter().all(|path| !path.hits.is_empty()));
            assert!(paths.iter().all(|path| {
                path.hits.iter().all(|(body, unit)| {
                    *body < bodies.len()
                        && bodies[*body]
                            .parts
                            .iter()
                            .any(|part| part.unit_index == *unit)
                })
            }));
        }
    }

    #[test]
    fn dense_world_fallback_keeps_ordered_hits_and_reconciles_first_hit_widths() {
        let bodies: Vec<_> = (0..=256)
            .map(|index| {
                let x = 4.0 + index as f64 * 2.0;
                OrganismBodyGeometry {
                    parts: vec![PlacedForm {
                        unit_index: 0,
                        form: Form::Rectangle {
                            width: 1.0,
                            height: 2.0,
                        },
                        x,
                        y: 0.0,
                        rotation_radians: 0.0,
                    }],
                    min_x: x - 0.5,
                    max_x: x + 0.5,
                    min_y: -1.0,
                    max_y: 1.0,
                }
            })
            .collect();
        let targets: Vec<_> = (0..bodies.len()).collect();
        let (widths, paths) =
            parallel_exposure_with_paths_for_targets((1.0, 0.0), &bodies, &targets)
                .expect("dense-world approximation should retain a valid path");
        assert!(
            paths.iter().any(|path| path.hits.len() == bodies.len()),
            "the fallback must preserve downstream hits through the ordered lane"
        );

        let mut first_hit_widths = vec![0.0_f64; bodies.len()];
        let mut path_width = 0.0;
        for path in &paths {
            let (first_body, _) = *path.hits.first().expect("path has a first hit");
            first_hit_widths[first_body] += path.projected_width;
            path_width += path.projected_width;
        }
        assert!((path_width - 2.0).abs() < 1e-7);
        for (body_index, body_widths) in widths.iter().enumerate() {
            let direct_width: f64 = body_widths.iter().map(|(_, width)| *width).sum();
            assert!(
                (first_hit_widths[body_index] - direct_width).abs() < 1e-7,
                "dense path first-hit width must reconcile for body {body_index}"
            );
        }
    }

    #[test]
    fn circular_receiver_is_shadowed_by_a_straight_occluder_in_the_same_parallel_field() {
        let bodies = [circle(4.0, 0.0, 1.0), rectangle(8.0, 0.0, 2.0, 1.0)];
        let widths =
            parallel_projected_widths_by_body_for_targets((1.0, 0.0), &bodies, &[0]).unwrap();
        assert!((widths[0][0].1 - 1.0).abs() < 0.02);
        assert_eq!(widths[1][0].1, 0.0);
    }

    #[test]
    fn polygonal_receiver_is_shadowed_by_a_circular_occluder_in_the_same_parallel_field() {
        let bodies = [rectangle(4.0, 0.0, 2.0, 2.0), circle(8.0, 0.0, 0.5)];
        let widths =
            parallel_projected_widths_by_body_for_targets((1.0, 0.0), &bodies, &[0]).unwrap();
        assert!((widths[0][0].1 - 1.0).abs() < 0.02);
        assert_eq!(widths[1][0].1, 0.0);
    }

    #[test]
    fn circular_receiver_is_shadowed_by_a_circular_occluder_in_the_same_parallel_field() {
        let bodies = [circle(4.0, 0.0, 1.0), circle(8.0, 0.0, 0.5)];
        let widths =
            parallel_projected_widths_by_body_for_targets((1.0, 0.0), &bodies, &[0]).unwrap();
        assert!((widths[0][0].1 - 1.0).abs() < 0.02);
        assert_eq!(widths[1][0].1, 0.0);
    }
}

#[cfg(test)]
mod parallel_lane_intersection_regression_tests {
    use super::ParallelRayFrame;

    #[test]
    fn collinear_disjoint_segments_do_not_disable_parallel_partition() {
        let frame = ParallelRayFrame::new((1.0, 0.0)).unwrap();
        assert_eq!(
            frame.segment_intersection_lane((0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (3.0, 0.0)),
            Some(None)
        );
    }

    #[test]
    fn collinear_touch_is_a_lane_event_but_positive_overlap_fails_closed() {
        let frame = ParallelRayFrame::new((1.0, 0.0)).unwrap();
        assert_eq!(
            frame.segment_intersection_lane((0.0, 0.0), (1.0, 0.0), (1.0, 0.0), (2.0, 0.0)),
            Some(Some(0.0))
        );
        assert_eq!(
            frame.segment_intersection_lane((0.0, 0.0), (2.0, 0.0), (1.0, 0.0), (3.0, 0.0)),
            None
        );
    }
}

#[cfg(test)]
mod parallel_polygonal_visibility_tests {
    use super::visible_parallel_polygonal_intervals_for_targets;
    use crate::organism_geometry::{OrganismBodyGeometry, PlacedForm};
    use crate::resources::Form;

    fn rectangle(x: f64, y: f64, width: f64, height: f64) -> OrganismBodyGeometry {
        let radius = (width * width + height * height).sqrt() * 0.5;
        OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index: 0,
                form: Form::Rectangle { width, height },
                x,
                y,
                rotation_radians: 0.0,
            }],
            min_x: x - radius,
            max_x: x + radius,
            min_y: y - radius,
            max_y: y + radius,
        }
    }

    #[test]
    fn aligned_disjoint_rectangles_do_not_fail_closed_as_collinear_overlap() {
        let bodies = [rectangle(0.0, 0.0, 2.0, 2.0), rectangle(5.0, 0.0, 2.0, 2.0)];
        let visible = visible_parallel_polygonal_intervals_for_targets((1.0, 0.0), &bodies, &[0]);
        assert!(visible.is_some());
        assert!(visible
            .unwrap()
            .iter()
            .all(|interval| interval.body_index == 0));
    }

    #[test]
    fn parallel_classifier_splits_a_partially_shadowed_front_edge() {
        let bodies = [rectangle(4.0, 0.0, 2.0, 2.0), rectangle(8.0, 0.0, 2.0, 1.0)];
        let visible =
            visible_parallel_polygonal_intervals_for_targets((1.0, 0.0), &bodies, &[0]).unwrap();
        let front: Vec<_> = visible
            .iter()
            .filter(|interval| {
                interval.body_index == 0
                    && (interval.start.0 - 5.0).abs() < 1e-9
                    && (interval.end.0 - 5.0).abs() < 1e-9
            })
            .collect();
        assert_eq!(front.len(), 2);
        let mut ranges: Vec<_> = front
            .iter()
            .map(|interval| {
                (
                    interval.start.1.min(interval.end.1),
                    interval.start.1.max(interval.end.1),
                )
            })
            .collect();
        ranges.sort_by(|a, b| a.0.total_cmp(&b.0));
        assert!((ranges[0].0 + 1.0).abs() < 1e-9);
        assert!((ranges[0].1 + 0.5).abs() < 1e-9);
        assert!((ranges[1].0 - 0.5).abs() < 1e-9);
        assert!((ranges[1].1 - 1.0).abs() < 1e-9);
    }

    #[test]
    fn parallel_classifier_does_not_credit_a_shadowing_body() {
        let bodies = [rectangle(4.0, 0.0, 2.0, 2.0), rectangle(8.0, 0.0, 2.0, 1.0)];
        let visible =
            visible_parallel_polygonal_intervals_for_targets((1.0, 0.0), &bodies, &[0]).unwrap();
        assert!(visible.iter().all(|interval| interval.body_index == 0));
    }

    #[test]
    fn parallel_classifier_is_translation_invariant() {
        let original = [rectangle(4.0, 0.0, 2.0, 2.0), rectangle(8.0, 0.0, 2.0, 1.0)];
        let shifted = [
            rectangle(104.0, 0.0, 2.0, 2.0),
            rectangle(108.0, 0.0, 2.0, 1.0),
        ];
        let a =
            visible_parallel_polygonal_intervals_for_targets((1.0, 0.0), &original, &[0]).unwrap();
        let b =
            visible_parallel_polygonal_intervals_for_targets((1.0, 0.0), &shifted, &[0]).unwrap();
        let mut lengths_a: Vec<_> = a
            .iter()
            .map(|v| (v.end.0 - v.start.0).hypot(v.end.1 - v.start.1))
            .collect();
        let mut lengths_b: Vec<_> = b
            .iter()
            .map(|v| (v.end.0 - v.start.0).hypot(v.end.1 - v.start.1))
            .collect();
        lengths_a.sort_by(f64::total_cmp);
        lengths_b.sort_by(f64::total_cmp);
        assert_eq!(lengths_a.len(), lengths_b.len());
        for (left, right) in lengths_a.iter().zip(lengths_b.iter()) {
            assert!((left - right).abs() < 1e-9);
        }
    }

    #[test]
    fn parallel_classifier_accepts_explicit_polygonal_fluid_boundaries() {
        let mut fluid = rectangle(4.0, 0.0, 2.0, 2.0);
        fluid.parts[0].form = Form::Fluid {
            nominal_area: 4.0,
            boundary: Some(vec![(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]),
            arc_boundary: None,
            multi_contact_boundary: None,
        };
        let visible = visible_parallel_polygonal_intervals_for_targets((1.0, 0.0), &[fluid], &[0]);
        assert!(visible.is_some());
        assert!(!visible.unwrap().is_empty());
    }

    #[test]
    fn polygonal_fluid_boundary_occludes_a_receiver_in_the_shared_ray_field() {
        let receiver = rectangle(4.0, 0.0, 2.0, 2.0);
        let mut fluid = rectangle(8.0, 0.0, 2.0, 1.0);
        fluid.parts[0].form = Form::Fluid {
            nominal_area: 2.0,
            boundary: Some(vec![(-1.0, -0.5), (1.0, -0.5), (1.0, 0.5), (-1.0, 0.5)]),
            arc_boundary: None,
            multi_contact_boundary: None,
        };
        let visible =
            visible_parallel_polygonal_intervals_for_targets((1.0, 0.0), &[receiver, fluid], &[0])
                .unwrap();
        let receiver_front: Vec<_> = visible
            .iter()
            .filter(|interval| {
                interval.body_index == 0
                    && (interval.start.0 - 5.0).abs() < 1e-9
                    && (interval.end.0 - 5.0).abs() < 1e-9
            })
            .collect();
        assert_eq!(receiver_front.len(), 2);
    }

    #[test]
    fn parallel_classifier_fails_closed_when_curved_geometry_is_present() {
        let mut circle = rectangle(4.0, 0.0, 2.0, 2.0);
        circle.parts[0].form = Form::Circle { radius: 1.0 };
        assert!(
            visible_parallel_polygonal_intervals_for_targets((1.0, 0.0), &[circle], &[0]).is_none()
        );
    }
}

#[cfg(test)]
mod projected_extent_motion_tests {
    use super::*;

    #[test]
    fn projected_extent_motion_bound_contains_source_and_endpoint_perturbations() {
        let interval = ExposedBoundaryInterval {
            body_index: 0,
            unit_index: 0,
            start: (-1.0, 0.0),
            end: (1.0, 0.0),
            outward_normal: (0.0, 1.0),
        };
        let source = (0.3, 2.0);
        let source_bound = 0.01;
        let start_bound = 0.005;
        let end_bound = 0.007;
        let bound = point_source_projected_extent_change_bound(
            interval,
            source,
            source_bound,
            start_bound,
            end_bound,
        )
        .expect("non-singular straight interval should have a finite extent bound");
        let baseline = point_source_projected_extent(interval, source).expect("baseline extent");

        let mut state = 0x8f3f_73b5_cf1c_9ade_u64;
        let mut next_direction = || {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let angle = ((state >> 32) as u32) as f64 / u32::MAX as f64 * std::f64::consts::TAU;
            (angle.cos(), angle.sin())
        };
        for _ in 0..256 {
            let (sx, sy) = next_direction();
            let (ax, ay) = next_direction();
            let (bx, by) = next_direction();
            let moved_start = (
                interval.start.0 + ax * start_bound * 0.8,
                interval.start.1 + ay * start_bound * 0.8,
            );
            let moved_end = (
                interval.end.0 + bx * end_bound * 0.8,
                interval.end.1 + by * end_bound * 0.8,
            );
            let dx = moved_end.0 - moved_start.0;
            let dy = moved_end.1 - moved_start.1;
            let length = dx.hypot(dy);
            let moved_interval = ExposedBoundaryInterval {
                start: moved_start,
                end: moved_end,
                outward_normal: (-dy / length, dx / length),
                ..interval
            };
            let moved_source = (
                source.0 + sx * source_bound * 0.8,
                source.1 + sy * source_bound * 0.8,
            );
            let moved_extent = point_source_projected_extent(moved_interval, moved_source).unwrap();
            assert!(
                (moved_extent - baseline).abs() <= bound,
                "projected extent change exceeded certified envelope"
            );
        }
    }

    #[test]
    fn projected_extent_motion_bound_rejects_collapsed_or_singular_geometry() {
        let interval = ExposedBoundaryInterval {
            body_index: 0,
            unit_index: 0,
            start: (-1.0, 0.0),
            end: (1.0, 0.0),
            outward_normal: (0.0, 1.0),
        };
        assert!(
            point_source_projected_extent_change_bound(interval, (0.0, 0.0), 0.0, 0.0, 0.0,)
                .is_none()
        );
        assert!(
            point_source_projected_extent_change_bound(interval, (0.0, 2.0), 0.0, 1.0, 1.0,)
                .is_none()
        );
        assert!(point_source_projected_extent_change_bound(
            interval,
            (0.0, 2.0),
            f64::NAN,
            0.0,
            0.0,
        )
        .is_none());
    }
}
