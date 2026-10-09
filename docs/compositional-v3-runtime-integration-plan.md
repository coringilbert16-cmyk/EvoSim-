# Compositional-v3 runtime integration plan

Status: design note only. No production Rust code or catalogue data has been changed.

## Decision

Keep the compositional-v3 encoding as a separate on-disk representation behind a persistence adapter. Do not alter `GeometryFormation`, runtime family structs, canonical signatures, or logical family keys to match the file format. The adapter must reconstruct the same runtime objects that legacy files currently produce.

The checked-in `main` snapshot contains five JSONL files and 25,807 formations. The isolated practice migration measured 8,536,354 bytes for the compositional formation file versus 14,618,529 bytes for compact-v2 formation rows. Across all five JSONL files, this implies 141,542,681 bytes versus 181,511,850 original bytes, before manifest overhead. The reference chain statistics for this snapshot are maximum 3, mean 1.664, p95 3. These are snapshot measurements, not guarantees for the full local catalogue.

## On-disk format responsibilities

- `storage_manifest.json` identifies the storage format version, formation encoding, and formation-ID algorithm. Reject unsupported versions rather than guessing.
- Full formation row: retains `formation_id`, schema version, constituent list, bond list, and any other persisted fields; it has no `base_id`.
- Delta formation row: retains its own `formation_id` and schema version, then stores `base_id`, insertion index, full removed constituent, incident bond records with original bond positions, and any row-specific extra fields.
- Family rows use the compact-v2 family encoding and point to the formation via `formation_id`. Their in-memory `formation_signature` must be restored before constructing runtime family structs or logical deduplication keys.
- The formation ID is a lookup token, not the authoritative canonical signature. After decoding a formation, recompute its canonical signature from geometry and verify it maps to the expected ID. Detect any 128-bit ID collision between distinct canonical signatures and fail closed.

## Load algorithm

1. Parse the manifest and validate the supported versions and algorithms.
2. Parse all formation rows into a DTO index keyed by formation ID. Reject duplicate IDs, malformed rows, missing required fields, invalid bond positions, and duplicate incident-bond positions.
3. Decode full rows and delta rows into the same canonical runtime formation map. Use cycle detection and explicit handling for unresolved references. Prefer iterative/topological decoding or a bounded explicit stack rather than relying on unbounded recursion.
4. Validate constituent and bond indices after reconstruction; ensure every delta removes exactly one constituent from its base and that each stored incident bond touches the inserted constituent.
5. Recompute canonical signatures and verify every ID-to-signature mapping is unique and stable.
6. Load family rows. Resolve each `formation_id` to its canonical signature; reject unresolved IDs. Convert DTOs into the existing runtime family structs, preserving existing family `signature()` keys and deduplication semantics.
7. Compare the resulting runtime catalogue against a legacy load in a separate test process or fixture: formation signatures, all family logical signatures, counts, duplicate behavior, and representative lookup results must match exactly.
8. Only after equivalence passes should any explicit migration tool write compositional-v3 files to a new destination. Never rewrite the only copy in place.

## Required tests before production consideration

- Legacy-v1 and compact-v2 loading still work as before.
- Compositional-v3 full rows, one-level deltas, multi-level chains, and a catalogue with out-of-order row IDs decode identically.
- Missing base ID, cycle, duplicate formation ID, digest collision, unknown format version, malformed JSON, truncated final line, invalid insertion index, duplicate incident-bond positions, invalid bond endpoint, and signature mismatch fail explicitly.
- All extra persisted fields round-trip exactly.
- Every family type present in the actual catalogue resolves and retains its logical signature. Include `rigid_point_contact_families.jsonl` when it exists in the full local catalogue; it is absent from the current checked-in snapshot.
- Restart/reopen after migration gives identical objects and lookup outputs.
- Interrupted writes do not replace the previous catalogue; write into a fresh temporary destination and atomically publish only after all validation passes.
- Benchmark cold-load time, peak memory, and representative lookup latency separately from disk size. Compare both warm and cold OS-cache conditions where feasible.

## Implementation boundary

This work does not authorize a production-format switch. First implement and test the persistence DTO adapter on a dedicated integration branch against a runnable Rust checkout. Existing constructor failures remain separate; they should not be “fixed” as part of storage migration. Keep the current compact-v2 and legacy files recoverable until the adapter and equivalence suite pass on the complete local catalogue.
