#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
//! Runtime COMBINE execution boundary.
//! Physics is evaluated by `combine`; this module selects a physical
//! candidate, applies the returned result, mutates structure, and settles
//! the actual energy holder through the unified ledger authority.
use crate::combine::{bond_strength, eligible_candidates, formation_cost, FormationEvaluation};
use crate::contact::ConnectionCompatibilityCache;
use crate::developmental_blueprint::DevelopmentalFieldBlueprint;
use crate::energy_ledger::{EnergyLedgerAuthority, EnergyReason, EnergyTransaction};
use crate::physical_material::PhysicalMaterial;
use crate::resources::BaseResource;
use crate::state::{EnergyLedger, Environment, Organism};
use crate::structure::{BondEndpoint, ConnectionEndpoint, Placement};

const EPSILON: f64 = 1e-12;
/// Physical contact tolerance. Touch is permitted; penetration is not.
pub(crate) const COMBINE_CONTACT_TOLERANCE: f64 = 0.1;

/// Developmental context is solver intent only. Physical validity is still
/// established by the normal COMBINE candidate and formation checks.
pub(crate) type DevelopmentalContext<'a> = (&'a DevelopmentalFieldBlueprint, (f64, f64), f64, f64);

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CombineAttempt {
    pub unit_a: usize,
    pub unit_b: usize,
    pub endpoint_a: ConnectionEndpoint,
    pub endpoint_b: ConnectionEndpoint,
    pub work_cost: f64,
    pub energy_invested: f64,
    pub formation_threshold: f64,
    pub net_energy_change: f64,
    pub bond_strength: f64,
    pub bond_energy: f64,
    pub environmental_source: Option<(usize, PhysicalMaterial)>,
}

#[derive(Clone, Copy)]
struct BondFormationRequest {
    unit_a: usize,
    unit_b: usize,
    endpoint_a: ConnectionEndpoint,
    endpoint_b: ConnectionEndpoint,
    investment: f64,
}

fn energy_requirement(investment: f64, bond_energy: f64, work: f64) -> Option<f64> {
    if !investment.is_finite()
        || investment < 0.0
        || !bond_energy.is_finite()
        || bond_energy < 0.0
        || !work.is_finite()
        || work < 0.0
    {
        return None;
    }
    let required = investment + bond_energy + work;
    required.is_finite().then_some(required)
}

fn combine_energy_transaction(
    investment: f64,
    bond_energy: f64,
    work: f64,
    transient_fluid_bond: bool,
) -> Option<EnergyTransaction> {
    if !investment.is_finite()
        || investment < 0.0
        || !bond_energy.is_finite()
        || bond_energy < 0.0
        || !work.is_finite()
        || work < 0.0
    {
        return None;
    }
    // Only a bond that survives COMBINE retains structural energy. Formation
    // work is tracked separately; investment not retained as a bond is waste.
    let (structural_allocation, work_expended, dissipated_heat) = if transient_fluid_bond {
        (0.0, work, investment + bond_energy)
    } else {
        (bond_energy, work, investment)
    };
    EnergyTransaction::expenditure(
        EnergyReason::Combine,
        structural_allocation,
        work_expended,
        dissipated_heat,
    )
}

fn evaluate_candidate(
    structure: &crate::structure::OrganismStructure,
    ua: usize,
    ub: usize,
    candidate: crate::contact::ConnectionPairCandidate,
    catalog: &[BaseResource],
) -> Option<(FormationEvaluation, f64, f64, f64)> {
    if candidate.distance > COMBINE_CONTACT_TOLERANCE
        || !candidate.available_a
        || !candidate.available_b
    {
        return None;
    }
    let a = structure.units.get(ua)?.properties(catalog)?;
    let b = structure.units.get(ub)?.properties(catalog)?;
    // Chemical materials must have a real attractive interaction at the
    // realized contact. This makes chemistry an actual formation driver,
    // rather than merely a source of stored bond energy after COMBINE.
    if a.chemical_position.is_some() && b.chemical_position.is_some() {
        let interaction = crate::combine::chemical_interaction(
            a,
            b,
            candidate,
            crate::chemistry::CHEMICAL_K,
            crate::chemistry::CHEMICAL_D_MAX,
            crate::chemistry::CHEMICAL_CONTACT_RADIUS,
            crate::chemistry::CHEMICAL_MAX_FORCE,
        )?;
        if interaction.attraction <= EPSILON {
            return None;
        }
    }
    let evaluation = crate::combine::evaluate_formation(candidate, a.cohesion, b.cohesion);
    let (work, investment) = formation_cost(a, b, evaluation).ok()?;
    let strength = bond_strength(a, b);
    if !strength.is_finite() {
        return None;
    }
    let bond_energy = crate::combine::intrinsic_bond_potential(a, b, strength);
    if !bond_energy.is_finite() || bond_energy < 0.0 {
        return None;
    }
    let required = energy_requirement(investment, bond_energy, work)?;
    Some((evaluation, work, investment, required))
}

/// Shared access to the universal candidate evaluation. Construction
/// supplies the exact candidate; this function does not perform candidate search.
pub(crate) fn selected_candidate_evaluation(
    structure: &crate::structure::OrganismStructure,
    unit_a: usize,
    unit_b: usize,
    candidate: crate::contact::ConnectionPairCandidate,
    catalog: &[BaseResource],
) -> Option<(FormationEvaluation, f64, f64, f64)> {
    evaluate_candidate(structure, unit_a, unit_b, candidate, catalog)
}

pub(crate) fn form_specific_bond(
    structure: &mut crate::structure::OrganismStructure,
    unit_a: usize,
    unit_b: usize,
    endpoint_a: ConnectionEndpoint,
    endpoint_b: ConnectionEndpoint,
    catalog: &[BaseResource],
    cache: &mut ConnectionCompatibilityCache,
    ledger: &mut EnergyLedger,
    energy: &mut f64,
) -> Option<CombineAttempt> {
    let candidate = crate::contact::connection_pair_candidates_cached(
        structure, unit_a, unit_b, catalog, cache,
    )
    .into_iter()
    .find(|candidate| {
        candidate.endpoint_a == endpoint_a
            && candidate.endpoint_b == endpoint_b
            && candidate.distance <= COMBINE_CONTACT_TOLERANCE
            && candidate.available_a
            && candidate.available_b
    })?;
    let (_, _, investment, _) = evaluate_candidate(structure, unit_a, unit_b, candidate, catalog)?;
    form_selected_bond(
        structure, unit_a, unit_b, candidate, investment, catalog, cache, ledger, energy,
    )
}

