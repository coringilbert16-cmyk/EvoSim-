#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
//! Organism decision architecture.
//!
//! The decision layer separates physical eligibility, internal need pressure,
//! and learned consequence history. It does not calculate chemistry, geometry,
//! or predicted physical outcomes.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActionKind {
    Move,
    Combine,
    Break,
    Expel,
    NoTransaction,
}

impl ActionKind {
    /// Needs that make an action relevant. This is a relevance mapping, not
    /// an action mandate and not a utility score.
    pub fn relevant_needs(self) -> &'static [NeedKind] {
        match self {
            ActionKind::Move => &[NeedKind::Survival, NeedKind::Reproduction],
            ActionKind::Combine => &[
                NeedKind::Survival,
                NeedKind::Reproduction,
                NeedKind::Development,
            ],
            ActionKind::Break => &[
                NeedKind::Survival,
                NeedKind::Reproduction,
                NeedKind::Development,
            ],
            ActionKind::Expel => &[NeedKind::Survival],
            ActionKind::NoTransaction => &[
                NeedKind::Survival,
                NeedKind::Reproduction,
                NeedKind::Development,
            ],
        }
    }
}

/// The two true biological needs used by the decision system are survival and reproduction.
/// Development remains a juvenile developmental pressure so juveniles can advance toward
/// adulthood before reproduction is biologically available.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NeedKind {
    Survival,
    Reproduction,
    Development,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutcomeKind {
    Beneficial,
    Neutral,
    Harmful,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
pub struct ActionConsequence {
    /// Signed energetic consequence, normalized to [-1, 1].
    pub energy: f64,
    /// Signed structural/connectivity consequence, normalized to [-1, 1].
    pub structure: f64,
    /// Signed developmental consequence, normalized to [-1, 1].
    pub development: f64,
}

impl ActionConsequence {
    pub const NONE: Self = Self {
        energy: 0.0,
        structure: 0.0,
        development: 0.0,
    };

    pub fn contextual_value(self, needs: CurrentNeeds) -> f64 {
        let survival = (self.energy + 0.25 * self.structure) * needs.survival;
        let development = self.development * (needs.development + needs.reproduction);
        survival + development
    }

    pub fn outcome(self) -> OutcomeKind {
        let sum = self.energy + self.structure + self.development;
        if sum > f64::EPSILON {
            OutcomeKind::Beneficial
        } else if sum < -f64::EPSILON {
            OutcomeKind::Harmful
        } else {
            OutcomeKind::Neutral
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DecisionHistoryEntry {
    pub action: ActionKind,
    pub context_key: Option<String>,
    #[serde(default)]
    pub consequence: ActionConsequence,
    /// Compatibility summary for older serialized histories.
    pub outcome: OutcomeKind,
    pub count: u64,
}

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
        if let Some(existing) = self
            .entries
            .iter_mut()
            .find(|entry| entry.action == action && entry.context_key == context_key)
        {
            existing.outcome = outcome;
            existing.consequence = match outcome {
                OutcomeKind::Beneficial => ActionConsequence {
                    energy: 1.0,
                    structure: 0.0,
                    development: 0.0,
                },
                OutcomeKind::Harmful => ActionConsequence {
                    energy: -1.0,
                    structure: 0.0,
                    development: 0.0,
                },
                OutcomeKind::Neutral => ActionConsequence::NONE,
            };
            existing.count = existing.count.saturating_add(1);
            return;
        }
        if self.entries.len() >= Self::MAX_ENTRIES {
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
            consequence: match outcome {
                OutcomeKind::Beneficial => ActionConsequence {
                energy: 1.0,
                structure: 0.0,
                development: 0.0,
            },
                OutcomeKind::Harmful => ActionConsequence {
                energy: -1.0,
                structure: 0.0,
                development: 0.0,
            },
                OutcomeKind::Neutral => ActionConsequence::NONE,
            },
            outcome,
            count: 1,
        });
    }

    pub fn record_consequence(
        &mut self,
        action: ActionKind,
        context_key: Option<String>,
        consequence: ActionConsequence,
    ) {
        let outcome = consequence.outcome();
        if let Some(existing) = self
            .entries
            .iter_mut()
            .find(|entry| entry.action == action && entry.context_key == context_key)
        {
            existing.consequence = consequence;
            existing.outcome = outcome;
            existing.count = existing.count.saturating_add(1);
            return;
        }
        if self.entries.len() >= Self::MAX_ENTRIES {
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
            consequence,
            outcome,
            count: 1,
        });
    }

    pub fn consequence(
        &self,
        action: ActionKind,
        context_key: Option<&str>,
    ) -> Option<ActionConsequence> {
        self.entries
            .iter()
            .find(|entry| entry.action == action && entry.context_key.as_deref() == context_key)
            .map(|entry| entry.consequence)
            .or_else(|| {
                self.entries
                    .iter()
                    .find(|entry| entry.action == action && entry.context_key.is_none())
                    .map(|entry| entry.consequence)
            })
    }

    pub fn outcome(&self, action: ActionKind, context_key: Option<&str>) -> Option<OutcomeKind> {
        self.entries
            .iter()
            .find(|entry| entry.action == action && entry.context_key.as_deref() == context_key)
            .map(|entry| entry.outcome)
            .or_else(|| {
                self.entries
                    .iter()
                    .find(|entry| entry.action == action && entry.context_key.is_none())
                    .map(|entry| entry.outcome)
            })
    }

    pub fn has_knowledge(&self, action: ActionKind, context_key: Option<&str>) -> bool {
        self.outcome(action, context_key).is_some()
    }
}

