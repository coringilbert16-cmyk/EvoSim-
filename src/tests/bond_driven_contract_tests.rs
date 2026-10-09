#[cfg(test)]
mod tests {
    use crate::resources::{default_catalog, Material};
    use crate::state::EnergyLedger;
    use crate::structural_blueprint::{
        BlueprintConnection, BlueprintElement, BlueprintPlacement, StructuralBlueprint,
    };

    fn two_carbon_bond_blueprint() -> StructuralBlueprint {
        StructuralBlueprint::new(
            vec![
                BlueprintElement {
                    material: Material::free_base("Carbon", 1.0),
                    placement: BlueprintPlacement {
                        x: 0.0,
                        y: 0.0,
                        rotation_radians: 0.0,
                    },
                },
                BlueprintElement {
                    material: Material::free_base("Carbon", 1.0),
                    placement: BlueprintPlacement {
                        x: 0.0,
                        y: 0.0,
                        rotation_radians: 0.0,
                    },
                },
            ],
            vec![BlueprintConnection {
                element_a: 0,
                element_b: 1,
            }],
        )
    }

    #[test]
    fn bond_driven_constructor_realizes_the_declared_bond_graph() {
        let catalog = default_catalog();
        let blueprint = two_carbon_bond_blueprint();
        let mut ledger = EnergyLedger::default();
        let mut energy = 1.0e12;

        let (structure, _, _) = blueprint
            .realize_with_context(&catalog, &mut ledger, &mut energy)
            .expect("bond-driven constructor should realize a valid two-carbon bond");

        assert_eq!(structure.units.len(), 2);
        assert_eq!(structure.bonds.len(), 1);

        let bond = &structure.bonds[0];
        let a = structure
            .units
            .iter()
            .find(|unit| unit.physical_id == bond.endpoint_a.constituent_id)
            .expect("bond endpoint A must identify a realized constituent");
        let b = structure
            .units
            .iter()
            .find(|unit| unit.physical_id == bond.endpoint_b.constituent_id)
            .expect("bond endpoint B must identify a realized constituent");

        let pa = bond.endpoint_a.location.world_point(a, &catalog).unwrap();
        let pb = bond.endpoint_b.location.world_point(b, &catalog).unwrap();

        assert!(
            (pa.x - pb.x).hypot(pa.y - pb.y) < 1.0e-8,
            "realized bond endpoints must coincide at the selected joint"
        );
    }

    #[test]
    fn bond_driven_constructor_treats_declared_pose_as_a_preference_not_a_command() {
        let catalog = default_catalog();
        let blueprint = two_carbon_bond_blueprint();
        let mut ledger = EnergyLedger::default();
        let mut energy = 1.0e12;

        let (structure, _, _) = blueprint
            .realize_with_context(&catalog, &mut ledger, &mut energy)
            .expect("constructor should find a physical orientation");

        let second = &structure.units[1];
        assert!(
            second.placement.x.hypot(second.placement.y) > 0.0,
            "coincident declared poses must not force physical overlap"
        );
    }