/// Forms exactly the supplied physical contact.
///
/// This is the shared transaction boundary for generic COMBINE and bond-driven
/// construction. The caller has already selected the physical endpoint pair and pose.
pub(crate) fn form_selected_bond(
    structure: &mut crate::structure::OrganismStructure,
    unit_a: usize,
    unit_b: usize,
    candidate: crate::contact::ConnectionPairCandidate,
    investment: f64,
    catalog: &[BaseResource],
    cache: &mut ConnectionCompatibilityCache,
    ledger: &mut EnergyLedger,
    energy: &mut f64,
) -> Option<CombineAttempt> {
    form_selected_bond_diagnostic(
        structure, unit_a, unit_b, candidate, investment, catalog, cache, ledger, energy,
    )
    .ok()
}

/// Same transaction authority as the optional-return wrapper, but preserves the
/// first rejection reason for constructor diagnostics. Failure semantics remain
/// atomic: no structure mutation is committed on rejection.
pub(crate) fn form_selected_bond_diagnostic(
    structure: &mut crate::structure::OrganismStructure,
    unit_a: usize,
    unit_b: usize,
    candidate: crate::contact::ConnectionPairCandidate,
    investment: f64,
    catalog: &[BaseResource],
    cache: &mut ConnectionCompatibilityCache,
    ledger: &mut EnergyLedger,
    energy: &mut f64,
) -> Result<CombineAttempt, &'static str> {
    if unit_a >= structure.units.len() || unit_b >= structure.units.len() || unit_a == unit_b {
        return Err("invalid unit indices");
    }
    if candidate.distance > COMBINE_CONTACT_TOLERANCE
        || !candidate.available_a
        || !candidate.available_b
    {
        return Err("candidate contact is outside tolerance or an endpoint is unavailable");
    }
    let point_a = candidate
        .endpoint_a
        .world_point(&structure.units[unit_a], catalog)
        .ok_or("candidate endpoint A has no world point")?;
    let point_b = candidate
        .endpoint_b
        .world_point(&structure.units[unit_b], catalog)
        .ok_or("candidate endpoint B has no world point")?;
    if (point_a.x - point_b.x).hypot(point_a.y - point_b.y) > COMBINE_CONTACT_TOLERANCE {
        return Err("candidate endpoint world points exceed contact tolerance");
    }
    form_bond_from_candidate_diagnostic(
        structure,
        BondFormationRequest {
            unit_a,
            unit_b,
            endpoint_a: candidate.endpoint_a,
            endpoint_b: candidate.endpoint_b,
            investment,
        },
        candidate,
        catalog,
        cache,
        ledger,
        energy,
    )
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
        unit_a,
        unit_b,
        endpoint_a,
        endpoint_b,
        investment,
    } = request;
    let candidate = crate::contact::connection_pair_candidates_cached(
        structure, unit_a, unit_b, catalog, cache,
    )
    .into_iter()
    .find(|candidate| {
        candidate.endpoint_a == endpoint_a
            && candidate.endpoint_b == endpoint_b
            && candidate.distance <= COMBINE_CONTACT_TOLERANCE
            && candidate.available_a
            && candidate.available_b
    })?;
    form_bond_from_candidate(
        structure,
        BondFormationRequest {
            unit_a,
            unit_b,
            endpoint_a,
            endpoint_b,
            investment,
        },
        candidate,
        catalog,
        cache,
        ledger,
        energy,
    )
}

fn form_bond_from_candidate(
    structure: &mut crate::structure::OrganismStructure,
    request: BondFormationRequest,
    candidate: crate::contact::ConnectionPairCandidate,
    catalog: &[BaseResource],
    cache: &mut ConnectionCompatibilityCache,
    ledger: &mut EnergyLedger,
    energy: &mut f64,
) -> Option<CombineAttempt> {
    form_bond_from_candidate_diagnostic(
        structure, request, candidate, catalog, cache, ledger, energy,
    )
    .ok()
}

