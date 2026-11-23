//! Phase 3 reference geometry contract.
//!
//! Phase 3 deliberately supports polygonal solid boundaries and finite straight
//! line blockers only. Legacy circle and fluid geometry remains available to
//! unrelated simulation systems, but it must never enter Phase 3 calculations.
//! These checked adapters fail closed before delegating to shared legacy
//! geometry helpers, so unsupported forms cannot contribute cuts or occlusion.

use crate::organism_geometry::OrganismBodyGeometry;
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
        for part in &body.parts {
            if !is_phase3_supported_part(&part.form)
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
            has_polygon_receiver |= is_phase3_polygon(&part.form);
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
    source_exposure::visible_polygonal_intervals_for_targets(source, bodies, target_body_indices)
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
    fn phase3_visibility_accepts_finite_line_blockers_and_polygon_receivers() {
        let bodies = vec![
            target(),
            body(
                Form::Line { length: 1.0 },
                2.0,
                0.0,
                (2.0, -0.5),
                (2.0, 0.5),
            ),
        ];
        let intervals = visible_polygonal_intervals_for_targets((-5.0, 0.0), &bodies, &[0])
            .expect("polygon receiver and finite line blocker are Phase 3 geometry");
        assert!(!intervals.is_empty());
        assert!(intervals.iter().all(|interval| interval.body_index == 0));
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
        assert!(disjoint_polygonal_material_perimeter(&circle, 0).is_none());
        assert!(disjoint_polygonal_material_perimeter(&fluid, 0).is_none());
    }

    #[test]
    fn phase3_material_perimeter_returns_only_straight_polygon_edges() {
        let perimeter = disjoint_polygonal_material_perimeter(&target(), 3)
            .expect("rectangle is a supported Phase 3 solid");
        assert_eq!(perimeter.len(), 4);
        assert!(perimeter.iter().all(|interval| interval.body_index == 3));
    }
}
