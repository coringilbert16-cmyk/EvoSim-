#[cfg(test)]
mod integration_tests {
    use crate::decision::{
        ActionEligibility, ActionKind, CurrentNeeds, DecisionHistory, OutcomeKind,
    };
    use crate::decision_runtime::{select_action, ActionCandidate, DecisionContext};
    use crate::genome::initial_genome;
    use crate::state::{DevelopmentStage, Organism, Simulation};
    use crate::structure::{Placement, StructuralUnit};

    fn mature_organism() -> Organism {
        let mut organism = Simulation::create_initial_organism();
        organism.genome = initial_genome();
        organism.usable_energy = 16.0;
        organism.development_stage = DevelopmentStage::Adult;
        for i in 0..16 {
            organism.structure.add_unit(StructuralUnit::new(
                "Carbon",
                Placement {
                    x: i as f64,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            ));
        }
        organism
    }

    #[test]
    fn reproductive_readiness_requires_maturity_and_energy() {
        let environment = Simulation::new(1, 10.0).environment;
        let parameters = crate::decision::DecisionParameters::default();

        let mut immature = Simulation::create_initial_organism();
        immature.usable_energy = parameters.reproduction_reserve;
        Simulation::update_reproductive_readiness(&mut immature, &environment, parameters);
        assert_eq!(immature.reproductive_readiness, 0.0);

        let mut mature_without_energy = mature_organism();
        mature_without_energy.usable_energy = 0.0;
        Simulation::update_reproductive_readiness(
            &mut mature_without_energy,
            &environment,
            parameters,
        );
        assert_eq!(mature_without_energy.reproductive_readiness, 0.0);

        let mut ready = mature_organism();
        Simulation::update_reproductive_readiness(&mut ready, &environment, parameters);
        assert!(ready.reproductive_readiness > 0.0);
        assert!(ready.reproductive_readiness <= 1.0);
    }

    #[test]
    fn reproductive_readiness_accumulates_and_caps() {
        let environment = Simulation::new(2, 10.0).environment;
        let parameters = crate::decision::DecisionParameters::default();
        let mut organism = mature_organism();

        for _ in 0..200 {
            Simulation::update_reproductive_readiness(&mut organism, &environment, parameters);
        }

        assert!((organism.reproductive_readiness - 1.0).abs() < 1e-12);
    }

    #[test]
    fn decision_history_changes_selection_without_mutating_candidate_state() {
        let context = DecisionContext {
            needs: CurrentNeeds {
                survival: 1.0,
                reproduction: 0.0,
            },
            eligibility: ActionEligibility {
                can_move: true,
                can_break: true,
                ..Default::default()
            },
        };
        let candidates = vec![
            ActionCandidate {
                action: ActionKind::Move,
                context_key: None,
            },
            ActionCandidate {
                action: ActionKind::Break,
                context_key: Some("bond:1".into()),
            },
        ];
        let before = candidates.clone();
        let mut history = DecisionHistory::default();
        history.record(ActionKind::Move, None, OutcomeKind::Beneficial);

        let selected = select_action(context, &history, &candidates);

        assert_eq!(selected, Some(candidates[0].clone()));
        assert_eq!(candidates, before);
    }

    #[test]
    fn same_seed_produces_same_serialized_runtime_snapshots() {
        let mut first = Simulation::new(4242, 10.0);
        let mut second = Simulation::new(4242, 10.0);

        for _ in 0..100 {
            first.step();
            second.step();
        }

        let first_json =
            serde_json::to_string(&first.snapshot()).expect("first snapshot serializes");
        let second_json =
            serde_json::to_string(&second.snapshot()).expect("second snapshot serializes");
        assert_eq!(first_json, second_json);
    }

    #[test]
    fn different_seeds_do_not_force_identical_runtime_history() {
        let mut first = Simulation::new(100, 10.0);
        let mut second = Simulation::new(200, 10.0);

        for _ in 0..100 {
            first.step();
            second.step();
        }

        let first_json =
            serde_json::to_string(&first.snapshot()).expect("first snapshot serializes");
        let second_json =
            serde_json::to_string(&second.snapshot()).expect("second snapshot serializes");
        assert_ne!(first_json, second_json);
    }

    #[test]
    fn active_transformation_blocks_new_action_candidates() {
        let mut sim = Simulation::new(9, 10.0);
        let organism = &mut sim.organisms[0];
        organism.active_transformation_id = Some(99);
        let needs = CurrentNeeds {
            survival: 1.0,
            reproduction: 1.0,
        };
        let eligibility = Simulation::action_eligibility(organism, &sim.environment);
        assert!(!eligibility.can_move);
        assert!(!eligibility.can_combine);
        assert!(!eligibility.can_break);
        assert!(!eligibility.can_acquire);
        assert!(Simulation::decision_candidates(organism, needs, eligibility).is_empty());
    }

    #[test]
    fn maintenance_is_independent_of_action_selection() {
        let mut sim = Simulation::new(12, 10.0);
        let before_energy = sim.organisms[0].usable_energy;
        sim.step();
        let after_energy = sim.organisms[0].usable_energy;
        assert!(after_energy < before_energy);
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
        assert_eq!(
            snapshot.active_transformations.len(),
            restored.active_transformations.len()
        );
        assert_eq!(
            snapshot.energy_ledger.total_potential_energy_released,
            restored.energy_ledger.total_potential_energy_released
        );
        assert_eq!(
            snapshot.organisms[0].structure.units.len(),
            restored.organisms[0].structure.units.len()
        );
        assert_eq!(
            snapshot.organisms[0].structure.bonds.len(),
            restored.organisms[0].structure.bonds.len()
        );
        assert_eq!(
            snapshot.organisms[0].decision_history.entries.len(),
            restored.organisms[0].decision_history.entries.len()
        );
    }
}
