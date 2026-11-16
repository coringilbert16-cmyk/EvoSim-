# PR 2: Apply simple resonance movement to materials

**Status:** Planning outline; implementation not yet started.
**Base:** `main`
**Goal:** Turn the pairwise movement vectors from PR 1 into actual positional changes.

## Goal
Apply the simple resonance push/pull contributions to free environmental materials using direct positional movement.

## Outline
1. Accumulate all pair contributions for each material from PR 1.
2. Convert the combined response into a bounded positional displacement using one documented movement-response scale.
3. Preserve shape, orientation, composition, internal bonds, and material identity.
4. Reuse existing placement and geometry validation to reject overlap, invalid placement, and boundary violations.
5. Commit accepted positions through the existing field-placement mechanism.

## Explicit exclusions
No persistent velocity, acceleration, angular momentum, torque, moment of inertia, rigid-body integrator, kinetic-energy inventory, generic damping model, or new collision solver.

## Acceptance
- A material moves in the expected direction for isolated attraction and repulsion cases.
- Multiple pair contributions combine deterministically.
- Movement is bounded and stable.
- Existing geometry and placement invariants remain intact.
- Focused tests and full CI pass.
- Chemistry integration remains unchanged in this PR.

## Sequence
This is step 2 of 5. Complete and merge the preceding step before beginning implementation here. This PR must remain independently scoped to the objective above.
