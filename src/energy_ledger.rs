use crate::state::EnergyLedger;

const EPSILON: f64 = 1e-9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EnergyReason {
    Combine,
    Break,
    Maintenance,
    Decomposition,
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

pub(crate) trait EnergyLedgerAuthority {
    fn settle_transaction(&mut self, transaction: EnergyTransaction) -> bool;
    fn settle_combine(&mut self, interaction: f64, investment: f64, work: f64) -> bool;
    fn settle_break(&mut self, bond_energy: f64, interaction: f64, work: f64) -> bool;
    fn settle_maintenance(&mut self, paid: f64) -> bool;
    fn settle_decomposition(&mut self, bond_energy: f64, interaction: f64, work: f64) -> bool;
}

impl EnergyLedgerAuthority for EnergyLedger {
    fn settle_transaction(&mut self, transaction: EnergyTransaction) -> bool {
        if !transaction.balanced() {
            return false;
        }

        if transaction.potential_released > 0.0 {
            self.total_potential_energy_released += transaction.potential_released;
        }
        if transaction.usable_delta > 0.0 {
            self.total_usable_energy_gained += transaction.usable_delta;
        }
        self.total_heat_dissipated += transaction.heat_dissipated;
        true
    }

    fn settle_combine(&mut self, interaction: f64, investment: f64, work: f64) -> bool {
        if !interaction.is_finite()
            || !investment.is_finite()
            || !work.is_finite()
            || investment < 0.0
            || work < 0.0
        {
            return false;
        }

        let positive_interaction = interaction.max(0.0);
        let negative_interaction = (-interaction).max(0.0);
        self.settle_transaction(EnergyTransaction {
            reason: EnergyReason::Combine,
            potential_released: positive_interaction,
            usable_delta: interaction - investment - work,
            structural_delta: investment,
            heat_dissipated: work + negative_interaction,
        })
    }

    fn settle_break(&mut self, bond_energy: f64, interaction: f64, work: f64) -> bool {
        if !bond_energy.is_finite()
            || !interaction.is_finite()
            || !work.is_finite()
            || bond_energy < 0.0
            || work < 0.0
        {
            return false;
        }

        let positive_interaction = interaction.max(0.0);
        let negative_interaction = (-interaction).max(0.0);
        self.settle_transaction(EnergyTransaction {
            reason: EnergyReason::Break,
            potential_released: positive_interaction,
            usable_delta: bond_energy + interaction - work,
            structural_delta: -bond_energy,
            heat_dissipated: work + negative_interaction,
        })
    }

    fn settle_maintenance(&mut self, paid: f64) -> bool {
        if !paid.is_finite() || paid < 0.0 {
            return false;
        }

        self.settle_transaction(EnergyTransaction {
            reason: EnergyReason::Maintenance,
            potential_released: 0.0,
            usable_delta: -paid,
            structural_delta: 0.0,
            heat_dissipated: paid,
        })
    }

    fn settle_decomposition(&mut self, bond_energy: f64, interaction: f64, work: f64) -> bool {
        if !bond_energy.is_finite()
            || !interaction.is_finite()
            || !work.is_finite()
            || bond_energy < 0.0
            || work < 0.0
        {
            return false;
        }

        let positive_interaction = interaction.max(0.0);
        let negative_interaction = (-interaction).max(0.0);
        self.settle_transaction(EnergyTransaction {
            reason: EnergyReason::Decomposition,
            potential_released: positive_interaction,
            usable_delta: bond_energy + interaction - work,
            structural_delta: -bond_energy,
            heat_dissipated: work + negative_interaction,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combine_transaction_balances() {
        let tx = EnergyTransaction {
            reason: EnergyReason::Combine,
            potential_released: 5.0,
            usable_delta: 2.0,
            structural_delta: 2.0,
            heat_dissipated: 1.0,
        };
        assert!(tx.balanced());
    }

    #[test]
    fn break_transaction_balances() {
        let tx = EnergyTransaction {
            reason: EnergyReason::Break,
            potential_released: 3.0,
            usable_delta: 9.0,
            structural_delta: -10.0,
            heat_dissipated: 4.0,
        };
        assert!(tx.balanced());
    }

    #[test]
    fn decomposition_keeps_net_energy_recoverable() {
        let tx = EnergyTransaction {
            reason: EnergyReason::Decomposition,
            potential_released: 3.0,
            usable_delta: 9.0,
            structural_delta: -10.0,
            heat_dissipated: 4.0,
        };
        assert!(tx.balanced());
    }
}
