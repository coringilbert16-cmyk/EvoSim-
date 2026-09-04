#[cfg(test)]
mod integration_tests {
    use crate::genome::initial_genome_variant;
    use crate::resources::Material;
    use crate::state::Simulation;
    use crate::structure::{OrganismStructure, Placement, StructuralUnit};

    #[test]
    fn fresh_organism_has_a_valid_inherited_seed_structure() {
        let organism = Simulation::create_initial_organism();
        let blueprint = &organism.genome.structural_blueprint;

        assert!(blueprint.validate().is_ok());
        assert!(!organism.structure.units.is_empty());
        assert_eq!(organism.structure.units.len(), blueprint.elements.len());
        assert_eq!(organism.structure.bonds.len(), blueprint.connections.len());
        assert!(organism.structure.units.iter().all(|unit| unit.blueprint_index.is_some()));
    }

    #[test]
    fn seed_variants_can_produce_different_valid_configurations() {
        let first = initial_genome_variant(0).structural_blueprint;
        let second = initial_genome_variant(1).structural_blueprint;

        assert!(first.validate().is_ok());
        assert!(second.validate().is_ok());
        assert_ne!(first.elements[1].placement, second.elements[1].placement);
    }

    #[test]
    fn store_unbonded_material_accepts_only_unbonded_stock() {
        let mut organism = Simulation::create_initial_organism();
        let initial_structure_units = organism.structure.units.len();

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
        assert_eq!(organism.structure.units.len(), initial_structure_units);
    }

    #[test]
    fn growth_adds_only_the_next_inherited_blueprint_element() {
        let mut organism = Simulation::create_initial_organism();
        let blueprint = organism.genome.structural_blueprint.clone();
        let catalog = crate::resources::default_catalog();

        organism.structure = OrganismStructure::new();
        let first = &blueprint.elements[0];
        organism.structure.add_unit(StructuralUnit::from_blueprint_indexed(
            first.material.clone(),
            first.geometry.clone(),
            first.placement,
            0,
        ));
        let second = &blueprint.elements[1];
        organism.stored_unbonded = Material {
            parts: second.material.material.parts.clone(),
            bonded: false,
        };
        let before_position = organism.structure.units[0].placement;

        assert!(crate::growth::grow_one_element(&mut organism, &catalog));
        assert_eq!(organism.structure.units.len(), 2);
        assert_eq!(organism.structure.units[0].placement, before_position);
        assert!(organism.structure.units.iter().any(|unit| unit.blueprint_index == Some(1)));
    }

    #[test]
    fn growth_cannot_invent_a_non_blueprint_unit() {
        let mut organism = Simulation::create_initial_organism();
        let catalog = crate::resources::default_catalog();
        organism.structure = OrganismStructure::new();
        organism.structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 },
        ));
        organism.stored_unbonded = Material::free_base("Hydrogen", 100.0);
        let before = organism.structure.units.len();

        assert!(!crate::growth::grow_one_element(&mut organism, &catalog));
        assert_eq!(organism.structure.units.len(), before);
    }

    #[test]
    fn repair_restores_only_a_missing_inherited_bond() {
        let mut organism = Simulation::create_initial_organism();
        let catalog = crate::resources::default_catalog();
        let expected_bonds = organism.genome.structural_blueprint.connections.len();
        assert!(expected_bonds > 0);

        organism.structure.bonds.clear();
        assert!(crate::repair::repair_one_element(&mut organism, &catalog));
        assert_eq!(organism.structure.bonds.len(), expected_bonds);
        assert!(organism.structure.units.iter().all(|unit| unit.blueprint_index.is_some()));
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
    fn movement_records_an_actual_physical_outcome() {
        let mut sim = Simulation::new(11, 10.0);
        for _ in 0..50 { sim.step(); }
        let entry = sim.organisms[0]
            .decision_history
            .entries
            .iter()
            .find(|entry| entry.action == crate::decision::ActionKind::Move)
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
