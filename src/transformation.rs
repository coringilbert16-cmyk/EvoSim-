use crate::break_runtime;
use crate::decision::{ActionKind, OutcomeKind};
use crate::decision_runtime::ActionCandidate;
use crate::energy_ledger::EnergyLedgerAuthority;
use crate::state::{ActiveTransformation, EnergyLedger, Environment, Organism, Simulation};

pub(crate) fn resolve_stress_break(
    organism: &mut Organism,
    environment: &Environment,
    ledger: &mut EnergyLedger,
) -> bool {
    let Some((_, target)) = organism.structure.bonds.iter().enumerate().min_by(|(_, a), (_, b)| {
        a.strength.partial_cmp(&b.strength).unwrap_or(std::cmp::Ordering::Equal)
    }) else { return false; };
    let target = *target;
    let Some(evaluation) = break_runtime::evaluate_organism_bond_break(
        organism, environment, target, crate::math::complexity(2.0),
    ) else { return false; };
    if !break_runtime::execute_break(&mut organism.structure, target, &mut organism.usable_energy, evaluation) {
        return false;
    }
    if !ledger.settle_break(evaluation.bond_energy, evaluation.interaction_energy, evaluation.work) {
        return false;
    }
    organism.add_transaction_stress(evaluation.work);
    true
}

impl Simulation {
    pub(crate) fn try_start_transformation(
        organism: &mut Organism,
        catalog: &[crate::resources::BaseResource],
        next_id: &mut u64,
        decision: &ActionCandidate,
    ) -> Option<ActiveTransformation> {
        if decision.action != ActionKind::Break || organism.active_transformation_id.is_some() { return None; }
        let key = decision.context_key.as_deref()?;
        let index = key.strip_prefix("bond:")?.parse::<usize>().ok()?;
        let bond = *organism.structure.bonds.get(index)?;
        if !bond.bond_energy.is_finite() || bond.bond_energy < 0.0 { return None; }
        let ia = organism.structure.unit_index(bond.endpoint_a.constituent_id)?;
        let ib = organism.structure.unit_index(bond.endpoint_b.constituent_id)?;
        let a = organism.structure.units.get(ia)?.properties(catalog)?;
        let b = organism.structure.units.get(ib)?.properties(catalog)?;
        let complexity = crate::math::complexity(2.0);
        let work = break_runtime::break_work_cost(a, b, complexity);
        if !work.is_finite() || work < 0.0 { return None; }
        let duration = 1_u64.max(complexity.ceil() as u64);
        let transformation = ActiveTransformation {
            id: *next_id, organism_id: organism.id.clone(), kind: crate::state::TransformationKind::Break,
            material: crate::resources::Material::free_base("", 0.0), bond: Some(bond), complexity,
            duration_ticks: duration, remaining_ticks: duration, decision_context_key: decision.context_key.clone(),
        };
        *next_id += 1;
        organism.active_transformation_id = Some(transformation.id);
        Some(transformation)
    }

    pub(crate) fn resolve_transformation(
        transformation: &ActiveTransformation,
        organism: &mut Organism,
        environment: &mut Environment,
        ledger: &mut EnergyLedger,
    ) {
        let Some(target) = transformation.bond else { organism.active_transformation_id = None; return; };
        let Some(evaluation) = break_runtime::evaluate_organism_bond_break(
            organism, environment, target, transformation.complexity,
        ) else { organism.active_transformation_id = None; return; };
        let net = evaluation.net_energy;
        if !break_runtime::execute_break(&mut organism.structure, target, &mut organism.usable_energy, evaluation) {
            organism.active_transformation_id = None;
            let candidate = ActionCandidate { action: ActionKind::Break, context_key: transformation.decision_context_key.clone() };
            crate::decision_runtime::record_outcome(&mut organism.decision_history, &candidate, OutcomeKind::Harmful);
            return;
        }
        if !ledger.settle_break(evaluation.bond_energy, evaluation.interaction_energy, evaluation.work) {
            organism.active_transformation_id = None;
            return;
        }
        organism.add_transaction_stress(evaluation.work);
        organism.active_transformation_id = None;
        let outcome = if net > f64::EPSILON { OutcomeKind::Beneficial } else if net < -f64::EPSILON { OutcomeKind::Harmful } else { OutcomeKind::Neutral };
        let candidate = ActionCandidate { action: ActionKind::Break, context_key: transformation.decision_context_key.clone() };
        crate::decision_runtime::record_outcome(&mut organism.decision_history, &candidate, outcome);
        if net > 0.0 {
            let reinforcement = (net * organism.genome.memory_strength()).clamp(0.0, 1.0);
            let (x, y) = organism.occupied_cells.first().map(|p| (p.x, p.y)).unwrap_or((0.0, 0.0));
            Self::reinforce_memory_point(organism, x, y, reinforcement);
        }
    }

    pub(crate) fn reinforce_memory_point(organism: &mut Organism, x: f64, y: f64, reinforcement: f64) {
        if let Some(point) = organism.memory.iter_mut().find(|p| (p.x - x).abs() < f64::EPSILON && (p.y - y).abs() < f64::EPSILON) {
            point.strength = crate::math::clamp01(point.strength + reinforcement);
        } else {
            organism.memory.push(crate::state::MemoryPoint { x, y, strength: crate::math::clamp01(reinforcement) });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn break_work_cost_is_owned_by_break_runtime() {
        let properties = crate::resources::ResourceProperties { mass: 1.0, potential_energy: 1.0, reactivity: 1.0, cohesion: 1.0 };
        assert!(break_runtime::break_work_cost(properties, properties, 2.0) >= 0.0);
    }
}
