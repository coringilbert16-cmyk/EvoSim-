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
        assert_eq!(o.stored_material.count_unstructured(), 5);
        assert_eq!(o.stored_material.count_structured(), 1);
        assert_eq!(o.stored_material.total_amount(), 8.0);
    }
    #[test]
    fn storage_preserves_a_compound_as_one_intact_object() {
        let mut o = Simulation::create_initial_organism();
        let m = structured_carbon_hydrogen();
        assert!(o.store_material(m.clone()));
        assert!(o.stored_material.materials_snapshot().contains(&m));
        assert_eq!(o.stored_material.count_structured(), 2);
    }
    #[test]
    // Containment is automatic; there is no organism-side acquisition action.
    fn contained_physical_material_becomes_storage_without_an_acquire_action() {
        let mut s = Simulation::new(21, 10.0);
        for cell in &mut s.environment.field.cells {
            cell.physical_materials.clear();
        }
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
        assert!((s.environment.field.total_amount() - field_before).abs() < 1e-9);
    }

    #[test]
    fn a_composite_crossing_the_boundary_is_partitioned_at_constituent_scale() {
        let mut s = Simulation::new(23, 10.0);
        for cell in &mut s.environment.field.cells {
            cell.materials.clear();
            cell.physical_materials.clear();
        }
        let organism = s.organisms[0].clone();
        let body = crate::organism_geometry::OrganismBodyGeometry::from_structure(
            &organism.structure,
            &s.environment.catalog,
        )
        .expect("initial organism must have a realized body");
        let mut realization = None;
        'search: for unit in &organism.structure.units {
            let anchor = unit.placement;
            if !body.contains_point(anchor.x, anchor.y) {
                continue;
            }
            for distance in [0.5, 0.8, 1.0, 1.2, 1.5, 1.8, 2.0, 2.2, 2.5, 3.0] {
                let outside = Placement {
                    x: anchor.x + distance,
                    y: anchor.y,
                    rotation_radians: 0.0,
                };
                if body.contains_point(outside.x, outside.y) {
                    continue;
                }
                let physical = PhysicalMaterial::realized(
                    structured_carbon_hydrogen(),
                    vec![anchor, outside],
                    &s.environment.catalog,
                );
                if let Some(physical) = physical {
                    realization = Some((anchor, physical));
                    break 'search;
                }
            }
        }
        let (anchor, physical) =
            realization.expect("test composite must straddle the realized body");
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
        assert_eq!(o.stored_material.count_structured(), 2);
    }

    fn add_test_break_bond(s: &mut Simulation) {
        let origin = s.organisms[0].occupied_cells[0];
        let physical = PhysicalMaterial::realized(
            structured_carbon_hydrogen(),
            vec![
                Placement {
                    x: origin.x + 4.0,
                    y: origin.y,
                    rotation_radians: 0.0,
                },
                Placement {
                    x: origin.x + 5.0,
                    y: origin.y,
                    rotation_radians: 0.0,
                },
            ],
            &s.environment.catalog,
        )
        .expect("stored bonded material should be physically realizable");
        assert!(s.organisms[0]
            .stored_material
            .store_physical_instance(physical));
    }

    fn start_test_break_transformation(s: &mut Simulation) {
        let candidate = crate::decision_runtime::ActionCandidate {
            action: ActionKind::Break,
            context_key: Some("stored:1:bond:0".into()),
        };
        let transformation = Simulation::try_start_transformation(
            &mut s.organisms[0],
            &s.environment.catalog,
            &mut s.next_transformation_id,
            &candidate,
        )
        .expect("test break candidate must start a transformation");
        s.active_transformations.push(transformation);
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
    fn unpaid_maintenance_accumulates_debt_without_direct_death() {
        let mut organism = Simulation::create_initial_organism();
        let catalog = crate::resources::default_catalog();
        let demand = organism.structural_mass(&catalog) * crate::state::MAINTENANCE_ENERGY_PER_MASS;
        organism.usable_energy = demand * 0.25;
        organism.stress = 0.0;
        let mut ledger = crate::state::EnergyLedger::default();

        organism.apply_maintenance(&catalog, &mut ledger);

        let deficit = demand * 0.75;
        assert!((organism.maintenance_debt - deficit).abs() < 1e-12);
        let paid = demand * 0.25;
        assert!((organism.stress - (paid + deficit + organism.maintenance_debt)).abs() < 1e-12);
        assert_eq!(organism.usable_energy, 0.0);
        assert!(organism.stress < organism.stress_threshold);
    }

    #[test]
    fn zero_maintenance_energy_accumulates_debt_and_increasing_stress_pressure() {
        let mut organism = Simulation::create_initial_organism();
        let catalog = crate::resources::default_catalog();
        let demand = organism.structural_mass(&catalog) * crate::state::MAINTENANCE_ENERGY_PER_MASS;
        organism.usable_energy = 0.0;
        organism.stress = 0.0;
        let mut ledger = crate::state::EnergyLedger::default();

        organism.apply_maintenance(&catalog, &mut ledger);
        let first_debt = organism.maintenance_debt;
        let first_stress = organism.stress;

        organism.stress *= crate::state::STRESS_DECAY_PER_TICK;
        organism.apply_maintenance(&catalog, &mut ledger);

        assert_eq!(organism.usable_energy, 0.0);
        assert!((first_debt - demand).abs() < 1e-12);
        assert!((organism.maintenance_debt - 2.0 * demand).abs() < 1e-12);
        assert!(organism.stress > first_stress * crate::state::STRESS_DECAY_PER_TICK);
        assert!(organism.stress > first_stress);
        assert_eq!(ledger.total_heat_dissipated, 0.0);
    }

    #[test]
    fn maintenance_recovery_stops_debt_growth_without_repaying_history() {
        let mut organism = Simulation::create_initial_organism();
        let catalog = crate::resources::default_catalog();
        let demand = organism.structural_mass(&catalog) * crate::state::MAINTENANCE_ENERGY_PER_MASS;
        organism.usable_energy = 0.0;
        let mut ledger = crate::state::EnergyLedger::default();

        organism.apply_maintenance(&catalog, &mut ledger);
        let debt = organism.maintenance_debt;
        let deficit = demand;
        organism.stress *= crate::state::STRESS_DECAY_PER_TICK;
        organism.usable_energy = demand + 1.0;
        organism.apply_maintenance(&catalog, &mut ledger);

        assert!((organism.maintenance_debt - debt).abs() < 1e-12);
        assert!((organism.usable_energy - 1.0).abs() < 1e-12);
        let expected_stress = (deficit + debt) * crate::state::STRESS_DECAY_PER_TICK + demand;
        assert!((organism.stress - expected_stress).abs() < 1e-12);
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
        s.organisms[0].usable_energy = 100.0;
        start_test_break_transformation(&mut s);
        assert!(s.organisms[0].active_transformation_id.is_some());
        assert_eq!(s.organisms[0].stored_material.physical_count(), 0);
    }
    #[test]
    fn break_resolution_changes_state_on_expected_tick() {
        let mut s = Simulation::new(7, 10.0);
        add_test_break_bond(&mut s);
        s.organisms[0].usable_energy = 100.0;
        start_test_break_transformation(&mut s);
        s.step();
        assert!(s.organisms[0].active_transformation_id.is_some());
        assert_eq!(s.organisms[0].stored_material.physical_count(), 0);
        s.step();
        assert!(s.organisms[0].active_transformation_id.is_none());
        assert_eq!(s.organisms[0].stored_material.physical_count(), 2);
        assert!(s.organisms[0]
            .decision_history
            .has_knowledge(ActionKind::Break, Some("stored:1:bond:0")));
    }
}
