#[cfg(test)]
mod integration_tests {
    use crate::decision::ActionKind;
    use crate::physical_material::PhysicalMaterial;
    use crate::resources::{InternalBond, Material};
    use crate::state::Simulation;
    use crate::structure::Placement;

    /// Independent material oracle. It walks authoritative holders directly,
    /// rejects fractional/unknown constituents, and never reads ledger totals.
    fn integer_material_inventory(
        simulation: &Simulation,
    ) -> Result<std::collections::BTreeMap<String, u64>, String> {
        fn add_material(
            inventory: &mut std::collections::BTreeMap<String, u64>,
            material: &Material,
            catalog: &[crate::resources::BaseResource],
        ) -> Result<(), String> {
            for (name, amount) in &material.parts {
                if !catalog.iter().any(|resource| resource.name == *name) {
                    return Err(format!("unknown resource identity: {name}"));
                }
                if !amount.is_finite()
                    || *amount <= 0.0
                    || amount.fract().abs() > 1e-9
                    || *amount > u64::MAX as f64
                {
                    return Err(format!(
                        "non-integer or invalid amount for {name}: {amount}"
                    ));
                }
                let count = amount.round() as u64;
                let total = inventory.entry(name.clone()).or_default();
                *total = total
                    .checked_add(count)
                    .ok_or_else(|| format!("constituent count overflow for {name}"))?;
            }
            Ok(())
        }

        let catalog = &simulation.environment.catalog;
        let mut inventory = std::collections::BTreeMap::new();

        // Environmental aggregate stock and already-realized objects.
        for cell in &simulation.environment.field.cells {
            for material in &cell.materials {
                add_material(&mut inventory, material, catalog)?;
            }
            for physical in &cell.physical_materials {
                add_material(&mut inventory, &physical.material, catalog)?;
            }
        }

        // Living or breached organisms retain their physical structure and storage.
        for organism in &simulation.organisms {
            for unit in &organism.structure.units {
                add_material(&mut inventory, &unit.material, catalog)?;
            }
            for material in organism.stored_material.iter_materials() {
                add_material(&mut inventory, material, catalog)?;
            }
            if let Some(construction) = &organism.reproductive_construction {
                for material in construction.committed_material.iter_materials() {
                    add_material(&mut inventory, material, catalog)?;
                }
                for unit in &construction.developing_structure.units {
                    add_material(&mut inventory, &unit.material, catalog)?;
                }
            }
        }

        // Active transformations own temporarily removed physical sources.
        // The material field and prepared structures/storage/ledgers are
        // descriptive or trial copies and must not be counted a second time.
        for transformation in &simulation.active_transformations {
            if let Some(physical) = &transformation.stored_material {
                add_material(&mut inventory, &physical.material, catalog)?;
            }
            if let Some((_, physical)) = &transformation.combine_environmental_source {
                add_material(&mut inventory, &physical.material, catalog)?;
            }
        }

        // Legacy decomposition carriers remain material owners if present.
        for body in &simulation.decomposing_bodies {
            for unit in &body.structure.units {
                add_material(&mut inventory, &unit.material, catalog)?;
            }
        }

        Ok(inventory)
    }

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
    fn integer_material_inventory_conserves_constituents_across_field_to_storage_transfer() {
        let mut simulation = Simulation::new(7103, 10.0);

        // Isolate the owners relevant to this transfer while retaining the
        // organism's ordinary physical structure as part of the inventory.
        for cell in &mut simulation.environment.field.cells {
            cell.materials.clear();
            cell.physical_materials.clear();
        }
        simulation.organisms[0].stored_material.entries.clear();
        simulation.environment.field.cells[0]
            .materials
            .push(Material::free_base("Carbon", 3.0));

        let before =
            integer_material_inventory(&simulation).expect("fixture inventory is integral");
        let taken = crate::material_transfer::take_whole_unstructured(
            &mut simulation.environment.field.cells[0].materials[0],
            1,
        )
        .expect("one whole carbon constituent should transfer");
        let placement = Placement {
            x: 10.0,
            y: 10.0,
            rotation_radians: 0.0,
        };
        let physical =
            PhysicalMaterial::realized(taken, vec![placement], &simulation.environment.catalog)
                .expect("whole carbon should be physically realizable");
        assert!(simulation.organisms[0]
            .stored_material
            .store_physical_instance(physical));

        let after = integer_material_inventory(&simulation).expect("result inventory is integral");
        assert_eq!(
            after, before,
            "material ownership transfer changed constituent totals"
        );
    }

