#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
//! Organism material inventory.
use crate::physical_material::PhysicalMaterial;
use crate::resources::Material;
use crate::structure::Placement;
use serde::{Deserialize, Serialize};

const MATERIAL_EPSILON: f64 = 1e-12;

/// One authoritative stored-material entry. Every entry is an already-realized
/// physical object; logical material descriptions cannot exist in organism storage.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) enum StoredMaterial {
    Physical(PhysicalMaterial),
}

impl StoredMaterial {
    fn material(&self) -> &Material {
        match self {
            Self::Physical(instance) => &instance.material,
        }
    }

    fn into_material(self) -> Material {
        match self {
            Self::Physical(instance) => instance.material,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub(crate) struct MaterialStorage {
    #[serde(default)]
    pub(crate) entries: Vec<StoredMaterial>,
}

impl MaterialStorage {
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn total_amount(&self) -> f64 {
        self.entries
            .iter()
            .map(|entry| entry.material().total_amount())
            .sum()
    }

    pub(crate) fn first_material(&self) -> Option<Material> {
        self.entries.first().map(|entry| entry.material().clone())
    }

    pub(crate) fn iter_materials(&self) -> impl Iterator<Item = &Material> {
        self.entries.iter().map(StoredMaterial::material)
    }

    pub(crate) fn materials_snapshot(&self) -> Vec<Material> {
        self.iter_materials().cloned().collect()
    }

    pub(crate) fn drain_entries(&mut self) -> Vec<StoredMaterial> {
        std::mem::take(&mut self.entries)
    }

    pub(crate) fn physical_count(&self) -> usize {
        self.entries.len()
    }

    fn is_discrete(material: &Material) -> bool {
        material.parts.iter().all(|(_, amount)| {
            amount.is_finite() && *amount > 0.0 && amount.fract().abs() <= MATERIAL_EPSILON
        })
    }

    pub(crate) fn store_physical_instance(&mut self, instance: PhysicalMaterial) -> bool {
        if !instance.is_realized()
            || instance.material.parts.is_empty()
            || !instance.material.is_valid()
            || !Self::is_discrete(&instance.material)
        {
            return false;
        }
        let Some(instance) = instance.into_intrinsic_frame() else {
            return false;
        };
        self.entries.insert(0, StoredMaterial::Physical(instance));
        true
    }

    pub(crate) fn store_physical_instance_at_owner_anchor(
        &mut self,
        instance: PhysicalMaterial,
        anchor: Placement,
    ) -> bool {
        let Some(origin) = instance
            .placements
            .as_ref()
            .and_then(|placements| placements.first())
            .copied()
        else {
            return false;
        };
        let Some(mut instance) = instance.into_intrinsic_frame() else {
            return false;
        };
        instance.owner_relative_origin = Some(Placement {
            x: origin.x - anchor.x,
            y: origin.y - anchor.y,
            rotation_radians: origin.rotation_radians,
        });
        if instance.material.parts.is_empty()
            || !instance.material.is_valid()
            || !Self::is_discrete(&instance.material)
        {
            return false;
        }
        self.entries.insert(0, StoredMaterial::Physical(instance));
        true
    }

    pub(crate) fn store_physical(
        &mut self,
        material: Material,
        placements: Vec<Placement>,
        catalog: &[crate::resources::BaseResource],
    ) -> bool {
        let Some(instance) = PhysicalMaterial::realized(material, placements, catalog) else {
            return false;
        };
        self.store_physical_instance(instance)
    }

    pub(crate) fn peek_one_unstructured(&self) -> Option<Material> {
        self.entries
            .iter()
            .map(StoredMaterial::material)
            .find(|material| !material.has_internal_structure() && !material.is_empty())
            .cloned()
    }

    pub(crate) fn peek_matching_physical(&self, target: &Material) -> Option<PhysicalMaterial> {
        self.entries.iter().find_map(|entry| match entry {
            StoredMaterial::Physical(instance)
                if instance.material == *target && !instance.material.is_empty() =>
            {
                Some(instance.clone())
            }
            _ => None,
        })
    }

    pub(crate) fn take_one_unstructured_named(&mut self, name: &str) -> Option<Material> {
        let index = self.entries.iter().position(|entry| {
            let material = entry.material();
            !material.has_internal_structure()
                && !material.is_empty()
                && material.parts.len() == 1
                && material.parts[0].0 == name
                && (material.parts[0].1 - 1.0).abs() <= MATERIAL_EPSILON
        })?;
        Some(self.entries.swap_remove(index).into_material())
    }

    pub(crate) fn take_physical_at(&mut self, index: usize) -> Option<PhysicalMaterial> {
        match self.entries.get(index)? {
            StoredMaterial::Physical(instance) if instance.is_realized() => {}
            _ => return None,
        }
        let StoredMaterial::Physical(instance) = self.entries.swap_remove(index);
        Some(instance)
    }

    pub(crate) fn take_matching_physical(&mut self, target: &Material) -> Option<PhysicalMaterial> {
        let index = self.entries.iter().position(|entry| {
            matches!(entry, StoredMaterial::Physical(instance)
                if instance.material == *target && !instance.material.is_empty())
        })?;
        let StoredMaterial::Physical(instance) = self.entries.swap_remove(index);
        Some(instance)
    }

    pub(crate) fn take_unstructured(&mut self, count: usize) -> Option<Vec<Material>> {
        if self.count_unstructured() < count {
            return None;
        }
        let mut out = Vec::with_capacity(count);
        let mut i = 0;
        while i < self.entries.len() && out.len() < count {
            if !self.entries[i].material().has_internal_structure()
                && !self.entries[i].material().is_empty()
            {
                out.push(self.entries.swap_remove(i).into_material());
            } else {
                i += 1;
            }
        }
        Some(out)
    }

    pub(crate) fn count_unstructured(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| {
                !entry.material().has_internal_structure() && !entry.material().is_empty()
            })
            .count()
    }

    pub(crate) fn count_structured(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| {
                entry.material().has_internal_structure() && !entry.material().is_empty()
            })
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::Material;

    fn catalog() -> Vec<crate::resources::BaseResource> {
        crate::resources::default_catalog()
    }

    fn carbon_instance(x: f64) -> PhysicalMaterial {
        PhysicalMaterial::realized(
            Material::free_base("Carbon", 1.0),
            vec![Placement {\n                x,\n                y: 0.0,\n                rotation_radians: 0.0,\n            }],
            &catalog(),
        )
        .expect("carbon must be physically realizable")
    }

    #[test]
    fn storage_contains_only_physical_entries() {
        let mut storage = MaterialStorage::default();
        assert!(storage.store_physical_instance(carbon_instance(10.0)));
        assert_eq!(storage.len(), 1);
        assert_eq!(storage.physical_count(), 1);
    }

    #[test]
    fn storage_rejects_unrealized_instances() {
        let mut storage = MaterialStorage::default();
        let invalid = PhysicalMaterial {
            id: 0,
            material: Material::free_base("Carbon", 1.0),
            placements: None,
            internal_connections: None,
            owner_relative_origin: None,
        };
        assert!(!storage.store_physical_instance(invalid));
        assert!(storage.is_empty());
    }

    #[test]
    fn structured_material_must_arrive_already_realized() {
        let mut storage = MaterialStorage::default();
        let material = Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![crate::resources::InternalBond {\n                part_a: 0,\n                part_b: 1,\n            }],
        };
        assert!(PhysicalMaterial::realized(material, vec![], &catalog()).is_none());
        assert!(storage.is_empty());
    }

    #[test]
    fn storage_preserves_physical_realization() {
        let mut storage = MaterialStorage::default();
        let instance = carbon_instance(10.0);
        assert!(storage.store_physical_instance(instance.clone()));
        let restored = storage.take_physical_at(0).expect("stored instance");
        assert_eq!(restored.material, instance.material);
        let placements = restored.placements.expect("realization");
        assert_eq!(placements[0].x, 0.0);
        assert_eq!(placements[0].y, 0.0);
    }

    #[test]
    fn storage_does_not_merge_independent_physical_objects() {
        let mut storage = MaterialStorage::default();
        assert!(storage.store_physical_instance(carbon_instance(0.0)));
        assert!(storage.store_physical_instance(carbon_instance(2.0)));
        assert_eq!(storage.len(), 2);
    }
}
