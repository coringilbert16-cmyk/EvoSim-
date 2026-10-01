- no logical Water entering storage;
- intact composite material remaining intact while crossing a boundary;
- a high-solid/low-Water boundary remaining substantially resistant to passage;
- same topology with different composition producing different boundary behavior;
- same composition with different topology producing different boundary behavior.

This section is the authoritative reference for the interior/permeability work.



# Bond-Driven Construction Migration Plan

**Status: ACTIVE — current authoritative implementation plan.**

**Current position (2026-10-01):** The constructor architecture has been migrated to the approved forward-only, bond-driven model. Actual physical material selection is integrated. Developmental construction and ordinary COMBINE share the same physical bond-formation authority. Realized structure is authoritative for cavity qualification, and harmonic environmental sensing uses realized physical material rather than logical material. The current work is **focused validation of the realized cavity → harmonic reception boundary and cleanup of retired constructor/harmonic paths**, not a redesign.

Recent implementation work has included:
- repairing stale constructor/scaffold call sites after the API migration;
- completing shared COMBINE/bond-formation integration;
- implementing deterministic forward candidate selection;
- retaining the globally nearest valid construction pose before committing;
- typing and simplifying forward-constructor candidate-selection state;
- pruning physically expensive candidates without changing the nearest-pose rule;
- scoping that pruning to the forward constructor so unrelated physical attachment behavior is not changed;
- calibrating the seed ring so its Carbon edges are physically sealed under the actual geometry rules;
- formatting and validating each of these changes incrementally.

## A. Authoritative construction contract

The developmental construction pipeline is:

`developmental blueprint preference → actual physical material selection → existing physical endpoint A → actual physical endpoint B → rotation of B around the selected joint → immediate geometry validation → exact one-bond transaction → permanent commit → next bond`

The blueprint is a **developmental preference**, not a command to reproduce an exact future physical graph. The realized physical structure is authoritative.

A construction step may evaluate acceptable physical materials, endpoints, and orientations. A failed candidate does not undo an already committed bond. Construction does not prevalidate future bonds and does not backtrack previously committed construction. If no usable physical material is currently available, construction enters a pending/waiting action state and resumes when material becomes available; it never blocks the simulation thread.

## B. Physical-material authority

Construction selects from the organism's existing **physical material inventory**.

The selector must:
- evaluate actual physical candidates against the developmental structural preference;
- preserve the identity and internal geometry of the selected physical material;
- never manufacture a replacement from a catalog description;
- never consume candidates during evaluation;
- reject candidates below the approved structural-match threshold;
- permit acceptable physical substitutes when the preferred resource is unavailable;
- report a material/construction need when no candidate is acceptable.

Composite physical material remains composite. Its internal structure is not flattened merely to satisfy a blueprint preference.

## C. Shared physical bond authority

Generic COMBINE and developmental construction use the same physical bond-formation transaction.

The shared transaction is responsible for:
- exact endpoint/contact validation;
- unintended-penetration rejection;
- universal physical bond rules;
- energy and ledger settlement;
- atomic bond addition.

Candidate discovery, developmental material preference, and orientation search remain caller responsibilities.

**There must be only one physical rule for whether a bond can be formed.** Construction must not acquire a special geometry/bonding path that ordinary COMBINE does not use.

## D. Forward candidate search

For each acceptable physical material, endpoint pair, and orientation, construction:
1. places the selected physical material at the exact joint;
2. keeps the selected endpoint coincident with the existing endpoint;
3. validates unintended penetration;
4. evaluates the candidate;
5. retains the best valid candidate according to the approved deterministic selection rule;
6. commits exactly one bond through the shared physical transaction.

Candidate failures continue the search. An unsuccessful candidate must not escape the search loop as an early return.

Performance pruning is allowed only when it is provably safe with respect to the approved candidate-selection rule. It must remain scoped to the constructor unless a separate contract authorizes a broader physical change.

## E. Geometry authority

Intended bond contact is not unintended overlap.

The geometry system must distinguish:
- the two endpoints intentionally meeting at a bond;
- unrelated material being penetrated.

Irregular structures and irregular cavities are valid. There is no universal 4N square/lattice requirement. Declared blueprint poses are preferences rather than commands.

Water and composite materials must obey the same physical admission rules as other material; they do not receive a special logical-material shortcut.

## F. Genome measurement scaffold and seed ring

The temporary genome measurement scaffold is a construction/measurement aid, not organism material. It must not become a hidden second construction authority.

The seed geometry must be physically realizable under the actual bond and penetration rules. The current seed-ring calibration specifically ensures that the Carbon boundary edges are physically sealed rather than merely appearing enclosed in blueprint space.

Cavity qualification is based on the **realized physical graph and realized geometry**, not on a predefined core, named cavity, or blueprint declaration.

## G. Realized cavity and harmonics

The dependency is:

`construction → realized physical graph → realized cavity qualification → harmonic environmental reception`

A cavity exists for these purposes only when the realized physical structure satisfies the cavity contract.

Harmonics must operate on realized physical material at the qualifying realized boundary. Logical material must not be introduced into the cell merely to make harmonic tests or perception succeed.

This dependency is why cavity and harmonics failures can legitimately appear while the constructor migration is still being validated.

## H. What is complete vs. what is currently active

