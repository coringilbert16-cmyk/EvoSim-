//! Organism decision architecture.
//!
//! This module separates three things that must not be conflated:
//!
//! 1. PHYSICS / MATH: whether an action is mechanically possible and what
//!    the physical equations produce.
//! 2. NEED: the organism's current internal/situational reason to act.
//! 3. KNOWLEDGE: what the organism has learned from its own prior outcomes.
//!
//! The decision layer does not calculate COMBINE/BREAK physics and does not
//! predict an outcome that has never been experienced. It answers a simple
//! question for an action candidate: "may I choose this action now?"
//!
//! Current-needs architecture: HYBRID.
//! Immediate state supplies intrinsic needs; current environmental
//! opportunities supply situational needs; bounded outcome history supplies
//! learned relevance. None of these replaces mechanical eligibility.
//!
//! The material/recipe caches used by COMBINE are intentionally separate from
//! this history. A computational cache answers "have we already calculated
//! this physical recipe?" Decision history answers "has this organism learned
//! anything from actually doing this?" They must never be treated as the same
//! information.

use serde::{Deserialize, Serialize};

/// Actions the organism may eventually choose among.
///
/// Expel is deliberately explicit. It is not folded into BREAK or treated as
/// an automatic cleanup side effect.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActionKind {
    Move,
    Acquire,
    Combine,
    Break,
    Expel,
}

/// Why an action is currently relevant. These are decision signals, not
/// physical quantities and not energy calculations.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NeedKind {
    Energy,
    Material,
    Construction,
    Relief,
    Exploration,
}

/// A single learned consequence of an action. `Unknown` is represented by the
/// absence of a record: the organism has no invented prediction for an action
/// it has never actually experienced.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutcomeKind {
    Beneficial,
    Neutral,
    Harmful,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DecisionHistoryEntry {
    pub action: ActionKind,
    /// Optional stable material/action key. For COMBINE this can identify a
    /// material pair/recipe; for BREAK it can identify the processed material
    /// composition. A missing key means the history applies only at the broad
    /// action level.
    pub context_key: Option<String>,
    pub outcome: OutcomeKind,
    pub count: u64,
}

/// Bounded learned history. This is deliberately not an event log.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DecisionHistory {
    pub entries: Vec<DecisionHistoryEntry>,
}

impl DecisionHistory {
    pub const MAX_ENTRIES: usize = 64;

    pub fn record(
        &mut self,
        action: ActionKind,
        context_key: Option<String>,
        outcome: OutcomeKind,
    ) {
        if let Some(existing) = self.entries.iter_mut().find(|entry| {
            entry.action == action && entry.context_key == context_key
        }) {
            existing.outcome = outcome;
            existing.count = existing.count.saturating_add(1);
            return;
        }

        if self.entries.len() >= Self::MAX_ENTRIES {
            // Evict the least-experienced entry. This keeps memory bounded
            // while preserving repeatedly reinforced knowledge.
            if let Some(index) = self
                .entries
                .iter()
                .enumerate()
                .min_by_key(|(_, entry)| entry.count)
                .map(|(index, _)| index)
            {
                self.entries.remove(index);
            }
        }

        self.entries.push(DecisionHistoryEntry {
            action,
            context_key,
            outcome,
            count: 1,
        });
    }

    pub fn outcome(
        &self,
        action: ActionKind,
        context_key: Option<&str>,
    ) -> Option<OutcomeKind> {
        self.entries
            .iter()
            .find(|entry| {
                entry.action == action
                    && entry.context_key.as_deref() == context_key
            })
            .map(|entry| entry.outcome)
            .or_else(|| {
                self.entries
                    .iter()
                    .find(|entry| {
                        entry.action == action && entry.context_key.is_none()
                    })
                    .map(|entry| entry.outcome)
            })
    }

    pub fn has_knowledge(
        &self,
        action: ActionKind,
        context_key: Option<&str>,
    ) -> bool {
        self.outcome(action, context_key).is_some()
    }
}

/// Current needs are a hybrid of internal state and current opportunity.
///
/// The fields are intentionally booleans. The decision layer does not score
/// physical equations or use COMBINE/BREAK numerical outputs to decide.
/// Thresholds and the exact derivation of these flags belong to the organism
/// state layer and can remain tunable without changing the decision API.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CurrentNeeds {
    pub energy: bool,
    pub material: bool,
    pub construction: bool,
    pub relief: bool,
    pub exploration: bool,
}

impl CurrentNeeds {
    pub fn contains(self, need: NeedKind) -> bool {
        match need {
            NeedKind::Energy => self.energy,
            NeedKind::Material => self.material,
            NeedKind::Construction => self.construction,
            NeedKind::Relief => self.relief,
            NeedKind::Exploration => self.exploration,
        }
    }
}

