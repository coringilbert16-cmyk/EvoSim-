# EvoSim Integration Audit & Large-File Truncation Plan

## Current status

**Phase 1 — executable runtime split: COMPLETE.**

**Phase 2 — environment split: COMPLETE.**

**Phase 3 — resources/chemistry split: COMPLETE.**

**Phase 4 — structural/energy/decision integration: IN PROGRESS.**

The current Phase 4 implementation has completed the BREAK energy model, atomic execution, ledger accounting, structural consequences, stable `BondId` identity, deterministic BREAK boundary coverage, and COMBINE → BREAK integration. The remaining work is primarily decision/memory verification, maintenance/stress integration, obsolete-path cleanup, and the final repository-wide integration gate.

## Objective

Make the repository internally coherent before further feature work, while restructuring large source files so future GitHub/AI file retrieval cannot truncate the architectural context needed for safe edits.

This is an integration-first audit. No simulation rule is changed merely to make the code easier to split.

## Locked rules that refactoring and integration must preserve

- Energy is emergent from resource interactions, not a fundamental environmental resource.
- COMBINE interaction value: potential energy establishes energetic direction; reactivity and geometry modify magnitude.
- Surplus investment maps to bond strength through a capped diminishing-returns curve.
- COMBINE consumes energy; formation deficits may require organism payment.
- BREAK depends on current structural/chemical state and may consume or release energy.
- Natural chemistry: compatible resources may combine; compatibility is not a hardcoded finite recipe list.
- Vents are indiscriminate transfer mechanisms; they do not prefer bonded over unbonded material.
- Layer 1 is the spatial deep reservoir; Layer 2 is the active field where organisms live and material/waste interact.
- Waste remains material in Layer 2 and is not automatically deleted or teleported to the reservoir.
- Physical structure is discrete structural units plus explicit bonds; bulk bonded material must not substitute for real structure.
- Bond energy is structural state and must not be reconstructed from raw resource potential energy during BREAK.
- Bond identity is stable and independent of vector position or mutable physical state.
- Decision history is bounded learned consequence history, not a physics cache and not fabricated prediction.
- Mechanical eligibility belongs to physical systems; decision logic chooses among mechanically eligible candidates.
- Maintenance is paid independently of action choice and is based on actual structural mass, not stored material.
- Maintenance cost is `M_t = M_b × M_structural`.
- Usable energy after maintenance is `E_{t+1} = max(0, E_t - M_t)`.
- Unpaid maintenance accumulates unbounded stress according to `S_{t+1} = S_t + k_d × max(0, M_t - E_t) / M_t` when `M_t > 0`.
- Successful maintenance payment reduces stress by `k_r`, clamped at zero.
- Approved default stress parameters are `k_d = 0.10`, `k_r = 0.02`, and `S_break = 1.0`.
- There is no stress cap. `S_break` is the structural-damage threshold, not a maximum.
- When stress reaches/exceeds `S_break`, accumulated stress causes structural damage by selecting a random existing structural bond for BREAK.
- Stress-induced structural damage uses the authoritative BREAK path and its normal bond-energy/work/heat accounting; it does not create a second fundamental energy resource.
- Structural damage does not automatically reset stress to zero. Stress remains above the threshold if sufficient accumulated stress remains, allowing continued damage under persistent maintenance deficit.

## Phase 4 — Structural, energy, decision, memory, and maintenance integration

### Completed

1. Audited the existing BREAK path and removed dependence on fixed energy rewards.
2. Established current-state BREAK work and the release/consume/neutral energy regimes.
3. Made BREAK accounting atomic: physical mutation occurs only after all energy/validity checks succeed.
4. Kept the energy ledger authoritative for cumulative transformation accounting.
5. Preserved structural units and unrelated bonds when one bond is broken; connected components remain derived from the bond graph.
6. Added stable `BondId` identity, monotonic allocation, serialization compatibility, legacy zero-ID migration, and ID-based BREAK resolution.
7. Updated decision candidates and active transformations to identify BREAK targets by stable `BondId`, not vector index.
8. Added deterministic BREAK boundary tests for release, consume, neutral, zero-work, epsilon, large finite, and non-finite states.
9. Added stale-target/index-shift coverage so an active transformation cannot break the wrong bond.
10. Added COMBINE → stored bond energy → BREAK integration coverage.
11. Verified the Rust formatter, full Rust tests, and Clippy through CI after the current implementation.

### Remaining

#### 4J — Decision integration verification

- Confirm candidate generation remains mechanical eligibility only.
- Confirm stable `BondId` context survives candidate selection and active transformation.
- Confirm selection never mutates physical state.
- Confirm actual outcomes, not predictions, are recorded after execution.
- Add/retain coverage showing consequence history changes future selection without duplicating chemistry or geometry.

#### 4K — Memory verification

- Spatial memory remains bounded, decaying, merged, and pruned.
- Confirm successful consequences can reinforce spatial memory without bypassing physical execution.
- Confirm decision history remains bounded and contains actual action consequences rather than predicted outcomes.
- Do not introduce arbitrary decay or learning mechanics unless the existing architecture requires them.

#### 4L — Maintenance/stress integration

- Implement maintenance as an independent per-tick physiological cost based on actual structural mass.
- Charge maintenance regardless of which action is selected or whether an action succeeds.
- Consume only currently available usable energy; maintenance does not create material or energy.
- Accumulate unbounded stress when maintenance cannot be fully paid.
- Recover stress only through successful maintenance payment at the approved recovery rate.
- At `S_break = 1.0`, route stress-induced structural damage through the authoritative BREAK mechanism and select an existing bond randomly.
- Preserve normal BREAK energy accounting and structural topology rules for stress-induced damage.
- Verify persistent maintenance deficit can produce repeated structural damage rather than resetting stress after one break.
- Add black-box runtime tests proving maintenance is independent of action consequences and that stress can trigger structural BREAK.

#### 4M — Obsolete-path cleanup

Search and classify stale paths as KEEP, UPDATE, DELETE, or MIGRATE. In particular inspect:

- old energy-resource fields or fixed BREAK rewards;
- bond-index identity and copied-live-bond transformation state;
- duplicate structural-combine implementations;
- legacy decision gates that bypass current candidate selection;
- stale phase markers and status documents;
- dead modules or declarations;
- comments describing superseded energy/structure rules;
- obsolete or conflicting stress constants/decay implementations.

Temporary Phase 6 marker files have already been removed. Historical specification documents are retained until their status/use is explicitly classified.

#### 4N — Final integration gate

Run the complete verification surface:

- `cargo fmt --all -- --check`;
- full Rust tests;
- Clippy;
- serialization/snapshot compatibility;
- material conservation;
- COMBINE/BREAK energy conservation;
- structural topology and connection-site invariants;
- decision integration and bounded history;
- spatial memory behavior;
- multi-tick transformation locking;
- stale transformation handling;
- deterministic seeded behavior;
- maintenance/stress accounting;
- snapshot/WebSocket integration;
- frontend/build checks;
- repository-wide obsolete-path search;
- large-file/truncation audit;
- documentation/status consistency.

## Large-file rule going forward

No new core simulation file should be allowed to grow into another monolith. Split when a file contains multiple independently testable responsibilities, especially when it approaches ~20–25 KB or when retrieval of the full file is no longer reliable.
