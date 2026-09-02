use crate::decision::{ActionKind, OutcomeKind};
use crate::decision_runtime::ActionCandidate;
use crate::state::{ActiveTransformation, EnergyLedger, Environment, Organism, Simulation, STRESS_DECAY_PER_TICK};

impl Simulation {
    pub(crate) fn try_start_transformation(organism: &mut Organism, next_id: &mut u64, decision: &ActionCandidate) -> Option<ActiveTransformation> {
        if decision.action != ActionKind::Break || organism.active_transformation_id.is_some() { return None; }
        let context_key = decision.context_key.as_deref()?;
        let bond_index = context_key.strip_prefix("bond:")?.parse::<usize>().ok()?;
        let bond = *organism.structure.bonds.get(bond_index)?;
        if !bond.bond_energy.is_finite() || bond.bond_energy < 0.0 { return None; }
        let complexity = crate::math::complexity(2.0);
        let duration = 1_u64.max(complexity.ceil() as u64);
        let transformation = ActiveTransformation {
            id: *next_id,
            organism_id: organism.id.clone(),
            kind: crate::state::TransformationKind::Break,
            material: crate::resources::Material { parts: Vec::new(), bonded: true },
            bond: Some(bond), complexity, duration_ticks: duration, remaining_ticks: duration,
            decision_context_key: decision.context_key.clone(),
        };
        *next_id += 1;
        organism.active_transformation_id = Some(transformation.id);
        Some(transformation)
    }

    pub(crate) fn resolve_transformation(transformation: &ActiveTransformation, organism: &mut Organism, _environment: &mut Environment, ledger: &mut EnergyLedger) {
        let Some(target_bond) = transformation.bond else { organism.active_transformation_id = None; return; };
        let Some(removed_bond) = organism.structure.break_matching_bond(target_bond) else { organism.active_transformation_id = None; return; };
        let released = removed_bond.bond_energy.max(0.0);
        organism.usable_energy += released;
        ledger.total_potential_energy_released += released;
        ledger.total_usable_energy_gained += released;
        organism.active_transformation_id = None;
        let outcome = if released > 0.0 { OutcomeKind::Beneficial } else { OutcomeKind::Neutral };
        let candidate = ActionCandidate { action: ActionKind::Break, context_key: transformation.decision_context_key.clone() };
        crate::decision_runtime::record_outcome(&mut organism.decision_history, &candidate, outcome);
        if released > 0.0 {
            let reinforcement = (released * organism.genome.memory_strength()).clamp(0.0, 1.0);
            let (px, py) = organism.occupied_cells.first().map(|p| (p.x, p.y)).unwrap_or((0.0, 0.0));
            reinforce_memory_point(organism, px, py, reinforcement);
        }
    }

    pub(crate) fn apply_energy_capacity(organism: &mut Organism) { organism.stress *= STRESS_DECAY_PER_TICK; }
}

pub(crate) fn reinforce_memory_point(organism: &mut Organism, x: f64, y: f64, reinforcement: f64) {
    if let Some(point) = organism.memory.iter_mut().find(|p| (p.x - x).abs() < f64::EPSILON && (p.y - y).abs() < f64::EPSILON) {
        point.strength = crate::math::clamp01(point.strength + reinforcement);
    } else {
        organism.memory.push(crate::memory::MemoryPoint { x, y, strength: crate::math::clamp01(reinforcement) });
    }
}
