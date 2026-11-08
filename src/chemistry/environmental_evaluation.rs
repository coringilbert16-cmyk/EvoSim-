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

/// Payload-free lifecycle entry suitable for retaining rejected, deferred,
/// and stale assessments alongside ready operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EvaluationRecord {
    pub(crate) pass_id: u64,
    pub(crate) reaction: EnvironmentalReactionKind,
    pub(crate) outcome: EvaluationRecordOutcome,
    /// Number of equivalent candidates represented by this compact record.
    pub(crate) multiplicity: usize,
}

/// Lifecycle state without a reaction-specific operation payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EvaluationRecordOutcome {
    Ready,
    Rejected {
        stage: EvaluationStage,
        reason: &'static str,
    },
    Deferred {
        stage: EvaluationStage,
        reason: &'static str,
    },
    Stale {
        stage: EvaluationStage,
    },
}

impl<T> From<EnvironmentalReactionEvaluation<T>> for EvaluationRecord {
    fn from(evaluation: EnvironmentalReactionEvaluation<T>) -> Self {
        let outcome = match evaluation.outcome {
            EvaluationOutcome::Ready(_) => EvaluationRecordOutcome::Ready,
            EvaluationOutcome::Rejected { stage, reason } => {
                EvaluationRecordOutcome::Rejected { stage, reason }
            }
            EvaluationOutcome::Deferred { stage, reason } => {
                EvaluationRecordOutcome::Deferred { stage, reason }
            }
            EvaluationOutcome::Stale { stage } => EvaluationRecordOutcome::Stale { stage },
        };
        Self {
            pass_id: evaluation.pass_id,
            reaction: evaluation.reaction,
            outcome,
            multiplicity: 1,
        }
    }
}

/// Append-only record sink for one evaluation pass. It deliberately stores
/// metadata rather than native payloads, so recording an outcome cannot alter
/// scheduling, retain invalid operations, or change reaction-specific state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct EvaluationJournal {
    records: Vec<EvaluationRecord>,
}

impl EvaluationJournal {
    pub(crate) fn record<T>(&mut self, evaluation: EnvironmentalReactionEvaluation<T>) {
        self.records.push(evaluation.into());
    }

    pub(crate) fn record_deferred_count(
        &mut self,
        pass_id: u64,
        reaction: EnvironmentalReactionKind,
        stage: EvaluationStage,
        reason: &'static str,
        count: usize,
    ) {
        if count == 0 {
            return;
        }
        self.records.push(EvaluationRecord {
            pass_id,
            reaction,
            outcome: EvaluationRecordOutcome::Deferred { stage, reason },
            multiplicity: count,
        });
    }

    pub(crate) fn record_rejected_count(
        &mut self,
        pass_id: u64,
        reaction: EnvironmentalReactionKind,
        stage: EvaluationStage,
        reason: &'static str,
        count: usize,
    ) {
        if count == 0 {
            return;
        }
        self.records.push(EvaluationRecord {
            pass_id,
            reaction,
            outcome: EvaluationRecordOutcome::Rejected { stage, reason },
            multiplicity: count,
        });
    }

    pub(crate) fn records(&self) -> &[EvaluationRecord] {
        &self.records
    }

    pub(crate) fn counts(&self) -> EvaluationOutcomeCounts {
        let mut counts = EvaluationOutcomeCounts::default();
        for record in &self.records {
            match record.outcome {
                EvaluationRecordOutcome::Ready => counts.ready += record.multiplicity,
                EvaluationRecordOutcome::Rejected { .. } => counts.rejected += record.multiplicity,
                EvaluationRecordOutcome::Deferred { .. } => counts.deferred += record.multiplicity,
                EvaluationRecordOutcome::Stale { .. } => counts.stale += record.multiplicity,
            }
        }
        counts
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct EvaluationOutcomeCounts {
    pub(crate) ready: usize,
    pub(crate) rejected: usize,
    pub(crate) deferred: usize,
    pub(crate) stale: usize,
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
    fn journal_retains_all_lifecycle_outcomes_without_native_payloads() {
        let mut journal = EvaluationJournal::default();
        journal.record(EnvironmentalReactionEvaluation::ready(
            41,
            EnvironmentalReactionKind::Combine,
            7_u8,
        ));
        journal.record(EnvironmentalReactionEvaluation::<u8>::rejected(
            41,
            EnvironmentalReactionKind::Combine,
            EvaluationStage::Chemistry,
            "candidate_evaluation_returned_none",
        ));
        journal.record(EnvironmentalReactionEvaluation::<u8>::deferred(
            41,
            EnvironmentalReactionKind::Combine,
            EvaluationStage::CandidateValidation,
            "bounded_pair_window_exhausted",
        ));
        journal.record(EnvironmentalReactionEvaluation::<u8>::stale(
            41,
            EnvironmentalReactionKind::Combine,
            EvaluationStage::CommitRevalidation,
        ));

        assert_eq!(
            journal.counts(),
            EvaluationOutcomeCounts {
                ready: 1,
                rejected: 1,
                deferred: 1,
                stale: 1,
            }
        );
        assert_eq!(journal.records().len(), 4);
        journal.record_deferred_count(
            41,
            EnvironmentalReactionKind::Combine,
            EvaluationStage::CandidateValidation,
            "bounded_constituent_pair_window_exhausted",
            17,
        );
        assert_eq!(journal.counts().deferred, 18);
        assert_eq!(
            journal.records()[1].outcome,
            EvaluationRecordOutcome::Rejected {
                stage: EvaluationStage::Chemistry,
                reason: "candidate_evaluation_returned_none",
            }
        );
        assert_eq!(
            journal.records()[2].outcome,
            EvaluationRecordOutcome::Deferred {
                stage: EvaluationStage::CandidateValidation,
                reason: "bounded_pair_window_exhausted",
            }
        );
        assert_eq!(
            journal.records()[3].outcome,
            EvaluationRecordOutcome::Stale {
                stage: EvaluationStage::CommitRevalidation,
            }
        );
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
