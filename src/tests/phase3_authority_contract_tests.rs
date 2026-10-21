#[cfg(test)]
mod tests {
    use crate::decomposition::{resolve_one_bond_with_ledger, DecomposingBody};
    use crate::environment::ActiveMaterialField;
    use crate::resources::default_catalog;
    use crate::state::{EnergyLedger, Environment, Position};

    #[test]
    fn decomposition_release_returns_realized_material_to_environment() {
        let catalog = default_catalog();
        let simulation = crate::state::Simulation::new(902, 10.0);
        let mut structure = simulation.organisms[0].structure.clone();
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
        let simulation = crate::state::Simulation::new(902, 10.0);
        let mut structure = simulation.organisms[0].structure.clone();
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
    fn stress_without_resonance_cannot_break_a_bond_or_spend_energy() {
        let catalog = default_catalog();
        let mut simulation = crate::state::Simulation::new(903, 10.0);
        let mut organism = simulation.organisms.remove(0);
        organism.usable_energy = 1000.0;
        organism.stress = 1.0e9;
        let original_bonds = organism.structure.bonds.clone();
        let original_usable_energy = organism.usable_energy;
        let environment = Environment {
            width: 1000.0,
            height: 1000.0,
            catalog,
            field: ActiveMaterialField::new(1000.0, 1000.0, 10.0),
        };
        let mut ledger = EnergyLedger::default();

        let _ = crate::simulation::Simulation::apply_survival_damage(
            &mut organism,
            &environment,
            &mut ledger,
            &mut simulation.rng,
        );

        assert_eq!(organism.structure.bonds, original_bonds);
        assert_eq!(organism.usable_energy, original_usable_energy);
        assert_eq!(ledger.total_structural_energy_delta, 0.0);
        assert_eq!(ledger.total_heat_dissipated, 0.0);
        assert_eq!(ledger.total_potential_energy_released, 0.0);
    }
}
