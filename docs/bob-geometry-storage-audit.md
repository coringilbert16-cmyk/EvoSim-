# Bob geometry storage audit and compact-format proposal

Status: storage integration is implemented on the `bob-automatic-compact-storage` branch and has passed focused CI tests plus a fresh-library persistence smoke test. The full application test suite and Clippy remain red for broader existing issues. No user's existing catalogue has been migrated or rewritten. This document does not authorize constructor integration or deleting/replacing existing data.

### Validation scope and dataset location

The compact-storage integration branch does not contain the full `geometry_library/data/*.jsonl` catalogue. The checked-in practice snapshot used by PR #179 is also smaller than the user's full local catalogue and must not be presented as the definitive baseline. The user's local data directory is not accessible from GitHub CI unless explicitly supplied.

Keep the validation gates separate:
- **Storage-tool validation:** parser correctness, stable-ID collision detection, exact reversible record transformation, counts, references, and logical-key equivalence on an isolated copy or fixture.
- **Storage integration validation:** focused Rust persistence tests and an isolated fresh-library create/reopen smoke test.
- **Whole-application validation:** full test suite and Clippy. Current failures are not proof that the focused storage tests failed, but they do block declaring the repository green.
- **Local-catalogue validation:** run read-only identity/coverage checks against the user's actual catalogue before any migration. Never infer equivalence from file size or row counts alone.

## Findings

### Repeated identity data

The canonical formation signature is built by `GeometryFormation::canonical_signature` from schema version, each constituent's resource and quantized placement, and bond pairs. It is also the authoritative key used by `GeometryLibrary.entries`; the signature is therefore not merely a display label.

The following persisted records carry a full `formation_signature`:

- `GeometryContactFamily` → `contact_families.jsonl`
- `GeometryFluidBoundaryFamily` → `fluid_boundary_families.jsonl`
- `GeometryRigidContactFamily` → `rigid_contact_families.jsonl`
- `GeometryRigidPointContactFamily` → `rigid_point_contact_families.jsonl`
- `GeometryRigidVertexContactFamily` → `rigid_vertex_contact_families.jsonl`
- `GeometryFrontierRecord` and its map key in `frontier.json`

Each family’s logical `signature()` also embeds the full formation signature. That logical key currently drives deduplication in the in-memory BTreeMaps and must remain stable through a storage-only migration. Frontier keys are built as `formation_signature|candidate_resource`; frontier compaction is a separate, later migration.

### Read and write paths

- `GeometryLibrary::open` reads formation rows first, then resolves family rows against the loaded formation map. It currently ignores invalid family rows rather than failing the entire library open.
- Formation rows are appended by `insert_many`; family rows are persisted by `insert_contact_families`, `insert_fluid_boundary_families`, `insert_rigid_contact_families`, `insert_rigid_point_contact_families`, `insert_rigid_vertex_contact_families`, and `insert_contact_family`.
- The family batch writers serialize the runtime family structs directly with `serde_json::to_writer`. Replacing the field in those structs would affect runtime callers, logical keys, and serialization together. Prefer dedicated on-disk DTOs so the runtime API and current logical family signatures remain unchanged.
- The frontier stores signatures in both a record and the record-map key. Do not include it in the first migration.
- The current loader streaming edits are present in the branch, but remain uncompiled and untested. The fluid loader’s error handling is not identical to the other loaders; retain and test the existing intended recovery policy rather than “cleaning it up” as part of a storage-format change.

### Worker/library API verification

A follow-up inspection of the exact `remove-library-auto-publisher` revision confirms that the worker-facing functions are present in `src/bob/library.rs`: `open_default_library`, `seed_base_catalogue`, `expand_formation_candidates`, `generate_rigid_contact_families`, `generate_rigid_point_contact_families`, and `generate_rigid_vertex_contact_families`. The server's `open_default_library` import is also defined. The earlier source audit incorrectly reported these symbols as missing; issue #178 records that mistaken finding and should be closed as a false alarm.

This source inspection confirms symbol presence only. It does not establish that the branch compiles or that tests pass. Before changing serialization, run formatting, compilation, and focused tests in a runnable checkout, and record the actual result. Do not treat the previous missing-symbol report as a confirmed blocker.

## Recommended compact family format

Keep runtime structs, formation signatures, and family `signature()` methods unchanged. Introduce dedicated versioned persistence DTOs for each family kind. A new row should contain:

- `storage_version: 2` (persistence format version, separate from geometry `schema_version`)
- `formation_id` (a fixed-length deterministic ID)
- all existing family-specific fields except `formation_signature`

The full canonical signature stays in `formations.jsonl` and remains authoritative. At load time, build a map from compact ID to canonical signature/formation. For each ID, detect whether another distinct canonical signature maps to the same ID; if so, fail closed with an explicit invalid-data error. Never silently choose one formation on collision. A stable cryptographic digest (for example, a 128-bit prefix of SHA-256 encoded as 32 hex characters) is preferable to Rust’s default hasher, whose output is not a persistence contract. This requires adding and locking a digest dependency or implementing and reviewing a stable algorithm explicitly.

