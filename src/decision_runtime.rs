#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
//! Runtime bridge for the decision architecture.
//!
//! This module contains no chemistry or structural math. It connects organism
//! state to the decision policy and returns an action candidate for simulation
//! to execute through its existing physical systems.

use crate::decision::{
    approve_action_for_current_needs, outcome_is_known, ActionEligibility, ActionKind,
    CurrentNeeds, DecisionHistory, DecisionResult, OutcomeKind,
};

/// Maximum influence a recorded consequence has on action selection.
///
/// Need pressure remains the primary driver. This value is deliberately small
/// and exposed so the decision layer can be tuned without changing chemistry
/// or physics.
pub const HISTORY_INFLUENCE: f64 = 0.25;

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
    history: &DecisionHistory,
    candidate: &ActionCandidate,
) -> Option<f64> {
    if approve(context, candidate.action) != DecisionResult::Approve {
        return None;
    }
    Some(
        need_pressure(candidate.action, context.needs)
            + history_adjustment(history, candidate),
    )
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
/// Need pressure is the primary relevance signal. When developmental pressure
/// is active and two candidates have equal need pressure, a supplied physical
/// developmental result breaks the tie before learned history is consulted.
/// Exact ties remain unresolved rather than inheriting the caller's candidate
/// ordering.
pub fn select_action(
    context: DecisionContext,
    history: &DecisionHistory,
    candidates: &[ActionCandidate],
) -> Option<ActionCandidate> {
    let scores = vec![None; candidates.len()];
    select_action_with_developmental_scores(context, history, candidates, &scores)
}

/// Select an action using physical developmental results when they are
/// available. The scores are candidate-specific results produced by the
/// physical/developmental subsystem; this module does not calculate them.
pub fn select_action_with_developmental_scores(
    context: DecisionContext,
    history: &DecisionHistory,
    candidates: &[ActionCandidate],
    developmental_scores: &[Option<f64>],
) -> Option<ActionCandidate> {
    let mut best: Option<(f64, Option<f64>, f64, ActionCandidate)> = None;
    let mut unresolved_tie = false;

    for (index, candidate) in candidates.iter().enumerate() {
        if approve(context, candidate.action) != DecisionResult::Approve {
            continue;
        }

        let need = need_pressure(candidate.action, context.needs);
        let history = history_adjustment(history, candidate);
        let decision_score = need + history;
        let developmental = developmental_scores
            .get(index)
            .copied()
            .flatten()
            .filter(|value| value.is_finite());

        match best.as_ref() {
            None => {
                best = Some((decision_score, developmental, history, candidate.clone()));
                unresolved_tie = false;
            }
            Some((best_score, best_developmental, _, best_candidate)) => {
                let ordering = if decision_score > *best_score {
                    std::cmp::Ordering::Greater
                } else if decision_score < *best_score {
                    std::cmp::Ordering::Less
                } else if context.needs.development > 0.0
                    && candidate
                        .action
                        .relevant_needs()
                        .contains(&crate::decision::NeedKind::Development)
                    && best_candidate
                        .action
                        .relevant_needs()
                        .contains(&crate::decision::NeedKind::Development)
                {
                    compare_optional_score(developmental, *best_developmental)
                } else {
                    std::cmp::Ordering::Equal
                };

                match ordering {
                    std::cmp::Ordering::Greater => {
                        best = Some((decision_score, developmental, history, candidate.clone()));
                        unresolved_tie = false;
                    }
                    std::cmp::Ordering::Equal => {
                        unresolved_tie = true;
                    }
                    std::cmp::Ordering::Less => {}
                }
            }
        }
    }

    if unresolved_tie {
        None
    } else {
        best.map(|(_, _, _, candidate)| candidate)
    }
}

fn compare_optional_score(a: Option<f64>, b: Option<f64>) -> std::cmp::Ordering {
    match (a, b) {
        (Some(a), Some(b)) => a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Greater,
        (None, Some(_)) => std::cmp::Ordering::Less,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

pub fn record_outcome(
    history: &mut DecisionHistory,
    candidate: &ActionCandidate,
    outcome: OutcomeKind,
) {
    history.record(candidate.action, candidate.context_key.clone(), outcome);
}

pub fn known_outcome(
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
            select_action(context, &history, &candidates),
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

        assert_eq!(select_action(context, &history, &candidates), None);
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
        history.record(ActionKind::Combine, None, OutcomeKind::Beneficial);

        assert_eq!(
            select_action(context, &history, &candidates),
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
            OutcomeKind::Harmful,
        );

        assert_eq!(
            select_action(context, &history, &candidates),
            Some(candidates[1].clone())
        );
    }

    #[test]
    fn no_action_is_selected_when_all_candidates_are_ineligible_or_irrelevant() {
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

        assert_eq!(select_action(context, &history, &candidates), None);
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

        assert_eq!(
            select_action(context, &DecisionHistory::default(), &candidates),
            None
        );
    }

    #[test]
    fn recorded_outcome_is_available_to_future_decisions() {
        let mut history = DecisionHistory::default();
        let candidate = ActionCandidate {
            action: ActionKind::Break,
            context_key: Some("Methane".into()),
        };
        record_outcome(&mut history, &candidate, OutcomeKind::Beneficial);
        assert!(known_outcome(&history, ActionKind::Break, Some("Methane")));
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
