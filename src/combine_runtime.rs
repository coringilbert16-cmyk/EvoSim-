#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
//! Runtime COMBINE execution boundary.
//! Physics is evaluated by `combine`; this module selects a physical
//! candidate, applies the returned result, mutates structure, and settles
//! the actual energy holder through the unified ledger authority.
use crate::combine::{
    bond_strength, eligible_candidates, required_investment, ExperimentalInteraction,
    FormationEvaluation,
};
use crate::contact::ConnectionCompatibilityCache;
use crate::developmental_blueprint::DevelopmentalFieldBlueprint;
use crate::energy_ledger::{EnergyLedgerAuthority, EnergyReason, EnergyTransaction};
use crate::resources::BaseResource;
use crate::state::{EnergyLedger, Environment, Organism};
use crate::structure::{BondEndpoint, ConnectionEndpoint, Placement};

const EPSILON: f64 = 1e-12;
pub(crate) const COMBINE_CONTACT_TOLERANCE: f64 = 1.0;

/// Developmental context is solver intent only. Physical validity is still
/// established by the normal COMBINE candidate and formation checks.
pub(crate) type DevelopmentalContext<'a> = (&'a DevelopmentalFieldBlueprint, (f64, f64), f64, f64);

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CombineAttempt {
    pub unit_a: usize,
    pub unit_b: usize,
    pub endpoint_a: ConnectionEndpoint,
    pub endpoint_b: ConnectionEndpoint,
    pub work_cost: f64,
    pub energy_invested: f64,
    pub interaction_direction: f64,
    pub interaction_magnitude: f64,
    pub interaction_energy: f64,
    pub formation_threshold: f64,
    pub net_energy_change: f64,
    pub bond_strength: f64,
    pub bond_energy: f64,
}

#[derive(Clone, Copy)]
struct BondFormationRequest {
    unit_a: usize,
    unit_b: usize,
    endpoint_a: ConnectionEndpoint,
    endpoint_b: ConnectionEndpoint,
    investment: f64,
}

fn energy_requirement(investment: f64, work: f64, interaction: f64) -> Option<f64> {
    if !investment.is_finite()
        || investment < 0.0
        || !work.is_finite()
        || work < 0.0
        || !interaction.is_finite()
    {
        return None;
    }
    let required = investment + work - interaction;
    required.is_finite().then_some(required.max(0.0))
}

fn evaluate_candidate(
    structure: &crate::structure::OrganismStructure,
    ua: usize,
    ub: usize,
    candidate: crate::contact::ConnectionPairCandidate,
    catalog: &[BaseResource],
) -> Option<(FormationEvaluation, ExperimentalInteraction, f64, f64, f64)> {
    if candidate.distance > COMBINE_CONTACT_TOLERANCE
        || !candidate.available_a
        || !candidate.available_b
    {
        return None;
    }
    let a = structure.units.get(ua)?.properties(catalog)?;
    let b = structure.units.get(ub)?.properties(catalog)?;
    let evaluation = crate::combine::evaluate_formation(candidate, a.cohesion, b.cohesion);
    let (interaction, work, investment) = required_investment(a, b, evaluation).ok()?;
    let required = energy_requirement(investment, work, interaction.signed_value)?;
    Some((evaluation, interaction, work, investment, required))
}

