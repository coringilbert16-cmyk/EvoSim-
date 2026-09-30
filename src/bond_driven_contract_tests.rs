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
            let Some((_, _, _, _, investment)) =
                crate::combine_runtime::construction_candidate_evaluation(
                    &trial, 0, 1, candidate, &catalog,
                )
            else {
                continue;
            };

            let mut ledger = EnergyLedger::default();
            let mut energy = 1.0e12;
            let before_units = trial.units.len();
            let attempt = crate::combine_runtime::form_construction_bond(
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

        let result = crate::combine_runtime::form_construction_bond(
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
}
