//! Runtime bridge for the decision architecture.
//!
//! This module deliberately contains no chemistry or structural math. It
//! provides the small boundary that simulation code can call when deciding
//! whether an already-identified action candidate may be attempted.
//!
//! Decision approval is separate from mechanical execution. The caller must
//! supply mechanical eligibility facts produced by the physical subsystem.

use crate::decision::{
    approve_action_for_current_needs, outcome_is_known, ActionEligibility,
    ActionKind, CurrentNeeds, DecisionHistory, DecisionResult,
};

/// Context passed from simulation state into the decision boundary.
///
/// `needs` is produced by the organism-state/need layer. `eligibility` is
/// produced by the physical subsystem. Keeping both explicit prevents the
/// decision layer from quietly calculating chemistry or geometry itself.
#[derive(Clone, Copy, Debug)]
pub struct DecisionContext {
    pub needs: CurrentNeeds,
    pub eligibility: ActionEligibility,
}

/// Apply the decision gate to an action candidate.
///
/// This is intentionally a yes/no operation. It does not score chemistry,
/// predict outcomes, or replace mechanical eligibility checks.
pub fn approve(context: DecisionContext, action: ActionKind) -> DecisionResult {
    approve_action_for_current_needs(action, context.eligibility, context.needs)
}

/// Returns whether the organism has actually learned an outcome for this
/// action/context. `false` means unknown; callers must not fabricate an
/// expected result from the physical equations.
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

    #[test]
    fn bridge_approves_needed_mechanically_eligible_action() {
        let context = DecisionContext {
            needs: CurrentNeeds {
                construction: true,
                ..CurrentNeeds::default()
            },
            eligibility: ActionEligibility {
                can_combine: true,
                ..ActionEligibility::default()
            },
        };

        assert_eq!(approve(context, ActionKind::Combine), DecisionResult::Approve);
    }

    #[test]
    fn bridge_rejects_mechanically_ineligible_action() {
        let context = DecisionContext {
            needs: CurrentNeeds {
                energy: true,
                ..CurrentNeeds::default()
            },
            eligibility: ActionEligibility::default(),
        };

        assert_eq!(approve(context, ActionKind::Break), DecisionResult::Reject);
    }

    #[test]
    fn bridge_does_not_invent_unknown_history() {
        let history = DecisionHistory::default();
        assert!(!known_outcome(
            &history,
            ActionKind::Break,
            Some("Methane"),
        ));
    }

    #[test]
    fn action_need_mapping_remains_owned_by_decision_layer() {
        assert!(ActionKind::Combine.relevant_needs().contains(&NeedKind::Construction));
    }
}
