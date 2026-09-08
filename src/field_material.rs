//! Spatially represented material held by the active ecological field.
//!
//! The field's inventory remains the conservation authority, while each
//! instance also has a physical placement so organism acquisition can be
//! decided from geometry rather than a cell-center shortcut.

use serde::{Deserialize, Serialize};

use crate::material_geometry::PhysicalMaterialInstance;
use crate::resources::Material;
use crate::structure::Placement;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FieldMaterial {
    pub id: u64,
    pub material: Material,
    pub placement: Placement,
}

impl FieldMaterial {
    pub fn new(id: u64, material: Material, placement: Placement) -> Self {
        Self { id, material, placement }
    }

    pub fn physical_instance(
        &self,
        catalog: &[crate::resources::BaseResource],
    ) -> Option<PhysicalMaterialInstance> {
        PhysicalMaterialInstance::new(self.material.clone(), &[self.placement.clone()], catalog)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::{default_catalog, Material};

    #[test]
    fn field_material_retains_spatial_identity() {
        let material = Material::free_base("Carbon", 1.0);
        let placement = Placement { x: 12.0, y: -4.0, rotation_radians: 0.5 };
        let field = FieldMaterial::new(7, material, placement.clone());
        assert_eq!(field.id, 7);
        assert_eq!(field.placement, placement);
        assert!(field.physical_instance(&default_catalog()).is_some());
    }
}