    #[test]
    fn exact_construction_bond_forms_the_supplied_contact() {
        let catalog = default_catalog();
        let mut structure = crate::structure::OrganismStructure::new();
        let anchor = crate::structure::StructuralUnit::new(
            "Carbon",
            crate::structure::Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        );
        structure.add_unit(anchor);

        let carbon = catalog.iter().find(|r| r.name == "Carbon").unwrap();
        let placements = crate::construction_runtime::candidate_placements(
            &structure,
            carbon,
            crate::structure::Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
            &[0],
            &catalog,
        );

        let mut found = false;
        for placement in placements {
            let mut trial = structure.clone();
            trial.add_unit(crate::structure::StructuralUnit::new("Carbon", placement));
            let mut cache = crate::contact::ConnectionCompatibilityCache::new();
            let candidate = crate::contact::connection_pair_candidates_cached(
                &trial, 0, 1, &catalog, &mut cache,
            )
            .into_iter()
            .find(|candidate| {
                candidate.distance <= crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
            });

            let Some(candidate) = candidate else {
                continue;
            };
            let Some((_, _, investment, _required_energy)) =
                crate::combine_runtime::selected_candidate_evaluation(
                    &trial, 0, 1, candidate, &catalog,
                )
            else {
                continue;
            };

            let mut ledger = EnergyLedger::default();
            let mut energy = 1.0e6;
            let before_energy = energy;
            let before_units = trial.units.len();
            let attempt = crate::combine_runtime::form_selected_bond(
                &mut trial,
                0,
                1,
                candidate,
                investment,
                &catalog,
                &mut cache,
                &mut ledger,
                &mut energy,
            );

            if let Some(attempt) = attempt {
                assert_eq!(attempt.unit_a, 0);
                assert_eq!(attempt.unit_b, 1);
                assert_eq!(trial.units.len(), before_units);
                assert_eq!(trial.bonds.len(), 1);
                assert!(
                    (trial.bonds[0].bond_energy - attempt.bond_energy).abs() < 1.0e-9,
                    "committed bond energy must match the transaction result"
                );
                assert!(
                    ((before_energy - energy)
                        - (attempt.energy_invested + attempt.work_cost))
                        .abs()
                        < 1.0e-6,
                    "holder energy loss must equal structural investment plus dissipated work"
                );
                assert!(
                    (attempt.net_energy_change - (energy - before_energy)).abs() < 1.0e-9,
                    "reported net energy change must match the holder balance"
                );
                assert!(
                    (ledger.total_heat_dissipated - attempt.work_cost).abs() < 1.0e-9,
                    "the ledger must record the transaction's dissipated work"
                );
                found = true;
                break;
            }
        }

        assert!(
            found,
            "no exact carbon construction contact could be formed"
        );
    }

    #[test]
    fn exact_construction_bond_rejects_stale_contact_without_mutation() {
        let catalog = default_catalog();
        let mut structure = crate::structure::OrganismStructure::new();
        structure.add_unit(crate::structure::StructuralUnit::new(
            "Carbon",
            crate::structure::Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));

        let carbon = catalog.iter().find(|r| r.name == "Carbon").unwrap();
        let placement = crate::construction_runtime::candidate_placements(
            &structure,
            carbon,
            crate::structure::Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
            &[0],
            &catalog,
        )
        .into_iter()
        .next()
        .expect("carbon should have candidate placements");

        structure.add_unit(crate::structure::StructuralUnit::new("Carbon", placement));
        let mut cache = crate::contact::ConnectionCompatibilityCache::new();
        let candidate = crate::contact::connection_pair_candidates_cached(
            &structure, 0, 1, &catalog, &mut cache,
        )
        .into_iter()
        .next()
        .expect("candidate contact should exist");

        structure.units[1].placement.x += 10.0;
        let before = structure.clone();
        let mut ledger = EnergyLedger::default();
        let mut energy = 1.0e12;

        let result = crate::combine_runtime::form_selected_bond(
            &mut structure,
            0,
            1,
            candidate,
            1.0,
            &catalog,
            &mut cache,
            &mut ledger,
            &mut energy,
        );

        assert!(result.is_none());
        assert_eq!(structure.bonds, before.bonds);
        assert_eq!(structure.units, before.units);
        assert_eq!(energy, 1.0e12);
    }
    #[test]
    fn failed_material_selection_does_not_consume_inventory() {
        let catalog = default_catalog();
        let blueprint = two_carbon_bond_blueprint();
        let mut storage = crate::material_storage::MaterialStorage::default();
        assert!(storage.store_physical(
            Material::free_base("Hydrogen", 1.0),
            vec![crate::structure::Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        ));
        let before = storage.materials_snapshot();

        let mut ledger = EnergyLedger::default();
        let mut energy = 1.0e12;
        assert!(blueprint
            .realize_with_materials(&catalog, &mut storage, &mut ledger, &mut energy,)
            .is_err());

        assert_eq!(storage.materials_snapshot(), before);
    }

    #[test]
    fn below_threshold_material_is_not_used_as_a_water_fallback() {
        let catalog = default_catalog();
        let blueprint = two_carbon_bond_blueprint();
        let mut storage = crate::material_storage::MaterialStorage::default();

        for name in ["Carbon", "Water"] {
            assert!(storage.store_physical(
                Material::free_base(name, 1.0),
                vec![crate::structure::Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                }],
                &catalog,
            ));
        }

        let before_storage = storage.materials_snapshot();
        let mut ledger = EnergyLedger::default();
        let mut energy = 1.0e12;

        assert!(blueprint
            .realize_with_materials(&catalog, &mut storage, &mut ledger, &mut energy)
            .is_err());

        assert_eq!(storage.materials_snapshot(), before_storage);
    }

