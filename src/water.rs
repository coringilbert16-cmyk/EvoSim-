//! Organism water accounting and experimental permeability parameters.
//!
//! Water is ordinary organism material, not a parallel resource pool. This
//! module derives the physically accessible water mass from material that is
//! actually owned by the organism and supplies the current experimental
//! parameters for the locked permeability model.
//!
//! The parameters below are deliberately marked experimental. They are
//! simulation-tuning placeholders, not immutable properties of Water and not
//! conclusions about the final biology. The permeability equation itself
//! remains centralized in `permeability.rs`.

use crate::resources::BaseResource;
use crate::state::Organism;

pub(crate) const EXPERIMENTAL_WATER_THRESHOLD: f64 = 1.0;
pub(crate) const EXPERIMENTAL_WATER_FULL_PERMEABILITY: f64 = 5.0;
pub(crate) const EXPERIMENTAL_MAXIMUM_TRANSFER_CAPACITY: f64 = 1.0;

pub(crate) fn physically_accessible_water_mass(organism:&Organism,catalog:&[BaseResource])->f64{let water_in=|parts:&[(String,f64)]|parts.iter().filter(|(name,amount)|name=="Water"&&amount.is_finite()&&*amount>0.0).map(|(_,amount)|*amount).sum::<f64>();let structural_water=organism.structure.units.iter().map(|unit|water_in(unit.material.constituents())).sum::<f64>();let stored_water=organism.stored_material.materials.iter().map(|material|water_in(&material.parts)).sum::<f64>();let total=structural_water+stored_water;if total.is_finite()&&catalog.iter().any(|resource|resource.name=="Water"){total}else{0.0}}
pub(crate) fn experimental_transfer_capacity(organism:&Organism,catalog:&[BaseResource])->Option<f64>{let water_mass=physically_accessible_water_mass(organism,catalog);crate::permeability::permeability_from_water_mass(water_mass,EXPERIMENTAL_WATER_THRESHOLD,EXPERIMENTAL_WATER_FULL_PERMEABILITY,EXPERIMENTAL_MAXIMUM_TRANSFER_CAPACITY)}

#[cfg(test)]mod tests{use super::*;use crate::decision::DecisionHistory;use crate::genome::initial_genome;use crate::material_storage::MaterialStorage;use crate::resources::{InternalBond,Material};use crate::state::{DevelopmentStage,MemoryPoint,Organism,ResourceSense};use crate::structure::{OrganismStructure,Placement,StructuralUnit};fn organism_with_material(structure:OrganismStructure,stored:MaterialStorage)->Organism{Organism{id:"test".into(),occupied_cells:vec![],genome:initial_genome(),resource_sense:ResourceSense{sensed_resources:vec![],sensed_organisms:vec![],direction_x:0.0,direction_y:0.0,direction_strength:0.0},memory:Vec::<MemoryPoint>::new(),decision_history:DecisionHistory::default(),usable_energy:0.0,stress:0.0,stress_threshold:100.0,stored_material:stored,structure,development_stage:DevelopmentStage::Offspring,age:0,reproductive_readiness:0.0,active_transformation_id:None,reproductive_construction:None}}
#[test]fn water_accounting_includes_structural_and_stored_material(){let mut structure=OrganismStructure::new();let hydrated=Material{parts:vec![("Carbon".into(),1.0),("Water".into(),3.0)],internal_bonds:vec![InternalBond{part_a:0,part_b:1}]};structure.add_unit(StructuralUnit::from_material(hydrated,Placement{x:0.0,y:0.0,rotation_radians:0.0}).unwrap());let mut stored=MaterialStorage::default();assert!(stored.store(Material::free_base("Water",2.0)));let organism=organism_with_material(structure,stored);assert_eq!(physically_accessible_water_mass(&organism,&crate::resources::default_catalog()),5.0)}
#[test]fn non_water_material_does_not_contribute_to_water_mass(){let mut stored=MaterialStorage::default();assert!(stored.store(Material::free_base("Carbon",5.0)));let organism=organism_with_material(OrganismStructure::new(),stored);assert_eq!(physically_accessible_water_mass(&organism,&crate::resources::default_catalog()),0.0)}
#[test]fn missing_water_resource_fails_closed(){let organism=organism_with_material(OrganismStructure::new(),MaterialStorage::default());let catalog=crate::resources::default_catalog().into_iter().filter(|resource|resource.name!="Water").collect::<Vec<_>>();assert_eq!(physically_accessible_water_mass(&organism,&catalog),0.0)}
#[test]fn experimental_permeability_is_zero_below_threshold(){let organism=organism_with_material(OrganismStructure::new(),MaterialStorage::default());assert_eq!(experimental_transfer_capacity(&organism,&crate::resources::default_catalog()),Some(0.0))}
#[test]fn experimental_permeability_reaches_full_capacity_at_full_water(){let mut stored=MaterialStorage::default();assert!(stored.store(Material::free_base("Water",EXPERIMENTAL_WATER_FULL_PERMEABILITY)));let organism=organism_with_material(OrganismStructure::new(),stored);assert_eq!(experimental_transfer_capacity(&organism,&crate::resources::default_catalog()),Some(EXPERIMENTAL_MAXIMUM_TRANSFER_CAPACITY))}}
