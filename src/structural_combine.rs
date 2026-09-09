//! Compatibility adapter for legacy callers.
//!
//! COMBINE physics lives in `combine.rs`. This module intentionally contains no
//! independent candidate selection, interaction, threshold, bond-strength, or
//! structural execution logic.

use crate::combine::{experimental_combine_work_cost, experimental_interaction, FormationEvaluation, ExperimentalInteraction};
use crate::resources::ResourceProperties;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StructuralCombineError {
    NonFiniteWorkCost,
    InvalidFormationThreshold,
}

/// Legacy adapter retained temporarily while callers migrate to `combine`.
/// All actual COMBINE calculations remain owned by `combine.rs`.
pub(crate) fn required_investment(
    a: ResourceProperties,
    b: ResourceProperties,
    evaluation: FormationEvaluation,
    water_field: f64,
) -> Result<(ExperimentalInteraction, f64, f64), StructuralCombineError> {
    let interaction = experimental_interaction(a, b, evaluation.candidate, water_field);
    let work = experimental_combine_work_cost(a, b, evaluation.candidate, water_field);
    if !work.is_finite() || work < 0.0 {
        return Err(StructuralCombineError::NonFiniteWorkCost);
    }
    if !evaluation.threshold.is_finite() || evaluation.threshold < 0.0 {
        return Err(StructuralCombineError::InvalidFormationThreshold);
    }
    Ok((interaction, work, evaluation.threshold))
}
