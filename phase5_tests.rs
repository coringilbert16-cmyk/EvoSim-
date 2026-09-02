#[cfg(test)]
mod integration_tests {
    use crate::decision::{ActionKind, DecisionHistory, OutcomeKind};
    use crate::state::{ActiveTransformation, Simulation, TransformationKind};
    use crate::structure::{Bond, BondId, Placement, StructuralUnit};

    fn prepare_bonded_pair(sim: &mut Simulation) -> BondId {
        let organism = &mut sim.organisms[0];
        organism.structure.units.clear();
        organism.structure.bonds.clear();
        organism.decision_history = DecisionHistory::default();
        organism.active_transformation_id = None;
        organism.usable_energy = 10.0;

        let a = organism.structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 500.0,
                y: 500.0,
                rotation_radians: 0.0,
            },
        ));
        let b = organism.structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 501.0,
                y: 500.0,
                rotation_radians: 0.0,
            },
        ));
        organism.structure.add_bond(Bond {
            id: BondId(0),
            unit_a: a,
            point_a: 0,
            unit_b: b,
            point_b: 0,
            strength: 0.5,
            bond_energy: 0.1,
        });
        organism.structure.bonds[0].id
    }

    fn active_break(sim: &mut Simulation, bond_id: BondId, remaining_ticks: u64) {
        sim.organisms[0].active_transformation_id = Some(1);
        sim.active_transformations.push(ActiveTransformation {
            id: 1,
            organism_id: "1".into(),
            kind: TransformationKind::Break,
            material: crate::resources::Material {
                parts: Vec::new(),
                bonded: true,
            },
            bond_id: Some(bond_id),
            legacy_bond: None,
            complexity: 2.0,
            duration_ticks: 2,
            remaining_ticks,
            decision_context_key: Some(format!("bond:{}", bond_id.0)),
        });
    }

    #[test]
    fn mature_energy_ready_organism_accumulates_reproductive_readiness() {
        let mut sim = Simulation::new(1, 10.0);
        let initial = sim.organisms[0].reproductive_readiness;
        sim.step();
        assert!(sim.organisms[0].reproductive_readiness > initial);
        assert!(sim.organisms[0].reproductive_readiness <= 1.0);
    }

    #[test]
    fn reproductive_readiness_requires_actual_structural_maturity() {
        let mut sim = Simulation::new(2, 10.0);
        let organism = &mut sim.organisms[0];
        organism.structure.units.clear();
        organism.structure.bonds.clear();
        organism.usable_energy = 100.0;
        organism.reproductive_readiness = 0.0;

        sim.step();

        assert_eq!(sim.organisms[0].reproductive_readiness, 0.0);
    }

    #[test]
    fn runtime_decision_history_changes_selected_action() {
        let mut sim = Simulation::new(3, 10.0);
        prepare_bonded_pair(&mut sim);
        sim.organisms[0].decision_history.record(
            ActionKind::Move,
            None,
            OutcomeKind::Beneficial,
        );

        sim.step();

        assert!(sim.active_transformations.is_empty());
        assert_eq!(sim.organisms[0].decision_history.entries.len(), 1);
        assert_eq!(
            sim.organisms[0].decision_history.entries[0].action,
            ActionKind::Move
        );
        assert_eq!(sim.organisms[0].decision_history.entries[0].count, 2);
    }

    #[test]
    fn active_transformation_blocks_new_decision_until_completion() {
        let mut sim = Simulation::new(4, 10.0);
        let bond_id = prepare_bonded_pair(&mut sim);
        active_break(&mut sim, bond_id, 2);
        let history_before = sim.organisms[0].decision_history.entries.len();

        sim.step();

        assert_eq!(sim.active_transformations.len(), 1);
        assert_eq!(sim.active_transformations[0].remaining_ticks, 1);
        assert_eq!(sim.organisms[0].active_transformation_id, Some(1));
        assert_eq!(sim.organisms[0].decision_history.entries.len(), history_before);

        sim.step();

        assert!(sim.active_transformations.is_empty());
        assert!(sim.organisms[0].active_transformation_id.is_none());
        assert!(sim.organisms[0].structure.bonds.is_empty());
    }

    #[test]
    fn same_seed_produces_same_runtime_snapshot() {
        let mut first = Simulation::new(4242, 10.0);
        let mut second = Simulation::new(4242, 10.0);

        for _ in 0..100 {
            first.step();
            second.step();
        }

        assert_eq!(
            serde_json::to_string(&first.snapshot()).expect("first snapshot serializes"),
            serde_json::to_string(&second.snapshot()).expect("second snapshot serializes")
        );
    }

    #[test]
    fn different_seeds_can_produce_different_runtime_history() {
        let mut first = Simulation::new(100, 10.0);
        let mut second = Simulation::new(200, 10.0);

        for _ in 0..100 {
            first.step();
            second.step();
        }

        assert_ne!(
            first.organisms[0].decision_history.entries,
            second.organisms[0].decision_history.entries
        );
    }

    #[test]
    fn runtime_snapshot_round_trip_preserves_structural_and_decision_state() {
        let mut sim = Simulation::new(13, 10.0);
        for _ in 0..20 {
            sim.step();
        }
        let snapshot = sim.snapshot();
        let json = serde_json::to_string(&snapshot).expect("snapshot serializes");
        let restored: crate::state::Snapshot =
            serde_json::from_str(&json).expect("snapshot deserializes");

        assert_eq!(snapshot.tick, restored.tick);
        assert_eq!(snapshot.organisms.len(), restored.organisms.len());
        assert_eq!(snapshot.active_transformations.len(), restored.active_transformations.len());
        assert_eq!(
            snapshot.energy_ledger.total_potential_energy_released,
            restored.energy_ledger.total_potential_energy_released
        );
        assert_eq!(
            snapshot.energy_ledger.total_usable_energy_gained,
            restored.energy_ledger.total_usable_energy_gained
        );
        assert_eq!(
            snapshot.energy_ledger.total_heat_dissipated,
            restored.energy_ledger.total_heat_dissipated
        );
        assert_eq!(snapshot.organisms[0].structure.units.len(), restored.organisms[0].structure.units.len());
        assert_eq!(snapshot.organisms[0].structure.bonds.len(), restored.organisms[0].structure.bonds.len());
        assert_eq!(snapshot.organisms[0].decision_history.entries, restored.organisms[0].decision_history.entries);
    }

    #[test]
    fn runtime_step_does_not_create_usable_energy_without_an_energy_producing_transformation() {
        let mut sim = Simulation::new(12, 10.0);
        let organism = &mut sim.organisms[0];
        organism.structure.units.clear();
        organism.structure.bonds.clear();
        organism.usable_energy = 7.0;
        organism.stored_unbonded = crate::resources::Material {
            parts: Vec::new(),
            bonded: false,
        };
        let before = organism.usable_energy;

        sim.step();

        assert_eq!(sim.organisms[0].usable_energy, before);
        assert_eq!(sim.energy_ledger.total_usable_energy_gained, 0.0);
        assert_eq!(sim.energy_ledger.total_potential_energy_released, 0.0);
    }
}
