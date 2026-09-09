//! Structural material owned by a physical structural unit.
use serde::{Deserialize, Serialize};
use crate::resources::{BaseResource, ConnectionSites, Material, ResourceProperties, Shape};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StructuralMaterial { pub material: Material }

impl StructuralMaterial {
    pub fn from_material(material: Material) -> Option<Self> {
        if !material.is_valid() || material.is_empty() { return None; }
        if !material.is_structured() && (material.composition().len() != 1 || (material.total_amount() - 1.0).abs() > f64::EPSILON) { return None; }
        Some(Self { material })
    }
    pub fn single(resource_name: impl Into<String>) -> Self { Self { material: Material::free_base(resource_name, 1.0) } }
    pub fn material(&self) -> &Material { &self.material }
    pub fn composition(&self) -> &[crate::resources::MaterialComponent] { self.material.composition() }
    pub fn structure(&self) -> Option<&crate::material_structure::MaterialStructure> { self.material.structure() }
    pub fn total_amount(&self) -> f64 { self.material.total_amount() }
    pub fn mass(&self, catalog: &[BaseResource]) -> f64 { self.material.mass(catalog) }
    pub fn weighted_properties(&self, catalog: &[BaseResource]) -> ResourceProperties { self.material.weighted_properties(catalog) }
    pub fn is_composite(&self) -> bool { self.material.structure().map_or(false, |s| s.constituents.len() > 1) }
    pub fn is_valid(&self) -> bool { self.material.is_valid() && !self.material.is_empty() }
    pub fn resolves_in_catalog(&self, catalog: &[BaseResource]) -> bool {
        if let Some(structure) = self.material.structure() { structure.constituents.iter().all(|c| catalog.iter().any(|base| base.name == c.resource)) }
        else { self.material.composition().iter().all(|c| catalog.iter().any(|base| base.name == c.resource)) }
    }
    pub fn connection_sites(&self, catalog: &[BaseResource]) -> Option<ConnectionSites> {
        if self.material.is_structured() { return None; }
        let [component] = self.material.composition() else { return None; };
        if (component.amount - 1.0).abs() > f64::EPSILON { return None; }
        catalog.iter().find(|base| base.name == component.resource).map(|base| base.shape.connection_sites())
    }
    pub fn shape<'a>(&self, catalog: &'a [BaseResource]) -> Option<&'a Shape> {
        if self.material.is_structured() { return None; }
        let [component] = self.material.composition() else { return None; };
        if (component.amount - 1.0).abs() > f64::EPSILON { return None; }
        catalog.iter().find(|base| base.name == component.resource).map(|base| &base.shape)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attachment::{AttachmentFeature, ConstituentAttachment, ConstituentId};
    use crate::material_structure::{InternalAttachmentBond, MaterialConstituent, MaterialStructure};
    use crate::resources::{default_catalog, Material};
    #[test] fn free_unit_is_valid_structural_material() {
        let s = StructuralMaterial::from_material(Material::free_base("Carbon", 1.0)).unwrap(); let c = default_catalog();
        assert!(!s.is_composite()); assert!(s.resolves_in_catalog(&c)); assert!(s.shape(&c).is_some()); assert!(s.connection_sites(&c).is_some());
    }
    #[test] fn free_aggregate_cannot_become_one_structural_unit() { assert!(StructuralMaterial::from_material(Material::free_base("Carbon", 2.0)).is_none()); }
    #[test] fn structured_material_does_not_choose_a_scaffold() {
        let structure = MaterialStructure { constituents: vec![
            MaterialConstituent { id: ConstituentId(1), resource: "Carbon".into() }, MaterialConstituent { id: ConstituentId(2), resource: "Hydrogen".into() }],
            internal_bonds: vec![InternalAttachmentBond { a: ConstituentAttachment { constituent: ConstituentId(1), feature: AttachmentFeature::Discrete(0) }, b: ConstituentAttachment { constituent: ConstituentId(2), feature: AttachmentFeature::Discrete(0) } }] };
        let s = StructuralMaterial::from_material(Material { composition: Vec::new(), structure: Some(structure) }).unwrap(); let c = default_catalog();
        assert!(s.is_composite()); assert!(s.shape(&c).is_none()); assert!(s.connection_sites(&c).is_none());
    }
    #[test] fn serialization_round_trip_preserves_material_identity() {
        let s = StructuralMaterial::from_material(Material::free_base("Carbon", 1.0)).unwrap(); let r: StructuralMaterial = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap(); assert_eq!(r, s);
    }
}
