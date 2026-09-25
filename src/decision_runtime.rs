#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
//! Runtime bridge for the decision architecture.
//!
//! This module contains no chemistry or structural math. It connects organism
//! state to the decision policy and returns an action candidate for simulation
//! to execute through its existing physical systems.

use crate::decision::{
    approve_action_for_current_needs, ActionConsequence, ActionEligibility, ActionKind,
    CurrentNeeds, DecisionHistory, DecisionResult,
};
use rand::Rng;
use rand_chacha::ChaCha8Rng;

fn immediate_consequence(action: ActionKind) -> ActionConsequence {
    match action {
        ActionKind::Break => ActionConsequence {
            structural_delta: -1.0,
            developmental_delta: -1.0,
            ..ActionConsequence::NONE
        },
        _ => ActionConsequence::NONE,
    }
}

fn historical_consequence(
    history: &DecisionHistory,
    candidate: &ActionCandidate,
) -> ActionConsequence {
    history
        .consequence(candidate.action, candidate.context_key.as_deref())
        .unwrap_or_else(|| immediate_consequence(candidate.action))
}

fn consequence_components(
    consequence: ActionConsequence,
    needs: CurrentNeeds,
) -> [f64; 4] {
    [
        consequence.energy_delta * needs.survival.max(0.0),
        consequence.structural_delta * needs.survival.max(needs.reproduction).max(needs.development),
        consequence.developmental_delta * needs.development.max(needs.reproduction),
        -consequence.stress_delta * needs.survival.max(0.0),
    ]
}

fn dominates(a: [f64; 4], b: [f64; 4]) -> bool {
    let at_least_as_good = a.iter().zip(b.iter()).all(|(left, right)| left >= right);
    let strictly_better = a.iter().zip(b.iter()).any(|(left, right)| left > right);
    at_least_as_good && strictly_better
}

#[derive(Clone, Copy, Debug)]
pub struct DecisionContext {
    pub needs: CurrentNeeds,
    pub eligibility: ActionEligibility,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionCandidate {
    pub action: ActionKind,
    pub context_key: Option<String>,
}

pub fn approve(context: DecisionContext, action: ActionKind) -> DecisionResult {
    approve_action_for_current_needs(action, context.eligibility, context.needs)
}

fn need_pressure(action: ActionKind, needs: CurrentNeeds) -> f64 {
    action
        .relevant_needs()
        .iter()
        .map(|need| needs.pressure(*need))
        .fold(0.0_f64, |best, pressure| best.max(pressure))
}

fn history_adjustment(history: &DecisionHistory, candidate: &ActionCandidate) -> f64 {
    match history.outcome(candidate.action, candidate.context_key.as_deref()) {
        Some(OutcomeKind::Beneficial) => HISTORY_INFLUENCE,
        Some(OutcomeKind::Harmful) => -HISTORY_INFLUENCE,
        Some(OutcomeKind::Neutral) | None => 0.0,
    }
}

fn cheap_decision_score(
    context: DecisionContext,
    candidate: &ActionCandidate,
) -> Option<f64> {
    if approve(context, candidate.action) != DecisionResult::Approve {
        return None;
    }
    Some(need_pressure(candidate.action, context.needs))
}

/// Identify candidates that genuinely require a physical developmental
/// comparison. Need pressure and learned consequence are resolved first.
/// Physical preview is needed only when distinct development-relevant action
/// kinds remain tied at that stage.
pub fn developmental_competition_indices(
    context: DecisionContext,
    history: &DecisionHistory,
    candidates: &[ActionCandidate],
) -> Vec<usize> {
    if context.needs.development <= 0.0 {
        return Vec::new();
    }

    let mut best_score = None;
    for candidate in candidates {
        let Some(score) = cheap_decision_score(context, history, candidate) else {
            continue;
        };
        best_score = Some(best_score.map_or(score, |best: f64| best.max(score)));
    }
    let Some(best_score) = best_score else {
        return Vec::new();
    };

    let mut indices = Vec::new();
    let mut action_kinds = Vec::new();
    for (index, candidate) in candidates.iter().enumerate() {
        let Some(score) = cheap_decision_score(context, history, candidate) else {
            continue;
        };
        if score != best_score
            || !candidate
                .action
                .relevant_needs()
                .contains(&crate::decision::NeedKind::Development)
        {
            continue;
        }
        if !action_kinds.contains(&candidate.action) {
            action_kinds.push(candidate.action);
        }
        indices.push(index);
    }

    if action_kinds.len() >= 2 {
        indices
    } else {
        Vec::new()
    }
}

/// Select exactly one approved action from candidates.
///
/// Need pressure is the primary relevance signal. Learned consequences refine
/// that score. Physical developmental results are used only when all tied
/// candidates have comparable results. Any remaining genuine tie is resolved
/// randomly rather than by candidate ordering.
pub fn select_action(
    context: DecisionContext,
    history: &DecisionHistory,
    candidates: &[ActionCandidate],
    rng: &mut ChaCha8Rng,
) -> Option<ActionCandidate> {
    let scores = vec![None; candidates.len()];
    select_action_with_developmental_scores(context, history, candidates, &scores, rng)
}

pub fn select_action_with_developmental_scores(
    context: DecisionContext,
    history: &DecisionHistory,
    candidates: &[ActionCandidate],
    developmental_scores: &[Option<f64>],
    rng: &mut ChaCha8Rng,
) -> Option<ActionCandidate> {
    let scored: Vec<_> = candidates
        .iter()
        .enumerate()
        .filter_map(|(index, candidate)| {
            let score = cheap_decision_score(context, candidate)?;
            Some((
                index,
                score,
                historical_consequence(history, candidate),
                developmental_scores
                    .get(index)
                    .copied()
                    .flatten()
                    .filter(|value| value.is_finite()),
            ))
        })
        .collect();
    let best_need = scored
        .iter()
        .map(|(_, score, _, _)| *score)
        .max_by(f64::total_cmp)?;
    let mut tied: Vec<_> = scored
        .into_iter()
        .filter(|(_, score, _, _)| score.total_cmp(&best_need).is_eq())
        .collect();

    if tied.len() > 1 {
        let components: Vec<_> = tied
            .iter()
            .map(|(_, _, consequence, _)| consequence_components(*consequence, context.needs))
            .collect();
        let nondominated: Vec<_> = tied
            .iter()
            .enumerate()
            .filter(|(index, _)| {
                !components
                    .iter()
                    .enumerate()
                    .any(|(other, other_components)| other != *index && dominates(*other_components, components[*index]))
            })
            .map(|(_, candidate)| *candidate)
            .collect();
        if !nondominated.is_empty() {
            tied = nondominated;
        }
    }

    if tied.len() > 1 && tied.iter().all(|(_, _, _, score)| score.is_some()) {
        let best_developmental = tied
            .iter()
            .filter_map(|(_, _, _, score)| *score)
            .max_by(f64::total_cmp)
            .expect("all tied candidates have developmental scores");
        tied.retain(|(_, _, _, score)| score.is_some_and(|score| score == best_developmental));
    }

    let selected_index = rng.gen_range(0..tied.len());
    candidates.get(tied[selected_index].0).cloned()
}

pub fn record_consequence(
    history: &mut DecisionHistory,
    candidate: &ActionCandidate,
    consequence: ActionConsequence,
) {
    history.record_consequence(candidate.action, candidate.context_key.clone(), consequence);
}

pub fn known_consequence(
    history: &DecisionHistory,
    action: ActionKind,
    context_key: Option<&str>,
) -> bool {
    history.has_knowledge(action, context_key)
}


