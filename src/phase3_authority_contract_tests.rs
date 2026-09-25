#[cfg(test)]
mod tests {
    use crate::decomposition::{resolve_one_bond_with_ledger, DecomposingBody};
    use crate::environment::ActiveMaterialField;
    use crate::resources::default_catalog;
    use crate::state::{EnergyLedger, Environment, Position};

    #[test]
    fn decomposition_release_returns_realized_material_to_environment() {
        let catalog = default_catalog();
        let blueprint = crate::juvenile::confirmed_seed_baseline(&catalog).unwrap();
        let mut structure = blueprint.realize(&catalog).unwrap();
        assert!(!structure.bonds.is_empty());
        structure.bonds.truncate(1);
        let expected_units = structure.units.len();
        let position = Position { x: 50.0, y: 50.0 };
        let mut body = DecomposingBody::new(structure, 1000.0, position.clone()).unwrap();
        let mut environment = Environment {
            width: 100.0,
            height: 100.0,
            catalog: catalog.clone(),
            field: ActiveMaterialField::new(100.0, 100.0, 10.0),\n            resource_cloud: crate::environmental_materials::ResourceCloud::initial(50.0, 50.0),
        };
        let mut ledger = EnergyLedger::default();

        let step = resolve_one_bond_with_ledger(&mut body, &environment, &mut ledger).unwrap();
        let released = step.released_material.unwrap();
        assert_eq!(released.len(), expected_units);
        assert!(released.iter().all(|material| material.is_realized()));

        for material in released {
            assert!(environment.field.deposit(position.x, position.y, material));
        }
        let index = environment
            .field
            .index_for_position(position.x, position.y)
            .unwrap();
        let cell = &environment.field.cells[index];
        assert_eq!(cell.physical_materials.len(), expected_units);
        assert!(cell.materials.is_empty());
    }
}
