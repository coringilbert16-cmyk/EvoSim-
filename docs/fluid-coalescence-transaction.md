# Fluid-to-fluid coalescence transaction

Status: bounded exact-interface path focused-test verified; full workflow remains red, 2026-10-12.

**Latest clean-stack verification:** run [38266873667](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38266873667) on rebased head `c311cda524d1331a6bd779680412293eb303543a` passed all focused gates, formatting, source-size, and architecture checks. Full suite: **398 passed, 29 failed, 1 ignored**; the additional failure relative to the prior run is `movement::tests::blocked_zero_move_and_nonfinite_move_are_rejected`, which failed because the test could not find the persistent Bob geometry library. Strict Clippy remains red on repository-wide lint findings. This is not a green overall workflow.

The current implementation supports a conservative subset of fluid-to-fluid COMBINE: two same-resource, single-part fluids with valid exact multi-contact boundaries sharing one complete reversed solid-contact segment. The runtime forms the bond on isolated trial state, requires both fluid-bond endpoints to lie on and coincide across the exact shared segment, coalesces the pair, remaps retained external fluid endpoints, removes the consumed internal bond, validates the staged graph, and commits only on success. Focused exact-head tests pass; general arbitrary-boundary union, mixed-resource fusion, and full-workflow health remain open.

## Required behavior

- Touching is not connectivity. Proximity and collision queries must never merge fluids or create bonds.
- A fluid-fluid bond may be committed only as part of the same transaction that coalesces the two fluid constituents into one conserved fluid body.
- The merged nominal area is the sum of the input areas. If both constituents are the same resource, the surviving material amount is the sum of their amounts; no energy or material may be created by geometry realization.
- The merge must preserve every pre-existing valid bond from either fluid constituent to rigid constituents. Their fluid-side endpoints must be resolved against the new boundary, not blindly copied or radially projected.
- The internal fluid-fluid bond is consumed by coalescence; it must not remain as a graph edge between two separate fluid units.
- The operation is all-or-nothing across structure, constituent identity, material amounts, realized geometry, bond endpoints, usable energy, and the energy ledger. A failed late validation must leave the caller's structure and energy unchanged.
- Never approximate exact circular arcs with sampled polygons. Unsupported topology or ambiguous contact remapping must fail closed.

## Source audit findings

At the earlier base checkpoint `b64825a9`, the source audit found no fluid-fluid coalescence stage. That finding is superseded by the implementation now present on `fix/fluid-coalescence-transaction-2026-10-12`:

1. `combine_pair_in_direction` and the environmental/stored-material COMBINE paths call `coalesce_fluid_pair_after_bond` after forming the bond on trial state.
2. The helper requires an actual bond between the pair, validates supported same-resource material amounts and exact multi-contact boundaries, invokes placement-aware geometry union, remaps retained fluid-side endpoints, removes the internal fluid-fluid bond, preserves the survivor physical ID, updates genome IDs, validates retained bonds, and only then replaces the trial structure.
3. The callers stage structure, usable energy, and ledger changes in isolated trial values and discard them if coalescence rejects the candidate. Environmental storage consumption is performed only after the hypothetical structure passes coalescence.
4. `src/geometry/capillary_geometry.rs` has exact circular-arc operations and a validated `MultiContactFluidBoundary`, including closure, non-adjacent intersection checks, and Green's-theorem area validation. These are useful primitives but are not a general exact Boolean-union algorithm.
5. `Form::Fluid` in `src/materials/resources.rs` can hold one of a polygon boundary, a circular-arc segment, or a multi-contact boundary. The variants are mutually exclusive. A general merge therefore cannot simply concatenate arbitrary source boundaries and call the existing validator.
6. `PhysicalMaterial::with_single_fluid_shape` only accepts one-part material with amount 1.0. It is not a valid path for representing the merged material amount without a new, explicitly validated material/shape realization path.
7. `OrganismStructure` uses stable physical constituent IDs in bonds. Removing one fluid constituent therefore requires a deliberate survivor-ID policy and remapping of every surviving bond endpoint, including fluid boundary endpoints.

## Implementation sequence

### Gate 1 — characterize supported union topology

