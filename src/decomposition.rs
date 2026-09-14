use crate::break_runtime;
use crate::energy_ledger::EnergyLedgerAuthority;
use crate::resources::Material;
use crate::state::{EnergyLedger, Environment, Organism, Position};
use crate::structure::OrganismStructure;

#[derive(Clone, Debug)]
pub(crate) struct DecomposingBody { pub(crate) structure: OrganismStructure, pub(crate) energy_budget: f64, pub(crate) position: Position }
impl DecomposingBody { pub(crate) fn new(structure: OrganismStructure, energy_budget: f64, position: Position) -> Option<Self> { if !energy_budget.is_finite() || energy_budget < 0.0 { return None; } Some(Self { structure, energy_budget, position }) } pub(crate) fn is_finished(&self) -> bool { self.structure.bonds.is_empty() } }

pub(crate) struct DecompositionStep { pub(crate) net_energy: f64, pub(crate) bond_energy: f64, pub(crate) break_interaction_energy: f64, pub(crate) heat: f64, pub(crate) released_material: Option<Vec<Material>> }

pub(crate) fn resolve_one_bond(body: &mut DecomposingBody, environment: &Environment, ledger: &mut EnergyLedger) -> Option<DecompositionStep> {
    let target = *body.structure.bonds.first()?;
    let ia = body.structure.unit_index(target.endpoint_a.constituent_id)?;
    let ib = body.structure.unit_index(target.endpoint_b.constituent_id)?;
    let a = body.structure.units.get(ia)?.properties(&environment.catalog)?;
    let b = body.structure.units.get(ib)?.properties(&environment.catalog)?;
    let candidate = crate::contact::connection_pair_candidates(&body.structure, ia, ib, &environment.catalog).into_iter().find(|candidate| candidate.endpoint_a == target.endpoint_a.location && candidate.endpoint_b == target.endpoint_b.location)?;
    let evaluation = break_runtime::evaluate_bond_break(&body.structure, target, a, b, candidate, break_runtime::water_field_amount(environment, &body.position), crate::math::complexity(2.0))?;
    let previous_energy = body.energy_budget;
    if !break_runtime::execute_break(&mut body.structure, target, &mut body.energy_budget, evaluation) { return None; }
    if !ledger.settle_decomposition(evaluation.bond_energy, evaluation.interaction_energy, evaluation.work) { body.energy_budget = previous_energy; return None; }
    let released_material = if body.is_finished() { Some(body.structure.units.iter().map(|unit| unit.material.clone()).collect()) } else { None };
    Some(DecompositionStep { net_energy: evaluation.net_energy, bond_energy: evaluation.bond_energy, break_interaction_energy: evaluation.interaction_energy, heat: evaluation.work, released_material })
}

pub(crate) fn harvestable_decomposition_energy(organisms: &[Organism], position: &Position) -> Option<usize> { organisms.iter().enumerate().filter_map(|(index, organism)| { let organism_position = organism.occupied_cells.first()?; let distance = (organism_position.x - position.x).hypot(organism_position.y - position.y); Some((index, distance)) }).min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)).map(|(index, _)| index) }

#[cfg(test)]
mod tests { use super::*; use crate::genome::initial_genome; use crate::resources::default_catalog;
#[test] fn retains_structure_budget_and_position(){let genome=initial_genome();let structure=genome.structural_blueprint.realize(&default_catalog()).unwrap();let body=DecomposingBody::new(structure,4.0,Position{x:2.0,y:3.0}).unwrap();assert!(!body.structure.units.is_empty());assert_eq!(body.energy_budget,4.0);assert_eq!(body.position,Position{x:2.0,y:3.0});}
#[test] fn zero_bond_structure_is_finished(){let genome=initial_genome();let mut structure=genome.structural_blueprint.realize(&default_catalog()).unwrap();structure.bonds.clear();let body=DecomposingBody::new(structure,0.0,Position{x:0.0,y:0.0}).unwrap();assert!(body.is_finished());}}
