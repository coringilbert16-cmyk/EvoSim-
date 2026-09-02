#[cfg(test)]
mod phase3_tests {
    use crate::combine_runtime::try_combine;
    use crate::resources::default_catalog;
    use crate::state::Simulation;
    use crate::structure::{Placement, StructuralUnit};

    #[test]
    fn combine_attempt_reports_potential_release_and_partitioned_surplus() {
        let mut sim = Simulation::new(17, 10.0);
        let organism = &mut sim.organisms[0];
        organism.structure.units.clear();
        organism.structure.bonds.clear();
        organism.usable_energy = 25.0;
        organism.structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 500.0,
                y: 500.0,
                rotation_radians: 0.0,
            },
        ));
        organism.structure.add_unit(StructuralUnit::new(
            "Methane",
            Placement {
                x: 501.0,
                y: 500.0,
                rotation_radians: 0.0,
            },
        ));

        let catalog = default_catalog();
        let mut cache = crate::contact::ConnectionCompatibilityCache::new();
        let attempt = try_combine(organism, &sim.environment, &mut cache)
            .expect("Carbon-Methane at contact should produce a COMBINE attempt");

        assert!(attempt.potential_energy_released > 0.0);
        assert!(attempt.work_cost >= 0.0);
        assert!(attempt.energy_paid >= 0.0);
        assert!((attempt.surplus
            - (attempt.potential_energy_released - attempt.formation_threshold).max(0.0))
            .abs()
            < 1e-12);
        assert!((attempt.bond_energy + attempt.usable_energy_gained + attempt.heat_dissipated
            - attempt.surplus)
            .abs()
            < 1e-9 * attempt.surplus.max(1.0));
        assert_eq!(organism.structure.bonds.len(), 1);
        assert!(catalog.iter().any(|resource| resource.name == "Carbon"));
    }

    #[test]
    fn simulation_energy_ledger_tracks_combine_attempt_when_executed() {
        let mut sim = Simulation::new(23, 10.0);
        sim.decision_parameters.survival_reserve = 100.0;
        let organism = &mut sim.organisms[0];
        organism.structure.units.clear();
        organism.structure.bonds.clear();
        organism.usable_energy = 25.0;
        organism.structure.add_unit(StructuralUnit::new(
            "Carbon",
            Placement {
                x: 500.0,
                y: 500.0,
                rotation_radians: 0.0,
            },
        ));
        organism.structure.add_unit(StructuralUnit::new(
            "Methane",
            Placement {
                x: 501.0,
                y: 500.0,
                rotation_radians: 0.0,
            },
        ));

        sim.step();

        assert!(sim.energy_ledger.total_potential_energy_released > 0.0);
        assert!(sim.energy_ledger.total_usable_energy_gained > 0.0);
        assert!(sim.energy_ledger.total_heat_dissipated >= 0.0);
        assert!((sim.energy_ledger.total_usable_energy_held
            - sim.organisms.iter().map(|o| o.usable_energy).sum::<f64>())
            .abs()
            < 1e-12);
    }
}
