
    let attempt = combine_specific_pair(
        &mut structure,
        ua,
        ub,
        &catalog,
        &mut cache,
        &mut ledger,
        &mut energy,
    )
    .expect("COMBINE should form the new bond");

    assert_eq!(structure.bonds.len(), 1);
    assert!(attempt.energy_invested >= 0.0);
    assert!(ledger.total_usable_energy_gained.is_finite());
    assert_eq!(ledger.total_heat_dissipated, attempt.work_cost);
    assert!(energy < 100.0 || attempt.energy_invested == 0.0);
}

#[test]
fn stored_realized_single_constituent_enters_combine_through_physical_path() {
    let mut simulation = Simulation::new(11, 20.0);
    let organism = &mut simulation.organisms[0];
    organism.usable_energy = 1_000.0;