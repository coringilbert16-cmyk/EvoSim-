//! Genome-core construction candidate scoring and selection.
//!
//! The blueprint is an inherited preference, never a rigid construction command.
//! Physical feasibility remains a hard gate outside this scorer.

use rand::Rng;
use rand_chacha::ChaCha8Rng;

use crate::resources::Material;
use crate::structure::Placement;

pub const BLUEPRINT_WEIGHT: f64 = 0.40;
pub const TOPOLOGY_WEIGHT: f64 = 0.20;
pub const CAVITY_WEIGHT: f64 = 0.12;
pub const GROWTH_WEIGHT: f64 = 0.12;
pub const MATERIAL_WEIGHT: f64 = 0.11;
pub const HISTORY_WEIGHT: f64 = 0.05;
pub const MAX_MUTATION_SCORE_SHIFT: f64 = 0.20;
pub const DEFAULT_TEMPERATURE: f64 = 0.15;
const EPSILON: f64 = 1.0e-12;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CandidateAttachment {
    pub existing_unit_index: usize,
    pub existing_endpoint_index: usize,
    pub candidate_endpoint_index: usize,
    pub contact_distance: f64,
    pub normal_alignment: f64,
    pub bond_strength: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandidateScoreInputs {
    pub candidate_position: (f64, f64),
    pub candidate_orientation: f64,
    pub blueprint_position: (f64, f64),
    pub blueprint_orientation: f64,
    pub blueprint_radius: f64,
    pub expected_blueprint_connections: usize,
    pub matched_blueprint_connections: usize,
    pub attachments: Vec<CandidateAttachment>,
    pub cavity_area_before: f64,
    pub cavity_area_after: f64,
    pub cavity_threshold_area: f64,
    pub has_qualifying_cavity: bool,
    pub candidate_step_length: f64,
    pub reference_step_length: Option<f64>,
    pub candidate_growth_direction: (f64, f64),
    pub recent_growth_directions: Vec<(f64, f64)>,
    pub material_cohesion: f64,
    pub material_internal_bond_strengths: Vec<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CandidateScores {
    pub blueprint: f64,
    pub topology: f64,
    pub cavity: f64,
    pub growth: f64,
    pub material: f64,
    pub history: f64,
}

impl CandidateScores {
    pub fn weighted_sum(self) -> f64 {
        BLUEPRINT_WEIGHT * self.blueprint
            + TOPOLOGY_WEIGHT * self.topology
            + CAVITY_WEIGHT * self.cavity
            + GROWTH_WEIGHT * self.growth
            + MATERIAL_WEIGHT * self.material
            + HISTORY_WEIGHT * self.history
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MutationEffect {
    pub direction: [f64; 3],
    pub magnitude: f64,
}

impl MutationEffect {
    pub fn neutral() -> Self {
        Self { direction: [0.0; 3], magnitude: 0.0 }
    }

    pub fn sample(rng: &mut ChaCha8Rng, probability: f64, magnitude: f64) -> Self {
        if rng.gen::<f64>() >= probability.clamp(0.0, 1.0) {
            return Self::neutral();
        }
        let mut direction = [
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
        ];
        let norm = (direction[0] * direction[0]
            + direction[1] * direction[1]
            + direction[2] * direction[2])
            .sqrt();
        if norm <= EPSILON {
            direction = [1.0, 0.0, 0.0];
        } else {
            for value in &mut direction {
                *value /= norm;
            }
        }
        Self {
            direction,
            magnitude: magnitude.abs().min(MAX_MUTATION_SCORE_SHIFT),
        }
    }

    pub fn deviation_alignment(&self, deviation: [f64; 3]) -> f64 {
        let norm = (deviation[0] * deviation[0]
            + deviation[1] * deviation[1]
            + deviation[2] * deviation[2])
            .sqrt();
        if norm <= EPSILON || self.magnitude <= EPSILON {
            return 0.0;
        }
        let normalized = [deviation[0] / norm, deviation[1] / norm, deviation[2] / norm];
        (self.direction[0] * normalized[0]
            + self.direction[1] * normalized[1]
            + self.direction[2] * normalized[2])
            .clamp(-1.0, 1.0)
    }

    pub fn score_shift(&self, deviation: [f64; 3]) -> f64 {
        (self.magnitude * self.deviation_alignment(deviation))
            .clamp(-MAX_MUTATION_SCORE_SHIFT, MAX_MUTATION_SCORE_SHIFT)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConstructionCandidate {
    pub blueprint_element_index: usize,
    pub material: Material,
    pub placement: Placement,
    pub attachments: Vec<CandidateAttachment>,
    pub score_inputs: CandidateScoreInputs,
    pub scores: CandidateScores,
    pub mutation: MutationEffect,
    pub base_score: f64,
    pub effective_score: f64,
}

fn clamp01(value: f64) -> f64 {
    if !value.is_finite() { return 0.0; }
    value.clamp(0.0, 1.0)
}

fn angular_difference(a: f64, b: f64) -> f64 {
    let mut d = (a - b).rem_euclid(std::f64::consts::TAU);
    if d > std::f64::consts::PI { d -= std::f64::consts::TAU; }
    d
}

pub fn blueprint_score(input: &CandidateScoreInputs) -> f64 {
    let dx = input.candidate_position.0 - input.blueprint_position.0;
    let dy = input.candidate_position.1 - input.blueprint_position.1;
    let radius = input.blueprint_radius.abs().max(EPSILON);
    let normalized_distance = dx.hypot(dy) / radius;
    let position = (-normalized_distance * normalized_distance).exp();
    let orientation = 1.0
        - angular_difference(input.candidate_orientation, input.blueprint_orientation).abs()
            / std::f64::consts::PI;
    let expected = input.expected_blueprint_connections.max(1) as f64;
    let matched = input.matched_blueprint_connections.min(input.expected_blueprint_connections) as f64;
    let connections = matched / expected;
    clamp01(0.60 * position + 0.20 * clamp01(orientation) + 0.20 * clamp01(connections))
}

pub fn topology_score(input: &CandidateScoreInputs) -> f64 {
    if input.attachments.is_empty() { return 0.0; }
    let count = input.attachments.len() as f64;
    let alignment = input.attachments.iter().map(|a| clamp01(a.normal_alignment)).sum::<f64>() / count;
    let bond = input.attachments.iter().map(|a| clamp01(a.bond_strength)).sum::<f64>() / count;
    clamp01(0.60 * alignment + 0.40 * bond)
}

pub fn cavity_score(input: &CandidateScoreInputs) -> f64 {
    let before = input.cavity_area_before.max(0.0);
    let after = input.cavity_area_after.max(0.0);
    let threshold = input.cavity_threshold_area.max(EPSILON);
    let progress = ((after - before).max(0.0) / threshold).min(1.0);
    let qualified = if input.has_qualifying_cavity { 1.0 } else { 0.0 };
    clamp01(0.75 * progress + 0.25 * qualified)
}

pub fn growth_score(input: &CandidateScoreInputs) -> f64 {
    let Some(reference) = input.reference_step_length else { return 0.5; };
    if !reference.is_finite() || reference.abs() <= EPSILON { return 0.5; }
    let deviation = (input.candidate_step_length - reference).abs() / reference.abs().max(EPSILON);
    clamp01((-deviation * deviation).exp())
}

pub fn material_score(input: &CandidateScoreInputs) -> f64 {
    if !input.material_internal_bond_strengths.is_empty() {
        return clamp01(input.material_internal_bond_strengths.iter()
            .map(|v| clamp01(*v)).sum::<f64>() / input.material_internal_bond_strengths.len() as f64);
    }
    clamp01(input.material_cohesion)
}

fn normalized_direction(direction: (f64, f64)) -> Option<(f64, f64)> {
    let length = direction.0.hypot(direction.1);
    if !length.is_finite() || length <= EPSILON { None } else { Some((direction.0 / length, direction.1 / length)) }
}

pub fn history_score(input: &CandidateScoreInputs) -> f64 {
    if input.recent_growth_directions.is_empty() { return 0.5; }
    let Some(candidate) = normalized_direction(input.candidate_growth_direction) else { return 0.5; };
    let mut total = 0.0;
    let mut count = 0usize;
    for direction in input.recent_growth_directions.iter().take(4) {
        let Some(previous) = normalized_direction(*direction) else { continue; };
        total += (1.0 + candidate.0 * previous.0 + candidate.1 * previous.1) / 2.0;
        count += 1;
    }
    if count == 0 { 0.5 } else { clamp01(total / count as f64) }
}

pub fn candidate_deviation(input: &CandidateScoreInputs) -> [f64; 3] {
    let radius = input.blueprint_radius.abs().max(EPSILON);
    let position = (
        (input.candidate_position.0 - input.blueprint_position.0) / radius,
        (input.candidate_position.1 - input.blueprint_position.1) / radius,
    );
    let orientation = angular_difference(input.candidate_orientation, input.blueprint_orientation)
        / std::f64::consts::PI;
    let expected = input.expected_blueprint_connections.max(1) as f64;
    let connection = (input.matched_blueprint_connections as f64
        - input.expected_blueprint_connections as f64) / expected;
    [position.0 + position.1, orientation, connection]
}

pub fn score_candidate(input: &CandidateScoreInputs, mutation: MutationEffect) -> (CandidateScores, f64, f64) {
    let scores = CandidateScores {
        blueprint: blueprint_score(input),
        topology: topology_score(input),
        cavity: cavity_score(input),
        growth: growth_score(input),
        material: material_score(input),
        history: history_score(input),
    };
    let base = scores.weighted_sum();
    let effective = base + mutation.score_shift(candidate_deviation(input));
    (scores, base, effective)
}

pub fn softmax_probabilities(scores: &[f64], temperature: f64) -> Vec<f64> {
    if scores.is_empty() { return Vec::new(); }
    let temperature = temperature.max(EPSILON);
    let max_score = scores.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let weights = scores.iter().map(|score| ((score - max_score) / temperature).exp()).collect::<Vec<_>>();
    let total = weights.iter().sum::<f64>();
    if !total.is_finite() || total <= EPSILON {
        return vec![1.0 / scores.len() as f64; scores.len()];
    }
    weights.into_iter().map(|weight| weight / total).collect()
}

pub fn select_candidate_index(candidates: &[ConstructionCandidate], temperature: f64, rng: &mut ChaCha8Rng) -> Option<usize> {
    if candidates.is_empty() { return None; }
    let probabilities = softmax_probabilities(
        &candidates.iter().map(|c| c.effective_score).collect::<Vec<_>>(),
        temperature,
    );
    let draw = rng.gen::<f64>();
    let mut cumulative = 0.0;
    for (index, probability) in probabilities.iter().enumerate() {
        cumulative += *probability;
        if draw < cumulative || index + 1 == probabilities.len() { return Some(index); }
    }
    Some(probabilities.len() - 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn input() -> CandidateScoreInputs {
        CandidateScoreInputs {
            candidate_position: (0.0, 0.0), candidate_orientation: 0.0,
            blueprint_position: (0.0, 0.0), blueprint_orientation: 0.0,
            blueprint_radius: 4.0, expected_blueprint_connections: 2, matched_blueprint_connections: 2,
            attachments: vec![CandidateAttachment { existing_unit_index: 0, existing_endpoint_index: 0,
                candidate_endpoint_index: 0, contact_distance: 0.0, normal_alignment: 1.0, bond_strength: 1.0 }],
            cavity_area_before: 0.0, cavity_area_after: 1.0, cavity_threshold_area: 4.0,
            has_qualifying_cavity: false, candidate_step_length: 2.0, reference_step_length: Some(2.0),
            candidate_growth_direction: (1.0, 0.0), recent_growth_directions: vec![(1.0, 0.0)],
            material_cohesion: 0.8, material_internal_bond_strengths: Vec::new(),
        }
    }

    #[test]
    fn scores_are_normalized() {
        let (scores, base, effective) = score_candidate(&input(), MutationEffect::neutral());
        assert!((0.0..=1.0).contains(&scores.blueprint));
        assert!((0.0..=1.0).contains(&scores.topology));
        assert!((0.0..=1.0).contains(&scores.cavity));
        assert!((0.0..=1.0).contains(&scores.growth));
        assert!((0.0..=1.0).contains(&scores.material));
        assert!((0.0..=1.0).contains(&scores.history));
        assert!((base - effective).abs() < EPSILON);
    }

    #[test]
    fn perfect_blueprint_match_scores_one() { assert!((blueprint_score(&input()) - 1.0).abs() < EPSILON); }

    #[test]
    fn mutation_is_bounded() {
        let mut rng = ChaCha8Rng::seed_from_u64(7);
        for _ in 0..1000 {
            let mutation = MutationEffect::sample(&mut rng, 1.0, 10.0);
            assert!(mutation.score_shift([1.0, 0.0, 0.0]).abs() <= MAX_MUTATION_SCORE_SHIFT + EPSILON);
        }
    }

    #[test]
    fn softmax_probabilities_sum_to_one() {
        let probabilities = softmax_probabilities(&[0.1, 0.4, 0.9], DEFAULT_TEMPERATURE);
        assert!((probabilities.iter().sum::<f64>() - 1.0).abs() < EPSILON);
        assert!(probabilities.iter().all(|p| (0.0..=1.0).contains(p)));
    }

    #[test]
    fn selection_is_deterministic_for_seed() {
        let mut rng_a = ChaCha8Rng::seed_from_u64(99);
        let mut rng_b = ChaCha8Rng::seed_from_u64(99);
        let probabilities = softmax_probabilities(&[0.1, 0.4, 0.9], DEFAULT_TEMPERATURE);
        let draw_a = rng_a.gen::<f64>();
        let draw_b = rng_b.gen::<f64>();
        assert_eq!(draw_a, draw_b);
        assert!((probabilities.iter().sum::<f64>() - 1.0).abs() < EPSILON);
    }
}
