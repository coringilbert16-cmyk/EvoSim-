#[cfg(test)]
mod integration_tests {
    use crate::decision::{ActionKind, OutcomeKind};
    use crate::decision_runtime::ActionCandidate;
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
    fn raw_material_instantiation_consumes_exactly_one_unit_and_preserves_placement() {
        let catalog = crate::resources::default_catalog();
        let mut organism = Simulation::create_initial_organism();
        organism.store_unbonded_material(Material::free_base("Carbon", 3.0));
        let placement = Placement {
            x: 12.5,
            y: -4.0,
            rotation_radians: 0.75,
        };

        let index = crate::combine_runtime::instantiate_one_unit(
            &mut organism,
            "Carbon",
            placement,
            &catalog,
        )
        .expect("sufficient raw Carbon should instantiate");

        assert_eq!(index, 0);
        assert!((organism.stored_unbonded.total_amount() - 2.0).abs() < 1e-12);
        assert_eq!(organism.structure.units.len(), 1);
        assert_eq!(organism.structure.units[index].resource_name, "Carbon");
        assert_eq!(organism.structure.units[index].placement, placement);
    }

    #[test]
    fn raw_material_instantiation_is_atomic_when_material_is_insufficient() {
        let catalog = crate::resources::default_catalog();
        let mut organism = Simulation::create_initial_organism();
        organism.store_unbonded_material(Material::free_base("Carbon", 0.5));
        let before = organism.stored_unbonded.clone();
        let result = crate::combine_runtime::instantiate_one_unit(
            &mut organism,
            "Carbon",
            Placement {
                x: 1.0,
                y: 2.0,
                rotation_radians: 0.0,
            },
            &catalog,
        );

        assert!(result.is_none());
        assert_eq!(organism.stored_unbonded, before);
        assert!(organism.structure.units.is_empty());
    }

    #[test]
    fn raw_material_instantiation_rejects_bonded_storage_without_mutation() {
        let catalog = crate::resources::default_catalog();
        let mut organism = Simulation::create_initial_organism();
        organism.stored_unbonded = Material {
            parts: vec![("Carbon".into(), 2.0)],
            bonded: true,
        };
        let result = crate::combine_runtime::instantiate_one_unit(
            &mut organism,
            "Carbon",
            Placement {
                x: 1.0,
                y: 2.0,
                rotation_radians: 0.0,
            },
            &catalog,
        );

        assert!(result.is_none());
        assert_eq!(organism.stored_unbonded.total_amount(), 2.0);
        assert!(organism.structure.units.is_empty());
    }

    #[test]
    fn raw_material_instantiation_rejects_unknown_resource_without_mutation() {
        let catalog = crate::resources::default_catalog();
        let mut organism = Simulation::create_initial_organism();
        organism.store_unbonded_material(Material::free_base("Carbon", 2.0));
        let before = organism.stored_unbonded.clone();
        let result = crate::combine_runtime::instantiate_one_unit(
            &mut organism,
            "NotAResource",
            Placement {
                x: 1.0,
                y: 2.0,
                rotation_radians: 0.0,
            },
            &catalog,
        );

        assert!(result.is_none());
        assert_eq!(organism.stored_unbonded, before);
        assert!(organism.structure.units.is_empty());
    }

    #[test]
    fn raw_material_instantiation_rejects_nonfinite_placement_without_mutation() {
        let catalog = crate::resources::default_catalog();
        let mut organism = Simulation::create_initial_organism();
        organism.store_unbonded_material(Material::free_base("Carbon", 2.0));
        let before = organism.stored_unbonded.clone();
        let result = crate::combine_runtime::instantiate_one_unit(
            &mut organism,
            "Carbon",
            Placement {
                x: f64::NAN,
                y: 2.0,
                rotation_radians: 0.0,
            },
            &catalog,
        );

        assert!(result.is_none());
        assert_eq!(organism.stored_unbonded, before);
        assert!(organism.structure.units.is_empty());
    }

    #[test]
    fn raw_material_instantiation_rejects_nonfinite_rotation_without_mutation() {
        let catalog = crate::resources::default_catalog();
        let mut organism = Simulation::create_initial_organism();
        organism.store_unbonded_material(Material::free_base("Carbon", 2.0));
        let before = organism.stored_unbonded.clone();
        let result = crate::combine_runtime::instantiate_one_unit(
            &mut organism,
            "Carbon",
            Placement {
                x: 1.0,
                y: 2.0,
                rotation_radians: f64::INFINITY,
            },
            &catalog,
        );

        assert!(result.is_none());
        assert_eq!(organism.stored_unbonded, before);
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

    #[test]
    fn organism_can_break_a_structural_bond_and_records_outcome() {
        let mut sim = Simulation::new(7, 10.0);
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
        let before = sim.organisms[0].usable_energy;
        let decision = ActionCandidate {
            action: ActionKind::Break,
            context_key: Some("bond:0".into()),
        };
        let transformation = Simulation::try_start_transformation(
            &mut sim.organisms[0],
            &mut sim.next_transformation_id,
            &decision,
        )
        .expect("valid bond should start BREAK transformation");
        sim.active_transformations.push(transformation);
        for _ in 0..20 {
            sim.step();
            if sim.organisms[0].structure.bonds.is_empty() {
                break;
            }
        }
        assert!(sim.organisms[0].structure.bonds.is_empty());
        assert!((sim.organisms[0].usable_energy - before - 12.5).abs() < 1e-12);
        assert!(sim
            .organisms[0]
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
