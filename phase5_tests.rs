#[cfg(test)]
mod integration_tests {
    use crate::decision::{
        ActionEligibility, ActionKind, CurrentNeeds, DecisionHistory, OutcomeKind,
    };
    use crate::decision_runtime::{select_action, ActionCandidate, DecisionContext};
    use crate::state::Simulation;

    #[test]
    fn runtime_accumulates_reproductive_readiness_when_mature_and_energy_ready() {
        let mut sim = Simulation::new(1, 10.0);
        let initial = sim.organisms[0].reproductive_readiness;
        sim.step();
        let readiness = sim.organisms[0].reproductive_readiness;

        assert!(readiness > initial);
        assert!(readiness <= 1.0);
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
    fn active_transformation_blocks_all_action_eligibility() {
        let context = DecisionContext {
            needs: CurrentNeeds {
                survival: 1.0,
                reproduction: 1.0,
            },
            eligibility: ActionEligibility::default(),
        };
        let candidates = vec![
            ActionCandidate {
                action: ActionKind::Move,
                context_key: None,
            },
            ActionCandidate {
                action: ActionKind::Acquire,
                context_key: None,
            },
            ActionCandidate {
                action: ActionKind::Combine,
                context_key: None,
            },
            ActionCandidate {
                action: ActionKind::Break,
                context_key: Some("bond:1".into()),
            },
        ];

        assert!(select_action(context, &DecisionHistory::default(), &candidates).is_none());
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

    #[test]
    fn maintenance_reduces_usable_energy_during_runtime_step() {
        let mut sim = Simulation::new(12, 10.0);
        let before_energy = sim.organisms[0].usable_energy;
        sim.step();
        let after_energy = sim.organisms[0].usable_energy;
        assert!(after_energy < before_energy);
    }
}
