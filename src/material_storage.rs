//! Organism material inventory.
//! Storage is a collection of independent material objects. It never combines,
//! breaks, or transforms them.
use serde::{Deserialize, Serialize};
use crate::resources::Material;
const MATERIAL_EPSILON:f64=1e-12;
#[derive(Serialize,Deserialize,Clone,Debug,Default,PartialEq)]pub(crate)struct MaterialStorage{pub(crate)materials:Vec<Material>}
impl MaterialStorage{
 pub(crate)fn is_empty(&self)->bool{self.materials.is_empty()}
 pub(crate)fn total_amount(&self)->f64{self.materials.iter().map(Material::total_amount).sum()}
 fn is_discrete(material:&Material)->bool{if material.is_structured(){return material.is_valid()}material.composition().iter().all(|c|c.amount.is_finite()&&c.amount>0.0&&c.amount.fract().abs()<=MATERIAL_EPSILON)}
 pub(crate)fn store(&mut self,material:Material)->bool{if !material.is_valid()||material.is_empty()||!Self::is_discrete(&material){return false}if material.is_structured(){self.materials.push(material);return true}for c in material.composition(){for _ in 0..c.amount.round()as u64{self.materials.push(Material::free_base(c.resource.clone(),1.0));}}true}
 pub(crate)fn peek_one_unstructured(&self)->Option<Material>{self.materials.iter().find(|m|!m.is_structured()&&!m.is_empty()).cloned()}
 pub(crate)fn take_one_unstructured(&mut self)->Option<Material>{let i=self.materials.iter().position(|m|!m.is_structured()&&!m.is_empty())?;Some(self.materials.swap_remove(i))}
 pub(crate)fn take_one_unstructured_named(&mut self,name:&str)->Option<Material>{let i=self.materials.iter().position(|m|!m.is_structured()&&!m.is_empty()&&m.composition().len()==1&&m.composition()[0].resource==name&&(m.composition()[0].amount-1.0).abs()<=MATERIAL_EPSILON)?;Some(self.materials.swap_remove(i))}
 pub(crate)fn take_matching(&mut self,target:&Material)->Option<Material>{let i=self.materials.iter().position(|m|m==target&&!m.is_empty())?;Some(self.materials.swap_remove(i))}
 pub(crate)fn take_unstructured(&mut self,count:usize)->Option<Vec<Material>>{if self.count_unstructured()<count{return None}let mut out=Vec::with_capacity(count);let mut i=0;while i<self.materials.len()&&out.len()<count{if !self.materials[i].is_structured()&&!self.materials[i].is_empty(){out.push(self.materials.swap_remove(i))}else{i+=1}}Some(out)}
 pub(crate)fn count_unstructured(&self)->usize{self.materials.iter().filter(|m|!m.is_structured()&&!m.is_empty()).count()}
 pub(crate)fn count_structured(&self)->usize{self.materials.iter().filter(|m|m.is_structured()&&!m.is_empty()).count()}
}
#[cfg(test)]mod tests{use super::*;use crate::attachment::{AttachmentFeature,ConstituentAttachment,ConstituentId};use crate::material_structure::{InternalAttachmentBond,MaterialConstituent,MaterialStructure};fn compound()->Material{Material{composition:Vec::new(),structure:Some(MaterialStructure{constituents:vec![MaterialConstituent{id:ConstituentId(1),resource:"Carbon".into()},MaterialConstituent{id:ConstituentId(2),resource:"Hydrogen".into()}],internal_bonds:vec![InternalAttachmentBond{a:ConstituentAttachment{constituent:ConstituentId(1),feature:AttachmentFeature::Discrete(0)},b:ConstituentAttachment{constituent:ConstituentId(2),feature:AttachmentFeature::Discrete(0)}}]})}}
#[test]fn free_material_is_stored_as_discrete_units(){let mut s=MaterialStorage::default();assert!(s.store(Material::free_base("Carbon",3.0)));assert_eq!(s.materials.len(),3);assert_eq!(s.count_unstructured(),3);assert_eq!(s.total_amount(),3.0)}
#[test]fn peek_does_not_consume_free_material(){let mut s=MaterialStorage::default();s.store(Material::free_base("Carbon",1.0));assert_eq!(s.peek_one_unstructured(),Some(Material::free_base("Carbon",1.0)));assert_eq!(s.count_unstructured(),1)}
#[test]fn structured_material_is_stored_intact(){let mut s=MaterialStorage::default();let m=compound();assert!(s.store(m.clone()));assert_eq!(s.materials,vec![m]);assert_eq!(s.count_structured(),1)}
#[test]fn structured_material_is_taken_intact(){let mut s=MaterialStorage::default();let m=compound();s.store(m.clone());assert_eq!(s.take_matching(&m),Some(m));assert!(s.is_empty())}
#[test]fn storage_never_merges_independent_atoms(){let mut s=MaterialStorage::default();s.store(Material::free_base("Carbon",1.0));s.store(Material::free_base("Carbon",1.0));assert_eq!(s.materials.len(),2)}
#[test]fn storage_never_opens_a_compound(){let mut s=MaterialStorage::default();let m=compound();s.store(m.clone());assert!(s.take_unstructured(1).is_none());assert_eq!(s.materials,vec![m])}
#[test]fn fractional_material_is_rejected_at_storage_boundary(){let mut s=MaterialStorage::default();assert!(!s.store(Material::free_base("Carbon",1.5)));assert!(s.is_empty())}}
