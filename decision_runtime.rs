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
        .fold(0.0, f64::max)
}

fn history_adjustment(
    history: &DecisionHistory,
    candidate: &ActionCandidate,
) -> f64 {
    match history.outcome(candidate.action, candidate.context_key.as_deref()) {
        Some(OutcomeKind::Beneficial) => HISTORY_INFLUENCE,
        Some(OutcomeKind::Harmful) => -HISTORY_INFLUENCE,
        Some(OutcomeKind::Neutral) | None => 0.0,
    }
}

/// Select exactly one approved action from candidates.
///
/// Current need pressure provides the primary action relevance. Recorded
/// consequences can strengthen or weaken that pressure, but the decision
/// layer never predicts an unobserved physical outcome. Ties remain stable by
/// candidate order so seeded simulations stay deterministic.
pub fn select_action(
    context: DecisionContext,
    history: &DecisionHistory,
    candidates: &[ActionCandidate],
) -> Option<ActionCandidate> {
    let mut best: Option<(f64, ActionCandidate)> = None;

    for candidate in candidates {
        if approve(context, candidate.action) != DecisionResult::Approve {
            continue;
        }

        let score = need_pressure(candidate.action, context.needs)
            + history_adjustment(history, candidate);

        if best
            .as_ref()
            .map_or(true, |(best_score, _)| score > *best_score)
        {
            best = Some((score, candidate.clone()));
        }
    }

    best.map(|(_, candidate)| candidate)
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

        assert_eq!(
            select_action(context, &history, &candidates),
            Some(candidates[1].clone())
        );
    }

    #[test]
    fn beneficial_history_can_change_selection_when_need_pressure_is_close() {
        let context = DecisionContext {
            needs: CurrentNeeds {
                survival: 0.60,
                reproduction: 0.40,
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
        history.record(ActionKind::Break, Some("bond:0".into()), OutcomeKind::Harmful);

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
        assert!(!ActionKind::Break
            .relevant_needs()
            .contains(&NeedKind::Reproduction));
    }
}
