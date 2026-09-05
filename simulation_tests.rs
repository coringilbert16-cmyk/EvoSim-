#[cfg(test)]
mod integration_tests {
    use crate::decision::{ActionKind, OutcomeKind};
    use crate::resources::{InternalBond, Material};
    use crate::state::{Position, Simulation, PROCESSING_RATE};
    use crate::structure::{Bond, Placement, StructuralUnit};

    #[test]
    fn fresh_organism_owns_an_empty_structure() {
        let organism = Simulation::create_initial_organism();
        assert!(organism.structure.units.is_empty());
        assert!(organism.structure.bonds.is_empty());
        assert!(organism.decision_history.entries.is_empty());
    }

    #[test]
    fn store_material_accepts_free_material_only() {
        let mut organism = Simulation::create_initial_organism();
        organism.store_material(Material::free_base("Carbon", 5.0));
        assert!((organism.stored_material.total_amount() - 5.0).abs() < 1e-9);

        let structured = Material {
            parts: vec![("Carbon".into(), 1.5), ("Hydrogen".into(), 1.5)],
            internal_bonds: vec![InternalBond { part_a: 0, part_b: 1 }],
        };
        assert!(structured.has_internal_structure());
        organism.store_material(structured);
        assert!((organism.stored_material.total_amount() - 5.0).abs() < 1e-9);
        assert!(organism.structure.units.is_empty());
    }

