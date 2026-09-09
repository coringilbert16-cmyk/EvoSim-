//! Runtime COMBINE execution boundary.
//! Physics is evaluated by `combine`; this module only selects a physical
//! candidate, applies the returned result, mutates structure, and settles state.
use crate::combine::{bond_strength, eligible_candidates, required_investment, ExperimentalInteraction, FormationEvaluation};
use crate::contact::ConnectionCompatibilityCache;
use crate::resources::{BaseResource, ConnectionSites, Material};
use crate::state::{Environment, Organism};
use crate::structure::{BondEndpoint, ConnectionEndpoint, Placement, StructuralUnit};

const EPSILON: f64 = 1e-12;
const COMBINE_CONTACT_TOLERANCE: f64 = 1.0;

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

fn water_field_amount(environment: &Environment, organism: &Organism) -> f64 {
    organism.occupied_cells.first()
        .and_then(|p| environment.field.index_for_position(p.x, p.y))
        .map(|i| environment.field.cells[i].materials.iter().flat_map(|m| m.parts.iter()).filter(|(n, _)| n == "Water").map(|(_, a)| *a).sum())
        .unwrap_or(0.0)
}

fn placement_for_connection(existing: &crate::structure::StructuralUnit, existing_point: crate::resources::ConnectionPoint, new_point: crate::resources::ConnectionPoint) -> Placement {
    let w = crate::contact::world_connection_point(existing_point, existing);
    let angle = w.normal_y.atan2(w.normal_x);
    let rotation = angle + std::f64::consts::PI - new_point.direction_radians;
    let (s, c) = rotation.sin_cos();
    let rx = new_point.x * c - new_point.y * s;
    let ry = new_point.x * s + new_point.y * c;
    Placement { x: w.x - rx, y: w.y - ry, rotation_radians: rotation }
}

fn placement_for_continuous_new(existing: &crate::structure::StructuralUnit, existing_point: crate::resources::ConnectionPoint, new_shape: &crate::resources::Shape) -> Placement {
    let w = crate::contact::world_connection_point(existing_point, existing);
    let radius = new_shape.form.bounding_radius();
    Placement { x: w.x + w.normal_x * radius, y: w.y + w.normal_y * radius, rotation_radians: 0.0 }
}

fn energy_requirement(investment: f64, work: f64, interaction: f64) -> Option<f64> {
    if !investment.is_finite() || investment < 0.0 || !work.is_finite() || work < 0.0 || !interaction.is_finite() { return None; }
    let required = investment + work - interaction;
    required.is_finite().then_some(required.max(0.0))
}

fn settle_energy(energy: &mut f64, investment: f64, work: f64, interaction: f64) -> Option<f64> {
    let required = energy_requirement(investment, work, interaction)?;
    if *energy + EPSILON < required { return None; }
    let net = interaction - investment - work;
    let next = *energy + net;
    if !net.is_finite() || !next.is_finite() || next < -EPSILON { return None; }
    *energy = next.max(0.0);
    Some(net)
}

fn evaluate_candidate(structure: &crate::structure::OrganismStructure, ua: usize, ub: usize, candidate: crate::contact::ConnectionPairCandidate, catalog: &[BaseResource], water: f64) -> Option<(FormationEvaluation, ExperimentalInteraction, f64, f64, f64)> {
    if candidate.distance > COMBINE_CONTACT_TOLERANCE || !candidate.available_a || !candidate.available_b { return None; }
    let a = structure.units.get(ua)?.properties(catalog)?;
    let b = structure.units.get(ub)?.properties(catalog)?;
    let evaluation = crate::combine::evaluate_formation(candidate, a.cohesion, b.cohesion);
    let (interaction, work, investment) = required_investment(a, b, evaluation, water).ok()?;
    let required = energy_requirement(investment, work, interaction.signed_value)?;
    Some((evaluation, interaction, work, investment, required))
}

