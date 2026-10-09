# Replacement Constructor — Phase 1 Dependency Audit

Status: **source inspection in progress; no implementation changes, compilation, or tests performed in this audit.**

Audit target: `remove-library-auto-publisher` source branch at the time of inspection. This is not a claim about `main`, which has a different data-only snapshot. The geometry storage practice branch is an isolated data-format experiment, not a runnable Rust integration branch.

## Executive findings

1. **The active initial-constructor entry point is still a fixed Carbon scaffold.** `Simulation::create_initial_organism` calls `initial_organism_constructor::construct_valid`. The constructor builds a 54-unit-style ring/support/outer-ring pattern from fixed constants, checks the inner cavity, then completes the fixed scaffold and searches for acquisition placements. It does not use Bob's geometry library to choose its growth sequence.
2. **The geometry library is a useful candidate/reference source, not a complete constructor.** It stores canonicalized formations, bonded adjacency, and rigid edge/point/vertex plus fluid-boundary contact families. It validates candidate formations for schema, resource existence, graph connectivity, actual declared contact, and pairwise penetration. A stored arrangement still needs contextual revalidation against the organism's current full structure and runtime bond/acquisition rules.
3. **Geometry-library lookup is not yet a universal live-contact resolver.** The library can project some realized interface queries against stored families, but its own classification helper currently labels the supported live interface classes as unresolved. The indexed resolver handles rigid edge/point/vertex queries, while fluid-boundary and other cases need separate treatment. Do not treat every live contact as resolvable to a unique family.
4. **Chemistry is split across several authorities.** Immutable resource properties live in `materials/resources.rs`; chemical-position interaction math lives in `chemistry/chemistry.rs`; pair work and cohesion-derived bond strength live in `chemistry/combine.rs`; transaction accounting lives in `chemistry/energy_ledger.rs`; and `chemistry/chemistry_library.rs` persists interface-keyed static potentials. The constructor should call these authorities rather than duplicate their formulas.
5. **There are concrete specification/runtime mismatches to resolve before constructor integration.** The current default Phosphorus shape is an L-like six-vertex polygon, not the approved isosceles trapezoid. The constructor uses `CONSTRUCTION_ENERGY = 1.0e12`, and simulation startup derives initial energy from `1.0e12 - construction.energy`; this conflates a construction-computation allowance with organism energy. The current bond transaction calculates strength from the geometric mean of the two cohesion values without applying the approved contact-feature scale (full/half by endpoint feature class).
6. **The current audit does not establish a green build.** The inspected source and tests were read through GitHub. No Rust checkout was executed here, so compilation, focused tests, worker startup, persistent-library reads, and end-to-end construction remain unverified.

## Dependency map

| Responsibility | Current authority / entry point | Constructor implication |
|---|---|---|
| Live genesis | `simulation.rs::Simulation::create_initial_organism` | One live caller; currently hard-wired to `construct_valid`. |
| Initial construction | `construction/initial_organism_constructor.rs` | Temporary scaffold implementation; must ultimately become the milestone-based engine, not a permanent second path. |
| Geometry records and queries | `bob/library.rs` (`GeometryLibrary`, `GeometryFormation`, contact-family records, `validate_formation`) | Supplies candidate arrangements/contact features; not a substitute for validating the candidate against the current organism. |
| Geometry persistence | `GeometryLibrary::open`, formation/family JSONL readers and writers | Keep canonical signatures and logical family keys stable; storage DTO changes are separate from constructor work. |
| Resource catalog and immutable properties | `materials/resources.rs::default_catalog`, `BaseResource`, `ResourceProperties`, `Form` | Single source for material properties and default shape. Geometry mismatches must be corrected in a separately approved geometry/catalog task. |
| Runtime placement/contact | `geometry/material_geometry.rs`, `geometry/contact.rs`, `geometry/structure.rs`, `chemistry/combine_runtime.rs` | Candidate placements must pass current physical contact, nonpenetration, endpoint availability, and bond transaction checks. |
| Bond/chemistry equations | `chemistry/chemistry.rs`, `chemistry/combine.rs` | Use chemical positions for static interaction; cohesion-derived strength and feature scaling must have one explicit authority. |
| Chemistry lookup/cache | `chemistry/chemistry_library.rs` | Currently stores interface-keyed static potentials; it is not the authority for material mass, potential-energy ledger, bond geometry, or the whole reaction policy. |
| Energy accounting | `chemistry/energy_ledger.rs` and `state::EnergyLedger` | Construction must report real physical transactions and reconcile energy; no synthetic budget-to-energy conversion. |
| Genome cavity | `geometry/cavity.rs::analyze_genome_cavity`, backed by `interior_geometry.rs` | The realized structure and bonded seal decide whether the genome milestone is reached. |
| Viability | `organism/viability.rs::validate_realized_organism` plus constructor acquisition checks | Keep cavity, external structure, and physical acquisition as separate, explicit conditions. |
| Resource storage/restoration | `materials/material_storage.rs`, `materials/material_restoration.rs`, `materials/physical_material.rs` | Preserve physical constituent graphs and material identity; never substitute logical storage for physical realization. |

