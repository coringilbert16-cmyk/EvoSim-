# PR 4: Validate movement invariants and safe placement

**Status:** Focused invariant tests added; full CI validation in progress.
**Base:** `main`
**Goal:** Complete focused correctness validation for simple movement and its interaction with existing field invariants.

## Goal
Prove that the simple resonance movement implementation is safe, deterministic, and preserves existing simulation invariants.

## Implemented validation
1. Reject out-of-bounds translations without changing field cells or revisions.
2. Reject movement into occupied geometry without changing field cells or revisions.
3. Move a bonded Carbon–Hydrogen composite and verify identity, composition, internal physical connections, realized shapes, orientation, field-cell membership, and total material inventory are preserved.

## Remaining validation
- Add any additional focused cases exposed by CI or review.
- Run the full focused and repository CI suite.

## Explicit exclusions
No new global energy model, kinetic-energy ledger, force/torque conservation framework, generalized transaction engine, or elaborate numerical oracle. No changes to movement physics or chemistry rules.

## Sequence
This is step 4 of 5. Complete and merge the preceding step before beginning implementation here. This PR remains independently scoped to the objective above.
