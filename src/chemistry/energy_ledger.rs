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

/// An explicit source-backed input. `source_available` is this receiver's
/// local incident-energy allowance for the tick, derived from its unshadowed
/// projected exposure and source intensity; it is not necessarily a global
/// source pool. Callers must prevent duplicate claims within that receiver's
/// capture scope. Absorbed input is allocated exactly among usable energy,
/// work, and immediate dissipation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ExternalInputTransaction {
    pub(crate) source: ExternalEnergySource,
    pub(crate) source_available: f64,
    pub(crate) external_input: f64,
    pub(crate) usable_delta: f64,
    pub(crate) work_expended: f64,
    pub(crate) heat_dissipated: f64,
}

/// Mutable allowance shared by receivers drawing from the same finite
/// allocation budget during an accounting interval. This is an accounting
/// primitive; callers define the physical scope. In particular, the sun's
/// configured output is an intensity per tick per unit of exposed projected
/// width, not one finite world-wide pool to divide among every receiver. Solar
/// callers must derive local allowances from unique, unshadowed exposure and
/// prevent duplicate claims within that scope.
/// Intentionally not `Copy` or `Clone`: a genuinely shared finite allowance
/// must be passed by mutable reference rather than accidentally duplicated.
#[derive(Debug, PartialEq)]
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

/// Shared accounting destinations for any energy transaction.
///
/// Source identity and source-availability rules remain on the concrete
/// transaction. This common allocation enforces the same balance equation
/// without treating external input as released chemical potential.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct EnergyAllocation {
    pub(crate) source_energy: f64,
    pub(crate) usable_delta: f64,
    /// Signed change in energy held by persistent structure: positive when
    /// stored, negative when an existing structural store is released.
    pub(crate) structural_delta: f64,
    pub(crate) work_expended: f64,
    pub(crate) heat_dissipated: f64,
}

impl EnergyAllocation {
    pub(crate) fn balanced(self) -> bool {
        self.source_energy.is_finite()
            && self.usable_delta.is_finite()
            && self.structural_delta.is_finite()
            && self.work_expended.is_finite()
            && self.heat_dissipated.is_finite()
            && self.source_energy >= 0.0
            && self.work_expended >= 0.0
            && self.heat_dissipated >= 0.0
            && (self.source_energy
                - self.usable_delta
                - self.structural_delta
                - self.work_expended
                - self.heat_dissipated)
                .abs()
                <= EPSILON
    }
}

impl ExternalInputTransaction {
    pub(crate) fn allocation(self) -> EnergyAllocation {
        EnergyAllocation {
            source_energy: self.external_input,
            usable_delta: self.usable_delta,
            structural_delta: 0.0,
            work_expended: self.work_expended,
            heat_dissipated: self.heat_dissipated,
        }
    }

    pub(crate) fn balanced(self) -> bool {
        self.source_available.is_finite()
            && self.source_available >= 0.0
            && self.external_input.is_finite()
            && self.external_input >= 0.0
            && self.external_input <= self.source_available + EPSILON
            && self.usable_delta >= 0.0
            && self.allocation().balanced()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct EnergyTransaction {
    pub(crate) reason: EnergyReason,
    /// Non-structural energy source explicitly released by this event.
    /// Energy already stored in a bond is represented by a negative
    /// structural_delta when that bond is removed, not by this field.
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

    pub(crate) fn allocation(self) -> EnergyAllocation {
        EnergyAllocation {
            source_energy: self.potential_released,
            usable_delta: self.usable_delta,
            structural_delta: self.structural_delta,
            work_expended: self.work_expended,
            heat_dissipated: self.heat_dissipated,
        }
    }

    pub(crate) fn balanced(self) -> bool {
        self.allocation().balanced()
    }
}

/// A point-in-time inventory used to reconcile world-level energy accounting.
///
/// Structural energy is the total energy currently stored in realized bonds,
/// counted once across all structures. The initial values are the tick-zero
/// baseline; cumulative flows are totals since that baseline. Transfers among
/// internal holders are intentionally absent because they cancel at world scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct EnergyInventorySnapshot {
    pub(crate) initial_usable_energy: f64,
    pub(crate) current_usable_energy: f64,
    pub(crate) initial_resonant_energy: f64,
    pub(crate) current_resonant_energy: f64,
    pub(crate) initial_structural_energy: f64,
    pub(crate) current_structural_energy: f64,
    pub(crate) external_energy_input: f64,
    /// Energy carried out of the modeled world by a continuing signal.
    pub(crate) world_tone_energy_escaped: f64,
    /// Explicit non-structural sources only; never count bond breakdown here.
    pub(crate) other_energy_released: f64,
    pub(crate) work_expended: f64,
    pub(crate) heat_dissipated: f64,
}

impl EnergyInventorySnapshot {
    /// Signed conservation residual. A value of zero means the change in
    /// usable, resonant, and structural energy plus work, heat, and escaped tone
    /// equals all external sources.
    pub(crate) fn residual(self) -> Option<f64> {
        let values = [
            self.initial_usable_energy,
            self.current_usable_energy,
            self.initial_resonant_energy,
            self.current_resonant_energy,
            self.initial_structural_energy,
            self.current_structural_energy,
            self.external_energy_input,
            self.world_tone_energy_escaped,
            self.other_energy_released,
            self.work_expended,
            self.heat_dissipated,
        ];
        if values
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
        {
            return None;
        }
        let usable_delta = self.current_usable_energy - self.initial_usable_energy;
        let resonant_delta = self.current_resonant_energy - self.initial_resonant_energy;
        let structural_delta = self.current_structural_energy - self.initial_structural_energy;
        let residual = usable_delta
            + resonant_delta
            + structural_delta
            + self.work_expended
            + self.heat_dissipated
            + self.world_tone_energy_escaped
            - self.external_energy_input
            - self.other_energy_released;
        residual.is_finite().then_some(residual)
    }

