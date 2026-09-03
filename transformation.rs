use crate::bond_interaction::BondInteractionSnapshot;
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
        let interaction_snapshot = BondInteractionSnapshot::from_bond(&bond)?;
        let complexity = crate::math::complexity(2.0);
        let duration = 1_u64.max(complexity.ceil() as u64);
        let break_work = break_work_cost(&bond, complexity);
        let required_energy = (break_work - interaction_snapshot.interaction.bond_energy()).max(0.0);
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
            // The complete bond is a snapshot of the interaction at decision
            // time. Structural identity and interaction state are conceptually
            // separate even though the serialized compatibility representation
            // still carries them together in Bond.
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
        let Some(interaction_snapshot) = BondInteractionSnapshot::from_bond(&target_bond) else {
            organism.active_transformation_id = None;
            return;
        };

        // VALIDATION: Verify bond endpoints still exist and are valid
        if target_bond.unit_a >= organism.structure.units.len()
            || target_bond.unit_b >= organism.structure.units.len()
        {
            eprintln!(
                "BREAK resolution failed for organism {}: \
                 bond endpoints reference invalid units (unit_a={}, unit_b={}, total units={})",
                organism.id, target_bond.unit_a, target_bond.unit_b, organism.structure.units.len()
            );
            organism.active_transformation_id = None;
            return;
        }

        // VALIDATION: Verify the stable structural identity still exists.
        if !organism.structure.bonds.iter().any(|bond| {
            crate::bond_interaction::BondIdentity::from_bond(bond)
                .matches(&interaction_snapshot.identity)
        }) {
            eprintln!(
                "BREAK resolution failed for organism {}: \
                 bond not found in structure. Expected bond with identity: \
                 unit_a={}, point_a={}, unit_b={}, point_b={}. \
                 This indicates a locking violation or structural corruption.",
                organism.id, target_bond.unit_a, target_bond.point_a, target_bond.unit_b, target_bond.point_b
            );
            organism.active_transformation_id = None;
            return;
        }

        // PRIMARY STRATEGY: Use structural identity matching only.
        // The organism is locked during transformation, so the bond structure cannot change.
        let Some(_removed_bond) = organism.structure.break_matching_bond(target_bond) else {
            eprintln!(
                "CRITICAL: BREAK resolution failed for organism {}: \
                 bond removal returned None after validation passed. \
                 This indicates an internal consistency error in break_matching_bond().",
                organism.id
            );
            organism.active_transformation_id = None;
            return;
        };

        // BREAK uses the interaction snapshot captured when the transformation
        // began. The bond's stored energy is the energy invested into that
        // interaction at formation; resolving BREAK releases that bond energy
        // and spends work against it. Only the unrecovered remainder is new
        // usable energy; it is not counted as new Floor 0 potential energy.
        let break_work = break_work_cost(&target_bond, transformation.complexity);
        let bond_energy = interaction_snapshot.interaction.bond_energy();
        let net_energy = bond_energy - break_work;
        ledger.total_bond_energy_released += bond_energy;
        ledger.total_work_consumed += break_work;
        if net_energy >= 0.0 {
            organism.usable_energy += net_energy;
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
/// Bond strength is the structural resistance and transformation complexity
/// supplies the work scale. The formation interaction snapshot supplies the
/// separate stored bond energy used by the BREAK energy accounting.
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
