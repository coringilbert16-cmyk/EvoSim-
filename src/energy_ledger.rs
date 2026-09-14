use crate::state::EnergyLedger;

const EPSILON: f64 = 1e-9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EnergyReason {
    Combine,
    Break,
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
    fn effective_potential_released(self) -> Option<f64> {
        let released = match self.reason {
            EnergyReason::Combine => {
                // COMBINE's physical interaction is the source of the energy
                // budget. The structural investment is the portion locked into
                // the new bond; usable_delta is already net of that investment
                // and work. Therefore the authoritative released amount is the
                // complete interaction energy represented by the transaction.
                self.usable_delta + self.structural_delta + self.heat_dissipated
            }
            _ => self.potential_released,
        };
        released.is_finite().then_some(released)
    }

    pub(crate) fn balanced(self) -> bool {
        let Some(released) = self.effective_potential_released() else {
            return false;
        };
        self.usable_delta.is_finite()
            && self.structural_delta.is_finite()
            && self.heat_dissipated.is_finite()
            && released >= 0.0
            && self.heat_dissipated >= 0.0
            && (released - self.usable_delta - self.structural_delta - self.heat_dissipated).abs()
                <= EPSILON
    }
}

/// Simulation-wide energy accounting infrastructure.
///
/// The ledger deliberately knows nothing about COMBINE, BREAK, reproduction,
/// maintenance, or decomposition. Callers calculate the physical transaction
/// and provide the holder that owns the usable-energy balance. This keeps the
/// authority independent of every subsystem that needs to use it.
pub(crate) trait EnergyLedgerAuthority {
    fn settle_transaction(&mut self, holder: &mut f64, transaction: EnergyTransaction) -> bool;
    fn transfer(&mut self, from: &mut f64, to: &mut f64, amount: f64) -> bool;
}

impl EnergyLedgerAuthority for EnergyLedger {
    fn settle_transaction(&mut self, holder: &mut f64, transaction: EnergyTransaction) -> bool {
        if !holder.is_finite() || *holder < -EPSILON || !transaction.balanced() {
            return false;
        }
        let next = *holder + transaction.usable_delta;
        if !next.is_finite() || next < -EPSILON {
            return false;
        }
        *holder = next.max(0.0);
        self.record_transaction(transaction);
        true
    }

    fn transfer(&mut self, from: &mut f64, to: &mut f64, amount: f64) -> bool {
        if !from.is_finite()
            || !to.is_finite()
            || !amount.is_finite()
            || amount < 0.0
            || *from + EPSILON < amount
        {
            return false;
        }
        *from -= amount;
        *to += amount;
        if !from.is_finite() || !to.is_finite() || *from < -EPSILON {
            return false;
        }
        *from = from.max(0.0);
        true
    }
}
