//! Organism decision architecture.
//! Learned consequence history is retained separately from physical state.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActionKind { Move, Acquire, Combine, Break, Expel }
impl ActionKind { pub fn relevant_needs(self) -> &'static [NeedKind] { match self { ActionKind::Move | ActionKind::Acquire => &[NeedKind::Survival, NeedKind::Reproduction], ActionKind::Combine => &[NeedKind::Reproduction], ActionKind::Break | ActionKind::Expel => &[NeedKind::Survival] } } }
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NeedKind { Survival, Reproduction }
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutcomeKind { Beneficial, Neutral, Harmful }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DecisionHistoryEntry { pub action: ActionKind, pub context_key: Option<String>, pub outcome: OutcomeKind, pub count: u64, #[serde(default)] pub beneficial_count: u64, #[serde(default)] pub neutral_count: u64, #[serde(default)] pub harmful_count: u64 }
impl DecisionHistoryEntry {
    fn record_outcome(&mut self, outcome: OutcomeKind) { self.count = self.count.saturating_add(1); self.outcome = outcome; match outcome { OutcomeKind::Beneficial => self.beneficial_count = self.beneficial_count.saturating_add(1), OutcomeKind::Neutral => self.neutral_count = self.neutral_count.saturating_add(1), OutcomeKind::Harmful => self.harmful_count = self.harmful_count.saturating_add(1) } }
    fn learned_outcome(&self) -> OutcomeKind { if self.beneficial_count == 0 && self.neutral_count == 0 && self.harmful_count == 0 { return self.outcome; } if self.beneficial_count > self.harmful_count && self.beneficial_count >= self.neutral_count { OutcomeKind::Beneficial } else if self.harmful_count > self.beneficial_count && self.harmful_count >= self.neutral_count { OutcomeKind::Harmful } else { OutcomeKind::Neutral } }
}
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DecisionHistory { pub entries: Vec<DecisionHistoryEntry> }
impl DecisionHistory {
    pub const MAX_ENTRIES: usize = 64;
    pub fn record(&mut self, action: ActionKind, context_key: Option<String>, outcome: OutcomeKind) { if let Some(existing) = self.entries.iter_mut().find(|e| e.action == action && e.context_key == context_key) { existing.record_outcome(outcome); return; } if self.entries.len() >= Self::MAX_ENTRIES { if let Some(i) = self.entries.iter().enumerate().min_by_key(|(_, e)| e.count).map(|(i, _)| i) { self.entries.remove(i); } } let mut entry = DecisionHistoryEntry { action, context_key, outcome, count: 0, beneficial_count: 0, neutral_count: 0, harmful_count: 0 }; entry.record_outcome(outcome); self.entries.push(entry); }
    pub fn outcome(&self, action: ActionKind, context_key: Option<&str>) -> Option<OutcomeKind> { self.entries.iter().find(|e| e.action == action && e.context_key.as_deref() == context_key).map(DecisionHistoryEntry::learned_outcome).or_else(|| self.entries.iter().find(|e| e.action == action && e.context_key.is_none()).map(DecisionHistoryEntry::learned_outcome)) }
    pub fn has_knowledge(&self, action: ActionKind, context_key: Option<&str>) -> bool { self.outcome(action, context_key).is_some() }
}
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq)]
pub struct CurrentNeeds { pub survival: f64, pub reproduction: f64 }
impl CurrentNeeds { pub fn contains(self, need: NeedKind) -> bool { self.pressure(need) > 0.0 } pub fn pressure(self, need: NeedKind) -> f64 { match need { NeedKind::Survival => self.survival, NeedKind::Reproduction => self.reproduction } } pub fn any_for(self, needs: &[NeedKind]) -> bool { needs.iter().copied().any(|n| self.pressure(n) > 0.0) } }
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct DecisionParameters { pub survival_reserve: f64, pub reproduction_reserve: f64, pub reproduction_accumulation_rate: f64, pub adult_mass: f64 }
impl Default for DecisionParameters { fn default() -> Self { Self { survival_reserve: 1.0, reproduction_reserve: 16.0, reproduction_accumulation_rate: 0.01, adult_mass: 16.0 } } }
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActionEligibility { pub can_move: bool, pub can_acquire: bool, pub can_combine: bool, pub can_break: bool, pub can_expel: bool }
impl ActionEligibility { pub fn permits(self, action: ActionKind) -> bool { match action { ActionKind::Move => self.can_move, ActionKind::Acquire => self.can_acquire, ActionKind::Combine => self.can_combine, ActionKind::Break => self.can_break, ActionKind::Expel => self.can_expel } } }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecisionResult { Approve, Reject }
pub fn approve_action(action: ActionKind, eligibility: ActionEligibility, needs: CurrentNeeds, required_need: NeedKind) -> DecisionResult { if !eligibility.permits(action) || needs.pressure(required_need) <= 0.0 { DecisionResult::Reject } else { DecisionResult::Approve } }
pub fn approve_action_for_current_needs(action: ActionKind, eligibility: ActionEligibility, needs: CurrentNeeds) -> DecisionResult { if !eligibility.permits(action) || !needs.any_for(action.relevant_needs()) { DecisionResult::Reject } else { DecisionResult::Approve } }
pub fn outcome_is_known(history: &DecisionHistory, action: ActionKind, context_key: Option<&str>) -> bool { history.has_knowledge(action, context_key) }
#[cfg(test)] mod tests { use super::*; #[test] fn history_accumulates_outcomes(){let mut h=DecisionHistory::default();h.record(ActionKind::Break,Some("bond:stable".into()),OutcomeKind::Beneficial);h.record(ActionKind::Break,Some("bond:stable".into()),OutcomeKind::Beneficial);h.record(ActionKind::Break,Some("bond:stable".into()),OutcomeKind::Harmful);let e=&h.entries[0];assert_eq!(e.count,3);assert_eq!(e.beneficial_count,2);assert_eq!(e.harmful_count,1);assert_eq!(h.outcome(ActionKind::Break,Some("bond:stable")),Some(OutcomeKind::Beneficial));} #[test] fn history_records_neutral(){let mut h=DecisionHistory::default();h.record(ActionKind::Move,None,OutcomeKind::Neutral);assert_eq!(h.entries[0].neutral_count,1);} #[test] fn mechanically_ineligible_action_is_rejected_even_when_needed(){let e=ActionEligibility::default();let n=CurrentNeeds{survival:1.0,reproduction:0.0};assert_eq!(approve_action_for_current_needs(ActionKind::Break,e,n),DecisionResult::Reject);} #[test] fn survival_pressure_makes_break_relevant(){let e=ActionEligibility{can_break:true,..Default::default()};let n=CurrentNeeds{survival:0.5,reproduction:0.0};assert_eq!(approve_action_for_current_needs(ActionKind::Break,e,n),DecisionResult::Approve);} #[test] fn reproduction_pressure_makes_combine_relevant(){let e=ActionEligibility{can_combine:true,..Default::default()};let n=CurrentNeeds{survival:0.0,reproduction:0.5};assert_eq!(approve_action_for_current_needs(ActionKind::Combine,e,n),DecisionResult::Approve);} #[test] fn move_and_acquire_are_relevant_to_either_need(){let e=ActionEligibility{can_move:true,can_acquire:true,..Default::default()};let s=CurrentNeeds{survival:0.5,reproduction:0.0};let r=CurrentNeeds{survival:0.0,reproduction:0.5};assert_eq!(approve_action_for_current_needs(ActionKind::Move,e,s),DecisionResult::Approve);assert_eq!(approve_action_for_current_needs(ActionKind::Acquire,e,r),DecisionResult::Approve);} #[test] fn zero_pressure_does_not_make_a_need_relevant(){let e=ActionEligibility{can_break:true,..Default::default()};assert_eq!(approve_action_for_current_needs(ActionKind::Break,e,CurrentNeeds::default()),DecisionResult::Reject);} #[test] fn unknown_history_does_not_invent_an_outcome(){let h=DecisionHistory::default();assert!(!outcome_is_known(&h,ActionKind::Combine,Some("Carbon+Methane")));} #[test] fn history_is_bounded(){let mut h=DecisionHistory::default();for i in 0..(DecisionHistory::MAX_ENTRIES+10){h.record(ActionKind::Break,Some(format!("material-{i}")),OutcomeKind::Neutral);}assert_eq!(h.entries.len(),DecisionHistory::MAX_ENTRIES);} }
