)> {
    let existing_indices = realized_units[neighbor].as_ref()?.clone();
    let new_endpoints = physical_material_endpoint_options(new_material, catalog);

    if existing_indices.is_empty() || new_endpoints.is_empty() {
        return None;
    }

    let target = blueprint.elements[_index].placement;
    let (s, c) = genome_anchor.rotation_radians.sin_cos();
    let target_world = (
        genome_anchor.x + (target.x - anchor_declared.x) * c
            - (target.y - anchor_declared.y) * s,
        genome_anchor.y
            + (target.x - anchor_declared.x) * s
            + (target.y - anchor_declared.y) * c,
    );
    let mut best: Option<(
        f64,
        f64,
        OrganismStructure,
        Vec<usize>,
        usize,
        crate::combine_runtime::CombineAttempt,
        EnergyLedger,
        f64,
    )> = None;

    for existing_index in existing_indices {
        let existing_unit = structure.units.get(existing_index)?;
        let existing_endpoints = structure_unit_endpoint_options(existing_unit, catalog);
        for endpoint_a in existing_endpoints {
            let joint = endpoint_a.world_point(&structure.units[existing_index], catalog)?;
            for (part_index, endpoint_b) in new_endpoints.iter().copied() {
                let local_b = physical_material_endpoint_local_point(
                    new_material,
                    part_index,
                    endpoint_b,
                    catalog,
                )?;

                // The blueprint pose is a preference, not a placement command.
                // The analytic target angle is followed by exact boundary alignments.
                let ideal_angle = (joint.y - target_world.1).atan2(joint.x - target_world.0)
                    - local_b.y.atan2(local_b.x);

                let Some(existing_shape) = existing_unit.shape(catalog) else {
                    continue;
                };
                let Some(candidate_shape) = new_material
                    .material
                    .parts
                    .get(part_index)
                    .and_then(|(name, _)| resource(catalog, name))
                    .map(|resource| &resource.shape)
                else {
                    continue;
                };
                let angles = construction_angle_candidates(
                    existing_shape,
                    endpoint_a,
                    existing_unit.placement.rotation_radians,
                    candidate_shape,
                    endpoint_b,
                    new_material
                        .placements
                        .as_ref()
                        .and_then(|placements| placements.get(part_index))
                        .map(|placement| placement.rotation_radians)
                        .unwrap_or(0.0),
                    ideal_angle,
                );
                for angle in angles {
                    let candidate_origin =
                        placement_for_joint((local_b.x, local_b.y), (joint.x, joint.y), angle);
                    *nodes += 1;

                    let mut trial = structure.clone();
                    let Some(indices) = crate::material_restoration::restore_material(
                        &mut trial,
                        new_material,
                        candidate_origin,
                        catalog,
                    ) else {
                        continue;
                    };

                    let new_unit_index = *indices.get(part_index)?;
                    let ignored_units = indices.clone();
                    if indices.iter().any(|index| {
                        placed_unit_overlaps(&trial, &trial.units[*index], &ignored_units, catalog)
                    }) {
                        continue;
                    }

                    let mut trial_ledger = *ledger;
                    let mut trial_energy = available_energy;
                    let mut bond_cache = crate::contact::ConnectionCompatibilityCache::new();

                    // The official connection points determine where the
                    // constructor works from and how the new material is placed.
                    // Once the material is physically placed, the actual bond may
                    // land anywhere on the touching boundaries.
                    let Some(candidate) = crate::contact::connection_pair_candidates_cached(
                        &trial,
                        existing_index,
                        new_unit_index,
                        catalog,
                        &mut bond_cache,
                    )
                    .into_iter()
                    .filter(|candidate| {
                        candidate.distance <= crate::combine_runtime::COMBINE_CONTACT_TOLERANCE
                            && candidate.available_a
                            && candidate.available_b
                    })
                    .min_by(|a, b| {
                        a.distance
                            .partial_cmp(&b.distance)
                            .unwrap_or(std::cmp::Ordering::Equal)
                            .then_with(|| {
                                b.facing
                                    .partial_cmp(&a.facing)
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            })
                    }) else {
                        continue;
                    };

                    let Some((_, _, investment, _required_energy)) =
                        crate::combine_runtime::selected_candidate_evaluation(
                            &trial,
                            existing_index,
                            new_unit_index,
                            candidate,
                            catalog,
                        )
                    else {
                        continue;
                    };

                    let Some(attempt) = crate::combine_runtime::form_selected_bond(
                        &mut trial,
                        existing_index,
                        new_unit_index,
                        candidate,
                        investment,
                        catalog,
                        &mut bond_cache,
                        &mut trial_ledger,
                        &mut trial_energy,
                    ) else {
                        continue;
                    };

                    // Several current attachments may be physically valid. Choose the
                    // one that best preserves the declared spatial preference; this uses
                    // no information about future connections.
                    let placed = trial.units[new_unit_index].placement;
                    let distance_to_target =
                        (placed.x - target_world.0).hypot(placed.y - target_world.1);
                    let rotation_error =
                        normalize_construction_angle(placed.rotation_radians - target.rotation_radians)
                            .abs();
                    let better = best.as_ref().is_none_or(|current| {
                        (distance_to_target, rotation_error) < (current.0, current.1)
                    });
                    if better {
                        best = Some((
                            distance_to_target,
                            rotation_error,
                            trial,
                            indices,
                            part_index,
                            attempt,
                            trial_ledger,
                            trial_energy,
                        ));
                    }
                }
            }
        }
    }

    best.map(|(_, _, trial, indices, part_index, attempt, trial_ledger, trial_energy)| {
        (trial, indices, part_index, attempt, trial_ledger, trial_energy)
    })
}

/// Bond-driven construction is forward-only. Once a bond is formed it is
/// never undone. When a requested joint cannot be realized, the constructor
/// keeps trying available material variants and orientations until it finds a
/// physically valid attachment. The resulting physical graph, not the declared
/// poses, is authoritative.
pub(crate) fn construct_blueprint_bond_driven(
    blueprint: &crate::structural_blueprint::StructuralBlueprint,