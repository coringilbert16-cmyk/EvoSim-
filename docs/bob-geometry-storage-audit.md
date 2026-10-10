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

- `GeometryLibrary::open` reads formation rows first, then resolves family rows against the loaded formation map. Current loader recovery behavior is not uniform across family types; invalid-row handling must be explicitly covered by tests before migration.
- Formation rows are appended by `insert_many`; family rows are persisted by the family insertion methods.
- Runtime family structs and their logical `signature()` methods retain the full canonical `formation_signature`. On-disk compact rows are produced by `compact_family_value`, which removes that field and writes a deterministic `formation_id`. The reader restores the full signature before deserializing the runtime struct.
- The family encoding is declared once in `storage_manifest.json` as `formation-id-reference-v2`; current compact family rows do **not** add a per-row `storage_version`. Do not introduce a per-row version requirement without a separate compatibility design.
- The current family ID is a 128-bit prefix of SHA-256, encoded as 22-character unpadded base64url. The read-only size estimator must use this exact ID shape if its savings estimate is to be meaningful.
- The frontier stores signatures in both a record and the record-map key. Do not include it in the first migration.
- The current integration passes focused storage tests and the isolated fresh-library create/reopen smoke test (see the dated CI status below). These checks do not prove the full local catalogue is equivalent after migration.
- Invalid family rows and unresolved references require special scrutiny: the existing loader can skip invalid family rows rather than fail the whole open, which risks hiding data loss if used as a migration verifier.

## Current compact-family behavior

The integrated writer uses compact family references on new writes, regardless of whether formation rows are legacy or v3. The formation storage version and family encoding are related through the manifest but are distinct concerns:
- **Formation rows:** a fresh empty library uses compositional-v3 with a deterministic formation ID and, when profitable and exactly reconstructible, an exact one-constituent delta. A nonempty legacy formation store remains in legacy formation mode; it is not automatically migrated.
- **Family rows:** writers replace repeated full `formation_signature` values with deterministic `formation_id` values. On load, those IDs are resolved to the canonical formation signatures used by the runtime.
- **Frontier:** still stores full signatures and remains outside this compacting change.

The current smoke test proves a fresh isolated library writes v3 metadata and at least one formation delta, then can be reopened. It does not yet prove legacy family data in the user's complete local catalogue can be converted without loss. Keep the actual-catalogue equivalence and separate-destination migration gates below.

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


## Live-family resolution audit — 2026-10-10

Read-only inspection of `src/bob/library.rs` found that the lookup gap is narrower and more specific than “Bob has no live lookup”:

- `GeometryLibrary::resolve_persistent_interface` does perform indexed lookup for the three query variants currently represented in `LiveGeometryQuery`: `RigidEdge`, `RigidPoint`, and `RigidVertex`. It returns `Unresolved`, `Unique`, or `Ambiguous` based on distinct canonical family projections.
- `indexed_interface_projections` has no query variant for fluid-boundary contacts. `resolve_live_contact_candidate` can label an interface `fluid_boundary`, but its query representation does not encode the fluid-boundary geometry, so that interface has no indexed persistent-family path.
- The persisted `GeometryContactFamily` and `GeometryFluidBoundaryFamily` collections are not consulted by `indexed_interface_projections`; the implemented indexed resolver only reads the rigid edge, rigid point, and rigid vertex family indexes.
- `classify_live_family_resolution` is a separate topology-only function that returns `Unresolved` for every interface class. It is not a meaningful availability test for the indexed resolver and should not be treated as evidence that all actual indexed lookups fail.
- The existing focused lookup contract test proves rejection of one wrong-anchor-material case. The ignored throughput test expects every sampled query to resolve, but it requires the persistent local catalogue and was not run in the reported CI. Current evidence therefore does not establish lookup coverage or correctness across real live contacts.

### Required next lookup work

1. Add deterministic fixtures for each supported query variant: known unique projection, no match, and multiple distinct projections; include endpoint-order invariance and boundary tolerance cases.
2. Decide whether `classify_live_family_resolution` should be removed, renamed to describe topology-only classification, or replaced with a library-aware call. Do not make it claim uniqueness without consulting indexed records.
3. Define the runtime geometry/query contract for fluid-boundary contacts before implementing a fluid lookup. Do not guess from the class string alone.
4. Decide whether `GeometryContactFamily` is intended to participate in live resolution. If so, specify a stable query-to-family projection and ambiguity rules first.
5. Run lookup coverage against the actual local catalogue and report unique/ambiguous/unresolved counts by query class. The checked-in snapshot is useful for development but is not a substitute for the full local catalogue.

