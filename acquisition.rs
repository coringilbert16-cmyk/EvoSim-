//! Physical acquisition target resolution and authorization.
//!
//! This module maps an active-field resource observation to the immutable
//! geometry of its resource type and the physical location of the field cell.
//! It does not transfer material or decide which action an organism should take.

use crate::environment::ActiveMaterialField;
use crate::physical_space::acquisition_is_eligible;
use crate::resources::{BaseResource, Shape};
use crate::state::Position;
use crate::structural_blueprint::BlueprintPhysicalSpace;
use crate::structure::Placement;

pub(crate) fn resolve_field_target(
    catalog: &[BaseResource],
    field: &ActiveMaterialField,
    resource_name: &str,
    field_index: usize,
) -> Option<(Shape, Placement)> {
    let resource = catalog.iter().find(|resource| resource.name == resource_name)?;
    if field_index >= field.cells.len() {
        return None;
    }
    let (x, y) = field.cell_center(field_index);
    Some((resource.shape.clone(), Placement { x, y, rotation_radians: 0.0 }))
}

pub(crate) fn field_target_is_eligible(
    physical_space: &BlueprintPhysicalSpace,
    catalog: &[BaseResource],
    field: &ActiveMaterialField,
    resource_name: &str,
    field_index: usize,
    organism_position: Position,
) -> bool {
    let Some((shape, target_placement)) = resolve_field_target(catalog, field, resource_name, field_index) else {
        return false;
    };
    if !organism_position.x.is_finite() || !organism_position.y.is_finite() {
        return false;
    }
    acquisition_is_eligible(
        physical_space,
        &shape,
        Placement {
            x: target_placement.x - organism_position.x,
            y: target_placement.y - organism_position.y,
            rotation_radians: target_placement.rotation_radians,
        },
    )
}

/// Returns whether a concrete field target is authorized for acquisition.
///
/// Authorization requires actual material of the requested resource type in
/// the field cell and complete physical enclosure by the organism's inherited
/// boundary. This is a pure gate: it does not transfer material, break bonds,
/// or change organism state.
pub(crate) fn field_target_is_authorized(
    physical_space: &BlueprintPhysicalSpace,
    catalog: &[BaseResource],
    field: &ActiveMaterialField,
    resource_name: &str,
    field_index: usize,
    organism_position: Position,
) -> bool {
    if field_index >= field.cells.len() {
        return false;
    }
    let cell = &field.cells[field_index];
    let present = cell
        .bonded
        .parts
        .iter()
        .chain(cell.unbonded.parts.iter())
        .any(|(name, amount)| name == resource_name && amount.is_finite() && *amount > 0.0);
    present
        && field_target_is_eligible(
            physical_space,
            catalog,
            field,
            resource_name,
            field_index,
            organism_position,
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::ActiveMaterialField;
    use crate::resources::{default_catalog, Material};

    fn physical_space(radius: f64) -> BlueprintPhysicalSpace {
        BlueprintPhysicalSpace { boundary: Shape { form: crate::resources::Form::Circle { radius } } }
    }

    #[test]
    fn field_target_uses_catalog_geometry_and_cell_center() {
        let catalog = default_catalog();
        let field = ActiveMaterialField::new(100.0, 100.0, 25.0);
        let (shape, placement) = resolve_field_target(&catalog, &field, "Carbon", 5).expect("target should resolve");
        let expected_shape = catalog.iter().find(|resource| resource.name == "Carbon").expect("Carbon should exist").shape.form.clone();
        assert_eq!(placement.x, 37.5);
        assert_eq!(placement.y, 37.5);
        assert_eq!(shape.form, expected_shape);
    }

    #[test]
    fn field_target_eligibility_uses_organism_local_coordinates() {
        let catalog = default_catalog();
        let field = ActiveMaterialField::new(100.0, 100.0, 25.0);
        assert!(field_target_is_eligible(&physical_space(100.0), &catalog, &field, "Carbon", 5, Position { x: 37.5, y: 37.5 }));
        assert!(!field_target_is_eligible(&physical_space(10.0), &catalog, &field, "Carbon", 5, Position { x: 0.0, y: 0.0 }));
    }

    #[test]
    fn field_target_authorization_requires_material_and_enclosure() {
        let catalog = default_catalog();
        let mut field = ActiveMaterialField::new(100.0, 100.0, 25.0);
        let position = Position { x: 37.5, y: 37.5 };
        assert!(!field_target_is_authorized(&physical_space(100.0), &catalog, &field, "Carbon", 5, position.clone()));
        field.deposit_at_index(5, Material::free_base("Carbon", 1.0));
        assert!(field_target_is_authorized(&physical_space(100.0), &catalog, &field, "Carbon", 5, position));
        assert!(!field_target_is_authorized(&physical_space(10.0), &catalog, &field, "Carbon", 5, Position { x: 0.0, y: 0.0 }));
    }

    #[test]
    fn unknown_resource_does_not_resolve() {
        let catalog = default_catalog();
        let field = ActiveMaterialField::new(100.0, 100.0, 25.0);
        assert!(resolve_field_target(&catalog, &field, "NotAResource", 0).is_none());
    }

    #[test]
    fn invalid_field_index_does_not_resolve() {
        let catalog = default_catalog();
        let field = ActiveMaterialField::new(100.0, 100.0, 25.0);
        assert!(resolve_field_target(&catalog, &field, "Carbon", field.cells.len()).is_none());
    }
}
