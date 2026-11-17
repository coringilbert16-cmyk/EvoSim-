# PR 6: Reach the movement-enabled throughput target

**Target:** More than 1,009 completed simulation ticks per minute (>16.82 ticks/s) with resonance movement enabled, seed 42, 60 seconds, and the standard 1,280-object environment.

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

A 60-second run must complete **more than 1,009 ticks** with movement enabled and the standard 1,280 physical objects. It must also show valid chemistry accounting, no lifecycle errors, preserved molecular identity, and no geometry/invariant regressions.


## First matched profile (seed 42, 60 seconds)

The movement-enabled run completed 208 ticks. The raw per-tick profile covered ticks 1–208 and averaged:

- 1,284 live objects and 823,394 unordered pairs per tick (the initial environment contained 1,280 objects).
- Resonance emitter construction: 2.68 ms/tick.
- Spectrum validation: 0.08 ms/tick.
- Resonance pair loop: 148.26 ms/tick.
- Bounded response calculation: 0.05 ms/tick.
- Placement transaction: 39.53 ms/tick, including 17.31 ms of original-position collision checks and 16.04 ms of mover-overlap checks.

The pair loop is the primary movement bottleneck. Sampled profiling estimates approximately 125.9 ms/tick in frequency matching. Placement collision validation is the second target.

## Pair-frequency lookup optimization — rejected

A bounded sorted-index lookup was evaluated and rejected. It materially regressed throughput (139 ticks in 60.408 seconds versus 208 ticks in the preceding run) and increased the profiled pair-loop cost. The index and its differential test were reverted. The retained implementation is the direct all-pairs scan with the safe outer-loop geometry reuse described above. Do not reintroduce sorted lookup without a materially different design and a measured reason to retest it.

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

The follow-up run split world-tone settlement from hotspot settlement on the same 60-second seed-42 workload. The measurements identify world-tone settlement—not hotspot settlement—as the substantial non-movement target. The standard workload contains no organisms, so organism-processing costs must be measured separately before generalizing to organism-present workloads. No simulation behavior or chemistry rules are changed by this diagnostic instrumentation.

## Whole-tick profile results (seed 42, 60 seconds)

The first expanded-profile run completed 164 ticks in 60.085 seconds (about 164 ticks/minute), with 1,280 initial physical objects and 1,360 constituents. The run used the Ubuntu 26.04.1 hosted image. It retained 1,286 physical resources and 1,360 constituents at completion; the energy residual was approximately `-4.92e-7`.

Median sampled phase times across 16 samples (one tick in ten):

| Phase | Median |
|---|---:|
| Resonance movement | 246.2 ms/tick |
| World-tone organism settlement plus hotspot | 79.6 ms/tick |
| Environmental BREAK discovery | 12.4 ms/tick |
| Ambient transport | 11.5 ms/tick |
| Environmental BREAK resolution and event recording | 9.7 ms/tick |
| Resonance snapshot plus environmental bond index | 2.7 ms/tick |
| Environmental COMBINE discovery | 0.23 ms/tick |
| Displacement discovery | 0.34 ms/tick |
| Organism processing | below 0.01 ms/tick in this organism-free workload |

The movement subprofile measured a median 181.7 ms/tick in the resonance pair loop and 58.3 ms/tick in placement validation. Thus movement remains the largest measured cost, but the combined world-tone/hotspot phase is a substantial second target. The organism phases are effectively absent because this standard chemistry workload starts with zero organisms; they must be measured separately on an organism-present workload before generalizing.

The run confirms that the next optimization should not be selected solely from movement profiling. I split the world-input measurement into world-tone organism settlement and hotspot settlement to identify which part owns the approximately 80 ms/tick. No physics or chemistry behavior is changed by this instrumentation.

The 164-tick profiling run was executed while several full Rust CI workflows were running concurrently. Treat its absolute throughput as diagnostic only; the phase proportions are useful for prioritization, but an isolated matched run is required before accepting a throughput comparison. The world-input phase has since been split into separate world-tone organism settlement and hotspot settlement measurements. A follow-up run will be launched after CI is idle.


## Final split profile and diagnostic cleanup (2026-11-17)

The split 60-second seed-42 experiment completed successfully (workflow run [44115581483](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/44115581483)). The experiment and aggregate analysis jobs passed.

- World-tone settlement: approximately 95–114 ms/tick.
- Hotspot settlement: approximately 1.0–1.1 ms/tick.
- Movement remained the dominant measured phase in this profiled run.
- The run's absolute throughput is diagnostic only because profiling and concurrent hosted-runner activity affect timing.

The whole-tick profiler now caches the opt-in environment setting with `OnceLock`, avoiding an environment lookup on every simulation tick. It remains disabled unless `EVOSIM_PROFILE_TICK` is set before process startup.

A final clean 60-second seed-42 run with movement enabled and both profilers disabled is in progress. Its throughput, chemistry diagnostics, inventory, lifecycle checks, and energy residual are the acceptance evidence; profiler runs are not substitutes for this clean measurement.

See [the staged parallel optimization roadmap](parallel-optimization-roadmap.md) for the proposed PR 363–367 sequence. Each implementation PR is intended to target the then-current `main`, not another feature branch.