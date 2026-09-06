//! Organism material inventory.
//!
//! A `Material` is one coherent material object: its composition and internal
//! bonds belong together. `MaterialStorage` is therefore deliberately a
//! collection of independent `Material` objects rather than another Material.
//!
//! Storage does not combine, break, or otherwise transform its contents.

use serde::{Deserialize, Serialize};

use crate::resources::Material;

const MATERIAL_EPSILON: f64 = 1e-12;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub(crate) struct MaterialStorage {
    /// Independent acquired material objects.
    ///
    /// Unstructured material enters storage as one object per constituent unit.
    /// Structured material is stored as one intact object and is never
    /// flattened here.
    pub(crate) materials: Vec<Material>,
}

impl MaterialStorage {
    pub(crate) fn is_empty(&self) -> bool {
        self.materials.is_empty()
    }

    pub(crate) fn total_amount(&self) -> f64 {
        self.materials
            .iter()
            .map(Material::total_amount)
            .sum()
    }

    /// Store material without changing its physical identity.
    ///
    /// An unstructured field stock can represent many independent atoms in one
    /// aggregate Material. Storage expands that aggregate into discrete
    /// one-unit objects. Structured material is transferred intact.
    pub(crate) fn store(&mut self, material: Material) -> bool {
        if material.parts.is_empty() || !material.is_valid() {
            return false;
        }

        if material.has_internal_structure() {
            self.materials.push(material);
            return true;
        }

        for (name, amount) in material.parts {
            if amount <= 0.0 || !amount.is_finite() || (amount.fract()).abs() > MATERIAL_EPSILON {
                return false;
            }
            let count = amount.round() as u64;
            for _ in 0..count {
                self.materials.push(Material::free_base(name.clone(), 1.0));
            }
        }
        true
    }

    /// Remove one independent material object suitable for direct structural
    /// construction. Internal bonds are never opened by this operation.
    pub(crate) fn take_one_unstructured(&mut self) -> Option<Material> {
        let index = self
            .materials
            .iter()
            .position(|material| !material.has_internal_structure() && !material.is_empty())?;
        Some(self.materials.swap_remove(index))
    }

    /// Remove exactly `count` independent unstructured material objects.
    ///
    /// This is inventory allocation, not COMBINE: the returned objects remain
    /// independent and have no new internal bonds.
    pub(crate) fn take_unstructured(&mut self, count: usize) -> Option<Vec<Material>> {
        if self
            .materials
            .iter()
            .filter(|material| !material.has_internal_structure() && !material.is_empty())
            .count()
            < count
        {
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
    fn structured_material_is_stored_intact() {
        let mut storage = MaterialStorage::default();
        let compound = Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![InternalBond { part_a: 0, part_b: 1 }],
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
            internal_bonds: vec![InternalBond { part_a: 0, part_b: 1 }],
        };
        assert!(storage.store(compound.clone()));
        let atoms = storage.take_unstructured(1);
        assert!(atoms.is_none());
        assert_eq!(storage.materials, vec![compound]);
    }
}
