#![expect(dead_code, reason = "Staged API retained for subsystem integration")]
use crate::state::EnergyLedger;

const EPSILON: f64 = 1e-9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EnergyReason {
    Combine,
    Break,
    Move,
    Maintenance,
    Decomposition,
    Transfer,
}

/// The external source whose energy crosses into a holder. This is kept
/// separate from bond-energy release: world tone is not chemical potential.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExternalEnergySource {
    WorldTone,
}

/// An explicit source-backed input. `source_available` is the source's local
/// per-tick availability for this capture attempt; callers must enforce the
/// once-per-organism-per-tick capture schedule. Captured input is allocated
/// exactly among usable energy, work, and immediate dissipation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ExternalInputTransaction {
    pub(crate) source: ExternalEnergySource,
    pub(crate) source_available: f64,
    pub(crate) external_input: f64,
    pub(crate) usable_delta: f64,
    pub(crate) work_expended: f64,
    pub(crate) heat_dissipated: f64,
}

/// Mutable source allowance shared by all receivers drawing from the same
/// source budget during an accounting interval (for example, one source/tick).
/// This is an accounting primitive; callers define the physical scope of a budget.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct EnergySourceBudget {
    pub(crate) source: ExternalEnergySource,
    pub(crate) remaining: f64,
}

impl EnergySourceBudget {
    pub(crate) fn new(source: ExternalEnergySource, available: f64) -> Option<Self> {
        (available.is_finite() && available >= 0.0).then_some(Self {
            source,
            remaining: available,
        })
    }
}