fn form_bond(
    structure: &mut crate::structure::OrganismStructure,
    request: BondFormationRequest,
    catalog: &[BaseResource],
    cache: &mut ConnectionCompatibilityCache,
    ledger: &mut EnergyLedger,
    energy: &mut f64,
) -> Option<CombineAttempt> {
    let BondFormationRequest {
        unit_a: ua,
        unit_b: ub,
        endpoint_a,
        endpoint_b,
        investment,
    } = request;
    if ua >= structure.units.len() || ub >= structure.units.len() || ua == ub {
        return None;
    }
    let id_a = structure.physical_id(ua)?;
    let id_b = structure.physical_id(ub)?;
    let candidate =
        crate::contact::connection_pair_candidates_cached(structure, ua, ub, catalog, cache)
            .into_iter()
            .find(|c| {
                c.endpoint_a == endpoint_a
                    && c.endpoint_b == endpoint_b
                    && c.distance <= COMBINE_CONTACT_TOLERANCE
                    && c.available_a
                    && c.available_b
            })?;
    let a = structure.units[ua].properties(catalog)?;
    let b = structure.units[ub].properties(catalog)?;
    let evaluation = crate::combine::evaluate_formation(candidate, a.cohesion, b.cohesion);
    if !crate::combine::formation_succeeds(evaluation, investment) {
        return None;
    }
    let (interaction, work, threshold) = required_investment(a, b, evaluation).ok()?;
    if (threshold - investment).abs() > EPSILON || interaction.signed_value < 0.0 {
        return None;
    }
    let strength = bond_strength(a, b);
    if !strength.is_finite() {
        return None;
    }
    let mut trial_structure = structure.clone();
    let bond = crate::structure::Bond {
        endpoint_a: BondEndpoint::new(id_a, endpoint_a),
        endpoint_b: BondEndpoint::new(id_b, endpoint_b),
        strength,
        bond_energy: investment,
    };
    crate::contact::try_add_bond(&mut trial_structure, bond, catalog).ok()?;
    let before = *energy;
    let transaction = EnergyTransaction {
        reason: EnergyReason::Combine,
        potential_released: interaction.signed_value,
        usable_delta: interaction.signed_value - investment - work,
        structural_delta: investment,
        heat_dissipated: work,
    };
    if !ledger.settle_transaction(energy, transaction) {
        *energy = before;
        return None;
    }
    let net = *energy - before;
    *structure = trial_structure;
    Some(CombineAttempt {
        unit_a: ua,
        unit_b: ub,
        endpoint_a,
        endpoint_b,
        work_cost: work,
        energy_invested: investment,
        interaction_direction: interaction.direction,
        interaction_magnitude: interaction.magnitude,
        interaction_energy: interaction.signed_value,
        formation_threshold: threshold,
        net_energy_change: net,
        bond_strength: strength,
        bond_energy: investment,
    })
}

pub(crate) fn instantiate_one_unit(
    organism: &mut Organism,
    catalog: &[BaseResource],
) -> Option<usize> {
    let material = organism.stored_material.first_material()?;
    if !material.is_valid() || material.is_empty() {
        return None;
    }
    let instance = organism.stored_material.peek_matching_physical(&material)?;
    if !instance.is_realized() {
        return None;
    }
    let (x, y) = organism
        .occupied_cells
        .first()
        .map(|p| (p.x, p.y))
        .unwrap_or((0.0, 0.0));
    let mut trial = organism.structure.clone();
    let indices = crate::material_restoration::restore_material(
        &mut trial,
        &instance,
        Placement {
            x,
            y,
            rotation_radians: 0.0,
        },
        catalog,
    )?;
    organism.stored_material.take_matching_physical(&material)?;
    organism.structure = trial;
    indices.first().copied()
}

