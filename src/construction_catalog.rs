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
    let Some(a_vertices) = resource_a.shape.form.polygon_vertices() else {
        return out;
    };
    let Some(b_vertices) = resource_b.shape.form.polygon_vertices() else {
        return out;
    };

    for a_index in 0..a_vertices.len() {
        let Some(a_normal) =
            crate::rigid_boundary::corner_normal(&resource_a.shape, a_index)
        else {
            continue;
        };
        let a_angle = a_normal.1.atan2(a_normal.0);
        let a_segment_length = incident_segment_length(&a_vertices, a_index);

        for b_index in 0..b_vertices.len() {
            let Some(b_normal) =
                crate::rigid_boundary::corner_normal(&resource_b.shape, b_index)
            else {
                continue;
            };
            let b_angle = b_normal.1.atan2(b_normal.0);
            let b_segment_length = incident_segment_length(&b_vertices, b_index);

            if !segment_lengths_compatible(a_segment_length, b_segment_length) {
                continue;
            }

            let rotation = b_angle + std::f64::consts::PI - a_angle;
            let Some(local_a) = crate::rigid_boundary::world_vertex(
                &resource_a.shape,
                a_index,
                Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: rotation,
                },
            ) else {
                continue;
            };

            out.push(PairFormation {
                resource_a: resource_a_index,
                resource_b: resource_b_index,
                endpoint_a: ConnectionEndpoint::Corner {
                    point_index: a_index,
                },
                endpoint_b: ConnectionEndpoint::Corner {
                    point_index: b_index,
                },
                placement_a_relative_to_b: RelativePlacement {
                    x: -local_a.0,
                    y: -local_a.1,
                    rotation_radians: rotation,
                },
                segment_length_a: a_segment_length,
                segment_length_b: b_segment_length,
            });
        }
    }

    out
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

    // A corner is represented by the two incident sides.  Use the shorter
    // side as the connection scale so a small side cannot accidentally be
    // treated as a larger attachment merely because its neighboring side is
    // long.
    previous_length.min(next_length)
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
    fn_two_scale_jump_cannot_bond() {
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
    fn fluid_resources_are_not_treated_as_rigid_pair_geometry() {
        let catalog = crate::resources::default_catalog();
        let construction = ConstructionCatalog::build(&catalog);
        let water = catalog.iter().position(|r| r.name == "Water").unwrap();
        let carbon = catalog.iter().position(|r| r.name == "Carbon").unwrap();
        assert_eq!(construction.pair_formations_for(water, carbon).count(), 0);
    }
}
