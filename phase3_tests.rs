#[cfg(test)]
mod phase3_tests {
    use crate::combine::{experimental_interaction, ExperimentalInteraction};
    use crate::combine_runtime::try_combine;
    use crate::contact::ConnectionPairCandidate;
    use crate::resources::ResourceProperties;
    use crate::state::Simulation;
    use crate::structure::{Placement, StructuralUnit};

    fn candidate(facing: f64, distance: f64) -> ConnectionPairCandidate {
        ConnectionPairCandidate {
            point_a: 0,
            point_b: 0,
            distance,
            facing,
            load_a: 0.0,
            load_b: 0.0,
            available_a: true,
            available_b: true,
        }
    }

    fn props(potential_energy: f64, reactivity: f64, cohesion: f64) -> ResourceProperties {
        ResourceProperties {
            mass: 1.0,
            potential_energy,
            reactivity,
            cohesion,
        }
    }

    fn prepare_pair(sim: &mut Simulation, a: &str, b: &str) {
        let organism = &mut sim.organisms[0];
        organism.structure.units.clear();
        organism.structure.bonds.clear();
        organism.usable_energy = 25.0;
        organism.structure.add_unit(StructuralUnit::new(
            a,
            Placement {
                x: 500.0,
                y: 500.0,
                rotation_radians: 0.0,
            },
        ));
        organism.structure.add_unit(StructuralUnit::new(
            b,
            Placement {
                x: 501.0,
                y: 500.0,
                rotation_radians: 0.0,
            },
        ));
    }

    #[test]
    fn combine_surplus_obeys_complete_energy_conservation() {
        let mut sim = Simulation::new(17, 10.0);
        prepare_pair(&mut sim, "Carbon", "Methane");
        let organism = &mut sim.organisms[0];
        let energy_before = organism.usable_energy;
        let mut cache = crate::contact::ConnectionCompatibilityCache::new();

        let attempt = try_combine(organism, &sim.environment, &mut cache)
            .expect("Carbon-Methane at contact should produce a COMBINE attempt");

        assert!(attempt.potential_energy_released > attempt.formation_work);
        assert_eq!(attempt.energy_paid, 0.0);
        let conservation_balance = attempt.energy_paid + attempt.potential_energy_released
            - attempt.formation_work
            - attempt.bond_energy
            - attempt.usable_energy_gained
            - attempt.heat_dissipated;
        let conservation_tolerance = 1e-9 * attempt.potential_energy_released.max(1.0);
        assert!(conservation_balance.abs() < conservation_tolerance);
        let energy_balance = organism.usable_energy
            - (energy_before + attempt.usable_energy_gained - attempt.energy_paid);
        assert!(energy_balance.abs() < 1e-12);
        assert_eq!(organism.structure.bonds.len(), 1);
    }

    #[test]
    fn ledger_deficit_conserves_energy_with_organism_subsidy() {
        let mut ledger = crate::state::EnergyLedger::default();
        let release = 2.0;
        let paid = 3.0;
        let formation_work = 5.0;
        ledger.record_combine(release, paid, formation_work, 0.0, 0.0, 0.0);

        assert!((release + paid - formation_work).abs() < 1e-12);
        let balance = ledger.total_potential_energy_released + ledger.total_usable_energy_spent
            - ledger.total_formation_energy_spent
            - ledger.total_bond_energy_created
            - ledger.total_usable_energy_gained
            - ledger.total_heat_dissipated;
        assert!(balance.abs() < 1e-12);
    }

    #[test]
    fn insufficient_combine_energy_is_atomic() {
        let mut sim = Simulation::new(19, 10.0);
        prepare_pair(&mut sim, "Carbon", "Nitrogen");
        let organism = &mut sim.organisms[0];
        organism.usable_energy = 0.0;
        let units_before = organism.structure.units.len();
        let bonds_before = organism.structure.bonds.len();
        let energy_before = organism.usable_energy;
        let stored_before = organism.stored_unbonded.parts.clone();
        let mut cache = crate::contact::ConnectionCompatibilityCache::new();

        let attempt = try_combine(organism, &sim.environment, &mut cache);

        assert!(attempt.is_none());
        assert_eq!(organism.structure.units.len(), units_before);
        assert_eq!(organism.structure.bonds.len(), bonds_before);
        assert_eq!(organism.usable_energy, energy_before);
        assert_eq!(organism.stored_unbonded.parts, stored_before);
    }

    #[test]
    fn combine_ledger_accumulates_multiple_events() {
        let mut ledger = crate::state::EnergyLedger::default();
        ledger.record_combine(10.0, 0.0, 4.0, 3.0, 2.0, 1.0);
        ledger.record_combine(5.0, 2.0, 7.0, 0.0, 0.0, 0.0);

        assert_eq!(ledger.total_potential_energy_released, 15.0);
        assert_eq!(ledger.total_formation_energy_spent, 11.0);
        assert_eq!(ledger.total_usable_energy_spent, 2.0);
        assert_eq!(ledger.total_bond_energy_created, 3.0);
        assert_eq!(ledger.total_usable_energy_gained, 2.0);
        assert_eq!(ledger.total_heat_dissipated, 1.0);
        let balance = ledger.total_potential_energy_released + ledger.total_usable_energy_spent
            - ledger.total_formation_energy_spent
            - ledger.total_bond_energy_created
            - ledger.total_usable_energy_gained
            - ledger.total_heat_dissipated;
        assert!(balance.abs() < 1e-12);
    }