/// Continuous current pressures, each independently derived from organism
/// state. They are not forced to sum to one and may both be high or low.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
pub struct CurrentNeeds {
    pub survival: f64,
    pub reproduction: f64,
    /// Juvenile developmental pressure. This is a developmental drive, not a third
    /// true biological need; adults have zero developmental pressure.
    pub development: f64,
}

impl CurrentNeeds {
    pub fn contains(self, need: NeedKind) -> bool {
        self.pressure(need) > 0.0
    }

    pub fn pressure(self, need: NeedKind) -> f64 {
        match need {
            NeedKind::Survival => self.survival,
            NeedKind::Reproduction => self.reproduction,
            NeedKind::Development => self.development,
        }
    }

    pub fn any_for(self, needs: &[NeedKind]) -> bool {
        needs.iter().copied().any(|need| self.pressure(need) > 0.0)
    }
}

/// Parameters governing the derivation of current need pressures. These are
/// decision-layer policy parameters, not chemistry constants.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct DecisionParameters {
    /// Immediate usable-energy reserve at which survival pressure reaches 0.
    pub survival_reserve: f64,
}

impl Default for DecisionParameters {
    fn default() -> Self {
        Self {
            survival_reserve: 1.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActionEligibility {
    pub can_move: bool,
    pub can_combine: bool,
    pub can_break: bool,
    pub can_expel: bool,
    pub can_no_transaction: bool,
}

impl ActionEligibility {
    pub fn permits(self, action: ActionKind) -> bool {
        match action {
            ActionKind::Move => self.can_move,
            ActionKind::Combine => self.can_combine,
            ActionKind::Break => self.can_break,
            ActionKind::Expel => self.can_expel,
            ActionKind::NoTransaction => self.can_no_transaction,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecisionResult {
    Approve,
    Reject,
}

/// Legacy single-need gate retained as a small compatibility primitive.
pub fn approve_action(
    action: ActionKind,
    eligibility: ActionEligibility,
    needs: CurrentNeeds,
    required_need: NeedKind,
) -> DecisionResult {
    if !eligibility.permits(action) || needs.pressure(required_need) <= 0.0 {
        DecisionResult::Reject
    } else {
        DecisionResult::Approve
    }
}

pub fn approve_action_for_current_needs(
    action: ActionKind,
    eligibility: ActionEligibility,
    needs: CurrentNeeds,
) -> DecisionResult {
    if !eligibility.permits(action) {
        return DecisionResult::Reject;
    }
    if action == ActionKind::NoTransaction {
        return DecisionResult::Approve;
    }
    if !needs.any_for(action.relevant_needs()) {
        DecisionResult::Reject
    } else {
        DecisionResult::Approve
    }
}

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
    fn mixed_consequence_preserves_beneficial_and_harmful_dimensions() {
        let consequence = ActionConsequence {
            energy: 1.0,
            structure: -1.0,
            development: -1.0,
        };
        assert_eq!(consequence.energy, 1.0);
        assert_eq!(consequence.structure, -1.0);
        assert_eq!(consequence.development, -1.0);
        assert_eq!(consequence.outcome(), OutcomeKind::Harmful);
    }

    #[test]
    fn no_transaction_is_a_real_eligible_action() {
        let eligibility = ActionEligibility {
            can_no_transaction: true,
            ..Default::default()
        };
        let needs = CurrentNeeds {
            survival: 1.0,
            reproduction: 0.0,
            development: 0.0,
        };
        assert_eq!(
            approve_action_for_current_needs(ActionKind::NoTransaction, eligibility, needs),
            DecisionResult::Approve
        );
    }

    #[test]
    fn mechanically_ineligible_action_is_rejected_even_when_needed() {
        let eligibility = ActionEligibility::default();
        let needs = CurrentNeeds {
            survival: 1.0,
            reproduction: 0.0,
            development: 0.0,
        };
        assert_eq!(
            approve_action_for_current_needs(ActionKind::Break, eligibility, needs),
            DecisionResult::Reject
        );
    }

    #[test]
    fn survival_pressure_makes_break_relevant() {
        let eligibility = ActionEligibility {
            can_break: true,
            ..Default::default()
        };
        let needs = CurrentNeeds {
            survival: 0.5,
            reproduction: 0.0,
            development: 0.0,
        };
        assert_eq!(
            approve_action_for_current_needs(ActionKind::Break, eligibility, needs),
            DecisionResult::Approve
        );
    }

    #[test]
    fn survival_pressure_makes_combine_relevant() {
        let eligibility = ActionEligibility {
            can_combine: true,
            ..Default::default()
        };
        let needs = CurrentNeeds {
            survival: 0.5,
            reproduction: 0.0,
            development: 0.0,
        };
        assert_eq!(
            approve_action_for_current_needs(ActionKind::Combine, eligibility, needs),
            DecisionResult::Approve
        );
    }

    #[test]
    fn reproduction_pressure_makes_combine_relevant() {
        let eligibility = ActionEligibility {
            can_combine: true,
            ..Default::default()
        };
        let needs = CurrentNeeds {
            survival: 0.0,
            reproduction: 0.5,
            development: 0.0,
        };
        assert_eq!(
            approve_action_for_current_needs(ActionKind::Combine, eligibility, needs),
            DecisionResult::Approve
        );
    }

    #[test]
    fn move_is_relevant_to_survival() {
        let eligibility = ActionEligibility {
            can_move: true,
            ..Default::default()
        };
        let needs = CurrentNeeds {
            survival: 0.5,
            ..Default::default()
        };
        assert_eq!(
            approve_action_for_current_needs(ActionKind::Move, eligibility, needs),
            DecisionResult::Approve
        );
    }

    #[test]
    fn zero_pressure_does_not_make_a_need_relevant() {
        let eligibility = ActionEligibility {
            can_break: true,
            ..Default::default()
        };
        assert_eq!(
            approve_action_for_current_needs(
                ActionKind::Break,
                eligibility,
                CurrentNeeds::default()
            ),
            DecisionResult::Reject
        );
    }

    #[test]
    fn unknown_history_does_not_invent_an_outcome() {
        let history = DecisionHistory::default();
        assert!(!outcome_is_known(
            &history,
            ActionKind::Combine,
            Some("Carbon+Methane")
        ));
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
