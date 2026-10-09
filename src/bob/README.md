# Bob

Bob is EvoSim's persistent geometry knowledge engine.

- `library.rs` owns durable formations, contact families, canonicalization, and indexed runtime lookup.
- `worker.rs` expands and persists geometry knowledge.
- `server.rs` exposes the read-only geometry viewer.

Bob is knowledge, not live simulation state. A stored result describes previously validated geometry; it does not authorize a live physical change or replace validation of a realized structure.

## Current engineering scope

The current phase is limited to **Bob's internal consistency, library lookup, and measured lookup performance**.

**The initial-organism constructor is out of scope for this phase.** Do not change its algorithm, wire it to Bob, or add constructor-specific cache assumptions as part of this work. Constructor integration requires a separate decision after Bob's interface and behavior are stable.


## Approved storage-efficiency work

The checked-in geometry library is already large, and the local catalogue is larger than the repository snapshot. The immediate storage work is a lossless data-model reduction, not deletion of formations, reduced geometric precision, or exhaustive catalogue pruning.

### Findings from source inspection

- The rigid edge, rigid point, rigid vertex, fluid-boundary, and water-contact family records each serialize a full `formation_signature`. That signature encodes the formation's constituents, quantized positions/rotations, and bonds. The same string is repeated in every family row associated with that formation.
- Family deduplication signatures also embed the full formation signature, so changing the serialized representation must preserve the current logical identity and duplicate behavior.
- Loaders currently validate family rows by resolving `formation_signature` into the in-memory formation map. A compact reference therefore needs a reliable ID-to-formation index built from the authoritative formation records.
- The formation, generic contact-family, rigid-edge-family, and fluid-boundary JSONL loaders now parse records incrementally instead of collecting the entire file into `Vec<String>`. The streaming change is committed on this branch; compilation and tests have not yet been run, so behavior preservation still requires verification. The rigid point/vertex loaders were already line-streaming.
- The frontier also repeats formation signatures in both its map key and record. It is a secondary target after family storage, because frontier resume semantics must remain exact.

### Storage migration design

1. **Measure a baseline.** Run `python3 tools/audit_geometry_storage.py geometry_library/data` to record per-file bytes, rows, row lengths, repeated-signature bytes, and a rough compact-size estimate. This read-only report does not modify the catalogue and is an estimate, not a substitute for measured migration output. Separately record load time and peak memory from the real local catalogue when a runnable checkout is available. Do not estimate savings from the GitHub snapshot as though it were the user's 79k-formation local store.
2. **Add deterministic compact formation IDs.** Derive an ID from the canonical formation signature using a stable, explicitly selected hash algorithm. During load, build an ID-to-formation mapping and detect any ID collision where distinct canonical signatures map to the same ID; fail closed rather than resolving a collision to the wrong formation. Do not use Rust's default hasher for persisted IDs.
3. **Version the family-record schema.** New compact records store the formation ID instead of repeating the full signature. The formation's full canonical signature remains authoritative in the formation record. Runtime APIs may resolve IDs back to formations, but must preserve family identity, validation, index behavior, and duplicate detection.
4. **Migrate transactionally.** Keep the current files untouched as the source of truth until all old records are parsed, every reference resolves, counts and logical family signatures are checked, and new files are flushed successfully. Write to a separate versioned output or temporary paths first; never overwrite the only copy during conversion. A restart must not mistake a partially written migration for a completed one.
5. **Preserve compatibility deliberately.** Old and new formats must be distinguishable by schema/version metadata. Either support reading the legacy format during a migration window or provide an explicit one-way migration command; never silently reinterpret an old row as a new schema. Do not bump the whole library schema until the implications for formations, frontier, and catalogue compatibility are resolved.
6. **Stream loaders (source change made; verification pending).** The formation, generic contact-family, rigid-edge-family, and fluid-boundary loaders now parse one line at a time; rigid point/vertex loaders already did. The change retains the prior final-record recovery checks for the three main loaders and the fluid loader's all-or-empty I/O-error behavior. Run formatting, compilation, and focused restart/corrupt-tail tests before treating this as verified.
7. **Prove equivalence and savings.** Compare legacy and compact data by row counts, resolved formation references, family keys/projections, and lookup results. Benchmark cold load, memory, and indexed lookup before and after. Report measured disk savings separately from memory and speed changes.
8. **Apply to remaining repeated references only after the first migration is proven.** The three largest rigid-family files are the first target. Then evaluate fluid/water families and frontier records with the same measurements. Lossless compression can be evaluated afterward; it must not replace fixing repeated data in the model.