fn form_bond_from_candidate_diagnostic(
    structure: &mut crate::structure::OrganismStructure,
    request: BondFormationRequest,
    candidate: crate::contact::ConnectionPairCandidate,
    catalog: &[BaseResource],
    _cache: &mut ConnectionCompatibilityCache,
    ledger: &mut EnergyLedger,
    energy: &mut f64,
) -> Result<CombineAttempt, &'static str> {
    let BondFormationRequest {
        unit_a: ua,
        unit_b: ub,
        endpoint_a,
        endpoint_b,
        investment,
    } = request;
    if ua >= structure.units.len() || ub >= structure.units.len() || ua == ub {
        return Err("invalid unit indices at bond commit");
    }
    let id_a = structure
        .physical_id(ua)
        .ok_or("unit A has no physical ID")?;
    let id_b = structure
        .physical_id(ub)
        .ok_or("unit B has no physical ID")?;
    let a = structure.units[ua]
        .properties(catalog)
        .ok_or("unit A properties unavailable")?;
    let b = structure.units[ub]
        .properties(catalog)
        .ok_or("unit B properties unavailable")?;
    let evaluation = crate::combine::evaluate_formation(candidate, a.cohesion, b.cohesion);
    if !crate::combine::formation_succeeds(evaluation, investment) {
        return Err("COMBINE formation criteria rejected candidate");
    }
    let (work, threshold) =
        formation_cost(a, b, evaluation).map_err(|_| "formation cost calculation failed")?;
    if (threshold - investment).abs() > EPSILON {
        return Err("selected investment differs from formation threshold");
    }
    let strength = bond_strength(a, b);
    if !strength.is_finite() {
        return Err("calculated bond strength is not finite");
    }
    let bond_energy = crate::combine::intrinsic_bond_potential(a, b, strength);
    if !bond_energy.is_finite() || bond_energy < 0.0 {
        return Err("calculated intrinsic bond energy is invalid");
    }
    let mut trial_structure = structure.clone();
    let bond = crate::structure::Bond {
        endpoint_a: BondEndpoint::new(id_a, endpoint_a),
        endpoint_b: BondEndpoint::new(id_b, endpoint_b),
        strength,
        bond_energy,
    };
    crate::contact::try_add_bond(&mut trial_structure, bond, catalog)
        .map_err(|_| "physical bond insertion rejected endpoint pair")?;
    let before = *energy;
    let fluid_pair = [ua, ub].into_iter().all(|index| {
        structure.units[index]
            .material
            .parts
            .first()
            .and_then(|(name, _)| catalog.iter().find(|resource| resource.name == *name))
            .is_some_and(|resource| {
                resource.physical_state == crate::resources::PhysicalState::Fluid
            })
    });
    let transaction = combine_energy_transaction(investment, bond_energy, work, fluid_pair)
        .ok_or("COMBINE energy transaction could not be constructed")?;
    if !ledger.settle_transaction(energy, transaction) {
        *energy = before;
        return Err("energy ledger rejected COMBINE expenditure");
    }
    let net = *energy - before;
    *structure = trial_structure;
    Ok(CombineAttempt {
        unit_a: ua,
        unit_b: ub,
        endpoint_a,
        endpoint_b,
        work_cost: work,
        energy_invested: investment + bond_energy,
        formation_threshold: threshold,
        net_energy_change: net,
        bond_strength: strength,
        bond_energy,
        environmental_source: None,
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
    organism
        .stored_material
        .take_matching_physical(&material, catalog)?;
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
                if indices.iter().any(|index| {
                    crate::construction_runtime::placed_unit_overlaps(
                        &hypothetical,
                        &hypothetical.units[*index],
                        &indices,
                        &environment.catalog,
                    )
                }) {
                    continue;
                }
                for (part_index, &ub) in indices.iter().enumerate() {
                    for candidate in crate::contact::connection_pair_candidates_cached(
                        &hypothetical,
                        ua,
                        ub,
                        &environment.catalog,
                        cache,
                    ) {
                        if let Some((evaluation, _, _, required)) = evaluate_candidate(
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
                if !coalesce_fluid_pair_after_bond(&mut hypothetical, ua, ub, &environment.catalog)
                {
                    continue;
                }
                organism
                    .stored_material
                    .take_matching_physical(&raw, &environment.catalog)?;
                organism.structure = hypothetical;
                organism.usable_energy = candidate_energy;
                *ledger = candidate_ledger;
                organism.mark_structure_changed();
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
            if indices.iter().any(|index| {
                crate::construction_runtime::placed_unit_overlaps(
                    &hypothetical,
                    &hypothetical.units[*index],
                    &indices,
                    &environment.catalog,
                )
            }) {
                continue;
            }
            let ub = *indices.first()?;
            for candidate in crate::contact::connection_pair_candidates_cached(
                &hypothetical,
                ua,
                ub,
                &environment.catalog,
                cache,
            ) {
                if let Some((evaluation, _, _, required)) =
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
            if !coalesce_fluid_pair_after_bond(&mut hypothetical, ua, ub, &environment.catalog) {
                continue;
            }
            organism
                .stored_material
                .take_matching_physical(&raw, &environment.catalog)?;
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

/// Coalesce a successfully bonded pair when both constituents are fluids.
///
/// Unsupported fluid topology rejects the trial transaction. Rigid-fluid and
/// rigid-rigid COMBINE retain their ordinary path. The live caller must pass
/// trial state: this helper stages a second clone and commits only after every
/// retained endpoint, material amount, and bond validates.
fn coalesce_fluid_pair_after_bond(
    structure: &mut crate::structure::OrganismStructure,
    first_index: usize,
    second_index: usize,
    catalog: &[BaseResource],
) -> bool {
    use crate::capillary_geometry::{
        coalesce_placed_multi_contact_boundaries, map_boundary_point_through_coalescence,
        MultiContactFluidBoundary, PlacedFluidCoalescence,
    };
    use crate::resources::{Form, PhysicalState, Shape};
    use crate::structure::{
        BondEndpoint, ConnectionEndpoint, PhysicalConstituentId, StructuralUnit,
    };

    if first_index >= structure.units.len()
        || second_index >= structure.units.len()
        || first_index == second_index
    {
        return false;
    }
    let first = structure.units[first_index].clone();
    let second = structure.units[second_index].clone();
    let is_fluid = |unit: &StructuralUnit| {
        unit.material
            .parts
            .first()
            .and_then(|(name, _)| catalog.iter().find(|resource| resource.name == *name))
            .is_some_and(|resource| resource.physical_state == PhysicalState::Fluid)
    };
    if !is_fluid(&first) || !is_fluid(&second) {
        return true;
    }

    // This conservative path currently supports only two single-resource,
    // same-resource fluids with exact multi-contact boundaries.
    if first.material.parts.len() != 1
        || second.material.parts.len() != 1
        || first.material.has_internal_structure()
        || second.material.has_internal_structure()
    {
        return false;
    }
    let (first_name, first_amount) = &first.material.parts[0];
    let (second_name, second_amount) = &second.material.parts[0];
    if first_name != second_name
        || !first_amount.is_finite()
        || !second_amount.is_finite()
        || *first_amount <= 0.0
        || *second_amount <= 0.0
    {
        return false;
    }
    let combined_amount = *first_amount + *second_amount;
    if !combined_amount.is_finite() || combined_amount <= 0.0 {
        return false;
    }

    let boundary_for = |unit: &StructuralUnit| -> Option<MultiContactFluidBoundary> {
        match &unit.shape(catalog)?.form {
            Form::Fluid {
                multi_contact_boundary: Some(boundary),
                boundary: None,
                arc_boundary: None,
                ..
            } => Some(boundary.clone()),
            _ => None,
        }
    };
    let Some(first_boundary) = boundary_for(&first) else {
        return false;
    };
    let Some(second_boundary) = boundary_for(&second) else {
        return false;
    };
    let placed = match coalesce_placed_multi_contact_boundaries(
        &first_boundary,
        (
            first.placement.x,
            first.placement.y,
            first.placement.rotation_radians,
        ),
        &second_boundary,
        (
            second.placement.x,
            second.placement.y,
            second.placement.rotation_radians,
        ),
    ) {
        Some(placed) => placed,
        None => return false,
    };

    let merged_shape = Shape {
        form: Form::Fluid {
            nominal_area: placed.merged_in_first_frame.nominal_area,
            boundary: None,
            arc_boundary: None,
            multi_contact_boundary: Some(placed.merged_in_first_frame.clone()),
        },
    };
    if !merged_shape.is_valid() {
        return false;
    }

    let first_id = first.physical_id;
    let second_id = second.physical_id;
    if first_id.0 == 0 || second_id.0 == 0 || first_id == second_id {
        return false;
    }

    fn remap_endpoint(
        endpoint: &mut BondEndpoint,
        first_id: PhysicalConstituentId,
        second_id: PhysicalConstituentId,
        first: &StructuralUnit,
        second: &StructuralUnit,
        placed: &PlacedFluidCoalescence,
        merged_shape: &Shape,
        catalog: &[BaseResource],
    ) -> bool {
        const MAP_TOLERANCE: f64 = 1e-7;
        let (source_unit, source_boundary, piece_map) = if endpoint.constituent_id == first_id {
            (first, &placed.first_world, &placed.first_piece_to_merged)
        } else if endpoint.constituent_id == second_id {
            (second, &placed.second_world, &placed.second_piece_to_merged)
        } else {
            return true;
        };
        let Some(world_point) = endpoint.location.world_point(source_unit, catalog) else {
            return false;
        };
        let Some(mapped) = map_boundary_point_through_coalescence(
            world_point.x,
            world_point.y,
            source_boundary,
            piece_map,
            &placed.merged_world,
            MAP_TOLERANCE,
        ) else {
            return false;
        };

        let dx = mapped.x - first.placement.x;
        let dy = mapped.y - first.placement.y;
        let (sin, cos) = first.placement.rotation_radians.sin_cos();
        let local_x = dx * cos + dy * sin;
        let local_y = -dx * sin + dy * cos;
        if !local_x.is_finite() || !local_y.is_finite() {
            return false;
        }
        let new_location = match endpoint.location {
            ConnectionEndpoint::Fluid { .. } => ConnectionEndpoint::Fluid {
                x: local_x,
                y: local_y,
            },
            ConnectionEndpoint::Boundary { .. } => {
                // Keep the directional endpoint representation only when its
                // ray still resolves to this exact retained boundary point.
                let angle = local_y.atan2(local_x);
                let Some(point) = crate::surface_geometry::boundary_point_toward(
                    merged_shape,
                    angle.cos(),
                    angle.sin(),
                ) else {
                    return false;
                };
                if (point.x - local_x).hypot(point.y - local_y) > MAP_TOLERANCE {
                    return false;
                }
                ConnectionEndpoint::Boundary {
                    angle_radians: angle,
                }
            }
            ConnectionEndpoint::Corner { .. } | ConnectionEndpoint::LineEndpoint { .. } => {
                return false;
            }
        };
        endpoint.constituent_id = first_id;
        endpoint.location = new_location;
        true
    }

    // A fluid-fluid bond must exist and both live endpoints must lie on the
    // exact shared SolidContact interface selected by the geometry union. Bond
    // existence alone must never authorize merging across a different surface.
    fn point_lies_on_shared_contact(
        first: &MultiContactFluidBoundary,
        second: &MultiContactFluidBoundary,
        x: f64,
        y: f64,
    ) -> bool {
        const TOLERANCE: f64 = 1e-7;
        let distance_to_segment = |point: (f64, f64), start: (f64, f64), end: (f64, f64)| {
            let dx = end.0 - start.0;
            let dy = end.1 - start.1;
            let length_squared = dx * dx + dy * dy;
            if !length_squared.is_finite() || length_squared <= f64::EPSILON {
                return f64::INFINITY;
            }
            let t = (((point.0 - start.0) * dx + (point.1 - start.1) * dy) / length_squared)
                .clamp(0.0, 1.0);
            let closest = (start.0 + t * dx, start.1 + t * dy);
            (point.0 - closest.0).hypot(point.1 - closest.1)
        };
        for first_piece in &first.pieces {
            let crate::capillary_geometry::FluidBoundaryPiece::SolidContact {
                start_x: first_sx,
                start_y: first_sy,
                end_x: first_ex,
                end_y: first_ey,
                ..
            } = first_piece
            else {
                continue;
            };
            for second_piece in &second.pieces {
                let crate::capillary_geometry::FluidBoundaryPiece::SolidContact {
                    start_x: second_sx,
                    start_y: second_sy,
                    end_x: second_ex,
                    end_y: second_ey,
                    ..
                } = second_piece
                else {
                    continue;
                };
                let reversed_shared_segment = (first_sx - second_ex).hypot(first_sy - second_ey)
                    <= TOLERANCE
                    && (first_ex - second_sx).hypot(first_ey - second_sy) <= TOLERANCE;
                if reversed_shared_segment
                    && distance_to_segment((x, y), (*first_sx, *first_sy), (*first_ex, *first_ey))
                        <= TOLERANCE
                {
                    return true;
                }
            }
        }
        false
    }

    let internal_bonds: Vec<_> = structure
        .bonds
        .iter()
        .filter(|bond| {
            (bond.endpoint_a.constituent_id == first_id
                && bond.endpoint_b.constituent_id == second_id)
                || (bond.endpoint_a.constituent_id == second_id
                    && bond.endpoint_b.constituent_id == first_id)
        })
        .collect();
    if internal_bonds.is_empty() {
        return false;
    }
    for bond in internal_bonds {
        let world_point_for = |endpoint: BondEndpoint| {
            let source = if endpoint.constituent_id == first_id {
                &first
            } else if endpoint.constituent_id == second_id {
                &second
            } else {
                return None;
            };
            endpoint.location.world_point(source, catalog)
        };
        let Some(point_a) = world_point_for(bond.endpoint_a) else {
            return false;
        };
        let Some(point_b) = world_point_for(bond.endpoint_b) else {
            return false;
        };
        if (point_a.x - point_b.x).hypot(point_a.y - point_b.y) > COMBINE_CONTACT_TOLERANCE {
            return false;
        }
        for point in [point_a, point_b] {
            if !point_lies_on_shared_contact(
                &placed.first_world,
                &placed.second_world,
                point.x,
                point.y,
            ) {
                return false;
            }
        }
    }

    let mut retained_bonds = Vec::with_capacity(structure.bonds.len());
    for mut bond in structure.bonds.iter().copied() {
        let joins_pair = (bond.endpoint_a.constituent_id == first_id
            && bond.endpoint_b.constituent_id == second_id)
            || (bond.endpoint_a.constituent_id == second_id
                && bond.endpoint_b.constituent_id == first_id);
        if joins_pair {
            // All bonds internal to the newly unified fluid body are consumed.
            continue;
        }
        if !remap_endpoint(
            &mut bond.endpoint_a,
            first_id,
            second_id,
            &first,
            &second,
            &placed,
            &merged_shape,
            catalog,
        ) || !remap_endpoint(
            &mut bond.endpoint_b,
            first_id,
            second_id,
            &first,
            &second,
            &placed,
            &merged_shape,
            catalog,
        ) {
            return false;
        }
        retained_bonds.push(bond);
    }
    for i in 0..retained_bonds.len() {
        if retained_bonds[i + 1..]
            .iter()
            .any(|other| retained_bonds[i].has_same_identity(other))
        {
            return false;
        }
    }

    let mut survivor = first;
    survivor.material.parts[0].1 = combined_amount;
    if !survivor.realize_fluid_geometry(merged_shape, catalog) {
        return false;
    }

    let genome_ids: Vec<_> = structure
        .genome_constituent_ids()
        .iter()
        .map(|id| if *id == second_id { first_id } else { *id })
        .collect();
    let mut staged = structure.clone();
    staged.units[first_index] = survivor;
    staged.bonds = retained_bonds;
    staged.remove_units_by_physical_ids(&[second_id]);
    staged.set_genome_constituent_ids(genome_ids);
    if staged
        .bonds
        .iter()
        .any(|bond| !staged.is_valid_bond(bond, catalog))
    {
        return false;
    }
    *structure = staged;
    true
}

fn combine_pair_in_direction(
    structure: &mut crate::structure::OrganismStructure,
    unit_a: usize,
    unit_b: usize,
    catalog: &[BaseResource],
    cache: &mut ConnectionCompatibilityCache,
    ledger: &mut EnergyLedger,
    energy: &mut f64,
) -> Option<CombineAttempt> {
    let mut candidates = eligible_candidates(structure, unit_a, unit_b, catalog, cache)
        .into_iter()
        .filter_map(|candidate| {
            let (evaluation, _, _, required) =
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
            if !coalesce_fluid_pair_after_bond(&mut trial_structure, unit_a, unit_b, catalog) {
                continue;
            }
            *structure = trial_structure;
            *ledger = trial_ledger;
            *energy = trial_energy;
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

    // Bond direction is a biological choice. Try the requested orientation first,
    // then let the same physical pair form in the reverse orientation if the
    // first direction cannot produce a valid interaction.
    combine_pair_in_direction(structure, unit_a, unit_b, catalog, cache, ledger, energy).or_else(
        || combine_pair_in_direction(structure, unit_b, unit_a, catalog, cache, ledger, energy),
    )
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

    let candidate_cells = environment
        .field
        .cells_intersecting_bounds(body.min_x, body.max_x, body.min_y, body.max_y);
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
            if indices.iter().any(|index| {
                crate::construction_runtime::placed_unit_overlaps(
                    &hypothetical,
                    &hypothetical.units[*index],
                    &indices,
                    &environment.catalog,
                )
            }) {
                continue;
            }
            for ua in 0..organism.structure.units.len() {
                for &ub in &indices {
                    for candidate in crate::contact::connection_pair_candidates_cached(
                        &hypothetical,
                        ua,
                        ub,
                        &environment.catalog,
                        cache,
                    ) {
                        let Some(a) = candidate
                            .endpoint_a
                            .world_point(&hypothetical.units[ua], &environment.catalog)
                        else {
                            continue;
                        };
                        // The organism-side connection point is the relevant
                        // access point. A contact midpoint is not sufficient: it
                        // could be inside while the actual organism connection
                        // point is outside the accessible interior.
                        if !regions.iter().any(|region| region.contains_point(a.x, a.y)) {
                            continue;
                        }
                        if let Some((evaluation, _, _, required)) = evaluate_candidate(
                            &hypothetical,
                            ua,
                            ub,
                            candidate,
                            &environment.catalog,
                        ) {
                            let developmental_score = developmental
                                .map(|(blueprint, origin, orientation, preferred_length)| {
                                    let Some(a) = candidate
                                        .endpoint_a
                                        .world_point(&hypothetical.units[ua], &environment.catalog)
                                    else {
                                        return 0.0;
                                    };
                                    let local = crate::developmental_blueprint::developmental_point(
                                        a.x,
                                        a.y,
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
        b.7.partial_cmp(&a.7)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.5.partial_cmp(&b.5).unwrap_or(std::cmp::Ordering::Equal))
    });

    for (cell_index, material_index, ua, ub, evaluation, _distance, required, _score) in candidates
    {
        if organism.usable_energy + EPSILON < required {
            continue;
        }
        let instance = environment
            .field
            .cells
            .get(cell_index)
            .and_then(|cell| cell.physical_materials.get(material_index))
            .cloned();
        let Some(instance) = instance else {
            continue;
        };
        let mut trial_structure = organism.structure.clone();
        let Some(indices) = crate::material_restoration::restore_material(
            &mut trial_structure,
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
        let Some(restored_ub) = indices
            .get(ub.saturating_sub(organism.structure.units.len()))
            .copied()
        else {
            continue;
        };
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
        if !coalesce_fluid_pair_after_bond(
            &mut trial_structure,
            ua,
            restored_ub,
            &environment.catalog,
        ) {
            continue;
        }

        let Some(cell) = environment.field.cells.get_mut(cell_index) else {
            continue;
        };
        let Some(removed) = cell.physical_materials.get(material_index).cloned() else {
            continue;
        };
        if removed != instance {
            continue;
        }
        environment.field.cells[cell_index]
            .physical_materials
            .remove(material_index);
        environment.field.mark_changed_at_index(cell_index);
        let mut attempt = attempt;
        attempt.environmental_source = Some((cell_index, removed));
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
                if let Some((evaluation, _, _, required)) =
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
            if !coalesce_fluid_pair_after_bond(&mut trial_structure, ua, ub, catalog) {
                continue;
            }
            organism.structure = trial_structure;
            organism.usable_energy = trial_energy;
            *ledger = trial_ledger;
            organism.add_transaction_stress(attempt.work_cost);
            return Some(attempt);
        }
    }
    None
}

#[cfg(test)]
mod fluid_coalescence_transaction_tests {
    use super::*;
    use crate::capillary_geometry::{
        CircularArcBoundary, FluidBoundaryPiece, MultiContactFluidBoundary,
    };
    use crate::resources::{default_catalog, Form, Material, Shape};
    use crate::structure::{
        Bond, BondEndpoint, ConnectionEndpoint, OrganismStructure, Placement, StructuralUnit,
    };
    use std::f64::consts::PI;

    fn semicircle_shape(radius: f64, upper: bool) -> Shape {
        let (start_angle, chord_start, chord_end) = if upper {
            (0.0, (-radius, 0.0), (radius, 0.0))
        } else {
            (PI, (radius, 0.0), (-radius, 0.0))
        };
        let boundary = MultiContactFluidBoundary {
            nominal_area: PI * radius * radius * 0.5,
            pieces: vec![
                FluidBoundaryPiece::FreeArc(CircularArcBoundary {
                    center_x: 0.0,
                    center_y: 0.0,
                    radius,
                    start_angle_radians: start_angle,
                    sweep_radians: PI,
                }),
                FluidBoundaryPiece::SolidContact {
                    start_x: chord_start.0,
                    start_y: chord_start.1,
                    end_x: chord_end.0,
                    end_y: chord_end.1,
                    contact_angle_radians: PI * 0.5,
                },
            ],
        };
        Shape {
            form: Form::Fluid {
                nominal_area: boundary.nominal_area,
                boundary: None,
                arc_boundary: None,
                multi_contact_boundary: Some(boundary),
            },
        }
    }

    fn build_bonded_pair(
        second_x: f64,
        with_rigid_contact: bool,
    ) -> (OrganismStructure, Vec<BaseResource>, f64) {
        let catalog = default_catalog();
        let water = catalog
            .iter()
            .find(|resource| resource.name == "Water")
            .unwrap();
        let unit_area = match &water.shape.form {
            Form::Circle { radius } => PI * radius * radius,
            Form::Fluid { nominal_area, .. } => *nominal_area,
            _ => panic!("Water test fixture requires a measurable fluid area"),
        };
        let radius = (2.0 * unit_area / PI).sqrt();
        let placement_a = Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        };
        let placement_b = Placement {
            x: second_x,
            y: 0.0,
            rotation_radians: 0.0,
        };
        let mut upper =
            StructuralUnit::from_material(Material::free_base("Water", 1.0), placement_a).unwrap();
        let mut lower =
            StructuralUnit::from_material(Material::free_base("Water", 1.0), placement_b).unwrap();
        assert!(upper.realize_fluid_geometry(semicircle_shape(radius, true), &catalog));
        assert!(lower.realize_fluid_geometry(semicircle_shape(radius, false), &catalog));

        let mut structure = OrganismStructure::new();
        structure.add_unit(upper);
        structure.add_unit(lower);
        let first_id = structure.physical_id(0).unwrap();
        let second_id = structure.physical_id(1).unwrap();
        structure.set_genome_constituent_ids([first_id, second_id]);
        structure.bonds.push(Bond {
            endpoint_a: BondEndpoint::new(first_id, ConnectionEndpoint::Fluid { x: 0.0, y: 0.0 }),
            endpoint_b: BondEndpoint::new(second_id, ConnectionEndpoint::Fluid { x: 0.0, y: 0.0 }),
            strength: 0.5,
            bond_energy: 0.0,
        });

        if with_rigid_contact {
            let carbon = catalog
                .iter()
                .find(|resource| resource.name == "Carbon")
                .unwrap();
            let vertices = carbon.shape.form.polygon_vertices().unwrap();
            let (vertex_index, (vx, vy)) = vertices
                .iter()
                .enumerate()
                .min_by(|(_, left), (_, right)| left.1.total_cmp(&right.1))
                .map(|(index, vertex)| (index, *vertex))
                .unwrap();
            let rotation = -PI * 0.5 - vy.atan2(vx);
            let carbon_radius = vx.hypot(vy);
            let carbon_placement = Placement {
                x: 0.0,
                y: radius + carbon_radius,
                rotation_radians: rotation,
            };
            let mut rigid = StructuralUnit::new("Carbon", carbon_placement);
            assert!(rigid.realize_default_geometry(&catalog));
            structure.add_unit(rigid);
            let rigid_id = structure.physical_id(2).unwrap();
            structure.bonds.push(Bond {
                endpoint_a: BondEndpoint::new(
                    first_id,
                    ConnectionEndpoint::Boundary {
                        angle_radians: PI * 0.5,
                    },
                ),
                endpoint_b: BondEndpoint::new(
                    rigid_id,
                    ConnectionEndpoint::Corner {
                        point_index: vertex_index,
                    },
                ),
                strength: 0.5,
                bond_energy: 0.0,
            });
        }
        (structure, catalog, unit_area)
    }

    #[test]
    fn successful_live_combine_reconciles_independent_world_energy_inventory() {
        let (structure, catalog, _) = build_bonded_pair(0.0, false);
        let mut structure = structure;
        structure.bonds.clear();

        let mut simulation = crate::state::Simulation::new(0xC0B1, 10.0);
        simulation.organisms[0].structure = structure;
        // The test replaces the constructor's initial structure with a bond-free
        // fixture. Reset only the corresponding structural-energy baseline so the
        // independent inventory starts from the fixture's actual state.
        simulation.energy_ledger.initial_structural_energy_held = 0.0;
        simulation.energy_ledger.total_structural_energy_held = 0.0;

        let before = simulation.energy_inventory_snapshot();
        assert!(
            before.is_balanced_without_creation(1e-9),
            "fixture must start with a balanced world-energy inventory: {:?}",
            before.residual()
        );

        let mut cache = ConnectionCompatibilityCache;
        let organism = &mut simulation.organisms[0];
        let attempt = combine_specific_pair(
            &mut organism.structure,
            0,
            1,
            &catalog,
            &mut cache,
            &mut simulation.energy_ledger,
            &mut organism.usable_energy,
        )
        .expect("the touching fluid pair should COMBINE");

        assert!(attempt.work_cost.is_finite() && attempt.work_cost >= 0.0);
        let after = simulation.energy_inventory_snapshot();
        assert!(
            after.is_balanced_without_creation(1e-9),
            "successful COMBINE must reconcile the independent world-energy inventory: residual={:?}, inventory={after:?}",
            after.residual()
        );
        assert_eq!(simulation.organisms[0].structure.units.len(), 1);
    }

    #[test]
    fn live_combine_coalesces_fluid_pair_atomically() {
        let (mut structure, catalog, unit_area) = build_bonded_pair(0.0, false);
        structure.bonds.clear();
        let survivor_id = structure.physical_id(0).unwrap();
        let mut cache = ConnectionCompatibilityCache;
        let mut ledger = EnergyLedger::default();
        let mut energy = 100.0;

        let attempt = combine_specific_pair(
            &mut structure,
            0,
            1,
            &catalog,
            &mut cache,
            &mut ledger,
            &mut energy,
        );
        assert!(attempt.is_some(), "supported fluid pair should COMBINE");
        assert_eq!(structure.units.len(), 1);
        assert_eq!(structure.physical_id(0), Some(survivor_id));
        assert_eq!(structure.units[0].material.parts[0].0, "Water");
        assert!((structure.units[0].material.parts[0].1 - 2.0).abs() < 1e-9);
        assert_eq!(structure.genome_constituent_ids(), &[survivor_id]);
        assert!(
            structure.bonds.is_empty(),
            "consumed fluid-fluid bond must not persist"
        );
        let shape = structure.units[0].shape(&catalog).unwrap();
        let Form::Fluid {
            nominal_area,
            multi_contact_boundary: Some(boundary),
            ..
        } = &shape.form
        else {
            panic!("live COMBINE must install the merged exact fluid boundary");
        };
        assert!((*nominal_area - 2.0 * unit_area).abs() < 1e-9);
        assert!(boundary.is_valid());
        assert!(energy.is_finite() && energy >= 0.0);
        let residual = energy - 100.0
            + ledger.total_structural_energy_delta
            + ledger.total_work_expended
            + ledger.total_heat_dissipated
            - ledger.total_external_energy_input
            - ledger.total_potential_energy_released;
        assert!(
            residual.abs() <= 1e-9,
            "live fluid COMBINE must reconcile energy; residual={residual}, ledger={ledger:?}"
        );
    }

    #[test]
    fn coalesced_fluid_boundary_survives_physical_material_roundtrip_and_restore() {
        // Coalescence is triggered by a successful fluid-fluid bond, not by touch.
        // The transaction consumes this internal bond while preserving the merged shape.
        let (mut structure, catalog, unit_area) = build_bonded_pair(0.0, false);
        assert!(coalesce_fluid_pair_after_bond(
            &mut structure,
            0,
            1,
            &catalog
        ));

        let shape = structure.units[0].shape(&catalog).unwrap();
        let Form::Fluid {
            nominal_area,
            multi_contact_boundary: Some(boundary),
            ..
        } = &shape.form
        else {
            panic!("coalesced fluid must retain its exact multi-contact boundary");
        };
        assert!((nominal_area - 2.0 * unit_area).abs() < 1e-9);
        assert!(boundary.is_valid());

        let origin = Placement {
            x: 0.0,
            y: 0.0,
            rotation_radians: 0.0,
        };
        let stored = crate::physical_material::PhysicalMaterial::realized_with_single_fluid_shape(
            Material::free_base("Water", 2.0),
            origin,
            shape.clone(),
            &catalog,
        )
        .expect("merged shape should be accepted as a realized single-fluid material");
        let encoded = serde_json::to_string(&stored).unwrap();
        let decoded: crate::physical_material::PhysicalMaterial =
            serde_json::from_str(&encoded).unwrap();

        let mut restored = OrganismStructure::new();
        assert_eq!(
            crate::material_restoration::restore_material(
                &mut restored,
                &decoded,
                origin,
                &catalog,
            ),
            Some(vec![0])
        );
        assert_eq!(restored.units[0].shape(&catalog), Some(shape));
        assert_eq!(restored.units[0].material.parts[0].1, 2.0);
        assert!(restored.units[0].shape(&catalog).unwrap().is_valid());
    }

    #[test]
    fn bonded_fluid_pair_merges_area_and_preserves_rigid_bond() {
        let (mut structure, catalog, unit_area) = build_bonded_pair(0.0, true);
        let survivor_id = structure.physical_id(0).unwrap();
        let rigid_id = structure.physical_id(2).unwrap();

        assert!(coalesce_fluid_pair_after_bond(
            &mut structure,
            0,
            1,
            &catalog
        ));
        assert_eq!(structure.units.len(), 2);
        assert_eq!(structure.units[0].physical_id, survivor_id);
        assert_eq!(structure.units[0].material.parts[0].0, "Water");
        assert!((structure.units[0].material.parts[0].1 - 2.0).abs() < 1e-9);
        assert_eq!(structure.genome_constituent_ids(), &[survivor_id]);
        assert_eq!(structure.bonds.len(), 1);
        let retained = structure.bonds[0];
        assert!(
            retained.endpoint_a.constituent_id == survivor_id
                || retained.endpoint_b.constituent_id == survivor_id
        );
        assert!(
            retained.endpoint_a.constituent_id == rigid_id
                || retained.endpoint_b.constituent_id == rigid_id
        );
        assert!(structure.is_valid_bond(&retained, &catalog));

        let merged_shape = structure.units[0].shape(&catalog).unwrap();
        let Form::Fluid {
            nominal_area,
            multi_contact_boundary: Some(boundary),
            ..
        } = &merged_shape.form
        else {
            panic!("merged water must retain its exact multi-contact boundary");
        };
        assert!((*nominal_area - 2.0 * unit_area).abs() < 1e-9);
        assert!(boundary.is_valid());
        assert_eq!(boundary.pieces.len(), 2);
        assert!(boundary
            .pieces
            .iter()
            .all(|piece| matches!(piece, FluidBoundaryPiece::FreeArc(_))));
    }

    #[test]
    fn coalescence_remaps_rigid_bonds_from_both_fluid_constituents() {
        let (mut structure, catalog, _) = build_bonded_pair(0.0, true);
        let second_fluid_id = structure.physical_id(1).unwrap();

        let carbon = catalog
            .iter()
            .find(|resource| resource.name == "Carbon")
            .unwrap();
        let vertices = carbon.shape.form.polygon_vertices().unwrap();
        let (vertex_index, (vx, vy)) = vertices
            .iter()
            .enumerate()
            .max_by(|(_, left), (_, right)| left.1.total_cmp(&right.1))
            .map(|(index, vertex)| (index, *vertex))
            .unwrap();
        let carbon_radius = vx.hypot(vy);
        let water = catalog
            .iter()
            .find(|resource| resource.name == "Water")
            .unwrap();
        let fluid_radius = match &water.shape.form {
            Form::Circle { radius } => *radius,
            Form::Fluid { nominal_area, .. } => (*nominal_area / PI).sqrt(),
            _ => panic!("Water test fixture requires a measurable fluid area"),
        };
        let carbon_placement = Placement {
            x: 0.0,
            y: -fluid_radius - carbon_radius,
            rotation_radians: PI * 0.5 - vy.atan2(vx),
        };
        let mut lower_rigid = StructuralUnit::new("Carbon", carbon_placement);
        assert!(lower_rigid.realize_default_geometry(&catalog));
        structure.add_unit(lower_rigid);
        let lower_rigid_id = structure.physical_id(3).unwrap();
        structure.bonds.push(Bond {
            endpoint_a: BondEndpoint::new(
                second_fluid_id,
                ConnectionEndpoint::Boundary {
                    angle_radians: -PI * 0.5,
                },
            ),
            endpoint_b: BondEndpoint::new(
                lower_rigid_id,
                ConnectionEndpoint::Corner {
                    point_index: vertex_index,
                },
            ),
            strength: 0.75,
            bond_energy: 0.0,
        });
        assert!(structure.is_valid_bond(structure.bonds.last().unwrap(), &catalog));

        assert!(coalesce_fluid_pair_after_bond(
            &mut structure,
            0,
            1,
            &catalog
        ));
        let survivor_id = structure.physical_id(0).unwrap();
        assert_eq!(structure.units.len(), 3);
        assert_eq!(structure.bonds.len(), 2);
        for retained in &structure.bonds {
            assert!(structure.is_valid_bond(retained, &catalog));
            let fluid_endpoints = [retained.endpoint_a, retained.endpoint_b]
                .into_iter()
                .filter(|endpoint| endpoint.constituent_id == survivor_id)
                .count();
            assert_eq!(
                fluid_endpoints, 1,
                "each retained rigid bond must attach once to the merged fluid"
            );
        }
        assert!(structure.bonds.iter().any(|bond| {
            bond.endpoint_a.constituent_id == lower_rigid_id
                || bond.endpoint_b.constituent_id == lower_rigid_id
        }));
        assert!(structure.bonds.iter().all(|bond| {
            bond.endpoint_a.constituent_id != second_fluid_id
                && bond.endpoint_b.constituent_id != second_fluid_id
        }));
    }

    #[test]
    fn fluid_bond_off_shared_interface_rejects_coalescence_atomically() {
        let (mut structure, catalog, unit_area) = build_bonded_pair(0.0, false);
        let radius = (2.0 * unit_area / PI).sqrt();
        structure.bonds[0].endpoint_a.location = ConnectionEndpoint::Fluid { x: 0.0, y: radius };
        let original_units = structure.units.clone();
        let original_bonds = structure.bonds.clone();
        let original_genome_ids = structure.genome_constituent_ids().to_vec();

        assert!(
            !coalesce_fluid_pair_after_bond(&mut structure, 0, 1, &catalog),
            "a bond away from the shared solid-contact interface must not authorize coalescence"
        );
        assert_eq!(structure.units, original_units);
        assert_eq!(structure.bonds, original_bonds);
        assert_eq!(
            structure.genome_constituent_ids(),
            original_genome_ids.as_slice()
        );
    }

    #[test]
    fn separated_bond_endpoints_on_shared_segment_reject_coalescence_atomically() {
        let (mut structure, catalog, unit_area) = build_bonded_pair(0.0, false);
        let radius = (2.0 * unit_area / PI).sqrt();
        structure.bonds[0].endpoint_a.location = ConnectionEndpoint::Fluid {
            x: -0.4 * radius,
            y: 0.0,
        };
        structure.bonds[0].endpoint_b.location = ConnectionEndpoint::Fluid {
            x: 0.4 * radius,
            y: 0.0,
        };
        let original_units = structure.units.clone();
        let original_bonds = structure.bonds.clone();
        let original_genome_ids = structure.genome_constituent_ids().to_vec();

        assert!(
            !coalesce_fluid_pair_after_bond(&mut structure, 0, 1, &catalog),
            "bond endpoints separated along the shared segment must not authorize coalescence"
        );
        assert_eq!(structure.units, original_units);
        assert_eq!(structure.bonds, original_bonds);
        assert_eq!(
            structure.genome_constituent_ids(),
            original_genome_ids.as_slice()
        );
    }

    #[test]
    fn unsupported_fluid_corner_and_line_endpoints_reject_coalescence_atomically() {
        for unsupported in [
            ConnectionEndpoint::Corner { point_index: 0 },
            ConnectionEndpoint::LineEndpoint { point_index: 0 },
        ] {
            let (mut structure, catalog, _) = build_bonded_pair(0.0, true);
            let first_id = structure.physical_id(0).unwrap();
            let rigid_id = structure.physical_id(2).unwrap();
            let bond = structure
                .bonds
                .iter_mut()
                .find(|bond| {
                    (bond.endpoint_a.constituent_id == first_id
                        && bond.endpoint_b.constituent_id == rigid_id)
                        || (bond.endpoint_b.constituent_id == first_id
                            && bond.endpoint_a.constituent_id == rigid_id)
                })
                .expect("fixture must contain a fluid-to-rigid bond");
            if bond.endpoint_a.constituent_id == first_id {
                bond.endpoint_a.location = unsupported;
            } else {
                bond.endpoint_b.location = unsupported;
            }

            let original_units = structure.units.clone();
            let original_bonds = structure.bonds.clone();
            let original_genome_ids = structure.genome_constituent_ids().to_vec();
            assert!(
                !coalesce_fluid_pair_after_bond(&mut structure, 0, 1, &catalog),
                "unsupported fluid endpoint representation must fail closed"
            );
            assert_eq!(structure.units, original_units);
            assert_eq!(structure.bonds, original_bonds);
            assert_eq!(
                structure.genome_constituent_ids(),
                original_genome_ids.as_slice()
            );
        }
    }

    #[test]
    fn consumed_fluid_bond_potential_is_heat_not_persistent_structure() {
        let fluid_tx = combine_energy_transaction(2.0, 3.0, 4.0, true).unwrap();
        let rigid_tx = combine_energy_transaction(2.0, 3.0, 4.0, false).unwrap();

        assert!(fluid_tx.balanced());
        assert!(rigid_tx.balanced());
        assert_eq!(fluid_tx.usable_delta, -9.0);
        assert_eq!(rigid_tx.usable_delta, -9.0);
        assert_eq!(fluid_tx.structural_delta, 0.0);
        assert_eq!(fluid_tx.work_expended, 4.0);
        assert_eq!(fluid_tx.heat_dissipated, 5.0);
        assert_eq!(rigid_tx.structural_delta, 3.0);
        assert_eq!(rigid_tx.work_expended, 4.0);
        assert_eq!(rigid_tx.heat_dissipated, 2.0);
    }

    #[test]
    fn touching_fluid_boundaries_without_a_bond_do_not_coalesce() {
        let (mut structure, catalog, _) = build_bonded_pair(0.0, false);
        structure.bonds.clear();
        let original_units = structure.units.clone();
        let original_genome_ids = structure.genome_constituent_ids().to_vec();

        assert!(!coalesce_fluid_pair_after_bond(
            &mut structure,
            0,
            1,
            &catalog
        ));
        assert_eq!(structure.units, original_units);
        assert!(structure.bonds.is_empty());
        assert_eq!(
            structure.genome_constituent_ids(),
            original_genome_ids.as_slice()
        );
    }

    #[test]
    fn failed_live_combine_rolls_back_energy_and_ledger_when_union_is_unsupported() {
        // A small relative rotation keeps the physical candidate within
        // COMBINE tolerance while making the two chord interfaces non-identical.
        // Candidate formation should succeed; exact union must then reject.
        let (mut structure, catalog, _) = build_bonded_pair(0.0, false);
        structure.units[1].placement.rotation_radians = 0.01;
        structure.bonds.clear();
        let mut cache = ConnectionCompatibilityCache;
        let candidates = eligible_candidates(&structure, 0, 1, &catalog, &mut cache);
        assert!(!candidates.is_empty());
        assert!(candidates
            .iter()
            .copied()
            .any(|candidate| evaluate_candidate(&structure, 0, 1, candidate, &catalog).is_some()));

        let original_units = structure.units.clone();
        let original_bonds = structure.bonds.clone();
        let original_genome_ids = structure.genome_constituent_ids().to_vec();
        let mut ledger = EnergyLedger::default();
        let original_ledger = ledger;
        let mut energy = 100.0;

        assert!(combine_specific_pair(
            &mut structure,
            0,
            1,
            &catalog,
            &mut cache,
            &mut ledger,
            &mut energy,
        )
        .is_none());
        assert_eq!(structure.units, original_units);
        assert_eq!(structure.bonds, original_bonds);
        assert_eq!(
            structure.genome_constituent_ids(),
            original_genome_ids.as_slice()
        );
        assert_eq!(energy, 100.0);
        assert_eq!(
            ledger.total_potential_energy_released,
            original_ledger.total_potential_energy_released
        );
        assert_eq!(
            ledger.total_usable_energy_gained,
            original_ledger.total_usable_energy_gained
        );
        assert_eq!(
            ledger.total_work_expended,
            original_ledger.total_work_expended
        );
        assert_eq!(
            ledger.total_heat_dissipated,
            original_ledger.total_heat_dissipated
        );
    }

    #[test]
    fn unsupported_fluid_union_rolls_back_structure_and_bond() {
        let (mut structure, catalog, _) = build_bonded_pair(4.0, false);
        let original_units = structure.units.clone();
        let original_bonds = structure.bonds.clone();
        let original_genome_ids = structure.genome_constituent_ids().to_vec();

        assert!(!coalesce_fluid_pair_after_bond(
            &mut structure,
            0,
            1,
            &catalog
        ));
        assert_eq!(structure.units, original_units);
        assert_eq!(structure.bonds, original_bonds);
        assert_eq!(
            structure.genome_constituent_ids(),
            original_genome_ids.as_slice()
        );
    }
}
