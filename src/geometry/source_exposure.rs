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

fn add_circle_tangent_points_tagged(
    center: (f64, f64),
    radius: f64,
    source: (f64, f64),
    body_index: usize,
    critical_points: &mut Vec<(usize, f64, f64)>,
) {
    let mut points = Vec::with_capacity(2);
    add_circle_tangent_points(center, radius, source, &mut points);
    critical_points.extend(points.into_iter().map(|(x, y)| (body_index, x, y)));
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
pub(crate) fn visible_polygonal_intervals_for_targets(
    source: (f64, f64),
    bodies: &[crate::organism_geometry::OrganismBodyGeometry],
    target_body_indices: &[usize],
) -> Option<Vec<ExposedBoundaryInterval>> {
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

    let mut straight_targets = Vec::new();
    let mut straight_edges = Vec::new();
    let mut critical_points: Vec<(usize, f64, f64)> = Vec::new();

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
                critical_points.extend(vertices.iter().copied().map(|(x, y)| (body_index, x, y)));
                for index in 0..vertices.len() {
                    straight_edges.push((
                        body_index,
                        part.unit_index,
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
                    critical_points
                        .extend([(body_index, start.0, start.1), (body_index, end.0, end.1)]);
                    straight_edges.push((body_index, part.unit_index, start, end));
                }
                Form::Circle { radius } => {
                    add_circle_tangent_points_tagged(
                        (part.x, part.y),
                        *radius,
                        source,
                        body_index,
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
                        critical_points
                            .extend(world.iter().copied().map(|(x, y)| (body_index, x, y)));
                        for index in 0..world.len() {
                            straight_edges.push((
                                body_index,
                                part.unit_index,
                                world[index],
                                world[(index + 1) % world.len()],
                            ));
                        }
                    }
                    if let Some(arc) = arc_boundary {
                        let world = arc.transformed(part.x, part.y, part.rotation_radians)?;
                        critical_points.extend(
                            [world.start_point(), world.end_point()]
                                .map(|(x, y)| (body_index, x, y)),
                        );
                        add_circle_tangent_points_tagged(
                            (world.center_x, world.center_y),
                            world.radius,
                            source,
                            body_index,
                            &mut critical_points,
                        );
                        straight_edges.push((
                            body_index,
                            part.unit_index,
                            world.start_point(),
                            world.end_point(),
                        ));
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
                                    let transform = |x: f64, y: f64| {
                                        (part.x + x * cos - y * sin, part.y + x * sin + y * cos)
                                    };
                                    let start = transform(*start_x, *start_y);
                                    let end = transform(*end_x, *end_y);
                                    critical_points.extend([
                                        (body_index, start.0, start.1),
                                        (body_index, end.0, end.1),
                                    ]);
                                    straight_edges.push((body_index, part.unit_index, start, end));
                                }
                                FluidBoundaryPiece::FreeArc(arc) => {
                                    let world =
                                        arc.transformed(part.x, part.y, part.rotation_radians)?;
                                    critical_points.extend(
                                        [world.start_point(), world.end_point()]
                                            .map(|(x, y)| (body_index, x, y)),
                                    );
                                    add_circle_tangent_points_tagged(
                                        (world.center_x, world.center_y),
                                        world.radius,
                                        source,
                                        body_index,
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
        .any(|(_, x, y)| !x.is_finite() || !y.is_finite())
        || straight_edges.iter().any(|(_, _, a, b)| {
            !a.0.is_finite() || !a.1.is_finite() || !b.0.is_finite() || !b.1.is_finite()
        })
    {
        return None;
    }

    let mut critical_points_by_body: Vec<Vec<(f64, f64)>> =
        (0..bodies.len()).map(|_| Vec::new()).collect();
    for (body_index, x, y) in critical_points {
        critical_points_by_body[body_index].push((x, y));
    }
    type StraightEdge = (usize, (f64, f64), (f64, f64));
    let mut straight_edges_by_body: Vec<Vec<StraightEdge>> =
        (0..bodies.len()).map(|_| Vec::new()).collect();
    for (body_index, unit_index, start, end) in straight_edges {
        straight_edges_by_body[body_index].push((unit_index, start, end));
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

            let candidate_body_indices = candidate_body_indices_for_bounds(
                source,
                (a.0.min(b.0), a.0.max(b.0), a.1.min(b.1), a.1.max(b.1)),
                reference_angle,
                &body_intervals,
                &body_min_distances,
            )?;
            let mut cuts = vec![0.0_f64, 1.0_f64];
            for &candidate_body in &candidate_body_indices {
                for &(vx, vy) in &critical_points_by_body[candidate_body] {
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
                        cuts.push(t);
                    }
                }
            }

            // Directly split at straight-boundary intersections, including
            // contact chords. Collinear overlaps are split at both endpoints.
            for &block_body in &candidate_body_indices {
                for &(block_unit, c, d) in &straight_edges_by_body[block_body] {
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
                            for point in [c, d] {
                                let t =
                                    ((point.0 - a.0) * ex + (point.1 - a.1) * ey) / length_squared;
                                if t > 1e-10 && t < 1.0 - 1e-10 {
                                    cuts.push(t);
                                }
                            }
                        }
                        continue;
                    }
                    let t = (qx * sy - qy * sx) / denominator;
                    let u = (qx * ey - qy * ex) / denominator;
                    if t > 1e-10 && t < 1.0 - 1e-10 && (-1e-10..=1.0 + 1e-10).contains(&u) {
                        cuts.push(t);
                    }
                }
            }

            // Curved-boundary intersections with this straight target edge are
            // exact cut points, not a reason to approximate the curve.
            for &block_body_index in &candidate_body_indices {
                let block_body = bodies.get(block_body_index)?;
                for blocker in &block_body.parts {
                    if block_body_index == body_index && blocker.unit_index == unit_index {
                        continue;
                    }
                    for arc in realized_circle_arcs(block_body_index, blocker)? {
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
                                cuts.push(t);
                            }
                        }
                    }
                }
            }

            cuts.sort_by(f64::total_cmp);
            cuts.dedup_by(|a, b| (*a - *b).abs() <= 1e-10);
            for pair in cuts.windows(2) {
                let (t0, t1) = (pair[0], pair[1]);
                if t1 - t0 <= 1e-10 {
                    continue;
                }
                let t = (t0 + t1) * 0.5;
                let point = (a.0 + ex * t, a.1 + ey * t);
                if crate::organism_geometry::source_path_blocked_for_candidates(
                    source,
                    point,
                    bodies,
                    body_index,
                    unit_index,
                    &candidate_body_indices,
                ) {
                    continue;
                }
                result.push(ExposedBoundaryInterval {
                    body_index,
                    unit_index,
                    start: (a.0 + ex * t0, a.1 + ey * t0),
                    end: (a.0 + ex * t1, a.1 + ey * t1),
                    outward_normal: normal,
                });
            }
        }
    }
    Some(result)
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

#[derive(Clone, Copy)]
struct WorldCircleArc {
    body_index: usize,
    unit_index: usize,
    center: (f64, f64),
    radius: f64,
    start: f64,
    sweep: f64,
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
    for first_index in 0..all_arcs.len() {
        let first = all_arcs[first_index];
        if !target_body_mask[first.body_index] {
            continue;
        }
        for second_index in 0..all_arcs.len() {
            if second_index == first_index {
                continue;
            }
            let second = all_arcs[second_index];
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
pub(crate) fn parallel_projected_widths_by_body_for_targets(
    toward_source: (f64, f64),
    bodies: &[crate::organism_geometry::OrganismBodyGeometry],
    target_body_indices: &[usize],
) -> Option<Vec<Vec<(usize, f64)>>> {
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
                let Some(vertices) = (match &part.form {
                    Form::Fluid {
                        boundary: Some(vertices),
                        ..
                    } => Some(vertices.clone()),
                    _ => part.form.polygon_vertices(),
                }) else {
                    return None;
                };
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

    for arc in &arcs {
        let (lo, hi) = frame.circle_critical_lanes(arc.center, arc.radius)?;
        curved_lanes.extend([lo, hi]);
        for angle in [arc.start, arc.start + arc.sweep] {
            let point = (
                arc.center.0 + arc.radius * angle.cos(),
                arc.center.1 + arc.radius * angle.sin(),
            );
            curved_lanes.push(round_geometry_to_cent(frame.lane(point)?)?);
        }
    }

    for left in 0..segments.len() {
        for right in (left + 1)..segments.len() {
            let a = segments[left];
            let b = segments[right];
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

    for arc in &arcs {
        for segment in &segments {
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

    for first_index in 0..arcs.len() {
        for second_index in (first_index + 1)..arcs.len() {
            let first = arcs[first_index];
            let second = arcs[second_index];
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

    let partition = ParallelLanePartition::from_boundary_events(straight_lanes, curved_lanes)?;
    for (lo, hi, lane) in partition.intervals() {
        let mut nearest: Option<(f64, Surface)> = None;
        let mut tied = false;
        let mut consider = |depth: f64, surface: Surface| -> Option<()> {
            if !depth.is_finite() {
                return None;
            }
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

        for (index, segment) in segments.iter().enumerate() {
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

        for (index, arc) in arcs.iter().enumerate() {
            let Some((near, far)) = frame.circle_hit_depths(lane, arc.center, arc.radius)? else {
                continue;
            };
            for depth in [near, far] {
                let point = (
                    frame.toward_source.0 * depth + frame.perpendicular.0 * lane,
                    frame.toward_source.1 * depth + frame.perpendicular.1 * lane,
                );
                let angle = (point.1 - arc.center.1).atan2(point.0 - arc.center.0);
                if arc_has_angle(*arc, angle) {
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
    Some(widths)
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
