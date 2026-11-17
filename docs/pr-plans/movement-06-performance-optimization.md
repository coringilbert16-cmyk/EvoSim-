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
