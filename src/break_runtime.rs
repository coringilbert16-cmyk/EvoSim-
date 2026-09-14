use crate::combine::experimental_interaction;
use crate::contact::ConnectionPairCandidate;
use crate::energy_ledger::EnergyLedgerAuthority;
use crate::resources::ResourceProperties;
use crate::state::{EnergyLedger, Environment, Organism, Position};
use crate::structure::{Bond, OrganismStructure};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BreakEvaluation { pub(crate) net_energy:f64, pub(crate) bond_energy:f64, pub(crate) interaction_energy:f64, pub(crate) work:f64 }
pub(crate) fn water_field_amount(environment:&Environment,position:&Position)->f64{environment.field.index_for_position(position.x,position.y).map(|index|environment.field.cells[index].materials.iter().flat_map(|material|material.parts.iter()).filter(|(name,_)|name=="Water").map(|(_,amount)|*amount).sum()).unwrap_or(0.0)}
pub(crate) fn break_work_cost(a:ResourceProperties,b:ResourceProperties,complexity:f64)->f64{crate::combine::bond_strength(a,b)*complexity.max(0.0)}
pub(crate) fn evaluate_bond_break(structure:&OrganismStructure,target:Bond,a:ResourceProperties,b:ResourceProperties,candidate:ConnectionPairCandidate,water:f64,complexity:f64)->Option<BreakEvaluation>{if !target.bond_energy.is_finite()||target.bond_energy<0.0{return None}let interaction=experimental_interaction(a,b,candidate,water);let interaction_energy=-interaction.signed_value;let work=break_work_cost(a,b,complexity);if !interaction_energy.is_finite()||!work.is_finite()||work<0.0{return None}let net_energy=target.bond_energy+interaction_energy-work;if !net_energy.is_finite(){return None}if !structure.bonds.iter().any(|bond|bond.has_same_identity(&target)){return None}Some(BreakEvaluation{net_energy,bond_energy:target.bond_energy,interaction_energy,work})}

pub(crate) fn execute_break(structure:&mut OrganismStructure,target:Bond,available_energy:&mut f64,ledger:&mut EnergyLedger,evaluation:BreakEvaluation)->bool{
    if !available_energy.is_finite()||evaluation.net_energy.is_nan(){return false}
    let mut trial_energy=*available_energy;
    if !ledger.settle_break_holder(&mut trial_energy,evaluation.bond_energy,evaluation.interaction_energy,evaluation.work){return false}
    if structure.break_matching_bond(target).is_none(){return false}
    *available_energy=trial_energy;
    true
}

pub(crate) fn evaluate_organism_bond_break(organism:&Organism,environment:&Environment,target:Bond,complexity:f64)->Option<BreakEvaluation>{let ia=organism.structure.unit_index(target.endpoint_a.constituent_id)?;let ib=organism.structure.unit_index(target.endpoint_b.constituent_id)?;let a=organism.structure.units.get(ia)?.properties(&environment.catalog)?;let b=organism.structure.units.get(ib)?.properties(&environment.catalog)?;let candidate=crate::contact::connection_pair_candidates(&organism.structure,ia,ib,&environment.catalog).into_iter().find(|candidate|candidate.endpoint_a==target.endpoint_a.location&&candidate.endpoint_b==target.endpoint_b.location)?;let position=organism.occupied_cells.first().cloned().unwrap_or(Position{x:0.0,y:0.0});evaluate_bond_break(&organism.structure,target,a,b,candidate,water_field_amount(environment,&position),complexity)}
#[cfg(test)]mod tests{use super::*;#[test]fn break_work_cost_is_nonnegative_for_nonnegative_complexity(){let properties=ResourceProperties{mass:1.0,potential_energy:1.0,reactivity:1.0,cohesion:1.0};assert!(break_work_cost(properties,properties,2.0)>=0.0)}#[test]fn evaluation_rejects_missing_bond(){let structure=OrganismStructure::new();let target=Bond{endpoint_a:crate::structure::BondEndpoint::new(crate::structure::PhysicalConstituentId(1),crate::structure::ConnectionEndpoint::Fluid{x:0.0,y:0.0}),endpoint_b:crate::structure::BondEndpoint::new(crate::structure::PhysicalConstituentId(2),crate::structure::ConnectionEndpoint::Fluid{x:1.0,y:0.0}),strength:0.5,bond_energy:1.0};let candidate=ConnectionPairCandidate{endpoint_a:target.endpoint_a.location,endpoint_b:target.endpoint_b.location,distance:0.0,facing:1.0,load_a:0.0,load_b:0.0,available_a:true,available_b:true};let properties=ResourceProperties{mass:1.0,potential_energy:1.0,reactivity:1.0,cohesion:1.0};assert!(evaluate_bond_break(&structure,target,properties,properties,candidate,0.0,2.0).is_none())}}