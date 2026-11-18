# Optimization 03 — Dependency-Driven Invalidation and Event-Triggered Recalculation

## Status

Investigation branch created from `main`. No production behavior changes are authorized by this document.

## Context

Workstream 2 (PR #370) found repeated material-property and geometry validation work. Batching one shape-resolution path reduced its measured time per tick, but a matched 10-pair, 60-second benchmark found no statistically supported end-to-end throughput improvement:

- Mean paired throughput change: +0.219%.
- Median paired change: -0.017%.
- 95% confidence interval: -0.488% to +0.925%.

The result suggests that local calculation reductions are insufficient when larger simulation phases dominate. Workstream 3 therefore investigates whether derived work can be avoided altogether when its dependencies have not changed.

## Objective

Identify calculations that can be safely skipped or incrementally updated until one of their actual dependencies changes, without changing exact chemistry, physics, geometry, energy accounting, candidate completeness, or deterministic behavior.

## Existing infrastructure to examine

Prior audits identified these potentially useful mechanisms, but none should be assumed to provide complete invalidation coverage without tracing all mutation paths:

- Stable physical material identities.
- Material topology generations and state versions in staged environmental-interaction infrastructure.
- Maintained environmental bond indexes.
- Per-cell and global field revisions.
- Deterministic parallel execution and ordered reconciliation.

In particular, staged topology/state revisions were not wired across all production mutation paths in the previous audit. They are not yet a universal cache-invalidation contract.

## Investigation plan

1. **Dependency inventory:** identify expensive derived calculations and enumerate every input that can affect their outputs.
2. **Mutation-path audit:** trace all writers that can change those inputs, including transport, movement, reactions, split/merge, catalog mutation, storage, construction, and environmental field updates.
3. **Revision-contract audit:** determine which existing identities/revisions are authoritative, which mutation paths update them, and whether any path can leave a derived result stale.
4. **Recalculation classification:** classify work as invariant until dependency change, incrementally maintainable, or inherently tick-/phase-/space-dependent and therefore still requiring regular evaluation.
5. **Correctness design:** define exact invalidation and stale-result rejection rules before implementation.
6. **Narrow prototype:** only after the audit, implement the smallest exact change with focused mutation-coverage and deterministic-output tests.
7. **Measurement:** use the standard 60-second real-clock workload and compare end-to-end performance. Avoid broad profiler instrumentation if it would materially distort the measured path.

## Non-negotiable constraints

- No approximations, arbitrary locality assumptions, reduced interaction sets, or omitted candidates.
- No weakening or skipping correctness validation without an authoritative validated-state contract.
- No global mutable cache or incomplete parallel revision counters.
- Preserve deterministic behavior and ordered reconciliation where required.
- Preserve continuous/global interactions when event-driven replacement would change semantics.
- Keep priority 4 separate; do not stack PRs.

## Acceptance gate

No production behavior change until the dependency graph and mutation coverage are source-audited. Any implementation must demonstrate that every relevant mutation invalidates affected derived state, unchanged dependencies preserve exact outputs, and stale state cannot be consumed. No merge without full required CI and a demonstrated meaningful end-to-end improvement.

## Initial status

- [x] Dedicated branch created from current `main`.
- [x] Investigation scope and acceptance constraints recorded.
- [ ] Source-backed dependency inventory.
- [ ] Complete mutation-path/revision coverage audit.
- [ ] Recalculation classification and implementation proposal.
- [ ] Focused correctness tests and performance evidence.
