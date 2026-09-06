use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::resources::{InternalBond, Material};
use crate::structural_blueprint::{BlueprintConnection, BlueprintElement, StructuralBlueprint};
use crate::structure::Placement;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TraitDef { pub name: String, pub value: f64, pub mutation_probability: f64, pub mutation_sigma: f64 }
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Genome { pub traits: Vec<TraitDef>, #[serde(default = "default_structural_blueprint")] pub structural_blueprint: StructuralBlueprint }
impl Genome{
 pub fn trait_value(&self,name:&str,default:f64)->f64{self.traits.iter().find(|t|t.name==name).map(|t|t.value).unwrap_or(default)}
 pub fn mass_affinity(&self)->f64{self.trait_value("mass_affinity",0.0).clamp(-1.0,1.0)}
 pub fn potential_energy_affinity(&self)->f64{self.trait_value("potential_energy_affinity",0.0).clamp(-1.0,1.0)}
 pub fn reactivity_affinity(&self)->f64{self.trait_value("reactivity_affinity",0.0).clamp(-1.0,1.0)}
 pub fn cohesion_affinity(&self)->f64{self.trait_value("cohesion_affinity",0.0).clamp(-1.0,1.0)}
 pub fn memory_strength(&self)->f64{self.trait_value("memory_strength",0.5).clamp(0.0,1.0)}
 pub fn perception_radius(&self)->f64{self.trait_value("perception_radius",100.0).max(0.0)}
 pub fn sensory_resolution(&self)->f64{self.trait_value("sensory_resolution",0.5).clamp(0.0,1.0)}
 pub fn directional_resolution(&self)->f64{self.trait_value("directional_resolution",1.0).clamp(0.0,1.0)}
 pub fn processing_efficiency(&self)->f64{self.trait_value("processing_efficiency",0.8).clamp(0.05,1.0)}
 pub fn movement_efficiency(&self)->f64{self.trait_value("movement_efficiency",0.8).clamp(0.05,1.0)}
 pub fn reproductive_investment(&self)->f64{self.trait_value("reproductive_investment",0.5).clamp(0.15,1.0)}
 pub fn mutate(&mut self,rng:&mut ChaCha8Rng){for t in &mut self.traits{if rng.gen::<f64>()<t.mutation_probability.clamp(1e-6,0.25){let delta=rng.gen_range(-1.0..1.0)*t.mutation_sigma.max(0.0);t.value+=delta;}if rng.gen::<f64>()<0.001{t.mutation_probability=(t.mutation_probability*rng.gen_range(0.5..1.5)).clamp(1e-6,0.1);}}}
}
fn trait_def(name:&str,value:f64,sigma:f64)->TraitDef{TraitDef{name:name.into(),value,mutation_probability:0.001,mutation_sigma:sigma}}
fn hydrated_carbon_water()->Material{Material{parts:vec![("Carbon".into(),1.0),("Water".into(),1.0)],internal_bonds:vec![InternalBond{part_a:0,part_b:1}]}}
fn hydrated_carbon_nitrogen_water()->Material{Material{parts:vec![("Carbon".into(),1.0),("Nitrogen".into(),1.0),("Water".into(),1.0)],internal_bonds:vec![InternalBond{part_a:0,part_b:1},InternalBond{part_a:1,part_b:2}]}}

/// Seed body plan:
/// - rings 0..2: 19-unit Carbon F-Core, deliberately larger than the old
///   six-unit minimum while retaining a strong but not maximally rigid core;
/// - ring 3: 18-unit Carbon+Water hydrated lattice, mechanically softer and
///   permeable through its composition;
/// - ring 4: 24-unit Carbon+Nitrogen+Water hydrated membrane, a firmer outer
///   layer than the lattice while retaining substantial water content.
///
/// All three layers share the same rigid Carbon hexagonal scaffold geometry.
/// Adjacent cells share full hexagonal edges, represented by two structural
/// bonds at the shared corners. This makes the F-Core genuinely solid rather
/// than a sparse corner-connected frame, while retaining discrete bond sites
/// for later structural damage and pressure failure.
fn default_structural_blueprint()->StructuralBlueprint{
 let r=0.438_691_f64;
 let mut elements=Vec::with_capacity(61);
 let mut index_by_axial=HashMap::new();
 for q in -4_i32..=4_i32{
  for axial_r in -4_i32..=4_i32{
   let ring=q.abs().max(axial_r.abs()).max((q+axial_r).abs());
   if ring>4{continue}
   let material=match ring{0..=2=>Material::free_base("Carbon",1.0),3=>hydrated_carbon_water(),4=>hydrated_carbon_nitrogen_water(),_=>unreachable!()};
   let x=3.0_f64.sqrt()*r*(q as f64+0.5*axial_r as f64);
   let y=1.5*r*axial_r as f64;
   let index=elements.len();
   elements.push(BlueprintElement{material,placement:Placement{x,y,rotation_radians:std::f64::consts::FRAC_PI_6}});
   index_by_axial.insert((q,axial_r),index);
  }
 }
 let mut connections=Vec::with_capacity(312);
 for q in -4_i32..=4_i32{
  for axial_r in -4_i32..=4_i32{
   let Some(&a)=index_by_axial.get(&(q,axial_r))else{continue};
   for (dq,dr,pa1,pb1,pa2,pb2)in[(1_i32,0_i32,0_usize,2_usize,5_usize,3_usize),(0_i32,1_i32,0_usize,4_usize,1_usize,3_usize),(-1_i32,1_i32,2_usize,4_usize,1_usize,5_usize)]{
    let Some(&b)=index_by_axial.get(&(q+dq,axial_r+dr))else{continue};
    connections.push(BlueprintConnection{element_a:a,point_a:pa1,element_b:b,point_b:pb1});
    connections.push(BlueprintConnection{element_a:a,point_a:pa2,element_b:b,point_b:pb2});
   }
  }
 }
 StructuralBlueprint::new(elements,connections)
}
pub fn initial_genome()->Genome{Genome{traits:vec![trait_def("memory_strength",0.5,0.05),trait_def("perception_radius",100.0,1.0),trait_def("sensory_resolution",0.5,0.05),trait_def("directional_resolution",1.0,0.05),trait_def("mass_affinity",0.0,0.05),trait_def("potential_energy_affinity",0.5,0.05),trait_def("reactivity_affinity",0.0,0.05),trait_def("cohesion_affinity",0.0,0.05),trait_def("processing_efficiency",0.8,0.05),trait_def("movement_efficiency",0.8,0.05),trait_def("reproductive_investment",0.5,0.05)],structural_blueprint:default_structural_blueprint()}}
#[cfg(test)]mod tests{use super::*;use crate::resources::default_catalog;#[test]fn seed_blueprint_has_strong_core_hydrated_lattice_and_hydrated_membrane(){let g=initial_genome();let b=&g.structural_blueprint;assert_eq!(b.elements.len(),61);assert_eq!(b.connections.len(),312);assert!(b.validate().is_ok());assert_eq!(b.elements[0].material.parts[0].0,"Carbon");let hydrated=b.elements.iter().filter(|e|e.material.parts.iter().any(|(n,_)|n=="Water")).count();assert_eq!(hydrated,42);assert!(b.realize(&default_catalog()).is_ok())}#[test]fn seed_blueprint_is_connected(){assert!(initial_genome().structural_blueprint.is_connected())}#[test]fn seed_layers_have_expected_compositions(){let b=initial_genome().structural_blueprint;assert_eq!(b.elements.iter().filter(|e|e.material.parts.len()==1).count(),19);assert_eq!(b.elements.iter().filter(|e|e.material.parts.len()==2).count(),18);assert_eq!(b.elements.iter().filter(|e|e.material.parts.len()==3).count(),24)}}