**Acceptance criteria:** no formation or valid family is lost; every compact reference resolves uniquely; collision checks are enforced; live physical validation remains authoritative; old data remains recoverable until verification passes; and before/after size and load measurements are recorded. Constructor code and constructor-to-Bob integration remain out of scope.

## Latest source-audit blocker

A focused source audit found that `src/bob/worker.rs` imports `open_default_library`, `seed_base_catalogue`, `expand_formation_candidates`, and three rigid-family generators that are not defined in the currently mapped `src/bob/library.rs` revision. Treat this as an unresolved source/API mismatch and likely compile blocker until checked in a runnable checkout. Do not implement compact serialization on top of this unresolved API. See [the detailed storage audit and migration proposal](../../docs/bob-geometry-storage-audit.md).

## Recommended work plan

### 1. Reconcile the source/API contract

Audit the module mapping and the public functions used by `worker.rs`, `library.rs`, and the application entry point.

- Every imported library function must resolve to the intended authoritative implementation.
- The worker's discovery/expansion API must not be confused with the read-only lookup API.
- Remove or correct stale names, duplicate implementations, and documentation that describes functions or behavior that do not exist.
- Keep persistent formations, contact families, canonicalization, and indexed lookup under one clear library interface.
- Record unresolved mismatches rather than building new features on an unverified contract.

**Exit condition:** module wiring and imports agree, the documented public API matches the source, and compilation is verified in a runnable checkout.

### 2. Specify lookup contracts

Document each runtime lookup by its input, result, and guarantees.

- Define how formation identity and canonicalization affect matching.
- Define how rigid contact families and fluid/contact-family records are queried.
- Respect catalogue/schema and resource-shape versions; stale or incompatible records must not be treated as current.
- Make clear whether a result is an exact formation, a contact-family description, or a candidate that still needs instantiation.
- Keep the full physical validator authoritative whenever a symbolic family is instantiated or a stored formation is applied to a live physical situation.

A library hit is knowledge reuse, not a bypass around physical rules.

### 3. Measure before adding a priority cache

The persistent library already has indexed lookup. Establish the actual bottleneck before adding another layer.

Measure representative formation and contact-family queries, including repeated hot queries and varied cold queries. Record query count, elapsed time, and relevant catalogue size; keep benchmark results reproducible and separate from correctness tests.

If measurements show repeated lookups justify it, implement a **bounded, frequency-aware in-memory hot cache** for query results:

- Promote entries according to observed reuse frequency, not only most-recent access.
- Enforce an explicit memory/entry bound and a defined eviction policy.
- Track hits, misses, promotions, and evictions so the benefit is measurable.
- Treat cache contents as disposable acceleration state; the persistent library remains the durable knowledge source.
- Ensure cold start, cache eviction, and cache-disabled operation preserve identical lookup semantics.
- Do not persist cache state unless a measured, documented need justifies that separate design.

Do not add a cache simply because the catalogue is expected to grow. If indexed lookup is already fast enough, retain the simpler design and document the measurements.

### 4. Verify Bob in isolation

Use focused tests for:

- API/module consistency and supported query behavior;
- canonical formation lookup and duplicate handling;
- rigid and fluid/contact-family lookup semantics;
- schema/version rejection or migration behavior;
- cold-start and repeated lookup equivalence;
- cache bounds and promotion/eviction, if a cache is implemented;
- worker persistence and restart/resume behavior;
- separation between worker writes and read-only viewer/lookup paths.

Any test that writes catalogue data must use an isolated temporary library root. Do not let tests mutate the persistent production catalogue.

Run formatting, compilation, and focused tests in a runnable checkout. Report each check as passed, failed, or not run; source inspection alone is not a test result.

### 5. Explicitly defer constructor integration

This phase ends when Bob's API is internally consistent, its lookup guarantees are documented, and measured lookup behavior is understood.

Only after that exit condition is met should a separate proposal describe how a future constructor might query Bob, validate a candidate using the live physical rules, and fall back to on-demand work when the library has no suitable answer. That later proposal is not authorization to modify the constructor now.

## Non-negotiable boundaries

- Bob stores reusable geometry knowledge; it is not live organism state.
- Persisted records and cache entries never override current physical validation.
- The worker discovers and persists knowledge; read-only lookup does not silently run discovery.
- Avoid exhaustive preplanning of every possible organism or universe configuration. Grow the catalogue from validated reusable formations and use symbolic contact families where a continuum cannot be represented as a finite list of samples.
- Prefer the smallest implementation supported by measurements over speculative layers.