pub(crate) fn try_combine_stored_unit(
    organism: &mut Organism,
    environment: &Environment,
    cache: &mut ConnectionCompatibilityCache,
    ledger: &mut EnergyLedger,
    developmental: Option<DevelopmentalContext<'_>>,
) -> Option<CombineAttempt> {
    let raw = organism.stored_material.first_material()?;
    if !raw.is_valid() || raw.is_empty() {
        return None;
    }

    if raw.has_internal_structure() {
        let instance = organism.stored_material.peek_matching_physical(&raw)?;
        if !instance.is_realized() {
            return None;
        }
        let first_resource = raw.parts.first()?.0.as_str();
        let geometry_source = environment
            .catalog
            .iter()
            .find(|resource| resource.name == first_resource)?;
        let mut candidates = Vec::new();
        for ua in 0..organism.structure.units.len() {
            let anchor = organism.structure.units[ua].placement;
            for origin in crate::construction_runtime::candidate_placements(
                &organism.structure,
                geometry_source,
                anchor,
                &[ua],
                &environment.catalog,
            ) {
                let mut hypothetical = organism.structure.clone();
                let indices = crate::material_restoration::restore_material(
                    &mut hypothetical,
                    &instance,
                    origin,
                    &environment.catalog,
                )?;
                for (part_index, &ub) in indices.iter().enumerate() {
                    for candidate in crate::contact::connection_pair_candidates_cached(
                        &hypothetical,
                        ua,
                        ub,
                        &environment.catalog,
                        cache,
                    ) {
                        if let Some((evaluation, _, _, _, required)) = evaluate_candidate(
                            &hypothetical,
                            ua,
                            ub,
                            candidate,
                            &environment.catalog,
                        ) {
                            let developmental_score = developmental
                                .map(|(blueprint, origin_ref, orientation, preferred_length)| {
                                    let Some(world) = candidate
                                        .endpoint_a
                                        .world_point(&hypothetical.units[ua], &environment.catalog)
                                    else {
                                        return 0.0;
                                    };
                                    let local = crate::developmental_blueprint::developmental_point(
                                        world.x,
                                        world.y,
                                        origin_ref,
                                        orientation,
                                    );
                                    crate::developmental_blueprint::CANDIDATE_MATERIAL_WEIGHT
                                        * blueprint.material_preference_scaled(
                                            first_resource,
                                            local.0,
                                            local.1,
                                            preferred_length,
                                        )
                                        + crate::developmental_blueprint::CANDIDATE_DENSITY_WEIGHT
                                            * blueprint.density_preference_scaled(
                                                local.0,
                                                local.1,
                                                preferred_length,
                                            )
                                        + crate::developmental_blueprint::CANDIDATE_CONNECTIVITY_WEIGHT
                                            * blueprint.connectivity_preference_scaled(
                                                local.0,
                                                local.1,
                                                preferred_length,
                                            )
                                })
                                .unwrap_or(0.0);
                            candidates.push((
                                ua,
                                part_index,
                                origin,
                                evaluation,
                                candidate.distance,
                                required,
                                developmental_score,
                            ));
                        }
                    }
                }
            }
        }
        candidates.sort_by(|a, b| {
            b.6.partial_cmp(&a.6)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.4.partial_cmp(&b.4).unwrap_or(std::cmp::Ordering::Equal))
        });
        for (ua, part_index, origin, evaluation, _, required, _) in candidates {
            if organism.usable_energy + EPSILON < required {
                continue;
            }
            let mut hypothetical = organism.structure.clone();
            let indices = crate::material_restoration::restore_material(
                &mut hypothetical,
                &instance,
                origin,
                &environment.catalog,
            )?;
            let ub = *indices.get(part_index)?;
            let mut candidate_ledger = *ledger;
            let mut candidate_energy = organism.usable_energy;
            if let Some(attempt) = form_bond(
                &mut hypothetical,
                BondFormationRequest {
                    unit_a: ua,
                    unit_b: ub,
                    endpoint_a: evaluation.candidate.endpoint_a,
                    endpoint_b: evaluation.candidate.endpoint_b,
                    investment: evaluation.threshold,
                },
                &environment.catalog,
                cache,
                &mut candidate_ledger,
                &mut candidate_energy,
            ) {
                organism.stored_material.take_matching_physical(&raw)?;
                organism.structure = hypothetical;
                organism.usable_energy = candidate_energy;
                *ledger = candidate_ledger;
                organism.add_transaction_stress(attempt.work_cost);
                return Some(attempt);
            }
        }
        return None;
    }

    let first_resource = raw.parts.first()?.0.as_str();
    let geometry_source = raw
        .parts
        .first()
        .and_then(|(name, _)| environment.catalog.iter().find(|b| b.name == *name))?;
    let physical_instance = organism
        .stored_material
        .peek_matching_physical(&raw)
        .filter(|instance| instance.is_realized())?;
    let mut candidates = Vec::new();
    for ua in 0..organism.structure.units.len() {
        let anchor = organism.structure.units[ua].placement;
        for placement in crate::construction_runtime::candidate_placements(
            &organism.structure,
            geometry_source,
            anchor,
            &[ua],
            &environment.catalog,
        ) {
            let mut hypothetical = organism.structure.clone();
            let indices = crate::material_restoration::restore_material(
                &mut hypothetical,
                &physical_instance,
                placement,
                &environment.catalog,
            )?;
            let ub = *indices.first()?;
            for candidate in crate::contact::connection_pair_candidates_cached(
                &hypothetical,
                ua,
                ub,
                &environment.catalog,
                cache,
            ) {
                if let Some((evaluation, _, _, _, required)) =
                    evaluate_candidate(&hypothetical, ua, ub, candidate, &environment.catalog)
                {
                    let developmental_score = developmental
                        .map(|(blueprint, origin, orientation, preferred_length)| {
                            let Some(wa) = candidate
                                .endpoint_a
                                .world_point(&hypothetical.units[ua], &environment.catalog)
                            else {
                                return 0.0;
                            };
                            let local = crate::developmental_blueprint::developmental_point(
                                wa.x,
                                wa.y,
                                origin,
                                orientation,
                            );
                            crate::developmental_blueprint::CANDIDATE_MATERIAL_WEIGHT
                                * blueprint.material_preference_scaled(
                                    first_resource,
                                    local.0,
                                    local.1,
                                    preferred_length,
                                )
                                + crate::developmental_blueprint::CANDIDATE_DENSITY_WEIGHT
                                    * blueprint.density_preference_scaled(
                                        local.0,
                                        local.1,
                                        preferred_length,
                                    )
                                + crate::developmental_blueprint::CANDIDATE_CONNECTIVITY_WEIGHT
                                    * blueprint.connectivity_preference_scaled(
                                        local.0,
                                        local.1,
                                        preferred_length,
                                    )
                        })
                        .unwrap_or(0.0);
                    candidates.push((
                        ua,
                        placement,
                        evaluation,
                        candidate.distance,
                        required,
                        developmental_score,
                    ));
                }
            }
        }
    }
    candidates.sort_by(|a, b| {
        b.5.partial_cmp(&a.5)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal))
    });
    for (ua, placement, evaluation, _, required, _) in candidates {
        if organism.usable_energy + EPSILON < required {
            continue;
        }
        let mut hypothetical = organism.structure.clone();
        let indices = crate::material_restoration::restore_material(
            &mut hypothetical,
            &physical_instance,
            placement,
            &environment.catalog,
        )?;
        let ub = *indices.first()?;
        let mut candidate_ledger = *ledger;
        let mut candidate_energy = organism.usable_energy;
        if let Some(attempt) = form_bond(
            &mut hypothetical,
            BondFormationRequest {
                unit_a: ua,
                unit_b: ub,
                endpoint_a: evaluation.candidate.endpoint_a,
                endpoint_b: evaluation.candidate.endpoint_b,
                investment: evaluation.threshold,
            },
            &environment.catalog,
            cache,
            &mut candidate_ledger,
            &mut candidate_energy,
        ) {
            organism.stored_material.take_matching_physical(&raw)?;
            organism.structure = hypothetical;
            organism.mark_structure_changed();
            organism.usable_energy = candidate_energy;
            *ledger = candidate_ledger;
            organism.add_transaction_stress(attempt.work_cost);
            return Some(attempt);
        }
    }
    None
}

