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
- [x] Identified a hard integration gap: environmental `PhysicalMaterialBond` records store topology/endpoints but no bond-energy holder, while decomposing bodies use a separate bond-break settlement path. Environment-only realization must not invent an energy source; the shared transaction/ownership bridge needs to preserve or derive actual energy provenance.
- [ ] Refactor chemical BREAK settlement so its energy transaction can be staged independently of a graph mutation and then committed atomically with the selected source mutation. Physical-material internal bonds currently have zero stored energy on restoration; charge work to organism usable energy/resonance and credit no bond-energy release unless a real stored-energy allocation path exists.
- [ ] Add environmental material to the **chemical** candidate/activation runtime using local spatial neighborhoods and the same transaction boundary. Existing `environmental_break_candidates`/`try_start_environmental_break` is a separate voluntary BREAK transformation, not chemical activation; it does not satisfy this gate.
- [ ] Add acceptance coverage for dead-material behavior. Audit confirms nonliving organisms remain in `Simulation::step`'s organism loop and pass through the same `chemical_reaction::accumulate` path; do not reactivate the disconnected legacy recycling/decomposition path as a second chemistry engine.
- [ ] Add transactional and independent-inventory acceptance tests for environment-only and organism-held reactions.
- [ ] Benchmark candidate discovery and document bounded-work behavior.
- [x] Reconciled latest `main` (`29f0f510062eb1ec4a1db3e1f7bb3cce12ba2d08`) into this branch; the branch is no longer behind main.
- [x] Exact integration-head verification: formatting, architecture checks, and all five focused test groups pass. Full suite: 763 passed, 10 failed, 1 ignored in 307.58s; all 10 named failures are known baseline failures (the accepted baseline had 21 distinct failures). Clippy remains at the existing 34 app-target and 14 test-target diagnostics.
- [ ] Exact implementation-head verification after the next code change; compare named failures against the accepted baseline.


## Correct implementation order after main integration (2026-10-27)

1. Refactor chemical BREAK settlement into a staged energy transaction that can be committed atomically with either a graph-bond mutation or an environmental physical-material split. For `PhysicalMaterialBond`, the current source-backed value is zero stored bond energy: restoration derives bond strength but explicitly sets `bond_energy: 0.0`. Therefore an environmental chemical BREAK may release no bond energy and must charge required work to the organism's usable-energy/resonance sources; do not infer stored energy from `ResourceProperties::potential_energy` or activation signal. Only add nonzero stored bond energy if a real formation/allocation path and inventory ownership are implemented together.
2. Extend local/indexed discovery and activation to eligible environmental physical material, then route realization through the existing transactional boundary rather than duplicating it.
3. Keep dead organisms on the canonical reaction path; add an acceptance regression rather than enabling the legacy recycling carrier.
4. Add environment-only and cross-holder acceptance tests with independent integer-material and world-energy inventory oracles.
5. Benchmark the completed path and document current temporary safeguards. The 307.58-second full-suite run slightly exceeded the five-minute command target, so recheck runtime after implementation; do not weaken test coverage to meet it.


### Scope clarification from live code audit (2026-10-27)

Environmental material already has a physical BREAK action path (`environmental_break_candidates` and `try_start_environmental_break`) with escrow/rollback and a zero-energy settlement for pre-existing internal connections. That is **not** the missing shared chemical runtime: it does not accumulate chemical activation for environmental interfaces. Keep it intact; do not mistake it for chemical environmental reaction support or add a parallel chemical law. The remaining gate is to connect eligible environmental interfaces to the canonical chemistry scheduler only after their energy/ownership semantics are defined. The current workflow test job is still in progress; the GitHub log endpoint returned a missing blob, so no final status is inferred from that endpoint.
