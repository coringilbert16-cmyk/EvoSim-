use crate::decision::{ActionKind, OutcomeKind};
use crate::decision_runtime::ActionCandidate;
use crate::state::{
    ActiveTransformation, EnergyLedger, Environment, Organism, Simulation, STRESS_DECAY_PER_TICK,
};
use crate::structure::{formation_threshold, Bond};

const EPSILON: f64 = 1e-12;
const BREAK_SURPLUS_TO_USABLE: f64 = 0.40;
const BREAK_SURPLUS_TO_HEAT: f64 = 0.60;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BreakAttempt {
    pub bond: Bond,
    pub break_work: f64,
    pub usable_energy_spent: f64,
    pub usable_energy_gained: f64,
    pub heat_dissipated: f64,
}

fn calculate_break_attempt(
    organism: &Organism,
    environment: &Environment,
    bond: Bond,
) -> Option<BreakAttempt> {
    if !bond.bond_energy.is_finite()
        || bond.bond_energy < 0.0
        || !bond.strength.is_finite()
        || !(0.0..=1.0).contains(&bond.strength)
        || bond.unit_a >= organism.structure.units.len()
        || bond.unit_b >= organism.structure.units.len()
    {
        return None;
    }

    let unit_a = organism.structure.units.get(bond.unit_a)?;
    let unit_b = organism.structure.units.get(bond.unit_b)?;
    let props_a = *unit_a.properties(&environment.catalog)?;
    let props_b = *unit_b.properties(&environment.catalog)?;
    let load_a = organism
        .structure
        .connection_load(bond.unit_a, bond.point_a);
    let load_b = organism
        .structure
        .connection_load(bond.unit_b, bond.point_b);
    if !load_a.is_finite() || !load_b.is_finite() {
        return None;
    }

    // BREAK work reflects the bond's current state rather than the energy
    // originally invested to form it. Current material cohesion, connection
    // loading, and remaining bond strength all contribute to the work needed.
    let state_work = formation_threshold(props_a.cohesion, props_b.cohesion, load_a, load_b);
    if !state_work.is_finite() || state_work < 0.0 {
        return None;
    }
    let break_work = state_work * bond.strength;
    if !break_work.is_finite() || break_work < 0.0 {
        return None;
    }

    let bond_energy = bond.bond_energy;
    let (usable_energy_spent, usable_energy_gained, heat_dissipated) =
        if bond_energy > break_work + EPSILON {
            let surplus = bond_energy - break_work;
            let usable = surplus * BREAK_SURPLUS_TO_USABLE;
            let heat = surplus * BREAK_SURPLUS_TO_HEAT;
            if !usable.is_finite() || !heat.is_finite() {
                return None;
            }
            (0.0, usable, heat)
        } else if break_work > bond_energy + EPSILON {
            (break_work - bond_energy, 0.0, 0.0)
        } else {
            (0.0, 0.0, 0.0)
        };

    if !usable_energy_spent.is_finite()
        || !usable_energy_gained.is_finite()
        || !heat_dissipated.is_finite()
        || organism.usable_energy + EPSILON < usable_energy_spent
    {
        return None;
    }

    let conservation_balance =
        bond_energy + usable_energy_spent - break_work - usable_energy_gained - heat_dissipated;
    let tolerance = 1e-9 * bond_energy.max(break_work).max(1.0);
    if conservation_balance.abs() > tolerance {
        return None;
    }

    Some(BreakAttempt {
        bond,
        break_work,
        usable_energy_spent,
        usable_energy_gained,
        heat_dissipated,
    })
}

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
        environment: &mut Environment,
        ledger: &mut EnergyLedger,
    ) {
        let Some(target_bond) = transformation.bond else {
            organism.active_transformation_id = None;
            return;
        };

        // Calculate every physical and energetic consequence before touching
        // the structure. An energy failure therefore cannot partially break a
        // bond or alter organism energy.
        let Some(attempt) = calculate_break_attempt(organism, environment, target_bond) else {
            organism.active_transformation_id = None;
            return;
        };

        if organism
            .structure
            .break_matching_bond(attempt.bond)
            .is_none()
        {
            organism.active_transformation_id = None;
            return;
        }

        organism.usable_energy -= attempt.usable_energy_spent;
        organism.usable_energy += attempt.usable_energy_gained;
        ledger.record_break(
            attempt.bond.bond_energy,
            attempt.usable_energy_spent,
            attempt.break_work,
            attempt.usable_energy_gained,
            attempt.heat_dissipated,
        );
        organism.active_transformation_id = None;

        let outcome = if attempt.usable_energy_gained > EPSILON {
            OutcomeKind::Beneficial
        } else if attempt.usable_energy_spent > EPSILON {
            OutcomeKind::Harmful
        } else {
            OutcomeKind::Neutral
        };
        let candidate = ActionCandidate {
            action: ActionKind::Break,
            context_key: transformation.decision_context_key.clone(),
        };
        crate::decision_runtime::record_outcome(
            &mut organism.decision_history,
            &candidate,
            outcome,
        );
        if attempt.usable_energy_gained > EPSILON {
            let reinforcement =
                (attempt.usable_energy_gained * organism.genome.memory_strength()).clamp(0.0, 1.0);
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

pub(crate) fn reinforce_memory_point(organism: &mut Organism, x: f64, y: f64, reinforcement: f64) {
    if let Some(point) = organism
        .memory
        .iter_mut()
        .find(|p| (p.x - x).abs() < f64::EPSILON && (p.y - y).abs() < f64::EPSILON)
    {
        point.strength = (point.strength + reinforcement).clamp(0.0, 1.0);
    } else {
        organism.memory.push(crate::state::MemoryPoint {
            x,
            y,
            strength: reinforcement.clamp(0.0, 1.0),
        });
    }
}
