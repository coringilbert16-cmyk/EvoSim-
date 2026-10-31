# PR 4 — Resonance energy conservation and environmental integration

**Goal:** Integrate the aura-derived pair interaction from PR 3 into material movement and chemistry, with complete energy accounting. Reuse the existing resonance aura and its established spatial attenuation/geometric reception; do not add a second field or propagation mechanism.

## Existing accounting audit (against the current PR 3 dependency stack)

The current code already has several distinct energy channels; PR 4 must extend them rather than collapse them:

- `EnergyLedger` and `EnergyInventorySnapshot` already track usable energy, retained resonant energy, signed structural bond potential, external input, escaped world-tone energy, work expended, and heat. Structural energy is recomputed from authoritative bond state and counted once.
- `ResonantEnergyReservoir` is a frequency-tagged store of retained energy used by organism and environment holders. Its `withdraw_compatible` operation weights withdrawal by logarithmic frequency compatibility. It stores energy, not phase, and is not the pair-interaction potential.
- World-tone reception already stages reservoir changes and ledger settlement atomically. The source frame is accounted as captured resonance, heat, and escaped signal; nonliving materials dissipate rather than retain the energy.
- Movement currently charges its fixed locomotion cost entirely to heat (`work_expended = 0`) after a staged ledger preflight. It has no pair-force work term yet.
- COMBINE's `formation_work_cost` integrates the legacy nonnegative chemical attraction over the remaining approach distance. It is separate from `intrinsic_bond_potential`, which becomes stored structural bond energy. PR 4 must replace the old approach-work authority with the new potential difference without charging both.
- BREAK already distinguishes bond structural-potential change from physical work and heat. Do not rewrite this into a generic reservoir transfer that erases the source attribution.

**Missing inventory category:** the world inventory has no signed current/baseline term for resonance pair-interaction potential. PR 4 must add one authoritative world-level held-potential term (or prove, with an explicit field-energy inventory, that it is already included elsewhere). It must be signed because the PR 3 potential can be attractive or repulsive. It must not be written into `ResonantEnergyReservoir`, `total_resonant_energy_held`, or `total_structural_energy_held`.

**Approved conservative exchange rule:** for a geometry change, calculate the signed pair-potential change `ΔU_pair` before committing movement. If `ΔU_pair > 0`, the active mover pays the increase from its usable energy. If `ΔU_pair < 0`, the released potential is returned to compatible biological resonance storage when a living holder can harness it; any amount that cannot be retained is dissipated as heat exactly once. Do not withdraw the positive increase from both participants' resonance reservoirs. Insufficient usable energy rejects the movement/reaction and rolls back all staged state. The pair-potential change is itself the stored-energy delta: do not also count the same transfer as a second energy sink in `total_work_expended`. Keep that cumulative field for genuinely expended work that leaves the modeled held-energy inventory, not for energy already represented by `ΔU_pair`.

The current shared phase clock should also be tested for stationary relative phase: same-frequency components driven by the same clock have a common phase advance, while material-specific oscillator lags remain fixed. Under that current model, a stationary pair should not gain or lose pair potential merely because the global phase clock advances. If future independent sources or phase modulation break that invariance, their energy exchange must be explicitly sourced rather than silently credited.

**Atomicity constraint:** a movement/reaction candidate must stage the geometry change, recomputed pair-potential delta, compatible resonance withdrawal (when that is the approved work source), usable-energy/heat changes, and ledger update together. Commit none of them if any precondition or inventory check fails.

## Implement

- Consume PR 3's single symmetric pair potential, its signed frequency-resolved components, and the equal-and-opposite force derived from that potential. The active mover's usable energy funds positive `ΔU_pair`; frequency-resolved components guide compatibility when returning negative `ΔU_pair` to biological resonance storage. Do not collapse the release into an unqualified scalar-frequency deposit.
- Keep interaction potential distinct from retained resonance energy in `ResonantEnergyReservoir`. The former is the conservative energy associated with the pair's configuration; the latter is energy stored by a material across ticks. Do not count either quantity as the other.
- Account for work done by attractive and repulsive interactions as materials move.
- Require real energy input for movement against repulsion; do not permit negative-cost or energy-free displacement.
- Settle changes in pair interaction potential, movement work, usable energy, structural energy, retained resonance, and heat atomically.
- Integrate with environmental materials, live chemistry, COMBINE, and BREAK using the same authoritative aura-derived interaction.
- Remove obsolete parallel chemical-attraction movement/work mechanisms only after the replacement is verified. Preserve reaction eligibility and intrinsic bond potential.

## Acceptance

- Independent energy inventory balances through attraction, repulsion, approach, separation, COMBINE, and BREAK.
- No double-counting between aura signal energy, retained resonance, pair interaction potential, usable energy, structural energy, and heat.
- Failed movement/reaction transactions roll back completely.
- Movement against repulsion requires sufficient available energy and settles the actual work.
- Existing COMBINE and BREAK behavior remains valid except where the approved physical interaction model intentionally changes approach/force outcomes.
- Environmental and organism-material interactions use the same aura-derived mechanism and realized geometry rules.
- No duplicate application of a pair force and no hidden energy source/sink.

**Dependency:** PR 3. Draft only until PR 3 is accepted.


## Implementation audit against the current code

The following constraints are confirmed by the current implementation and must shape the first coding pass:

