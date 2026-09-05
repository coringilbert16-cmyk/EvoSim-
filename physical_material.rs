//! A physically realized environmental material instance.
//!
//! `Material` owns composition + structure. `MaterialGeometry` owns the
//! spatial realization. This wrapper keeps those two facts together without
//! changing material identity.

use crate::material_geometry::MaterialGeometry;
use crate::resources::{BaseResource, Material};
use crate::structure::Placement;

#[derive(Clone, Debug, PartialEq)]
pub struct PhysicalMaterialInstance {
    pub material: Material,
    pub geometry: MaterialGeometry,
}

impl PhysicalMaterialInstance {
    pub fn new(
        material: Material,
        placements: &[Placement],
        catalog: &[BaseResource],
    ) -> Option<Self> {
        let geometry = MaterialGeometry::new(&material, placements, catalog)?;
        Some(Self { material, geometry })
    }

    pub fn is_valid(&self) -> bool {
        self.material.is_valid()
            && self.geometry.parts.len() == self.material.parts.len()
            && self.geometry.parts.iter().enumerate().all(|(index, part)| {
                part.part_index == index && part.form.is_valid()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;

    #[test]
    fn instance_keeps_material_identity_separate_from_geometry() {
        let material = Material::free_base("Carbon", 1.0);
        let placements = [Placement {
            x: 10.0,
            y: 20.0,
            rotation_radians: 0.0,
        }];
        let instance =
            PhysicalMaterialInstance::new(material.clone(), &placements, &default_catalog())
                .unwrap();

        assert_eq!(instance.material, material);
        assert_eq!(instance.geometry.parts[0].placement, placements[0]);
        assert!(instance.is_valid());
    }
}
