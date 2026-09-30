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

        let (structure, _) = blueprint
            .realize_with_context(&catalog, &mut ledger, &mut energy)
            .expect("bond-driven constructor should realize a valid two-carbon bond");

        assert_eq!(structure.units.len(), 2);
        assert_eq!(structure.bonds.len(), 1);

        let bond = &structure.bonds[0];
        let a = structure
            .units
            .iter()
            .find(|unit| unit.id == bond.endpoint_a.constituent_id)
            .expect("bond endpoint A must identify a realized constituent");
        let b = structure
            .units
            .iter()
            .find(|unit| unit.id == bond.endpoint_b.constituent_id)
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

        let (structure, _) = blueprint
            .realize_with_context(&catalog, &mut ledger, &mut energy)
            .expect("constructor should find a physical orientation");

        let second = &structure.units[1];
        assert!(
            second.placement.x.hypot(second.placement.y) > 0.0,
            "coincident declared poses must not force physical overlap"
        );
    }
}
