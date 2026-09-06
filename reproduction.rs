//! Physical reproduction lifecycle.
//!
//! Reproduction commits the actual material required by the inherited
//! structural blueprint, including composite hydrated materials. The parent's
//! runtime structure is never copied directly into the child; the blueprint
//! carried by the child genome is the construction authority.
use rand_chacha::ChaCha8Rng;
use crate::material_storage::MaterialStorage;
use crate::resources::{BaseResource, Material};
use crate::state::{DevelopmentStage, Organism, ReproductiveConstruction, ResourceSense};
use crate::structure::OrganismStructure;

fn assemble_blueprint_material(remaining:&mut MaterialStorage,target:&Material)->Option<Material>{
 let mut inputs=Vec::with_capacity(target.parts.len());
 for(name,amount)in &target.parts{
  if(*amount-1.0).abs()>f64::EPSILON{return None}
  inputs.push(remaining.take_one_unstructured_named(name)?);
 }
 let assembled=if inputs.len()==1{inputs.into_iter().next()?}else{crate::resources::combine_materials(&inputs)};
 if assembled==*target{Some(assembled)}else{None}
}

pub(crate)fn begin_reproduction(parent:&mut Organism,rng:&mut ChaCha8Rng,catalog:&[BaseResource])->bool{
 if !matches!(parent.development_stage,DevelopmentStage::Adult)||parent.reproductive_readiness<1.0-f64::EPSILON||parent.reproductive_construction.is_some(){return false}
 let blueprint=parent.genome.structural_blueprint.clone();
 if blueprint.realize(catalog).is_err(){return false}
 let mut remaining=parent.stored_material.clone();
 let mut committed=Vec::with_capacity(blueprint.elements.len());
 for element in &blueprint.elements{let Some(material)=assemble_blueprint_material(&mut remaining,&element.material)else{return false};committed.push(material);}
 let mut child_genome=parent.genome.clone();
 child_genome.mutate(rng);
 parent.stored_material=remaining;
 parent.reproductive_readiness=0.0;
 parent.reproductive_construction=Some(ReproductiveConstruction{committed_material:MaterialStorage{materials:committed},developing_structure:OrganismStructure::new(),child_genome});
 true
}

pub(crate)fn advance_construction(construction:&mut ReproductiveConstruction,catalog:&[BaseResource],parent: &mut Organism)->bool{
 let next_index=construction.developing_structure.units.len();
 let Some(element)=construction.child_genome.structural_blueprint.elements.get(next_index)else{return false};
 let Some(material)=construction.committed_material.take_matching(&element.material)else{return false};
 let Some(unit)=crate::structure::StructuralUnit::from_material(material,element.placement)else{return false};
 let mut candidate=construction.developing_structure.clone();
 let new_index=candidate.add_unit(unit);
 for connection in &construction.child_genome.structural_blueprint.connections{
  if connection.element_a>new_index||connection.element_b>new_index{continue}
  if connection.element_a>=candidate.units.len()||connection.element_b>=candidate.units.len(){continue}
  if candidate.bonds.iter().any(|bond|(bond.unit_a==connection.element_a&&bond.point_a==connection.point_a&&bond.unit_b==connection.element_b&&bond.point_b==connection.point_b)||(bond.unit_a==connection.element_b&&bond.point_a==connection.point_b&&bond.unit_b==connection.element_a&&bond.point_b==connection.point_a)){continue}
  let a=candidate.connection_site(crate::structure::ConnectionSiteRef{unit_index:connection.element_a,point_index:connection.point_a},catalog);
  let b=candidate.connection_site(crate::structure::ConnectionSiteRef{unit_index:connection.element_b,point_index:connection.point_b},catalog);
  let(Some(a),Some(b))=(a,b)else{return false};
  if !crate::contact::connection_points_contact(a,&candidate.units[connection.element_a],b,&candidate.units[connection.element_b],1e-9,1.0-1e-9){return false}
  let pa=candidate.units[connection.element_a].properties(catalog).unwrap();
  let pb=candidate.units[connection.element_b].properties(catalog).unwrap();
  let strength=crate::combine::bond_strength(pa,pb);
  if !strength.is_finite()||!(0.0..=1.0).contains(&strength){return false}
  candidate.add_bond(crate::structure::Bond{unit_a:connection.element_a,point_a:connection.point_a,unit_b:connection.element_b,point_b:connection.point_b,strength,bond_energy:0.0});
  parent.add_transaction_stress(strength);
 }
 construction.developing_structure=candidate;
 true
}

