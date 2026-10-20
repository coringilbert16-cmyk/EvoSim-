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
        for part in &body.parts {
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
