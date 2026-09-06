//! Organism material inventory.
//!
//! A `Material` is one coherent material object: its composition and internal
//! bonds belong together. `MaterialStorage` is a collection of independent
//! material objects. Storage itself never combines, breaks, or otherwise
//! transforms its contents.

use serde::{Deserialize, Serialize};

use crate::resources::Material;

const MATERIAL_EPSILON: f64 = 1e-12;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub(crate) struct MaterialStorage {
    /// Independent material objects held by the organism.
    ///
    /// Unstructured field stock is expanded into one-unit objects when it
    /// enters storage. Structured material is stored as one intact object.
    pub(crate) materials: Vec<Material>,
}

impl MaterialStorage {
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

    /// Store material without changing its physical identity.
    ///
    /// Free field stock may be an aggregate count, so it is expanded into
    /// discrete one-unit material objects. A structured material is stored
    /// as exactly one intact object; its internal bonds are never touched.
    pub(crate) fn store(&mut self, material: Material) -> bool {
        if material.parts.is_empty() || !material.is_valid() || !Self::is_discrete(&material) {
            return false;
        }

        if material.has_internal_structure() {
            self.materials.push(material);
            return true;
        }

        for (name, amount) in material.parts {
            let count = amount.round() as u64;
            for _ in 0..count {
                self.materials.push(Material::free_base(name.clone(), 1.0));
            }
        }
        true
    }

    /// Return a copy of one independent unstructured material object without
    /// removing it. This is a planning/validation operation, not a transfer.
    pub(crate) fn peek_one_unstructured(&self) -> Option<Material> {
        self.materials
            .iter()
            .find(|material| !material.has_internal_structure() && !material.is_empty())
            .cloned()
    }

    /// Remove one independent unstructured material object. This is inventory
    /// allocation only; no internal material bond is opened.
    pub(crate) fn take_one_unstructured(&mut self) -> Option<Material> {
        let index = self
            .materials
            .iter()
            .position(|material| !material.has_internal_structure() && !material.is_empty())?;
        Some(self.materials.swap_remove(index))
    }

    /// Remove one independent unstructured material object of the requested
    /// resource type without touching structured inventory.
    pub(crate) fn take_one_unstructured_named(&mut self, name: &str) -> Option<Material> {
        let index = self.materials.iter().position(|material| {
            !material.has_internal_structure()
                && !material.is_empty()
                && material.parts.len() == 1
                && material.parts[0].0 == name
                && (material.parts[0].1 - 1.0).abs() <= MATERIAL_EPSILON
        })?;
        Some(self.materials.swap_remove(index))
    }

    /// Remove exactly `count` independent unstructured material objects.
    /// This is allocation, not COMBINE.
    pub(crate) fn take_unstructured(&mut self, count: usize) -> Option<Vec<Material>> {
        if self.count_unstructured() < count {
            return None;
        }
        let mut out = Vec::with_capacity(count);
        let mut i = 0;
        while i < self.materials.len() && out.len() < count {
            if !self.materials[i].has_internal_structure() && !self.materials[i].is_empty() {
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
    use crate::resources::{InternalBond, Material};

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
        assert!(storage.store(Material::free_base("Carbon", 1.0)));
        let peeked = storage.peek_one_unstructured().expect("stored unit");
        assert_eq!(peeked, Material::free_base("Carbon", 1.0));
        assert_eq!(storage.count_unstructured(), 1);
    }

    #[test]
    fn structured_material_is_stored_intact() {
        let mut storage = MaterialStorage::default();
        let compound = Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![InternalBond {
                part_a: 0,
                part_b: 1,
            }],
        };
        assert!(storage.store(compound.clone()));
        assert_eq!(storage.materials, vec![compound]);
        assert_eq!(storage.count_structured(), 1);
    }

    #[test]
    fn storage_never_merges_independent_atoms() {
        let mut storage = MaterialStorage::default();
        assert!(storage.store(Material::free_base("Carbon", 1.0)));
        assert!(storage.store(Material::free_base("Carbon", 1.0)));
        assert_eq!(storage.materials.len(), 2);
    }

    #[test]
    fn storage_never_opens_a_compound() {
        let mut storage = MaterialStorage::default();
        let compound = Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![InternalBond {
                part_a: 0,
                part_b: 1,
            }],
        };
        assert!(storage.store(compound.clone()));
        assert!(storage.take_unstructured(1).is_none());
        assert_eq!(storage.materials, vec![compound]);
    }

    #[test]
    fn fractional_material_is_rejected_at_storage_boundary() {
        let mut storage = MaterialStorage::default();
        let fractional = Material::free_base("Carbon", 1.5);
        assert!(!storage.store(fractional));
        assert!(storage.is_empty());
    }
}
