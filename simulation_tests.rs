#[cfg(test)]
mod integration_tests {
    use crate::decision::{ActionKind, OutcomeKind};
    use crate::resources::{InternalBond, Material};
    use crate::state::{Position, Simulation};
    use crate::structure::{Bond, Placement, StructuralUnit};

    fn structured_carbon_hydrogen() -> Material {
        Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![InternalBond { part_a: 0, part_b: 1 }],
        }
    }

    #[test]
    fn fresh_organism_owns_an_empty_structure_and_storage() {
        let organism = Simulation::create_initial_organism();
        assert!(organism.structure.units.is_empty());
        assert!(organism.structure.bonds.is_empty());
        assert!(organism.stored_material.is_empty());
        assert!(organism.decision_history.entries.is_empty());
    }

    #[test]
    fn storage_contains_discrete_independent_material_objects() {
        let mut organism = Simulation::create_initial_organism();
        assert!(organism.store_material(Material::free_base("Carbon", 5.0)));
        assert_eq!(organism.stored_material.materials.len(), 5);
        assert_eq!(organism.stored_material.count_unstructured(), 5);
        assert_eq!(organism.stored_material.total_amount(), 5.0);
    }

    #[test]
    fn storage_preserves_a_compound_as_one_intact_object() {
        let mut organism = Simulation::create_initial_organism();
        let compound = structured_carbon_hydrogen();
        assert!(organism.store_material(compound.clone()));
        assert_eq!(organism.stored_material.materials, vec![compound]);
        assert_eq!(organism.stored_material.count_structured(), 1);
        assert!(organism.structure.units.is_empty());
    }

    #[test]
    fn acquire_moves_one_free_unit_into_storage_and_conserves_material() {
        let mut sim = Simulation::new(21, 10.0);
        let target = sim.environment.field.index_for_position(500.0, 500.0).unwrap();
        sim.environment.field.deposit_at_index(target, Material::free_base("Carbon", 10.0));
        sim.organisms[0].decision_history.record(ActionKind::Move, None, OutcomeKind::Harmful);

        let before = sim.total_material_in_system();
        sim.step();
        let after = sim.total_material_in_system();

        assert!((after - before).abs() < 1e-3);
        assert_eq!(sim.organisms[0].stored_material.total_amount(), 1.0);
        assert_eq!(sim.organisms[0].stored_material.materials.len(), 1);
        assert_eq!(sim.organisms[0].stored_material.materials[0].parts[0].0, "Carbon");
    }

    #[test]
    fn acquire_accepts_and_preserves_structured_material() {
        let mut sim = Simulation::new(23, 10.0);
        let target = sim.environment.field.index_for_position(500.0, 500.0).unwrap();
        let structured = structured_carbon_hydrogen();
        sim.environment.field.deposit_at_index(target, structured.clone());
        sim.organisms[0].decision_history.record(ActionKind::Move, None, OutcomeKind::Harmful);

        sim.step();

        assert!(sim.environment.field.cells[target].materials.is_empty());
        assert_eq!(sim.organisms[0].stored_material.materials, vec![structured]);
        assert_eq!(sim.organisms[0].stored_material.count_structured(), 1);
    }

    #[test]
    fn acquire_only_considers_the_currently_occupied_field_cell() {
        let mut sim = Simulation::new(22, 10.0);
        let distant = sim.environment.field.index_for_position(600.0, 500.0).unwrap();
        sim.environment.field.deposit_at_index(distant, Material::free_base("Carbon", 10.0));

        sim.step();

        assert!(sim.organisms[0].stored_material.is_empty());
        assert!(!sim.organisms[0].decision_history.entries.iter().any(|entry| entry.action == ActionKind::Acquire));
    }

    #[test]
    fn structural_material_is_not_opened_by_storage() {
        let mut organism = Simulation::create_initial_organism();
        assert!(organism.store_material(structured_carbon_hydrogen()));
        assert_eq!(organism.stored_material.count_unstructured(), 0);
        assert_eq!(organism.stored_material.count_structured(), 1);
    }

    #[test]
    fn fresh_simulation_conserves_total_material_over_many_ticks() {
        let mut sim = Simulation::new(1, 10.0);
        let before = sim.total_material_in_system();
        for _ in 0..300 { sim.step(); }
        let after = sim.total_material_in_system();
        assert!((before - after).abs() < 1e-3);
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
            unit_a: a, point_a: 0, unit_b: b, point_b: 0, strength: 0.8, bond_energy: 12.5,
        });
    }

    #[test]
    fn break_action_starts_a_transformation_before_resolution() {
        let mut sim = Simulation::new(7, 10.0);
        add_test_break_bond(&mut sim);
        sim.step();
        assert_eq!(sim.organisms[0].structure.bonds.len(), 1);
        assert!(sim.organisms[0].active_transformation_id.is_some());
        assert_eq!(sim.active_transformations.len(), 1);
    }

    #[test]
    fn break_resolution_changes_state_on_expected_tick() {
        let mut sim = Simulation::new(7, 10.0);
        add_test_break_bond(&mut sim);
        sim.step(); sim.step(); sim.step();
        let expected_strength = crate::combine::bond_strength(
            *sim.organisms[0].structure.units[0].properties(&sim.environment.catalog).unwrap(),
            *sim.organisms[0].structure.units[1].properties(&sim.environment.catalog).unwrap(),
        );
        let expected_net_energy = 12.5 - expected_strength * 2.0;
        assert!(sim.organisms[0].structure.bonds.is_empty());
        assert!((sim.organisms[0].usable_energy - expected_net_energy).abs() < 1e-12);
        assert!(sim.organisms[0].decision_history.has_knowledge(ActionKind::Break, Some("bond:0")));
    }
}
