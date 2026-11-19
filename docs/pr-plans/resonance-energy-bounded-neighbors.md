# Resonance movement resolution-bounded neighbor discovery

## Purpose

Reduce environmental resonance movement cost by discovering only material pairs whose existing interaction can produce a meaningful per-pair movement response.

This PR changes only environmental movement candidate discovery. It does not implement the source-perimeter geometry foundation in PR #378, change the resonance propagation field, add a new energy source/store, or alter energy settlement.

## Existing movement law

For an unordered pair, the current movement calculation:

1. Compares matching frequency components while retaining their relative phase.
2. Calculates signed cross-energy from the phase-aware spectra.
3. Applies each material's existing mass scale.
4. Applies the existing continuous inverse-square attenuation:
   `strength(d) = N_ab / (1 + d)^2`,
   where `N_ab = abs(cross_energy_ab) * mass_scale_a * mass_scale_b`.
5. Accumulates equal-and-opposite pair contributions and applies the existing bounded movement response.

The propagation/attenuation law itself remains continuous. The optimization defines a finite resolution for whether an individual pair is worth evaluating; it does not claim the underlying field becomes physically zero at that distance.

## Explicit movement-resolution choice

The minimum meaningful displacement from one isolated pair is **0.001 world units**. Hydrogen's constituent width is 0.1 world units, so this is 1% of that reference feature.

The existing response is:

`r = 0.1 s / (1 + 0.1 s)`

For `r_min = 0.001`, the minimum pair interaction strength is:

`s_min = r_min / (0.1 (1 - r_min))`

The pair's maximum center separation at that resolution is:

`d_max = sqrt(N_ab / s_min) - 1`

If `d_max` is negative, the pair never reaches the configured per-pair resolution, even at zero separation.

This is an explicit model-resolution parameter, not a mathematically derived zero and not a floating-point epsilon. It is intentionally applied per pair. Multiple individually sub-threshold contributions that would otherwise add in the same direction are omitted; the matched experiment must determine whether this resolution choice has an acceptable effect on behavior.

## Efficient candidate discovery

Each emitter carries a conservative amplitude-norm bound including its existing mass scale. Cauchy-Schwarz provides a conservative maximum pair-strength bound for the x-axis sweep. Each broadphase candidate then receives an exact frequency- and phase-aware support check using its actual cross-energy and mass scales.

The candidate set is canonical and contains each eligible unordered pair once. The exhaustive reference enumerates all unordered pairs and applies the identical support predicate. Both paths use the same force evaluator and can be run sequentially for a matched comparison.

## Scope boundaries

- Preserve the existing frequency matching, harmonics, phase, mass scaling, inverse-square attenuation, signed pair force, and bounded movement response for retained pairs.
- Do not add a global interaction radius, source-radius extension, separate decay coefficient, resonance field, energy reservoir, or energy source.
- Do not change propagation-energy debit, heat settlement, chemistry, or organism movement.
- Keep PR #378 source-geometry work separate. This PR does not claim movement already uses exact source-perimeter reception.
- Do not change the independent resonance-field architecture.

## Validation

Focused tests cover:
- Conversion from the configured displacement floor to minimum interaction strength.
- The exact inverse-square support boundary and just-outside rejection.
- Pair-specific amplitude, mass, frequency, and phase dependence.
- Sweep-and-prune equivalence to exhaustive same-cutoff discovery.
- Sequential/parallel force ordering and identical retained-pair force results.

The PR workflow compares spatial discovery with exhaustive discovery using the same code, same cutoff, same sequential force evaluator, and the standard 60-second, seed-42, 1,280-material workload. Report candidate counts, discovery and pair-evaluation time, throughput, final material/constituent counts, and energy residual. The comparison validates the optimization against the same cutoff; it does not compare the new resolution choice against the former one-radius model.

## Current status

- The provisional one-source-radius limit and its geometry-radius dependency have been removed from the movement candidate path.
- The explicit 0.001-unit per-pair movement-resolution threshold is implemented.
- The exhaustive same-cutoff reference and matched benchmark workflow are updated.
- Full CI and the matched 60-second comparison remain required before acceptance.
