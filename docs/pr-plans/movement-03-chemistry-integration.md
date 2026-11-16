# PR 3: Integrate resonance movement with environmental chemistry

**Status:** Implementation in progress.
**Base:** `main`
**Goal:** Run one simple resonance-driven movement pass in the normal environmental tick without replacing chemistry rules.

## Implementation
1. Precompute each field material's existing environmental resonance once per tick.
2. Evaluate each unordered pair once using same-frequency phasor cross-energy and the existing continuous aura attenuation.
3. Accumulate equal-and-opposite pair contributions and convert each material's net response into a bounded direct displacement.
4. Apply accepted displacements through the existing whole-object field placement transaction.
5. Run movement after ambient transport and before environmental reaction discovery, so chemistry sees the updated contacts in the same tick.

## Invariants
- Existing COMBINE, BREAK, activation, bond-strength, and energy eligibility remain unchanged.
- Materials retain identity, composition, realized shapes, orientation, and internal bonds.
- Invalid, overlapping, or out-of-bounds placements are rejected by the existing field placement check.
- Movement executes once per environmental tick and does not create a separate chemistry path.

## Explicit exclusions
No persistent velocity, acceleration, torque, inertia, kinetic-energy ledger, new collision solver, new aura, organism movement, or chemistry redesign.

## Acceptance
- Same-phase materials repel, opposing-phase materials attract, quadrature/different-frequency pairs have no cross-term.
- Distance attenuation is continuous and uses the existing `1 / (1 + distance)` aura law.
- Pair contributions are symmetric and evaluated once.
- Movement is bounded and preserves field integrity.
- Movement changes proximity before ordinary chemistry discovery.
- Focused tests and full CI pass.
- Standard 60-second, seed-42, 1,280-object performance is checked before merge.

**Sequence:** Step 3 of 5.