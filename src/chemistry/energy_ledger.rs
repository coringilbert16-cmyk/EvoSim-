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
    /// The explicit one-time energy grant that initializes the simulation's first organism.
    InitialCondition,
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
/// Signed structural potential for the authoritative realized bond graph.
/// A stable bond contributes negative potential relative to separated materials.
pub(crate) fn structural_potential(structure: &crate::structure::OrganismStructure) -> Option<f64> {
    let potential = structure.bonds.iter().try_fold(0.0, |total, bond| {
        let next = total - bond.bond_energy;
        (bond.bond_energy.is_finite() && bond.bond_energy >= 0.0 && next.is_finite())
            .then_some(next)
    })?;
    potential.is_finite().then_some(potential)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct EnergyAllocation {
    pub(crate) source_energy: f64,
    pub(crate) usable_delta: f64,
    /// Signed change in structural potential: negative when a stable bond
    /// forms and positive when a bond is broken.
    pub(crate) structural_delta: f64,
    pub(crate) work_expended: f64,
    pub(crate) heat_dissipated: f64,
}

impl EnergyAllocation {
    pub(crate) fn balanced(self) -> bool {
        let residual = self.source_energy
            - self.usable_delta
            - self.structural_delta
            - self.work_expended
            - self.heat_dissipated;
        self.source_energy.is_finite()
            && self.usable_delta.is_finite()
            && self.structural_delta.is_finite()
            && self.work_expended.is_finite()
            && self.heat_dissipated.is_finite()
            && self.source_energy >= 0.0
            && self.work_expended >= 0.0
            && self.heat_dissipated >= 0.0
            && residual.is_finite()
            && (0.0..=EPSILON).contains(&residual)
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
            && self.external_input <= self.source_available
            && self.usable_delta >= 0.0
            && (self.external_input != 0.0
                || (self.usable_delta == 0.0
                    && self.work_expended == 0.0
                    && self.heat_dissipated == 0.0))
            && self.allocation().balanced()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct EnergyTransaction {
    pub(crate) reason: EnergyReason,
    /// Legacy compatibility field. Must remain exactly zero: this transaction
    /// has no named non-structural source owner to debit. Bond formation/breaking
    /// is represented by the signed structural_delta. External input must use
    /// ExternalInputTransaction instead.
    pub(crate) potential_released: f64,
    pub(crate) usable_delta: f64,
    pub(crate) structural_delta: f64,
    pub(crate) work_expended: f64,
    pub(crate) heat_dissipated: f64,
}

impl EnergyTransaction {
    /// A pure expenditure with no structural-potential change.
    /// Any bond formation/breaking must use the shared reaction settlement
    /// contract so its signed pre/post-state delta cannot be treated as a cost
    /// destination or a stored-energy reservoir.
    pub(crate) fn expenditure(
        reason: EnergyReason,
        structural_delta: f64,
        work_expended: f64,
        heat_dissipated: f64,
    ) -> Option<Self> {
        if !structural_delta.is_finite()
            || structural_delta != 0.0
            || !work_expended.is_finite()
            || work_expended < 0.0
            || !heat_dissipated.is_finite()
            || heat_dissipated < 0.0
        {
            return None;
        }
        let cost = work_expended + heat_dissipated;
        cost.is_finite().then_some(Self {
            reason,
            potential_released: 0.0,
            usable_delta: -cost,
            structural_delta: 0.0,
            work_expended,
            heat_dissipated,
        })
    }

    /// Settle a reaction whose structural-potential change may be favorable
    /// (negative) or unfavorable (positive). Bond energy is a dissociation
    /// magnitude; a newly formed stable bond lowers structural potential, while
    /// breaking it raises structural potential and therefore requires energy.
    /// The usable holder receives any surplus or pays any deficit after work
    /// and heat are accounted for.
    pub(crate) fn reaction(
        reason: EnergyReason,
        structural_delta: f64,
        work_expended: f64,
        heat_dissipated: f64,
    ) -> Option<Self> {
        if !structural_delta.is_finite()
            || !work_expended.is_finite()
            || work_expended < 0.0
            || !heat_dissipated.is_finite()
            || heat_dissipated < 0.0
        {
            return None;
        }
        // Subtract in the same order used by the world inventory residual.
        // This avoids a tiny positive rounding residual for otherwise exact
        // dissociation transactions.
        let usable_delta = -structural_delta - work_expended - heat_dissipated;
        if !usable_delta.is_finite() {
            return None;
        }
        Some(Self {
            reason,
            potential_released: 0.0,
            usable_delta,
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
        // A generic transaction has no named non-structural source owner to
        // debit. Bond energy must be represented by structural_delta; any
        // genuinely external input must use ExternalInputTransaction.
        // Unlike an external-input allocation, a reaction's algebraic balance
        // can round a few ULPs to either side of zero. Permit that signed
        // construction error here; settle_transaction checks the represented
        // holder/counter increments and rejects any actual energy creation.
        let allocation = self.allocation();
        let residual = allocation.source_energy
            - allocation.usable_delta
            - allocation.structural_delta
            - allocation.work_expended
            - allocation.heat_dissipated;
        self.potential_released == 0.0
            && allocation.source_energy.is_finite()
            && allocation.usable_delta.is_finite()
            && allocation.structural_delta.is_finite()
            && allocation.work_expended.is_finite()
            && allocation.work_expended >= 0.0
            && allocation.heat_dissipated.is_finite()
            && allocation.heat_dissipated >= 0.0
            && residual.is_finite()
            && residual.abs() <= EPSILON
    }
}

/// Structural-energy result for a proposed physical reaction, before live state mutates.
///
/// The current runtime supplies the signed potential of the authoritative
/// reactant and product bond graphs. This does not claim to model every possible
/// chemical free-energy term; a reaction that changes composition without a
/// represented structural-potential change needs additional chemistry data before
/// its true thermodynamic favorability can be inferred. Physical work and
/// dissipation are outputs from this same budget. Activation is deliberately
/// separate: a threshold controls whether a reaction may proceed, but is not
/// an energy source.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ReactionEnergyBalance {
    pub(crate) reactant_energy: f64,
    pub(crate) product_energy: f64,
    pub(crate) physical_work: f64,
    pub(crate) dissipated_heat: f64,
    /// Signed product-minus-reactant energy before work and dissipation.
    pub(crate) net_reaction_change: f64,
    /// Minimum legitimate external energy needed after accounting for the
    /// reactant-to-product energy change and modeled work/heat.
    pub(crate) required_external_input: f64,
    /// Energy left over after work and dissipation, available for explicit
    /// modeled destinations. It is not credited automatically by this plan.
    pub(crate) released_surplus: f64,
}

impl ReactionEnergyBalance {
    /// Derive the reaction budget from actual pre/post energy states.
    ///
    /// A negative net balance can fund work and heat without external input;
    /// a positive deficit must be funded by a named source before commit.
    /// This constructor only plans the balance and never mutates a holder.
    pub(crate) fn calculate(
        reactant_energy: f64,
        product_energy: f64,
        physical_work: f64,
        dissipated_heat: f64,
    ) -> Option<Self> {
        if !reactant_energy.is_finite()
            || !product_energy.is_finite()
            || !physical_work.is_finite()
            || physical_work < 0.0
            || !dissipated_heat.is_finite()
            || dissipated_heat < 0.0
        {
            return None;
        }
        let net_reaction_change = product_energy - reactant_energy;
        let net_budget_change = net_reaction_change + physical_work + dissipated_heat;
        if !net_reaction_change.is_finite() || !net_budget_change.is_finite() {
            return None;
        }
        let required_external_input = net_budget_change.max(0.0);
        let released_surplus = (-net_budget_change).max(0.0);
        Some(Self {
            reactant_energy,
            product_energy,
            physical_work,
            dissipated_heat,
            net_reaction_change,
            required_external_input,
            released_surplus,
        })
    }

    pub(crate) fn can_commit_with(self, legitimate_external_input: f64) -> bool {
        legitimate_external_input.is_finite()
            && legitimate_external_input >= self.required_external_input
            && (legitimate_external_input - self.required_external_input).is_finite()
    }

    /// Build the ledger transaction from the same pre/post-state balance
    /// used to decide whether the reaction is favorable or needs energy input.
    pub(crate) fn settlement_transaction(self, reason: EnergyReason) -> Option<EnergyTransaction> {
        EnergyTransaction::reaction(
            reason,
            self.net_reaction_change,
            self.physical_work,
            self.dissipated_heat,
        )
    }

    /// The signed conservation residual after applying the supplied input and
    /// recording all explicit destinations. Positive residual means creation.
    pub(crate) fn residual(self, legitimate_external_input: f64) -> Option<f64> {
        if !legitimate_external_input.is_finite() || legitimate_external_input < 0.0 {
            return None;
        }
        // Any source input beyond the deficit remains in the source holder;
        // it is not a reaction output and must not be mistaken for creation.
        let retained_input = (legitimate_external_input - self.required_external_input).max(0.0);
        let residual = self.reactant_energy + legitimate_external_input
            - self.product_energy
            - self.physical_work
            - self.dissipated_heat
            - self.released_surplus
            - retained_input;
        residual.is_finite().then_some(residual)
    }
}

/// A point-in-time inventory used to reconcile world-level energy accounting.
///
/// Structural energy is the signed potential of realized bonds relative to the
/// separated-material baseline: stable bonds contribute negative potential,
/// counted once across all structures. Initial values are the tick-zero baseline;
/// cumulative flows are totals since that baseline. Internal transfers cancel
/// at world scale and are intentionally absent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct EnergyInventorySnapshot {
    pub(crate) initial_usable_energy: f64,
    pub(crate) current_usable_energy: f64,
    pub(crate) initial_resonant_energy: f64,
    pub(crate) current_resonant_energy: f64,
    pub(crate) initial_structural_energy: f64,
    pub(crate) current_structural_energy: f64,
    /// Signed pair-interaction potential relative to the tick-zero reference.
    pub(crate) initial_pair_interaction_potential: f64,
    pub(crate) current_pair_interaction_potential: f64,
    pub(crate) external_energy_input: f64,
    /// External resonant-field energy that leaves the modeled world after material response.
    pub(crate) world_tone_energy_escaped: f64,
    /// Reserved compatibility slot. Must be zero until an explicitly owned
    /// non-structural source type exists; bond breakdown is structural_delta.
    pub(crate) other_energy_released: f64,
    pub(crate) work_expended: f64,
    pub(crate) heat_dissipated: f64,
}

impl EnergyInventorySnapshot {
    /// Signed conservation residual. A value of zero means the change in
    /// usable, resonant, and structural energy plus work, heat, and escaped tone
    /// equals all external sources.
    pub(crate) fn residual(self) -> Option<f64> {
        let nonnegative_values = [
            self.initial_usable_energy,
            self.current_usable_energy,
            self.initial_resonant_energy,
            self.current_resonant_energy,
            self.external_energy_input,
            self.world_tone_energy_escaped,
            self.other_energy_released,
            self.work_expended,
            self.heat_dissipated,
        ];
        if self.other_energy_released != 0.0
            || !self.initial_structural_energy.is_finite()
            || !self.current_structural_energy.is_finite()
            || !self.initial_pair_interaction_potential.is_finite()
            || !self.current_pair_interaction_potential.is_finite()
            || nonnegative_values
                .iter()
                .any(|value| !value.is_finite() || *value < 0.0)
        {
            return None;
        }
        let usable_delta = self.current_usable_energy - self.initial_usable_energy;
        let resonant_delta = self.current_resonant_energy - self.initial_resonant_energy;
        let structural_delta = self.current_structural_energy - self.initial_structural_energy;
        let pair_potential_delta =
            self.current_pair_interaction_potential - self.initial_pair_interaction_potential;
        let residual = usable_delta
            + resonant_delta
            + structural_delta
            + pair_potential_delta
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

    /// Stronger release-gate oracle: tolerate tiny missing energy from numeric
    /// rounding, but never accept a positive residual that represents more
    /// stored/spent energy than the modeled sources supplied.
    pub(crate) fn is_balanced_without_creation(self, tolerance: f64) -> bool {
        tolerance.is_finite()
            && tolerance >= 0.0
            && self
                .residual()
                .is_some_and(|residual| residual <= 0.0 && residual.abs() <= tolerance)
    }
}

/// Settle a signed change in the conservative resonance pair potential.
///
/// Positive changes are funded only by the active mover's usable energy.
/// Negative changes are returned to the supplied biological resonance store
/// in proportion to their negative frequency-resolved contributions; energy
/// that cannot be retained is dissipated as heat. The pair-potential change
/// is recorded as held energy and is never also counted as expended work.
#[allow(dead_code)] // Staged for the world-potential consumer after inventory validation.
pub(crate) fn settle_pair_potential_change(
    ledger: &mut EnergyLedger,
    active_mover_usable_energy: &mut f64,
    biological_resonance_holders: &mut [&mut crate::harmonics::ResonantEnergyReservoir],
    components: &[crate::harmonics::ResonancePairPotentialComponent],
) -> bool {
    if !active_mover_usable_energy.is_finite()
        || *active_mover_usable_energy < 0.0
        || !ledger.total_pair_interaction_potential_held.is_finite()
        || !ledger.initial_pair_interaction_potential_held.is_finite()
        || !ledger.total_usable_energy_gained.is_finite()
        || !ledger.total_resonant_energy_held.is_finite()
        || ledger.total_resonant_energy_held < 0.0
        || !ledger.total_heat_dissipated.is_finite()
        || ledger.total_heat_dissipated < 0.0
        || components.iter().any(|component| {
            !component.frequency_cycles_per_day.is_finite()
                || component.frequency_cycles_per_day <= 0.0
                || !component.energy.is_finite()
        })
    {
        return false;
    }

    let delta = components
        .iter()
        .map(|component| component.energy)
        .sum::<f64>();
    if !delta.is_finite() {
        return false;
    }
    if delta == 0.0 {
        return true;
    }

    let next_pair_potential = ledger.total_pair_interaction_potential_held + delta;
    if !next_pair_potential.is_finite() {
        return false;
    }

    if delta > 0.0 {
        let next_usable = *active_mover_usable_energy - delta;
        if !next_usable.is_finite() || next_usable < 0.0 {
            return false;
        }
        let next_usable_gained = ledger.total_usable_energy_gained - delta;
        if !next_usable_gained.is_finite() {
            return false;
        }
        let actual_usable_delta = next_usable - *active_mover_usable_energy;
        let actual_pair_delta = next_pair_potential - ledger.total_pair_interaction_potential_held;
        let residual = actual_usable_delta + actual_pair_delta;
        if !residual.is_finite() || residual > 0.0 || residual.abs() > EPSILON {
            return false;
        }

        *active_mover_usable_energy = next_usable;
        ledger.total_usable_energy_gained = next_usable_gained;
        ledger.total_pair_interaction_potential_held = next_pair_potential;
        return true;
    }

    let released = -delta;
    let negative_component_total = components
        .iter()
        .filter(|component| component.energy < 0.0)
        .map(|component| -component.energy)
        .sum::<f64>();
    if !released.is_finite()
        || !negative_component_total.is_finite()
        || negative_component_total < released
        || negative_component_total <= 0.0
    {
        return false;
    }

    for (index, holder) in biological_resonance_holders.iter().enumerate() {
        if biological_resonance_holders[..index]
            .iter()
            .any(|previous| std::ptr::eq(*previous, **holder))
        {
            return false;
        }
    }
    let original_resonance_total = biological_resonance_holders
        .iter()
        .map(|reservoir| reservoir.total_energy())
        .collect::<Option<Vec<_>>>()
        .map(|totals| totals.into_iter().sum::<f64>());
    let Some(original_resonance_total) = original_resonance_total else {
        return false;
    };
    if !original_resonance_total.is_finite() || original_resonance_total < 0.0 {
        return false;
    }
    let mut staged_resonance = biological_resonance_holders
        .iter()
        .map(|reservoir| (**reservoir).clone())
        .collect::<Vec<_>>();
    for component in components.iter().filter(|component| component.energy < 0.0) {
        let share = released * (-component.energy / negative_component_total);
        if !share.is_finite() || share < 0.0 {
            return false;
        }
        let mut remaining = share;
        for reservoir in &mut staged_resonance {
            let before = match reservoir.total_energy() {
                Some(total) => total,
                None => return false,
            };
            if !reservoir.deposit(component.frequency_cycles_per_day, remaining) {
                continue;
            }
            let after = match reservoir.total_energy() {
                Some(total) => total,
                None => return false,
            };
            let accepted = after - before;
            if !accepted.is_finite() || accepted < 0.0 || accepted > remaining + EPSILON {
                return false;
            }
            remaining = (remaining - accepted).max(0.0);
            if remaining <= f64::EPSILON {
                break;
            }
        }
    }

    let retained_total = staged_resonance
        .iter()
        .map(|reservoir| reservoir.total_energy())
        .collect::<Option<Vec<_>>>()
        .map(|totals| totals.into_iter().sum::<f64>());
    let Some(retained_total) = retained_total else {
        return false;
    };
    let retained_delta = retained_total - original_resonance_total;
    if !retained_delta.is_finite() || retained_delta < 0.0 || retained_delta > released + EPSILON {
        return false;
    }
    let next_resonant_held = ledger.total_resonant_energy_held + retained_delta;
    let heat_delta = released - retained_delta;
    let mut next_heat = ledger.total_heat_dissipated + heat_delta;
    if !next_resonant_held.is_finite() || !next_heat.is_finite() || heat_delta < 0.0 {
        return false;
    }

    let actual_pair_delta = next_pair_potential - ledger.total_pair_interaction_potential_held;
    let actual_resonant_delta = next_resonant_held - ledger.total_resonant_energy_held;
    let mut actual_heat_delta = next_heat - ledger.total_heat_dissipated;
    let mut residual = actual_pair_delta + actual_resonant_delta + actual_heat_delta;
    if !residual.is_finite() {
        return false;
    }
    // If cumulative f64 counters round the allocation upward, debit only this
    // settlement's new heat. Never disguise an over-allocation as tolerance.
    if residual > 0.0 {
        next_heat -= residual;
        if !next_heat.is_finite() || next_heat < ledger.total_heat_dissipated {
            return false;
        }
        actual_heat_delta = next_heat - ledger.total_heat_dissipated;
        residual = actual_pair_delta + actual_resonant_delta + actual_heat_delta;
    }
    if !residual.is_finite() || residual > 0.0 || residual.abs() > EPSILON {
        return false;
    }

    for (target, staged) in biological_resonance_holders
        .iter_mut()
        .zip(staged_resonance.into_iter())
    {
        **target = staged;
    }
    ledger.total_pair_interaction_potential_held = next_pair_potential;
    ledger.total_resonant_energy_held = next_resonant_held;
    ledger.total_heat_dissipated = next_heat;
    true
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
            || transaction.external_input > budget.remaining
        {
            return false;
        }
        let next_remaining = budget.remaining - transaction.external_input;
        if !next_remaining.is_finite() || next_remaining < 0.0 {
            return false;
        }
        if !self.settle_external_input(holder, transaction) {
            return false;
        }
        budget.remaining = next_remaining;
        true
    }

    fn settle_external_input(
        &mut self,
        holder: &mut f64,
        transaction: ExternalInputTransaction,
    ) -> bool {
        if (transaction.source == ExternalEnergySource::InitialCondition
            && (self.initial_condition_energy_settled || transaction.external_input <= 0.0))
            || !holder.is_finite()
            || *holder < 0.0
            || !transaction.balanced()
        {
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
        if next_holder < 0.0 {
            return false;
        }
        *holder = next_holder;
        self.total_external_energy_input = next_external;
        self.total_usable_energy_gained = next_gained;
        self.total_work_expended = next_work;
        self.total_heat_dissipated = next_heat;
        if transaction.source == ExternalEnergySource::InitialCondition {
            self.initial_condition_energy_settled = true;
        }
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
        let residual = incident_energy - allocated;
        if !allocated.is_finite()
            || !residual.is_finite()
            || residual < 0.0
            || residual > EPSILON * incident_energy.max(1.0)
        {
            return false;
        }
        let next_external = self.total_external_energy_input + incident_energy;
        let next_resonant = self.total_resonant_energy_held + resonant_energy_captured;
        let mut next_heat = self.total_heat_dissipated + heat_dissipated;
        let next_escaped = self.total_world_tone_energy_escaped + escaped_energy;
        if !next_external.is_finite()
            || !next_resonant.is_finite()
            || !next_heat.is_finite()
            || !next_escaped.is_finite()
        {
            return false;
        }

        // Validate the increments that the cumulative f64 counters can
        // actually represent, not just the ideal per-frame allocations. Adding
        // a small frame to a large cumulative total can round the source and
        // sinks in different directions; the aggregate world inventory must
        // never report more tone energy allocated than actually credited.
        let actual_incident = next_external - self.total_external_energy_input;
        let actual_capture = next_resonant - self.total_resonant_energy_held;
        let actual_escape = next_escaped - self.total_world_tone_energy_escaped;
        let mut actual_heat = next_heat - self.total_heat_dissipated;
        let mut cumulative_residual =
            actual_incident - (actual_capture + actual_heat + actual_escape);
        if !actual_incident.is_finite()
            || actual_incident < 0.0
            || !actual_capture.is_finite()
            || actual_capture < 0.0
            || !actual_escape.is_finite()
            || actual_escape < 0.0
            || !actual_heat.is_finite()
            || actual_heat < 0.0
            || !cumulative_residual.is_finite()
        {
            return false;
        }
        if cumulative_residual < 0.0 {
            // The cumulative counters introduced an ulp-scale over-allocation.
            // Debit only the new frame's heat. If subtraction rounds away, step
            // down one representable f64; a tiny unallocated remainder is safer
            // than accepting energy creation.
            let overrun = -cumulative_residual;
            if overrun > actual_heat {
                return false;
            }
            let mut corrected_heat = next_heat - overrun;
            if corrected_heat >= next_heat {
                if next_heat <= self.total_heat_dissipated {
                    return false;
                }
                corrected_heat = f64::from_bits(next_heat.to_bits() - 1);
            }
            if !corrected_heat.is_finite() || corrected_heat < self.total_heat_dissipated {
                return false;
            }
            next_heat = corrected_heat;
            actual_heat = next_heat - self.total_heat_dissipated;
            cumulative_residual = actual_incident - (actual_capture + actual_heat + actual_escape);
            if !actual_heat.is_finite() || actual_heat < 0.0 || cumulative_residual < 0.0 {
                return false;
            }
        }

        self.total_external_energy_input = next_external;
        self.total_resonant_energy_held = next_resonant;
        self.total_heat_dissipated = next_heat;
        self.total_world_tone_energy_escaped = next_escaped;
        true
    }

    fn settle_transaction(&mut self, holder: &mut f64, transaction: EnergyTransaction) -> bool {
        if !holder.is_finite() || *holder < 0.0 || !transaction.balanced() {
            #[cfg(test)]
            eprintln!(
                "[settle-reject:precondition] holder={:?} balanced={:?} allocation={:?}",
                *holder,
                transaction.balanced(),
                transaction.allocation()
            );
            return false;
        }
        let mut next_holder = *holder + transaction.usable_delta;
        if !next_holder.is_finite() || next_holder < 0.0 {
            #[cfg(test)]
            eprintln!(
                "[settle-reject:holder] before={:?} usable_delta={:?} next={:?}",
                *holder, transaction.usable_delta, next_holder
            );
            return false;
        }
        let next_released = self.total_potential_energy_released + transaction.potential_released;
        let next_structural = self.total_structural_energy_delta + transaction.structural_delta;
        let next_work = self.total_work_expended + transaction.work_expended;
        let next_heat = self.total_heat_dissipated + transaction.heat_dissipated;
        if !next_released.is_finite()
            || !next_structural.is_finite()
            || !next_work.is_finite()
            || !next_heat.is_finite()
        {
            #[cfg(test)]
            eprintln!(
                "[settle-reject:counters] next=(released={:?},structural={:?},work={:?},heat={:?})",
                next_released, next_structural, next_work, next_heat
            );
            return false;
        }

        // A transaction can be balanced algebraically but become unbalanced
        // after the holder and cumulative f64 counters round their increments.
        // Use the increments those stores actually represent. If rounding would
        // create energy, move the usable holder down by one representable step.
        // Do not manufacture extra heat to hide an unrepresentable transaction:
        // reject atomically if the resulting non-creating residual exceeds the
        // existing tolerance.
        let actual_work_delta = next_work - self.total_work_expended;
        let actual_heat_delta = next_heat - self.total_heat_dissipated;
        let residual_for = |candidate_holder: f64| {
            let actual_usable_delta = candidate_holder - *holder;
            actual_usable_delta
                + transaction.structural_delta
                + actual_work_delta
                + actual_heat_delta
        };
        let mut residual = residual_for(next_holder);
        let mut rounding_steps = 0;
        while residual > 0.0 {
            if next_holder <= 0.0 || rounding_steps >= 64 {
                #[cfg(test)]
                eprintln!(
                    "[settle-reject:rounding-loop] before={:?} next={:?} transaction={:?} actual_work={:?} actual_heat={:?} residual={:?} steps={}",
                    *holder,
                    next_holder,
                    transaction,
                    actual_work_delta,
                    actual_heat_delta,
                    residual,
                    rounding_steps
                );
                return false;
            }
            next_holder = f64::from_bits(next_holder.to_bits() - 1);
            residual = residual_for(next_holder);
            rounding_steps += 1;
        }
        if !residual.is_finite() || residual.abs() > EPSILON {
            #[cfg(test)]
            eprintln!(
                "[settle-reject:residual] before={:?} next={:?} transaction={:?} actual_work={:?} actual_heat={:?} residual={:?} steps={}",
                *holder,
                next_holder,
                transaction,
                actual_work_delta,
                actual_heat_delta,
                residual,
                rounding_steps
            );
            return false;
        }

        let actual_usable_delta = next_holder - *holder;
        let next_gained = self.total_usable_energy_gained + actual_usable_delta.max(0.0);
        if !next_gained.is_finite() {
            return false;
        }

        *holder = next_holder;
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
            || *from < 0.0
            || *to < 0.0
            || *from < amount
        {
            return false;
        }
        let next_from = *from - amount;
        let next_to = *to + amount;
        if next_from < 0.0 || !next_to.is_finite() {
            return false;
        }
        *from = next_from;
        *to = next_to;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positive_pair_potential_change_is_paid_only_by_active_mover() {
        let mut ledger = EnergyLedger::default();
        let mut usable = 10.0;
        let mut resonance = crate::harmonics::ResonantEnergyReservoir::default();
        let components = [crate::harmonics::ResonancePairPotentialComponent {
            frequency_cycles_per_day: 440.0,
            energy: 3.0,
        }];

        assert!(settle_pair_potential_change(
            &mut ledger,
            &mut usable,
            &mut [&mut resonance],
            &components,
        ));
        assert_eq!(usable, 7.0);
        assert_eq!(resonance.total_energy(), Some(0.0));
        assert_eq!(ledger.total_pair_interaction_potential_held, 3.0);
        assert_eq!(ledger.total_work_expended, 0.0);
        assert_eq!(ledger.total_heat_dissipated, 0.0);
    }

    #[test]
    fn unaffordable_positive_pair_potential_change_rolls_back_atomically() {
        let mut ledger = EnergyLedger::default();
        let before = ledger;
        let mut usable = 2.0;
        let mut resonance = crate::harmonics::ResonantEnergyReservoir::default();
        let resonance_before = resonance.clone();
        let components = [crate::harmonics::ResonancePairPotentialComponent {
            frequency_cycles_per_day: 440.0,
            energy: 3.0,
        }];

        assert!(!settle_pair_potential_change(
            &mut ledger,
            &mut usable,
            &mut [&mut resonance],
            &components,
        ));
        assert_eq!(usable, 2.0);
        assert_eq!(resonance, resonance_before);
        assert_eq!(ledger, before);
    }

    #[test]
    fn negative_pair_potential_change_is_retained_by_frequency_without_work() {
        let mut ledger = EnergyLedger::default();
        let mut usable = 10.0;
        let mut resonance = crate::harmonics::ResonantEnergyReservoir::default();
        let components = [
            crate::harmonics::ResonancePairPotentialComponent {
                frequency_cycles_per_day: 220.0,
                energy: -6.0,
            },
            crate::harmonics::ResonancePairPotentialComponent {
                frequency_cycles_per_day: 440.0,
                energy: -4.0,
            },
            crate::harmonics::ResonancePairPotentialComponent {
                frequency_cycles_per_day: 880.0,
                energy: 2.0,
            },
        ];

        assert!(settle_pair_potential_change(
            &mut ledger,
            &mut usable,
            &mut [&mut resonance],
            &components,
        ));
        assert_eq!(usable, 10.0);
        assert!((resonance.total_energy().unwrap() - 8.0).abs() < 1e-12);
        assert_eq!(ledger.total_pair_interaction_potential_held, -8.0);
        assert_eq!(ledger.total_resonant_energy_held, 8.0);
        assert_eq!(ledger.total_heat_dissipated, 0.0);
        assert_eq!(ledger.total_work_expended, 0.0);
        assert_eq!(resonance.components.len(), 2);
        assert!((resonance.components[0].energy - 4.8).abs() < 1e-12);
        assert!((resonance.components[1].energy - 3.2).abs() < 1e-12);
    }

    #[test]
    fn unretained_pair_potential_release_becomes_heat_once() {
        let mut ledger = EnergyLedger::default();
        let mut usable = 0.0;
        let components = [crate::harmonics::ResonancePairPotentialComponent {
            frequency_cycles_per_day: 440.0,
            energy: -5.0,
        }];

        assert!(settle_pair_potential_change(
            &mut ledger,
            &mut usable,
            &mut [],
            &components,
        ));
        assert_eq!(ledger.total_pair_interaction_potential_held, -5.0);
        assert_eq!(ledger.total_resonant_energy_held, 0.0);
        assert_eq!(ledger.total_heat_dissipated, 5.0);
        assert_eq!(ledger.total_work_expended, 0.0);
    }

    #[test]
    fn full_resonance_spectrum_routes_unretained_pair_release_to_heat() {
        let mut ledger = EnergyLedger::default();
        let mut usable = 0.0;
        let mut resonance = crate::harmonics::ResonantEnergyReservoir::default();
        for index in 0..8 {
            assert!(resonance.deposit(100.0 + index as f64 * 100.0, 1.0));
        }
        ledger.total_resonant_energy_held = 8.0;
        ledger.initial_resonant_energy_held = 8.0;
        let components = [crate::harmonics::ResonancePairPotentialComponent {
            frequency_cycles_per_day: 2_000.0,
            energy: -2.5,
        }];

        assert!(settle_pair_potential_change(
            &mut ledger,
            &mut usable,
            &mut [&mut resonance],
            &components,
        ));
        assert_eq!(resonance.total_energy(), Some(8.0));
        assert_eq!(ledger.total_resonant_energy_held, 0.0);
        assert_eq!(ledger.total_pair_interaction_potential_held, -2.5);
        assert_eq!(ledger.total_heat_dissipated, 2.5);
    }

    #[test]
    fn invalid_pair_potential_components_are_rejected_without_mutation() {
        let mut ledger = EnergyLedger::default();
        let before = ledger;
        let mut usable = 5.0;
        let mut resonance = crate::harmonics::ResonantEnergyReservoir::default();
        let components = [crate::harmonics::ResonancePairPotentialComponent {
            frequency_cycles_per_day: f64::NAN,
            energy: -1.0,
        }];

        assert!(!settle_pair_potential_change(
            &mut ledger,
            &mut usable,
            &mut [&mut resonance],
            &components,
        ));
        assert_eq!(ledger, before);
        assert_eq!(usable, 5.0);
        assert_eq!(resonance.total_energy(), Some(0.0));
    }

    #[test]
    fn pair_potential_inventory_balances_positive_change_against_usable_energy() {
        let inventory = EnergyInventorySnapshot {
            initial_usable_energy: 10.0,
            current_usable_energy: 7.0,
            initial_resonant_energy: 0.0,
            current_resonant_energy: 0.0,
            initial_structural_energy: 0.0,
            current_structural_energy: 0.0,
            initial_pair_interaction_potential: 0.0,
            current_pair_interaction_potential: 3.0,
            external_energy_input: 0.0,
            world_tone_energy_escaped: 0.0,
            other_energy_released: 0.0,
            work_expended: 0.0,
            heat_dissipated: 0.0,
        };
        assert_eq!(inventory.residual(), Some(0.0));
        assert!(inventory.is_balanced_without_creation(1e-12));
    }

    #[test]
    fn pair_potential_inventory_balances_retained_release() {
        let inventory = EnergyInventorySnapshot {
            initial_usable_energy: 0.0,
            current_usable_energy: 0.0,
            initial_resonant_energy: 2.0,
            current_resonant_energy: 10.0,
            initial_structural_energy: 0.0,
            current_structural_energy: 0.0,
            initial_pair_interaction_potential: 0.0,
            current_pair_interaction_potential: -8.0,
            external_energy_input: 0.0,
            world_tone_energy_escaped: 0.0,
            other_energy_released: 0.0,
            work_expended: 0.0,
            heat_dissipated: 0.0,
        };
        assert_eq!(inventory.residual(), Some(0.0));
        assert!(inventory.is_balanced_without_creation(1e-12));
    }

    #[test]
    fn pair_potential_inventory_balances_unretained_release_as_heat() {
        let inventory = EnergyInventorySnapshot {
            initial_usable_energy: 0.0,
            current_usable_energy: 0.0,
            initial_resonant_energy: 0.0,
            current_resonant_energy: 0.0,
            initial_structural_energy: 0.0,
            current_structural_energy: 0.0,
            initial_pair_interaction_potential: 0.0,
            current_pair_interaction_potential: -5.0,
            external_energy_input: 0.0,
            world_tone_energy_escaped: 0.0,
            other_energy_released: 0.0,
            work_expended: 0.0,
            heat_dissipated: 5.0,
        };
        assert_eq!(inventory.residual(), Some(0.0));
        assert!(inventory.is_balanced_without_creation(1e-12));
    }

    #[test]
    fn favorable_reaction_can_fund_work_without_external_input() {
        let balance = ReactionEnergyBalance::calculate(10.0, 6.0, 2.0, 1.0).unwrap();
        assert_eq!(balance.net_reaction_change, -4.0);
        assert_eq!(balance.required_external_input, 0.0);
        assert_eq!(balance.released_surplus, 1.0);
        assert!(balance.can_commit_with(0.0));
        assert_eq!(balance.residual(0.0), Some(0.0));
    }

    #[test]
    fn stable_bond_formation_is_a_negative_product_potential_change() {
        let balance = ReactionEnergyBalance::calculate(0.0, -5.0, 2.0, 1.0).unwrap();
        assert_eq!(balance.net_reaction_change, -5.0);
        assert_eq!(balance.required_external_input, 0.0);
        assert_eq!(balance.released_surplus, 2.0);
        assert_eq!(balance.residual(0.0), Some(0.0));
    }

    #[test]
    fn unfavorable_reaction_requires_legitimate_input_for_the_deficit() {
        let balance = ReactionEnergyBalance::calculate(3.0, 8.0, 1.0, 0.5).unwrap();
        assert_eq!(balance.net_reaction_change, 5.0);
        assert_eq!(balance.required_external_input, 6.5);
        assert_eq!(balance.released_surplus, 0.0);
        assert!(!balance.can_commit_with(6.49));
        assert!(balance.can_commit_with(6.5));
        assert_eq!(balance.residual(6.5), Some(0.0));
    }

    #[test]
    fn reaction_balance_accepts_tiny_negative_algebraic_rounding() {
        let transaction = EnergyTransaction::reaction(
            EnergyReason::Break,
            12.98158853027627,
            1.258588936436521,
            0.21107516949790017,
        )
        .unwrap();
        let allocation = transaction.allocation();
        let residual = allocation.source_energy
            - allocation.usable_delta
            - allocation.structural_delta
            - allocation.work_expended
            - allocation.heat_dissipated;
        assert!(residual < 0.0 && residual.abs() < EPSILON);
        assert!(
            transaction.balanced(),
            "signed algebraic rounding within tolerance must reach represented-state settlement"
        );

        let mut ledger = EnergyLedger::default();
        let mut usable = 1_000.0;
        assert!(
            ledger.settle_transaction(&mut usable, transaction),
            "the represented transaction must settle without creating energy"
        );
    }

    #[test]
    fn reaction_settlement_corrects_tiny_rounding_without_inventing_heat() {
        // At this holder magnitude, the nominal +0.07 increment rounds to a
        // slightly larger represented delta. Correct only the holder by one
        // ULP, leaving a tiny non-creating residual within the existing limit.
        let transaction =
            EnergyTransaction::reaction(EnergyReason::Combine, -0.1, 0.03, 0.0).unwrap();
        let mut ledger = EnergyLedger::default();
        let mut usable = 1_000.0;
        let initial_usable = usable;

        assert!(ledger.settle_transaction(&mut usable, transaction));

        let actual_usable_delta = usable - initial_usable;
        let residual = actual_usable_delta
            + transaction.structural_delta
            + ledger.total_work_expended
            + ledger.total_heat_dissipated;
        assert!(
            residual <= 0.0 && residual.abs() <= EPSILON,
            "rounded reaction settlement must not create energy: usable_delta={actual_usable_delta}, work={}, heat={}, residual={residual}",
            ledger.total_work_expended,
            ledger.total_heat_dissipated
        );
        assert_eq!(ledger.total_usable_energy_gained, actual_usable_delta);
        assert_eq!(ledger.total_heat_dissipated, 0.0);
    }

    #[test]
    fn reaction_settlement_rejects_unrepresentable_rounding_atomically() {
        // At this magnitude, adding +99 rounds to +100 and subtracting one ULP
        // would overshoot by a full energy unit. Do not disguise that mismatch
        // as heat; reject without changing the holder or ledger.
        let transaction =
            EnergyTransaction::reaction(EnergyReason::Combine, -100.0, 1.0, 0.0).unwrap();
        let mut ledger = EnergyLedger::default();
        let ledger_before = ledger;
        let mut usable = 1.0e16;
        let usable_before = usable;

        assert!(!ledger.settle_transaction(&mut usable, transaction));
        assert_eq!(usable, usable_before);
        assert_eq!(ledger, ledger_before);
    }

    #[test]
    fn energy_neutral_reaction_still_accounts_for_work_and_heat() {
        let balance = ReactionEnergyBalance::calculate(4.0, 4.0, 1.0, 0.25).unwrap();
        assert_eq!(balance.net_reaction_change, 0.0);
        assert_eq!(balance.required_external_input, 1.25);
        assert_eq!(balance.released_surplus, 0.0);
    }

    #[test]
    fn surplus_external_input_is_retained_without_being_double_counted() {
        let balance = ReactionEnergyBalance::calculate(2.0, 5.0, 1.0, 0.0).unwrap();
        assert_eq!(balance.required_external_input, 4.0);
        assert!(balance.can_commit_with(7.0));
        assert_eq!(balance.residual(7.0), Some(0.0));
    }

    #[test]
    fn reaction_balance_rejects_non_finite_or_negative_state_values() {
        assert!(ReactionEnergyBalance::calculate(0.0, f64::NAN, 0.0, 0.0).is_none());
        assert!(ReactionEnergyBalance::calculate(0.0, 0.0, -1.0, 0.0).is_none());
        assert!(ReactionEnergyBalance::calculate(0.0, 0.0, f64::INFINITY, 0.0).is_none());
        let balance = ReactionEnergyBalance::calculate(1.0, 1.0, 0.0, 0.0).unwrap();
        assert!(!balance.can_commit_with(f64::NAN));
        assert!(balance.residual(-1.0).is_none());
    }

    #[test]
    fn favorable_bond_formation_releases_binding_energy_after_work_and_heat() {
        let transaction =
            EnergyTransaction::reaction(EnergyReason::Combine, -5.0, 2.0, 1.0).unwrap();
        assert_eq!(transaction.usable_delta, 2.0);
        assert!(transaction.balanced());

        let mut ledger = EnergyLedger::default();
        let mut usable = 0.0;
        assert!(ledger.settle_transaction(&mut usable, transaction));
        assert_eq!(usable, 2.0);
        assert_eq!(ledger.total_structural_energy_delta, -5.0);
        assert_eq!(ledger.total_work_expended, 2.0);
        assert_eq!(ledger.total_heat_dissipated, 1.0);
    }

    #[test]
    fn bond_breaking_requires_input_for_binding_energy_and_work() {
        let transaction = EnergyTransaction::reaction(EnergyReason::Break, 5.0, 1.0, 0.0).unwrap();
        assert_eq!(transaction.usable_delta, -6.0);
        assert!(transaction.balanced());

        let mut ledger = EnergyLedger::default();
        let mut insufficient = 5.0;
        assert!(!ledger.settle_transaction(&mut insufficient, transaction));
        assert_eq!(insufficient, 5.0);
        assert_eq!(ledger.total_structural_energy_delta, 0.0);

        let mut sufficient = 6.0;
        assert!(ledger.settle_transaction(&mut sufficient, transaction));
        assert_eq!(sufficient, 0.0);
        assert_eq!(ledger.total_structural_energy_delta, 5.0);
    }

    #[test]
    fn strict_world_inventory_oracle_rejects_even_sub_tolerance_energy_creation() {
        let created = EnergyInventorySnapshot {
            initial_usable_energy: 0.0,
            current_usable_energy: 5e-10,
            initial_resonant_energy: 0.0,
            current_resonant_energy: 0.0,
            initial_structural_energy: 0.0,
            current_structural_energy: 0.0,
            initial_pair_interaction_potential: 0.0,
            current_pair_interaction_potential: 0.0,
            external_energy_input: 0.0,
            world_tone_energy_escaped: 0.0,
            other_energy_released: 0.0,
            work_expended: 0.0,
            heat_dissipated: 0.0,
        };
        assert!(created.is_balanced(1e-9));
        assert!(!created.is_balanced_without_creation(1e-9));

        let rounding_loss = EnergyInventorySnapshot {
            external_energy_input: 5e-10,
            ..created
        };
        assert!(rounding_loss.is_balanced_without_creation(1e-9));
    }

    #[test]
    fn world_inventory_counts_retained_resonance_as_a_distinct_energy_store() {
        let inventory = EnergyInventorySnapshot {
            initial_usable_energy: 10.0,
            current_usable_energy: 10.0,
            initial_resonant_energy: 0.0,
            current_resonant_energy: 3.0,
            initial_structural_energy: 4.0,
            current_structural_energy: 4.0,
            initial_pair_interaction_potential: 0.0,
            current_pair_interaction_potential: 0.0,
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
        assert_eq!(ledger.total_resonant_energy_held, 4.0);
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
            initial_pair_interaction_potential: 0.0,
            current_pair_interaction_potential: 0.0,
            external_energy_input: ledger.total_external_energy_input,
            world_tone_energy_escaped: ledger.total_world_tone_energy_escaped,
            other_energy_released: 0.0,
            work_expended: 0.0,
            heat_dissipated: ledger.total_heat_dissipated,
        };
        assert!(inventory.is_balanced(1e-12));
    }

    #[test]
    fn cumulative_world_tone_rounding_is_debited_from_new_heat() {
        let mut ledger = EnergyLedger {
            total_external_energy_input: 7223.128207,
            total_resonant_energy_held: 1501893.525368,
            total_heat_dissipated: 177.428491,
            total_world_tone_energy_escaped: 611232.425726,
            ..EnergyLedger::default()
        };
        let before = ledger;

        assert!(ledger.settle_world_tone_paths(80.330104, 17.734988, 17.793927, 44.801189,));

        let actual_incident =
            ledger.total_external_energy_input - before.total_external_energy_input;
        let actual_capture = ledger.total_resonant_energy_held - before.total_resonant_energy_held;
        let actual_heat = ledger.total_heat_dissipated - before.total_heat_dissipated;
        let actual_escape =
            ledger.total_world_tone_energy_escaped - before.total_world_tone_energy_escaped;
        assert!(
            actual_incident >= actual_capture + actual_heat + actual_escape,
            "cumulative f64 counters must not allocate more energy than the represented source"
        );
        assert!(
            actual_heat < 17.793927,
            "rounding correction must be debited from this frame's heat, not hidden by relaxing conservation"
        );
    }

    #[test]
    fn world_tone_cannot_allocate_more_than_incident_energy_within_tolerance() {
        let mut ledger = EnergyLedger::default();
        assert!(
            !ledger.settle_world_tone_paths(1.0, 1.0 + EPSILON * 0.5, 0.0, 0.0),
            "world-tone allocation must not exceed the incident source"
        );
        assert_eq!(ledger.total_external_energy_input, 0.0);
        assert_eq!(ledger.total_resonant_energy_held, 0.0);
        assert_eq!(ledger.total_heat_dissipated, 0.0);
        assert_eq!(ledger.total_world_tone_energy_escaped, 0.0);
    }

    #[test]
    fn zero_incident_world_tone_cannot_create_energy_within_tolerance() {
        let mut ledger = EnergyLedger::default();
        assert!(
            !ledger.settle_world_tone_paths(0.0, 5e-10, 0.0, 0.0),
            "a zero-energy signal cannot credit a nonzero captured amount just because it is within tolerance"
        );
        assert_eq!(ledger.total_external_energy_input, 0.0);
        assert_eq!(ledger.total_resonant_energy_held, 0.0);
        assert_eq!(ledger.total_heat_dissipated, 0.0);
        assert_eq!(ledger.total_world_tone_energy_escaped, 0.0);
    }

    #[test]
    fn world_tone_settlement_rejects_counter_overflow_without_partial_updates() {
        let mut ledger = EnergyLedger {
            total_external_energy_input: f64::MAX,
            total_heat_dissipated: 7.0,
            total_world_tone_energy_escaped: 3.0,
            ..EnergyLedger::default()
        };

        assert!(
            !ledger.settle_world_tone_paths(f64::MAX, 0.0, 0.0, f64::MAX),
            "external-input counter overflow must reject the entire settlement"
        );
        assert_eq!(ledger.total_external_energy_input, f64::MAX);
        assert_eq!(ledger.total_heat_dissipated, 7.0);
        assert_eq!(ledger.total_world_tone_energy_escaped, 3.0);

        let mut heat_overflow = EnergyLedger {
            total_external_energy_input: 11.0,
            total_heat_dissipated: f64::MAX,
            total_world_tone_energy_escaped: 13.0,
            ..EnergyLedger::default()
        };
        assert!(
            !heat_overflow.settle_world_tone_paths(f64::MAX, 0.0, f64::MAX, 0.0),
            "heat counter overflow must reject the entire settlement"
        );
        assert_eq!(heat_overflow.total_external_energy_input, 11.0);
        assert_eq!(heat_overflow.total_heat_dissipated, f64::MAX);
        assert_eq!(heat_overflow.total_world_tone_energy_escaped, 13.0);

        let mut escape_overflow = EnergyLedger {
            total_external_energy_input: 17.0,
            total_heat_dissipated: 19.0,
            total_world_tone_energy_escaped: f64::MAX,
            ..EnergyLedger::default()
        };
        assert!(
            !escape_overflow.settle_world_tone_paths(f64::MAX, 0.0, 0.0, f64::MAX),
            "escape counter overflow must reject the entire settlement"
        );
        assert_eq!(escape_overflow.total_external_energy_input, 17.0);
        assert_eq!(escape_overflow.total_heat_dissipated, 19.0);
        assert_eq!(escape_overflow.total_world_tone_energy_escaped, f64::MAX);

        let mut resonant_overflow = EnergyLedger {
            total_external_energy_input: 5.0,
            total_resonant_energy_held: f64::MAX,
            total_heat_dissipated: 7.0,
            total_world_tone_energy_escaped: 3.0,
            ..EnergyLedger::default()
        };
        assert!(
            !resonant_overflow.settle_world_tone_paths(f64::MAX, f64::MAX, 0.0, 0.0),
            "resonant-holder counter overflow must reject the entire settlement"
        );
        assert_eq!(resonant_overflow.total_external_energy_input, 5.0);
        assert_eq!(resonant_overflow.total_resonant_energy_held, f64::MAX);
        assert_eq!(resonant_overflow.total_heat_dissipated, 7.0);
        assert_eq!(resonant_overflow.total_world_tone_energy_escaped, 3.0);

        assert!(
            !ledger.settle_world_tone_paths(f64::INFINITY, 0.0, 0.0, 0.0),
            "non-finite incident energy must reject before any ledger mutation"
        );
        assert_eq!(ledger.total_external_energy_input, f64::MAX);
        assert_eq!(ledger.total_heat_dissipated, 7.0);
        assert_eq!(ledger.total_world_tone_energy_escaped, 3.0);
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
            initial_pair_interaction_potential: 0.0,
            current_pair_interaction_potential: 0.0,
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
            initial_pair_interaction_potential: 0.0,
            current_pair_interaction_potential: 0.0,
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
    fn world_inventory_rejects_unowned_non_structural_release() {
        let snapshot = EnergyInventorySnapshot {
            initial_usable_energy: 0.0,
            current_usable_energy: 1.0,
            initial_resonant_energy: 0.0,
            current_resonant_energy: 0.0,
            initial_structural_energy: 0.0,
            current_structural_energy: 0.0,
            initial_pair_interaction_potential: 0.0,
            current_pair_interaction_potential: 0.0,
            external_energy_input: 0.0,
            world_tone_energy_escaped: 0.0,
            other_energy_released: 1.0,
            work_expended: 0.0,
            heat_dissipated: 0.0,
        };
        assert_eq!(snapshot.residual(), None);
        assert!(!snapshot.is_balanced(1.0));
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
            initial_pair_interaction_potential: 0.0,
            current_pair_interaction_potential: 0.0,
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
                potential_released: 0.0,
                usable_delta: -3.0,
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
    fn favorable_bond_formation_can_be_represented_as_a_negative_structural_delta() {
        let tx = EnergyTransaction {
            reason: EnergyReason::Combine,
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
    fn reaction_energy_balance_favorable_reaction_funds_work_and_heat() {
        let balance = ReactionEnergyBalance::calculate(12.0, 5.0, 3.0, 1.0).unwrap();
        assert_eq!(balance.net_reaction_change, -7.0);
        assert_eq!(balance.required_external_input, 0.0);
        assert_eq!(balance.released_surplus, 3.0);
        assert_eq!(balance.residual(0.0), Some(0.0));
        assert!(balance.can_commit_with(0.0));
    }

    #[test]
    fn reaction_energy_balance_unfavorable_reaction_requires_real_input() {
        let balance = ReactionEnergyBalance::calculate(5.0, 9.0, 2.0, 1.0).unwrap();
        assert_eq!(balance.net_reaction_change, 4.0);
        assert_eq!(balance.required_external_input, 7.0);
        assert_eq!(balance.released_surplus, 0.0);
        assert!(!balance.can_commit_with(6.999));
        assert!(balance.can_commit_with(7.0));
        assert_eq!(balance.residual(7.0), Some(0.0));
    }

    #[test]
    fn reaction_energy_balance_neutral_reaction_accounts_for_work() {
        let balance = ReactionEnergyBalance::calculate(5.0, 5.0, 2.0, 1.0).unwrap();
        assert_eq!(balance.required_external_input, 3.0);
        assert_eq!(balance.released_surplus, 0.0);
        assert_eq!(balance.residual(3.0), Some(0.0));
    }

    #[test]
    fn reaction_transaction_reconciles_favorable_and_unfavorable_structural_changes() {
        let favorable = ReactionEnergyBalance::calculate(12.0, 5.0, 3.0, 1.0)
            .unwrap()
            .settlement_transaction(EnergyReason::Combine)
            .unwrap();
        assert!(favorable.balanced());
        assert_eq!(favorable.usable_delta, 3.0);

        let unfavorable = ReactionEnergyBalance::calculate(5.0, 12.0, 3.0, 1.0)
            .unwrap()
            .settlement_transaction(EnergyReason::Break)
            .unwrap();
        assert!(unfavorable.balanced());
        assert_eq!(unfavorable.usable_delta, -11.0);
    }

    #[test]
    fn favorable_reaction_credits_surplus_once_without_external_input() {
        let mut ledger = EnergyLedger::default();
        let mut usable = 0.0;
        let transaction =
            EnergyTransaction::reaction(EnergyReason::Combine, -8.0, 3.0, 1.0).unwrap();

        assert!(ledger.settle_transaction(&mut usable, transaction));
        assert_eq!(usable, 4.0);
        assert_eq!(ledger.total_external_energy_input, 0.0);
        assert_eq!(ledger.total_structural_energy_delta, -8.0);
        assert_eq!(ledger.total_work_expended, 3.0);
        assert_eq!(ledger.total_heat_dissipated, 1.0);
    }

    #[test]
    fn unaffordable_break_is_rejected_without_mutating_holder_or_ledger() {
        let mut ledger = EnergyLedger::default();
        let mut usable = 2.0;
        let transaction = EnergyTransaction::reaction(EnergyReason::Break, 5.0, 1.0, 0.0).unwrap();

        assert!(!ledger.settle_transaction(&mut usable, transaction));
        assert_eq!(usable, 2.0);
        assert_eq!(ledger.total_structural_energy_delta, 0.0);
        assert_eq!(ledger.total_work_expended, 0.0);
        assert_eq!(ledger.total_heat_dissipated, 0.0);
        assert_eq!(ledger.total_usable_energy_gained, 0.0);
    }

    #[test]
    fn pure_expenditure_does_not_change_structural_potential() {
        let tx = EnergyTransaction::expenditure(EnergyReason::Combine, 0.0, 1.0, 2.0).unwrap();
        assert!(tx.balanced());
        assert_eq!(tx.potential_released, 0.0);
        assert_eq!(tx.usable_delta, -3.0);
        assert_eq!(tx.structural_delta, 0.0);
        assert_eq!(tx.work_expended, 1.0);
        assert_eq!(tx.heat_dissipated, 2.0);
    }

    #[test]
    fn pure_expenditure_rejects_structural_potential_changes() {
        assert!(EnergyTransaction::expenditure(EnergyReason::Combine, 1.0, 0.0, 0.0).is_none());
        assert!(EnergyTransaction::expenditure(EnergyReason::Combine, -1.0, 0.0, 0.0).is_none());
    }

    #[test]
    fn unowned_non_structural_release_is_rejected_atomically() {
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
        assert!(!ledger.settle_transaction(&mut energy, tx));
        assert_eq!(energy, 10.0);
        assert_eq!(ledger.total_potential_energy_released, 0.0);
        assert_eq!(ledger.total_work_expended, 0.0);
    }

    #[test]
    fn initial_condition_energy_source_can_only_be_settled_once() {
        let mut ledger = EnergyLedger::default();
        let mut holder = 0.0;
        let initial = ExternalInputTransaction {
            source: ExternalEnergySource::InitialCondition,
            source_available: 12.5,
            external_input: 12.5,
            usable_delta: 12.5,
            work_expended: 0.0,
            heat_dissipated: 0.0,
        };

        assert!(ledger.settle_external_input(&mut holder, initial));
        assert_eq!(holder, 12.5);
        assert_eq!(ledger.total_external_energy_input, 12.5);
        assert!(ledger.initial_condition_energy_settled);

        assert!(!ledger.settle_external_input(&mut holder, initial));
        assert_eq!(
            holder, 12.5,
            "duplicate initial source must not credit holder"
        );
        assert_eq!(
            ledger.total_external_energy_input, 12.5,
            "duplicate initial source must not increase source accounting"
        );
    }

    #[test]
    fn settling_favorable_bond_formation_credits_surplus_without_faking_a_source() {
        let mut ledger = EnergyLedger::default();
        let mut energy = 0.0;
        let tx = EnergyTransaction {
            reason: EnergyReason::Combine,
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
    fn energy_transaction_cannot_create_energy_within_balance_tolerance() {
        let mut ledger = EnergyLedger::default();
        let mut holder = 0.0;
        let transaction = EnergyTransaction {
            reason: EnergyReason::Transfer,
            potential_released: 0.0,
            usable_delta: EPSILON * 0.5,
            structural_delta: 0.0,
            work_expended: 0.0,
            heat_dissipated: 0.0,
        };

        assert!(!ledger.settle_transaction(&mut holder, transaction));
        assert_eq!(holder, 0.0);
        assert_eq!(ledger.total_potential_energy_released, 0.0);
        assert_eq!(ledger.total_structural_energy_delta, 0.0);
        assert_eq!(ledger.total_usable_energy_gained, 0.0);
    }

    #[test]
    fn external_input_rejects_negative_holder_even_within_tolerance() {
        let mut ledger = EnergyLedger::default();
        let mut holder = -EPSILON * 0.5;
        let tx = external_input(0.0, 0.0, 0.0, 0.0, 0.0);

        assert!(!ledger.settle_external_input(&mut holder, tx));
        assert_eq!(holder, -EPSILON * 0.5);
        assert_eq!(ledger.total_external_energy_input, 0.0);
        assert_eq!(ledger.total_usable_energy_gained, 0.0);
    }

    #[test]
    fn zero_external_input_cannot_credit_holder_within_balance_tolerance() {
        let mut ledger = EnergyLedger::default();
        let mut holder = 0.0;
        let tx = external_input(0.0, 0.0, EPSILON * 0.5, 0.0, 0.0);

        assert!(
            !ledger.settle_external_input(&mut holder, tx),
            "zero external input cannot fund a positive usable-energy delta"
        );
        assert_eq!(holder, 0.0);
        assert_eq!(ledger.total_external_energy_input, 0.0);
        assert_eq!(ledger.total_usable_energy_gained, 0.0);
        assert_eq!(ledger.total_heat_dissipated, 0.0);
    }

    #[test]
    fn external_input_cannot_overdraw_local_source_allowance_within_tolerance() {
        let mut ledger = EnergyLedger::default();
        let mut holder = 0.0;
        let claim = 5.0 + EPSILON * 0.5;
        let tx = external_input(5.0, claim, claim, 0.0, 0.0);

        assert!(!ledger.settle_external_input(&mut holder, tx));
        assert_eq!(holder, 0.0);
        assert_eq!(ledger.total_external_energy_input, 0.0);
        assert_eq!(ledger.total_usable_energy_gained, 0.0);
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
    fn source_budget_rejects_sub_tolerance_overspend_atomically() {
        let mut ledger = EnergyLedger::default();
        let mut budget = EnergySourceBudget::new(ExternalEnergySource::WorldTone, 5.0).unwrap();
        let mut holder = 1.0;
        let claim = 5.0 + EPSILON * 0.5;
        let tx = external_input(claim, claim, claim, 0.0, 0.0);

        assert!(
            !ledger.settle_external_input_from_budget(&mut holder, &mut budget, tx),
            "source budget must not authorize external input beyond its exact remaining amount"
        );
        assert_eq!(holder, 1.0);
        assert_eq!(budget.remaining, 5.0);
        assert_eq!(ledger.total_external_energy_input, 0.0);
        assert_eq!(ledger.total_usable_energy_gained, 0.0);
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
    fn energy_transaction_rejects_negative_holder_even_within_tolerance() {
        let mut ledger = EnergyLedger::default();
        let mut holder = -EPSILON * 0.5;
        let transaction =
            EnergyTransaction::expenditure(EnergyReason::Transfer, 0.0, 0.0, 0.0).unwrap();

        assert!(!ledger.settle_transaction(&mut holder, transaction));
        assert_eq!(holder, -EPSILON * 0.5);
        assert_eq!(ledger.total_potential_energy_released, 0.0);
        assert_eq!(ledger.total_structural_energy_delta, 0.0);
        assert_eq!(ledger.total_usable_energy_gained, 0.0);
    }

    #[test]
    fn transfer_rejects_sub_tolerance_overdraw_without_creating_energy() {
        let mut ledger = EnergyLedger::default();
        let mut from = 1.0;
        let mut to = 0.0;
        let total_before = from + to;
        assert!(
            !ledger.transfer(&mut from, &mut to, 1.0 + EPSILON * 0.5),
            "a transfer must not spend more than its source holder contains"
        );
        assert_eq!(from + to, total_before);
        assert_eq!(from, 1.0);
        assert_eq!(to, 0.0);
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
