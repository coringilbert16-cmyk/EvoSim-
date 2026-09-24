#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
//! Runtime bridge for the decision architecture.
//!
//! This module contains no chemistry or structural math. It connects organism
//! state to the decision policy and returns an action candidate for simulation
//! to execute through its existing physical systems.

use crate::decision::{
    approve_action_for_current_needs, outcome_is_known, ActionConsequence, ActionEligibility,
    ActionKind, CurrentNeeds, DecisionHistory, DecisionResult,
};
use rand::Rng;
use rand_chacha::ChaCha8Rng;

/// Maximum influence a recorded consequence has on action selection.
///
/// Need pressure remains the primary driver. This value is deliberately small
/// and exposed so the decision layer can be tuned without changing chemistry
/// or physics.
pub const HISTORY_INFLUENCE: f64 = 0.25;
/// Small default preference for preserving the current state. A transaction
/// must therefore have enough current need or learned relevance to justify itself.
pub const NO_TRANSACTION_BASELINE: f64 = 0.25;

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
    if action == ActionKind::NoTransaction {
        return DecisionResult::Approve;
    }
    approve_action_for_current_needs(action, context.eligibility, context.needs)
}

fn need_pressure(action: ActionKind, needs: CurrentNeeds) -> f64 {
    action
        .relevant_needs()
        .iter()
        .map(|need| needs.pressure(*need))
        .fold(0.0_f64, |best, pressure| best.max(pressure))
}

fn history_adjustment(
    history: &DecisionHistory,
    candidate: &ActionCandidate,
    needs: CurrentNeeds,
) -> f64 {
    history
        .consequence(candidate.action, candidate.context_key.as_deref())
        .map(|consequence| {
            (consequence.weighted_relevance(needs) * HISTORY_INFLUENCE)
                .clamp(-HISTORY_INFLUENCE, HISTORY_INFLUENCE)
        })
        .unwrap_or(0.0)
}

