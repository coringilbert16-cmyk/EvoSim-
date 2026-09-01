use crate::decision::{ActionKind, OutcomeKind};
use crate::decision_runtime::ActionCandidate;
use crate::state::{
    ActiveTransformation, EnergyLedger, Environment, Organism, Simulation,
    PROCESSING_REACH, STRESS_DECAY_PER_TICK,
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

        // BREAK acts on an existing structural bond. It does not remove bulk
        // field material and it never recomputes energy from raw resource
        // potential energy. The bond itself carries the stored energetic state.
        let duration = 1_u64.max(crate::math::complexity(2.0).ceil() as u64);
        let transformation = ActiveTransformation {
            id: *next_id,
            organism_id: organism.id.clone(),
            kind: crate::state::TransformationKind::Break,
            material: crate::resources::Material { parts: Vec::new(), bonded: true },
            bond: Some(bond),
            complexity: crate::math::complexity(2.0),
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
            // Legacy/incomplete transformations have no authoritative bond
            // energy. Do not fall back to raw material potential energy.
            organism.active_transformation_id = None;
            return;
        };

        let Some(removed_bond) = organism.structure.break_matching_bond(target_bond) else {
            organism.active_transformation_id = None;
            return;
        };

        let released = removed_bond.bond_energy.max(0.0);
        organism.usable_energy += released;
        ledger.total_potential_energy_released += released;
        ledger.total_usable_energy_gained += released;
        organism.active_transformation_id = None;

        let outcome = if released > 0.0 {
            OutcomeKind::Beneficial
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

        if released > 0.0 {
            let reinforcement = (released * organism.genome.memory_strength()).clamp(0.0, 1.0);
            let (px, py) = organism
                .occupied_cells
                .first()
                .map(|p| (p.x, p.y))
                .unwrap_or((0.0, 0.0));
            Self::reinforce_memory_point(organism, px, py, reinforcement);
        }
    }

    pub(crate) fn apply_energy_capacity(organism: &mut Organism) {
        organism.stress *= STRESS_DECAY_PER_TICK;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::{ActionKind, DecisionHistory};
    use crate::decision_runtime::ActionCandidate;
    use crate::genome::initial_genome;
    use crate::resources::Material;
    use crate::state::{DevelopmentStage, Position, ResourceSense};
    use crate::structure::{Bond, OrganismStructure, Placement, StructuralUnit};

    fn organism() -> Organism {
        Organism {
            id: "test".into(),
            occupied_cells: vec![Position { x: 0.0, y: 0.0 }],
            genome: initial_genome(),
            resource_sense: ResourceSense { sensed_resources: Vec::new(), direction_x: 0.0, direction_y: 0.0, direction_strength: 0.0 },
            memory: Vec::new(),
            decision_history: DecisionHistory::default(),
            usable_energy: 0.0,
            stress: 0.0,
            stored_unbonded: Material { parts: Vec::new(), bonded: false },
            structure: OrganismStructure::new(),
            development_stage: DevelopmentStage::Juvenile,
            age: 0,
            active_transformation_id: None,
        }
    }

    #[test]
    fn break_transformation_uses_stored_bond_energy() {
        let mut o = organism();
        let a = o.structure.add_unit(StructuralUnit::new("Carbon", Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 }));
        let b = o.structure.add_unit(StructuralUnit::new("Methane", Placement { x: 1.0, y: 0.0, rotation_radians: 0.0 }));
        o.structure.bonds.push(Bond { unit_a: a, point_a: 0, unit_b: b, point_b: 0, strength: 0.8, bond_energy: 7.25 });
        let decision = ActionCandidate { action: ActionKind::Break, context_key: Some("bond:0".into()) };
        let t = Simulation::try_start_transformation(&mut o, &mut 1, &decision).unwrap();
        let mut ledger = EnergyLedger::default();
        let mut environment = Simulation::new(1, 1.0).environment;
        Simulation::resolve_transformation(&t, &mut o, &mut environment, &mut ledger);
        assert!((o.usable_energy - 7.25).abs() < 1e-12);
        assert!((ledger.total_usable_energy_gained - 7.25).abs() < 1e-12);
        assert!(o.structure.bonds.is_empty());
    }

    #[test]
    fn break_does_not_fall_back_to_raw_material_potential_energy() {
        let mut o = organism();
        let a = o.structure.add_unit(StructuralUnit::new("Carbon", Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 }));
        let b = o.structure.add_unit(StructuralUnit::new("Methane", Placement { x: 1.0, y: 0.0, rotation_radians: 0.0 }));
        let decision = ActionCandidate { action: ActionKind::Break, context_key: Some("bond:0".into()) };
        assert!(Simulation::try_start_transformation(&mut o, &mut 1, &decision).is_none());
        assert_eq!(a, 0);
        assert_eq!(b, 1);
    }
}
