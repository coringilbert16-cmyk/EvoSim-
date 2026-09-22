#[cfg(test)]
mod integration_tests {
    use crate::decision::ActionKind;
    use crate::physical_material::PhysicalMaterial;
    use crate::resources::{InternalBond, Material};
    use crate::state::{DevelopmentStage, Simulation};
    use crate::structure::{Bond, BondEndpoint, ConnectionEndpoint, Placement, StructuralUnit};

    fn structured_carbon_hydrogen() -> Material {
        Material {
            parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
            internal_bonds: vec![InternalBond {
                part_a: 0,
                part_b: 1,
            }],
        }
    }

    fn realized_structured_carbon_hydrogen(
        catalog: &[crate::resources::BaseResource],
    ) -> PhysicalMaterial {
        PhysicalMaterial::realized(
            structured_carbon_hydrogen(),
            vec![
                Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
                Placement {
                    x: 1.0,
                    y: 0.0,
                    rotation_radians: 0.25,
                },
            ],
            catalog,
        )
        .expect("test composite must have a valid physical realization")
    }

    #[test]
    fn fresh_organism_is_a_physically_realized_juvenile() {
        let o = Simulation::create_initial_organism();
        assert!(!o.structure.units.is_empty());
        assert!(crate::cavity::analyze_genome_cavity(
            &o.structure,
            &crate::resources::default_catalog(),
        )
        .unwrap()
        .is_some());
        assert!(!o.structure.units.is_empty());
        assert!(!o.stored_material.is_empty());
        assert!(matches!(o.development_stage, DevelopmentStage::Juvenile));
        assert_eq!(
            o.stored_material.materials_snapshot(),
            vec![o.genome.juvenile_reserve.clone()]
        );
        assert!(o.usable_energy >= o.genome.juvenile_energy_reserve);
        assert!(o.decision_history.entries.is_empty());
    }

    #[test]
    fn initial_organism_has_a_valid_developmental_blueprint_and_physical_mass() {
        let o = Simulation::create_initial_organism();
        assert!(o.genome.developmental_blueprint.validate().is_ok());
        let catalog = crate::resources::default_catalog();
        assert!(o.structural_mass(&catalog) > 0.0);
        let realization = o.genome.developmental_blueprint.realization(
            &o.structure,
            &catalog,
            (o.developmental_origin.x, o.developmental_origin.y),
            o.developmental_orientation_radians,
            o.genome.adult_mass(),
        );
        assert!(realization.overall < 0.90);
        assert!(matches!(o.development_stage, DevelopmentStage::Juvenile));
    }

    #[test]
    fn death_with_no_remaining_bonds_releases_realized_structure() {
        let mut s = Simulation::new(41, 10.0);
        s.environment.vents.clear();
        let before_environment_amount = s.environment.field.total_amount();
        let organism = &mut s.organisms[0];
        organism.structure.bonds.clear();
        organism.stress = (organism.stress_threshold / crate::state::STRESS_DECAY_PER_TICK) * 1.01;
        let initial_units = organism.structure.units.len();
        assert!(initial_units > 0);

        s.step();

        assert!(s.organisms.is_empty());
        assert!(s.decomposing_bodies.is_empty());
        assert!(s.environment.field.total_amount() > before_environment_amount);
    }

    #[test]
    fn adulthood_is_irreversible_after_structural_loss() {
        let mut s = Simulation::new(32, 10.0);
        s.organisms[0].development_stage = DevelopmentStage::Adult;
        s.organisms[0].structure.units.clear();
        s.organisms[0].structure.bonds.clear();
        s.step();
        assert!(matches!(
            s.organisms[0].development_stage,
            DevelopmentStage::Adult
        ));
    }
    #[test]
    fn storage_contains_discrete_independent_material_objects() {
        let mut o = Simulation::create_initial_organism();
        assert!(o.store_material(Material::free_base("Carbon", 5.0)));
        assert_eq!(o.stored_material.len(), 6);
        assert_eq!(o.stored_material.count_unstructured(), 6);
        assert_eq!(o.stored_material.total_amount(), 6.0);
    }
    #[test]
    fn storage_preserves_a_compound_as_one_intact_object() {
        let mut o = Simulation::create_initial_organism();
        let m = structured_carbon_hydrogen();
        assert!(o.store_material(m.clone()));
        assert!(o.stored_material.materials_snapshot().contains(&m));
        assert_eq!(o.stored_material.count_structured(), 1);
    }
    #[test]
    // Containment is automatic; there is no organism-side acquisition action.
    fn contained_physical_material_becomes_storage_without_an_acquire_action() {
        let mut s = Simulation::new(21, 10.0);
        s.environment.vents.clear();
        let organism = s.organisms[0].clone();
        let anchor = organism.structure.units[0].placement;
        let physical = PhysicalMaterial::realized(
            Material::free_base("Carbon", 1.0),
            vec![anchor],
            &s.environment.catalog,
        )
        .expect("carbon should have a valid physical realization");
        let before = s.organisms[0].stored_material.total_amount();
        let field_before = s.environment.field.total_amount();
        s.environment.field.deposit(anchor.x, anchor.y, physical);
        Simulation::transfer_contained_environmental_material(
            &mut s.organisms[0],
            &mut s.environment,
        );
        assert_eq!(s.organisms[0].stored_material.total_amount(), before + 1.0);
        assert!(s.environment.field.total_amount() < field_before);
    }

