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
- The aura has a finite effective support only where the complete movement calculation establishes that the received contribution can no longer produce a physically meaningful change. This is an exact-zero physical rule, not machine epsilon, floating-point underflow, or a guessed threshold.
- Do not add an arbitrary interaction radius, fixed distance cap, or independent decay coefficient.
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

### Phase C — derive effective support

- Derive the effective-zero boundary from the complete movement accumulation and final state update.
- Prove that omitted contributions cannot change the resulting physical movement state.
- Do not use a guessed radius, arbitrary energy threshold, machine epsilon, or floating-point underflow as the physical rule.
- If a conservative finite support cannot be proven, retain the all-pairs path as the correctness authority and do not enable spatial omission.

### Phase D — spatial candidate discovery

- Build conservative spatial bounds from the proven effective support and complete source geometry.
- Use a uniform spatial grid if it provides a complete and efficient candidate set; use a spatial tree only if needed.
- Generate every eligible unordered pair exactly once using stable identities.
- Handle moving sources, world boundaries/wrapping, source extents, and degenerate/coincident positions.
- Preserve canonical pair order before floating-point force accumulation.
- Keep the all-pairs implementation available as a test oracle and safe fallback.

### Phase E — validation and performance

- Compare candidate pairs, propagated signal, forces, and final movement against the all-pairs/reference calculation.
- Test sparse and dense arrangements, varied source sizes, multiple harmonics, near-equal frequencies, occlusion, boundary cases, moving sources, and coincident positions.
- Verify deterministic output, no missed meaningful interactions, no duplicate pairs, and conservative energy/heat settlement.
- Run full tests, formatting, and Clippy.
- Run the standard 60-second, seed-42 chemistry experiment with no tick cap. Report possible pairs, discovered candidates, evaluated pairs, discovery time, pair-loop time, total movement time, throughput, object/constituent counts, and energy residual.
- Compare equivalent baseline and candidate runs; candidate-count reduction alone is not a performance result.

## Current status

- The movement bottleneck has been profiled.
- The current all-pairs implementation remains unchanged and authoritative.
- PR #378 contains the reusable source-geometry foundation.
- No finite cutoff, energy debit, propagation-heat settlement, live movement migration, or spatial neighbor pruning has been implemented.
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