fn cheap_decision_score(
    context: DecisionContext,
    history: &DecisionHistory,
    candidate: &ActionCandidate,
) -> Option<f64> {
    if approve(context, candidate.action) != DecisionResult::Approve {
        return None;
    }
    Some(if candidate.action == ActionKind::NoTransaction {
        NO_TRANSACTION_BASELINE
    } else {
        need_pressure(candidate.action, context.needs)
            + history_adjustment(history, candidate, context.needs)
    })
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

/// Select an action using physical developmental results when they are
/// available. The scores are candidate-specific results produced by the
/// physical/developmental subsystem; this module does not calculate them.
pub fn select_action_with_developmental_scores(
    context: DecisionContext,
    history: &DecisionHistory,
    candidates: &[ActionCandidate],
    developmental_scores: &[Option<f64>],
    rng: &mut ChaCha8Rng,
) -> Option<ActionCandidate> {
    let mut scored = Vec::new();
    for (index, candidate) in candidates.iter().enumerate() {
        if approve(context, candidate.action) != DecisionResult::Approve {
            continue;
        }
        scored.push((
            index,
            if candidate.action == ActionKind::NoTransaction {
                NO_TRANSACTION_BASELINE
            } else {
                need_pressure(candidate.action, context.needs)
                    + history_adjustment(history, candidate, context.needs)
            },
            developmental_scores
                .get(index)
                .copied()
                .flatten()
                .filter(|value| value.is_finite()),
        ));
    }

    let best_score = scored
        .iter()
        .map(|(_, score, _)| *score)
        .max_by(f64::total_cmp)?;

    let mut tied: Vec<_> = scored
        .into_iter()
        .filter(|(_, score, _)| score.total_cmp(&best_score).is_eq())
        .collect();

    if tied.len() > 1 && tied.iter().all(|(_, _, score)| score.is_some()) {
        let best_developmental = tied
            .iter()
            .filter_map(|(_, _, score)| *score)
            .max_by(f64::total_cmp)
            .expect("all tied candidates have developmental scores");
        tied.retain(|(_, _, score)| score.is_some_and(|score| score == best_developmental));
    }

    let selected_index = rng.gen_range(0..tied.len());
    candidates.get(tied[selected_index].0).cloned()
}

pub fn record_consequence(
    history: &mut DecisionHistory,
    candidate: &ActionCandidate,
    consequence: ActionConsequence,
) {
    history.record(candidate.action, candidate.context_key.clone(), consequence);
}

pub fn known_consequence(
    history: &DecisionHistory,
    action: ActionKind,
    context_key: Option<&str>,
) -> bool {
    outcome_is_known(history, action, context_key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::NeedKind;
    use rand::SeedableRng;

    fn context() -> DecisionContext {
        DecisionContext {
            needs: CurrentNeeds {
                survival: 1.0,
                reproduction: 0.5,
                development: 0.0,
            },
            eligibility: ActionEligibility {
                can_move: true,
                can_break: true,
                can_combine: true,
                ..Default::default()
            },
        }
    }

    #[test]
    fn bridge_approves_needed_mechanically_eligible_action() {
        assert_eq!(
            approve(context(), ActionKind::Break),
            DecisionResult::Approve
        );
    }

    #[test]
    fn bridge_rejects_mechanically_ineligible_action() {
        let context = DecisionContext {
            needs: CurrentNeeds {
                survival: 1.0,
                reproduction: 0.0,
                development: 0.0,
            },
            eligibility: Default::default(),
        };
        assert_eq!(approve(context, ActionKind::Break), DecisionResult::Reject);
    }

    #[test]
    fn survival_pressure_selects_survival_relevant_action() {
        let context = DecisionContext {
            needs: CurrentNeeds {
                survival: 1.0,
                reproduction: 0.0,
                development: 0.0,
            },
            eligibility: ActionEligibility {
                can_break: true,
                can_combine: true,
                ..Default::default()
            },
        };
        let history = DecisionHistory::default();
        let candidates = vec![
            ActionCandidate {
                action: ActionKind::Combine,
                context_key: None,
            },
            ActionCandidate {
                action: ActionKind::Break,
                context_key: Some("bond:0".into()),
            },
        ];

        assert_eq!(
            select_action(
                context,
                &history,
                &candidates,
                &mut ChaCha8Rng::seed_from_u64(1),
            ),
            Some(candidates[1].clone())
        );
    }

    #[test]
    fn reproduction_pressure_selects_reproduction_relevant_action() {
        let context = DecisionContext {
            needs: CurrentNeeds {
                survival: 0.0,
                reproduction: 1.0,
                development: 0.0,
            },
            eligibility: ActionEligibility {
                can_break: true,
                can_combine: true,
                ..Default::default()
            },
        };
        let history = DecisionHistory::default();
        let candidates = vec![
            ActionCandidate {
                action: ActionKind::Break,
                context_key: Some("bond:0".into()),
            },
            ActionCandidate {
                action: ActionKind::Combine,
                context_key: None,
            },
        ];

        let selected = select_action(
            context,
            &history,
            &candidates,
            &mut ChaCha8Rng::seed_from_u64(1),
        );
        assert!(selected.is_some_and(|candidate| candidates.contains(&candidate)));
    }

    #[test]
    fn beneficial_history_can_change_selection_when_need_pressure_is_close() {
        let context = DecisionContext {
            needs: CurrentNeeds {
                survival: 0.60,
                reproduction: 0.40,
                development: 0.0,
            },
            eligibility: ActionEligibility {
                can_break: true,
                can_combine: true,
                ..Default::default()
            },
        };
        let candidates = vec![
            ActionCandidate {
                action: ActionKind::Break,
                context_key: Some("bond:0".into()),
            },
            ActionCandidate {
                action: ActionKind::Combine,
                context_key: None,
            },
        ];
        let mut history = DecisionHistory::default();
        history.record(
            ActionKind::Combine,
            None,
            ActionConsequence {
                structural_delta: 1.0,
                ..Default::default()
            },
        );

        assert_eq!(
            select_action(
                context,
                &history,
                &candidates,
                &mut ChaCha8Rng::seed_from_u64(1),
            ),
            Some(candidates[1].clone())
        );
    }

    #[test]
    fn harmful_history_can_weaken_a_competing_action() {
        let context = DecisionContext {
            needs: CurrentNeeds {
                survival: 0.60,
                reproduction: 0.40,
                development: 0.0,
            },
            eligibility: ActionEligibility {
                can_break: true,
                can_combine: true,
                ..Default::default()
            },
        };
        let candidates = vec![
            ActionCandidate {
                action: ActionKind::Break,
                context_key: Some("bond:0".into()),
            },
            ActionCandidate {
                action: ActionKind::Combine,
                context_key: None,
            },
        ];
        let mut history = DecisionHistory::default();
        history.record(
            ActionKind::Break,
            Some("bond:0".into()),
            ActionConsequence {
                energy_delta: -1.0,
                stress_delta: 1.0,
                ..Default::default()
            },
        );

        assert_eq!(
            select_action(
                context,
                &history,
                &candidates,
                &mut ChaCha8Rng::seed_from_u64(1),
            ),
            Some(candidates[1].clone())
        );
    }

    #[test]
    fn zero_pressure_candidates_are_rejected_as_irrelevant() {
        let context = DecisionContext {
            needs: CurrentNeeds::default(),
            eligibility: ActionEligibility {
                can_break: true,
                can_combine: true,
                ..Default::default()
            },
        };
        let history = DecisionHistory::default();
        let candidates = vec![
            ActionCandidate {
                action: ActionKind::Break,
                context_key: Some("bond:0".into()),
            },
            ActionCandidate {
                action: ActionKind::Combine,
                context_key: None,
            },
        ];

        assert_eq!(
            select_action(
                context,
                &history,
                &candidates,
                &mut ChaCha8Rng::seed_from_u64(1),
            ),
            None
        );
    }

    #[test]
    fn physical_developmental_result_breaks_equal_need_tie() {
        let context = DecisionContext {
            needs: CurrentNeeds {
                survival: 0.0,
                reproduction: 0.0,
                development: 1.0,
            },
            eligibility: ActionEligibility {
                can_break: true,
                can_combine: true,
                ..Default::default()
            },
        };
        let candidates = vec![
            ActionCandidate {
                action: ActionKind::Break,
                context_key: Some("bond:0".into()),
            },
            ActionCandidate {
                action: ActionKind::Combine,
                context_key: None,
            },
        ];
        let scores = vec![Some(0.2), Some(0.8)];

        assert_eq!(
            select_action_with_developmental_scores(
                context,
                &DecisionHistory::default(),
                &candidates,
                &scores,
                &mut ChaCha8Rng::seed_from_u64(1),
            ),
            Some(candidates[1].clone())
        );
    }

    #[test]
    fn unresolved_equal_candidates_do_not_use_candidate_order() {
        let context = DecisionContext {
            needs: CurrentNeeds {
                survival: 0.0,
                reproduction: 1.0,
                development: 0.0,
            },
            eligibility: ActionEligibility {
                can_break: true,
                can_combine: true,
                ..Default::default()
            },
        };
        let candidates = vec![
            ActionCandidate {
                action: ActionKind::Break,
                context_key: Some("bond:0".into()),
            },
            ActionCandidate {
                action: ActionKind::Combine,
                context_key: None,
            },
        ];

        let selected = select_action(
            context,
            &DecisionHistory::default(),
            &candidates,
            &mut ChaCha8Rng::seed_from_u64(1),
        );
        assert!(selected.is_some_and(|candidate| candidates.contains(&candidate)));
    }

    #[test]
    fn universal_tie_breaker_handles_different_action_kinds() {
        let context = DecisionContext {
            needs: CurrentNeeds {
                survival: 1.0,
                reproduction: 0.0,
                development: 0.0,
            },
            eligibility: ActionEligibility {
                can_move: true,
                can_break: true,
                ..Default::default()
            },
        };
        let candidates = vec![
            ActionCandidate {
                action: ActionKind::Move,
                context_key: None,
            },
            ActionCandidate {
                action: ActionKind::Break,
                context_key: Some("bond:0".into()),
            },
        ];
        let mut first_rng = ChaCha8Rng::seed_from_u64(7);
        let mut second_rng = ChaCha8Rng::seed_from_u64(7);
        let first = select_action(
            context,
            &DecisionHistory::default(),
            &candidates,
            &mut first_rng,
        );
        let second = select_action(
            context,
            &DecisionHistory::default(),
            &candidates,
            &mut second_rng,
        );
        assert_eq!(first, second);
        assert!(first.is_some_and(|candidate| candidates.contains(&candidate)));
    }

    #[test]
    fn recorded_consequence_is_available_to_future_decisions() {
        let mut history = DecisionHistory::default();
        let candidate = ActionCandidate {
            action: ActionKind::Break,
            context_key: Some("Methane".into()),
        };
        record_consequence(
            &mut history,
            &candidate,
            ActionConsequence {
                energy_delta: 1.0,
                ..Default::default()
            },
        );
        assert!(known_consequence(
            &history,
            ActionKind::Break,
            Some("Methane")
        ));
    }

    #[test]
    fn no_transaction_is_always_an_approved_option() {
        let context = DecisionContext {
            needs: CurrentNeeds::default(),
            eligibility: ActionEligibility::default(),
        };
        assert_eq!(
            approve(context, ActionKind::NoTransaction),
            DecisionResult::Approve
        );
    }

    #[test]
    fn action_need_mapping_is_owned_by_decision_layer() {
        assert!(ActionKind::Combine
            .relevant_needs()
            .contains(&NeedKind::Reproduction));
        assert!(ActionKind::Break
            .relevant_needs()
            .contains(&NeedKind::Survival));
        assert!(ActionKind::Break
            .relevant_needs()
            .contains(&NeedKind::Reproduction));
    }
}
