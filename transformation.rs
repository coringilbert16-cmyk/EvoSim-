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
        let identity = parse_bond_context_key(context_key)?;
        let bond = *organism
            .structure
            .bonds
            .iter()
            .find(|bond| bond_identity(bond) == identity)?;
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
            // The complete bond is a snapshot of the interaction at decision
            // time. Structural identity is used to locate the bond that must
            // be removed at resolution.
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

        // VALIDATION: Verify the bond actually exists in the structure before attempting removal
        if !organism
            .structure
            .bonds
            .iter()
            .any(|b| b.has_same_identity(&target_bond))
        {
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
        // began. The canonical strength is derived from the bond's formation
        // surplus; legacy Bond.strength is not authoritative.
        let break_work = break_work_cost(&target_bond, transformation.complexity);
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
    }

    pub(crate) fn apply_energy_capacity(organism: &mut Organism) {
        organism.stress *= STRESS_DECAY_PER_TICK;
    }
}

/// Return a canonical identity tuple for a bond. Endpoint ordering does not
/// matter, so the same structural bond has one decision-history context key.
fn bond_identity(bond: &crate::structure::Bond) -> ((usize, usize), (usize, usize)) {
    let left = (bond.unit_a, bond.point_a);
    let right = (bond.unit_b, bond.point_b);
    if left <= right {
        (left, right)
    } else {
        (right, left)
    }
}

fn parse_bond_context_key(key: &str) -> Option<((usize, usize), (usize, usize))> {
    let mut parts = key.strip_prefix("bond:")?.split(':');
    let unit_a = parts.next()?.parse().ok()?;
    let point_a = parts.next()?.parse().ok()?;
    let unit_b = parts.next()?.parse().ok()?;
    let point_b = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    let left = (unit_a, point_a);
    let right = (unit_b, point_b);
    Some(if left <= right {
        (left, right)
    } else {
        (right, left)
    })
}

/// Experimental BREAK work model.
///
/// Bond strength is derived from formation surplus. Bond.strength is retained
/// for compatibility with serialized legacy state but is not authoritative.
pub(crate) fn break_work_cost(bond: &crate::structure::Bond, complexity: f64) -> f64 {
    crate::combine::experimental_bond_strength(bond.bond_energy) * complexity.max(0.0)
}