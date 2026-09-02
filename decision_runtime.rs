//! Runtime bridge for the decision architecture.
//!
//! This module contains no chemistry or structural math. It connects organism
//! state to the decision policy and returns an action candidate for simulation
//! to execute through its existing physical systems.

use crate::decision::{
    approve_action_for_current_needs, outcome_is_known, ActionEligibility, ActionKind,
    CurrentNeeds, DecisionHistory, DecisionResult, OutcomeKind,
};

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

/// Select one action from candidates that have already been generated. The
/// ranking remains the existing runtime behavior; need pressure itself is not
/// converted into a utility score here.
pub fn select_action(
    context: DecisionContext,
    history: &DecisionHistory,
    candidates: &[ActionCandidate],
) -> Option<ActionCandidate> {
    let mut best: Option<(u8, ActionCandidate)> = None;
    for candidate in candidates {
        if approve(context, candidate.action) != DecisionResult::Approve {
            continue;
        }
        let rank = match history.outcome(candidate.action, candidate.context_key.as_deref()) {
            Some(OutcomeKind::Beneficial) => 2,
            Some(OutcomeKind::Neutral) | Some(OutcomeKind::Harmful) => 0,
            None => 1,
        };
        if best
            .as_ref()
            .map_or(true, |(best_rank, _)| rank > *best_rank)
        {
            best = Some((rank, candidate.clone()));
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
    fn unknown_action_can_be_selected_when_needed_and_eligible() {
        let history = DecisionHistory::default();
        let candidates = vec![ActionCandidate {
            action: ActionKind::Break,
            context_key: Some("Methane".into()),
        }];
        assert_eq!(
            select_action(context(), &history, &candidates),
            Some(candidates[0].clone())
        );
    }

    #[test]
    fn known_beneficial_action_is_preferred_to_unknown_action() {
        let mut history = DecisionHistory::default();
        history.record(ActionKind::Move, None, OutcomeKind::Beneficial);
        let candidates = vec![
            ActionCandidate {
                action: ActionKind::Break,
                context_key: Some("Methane".into()),
            },
            ActionCandidate {
                action: ActionKind::Move,
                context_key: None,
            },
        ];
        assert_eq!(
            select_action(context(), &history, &candidates),
            Some(candidates[1].clone())
        );
    }

    #[test]
    fn harmful_history_does_not_invent_a_better_prediction_for_unknown_action() {
        let mut history = DecisionHistory::default();
        history.record(
            ActionKind::Break,
            Some("Methane".into()),
            OutcomeKind::Harmful,
        );
        let candidates = vec![
            ActionCandidate {
                action: ActionKind::Break,
                context_key: Some("Methane".into()),
            },
            ActionCandidate {
                action: ActionKind::Move,
                context_key: None,
            },
        ];
        assert_eq!(
            select_action(context(), &history, &candidates),
            Some(candidates[1].clone())
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
        assert!(!ActionKind::Break
            .relevant_needs()
            .contains(&NeedKind::Reproduction));
    }
}
