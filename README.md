- no logical Water entering storage;
- intact composite material remaining intact while crossing a boundary;
- a high-solid/low-Water boundary remaining substantially resistant to passage;
- same topology with different composition producing different boundary behavior;
- same composition with different topology producing different boundary behavior.

This section is the authoritative reference for the interior/permeability work.



# Bond-Driven Construction Migration Plan

**Status: ACTIVE — authoritative implementation plan for the current construction migration.**

**Implementation status:** The Phase 1–7 architecture is implemented at the construction/reproduction layers. Phase 4's developmental caller audit is complete: the obsolete recursive whole-graph constructor is retired, `candidate_placements()` remains only for generic COMBINE/reproduction callers, and the developmental path is forward-only bond-driven. The current blocker is Phase 5 contract verification: the temporary three-Carbon scaffold must be physically framed around the blueprint cavity rather than around the boundary anchor, and the confirmed seed must provide enough physical room for that scaffold. Full contract verification remains pending.

This section establishes the target construction architecture and migration order. It exists specifically to prevent piecemeal fixes from silently creating competing construction authorities.

## A. Target construction contract

The authoritative developmental construction pipeline is:

`developmental blueprint preference → actual physical material selection → existing physical endpoint A → actual physical endpoint B → rotation of B around the selected joint → immediate geometry validation → exact one-bond transaction → permanent commit → next bond`

The blueprint is a **developmental preference**, not a command to reproduce an exact future physical graph. The realized physical structure is authoritative. A construction step may try different acceptable physical materials, endpoints, and orientations. A failed candidate does not undo an already committed bond. Once a bond is committed, it is never backtracked by construction. The constructor does not validate future bonds before committing the current bond.

## B. Material-selection authority

Construction must select from the organism's existing **physical material inventory**. The selector compares actual physical candidates against the blueprint's structural preference. It must return an actual physical material instance or storage entry, never manufacture a replacement from a catalog description, never consume a candidate while evaluating it, reject candidates below the approved structural-match threshold, permit acceptable substitutes when the preferred resource is unavailable, and return a construction/material need when no candidate is sufficiently close. Existing material preference, density, and connectivity fields should be reused.

## C. Exact construction-bond primitive

Construction requires a construction-specific low-level transaction taking the existing structure, actual materials A/B, exact endpoints A/B, exact pose for B, catalog/physical properties, and energy ledger/energy. It only verifies endpoints, exact contact, unintended penetration, universal bond-formation rules, energy/ledger settlement, and atomic addition of the supplied bond. It must not discover another endpoint/orientation, validate future bonds, or perform material selection. Generic COMBINE candidate discovery remains separate.

## D. Orientation and candidate search

For a selected endpoint pair, construction rotates B around the exact joint so B's selected endpoint remains at A's selected endpoint. The authoritative search is: for each acceptable material, endpoint pair, and orientation, place B at the exact joint, reject unintended penetration, attempt the exact construction bond, and stop only on success. Failed orientations/materials/endpoints continue the search. An `Option` failure from the exact-bond primitive must never escape the orientation loop as an early return.

## E. Geometry authority

Bond contact is not unintended overlap. Construction geometry must distinguish intended endpoint contact from unintended penetration into unrelated physical material. Irregular geometry and irregular cavities are valid. There is no 4N square/lattice requirement and declared blueprint poses are preferences, not commands. Water may participate in valid irregular construction where normal material-selection rules permit it.

## F. Genome measurement scaffold

The temporary genome measurement scaffold is exactly three Carbon resources arranged as an equilateral triangle with all three internal bonds:

`    C
    / \\
   C---C`

The scaffold is a measurement/construction aid, not organism material. Construction builds around it and never connects to it. After genome construction, the scaffold is removed conceptually, leaving the physical cavity it defined. Genome qualification is based on the realized physical structure/cavity.

## G. Construction state invariants

Every committed step preserves: no committed-bond backtracking; no future-bond validation; physical inventory identity; atomic material consumption; exact successful pose; exact successful endpoints; physical graph authority; and energy-ledger correctness. A failed candidate leaves structure, inventory, energy, and ledger unchanged.

## H. Migration order

### 1. Establish the exact-bond primitive
Separate construction's exact joint transaction from generic COMBINE candidate discovery. Add isolated tests for endpoint coincidence, intended boundary contact, unintended penetration, energy/ledger accounting, and atomic failure.

### 2. Integrate actual physical inventory
Make material selection return the actual stored physical instance and make both anchor and subsequent construction steps use that instance. Add tests proving distinctive stored geometry is the geometry constructed.

### 3. Simplify candidate search
Remove generic `candidate_placements()` from the authoritative bond-driven path. Enumerate endpoint pairs and rotate the actual material around the selected joint. Failed orientations/materials/endpoints continue the search.

### 4. Remove future lookahead
Audit and retire old recursive whole-graph construction behavior from the developmental path. Construction commits one bond at a time.

### 5. Enforce scaffold/cavity invariants
Add scaffold non-contact/non-penetration tests and irregular cavity tests. Remove remaining 4N/lattice assumptions from the authoritative construction path.

### 6. Integrate construction need
When no physical material sufficiently matches developmental preference, connect existing construction/acquisition behavior to that unmet need without inventing a large new need taxonomy.

### 7. Reproduction integration
Ensure offspring construction uses the same bond-driven physical construction authority rather than maintaining a separate geometry implementation.

## I. Required test matrix

### Exact bond
- exact endpoints succeed;
- separated endpoints fail;
- intended boundary contact succeeds;
- unintended penetration fails;
- successful transaction changes energy exactly once;
- failed transaction leaves structure/energy/ledger unchanged.

### Search
- later orientation succeeds after earlier failures;
- later endpoint succeeds after earlier endpoint failures;
- later acceptable material succeeds after earlier acceptable material geometry failure;
- no candidate failure aborts the search prematurely.

### Material preference
- exact preferred material is selected;
- acceptable structural substitute is selected;
- below-threshold material is rejected;
- no candidate produces a construction/material need;
- actual physical storage instance is used.

### Forward-only construction
- committed bonds are never undone;
- future closure failure does not invalidate an already committed bond;
- later construction may use a different endpoint/material to close a gap;
- no future bond is prevalidated.

### Scaffold/cavity
- scaffold is triangular and equilateral;
- scaffold has exactly three internal bonds;
- scaffold cannot be bonded to;
- scaffold cannot be penetrated;
- irregular cavities qualify when physically valid;
- no 4N/lattice assumption is required.

### Inventory
- failed construction does not consume material;
- successful construction consumes exactly the selected physical instance;
- alternate candidate consumption is correct;
- composite physical material retains its internal geometry/bonds.

## J. Legacy construction code

Existing helpers are not deleted merely because they are old. Before removal, audit all callers. Helpers needed by generic COMBINE, material restoration, or unrelated physical operations may remain. Helpers whose only purpose is the obsolete developmental whole-graph construction model must be retired after the bond-driven path is proven by contract tests. The bond-driven constructor becomes the sole authoritative developmental construction path.

## K. Verification gate

After each migration phase: format; compile; run focused construction tests; inspect failures by contract layer; only then proceed. Do not modify unrelated geometry, energy, movement, reproduction, or biological behavior to make a construction test pass unless a direct dependency is demonstrated. Long simulation runs happen only after focused construction contract tests are green.

**Current task:** Phase 4 caller audit is complete. Continue at Phase 5: enforce the triangular scaffold/cavity invariants, verify that the scaffold is framed in blueprint space rather than centered on a boundary anchor, and prove the confirmed seed can be physically realized without backtracking. Do not add geometry heuristics to the generic COMBINE path.