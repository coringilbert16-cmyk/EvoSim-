//! Structural material owned by a physical structural unit.
//!
//! `Material` remains the authoritative composition + internal-structure
//! representation. `StructuralMaterial` carries that exact material into a
//! physical `StructuralUnit` without reconstructing identity from a name.
use serde::{Deserialize, Serialize};
use crate::resources::{BaseResource, ConnectionSites, Material, ResourceProperties, Shape};
#[derive(Serialize,Deserialize,Clone,Debug,PartialEq)]pub struct StructuralMaterial{pub material:Material}
impl StructuralMaterial{
 pub fn from_material(material:Material)->Option<Self>{if !material.is_valid()||material.is_empty(){return None}if !material.has_internal_structure()&&(material.parts.len()!=1||(material.total_amount()-1.0).abs()>f64::EPSILON){return None}Some(Self{material})}
 pub fn single(resource_name:impl Into<String>)->Self{Self{material:Material::free_base(resource_name,1.0)}}
 pub fn material(&self)->&Material{&self.material}
 pub fn constituents(&self)->&[(String,f64)]{&self.material.parts}
 pub fn internal_bonds(&self)->&[crate::resources::InternalBond]{&self.material.internal_bonds}
 pub fn total_amount(&self)->f64{self.material.total_amount()}
 pub fn mass(&self,catalog:&[BaseResource])->f64{self.material.mass(catalog)}
 pub fn weighted_properties(&self,catalog:&[BaseResource])->ResourceProperties{self.material.weighted_properties(catalog)}
 pub fn is_composite(&self)->bool{self.material.parts.len()>1}
 pub fn is_valid(&self)->bool{self.material.is_valid()&&!self.material.is_empty()}
 pub fn resolves_in_catalog(&self,catalog:&[BaseResource])->bool{self.material.parts.iter().all(|(name,_)|catalog.iter().any(|base|base.name==*name))}
 pub fn connection_sites(&self,catalog:&[BaseResource])->Option<ConnectionSites>{let[(name,amount)]=self.material.parts.as_slice()else{return None};if !self.material.internal_bonds.is_empty()||(*amount-1.0).abs()>f64::EPSILON{return None}catalog.iter().find(|base|base.name==*name).map(|base|base.shape.connection_sites())}
 pub fn shape(&self,catalog:&[BaseResource])->Option<&Shape>{let[(name,amount)]=self.material.parts.as_slice()else{return None};if !self.material.internal_bonds.is_empty()||(*amount-1.0).abs()>f64::EPSILON{return None}catalog.iter().find(|base|base.name==*name).map(|base|&base.shape)}
}
#[cfg(test)]mod tests{use super::*;use crate::resources::{default_catalog,InternalBond};#[test]fn storage_material_becomes_owned_structural_material_without_losing_structure(){let m=Material{parts:vec![("Carbon".into(),1.0),("Hydrogen".into(),1.0)],internal_bonds:vec![InternalBond{part_a:0,part_b:1}]};let s=StructuralMaterial::from_material(m.clone()).unwrap();assert_eq!(s.material(),&m);assert!(s.is_composite());assert_eq!(s.internal_bonds(),m.internal_bonds.as_slice())}#[test]fn free_unit_is_valid_structural_material(){let s=StructuralMaterial::from_material(Material::free_base("Carbon",1.0)).unwrap();assert!(!s.is_composite());assert!(s.resolves_in_catalog(&default_catalog()))}#[test]fn free_aggregate_cannot_become_one_structural_unit(){assert!(StructuralMaterial::from_material(Material::free_base("Carbon",2.0)).is_none())}#[test]fn composite_does_not_get_invented_single_unit_geometry(){let m=Material{parts:vec![("Carbon".into(),1.0),("Hydrogen".into(),1.0)],internal_bonds:vec![InternalBond{part_a:0,part_b:1}]};let s=StructuralMaterial::from_material(m).unwrap();let c=default_catalog();assert!(s.connection_sites(&c).is_none());assert!(s.shape(&c).is_none())}#[test]fn serialization_round_trip_preserves_material_identity(){let m=Material{parts:vec![("Carbon".into(),1.0),("Hydrogen".into(),1.0)],internal_bonds:vec![InternalBond{part_a:0,part_b:1}]};let s=StructuralMaterial::from_material(m).unwrap();let r:StructuralMaterial=serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();assert_eq!(r,s)}}