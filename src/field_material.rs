//! Spatially represented material held by the active ecological field.
//!
//! `Material` remains the authoritative chemical/compositional identity. A
//! `FieldMaterial` adds the physical realization of that material so spatial
//! interaction can be decided from actual geometry rather than field-cell
//! membership alone.

use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

use crate::material_geometry::PhysicalMaterialInstance;
use crate::resources::Material;
use crate::structure::Placement;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FieldMaterial {
    pub id: u64,
    pub material: Material,
    /// For free material, one placement is retained per constituent entry.
    /// For structured material, the material itself is the physical object and
    /// its rigid external geometry is represented by one authoritative unit
    /// placement. Internal bonds do not imply independently placed geometry.
    pub placements: Vec<Placement>,
}

impl Deref for FieldMaterial {
    type Target = Material;
    fn deref(&self) -> &Self::Target { &self.material }
}

impl DerefMut for FieldMaterial {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.material }
}

impl FieldMaterial {
    pub fn new(id: u64, material: Material, placements: Vec<Placement>) -> Option<Self> {
        if material.is_empty() || !material.is_valid() {
            return None;
        }
        let valid_placement_count = if material.has_internal_structure() {
            placements.len() == 1
        } else {
            placements.len() == material.parts.len()
        };
        if !valid_placement_count {
            return None;
        }
        Some(Self { id, material, placements })
    }

    pub fn physical_instance(
        &self,
        catalog: &[crate::resources::BaseResource],
    ) -> Option<PhysicalMaterialInstance> {
        PhysicalMaterialInstance::new(self.material.clone(), &self.placements, catalog)
    }

    pub fn translated(&self, dx: f64, dy: f64) -> Option<Self> {
        if !dx.is_finite() || !dy.is_finite() { return None; }
        let placements = self.placements.iter().map(|p| Placement {
            x: p.x + dx,
            y: p.y + dy,
            rotation_radians: p.rotation_radians,
        }).collect();
        Self::new(self.id, self.material.clone(), placements)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::{default_catalog, InternalBond};

    #[test]
    fn field_material_retains_spatial_identity() {
        let material = Material::free_base("Carbon", 1.0);
        let placement = Placement { x: 12.0, y: -4.0, rotation_radians: 0.5 };
        let field = FieldMaterial::new(7, material, vec![placement]).unwrap();
        assert_eq!(field.id, 7);
        assert_eq!(field.placements.len(), 1);
        assert!(field.physical_instance(&default_catalog()).is_some());
    }

    #[test]
    fn structured_material_uses_one_authoritative_unit_placement() {
        let material = Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![InternalBond { part_a: 0, part_b: 1 }],
        };
        assert!(FieldMaterial::new(
            1,
            material.clone(),
            vec![Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 }],
        ).is_some());
        assert!(FieldMaterial::new(1, material, vec![]).is_none());
    }

    #[test]
    fn translation_preserves_relative_geometry() {
        let material = Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![InternalBond { part_a: 0, part_b: 1 }],
        };
        let field = FieldMaterial::new(
            4,
            material,
            vec![
                Placement { x: 10.0, y: 20.0, rotation_radians: 0.0 },
            ],
        ).unwrap();
        let moved = field.translated(25.0, -5.0).unwrap();
        assert_eq!(moved.id, 4);
        assert_eq!(moved.placements[0].x, 35.0);
        assert_eq!(moved.placements[0].y, 15.0);
    }

    #[test]
    fn material_fields_remain_accessible_through_deref() {
        let field = FieldMaterial::new(
            1,
            Material::free_base("Carbon", 2.0),
            vec![Placement { x: 1.0, y: 2.0, rotation_radians: 0.0 }],
        ).unwrap();
        assert_eq!(field.parts[0].0, "Carbon");
        assert_eq!(field.total_amount(), 2.0);
    }
}
