# PR 6: Reach the movement-enabled throughput target

**Target:** More than 800 completed simulation ticks per minute (>13.33 ticks/s) with resonance movement enabled, seed 42, 60 seconds, and the standard 1,280-object environment.

## Baseline and constraint

- Movement disabled: 666 ticks / 60.024 s (11.10 ticks/s).
- Movement enabled: 157 ticks / 60.192 s (2.61 ticks/s).
- Movement pass: 260.5 ms/tick; resonance pair loop: 197.3 ms/tick; placement validation: 59.2 ms/tick.

The movement-disabled baseline is already below the acceptance target. Therefore movement optimization alone cannot meet the target: the end-to-end simulation tick must also become faster than the current baseline.

## Scope

1. Reduce overhead in the exact pairwise resonance calculation without changing its physical law.
2. Reduce placement-validation cost only after measuring the first pair-loop change.
3. Profile the remaining ordinary simulation tick if movement costs no longer dominate.
4. Re-run the matched 60-second seed-42 workload and retain all movement, chemistry, molecular-integrity, and energy-accounting checks.

## Preserved behavior

- Every unordered pair remains considered; no distance cutoff or dropped interactions.
- Same-frequency cross-energy tolerance remains 1e-9.
- Same-phase repulsion, opposing-phase attraction, quadrature neutrality, mass scaling, distance attenuation, and bounded response remain unchanged.
- Existing atomic placement transaction and conservative collision rules remain unchanged.
- No persistent velocity/acceleration, Newtonian mechanics, multipole approximation, or chemistry redesign.

## Acceptance

A 60-second run must complete **more than 800 ticks** with movement enabled and the standard 1,280 physical objects. It must also show valid chemistry accounting, no lifecycle errors, preserved molecular identity, and no geometry/invariant regressions.


## First matched profile (seed 42, 60 seconds)

The movement-enabled run completed 208 ticks. The raw per-tick profile covered ticks 1–208 and averaged:

- 1,284 live objects and 823,394 unordered pairs per tick (the initial environment contained 1,280 objects).
- Resonance emitter construction: 2.68 ms/tick.
- Spectrum validation: 0.08 ms/tick.
- Resonance pair loop: 148.26 ms/tick.
- Bounded response calculation: 0.05 ms/tick.
- Placement transaction: 39.53 ms/tick, including 17.31 ms of original-position collision checks and 16.04 ms of mover-overlap checks.

The pair loop is the primary movement bottleneck. Sampled profiling estimates approximately 125.9 ms/tick in frequency matching. Placement collision validation is the second target.

## Pair-frequency lookup optimization

The next implementation replaces the full nested frequency comparison scan with a bounded sorted-index range lookup:

- Each emitter precomputes a sorted index of its at-most-four spectral components.
- For each left component, binary search finds a conservative range of possible right-side matches.
- The original absolute-difference predicate (less than or equal to 1e-9) is still applied to every candidate, including a floating-point rounding margin around the search interval.
- Matching components are accumulated in their original order, preserving the prior floating-point summation order.
- No physical pairs or valid frequency matches are omitted.

A differential unit test compares the optimized result exactly against the original nested all-pairs implementation, including tolerance-boundary and multiple-match cases.

The 208-tick result is a useful improvement over the previous 157-tick run, but the >800 ticks/minute target remains unmet. The new lookup must pass full CI and a matched experiment before its performance effect is accepted.

## Frequency-index regression and rollback (2026-11-17)

The matched 60-second seed-42 run after the sorted-frequency-index change completed 139 ticks in 60.408 seconds (about 138 ticks/minute), compared with 208 ticks/minute on the preceding pair-loop restructuring. The profiled pair loop increased from about 148 ms/tick to 290–293 ms/tick, while placement remained about 40 ms/tick. Chemistry integrity and energy accounting remained valid, but this optimization materially regressed throughput.

The sorted-frequency index and its differential test have therefore been reverted. The earlier all-pairs restructuring from commit 4d91b58190317ed7b786170149caf8f2a12bfd43 is retained unchanged. No interaction pairs, frequency tolerance, force law, placement rules, or chemistry behavior are intentionally changed by this rollback.

Next validation: rerun the standard 60-second seed-42 movement-enabled experiment with the same 1,280-object workload and profiling enabled. Compare against 208 ticks/minute. Do not proceed to another optimization until this rollback measurement confirms the expected recovery.

## Whole-tick profiling expansion (2026-11-17)

The rollback experiment did not provide a fair comparison with the 208-tick run because the hosted runner image changed. The rollback run completed 136 ticks in 60.41 seconds; the pair loop measured 211.8 ms/tick and placement 72.8 ms/tick. A second run on the older runner image showed 139 ticks and a 291.2 ms/tick pair loop. The large shifts in both movement and placement timings make runner variation a material confounder. No further performance conclusion will be drawn from those cross-image comparisons.

The next diagnostic pass expands beyond movement. An opt-in `EVOSIM_PROFILE_TICK` sampler records one tick in ten and separates:
- Tick prelude and transformation/living-state work.
- Ambient transport.
- Resonance movement.
- World-tone inputs and hotspot settlement.
- Local resonance snapshot and environmental bond-index construction.
- Environmental BREAK, COMBINE, and displacement discovery/resolution.
- Organism chemistry and maintenance.
- Organism decisions and movement.
- Reproduction and tick finalization.

The sampler is disabled by default and creates no timers or phase records on ordinary ticks when disabled. Existing movement profiling remains available independently. The experiment workflow now exposes the whole-tick profiler as a separate opt-in setting.

The next standard experiment will collect both profiles on the same runner image and same 60-second seed-42 workload. We will use the measured phase costs to prioritize the next optimization, including non-movement work such as ambient transport snapshot construction and organism processing. No simulation behavior or chemistry rules are changed by this diagnostic instrumentation.