    #[test]
    fn integer_material_inventory_conserves_constituents_across_storage_restore() {
        let mut simulation = Simulation::new(7106, 10.0);
        for cell in &mut simulation.environment.field.cells {
            cell.materials.clear();
            cell.physical_materials.clear();
        }
        simulation.organisms[0].stored_material.entries.clear();

        let catalog = simulation.environment.catalog.clone();
        let physical = PhysicalMaterial::realized(
            Material::free_base("Hydrogen", 1.0),
            vec![Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        )
        .expect("Hydrogen strip should be physically realizable");
        assert!(simulation.organisms[0]
            .stored_material
            .store_physical_instance(physical));
        let before = integer_material_inventory(&simulation).expect("stored inventory is integral");

        let stored = simulation.organisms[0]
            .stored_material
            .take_matching_physical(&Material::free_base("Hydrogen", 1.0))
            .expect("stored Hydrogen should be removable for restoration");
        let restored = crate::material_restoration::restore_material(
            &mut simulation.organisms[0].structure,
            &stored,
            Placement {
                x: 100.0,
                y: 100.0,
                rotation_radians: 0.0,
            },
            &catalog,
        )
        .expect("intact Hydrogen should restore into the physical structure");
        assert_eq!(restored.len(), 1);

        let after =
            integer_material_inventory(&simulation).expect("restored inventory is integral");
        assert_eq!(
            after, before,
            "restoring stored physical material must preserve constituent totals"
        );
    }

    #[test]
    fn integer_material_inventory_preserves_state_when_restore_rejects_invalid_connections() {
        let mut simulation = Simulation::new(7107, 10.0);
        for cell in &mut simulation.environment.field.cells {
            cell.materials.clear();
            cell.physical_materials.clear();
        }
        simulation.organisms[0].stored_material.entries.clear();

        let catalog = simulation.environment.catalog.clone();
        let physical = PhysicalMaterial::realized(
            structured_carbon_hydrogen(),
            vec![
                Placement {
                    x: 20.0,
                    y: 20.0,
                    rotation_radians: 0.0,
                },
                Placement {
                    x: 20.838,
                    y: 20.0,
                    rotation_radians: 0.0,
                },
            ],
            &catalog,
        )
        .expect("compound should be physically realizable");
        assert!(simulation.organisms[0]
            .stored_material
            .store_physical_instance(physical));

        let inventory_before =
            integer_material_inventory(&simulation).expect("pre-restore inventory is integral");
        let structure_before = simulation.organisms[0].structure.clone();
        let storage_before = simulation.organisms[0].stored_material.clone();
        let mut malformed = simulation.organisms[0]
            .stored_material
            .peek_matching_physical(&structured_carbon_hydrogen())
            .expect("stored compound should be available for validation");
        malformed.internal_connections = Some(Vec::new());

        assert!(crate::material_restoration::restore_material(
            &mut simulation.organisms[0].structure,
            &malformed,
            Placement {
                x: 100.0,
                y: 100.0,
                rotation_radians: 0.0,
            },
            &catalog,
        )
        .is_none());

        assert_eq!(simulation.organisms[0].structure, structure_before);
        assert_eq!(simulation.organisms[0].stored_material, storage_before);
        assert_eq!(
            integer_material_inventory(&simulation).expect("post-rejection inventory is integral"),
            inventory_before,
            "rejected physical restoration must not mutate material ownership"
        );
    }

    #[test]
    fn integer_material_inventory_conserves_constituents_when_anchor_moves_to_offspring() {
        let mut simulation = Simulation::new(23, 20.0);
        assert!(simulation.organisms[0].store_material(Material::free_base("Carbon", 1.0)));
        let catalog = simulation.environment.catalog.clone();
        let before =
            integer_material_inventory(&simulation).expect("initial inventory is integral");
        let mut ledger = crate::state::EnergyLedger::default();

        let (parent, rng) = (&mut simulation.organisms[0], &mut simulation.rng);
        assert!(crate::reproduction::begin_reproduction(
            parent,
            rng,
            &catalog,
            &mut ledger,
        ));

        let after =
            integer_material_inventory(&simulation).expect("post-transfer inventory is integral");
        assert_eq!(
            after, before,
            "moving the physical anchor into developing offspring must preserve every constituent"
        );
    }

    #[test]
    fn integer_material_inventory_rejects_unknown_constituents_in_authoritative_holders() {
        let mut simulation = Simulation::new(7104, 10.0);
        for cell in &mut simulation.environment.field.cells {
            cell.materials.clear();
            cell.physical_materials.clear();
        }
        simulation.environment.field.cells[0]
            .materials
            .push(Material::free_base("Unobtainium", 1.0));

        let error = integer_material_inventory(&simulation)
            .expect_err("unknown material identity must fail the independent inventory");
        assert!(error.contains("unknown resource identity: Unobtainium"));
    }

    #[test]
    fn integer_material_inventory_rejects_fractional_constituents_in_authoritative_holders() {
        let mut simulation = Simulation::new(7105, 10.0);
        for cell in &mut simulation.environment.field.cells {
            cell.materials.clear();
            cell.physical_materials.clear();
        }
        simulation.environment.field.cells[0]
            .materials
            .push(Material::free_base("Carbon", 1.5));

        let error = integer_material_inventory(&simulation)
            .expect_err("fractional base constituents must fail the independent inventory");
        assert!(error.contains("non-integer or invalid amount for Carbon: 1.5"));
    }

    #[test]
    fn fresh_organism_is_a_physically_realized_organism() {
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
        let stored = o.stored_material.materials_snapshot();
        assert_eq!(stored.len(), 4);
        assert!(stored
            .iter()
            .any(|material| { material.parts.iter().any(|(name, _)| name == "Water") }));
        assert_eq!(
            stored
                .iter()
                .filter(|material| { material.parts.iter().all(|(name, _)| name != "Water") })
                .count(),
            3
        );
        assert!(o.usable_energy >= o.genome.reproductive_energy_allocation);
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
            o.genome.preferred_mass(),
        );
        assert!(realization.overall < 0.90);
    }

