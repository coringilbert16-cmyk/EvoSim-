//! COMBINE physics: candidate interaction, formation threshold, and intrinsic bond strength.
use crate::contact::{ConnectionCompatibilityCache, ConnectionPairCandidate};
use crate::math::exponential_influence;
use crate::resources::{effective_reactivity, BaseResource, ResourceProperties};
use crate::structure::{formation_threshold, OrganismStructure};

const EPSILON: f64 = 1e-12;
pub const EXPERIMENTAL_BOND_STRENGTH_SCALE: f64 = 1.0;
pub const EXPERIMENTAL_MAX_BOND_STRENGTH: f64 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExperimentalInteraction {
    pub direction: f64,
    pub magnitude: f64,
    pub signed_value: f64,
}

pub fn experimental_interaction(a: ResourceProperties, b: ResourceProperties, candidate: ConnectionPairCandidate, water_field: f64) -> ExperimentalInteraction {
    let d = b.potential_energy - a.potential_energy;
    let direction = if d > EPSILON { 1.0 } else if d < -EPSILON { -1.0 } else { 0.0 };
    let reactivity = (exponential_influence(effective_reactivity(a.reactivity.max(0.0), water_field))
        + exponential_influence(effective_reactivity(b.reactivity.max(0.0), water_field))) / 2.0;
    let facing = ((candidate.facing.clamp(-1.0, 1.0) + 1.0) * 0.5).clamp(0.0, 1.0);
    let distance = if candidate.distance.is_finite() { candidate.distance.max(0.0) } else { f64::INFINITY };
    let factor = if distance.is_finite() { 1.0 / (1.0 + distance) } else { 0.0 };
    let magnitude = d.abs() * reactivity * facing * factor;
    ExperimentalInteraction { direction, magnitude, signed_value: direction * magnitude }
}

pub fn experimental_combine_work_cost(a: ResourceProperties, b: ResourceProperties, candidate: ConnectionPairCandidate, water_field: f64) -> f64 {
    let interaction = experimental_interaction(a, b, candidate, water_field);
    let complexity = 1.0 + ((a.mass.max(0.0) + b.mass.max(0.0)) * 0.5).sqrt();
    let cohesion = 1.0 + ((a.cohesion.clamp(0.0, 1.0) + b.cohesion.clamp(0.0, 1.0)) * 0.5);
    (0.25 + interaction.magnitude) * complexity * cohesion
}

pub fn experimental_bond_strength(surplus: f64) -> f64 {
    if !surplus.is_finite() || surplus <= 0.0 { return 0.0; }
    let scale = EXPERIMENTAL_BOND_STRENGTH_SCALE.max(EPSILON);
    EXPERIMENTAL_MAX_BOND_STRENGTH.max(0.0) * (1.0 - (-surplus / scale).exp())
}

pub fn bond_strength(a: ResourceProperties, b: ResourceProperties) -> f64 {
    if !a.cohesion.is_finite() || !b.cohesion.is_finite() { return 0.0; }
    (a.cohesion.clamp(0.0, 1.0) * b.cohesion.clamp(0.0, 1.0)).sqrt()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FormationEvaluation { pub candidate: ConnectionPairCandidate, pub threshold: f64 }

pub fn evaluate_formation(candidate: ConnectionPairCandidate, cohesion_a: f64, cohesion_b: f64) -> FormationEvaluation {
    FormationEvaluation { candidate, threshold: formation_threshold(cohesion_a, cohesion_b, candidate.load_a, candidate.load_b) }
}

pub fn formation_succeeds(e: FormationEvaluation, investment: f64) -> bool {
    investment.is_finite() && e.threshold.is_finite() && investment >= e.threshold
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombineEvaluationError { NonFiniteWorkCost, InvalidFormationThreshold }

pub fn required_investment(a: ResourceProperties, b: ResourceProperties, e: FormationEvaluation, water: f64) -> Result<(ExperimentalInteraction, f64, f64), CombineEvaluationError> {
    let interaction = experimental_interaction(a, b, e.candidate, water);
    let work = experimental_combine_work_cost(a, b, e.candidate, water);
    if !work.is_finite() || work < 0.0 { return Err(CombineEvaluationError::NonFiniteWorkCost); }
    if !e.threshold.is_finite() || e.threshold < 0.0 { return Err(CombineEvaluationError::InvalidFormationThreshold); }
    Ok((interaction, work, e.threshold))
}

pub fn eligible_candidates(s: &OrganismStructure, a: usize, b: usize, c: &[BaseResource], cache: &mut ConnectionCompatibilityCache) -> Vec<ConnectionPairCandidate> {
    crate::contact::connection_pair_candidates_cached(s, a, b, c, cache)
}

pub fn evaluate_candidates(s: &OrganismStructure, a: usize, b: usize, c: &[BaseResource], cache: &mut ConnectionCompatibilityCache) -> Vec<FormationEvaluation> {
    let (Some(ua), Some(ub)) = (s.units.get(a), s.units.get(b)) else { return Vec::new(); };
    let (Some(ca), Some(cb)) = (ua.properties(c).map(|p| p.cohesion), ub.properties(c).map(|p| p.cohesion)) else { return Vec::new(); };
    eligible_candidates(s, a, b, c, cache).into_iter().map(|candidate| evaluate_formation(candidate, ca, cb)).collect()
}
