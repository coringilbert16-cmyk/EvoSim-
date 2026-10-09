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

A GitHub Actions run reached `cargo test --all-targets`, but compilation failed before tests could execute. The compiler reported **51 errors** across the current source branch. Important blockers relevant to this plan include:

- Bob's `library.rs` calls missing helpers for parsing edge/point descriptors, indexing contact buckets, and projecting rigid edge/point/vertex families. These are direct blockers to compiling or testing the geometry library.
- The constructor candidate-evaluation API returns four values, while the initial-constructor call site had stale five-value destructuring. That call site and stale contract-test destructuring were aligned with the actual four-value API on this audit branch.
- `ChemistryKey` is used as a `HashMap` key but does not implement `Hash`.
- Other compile errors are outside the immediate constructor/library scope (including stale resource, diagnostics, genome, observation, and transformation references).

The run's formatting, large-file, and COMBINE architecture checks passed. Rust compilation failed, so neither the new geometry tests nor the full test suite ran. This is a verified build failure, not a test failure and not evidence that the newly added tests themselves fail.

## Evidence boundary and next action

This is still an audit/test-only branch; no production catalogue data, geometry parameters, or live constructor algorithm were changed. The CI run did expose existing source inconsistencies, and this branch contains only narrowly scoped API-alignment fixes rather than a broad attempt to repair all 51 errors.

Next safe task: restore and contract-test Bob's missing helper functions as a bounded library repair, then re-run CI to expose the next layer. Keep unrelated simulation compile failures separate instead of turning this phase into a whole-repository repair sprint. Once Bob can compile, continue with the synthetic candidate/transaction harness and prove (a) candidate proposal is non-mutating, (b) full-structure nonpenetration and endpoint compatibility are checked, (c) failed transactions leave structure and ledger logically unchanged, and (d) successful commits reconcile bonds and energy.
