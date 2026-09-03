#[cfg(test)]
mod integration_tests {
    use crate::decision::{ActionKind, OutcomeKind};
    use crate::resources::Material;
    use crate::state::Simulation;
    use crate::structure::{Bond, Placement, StructuralUnit};

    #[test]
    fn fresh_organism_owns_an_empty_structure() {
        let organism = Simulation::create_initial_organism();
        assert!(organism.structure.units.is_empty());
        assert!(organism.structure.bonds.is_empty());
        assert!(organism.decision_history.entries.is_empty());
    }

    #[test]
    fn store_unbonded_material_accepts_raw_material_only() {
        let mut organism = Simulation::create_initial_organism();
        organism.store_unbonded_material(Material {
            parts: vec![("Carbon".into(), 5.0)],
            bonded: false,
        });
        assert!((organism.stored_unbonded.total_amount() - 5.0).abs() < 1e-9);
        organism.store_unbonded_material(Material {
            parts: vec![("Carbon".into(), 3.0)],
            bonded: true,
        });
        assert!((organism.stored_unbonded.total_amount() - 5.0).abs() < 1e-9);
        assert!(organism.structure.units.is_empty());
    }

    #[test]
    fn structural_units_count_toward_total_material_conservation() {
        let mut sim = Simulation::new(1, 10.0);
        sim.organisms[0].structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 500.0,
                y: 500.0,
                rotation_radians: 0.0,
            },
        ));
        let before = sim.total_material_in_system();
        for _ in 0..500 {
            sim.step();
        }
        let after = sim.total_material_in_system();
        assert!((before - after).abs() < 1e-3);
    }

    #[test]
    fn fresh_simulation_conserves_total_material_over_many_ticks() {
        let mut sim = Simulation::new(1, 10.0);
        let before = sim.total_material_in_system();
        for _ in 0..300 {
            sim.step();
        }
        let after = sim.total_material_in_system();
        assert!((before - after).abs() < 1e-3);
    }

    #[test]
    fn environment_contains_field_and_reservoir_layers() {
        let sim = Simulation::new(1, 10.0);
        assert!(!sim.environment.field.cells.is_empty());
        assert!(!sim.environment.reservoir.cells.is_empty());
    }

    #[test]
    fn organism_can_perceive_material_from_the_field() {
        let mut sim = Simulation::new(7, 10.0);
        let mut ever_sensed_something = false;
        for _ in 0..500 {
            sim.step();
            if !sim.organisms[0].resource_sense.sensed_resources.is_empty() {
                ever_sensed_something = true;
            }
        }
        assert!(ever_sensed_something);
    }

    fn add_test_break_bond(sim: &mut Simulation) {
        let a = sim.organisms[0].structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 500.0,
                y: 500.0,
                rotation_radians: 0.0,
            },
        ));
        let b = sim.organisms[0].structure.add_unit(StructuralUnit::new(
            "Methane",
            Placement {
                x: 501.0,
                y: 500.0,
                rotation_radians: 0.0,
            },
        ));
        sim.organisms[0].structure.add_bond(Bond {
            unit_a: a,
            point_a: 0,
            unit_b: b,
            point_b: 0,
            strength: 0.8,
            bond_energy: 12.5,
        });
    }

    #[test]
    fn break_action_starts_a_transformation_before_resolution() {
        let mut sim = Simulation::new(7, 10.0);
        add_test_break_bond(&mut sim);

        assert_eq!(sim.organisms[0].structure.bonds.len(), 1);
        sim.step();

        assert_eq!(sim.organisms[0].structure.bonds.len(), 1);
        assert!(
            sim.organisms[0].active_transformation_id.is_some(),
            "BREAK should start a transformation before its delayed resolution"
        );
        assert_eq!(sim.active_transformations.len(), 1);
        assert_eq!(
            sim.active_transformations[0].decision_context_key.as_deref(),
            Some("bond:0")
        );
    }

    #[test]
    fn break_resolution_changes_state_on_expected_tick() {
        let mut sim = Simulation::new(7, 10.0);
        add_test_break_bond(&mut sim);

        sim.step();
        assert_eq!(sim.active_transformations.len(), 1);
        assert_eq!(sim.active_transformations[0].remaining_ticks, 2);
        assert_eq!(sim.organisms[0].active_transformation_id, Some(1));

        sim.step();
        assert_eq!(sim.active_transformations.len(), 1);
        assert_eq!(sim.active_transformations[0].remaining_ticks, 1);
        assert_eq!(sim.organisms[0].structure.bonds.len(), 1);
        assert_eq!(sim.organisms[0].usable_energy, 0.0);

        sim.step();
        assert_eq!(sim.active_transformations.len(), 0);
        assert_eq!(sim.organisms[0].active_transformation_id, None);
        assert_eq!(sim.organisms[0].structure.bonds.len(), 0);
        assert!((sim.organisms[0].usable_energy - 10.9).abs() < 1e-12);
        assert!((sim.energy_ledger.total_usable_energy_gained - 10.9).abs() < 1e-12);
        assert!(sim.organisms[0]
            .decision_history
            .has_knowledge(ActionKind::Break, Some("bond:0")));
    }

    #[test]
    fn resolved_transformation_cannot_trigger_a_second_action_in_the_same_tick() {
        let mut sim = Simulation::new(7, 10.0);
        add_test_break_bond(&mut sim);

        sim.step();
        sim.step();
        sim.step();

        assert_eq!(sim.organisms[0].structure.bonds.len(), 0);
        assert_eq!(sim.active_transformations.len(), 0);
        assert_eq!(
            sim.organisms[0].decision_history.entries.len(),
            1,
            "BREAK resolution must not permit a second action during the same tick"
        );
        assert_eq!(sim.organisms[0].decision_history.entries[0].action, ActionKind::Break);
    }

    #[test]
    fn break_can_consume_usable_energy_when_work_exceeds_bond_energy() {
        let mut sim = Simulation::new(17, 10.0);
        let a = sim.organisms[0].structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 500.0,
                y: 500.0,
                rotation_radians: 0.0,
            },
        ));
        let b = sim.organisms[0].structure.add_unit(StructuralUnit::new(
            "Hydrogen",
            Placement {
                x: 501.0,
                y: 500.0,
                rotation_radians: 0.0,
            },
        ));
        sim.organisms[0].structure.add_bond(Bond {
            unit_a: a,
            point_a: 0,
            unit_b: b,
            point_b: 0,
            strength: 0.5,
            bond_energy: 0.5,
        });
        sim.organisms[0].usable_energy = 0.5;

        sim.step();
        sim.step();
        sim.step();

        assert!(sim.organisms[0].structure.bonds.is_empty());
        assert!((sim.organisms[0].usable_energy - 0.0).abs() < 1e-12);
        assert!((sim.energy_ledger.total_usable_energy_gained - 0.0).abs() < 1e-12);
        assert!((sim.energy_ledger.total_heat_dissipated - 0.5).abs() < 1e-12);
        assert!(matches!(
            sim.organisms[0]
                .decision_history
                .outcome(ActionKind::Break, Some("bond:0")),
            Some(OutcomeKind::Harmful)
        ));
    }

    #[test]
    fn organism_can_break_a_structural_bond_and_records_outcome() {
        let mut sim = Simulation::new(7, 10.0);
        add_test_break_bond(&mut sim);
        let before = sim.organisms[0].usable_energy;
        for _ in 0..20 {
            sim.step();
            if sim.organisms[0].structure.bonds.is_empty() {
                break;
            }
        }
        assert!(sim.organisms[0].structure.bonds.is_empty());
        assert!((sim.organisms[0].usable_energy - before - 10.9).abs() < 1e-12);
        assert!(sim.organisms[0]
            .decision_history
            .has_knowledge(ActionKind::Break, Some("bond:0")));
        assert!(matches!(
            sim.organisms[0]
                .decision_history
                .outcome(ActionKind::Break, Some("bond:0")),
            Some(OutcomeKind::Beneficial | OutcomeKind::Neutral | OutcomeKind::Harmful)
        ));
    }

    #[test]
    fn movement_records_an_actual_physical_outcome() {
        let mut sim = Simulation::new(11, 10.0);
        for _ in 0..50 {
            sim.step();
        }
        let entry = sim.organisms[0]
            .decision_history
            .entries
            .iter()
            .find(|entry| entry.action == ActionKind::Move)
            .expect("MOVE should have an executed outcome");
        assert!(entry.count > 0);
        assert_eq!(entry.context_key, None);
    }

    #[test]
    fn vent_emission_does_not_create_or_destroy_material() {
        let mut sim = Simulation::new(3, 10.0);
        let before = sim.total_material_in_system();
        for _ in 0..300 {
            sim.step_environment();
            sim.tick += 1;
        }
        let after = sim.total_material_in_system();
        assert!((before - after).abs() < 1e-3);
    }
}
