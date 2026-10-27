# Build a Shared Chemistry Runtime and Reaction Scheduler

> **Implementation in progress.** This document is the acceptance contract for PR #214; implementation and tests are being added incrementally in this same PR.

## Purpose

Make environmental materials and material held by cells/organisms participate in the **same chemistry rules and settlement pathway**. The current organism-bond reaction accumulator is not, by itself, a world chemistry engine. This phase closes that gap without inventing a second environmental chemistry system.

## Scope

- Audit the live tick loop and map every source/holder of environmental, stored, and physically realized material to the same reaction-policy entry point.
- Define eligible local contacts/neighborhoods and candidate discovery without a world-wide all-pairs scan. Use spatial/local indexes already authoritative in the simulation where possible.
- Separate candidate detection, multi-tick activation/progress, successful physical realization, and committed accounting; candidate/progress state is not stored energy.
- Route all committed reactions through the shared transaction boundary for live geometry/material validation, material ownership changes, bond changes, energy settlement, and rollback.
- Specify deterministic tick ordering and bounded per-tick work so large populations do not trigger unbounded event histories or repeated global geometry searches.
- Make ordinary chemistry available to environmental stock, cell/organism-held material, and dead material; lifecycle may change biological behavior, not the underlying chemistry law.

## Acceptance criteria

- At least one representative environmental-material reaction can be discovered, activated, physically realized, and accounted for without an organism being present.
- A corresponding reaction involving organism/cell-held material uses the same rule and settlement contract rather than a parallel special case.
- A rejected/stale candidate leaves material, geometry, bonds, energy, and inventories unchanged.
- Successful reactions reconcile integer material counts and independent world-energy inventory; activation accumulation creates no energy.
- Tick ordering is deterministic; tests cover repeatability and stale candidates.
- Candidate discovery is local/indexed rather than an O(N²) scan over all material pairs. Benchmark workloads and limits are documented.
- The scheduler exposes enough structured events for tests/diagnostics without requiring unbounded retained history.

## Dependencies and boundaries

- Depends on PR #207 material identity/ownership, PR #211 cache-version boundary, and PR #208 unified energy accounting.
- The shared runtime owns candidate scheduling and activation-progress lifecycle. PR #209 owns the atomic realization/settlement of a selected reaction; coordinate APIs rather than duplicate them.
- PR #210 owns death-state transition, not a second chemistry engine.
- PR #215 owns world event inspection and environment-first experiments, not reaction semantics.
- Keep sunlight approximation separate in its own validation phase. PR #196 constructor/Bob work remains out of scope.

## Guardrails

- Audit current main immediately before implementation; compare named test failures against the latest accepted baseline, not only aggregate counts.
- Keep every command/test under five minutes. A timeout is incomplete evidence.
- No hard-coded organism roles, body plans, or fixed reaction recipes as a substitute for shared rules.
- No silent conservation repair, all-pairs world scan, or duplicate environmental material owner.

## Implementation progress (2026-10-27)