pub(crate) fn combine_specific_pair(
    structure: &mut crate::structure::OrganismStructure,
    unit_a: usize,
    unit_b: usize,
    catalog: &[BaseResource],
    cache: &mut ConnectionCompatibilityCache,
    ledger: &mut EnergyLedger,
    energy: &mut f64,
) -> Option<CombineAttempt> {
    if unit_a >= structure.units.len() || unit_b >= structure.units.len() || unit_a == unit_b {
        return None;
    }
    let mut candidates = eligible_candidates(structure, unit_a, unit_b, catalog, cache)
        .into_iter()
        .filter_map(|candidate| {
            let (evaluation, _, _, _, required) =
                evaluate_candidate(structure, unit_a, unit_b, candidate, catalog)?;
            Some((evaluation, candidate.distance, required))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    for (evaluation, _, required) in candidates {
        if *energy + EPSILON < required {
            continue;
        }
        let mut trial_structure = structure.clone();
        let mut trial_ledger = *ledger;
        let mut trial_energy = *energy;
        if let Some(attempt) = form_bond(
            &mut trial_structure,
            BondFormationRequest {
                unit_a,
                unit_b,
                endpoint_a: evaluation.candidate.endpoint_a,
                endpoint_b: evaluation.candidate.endpoint_b,
                investment: evaluation.threshold,
            },
            catalog,
            cache,
            &mut trial_ledger,
            &mut trial_energy,
        ) {
            *structure = trial_structure;
            *ledger = trial_ledger;
            *energy = trial_energy;
            return Some(attempt);
        }
    }
    None
}

pub(crate) fn can_combine(organism: &Organism, _environment: &Environment) -> bool {
    if organism.active_transformation_id.is_some() {
        return false;
    }

    // Environmental physical material can participate directly in COMBINE when
    // a relevant connection point is inside an accessible interior region.
    // Candidate search remains deferred to action resolution.
    if !organism.structure.units.is_empty() {
        return true;
    }
    !organism.stored_material.is_empty()
}

fn try_combine_environmental(
    organism: &mut Organism,
    environment: &mut Environment,
    cache: &mut ConnectionCompatibilityCache,
    ledger: &mut EnergyLedger,
    developmental: Option<DevelopmentalContext<'_>>,
) -> Option<CombineAttempt> {
    let regions = crate::interior_geometry::find_accessible_interior_regions(
        &organism.structure,
        &environment.catalog,
    )
    .ok()?;
    if regions.is_empty() {
        return None;
    }
    let body = crate::organism_geometry::OrganismBodyGeometry::from_structure(
        &organism.structure,
        &environment.catalog,
    )?;

    let candidate_cells = environment.field.cells_intersecting_bounds(
        body.min_x,
        body.max_x,
        body.min_y,
        body.max_y,
    );
    let mut candidates = Vec::new();

    for cell_index in candidate_cells {
        let physicals = environment
            .field
            .cells
            .get(cell_index)
            .map(|cell| cell.physical_materials.clone())
            .unwrap_or_default();
        for (material_index, instance) in physicals.into_iter().enumerate() {
            if !instance.is_realized() || instance.material.is_empty() {
                continue;
            }
            let mut hypothetical = organism.structure.clone();
            let Some(indices) = crate::material_restoration::restore_material(
                &mut hypothetical,
                &instance,
                Placement {
                    x: 0.0,
                    y: 0.0,
                    rotation_radians: 0.0,
                },
                &environment.catalog,
            ) else {
                continue;
            };
            for ua in 0..organism.structure.units.len() {
                for &ub in &indices {
                    for candidate in crate::contact::connection_pair_candidates_cached(
                        &hypothetical,
                        ua,
                        ub,
                        &environment.catalog,
                        cache,
                    ) {
                        if !crate::interior_geometry::endpoint_in_accessible_interior(
                            candidate.endpoint_a,
                            &hypothetical.units[ua],
                            &environment.catalog,
                            &regions,
                        ) || !crate::interior_geometry::endpoint_in_accessible_interior(
                            candidate.endpoint_b,
                            &hypothetical.units[ub],
                            &environment.catalog,
                            &regions,
                        ) {
                            continue;
                        }
                        if let Some((evaluation, _, _, _, required)) = evaluate_candidate(
                            &hypothetical,
                            ua,
                            ub,
                            candidate,
                            &environment.catalog,
                        ) {
                            let developmental_score = developmental
                                .map(|(blueprint, origin, orientation, preferred_length)| {
                                    let Some(a) = candidate.endpoint_a.world_point(
                                        &hypothetical.units[ua],
                                        &environment.catalog,
                                    ) else {
                                        return 0.0;
                                    };
                                    let Some(b) = candidate.endpoint_b.world_point(
                                        &hypothetical.units[ub],
                                        &environment.catalog,
                                    ) else {
                                        return 0.0;
                                    };
                                    let local = crate::developmental_blueprint::developmental_point(
                                        (a.x + b.x) * 0.5,
                                        (a.y + b.y) * 0.5,
                                        origin,
                                        orientation,
                                    );
                                    crate::developmental_blueprint::CANDIDATE_CONNECTIVITY_WEIGHT
                                        * blueprint.connectivity_preference_scaled(
                                            local.0,
                                            local.1,
                                            preferred_length,
                                        )
                                })
                                .unwrap_or(0.0);
                            candidates.push((
                                cell_index,
                                material_index,
                                instance,
                                ua,
                                ub,
                                evaluation,
                                candidate.distance,
                                required,
                                developmental_score,
                            ));
                        }
                    }
                }
            }
        }
    }

    candidates.sort_by(|a, b| {
        b.8.partial_cmp(&a.8)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.6.partial_cmp(&b.6).unwrap_or(std::cmp::Ordering::Equal))
    });

    for (
        cell_index,
        material_index,
        instance,
        ua,
        ub,
        evaluation,
        _distance,
        required,
        _score,
    ) in candidates
    {
        if organism.usable_energy + EPSILON < required {
            continue;
        }
        let mut trial_structure = organism.structure.clone();
        let indices = crate::material_restoration::restore_material(
            &mut trial_structure,
            &instance,
            Placement {
                x: 0.0,
                y: 0.0,
                rotation_radians: 0.0,
            },
            &environment.catalog,
        )?;
        let restored_ub = *indices.get(ub.saturating_sub(organism.structure.units.len()))?;
        let mut trial_ledger = *ledger;
        let mut trial_energy = organism.usable_energy;
        let Some(attempt) = form_bond(
            &mut trial_structure,
            BondFormationRequest {
                unit_a: ua,
                unit_b: restored_ub,
                endpoint_a: evaluation.candidate.endpoint_a,
                endpoint_b: evaluation.candidate.endpoint_b,
                investment: evaluation.threshold,
            },
            &environment.catalog,
            cache,
            &mut trial_ledger,
            &mut trial_energy,
        ) else {
            continue;
        };

        let removed = environment
            .field
            .cells
            .get_mut(cell_index)?
            .physical_materials
            .get(material_index)?
            .clone();
        if removed != instance {
            continue;
        }
        environment.field.cells[cell_index]
            .physical_materials
            .remove(material_index);
        environment.field.mark_changed_at_index(cell_index);
        organism.structure = trial_structure;
        organism.usable_energy = trial_energy;
        *ledger = trial_ledger;
        organism.add_transaction_stress(attempt.work_cost);
        return Some(attempt);
    }
    None
}

pub(crate) fn try_combine(
    organism: &mut Organism,
    environment: &mut Environment,
    cache: &mut ConnectionCompatibilityCache,
    ledger: &mut EnergyLedger,
    developmental: Option<DevelopmentalContext<'_>>,
) -> Option<CombineAttempt> {
    if !organism.structure.units.is_empty() && !organism.stored_material.is_empty() {
        if let Some(attempt) =
            try_combine_stored_unit(organism, environment, cache, ledger, developmental)
        {
            return Some(attempt);
        }
    }
    if let Some(attempt) =
        try_combine_environmental(organism, environment, cache, ledger, developmental)
    {
        return Some(attempt);
    }
    if organism.structure.units.len() < 2 {
        return None;
    }
    let catalog = &environment.catalog;
    let mut pairs = Vec::new();
    for ua in 0..organism.structure.units.len() {
        for ub in ua + 1..organism.structure.units.len() {
            for candidate in eligible_candidates(&organism.structure, ua, ub, catalog, cache) {
                if let Some((evaluation, _, _, _, required)) =
                    evaluate_candidate(&organism.structure, ua, ub, candidate, catalog)
                {
                    let developmental_score = developmental
                        .map(|(blueprint, origin, orientation, preferred_length)| {
                            let Some(wa) = candidate
                                .endpoint_a
                                .world_point(&organism.structure.units[ua], catalog)
                            else {
                                return 0.0;
                            };
                            let Some(wb) = candidate
                                .endpoint_b
                                .world_point(&organism.structure.units[ub], catalog)
                            else {
                                return 0.0;
                            };
                            let la = crate::developmental_blueprint::developmental_point(
                                wa.x,
                                wa.y,
                                origin,
                                orientation,
                            );
                            let lb = crate::developmental_blueprint::developmental_point(
                                wb.x,
                                wb.y,
                                origin,
                                orientation,
                            );
                            blueprint.connectivity_preference_scaled(
                                (la.0 + lb.0) * 0.5,
                                (la.1 + lb.1) * 0.5,
                                preferred_length,
                            )
                        })
                        .unwrap_or(0.0);
                    pairs.push((
                        ua,
                        ub,
                        evaluation,
                        candidate.distance,
                        required,
                        developmental_score,
                    ));
                }
            }
        }
    }
    pairs.sort_by(|a, b| {
        b.5.partial_cmp(&a.5)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal))
    });
    for (ua, ub, evaluation, _, required, _) in pairs {
        if organism.usable_energy + EPSILON < required {
            continue;
        }
        let mut trial_structure = organism.structure.clone();
        let mut trial_ledger = *ledger;
        let mut trial_energy = organism.usable_energy;
        if let Some(attempt) = form_bond(
            &mut trial_structure,
            BondFormationRequest {
                unit_a: ua,
                unit_b: ub,
                endpoint_a: evaluation.candidate.endpoint_a,
                endpoint_b: evaluation.candidate.endpoint_b,
                investment: evaluation.threshold,
            },
            catalog,
            cache,
            &mut trial_ledger,
            &mut trial_energy,
        ) {
            organism.structure = trial_structure;
            organism.usable_energy = trial_energy;
            *ledger = trial_ledger;
            organism.add_transaction_stress(attempt.work_cost);
            return Some(attempt);
        }
    }
    None
}
