# Transport obstacle-bin buffer reuse experiment

## Status

Experiment setup only. This document records the previously measured bottleneck and the isolated candidate to implement and benchmark. It does not claim a performance improvement.

## Prior profiling evidence

The opt-in transport stage profiling in PRs #271 and #272 measured snapshot construction, obstacle extraction, spatial-bin construction, movement evaluation, and movement commitment on matched distributed and dense 256-object layouts.

The accepted baseline's dense movement-evaluation stage was the dominant cost at approximately **17,836 microseconds**. Spatial-bin construction was the next target at approximately **1,984 microseconds**. PR #277's follow-up confirmed that the dense movement-evaluation workload contained 65,536 center candidate references. PR #278 documented why those references are legitimate physical interactions in that deliberately packed layout: filtering them would change the physical result.

This experiment therefore targets only the secondary spatial-bin construction/allocation cost. It must not be presented as the dominant-stage optimization.

## Hypothesis

The current transport pass rebuilds the `Vec<Vec<Obstacle>>` bin grid and its per-bin backing allocations on every tick. Reusing the grid and its vector capacities across ticks may reduce allocator and initialization overhead.

## Candidate design

- Introduce a transport-owned workspace whose lifetime spans simulation ticks.
- Keep the grid dimensions explicit; resize only when world dimensions or bin geometry require it.
- At the beginning of each transport pass, clear every bin while retaining its allocated capacity, then repopulate from the freshly extracted obstacles.
- Rebuild obstacle records and bin membership every tick. Materials translate and rotate, so cached obstacle positions, radii, solidity, source identity, and bin membership must never be reused across ticks.
- Preserve the current 5-unit bin size, row wrapping, horizontal bounds, obstacle influence equations, complete local candidate coverage, movement ordering, snapshot/commit behavior, and all chemistry/energy semantics.
- Do not add candidate caps, truncate bins, skip obstacles, or reuse stale candidate lists.
- Keep a baseline path selectable in the same full simulation benchmark so the only intended difference is workspace/buffer reuse.

## Correctness gates

- Compare baseline and candidate movement decisions and resulting physical state for deterministic small fixtures.
- Preserve exact material inventory, object count, constituent count, internal bonds, realized shapes, and field revisions.
- Verify every candidate returned by the reused workspace matches the fresh-build path for distributed, dense, seam-wrapping, large-radius, and changing-object-count cases.
- Exercise workspace resize/shrink/grow behavior and ensure removed obstacles cannot survive a clear/rebuild cycle.
- Run formatting, architecture checks, focused transport tests, full Rust tests, and Clippy on the exact PR head.

## Performance acceptance

Use the merged PR #301 molecular transport fixture and the same 60-second full-simulation protocol recorded in PR #302:

- 250 × 250 world, seed 42.
- 1,280 physical objects: 1,200 individual resources plus 80 intact two-constituent molecules (1,360 constituents).
- Independently rebuild the identical initial physical state for baseline and candidate.
- Run one uninterrupted 60-second wall-clock interval per implementation at maximum simulation speed; no tick cap or fixed tick batching.
- Record elapsed time, completed ticks, ticks/second, final object/constituent counts, and conservation checks.
- Also retain stage-level measurements to confirm whether bin construction/allocation actually fell and to quantify its contribution to total tick time.

Do not merge unless exact-head CI passes, correctness is equivalent, and the full 60-second simulation shows a measurable total-throughput improvement. A reduction in bin-stage time alone is insufficient if it does not improve total simulation throughput.

## Related work

- PR #271/#272: opt-in stage profiling that identified dense movement evaluation as the dominant stage and spatial-bin construction as the secondary target.
- PR #277: reused center velocity to remove duplicate obstacle evaluation; this is now represented by the merged transport optimization.
- PR #278: exact modulo fast path for dense obstacle distance queries; historical evidence explains why candidate filtering is not acceptable.
- PR #301: merged reproducible 1,280-object molecular transport benchmark fixture.
- PR #302: merged singleton center-velocity reuse, providing the current transport baseline for this experiment.
