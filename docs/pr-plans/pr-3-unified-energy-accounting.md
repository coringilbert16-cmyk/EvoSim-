# Unify Energy Accounting Across Sources, Consumers, and Initialization

> **Implementation in progress.** The energy-path audit and accounting contract are recorded below. Initial-source and unowned-release safeguards have been implemented; focused verification is in progress.

## Summary

Establish one consistent energy accounting model across initialization, sources, reactions, consumers, settlement, and dissipation. Make changes reconcilable against an independent inventory.

## Outline

- [x] Audit energy entry points, spending paths, transfers, and sinks against merged #207.
- [x] Define accounting contracts and units at each boundary.
- [ ] Close duplicate or inconsistent energy ownership and settlement paths (initial-source shortcut fix started).
- [ ] Test rejected settlement, initialization, and cross-system reconciliation.

## Acceptance criteria

- [ ] Every energy change has an identified source, transfer, storage change, or sink.
- [ ] Failed operations do not partially debit or credit energy.
- [ ] Independent world-energy accounting reconciles within defined tolerances.
- [ ] Legacy source-budget shortcuts cannot silently create energy; the initial constructor-budget-to-energy conversion is removed and covered by an acceptance assertion.

## Guardrails

- Start with one focused audit of the current merged `main` before implementation.
- Use current merged `main` at #207 merge commit `4645acaebd34055f6ab1570bc51e0c12953bf3d9` as the forward verification baseline. Retain PR #205 merge commit `10521705d8f921ceb071b17240f1b7e069f52f1a` as historical context; compare named test failure identities, not only totals.
- Keep each command and test run within five minutes; a timeout is incomplete evidence.
- Keep this PR limited to its primary invariant. Do not fold in unrelated cleanup.
- PR #204 is the roadmap/handoff; PR #196 constructor/Bob work is out of scope.


## Cross-PR boundary and dependency clarification (2026-10-26)

This phase owns the **single energy-accounting contract and independent world-energy oracle**. Reconcile initialization, external source input, storage, work, heat, bond energy, and reaction settlement. Do not implement reaction candidate discovery or environmental scheduling. Reaction tests must use the independent inventory, not only production-ledger counters.

**Dependency:** material identity/ownership in PR #207. The shared chemistry runtime and transactional reaction realization depend on this accounting boundary. This PR does not prove that the environment already executes reactions.

## Audit findings against the #207 merge (2026-10-26)

Audited the actual #207 merge commit `4645acaebd34055f6ab1570bc51e0c12953bf3d9`, not this planning branch's older base snapshot.

- The live world-tick sunlight path is `WorldToneOccluderSnapshot::settle_living_organism_inputs`; it accounts for incident source energy across captured resonance, heat, and escaped propagated energy. The older per-receiver `WorldToneCaptureResult` settlement path in `world_tone.rs` has no production caller and must not be mistaken for a second active source path.
- Current usable holdings are independently summed from organism balances and decomposition-body budgets. Aggregate-owned reproductive construction is not a second energy holder; stale detached/duplicate construction state fails the inventory closed.
- Resonant holdings are summed from organism reservoirs. Structural holdings are summed from realized bond energies in organism and decomposition structures.
- `Simulation::energy_inventory_snapshot` is test-only. It derives current holdings from state and combines them with cumulative source/sink flows from the ledger; those cumulative counters are not a substitute for the independent current-state sums.
- `EnergyTransaction` uses signed deltas and validates a balance equation, but its generic `potential_released` field does not identify a source owner. Production transaction call sites audited so far use zero for this field; keep non-structural energy creation/release explicitly source-backed rather than relying on this generic allowance.
- The live initial organism previously received `(1.0e12 - construction.energy).max(0.0) + genome.reproductive_energy_allocation`. That conversion is now removed: initial usable energy comes only from the genome's declared starting allocation. The constructor's computation allowance is not a physical source; the initial realized structure remains part of the tick-zero structural baseline.
- `EnergySourceBudget` provides atomic shared-budget settlement but is not used by the active sunlight snapshot. That is not itself a defect: the sun is modeled as intensity per unit of unshadowed projected width, not a single finite global pool. The active path must continue to prove unique exposure/propagation accounting and source allocation exactly once.

## Accounting contract and units

- **Unit:** all values called energy are in the simulation's common energy unit. The unit is an accounting quantity, not a harmonic amplitude; spectrum amplitude must be converted to energy before entering the ledger.
- **Current stores:** usable energy, retained resonant energy, and energy held in realized bonds are inventory terms. Each physical store is counted exactly once across authoritative owners.
- **External sources:** initial-condition energy is a one-time explicit source transaction; world-tone input is counted once at the source boundary. Internal transfers between holders do not add source energy.
- **Internal transaction balance:** `source_energy = usable_delta + structural_delta + work_expended + heat_dissipated`, with signed usable/structural deltas. Releasing an existing bond is a negative structural delta, not a second non-structural source. Numerical tolerance only handles floating-point comparison; it is never modeled as dissipation.
- **World balance:** change in usable, resonant, and structural stores plus cumulative work, heat, and escaped tone must equal cumulative external input plus explicitly approved non-structural sources. A positive residual indicating creation is never accepted, even if it falls inside ordinary comparison tolerance.
- **Failure behavior:** a rejected operation changes no authoritative holder, source allowance, ledger flow, bond store, or physical structure. Related physical and energy changes publish together or not at all.
- **Initialization:** the constructor's `1.0e12` allowance may be used only for construction calculations; it must not be converted into organism energy. The declared genome starting allocation is the explicit initial usable-energy grant.

