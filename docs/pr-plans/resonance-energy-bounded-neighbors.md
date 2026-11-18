# Resonance Energy-Bounded Spatial Neighbor Discovery

## Purpose

Reduce the cost of environmental resonance movement by discovering only spatial interactions that can still affect the physical result. The optimization must use the existing harmonics/resonance signal and its established propagation physics; it must not introduce a new energy source, arbitrary radius, or numerical cutoff.

This work depends on the source-shape geometry foundation tracked separately in PR #378. The geometry PR supplies the physical boundary representation; this PR owns propagation-to-movement integration, effective support, spatial candidate discovery, and performance validation.

## Physical model and constraints

- A whole physical material (element or molecule) is the emitter. Its aura originates from the complete exposed perimeter of that material, not its centroid, bounding circle, or independently emitting constituents.
- Aura amplitude attenuates continuously with distance: `A(d) = A0 / (1 + d)`.
- Propagated energy is amplitude squared: `E(d) = E0 / (1 + d)^2`.
- The aura carries the same harmonics/resonance energy already represented by the existing simulation. It is not a new source, a second field, or a second energy store.
- The energy no longer carried by propagation becomes environmental heat exactly once. Receiver count and overlapping observations must not multiply the same propagation loss.
- Energy is allocated linearly over the complete exposed source perimeter: `dE = E_source × dℓ / L_source`. Receiver-visible portions use the complete source-global perimeter as the denominator; they are never renormalized to each receiver's visible subset.
- The aura extends outward from the complete source boundary by at most one radius of the emitting material. This is a source-specific physical maximum, not a world-wide interaction radius. Energy attenuation remains continuous inside that support; an energy-derived tighter support may reduce the candidate region but may never exceed this material-relative maximum.
- Do not add a global interaction radius, guessed energy threshold, machine-epsilon cutoff, floating-point-underflow rule, or independent decay coefficient.
- Preserve frequency, harmonic, phase, attraction/repulsion, equal-and-opposite interaction, deterministic accumulation, and movement/placement semantics unless an explicit physical-model decision authorizes a change.

## Current reference profile

The detailed movement profile recorded approximately 822,403 unordered emitter pairs per tick at 1,283 emitters. Median pair-loop time was approximately 210.92 ms/tick; the complete resonance movement phase was approximately 289.8 ms/tick. These are profiling observations, not guaranteed targets.

The current live movement path is centroid-based and evaluates every unordered noncoincident pair. It computes signed same-frequency phase-aware cross-energy, applies mass scales and continuous inverse-square distance attenuation, accumulates pair forces in canonical order, and then applies the existing nonlinear movement response.

The geometry-aware aura reception API is not yet the live movement-force authority. It must not be treated as proof that live movement already follows the complete source shape.

## Required source and energy audit

Before migrating live movement:

1. Trace the existing harmonics/resonance signal from material response through propagation, reception, pair interaction, and environmental heat settlement.
2. Preserve the distinction between source-backed world-tone incident/retained energy, normalized material-response spectra used by movement emitters, and signed pairwise cross-energy.
3. Establish how the same existing signal is attributed to a physical material and propagated without creating a second source or debiting the shared environment reservoir once per emitter.
4. Preserve tick causality. Movement currently runs before world-tone settlement; simply moving settlement earlier or consuming the shared reservoir later changes physical semantics and requires an explicit model decision.
5. Account propagation loss independently of receiver count. Signed pair cross-energy is an interaction term, not a nonnegative heat packet.
6. Establish the complete deterministic movement accumulation rule used to define physical effective zero. A per-pair epsilon or a bound that ignores contributions from other pairs is insufficient.

If the existing transaction cannot provide a conservative, source-attributed propagation path without changing physical semantics, stop and document the missing ownership relation rather than inventing one.

## Implementation phases

### Phase A — integrate the established source geometry

- Consume the complete exposed source perimeter supplied by PR #378.
- Ensure receiver visibility is derived from the source surface, not selected first from a centroid.
- Preserve exact straight and curved boundary measures; do not replace the source with a centroid, bounding circle, or tessellated approximation.
- Keep unsupported or ambiguous geometry fail-closed.

### Phase B — connect the existing signal conservatively

