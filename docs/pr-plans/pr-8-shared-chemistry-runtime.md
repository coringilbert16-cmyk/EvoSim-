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
- [ ] Generalize candidate discovery/activation to environmental and organism-held material using local spatial neighborhoods.
- [ ] Bring dead/decomposing material onto the same chemistry policy without duplicating its physical transaction path.
- [ ] Add transactional and independent-inventory acceptance tests for environment-only and organism-held reactions.
- [ ] Benchmark candidate discovery and document bounded-work behavior.
- [ ] Exact-head verification and named-failure comparison against the accepted baseline.