/// The only runtime primitive that turns a COMBINE physics result into a bond.
/// It validates the requested physical candidate again through the structural
/// admission authority before committing either structure or energy.
fn form_bond(structure: &mut crate::structure::OrganismStructure, ua: usize, endpoint_a: ConnectionEndpoint, ub: usize, endpoint_b: ConnectionEndpoint, catalog: &[BaseResource], cache: &mut ConnectionCompatibilityCache, investment: f64, water: f64, energy: &mut f64) -> Option<CombineAttempt> {
    if ua >= structure.units.len() || ub >= structure.units.len() || ua == ub { return None; }
    let candidate = crate::contact::connection_pair_candidates_cached(structure, ua, ub, catalog, cache).into_iter().find(|c| c.endpoint_a == endpoint_a && c.endpoint_b == endpoint_b && c.distance <= COMBINE_CONTACT_TOLERANCE && c.available_a && c.available_b)?;
    let a = structure.units[ua].properties(catalog)?;
    let b = structure.units[ub].properties(catalog)?;
    let evaluation = crate::combine::evaluate_formation(candidate, a.cohesion, b.cohesion);
    if !crate::combine::formation_succeeds(evaluation, investment) { return None; }
    let (interaction, work, threshold) = required_investment(a, b, evaluation, water).ok()?;
    if (threshold - investment).abs() > EPSILON { return None; }
    let strength = bond_strength(a, b);
    if !strength.is_finite() { return None; }
    let mut trial_structure = structure.clone();
    let bond = crate::structure::Bond { endpoint_a: BondEndpoint { unit_index: ua, location: endpoint_a }, endpoint_b: BondEndpoint { unit_index: ub, location: endpoint_b }, strength, bond_energy: investment };
    crate::contact::try_add_bond(&mut trial_structure, bond, catalog).ok()?;
    let mut trial_energy = *energy;
    let net = settle_energy(&mut trial_energy, investment, work, interaction.signed_value)?;
    *structure = trial_structure;
    *energy = trial_energy;
    Some(CombineAttempt { unit_a: ua, unit_b: ub, endpoint_a, endpoint_b, work_cost: work, energy_invested: investment, interaction_direction: interaction.direction, interaction_magnitude: interaction.magnitude, interaction_energy: interaction.signed_value, formation_threshold: threshold, net_energy_change: net, bond_strength: strength, bond_energy: investment })
}

pub(crate) fn instantiate_one_unit(organism: &mut Organism, catalog: &[BaseResource]) -> Option<usize> {
    let material = organism.stored_material.peek_one_unstructured()?;
    let resource_name = material.parts.first().filter(|(_, a)| (*a - 1.0).abs() <= EPSILON).map(|(n, _)| n.clone())?;
    if material.parts.len() != 1 || catalog.iter().all(|b| b.name != resource_name) { return None; }
    let material = organism.stored_material.take_one_unstructured_named(&resource_name)?;
    let (x, y) = organism.occupied_cells.first().map(|p| (p.x, p.y)).unwrap_or((0.0, 0.0));
    Some(organism.structure.add_unit(StructuralUnit::from_material(material, Placement { x, y, rotation_radians: 0.0 })?))
}

pub(crate) fn try_combine_stored_unit(organism: &mut Organism, environment: &Environment, cache: &mut ConnectionCompatibilityCache) -> Option<CombineAttempt> {
    let raw = organism.stored_material.peek_one_unstructured()?;
    if raw.parts.len() != 1 || !raw.internal_bonds.is_empty() || (raw.parts[0].1 - 1.0).abs() > EPSILON || environment.catalog.iter().all(|b| b.name != raw.parts[0].0) { return None; }
    let resource_name = raw.parts[0].0.clone();
    let resource = environment.catalog.iter().find(|b| b.name == resource_name)?;
    let water = water_field_amount(environment, organism);
    let mut best: Option<(usize, Placement, FormationEvaluation, f64)> = None;

    for ua in 0..organism.structure.units.len() {
        let Some(ConnectionSites::Corners(existing)) = organism.structure.units[ua].connection_sites(&environment.catalog) else { continue; };
        for &ep in &existing {
            let placements = match resource.shape.connection_sites() {
                ConnectionSites::Corners(new_sites) => new_sites.iter().map(|np| placement_for_connection(&organism.structure.units[ua], ep, *np)).collect::<Vec<_>>(),
                ConnectionSites::Circumference { .. } | ConnectionSites::Undetermined => vec![placement_for_continuous_new(&organism.structure.units[ua], ep, &resource.shape)],
            };
            for placement in placements {
                let mut hypothetical = organism.structure.clone();
                let ub = hypothetical.add_unit(StructuralUnit::from_material(raw.clone(), placement)?);
                for candidate in crate::contact::connection_pair_candidates_cached(&hypothetical, ua, ub, &environment.catalog, cache) {
                    let Some((evaluation, _, _, _, required)) = evaluate_candidate(&hypothetical, ua, ub, candidate, &environment.catalog, water) else { continue; };
                    if organism.usable_energy + EPSILON < required { continue; }
                    if best.as_ref().map(|x| candidate.distance < x.3).unwrap_or(true) { best = Some((ua, placement, evaluation, candidate.distance)); }
                }
            }
        }
    }

    let (ua, placement, evaluation, _) = best?;
    let ub = organism.structure.units.len();
    let mut hypothetical = organism.structure.clone();
    hypothetical.add_unit(StructuralUnit::from_material(raw.clone(), placement)?);
    let endpoint_a = evaluation.candidate.endpoint_a;
    let endpoint_b = evaluation.candidate.endpoint_b;
    let investment = evaluation.threshold;
    let mut energy = organism.usable_energy;
    let attempt = form_bond(&mut hypothetical, ua, endpoint_a, ub, endpoint_b, &environment.catalog, cache, investment, water, &mut energy)?;
    let material = organism.stored_material.take_one_unstructured_named(&resource_name)?;
    organism.structure = hypothetical;
    organism.usable_energy = energy;
    organism.add_transaction_stress(attempt.work_cost);
    let _ = material;
    Some(attempt)
}

