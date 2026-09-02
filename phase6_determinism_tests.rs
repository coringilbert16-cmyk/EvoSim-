#[cfg(test)]
mod determinism_tests {
    use crate::state::Simulation;

    #[test]
    fn same_seed_produces_identical_initial_snapshot() {
        let left = Simulation::new(12345, 10.0).snapshot();
        let right = Simulation::new(12345, 10.0).snapshot();

        let left_json = serde_json::to_string(&left).expect("snapshot serializes");
        let right_json = serde_json::to_string(&right).expect("snapshot serializes");
        assert_eq!(left_json, right_json);
    }

    #[test]
    fn same_seed_produces_identical_trajectory() {
        let mut left = Simulation::new(12345, 10.0);
        let mut right = Simulation::new(12345, 10.0);

        for _ in 0..100 {
            let left_json = serde_json::to_string(&left.step()).expect("snapshot serializes");
            let right_json = serde_json::to_string(&right.step()).expect("snapshot serializes");
            assert_eq!(left_json, right_json);
        }
    }

    #[test]
    fn different_seeds_change_initial_environment_distribution() {
        let left = Simulation::new(12345, 10.0).snapshot();
        let right = Simulation::new(54321, 10.0).snapshot();

        let left_json = serde_json::to_string(&left.environment.reservoir)
            .expect("reservoir serializes");
        let right_json = serde_json::to_string(&right.environment.reservoir)
            .expect("reservoir serializes");
        assert_ne!(left_json, right_json);

        assert_eq!(left.organisms.len(), right.organisms.len());
        assert_eq!(left.organisms[0].structure.units.len(), right.organisms[0].structure.units.len());
        assert_eq!(left.organisms[0].usable_energy, right.organisms[0].usable_energy);
    }
}
