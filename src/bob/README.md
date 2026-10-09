# Bob

Bob is EvoSim's persistent geometry knowledge engine.

- `library.rs` owns durable formations, contact families, canonicalization, and indexed runtime lookup.
- `worker.rs` expands and persists geometry knowledge.
- `server.rs` exposes the read-only geometry viewer.

Bob is knowledge, not live simulation state. A stored result describes previously validated geometry; it does not authorize a live physical change or replace validation of a realized structure.

## Current engineering scope

The current phase is limited to **Bob's internal consistency, library lookup, and measured lookup performance**.

**The initial-organism constructor is out of scope for this phase.** Do not change its algorithm, wire it to Bob, or add constructor-specific cache assumptions as part of this work. Constructor integration requires a separate decision after Bob's interface and behavior are stable.

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
