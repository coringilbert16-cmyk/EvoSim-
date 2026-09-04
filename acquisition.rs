//! Physical acquisition target resolution.
//!
//! This module maps an active-field resource observation to the immutable
//! geometry of its resource type and the physical location of the field cell.
//! It does not transfer material or decide whether acquisition is allowed.

use crate::resources::{BaseResource, Shape};
use crate::structure::Placement;

/// Resolves the physical shape and placement of a resource represented in an
/// active-field cell.
///
/// Resource geometry comes only from the immutable resource catalog. Position
/// comes only from the field cell containing the material. No overlap area,
/// organism state, or material amount participates in this resolution.
pub(crate) fn resolve_field_target(
    catalog: &[BaseResource],
    field: &crate::field::ActiveMaterialField,
    resource_name: &str,
    field_index: usize,
) -> Option<(Shape, Placement)> {
    let resource = catalog.iter().find(|resource| resource.name == resource_name)?;
    if field_index >= field.cells.len() {
        return None;
    }
    let (x, y) = field.cell_center(field_index);
    Some((
        resource.shape.clone(),
        Placement {
            x,
            y,
            rotation_radians: 0.0,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::ActiveMaterialField;
    use crate::resources::{default_catalog, Form};

    #[test]
    fn field_target_uses_catalog_geometry_and_cell_center() {
        let catalog = default_catalog();
        let field = ActiveMaterialField::new(100.0, 100.0, 25.0);
        let (shape, placement) =
            resolve_field_target(&catalog, &field, "Carbon", 5).expect("target should resolve");

        assert_eq!(placement.x, 37.5);
        assert_eq!(placement.y, 37.5);
        assert!(matches!(shape.form, Form::Circle { .. }));
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