    pub(crate) fn is_balanced(self, tolerance: f64) -> bool {
        tolerance.is_finite()
            && tolerance >= 0.0
            && self
                .residual()
                .is_some_and(|residual| residual.abs() <= tolerance)
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
    fn settle_world_tone_paths(
        &mut self,
        incident_energy: f64,
        resonant_energy_captured: f64,
        heat_dissipated: f64,
        escaped_energy: f64,
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

    fn settle_world_tone_paths(
        &mut self,
        incident_energy: f64,
        resonant_energy_captured: f64,
        heat_dissipated: f64,
        escaped_energy: f64,
    ) -> bool {
        let values = [
            incident_energy,
            resonant_energy_captured,
            heat_dissipated,
            escaped_energy,
        ];
        if values
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
        {
            return false;
        }
        let allocated = resonant_energy_captured + heat_dissipated + escaped_energy;
        if !allocated.is_finite()
            || (incident_energy - allocated).abs() > EPSILON * incident_energy.max(1.0)
        {
            return false;
        }
        let next_external = self.total_external_energy_input + incident_energy;
        let next_heat = self.total_heat_dissipated + heat_dissipated;
        let next_escaped = self.total_world_tone_energy_escaped + escaped_energy;
        if !next_external.is_finite() || !next_heat.is_finite() || !next_escaped.is_finite() {
            return false;
        }
        self.total_external_energy_input = next_external;
        self.total_heat_dissipated = next_heat;
        self.total_world_tone_energy_escaped = next_escaped;
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
        let next_structural = self.total_structural_energy_delta + transaction.structural_delta;
        let next_gained = self.total_usable_energy_gained + transaction.usable_delta.max(0.0);
        let next_work = self.total_work_expended + transaction.work_expended;
        let next_heat = self.total_heat_dissipated + transaction.heat_dissipated;
        if !next_released.is_finite()
            || !next_structural.is_finite()
            || !next_gained.is_finite()
            || !next_work.is_finite()
            || !next_heat.is_finite()
        {
            return false;
        }
        *holder = next_holder.max(0.0);
        self.total_potential_energy_released = next_released;
        self.total_structural_energy_delta = next_structural;
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
    fn world_inventory_counts_retained_resonance_as_a_distinct_energy_store() {
        let inventory = EnergyInventorySnapshot {
            initial_usable_energy: 10.0,
            current_usable_energy: 10.0,
            initial_resonant_energy: 0.0,
            current_resonant_energy: 3.0,
            initial_structural_energy: 4.0,
            current_structural_energy: 4.0,
            external_energy_input: 5.0,
            world_tone_energy_escaped: 0.0,
            other_energy_released: 0.0,
            work_expended: 1.0,
            heat_dissipated: 1.0,
        };
        assert_eq!(inventory.residual(), Some(0.0));
        assert!(inventory.is_balanced(1e-12));
    }

    #[test]
    fn world_tone_path_settlement_accounts_for_capture_heat_and_escape_once() {
        let mut ledger = EnergyLedger::default();
        assert!(ledger.settle_world_tone_paths(10.0, 4.0, 2.0, 4.0));
        assert_eq!(ledger.total_external_energy_input, 10.0);
        assert_eq!(ledger.total_heat_dissipated, 2.0);
        assert_eq!(ledger.total_world_tone_energy_escaped, 4.0);
        assert!(!ledger.settle_world_tone_paths(10.0, 4.0, 2.0, 3.0));
        assert_eq!(ledger.total_external_energy_input, 10.0);
        assert_eq!(ledger.total_world_tone_energy_escaped, 4.0);

        let inventory = EnergyInventorySnapshot {
            initial_usable_energy: 0.0,
            current_usable_energy: 0.0,
            initial_resonant_energy: 0.0,
            current_resonant_energy: 4.0,
            initial_structural_energy: 0.0,
            current_structural_energy: 0.0,
            external_energy_input: ledger.total_external_energy_input,
            world_tone_energy_escaped: ledger.total_world_tone_energy_escaped,
            other_energy_released: 0.0,
            work_expended: 0.0,
            heat_dissipated: ledger.total_heat_dissipated,
        };
        assert!(inventory.is_balanced(1e-12));
    }

    #[test]
    fn world_inventory_reconciles_external_input_work_and_heat() {
        let inventory = EnergyInventorySnapshot {
            initial_usable_energy: 10.0,
            current_usable_energy: 12.0,
            initial_resonant_energy: 0.0,
            current_resonant_energy: 0.0,
            initial_structural_energy: 8.0,
            current_structural_energy: 9.0,
            external_energy_input: 7.0,
            world_tone_energy_escaped: 0.0,
            other_energy_released: 0.0,
            work_expended: 2.0,
            heat_dissipated: 2.0,
        };
        assert_eq!(inventory.residual(), Some(0.0));
        assert!(inventory.is_balanced(1e-12));
    }

    #[test]
    fn world_inventory_counts_bond_breakdown_as_structural_decrease_once() {
        let inventory = EnergyInventorySnapshot {
            initial_usable_energy: 0.0,
            current_usable_energy: 2.0,
            initial_resonant_energy: 0.0,
            current_resonant_energy: 0.0,
            initial_structural_energy: 5.0,
            current_structural_energy: 0.0,
            external_energy_input: 0.0,
            world_tone_energy_escaped: 0.0,
            other_energy_released: 0.0,
            work_expended: 0.0,
            heat_dissipated: 3.0,
        };
        assert_eq!(inventory.residual(), Some(0.0));
        assert!(inventory.is_balanced(1e-12));
    }

    #[test]
    fn world_inventory_rejects_unbalanced_or_non_finite_snapshots() {
        let mut inventory = EnergyInventorySnapshot {
            initial_usable_energy: 10.0,
            current_usable_energy: 14.0,
            initial_resonant_energy: 0.0,
            current_resonant_energy: 0.0,
            initial_structural_energy: 8.0,
            current_structural_energy: 9.0,
            external_energy_input: 7.0,
            world_tone_energy_escaped: 0.0,
            other_energy_released: 0.0,
            work_expended: 2.0,
            heat_dissipated: 2.0,
        };
        inventory.current_usable_energy += 1.0;
        assert!(!inventory.is_balanced(1e-12));
        inventory.current_usable_energy = f64::NAN;
        assert_eq!(inventory.residual(), None);
        assert!(!inventory.is_balanced(f64::NAN));
    }

    #[test]
    fn both_transaction_types_use_the_shared_allocation_balance() {
        let chemical = EnergyTransaction {
            reason: EnergyReason::Break,
            potential_released: 5.0,
            usable_delta: 2.0,
            structural_delta: 1.0,
            work_expended: 1.0,
            heat_dissipated: 1.0,
        };
        let external = external_input(5.0, 5.0, 2.0, 1.0, 2.0);

        assert!(chemical.allocation().balanced());
        assert!(external.allocation().balanced());
        assert_eq!(chemical.allocation().source_energy, 5.0);
        assert_eq!(external.allocation().source_energy, 5.0);
        assert_eq!(chemical.allocation().structural_delta, 1.0);
        assert_eq!(external.allocation().structural_delta, 0.0);
    }

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
    fn unbalanced_transaction_with_negative_structural_delta_is_rejected() {
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
    fn bond_energy_release_can_be_represented_as_a_negative_structural_delta() {
        let tx = EnergyTransaction {
            reason: EnergyReason::Break,
            potential_released: 0.0,
            usable_delta: 2.0,
            structural_delta: -5.0,
            work_expended: 0.0,
            heat_dissipated: 3.0,
        };
        assert!(tx.balanced());
        assert_eq!(tx.allocation().source_energy, 0.0);
        assert_eq!(tx.allocation().structural_delta, -5.0);
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
    fn settling_bond_release_debits_structural_store_without_faking_a_source() {
        let mut ledger = EnergyLedger::default();
        let mut energy = 0.0;
        let tx = EnergyTransaction {
            reason: EnergyReason::Break,
            potential_released: 0.0,
            usable_delta: 2.0,
            structural_delta: -5.0,
            work_expended: 0.0,
            heat_dissipated: 3.0,
        };

        assert!(ledger.settle_transaction(&mut energy, tx));
        assert_eq!(energy, 2.0);
        assert_eq!(ledger.total_potential_energy_released, 0.0);
        assert_eq!(ledger.total_structural_energy_delta, -5.0);
        assert_eq!(ledger.total_heat_dissipated, 3.0);
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
        assert_eq!(ledger.total_structural_energy_delta, 0.0);
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

        // Local availability may exceed the aggregate remainder; a smaller
        // claim is still valid when it fits both constraints.
        let smaller_claim = external_input(5.0, 1.0, 1.0, 0.0, 0.0);
        assert!(ledger.settle_external_input_from_budget(
            &mut second_holder,
            &mut budget,
            smaller_claim
        ));
        assert_eq!(second_holder, 1.0);
        assert_eq!(budget.remaining, 1.0);
        assert_eq!(ledger.total_external_energy_input, 4.0);
    }

    #[test]
    fn source_budget_rejects_claim_above_remaining_budget_atomically() {
        let mut ledger = EnergyLedger::default();
        let mut budget = EnergySourceBudget::new(ExternalEnergySource::WorldTone, 5.0).unwrap();
        let mut holder = 1.0;
        let tx = external_input(6.0, 6.0, 6.0, 0.0, 0.0);
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
