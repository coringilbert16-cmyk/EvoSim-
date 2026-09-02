//! Organism decision architecture.
//!
//! The decision layer separates physical eligibility, internal need pressure,
//! and learned consequence history. It does not calculate chemistry, geometry,
//! or predicted physical outcomes.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActionKind {
    Move,
    Acquire,
    Combine,
    Break,
    Expel,
}

impl ActionKind {
    /// Needs that make an action relevant. This is a relevance mapping, not
    /// an action mandate and not a utility score.
    pub fn relevant_needs(self) -> &'static [NeedKind] {
        match self {
            ActionKind::Move => &[NeedKind::Survival, NeedKind::Reproduction],
            ActionKind::Acquire => &[NeedKind::Survival, NeedKind::Reproduction],
            ActionKind::Combine => &[NeedKind::Reproduction],
            ActionKind::Break => &[NeedKind::Survival],
            ActionKind::Expel => &[NeedKind::Survival],
        }
    }
}

/// The two continuous internal pressures used by the decision system.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NeedKind {
    Survival,
    Reproduction,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutcomeKind {
    Beneficial,
    Neutral,
    Harmful,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DecisionHistoryEntry {
    pub action: ActionKind,
    pub context_key: Option<String>,
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
            outcome,
            count: 1,
        });
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
}

impl CurrentNeeds {
    pub fn contains(self, need: NeedKind) -> bool {
        self.pressure(need) > 0.0
    }

    pub fn pressure(self, need: NeedKind) -> f64 {
        match need {
            NeedKind::Survival => self.survival,
            NeedKind::Reproduction => self.reproduction,
        }
    }

    pub fn any_for(self, needs: &[NeedKind]) -> bool {
        needs
            .iter()
            .copied()
            .any(|need| self.pressure(need) > 0.0)
    }
}

/// Parameters governing the derivation of current need pressures. These are
/// decision-layer policy parameters, not chemistry constants.
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct DecisionParameters {
    /// Immediate usable-energy reserve at which survival pressure reaches 0.
    pub survival_reserve: f64,
    /// Usable energy at which a mature organism has full energetic readiness
    /// for reproduction.
    pub reproduction_reserve: f64,
    /// Fraction of reproductive readiness accumulated per tick under fully
    /// mature, fully energy-ready conditions.
    pub reproduction_accumulation_rate: f64,
    /// Structural mass at which maturity reaches 1.0.
    pub adult_mass: f64,
}

impl Default for DecisionParameters {
    fn default() -> Self {
        Self {
            survival_reserve: 1.0,
            reproduction_reserve: 16.0,
            reproduction_accumulation_rate: 0.01,
            adult_mass: 16.0,
        }
    }
}

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
    if !eligibility.permits(action) || !needs.any_for(action.relevant_needs()) {
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
    fn mechanically_ineligible_action_is_rejected_even_when_needed() {
        let eligibility = ActionEligibility::default();
        let needs = CurrentNeeds {
            survival: 1.0,
            reproduction: 0.0,
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
        };
        assert_eq!(
            approve_action_for_current_needs(ActionKind::Break, eligibility, needs),
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
        };
        assert_eq!(
            approve_action_for_current_needs(ActionKind::Combine, eligibility, needs),
            DecisionResult::Approve
        );
    }

    #[test]
    fn move_and_acquire_are_relevant_to_either_need() {
        let eligibility = ActionEligibility {
            can_move: true,
            can_acquire: true,
            ..Default::default()
        };
        let survival_only = CurrentNeeds {
            survival: 0.5,
            reproduction: 0.0,
        };
        let reproduction_only = CurrentNeeds {
            survival: 0.0,
            reproduction: 0.5,
        };
        assert_eq!(
            approve_action_for_current_needs(ActionKind::Move, eligibility, survival_only),
            DecisionResult::Approve
        );
        assert_eq!(
            approve_action_for_current_needs(ActionKind::Acquire, eligibility, reproduction_only),
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
            approve_action_for_current_needs(ActionKind::Break, eligibility, CurrentNeeds::default()),
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