    #[test]
    fn a_composite_crossing_the_boundary_is_partitioned_at_constituent_scale() {
        let mut s = Simulation::new(23, 10.0);
        s.environment.vents.clear();
        let organism = s.organisms[0].clone();
        let anchor = organism.structure.units[0].placement;
        let material = structured_carbon_hydrogen();
        let physical = PhysicalMaterial::realized(
            material.clone(),
            vec![
                Placement {
                    x: anchor.x,
                    y: anchor.y,
                    rotation_radians: 0.0,
                },
                Placement {
                    x: anchor.x + 1.8,
                    y: anchor.y,
                    rotation_radians: 0.0,
                },
            ],
            &s.environment.catalog,
        )
        .expect("test composite must have a valid physical realization");
        let before = s.organisms[0].stored_material.total_amount();
        s.environment.field.deposit(anchor.x, anchor.y, physical);
        Simulation::transfer_contained_environmental_material(
            &mut s.organisms[0],
            &mut s.environment,
        );
        assert_eq!(s.organisms[0].stored_material.total_amount(), before + 1.0);
        let stored = s.organisms[0].stored_material.materials_snapshot();
        assert!(stored
            .iter()
            .any(|m| m.parts == vec![("Carbon".into(), 1.0)]));
        let remaining: Vec<_> = s
            .environment
            .field
            .cells
            .iter()
            .flat_map(|cell| cell.physical_materials.iter())
            .collect();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].material.parts, vec![("Hydrogen".into(), 1.0)]);
        assert!(remaining[0].material.internal_bonds.is_empty());
    }

    #[test]
    fn structural_material_is_not_opened_by_storage() {
        let mut o = Simulation::create_initial_organism();
        assert!(o.store_material(structured_carbon_hydrogen()));
        assert_eq!(o.stored_material.count_structured(), 1);
    }
    #[test]
    fn fresh_simulation_material_flow_remains_finite_with_direct_vent_sources() {
        let mut s = Simulation::new(1, 10.0);
        for _ in 0..300 {
            s.step();
        }
        assert!(s.total_material_in_system().is_finite());
        assert!(s.total_material_in_system() > 0.0);
    }
    fn add_test_break_bond(s: &mut Simulation) {
        s.organisms[0].structure.bonds.clear();
        let a = s.organisms[0].structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 500.0,
                y: 500.0,
                rotation_radians: 0.0,
            },
        ));
        let b = s.organisms[0].structure.add_unit(StructuralUnit::new(
            "Methane",
            Placement {
                x: 501.0,
                y: 500.0,
                rotation_radians: 0.0,
            },
        ));
        let id_a = s.organisms[0].structure.physical_id(a).unwrap();
        let id_b = s.organisms[0].structure.physical_id(b).unwrap();
        s.organisms[0].structure.push_bond_unchecked(Bond {
            endpoint_a: BondEndpoint::new(id_a, ConnectionEndpoint::Corner { point_index: 0 }),
            endpoint_b: BondEndpoint::new(id_b, ConnectionEndpoint::Corner { point_index: 0 }),
            strength: 0.8,
            bond_energy: 12.5,
        });
    }
    #[test]
    fn maintenance_is_based_on_realized_structural_mass_and_ledger_settlement() {
        let mut organism = Simulation::create_initial_organism();
        let catalog = crate::resources::default_catalog();
        let demand = organism.structural_mass(&catalog) * crate::state::MAINTENANCE_ENERGY_PER_MASS;
        organism.usable_energy = demand + 1.0;
        organism.stress = 0.0;
        let mut ledger = crate::state::EnergyLedger::default();

        organism.apply_maintenance(&catalog, &mut ledger);

        assert!((organism.usable_energy - 1.0).abs() < 1e-12);
        assert!((organism.stress - demand).abs() < 1e-12);
        assert!((ledger.total_heat_dissipated - demand).abs() < 1e-12);
    }

    #[test]
    fn unpaid_maintenance_becomes_stress_without_direct_death() {
        let mut organism = Simulation::create_initial_organism();
        let catalog = crate::resources::default_catalog();
        let demand = organism.structural_mass(&catalog) * crate::state::MAINTENANCE_ENERGY_PER_MASS;
        organism.usable_energy = demand * 0.25;
        organism.stress = 0.0;
        let mut ledger = crate::state::EnergyLedger::default();

        organism.apply_maintenance(&catalog, &mut ledger);

        let expected_stress = demand;
        assert!((organism.stress - expected_stress).abs() < 1e-12);
        assert_eq!(organism.usable_energy, 0.0);
        assert!(organism.stress < organism.stress_threshold);
    }

    #[test]
    fn zero_maintenance_energy_creates_stress_without_changing_energy() {
        let mut organism = Simulation::create_initial_organism();
        let catalog = crate::resources::default_catalog();
        let demand = organism.structural_mass(&catalog) * crate::state::MAINTENANCE_ENERGY_PER_MASS;
        organism.usable_energy = 0.0;
        organism.stress = 0.0;
        let mut ledger = crate::state::EnergyLedger::default();

        organism.apply_maintenance(&catalog, &mut ledger);

        assert_eq!(organism.usable_energy, 0.0);
        assert!((organism.stress - demand).abs() < 1e-12);
        assert_eq!(ledger.total_heat_dissipated, 0.0);
    }

    #[test]
    fn maintenance_heat_dissipates_without_repairing_structure() {
        let mut organism = Simulation::create_initial_organism();
        let catalog = crate::resources::default_catalog();
        let initial_units = organism.structure.units.clone();
        let initial_bonds = organism.structure.bonds.clone();
        let demand = organism.structural_mass(&catalog) * crate::state::MAINTENANCE_ENERGY_PER_MASS;
        organism.usable_energy = demand + 1.0;
        let mut ledger = crate::state::EnergyLedger::default();

        organism.apply_maintenance(&catalog, &mut ledger);
        let after_maintenance_stress = organism.stress;
        organism.stress *= crate::state::STRESS_DECAY_PER_TICK;

        assert!(after_maintenance_stress > 0.0);
        assert!(organism.stress < after_maintenance_stress);
        assert_eq!(organism.structure.units, initial_units);
        assert_eq!(organism.structure.bonds, initial_bonds);
    }

    #[test]
    fn break_action_starts_a_transformation_before_resolution() {
        let mut s = Simulation::new(7, 10.0);
        add_test_break_bond(&mut s);
        s.organisms[0].usable_energy = 0.0;
        s.step();
        assert_eq!(s.organisms[0].structure.bonds.len(), 1);
        assert!(s.organisms[0].active_transformation_id.is_some());
    }
    #[test]
    fn break_resolution_changes_state_on_expected_tick() {
        let mut s = Simulation::new(7, 10.0);
        add_test_break_bond(&mut s);
        s.organisms[0].usable_energy = 0.0;
        s.step();
        s.step();
        s.step();
        let a_index = s.organisms[0].structure.units.len() - 2;
        let b_index = s.organisms[0].structure.units.len() - 1;
        let a = s.organisms[0].structure.units[a_index]
            .properties(&s.environment.catalog)
            .unwrap();
        let b = s.organisms[0].structure.units[b_index]
            .properties(&s.environment.catalog)
            .unwrap();
        let candidate = crate::contact::connection_pair_candidates(
            &s.organisms[0].structure,
            a_index,
            b_index,
            &s.environment.catalog,
        )
        .into_iter()
        .find(|c| {
            c.endpoint_a == ConnectionEndpoint::Corner { point_index: 0 }
                && c.endpoint_b == ConnectionEndpoint::Corner { point_index: 0 }
        })
        .expect("test bond endpoints must remain resolvable");
        let water = s.organisms[0]
            .occupied_cells
            .first()
            .and_then(|p| s.environment.field.index_for_position(p.x, p.y))
            .map(|i| {
                s.environment.field.cells[i]
                    .materials
                    .iter()
                    .flat_map(|m| m.parts.iter())
                    .filter(|(name, _)| name == "Water")
                    .map(|(_, amount)| *amount)
                    .sum::<f64>()
            })
            .unwrap_or(0.0);
        let formation_interaction =
            crate::combine::experimental_interaction(a, b, candidate, water);
        let break_interaction = -formation_interaction.signed_value;
        let work = crate::transformation::break_work_cost(a, b, crate::math::complexity(2.0));
        let maintenance = s.organisms[0].structural_mass(&s.environment.catalog)
            * crate::state::MAINTENANCE_ENERGY_PER_MASS;
        let expected = 12.5 + break_interaction - work - maintenance;
        assert!(s.organisms[0].structure.bonds.is_empty());
        assert!((s.organisms[0].usable_energy - expected).abs() < 1e-12);
        assert!(s.organisms[0]
            .decision_history
            .has_knowledge(ActionKind::Break, Some("bond:0")));
    }
}
