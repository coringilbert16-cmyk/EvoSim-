use crate::decision::{ActionKind, OutcomeKind};
use crate::decision_runtime::ActionCandidate;
use crate::state::{
    ActiveTransformation, EnergyLedger, Environment, Organism, Simulation, STRESS_DECAY_PER_TICK,
};

impl Simulation {
    pub(crate) fn try_start_transformation(
        organism: &mut Organism,
        next_id: &mut u64,
        decision: &ActionCandidate,
    ) -> Option<ActiveTransformation> {
        if decision.action != ActionKind::Break || organism.active_transformation_id.is_some() {
            return None;
        }
        let context_key = decision.context_key.as_deref()?;
        let bond_index = context_key.strip_prefix("bond:")?.parse::<usize>().ok()?;
        let bond = *organism.structure.bonds.get(bond_index)?;
        if !bond.bond_energy.is_finite() || bond.bond_energy < 0.0 {
            return None;
        }
        let complexity = crate::math::complexity(2.0);
        let duration = 1_u64.max(complexity.ceil() as u64);
        let break_work = break_work_cost(&bond, complexity);
        let required_energy = (break_work - bond.bond_energy).max(0.0);
        if organism.usable_energy + f64::EPSILON < required_energy {
            return None;
        }
        let transformation = ActiveTransformation {
            id: *next_id,
            organism_id: organism.id.clone(),
            kind: crate::state::TransformationKind::Break,
            material: crate::resources::Material {
                parts: Vec::new(),
                bonded: true,
            },
            bond: Some(bond),
            complexity,
            duration_ticks: duration,
            remaining_ticks: duration,
            decision_context_key: decision.context_key.clone(),
        };
        *next_id += 1;
        organism.active_transformation_id = Some(transformation.id);
        Some(transformation)
    }

    pub(crate) fn resolve_transformation(
        transformation: &ActiveTransformation,
        organism: &mut Organism,
        _environment: &mut Environment,
        ledger: &mut EnergyLedger,
    ) {
        let Some(target_bond) = transformation.bond else {
            organism.active_transformation_id = None;
            return;
        };

        // PRIMARY STRATEGY: Use structural identity matching only.
        // The organism is locked during transformation, so the bond structure cannot change.
        // This is the only correct strategy and should always succeed.
        let Some(removed_bond) = organism.structure.break_matching_bond(target_bond) else {
            // If we get here, something went wrong during the transformation.
            // The bond should ALWAYS exist because structure is locked and we validated
            // it at the start of the transformation.
            eprintln!(
                "CRITICAL: BREAK resolution failed for organism {}: \
                 bond with identity unit_a={}, point_a={}, unit_b={}, point_b={} not found. \
                 This indicates a locking violation or structural corruption.",
                organism.id,
                target_bond.unit_a,
                target_bond.point_a,
                target_bond.unit_b,
                target_bond.point_b
            );
            organism.active_transformation_id = None;
            return;
        };

        // BREAK is state-dependent. The bond carries stored interaction energy,
        // while its current structural strength determines how much work is
        // required to sever it. The net result may therefore release usable
        // energy or consume it. No mutable material-energy field is involved.
        let break_work = break_work_cost(&removed_bond, transformation.complexity);
        let net_energy = removed_bond.bond_energy - break_work;
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
        let outcome = if net_energy > 0.0 {
            OutcomeKind::Beneficial
        } else if net_energy < 0.0 {
            OutcomeKind::Harmful
        } else {
            OutcomeKind::Neutral
        };
        let candidate = ActionCandidate {
            action: ActionKind::Break,
            context_key: transformation.decision_context_key.clone(),
        };
        crate::decision_runtime::record_outcome(&mut organism.decision_history, &candidate, outcome);
        if net_energy > 0.0 {
            let reinforcement = (net_energy * organism.genome.memory_strength()).clamp(0.0, 1.0);
            let (px, py) = organism
                .occupied_cells
                .first()
                .map(|p| (p.x, p.y))
                .unwrap_or((0.0, 0.0));
            reinforce_memory_point(organism, px, py, reinforcement);
        }
    }

    pub(crate) fn apply_energy_capacity(organism: &mut Organism) {
        organism.stress *= STRESS_DECAY_PER_TICK;
    }
}

/// Experimental BREAK work model.
///
/// The locked rule requires current structural state to determine whether BREAK
/// releases or consumes usable energy. Bond strength is the current structural
/// resistance and transformation complexity supplies the work scale. The
/// parameterization is deliberately isolated here so it can be refined without
/// changing the transformation lifecycle.
pub(crate) fn break_work_cost(bond: &crate::structure::Bond, complexity: f64) -> f64 {
    bond.strength.clamp(0.0, 1.0) * complexity.max(0.0)
}

pub(crate) fn reinforce_memory_point(
    organism: &mut Organism,
    x: f64,
    y: f64,
    reinforcement: f64,
) {
    if let Some(point) = organism
        .memory
        .iter_mut()
        .find(|p| (p.x - x).abs() < f64::EPSILON && (p.y - y).abs() < f64::EPSILON)
    {
        point.strength = crate::math::clamp01(point.strength + reinforcement);
    } else {
        organism.memory.push(crate::state::MemoryPoint {
            x,
            y,
            strength: crate::math::clamp01(reinforcement),
        });
    }
}
