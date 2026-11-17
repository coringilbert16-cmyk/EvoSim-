# Parallel optimization roadmap

This roadmap is intentionally sequential. Each implementation PR targets `main`, is independently testable and reversible, and begins only after the preceding stage is validated. Do not stack implementation branches or merge a later stage ahead of its prerequisites.

## Shared acceptance contract

- Standard throughput run: 60 seconds of real elapsed time, seed 42, 1,280 starting physical objects (1,200 independent resources plus 80 molecules), 1,360 constituents.
- Target: more than 1,009 completed ticks per minute with all normal simulation functionality enabled.
- Report throughput separately from profiler runs; profiling is diagnostic and must be disabled for acceptance measurements.
- Preserve chemistry outcomes, molecular identity/lifecycle, geometry, movement, and energy accounting. Compare reaction counts, inventory, diagnostics, and energy residuals as well as throughput.
- No omitted work, arbitrary distance cutoffs, dropped interactions, changed physical laws, or approximations that alter behavior.
- Workers calculate from immutable snapshots. A single authoritative simulation state owns commits and the energy ledger. Results are reconciled deterministically in a stable order. Retain a sequential reference/fallback.

## PR 363 — Deterministic parallel execution foundation

**Purpose:** Establish safe infrastructure before parallelizing simulation subsystems.

**Scope**
- Define immutable per-tick input snapshots and worker-owned result buffers.
- Define deterministic result collection, ordering, validation, and reconciliation.
- Define behavior for incomplete, invalid, or failed worker results; never silently drop work.
- Keep authoritative state mutation, transaction commits, and energy-ledger ownership on the simulation thread.
- Add differential test utilities that compare sequential and parallel calculations.
- No subsystem behavior or scheduling changes in this PR.

**Acceptance**
- Sequential and parallel test paths produce equivalent results within explicitly documented floating-point tolerances.
- Deterministic reconciliation is repeatable across worker scheduling/order.
- Full Rust tests, architecture checks, formatting, and Clippy pass.
- No throughput claim required; this is the correctness foundation.

## PR 364 — Parallel resonance movement calculations

**Purpose:** Reduce the dominant measured movement pair-loop cost.

**Scope**
- Partition the complete unordered emitter-pair workload into balanced chunks.
- Evaluate every pair exactly once; do not add cutoffs or omit distant/nonmatching pairs.
- Use worker-local force-contribution buffers and stable-order reconciliation.
- Preserve the existing frequency tolerance, phasor cross-energy, attenuation, mass scaling, force direction, and response rules.
- Keep placement validation and all authoritative movement commits sequential and unchanged in this stage.

**Acceptance**
- Pair coverage tests prove no missing or duplicate unordered pairs.
- Differential tests compare per-emitter force contributions and final movement against the sequential reference.
- Molecular integrity, boundaries, collisions, energy accounting, and lifecycle checks remain valid.
- Full CI passes and a clean matched benchmark demonstrates a repeatable throughput improvement.

## PR 365 — Movement placement validation optimization

**Purpose:** Reduce the second major movement cost after parallel pair calculation is isolated.

**Scope**
- Profile original-position checks and mover-to-mover overlap checks independently.
- Investigate exact spatial indexing or reusable geometry only where it can prove candidate completeness.
- Preserve the current conservative collision acceptance/rejection decisions.
- Keep the placement transaction atomic; do not weaken validation to improve speed.

**Acceptance**
- Differential tests show identical placement decisions for adversarial and randomized cases.
- No new overlaps, boundary violations, lost/duplicated materials, or molecular-integrity regressions.
- Full CI passes and placement cost falls measurably in an isolated profile.

## PR 366 — Parallel world-tone exposure calculations

**Purpose:** Address the largest measured non-movement phase.

**Evidence:** PR 362 profiling separated world-tone settlement at approximately 95–114 ms/tick from hotspot settlement at approximately 1.0–1.1 ms/tick. Parallelize world-tone exposure; do not prioritize hotspot settlement on this evidence.

**Scope**
- Separate immutable geometry/snapshot preparation from independent receiver exposure calculations.
- Share read-only geometry across workers and partition receiver work.
- Retain every realized body as a receiver and potential occluder.
- Preserve exact visibility, occlusion, path, and exposure calculations; do not reduce the occluder set or lower geometry limits.
- Reconcile energy settlement through the authoritative simulation/ledger path.

**Acceptance**
- Every receiver is accounted for exactly once; every occluder remains available to all relevant calculations.
- Differential exposure and energy-settlement tests match the sequential reference within explicit tolerances.
- Energy residuals, sunlight/resonance behavior, and all geometry invariants remain valid.
- Full CI passes and a clean matched benchmark demonstrates a repeatable improvement.

## PR 367 — Other independent subsystem calculations

**Purpose:** Continue optimization only where profiling demonstrates meaningful cost and independent work exists.

**Candidates**
- Ambient transport snapshot construction and per-material obstacle/flow calculations.
- Environmental BREAK candidate discovery and other chemistry discovery work.
- Other geometry/perception calculations identified by fresh phase profiles.

**Scope**
- Re-profile after PRs 364–366; do not assume earlier phase costs remain dominant.
- Identify real data dependencies before parallelizing. Discovery may be parallelizable even where transaction resolution is not.
- Preserve candidate completeness, stable ordering, reaction eligibility, transaction atomicity, and energy-ledger authority.
- Implement only measured, behavior-preserving opportunities. Split this PR if a candidate has materially different correctness boundaries.

**Acceptance**
- Differential tests prove candidate/work completeness and deterministic reconciliation.
- Chemistry reaction counts and outcomes, lifecycle records, molecular integrity, and energy accounting remain valid.
- Full CI passes and each retained optimization shows a repeatable clean-run improvement.

## Execution order

1. Complete and validate PR 362 profiling/diagnostic cleanup.
2. Implement and validate PR 363 before any worker-based subsystem execution.
3. Implement PRs 364, 365, and 366 sequentially, each targeting the then-current `main`.
4. Re-profile and select PR 367 work based on measured post-change bottlenecks.
5. Do not merge any PR without explicit approval and full green CI.