pub(crate)fn finish_reproduction(parent:&mut Organism,child_id:String)->Option<Organism>{
 let construction=parent.reproductive_construction.take()?;
 let blueprint=&construction.child_genome.structural_blueprint;
 if !construction.committed_material.is_empty()||construction.developing_structure.units.len()!=blueprint.elements.len()||!blueprint.is_valid(){parent.reproductive_construction=Some(construction);return None}
 let position=match parent.occupied_cells.first().cloned(){Some(p)=>p,None=>{parent.reproductive_construction=Some(construction);return None}};
 Some(Organism{id:child_id,occupied_cells:vec![position],genome:construction.child_genome,resource_sense:ResourceSense{sensed_resources:Vec::new(),direction_x:0.0,direction_y:0.0,direction_strength:0.0},memory:Vec::new(),decision_history:crate::decision::DecisionHistory::default(),usable_energy:0.0,stress:0.0,stress_threshold:crate::state::INITIAL_STRESS_THRESHOLD,stored_material:MaterialStorage::default(),structure:construction.developing_structure,development_stage:DevelopmentStage::Offspring,age:0,reproductive_readiness:0.0,active_transformation_id:None,reproductive_construction:None})
}

#[cfg(test)]
mod tests{
 use super::*;use rand::SeedableRng;use crate::decision::DecisionHistory;use crate::genome::initial_genome;use crate::state::{Position,ResourceSense};
 fn adult_parent()->Organism{let catalog=crate::resources::default_catalog();let genome=initial_genome();let structure=genome.structural_blueprint.realize(&catalog).unwrap();let mut storage=MaterialStorage::default();for element in &genome.structural_blueprint.elements{assert!(storage.store(element.material.clone()));}Organism{id:"parent".into(),occupied_cells:vec![Position{x:50.0,y:50.0}],genome,resource_sense:ResourceSense{sensed_resources:Vec::new(),direction_x:0.0,direction_y:0.0,direction_strength:0.0},memory:Vec::new(),decision_history:DecisionHistory::default(),usable_energy:10.0,stress:0.0,stress_threshold:crate::state::INITIAL_STRESS_THRESHOLD,stored_material:storage,structure,development_stage:DevelopmentStage::Adult,age:10,reproductive_readiness:1.0,active_transformation_id:None,reproductive_construction:None}}
 #[test]fn reproduction_commits_blueprint_material_without_touching_parent_structure(){let catalog=crate::resources::default_catalog();let mut parent=adult_parent();let original_structure=parent.structure.clone();let mut rng=ChaCha8Rng::seed_from_u64(7);assert!(begin_reproduction(&mut parent,&mut rng,&catalog));assert_eq!(parent.structure,original_structure);assert!(parent.stored_material.is_empty());assert_eq!(parent.reproductive_readiness,0.0);let construction=parent.reproductive_construction.as_ref().unwrap();assert_eq!(construction.committed_material.total_amount(),183.0);assert_eq!(construction.committed_material.count_structured(),61)}
 #[test]fn construction_uses_inherited_blueprint_geometry_and_intact_composites(){let catalog=crate::resources::default_catalog();let mut parent=adult_parent();let mut rng=ChaCha8Rng::seed_from_u64(7);assert!(begin_reproduction(&mut parent,&mut rng,&catalog));let construction=parent.reproductive_construction.as_mut().unwrap();for _ in 0..20{assert!(advance_construction(construction,&catalog,&mut parent));}assert_eq!(construction.developing_structure.units.len(),20);assert_eq!(construction.developing_structure.units[0].placement,parent.genome.structural_blueprint.elements[0].placement);assert!(construction.developing_structure.units.iter().all(|u|u.material.is_composite()));assert_eq!(construction.committed_material.total_amount(),123.0);assert!(parent.stress>0.0)}
 #[test]fn completed_construction_becomes_an_independent_offspring(){let catalog=crate::resources::default_catalog();let mut parent=adult_parent();let mut rng=ChaCha8Rng::seed_from_u64(7);assert!(begin_reproduction(&mut parent,&mut rng,&catalog));let construction=parent.reproductive_construction.as_mut().unwrap();for _ in 0..61{assert!(advance_construction(construction,&catalog,&mut parent));}assert!(!advance_construction(construction,&catalog,&mut parent));let child=finish_reproduction(&mut parent,"2".into()).unwrap();assert_eq!(child.id,"2");assert!(matches!(child.development_stage,DevelopmentStage::Offspring));assert_eq!(child.structure.units.len(),61);assert_eq!(child.structure.bonds.len(),312);assert!(parent.reproductive_construction.is_none())}
 #[test]fn failed_material_commit_is_transactional(){let catalog=crate::resources::default_catalog();let mut parent=adult_parent();parent.stored_material=MaterialStorage::default();let mut rng=ChaCha8Rng::seed_from_u64(7);assert!(!begin_reproduction(&mut parent,&mut rng,&catalog));assert!(parent.stored_material.is_empty());assert!(parent.reproductive_construction.is_none());assert_eq!(parent.reproductive_readiness,1.0)}
}