### Implemented architecture
- forward-only developmental construction;
- actual physical-material selection;
- shared construction/COMBINE physical bond transaction;
- deterministic candidate evaluation/selection;
- physical overlap/penetration validation;
- realized-structure authority;
- realized cavity qualification;
- physical-only harmonic environmental material;
- seed-ring physical calibration.

### Current active work
- focused constructor and realized-structure contract validation;
- correcting genuine implementation defects exposed by those tests;
- preserving constructor performance without changing its selection semantics;
- separating expected downstream test fallout from unrelated regressions;
- migrating dependent tests only after the underlying contract is confirmed.

### Not currently the task
- redesigning the constructor from scratch;
- restoring placement-authoritative construction;
- adding a second construction-specific bond/geometry authority;
- changing unrelated movement, energy, reproduction, or biological behavior merely to make downstream tests green;
- performing long whole-simulation runs before the focused construction contracts are stable.

## I. Failure classification

Current red tests must be classified before changing code.

### Expected migration fallout
A test may still encode an obsolete placement-authoritative, predefined-cavity, logical-material, or other retired assumption.

These failures should be migrated to the new contract **after the implementation contract they depend on is proven**.

### Genuine implementation defect
The current code violates an approved physical or architectural rule.

These must be fixed in the implementation.

### Accidental collateral change
A recent patch changes behavior outside its intended contract, such as an unrelated physical attachment path or an unrelated helper.

These must be fixed immediately and isolated from the architectural migration.

**Do not treat all cascading failures as one category.**

## J. Required focused test matrix

### Exact bond
- exact endpoints succeed;
- separated endpoints fail;
- intended boundary contact succeeds;
- unintended penetration fails;
- successful transaction changes energy exactly once;
- failed transaction leaves structure, inventory, energy, and ledger unchanged.

### Candidate search
- later orientation succeeds after earlier failures;
- later endpoint succeeds after earlier endpoint failures;
- later acceptable material succeeds after earlier geometry failure;
- no candidate failure aborts the search prematurely;
- best-candidate selection remains deterministic;
- safe pruning does not change the selected valid candidate.

### Material preference
- preferred physical material is selected when valid;
- acceptable structural substitutes are selected when appropriate;
- below-threshold material is rejected;
- no acceptable candidate produces a construction/material need;
- the actual physical inventory instance is used.

### Forward-only construction
- committed bonds are never undone;
- future closure failure does not invalidate earlier committed bonds;
- later construction can use another valid endpoint/material;
- future bonds are never prevalidated.

### Geometry/cavity
- seed-ring geometry is physically sealed;
- irregular cavities can qualify;
- no predefined core is required;
- no 4N/lattice assumption is required;
- scaffold/measurement geometry cannot become an unintended bond target.

### Inventory
- failed construction consumes nothing;
- successful construction consumes exactly the selected physical instance;
- alternate-candidate consumption is correct;
- composite material retains its internal geometry and bonds.

### Harmonics
- Environmental atomic, composite, and future organic material emits one resonance spectrum as a whole realized material.
- Organisms are deaf to their own emitted tone; organism material is the receiver, not an environmental self-source.
- Each physical resource forming the realized genome-cavity boundary receives environmental resonance independently.
- Receiver geometry is the antenna: actual realized cavity-boundary segments provide directional coupling.
- Environmental source position is preserved in each received channel; channels remain separate through perception so directional information is not averaged away.
- Distance attenuates the resonance aura continuously; there is no hard perception radius.
- The genome-cavity aggregate is derived only from those physical directional receptions. Logical environmental material is never injected as a sensory substitute.
- Current implementation uses the existing realized physical material inventory and realized cavity geometry; no separate sensory organs, sensor radius, or second perception grid is introduced.

- reception depends on a qualifying realized cavity;
- environmental material is received from the realized physical boundary;
- logical material is not injected into the organism to satisfy perception.

## K. Migration order from this point

The remaining work proceeds in this order:

1. **Finish focused constructor contract validation.**
2. **Fix any genuine constructor/physical-bond defects found by those tests.**
3. **Verify the seed-ring/realized-cavity contract end-to-end.**
4. **Verify harmonics against the realized cavity and physical material.**
5. **Classify the remaining suite failures by contract layer.**
6. **Migrate stale downstream tests without weakening the new architecture.**
7. **Only after those layers are stable, proceed through reproduction/lifecycle tests and broader simulation validation.**

This order is intentional. Downstream failures may cascade from an unfinished upstream contract, but they must not be allowed to dictate a return to the obsolete architecture.

## L. Verification gate

For every implementation change:

1. inspect the exact contract being changed;
2. make the smallest isolated change;
3. run `cargo fmt --all`;
4. compile/check;
5. run the focused tests for that contract;
6. inspect the failures by dependency layer;
7. only then broaden validation.

Long simulation runs are appropriate after the focused physical contracts are stable. They are not substitutes for unit/contract validation.

**Current task:** Continue from the realized cavity → harmonic reception validation boundary. The constructor architecture is established and the retired constructor scaffolding has been removed. The goal now is to prove directional harmonic reception, correct genuine defects without introducing unrelated behavior changes, then classify downstream test fallout and remove remaining obsolete paths.

**Non-negotiable:** Do not introduce parallel construction authorities, restore placement-authoritative construction, or add unrelated geometry heuristics merely to satisfy downstream tests.
