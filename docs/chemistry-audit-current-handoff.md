# Chemistry Unification — Current Handoff

**Reconciled:** 2026-10-22  
**Plan head:** `f784f13e31f60f619befd9e82827af0b41e78fe8`  
**PR #205 head audited:** `12d8079230458da68a8dc3755a1d6a9b41347862`  
**Exact-head workflow:** [39833438679](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39833438679).

PR #204 is documentation-only. Detailed scientific decisions, audit register, evidence, and dated findings are in `docs/chemistry-audit-unification-plan.md`, Sections 14–17. No implementation source/tests changed in the current audit series.

## Governing decisions and baseline

- Frozen `main`: `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5`; workflow [39705628427](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39705628427): 563 passed, 28 failed, 1 ignored. Compare named failure identities, not counts alone.
- Resource ID 6 is Oxygen. PR #196 constructor/Bob remains separate. One focused audit before each code change; hard five-minute command/test limit.
- Owner-approved aggregate model (Section 17.31): parent owns primary structure, storage, committed material, developing structure/bonds, and associated energy until division. Core breach ends biological processing but does not transfer ownership or create a separate decomposition object. Dead organisms remain physical and continue ordinary chemistry. Ownership alone does not guarantee chemical or sunlight-receiver eligibility.
- Stored `PhysicalMaterialBond` has topology/endpoints but no allocated numeric bond energy. Do not invent a stored-energy credit from resource potential.

## Verification status

Workflow 39833438679 on PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`: formatting/focused gates passed, including chemistry-library partial-append rollback/reopen. Full suite: 585 passed, 27 failed, 1 ignored in 283.74 seconds. All 27 failing identities match frozen baseline identities minus one still-resolved containment-to-storage failure. Strict Clippy has 34 application-target and 15 test-target diagnostics; normalized identities match baseline. Suite remains baseline-red. No representative performance benchmark has been run; whole-world conservation is not proven.

## Confirmed findings

1. **Reaction reachability:** ordinary chemistry iterates bonds in the primary graph only. Stored/committed material and developing-graph bonds are not enumerated. Their topology/energy representation differs, so do not broaden reaction scope blindly.
2. **Stored BREAK:** output pieces are committed one at a time; rejected storage falls back to a field deposit whose result is ignored. Malformed completion can clear the active marker without restoring escrow. Fallback coordinates are not proven world-relative.
3. **Contained acquisition and expulsion:** acquisition removes field sources before storage and ignores failed fallback deposits. Expulsion removes parent storage and does not restore the original if the final environment deposit rejects.
4. **Movement:** position/occupancy/environment mutate before ledger settlement. If settlement rejects, geometry stays moved without charging energy and movement progress is not decremented. Movement cost is currently all heat and zero explicit work; reconcile with D7 before changing the equation.
5. **Better transaction patterns:** COMBINE bond formation validates trial geometry, settles energy, then commits. Maintenance changes stress only after settlement. Chemical/resonance BREAK stage reservoir withdrawal and graph mutation, then commit after ledger acceptance.
6. **World tone:** frame paths validate incident energy = retained resonance + heat + escaped energy and stage reservoir commit until ledger settlement succeeds. Exact-small-world geometry/error and scale benchmarks remain open. Tick order means a core breached by ordinary chemistry can still receive the earlier world-tone input that tick; test before changing ordering.
7. **Receiver clarification needed:** D5 does not settle whether uniquely parent-owned stored/committed/developing surfaces may credit the parent's resonance reservoir. Section 17.38 records aggregate-wide versus primary-structure-only capture. Do not change mapping until owner decision.
8. **Inventory:** integer material oracle covers field, organism structure/storage, committed/developing construction, active transformation escrow, and legacy carriers. Energy snapshot recomputes usable/resonant/graph-bond stores independently of cached held totals, but ledger history still supplies cumulative source/sink flows.
9. **Cache and activation:** schema 4/fingerprint and strict manifest/record checks are present, with append/manifest rollback tests. Static potential depends on chemical-position values plus K/Dmax; ChemistryKey has names/interface, not numeric positions, so catalog invalidation relies on a manually updated fingerprint marker. Test that a chemical-position change requires a fingerprint bump and rejects old cache. Also, reaction accumulation returns before stale-key cleanup when an organism has no bonds, allowing obsolete activation entries to persist.
10. **Ledger diagnostics:** prepared COMBINE computes deltas for escaped-tone and resonant-held fields but does not apply/rollback them; the current candidate should leave them zero. Assert that contract or centralize field-wise delta application. `total_world_tone_energy_unabsorbed` is documented as cumulative throughput but is not updated by live frame settlement; resolve telemetry semantics without adding it as a separate conservation term.

## Phase status and next work

- Phases 0–1 recorded. Phase 2 oracles exist but cross-domain failure coverage is incomplete. Phase 3 holder-return atomicity is open. Phase 4 schema/fingerprint/rollback guards exist but catalog invalidation/restart coverage remains open. Phases 5–9 are partial and not release-ready; Phases 10–11 remain open.
- Highest-priority candidates after audit closure: (a) stored BREAK all-or-nothing ownership; (b) checked acquisition/expulsion returns; (c) movement geometry/ledger atomicity; (d) bondless reaction-accumulation cleanup; (e) explicit catalog-fingerprint policy and prepared-ledger field invariants.
- Finish the source/holder impact matrix and transaction-failure matrix. Re-fetch PR #205 head and current CI before any code change. Do not claim chemistry unified until Section 17.6 release gates have evidence.

## Separate future reproduction requirement

Internal/adjacent budding begins a second genome core at 100% of original size, grows to 160% total, then splits into two roughly 80%-size children. Current reproduction code does not satisfy this; it remains separate from chemistry PR #205 unless scope is explicitly changed.
