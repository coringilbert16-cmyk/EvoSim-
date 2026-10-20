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
            field: ActiveMaterialField::new(100.0, 100.0, 10.0),
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
    #[test]
    fn decomposition_break_reconciles_usable_and_structural_energy() {
        let catalog = default_catalog();
        let blueprint = crate::juvenile::confirmed_seed_baseline(&catalog).unwrap();
        let mut structure = blueprint.realize(&catalog).unwrap();
        assert!(!structure.bonds.is_empty());
        structure.bonds.truncate(1);

        let initial_structural_energy: f64 =
            structure.bonds.iter().map(|bond| bond.bond_energy).sum();
        assert!(initial_structural_energy > 0.0);
        let initial_usable_energy = 1000.0;
        let mut body = DecomposingBody::new(
            structure,
            initial_usable_energy,
            Position { x: 50.0, y: 50.0 },
        )
        .unwrap();
        let environment = Environment {
            width: 100.0,
            height: 100.0,
            catalog,
            field: ActiveMaterialField::new(100.0, 100.0, 10.0),
        };
        let mut ledger = EnergyLedger::default();

        let step = resolve_one_bond_with_ledger(&mut body, &environment, &mut ledger).unwrap();
        let remaining_structural_energy: f64 = body
            .structure
            .bonds
            .iter()
            .map(|bond| bond.bond_energy)
            .sum();
        let residual = (body.energy_budget - initial_usable_energy)
            + (remaining_structural_energy - initial_structural_energy)
            + step.heat;

        assert!(
            residual.abs() <= 1e-9,
            "decomposition energy residual: {residual}"
        );
        assert_eq!(ledger.total_potential_energy_released, 0.0);
        assert!((ledger.total_structural_energy_delta + initial_structural_energy).abs() <= 1e-9);
        assert!((ledger.total_heat_dissipated - step.heat).abs() <= 1e-9);
    }

    #[test]
    fn stress_break_reconciles_usable_and_structural_energy() {
        use rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;

        let catalog = default_catalog();
        let mut organism = crate::state::Simulation::create_initial_organism();
        organism.usable_energy = 1000.0;
        assert!(!organism.structure.bonds.is_empty());
        let initial_usable_energy = organism.usable_energy;
        let initial_structural_energy: f64 = organism
            .structure
            .bonds
            .iter()
            .map(|bond| bond.bond_energy)
            .sum();
        let environment = Environment {
            width: 1000.0,
            height: 1000.0,
            catalog: catalog.clone(),
            field: ActiveMaterialField::new(1000.0, 1000.0, 10.0),
        };
        let mut ledger = EnergyLedger::default();
        let mut rng = ChaCha8Rng::seed_from_u64(19);

        assert!(crate::transformation::resolve_stress_break(
            &mut organism,
            &environment,
            &mut ledger,
            &mut rng,
        ));

        let remaining_structural_energy: f64 = organism
            .structure
            .bonds
            .iter()
            .map(|bond| bond.bond_energy)
            .sum();
        let residual = (organism.usable_energy - initial_usable_energy)
            + (remaining_structural_energy - initial_structural_energy)
            + ledger.total_work_expended
            + ledger.total_heat_dissipated;

        assert!(
            residual.abs() <= 1e-9,
            "stress-break energy residual: {residual}"
        );
        assert_eq!(ledger.total_potential_energy_released, 0.0);
        assert!(
            (ledger.total_structural_energy_delta
                - (remaining_structural_energy - initial_structural_energy))
                .abs()
                <= 1e-9
        );
    }
}
