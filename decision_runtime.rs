//! Runtime bridge for the decision architecture.
use crate::decision::{approve_action_for_current_needs, outcome_is_known, ActionEligibility, ActionKind, CurrentNeeds, DecisionHistory, DecisionResult, OutcomeKind};
pub const HISTORY_INFLUENCE:f64=0.25;
#[derive(Clone,Copy,Debug)]pub struct DecisionContext{pub needs:CurrentNeeds,pub eligibility:ActionEligibility}
#[derive(Clone,Debug,PartialEq,Eq)]pub struct ActionCandidate{pub action:ActionKind,pub context_key:Option<String>}
pub fn approve(context:DecisionContext,action:ActionKind)->DecisionResult{approve_action_for_current_needs(action,context.eligibility,context.needs)}
fn need_pressure(action:ActionKind,needs:CurrentNeeds)->f64{action.relevant_needs().iter().map(|need|needs.pressure(*need)).fold(0.0,|best,p|best.max(p))}
fn history_adjustment(history:&DecisionHistory,candidate:&ActionCandidate)->f64{match history.outcome(candidate.action,candidate.context_key.as_deref()){Some(OutcomeKind::Beneficial)=>HISTORY_INFLUENCE,Some(OutcomeKind::Harmful)=>-HISTORY_INFLUENCE,_=>0.0}}
pub fn select_action(context:DecisionContext,history:&DecisionHistory,candidates:&[ActionCandidate])->Option<ActionCandidate>{let mut best=None;for candidate in candidates{if approve(context,candidate.action)!=DecisionResult::Approve{continue}let score=need_pressure(candidate.action,context.needs)+history_adjustment(history,candidate);if best.as_ref().map_or(true,|(s,_)|score>*s){best=Some((score,candidate.clone()));}}best.map(|(_,c)|c)}
pub fn record_outcome(history:&mut DecisionHistory,candidate:&ActionCandidate,outcome:OutcomeKind){history.record(candidate.action,candidate.context_key.clone(),outcome)}
pub fn known_outcome(history:&DecisionHistory,action:ActionKind,context_key:Option<&str>)->bool{outcome_is_known(history,action,context_key)}