## Geometry library contract observed

- Schema constant: `GEOMETRY_LIBRARY_SCHEMA_VERSION = 1`.
- Formation identity is canonicalized from schema, constituent resource/placement data, and bond pairs. The canonical signature is also the in-memory key; it is an identity contract, not disposable display text.
- `validate_formation` rejects unknown resources, invalid poses, invalid/repeated bond pairs, disconnected formations, fluid-to-fluid rigid bonds, declared bonds without actual boundary contact, and pairwise penetration. These checks are strict geometric checks; they do not prove that a proposed candidate fits the complete current organism after placement.
- `expand_formation_candidates` generates candidates by attaching a resource to each constituent through two-constituent arrangements, then canonicalizes/deduplicates them. It is an enumeration primitive, not a viability search or a growth policy.
- Persistent contact families describe candidate interface geometry and can be instantiated against a formation. They do not, by themselves, encode the whole-structure transaction, energy ledger, genome milestone, or acquisition policy.
- `GEOMETRY_EQUIVALENCE_TOLERANCE = 0.5` is a Bob record-equivalence tolerance. It must not be confused with the approved live physical contact tolerance of 0.1 or with the much stricter overlap/contact tolerances used in exact formation validation.
- Current storage audit notes that the full local catalogue is larger than the checked-in snapshot and that the runtime format experiment has not yet been integrated into Rust. Do not switch production persistence formats as part of this constructor audit.

## Chemistry and bond contract observed

- `ResourceProperties` contains mass, potential energy, reactivity, optional chemical position, and cohesion. Resource-type values are immutable catalog data.
- `Material::potential_energy` derives the fresh-material potential from catalog potential-energy values and amounts. Mixed-material weighted properties intentionally do not average chemical position; constituent identities must be retained.
- Chemistry interaction uses `chemical_position`, a bounded static potential, and a distance/contact factor. It does not use the legacy `reactivity` field for the approved chemical-position interaction.
- `formation_work_cost` is distinct from stored intrinsic bond potential. `bond_strength` currently uses the geometric mean of clamped cohesion values. The live bond transaction derives a bond's intrinsic potential, checks a candidate through a cloned trial structure, settles an energy transaction, and only then commits the trial structure.
- `ChemistryLibrary` schema is 2. It keys records by sorted material names plus interface class/signature and persists a static potential. Existing-record lookup is a cache/reference lookup; a miss requires the caller to calculate and persist a potential.
- The constructor should not interpret resource `potential_energy` as mutable energy content, and must not use a large artificial construction constant to manufacture initial usable energy.

## Blocking gaps before a transactional growth prototype