- `EnergyLedger` tracks cumulative work/heat and aggregate usable, resonant, and structural stores. `EnergyInventorySnapshot` is a test-only independent oracle. Neither currently has a signed pair-interaction-potential inventory term.
- Current retained resonance is owned by organism reservoirs plus the environment reservoir. `ResonantEnergyReservoir` stores nonnegative energy by frequency, not phase; its compatible withdrawal/deposit operations are holder-local. A pair cross-term is symmetric and signed, so PR 4 must not silently assign all of its cost or release to whichever material happened to be called the receiver.
- Movement currently charges its per-step cost entirely as heat (`work_expended = 0`) and has no pair-force work or kinetic-energy settlement. Adding a force without a corresponding energy transaction would create or destroy energy.
- Live COMBINE still obtains approach work from `formation_work_cost`, the integral of the legacy nonnegative chemical attraction. Its intrinsic bond potential is a separate stored structural term. Environmental COMBINE also still reads/calculates the legacy static chemical potential. These are distinct consumers and must not be migrated in one unverified replacement.
- PR 3's pair potential is available as signed frequency-resolved components. Its no-occluder force helper is valid only for an isolated pair. The occluder-aware potential can account for a blocker, but the current two-body force helper deliberately returns no force in that case because the blocker’s reaction force is not yet derived.

### Required decisions before force consumers are activated

1. **Pair-potential ownership and baseline:** define how the world records the signed current pair-potential inventory and its tick-zero baseline, including pairs that appear or disappear as physical materials enter, leave, or change geometry. The independent inventory oracle must recompute or otherwise independently verify the held term.
2. **Energy source for ΔU (approved):** a positive pair-potential change is paid by the active mover from usable energy. A negative change is returned to compatible biological resonance storage when it can be harnessed; any unretained remainder becomes heat exactly once. Do not automatically withdraw from both participants' reservoirs.
3. **Mechanical work boundary:** distinguish conservative pair-potential change from genuinely dissipated movement work. If ΔU is included as a held inventory term, do not also count that same energy as `work_expended`. Movement against a repulsive potential must debit the active mover's usable energy; no implicit free displacement or reservoir withdrawal is permitted. The pair potential is a signed cross-energy term of two emitted fields, not energy already stored in either participant's reservoir.
4. **Occluded systems:** do not apply the isolated-pair force helper when other physical bodies can occlude the pair. Either derive the full N-body potential gradient, including reaction forces on blockers, or keep those force consumers inactive.

These are implementation gates, not reasons to add arbitrary efficiencies, polarity assignments, or a second energy field.


## First implementation slice — signed settlement foundation

The current branch now contains the first isolated accounting implementation:

- EnergyLedger has a signed total_pair_interaction_potential_held counter and a tick-zero baseline. The value is the net conservative pair-potential change relative to the tick-zero reference; it is not stored in either resonance or structural bond inventory.
- EnergyInventorySnapshot includes the signed pair-potential change in its conservation residual.
- settle_pair_potential_change stages the entire settlement before mutation. A positive delta is paid only from the active mover's usable energy. A negative delta is allocated across its negative frequency-resolved components, retained across eligible biological resonance reservoirs when their bounded spectra accept them. A full holder is skipped in favor of the next compatible holder; only the unretained remainder becomes heat. The same delta is never counted as work_expended.
- Focused tests cover positive settlement, insufficient-energy rollback, multi-frequency release allocation, no-holder heat fallback, full-holder fallback to a second compatible biological reservoir, invalid-component rollback, and inventory balance for each sign.

**Still outstanding before this can be considered a complete world inventory:** the current snapshot currently reads the signed pair-potential counter from the ledger; it does not independently recompute that value from the current physical world. The independent oracle must enumerate the same authoritative physical bodies as the world-tone snapshot (organism aggregates, environmental physical materials, decomposing bodies, physically stored/committed material, and in-flight transformation escrow) and calculate the signed pair potential with the complete occlusion set. The existing PR 3 pair API currently accepts PhysicalMaterial, while the world snapshot represents some bodies as aggregate realized geometry plus weighted properties. A safe adapter/generalized geometry API and a benchmark are required before using a naive all-pairs world calculation. Do not activate movement or chemistry consumers until that oracle agrees with the ledger and its cost is acceptable.

The pair-potential baseline is currently initialized to zero as a tick-zero reference for changes; it is not a claim that the absolute static pair potential of the initial world is zero. Future world-oracle work must compute the current world potential and its tick-zero reference consistently, or maintain the same reference through explicit, conserved pair-set creation/removal transactions.

## Independent world-potential oracle

The current branch now includes an explicit world-state recomputation path. It reconstructs the authoritative realized physical bodies represented by the world-tone snapshot: organism aggregates, committed reproductive material, environmental field material, decomposing structures, owner-relative stored material, and physical sources held by active transformations. It then sums every unordered resonant-participant pair while passing the complete physical-body set as the occlusion set.

Stored material and in-flight stored BREAK sources remain physical blockers, but are excluded as resonance sources and receivers until released. Committed reproductive material, organism aggregates, environmental material, decomposing structures, and in-flight environmental COMBINE sources participate in the pair-potential sum.

Acceptance tests verify that the oracle enumerates the same physical body count as the world-tone snapshot, that stored material blocks without becoming a pair participant, and that a geometry change's independently recomputed frequency-resolved ΔU settles into the ledger without creating energy.

**Performance boundary:** this explicit oracle is not called from the simulation tick. The current implementation recomputes physical geometry for each pair and every occluder, so it is intentionally a correctness oracle for small/controlled worlds, not an acceptable tick-loop implementation. Geometry precomputation/caching and a measured world-scale benchmark are still required before any movement or chemistry consumer can use it.

The energy snapshot continues to read the authoritative signed pair-potential ledger counter. The independent world oracle is used to verify the counter's expected ΔU in acceptance tests; it is not silently substituted as an expensive per-snapshot recalculation.