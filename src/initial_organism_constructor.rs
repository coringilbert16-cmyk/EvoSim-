    // shapes on every frontier-growth step.
    let construction_candidates = rigid_resources(catalog)
        .filter_map(|resource| {
            Some((
                resource.name.clone(),
                realized_single_resource(resource, catalog)?,
            ))
        })
        .collect::<Vec<_>>();

    loop {
        let acquisition_candidates = available_acquisition_resources(catalog);
        if acquisition_candidates.len() >= 3
            && catalog.iter().any(|resource| resource.name == "Water")
            && valid_construction(&structure, catalog, &acquisition_candidates).is_some()
        {
            return Ok((structure, ledger, energy));
        }

        let Some((next_ledger, next_energy, next_frontier)) =
            grow_one_step(
                &mut structure,
                catalog,
                &construction_candidates,
                &frontier,
                &mut occupancy,
                &mut spatial_index,
                &mut nodes,
                &ledger,
                energy,
            )
        else {
            let cavity = crate::cavity::analyze_genome_cavity(&structure, catalog)
                .ok()
                .flatten()
                .is_some_and(|cavity| cavity.qualifies());
            return Err(format!(
                "forward physical construction reached a state with no valid local growth step: units={}, bonds={}, genome_cavity={}, placement_nodes={nodes}",
                structure.units.len(),
                structure.bonds.len(),
                cavity
            ));
        };

        ledger = next_ledger;
        energy = next_energy;
        frontier = next_frontier;
    }
}

pub(crate) fn construct_valid(catalog: &[BaseResource]) -> Result<ValidConstruction, String> {