use crate::decision::{ActionKind, OutcomeKind};
use crate::decision_runtime::ActionCandidate;
use crate::state::{ActiveTransformation, EnergyLedger, Environment, Organism, Simulation, STRESS_DECAY_PER_TICK};

impl Simulation {
    /// Organisms cannot intentionally BREAK their own structure. Structural
    /// bonds may still be removed by physical damage/repair pathways through
    /// the low-level structure API; this decision boundary is inert.
    pub(crate) fn try_start_transformation(
        _organism: &mut Organism,
        _catalog: &[crate::resources::BaseResource],
        _next_id: &mut u64,
        decision: &ActionCandidate,
    ) -> Option<ActiveTransformation> {
        if decision.action == ActionKind::Break {
            return None;
        }
        None
    }

    pub(crate) fn resolve_transformation(
        transformation: &ActiveTransformation,
        organism: &mut Organism,
        environment: &mut Environment,
        ledger: &mut EnergyLedger,
    ) {
        let Some(target_bond) = transformation.bond else {
            organism.active_transformation_id = None;
            return;
        };
        if target_bond.unit_a >= organism.structure.units.len() || target_bond.unit_b >= organism.structure.units.len() {
            organism.active_transformation_id = None;
            return;
        }
        if !organism.structure.bonds.iter().any(|b| b.has_same_identity(&target_bond)) {
            organism.active_transformation_id = None;
            return;
        }

        let Some(props_a) = organism.structure.units[target_bond.unit_a].properties(&environment.catalog).map(|p| *p) else {
            organism.active_transformation_id = None;
            return;
        };
        let Some(props_b) = organism.structure.units[target_bond.unit_b].properties(&environment.catalog).map(|p| *p) else {
            organism.active_transformation_id = None;
            return;
        };
        let break_work = break_work_cost(props_a, props_b, transformation.complexity);
        let Some(_removed_bond) = organism.structure.break_matching_bond(target_bond) else {
            organism.active_transformation_id = None;
            return;
        };

        let net_energy = target_bond.bond_energy - break_work;
        if net_energy >= 0.0 {
            organism.usable_energy += net_energy;
            ledger.total_potential_energy_released += net_energy;
            ledger.total_usable_energy_gained += net_energy;
        } else {
            let consumed = -net_energy;
            organism.usable_energy = (organism.usable_energy - consumed).max(0.0);
            ledger.total_heat_dissipated += consumed;
        }
        organism.active_transformation_id = None;
        let outcome = if net_energy > 0.0 { OutcomeKind::Beneficial } else if net_energy < 0.0 { OutcomeKind::Harmful } else { OutcomeKind::Neutral };
        let candidate = ActionCandidate { action: ActionKind::Break, context_key: transformation.decision_context_key.clone() };
        crate::decision_runtime::record_outcome(&mut organism.decision_history, &candidate, outcome);
        if net_energy > 0.0 {
            let reinforcement = (net_energy * organism.genome.memory_strength()).clamp(0.0, 1.0);
            let (px, py) = organism.occupied_cells.first().map(|p| (p.x, p.y)).unwrap_or((0.0, 0.0));
            reinforce_memory_point(organism, px, py, reinforcement);
        }
    }

    pub(crate) fn apply_energy_capacity(organism: &mut Organism) {
        organism.stress *= STRESS_DECAY_PER_TICK;
    }
}

pub(crate) fn break_work_cost(a: crate::resources::ResourceProperties, b: crate::resources::ResourceProperties, complexity: f64) -> f64 {
    crate::combine::bond_strength(a, b) * complexity.max(0.0)
}

pub(crate) fn reinforce_memory_point(organism: &mut Organism, x: f64, y: f64, reinforcement: f64) {
    if let Some(point) = organism.memory.iter_mut().find(|p| (p.x - x).abs() < f64::EPSILON && (p.y - y).abs() < f64::EPSILON) {
        point.strength = crate::math::clamp01(point.strength + reinforcement);
    } else {
        organism.memory.push(crate::state::MemoryPoint { x, y, strength: crate::math::clamp01(reinforcement) });
    }
}
