//! Transitional containment gate.
//!
//! The organism's physical boundary is no longer defined by blueprint-owned
//! geometry. Until containment can be derived from the live OrganismStructure,
//! this module deliberately refuses acquisition eligibility rather than using
//! the legacy circular boundary as a source of physical truth.

use crate::resources::Shape;
use crate::structural_blueprint::BlueprintPhysicalSpace;
use crate::structure::Placement;

pub(crate) fn contains_shape(
    _physical_space: &BlueprintPhysicalSpace,
    _target: &Shape,
    _placement: Placement,
) -> bool {
    false
}

pub(crate) fn acquisition_is_eligible(
    physical_space: &BlueprintPhysicalSpace,
    target: &Shape,
    placement: Placement,
) -> bool {
    contains_shape(physical_space, target, placement)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::{default_catalog, Form, Shape};

    fn legacy_space() -> BlueprintPhysicalSpace {
        BlueprintPhysicalSpace {
            boundary: Shape { form: Form::Circle { radius: 100.0 } },
        }
    }

    #[test]
    fn legacy_boundary_never_authorizes_containment() {
        let target = default_catalog()
            .iter()
            .find(|resource| resource.name == "Carbon")
            .unwrap()
            .shape
            .clone();
        assert!(!contains_shape(
            &legacy_space(),
            &target,
            Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 },
        ));
    }

    #[test]
    fn acquisition_is_not_authorized_by_legacy_boundary() {
        let target = Shape { form: Form::Circle { radius: 1.0 } };
        assert!(!acquisition_is_eligible(
            &legacy_space(),
            &target,
            Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 },
        ));
    }
}
