# Validate Deterministic Sunlight and World-Tone Approximation

> **Planning scaffold only.** This is a later validation gate for sunlight/world tone; it does not implement production changes.

## Purpose

Validate the performance-oriented directional first-hit/single-sweep sunlight approximation against a deterministic exact small-world reference, while preserving energy accounting and the intended frequency-shifted onward propagation model.

## Scope

- Audit current source-frame generation, exposure assignment, occlusion, signal propagation, and energy-ledger settlement on current main.
- Compare approximate exposure/capture against an exact reference on small deterministic worlds and report error rather than hiding it.
- Test occlusion, edge cases, deterministic source rotation, duplicate claims, and the source-energy budget.
- Verify that onward propagation changes frequency composition as modeled and is not implemented as literal mirror-ray reflection.
- Benchmark representative larger worlds, including thousands of objects, and report runtime/memory/workload details.

## Acceptance criteria

- Exact-reference comparisons report measured approximation error with documented bounds.
- Occlusion and first-hit semantics are deterministic and tested.
- No object or surface can claim the same source energy more than once per frame.
- Captured energy, reflected/propagated energy, and modeled sinks reconcile against the independent world-energy inventory.
- Frequency-shifted onward propagation remains distinct from mirror-ray tracing.
- Representative performance measurements demonstrate the approximation's benefit; no per-object ray tracing or all-pairs scan is introduced.

## Dependencies and boundaries

- Follow the approved sequence: cache-version safety, material/energy accounting, shared chemistry/transaction path, and post-death lifecycle contract first.
- Depends on PR #208 energy accounting and PR #210 lifecycle integration; coordinate with PR #214 shared chemistry runtime where world-tone-delivered energy can enter chemistry.
- This phase is deliberately separate from PR #211 cache versioning. Do not mix cache schema work and sunlight approximation in one implementation diff.
- PR #196 constructor/Bob work remains out of scope.

## Guardrails

- Audit current main before implementation and compare named test failures against the latest accepted baseline.
- Keep every command/test under five minutes; timeout is incomplete evidence.
- Do not claim world-tone capture or whole-world energy conservation is proven until exact-reference, source-budget, occlusion, and inventory checks pass.

## Status

- [ ] Current-main source audit
- [ ] Exact-small-world reference comparison
- [ ] Source budget, occlusion, and duplicate-claim tests
- [ ] Representative performance benchmarks
- [ ] Exact-head verification and baseline comparison

## Current dependency/status reconciliation — 2026-10-30

- PR #208 independent energy accounting, PR #211 cache schema/fingerprint validation, and PR #214 shared chemistry runtime/scheduler are merged.
- PR #226 flow/obstacle-aware transport and PRs #229–#240 chemistry/molecular characterization are merged.
- PR #210's death/continued-chemistry **plan** is now merged, but its lifecycle implementation and conservation acceptance gate are not complete. This sunlight phase remains blocked on that implementation, not merely on the existence of the plan.
- This PR remains planning-only. No sunlight equations, runtime, or production code are changed here.

Before implementation, audit the current main source-frame, first-hit exposure, occlusion, frequency-shifted onward propagation, staged settlement, and independent world-energy inventory paths. Preserve the owner-approved deterministic rotating off-screen source and parallel-ray approximation. Compare it against an exact small-world reference; do not substitute mirror reflection or introduce all-pairs/per-object ray tracing.
