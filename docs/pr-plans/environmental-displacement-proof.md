# Environmental displacement proof — implementation plan

## Goal
Establish whether the existing environmental chemistry model can perform a genuine, causally driven replacement reaction: an incoming constituent C participates in replacing an existing bonded constituent B in A–B, producing A–C and a physically separate B.

This is a proof within EvoSim's current simplified chemistry model, not a claim of complete real-world chemical thermodynamics.

## Scope
- Add a coupled three-participant environmental reaction evaluation for A–B + C -> A–C + B.
- Reuse existing physical-contact validation, chemical interaction/activation, bond formation, structural-energy accounting, energy settlement, and graph identity utilities wherever their contracts permit.
- Evaluate the replacement on trial state only. Temporarily freeing the old bond endpoint must happen in the trial graph before discovering/validating the proposed new bond.
- Evaluate old-bond disruption and new-bond formation as one net reaction; do not run separate committed BREAK and COMBINE operations or double-count work/energy.
- Export resulting connected components as distinct physical products while preserving constituent identities, geometry, and internal bond-energy allocations.
- Commit atomically only after all geometry, chemistry, energy, and product-export checks succeed.

## Diagnostic cases
1. Favorable replacement: A–B exists; C physically contacts A; modeled A–C replacement is energetically preferable; sufficient incoming interaction/activation and valid energy settlement permit replacement.
2. Unfavorable replacement: equivalent physical setup, but A–C is energetically worse; reject unless the existing energy model identifies a legitimate funded pathway.
3. Insufficient incoming influence: A–C is favorable, but C's accumulated interaction has not overcome the old-bond disruption barrier; reject without mutation.
4. Failure/rollback: force a late validation or settlement failure and prove original graph, materials, and ledgers remain unchanged.

Use catalog-backed materials where the existing equations yield a meaningful favorable/unfavorable contrast; do not add a displacement bonus, arbitrary efficiency multiplier, or special-case catalog rule merely to make a test pass.

## Invariants and acceptance criteria
- No constituent is lost, duplicated, or fractionally split.
- Product connected components and bond topology match the intended reaction.
- Old and new bond energies are each accounted for exactly once.
- Usable energy, structural potential, resonance, physical work, and heat reconcile under the existing ledger contracts.
- A failed reaction leaves the original state unchanged.
- Incoming C must contribute to activation; an unrelated environmental BREAK followed by an independent COMBINE does not satisfy the proof.
- Existing environmental BREAK and COMBINE behavior remains unchanged.
- Focused tests, relevant chemistry test suites, full Rust tests, formatting, and Clippy pass.

## Implementation sequence
1. Reconfirm current APIs and identify reusable transactional boundaries.
2. Add focused evaluator/transaction primitives with no change to default reaction behavior.
3. Add product-component export if existing BREAK conversion cannot safely be reused.
4. Add the three diagnostic cases plus conservation, ledger, and rollback assertions.
5. Run focused tests first, then broader chemistry tests, full test suite, formatting, and Clippy.
6. Review the diff for scope, unintended behavior changes, and duplicate energy settlement.

## Explicit non-goals
- Redesigning the chemical-position interaction equation or claiming physical validation against real chemistry.
- Organism movement, organism-specific chemistry, genome behavior, or reproduction.
- Broad refactoring of BREAK/COMBINE or unrelated optimization work.
- Merging without green checks and review of the resulting behavior.

## Stop conditions
If the current pairwise interaction/activation model cannot distinguish favorable from unfavorable replacement without inventing a new arbitrary bonus, stop and report the evidence rather than weakening the criteria. If the energy ledger cannot settle the complete reaction atomically using existing semantics, stop and propose the smallest justified model change before proceeding.