Verified geometry primitives: `coalesce_adjacent_polygon_boundaries` handles two simple polygon loops sharing one complete reversed edge; `coalesce_multi_contact_boundaries_with_mapping` handles valid piecewise boundaries sharing one complete reversed `SolidContact` segment. The union removes the internal contact, retains exact free arcs, and returns source-piece provenance. `coalesce_placed_multi_contact_boundaries` transforms explicit source placements to world coordinates and returns the merged boundary in the survivor's local frame. These helpers are now used by the live COMBINE transaction; they do not split pieces at intersections or support arbitrary Boolean unions. Tangential-only point contact remains a rejection, not a merge.

### Gate 2 — pure geometry operation

The pure geometry stage returns piece provenance and conservatively resolves points on retained pieces. Live COMBINE uses the placement-aware helper and maps retained endpoints from either source; unsupported fluid-side `Corner`/`LineEndpoint` forms fail closed. Regressions reject ambiguous vertices, invalid maps, off-boundary points, and unsupported placements. Area, closure, orientation, self-intersection validity, and exact arcs remain required invariants.

### Gate 3 — material and identity mapping

**Implemented for the bounded same-resource case:** amount is summed, the first constituent's stable physical ID survives, the second ID is removed, genome membership is redirected, and retained fluid-side endpoints are mapped from their source piece to the merged boundary. Mixed-resource constituents, internal structures, legacy polygon-only shapes, and fluid forms without exact multi-contact boundaries fail closed. The current endpoint remapper accepts fluid `Fluid` and `Boundary` endpoint representations; rigid-style `Corner`/`LineEndpoint` endpoints on a fluid constituent are rejected.

### Gate 4 — atomic COMBINE integration

**Bounded path implemented and focused-test verified.** The live COMBINE callers run coalescence on trial state after bond formation and commit structure, usable energy, and ledger only on success. `combine_energy_transaction` keeps total expenditure unchanged for fluid pairs while classifying the consumed bond's intrinsic potential as dissipated heat. Focused accounting and rollback regressions pass.

### Gate 5 — regressions

Required tests:
- touching without a bond does not merge;
- valid fluid-fluid bond merges exactly once;
- nominal area and same-resource amount are conserved;
- no internal fluid-fluid bond remains after merge;
- all prior rigid bonds from both inputs survive with valid endpoints (including bonds attached to the second constituent);
- a third fluid contact and multi-contact boundary remain valid after remapping;
- malformed, overlapping, ambiguous, or unsupported union inputs fail closed;
- deliberately failing the final endpoint/energy check leaves original structure, energy, ledger, stable IDs, and bonds unchanged;
- serialize/restore the merged material and query the restored boundary;
- ordinary rigid-fluid bonds continue to work.

## Current gate

The bounded live path is focused-test verified through exact-head run [38264734792](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38264734792): fluid coalescence/restoration, endpoint guards, focused Bob/fluid, storage/reopen, formatting, source-size policy, COMBINE architecture, and chemical BREAK gates passed. The full Rust suite reports 396 passed, 28 failed, 1 ignored; strict Clippy remains red on broad repository lint findings. A follow-up exact-point query guard now rejects invalid open multi-contact boundaries; it passed on exact-head run [38265289112](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38265289112) after a formatting-only correction. Do not start broad Bob catalogue generation. General arbitrary-boundary and mixed-resource support remain separate future gates.


### Exact-head verification — 2026-10-12

Workflow [38259310228](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38259310228) on code head `02e8acac9463a9f6ed1c58777f331a7a4398ef58` confirms the focused fluid-coalescence geometry and live-transaction tests all pass. This includes the outer COMBINE rollback regression: the unsupported partial interface rejects without committing structure, energy, or ledger changes. The focused Bob/fluid and chemical BREAK workflow gates also pass, as do formatting, source-size policy, COMBINE architecture, and library persistence/reopen gates.

The overall workflow remains failed: the full suite reported **396 passed, 28 failed, 1 ignored** on exact-head run [38264734792](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38264734792), and strict Clippy remains red. The failures span multiple subsystems and are not attributed to coalescence without triage. The bounded same-resource, complete-shared-segment path is focused-test verified, not fully workflow-green. Merged-fluid serialization/restoration is verified. Keep PR #194 draft; any support for mixed resources or split/partial interfaces needs separate conservative gates.

### Persistence and restoration regression — 2026-10-12

The regression `coalesced_fluid_boundary_survives_physical_material_roundtrip_and_restore` exercises the merged boundary through `PhysicalMaterial` serialization/deserialization and `restore_material` into a fresh structure. It checks exact shape equality, valid boundary geometry, nominal area, and material amount. It passed on exact-head run [38264734792](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38264734792).