/// Tunable decision parameters. These are policy parameters, not physics
/// constants. They may be changed for experiments without changing the
/// underlying COMBINE/BREAK equations.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct DecisionParameters {
    pub energy_need_threshold: f64,
    pub raw_material_need_threshold: f64,
    pub stress_relief_threshold: f64,
    pub construction_material_threshold: f64,
}

impl Default for DecisionParameters {
    fn default() -> Self {
        Self {
            energy_need_threshold: 1.0,
            raw_material_need_threshold: 1.0,
            stress_relief_threshold: 1.0,
            construction_material_threshold: 1.0,
        }
    }
}

/// Mechanical eligibility supplied by the physical systems.
///
/// This is intentionally a set of facts rather than calculations. For
/// example, COMBINE geometry and threshold evaluation happen elsewhere; this
/// structure merely tells the decision layer whether an eligible candidate
/// currently exists.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActionEligibility {
    pub can_move: bool,
    pub can_acquire: bool,
    pub can_combine: bool,
    pub can_break: bool,
    pub can_expel: bool,
}

impl ActionEligibility {
    pub fn permits(self, action: ActionKind) -> bool {
        match action {
            ActionKind::Move => self.can_move,
            ActionKind::Acquire => self.can_acquire,
            ActionKind::Combine => self.can_combine,
            ActionKind::Break => self.can_break,
            ActionKind::Expel => self.can_expel,
        }
    }
}

/// The decision returned for one candidate action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecisionResult {
    Approve,
    Reject,
}

/// The decision engine is deliberately small. It does not know the physical
/// equations. It only gates an action by mechanical eligibility and current
/// need. History is exposed separately so future action-selection code can
/// prefer known-positive experiences without pretending that an unseen action
/// has a known consequence.
pub fn approve_action(
    action: ActionKind,
    eligibility: ActionEligibility,
    needs: CurrentNeeds,
    required_need: NeedKind,
) -> DecisionResult {
    if !eligibility.permits(action) {
        return DecisionResult::Reject;
    }

    if needs.contains(required_need) {
        DecisionResult::Approve
    } else {
        DecisionResult::Reject
    }
}

/// Whether an outcome is known for this exact action/context. `false` means
/// the organism must not be given a fabricated expectation of the result.
pub fn outcome_is_known(
    history: &DecisionHistory,
    action: ActionKind,
    context_key: Option<&str>,
) -> bool {
    history.has_knowledge(action, context_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mechanically_ineligible_action_is_rejected_even_when_needed() {
        let eligibility = ActionEligibility {
            can_combine: false,
            ..ActionEligibility::default()
        };
        let needs = CurrentNeeds {
            construction: true,
            ..CurrentNeeds::default()
        };
        assert_eq!(
            approve_action(ActionKind::Combine, eligibility, needs, NeedKind::Construction),
            DecisionResult::Reject
        );
    }

    #[test]
    fn mechanically_eligible_needed_action_is_approved() {
        let eligibility = ActionEligibility {
            can_break: true,
            ..ActionEligibility::default()
        };
        let needs = CurrentNeeds {
            energy: true,
            ..CurrentNeeds::default()
        };
        assert_eq!(
            approve_action(ActionKind::Break, eligibility, needs, NeedKind::Energy),
            DecisionResult::Approve
        );
    }

    #[test]
    fn action_without_current_need_is_rejected() {
        let eligibility = ActionEligibility {
            can_expel: true,
            ..ActionEligibility::default()
        };
        assert_eq!(
            approve_action(
                ActionKind::Expel,
                eligibility,
                CurrentNeeds::default(),
                NeedKind::Relief,
            ),
            DecisionResult::Reject
        );
    }

    #[test]
    fn unseen_action_has_no_fabricated_outcome() {
        let history = DecisionHistory::default();
        assert!(!outcome_is_known(
            &history,
            ActionKind::Combine,
            Some("Carbon+Methane"),
        ));
    }

    #[test]
    fn exact_context_knowledge_is_preferred_over_broad_action_knowledge() {
        let mut history = DecisionHistory::default();
        history.record(ActionKind::Combine, None, OutcomeKind::Neutral);
        history.record(
            ActionKind::Combine,
            Some("Carbon+Methane".into()),
            OutcomeKind::Beneficial,
        );

        assert_eq!(
            history.outcome(ActionKind::Combine, Some("Carbon+Methane")),
            Some(OutcomeKind::Beneficial)
        );
        assert_eq!(
            history.outcome(ActionKind::Combine, Some("Hydrogen+Carbon")),
            Some(OutcomeKind::Neutral)
        );
    }

    #[test]
    fn history_is_bounded() {
        let mut history = DecisionHistory::default();
        for i in 0..(DecisionHistory::MAX_ENTRIES + 10) {
            history.record(
                ActionKind::Break,
                Some(format!("material-{i}")),
                OutcomeKind::Neutral,
            );
        }
        assert_eq!(history.entries.len(), DecisionHistory::MAX_ENTRIES);
    }
}