    #[test]
    fn breached_genome_core_does_not_remove_or_recycle_physical_organism() {
        let mut s = Simulation::new(41, 10.0);
        let before_environment_amount = s.environment.field.total_amount();
        let organism = &mut s.organisms[0];
        organism.structure.bonds.clear();
        organism.stress = (organism.stress_threshold / crate::state::STRESS_DECAY_PER_TICK) * 1.01;
        let initial_units = organism.structure.units.len();
        assert!(initial_units > 0);

        let breached = Simulation::apply_survival_damage(
            &mut s.organisms[0],
            &s.environment,
            &mut s.energy_ledger,
            &mut s.rng,
        );
        assert!(breached);
        assert_eq!(s.living_population(), 0);
        assert_eq!(s.organisms.len(), 1);
        assert_eq!(s.organisms[0].structure.units.len(), initial_units);
        assert!(s.decomposing_bodies.is_empty());
        assert_eq!(
            s.environment.field.total_amount(),
            before_environment_amount
        );
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
    fn accessible_interior_starts_without_synthetic_water() {
        let s = Simulation::new(22, 10.0);
        let catalog = s.environment.catalog.clone();
        let blueprint = crate::juvenile::confirmed_seed_baseline(&catalog)
            .expect("confirmed seed baseline should be valid");
        let (structure, _, _) = crate::juvenile::realize_initial(&blueprint, &catalog)
            .expect("initial realization should produce the seed boundary");
        let regions =
            crate::interior_geometry::find_accessible_interior_regions(&structure, &catalog)
                .expect("initial seed should expose accessible topology");
        assert!(!regions.is_empty());
        // Boundary Water is ordinary realized structure; no Water is synthesized
        // into the accessible interior.
        assert!(structure.units.iter().all(|unit| {
            let Some((name, _)) = unit.material.parts.first() else {
                return true;
            };
            if name != "Water" {
                return true;
            }
            matches!(
                unit.shape(&catalog).map(|shape| &shape.form),
                Some(crate::resources::Form::Fluid {
                    boundary: Some(_),
                    ..
                })
            )
        }));
    }

    #[test]
    // Containment is automatic; there is no organism-side acquisition action.
    fn contained_physical_material_becomes_storage_without_an_acquire_action() {
        let mut s = Simulation::new(21, 10.0);
        for cell in &mut s.environment.field.cells {
            cell.materials.clear();
            cell.physical_materials.clear();
        }
        let organism = s.organisms[0].clone();
        let regions = crate::interior_geometry::find_accessible_interior_regions(
            &organism.structure,
            &s.environment.catalog,
        )
        .expect("initial organism should expose an accessible interior region");
        let region = regions
            .iter()
            .max_by(|a, b| {
                a.area
                    .partial_cmp(&b.area)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .expect("initial organism should have an accessible interior region");
        // The topological sample point only proves that the point is inside;
        // it is not guaranteed to contain an entire rigid resource. Find a
        // placement where the actual Hydrogen geometry fits, then exercise the
        // production containment path. This keeps the test independent of
        // arbitrary chamber coordinates while still requiring real geometry.
        let (min_x, max_x, min_y, max_y) = region.boundary.iter().fold(
            (
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
            ),
            |(min_x, max_x, min_y, max_y), &(x, y)| {
                (min_x.min(x), max_x.max(x), min_y.min(y), max_y.max(y))
            },
        );
        let mut anchor = None;
        'candidate: for ix in 0..=80 {
            let x = min_x + (max_x - min_x) * ix as f64 / 80.0;
            for iy in 0..=80 {
                let y = min_y + (max_y - min_y) * iy as f64 / 80.0;
                let candidate = Placement {
                    x,
                    y,
                    rotation_radians: 0.0,
                };
                let physical = PhysicalMaterial::realized(
                    Material::free_base("Hydrogen", 1.0),
                    vec![candidate],
                    &s.environment.catalog,
                )
                .expect("base resource should have a valid physical realization");
                if crate::environment::ActiveMaterialField::physical_is_fully_inside_any_region(
                    &physical,
                    std::slice::from_ref(region),
                    &s.environment.catalog,
                ) {
                    anchor = Some(candidate);
                    break 'candidate;
                }
            }
        }
        let anchor = anchor.expect("seed chamber must contain a full Hydrogen constituent");
        let physical = PhysicalMaterial::realized(
            Material::free_base("Hydrogen", 1.0),
            vec![anchor],
            &s.environment.catalog,
        )
        .expect("base resource should have a valid physical realization");
        let before = s.organisms[0].stored_material.total_amount();
        s.environment.field.deposit(anchor.x, anchor.y, physical);
        let inventory_before =
            integer_material_inventory(&s).expect("pre-transfer inventory is integral");
        Simulation::transfer_contained_environmental_material(
            &mut s.organisms[0],
            &mut s.environment,
        );
        assert_eq!(s.organisms[0].stored_material.total_amount(), before + 1.0);
        let inventory_after =
            integer_material_inventory(&s).expect("post-transfer inventory is integral");
        assert_eq!(
            inventory_after, inventory_before,
            "production containment transfer must preserve constituent totals"
        );
    }

    #[test]
    fn a_composite_crossing_the_boundary_stays_intact() {
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
        let original = physical.clone();
        let before = s.organisms[0].stored_material.total_amount();
        s.environment.field.deposit(anchor.x, anchor.y, physical);
        Simulation::transfer_contained_environmental_material(
            &mut s.organisms[0],
            &mut s.environment,
        );
        assert_eq!(s.organisms[0].stored_material.total_amount(), before);
        let remaining: Vec<_> = s
            .environment
            .field
            .cells
            .iter()
            .flat_map(|cell| cell.physical_materials.iter())
            .collect();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].material, original.material);
        assert_eq!(
            remaining[0].internal_connections,
            original.internal_connections
        );
        assert_eq!(remaining[0].placements, original.placements);
    }

