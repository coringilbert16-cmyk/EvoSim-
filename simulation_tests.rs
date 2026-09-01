#[cfg(test)]
mod integration_tests {
    use crate::decision::{ActionKind, OutcomeKind};
    use crate::resources::Material;
    use crate::state::Simulation;

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
        organism.store_unbonded_material(Material { parts: vec![("Carbon".into(), 5.0)], bonded: false });
        assert!((organism.stored_unbonded.total_amount() - 5.0).abs() < 1e-9);
        organism.store_unbonded_material(Material { parts: vec![("Carbon".into(), 3.0)], bonded: true });
        assert!((organism.stored_unbonded.total_amount() - 5.0).abs() < 1e-9);
        assert!(organism.structure.units.is_empty());
    }

    #[test]
    fn structural_units_count_toward_total_material_conservation() {
        let mut sim = Simulation::new(1, 10.0);
        sim.organisms[0].structure.add_unit(crate::structure::StructuralUnit::new(
            "Carbon",
            crate::structure::Placement { x: 500.0, y: 500.0, rotation_radians: 0.0 },
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
        for _ in 0..3000 { sim.step(); }
        let after = sim.total_material_in_system();
        assert!((before - after).abs() < 1e-3);
    }

    #[test]
    fn no_resource_cloud_pathway_exists() {
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
    fn organism_can_break_bonded_material_once_available_and_records_outcome() {
        let mut sim = Simulation::new(7, 10.0);
        let (px, py) = {
            let p = &sim.organisms[0].occupied_cells[0];
            (p.x, p.y)
        };
        sim.environment.field.deposit(
            px,
            py,
            Material { parts: vec![("Methane".into(), 50.0)], bonded: true },
        );

        let mut ever_gained_energy = false;
        for _ in 0..200 {
            sim.step();
            if sim.organisms[0].usable_energy > 0.0 {
                ever_gained_energy = true;
                break;
            }
        }
        assert!(ever_gained_energy);
        assert!(sim.organisms[0].decision_history.has_knowledge(
            ActionKind::Break,
            Some("Methane")
        ));
        assert!(matches!(
            sim.organisms[0].decision_history.outcome(ActionKind::Break, Some("Methane")),
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
        for _ in 0..1000 {
            sim.step_environment();
            sim.tick += 1;
        }
        let after = sim.total_material_in_system();
        assert!((before - after).abs() < 1e-3);
    }
}
