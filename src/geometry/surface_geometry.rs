//! Continuous physical-boundary queries.
use crate::resources::{Form, Shape};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundaryPoint {
    pub x: f64,
    pub y: f64,
    pub normal_x: f64,
    pub normal_y: f64,
}
fn normalized(x: f64, y: f64) -> Option<(f64, f64)> {
    let length = x.hypot(y);
    if !length.is_finite() || length <= f64::EPSILON {
        None
    } else {
        Some((x / length, y / length))
    }
}
fn polygon_winding(vertices: &[(f64, f64)]) -> f64 {
    vertices
        .iter()
        .enumerate()
        .map(|(i, &(ax, ay))| {
            let (bx, by) = vertices[(i + 1) % vertices.len()];
            ax * by - ay * bx
        })
        .sum()
}
fn polygon_boundary_toward(
    vertices: &[(f64, f64)],
    target_x: f64,
    target_y: f64,
) -> Option<BoundaryPoint> {
    if vertices.len() < 3 {
        return None;
    }
    let (ux, uy) = normalized(target_x, target_y)?;
    let winding = polygon_winding(vertices);
    if winding.abs() <= 1e-12 {
        return None;
    }
    let mut best = None;
    for i in 0..vertices.len() {
        let (ax, ay) = vertices[i];
        let (bx, by) = vertices[(i + 1) % vertices.len()];
        let ex = bx - ax;
        let ey = by - ay;
        let denominator = ux * ey - uy * ex;
        if denominator.abs() <= 1e-12 {
            continue;
        }
        let t = (ax * ey - ay * ex) / denominator;
        let s = (ax * uy - ay * ux) / denominator;
        if t <= 1e-12 || !(-1e-10..=1.0000000001).contains(&s) {
            continue;
        }
        let edge_length = ex.hypot(ey);
        if edge_length <= f64::EPSILON {
            continue;
        }
        let sign = winding.signum();
        let point = BoundaryPoint {
            x: t * ux,
            y: t * uy,
            normal_x: sign * ey / edge_length,
            normal_y: -sign * ex / edge_length,
        };
        if best.map_or(true, |(best_t, _)| t < best_t) {
            best = Some((t, point))
        }
    }
    best.map(|(_, p)| p)
}
fn line_endpoint_toward(length: f64, target_x: f64, target_y: f64) -> Option<BoundaryPoint> {
    let half = length / 2.0;
    let candidates = [(-half, 0.0, -1.0, 0.0), (half, 0.0, 1.0, 0.0)];
    let (ux, uy) = normalized(target_x, target_y)?;
    candidates
        .into_iter()
        .min_by(|a, b| {
            let da = a.0 * ux + a.1 * uy;
            let db = b.0 * ux + b.1 * uy;
            db.partial_cmp(&da).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(x, y, nx, ny)| BoundaryPoint {
            x,
            y,
            normal_x: nx,
            normal_y: ny,
        })
}
fn multi_contact_boundary_toward(
    boundary: &crate::capillary_geometry::MultiContactFluidBoundary,
    target_x: f64,
    target_y: f64,
) -> Option<BoundaryPoint> {
    let (ux, uy) = normalized(target_x, target_y)?;
    if !boundary.is_valid() {
        return None;
    }
    let mut best: Option<(f64, BoundaryPoint)> = None;
    for piece in &boundary.pieces {
        let candidate = match piece {
            crate::capillary_geometry::FluidBoundaryPiece::SolidContact {
                start_x,
                start_y,
                end_x,
                end_y,
                ..
            } => {
                let ex = end_x - start_x;
                let ey = end_y - start_y;
                let denominator = ux * ey - uy * ex;
                if denominator.abs() <= 1e-12 {
                    continue;
                }
                let t = (start_x * ey - start_y * ex) / denominator;
                let along = (start_x * uy - start_y * ux) / denominator;
                if t <= 1e-12 || !(-1e-10..=1.0000000001).contains(&along) {
                    continue;
                }
                let length = ex.hypot(ey);
                if length <= f64::EPSILON {
                    continue;
                }
                Some((
                    t,
                    BoundaryPoint {
                        x: t * ux,
                        y: t * uy,
                        // Valid multi-contact boundaries have positive signed
                        // area, so their ordered pieces are counter-clockwise.
                        normal_x: ey / length,
                        normal_y: -ex / length,
                    },
                ))
            }
            crate::capillary_geometry::FluidBoundaryPiece::FreeArc(arc) => arc
                .ray_intersection(0.0, 0.0, ux, uy)
                .and_then(|(t, x, y)| {
                    let angle = (y - arc.center_y).atan2(x - arc.center_x);
                    let (normal_x, normal_y) = arc.normal_at_angle(angle)?;
                    Some((
                        t,
                        BoundaryPoint {
                            x,
                            y,
                            normal_x,
                            normal_y,
                        },
                    ))
                }),
        };
        if let Some((t, point)) = candidate {
            if best.as_ref().is_none_or(|(best_t, _)| t < *best_t) {
                best = Some((t, point));
            }
        }
    }
    best.map(|(_, point)| point)
}
pub fn boundary_point_at_multi_contact(
    boundary: &crate::capillary_geometry::MultiContactFluidBoundary,
    x: f64,
    y: f64,
) -> Option<BoundaryPoint> {
    if !boundary.is_valid() || !x.is_finite() || !y.is_finite() {
        return None;
    }
    let mut best: Option<(f64, BoundaryPoint)> = None;
    for piece in &boundary.pieces {
        let candidate = match piece {
            crate::capillary_geometry::FluidBoundaryPiece::SolidContact {
                start_x,
                start_y,
                end_x,
                end_y,
                ..
            } => {
                let dx = end_x - start_x;
                let dy = end_y - start_y;
                let length_squared = dx * dx + dy * dy;
                if length_squared <= f64::EPSILON {
                    continue;
                }
                let t =
                    (((x - start_x) * dx + (y - start_y) * dy) / length_squared).clamp(0.0, 1.0);
                let px = start_x + t * dx;
                let py = start_y + t * dy;
                let length = length_squared.sqrt();
                Some(BoundaryPoint {
                    x: px,
                    y: py,
                    normal_x: dy / length,
                    normal_y: -dx / length,
                })
            }
            crate::capillary_geometry::FluidBoundaryPiece::FreeArc(arc) => {
                let Some((px, py)) = arc.closest_point(x, y) else {
                    continue;
                };
                let angle = (py - arc.center_y).atan2(px - arc.center_x);
                let Some((normal_x, normal_y)) = arc.normal_at_angle(angle) else {
                    continue;
                };
                Some(BoundaryPoint {
                    x: px,
                    y: py,
                    normal_x,
                    normal_y,
                })
            }
        };
        let Some(candidate) = candidate else {
            continue;
        };
        let distance_squared = (x - candidate.x).powi(2) + (y - candidate.y).powi(2);
        if best.is_none_or(|(best_distance, _)| distance_squared < best_distance) {
            best = Some((distance_squared, candidate));
        }
    }
    let (distance_squared, point) = best?;
    let tolerance = 1e-7 * boundary.bounding_radius().max(1.0);
    (distance_squared.sqrt() <= tolerance).then_some(point)
}

pub fn boundary_point_toward(shape: &Shape, target_x: f64, target_y: f64) -> Option<BoundaryPoint> {
    match &shape.form {
        Form::Circle { radius } => {
            let (ux, uy) = normalized(target_x, target_y)?;
            Some(BoundaryPoint {
                x: radius * ux,
                y: radius * uy,
                normal_x: ux,
                normal_y: uy,
            })
        }
        Form::Line { length } => line_endpoint_toward(length.to_owned(), target_x, target_y),
        Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. } => {
            polygon_boundary_toward(
                shape.form.polygon_vertices()?.as_slice(),
                target_x,
                target_y,
            )
        }
        Form::Fluid {
            boundary,
            arc_boundary,
            multi_contact_boundary,
            ..
        } => {
            if let Some(multi) = multi_contact_boundary {
                multi_contact_boundary_toward(multi, target_x, target_y)
            } else if let Some(arc) = arc_boundary {
                let (ux, uy) = normalized(target_x, target_y)?;
                let (_, x, y) = arc.ray_intersection(0.0, 0.0, ux, uy)?;
                let angle = (y - arc.center_y).atan2(x - arc.center_x);
                let (normal_x, normal_y) = arc.normal_at_angle(angle)?;
                Some(BoundaryPoint {
                    x,
                    y,
                    normal_x,
                    normal_y,
                })
            } else {
                boundary.as_ref().and_then(|vertices| {
                    polygon_boundary_toward(vertices.as_slice(), target_x, target_y)
                })
            }
        }
    }
}
#[allow(dead_code)]
pub fn segment_endpoints(
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
) -> Option<(BoundaryPoint, BoundaryPoint)> {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let (nx, ny) = normalized(dx, dy)?;
    Some((
        BoundaryPoint {
            x: x0,
            y: y0,
            normal_x: -nx,
            normal_y: -ny,
        },
        BoundaryPoint {
            x: x1,
            y: y1,
            normal_x: nx,
            normal_y: ny,
        },
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn circle() -> Shape {
        Shape {
            form: Form::Circle { radius: 2.0 },
        }
    }
    #[test]
    fn multi_contact_endpoint_resolution_preserves_exact_contact_point() {
        use crate::capillary_geometry::{
            solve_water_against_collinear_contacts, CapillaryContactFamily,
        };

        let area = 0.5;
        let family = CapillaryContactFamily::solve(area, std::f64::consts::FRAC_PI_2).unwrap();
        let third = family.contact_length / 3.0;
        let segments = [
            ((-family.contact_length * 0.5, 0.0), (-third, 0.0)),
            ((-third, 0.0), (third, 0.0)),
            ((third, 0.0), (family.contact_length * 0.5, 0.0)),
        ];
        let boundary =
            solve_water_against_collinear_contacts(area, family.contact_angle_radians, &segments)
                .unwrap();
        let point =
            boundary_point_at_multi_contact(&boundary, -family.contact_length * 0.25, 0.0).unwrap();
        assert!((point.x + family.contact_length * 0.25).abs() < 1e-12);
        assert!(point.y.abs() < 1e-12);
        assert!(point.normal_y < -0.99);
        assert!(point.normal_x.abs() < 1e-12);
        assert!(boundary_point_at_multi_contact(&boundary, 100.0, 100.0).is_none());
    }

    #[test]
    fn circle_boundary_is_derived_from_requested_direction() {
        let p = boundary_point_toward(&circle(), 3.0, 4.0).unwrap();
        assert!((p.x - 1.2).abs() < 1e-12);
        assert!((p.y - 1.6).abs() < 1e-12);
        assert!((p.normal_x - 0.6).abs() < 1e-12);
        assert!((p.normal_y - 0.8).abs() < 1e-12)
    }
    #[test]
    fn polygon_boundary_uses_first_ray_intersection() {
        let s = Shape {
            form: Form::Rectangle {
                width: 2.0,
                height: 4.0,
            },
        };
        let p = boundary_point_toward(&s, 3.0, 1.0).unwrap();
        assert!((p.x - 1.0).abs() < 1e-12);
        assert!((p.y - 1.0 / 3.0).abs() < 1e-12)
    }
    #[test]
    fn clockwise_polygon_still_produces_an_outward_normal() {
        let s = Shape {
            form: Form::Polygon {
                vertices: vec![(1.0, 2.0), (1.0, -2.0), (-1.0, -2.0), (-1.0, 2.0)],
            },
        };
        let p = boundary_point_toward(&s, 3.0, 1.0).unwrap();
        assert!(p.normal_x > 0.99);
        assert!(p.normal_y.abs() < 0.01)
    }
    #[test]
    fn line_returns_the_first_endpoint_in_requested_direction() {
        let s = Shape {
            form: Form::Line { length: 4.0 },
        };
        let p = boundary_point_toward(&s, -1.0, 0.1).unwrap();
        assert_eq!(p.x, -2.0);
        let q = boundary_point_toward(&s, 1.0, 0.1).unwrap();
        assert_eq!(q.x, 2.0)
    }
    #[test]
    fn zero_direction_does_not_invent_a_boundary_location() {
        assert!(boundary_point_toward(&circle(), 0.0, 0.0).is_none())
    }
    #[test]
    fn fluid_placeholder_has_no_boundary_until_realized_geometry_exists() {
        let s = Shape {
            form: Form::Fluid {
                nominal_area: 0.5,
                boundary: None,
                arc_boundary: None,
                multi_contact_boundary: None,
            },
        };
        assert!(boundary_point_toward(&s, 1.0, 0.0).is_none())
    }
    #[test]
    fn exact_fluid_arc_boundary_query_does_not_sample_polygon_vertices() {
        let family = crate::capillary_geometry::CapillaryContactFamily::solve(
            0.5,
            std::f64::consts::FRAC_PI_2,
        )
        .unwrap();
        let arc =
            crate::capillary_geometry::CircularArcBoundary::from_flat_contact(family).unwrap();
        let shape = Shape {
            form: Form::Fluid {
                nominal_area: 0.5,
                boundary: None,
                arc_boundary: Some(arc),
                multi_contact_boundary: None,
            },
        };
        assert!(shape.is_valid());
        assert!(shape.form.polygon_vertices().is_none());

        let point = boundary_point_toward(&shape, 0.0, 1.0).unwrap();
        assert!((point.x).abs() < 1e-10);
        assert!((point.y - arc.radius).abs() < 1e-10);
        assert!((point.normal_x).abs() < 1e-10);
        assert!((point.normal_y - 1.0).abs() < 1e-10);
    }

    #[test]
    fn multi_contact_fluid_boundary_query_uses_exact_segments_and_outward_normals() {
        let shape = Shape {
            form: Form::Fluid {
                nominal_area: 4.0,
                boundary: None,
                arc_boundary: None,
                multi_contact_boundary: Some(
                    crate::capillary_geometry::MultiContactFluidBoundary {
                        nominal_area: 4.0,
                        pieces: vec![
                            crate::capillary_geometry::FluidBoundaryPiece::SolidContact {
                                start_x: -1.0,
                                start_y: -1.0,
                                end_x: 1.0,
                                end_y: -1.0,
                                contact_angle_radians: std::f64::consts::FRAC_PI_2,
                            },
                            crate::capillary_geometry::FluidBoundaryPiece::SolidContact {
                                start_x: 1.0,
                                start_y: -1.0,
                                end_x: 1.0,
                                end_y: 1.0,
                                contact_angle_radians: std::f64::consts::FRAC_PI_2,
                            },
                            crate::capillary_geometry::FluidBoundaryPiece::SolidContact {
                                start_x: 1.0,
                                start_y: 1.0,
                                end_x: -1.0,
                                end_y: 1.0,
                                contact_angle_radians: std::f64::consts::FRAC_PI_2,
                            },
                            crate::capillary_geometry::FluidBoundaryPiece::SolidContact {
                                start_x: -1.0,
                                start_y: 1.0,
                                end_x: -1.0,
                                end_y: -1.0,
                                contact_angle_radians: std::f64::consts::FRAC_PI_2,
                            },
                        ],
                    },
                ),
            },
        };
        assert!(shape.is_valid());
        let right = boundary_point_toward(&shape, 1.0, 0.0).unwrap();
        assert!((right.x - 1.0).abs() < 1e-12);
        assert!(right.normal_x > 0.99);
        assert!(right.normal_y.abs() < 1e-12);

        let top = boundary_point_toward(&shape, 0.0, 1.0).unwrap();
        assert!((top.y - 1.0).abs() < 1e-12);
        assert!(top.normal_y > 0.99);
        assert!(top.normal_x.abs() < 1e-12);
    }

    #[test]
    fn segment_endpoints_are_distinct_physical_locations() {
        let (a, b) = segment_endpoints(-1.0, 0.0, 1.0, 0.0).unwrap();
        assert_eq!(a.x, -1.0);
        assert_eq!(b.x, 1.0);
        assert_eq!((a.normal_x, a.normal_y), (-1.0, 0.0));
        assert_eq!((b.normal_x, b.normal_y), (1.0, 0.0))
    }
}
