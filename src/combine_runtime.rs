//! Runtime COMBINE boundary.
use crate::combine::FormationEvaluation;
use crate::resources::BaseResource;
use crate::state::{Environment, Organism};
use crate::structure::{Bond, ConnectionEndpoint, OrganismStructure, Placement, StructuralUnit};
const EPSILON: f64 = 1e-12;
const COMBINE_CONTACT_TOLERANCE: f64 = 1.0;
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CombineAttempt {
    pub unit_a: usize,
    pub unit_b: usize,
    pub point_a: ConnectionEndpoint,
    pub point_b: ConnectionEndpoint,
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
fn energy_requirements(investment: f64, work: f64, interaction: f64) -> Option<f64> {
    if !investment.is_finite() || investment < 0.0 || !work.is_finite() || work < 0.0 || !interaction.is_finite() { return None; }
    let required = investment + work - interaction;
    required.is_finite().then_some(required.max(0.0))
}
fn settle_energy(energy: &mut f64, investment: f64, work: f64, interaction: f64) -> Option<f64> {
    let required = energy_requirements(investment, work, interaction)?;
    if *energy + EPSILON < required { return None; }
    let net = interaction - investment - work;
    let next = *energy + net;
    if !net.is_finite() || !next.is_finite() || next < -EPSILON { return None; }
    *energy = next.max(0.0);
    Some(net)
}
fn candidate_is_fully_feasible(structure: &OrganismStructure, ua: usize, ub: usize, candidate: crate::contact::ConnectionPairCandidate, catalog: &[BaseResource], energy: f64) -> Option<FormationEvaluation> {
    if candidate.distance > COMBINE_CONTACT_TOLERANCE || !candidate.available_a || !candidate.available_b { return None; }
    let a = structure.units.get(ua)?.properties(catalog)?;
    let b = structure.units.get(ub)?.properties(catalog)?;
    let evaluation = crate::combine::evaluate_formation(candidate, a.cohesion, b.cohesion);
    if !evaluation.threshold.is_finite() || evaluation.threshold <= 0.0 { return None; }
    let interaction = crate::combine::experimental_interaction(a, b, candidate, 0.0);
    let work = crate::combine::experimental_combine_work_cost(a, b, candidate, 0.0);
    let required = energy_requirements(evaluation.threshold, work, interaction.signed_value)?;
    if energy + EPSILON < required { return None; }
    Some(evaluation)
}
/// The sole runtime bond-formation primitive. Candidate geometry and formation
/// physics are validated before the structure or energy ledger is mutated.
#[allow(clippy::too_many_arguments)]
pub(crate) fn form_bond(structure: &mut OrganismStructure, ua: usize, point_a: ConnectionEndpoint, ub: usize, point_b: ConnectionEndpoint, catalog: &[BaseResource], _cache: &mut crate::contact::ConnectionCompatibilityCache, investment: f64, water_dilution: f64, energy: &mut f64) -> Option<CombineAttempt> {
    if ua >= structure.units.len() || ub >= structure.units.len() || ua == ub { return None; }
    let candidate = crate::contact::connection_pair_candidates(structure, ua, ub, catalog).into_iter().find(|c| c.point_a == point_a && c.point_b == point_b && c.distance <= COMBINE_CONTACT_TOLERANCE && c.available_a && c.available_b)?;
    let a = structure.units[ua].properties(catalog)?;
    let b = structure.units[ub].properties(catalog)?;
    let evaluation = crate::combine::evaluate_formation(candidate, a.cohesion, b.cohesion);
    if !crate::combine::formation_succeeds(evaluation, investment) || !investment.is_finite() || investment <= 0.0 { return None; }
    let interaction = crate::combine::experimental_interaction(a, b, candidate, water_dilution);
    let work = crate::combine::experimental_combine_work_cost(a, b, candidate, water_dilution);
    if !work.is_finite() || work < 0.0 || !evaluation.threshold.is_finite() || evaluation.threshold <= 0.0 { return None; }
    let bs = crate::combine::bond_strength(a, b);
    if !bs.is_finite() { return None; }
    let mut candidate_structure = structure.clone();
    crate::contact::try_add_bond(&mut candidate_structure, Bond { unit_a: ua, point_a, unit_b: ub, point_b, strength: bs, bond_energy: investment }, catalog).ok()?;
    let mut next_energy = *energy;
    let net = settle_energy(&mut next_energy, investment, work, interaction.signed_value)?;
    *structure = candidate_structure;
    *energy = next_energy;
    Some(CombineAttempt { unit_a: ua, unit_b: ub, point_a, point_b, work_cost: work, energy_invested: investment, interaction_direction: interaction.direction, interaction_magnitude: interaction.magnitude, interaction_energy: interaction.signed_value, formation_threshold: evaluation.threshold, net_energy_change: net, bond_strength: bs, bond_energy: investment })
}
pub(crate) fn try_combine_stored_unit(organism: &mut Organism, environment: &Environment, cache: &mut crate::contact::ConnectionCompatibilityCache) -> Option<CombineAttempt> {
    let raw = organism.stored_material.peek_one_unstructured()?;
    if raw.parts.len() != 1 || !raw.internal_bonds.is_empty() || (raw.parts[0].1 - 1.0).abs() > EPSILON || environment.catalog.iter().all(|b| b.name != raw.parts[0].0) { return None; }
    let resource_name = raw.parts[0].0.clone();
    let new_sites = match environment.catalog.iter().find(|b| b.name == resource_name).map(|b| b.shape.connection_sites()) { Some(crate::resources::ConnectionSites::Corners(p)) => p, _ => return None };
    let mut best: Option<(usize, usize, Placement, usize, f64)> = None;
    for ua in 0..organism.structure.units.len() {
        let Some(crate::resources::ConnectionSites::Corners(existing)) = organism.structure.units[ua].connection_sites(&environment.catalog) else { continue; };
        for (pa, &ep) in existing.iter().enumerate() {
            for (point_b, &np) in new_sites.iter().enumerate() {
                let placement = placement_for_connection(&organism.structure.units[ua], ep, np);
                let mut hypothetical = organism.structure.clone();
                let ub = hypothetical.add_unit(StructuralUnit::from_material(raw.clone(), placement)?);
                let wanted_a = ConnectionEndpoint::Discrete(pa);
                let wanted_b = ConnectionEndpoint::Discrete(point_b);
                let Some(candidate) = crate::contact::connection_pair_candidates(&hypothetical, ua, ub, &environment.catalog).into_iter().find(|c| c.point_a == wanted_a && c.point_b == wanted_b && c.distance <= COMBINE_CONTACT_TOLERANCE && c.available_a && c.available_b) else { continue; };
                let Some(evaluation) = candidate_is_fully_feasible(&hypothetical, ua, ub, candidate, &environment.catalog, organism.usable_energy) else { continue; };
                if best.as_ref().map(|x| candidate.distance < x.4).unwrap_or(true) { best = Some((ua, pa, placement, point_b, candidate.distance)); }
            }
        }
    }
    let (ua, pa, placement, point_b, _) = best?;
    let mut hypothetical = organism.structure.clone();
    let ub = hypothetical.add_unit(StructuralUnit::from_material(raw.clone(), placement)?);
    let endpoint_a = ConnectionEndpoint::Discrete(pa);
    let endpoint_b = ConnectionEndpoint::Discrete(point_b);
    let candidate = crate::contact::connection_pair_candidates(&hypothetical, ua, ub, &environment.catalog).into_iter().find(|c| c.point_a == endpoint_a && c.point_b == endpoint_b && c.distance <= COMBINE_CONTACT_TOLERANCE && c.available_a && c.available_b)?;
    let a = hypothetical.units[ua].properties(&environment.catalog)?;
    let b = hypothetical.units[ub].properties(&environment.catalog)?;
    let investment = crate::combine::evaluate_formation(candidate, a.cohesion, b.cohesion).threshold;
    let mut energy = organism.usable_energy;
    let attempt = form_bond(&mut hypothetical, ua, endpoint_a, ub, endpoint_b, &environment.catalog, cache, investment, 0.0, &mut energy)?;
    let material = organism.stored_material.take_one_unstructured_named(&resource_name)?;
    organism.structure = hypothetical;
    organism.usable_energy = energy;
    organism.add_transaction_stress(attempt.work_cost);
    let _ = material;
    Some(CombineAttempt { unit_b: ub, ..attempt })
}
fn placement_for_connection(existing: &StructuralUnit, existing_point: crate::resources::ConnectionPoint, new_point: crate::resources::ConnectionPoint) -> Placement {
    let w = crate::contact::world_connection_point(existing_point, existing);
    let angle = w.normal_y.atan2(w.normal_x);
    let rotation = angle + std::f64::consts::PI - new_point.direction_radians;
    let (s, c) = rotation.sin_cos();
    let rx = new_point.x * c - new_point.y * s;
    let ry = new_point.x * s + new_point.y * c;
    Placement { x: w.x - rx, y: w.y - ry, rotation_radians: rotation }
}
pub(crate) fn try_combine(organism: &mut Organism, environment: &Environment, cache: &mut crate::contact::ConnectionCompatibilityCache) -> Option<CombineAttempt> {
    if !organism.structure.units.is_empty() && !organism.stored_material.is_empty() { if let Some(a) = try_combine_stored_unit(organism, environment, cache) { return Some(a); } }
    if organism.structure.units.len() < 2 { return None; }
    let catalog = &environment.catalog;
    let mut best: Option<(usize, usize, FormationEvaluation, f64)> = None;
    for ua in 0..organism.structure.units.len() { for ub in ua + 1..organism.structure.units.len() {
        for candidate in crate::contact::connection_pair_candidates(&organism.structure, ua, ub, catalog) {
            let Some(evaluation) = candidate_is_fully_feasible(&organism.structure, ua, ub, candidate, catalog, organism.usable_energy) else { continue; };
            if best.as_ref().map(|x| candidate.distance < x.3).unwrap_or(true) { best = Some((ua, ub, evaluation, candidate.distance)); }
        }
    }}
    let (ua, ub, evaluation, _) = best?;
    let mut energy = organism.usable_energy;
    let attempt = form_bond(&mut organism.structure, ua, evaluation.candidate.point_a, ub, evaluation.candidate.point_b, catalog, cache, evaluation.threshold, 0.0, &mut energy)?;
    organism.usable_energy = energy;
    organism.add_transaction_stress(attempt.work_cost);
    Some(attempt)
}
