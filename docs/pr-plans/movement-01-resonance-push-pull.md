# PR 1: Add simple resonance-based material push/pull

**Status:** Planning outline; implementation not yet started.
**Base:** `main`
**Goal:** Calculate a simple pairwise push/pull response from the existing resonance spectra.

## Goal
Implement the minimal pairwise resonance interaction calculation. This PR establishes whether two free environmental materials attract, repel, or have no net movement response based on their existing frequency-resolved resonance.

## Outline
1. Reuse the existing environmental material resonance/emitter preparation and aura attenuation.
2. Compare matching frequency components while preserving amplitude and phase.
3. Derive the signed interaction from the existing phase-dependent resonance potential: constructive/in-phase is repulsive, opposing/destructive is attractive, and quadrature is neutral.
4. Calculate one deterministic pair movement vector and apply equal-and-opposite contributions.
5. Keep this as a pure calculation; do not move materials or change chemistry in this PR.

## Explicit exclusions
No Newtonian state, persistent velocity, acceleration, torque, inertia, multipoles, adaptive trees, error-bound hierarchy, new aura, arbitrary interaction radius, new energy reservoir, or chemistry changes.

## Acceptance
- Same-phase pair repels.
- Opposing-phase pair attracts.
- Quadrature pair is neutral.
- Response weakens continuously with existing distance attenuation.
- Pair calculation is symmetric and counted once.
- Focused tests and full CI pass.
- No runtime movement is activated yet.

## Sequence
This is step 1 of 5. Complete and merge the preceding step before beginning implementation here. This PR must remain independently scoped to the objective above.