1. **Schema and catalog consistency:** confirm every resource shape and geometry scale against the approved material catalog before using Bob arrangements. The current Phosphorus shape mismatch is material and cannot be hidden by candidate validation.
2. **One contact contract:** separate Bob's 0.5 equivalence/deduplication threshold, the approved 0.1 live contact tolerance, and strict no-penetration validation. Write tests that pin all three meanings.
3. **Feature-scaled bond strength:** the live bond transaction must apply the approved feature-pair scale (edge-edge full; edge-corner half; corner-corner full; line-end-line-end full; line-end-corner full; line-end-edge half) exactly once. Current inspected bond-strength calculation does not show this scaling.
4. **Contextual candidate validation:** instantiate a library candidate, transform it into world coordinates, then validate against every relevant existing constituent and the real structure/bond authorities—not just the candidate's local formation.
5. **Atomic candidate transaction:** keep candidate geometry, resource accounting, bond endpoints, energy ledger, cavity state, and acquired material changes in a trial state; commit all together or leave the organism unchanged.
6. **Milestone transition:** run the actual cavity analyzer after relevant committed changes. Once a qualifying bonded-seal cavity exists, stop optimizing enclosure and switch to the rest-of-organism/acquisition goals.
7. **Energy semantics:** remove the `1.0e12 - construction.energy` path as part of the approved constructor migration. Do not alter energy formulas until their physical units and ledger transaction are explicitly reconciled.
8. **Reproducible library tests:** exercise missing records, ambiguous live interfaces, catalogue-order changes, malformed rows, duplicate records, invalid placements, and exact equivalence without changing production data.

## Removal checklist (not yet executed)

- [ ] Fixed ring/support/outer-ring construction recipe and its constants as the live genesis algorithm.
- [ ] Artificial construction-energy constant and its conversion into initial usable energy.
- [ ] Any constructor-local copies of geometry, material-property, bond, cavity, or acquisition rules that duplicate their authorities.
- [ ] Any candidate acceptance path that trusts a library record without whole-structure revalidation.
- [ ] Any duplicate constructor entry point or stale test that asserts an obsolete scaffold/topology rather than physical invariants.
- [ ] Contradictory implementation comments and docs after the replacement is verified.

Do not remove the current constructor or its valid physical invariants before the replacement is wired and independently validated.

## Phase 2 progress: geometry candidate contracts

A test-only contract suite has been added to `src/bob/library.rs` on the audit branch. It covers:

- Rejecting overlapping rigid constituents even if the formation declares a bond.
- Ensuring two-constituent candidate generation does not mutate its seed, and returns only valid, unique canonical candidates.
- Checking that the resulting candidate-signature set is invariant to catalogue iteration order.

These tests exercise the geometry reference candidate generator, not the live initial constructor or the full energy/bond transaction.

A GitHub Actions run reached `cargo test --all-targets`, but compilation failed before tests could execute. The first compiler run reported **51 errors**. The audit branch then made three narrow repairs: restored Bob lookup helpers from an earlier repository commit, corrected the exhausted-frontier return type in the Bob worker, and derived `Hash` for the chemistry-library key. It also aligned the initial-constructor candidate-evaluation destructuring with the actual four-value API.

The latest CI run reports **8 remaining compile errors**, all outside the Bob geometry lookup helper set and the `ChemistryKey` map-key contract:

- `ChemicalBreakOperation` derives `Copy` despite owning a `String`.
- `resources.rs` references missing `exponential_influence`.
- Diagnostics references a missing `lifecycle_changes` field.
- Genome defaults reference missing `default_reproductive_energy_allocation`.
- A harmonics test initializes a removed `ResourceBaselines.chemical_position` field.
- Observation passes an immutable organism where the growth-fraction API requires a mutable reference.
- Transformation calls `break_energy_yield` while the available helper is named `bond_break_energy_yield`.

The latest run's formatting, large-file, and COMBINE architecture checks passed. Rust compilation still fails before tests execute, so the new geometry and transaction contract tests remain **unverified**, not failed. The reduction from 51 to 8 compile errors is real progress, but it is not a green build.

## Evidence boundary and next action

This is still an audit/test-only branch; no production catalogue data, geometry parameters, or live constructor algorithm were changed. The CI run did expose existing source inconsistencies, and this branch contains only narrowly scoped API-alignment fixes rather than a broad attempt to repair all 51 errors.

Next safe task: preserve the restored Bob helpers with direct parser/index tests and keep the eight remaining compile blockers explicitly scoped. Do not start a broad simulation repair sprint just to force the full test suite green. Once the unrelated compile blockers are resolved by their owning workstreams, run the focused geometry and transaction contracts; then prove (a) candidate proposal is non-mutating, (b) full-structure nonpenetration and endpoint compatibility are checked, (c) failed transactions leave structure and ledger logically unchanged, and (d) successful commits reconcile bonds and energy.

## Follow-up: rigid-family persistence boundary