    #[test]
    fn construction_uses_an_intact_composite_physical_material() {
        let catalog = default_catalog();
        let blueprint = two_carbon_bond_blueprint();
        let mut storage = crate::material_storage::MaterialStorage::default();

        assert!(storage.store_physical(
            Material {
                parts: vec![("Carbon".into(), 1.0), ("Hydrogen".into(), 1.0)],
                internal_bonds: vec![crate::resources::InternalBond {
                    part_a: 0,
                    part_b: 1,
                }],
            },
            vec![
                crate::structure::Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
                crate::structure::Placement {
                    x: 1.5,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
            ],
            &catalog,
        ));
        assert!(storage.store_physical(
            Material::free_base("Carbon", 1.0),
            vec![crate::structure::Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        ));

        let mut ledger = EnergyLedger::default();
        let mut energy = 1.0e12;
        let (structure, _, _) = blueprint
            .realize_with_materials(&catalog, &mut storage, &mut ledger, &mut energy)
            .expect("construction should use the stored composite material");

        assert_eq!(structure.units.len(), 3);
        assert_eq!(structure.bonds.len(), 2);
        assert!(storage.is_empty());
    }

    #[test]
    fn failed_later_bond_does_not_commit_earlier_ledger_or_energy() {
        let catalog = default_catalog();
        let blueprint = StructuralBlueprint::new(
            vec![
                BlueprintElement {
                    material: Material::free_base("Carbon", 1.0),
                    placement: BlueprintPlacement {
                        x: 0.0,
                        y: 0.0,
                        rotation_radians: 0.0,
                    },
                },
                BlueprintElement {
                    material: Material::free_base("Carbon", 1.0),
                    placement: BlueprintPlacement {
                        x: 0.0,
                        y: 0.0,
                        rotation_radians: 0.0,
                    },
                },
                BlueprintElement {
                    material: Material::free_base("Carbon", 1.0),
                    placement: BlueprintPlacement {
                        x: 0.0,
                        y: 0.0,
                        rotation_radians: 0.0,
                    },
                },
            ],
            vec![
                BlueprintConnection {
                    element_a: 0,
                    element_b: 1,
                },
                BlueprintConnection {
                    element_a: 1,
                    element_b: 2,
                },
            ],
        );
        let mut storage = crate::material_storage::MaterialStorage::default();
        for _ in 0..2 {
            assert!(storage.store_physical(
                Material::free_base("Carbon", 1.0),
                vec![crate::structure::Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                }],
                &catalog,
            ));
        }
        assert!(storage.store_physical(
            Material::free_base("Hydrogen", 1.0),
            vec![crate::structure::Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            }],
            &catalog,
        ));

        let before_storage = storage.materials_snapshot();
        let mut ledger = EnergyLedger::default();
        let before_ledger = ledger;
        let mut energy = 1.0e12;
        let before_energy = energy;

        assert!(blueprint
            .realize_with_materials(&catalog, &mut storage, &mut ledger, &mut energy)
            .is_err());

        assert_eq!(storage.materials_snapshot(), before_storage);
        assert_eq!(
            ledger.total_potential_energy_released,
            before_ledger.total_potential_energy_released
        );
        assert_eq!(
            ledger.total_usable_energy_gained,
            before_ledger.total_usable_energy_gained
        );
        assert_eq!(
            ledger.total_heat_dissipated,
            before_ledger.total_heat_dissipated
        );
        assert_eq!(energy, before_energy);
    }

