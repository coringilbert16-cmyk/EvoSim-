//! Phase 3 reference geometry contract.
//!
//! Phase 3 deliberately supports polygonal solid boundaries and finite straight
//! line blockers only. Legacy circle and fluid geometry remains available to
//! unrelated simulation systems, but it must never enter Phase 3 calculations.
//! These checked adapters fail closed before delegating to shared legacy
//! geometry helpers, so unsupported forms cannot contribute cuts or occlusion.

use crate::organism_geometry::OrganismBodyGeometry;
use crate::phase3_polygon::checked_polygon_area;
use crate::resources::Form;
use crate::source_exposure::{self, ExposedBoundaryInterval, VisibleMaterialPerimeter};

fn is_phase3_polygon(form: &Form) -> bool {
    matches!(
        form,
        Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. }
    )
}

fn is_phase3_supported_part(form: &Form) -> bool {
    is_phase3_polygon(form) || matches!(form, Form::Line { .. })
}

/// Strict Phase 3 validity is independent of legacy Form::is_valid() polygon
/// tolerances. Lines retain their existing finite-length validation.
fn is_phase3_form_valid(form: &Form) -> bool {
    if is_phase3_polygon(form) {
        checked_polygon_area(form).is_some()
    } else if matches!(form, Form::Line { .. }) {
        form.is_valid()
    } else {
        false
    }
}

/// Compute conservative world-space bounds from the exact supported geometry.
/// These bounds are used by visibility candidate pruning, so caller-supplied
/// aggregate bounds must never be trusted without checking them against parts.
fn phase3_part_bounds(part: &crate::organism_geometry::PlacedForm) -> Option<(f64, f64, f64, f64)> {
    let local_vertices = match &part.form {
        Form::Line { length } => {
            if !length.is_finite() || *length <= 0.0 {
                return None;
            }
            vec![(-length * 0.5, 0.0), (length * 0.5, 0.0)]
        }
        form if is_phase3_polygon(form) => form.polygon_vertices()?,
        _ => return None,
    };
    let (sin, cos) = part.rotation_radians.sin_cos();
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for (x, y) in local_vertices {
        let world_x = part.x + x * cos - y * sin;
        let world_y = part.y + x * sin + y * cos;
        if !world_x.is_finite() || !world_y.is_finite() {
            return None;
        }
        min_x = min_x.min(world_x);
        max_x = max_x.max(world_x);
        min_y = min_y.min(world_y);
        max_y = max_y.max(world_y);
    }
    Some((min_x, max_x, min_y, max_y))
}

/// Reject circles, fluids, and malformed placements across the entire geometry
/// snapshot. Lines are supported as finite blockers but are not receiver bodies.
fn validate_visibility_snapshot(
    bodies: &[OrganismBodyGeometry],
    target_body_indices: &[usize],
) -> Option<()> {
    let mut targets = vec![false; bodies.len()];
    for &index in target_body_indices {
        *targets.get_mut(index)? = true;
    }

    for (body_index, body) in bodies.iter().enumerate() {
        if !body.min_x.is_finite()
            || !body.max_x.is_finite()
            || !body.min_y.is_finite()
            || !body.max_y.is_finite()
            || body.min_x > body.max_x
            || body.min_y > body.max_y
        {
            return None;
        }

        let mut has_polygon_receiver = false;
        let (mut actual_min_x, mut actual_max_x, mut actual_min_y, mut actual_max_y) = (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        );
        for part in &body.parts {
            if !is_phase3_supported_part(&part.form)
                || !is_phase3_form_valid(&part.form)
                || !part.x.is_finite()
                || !part.y.is_finite()
                || !part.rotation_radians.is_finite()
            {
                return None;
            }
            match &part.form {
                Form::Line { length } if !length.is_finite() || *length <= 0.0 => {
                    return None;
                }
                Form::Rectangle { width, height }
                    if !width.is_finite()
                        || !height.is_finite()
                        || *width <= 0.0
                        || *height <= 0.0 =>
                {
                    return None;
                }
                Form::RegularPolygon { sides, radius }
                    if *sides < 3 || !radius.is_finite() || *radius <= 0.0 =>
                {
                    return None;
                }
                Form::Polygon { vertices }
                    if vertices.len() < 3
                        || vertices
                            .iter()
                            .any(|(x, y)| !x.is_finite() || !y.is_finite()) =>
                {
                    return None;
                }
                _ => {}
            }
            let (part_min_x, part_max_x, part_min_y, part_max_y) = phase3_part_bounds(part)?;
            actual_min_x = actual_min_x.min(part_min_x);
            actual_max_x = actual_max_x.max(part_max_x);
            actual_min_y = actual_min_y.min(part_min_y);
            actual_max_y = actual_max_y.max(part_max_y);
            has_polygon_receiver |= is_phase3_polygon(&part.form);
        }

        if !body.parts.is_empty()
            && (body.min_x > actual_min_x
                || body.max_x < actual_max_x
                || body.min_y > actual_min_y
                || body.max_y < actual_max_y)
        {
            return None;
        }

        if targets[body_index] && !has_polygon_receiver {
            return None;
        }
    }
    Some(())
}