- [x] Audited current `main`: organism-bond activation is implemented, but the scheduler currently resolves activated operations immediately inside the per-organism loop. Environmental material and decomposing bodies do not yet share that path.
- [x] Added a deterministic, shared per-tick cap for physical reaction-realization attempts. Activated operations deferred by the cap retain progress only and are revalidated on a later tick; this cap does not yet bound candidate discovery.
- [x] Added unit coverage for stable-key scheduling and budget limits.
- [x] Added a global 256-per-tick structural-candidate inspection cap, fairly shared across organism owners with unused budget carried forward.
- [x] The bounded scan reads only selected bonds by index; it no longer clones the full bond list before applying the cap.
- [x] Rotated each owner's bounded bond scan by tick and added tests for empty, zero-budget, truncated, and wraparound windows; this bounds discovery separately from the 64-per-tick physical realization cap.
- [x] Named the accumulated comparison value an activation signal and documented that it is not an energy source or inventory term; physical source provenance remains a separate invariant to verify.
- [x] Added a regression proving a large activation signal cannot pay a physical BREAK energy shortfall; focused chemistry tests passed, while exact-head full-suite/Clippy verification remains pending.
- [x] Separated activation from both usable-energy and compatible-resonance funding; added a regression proving available resonance cannot substitute for the independent activation threshold. This new regression was committed after the last completed workflow, so it still needs exact-head CI.
- [x] Identified a hard integration gap: environmental `PhysicalMaterialBond` records store topology/endpoints but no bond-energy holder, while decomposing bodies use a separate bond-break settlement path. Environment-only realization must not invent an energy source; the shared transaction/ownership bridge needs to preserve or derive actual energy provenance.
- [x] Refactored chemical BREAK energy accounting into `prepare_chemical_break_energy_with_resonance`, which stages the holder/ledger transaction without mutating either. The existing organism-graph path now stages its candidate graph and energy settlement before committing them together. Added focused tests for zero-stored-energy bond work and rejection of unfunded work; exact-head CI is pending.
- [x] Reuse the staged energy helper for environmental physical-material BREAK, committing the source split, environment-owned usable-energy holder, and ledger atomically. Physical-material internal bonds have zero stored energy on restoration; no bond-energy release is credited.
- [x] Add environmental material to the **chemical** candidate/activation runtime using bounded field discovery and the shared activation/scheduling policy. The existing `environmental_break_candidates`/`try_start_environmental_break` remains a separate voluntary physical BREAK transformation.
- [ ] Add acceptance coverage for dead-material behavior. Audit confirms nonliving organisms remain in `Simulation::step`'s organism loop and pass through the same `chemical_reaction::accumulate` path; do not reactivate the disconnected legacy recycling/decomposition path as a second chemistry engine.
- [x] Add transactional and independent-inventory tests for explicitly funded environment-only BREAK and organism-held reactions. Natural environment-energy sourcing remains unverified.
- [ ] Benchmark candidate discovery and document bounded-work behavior.
- [x] Reconciled latest `main` (`29f0f510062eb1ec4a1db3e1f7bb3cce12ba2d08`) into this branch; the branch is no longer behind main.
- [x] Exact integration-head workflow (run `40512804887`, before the newest activation regression and obsolete-helper cleanup): formatting, architecture checks, and all five focused test groups passed. Full suite: 766 passed, 10 failed, 1 ignored in 375.08s. The ten failures are `bond_driven_contract_tests::tests::construction_uses_an_intact_composite_physical_material`, `bond_driven_contract_tests::tests::exact_construction_bond_forms_the_supplied_contact`, `cavity::tests::unbonded_structure_is_not_a_genome`, `combine::tests::continuous_candidates_are_supported_by_combine`, `contact::tests::boundary_endpoints_preserve_physical_surface_normals`, `movement::tests::fluid_boundary_is_contact_geometry_without_becoming_a_wall`, `movement::tests::movement_distance_history_prefers_non_dominated_consequence`, `movement::tests::water_boundary_permits_full_environmental_penetration`, `organism_viability::tests::unsealed_genome_fails_viability`, and `simulation_tests::integration_tests::storage_contains_discrete_independent_material_objects`. The plan's accepted baseline records 21 distinct failures; verify against an exact-head run before calling this baseline-only. Clippy failed on existing repository-wide `-D warnings` debt, including unused staged APIs and constructor lint findings; it also identified an unused `settle_break_transaction` helper introduced during this refactor, which has since been removed. The current head still needs CI.
- [x] Exact implementation-head workflow after the activation regression and unused-helper cleanup (run `40514969075`, head `ba523e18e77ec135172c4a34988494bf80cee47f`): formatting, architecture checks, and all five focused test groups passed, including the new resonance/activation independence regression. Full suite: 767 passed, 10 failed, 1 ignored in 210.15s. The same ten failures listed above recur unchanged. Clippy still fails on repository-wide pre-existing `-D warnings` diagnostics; the refactor's unused `settle_break_transaction` diagnostic is gone. Environmental chemical discovery and settlement remain unimplemented, so this is verification of the current accounting slice only.


## Correct implementation order after main integration (2026-10-27)

1. Refactor chemical BREAK settlement into a staged energy transaction that can be committed atomically with either a graph-bond mutation or an environmental physical-material split. For `PhysicalMaterialBond`, the current source-backed value is zero stored bond energy: restoration derives bond strength but explicitly sets `bond_energy: 0.0`. Therefore an environmental chemical BREAK may release no bond energy and must charge required work to the organism's usable-energy/resonance sources; do not infer stored energy from `ResourceProperties::potential_energy` or activation signal. Only add nonzero stored bond energy if a real formation/allocation path and inventory ownership are implemented together.
2. Extend local/indexed discovery and activation to eligible environmental physical material, then route realization through the existing transactional boundary rather than duplicating it.
3. Keep dead organisms on the canonical reaction path; add an acceptance regression rather than enabling the legacy recycling carrier.
4. Add environment-only and cross-holder acceptance tests with independent integer-material and world-energy inventory oracles.
5. Benchmark the completed path and document current temporary safeguards. The 307.58-second full-suite run slightly exceeded the five-minute command target, so recheck runtime after implementation; do not weaken test coverage to meet it.


### Implementation update (2026-10-27)

The energy calculation is staged in prepare_chemical_break_energy_with_resonance, but the committing settle_chemical_break_energy_with_resonance still owns the organism-graph mutation. It has not yet been refactored into a mutation-agnostic commit primitive. Existing focused tests cover the energy preparation rules for zero-energy environmental bonds and rejection when the holder cannot fund the work; they do not prove live environmental chemical discovery/settlement.