For compatibility, parse a row as legacy only when it has no `storage_version` and has a string `formation_signature`; parse compact rows only when `storage_version == 2` and `formation_id` is present. Reject unknown versions. Resolve compact IDs to the full canonical signature before reconstructing the runtime struct, so validation, public APIs, indexes, and family deduplication keep their existing semantics. Do not change `GEOMETRY_LIBRARY_SCHEMA_VERSION` just to version the JSON representation.

## Migration sequence

1. Keep the storage tooling formatted and run it against a supplied catalogue copy or small synthetic fixture. Do not block this stage on constructor compilation. The earlier reported missing worker/library symbols were found on reinspection; the current whole-project CI failures are a separate integration problem.
2. Run `python3 tools/audit_geometry_storage.py geometry_library/data` against the full local catalogue when available. Preserve the report as the definitive baseline; use the checked-in snapshot only as a secondary comparison.
3. Implement DTO serialization/deserialization for the three largest rigid-family files first, without changing the runtime structs or logical signatures.
4. Add an explicit migration command that reads legacy rows and writes to a separate versioned destination. Never rewrite the only source files in place. The migration must stop on malformed required rows, unresolved formation references, ID collisions, or count/key mismatches.
5. Verify legacy-vs-compact equivalence: source and destination row counts; resolved references; sorted logical family signatures; lookup projections/results; duplicate handling; and behavior when a final line is truncated. Only then consider switching the default reader/writer to the new format.
6. Keep original files recoverable until equivalence checks pass and measured disk savings are recorded. Test restart/resume and interruption before any production catalogue is migrated.
7. Evaluate water/fluid family records next. Leave `frontier.json` for a distinct migration after the family format is proven.
8. Measure cold-load time, peak memory, and lookup latency separately from disk-size reduction. Do not add a cache or prune valid formations as part of this migration.

## Acceptance criteria

- No formation or valid family is lost.
- Every compact ID resolves to exactly one canonical formation.
- A collision is detected and stops the migration/load rather than choosing a wrong formation.
- Runtime family identity, deduplication, lookup results, and physical validation are unchanged.
- Legacy data remains intact until equivalence checks pass.
- Before/after size and performance measurements come from the actual local catalogue and are reported separately.
- Constructor code and constructor-to-Bob integration remain out of scope.


## Verified CI status — 2026-10-10

Commit `049c1533cc268c173545a9f424851b8922245c8f` passed the following CI steps:
- `cargo fmt --all -- --check`
- source-file size check
- COMBINE architecture check
- focused `compact_family_storage_tests`: **4 passed, 0 failed**
- fresh-library smoke test and reopen/persistence check

The fresh-library smoke test used a temporary directory through `EVOSIM_GEOMETRY_LIBRARY_DIR`, ran `cargo run -- --geometry-worker-once` twice, and verified that the resulting manifest declares storage format 3 and `exact-one-constituent-delta-v1`; it also verified that formation rows contain compact IDs and at least one exact constituent-delta row. This establishes fresh-store persistence in CI, not compatibility or equivalence of the user's existing local catalogue.

The full `cargo test --all-targets` step compiled and ran but reported **248 passed, 75 failed, 1 ignored**. Failures span constructor/placement closure, contact and endpoint geometry, cavity/genome checks, chemistry/COMBINE, movement, reproduction, and simulation integration. Many simulation tests cascade from initial construction failing with `physical bond transaction 19-3 failed`. Do not attribute these failures to compact storage without isolated evidence. They do mean the complete repository is not green.

Clippy also failed on broad lint debt, including unused/staged APIs and lint expectations across Bob, construction, geometry, organism, and chemistry modules. It should be handled as a separately scoped cleanup; do not silence lints wholesale just to make CI green.

### Remaining storage gates

1. Audit the actual local catalogue read-only: count each record type, validate JSONL parsing, identify logical keys and formation references, detect duplicates and unresolved references, and preserve a baseline report. The checked-in practice snapshot is not a substitute.
2. Prove legacy-vs-compact equivalence against a copy of the real catalogue: counts, resolved references, sorted logical family signatures, lookup results, duplicate behavior, and malformed/truncated-row behavior.
3. Only after equivalence, write a compact candidate to a separate destination and measure size, load time, peak memory, and lookup latency.
4. Investigate live-family lookup separately. `classify_live_family_resolution` currently classifies all listed interface classes as `Unresolved`; compact IDs reduce repeated storage but do not solve the semantic mapping from a realized live contact to a unique persisted family.
5. Keep constructor replacement and phosphorus geometry validation separate from this storage migration.