    #[test]
    fn structural_material_is_not_opened_by_storage() {
        let mut o = Simulation::create_initial_organism();
        assert!(o.store_material(structured_carbon_hydrogen()));
        assert_eq!(o.stored_material.count_structured(), 2);
    }

    fn add_test_break_bond(s: &mut Simulation) {
        s.organisms[0].stored_material.entries.clear();
        let origin = s.organisms[0].occupied_cells[0].clone();
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
            context_key: Some("stored:0:bond:0".into()),
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
        let inventory_before =
            integer_material_inventory(&s).expect("pre-transformation inventory is integral");
        start_test_break_transformation(&mut s);
        assert!(s.organisms[0].active_transformation_id.is_some());
        assert_eq!(s.organisms[0].stored_material.physical_count(), 0);
        let inventory_after =
            integer_material_inventory(&s).expect("active transformation inventory is integral");
        assert_eq!(
            inventory_after, inventory_before,
            "moving stored physical material into an active BREAK transformation must preserve constituents"
        );
    }
    #[test]
    fn energy_inventory_snapshot_recomputes_current_holders_independently_of_ledger_totals() {
        let mut simulation = Simulation::new(7102, 10.0);

        let expected_usable = simulation
            .organisms
            .iter()
            .map(|organism| {
                organism.usable_energy
                    + organism
                        .reproductive_construction
                        .as_ref()
                        .map_or(0.0, |construction| construction.developing_energy)
            })
            .sum::<f64>()
            + simulation
                .decomposing_bodies
                .iter()
                .map(|body| body.energy_budget)
                .sum::<f64>();
        let expected_resonant = simulation
            .organisms
            .iter()
            .map(|organism| organism.resonant_energy.total_energy().unwrap_or(f64::NAN))
            .sum::<f64>();
        let expected_structural = simulation
            .organisms
            .iter()
            .map(|organism| {
                organism
                    .structure
                    .bonds
                    .iter()
                    .map(|bond| bond.bond_energy)
                    .sum::<f64>()
                    + organism
                        .reproductive_construction
                        .as_ref()
                        .map_or(0.0, |construction| {
                            construction
                                .developing_structure
                                .bonds
                                .iter()
                                .map(|bond| bond.bond_energy)
                                .sum::<f64>()
                        })
            })
            .sum::<f64>()
            + simulation
                .decomposing_bodies
                .iter()
                .map(|body| {
                    body.structure
                        .bonds
                        .iter()
                        .map(|bond| bond.bond_energy)
                        .sum::<f64>()
                })
                .sum::<f64>();

        // Poison only cached current totals. The independent inventory must
        // still describe the live holders, not reproduce these ledger values.
        simulation.energy_ledger.total_usable_energy_held = -101.0;
        simulation.energy_ledger.total_resonant_energy_held = -102.0;
        simulation.energy_ledger.total_structural_energy_held = -103.0;

        let inventory = simulation.energy_inventory_snapshot();
        assert_eq!(inventory.current_usable_energy, expected_usable);
        assert_eq!(inventory.current_resonant_energy, expected_resonant);
        assert_eq!(inventory.current_structural_energy, expected_structural);
        assert_ne!(inventory.current_usable_energy, -101.0);
        assert_ne!(inventory.current_resonant_energy, -102.0);
        assert_ne!(inventory.current_structural_energy, -103.0);
    }

