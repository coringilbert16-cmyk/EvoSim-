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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct EnergyTransaction {
    pub(crate) reason: EnergyReason,
    pub(crate) potential_released: f64,
    pub(crate) usable_delta: f64,
    pub(crate) structural_delta: f64,
    pub(crate) heat_dissipated: f64,
}

impl EnergyTransaction {
    pub(crate) fn balanced(self) -> bool {
        self.potential_released.is_finite()
            && self.usable_delta.is_finite()
            && self.structural_delta.is_finite()
            && self.heat_dissipated.is_finite()
            && self.potential_released >= 0.0
            && self.heat_dissipated >= 0.0
            && (self.potential_released
                - self.usable_delta
                - self.structural_delta
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
    fn transfer(&mut self, from: &mut f64, to: &mut f64, amount: f64) -> bool;
}

impl EnergyLedgerAuthority for EnergyLedger {
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
        let next_heat = self.total_heat_dissipated + transaction.heat_dissipated;
        if !next_released.is_finite() || !next_gained.is_finite() || !next_heat.is_finite() {
            return false;
        }
        *holder = next_holder.max(0.0);
        self.total_potential_energy_released = next_released;
        self.total_usable_energy_gained = next_gained;
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
                heat_dissipated: 1.0,
            };
            assert!(tx.balanced());
        }
    }

    #[test]
    fn settlement_uses_transaction_potential_directly() {
        let tx = EnergyTransaction {
            reason: EnergyReason::Combine,
            potential_released: 10.0,
            usable_delta: 4.0,
            structural_delta: 5.0,
            heat_dissipated: 1.0,
        };
        let mut ledger = EnergyLedger::default();
        let mut energy = 10.0;
        assert!(ledger.settle_transaction(&mut energy, tx));
        assert_eq!(energy, 14.0);
        assert_eq!(ledger.total_potential_energy_released, 10.0);
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
            heat_dissipated: 6.0,
        };
        assert!(!ledger.settle_transaction(&mut energy, tx));
        assert_eq!(energy, 1.0);
        assert_eq!(ledger.total_potential_energy_released, 0.0);
        assert_eq!(ledger.total_usable_energy_gained, 0.0);
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
