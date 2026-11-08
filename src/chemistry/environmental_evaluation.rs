#![expect(
    dead_code,
    reason = "Stage 4 defines a typed evaluation envelope before production integration"
)]
//! Shared result contract for environmental reaction evaluation.
//!
//! The envelope is deliberately generic: each reaction keeps its own evidence,
//! activation representation, energy calculation, and commit payload. This
//! module provides lifecycle/coverage vocabulary only; it does not calculate a
//! common reaction score or decide chemistry policy.

/// Identifies which reaction-specific evaluator produced an assessment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EnvironmentalReactionKind {
    Combine,
    Break,
    Displacement,
}

/// The latest stage reached by one reaction-specific evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum EvaluationStage {
    CandidateValidation,
    Chemistry,
    Activation,
    EnergySettlement,
    Scheduling,
    CommitRevalidation,
}

/// Why an evaluation did not produce a commit-ready operation.
///
/// Reasons remain descriptive strings owned by the reaction-specific caller.
/// The shared contract intentionally does not collapse distinct rejection
/// policies into one global list of chemistry rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EvaluationOutcome<T> {
    /// The typed payload is ready for the existing reaction-specific scheduler.
    Ready(T),
    /// The candidate was evaluated and rejected by the named stage.
    Rejected {
        stage: EvaluationStage,
        reason: &'static str,
    },
    /// The candidate remains valid but requires more bounded work or time.
    Deferred {
        stage: EvaluationStage,
        reason: &'static str,
    },
    /// A source reference or snapshot became invalid and must be rediscovered.
    Stale { stage: EvaluationStage },
}

/// Shared envelope around a reaction-specific assessment.
///
/// `T` must remain the native payload for that reaction (for example,
/// `EnvironmentalCombineOperation`, `EnvironmentalChemicalBreakOperation`,
/// or a displacement-specific preflight/trial descriptor). There is no common
/// scalar score, activation threshold, energy unit, or settlement authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EnvironmentalReactionEvaluation<T> {
    pub(crate) pass_id: u64,
    pub(crate) reaction: EnvironmentalReactionKind,
    pub(crate) outcome: EvaluationOutcome<T>,
}

impl<T> EnvironmentalReactionEvaluation<T> {
    pub(crate) fn ready(pass_id: u64, reaction: EnvironmentalReactionKind, payload: T) -> Self {
        Self {
            pass_id,
            reaction,
            outcome: EvaluationOutcome::Ready(payload),
        }
    }

    pub(crate) fn rejected(
        pass_id: u64,
        reaction: EnvironmentalReactionKind,
        stage: EvaluationStage,
        reason: &'static str,
    ) -> Self {
        Self {
            pass_id,
            reaction,
            outcome: EvaluationOutcome::Rejected { stage, reason },
        }
    }

    pub(crate) fn deferred(
        pass_id: u64,
        reaction: EnvironmentalReactionKind,
        stage: EvaluationStage,
        reason: &'static str,
    ) -> Self {
        Self {
            pass_id,
            reaction,
            outcome: EvaluationOutcome::Deferred { stage, reason },
        }
    }

    pub(crate) fn stale(
        pass_id: u64,
        reaction: EnvironmentalReactionKind,
        stage: EvaluationStage,
    ) -> Self {
        Self {
            pass_id,
            reaction,
            outcome: EvaluationOutcome::Stale { stage },
        }
    }

    pub(crate) fn into_ready(self) -> Option<T> {
        match self.outcome {
            EvaluationOutcome::Ready(payload) => Some(payload),
            EvaluationOutcome::Rejected { .. }
            | EvaluationOutcome::Deferred { .. }
            | EvaluationOutcome::Stale { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct CombinePayload {
        endpoint_pair: (u8, u8),
        activation_key: u16,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct BreakPayload {
        bond_index: usize,
        disruption_key: u32,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct DisplacementPayload {
        removed_bond: usize,
        anchor: usize,
        incoming: usize,
        atomic_trial_key: u64,
    }

    #[test]
    fn reaction_envelope_preserves_distinct_native_payload_types() {
        let combine = EnvironmentalReactionEvaluation::ready(
            11,
            EnvironmentalReactionKind::Combine,
            CombinePayload {
                endpoint_pair: (2, 7),
                activation_key: 19,
            },
        );
        let break_assessment = EnvironmentalReactionEvaluation::ready(
            12,
            EnvironmentalReactionKind::Break,
            BreakPayload {
                bond_index: 4,
                disruption_key: 23,
            },
        );
        let displacement = EnvironmentalReactionEvaluation::ready(
            13,
            EnvironmentalReactionKind::Displacement,
            DisplacementPayload {
                removed_bond: 4,
                anchor: 1,
                incoming: 9,
                atomic_trial_key: 29,
            },
        );

        assert_eq!(combine.pass_id, 11);
        assert_eq!(combine.reaction, EnvironmentalReactionKind::Combine);
        assert_eq!(
            combine.into_ready(),
            Some(CombinePayload {
                endpoint_pair: (2, 7),
                activation_key: 19,
            })
        );
        assert_eq!(break_assessment.pass_id, 12);
        assert_eq!(
            break_assessment.into_ready(),
            Some(BreakPayload {
                bond_index: 4,
                disruption_key: 23,
            })
        );
        assert_eq!(displacement.pass_id, 13);
        assert_eq!(
            displacement.into_ready(),
            Some(DisplacementPayload {
                removed_bond: 4,
                anchor: 1,
                incoming: 9,
                atomic_trial_key: 29,
            })
        );
    }

    #[test]
    fn deferred_rejected_and_stale_are_not_ready_operations() {
        let deferred: EnvironmentalReactionEvaluation<u8> =
            EnvironmentalReactionEvaluation::deferred(
                3,
                EnvironmentalReactionKind::Break,
                EvaluationStage::Activation,
                "activation_accumulation_incomplete",
            );
        let rejected: EnvironmentalReactionEvaluation<u8> =
            EnvironmentalReactionEvaluation::rejected(
                3,
                EnvironmentalReactionKind::Displacement,
                EvaluationStage::EnergySettlement,
                "replacement_energy_not_favorable",
            );
        let stale: EnvironmentalReactionEvaluation<u8> = EnvironmentalReactionEvaluation::stale(
            3,
            EnvironmentalReactionKind::Combine,
            EvaluationStage::CommitRevalidation,
        );

        assert!(deferred.into_ready().is_none());
        assert!(rejected.into_ready().is_none());
        assert!(stale.into_ready().is_none());
    }

    #[test]
    fn stage_order_is_shared_without_defining_reaction_policy() {
        assert!(EvaluationStage::CandidateValidation < EvaluationStage::Chemistry);
        assert!(EvaluationStage::Chemistry < EvaluationStage::Activation);
        assert!(EvaluationStage::Activation < EvaluationStage::EnergySettlement);
        assert!(EvaluationStage::EnergySettlement < EvaluationStage::Scheduling);
        assert!(EvaluationStage::Scheduling < EvaluationStage::CommitRevalidation);
    }
}
