use crate::state::EnergyLedger;

const EPSILON: f64 = 1e-9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EnergyReason { Combine, Break, Maintenance, Decomposition, Transfer }

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct EnergyTransaction { pub(crate) reason: EnergyReason, pub(crate) potential_released: f64, pub(crate) usable_delta: f64, pub(crate) structural_delta: f64, pub(crate) heat_dissipated: f64 }

impl EnergyTransaction {
    pub(crate) fn balanced(self) -> bool {
        self.potential_released.is_finite() && self.usable_delta.is_finite() && self.structural_delta.is_finite() && self.heat_dissipated.is_finite()
            && self.potential_released >= 0.0 && self.heat_dissipated >= 0.0
            && (self.potential_released - self.usable_delta - self.structural_delta - self.heat_dissipated).abs() <= EPSILON
    }
}

pub(crate) trait EnergyLedgerAuthority {
    fn settle_transaction(&mut self, transaction: EnergyTransaction) -> bool;
    fn settle_combine_holder(&mut self, holder: &mut f64, interaction: f64, investment: f64, work: f64) -> bool;
    fn settle_break_holder(&mut self, holder: &mut f64, bond_energy: f64, interaction: f64, work: f64) -> bool;
    fn settle_maintenance(&mut self, holder: &mut f64, paid: f64) -> bool;
    fn settle_decomposition(&mut self, holder: &mut f64, bond_energy: f64, interaction: f64, work: f64) -> bool;
    fn transfer(&mut self, from: &mut f64, to: &mut f64, amount: f64) -> bool;
}

impl EnergyLedgerAuthority for EnergyLedger {
    fn settle_transaction(&mut self, transaction: EnergyTransaction) -> bool {
        if !transaction.balanced() { return false; }
        if transaction.potential_released > 0.0 { self.total_potential_energy_released += transaction.potential_released; }
        if transaction.usable_delta > 0.0 { self.total_usable_energy_gained += transaction.usable_delta; }
        self.total_heat_dissipated += transaction.heat_dissipated;
        self.total_potential_energy_released.is_finite() && self.total_usable_energy_gained.is_finite() && self.total_heat_dissipated.is_finite()
    }

    fn settle_combine_holder(&mut self, holder: &mut f64, interaction: f64, investment: f64, work: f64) -> bool {
        if !holder.is_finite() || *holder < -EPSILON || !interaction.is_finite() || !investment.is_finite() || !work.is_finite() || investment < 0.0 || work < 0.0 { return false; }
        let net = interaction - investment - work;
        let next = *holder + net;
        if !net.is_finite() || !next.is_finite() || next < -EPSILON { return false; }
        let tx = EnergyTransaction { reason: EnergyReason::Combine, potential_released: interaction.max(0.0), usable_delta: net, structural_delta: investment, heat_dissipated: work + (-interaction).max(0.0) };
        if !self.settle_transaction(tx) { return false; }
        *holder = next.max(0.0);
        true
    }

    fn settle_break_holder(&mut self, holder: &mut f64, bond_energy: f64, interaction: f64, work: f64) -> bool {
        if !holder.is_finite() || *holder < -EPSILON || !bond_energy.is_finite() || !interaction.is_finite() || !work.is_finite() || bond_energy < 0.0 || work < 0.0 { return false; }
        let net = bond_energy + interaction - work;
        if *holder + EPSILON < (-net).max(0.0) { return false; }
        let next = *holder + net;
        if !next.is_finite() || next < -EPSILON { return false; }
        let tx = EnergyTransaction { reason: EnergyReason::Break, potential_released: interaction.max(0.0), usable_delta: net, structural_delta: -bond_energy, heat_dissipated: work + (-interaction).max(0.0) };
        if !self.settle_transaction(tx) { return false; }
        *holder = next.max(0.0);
        true
    }

    fn settle_maintenance(&mut self, holder: &mut f64, paid: f64) -> bool {
        if !holder.is_finite() || *holder < -EPSILON || !paid.is_finite() || paid < 0.0 || *holder + EPSILON < paid { return false; }
        let next = *holder - paid;
        let tx = EnergyTransaction { reason: EnergyReason::Maintenance, potential_released: 0.0, usable_delta: -paid, structural_delta: 0.0, heat_dissipated: paid };
        if !self.settle_transaction(tx) { return false; }
        *holder = next.max(0.0);
        true
    }

    fn settle_decomposition(&mut self, holder: &mut f64, bond_energy: f64, interaction: f64, work: f64) -> bool {
        if !holder.is_finite() || *holder < -EPSILON || !bond_energy.is_finite() || !interaction.is_finite() || !work.is_finite() || bond_energy < 0.0 || work < 0.0 { return false; }
        let net = bond_energy + interaction - work;
        if *holder + EPSILON < (-net).max(0.0) { return false; }
        let next = *holder + net;
        if !next.is_finite() || next < -EPSILON { return false; }
        let tx = EnergyTransaction { reason: EnergyReason::Decomposition, potential_released: interaction.max(0.0), usable_delta: net, structural_delta: -bond_energy, heat_dissipated: work + (-interaction).max(0.0) };
        if !self.settle_transaction(tx) { return false; }
        *holder = next.max(0.0);
        true
    }

    fn transfer(&mut self, from: &mut f64, to: &mut f64, amount: f64) -> bool {
        if !amount.is_finite() || amount < 0.0 || !from.is_finite() || !to.is_finite() || *from + EPSILON < amount { return false; }
        let next_from = *from - amount;
        let next_to = *to + amount;
        if next_from < -EPSILON || !next_to.is_finite() { return false; }
        *from = next_from.max(0.0);
        *to = next_to;
        self.total_energy_transferred_out += amount;
        self.total_energy_transferred_in += amount;
        self.total_energy_transferred_out.is_finite() && self.total_energy_transferred_in.is_finite()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn combine_transaction_balances() { let tx = EnergyTransaction { reason: EnergyReason::Combine, potential_released: 5.0, usable_delta: 2.0, structural_delta: 2.0, heat_dissipated: 1.0 }; assert!(tx.balanced()); }
    #[test] fn transfer_conserves_holder_sum() { let mut ledger = EnergyLedger::default(); let mut a = 10.0; let mut b = 2.0; assert!(ledger.transfer(&mut a, &mut b, 4.0)); assert_eq!(a + b, 12.0); }
}
