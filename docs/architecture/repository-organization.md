# Repository organization

EvoSim source is organized by responsibility rather than historical file creation order.

Current target:
- src/simulation/ — simulation loop, state, runtime, server, process runner
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