    #[test]
    fn potential_energy_establishes_direction() {
        let a = props(1.0, 1.0, 0.5);
        let b = props(10.0, 1.0, 0.5);
        let forward = experimental_interaction(a, b, candidate(1.0, 0.0), 0.0);
        let reverse = experimental_interaction(b, a, candidate(1.0, 0.0), 0.0);
        let equal = experimental_interaction(a, props(1.0, 1.0, 0.5), candidate(1.0, 0.0), 0.0);

        assert_eq!(forward.direction, 1.0);
        assert_eq!(reverse.direction, -1.0);
        assert_eq!(equal.direction, 0.0);
        assert!(forward.magnitude > 0.0);
    }

    #[test]
    fn reactivity_and_geometry_modify_interaction_magnitude() {
        let low_reactivity = props(1.0, 0.5, 0.5);
        let high_reactivity = props(10.0, 2.0, 0.5);
        let high_reactivity_result =
            experimental_interaction(low_reactivity, high_reactivity, candidate(1.0, 0.0), 0.0);
        let low_reactivity_result = experimental_interaction(
            low_reactivity,
            props(10.0, 0.5, 0.5),
            candidate(1.0, 0.0),
            0.0,
        );
        let distant_result =
            experimental_interaction(low_reactivity, high_reactivity, candidate(1.0, 1.0), 0.0);
        let poor_facing_result =
            experimental_interaction(low_reactivity, high_reactivity, candidate(0.0, 0.0), 0.0);

        assert!(high_reactivity_result.magnitude > low_reactivity_result.magnitude);
        assert!(high_reactivity_result.magnitude > distant_result.magnitude);
        assert!(high_reactivity_result.magnitude > poor_facing_result.magnitude);
    }

    #[test]
    fn water_dilutes_reactivity() {
        let a = props(1.0, 1.0, 0.5);
        let b = props(10.0, 1.0, 0.5);
        let dry = experimental_interaction(a, b, candidate(1.0, 0.0), 0.0);
        let wet = experimental_interaction(a, b, candidate(1.0, 0.0), 100.0);

        assert!(wet.magnitude < dry.magnitude);
    }

    #[test]
    fn simulation_ledger_records_combine_outputs_and_current_holdings() {
        let mut sim = Simulation::new(23, 10.0);
        sim.decision_parameters.survival_reserve = 100.0;
        prepare_pair(&mut sim, "Carbon", "Methane");
        let energy_before = sim.organisms[0].usable_energy;

        sim.step();

        assert!(sim.energy_ledger.total_potential_energy_released > 0.0);
        assert!(sim.energy_ledger.total_formation_energy_spent > 0.0);
        assert!(sim.energy_ledger.total_bond_energy_created > 0.0);
        assert!(sim.energy_ledger.total_usable_energy_gained > 0.0);
        assert!(sim.energy_ledger.total_heat_dissipated >= 0.0);
        let ledger_balance = sim.energy_ledger.total_potential_energy_released
            + sim.energy_ledger.total_usable_energy_spent
            - sim.energy_ledger.total_formation_energy_spent
            - sim.energy_ledger.total_bond_energy_created
            - sim.energy_ledger.total_usable_energy_gained
            - sim.energy_ledger.total_heat_dissipated;
        let ledger_tolerance =
            1e-9 * sim.energy_ledger.total_potential_energy_released.max(1.0);
        assert!(ledger_balance.abs() < ledger_tolerance);
        let organism_balance = sim.organisms[0].usable_energy
            - (energy_before + sim.energy_ledger.total_usable_energy_gained
                - sim.energy_ledger.total_usable_energy_spent);
        assert!(organism_balance.abs() < 1e-9);
        let held_balance = sim.energy_ledger.total_usable_energy_held
            - sim.organisms.iter().map(|o| o.usable_energy).sum::<f64>();
        assert!(held_balance.abs() < 1e-12);
    }

    #[test]
    fn adult_mass_is_genomic() {
        let mut sim = Simulation::new(29, 10.0);
        let organism = &mut sim.organisms[0];
        organism
            .genome
            .traits
            .iter_mut()
            .find(|t| t.name == "adult_mass")
            .unwrap()
            .value = 40.0;

        assert_eq!(organism.genome.adult_mass(), 40.0);
    }

    #[test]
    fn interaction_result_remains_finite() {
        let result: ExperimentalInteraction = experimental_interaction(
            props(1.0, 1.0, 0.5),
            props(10.0, 1.0, 0.5),
            candidate(1.0, 0.0),
            0.0,
        );
        assert!(result.direction.is_finite());
        assert!(result.magnitude.is_finite());
        assert!(result.signed_value.is_finite());
    }
}
