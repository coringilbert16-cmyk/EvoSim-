//! Runtime bridge for the decision architecture.
//! Learned consequence history can influence selection, while need pressure remains primary.
use crate::decision::{approve_action_for_current_needs, outcome_is_known, ActionEligibility, ActionKind, CurrentNeeds, DecisionHistory, DecisionResult, OutcomeKind};
#[derive(Clone,Copy,Debug)] pub struct DecisionContext{pub needs:CurrentNeeds,pub eligibility:ActionEligibility}
#[derive(Clone,Debug,PartialEq,Eq)] pub struct ActionCandidate{pub action:ActionKind,pub context_key:Option<String>}
pub fn approve(context:DecisionContext,action:ActionKind)->DecisionResult{approve_action_for_current_needs(action,context.eligibility,context.needs)}
fn need_pressure(action:ActionKind,needs:CurrentNeeds)->f64{action.relevant_needs().iter().map(|n|needs.pressure(*n)).fold(0.0_f64,|best,p|best.max(p))}
pub fn select_action(context:DecisionContext,history:&DecisionHistory,candidates:&[ActionCandidate])->Option<ActionCandidate>{let mut best=None;for candidate in candidates{if approve(context,candidate.action)!=DecisionResult::Approve{continue}let learned=match history.outcome(candidate.action,candidate.context_key.as_deref()){Some(OutcomeKind::Beneficial)=>0.25,Some(OutcomeKind::Harmful)=>-0.25,_=>0.0};let score=need_pressure(candidate.action,context.needs)+learned;if best.as_ref().map_or(true,|(s,_)|score>*s){best=Some((score,candidate.clone()));}}best.map(|(_,c)|c)}
pub fn record_outcome(history:&mut DecisionHistory,candidate:&ActionCandidate,outcome:OutcomeKind){history.record(candidate.action,candidate.context_key.clone(),outcome)}
pub fn known_outcome(history:&DecisionHistory,action:ActionKind,context_key:Option<&str>)->bool{outcome_is_known(history,action,context_key)}
