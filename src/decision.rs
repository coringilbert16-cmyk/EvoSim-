#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
//! Organism decision architecture.
//!
//! The decision layer separates physical eligibility, internal need pressure,
//! and learned consequence history. It does not calculate chemistry, geometry,
//! or predicted physical outcomes.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActionKind {
    NoTransaction,
    Move,
    Combine,
    Break,
    Expel,
}

impl ActionKind {
    /// Needs that make an action relevant. This is a relevance mapping, not
    /// an action mandate and not a utility score.
    pub fn relevant_needs(self) -> &'static [NeedKind] {
        match self {
            ActionKind::NoTransaction => &[],
            ActionKind::Move => &[
                NeedKind::Survival,
                NeedKind::Reproduction,
                NeedKind::Acquisition,
            ],
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
    /// Resource-seeking pressure derived from depleted stored material.
    Acquisition,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
pub struct ActionConsequence {
    pub energy_delta: f64,
    pub structural_delta: f64,
    pub developmental_delta: f64,
    pub stress_delta: f64,
    pub position_delta: f64,
    pub storage_delta: f64,
}

impl ActionConsequence {
    pub fn finite(self) -> bool {
        self.energy_delta.is_finite()
            && self.structural_delta.is_finite()
            && self.developmental_delta.is_finite()
            && self.stress_delta.is_finite()
            && self.position_delta.is_finite()
            && self.storage_delta.is_finite()
    }

    pub fn weighted_relevance(self, needs: CurrentNeeds) -> f64 {
        let survival = self.energy_delta - self.stress_delta + self.storage_delta;
        let development = self.developmental_delta + self.structural_delta;
        let reproduction = self.structural_delta + self.position_delta;
        let acquisition = self.storage_delta;
        let total = needs.survival + needs.development + needs.reproduction + needs.acquisition;
        if total <= 0.0 {
            return 0.0;
        }
        (needs.survival * survival
            + needs.development * development
            + needs.reproduction * reproduction
            + needs.acquisition * acquisition)
            / total
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DecisionHistoryEntry {
    pub action: ActionKind,
    pub context_key: Option<String>,
    #[serde(default)]
    pub consequence: ActionConsequence,
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
        consequence: ActionConsequence,
    ) {
        if !consequence.finite() {
            return;
        }
        if let Some(existing) = self
            .entries
            .iter_mut()
            .find(|entry| entry.action == action && entry.context_key == context_key)
        {
            let n = existing.count as f64;
            let next = n + 1.0;
            existing.consequence.energy_delta =
                (existing.consequence.energy_delta * n + consequence.energy_delta) / next;
            existing.consequence.structural_delta =
                (existing.consequence.structural_delta * n + consequence.structural_delta) / next;
            existing.consequence.developmental_delta =
                (existing.consequence.developmental_delta * n + consequence.developmental_delta)
                    / next;
            existing.consequence.stress_delta =
                (existing.consequence.stress_delta * n + consequence.stress_delta) / next;
            existing.consequence.position_delta =
                (existing.consequence.position_delta * n + consequence.position_delta) / next;
            existing.consequence.storage_delta =
                (existing.consequence.storage_delta * n + consequence.storage_delta) / next;
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

    pub fn has_knowledge(&self, action: ActionKind, context_key: Option<&str>) -> bool {
        self.consequence(action, context_key).is_some()
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
    /// Pressure to seek new environmental material when storage is depleted.
    pub acquisition: f64,
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
            NeedKind::Acquisition => self.acquisition,
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
    /// Stored material amount at or above which acquisition pressure is zero.
    pub acquisition_reserve: f64,
}

impl Default for DecisionParameters {
    fn default() -> Self {
        Self {
            survival_reserve: 1.0,
            acquisition_reserve: 3.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActionEligibility {
    pub can_move: bool,
    pub can_combine: bool,
    pub can_break: bool,
    pub can_expel: bool,
}

impl ActionEligibility {
    pub fn permits(self, action: ActionKind) -> bool {
        match action {
            ActionKind::NoTransaction => true,
            ActionKind::Move => self.can_move,
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
                ActionConsequence::default(),
            );
        }
        assert_eq!(history.entries.len(), DecisionHistory::MAX_ENTRIES);
    }
}