A focused writer-versus-loader inspection found that the rigid-edge and rigid-point insertion methods could append a family whose `formation_signature` was absent from the in-memory formation library, or whose `anchor_constituent` was out of range. The corresponding loaders reject those records on restart. That means the insertion call could report a persisted family which disappears after reopening the library.

The narrow correction adds the same formation-reference and anchor-index checks to both insertion methods. A contract test now submits invalid rigid-edge and rigid-point records and checks that neither is accepted nor written. This does not migrate catalogue data or change geometry generation. The persistence regression test is pending CI validation; the existing application compile blockers may still prevent execution. The successful-bond contract in `src/tests/bond_driven_contract_tests.rs` also checks that committed bond potential matches the transaction result, holder energy loss equals investment plus dissipated work, and the ledger's heat total matches the reported work cost. These assertions are authored but not yet executed.

The loaders now also reject rigid-edge records whose edge indices exceed the stored materials' rigid boundary segment counts, and rigid-point records whose candidate is not a line or whose anchor edge is invalid. A contract test writes malformed records through the catalogue-less insertion API and verifies they are discarded on reopen. The writer API still cannot validate material shape/index compatibility at insertion time because it does not receive or retain the resource catalogue; changing that API is deferred rather than folded into this audit.



## Follow-up audit: chemistry cache and energy integration boundary (2026-10-09)

A second source pass inspected `chemistry/chemistry_library.rs`, `chemistry/combine.rs`, and `chemistry/energy_ledger.rs`. These are source observations, not executed-test results.

### Chemistry library observations

- `ChemistryKey` canonicalizes the two material names, then distinguishes the interface class and a signature whose floating-point fields are quantized to 1e-9. This is deterministic for the same input representation, but the equivalence semantics are implicit: future code must not assume quantization means physical tolerance or contact acceptance.
- The cache stores a nonnegative finite `static_potential`, keyed by material pair and interface signature. The schema version is 2, but records do not separately identify the equation/parameter version that produced the potential. Before live use, decide whether schema version alone is sufficient invalidation when chemistry equations or constants change.
- `get_or_insert_static_potential` returns the existing cached value on a hit and persists the caller-provided value on a miss. The caller must calculate and validate a miss value with the authoritative chemistry equations. A cache hit is reusable stored knowledge, not proof that the current geometry, bond state, or reaction transition is valid.
- Persistence appends and syncs a JSONL record, updates the in-memory map, then writes the manifest. The record append and manifest update are not one atomic filesystem transaction. Startup tolerates a malformed final JSONL line (consistent with a torn append) but rejects malformed non-final lines; an existing manifest's entry count is not visibly reconciled against the loaded map in the inspected code. These behaviors need explicit restart/corruption tests before the cache is relied on for construction-critical state.
- Existing tests cover material-order canonicalization, interface-class distinction, key stability across formation-context changes, cache miss persistence, and duplicate-key rejection. The full test suite cannot currently execute because compilation stops earlier; do not report these tests as passing on this branch.

### Energy and bond observations

- `energy_ledger.rs` is marked as a staged API retained for subsystem integration. Its transaction type provides a balanced-accounting shape and its settlement operation updates holder energy and aggregate ledger totals atomically on validation failure. It does not decide whether a specific physical operation is entitled to release potential energy; that remains the operation's responsibility.
- `combine.rs` is also marked staged. Its inspected `bond_strength` function returns the geometric mean of the two clamped cohesion values only. It does not accept endpoint feature classes, so it cannot itself apply the approved feature-pair multipliers. Apply those multipliers at one explicit transaction boundary, exactly once, and test all six approved endpoint-pair classes to prevent omission or double scaling.
- Keep three values distinct in the growth engine: intrinsic bond potential, work/energy expenditure for forming it, and the resulting holder/ledger energy deltas. Tests should reconcile them from the same transaction object; do not infer one from another or use the library's static chemical potential as bond energy.
- The current `CONSTRUCTION_ENERGY` / `1.0e12 - construction.energy` path remains an independent blocker. Do not use a computational search allowance as a physical energy source.

### Refined interface and transaction contract

Before implementing growth, make the candidate pipeline explicit and test each boundary:

