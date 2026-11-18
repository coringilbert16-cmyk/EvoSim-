# Resonance Movement Optimization

## Purpose

Reduce the computational cost of the existing live environmental resonance movement calculation without changing its physical model.

## Current reference behavior

- Each physical material produces one movement emitter using the existing centroid position, mass scale, and frequency/phase spectrum.
- Every unordered, noncoincident material pair is evaluated.
- Pair interaction uses the existing same-frequency phase-aware cross-energy.
- Pair strength uses the existing continuous inverse-square distance attenuation.
- Each pair contributes equal-and-opposite radial force.
- Pair contributions are accumulated in canonical lexicographic order before the existing bounded movement response is applied.
- The all-pairs calculation remains the correctness authority unless an exact optimization proves that omitted pairs contribute exactly zero under the current rules.

## Measured reference profile

The detailed movement profile recorded 822,403 unordered pairs per tick. Median pair-loop time was approximately 210.92 ms/tick; the complete resonance movement phase was approximately 289.8 ms/tick.

The sampled profiler estimated substantial work in both cross-energy calculation and frequency matching. Those estimates overlap and are not additive. The measured pair-loop duration is the reliable timing.

## Constraints

- Do not change the source or ownership of resonance energy.
- Do not change the current spectrum, harmonic, phase, attenuation, pair-force, or nonlinear movement-response rules.
- Do not add an arbitrary radius, cutoff, floating-point epsilon threshold, or guessed physical effective-zero rule.
- Preserve deterministic pair ordering and accumulation behavior.
- Do not introduce shape-aware perimeter geometry into this PR; that work is tracked in PR #378.
- Do not implement the independent resonance-field architecture here; its future design is documented in docs/design/resonance-field-architecture.md.

## Optimization approach

Investigate exact reductions in per-pair computation first. Any candidate-discovery optimization must prove that every omitted pair contributes exactly zero under the existing movement rules, retain canonical ordering, and be checked against the all-pairs reference.

Benchmark on the same standard environment and compare both pair-loop cost and end-to-end tick throughput. A lower candidate count alone is not sufficient evidence of improvement.

## Validation

- Focused unit tests for any new exact pair calculation or candidate discovery.
- Compare optimized output against the all-pairs reference, including deterministic ordering and edge cases such as coincident positions, empty spectra, multiple harmonics, and near-equal frequencies.
- Run the PR's focused CI and a controlled performance comparison before accepting the optimization.
