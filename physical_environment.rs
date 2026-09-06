//! Persistent physical environmental material instances.
//!
//! `ActiveMaterialField` remains the bulk ecological stock and spatial grid.
//! `PhysicalEnvironment` is the separate owner of material that has been
//! explicitly realized as a physical object with constituent geometry.

use crate::material_geometry::PhysicalMaterialInstance;
use crate::resources::{BaseResource, Material};
use crate::structure::Placement;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PhysicalEnvironment {
    pub materials: Vec<PhysicalMaterialInstance>,
}

impl PhysicalEnvironment {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.materials.len()
    }

    pub fn is_empty(&self) -> bool {
        self.materials.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&PhysicalMaterialInstance> {
        self.materials.get(index)
    }

    pub fn iter(&self) -> impl Iterator<Item = &PhysicalMaterialInstance> {
        self.materials.iter()
    }

    /// Realize an explicitly placed material as a persistent physical object.
    ///
    /// This operation does not remove material from bulk ecological stock.
    /// Callers must perform the corresponding bulk withdrawal only after
    /// successful realization, so failed geometry validation cannot destroy
    /// material.
    pub fn realize(
        &mut self,
        material: Material,
        placements: &[Placement],
        catalog: &[BaseResource],
    ) -> Result<usize, String> {
        let instance = PhysicalMaterialInstance::new(material, placements, catalog)
            .ok_or_else(|| "material cannot be physically realized".to_string())?;
        self.materials.push(instance);
        Ok(self.materials.len() - 1)
    }

    pub fn remove(&mut self, index: usize) -> Option<PhysicalMaterialInstance> {
        if index >= self.materials.len() {
            return None;
        }
        Some(self.materials.remove(index))
    }

    pub fn total_material_amount(&self) -> f64 {
        self.materials
            .iter()
            .map(|instance| instance.material.total_amount())
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;

    fn placement(x: f64, y: f64) -> Placement {
        Placement {
            x,
            y,
            rotation_radians: 0.0,
        }
    }

    #[test]
    fn realization_creates_one_persistent_physical_object() {
        let catalog = default_catalog();
        let mut environment = PhysicalEnvironment::new();
        let index = environment
            .realize(Material::free_base("Carbon", 5.0), &[placement(10.0, 20.0)], &catalog)
            .unwrap();

        assert_eq!(index, 0);
        assert_eq!(environment.len(), 1);
        assert_eq!(environment.get(0).unwrap().material.total_amount(), 5.0);
        assert_eq!(environment.get(0).unwrap().geometry.parts[0].placement, placement(10.0, 20.0));
    }

    #[test]
    fn failed_realization_does_not_create_an_object() {
        let catalog = default_catalog();
        let mut environment = PhysicalEnvironment::new();
        let result = environment.realize(
            Material::free_base("Carbon", 5.0),
            &[placement(10.0, 20.0)],
            &catalog,
        );
        assert!(result.is_ok());

        let result = environment.realize(
            Material::free_base("Carbon", 5.0),
            &[Placement {
                x: f64::NAN,
                y: 20.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        );
        assert!(result.is_err());
        assert_eq!(environment.len(), 1);
    }

    #[test]
    fn distinct_realizations_remain_distinct_objects() {
        let catalog = default_catalog();
        let mut environment = PhysicalEnvironment::new();
        environment
            .realize(Material::free_base("Carbon", 1.0), &[placement(0.0, 0.0)], &catalog)
            .unwrap();
        environment
            .realize(Material::free_base("Carbon", 1.0), &[placement(100.0, 0.0)], &catalog)
            .unwrap();

        assert_eq!(environment.len(), 2);
        assert_ne!(
            environment.get(0).unwrap().geometry.parts[0].placement,
            environment.get(1).unwrap().geometry.parts[0].placement
        );
    }

    #[test]
    fn removal_returns_the_physical_instance() {
        let catalog = default_catalog();
        let mut environment = PhysicalEnvironment::new();
        environment
            .realize(Material::free_base("Carbon", 2.0), &[placement(0.0, 0.0)], &catalog)
            .unwrap();

        let removed = environment.remove(0).unwrap();
        assert_eq!(removed.material.total_amount(), 2.0);
        assert!(environment.is_empty());
    }
}
