#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
//! Organism material inventory.
use crate::physical_material::PhysicalMaterial;
use crate::resources::Material;
use crate::structure::Placement;
use serde::{Deserialize, Serialize};

const MATERIAL_EPSILON: f64 = 1e-12;

/// One authoritative stored-material entry. A physical entry carries the
/// complete realized material; a logical entry carries only a material
/// description and therefore cannot masquerade as an existing physical object.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) enum StoredMaterial {
    Logical(Material),
    Physical(PhysicalMaterial),
}

impl StoredMaterial {
    fn material(&self) -> &Material {
        match self {
            Self::Logical(material) => material,
            Self::Physical(instance) => &instance.material,
        }
    }

    fn into_material(self) -> Material {
        match self {
            Self::Logical(material) => material,
            Self::Physical(instance) => instance.material,
        }
    }

    fn physical(&self) -> Option<PhysicalMaterial> {
        match self {
            Self::Logical(_) => None,
            Self::Physical(instance) => Some(instance.clone()),
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
        self.entries
            .iter()
            .filter(|entry| matches!(entry, StoredMaterial::Physical(_)))
            .count()
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
        if material.has_internal_structure() {
            self.entries.push(StoredMaterial::Logical(material));
            return true;
        }
        for (name, amount) in material.parts {
            let count = amount.round() as u64;
            for _ in 0..count {
                self.entries
                    .push(StoredMaterial::Logical(Material::free_base(
                        name.clone(),
                        1.0,
                    )));
            }
        }
        true
    }

    /// Store an already-realized physical instance without reconstructing its
    /// geometry or internal connection endpoints. The world realization is
    /// rebased only into storage's intrinsic frame.
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

    pub(crate) fn take_matching(&mut self, target: &Material) -> Option<Material> {
        let index = self
            .entries
            .iter()
            .position(|entry| entry.material() == target && !entry.material().is_empty())?;
        Some(self.entries.swap_remove(index).into_material())
    }

    pub(crate) fn take_physical_at(&mut self, index: usize) -> Option<PhysicalMaterial> {
        match self.entries.get(index)? {
            StoredMaterial::Physical(instance) if instance.is_realized() => {}
            _ => return None,
        }
        match self.entries.swap_remove(index) {
            StoredMaterial::Physical(instance) => Some(instance),
            StoredMaterial::Logical(_) => None,
        }
    }

    pub(crate) fn take_matching_physical(&mut self, target: &Material) -> Option<PhysicalMaterial> {
        let index = self.entries.iter().position(|entry| {
            matches!(entry, StoredMaterial::Physical(instance)
                if instance.material == *target && !instance.material.is_empty())
        })?;
        match self.entries.swap_remove(index) {
            StoredMaterial::Physical(instance) => Some(instance),
            StoredMaterial::Logical(_) => None,
        }
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
    use crate::resources::{BaseResource, InternalBond, Material};

    fn compound() -> Material {
        Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![InternalBond {
                part_a: 0,
                part_b: 1,
            }],
        }
    }

    fn catalog() -> Vec<BaseResource> {
        crate::resources::default_catalog()
    }

    #[test]
    fn free_material_is_stored_as_discrete_units() {
        let mut storage = MaterialStorage::default();
        assert!(storage.store(Material::free_base("Carbon", 3.0)));
        assert_eq!(storage.len(), 3);
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
        assert_eq!(storage.materials_snapshot(), vec![m]);
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
            Placement {
                x: 10.0,
                y: 20.0,
                rotation_radians: 0.5,
            },
            Placement {
                x: 10.838,
                y: 20.0,
                rotation_radians: 0.75,
            },
        ];
        assert!(storage.store_physical(m.clone(), placements, &catalog()));
        assert_eq!(storage.physical_count(), 1);
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
        let placements = vec![Placement {
            x: 10.0,
            y: 20.0,
            rotation_radians: 0.25,
        }];
        assert!(storage.store_physical(m.clone(), placements, &catalog()));
        assert_eq!(storage.physical_count(), 1);
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
        assert_eq!(storage.len(), 2);
    }

    #[test]
    fn storage_never_opens_a_compound() {
        let mut storage = MaterialStorage::default();
        let m = compound();
        storage.store(m.clone());
        assert!(storage.take_unstructured(1).is_none());
        assert_eq!(storage.materials_snapshot(), vec![m]);
    }

    #[test]
    fn fractional_material_is_rejected_at_storage_boundary() {
        let mut storage = MaterialStorage::default();
        assert!(!storage.store(Material::free_base("Carbon", 1.5)));
        assert!(storage.is_empty());
    }
}
