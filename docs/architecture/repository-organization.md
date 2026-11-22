# Repository organization

EvoSim source is organized by responsibility rather than historical file creation order.

Current target:
- src/simulation/ — simulation loop, state, runtime, server, process runner
- src/physics/ — independent physical field solvers and numerical reference models
- src/organism/ — lifecycle, genome, development, decisions, reproduction
- src/geometry/ — physical geometry, structure, contacts, boundaries
- src/construction/ — construction runtime, material selection, initial constructor
- src/chemistry/ — reaction, combination/break/decomposition, transformation, ledger
- src/materials/ — resource/material definitions, storage, transfer, restoration
- src/perception/ — observation, harmonics, movement and movement direction
- src/bob/ — persistent geometry knowledge engine
- src/infrastructure/ — cross-cutting process/library synchronization and diagnostics
- src/ui/ — server-served visualization support

Persistent knowledge data:
- chemistry_library/data/ — tracked chemistry cache records and manifest.
- Geometry-library code and runtime lookup/generation live under src/bob/; there is no separate top-level geometry_library/ directory in the audited main tree.

Supporting areas:
- src/tests/ — crate-internal contract and simulation tests.
- docs/ — architecture, mechanics, plans, and audit evidence.
- scripts/ — repository checks.
- tools/ — focused audit utilities.
- ui/ — browser assets, separate from Rust visualization support under src/ui/.

Migration rules:
1. Organize by responsibility, not historical filename.
2. Move one subsystem boundary at a time.
3. Fetch the current file before moving it.
4. Preserve existing crate-level module names with path attributes while migration is in progress.
5. Remove the old root copy in the same migration commit.
6. Do not duplicate executable source into persistent library directories.
7. Do not rewrite behavior merely to make the tree look cleaner.

Completed:
- Bob has its own src/bob/ boundary.
- Bob's three executable components now use canonical filenames: library.rs, worker.rs, and server.rs.
- Major root-level implementation files have been moved into responsibility directories.
- src/main.rs preserves existing crate-level module names through explicit path attributes.
- Geometry-library documentation points at Bob's current library implementation.
- The chemistry-library data root exists; tests remain under src/tests/ because they need crate-internal access.

Still in migration:
- Unit/contract tests remain beside executable modules in src/ because they depend on private crate internals.
- chemistry_library/ now contains tracked schema-3 cache records and a manifest; the persistence code exists, but runtime hit/miss coverage and the long-term write/refresh authority still need verification.
- Bob's large library.rs still needs an internal split into lookup/query/index/resolution/persistence/validation responsibilities.
- [x] Added `docs/README.md` as an index-first taxonomy for the existing document set; file moves remain deferred until incoming links are audited.
- [x] Audited responsibility groups in `source_exposure.rs`, `capillary_geometry.rs`, and Bob's `library.rs`; documented candidate splits without changing runtime code.
- Remaining documentation can be migrated under architecture, mechanics, libraries, decisions, and audits incrementally after link audits.

Authority boundaries:
Generated library data is runtime knowledge, not source code.
Bob owns persistent geometry knowledge and lookup/generation machinery. Live construction remains authoritative for physical validity.
Chemistry owns chemical calculations and transitions. Its persistent knowledge store remains separate from live chemistry implementation.


## Module integration policy

There is no fixed source-file size ceiling. The former sub-30 KB target was a temporary workaround for context and truncation limits, not an architectural requirement, and must not drive new module boundaries.

- Integrate implementation into the owning subsystem as it is developed; avoid creating thin one-purpose files solely to stay below a byte or line threshold.
- Split a module only at a stable conceptual boundary: distinct domain responsibility, independently meaningful lifecycle, clear ownership boundary, or a cohesive algorithm family with a useful private API.
- Prefer a cohesive larger module over fragmented files that require readers to jump between files for one operation.
- Keep genuinely independent subsystems separate even when they are small; this is not a mandate to merge unrelated code.
- Keep tests with the implementation when they rely on private internals; otherwise group contract tests by behavior.
- Do not perform a mechanical mass-merge. Audit callers, imports, visibility, tests, and cyclic dependencies; integrate one coherent area at a time with behavior-preserving diffs.
- File size may be monitored as a review signal, but it is never an acceptance threshold or a reason by itself to split or merge.


## Completed source integrations

- [x] Moved periodic movement-direction selection into `src/perception/movement.rs`, beside the movement behavior that owns its scoring and direction-selection policy. Updated the expulsion caller to use the movement subsystem and removed the standalone `src/perception/movement_direction.rs` module. This was a behavior-preserving consolidation; existing movement tests remain colocated in the movement implementation.

This is the first example of the revised policy, not a target-size rule. Further integrations should be selected from dependency and responsibility audits, not from a list of the smallest files.

- [x] Integrated `material_restoration.rs` into `src/materials/physical_material.rs`. Restoration is an operation on an already-realized `PhysicalMaterial`: it restores that value's authoritative constituent arrangement and pre-existing internal bonds. All callers now use `crate::physical_material::restore_material`; the standalone file and root module declaration were removed. This does not change COMBINE, energy settlement, or material-transfer semantics.

## Small modules intentionally kept separate

The source audit also identified small files that should not be merged merely because they are adjacent or short:

- `material_storage.rs` owns the organism inventory container and its storage transactions; `material_transfer.rs` extracts whole unstructured units from ecological aggregates. Neither is the same operation as restoring a realized `PhysicalMaterial`.
- `connection_faces.rs` models eligible chemical connection faces; `connection_geometry.rs` transforms already-derived physical connection points. Their contracts are deliberately distinct.
- `decision.rs` defines decision-policy data and rules; `decision_runtime.rs` adapts that policy to runtime candidates; `developmental_decision.rs` previews physical developmental actions. They may integrate further when the staged developmental path becomes active, but currently represent separate layers.
- `structure_authority.rs` is an audit/reporting boundary, not part of the structure mutation API.

These remain separate based on responsibility, not file size. Revisit only when implementation sharing or an actual dependency boundary shows that a different grouping is clearer.

- `material_storage.rs` is intentionally cohesive despite its size: the stored-entry model, inventory queries, atomic store/take operations, and invariant tests share one inventory lifecycle. Do not split it into CRUD-shaped files. `material_transfer.rs` is a separate boundary because it extracts whole unstructured units from an ecological aggregate before organism-facing physical realization; it is not an inventory operation or restoration of an existing `PhysicalMaterial`.
- Simulation modules remain separated by runtime layer: `runtime.rs` owns live runtime state/history and the child-process client; `sim_process.rs` owns the child-side command protocol and tick loop; `server.rs` owns the HTTP/WebSocket API; `simulation_runner.rs` owns CLI experiment execution. These layers communicate, but merging them would mix distinct process and interface responsibilities.
- Geometry modules with related names also have distinct contracts: `connection_faces.rs` describes eligible chemical connection faces/endpoints, while `connection_geometry.rs` transforms already-derived physical points; `physical_geometry.rs` owns immutable realized shape state, while `rigid_boundary.rs` calculates rigid alignment and boundary normals. Keep these separate until a shared implementation API—not just shared vocabulary—shows a better boundary.