No runtime code was changed during this audit. These findings narrow the next implementation task without changing the storage format or risking catalogue data.


### Additional matcher correctness risks found — 2026-10-10

A second read-only pass over `indexed_interface_projections` and the projection helpers found concrete reasons not to treat a `Unique` result as fully validated yet:

- **Rigid-point and rigid-vertex queries omit candidate rotation.** Their persisted family records contain candidate rotation start/end ranges, but the live query variants contain only material, endpoint/vertex, anchor edge, and anchor-edge parameter. The matcher therefore cannot check whether the realized candidate rotation lies in the stored range.
- **Projection keys discard part of each family's interval.** `rigid_family_projection`, `point_family_projection`, and `vertex_family_projection` include the anchor-parameter start but omit the end. Point/vertex projections also omit candidate-rotation ranges. Consequently, distinct stored family records may collapse to one projection even when the omitted ranges differ. This may be intentional if those fields are proven irrelevant to runtime identity, but that contract is not documented or tested.
- **Same-material edge contacts need an explicit test.** The rigid-edge matcher determines which query side is the candidate using material equality. When both sides have the same material, that test alone cannot distinguish candidate side from anchor side; the sorted local descriptors may make the order deterministic, but the lookup's candidate/anchor interpretation must be shown to agree with family generation.

These are risks identified by source inspection, not demonstrated runtime failures. Before changing behavior, define the canonical family projection contract and add tests that vary only the omitted rotation/interval fields. Then update the query representation or projection rules according to that contract. No source code or catalogue data was changed in this pass.


### Lookup contract test progress — 2026-10-10

The first deterministic edge-resolution tests are now committed in src/bob/library.rs:

- indexed_edge_lookup_distinguishes_unique_ambiguous_and_unresolved checks a single matching family, a query outside all stored parameter intervals, and two distinct matching projections.
- same_material_edge_lookup_is_endpoint_order_invariant covers a same-material contact where the candidate and anchor use different edge indices. This test exposed an ambiguity in choosing the candidate side from material equality alone. The matcher now uses the candidate/anchor edge indices to orient that case, rejecting records whose edges fit neither orientation.
- Existing live_interface_is_endpoint_order_invariant still checks canonical query identity when the two endpoint arguments are swapped.

This is a narrow matcher correction plus focused tests; it does not resolve the separate missing-rotation fields for rigid-point/rigid-vertex queries, nor the projection-key omission of interval ends and rotation ranges. Those remain explicit follow-up work because changing their semantics requires a stable identity contract.

Validation status: the commits are present on bob-automatic-compact-storage. The GitHub workflow/status query returned no runs or status records for the latest commit at the time of this note, so test execution and CI success are not verified from this interface. Do not treat these tests as locally run.


### Canonical family projection contract — 2026-10-10

For indexed live resolution, a projection represents a **contact-compatibility class**, not the full persisted record identity. Different formations may share one projection when their runtime contact geometry is equivalent. However, every field that changes the range or orientation of a compatible contact must be represented in the projection; otherwise separate compatibility classes collapse and ambiguity is hidden.

The projection helpers now include:

- Rigid edge: candidate resource, anchor constituent/edge, candidate edge, candidate rotation, and both anchor-parameter interval bounds.
- Rigid point: candidate resource, anchor constituent/edge, candidate endpoint, both anchor-parameter bounds, and both candidate-rotation bounds.
- Rigid vertex: candidate resource, anchor constituent/edge, candidate vertex, both anchor-parameter bounds, and both candidate-rotation bounds.

Focused tests vary the interval end and rotation-range bounds independently and assert that each change produces a distinct projection. This deliberately makes resolution conservative: if two matching records describe different contact intervals or candidate-rotation ranges, they count as distinct projections rather than silently collapsing into a false Unique result.

**Remaining limitation:** rigid-point and rigid-vertex live query variants still do not carry realized candidate rotation. The projection change prevents different stored rotation ranges from collapsing, but it does not yet prove that a single matching family's rotation range contains the live candidate's actual orientation. A follow-up must extend the runtime query contract (including how orientation is derived for the endpoint-only resolver) or fail closed for queries that lack it. Until then, a Unique point/vertex result is not fully rotation-validated and must not be treated as complete geometric proof.

The contract tests have been committed, but the available GitHub workflow/status query did not return a run or check result for these commits. Compilation and test success remain unverified until CI or a local Cargo run reports results.
