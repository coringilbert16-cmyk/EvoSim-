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

Top-level persistent knowledge stores:
- geometry_library/
- chemistry_library/

Supporting areas:
- tests/
- docs/
- scripts/
- ui/

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
- Top-level chemistry-library and tests boundaries now exist.

Still in migration:
- Unit/contract tests remain beside executable modules in src/ because they depend on private crate internals.
- chemistry_library/ needs its durable-record layout and runtime persistence boundary finalized.
- Bob's large library.rs still needs an internal split into lookup/query/index/resolution/persistence/validation responsibilities.
- Remaining documentation should be grouped under architecture, mechanics, libraries, decisions, and audits as those documents are touched.

Authority boundaries:
Generated library data is runtime knowledge, not source code.
Bob owns persistent geometry knowledge and lookup/generation machinery. Live construction remains authoritative for physical validity.
Chemistry owns chemical calculations and transitions. Its persistent knowledge store remains separate from live chemistry implementation.
