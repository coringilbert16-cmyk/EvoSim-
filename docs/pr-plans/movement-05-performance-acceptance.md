# PR 5: Benchmark and finalize resonance-driven movement

**Status:** Functional validation complete; full Rust CI passed. Performance target remains unmet; the measured regression is documented below and becomes the next focused optimization task.
**Base:** `main`
**Goal:** Verify the completed movement system at the standard environment scale, report its measured cost, and close the integration sequence without expanding the physics model.

## Scope completed

1. Ran a matched 60-second, seed-42 environmental chemistry baseline with movement disabled.
2. Ran the same 60-second, seed-42 experiment with resonance movement enabled and internal profiling.
3. Verified the same initial environment in both runs: 1,200 independent resources, 80 two-constituent molecules, 1,360 constituents, and 1,280 physical objects.
4. Confirmed movement, chemistry integration, energy accounting, and lifecycle validation in the movement-enabled run.
5. Ran full Rust CI and Clippy successfully on the PR head.

## Performance acceptance target

The required target is **more than 800 completed ticks per minute** (more than 13.33 ticks/second) with resonance movement enabled, using the standard 60-second seed-42 workload of 1,280 physical objects. Movement must remain active and chemistry/invariant checks must remain valid. Disabling movement or reducing the workload does not satisfy acceptance.

## Matched throughput results

| Run | Ticks | Wall time | Throughput |
|---|---:|---:|---:|
| Movement disabled | 666 | 60.024 s | 11.10 ticks/s |
| Movement enabled | 157 | 60.192 s | 2.61 ticks/s |

Movement reduced throughput by **76.5%** (approximately **4.25× slower**) under the matched starting conditions. This is a material regression, not measurement noise to ignore.

## Movement profile

Averaged over 157 movement-enabled ticks:

- Resonance calculation: **201.2 ms/tick**
- Placement validation and commit: **59.2 ms/tick**
- Total movement pass: **260.5 ms/tick**
- Resonance pair loop: **197.3 ms/tick**, evaluating about **822,186 unordered pairs/tick**
- Successfully moved objects: about **514/tick**

The pair loop is the dominant cost. Placement validation is a secondary cost; its original-collision and mover-overlap checks account for most of that phase.

## Functional and conservation results

Movement-enabled 60-second run:

- 11 committed BREAK reactions
- 5 committed COMBINE reactions
- 1 committed DISPLACEMENT reaction
- Approximately 514 successful object translations per tick on average
- 1,309 molecule identities created; 1,286 active at end
- 0 lifecycle validation errors
- Final energy residual: approximately `-4.47e-7`

Matched movement-disabled baseline:

- 24 committed BREAK reactions
- 13 committed COMBINE reactions
- 4 committed DISPLACEMENT reactions
- 0 lifecycle validation errors
- Final energy residual: approximately `-5.57e-6`

Both runs retained all six active non-Water resources and completed the full 60-second observation. The movement-enabled run confirms that the integrated movement pass is active and chemistry continues to execute; it does not establish that movement improves net chemical productivity per wall-clock second.

## CI

- Rust CI / Clippy passed on final PR head `64be6159bb4cad0cc73518be0478014196957109`.
- [Rust CI run](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/44100044295)
- [Matched movement-enabled experiment](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/44099663816)
- [Matched movement-disabled baseline](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/44100857940)

## Conclusion and deferred work

The five-step movement integration sequence is functionally complete: resonance movement runs in the ordinary environmental tick, chemistry sees the resulting positions, and movement preserves object and molecular invariants.

Performance is **not yet acceptable at this scale**. A follow-up should target the exact pair-calculation loop and then remeasure placement validation. Preserve the current pairwise resonance law, same-phase repulsion, opposing-phase attraction, quadrature neutrality, distance attenuation, and existing placement transaction. Do not introduce a multipole hierarchy, generalized physics engine, persistent velocity/acceleration, or unrelated chemistry redesign.

The >800 ticks/minute target is not met by these measurements. Optimization is deliberately outside this validation/reporting change; the measured regression is recorded as the next engineering priority.