1. **Propose:** library/index may produce candidate placements and interface hypotheses; proposal is non-mutating and may be incomplete.
2. **Resolve:** identify the actual live endpoint/interface pair. If the interface resolver cannot classify it unambiguously, reject or route to a documented fallback rather than inventing a family.
3. **Validate:** enforce the 0.1 live-contact tolerance, scale compatibility, strict nonpenetration against every existing rigid constituent, resource/endpoint availability, and the current realized bond graph.
4. **Evaluate:** compute chemistry, bond potential, work cost, and affordability through their authoritative APIs.
5. **Trial:** apply geometry, bond graph, resource changes, cavity analysis, and energy settlement to a temporary state.
6. **Commit or rollback:** publish all state changes together only if every invariant passes; otherwise the externally visible structure, energy holder, ledger, and resource inventory remain unchanged.

A library family must never bypass stages 2–6. If a stage lacks an authoritative API, document that as an implementation prerequisite rather than re-implementing the rule privately inside the constructor.

### Current gate

The audit has identified enough concrete contracts to design the first transactional candidate prototype, but not enough evidence to claim the live constructor can safely use the libraries today. Next work should add small tests for chemistry-cache restart/corruption semantics and endpoint feature scaling, then define the candidate/transaction interface. The eight unrelated compile blockers recorded above still prevent all Rust tests from running; keep that limitation explicit and avoid broad unrelated fixes.


## Follow-up audit: runtime interface resolution and cache integrity (2026-10-09)

### Geometry interface resolution is not yet a usable lookup path

- `resolve_live_contact_interface` builds a canonical identity from endpoint classes and endpoint descriptors. The richer `resolve_live_contact_candidate` can also derive local descriptors from realized structural units and the contact candidate.
- `classify_live_family_resolution` currently returns `Unresolved` for every possible interface class, including `rigid_edge`, `rigid_point`, `rigid_vertex`, `fluid_boundary`, and `rigid_surface`. This is fail-closed and avoids false confidence, but it is not a completed lookup implementation.
- The canonical `LiveGeometryQuery` records an interface identity, while persisted families are keyed to a formation signature and retain anchor constituent/edge, candidate edge/endpoint/vertex, continuous parameters, and rotation intervals. The source currently does not establish a general unique mapping from a live query to one persisted family. A query signature alone must not be treated as enough to reconstruct missing continuous placement details.
- A repository-wide symbol search found no external callsites for the richer resolver, the family-resolution classifier, or the chemistry cache insertion API. Treat these as staged APIs, not live constructor integrations. Before wiring them in, add explicit callsites and tests for endpoint-order invariance, reversed material order, all endpoint-class pairings, ambiguity, no-match, and unique match.
- Proposed policy: return a persisted family only if matching is unique under documented geometry equivalence and current catalog/schema; return explicit no-match or ambiguity otherwise. Never silently select the first family from map/catalog order.

### Chemistry cache integrity details

- During open, malformed JSON on the final line is skipped as a presumed torn append; malformed non-final JSON rejects the library. However, syntactically valid records that fail `ChemistryRecord::is_valid` are silently skipped regardless of position. Duplicate keys in the persisted file overwrite earlier records in memory, so a conflicting duplicate is resolved by file order rather than reported as corruption.
- If `manifest.json` exists, its schema version is checked, but its `entries` count is not compared with the number of valid unique records loaded from `chemistry.jsonl`. A stale manifest can therefore be accepted.
- The key constructors quantize geometry-derived floating-point values to integer nanounits. They do not independently validate those values as finite before quantization. Keep validation at the family/input boundary, and do not let invalid values become plausible-looking persisted signatures.
- These are audit findings, not proof of observed production corruption. Before this cache becomes construction-critical, add tests for conflicting duplicate keys, invalid-but-valid-JSON records, stale manifest counts, truncated final lines, and invalid numeric family inputs. Decide explicitly which cases should recover by skipping a torn tail and which should fail closed as inconsistent durable knowledge.

### Updated dependency gate

The geometry reference library has candidate generators and durable family storage, but **not yet a completed, unambiguous runtime family resolver**. The chemistry library has durable key/value storage, but its corruption and version invalidation policy needs tests and an explicit contract. These are blockers for using library results to make live construction decisions; they do not block designing a non-mutating proposal interface or transaction boundary.