impl ExternalInputTransaction {
    pub(crate) fn balanced(self) -> bool {
        self.source_available.is_finite()
            && self.source_available >= 0.0
            && self.external_input.is_finite()
            && self.external_input >= 0.0
            && self.external_input <= self.source_available + EPSILON
            && self.usable_delta.is_finite()
            && self.usable_delta >= 0.0
            && self.work_expended.is_finite()
            && self.work_expended >= 0.0
            && self.heat_dissipated.is_finite()
            && self.heat_dissipated >= 0.0
            && (self.external_input - self.usable_delta - self.work_expended - self.heat_dissipated)
                .abs()
                <= EPSILON
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct EnergyTransaction {
    pub(crate) reason: EnergyReason,
    pub(crate) potential_released: f64,
    pub(crate) usable_delta: f64,
    pub(crate) structural_delta: f64,
    pub(crate) work_expended: f64,
    pub(crate) heat_dissipated: f64,
}

impl EnergyTransaction {
    /// A pure expenditure: usable energy is consumed and allocated exactly
    /// once among persistent structure, work, and dissipated energy.
    pub(crate) fn expenditure(
        reason: EnergyReason,
        structural_delta: f64,
        work_expended: f64,
        heat_dissipated: f64,
    ) -> Option<Self> {
        if !structural_delta.is_finite()
            || structural_delta < 0.0
            || !work_expended.is_finite()
            || work_expended < 0.0
            || !heat_dissipated.is_finite()
            || heat_dissipated < 0.0
        {
            return None;
        }
        let cost = structural_delta + work_expended + heat_dissipated;
        cost.is_finite().then_some(Self {
            reason,
            potential_released: 0.0,
            usable_delta: -cost,
            structural_delta,
            work_expended,
            heat_dissipated,
        })
    }

    pub(crate) fn balanced(self) -> bool {
        self.potential_released.is_finite()
            && self.usable_delta.is_finite()
            && self.structural_delta.is_finite()
            && self.work_expended.is_finite()
            && self.heat_dissipated.is_finite()
            && self.potential_released >= 0.0
            && self.structural_delta >= 0.0
            && self.work_expended >= 0.0
            && self.heat_dissipated >= 0.0
            && (self.potential_released
                - self.usable_delta
                - self.structural_delta
                - self.work_expended
                - self.heat_dissipated)
                .abs()
                <= EPSILON
    }
}

/// Simulation-wide energy accounting infrastructure.
/// Every subsystem calculates its physical transaction and submits the same
/// transaction shape here. The ledger owns the accounting mutation; it does
/// not contain subsystem-specific rules.
pub(crate) trait EnergyLedgerAuthority {
    fn settle_transaction(&mut self, holder: &mut f64, transaction: EnergyTransaction) -> bool;
    fn settle_external_input(
        &mut self,
        holder: &mut f64,
        transaction: ExternalInputTransaction,
    ) -> bool;
    fn settle_external_input_from_budget(
        &mut self,
        holder: &mut f64,
        budget: &mut EnergySourceBudget,
        transaction: ExternalInputTransaction,
    ) -> bool;
    fn transfer(&mut self, from: &mut f64, to: &mut f64, amount: f64) -> bool;
}

impl EnergyLedgerAuthority for EnergyLedger {
    fn settle_external_input_from_budget(
        &mut self,
        holder: &mut f64,
        budget: &mut EnergySourceBudget,
        transaction: ExternalInputTransaction,
    ) -> bool {
        // Validate every budget constraint before settling either object, so a
        // failed claim cannot credit the holder or consume source availability.
        if transaction.source != budget.source
            || !budget.remaining.is_finite()
            || budget.remaining < 0.0
            || transaction.source_available > budget.remaining + EPSILON
            || transaction.external_input > budget.remaining + EPSILON
        {
            return false;
        }
        let next_remaining = budget.remaining - transaction.external_input;
        if !next_remaining.is_finite() || next_remaining < -EPSILON {
            return false;
        }
        if !self.settle_external_input(holder, transaction) {
            return false;
        }
        budget.remaining = next_remaining.max(0.0);
        true
    }

    fn settle_external_input(
        &mut self,
        holder: &mut f64,
        transaction: ExternalInputTransaction,
    ) -> bool {
        if !holder.is_finite() || *holder < -EPSILON || !transaction.balanced() {
            return false;
        }
        let next_holder = *holder + transaction.usable_delta;
        let next_external = self.total_external_energy_input + transaction.external_input;
        let next_gained = self.total_usable_energy_gained + transaction.usable_delta;
        let next_work = self.total_work_expended + transaction.work_expended;
        let next_heat = self.total_heat_dissipated + transaction.heat_dissipated;
        if !next_holder.is_finite()
            || !next_external.is_finite()
            || !next_gained.is_finite()
            || !next_work.is_finite()
            || !next_heat.is_finite()
        {
            return false;
        }
        *holder = next_holder.max(0.0);
        self.total_external_energy_input = next_external;
        self.total_usable_energy_gained = next_gained;
        self.total_work_expended = next_work;
        self.total_heat_dissipated = next_heat;
        true
    }

    fn settle_transaction(&mut self, holder: &mut f64, transaction: EnergyTransaction) -> bool {
        if !holder.is_finite() || *holder < -EPSILON || !transaction.balanced() {
            return false;
        }
        let next_holder = *holder + transaction.usable_delta;
        if !next_holder.is_finite() || next_holder < -EPSILON {
            return false;
        }
        let next_released = self.total_potential_energy_released + transaction.potential_released;
        let next_gained = self.total_usable_energy_gained + transaction.usable_delta.max(0.0);
        let next_work = self.total_work_expended + transaction.work_expended;
        let next_heat = self.total_heat_dissipated + transaction.heat_dissipated;
        if !next_released.is_finite()
            || !next_gained.is_finite()
            || !next_work.is_finite()
            || !next_heat.is_finite()
        {
            return false;
        }
        *holder = next_holder.max(0.0);
        self.total_potential_energy_released = next_released;
        self.total_usable_energy_gained = next_gained;
        self.total_work_expended = next_work;
        self.total_heat_dissipated = next_heat;
        true
    }

    fn transfer(&mut self, from: &mut f64, to: &mut f64, amount: f64) -> bool {
        if !amount.is_finite()
            || amount < 0.0
            || !from.is_finite()
            || !to.is_finite()
            || *from < -EPSILON
            || *to < -EPSILON
            || *from + EPSILON < amount
        {
            return false;
        }
        let next_from = *from - amount;
        let next_to = *to + amount;
        if next_from < -EPSILON || !next_to.is_finite() {
            return false;
        }
        *from = next_from.max(0.0);
        *to = next_to;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transaction_balances_without_reason_specific_accounting() {
        for reason in [
            EnergyReason::Combine,
            EnergyReason::Break,
            EnergyReason::Maintenance,
            EnergyReason::Decomposition,
            EnergyReason::Transfer,
        ] {
            let tx = EnergyTransaction {
                reason,
                potential_released: 5.0,
                usable_delta: 2.0,
                structural_delta: 2.0,
                work_expended: 0.0,
                heat_dissipated: 1.0,
            };
            assert!(tx.balanced());
        }
    }

    #[test]
    fn negative_structural_allocation_is_not_a_balanced_transaction() {
        let tx = EnergyTransaction {
            reason: EnergyReason::Break,
            potential_released: 0.0,
            usable_delta: -2.0,
            structural_delta: -1.0,
            work_expended: 0.0,
            heat_dissipated: 1.0,
        };
        assert!(!tx.balanced());
    }

    #[test]
    fn pure_expenditure_does_not_release_potential_energy() {
        let tx = EnergyTransaction::expenditure(EnergyReason::Combine, 3.0, 1.0, 2.0).unwrap();
        assert!(tx.balanced());
        assert_eq!(tx.potential_released, 0.0);
        assert_eq!(tx.usable_delta, -6.0);
        assert_eq!(tx.structural_delta, 3.0);
        assert_eq!(tx.work_expended, 1.0);
        assert_eq!(tx.heat_dissipated, 2.0);
    }

    #[test]
    fn settlement_uses_transaction_potential_directly() {
        let tx = EnergyTransaction {
            reason: EnergyReason::Combine,
            potential_released: 12.0,
            usable_delta: 4.0,
            structural_delta: 5.0,
            work_expended: 2.0,
            heat_dissipated: 1.0,
        };
        let mut ledger = EnergyLedger::default();
        let mut energy = 10.0;
        assert!(ledger.settle_transaction(&mut energy, tx));
        assert_eq!(energy, 14.0);
        assert_eq!(ledger.total_potential_energy_released, 12.0);
        assert_eq!(ledger.total_work_expended, 2.0);
    }

    #[test]
    fn failed_settlement_is_atomic() {
        let mut ledger = EnergyLedger::default();
        let mut energy = 1.0;
        let tx = EnergyTransaction {
            reason: EnergyReason::Break,
            potential_released: 0.0,
            usable_delta: -10.0,
            structural_delta: -4.0,
            work_expended: 0.0,
            heat_dissipated: 6.0,
        };
        assert!(!ledger.settle_transaction(&mut energy, tx));
        assert_eq!(energy, 1.0);
        assert_eq!(ledger.total_potential_energy_released, 0.0);
        assert_eq!(ledger.total_usable_energy_gained, 0.0);
        assert_eq!(ledger.total_work_expended, 0.0);
        assert_eq!(ledger.total_heat_dissipated, 0.0);
    }

    fn external_input(
        source_available: f64,
        external_input: f64,
        usable_delta: f64,
        work_expended: f64,
        heat_dissipated: f64,
    ) -> ExternalInputTransaction {
        ExternalInputTransaction {
            source: ExternalEnergySource::WorldTone,
            source_available,
            external_input,
            usable_delta,
            work_expended,
            heat_dissipated,
        }
    }

    #[test]
    fn external_input_is_separate_from_released_potential_energy() {
        let mut ledger = EnergyLedger::default();
        let mut energy = 2.0;
        let tx = external_input(10.0, 4.0, 2.5, 0.5, 1.0);
        assert!(ledger.settle_external_input(&mut energy, tx));
        assert_eq!(energy, 4.5);
        assert_eq!(ledger.total_external_energy_input, 4.0);
        assert_eq!(ledger.total_usable_energy_gained, 2.5);
        assert_eq!(ledger.total_work_expended, 0.5);
        assert_eq!(ledger.total_heat_dissipated, 1.0);
        assert_eq!(ledger.total_potential_energy_released, 0.0);
    }

    #[test]
    fn external_input_cannot_exceed_source_availability() {
        let mut ledger = EnergyLedger::default();
        let mut energy = 1.0;
        let tx = external_input(3.0, 3.5, 3.5, 0.0, 0.0);
        assert!(!ledger.settle_external_input(&mut energy, tx));
        assert_eq!(energy, 1.0);
        assert_eq!(ledger.total_external_energy_input, 0.0);
        assert_eq!(ledger.total_usable_energy_gained, 0.0);
        assert_eq!(ledger.total_heat_dissipated, 0.0);
    }

    #[test]
    fn shared_source_budget_prevents_double_claims() {
        let mut ledger = EnergyLedger::default();
        let mut budget = EnergySourceBudget::new(ExternalEnergySource::WorldTone, 5.0).unwrap();
        let mut first_holder = 0.0;
        let mut second_holder = 0.0;
        let first = external_input(5.0, 3.0, 3.0, 0.0, 0.0);
        let second = external_input(5.0, 3.0, 3.0, 0.0, 0.0);

        assert!(ledger.settle_external_input_from_budget(&mut first_holder, &mut budget, first));
        assert_eq!(budget.remaining, 2.0);
        assert!(!ledger.settle_external_input_from_budget(&mut second_holder, &mut budget, second));
        assert_eq!(first_holder, 3.0);
        assert_eq!(second_holder, 0.0);
        assert_eq!(budget.remaining, 2.0);
        assert_eq!(ledger.total_external_energy_input, 3.0);
    }

    #[test]
    fn source_budget_rejects_wrong_source_and_preserves_all_state() {
        let mut ledger = EnergyLedger::default();
        let mut budget = EnergySourceBudget::new(ExternalEnergySource::WorldTone, 5.0).unwrap();
        let mut holder = 1.0;
        let mut tx = external_input(5.0, 2.0, 2.0, 0.0, 0.0);
        tx.source = ExternalEnergySource::WorldTone;
        // The budget must remain unchanged when the transaction is invalid.
        tx.external_input = 2.0;
        tx.usable_delta = 1.0;
        assert!(!ledger.settle_external_input_from_budget(&mut holder, &mut budget, tx));
        assert_eq!(holder, 1.0);
        assert_eq!(budget.remaining, 5.0);
        assert_eq!(ledger.total_external_energy_input, 0.0);
    }

    #[test]
    fn invalid_source_budget_is_rejected() {
        assert!(EnergySourceBudget::new(ExternalEnergySource::WorldTone, f64::NAN).is_none());
        assert!(EnergySourceBudget::new(ExternalEnergySource::WorldTone, -1.0).is_none());
    }

    #[test]
    fn unbalanced_external_input_is_rejected_atomically() {
        let mut ledger = EnergyLedger::default();
        let mut energy = 1.0;
        let tx = external_input(10.0, 4.0, 2.0, 0.0, 1.0);
        assert!(!ledger.settle_external_input(&mut energy, tx));
        assert_eq!(energy, 1.0);
        assert_eq!(ledger.total_external_energy_input, 0.0);
        assert_eq!(ledger.total_usable_energy_gained, 0.0);
        assert_eq!(ledger.total_work_expended, 0.0);
        assert_eq!(ledger.total_heat_dissipated, 0.0);
    }

    #[test]
    fn transfer_conserves_holder_sum() {
        let mut ledger = EnergyLedger::default();
        let mut a = 10.0;
        let mut b = 2.0;
        assert!(ledger.transfer(&mut a, &mut b, 4.0));
        assert_eq!(a + b, 12.0);
    }
}
