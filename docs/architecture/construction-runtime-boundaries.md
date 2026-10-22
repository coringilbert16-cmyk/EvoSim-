# Construction runtime responsibility-boundary audit — 2026-10-22

## Scope and checkpoint

Audited `src/construction/runtime.rs` on `main` at source blob `663b4ffeb6d70e555cd2ab3a5102d676346bc1b0` (post-PR #202 main, 3,165 lines / approximately 126 KB). This is an architecture audit only; no constructor or construction-runtime code was changed. PR #196 remains separate.

## Responsibility clusters found

1. **Placement and geometry conversion** — `resource`, `placement`, `candidate_placements`, joint endpoint helpers, angle normalization/candidate generation, and form/point overlap routines.
2. **Canonical live-parent mapping into Bob's frame** — `CanonicalLiveRigidParent`, `canonical_live_rigid_parent`, and `bob_fluid_origins_for_live_parent`. This cluster maps stable live constituent IDs to canonical Bob slots and retrieves persisted geometry suggestions.
3. **Multi-contact bond formation** — `form_remaining_multi_contact_bonds` and its in-place implementation. This is transaction-sensitive: all required live contacts must be committed consistently rather than partially.
4. **Physical material attachment** — `physical_material_endpoint_options`, `try_attach_physical_material_bond_driven`, and associated contact/endpoint selection.
5. **Bond-driven blueprint construction** — `realize_next_bond_driven`, placement rejection accounting, and the public(crate) blueprint construction entry points through `construct_blueprint_bond_driven_internal`.

The file is not just one large algorithm. It combines geometric candidate generation, live-graph-to-library mapping, transactional bond logic, and the higher-level construction loop.

## Cross-module coupling observed

- `candidate_placements` is called by reproduction, COMBINE runtime, and bond-driven contract tests. Any extraction must preserve the current crate-level callable path or update all callers in the same change.
- `try_attach_physical_material_bond_driven` is called by reproduction; its energy and realized-geometry outputs are part of that caller's construction flow.
- `placed_unit_overlaps` is used by COMBINE runtime and contract tests, so geometry validation cannot simply become private to a new construction submodule.
- The Bob bridge depends on the existing geometry-library API and stable-ID/canonical-slot mapping. Keep it separate from generic candidate generation only if the shared parent/placement types can remain clear and cycle-free.
- The multi-contact bond functions are exercised by in-file tests and are guarded by existing fluid transaction tests. They should not be moved or rewritten as part of a generic organization sweep.
- The in-file tests occupy the final ~1,070 lines and exercise construction, Bob transformation/restoration, multi-contact fluid bonds, and overlap rejection. Moving implementation code while leaving tests behind may be possible, but the private helper visibility must be audited explicitly.

## Recommended extraction order, if approved later

1. Extract only pure geometry helpers (point/triangle tests, world vertices, point-in-form, overlap checks) into a focused construction-geometry module after auditing the exact shared type dependencies. Preserve `placed_unit_overlaps` as a stable crate-level entry point.
2. Extract the live-parent/Bob bridge as a separate module only after defining the ownership of `CanonicalLiveRigidParent` and its canonical pose/ID mapping. Keep runtime collision/contact checks authoritative; Bob suggestions remain candidate geometry, not truth.
3. Consider extracting physical-material attachment after mapping every caller and energy/ledger interaction.
4. Leave multi-contact transaction logic and the bond-driven construction loop for last. They are behaviorally coupled and have the highest regression risk.

A single large file move is not recommended. Prefer one responsibility per change, run focused bond-driven and fluid transaction tests, then the full suite and compare the 28 known baseline failure identities.

## Clippy implications

Do not use this audit to justify deleting constructor code or changing constructor behavior. Constructor-related lint work should be kept separate from structural extraction, and any staged API must be traced through current and intended runtime roles before removal.