### Scope clarification from live code audit (2026-10-27)

Environmental material already has a physical BREAK action path (`environmental_break_candidates` and `try_start_environmental_break`) with escrow/rollback and a zero-energy settlement for pre-existing internal connections. That is **not** the missing shared chemical runtime: it does not accumulate chemical activation for environmental interfaces. Keep it intact; do not mistake it for chemical environmental reaction support or add a parallel chemical law. The remaining gate is to connect eligible environmental interfaces to the canonical chemistry scheduler only after their energy/ownership semantics are defined. The latest workflow (40514969075) completed on exact head ba523e18e77ec135172c4a34988494bf80cee47f: formatting, architecture checks, and all five focused groups passed; full suite 767 passed, 10 failed, 1 ignored; Clippy failed. The 10 failures match the previous run's failure set. No claim that those are baseline-only is made.


### Refined environmental reaction boundary (2026-10-27)

The runtime now has an EnvironmentalChemicalBreakOperation, bounded environmental bond discovery, accumulated activation progress, deterministic realization scheduling, stale source/bond revalidation, and a resolver that stages zero-bond-energy work accounting before atomically splitting the exact environmental source. The current resolver pays disruption work only from an explicit environment-owned usable-energy holder; it does not withdraw resonance. That holder starts at zero, and a production path that funds it from a physically modeled source is still missing. Tests fund it through the ledger's explicit external-input transaction solely to verify settlement conservation; this does not yet prove naturally powered environmental chemistry. On success the resolver commits field pieces, the environment holder, and ledger; on failure it preserves material and energy inventories. Focused environment-only tests now cover funded success, insufficient-energy rollback, and independent world-inventory reconciliation. Remaining work is the runtime source-provenance bridge, dead/decomposing-material unification, and bounded-work benchmarking. Do not use try_start_environmental_break as the chemical scheduler: it is an explicit transformation path and has no accumulated activation signal.


### Atomic environmental field split primitive (2026-10-27)

Added ActiveMaterialField::split_physical_at_index as the field-side commit primitive for future environmental chemical BREAK settlement. It validates the exact escrowed source snapshot, every realized split piece, and every destination before mutation; on success it removes the source, deposits all pieces, and invalidates each touched cell once. New tests cover stale-source rejection, late invalid-destination rejection without partial mutation, and successful replacement. The environmental chemical scheduler now discovers environmental interfaces and calls this primitive from its resolver. This is no longer infrastructure-only; end-to-end acceptance tests and dead/decomposing-material coverage remain necessary before PR #214 can be considered complete.


### Environmental progress invalidation fix (2026-10-27)

Moved environmental reaction-key refresh until after barrier, resonance-frequency, and activation-signal validation succeeds. A candidate with invalid chemistry/geometry data can no longer refresh stale accumulation and accidentally suspend its per-tick dissipation. Nonpositive progress still removes its key explicitly. This is a narrow scheduler correctness fix; exact-head CI has not yet been observed for this commit.


### Commit-time operation revalidation (2026-10-27)

Both organism-held and environmental BREAK resolvers now re-check that the operation's activation signal meets its disruption barrier, and reject non-finite/non-positive reaction frequencies before staging resource changes. Environmental settlement also rejects an invalid resonance reservoir before inspecting or mutating the source field. Added environmental resolver regressions for under-activated operations and invalid resonance frequency; each asserts the structure/field and energy ledger/reservoir remain unchanged. Exact-head workflow is queued for commit e4edb068d7c55b0f493deba897c555196a5543b6; results are not yet available.

### Bounded environmental discovery traversal (2026-10-27)

Replaced full environmental bond enumeration and candidate-vector construction with a rotating nested traversal over intersecting cells, material instances, and each instance's internal bonds. Candidate storage and expensive chemistry evaluation now stop at the remaining discovery budget; cell/material/bond starting positions rotate by tick to reduce starvation. If an owner's discovery budget is already exhausted, environmental discovery returns after applying the normal accumulation dissipation rule rather than scanning the field. This bounds candidate materialization and bond inspection by available budget, though iterating the intersecting-cell list and empty material entries remains proportional to local field indexing. Exact-head compile/test results are still required; this does not close the remaining dead/decomposing-material unification gap.

### Death and decomposition boundary (2026-10-27)

Clarified the simulation invariant in code: genome-core breach changes biological eligibility, not physical ownership. The same shared chemistry pass continues to inspect the breached organism's graph; stored material and in-flight transactions remain owned by that organism. The legacy `DecomposingBody` path remains test-only and must not be enabled as an independent bond-yield law.