    #[test]
    fn stored_material_break_does_not_credit_unallocated_energy() {
        let mut s = Simulation::new(7, 10.0);
        for cell in &mut s.environment.field.cells {
            cell.physical_materials.clear();
        }
        add_test_break_bond(&mut s);
        s.organisms[0].usable_energy = 100.0;
        // This test sets a controlled holder balance after construction.
        s.energy_ledger.initial_usable_energy_held = 100.0;
        s.energy_ledger.total_usable_energy_held = 100.0;
        // This fixture replaces the constructed organism's initial condition;
        // reset its source counter so the synthetic 100-unit baseline is not
        // also charged for the constructor's discarded initial reserve.
        s.energy_ledger.total_external_energy_input = 0.0;
        let candidate = crate::decision_runtime::ActionCandidate {
            action: ActionKind::Break,
            context_key: Some("stored:0:bond:0".into()),
        };
        let mut transformation = Simulation::try_start_transformation(
            &mut s.organisms[0],
            &s.environment.catalog,
            &mut s.next_transformation_id,
            &candidate,
        )
        .expect("stored-material BREAK should start");
        let before_gained = s.energy_ledger.total_usable_energy_gained;
        let before_structural = s.energy_ledger.total_structural_energy_delta;
        let before_work = s.energy_ledger.total_work_expended;
        let before_heat = s.energy_ledger.total_heat_dissipated;
        let before_released = s.energy_ledger.total_potential_energy_released;

        assert!(Simulation::prepare_transformation(
            &mut transformation,
            &mut s.organisms[0],
            &mut s.environment,
            &mut s.energy_ledger,
            None,
        ));
        assert_eq!(s.organisms[0].usable_energy, 100.0);
        assert_eq!(s.energy_ledger.total_usable_energy_gained, before_gained);
        assert_eq!(
            s.energy_ledger.total_structural_energy_delta,
            before_structural
        );
        assert_eq!(s.energy_ledger.total_work_expended, before_work);
        assert_eq!(s.energy_ledger.total_heat_dissipated, before_heat);
        assert_eq!(
            s.energy_ledger.total_potential_energy_released,
            before_released
        );

        crate::transformation::resolve_transformation(
            &transformation,
            &mut s.organisms[0],
            &mut s.environment,
            &mut s.energy_ledger,
        );
        assert_eq!(s.organisms[0].stored_material.physical_count(), 2);
        assert_eq!(s.organisms[0].usable_energy, 100.0);
        s.energy_ledger.total_usable_energy_held =
            crate::reproduction::total_usable_energy_held(&s.organisms, &s.decomposing_bodies);
        s.energy_ledger.total_structural_energy_held =
            Simulation::structural_energy_held(&s.organisms, &s.decomposing_bodies);
        let inventory = s.energy_inventory_snapshot();
        assert!(
            inventory.is_balanced(1e-9),
            "stored-material BREAK must not create energy: {:?}",
            inventory.residual()
        );
    }

    #[test]
    fn break_resolution_changes_state_on_expected_tick() {
        let mut s = Simulation::new(7, 10.0);
        for cell in &mut s.environment.field.cells {
            cell.physical_materials.clear();
        }
        add_test_break_bond(&mut s);
        s.organisms[0].usable_energy = 100.0;
        start_test_break_transformation(&mut s);
        s.step();
        assert!(s.organisms[0].active_transformation_id.is_some());
        assert_eq!(s.organisms[0].stored_material.physical_count(), 0);
        s.step();
        assert!(s.organisms[0].active_transformation_id.is_none());
        assert_eq!(s.organisms[0].stored_material.physical_count(), 2);
        assert!(s.organisms[0].usable_energy <= 100.0);
        assert_eq!(s.energy_ledger.total_structural_energy_delta, 0.0);
        assert!(s.organisms[0]
            .decision_history
            .has_knowledge(ActionKind::Break, Some("stored:0:bond:0")));
    }
}
