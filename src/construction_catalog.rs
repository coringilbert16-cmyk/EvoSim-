//! Precomputed local construction geometry.
//!
//! This module deliberately catalogs reusable local formations rather than
//! organism-scale blueprints.  The first layer is the finite set of rigid
//! pair placements that can be reused by the constructor without repeatedly
//! deriving the same corner/endpoint transforms.

use crate::resources::{BaseResource, Form, PhysicalState};
use crate::structure::{ConnectionEndpoint, Placement};

const GEOMETRY_TOLERANCE: f64 = 1.0e-10;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RelativePlacement {
    pub x: f64,
    pub y: f64,
    pub rotation_radians: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PairFormation {
    pub resource_a: usize,
    pub resource_b: usize,
    pub endpoint_a: ConnectionEndpoint,
    pub endpoint_b: ConnectionEndpoint,
    pub placement_a_relative_to_b: RelativePlacement,
    pub segment_length_a: f64,
    pub segment_length_b: f64,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ConstructionCatalog {
    pair_formations: Vec<PairFormation>,
}

impl ConstructionCatalog {
    pub(crate) fn build(resources: &[BaseResource]) -> Self {
        let mut pair_formations = Vec::new();

        for (a_index, a) in resources.iter().enumerate() {
            if a.physical_state != PhysicalState::Rigid || !a.shape.is_valid() {
                continue;
            }
            for (b_index, b) in resources.iter().enumerate() {
                if b.physical_state != PhysicalState::Rigid || !b.shape.is_valid() {
                    continue;
                }

                pair_formations.extend(generate_pair_formations(
                    a_index, a, b_index, b,
                ));
            }
        }

        pair_formations.sort_by(|left, right| {
            left.resource_a
                .cmp(&right.resource_a)
                .then(left.resource_b.cmp(&right.resource_b))
                .then_with(|| {
                    left.placement_a_relative_to_b
                        .rotation_radians
                        .partial_cmp(
                            &right
                                .placement_a_relative_to_b
                                .rotation_radians,
                        )
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });

        pair_formations.dedup_by(|left, right| {
            left.resource_a == right.resource_a
                && left.resource_b == right.resource_b
                && left.endpoint_a == right.endpoint_a
                && left.endpoint_b == right.endpoint_b
                && same_placement(
                    left.placement_a_relative_to_b,
                    right.placement_a_relative_to_b,
                )
        });

        Self { pair_formations }
    }

    pub(crate) fn placements_for_pair(
        &self,
        resource_a: usize,
        resource_b: usize,
        anchor: Placement,
    ) -> Vec<Placement> {
        let (s, c) = anchor.rotation_radians.sin_cos();
        let mut out = vec![anchor];
        for formation in self.pair_formations_for(resource_a, resource_b) {
            let relative = formation.placement_a_relative_to_b;
            out.push(Placement {
                x: anchor.x + relative.x * c - relative.y * s,
                y: anchor.y + relative.x * s + relative.y * c,
                rotation_radians: relative.rotation_radians + anchor.rotation_radians,
            });
        }
        out.sort_by(|a, b| {
            (a.x - anchor.x)
                .hypot(a.y - anchor.y)
                .partial_cmp(&(b.x - anchor.x).hypot(b.y - anchor.y))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        out.dedup_by(|a, b| same_placement(
            RelativePlacement { x: a.x, y: a.y, rotation_radians: a.rotation_radians },
            RelativePlacement { x: b.x, y: b.y, rotation_radians: b.rotation_radians },
        ));
        out
    }

    pub(crate) fn pair_formations_for(
        &self,
        resource_a: usize,
        resource_b: usize,
    ) -> impl Iterator<Item = &PairFormation> {
        self.pair_formations.iter().filter(move |formation| {
            formation.resource_a == resource_a
                && formation.resource_b == resource_b
                && segment_lengths_compatible(
                    formation.segment_length_a,
                    formation.segment_length_b,
                )
        })
    }

    pub(crate) fn len(&self) -> usize {
        self.pair_formations.len()
    }
}

/// Two attachment surfaces may match at the same scale or one adjacent scale.
/// A two-level jump, such as 1.0 to 0.25, is not a valid local connection.
pub(crate) fn segment_lengths_compatible(a: f64, b: f64) -> bool {
    const SCALES: [f64; 3] = [0.25, 0.5, 1.0];

    let nearest_a = nearest_scale(a);
    let nearest_b = nearest_scale(b);
    if (nearest_a - a).abs() > GEOMETRY_TOLERANCE
        || (nearest_b - b).abs() > GEOMETRY_TOLERANCE
    {
        return false;
    }

    let ia = SCALES
        .iter()
        .position(|scale| (*scale - nearest_a).abs() <= GEOMETRY_TOLERANCE);
    let ib = SCALES
        .iter()
        .position(|scale| (*scale - nearest_b).abs() <= GEOMETRY_TOLERANCE);

    matches!((ia, ib), (Some(a), Some(b)) if a.abs_diff(b) <= 1)
}

fn nearest_scale(length: f64) -> f64 {
    [0.25_f64, 0.5, 1.0]
        .into_iter()
        .min_by(|a, b| {
            (length - *a)
                .abs()
                .partial_cmp(&(length - *b).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(0.25)
}

fn generate_pair_formations(
    resource_a_index: usize,
    resource_a: &BaseResource,
    resource_b_index: usize,
    resource_b: &BaseResource,
) -> Vec<PairFormation> {
    let mut out = Vec::new();

    match (&resource_a.shape.form, &resource_b.shape.form) {
        (Form::Line { length: a_length }, Form::Line { length: b_length }) => {
            if segment_lengths_compatible(*a_length, *b_length) {
                for a_index in 0..2 {
                    for b_index in 0..2 {
                        for rotation in crate::rigid_boundary::line_endpoint_alignment_rotations(
                            a_index,
                            b_index,
                            0.0,
                        ) {
                            let a_x = if a_index == 0 { -*a_length / 2.0 } else { *a_length / 2.0 };
                            let b_x = if b_index == 0 { -*b_length / 2.0 } else { *b_length / 2.0 };
                            let (s, c) = rotation.sin_cos();
                            out.push(PairFormation {
                                resource_a: resource_a_index,
                                resource_b: resource_b_index,
                                endpoint_a: ConnectionEndpoint::LineEndpoint {
                                    point_index: a_index,
                                },
                                endpoint_b: ConnectionEndpoint::LineEndpoint {
                                    point_index: b_index,
                                },
                                placement_a_relative_to_b: RelativePlacement {
                                    x: b_x - a_x * c,
                                    y: -a_x * s,
                                    rotation_radians: rotation,
                                },
                                segment_length_a: *a_length,
                                segment_length_b: *b_length,
                            });
                        }
                    }
                }
            }
        }
        (
            Form::Line { length: a_length },
            Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. },
        ) => {
            let Some(vertices) = resource_b.shape.form.polygon_vertices() else {
                return out;
            };
            for b_index in 0..vertices.len() {
                let Some(normal) =
                    crate::rigid_boundary::corner_normal(&resource_b.shape, b_index)
                else {
                    continue;
                };
                let target_angle = normal.1.atan2(normal.0);
                for a_index in 0..2 {
                    let endpoint_angle =
                        crate::rigid_boundary::line_endpoint_normal(&resource_a.shape, a_index)
                            .map(|n| n.1.atan2(n.0))
                            .unwrap_or(0.0);
                    let rotation = target_angle + std::f64::consts::PI - endpoint_angle;
                    let local_x = if a_index == 0 { -*a_length / 2.0 } else { *a_length / 2.0 };
                    let (s, c) = rotation.sin_cos();
                    let lx = local_x * c;
                    let ly = local_x * s;
                    let target = vertices[b_index];
                    out.push(PairFormation {
                        resource_a: resource_a_index,
                        resource_b: resource_b_index,
                        endpoint_a: ConnectionEndpoint::LineEndpoint {
                            point_index: a_index,
                        },
                        endpoint_b: ConnectionEndpoint::Corner {
                            point_index: b_index,
                        },
                        placement_a_relative_to_b: RelativePlacement {
                            x: target.0 - lx,
                            y: target.1 - ly,
                            rotation_radians: rotation,
                        },
                        segment_length_a: *a_length,
                        segment_length_b: incident_segment_length(&vertices, b_index),
                    });
                }
            }
        }
        (
            Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. },
            Form::Line { length: b_length },
        ) => {
            for formation in generate_pair_formations(
                resource_b_index,
                resource_b,
                resource_a_index,
                resource_a,
            ) {
                out.push(PairFormation {
                    resource_a: resource_a_index,
                    resource_b: resource_b_index,
                    endpoint_a: formation.endpoint_b,
                    endpoint_b: formation.endpoint_a,
                    placement_a_relative_to_b: invert_relative_placement(
                        formation.placement_a_relative_to_b,
                    ),
                    segment_length_a: formation.segment_length_b,
                    segment_length_b: formation.segment_length_a,
                });
            }
        }
        (
            Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. },
            Form::Rectangle { .. } | Form::RegularPolygon { .. } | Form::Polygon { .. },
        ) => {
            let Some(a_vertices) = resource_a.shape.form.polygon_vertices() else {
                return out;
            };
            let Some(b_vertices) = resource_b.shape.form.polygon_vertices() else {
                return out;
            };

            for a_edge in 0..a_vertices.len() {
                let a0 = a_vertices[a_edge];
                let a1 = a_vertices[(a_edge + 1) % a_vertices.len()];
                let a_length = (a1.0 - a0.0).hypot(a1.1 - a0.1);
                if !is_supported_segment_length(a_length) {
                    continue;
                }

                for b_edge in 0..b_vertices.len() {
                    let b0 = b_vertices[b_edge];
                    let b1 = b_vertices[(b_edge + 1) % b_vertices.len()];
                    let b_length = (b1.0 - b0.0).hypot(b1.1 - b0.1);
                    if !is_supported_segment_length(b_length)
                        || !segment_lengths_compatible(a_length, b_length)
                    {
                        continue;
                    }

                    let a_angle = (a1.1 - a0.1).atan2(a1.0 - a0.0);
                    let b_angle = (b1.1 - b0.1).atan2(b1.0 - b0.0);
                    let rotation = b_angle + std::f64::consts::PI - a_angle;
                    let (s, c) = rotation.sin_cos();
                    let ax0 = a0.0 * c - a0.1 * s;
                    let ay0 = a0.0 * s + a0.1 * c;
                    let ax1 = a1.0 * c - a1.1 * s;
                    let ay1 = a1.0 * s + a1.1 * c;

                    for (a_endpoint, b_endpoint, ax, ay, bx, by) in [
                        (
                            a_edge,
                            b_edge,
                            ax0,
                            ay0,
                            b1.0,
                            b1.1,
                        ),
                        (
                            (a_edge + 1) % a_vertices.len(),
                            (b_edge + 1) % b_vertices.len(),
                            ax1,
                            ay1,
                            b0.0,
                            b0.1,
                        ),
                    ] {
                        out.push(PairFormation {
                            resource_a: resource_a_index,
                            resource_b: resource_b_index,
                            endpoint_a: ConnectionEndpoint::Corner {
                                point_index: a_endpoint,
                            },
                            endpoint_b: ConnectionEndpoint::Corner {
                                point_index: b_endpoint,
                            },
                            placement_a_relative_to_b: RelativePlacement {
                                x: bx - ax,
                                y: by - ay,
                                rotation_radians: rotation,
                            },
                            segment_length_a: a_length,
                            segment_length_b: b_length,
                        });
                    }
                }
            }
        }
        _ => {}
    }

    out
}

fn invert_relative_placement(relative: RelativePlacement) -> RelativePlacement {
    let (s, c) = relative.rotation_radians.sin_cos();
    RelativePlacement {
        x: -(relative.x * c + relative.y * s),
        y: relative.x * s - relative.y * c,
        rotation_radians: -relative.rotation_radians,
    }
}

fn incident_segment_length(vertices: &[(f64, f64)], vertex: usize) -> f64 {
    if vertices.len() < 2 {
        return 0.0;
    }

    let current = vertices[vertex];
    let previous = vertices[(vertex + vertices.len() - 1) % vertices.len()];
    let next = vertices[(vertex + 1) % vertices.len()];

    let previous_length = (current.0 - previous.0).hypot(current.1 - previous.1);
    let next_length = (next.0 - current.0).hypot(next.1 - current.1);

    previous_length.min(next_length)
}

fn is_supported_segment_length(length: f64) -> bool {
    [0.25_f64, 0.5, 1.0]
        .iter()
        .any(|scale| (length - *scale).abs() <= GEOMETRY_TOLERANCE)
}

fn same_placement(a: RelativePlacement, b: RelativePlacement) -> bool {
    (a.x - b.x).abs() <= GEOMETRY_TOLERANCE
        && (a.y - b.y).abs() <= GEOMETRY_TOLERANCE
        && (a.rotation_radians - b.rotation_radians).abs() <= GEOMETRY_TOLERANCE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjacent_segment_scales_can_bond() {
        assert!(segment_lengths_compatible(1.0, 1.0));
        assert!(segment_lengths_compatible(1.0, 0.5));
        assert!(segment_lengths_compatible(0.5, 0.25));
        assert!(segment_lengths_compatible(0.25, 0.25));
    }

    #[test]
    fn two_scale_jump_cannot_bond() {
        assert!(!segment_lengths_compatible(1.0, 0.25));
    }

    #[test]
    fn default_catalog_produces_reusable_rigid_pair_formations() {
        let catalog = crate::resources::default_catalog();
        let construction = ConstructionCatalog::build(&catalog);
        assert!(construction.len() > 0);

        let carbon = catalog.iter().position(|r| r.name == "Carbon").unwrap();
        let phosphorus = catalog.iter().position(|r| r.name == "Phosphorus").unwrap();
        assert!(construction
            .pair_formations_for(carbon, phosphorus)
            .next()
            .is_some());
    }

    #[test]
    fn catalog_contains_the_declared_side_scale_matches() {
        let catalog = crate::resources::default_catalog();
        let construction = ConstructionCatalog::build(&catalog);
        let nitrogen = catalog.iter().position(|r| r.name == "Nitrogen").unwrap();
        let phosphorus = catalog.iter().position(|r| r.name == "Phosphorus").unwrap();
        let carbon = catalog.iter().position(|r| r.name == "Carbon").unwrap();

        assert!(construction
            .pair_formations_for(nitrogen, phosphorus)
            .any(|formation| {
                (formation.segment_length_a - 0.5).abs() < 1e-9
                    && (formation.segment_length_b - 0.5).abs() < 1e-9
            }));
        assert!(construction
            .pair_formations_for(carbon, phosphorus)
            .any(|formation| {
                (formation.segment_length_a - 1.0).abs() < 1e-9
                    && (formation.segment_length_b - 0.5).abs() < 1e-9
            }));
    }

    #[test]
    fn fluid_resources_are_not_treated_as_rigid_pair_geometry() {
        let catalog = crate::resources::default_catalog();
        let construction = ConstructionCatalog::build(&catalog);
        let water = catalog.iter().position(|r| r.name == "Water").unwrap();
        let carbon = catalog.iter().position(|r| r.name == "Carbon").unwrap();
        assert_eq!(construction.pair_formations_for(water, carbon).count(), 0);
    }
}
