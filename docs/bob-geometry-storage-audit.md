# Bob geometry storage audit and compact-format proposal

Status: source audit only. No compilation, tests, local-catalogue execution, or migration has been performed. This document deliberately does not authorize constructor integration or deleting/replacing existing data.

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

### Source/API blocker found

On branch `remove-library-auto-publisher`, `src/bob/worker.rs` imports these names from the module mapped to `src/bob/library.rs` in `src/main.rs`:

- `open_default_library`
- `seed_base_catalogue`
- `expand_formation_candidates`
- `generate_rigid_contact_families`
- `generate_rigid_point_contact_families`
- `generate_rigid_vertex_contact_families`

Those names are not defined in the fetched `src/bob/library.rs` revision. The file does define `generate_water_contact_families` and `generate_fluid_boundary_families`, but not the listed functions. This is a source-level inconsistency and likely compile blocker, not a test result. Resolve it against the intended authoritative implementation before changing serialization; otherwise the storage migration would be built on an unverified API surface.

## Recommended compact family format

Keep runtime structs, formation signatures, and family `signature()` methods unchanged. Introduce dedicated versioned persistence DTOs for each family kind. A new row should contain:

- `storage_version: 2` (persistence format version, separate from geometry `schema_version`)
- `formation_id` (a fixed-length deterministic ID)
- all existing family-specific fields except `formation_signature`

The full canonical signature stays in `formations.jsonl` and remains authoritative. At load time, build a map from compact ID to canonical signature/formation. For each ID, detect whether another distinct canonical signature maps to the same ID; if so, fail closed with an explicit invalid-data error. Never silently choose one formation on collision. A stable cryptographic digest (for example, a 128-bit prefix of SHA-256 encoded as 32 hex characters) is preferable to Rust’s default hasher, whose output is not a persistence contract. This requires adding and locking a digest dependency or implementing and reviewing a stable algorithm explicitly.

For compatibility, parse a row as legacy only when it has no `storage_version` and has a string `formation_signature`; parse compact rows only when `storage_version == 2` and `formation_id` is present. Reject unknown versions. Resolve compact IDs to the full canonical signature before reconstructing the runtime struct, so validation, public APIs, indexes, and family deduplication keep their existing semantics. Do not change `GEOMETRY_LIBRARY_SCHEMA_VERSION` just to version the JSON representation.

## Migration sequence

1. Fix and verify the Bob worker/library API mismatch first. Run formatting, compile, and focused tests in a runnable checkout; source inspection is not a substitute.
2. Run `python3 tools/audit_geometry_storage.py geometry_library/data` against the actual local catalogue. Preserve the report as the baseline. The checked-in GitHub snapshot is not a valid proxy for the user's larger local catalogue.
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
