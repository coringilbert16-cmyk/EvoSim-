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

    pub fn record(&mut self, action: ActionKind, context_key: Option<String>, outcome: OutcomeKind) {
        if let Some(existing) = self.entries.iter_mut().find(|entry| entry.action == action && entry.context_key == context_key) {
            existing.outcome = outcome;
            existing.count = existing.count.saturating_add(1);
            return;
        }
        if self.entries.len() >= Self::MAX_ENTRIES {
            if let Some(index) = self.entries.iter().enumerate().min_by_key(|(_, entry)| entry.count).map(|(index, _)| index) {
                self.entries.remove(index);
            }
        }
        self.entries.push(DecisionHistoryEntry { action, context_key, outcome, count: 1 });
    }

    pub fn outcome(&self, action: ActionKind, context_key: Option<&str>) -> Option<OutcomeKind> {
        self.entries.iter().find(|entry| entry.action == action && entry.context_key.as_deref() == context_key).map(|entry| entry.outcome).or_else(|| {
            self.entries.iter().find(|entry| entry.action == action && entry.context_key.is_none()).map(|entry| entry.outcome)
        })
    }

    pub fn has_knowledge(&self, action: ActionKind, context_key: Option<&str>) -> bool { self.outcome(action, context_key).is_some() }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
pub struct CurrentNeeds {
    pub survival: f64,
    pub reproduction: f64,
}

impl CurrentNeeds {
    pub fn contains(self, need: NeedKind) -> bool { self.pressure(need) > 0.0 }
    pub fn pressure(self, need: NeedKind) -> f64 {
        match need {
            NeedKind::Survival => self.survival,
            NeedKind::Reproduction => self.reproduction,
        }
    }
    pub fn any_for(self, needs: &[NeedKind]) -> bool { needs.iter().copied().any(|need| self.pressure(need) > 0.0) }
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
}

impl Default for DecisionParameters {
    fn default() -> Self {
        Self {
            survival_reserve: 1.0,
            reproduction_reserve: 16.0,
            reproduction_accumulation_rate: 0.01,
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