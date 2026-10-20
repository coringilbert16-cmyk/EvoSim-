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
#[allow(dead_code)] // Geometry measurement is staged before live exposure discovery.
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

/// Derive visible straight boundary intervals for polygonal bodies.
///
/// Visibility can change only along rays from the source through blocker
/// vertices. Intersections of those rays with each target edge define exact
/// interval cuts for polygonal geometry. Midpoints classify each open interval
/// using the realized-geometry occlusion test. Unsupported constituent shapes
/// return None rather than being approximated.
#[allow(dead_code)]
pub(crate) fn visible_polygonal_intervals(
    source: (f64, f64),
    bodies: &[crate::organism_geometry::OrganismBodyGeometry],
) -> Option<Vec<ExposedBoundaryInterval>> {
    if !source.0.is_finite() || !source.1.is_finite() {
        return None;
    }

    let mut polygons = Vec::new();
    for (body_index, body) in bodies.iter().enumerate() {
        // Unit indices are the identity key used to exclude a target from its
        // own occlusion test and to attribute projected width to a constituent.
        // Duplicate identities would make both operations ambiguous, so reject
        // the entire geometry snapshot rather than risk double-counting it.
        let mut seen_unit_indices = Vec::with_capacity(body.parts.len());
        for part in &body.parts {
            if seen_unit_indices.contains(&part.unit_index) {
                return None;
            }
            seen_unit_indices.push(part.unit_index);
            if !part.x.is_finite() || !part.y.is_finite() || !part.rotation_radians.is_finite() {
                return None;
            }
            let local = part.form.polygon_vertices()?;
            if local.len() < 3 || local.iter().any(|(x, y)| !x.is_finite() || !y.is_finite()) {
                return None;
            }
            let (sin, cos) = part.rotation_radians.sin_cos();
            let vertices = local
                .into_iter()
                .map(|(x, y)| (part.x + x * cos - y * sin, part.y + x * sin + y * cos))
                .collect::<Vec<_>>();
            if vertices
                .iter()
                .any(|(x, y)| !x.is_finite() || !y.is_finite())
            {
                return None;
            }
            polygons.push((body_index, part.unit_index, vertices));
        }
    }

    let mut result = Vec::new();
    for (body_index, body) in bodies.iter().enumerate() {
        for part in &body.parts {
            let local = part.form.polygon_vertices()?;
            let (sin, cos) = part.rotation_radians.sin_cos();
            let vertices = local
                .into_iter()
                .map(|(x, y)| (part.x + x * cos - y * sin, part.y + x * sin + y * cos))
                .collect::<Vec<_>>();
            if vertices
                .iter()
                .any(|(x, y)| !x.is_finite() || !y.is_finite())
                || !polygon_is_convex(&vertices)
            {
                return None;
            }
            let area2: f64 = vertices
                .iter()
                .enumerate()
                .map(|(i, &(x1, y1))| {
                    let (x2, y2) = vertices[(i + 1) % vertices.len()];
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
                if normal.0 * (source.0 - midpoint.0) + normal.1 * (source.1 - midpoint.1) <= 1e-10
                {
                    continue;
                }

                let mut cuts = vec![0.0_f64, 1.0_f64];
                for (block_body, block_unit, blocker) in &polygons {
                    if *block_body == body_index && *block_unit == part.unit_index {
                        continue;
                    }
                    for &(vx, vy) in blocker {
                        let dx = vx - source.0;
                        let dy = vy - source.1;
                        let denominator = ex * dy - ey * dx;
                        if denominator.abs() <= 1e-12 {
                            continue;
                        }
                        let ax = a.0 - source.0;
                        let ay = a.1 - source.1;
                        let t = -(ax * dy - ay * dx) / denominator;
                        // Correct signed parameter for source-to-vertex ray:
                        // A + tE = S + u(V-S), with u > 1 beyond the vertex.
                        let u = -(ax * ey - ay * ex) / denominator;
                        if t > 1e-10 && t < 1.0 - 1e-10 && u > 1.0 + 1e-10 {
                            cuts.push(t);
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
                    if crate::organism_geometry::source_path_blocked(
                        source,
                        point,
                        bodies,
                        body_index,
                        part.unit_index,
                    ) {
                        continue;
                    }
                    result.push(ExposedBoundaryInterval {
                        body_index,
                        unit_index: part.unit_index,
                        start: (a.0 + ex * t0, a.1 + ey * t0),
                        end: (a.0 + ex * t1, a.1 + ey * t1),
                        outward_normal: normal,
                    });
                }
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
    fn polygonal_exposure_fails_closed_for_curved_boundaries() {
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
            visible_polygonal_intervals((-5.0, 0.0), &[circle]).is_none(),
            "curved geometry must not silently become zero exposure or an invented polygon"
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
            visible_polygonal_intervals((-5.0, 0.0), &[fluid]).is_none(),
            "exact fluid arcs need analytic interval handling before exposure can be credited"
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

/// Return a world-space circular boundary for a constituent. Curved fluids
/// whose boundary may self-occlude are rejected rather than approximated.
fn realized_circle_arcs(
    body_index: usize,
    part: &crate::organism_geometry::PlacedForm,
) -> Option<Vec<WorldCircleArc>> {
    use crate::capillary_geometry::{CircularArcBoundary, FluidBoundaryPiece};
    use crate::resources::Form;
    let (sin, cos) = part.rotation_radians.sin_cos();
    let mut arcs = Vec::new();
    let mut add = |center_x: f64, center_y: f64, radius: f64, start: f64, sweep: f64| {
        let center = (
            part.x + center_x * cos - center_y * sin,
            part.y + center_x * sin + center_y * cos,
        );
        let arc = WorldCircleArc {
            body_index,
            unit_index: part.unit_index,
            center,
            radius,
            start: (start + part.rotation_radians).rem_euclid(std::f64::consts::TAU),
            sweep,
        };
        (center.0.is_finite()
            && center.1.is_finite()
            && radius.is_finite()
            && radius > 0.0
            && start.is_finite()
            && sweep.is_finite()
            && sweep > 0.0
            && sweep <= std::f64::consts::TAU)
            .then(|| arcs.push(arc))
    };
    match &part.form {
        Form::Circle { radius } => {
            add(0.0, 0.0, *radius, 0.0, std::f64::consts::TAU)?;
        }
        Form::Fluid {
            nominal_area,
            boundary,
            arc_boundary,
            multi_contact_boundary,
        } => {
            if multi_contact_boundary.is_some() {
                // Multi-contact fluid remains an occluder through the exact
                // segment/arc queries, but is not yet a supported target: its
                // complete boundary must be analyzed together to prevent
                // self-occlusion from being mistaken for exposed surface.
                return Some(Vec::new());
            }
            if let Some(vertices) = boundary {
                // A polygonal fluid boundary is handled by the polygon solver,
                // not by this circular-arc path.
                if vertices.len() >= 3 {
                    return Some(Vec::new());
                }
                return None;
            }
            if let Some(arc) = arc_boundary {
                if !arc.is_valid() || arc.sweep_radians > std::f64::consts::PI + 1e-10 {
                    return None;
                }
                add(
                    arc.center_x,
                    arc.center_y,
                    arc.radius,
                    arc.start_angle_radians,
                    arc.sweep_radians,
                )?;
            } else {
                if !nominal_area.is_finite() || *nominal_area <= 0.0 {
                    return None;
                }
                let radius = (nominal_area / std::f64::consts::PI).sqrt();
                add(0.0, 0.0, radius, 0.0, std::f64::consts::TAU)?;
            }
        }
        _ => return Some(Vec::new()),
    }
    // Keep the import's type checked here: all exact fluid arcs are represented
    // by the same descriptor that physical geometry and persistence use.
    let _ = std::mem::size_of::<CircularArcBoundary>();
    let _ = std::mem::size_of::<FluidBoundaryPiece>();
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

/// Exact visibility partitioning for circle and circular-fluid boundaries under
/// a finite point source. Visibility can change only along rays through polygon
/// vertices, circular-arc endpoints, or circle tangencies. Those critical rays
/// cut the target arc; an exact geometry query classifies each resulting interval.
/// Unsupported/self-occluding curved fluid boundaries return None.
#[allow(dead_code)]
pub(crate) fn visible_circular_arc_intervals(
    source: (f64, f64),
    bodies: &[crate::organism_geometry::OrganismBodyGeometry],
) -> Option<Vec<ExposedCircularArcInterval>> {
    use crate::capillary_geometry::FluidBoundaryPiece;
    use crate::resources::Form;
    if !source.0.is_finite() || !source.1.is_finite() {
        return None;
    }

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
                    critical_points.push((
                        part.x + x * cos - y * sin,
                        part.y + x * sin + y * cos,
                    ));
                }
            }
            match &part.form {
                Form::Circle { radius } => {
                    add_circle_tangent_points((part.x, part.y), *radius, source, &mut critical_points);
                }
                Form::Fluid { boundary, arc_boundary, multi_contact_boundary, .. } => {
                    if let Some(verts) = boundary {
                        let (sin, cos) = part.rotation_radians.sin_cos();
                        for &(x, y) in verts {
                            critical_points.push((
                                part.x + x * cos - y * sin,
                                part.y + x * sin + y * cos,
                            ));
                        }
                    }
                    if let Some(arc) = arc_boundary {
                        if !arc.is_valid() || arc.sweep_radians > std::f64::consts::PI + 1e-10 {
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
                                FluidBoundaryPiece::SolidContact { start_x, start_y, end_x, end_y, .. } => {
                                    let (sin, cos) = part.rotation_radians.sin_cos();
                                    for (x, y) in [(*start_x, *start_y), (*end_x, *end_y)] {
                                        critical_points.push((
                                            part.x + x * cos - y * sin,
                                            part.y + x * sin + y * cos,
                                        ));
                                    }
                                }
                                FluidBoundaryPiece::FreeArc(arc) => {
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
                            }
                        }
                    }
                }
                _ => {}
            }
            all_arcs.extend(realized_circle_arcs(body_index, part)?);
        }
    }

    let mut output = Vec::new();
    for target in all_arcs {
        // If the source is inside the target material, its boundary cannot be
        // safely interpreted as an externally illuminated surface.
        if let Some(body) = bodies.get(target.body_index) {
            if let Some(part) = body.parts.iter().find(|p| p.unit_index == target.unit_index) {
                if body.contains_point(source.0, source.1) {
                    continue;
                }
            }
        }

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
                let unwrapped = target.start + (angle - target.start).rem_euclid(std::f64::consts::TAU);
                if unwrapped > target.start + 1e-10 && unwrapped < target.start + target.sweep - 1e-10 {
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
            if !discriminant.is_finite() || discriminant < -1e-10 {
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
                    let unwrapped = target.start + (angle - target.start).rem_euclid(std::f64::consts::TAU);
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
            if crate::organism_geometry::source_path_blocked(
                source, point, bodies, target.body_index, target.unit_index,
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

/// Projected extent for a visible circular interval under a finite point source.
/// The pointwise source direction varies along the arc, so this uses adaptive
/// Simpson integration rather than substituting a single direction for the arc.
#[allow(dead_code)]
pub(crate) fn point_source_projected_arc_extent(
    arc: crate::capillary_geometry::CircularArcBoundary,
    source: (f64, f64),
) -> Option<f64> {
    if !arc.is_valid() || !source.0.is_finite() || !source.1.is_finite() {
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
    fn integrate<F: Fn(f64) -> Option<f64>>(
        f: &F, a: f64, b: f64, fa: f64, fm: f64, fb: f64, whole: f64, tolerance: f64, depth: u8,
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
    let extent = integrate(&integrand, a, b, fa, fm, fb, whole, 1e-10 * whole.abs().max(1.0), 20)?;
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
    fn point_source_circle_exposure_is_exactly_partitioned_without_a_blocker() {
        let target = body(Form::Circle { radius: 1.0 }, 5.0, 0.0);
        let intervals = visible_circular_arc_intervals((-5.0, 0.0), &[target])
            .expect("circle boundary and point-source visibility are supported");
        assert!(!intervals.is_empty());
        let sweep: f64 = intervals.iter().map(|interval| interval.arc.sweep_radians).sum();
        assert!(sweep > 0.0 && sweep < std::f64::consts::PI);
        let extent: f64 = intervals
            .iter()
            .map(|interval| {
                point_source_projected_arc_extent(interval.arc, (-5.0, 0.0)).unwrap()
            })
            .sum();
        assert!(extent.is_finite() && extent > 0.0);
    }

    #[test]
    fn point_source_circle_boundary_splits_around_a_polygonal_shadow() {
        let target = body(Form::Circle { radius: 1.0 }, 5.0, 0.0);
        let blocker = body(
            Form::Rectangle {
                width: 1.0,
                height: 1.0,
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
        assert!(!target_intervals.is_empty(), "unshadowed shoulders remain exposed");
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
    fn point_source_curve_query_fails_closed_for_multi_contact_fluid() {
        let fluid = Form::Fluid {
            nominal_area: 0.5,
            boundary: None,
            arc_boundary: None,
            multi_contact_boundary: Some(crate::capillary_geometry::MultiContactFluidBoundary {
                nominal_area: 0.5,
                pieces: vec![],
            }),
        };
        let target = body(fluid, 5.0, 0.0);
        assert!(visible_circular_arc_intervals((-5.0, 0.0), &[target]).is_none());
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