- Carry the same frequency-resolved, phase-aware harmonics/resonance signal through shape-based propagation.
- Allocate the finite source signal linearly over the source-global perimeter.
- Account the difference between incoming and outgoing propagated energy as environmental heat once, independent of receiver count.
- Preserve existing source totals and ledger invariants. Do not create a persistent second reservoir or count the same signal as both aura energy and a separate pair-energy source.

### Phase C — establish source-specific support

- Define each material's conservative radius from its movement center to the outermost realized constituent surface, including constituent shape extent.
- The aura may extend no farther than one additional source-material radius beyond that source boundary.
- For source radii `Ra` and `Rb`, a conservative center-distance candidate bound is `d <= max(2*Ra + Rb, 2*Rb + Ra)`. This includes either direction of source-to-receiver influence and uses no global radius.
- The current broadphase uses this maximum support. A tighter energy-derived bound is a later refinement only if it can be derived from the existing energy and movement model without an arbitrary threshold.

### Phase D — spatial candidate discovery

- Use a stable x-axis sweep-and-prune over the source-specific bounds; only pairs within the conservative pair bound become candidates.
- Use realized constituent shapes to derive conservative material extents; do not use a single global simulation radius.
- The candidate path retains parallel force evaluation for large candidate sets and reduces forces in canonical pair order. Set `EVOSIM_PROFILE_MOVEMENT=1` to profile the optimized candidate path; set `EVOSIM_REFERENCE_MOVEMENT_ALL_PAIRS=1` to run the original all-pairs reference path.
- Generate every eligible unordered pair exactly once and sort by stable `(a, b)` identity before force accumulation.
- Handle moving sources, world boundaries/wrapping, source extents, and degenerate/coincident positions.
- Keep the all-pairs implementation as a reference oracle. It must enumerate every unordered pair, apply the same source-specific support predicate, and preserve canonical accumulation. Validate candidate discovery against this exhaustive reference.

### Phase E — validation and performance

- Compare the candidate set against an exhaustive source-specific support oracle, then compare candidate forces and ordered accumulation against the same all-pairs reference filtered by that support rule.
- Test sparse and dense arrangements, varied source sizes, multiple harmonics, near-equal frequencies, occlusion, boundary cases, moving sources, and coincident positions.
- Verify deterministic output, no missed meaningful interactions, no duplicate pairs, and conservative energy/heat settlement.
- Run full tests, formatting, and Clippy.
- Run the standard 60-second, seed-42 chemistry experiment with no tick cap. Report possible pairs, discovered candidates, evaluated pairs, discovery time, pair-loop time, total movement time, throughput, object/constituent counts, and energy residual.
- Compare equivalent baseline and candidate runs; candidate-count reduction alone is not a performance result.

## Current status

- The movement bottleneck has been profiled.
- The exhaustive all-pairs reference remains available behind `EVOSIM_REFERENCE_MOVEMENT_ALL_PAIRS=1`; it enumerates every pair and applies the same source-specific support predicate before force evaluation.
- The normal live path now derives conservative source radii from realized constituent shapes and uses source-specific x-axis sweep-and-prune; large candidate sets retain deterministic parallel force evaluation.
- `EVOSIM_PROFILE_MOVEMENT=1` profiles the optimized path rather than silently reverting the simulation to all-pairs behavior.
- PR #378 contains the reusable exact source-geometry foundation; this PR's bounding radii are a broadphase only and do not replace exact source-perimeter reception.
- The current broadphase uses the full conservative one-source-radius maximum. A tighter energy-derived support bound and propagation heat settlement remain unimplemented.
- The independent resonance-field architecture is future work and is documented separately in `docs/design/resonance-field-architecture.md`.

## Acceptance criteria

- The same existing harmonics/resonance signal is used; no second source or store is introduced.
- Source-shape propagation and source-global perimeter allocation are respected.
- Propagation loss is accounted exactly once and the energy ledger remains conservative.
- Effective support is physically justified and proven against the complete movement calculation.
- No meaningful interaction is omitted and no unordered pair is duplicated.
- Deterministic accumulation and movement behavior are preserved or explicitly revalidated.
- The optimized path agrees with the reference calculation.
- Full CI passes and controlled experiments demonstrate a meaningful end-to-end movement improvement without chemistry or energy regressions.
