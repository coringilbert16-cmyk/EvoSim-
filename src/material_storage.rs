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
        self.materials.push(material);
        self.physical_instances.push(Some(instance));
        true
    }
    pub(crate) fn peek_one_unstructured(&self) -> Option<Material> {
        self.materials
            .iter()
            .find(|material| !material.has_internal_structure() && !material.is_empty())
            .cloned()
    }
    pub(crate) fn peek_first_realized(&self) -> Option<(Material, PhysicalMaterial)> {
        self.materials.iter().enumerate().find_map(|(index, material)| {
            let instance = self.physical_instances.get(index)?.as_ref()?;
            if material.is_empty() || !instance.is_realized() {
                return None;
            }
            Some((material.clone(), instance.clone()))
        })
    }
    pub(crate) fn take_first_realized(&mut self) -> Option<PhysicalMaterial> {
        self.normalize_legacy_alignment();
        let index = self
            .physical_instances
            .iter()
            .position(|instance| instance.as_ref().is_some_and(PhysicalMaterial::is_realized))?;
        self.physical_instances.swap_remove(index);
        self.materials.swap_remove(index);
        Some(instance_from_removed_slot(&self.physical_instances, self.materials.len(), index))
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

fn instance_from_removed_slot(
    _physical_instances: &[Option<PhysicalMaterial>],
    _remaining_materials: usize,
    _removed_index: usize,
) -> PhysicalMaterial {
    unreachable!("placeholder replaced in follow-up storage update")
}
