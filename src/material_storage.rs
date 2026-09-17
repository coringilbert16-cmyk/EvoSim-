//! Organism material inventory.
use crate::physical_material::PhysicalMaterial;
use crate::resources::Material;
use crate::structure::Placement;
use serde::{Deserialize, Serialize};

const MATERIAL_EPSILON: f64 = 1e-12;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub(crate) struct MaterialStorage {
    pub(crate) materials: Vec<Material>,
    #[serde(default)]
    pub(crate) physical_instances: Vec<Option<PhysicalMaterial>>,
}

impl MaterialStorage {
    fn normalize_legacy_alignment(&mut self) {
        while self.physical_instances.len() < self.materials.len() {
            self.physical_instances.push(None);
        }
        if self.physical_instances.len() > self.materials.len() {
            self.physical_instances.truncate(self.materials.len());
        }
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.materials.is_empty()
    }
    pub(crate) fn total_amount(&self) -> f64 {
        self.materials.iter().map(Material::total_amount).sum()
    }
    fn is_discrete(material: &Material) -> bool {
        material.parts.iter().all(|(_, amount)| {
            amount.is_finite() && *amount > 0.0 && amount.fract().abs() <= MATERIAL_EPSILON
        })
    }
    pub(crate) fn store(&mut self, material: Material) -> bool {
        if material.parts.is_empty() || !material.is_valid() || !Self::is_discrete(&material) {
            return false;
        }
        self.normalize_legacy_alignment();
        if material.has_internal_structure() {
            self.materials.push(material);
            self.physical_instances.push(None);
            return true;
        }
        for (name, amount) in material.parts {
            let count = amount.round() as u64;
            for _ in 0..count {
                self.materials.push(Material::free_base(name.clone(), 1.0));
                self.physical_instances.push(None);
            }
        }
        true
    }
    pub(crate) fn store_physical(
        &mut self,
        material: Material,
        placements: Vec<Placement>,
        catalog: &[crate::resources::BaseResource],
    ) -> bool {
        let instance = match PhysicalMaterial::realized(material.clone(), placements, catalog)
            .and_then(PhysicalMaterial::into_intrinsic_frame)
        {
            Some(instance) => instance,
            None => return false,
        };
        if material.parts.is_empty() || !material.is_valid() || !Self::is_discrete(&material) {
            return false;
        }
        self.normalize_legacy_alignment();
        self.materials.insert(0, material);
        self.physical_instances.insert(0, Some(instance));
        true
    }
    pub(crate) fn peek_one_unstructured(&self) -> Option<Material> {
        self.materials
            .iter()
            .find(|material| !material.has_internal_structure() && !material.is_empty())
            .cloned()
    }
    pub(crate) fn peek_matching_physical(&self, target: &Material) -> Option<PhysicalMaterial> {
        let index = self
            .materials
            .iter()
            .position(|material| material == target && !material.is_empty())?;
        self.physical_instances
            .get(index)
            .and_then(Clone::clone)
            .or_else(|| Some(PhysicalMaterial::logical(target.clone())))
    }
    pub(crate) fn take_one_unstructured_named(&mut self, name: &str) -> Option<Material> {
        let index = self.materials.iter().position(|material| {
            !material.has_internal_structure()
                && !material.is_empty()
                && material.parts.len() == 1
                && material.parts[0].0 == name
                && (material.parts[0].1 - 1.0).abs() <= MATERIAL_EPSILON
        })?;
        self.normalize_legacy_alignment();
        self.physical_instances.swap_remove(index);
        Some(self.materials.swap_remove(index))
    }
    pub(crate) fn take_matching(&mut self, target: &Material) -> Option<Material> {
        let index = self
            .materials
            .iter()
            .position(|material| material == target && !material.is_empty())?;
        self.normalize_legacy_alignment();
        self.physical_instances.swap_remove(index);
        Some(self.materials.swap_remove(index))
    }
    pub(crate) fn take_matching_physical(&mut self, target: &Material) -> Option<PhysicalMaterial> {
        let index = self
            .materials
            .iter()
            .position(|material| material == target && !material.is_empty())?;
        self.normalize_legacy_alignment();
        let material = self.materials.swap_remove(index);
        let instance = self.physical_instances.swap_remove(index);
        Some(instance.unwrap_or_else(|| PhysicalMaterial::logical(material)))
    }
    pub(crate) fn take_unstructured(&mut self, count: usize) -> Option<Vec<Material>> {
        if self.count_unstructured() < count {
            return None;
        }
        self.normalize_legacy_alignment();
        let mut out = Vec::with_capacity(count);
        let mut i = 0;
        while i < self.materials.len() && out.len() < count {
            if !self.materials[i].has_internal_structure() && !self.materials[i].is_empty() {
                self.physical_instances.swap_remove(i);
                out.push(self.materials.swap_remove(i));
            } else {
                i += 1;
            }
        }
        Some(out)
    }
    pub(crate) fn count_unstructured(&self) -> usize {
        self.materials
            .iter()
            .filter(|material| !material.has_internal_structure() && !material.is_empty())
            .count()
    }
    pub(crate) fn count_structured(&self) -> usize {
        self.materials
            .iter()
            .filter(|material| material.has_internal_structure() && !material.is_empty())
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::{BaseResource, InternalBond, Material};
    fn compound() -> Material {
        Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![InternalBond { part_a: 0, part_b: 1 }],
        }
    }
    fn catalog() -> Vec<BaseResource> {
        crate::resources::default_catalog()
    }
    #[test]
    fn free_material_is_stored_as_discrete_units() {
        let mut storage = MaterialStorage::default();
        assert!(storage.store(Material::free_base("Carbon", 3.0)));
        assert_eq!(storage.materials.len(), 3);
        assert_eq!(storage.count_unstructured(), 3);
        assert_eq!(storage.total_amount(), 3.0);
    }
    #[test]
    fn peek_does_not_consume_free_material() {
        let mut storage = MaterialStorage::default();
        storage.store(Material::free_base("Carbon", 1.0));
        let peeked = storage.peek_one_unstructured().expect("stored unit");
        assert_eq!(peeked, Material::free_base("Carbon", 1.0));
        assert_eq!(storage.count_unstructured(), 1);
    }
    #[test]
    fn structured_material_is_stored_intact() {
        let mut storage = MaterialStorage::default();
        let m = compound();
        assert!(storage.store(m.clone()));
        assert_eq!(storage.materials, vec![m]);
        assert_eq!(storage.count_structured(), 1);
    }
    #[test]
    fn structured_material_is_taken_intact() {
        let mut storage = MaterialStorage::default();
        let m = compound();
        storage.store(m.clone());
        assert_eq!(storage.take_matching(&m), Some(m.clone()));
        assert!(storage.is_empty());
    }
    #[test]
    fn physical_material_is_stored_in_intrinsic_frame() {
        let mut storage = MaterialStorage::default();
        let m = compound();
        let placements = vec![
            Placement { x: 10.0, y: 20.0, rotation_radians: 0.5 },
            Placement { x: 10.838, y: 20.0, rotation_radians: 0.75 },
        ];
        assert!(storage.store_physical(m.clone(), placements, &catalog()));
        let restored = storage.take_matching_physical(&m).expect("stored instance");
        assert_eq!(restored.material, m);
        let intrinsic = restored.placements.expect("intrinsic realization");
        assert!(intrinsic[0].x.abs() <= 1e-12);
        assert!(intrinsic[0].y.abs() <= 1e-12);
        assert!(intrinsic[0].rotation_radians.abs() <= 1e-12);
        assert!((intrinsic[1].x.hypot(intrinsic[1].y) - 0.838).abs() <= 1e-9);
        assert!((intrinsic[1].rotation_radians - 0.25).abs() <= 1e-12);
        assert!(restored.internal_connections.is_some());
    }
    #[test]
    fn physical_single_constituent_is_stored_with_its_realization() {
        let mut storage = MaterialStorage::default();
        let m = Material::free_base("Carbon", 1.0);
        let placements = vec![Placement { x: 10.0, y: 20.0, rotation_radians: 0.25 }];
        assert!(storage.store_physical(m.clone(), placements, &catalog()));
        let restored = storage.take_matching_physical(&m).expect("stored instance");
        assert_eq!(restored.material, m);
        let intrinsic = restored.placements.expect("intrinsic realization");
        assert!(intrinsic[0].x.abs() <= 1e-12);
        assert!(intrinsic[0].y.abs() <= 1e-12);
        assert!(intrinsic[0].rotation_radians.abs() <= 1e-12);
        assert!(restored.internal_connections.is_some());
    }
    #[test]
    fn storage_never_merges_independent_atoms() {
        let mut storage = MaterialStorage::default();
        storage.store(Material::free_base("Carbon", 1.0));
        storage.store(Material::free_base("Carbon", 1.0));
        assert_eq!(storage.materials.len(), 2);
    }
    #[test]
    fn storage_never_opens_a_compound() {
        let mut storage = MaterialStorage::default();
        let m = compound();
        storage.store(m.clone());
        assert!(storage.take_unstructured(1).is_none());
        assert_eq!(storage.materials, vec![m]);
    }
    #[test]
    fn fractional_material_is_rejected_at_storage_boundary() {
        let mut storage = MaterialStorage::default();
        assert!(!storage.store(Material::free_base("Carbon", 1.5)));
        assert!(storage.is_empty());
    }
}
