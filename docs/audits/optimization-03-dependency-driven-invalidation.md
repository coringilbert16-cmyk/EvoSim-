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


## Source audit — first pass (main at `f2d77bd`)

### Existing exact cache worth studying

`update_organism_harmonics` in `src/perception/harmonics.rs` already has a dependency key:

- `Organism::structure_revision`
- `Organism::position_revision`
- `ActiveMaterialField::revision`
- The current tick when a world-tone sample is present (otherwise zero)

It returns without rebuilding the harmonic spectrum when the key is unchanged. Organism structure and position mutation helpers clear this key and increment their respective revisions. This is an existing example of dependency-based recalculation, not a new cache proposal.

Important limitation: the field dependency is the single global field revision. Any marked field mutation invalidates every organism's harmonic result, even if the mutation is spatially unrelated. Whether that can be narrowed depends on the exact spatial dependencies of cavity/environment resonance; no local invalidation is justified yet.

### Environmental field revision contract

`ActiveMaterialField` maintains a global `revision` and per-cell `cell_revisions`. `mark_cell_changed` increments both the touched cell revision and global revision, and updates the maintained environmental bond index for that cell.

The main field mutation APIs inspected so far consistently call the invalidation path:

- Complete-object movement marks source and destination cells.
- Atomic split marks the source and every destination.
- Logical and physical deposits mark their destination.
- Batch physical deposits mark each committed destination.
- Environmental COMBINE and displacement commit paths mutate cell vectors directly, then explicitly mark every touched cell.
- Movement batches similarly mark every changed cell.

The field also has `local_revision_for_positions`, which folds revisions for cells containing supplied positions. This is a possible exact dependency token only if the caller can prove that the supplied positions cover every field cell that can influence the derived result. It is not automatically safe for interactions with global reach or occlusion.

### Staged interaction versions are not production invalidation

`EnvironmentalMaterialSnapshot::from_field` currently assigns `topology_generation = 0` and `state_version = 0` to every snapshot material. Its own comment explicitly scopes topology generation to the immutable pass snapshot. The versioned `MaterialRecord`/`ConstituentRef` inventory supports stale-reference rejection within that inventory, but these zero-valued fields are not persistent per-material mutation versions.

The inventory builder invalidates dependent bond/contact records when an upserted material's topology generation changes. That behavior is useful as a model for a revision contract, but it does not establish that production field mutations currently maintain such generations.

### Early classification

- Organism harmonic spectra: already dependency-cached; global field revision may over-invalidate. Tick remains a dependency when world-tone input is present.
- Genome cavity and developmental realization: already cached against organism structure revision. Developmental realization also uses a seed-scale reference derived from the catalog; catalog mutability and whether that reference can change need separate verification.
- Environmental chemistry broad-phase cursors: pass-local snapshots are guarded by field revision equality. This is stale-snapshot rejection, not a persistent cache.
- Environmental bond index: incrementally maintained per changed cell, with a full rebuild after deserialization. This is an established example of safe incremental maintenance tied to centralized mutation paths.
- Resonance movement pair forces, world-tone exposure, and environmental reaction discovery: remain tick-/phase-/spatially dependent in important ways. No event-driven replacement is proposed without a complete dependency proof.

### Open audit questions

1. Enumerate all production writes to `ActiveMaterialField.cells` and confirm each successful write reaches `mark_cell_changed`; distinguish test-only direct mutation.
2. Confirm all organism structure and position mutation sites call the corresponding revision helper.
3. Determine whether catalog contents can change after simulation construction, and identify any cache keys that omit catalog dependencies.
4. Trace the complete dependency footprint of organism harmonic spectra and cavity/world-tone contributions before considering per-cell invalidation.
5. Identify expensive recalculations with a narrower, provable dependency set than the current global field revision.


### Organism revision mutation-site findings

A repository-wide code search identified the production call sites for the organism revision helpers. The main paths include transformation BREAK, COMBINE, movement, reproduction, and simulation-level construction/cleanup. The movement commit path calls `mark_position_changed` after translating the organism's occupied cells, developmental origin, and all structural-unit placements.

However, the audit found a concrete invalidation concern that must be resolved before any new cache is designed:

- In `src/chemistry/combine_runtime.rs`, the stored-material COMBINE path assigns a newly formed `trial_structure` to `organism.structure` (around line 1756) without a nearby `mark_structure_changed`.
- The ordinary organism-to-organism COMBINE path also assigns `trial_structure` (around line 1869) without a nearby `mark_structure_changed`.
- The simulation scheduler calls `try_combine` and, after a successful attempt, records consequences and experience; the inspected caller does not itself mark the structure changed.
- Other COMBINE paths in the same module do call `mark_structure_changed` (around lines 948 and 1090), so invalidation behavior is not uniform across all structural mutation routes.

These are not yet declared confirmed cache bugs: the precise branch semantics, any indirect mutation helpers, and the affected cached consumers still need a complete call-chain review. They are high-priority audit targets because the cavity, developmental-realization, and harmonic caches depend on structure revision.

### Field mutation coverage findings

GitHub code search found `mark_cell_changed` only inside `src/simulation/environment.rs`, which is a good encapsulation boundary. Other production mutation modules use the field's `mark_changed_at_index` wrapper after direct cell-vector changes. The inspected COMBINE and displacement commits explicitly mark every source/destination cell after their staged mutation.

Direct `physical_materials.clear()` occurrences found in world-tone snapshot and chemistry modules are within test helpers/fixtures in the inspected locations; they are not evidence of production writes bypassing invalidation. A full search of every mutable borrow and mutation expression is still required before treating this as complete coverage.

### Revised priority

Before introducing new revision counters or event queues, first resolve the existing structure-revision inconsistencies. Adding more caching on top of incomplete invalidation would risk stale derived state. The safest next step is to trace every successful structural mutation to its owner and ensure one authoritative invalidation at the transaction commit boundary, with tests proving that cavity/developmental/harmonic consumers refresh after each mutation family.
