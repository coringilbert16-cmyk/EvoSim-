//! Spatially represented material held by the active ecological field.
//!
//! `Material` remains the authoritative chemical/compositional identity. A
//! `FieldMaterial` adds the physical realization of that material so spatial
//! interaction can be decided from actual geometry rather than field-cell
//! membership alone.

use serde::{Deserialize, Serialize};

use crate::material_geometry::PhysicalMaterialInstance;
use crate::resources::Material;
use crate::structure::Placement;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FieldMaterial {
    pub id: u64,
    pub material: Material,
    /// One placement for each constituent entry in `material.parts`.
    /// Structured material therefore cannot be represented without explicit
    /// constituent geometry; no implicit internal arrangement is created here.
    pub placements: Vec<Placement>,
}

impl FieldMaterial {
    pub fn new(id: u64, material: Material, placements: Vec<Placement>) -> Option<Self> {
        if material.is_empty() || !material.is_valid() || placements.len() != material.parts.len() {
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
        if !dx.is_finite() || !dy.is_finite() {
            return None;
        }
        let placements = self
            .placements
            .iter()
            .map(|p| Placement {
                x: p.x + dx,
                y: p.y + dy,
                rotation_radians: p.rotation_radians,
            })
            .collect();
        Self::new(self.id, self.material.clone(), placements)
    }

    pub fn with_material_and_placements(
        id: u64,
        material: Material,
        placements: Vec<Placement>,
    ) -> Option<Self> {
        Self::new(id, material, placements)
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
    fn structured_material_requires_one_placement_per_constituent() {
        let material = Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![InternalBond { part_a: 0, part_b: 1 }],
        };
        assert!(FieldMaterial::new(1, material.clone(), vec![Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 }]).is_none());
        assert!(FieldMaterial::new(
            1,
            material,
            vec![
                Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 },
                Placement { x: 1.0, y: 0.0, rotation_radians: 0.0 },
            ],
        ).is_some());
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
                Placement { x: 11.0, y: 20.0, rotation_radians: 0.0 },
            ],
        ).unwrap();
        let moved = field.translated(25.0, -5.0).unwrap();
        assert_eq!(moved.id, 4);
        assert_eq!(moved.placements[0].x, 35.0);
        assert_eq!(moved.placements[0].y, 15.0);
        assert_eq!(moved.placements[1].x, 36.0);
        assert_eq!(moved.placements[1].y, 15.0);
    }
}
