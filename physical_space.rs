//! Authoritative containment queries for inherited organism physical space.
//!
//! This module deliberately does not perform acquisition. It answers only the
//! geometric question required by acquisition: whether an entire target shape
//! is enclosed by the organism's inherited physical boundary.

use crate::resources::{Form, Shape};
use crate::structural_blueprint::BlueprintPhysicalSpace;
use crate::structure::Placement;

const EPSILON: f64 = 1e-9;

/// Returns true only when the entire target shape is contained by the
/// inherited organism boundary.
///
/// The current inherited organism boundary is a circle centered at the
/// organism's local origin. Rigid target shapes are tested by transforming
/// every vertex into the boundary's coordinate system. Circular and fluid
/// targets are tested by their radius. No overlap area or proportional
/// acquisition is involved.
pub(crate) fn contains_shape(
    physical_space: &BlueprintPhysicalSpace,
    target: &Shape,
    placement: Placement,
) -> bool {
    let Form::Circle { radius } = physical_space.boundary.form else {
        return false;
    };
    if !radius.is_finite() || radius <= 0.0 || !target.is_valid() {
        return false;
    }

    match &target.form {
        Form::Circle { radius: target_radius } => {
            let center_distance = (placement.x.powi(2) + placement.y.powi(2)).sqrt();
            center_distance.is_finite()
                && target_radius.is_finite()
                && center_distance + *target_radius <= radius + EPSILON
        }
        Form::Fluid { nominal_area } => {
            let target_radius = (*nominal_area / std::f64::consts::PI).max(0.0).sqrt();
            let center_distance = (placement.x.powi(2) + placement.y.powi(2)).sqrt();
            center_distance.is_finite()
                && target_radius.is_finite()
                && center_distance + target_radius <= radius + EPSILON
        }
        _ => {
            let Some(vertices) = target.form.polygon_vertices() else {
                return false;
            };
            let (sin, cos) = placement.rotation_radians.sin_cos();
            vertices.into_iter().all(|(x, y)| {
                let world_x = placement.x + x * cos - y * sin;
                let world_y = placement.y + x * sin + y * cos;
                world_x.is_finite()
                    && world_y.is_finite()
                    && world_x.powi(2) + world_y.powi(2) <= radius.powi(2) + EPSILON
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;

    fn space(radius: f64) -> BlueprintPhysicalSpace {
        BlueprintPhysicalSpace {
            boundary: Shape { form: Form::Circle { radius } },
        }
    }

    #[test]
    fn target_fully_inside_boundary_is_contained() {
        let target = default_catalog().iter().find(|resource| resource.name == "Carbon").unwrap().shape.clone();
        assert!(contains_shape(&space(2.0), &target, Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 }));
    }

    #[test]
    fn partial_overlap_is_not_containment() {
        let target = Shape { form: Form::Circle { radius: 1.0 } };
        assert!(!contains_shape(&space(1.5), &target, Placement { x: 0.75, y: 0.0, rotation_radians: 0.0 }));
    }

    #[test]
    fn boundary_touch_counts_as_full_containment() {
        let target = Shape { form: Form::Circle { radius: 1.0 } };
        assert!(contains_shape(&space(2.0), &target, Placement { x: 1.0, y: 0.0, rotation_radians: 0.0 }));
    }

    #[test]
    fn rotation_is_considered_for_rigid_shapes() {
        let target = Shape { form: Form::Rectangle { width: 1.0, height: 0.5 } };
        assert!(contains_shape(&space(1.0), &target, Placement { x: 0.0, y: 0.0, rotation_radians: std::f64::consts::FRAC_PI_2 }));
    }
}