pub(crate) fn try_combine(organism: &mut Organism, environment: &Environment, cache: &mut ConnectionCompatibilityCache) -> Option<CombineAttempt> {
    if !organism.structure.units.is_empty() && !organism.stored_material.is_empty() {
        if let Some(attempt) = try_combine_stored_unit(organism, environment, cache) { return Some(attempt); }
    }
    if organism.structure.units.len() < 2 { return None; }
    let catalog = &environment.catalog;
    let water = water_field_amount(environment, organism);
    let mut best: Option<(usize, usize, FormationEvaluation, f64)> = None;
    for ua in 0..organism.structure.units.len() {
        for ub in ua + 1..organism.structure.units.len() {
            for candidate in eligible_candidates(&organism.structure, ua, ub, catalog, cache) {
                let Some((evaluation, _, _, _, required)) = evaluate_candidate(&organism.structure, ua, ub, candidate, catalog, water) else { continue; };
                if organism.usable_energy + EPSILON < required { continue; }
                if best.as_ref().map(|x| candidate.distance < x.3).unwrap_or(true) { best = Some((ua, ub, evaluation, candidate.distance)); }
            }
        }
    }
    let (ua, ub, evaluation, _) = best?;
    let mut energy = organism.usable_energy;
    let attempt = form_bond(&mut organism.structure, ua, evaluation.candidate.endpoint_a, ub, evaluation.candidate.endpoint_b, catalog, cache, evaluation.threshold, water, &mut energy)?;
    organism.usable_energy = energy;
    organism.add_transaction_stress(attempt.work_cost);
    Some(attempt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::default_catalog;
    #[test]
    fn energy_requirement_is_single_runtime_accounting_rule() {
        assert_eq!(energy_requirement(2.0, 3.0, 1.0), Some(4.0));
        assert_eq!(energy_requirement(2.0, 3.0, 10.0), Some(0.0));
        assert!(energy_requirement(f64::NAN, 1.0, 0.0).is_none());
    }
    #[test]
    fn form_bond_requires_physical_admission() {
        let catalog = default_catalog();
        let mut structure = crate::structure::OrganismStructure::new();
        let a = structure.add_unit(StructuralUnit::new("Carbon", Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 }));
        let b = structure.add_unit(StructuralUnit::new("Carbon", Placement { x: 0.0, y: 0.0, rotation_radians: 0.0 }));
        let mut cache = ConnectionCompatibilityCache::new();
        let candidates = crate::contact::connection_pair_candidates(&structure, a, b, &catalog);
        let c = candidates.first().unwrap();
        let pa = structure.units[a].properties(&catalog).unwrap();
        let pb = structure.units[b].properties(&catalog).unwrap();
        let evaluation = crate::combine::evaluate_formation(*c, pa.cohesion, pb.cohesion);
        let mut energy = 100.0;
        let result = form_bond(&mut structure, a, c.endpoint_a, b, c.endpoint_b, &catalog, &mut cache, evaluation.threshold, 0.0, &mut energy);
        assert!(result.is_some());
        assert_eq!(structure.bonds.len(), 1);
    }
}
