use crate::decision::{ActionCandidate, ActionKind, OutcomeKind};
use crate::math::exponential_influence;
use crate::state::{
    ActiveTransformation, EnergyLedger, Environment, Organism, Simulation,
    PROCESSING_RATE, PROCESSING_REACH, STRESS_DECAY_PER_TICK, TransformationKind,
};

impl Simulation {
    pub(crate) fn try_start_transformation(
        organism: &mut Organism,
        field: &mut crate::environment::ActiveMaterialField,
        next_id: &mut u64,
        decision: &ActionCandidate,
    ) -> Option<ActiveTransformation> {
        if decision.action != ActionKind::Break || organism.active_transformation_id.is_some() {
            return None;
        }

        let target = organism.resource_sense.sensed_resources.iter()
            .filter(|r| {
                r.desirability > 0.0
                    && r.distance <= PROCESSING_REACH
                    && r.bonded
                    && decision.context_key.as_deref().map_or(true, |key| r.name == key)
            })
            .max_by(|a, b| a.desirability.partial_cmp(&b.desirability).unwrap_or(std::cmp::Ordering::Equal))?
            .clone();

        let cell = &mut field.cells[target.field_index];
        if !cell.bonded.can_break() {
            return None;
        }

        let committed_amount = PROCESSING_RATE.min(cell.bonded.total_amount());
        if committed_amount <= 0.0 {
            return None;
        }
        let committed = field.take_at_index(target.field_index, true, committed_amount)?;
        let n = 2.0_f64;
        let c = crate::math::complexity(n);
        let duration = c.ceil().max(1.0) as u64;

        let transformation = ActiveTransformation {
            id: *next_id,
            organism_id: organism.id.clone(),
            kind: TransformationKind::Break,
            material: committed,
            complexity: c,
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
        let props = transformation.material.weighted_properties(&environment.catalog);
        let input_potential_energy = transformation.material.potential_energy(&environment.catalog);
        let yield_fraction = exponential_influence(props.reactivity);
        let gross_extracted = input_potential_energy * yield_fraction;
        let cohesion_tax_fraction = (props.cohesion * 0.5).clamp(0.0, 1.0);
        let cohesion_tax = gross_extracted * cohesion_tax_fraction;
        let net_extracted = (gross_extracted - cohesion_tax).max(0.0);
        let processing_efficiency = organism.genome.processing_efficiency();
        let usable_gained = net_extracted * processing_efficiency;
        let heat = gross_extracted - usable_gained;

        organism.usable_energy += usable_gained;
        organism.stress += heat;

        let (px, py) = {
            let p = &organism.occupied_cells[0];
            (p.x, p.y)
        };
        if !transformation.material.is_empty() {
            let waste = crate::resources::Material {
                parts: transformation.material.parts.clone(),
                bonded: false,
            };
            environment.field.deposit(px, py, waste);
        }

        ledger.total_potential_energy_released += gross_extracted;
        ledger.total_usable_energy_gained += usable_gained;
        ledger.total_heat_dissipated += heat;
        organism.active_transformation_id = None;

        let outcome = if usable_gained > 0.0 && heat <= usable_gained {
            OutcomeKind::Beneficial
        } else if usable_gained > 0.0 {
            OutcomeKind::Neutral
        } else {
            OutcomeKind::Harmful
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

        if usable_gained > 0.0 {
            let reinforcement = (usable_gained * organism.genome.memory_strength()).clamp(0.0, 1.0);
            Self::reinforce_memory_point(organism, px, py, reinforcement);
        }
    }

    pub(crate) fn apply_energy_capacity(organism: &mut Organism) {
        organism.stress *= STRESS_DECAY_PER_TICK;
    }
}