    #[test]
    fn insufficient_energy_rolls_back_selected_bond_structure_and_ledger() {
        let catalog = default_catalog();
        let mut structure = crate::structure::OrganismStructure::new();
        structure.add_unit(crate::structure::StructuralUnit::new(
            "Carbon",
            crate::structure::Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));

        let carbon = catalog.iter().find(|r| r.name == "Carbon").unwrap();
        let mut found_failure = false;
        for placement in crate::construction_runtime::candidate_placements(
            &structure,
            carbon,
            crate::structure::Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
            &[0],
            &catalog,
        ) {
            let mut trial = structure.clone();
            trial.add_unit(crate::structure::StructuralUnit::new("Carbon", placement));
            let mut cache = crate::contact::ConnectionCompatibilityCache::new();
            let candidates = crate::contact::connection_pair_candidates_cached(
                &trial, 0, 1, &catalog, &mut cache,
            );
            for candidate in candidates {
                let Some((_, _, investment, required)) =
                    crate::combine_runtime::selected_candidate_evaluation(
                        &trial, 0, 1, candidate, &catalog,
                    )
                else {
                    continue;
                };
                if required <= 0.0 {
                    continue;
                }

                let before_structure = trial.clone();
                let mut ledger = EnergyLedger::default();
                let before_ledger = ledger;
                let mut energy = 0.0;

                let result = crate::combine_runtime::form_selected_bond(
                    &mut trial,
                    0,
                    1,
                    candidate,
                    investment,
                    &catalog,
                    &mut cache,
                    &mut ledger,
                    &mut energy,
                );

                assert!(result.is_none(), "bond should fail without enough energy");
                assert_eq!(trial.units, before_structure.units);
                assert_eq!(trial.bonds, before_structure.bonds);
                assert_eq!(energy, 0.0);
                assert_eq!(
                    ledger.total_potential_energy_released,
                    before_ledger.total_potential_energy_released
                );
                assert_eq!(
                    ledger.total_usable_energy_gained,
                    before_ledger.total_usable_energy_gained
                );
                assert_eq!(
                    ledger.total_heat_dissipated,
                    before_ledger.total_heat_dissipated
                );
                assert_eq!(
                    ledger.total_usable_energy_held,
                    before_ledger.total_usable_energy_held
                );
                found_failure = true;
                break;
            }
            if found_failure {
                break;
            }
        }

        assert!(
            found_failure,
            "no candidate with a positive energy requirement was found"
        );
    }

    #[test]
    fn whole_structure_overlap_check_catches_non_anchor_constituent() {
        let catalog = default_catalog();
        let mut structure = crate::structure::OrganismStructure::new();
        structure.add_unit(crate::structure::StructuralUnit::new(
            "Carbon",
            crate::structure::Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        structure.add_unit(crate::structure::StructuralUnit::new(
            "Carbon",
            crate::structure::Placement {
                x: 10.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));
        structure.add_unit(crate::structure::StructuralUnit::new(
            "Carbon",
            crate::structure::Placement {
                x: 10.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
        ));

        let candidate = structure.units[2].clone();
        assert!(
            crate::construction_runtime::placed_unit_overlaps(
                &structure,
                &candidate,
                &[2],
                &catalog,
            ),
            "candidate must be checked against every existing constituent, not only its anchor"
        );
    }
}