**Remaining implementation gap:** once shared chemical BREAK has removed all structural graph bonds from a dead organism, its still-owned structural units need a transactional release into the environmental physical-material field. That conversion must preserve each unit's exact physical realization and internal bonds, and must commit material release and any remaining energy settlement atomically. It must defer while active transformations, reproductive construction, stored material, or resonance remain owned, rather than dropping or duplicating those holders. Acceptance coverage should prove (1) dead graphs remain chemistry-eligible, (2) no separate decay path breaks bonds, (3) final release occurs only after graph bonds are gone and holders are safe to transfer, and (4) failed field insertion/energy settlement leaves the organism and inventory unchanged.

### Transactional dead-organism material release (2026-10-27)

Implemented the first shared death-release transition in `Simulation`. A breached organism is retained while its structural graph still has bonds or while an active transformation, reproductive construction, stored material, active movement, invalid resonance inventory, or missing destination position remains. Once its graph is bondless and those holders are clear, each structural unit is realized through the same exact physical-material conversion helper and batch-deposited into the environmental field. Any remaining usable energy is staged as decomposition heat; a rejected energy settlement or field batch leaves the organism and live ledger in place. Successful transfer removes the organism only after the environmental batch succeeds.

Added focused acceptance tests for successful one-time material release, failed destination rollback (including usable energy and ledger), and deferral when stored material remains owned. Exact-head workflow is pending. Follow-up gates still needed: prove an organically chemistry-broken dead graph reaches this release path on a live tick, verify inventory reconciliation across that end-to-end transition, and decide how retained resonance can be safely dissipated or transferred rather than deferred indefinitely. This is a first lifecycle slice, not proof that environmental and dead-material chemistry are fully unified.

A live-tick regression now exercises the lifecycle wiring, not only the helper: a bondless breached graph with no outstanding holders must deposit its material exactly once and retire the dead owner during `Simulation::step`. The exact-head CI result is still pending.

### Follow-up correction: preserve world placement during release (2026-10-27)

The release batch now routes each realized unit to the field cell selected by that unit's own preserved physical placement, instead of assigning every unit to the organism's first occupied cell. The rollback test now uses a finite but out-of-field unit placement to exercise destination rejection while allowing material realization to complete. This matters because a release that preserves geometry inside each material object but deposits every object into one unrelated field cell would still corrupt spatial indexing.

### Exact-head verification correction (2026-10-27)

The first placement-preservation edit left a stale `position` reference after the old organism-position binding was removed. The next CI run failed across test groups at compile time, so those failures were not meaningful behavioral results. Corrected the deposit mapping to zip realized materials with their structural units and use each unit's placement; the source was re-fetched after the update to verify the stale reference is gone. CI for `c8951e1` is now running; do not treat the previous run's focused-test failures as a regression baseline or a pass.

### Follow-up transaction guard and verification repair (2026-10-27)

The final-release eligibility gate now checks the simulation's in-flight transformation queue by owner ID as well as the organism's active-transformation marker. This protects the escrow case where a queued transaction is still authoritative even if the organism marker is missing or stale. Added an acceptance test that a bondless dead organism is retained while such an owner transaction exists.

Verification exposed a compile regression from a broad placement edit that had accidentally changed the legacy test-only `DecomposingBody` deposit loop. Its original body-position mapping has been restored in commit `60d5efd`; the new shared dead-release path continues to deposit each unit at its own physical placement. The workflow for the repaired commit passed formatting, architecture checks, and all five focused test groups; the full Rust suite was still running at the last check. The added transaction-queue guard/test are newer than that workflow and require fresh verification.

### Resonance is a real lifecycle blocker (2026-10-27)

Added a test that retained resonance prevents final release instead of being silently discarded. Inspection confirms the current resonance reservoir has bounded deposit/withdrawal and query APIs, but no passive damping/clear-to-heat transaction; the live harmonic refresh is inside the living-only branch. Consequently, a dead organism with nonzero retained resonance cannot naturally reach the current zero-resonance release gate. Do not weaken the gate by clearing the reservoir or treating it as usable energy. The shared energy ledger currently has no explicit resonant-store delta in its transaction shape, so the follow-up must add a conservation-correct resonance-to-heat settlement (or a physically grounded transfer of resonance into released material) before release can handle this case. This is a concrete remaining blocker, not a solved path.

### Verification results from the repaired baseline commit (2026-10-27)

The workflow for `60d5efd` passed formatting, architecture checks, and every focused test group. The full Rust suite completed with **778 passed, 10 failed, 1 ignored**; failures were in construction/contact/cavity/combine/movement/viability/storage tests rather than the new decomposition tests. Because this PR touches some of those subsystems, their baseline status and causal relationship to this branch are not yet proven. Clippy then failed on **34 application-target and 17 test-target errors**; the reported diagnostics were in other modules, but a clean baseline comparison is still needed before classifying all of them as pre-existing. This run predates the latest owner-transaction and retained-resonance tests. A newer run is verifying those changes; its formatting gate currently fails, so the exact rustfmt difference still needs to be isolated before the branch can be considered clean.