/// Phase 3 visibility: polygon receiver boundaries, polygon solid blockers, and
/// finite line blockers. Any circle or fluid anywhere in the snapshot rejects
/// the complete query rather than being silently included or approximated.
pub(crate) fn visible_polygonal_intervals_for_targets(
    source: (f64, f64),
    bodies: &[OrganismBodyGeometry],
    target_body_indices: &[usize],
) -> Option<Vec<ExposedBoundaryInterval>> {
    if !source.0.is_finite() || !source.1.is_finite() {
        return None;
    }
    validate_visibility_snapshot(bodies, target_body_indices)?;
    source_exposure::visible_phase3_polygonal_intervals_for_targets(
        source,
        bodies,
        target_body_indices,
    )
}

/// Phase 3 material perimeter: polygonal solids only. Finite lines are
/// zero-area blockers, not material perimeter receivers. Circles and fluids
/// are rejected even though the shared legacy perimeter helper still supports
/// circles for non-Phase-3 callers.
pub(crate) fn disjoint_polygonal_material_perimeter(
    body: &OrganismBodyGeometry,
    body_index: usize,
) -> Option<Vec<ExposedBoundaryInterval>> {
    if body.parts.is_empty()
        || body.parts.iter().any(|part| {
            !is_phase3_polygon(&part.form)
                || !is_phase3_form_valid(&part.form)
                || !part.x.is_finite()
                || !part.y.is_finite()
                || !part.rotation_radians.is_finite()
        })
    {
        return None;
    }
    let perimeter: VisibleMaterialPerimeter =
        source_exposure::disjoint_source_global_material_perimeter(body, body_index)?;
    if !perimeter.curved.is_empty() {
        return None;
    }
    Some(perimeter.straight)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organism_geometry::PlacedForm;

    fn body(form: Form, x: f64, y: f64, min: (f64, f64), max: (f64, f64)) -> OrganismBodyGeometry {
        OrganismBodyGeometry {
            parts: vec![PlacedForm {
                unit_index: 0,
                form,
                x,
                y,
                rotation_radians: 0.0,
            }],
            min_x: min.0,
            max_x: max.0,
            min_y: min.1,
            max_y: max.1,
        }
    }

    fn target() -> OrganismBodyGeometry {
        body(
            Form::Rectangle {
                width: 2.0,
                height: 2.0,
            },
            5.0,
            0.0,
            (4.0, -1.0),
            (6.0, 1.0),
        )
    }

    #[test]
    fn phase3_visibility_rejects_circle_blockers_instead_of_partitioning_against_them() {
        let bodies = vec![
            target(),
            body(
                Form::Circle { radius: 0.5 },
                0.0,
                0.0,
                (-0.5, -0.5),
                (0.5, 0.5),
            ),
        ];
        assert!(visible_polygonal_intervals_for_targets((-5.0, 0.0), &bodies, &[0]).is_none());

        let circle_target = vec![body(
            Form::Circle { radius: 1.0 },
            5.0,
            0.0,
            (4.0, -1.0),
            (6.0, 1.0),
        )];
        assert!(
            visible_polygonal_intervals_for_targets((-5.0, 0.0), &circle_target, &[0]).is_none()
        );
    }

    #[test]
    fn phase3_visibility_rejects_fluid_targets_and_blockers() {
        let fluid = || Form::Fluid {
            nominal_area: 1.0,
            boundary: None,
            arc_boundary: None,
            multi_contact_boundary: None,
        };
        let fluid_target = vec![body(fluid(), 5.0, 0.0, (4.0, -1.0), (6.0, 1.0))];
        assert!(
            visible_polygonal_intervals_for_targets((-5.0, 0.0), &fluid_target, &[0]).is_none()
        );

        let fluid_blocker = vec![target(), body(fluid(), 0.0, 0.0, (-1.0, -1.0), (1.0, 1.0))];
        assert!(
            visible_polygonal_intervals_for_targets((-5.0, 0.0), &fluid_blocker, &[0]).is_none()
        );
    }

    #[test]
    fn phase3_visibility_rejects_malformed_polygon_blockers() {
        let degenerate = body(
            Form::Polygon {
                vertices: vec![(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)],
            },
            0.0,
            0.0,
            (0.0, 0.0),
            (2.0, 0.0),
        );
        let self_intersecting = body(
            Form::Polygon {
                vertices: vec![(0.0, 0.0), (1.0, 1.0), (0.0, 1.0), (1.0, 0.0)],
            },
            0.0,
            0.0,
            (0.0, 0.0),
            (1.0, 1.0),
        );

        for blocker in [degenerate, self_intersecting] {
            assert!(
                visible_polygonal_intervals_for_targets((-5.0, 0.0), &[target(), blocker], &[0])
                    .is_none(),
                "malformed polygon blockers must fail closed"
            );
        }
    }

    #[test]
    fn phase3_visibility_accepts_finite_line_blockers_and_polygon_receivers() {
        let bodies = vec![
            target(),
            body(Form::Line { length: 1.0 }, 2.0, 0.0, (1.5, 0.0), (2.5, 0.0)),
        ];
        let intervals = visible_polygonal_intervals_for_targets((-5.0, 0.0), &bodies, &[0])
            .expect("polygon receiver and finite line blocker are Phase 3 geometry");
        assert_eq!(
            intervals.len(),
            2,
            "the finite segment must split the front-facing target edge into two visible shoulders"
        );
        assert!(intervals.iter().all(|interval| interval.body_index == 0));
    }

    #[test]
    fn phase3_visibility_rejects_undersized_bounds_that_could_hide_a_blocker() {
        let undersized_blocker = body(
            Form::Rectangle {
                width: 2.0,
                height: 2.0,
            },
            2.0,
            0.0,
            (1.75, -0.25),
            (2.25, 0.25),
        );
        let bodies = vec![target(), undersized_blocker];
        assert!(visible_polygonal_intervals_for_targets((-5.0, 0.0), &bodies, &[0]).is_none());
    }

    #[test]
    fn phase3_visibility_accepts_bounds_enclosing_rotated_polygon_geometry() {
        let rotated = body(
            Form::Rectangle {
                width: 4.0,
                height: 1.0,
            },
            2.0,
            0.0,
            (-1.0, -2.0),
            (5.0, 2.0),
        );
        let mut rotated = rotated;
        rotated.parts[0].rotation_radians = std::f64::consts::FRAC_PI_2;
        let bodies = vec![target(), rotated];
        assert!(visible_polygonal_intervals_for_targets((-5.0, 0.0), &bodies, &[0]).is_some());
    }

    #[test]
    fn phase3_material_perimeter_rejects_circle_and_fluid_parts() {
        let circle = body(
            Form::Circle { radius: 1.0 },
            0.0,
            0.0,
            (-1.0, -1.0),
            (1.0, 1.0),
        );
        let fluid = body(
            Form::Fluid {
                nominal_area: 1.0,
                boundary: None,
                arc_boundary: None,
                multi_contact_boundary: None,
            },
            0.0,
            0.0,
            (-1.0, -1.0),
            (1.0, 1.0),
        );
        let malformed_polygon = body(
            Form::Polygon {
                vertices: vec![(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)],
            },
            0.0,
            0.0,
            (0.0, 0.0),
            (2.0, 0.0),
        );
        assert!(disjoint_polygonal_material_perimeter(&circle, 0).is_none());
        assert!(disjoint_polygonal_material_perimeter(&fluid, 0).is_none());
        assert!(disjoint_polygonal_material_perimeter(&malformed_polygon, 0).is_none());
    }

    #[test]
    fn phase3_material_perimeter_returns_only_straight_polygon_edges() {
        let perimeter = disjoint_polygonal_material_perimeter(&target(), 3)
            .expect("rectangle is a supported Phase 3 solid");
        assert_eq!(perimeter.len(), 4);
        assert!(perimeter.iter().all(|interval| interval.body_index == 3));
    }

    #[test]
    fn phase3_material_perimeter_is_stable_under_large_translation() {
        let mut translated = target();
        translated.parts[0].x += 1.0e12;
        translated.parts[0].y -= 1.0e12;
        translated.min_x += 1.0e12;
        translated.max_x += 1.0e12;
        translated.min_y -= 1.0e12;
        translated.max_y -= 1.0e12;

        let perimeter = disjoint_polygonal_material_perimeter(&translated, 4)
            .expect("translated polygon remains a valid Phase 3 receiver");
        assert_eq!(perimeter.len(), 4);
        let length: f64 = perimeter
            .iter()
            .map(|interval| {
                (interval.end.0 - interval.start.0).hypot(interval.end.1 - interval.start.1)
            })
            .sum();
        assert!((length - 8.0).abs() < 1e-3);
    }
}