    #[test]
    fn structural_units_count_toward_total_material_conservation() {
        let mut sim = Simulation::new(1, 10.0);
        sim.organisms[0].structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement { x: 500.0, y: 500.0, rotation_radians: 0.0 },
        ));
        let before = sim.total_material_in_system();
        for _ in 0..500 { sim.step(); }
        let after = sim.total_material_in_system();
        assert!((before - after).abs() < 1e-3);
    }

    #[test]
    fn fresh_simulation_conserves_total_material_over_many_ticks() {
        let mut sim = Simulation::new(1, 10.0);
        let before = sim.total_material_in_system();
        for _ in 0..300 { sim.step(); }
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

    #[test]
    fn acquire_transfers_at_processing_rate_and_conserves_material() {
        let mut sim = Simulation::new(21, 10.0);
        let target = sim.environment.field.index_for_position(500.0, 500.0).unwrap();
        sim.environment.field.deposit_at_index(target, Material::free_base("Carbon", 10.0));
        sim.organisms[0].decision_history.record(ActionKind::Move, None, OutcomeKind::Harmful);

        let before = sim.total_material_in_system();
        sim.step();
        let after = sim.total_material_in_system();

        assert!((after - before).abs() < 1e-3);
        assert!((sim.organisms[0].stored_material.total_amount() - PROCESSING_RATE).abs() < 1e-9);
    }

    #[test]
    fn acquire_cannot_reach_distant_material() {
        let mut sim = Simulation::new(22, 10.0);
        let target = sim.environment.field.index_for_position(600.0, 500.0).unwrap();
        sim.environment.field.deposit_at_index(target, Material::free_base("Carbon", 10.0));

        sim.step();

        // Free ecological stock is allowed to diffuse between field cells.
        // This test therefore checks the actual acquisition invariant rather
        // than incorrectly requiring a distant cell to remain unchanged.
        assert!(sim.organisms[0].stored_material.is_empty());
        assert!(!sim.organisms[0].decision_history.entries.iter().any(|entry| entry.action == ActionKind::Acquire));
    }

    #[test]
    fn acquire_preserves_structured_material_in_field() {
        let mut sim = Simulation::new(23, 10.0);
        let target = sim.environment.field.index_for_position(500.0, 500.0).unwrap();
        let structured = Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![InternalBond { part_a: 0, part_b: 1 }],
        };
        sim.environment.field.deposit_at_index(target, structured.clone());
        let before = sim.environment.field.cells[target].materials.clone();

        sim.step();

        assert_eq!(sim.environment.field.cells[target].materials, before);
        assert!(sim.organisms[0].stored_material.is_empty());
    }

    #[test]
    fn acquire_selects_the_specific_target_and_records_that_outcome() {
        let mut sim = Simulation::new(24, 10.0);
        sim.organisms[0].occupied_cells[0] = Position { x: 500.0, y: 500.0 };
        let target_a = sim.environment.field.index_for_position(500.0, 500.0).unwrap();
        let target_b = sim.environment.field.index_for_position(475.0, 500.0).unwrap();
        sim.environment.field.deposit_at_index(target_a, Material::free_base("Carbon", 5.0));
        sim.environment.field.deposit_at_index(target_b, Material::free_base("Hydrogen", 5.0));
        let target_b_key = format!("target:{target_b}");
        sim.organisms[0].decision_history.record(ActionKind::Move, None, OutcomeKind::Harmful);
        sim.organisms[0].decision_history.record(
            ActionKind::Acquire,
            Some(format!("target:{target_a}")),
            OutcomeKind::Harmful,
        );

        sim.step();

        assert!(matches!(
            sim.organisms[0].decision_history.outcome(ActionKind::Acquire, Some(&target_b_key)),
            Some(OutcomeKind::Neutral)
        ));
        assert!(sim.organisms[0].stored_material.parts.iter().any(|(name, amount)| name == "Hydrogen" && *amount > 3.9));
        assert!(sim.organisms[0].stored_material.parts.iter().all(|(name, _)| name != "Carbon"));
    }

    fn add_test_break_bond(sim: &mut Simulation) {
        let a = sim.organisms[0].structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement { x: 500.0, y: 500.0, rotation_radians: 0.0 },
        ));
        let b = sim.organisms[0].structure.add_unit(StructuralUnit::new(
            "Methane",
            Placement { x: 501.0, y: 500.0, rotation_radians: 0.0 },
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
        assert!(sim.organisms[0].active_transformation_id.is_some());
        assert_eq!(sim.active_transformations.len(), 1);
        assert_eq!(sim.active_transformations[0].decision_context_key.as_deref(), Some("bond:0"));
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
        let expected_strength = crate::combine::bond_strength(
            *sim.organisms[0].structure.units[0].properties(&sim.environment.catalog).unwrap(),
            *sim.organisms[0].structure.units[1].properties(&sim.environment.catalog).unwrap(),
        );
        let expected_net_energy = 12.5 - expected_strength * 2.0;
        assert_eq!(sim.active_transformations.len(), 0);
        assert_eq!(sim.organisms[0].active_transformation_id, None);
        assert_eq!(sim.organisms[0].structure.bonds.len(), 0);
        assert!((sim.organisms[0].usable_energy - expected_net_energy).abs() < 1e-12);
        assert!((sim.energy_ledger.total_usable_energy_gained - expected_net_energy).abs() < 1e-12);
        assert!(sim.organisms[0].decision_history.has_knowledge(ActionKind::Break, Some("bond:0")));
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
        assert_eq!(sim.organisms[0].decision_history.entries.len(), 1);
        assert_eq!(sim.organisms[0].decision_history.entries[0].action, ActionKind::Break);
    }

    #[test]
    fn break_can_consume_usable_energy_when_work_exceeds_bond_energy() {
        let mut sim = Simulation::new(17, 10.0);
        let a = sim.organisms[0].structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement { x: 500.0, y: 500.0, rotation_radians: 0.0 },
        ));
        let b = sim.organisms[0].structure.add_unit(StructuralUnit::new(
            "Hydrogen",
            Placement { x: 501.0, y: 500.0, rotation_radians: 0.0 },
        ));
        sim.organisms[0].structure.add_bond(Bond {
            unit_a: a, point_a: 0, unit_b: b, point_b: 0, strength: 0.5, bond_energy: 0.1,
        });
        sim.organisms[0].usable_energy = 0.5;
        sim.step(); sim.step(); sim.step();
        let expected_work = crate::combine::bond_strength(
            *sim.organisms[0].structure.units[a].properties(&sim.environment.catalog).unwrap(),
            *sim.organisms[0].structure.units[b].properties(&sim.environment.catalog).unwrap(),
        ) * 2.0;
        let expected_remaining_energy = 0.5 - (expected_work - 0.1);
        let expected_heat = expected_work - 0.1;
        assert!(sim.organisms[0].structure.bonds.is_empty());
        assert!((sim.organisms[0].usable_energy - expected_remaining_energy).abs() < 1e-12);
        assert!((sim.energy_ledger.total_usable_energy_gained - 0.0).abs() < 1e-12);
        assert!((sim.energy_ledger.total_heat_dissipated - expected_heat).abs() < 1e-12);
        assert!(matches!(sim.organisms[0].decision_history.outcome(ActionKind::Break, Some("bond:0")), Some(OutcomeKind::Harmful)));
    }

    #[test]
    fn organism_can_break_a_structural_bond_and_records_outcome() {
        let mut sim = Simulation::new(7, 10.0);
        add_test_break_bond(&mut sim);
        let before = sim.organisms[0].usable_energy;
        for _ in 0..20 {
            sim.step();
            if sim.organisms[0].structure.bonds.is_empty() { break; }
        }
        assert!(sim.organisms[0].structure.bonds.is_empty());
        assert!((sim.organisms[0].usable_energy - before - (12.5 - crate::combine::bond_strength(
            *sim.organisms[0].structure.units[0].properties(&sim.environment.catalog).unwrap(),
            *sim.organisms[0].structure.units[1].properties(&sim.environment.catalog).unwrap(),
        ) * 2.0)).abs() < 1e-12);
        assert!(sim.organisms[0].decision_history.has_knowledge(ActionKind::Break, Some("bond:0")));
        assert!(matches!(sim.organisms[0].decision_history.outcome(ActionKind::Break, Some("bond:0")), Some(OutcomeKind::Beneficial | OutcomeKind::Neutral | OutcomeKind::Harmful)));
    }

    #[test]
    fn movement_records_an_actual_physical_outcome() {
        let mut sim = Simulation::new(11, 10.0);
        for _ in 0..50 { sim.step(); }
        let entry = sim.organisms[0].decision_history.entries.iter().find(|entry| entry.action == ActionKind::Move).expect("MOVE should have an executed outcome");
        assert!(entry.count > 0);
        assert_eq!(entry.context_key, None);
    }

    #[test]
    fn vent_emission_does_not_create_or_destroy_material() {
        let mut sim = Simulation::new(3, 10.0);
        let before = sim.total_material_in_system();
        for _ in 0..300 { sim.step_environment(); sim.tick += 1; }
        let after = sim.total_material_in_system();
        assert!((before - after).abs() < 1e-3);
    }
}
