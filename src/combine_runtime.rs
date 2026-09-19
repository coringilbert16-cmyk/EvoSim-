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
use crate::resources::{BaseResource, Material};
use crate::state::{EnergyLedger, Environment, Organism};
use crate::structure::{BondEndpoint, ConnectionEndpoint, Placement, StructuralUnit};

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
    water: f64,
}

fn water_field_amount(environment: &Environment, organism: &Organism) -> f64 {
    organism
        .occupied_cells
        .first()
        .and_then(|p| environment.field.index_for_position(p.x, p.y))
        .map(|i| {
            environment.field.cells[i]
                .materials
                .iter()
                .flat_map(|m| m.parts.iter())
                .filter(|(n, _)| n == "Water")
                .map(|(_, a)| *a)
                .sum()
        })
        .unwrap_or(0.0)
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
    water: f64,
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
    let (interaction, work, investment) = required_investment(a, b, evaluation, water).ok()?;
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
        water,
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
    let (interaction, work, threshold) = required_investment(a, b, evaluation, water).ok()?;
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

fn physical_material_candidate(
    material: &Material,
    placement: Placement,
    catalog: &[BaseResource],
) -> Option<StructuralUnit> {
    if !material.is_valid() || material.is_empty() || material.parts.is_empty() {
        return None;
    }
    if material.has_internal_structure() {
        return None;
    }
    let (name, amount) = material.parts.first()?;
    if (*amount - 1.0).abs() > EPSILON {
        return None;
    }
    let mut unit =
        StructuralUnit::from_material(Material::free_base(name.clone(), 1.0), placement)?;
    if !unit.realize_default_geometry(catalog) {
        return None;
    }
    Some(unit)
}

pub(crate) fn instantiate_one_unit(
    organism: &mut Organism,
    catalog: &[BaseResource],
) -> Option<usize> {
    let material = organism.stored_material.first_material()?;
    let (x, y) = organism
        .occupied_cells
        .first()
        .map(|p| (p.x, p.y))
        .unwrap_or((0.0, 0.0));
    if material.has_internal_structure() {
        let instance = organism.stored_material.peek_matching_physical(&material)?;
        if !instance.is_realized() {
            return None;
        }
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
        return indices.first().copied();
    }
    let unit = physical_material_candidate(
        &material,
        Placement {
            x,
            y,
            rotation_radians: 0.0,
        },
        catalog,
    )?;
    organism.stored_material.take_matching(&material)?;
    Some(organism.structure.add_unit(unit))
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
    let water = water_field_amount(environment, organism);

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
                            water,
                        ) {
                            let developmental_score = developmental
                                .map(|(blueprint, origin_ref, orientation, preferred_length)| {
                                    let local = crate::developmental_blueprint::developmental_point(
                                        candidate.endpoint_a.world_point(&hypothetical.units[ua], &environment.catalog)?.x,
                                        candidate.endpoint_a.world_point(&hypothetical.units[ua], &environment.catalog)?.y,
                                        origin_ref,
                                        orientation,
                                    );
                                    blueprint.material_preference_scaled(
                                        &first_resource,
                                        local.0,
                                        local.1,
                                        preferred_length,
                                    ) + blueprint.density_preference_scaled(
                                        local.0,
                                        local.1,
                                        preferred_length,
                                    ) + 0.25 * blueprint.connectivity_preference_scaled(
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
                    water,
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

    let geometry_source = raw
        .parts
        .first()
        .and_then(|(name, _)| environment.catalog.iter().find(|b| b.name == *name))?;
    let physical_instance = organism
        .stored_material
        .peek_matching_physical(&raw)
        .filter(|instance| instance.is_realized());
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
            let ub = if let Some(instance) = physical_instance.as_ref() {
                let indices = crate::material_restoration::restore_material(
                    &mut hypothetical,
                    instance,
                    placement,
                    &environment.catalog,
                )?;
                *indices.first()?
            } else {
                hypothetical.add_unit(physical_material_candidate(
                    &raw,
                    placement,
                    &environment.catalog,
                )?)
            };
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
                    water,
                ) {
                    candidates.push((ua, placement, evaluation, candidate.distance, required));
                }
            }
        }
    }
    candidates.sort_by(|a, b| a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal));
    for (ua, placement, evaluation, _, required) in candidates {
        if organism.usable_energy + EPSILON < required {
            continue;
        }
        let mut hypothetical = organism.structure.clone();
        let ub = if let Some(instance) = physical_instance.as_ref() {
            let indices = crate::material_restoration::restore_material(
                &mut hypothetical,
                instance,
                placement,
                &environment.catalog,
            )?;
            *indices.first()?
        } else {
            hypothetical.add_unit(physical_material_candidate(
                &raw,
                placement,
                &environment.catalog,
            )?)
        };
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
                water,
            },
            &environment.catalog,
            cache,
            &mut candidate_ledger,
            &mut candidate_energy,
        ) {
            if physical_instance.is_some() {
                organism.stored_material.take_matching_physical(&raw)?;
            } else {
                organism.stored_material.take_matching(&raw)?;
            }
            organism.structure = hypothetical;
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
    water: f64,
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
                evaluate_candidate(structure, unit_a, unit_b, candidate, catalog, water)?;
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
                water,
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

pub(crate) fn try_combine(
    organism: &mut Organism,
    environment: &Environment,
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
    if organism.structure.units.len() < 2 {
        return None;
    }
    let catalog = &environment.catalog;
    let water = water_field_amount(environment, organism);
    let mut pairs = Vec::new();
    for ua in 0..organism.structure.units.len() {
        for ub in ua + 1..organism.structure.units.len() {
            for candidate in eligible_candidates(&organism.structure, ua, ub, catalog, cache) {
                if let Some((evaluation, _, _, _, required)) =
                    evaluate_candidate(&organism.structure, ua, ub, candidate, catalog, water)
                {
                    let developmental_score = developmental.map(|(blueprint, origin, orientation, preferred_length)| {
                        let wa = candidate.endpoint_a.world_point(&organism.structure.units[ua], catalog)?;
                        let wb = candidate.endpoint_b.world_point(&organism.structure.units[ub], catalog)?;
                        let la = crate::developmental_blueprint::developmental_point(wa.x, wa.y, origin, orientation);
                        let lb = crate::developmental_blueprint::developmental_point(wb.x, wb.y, origin, orientation);
                        blueprint.connectivity_preference_scaled((la.0 + lb.0) * 0.5, (la.1 + lb.1) * 0.5, preferred_length)
                    }).unwrap_or(0.0);
                    pairs.push((ua, ub, evaluation, candidate.distance, required, developmental_score));
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
                water,
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
