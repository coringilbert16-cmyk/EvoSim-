# Repository stabilization audit — 2026-10-10

## Scope and evidence

This audit uses the GitHub source in `integration/unified-source-and-catalogue`, pull request [#186](https://github.com/coringilbert16-cmyk/EvoSim-/pull/186), plus the completed GitHub Actions run associated with commit `3551415ca9f9fdb010b268c518a4e0fe4a3835be`. No local checkout was available, and no constructor run was performed. This is a source/CI audit, not a claim of end-to-end runtime success.

## Executive finding

**EvoSim is not yet in a stable, clean-download state.** PR #186 is the correct current integration target because it brings the runnable Rust source tree back alongside the existing main-branch data and documentation. It is still draft-only and CI is red. Do not download `main` as the runnable application until the verified integration lineage is made the default branch; do not call PR #186 a stable release yet.

### Repository and branch state

- The GitHub default branch `main` is currently a data/documentation-only tree: the recorded tree audit found 15 entries and no `Cargo.toml` or `src/main.rs`.
- The former Bob branch and `main` diverged substantially. PR #186 was created as a single integration target to preserve the catalogue while restoring the application source, UI, runner scripts, and current documentation.
- PR #186 is open and draft-only. It reports 102 changed files and approximately 40,069 added lines relative to `main`; this is a lineage reconciliation, not a small feature change.
- The separate PRs #181–#184 have been closed/superseded by the integration effort. PR #179 remains a practice-only compact-storage study; it is not approval to switch or migrate the checked-in production catalogue.
- Keep one integration branch as the only stabilization target. Avoid further parallel implementation branches unless a task is explicitly isolated and then brought back through this integration line.

## Verified CI evidence

Workflow run: [Rust run 38055388755](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38055388755). The separate [runner validation run 38055388762](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38055388762) passed.

Passed:
- `cargo fmt --all -- --check`
- source-file-size audit
- COMBINE architecture check
- focused Bob compact-family-storage tests: 4 passed
- fresh Bob v3 store create/reopen check, including compact formation IDs and at least one compositional delta row
- runner script validation

Failed:
- `cargo test --all-targets`: **253 passed, 75 failed, 1 ignored** (328 executed/filtered status as reported by the runner). Failures span geometry/contact, cavity/genome qualification, constructor/acquisition, chemistry/COMBINE, movement/perception, observation, viability, material storage, reproduction, transformation, runtime history, and simulation integration.
- Clippy under the repository's `-D warnings` policy: the job failed. It reports unfulfilled `dead_code` expectations for staged APIs, unused helpers, and ordinary lints such as clone-on-Copy, needless range loops/lifetimes, manual contains, and default construction of a unit struct. Do not blanket-allow these to manufacture a green CI result; decide whether each API is live, genuinely staged, or obsolete, then fix the code/expectation accordingly.

The Bob persistence checks passing do **not** validate the application or the constructor. The full test result is a real failing baseline, not evidence that the focused storage changes caused all 75 failures.

## Source-level findings

### 1. The live initial constructor is still a scaffold

- `Simulation::create_initial_organism` calls `initial_organism_constructor::construct_valid`.
- The constructor still builds a fixed Carbon ring/support/outer-ring structure (54 units), rather than the approved blueprint-free, milestone-driven growth algorithm.
- It uses `CONSTRUCTION_ENERGY = 1.0e12`; the simulation initialization converts the unspent remainder into organism energy. This is a known specification mismatch and must be replaced by actual energy accounting, not tuned as a shortcut.
- The constructor's bond selection prefers corner-corner contacts, then falls back to the **farthest** eligible candidate if no corner-corner candidate exists. That fallback is not an approved physical ranking rule.
- It chooses each required resource placement independently instead of validating the complete acquisition set in a single trial state and committing atomically.
- The constructor does not use Bob to select a free-form growth sequence. The configured Bob root and worker exist, but the live constructor has not been demonstrated to use the library.

### 2. Physical authority and geometry still need reconciliation

- Bob's `GEOMETRY_EQUIVALENCE_TOLERANCE = 0.5` is a library-record equivalence rule; approved live contact tolerance is `0.1`. The two must remain explicitly separate, and neither may permit penetration.
- The current catalog's Phosphorus geometry remains L-like instead of the approved isosceles trapezoid (bottom 1.5, sides 0.5, top 1.0).
- The inspected bond-strength path does not yet apply the approved full/half contact-feature scaling consistently.
- Focused tests are present, but many geometry/contact and cavity contracts fail in the full run. First classify failures by shared authority/invariant; do not patch each test independently or weaken physical rules to satisfy fixtures.
- The geometry library's live-family resolution is still documented as unresolved for all interface classes. Candidate-family lookup cannot be treated as complete until live endpoints/features map unambiguously to stored records, or the runtime has a validated fallback that does not depend on that mapping.

### 3. Persistent data and cache integrity need explicit recovery rules

- Fresh empty Bob roots use compositional-v3 formation storage with stable IDs and compact family references; this has focused create/reopen evidence.
- Historical note (before the 2026-10-10 reset): the old generated catalogue had not been proven query-compatible. The user subsequently explicitly discarded that generated data as obsolete; the current branch heads no longer contain it. This does not delete generator/source logic or rewrite Git history.
- Family identity remains formation-scoped. Equal generated/added counts show that the tested generated records were persisted, not that every record is geometrically unique across formations. Cross-formation deduplication is unsafe without query-equivalence evidence.
- Existing chemistry-library loading has documented integrity gaps: invalid-but-parseable rows may be skipped, duplicate keys may overwrite by file order, and manifest counts are not reconciled with unique loaded keys. Define fail/skip/quarantine behavior and test corrupted files, duplicates, restart, and manifest mismatch before this cache becomes construction-critical.
- The worker's 20-constituent limit caps the size of any one formation; it does not cap the number of formations below that limit or prove the catalogue will finish in practical time. Full clean-generation completion and useful coverage remain unproven.

### 4. Documentation and CI are not yet a release gate

- The README correctly marks the replacement constructor as an approved plan rather than completed code and identifies PR #186 as the integration target.
- CI separately validates worker persistence, but it does not run the actual constructor as an acceptance test.
- A successful worker smoke test is not constructor acceptance. The required acceptance run must start from a clean or explicitly configured library root, run the actual initial-constructor path, and verify a physically realized organism, sealed analyzer-confirmed genome cavity, Water plus at least three other physically acquired resources, valid bonds/nonpenetration, and reconciled material/energy accounting.
- The release/default branch must not be switched until the source tree is runnable and the verification status is accurately recorded.

## Ordered work remaining

### Gate 1 — Freeze the integration line and baseline
1. Keep PR #186 as the only integration target; preserve all source definitions, approved rules, current data, and audit records.
2. Record the exact branch head and CI run for every stabilization change.
3. Do not run the worker as a separate data-generation exercise. The user's acceptance test is the actual constructor; use isolated temporary data only in automated persistence tests already required by CI.

### Gate 2 — Make build/lint status trustworthy
1. Triage strict Clippy errors into real defects, straightforward style corrections, intentionally staged APIs, and obsolete APIs.
2. Replace stale `expect(dead_code)` attributes with correct integration or remove obsolete code; avoid blanket `allow` attributes.
3. Re-run formatting, Clippy, and compilation after each cohesive correction set.
4. Preserve the existing full-test failure list and compare it after each fix. Do not report tests as passing unless they actually execute and pass.

### Gate 3 — Repair shared physical contracts before constructor integration
1. Establish one authoritative definition for geometry, contact, endpoints, bond formation, material realization, and energy/ledger settlement.
2. Resolve contact/nonpenetration and endpoint-feature semantics at their shared source, then update focused tests to the approved 0.1 live tolerance and full/half bond-feature rules.
3. Correct the Phosphorus catalog geometry to the approved trapezoid and test its realized boundary.
4. Fix the cavity analyzer and bonded-seal semantics so unbonded coincidence never qualifies as a genome.
5. Fix intact composite material handling through restoration, storage, movement, decomposition, and transformation.
6. Resolve the movement/observation and reproduction failures by tracing common contract breaks, not by treating each failing assertion as an independent design change.

### Gate 4 — Finish Bob's usable query contract
1. Specify which live interface queries can be answered by each persisted family type, including rotation/endpoint identity and fluid-boundary behavior.
2. Implement unambiguous family resolution or a validated fallback; fail closed when the query cannot be safely resolved.
3. Define corruption, duplicate, and manifest-recovery behavior for both geometry and chemistry stores.
4. Verify legacy checked-in data and fresh v3 data through the same logical query contract before considering a production format migration.
5. Measure clean-generation coverage/completion, family rows per formation/resource, bytes per record, cold-open/index cost, and memory. Do not deduplicate across formations solely to reduce storage.

### Gate 5 — Replace the live constructor
1. Implement the approved local, blueprint-free milestone constructor; use Bob as advisory candidate knowledge, never as physical authority.
2. Validate candidate placement against the whole realized structure; prohibit penetration and stale endpoint reuse.
3. Make each growth/bond/acquisition attempt transactional with rollback on any energy, geometry, bond, or resource failure.
4. Detect the genome from a realized cavity with a bonded seal; stop genome-building immediately when it qualifies.
5. Validate Water plus at least three other selected resources as a complete physical acquisition set before committing it.
6. Remove the arbitrary (10^{12}) construction-energy source and reconcile all material/energy ledgers.
7. Remove the fixed 54-unit scaffold and the unapproved farthest-contact fallback after the replacement passes its acceptance tests.

### Gate 6 — Prove end-to-end viability and release
1. Run the actual constructor against a clean/configured Bob root; do not substitute a worker-only smoke test.
2. Test multiple deterministic seeds and catalogue orderings; record time to cavity/viability, candidate counts, rejection reasons, and fallback use.
3. Verify the resulting organism through simulation handoff and downstream movement, acquisition, maintenance, transformation, and reproduction.
4. Re-run focused tests, full `cargo test --all-targets`, and strict Clippy. Document any remaining failures explicitly.
5. Only after those gates pass, finalize which generated data is tracked/rebuildable, merge the integration line, and make the verified runnable lineage the default branch.

## Definition of stable enough to download

- [ ] `main` contains the complete runnable source tree and project entry points.
- [ ] One authoritative README and one current integration branch describe the same architecture and status.
- [ ] Rust compilation, formatting, and Clippy pass without blanket suppressions.
- [ ] Shared physical-contract tests and the full test suite pass, or any accepted exception is explicitly justified and isolated.
- [ ] The actual constructor produces and verifies a physically viable organism from a clean/configured library root.
- [ ] Existing catalogue compatibility and clean regeneration are proven; no required data was silently deleted.
- [ ] The default branch points to that verified source/data lineage.

Until then, the integration branch is the working stabilization target, not a stable release.


## Follow-up source audit — 2026-10-10

This addendum records direct inspection of the integration branch's current source and checked-in library files. It supersedes earlier statements below where they conflict with the exact current files. This remains a source audit; no Rust build or constructor run was performed in this follow-up.

### Confirmed current state

1. **The initial constructor is still not the approved replacement.** `src/construction/initial_organism_constructor.rs` defines `CONSTRUCTION_ENERGY = 1.0e12`, a deterministic inner ring, six radial supports, and an outer ring. It does not query Bob to select a free-form growth sequence. The bond selector's fallback chooses the farthest eligible contact when no corner-corner candidate exists. The acquisition search uses a small fixed set of sample points and four rotations. These are source-confirmed migration targets, not inferred from old test names.
2. **The phosphorus shape discrepancy is resolved in this integration branch.** Both `src/materials/resources.rs::default_catalog` and `geometry_library/data/manifest.json` define the approved centered isosceles trapezoid: 1.5-unit bottom, 1.0-unit top, 0.5-unit sides. Earlier audit text claiming that the current integration branch still uses an L-like phosphorus shape is stale for this branch. No phosphorus geometry change is requested here. Its actual runtime realization and catalog-signature consistency still require tests.
3. **Chemistry persistence has a confirmed schema mismatch.** `src/chemistry/chemistry_library.rs` sets `CHEMISTRY_LIBRARY_SCHEMA_VERSION = 2` and rejects manifests whose schema is not 2. The checked-in `chemistry_library/data/manifest.json` declares schema 3, and all 17 checked-in rows carry key schema version 3. Therefore, opening this checked-in store through the current `ChemistryLibrary::open` fails with a schema-mismatch error; if the version check were bypassed, `ChemistryRecord::is_valid` would reject schema-3 keys under the schema-2 constant. This must be resolved before relying on persistent chemistry lookup.
4. **Chemistry loading silently skips semantically invalid rows and permits duplicate-key overwrite.** The loader inserts valid rows into a HashMap without rejecting duplicate keys and does not reconcile the manifest entry count with the number of unique loaded records. A malformed final non-empty row is silently ignored. These behaviors can conceal an incomplete or incompatible store.
5. **The live geometry-family resolver is explicitly unfinished.** `classify_live_family_resolution` currently returns `Unresolved` for all supported interface classes. A live canonical interface identity is not yet a completed mapping to a stored family record. Bob's catalog must not be described as fully queryable for constructor use until unique resolution or a validated fallback is implemented.
6. **Historical library-size snapshot (superseded by the geometry reset).** The former geometry manifest reported 25,737 formations and the chemistry manifest reported 17 records. The generated geometry files have since been deliberately removed from the current `main` and integration branch heads. Those geometry counts describe the discarded snapshot only; the chemistry records remain a separate cache, not a complete chemistry ruleset. Neither row count proves constructor utility.

### Decision required before chemistry-store repair

The checked-in manifest and all checked-in rows consistently use schema 3, while the source constant and validator expect schema 2. The source/readme do not specify a schema-2-to-schema-3 compatibility policy.

Recommended default: treat schema 3 as the current persisted format, add an explicit, tested migration or compatibility reader for schema 2 if any schema-2 store must remain supported, and reject unknown schemas with a clear diagnostic. Do not silently relabel old rows, discard data, or weaken validation. This policy needs confirmation before changing chemistry persistence code.

### Next implementation order

1. Resolve and test the chemistry schema compatibility contract without deleting or rewriting the checked-in store.
2. Add corruption/duplicate/manifest-count tests and make the loader's recovery behavior explicit.
3. Finish unique live geometry-family resolution (or a physically validated fallback) and prove queries against the existing catalog.
4. Repair shared physical-contract failures, then replace the scaffold constructor with the approved milestone-driven engine.
5. Use the actual constructor run as the end-to-end acceptance test; do not substitute a worker-only generation run.

No catalog records or physical rules were changed as part of this follow-up audit.

### Follow-up implementation — chemistry persistence integrity

The schema policy has been approved and implemented in source on this branch.

- Schema 3 is now the current chemistry format.
- The loader accepts schema-2 manifests/keys as a legacy compatibility read path without silently relabeling legacy keys. This preserves legacy data identity; it does not pretend a v2 key is identical to a v3 key.
- Unknown manifest versions, malformed records (including a malformed final non-empty row), semantically invalid records, duplicate persisted keys, and manifest entry-count mismatches now fail with `InvalidData` diagnostics rather than being silently skipped or overwritten.
- Six focused tests were added for schema-3 loading, schema-2 compatibility, malformed tail rejection, duplicate rejection, manifest count mismatch, and semantically invalid records.
- Commits: [chemistry loader fix](https://github.com/coringilbert16-cmyk/EvoSim-/commit/4df90c6f9a8ae01ba47cdcb14525dfa85113ddaf), [integrity tests](https://github.com/coringilbert16-cmyk/EvoSim-/commit/e9dcefa6b2740cce3f292b168b2e2c1d8f2b7b98), and [test compile correction](https://github.com/coringilbert16-cmyk/EvoSim-/commit/38982339c5b5023d5b0e1f1b8b17c8100e399896).
- These changes are committed but **not compiled or executed in this environment**. They are not yet verified as passing.

### Follow-up geometry lookup clarification

The earlier statement that the geometry resolver is wholly unimplemented was too broad. `GeometryLibrary::resolve_persistent_interface` and `persistent_interface_projection` do exist. The indexed resolver attempts matching for rigid-edge, rigid-point, and rigid-vertex queries and distinguishes unique, ambiguous, and unresolved results; focused unit tests cover these paths. The separate `classify_live_family_resolution` helper still returns `Unresolved` for every class, and live contact queries do not currently provide a corresponding query for fluid-boundary families. Therefore, Bob has a **partial indexed resolver**, not a complete universal live-contact resolver. The next audit should compare real runtime contacts against persisted family records, including same-material endpoint ambiguity, candidate rotation intervals, and fluid boundaries, and determine whether this resolver is called by the constructor at all.

The current checked-in phosphorus trapezoid is confirmed correct in this integration branch. Earlier top-level README/handoff statements claiming it is still L-shaped should be treated as stale until corrected; do not change the physical shape to compensate for stale documentation.

### Follow-up source audit — chemistry cache key reachability

The chemistry cache loader mismatch was not the only reason the checked-in chemistry rows were ineffective. The checked-in schema-3 rows use base material names such as `Carbon` and `Hydrogen`, but the live reaction path previously keyed even a single one-unit base material as `Carbon@1000000000`. That meant the runtime lookup key could not match the existing base-material records even when the interface class and signature matched.

The live reaction key builder now preserves the resource name for a one-part, one-unit base material while keeping amount-qualified canonical identities for mixtures and non-unit quantities. Tests were added for pure-base compatibility, composite identity preservation, and exact key construction for a checked-in Carbon–Hydrogen rigid-point interface. Commit: [base-material chemistry key compatibility](https://github.com/coringilbert16-cmyk/EvoSim-/commit/692741c68536b4e40494d46457ed11a11ea0bfd3) and [exact cache-key contract test](https://github.com/coringilbert16-cmyk/EvoSim-/commit/214a9b3ebb17c95e8e89d2605759cde09d0b0645). These changes have not been compiled or executed here.

Implication for library coverage: the 17 checked-in chemistry rows are all marked `Valid`, but they are only specific material/interface cache records, not a complete chemistry ruleset. Their value depends on runtime key compatibility and the caller's cache-miss path. Do not inflate the catalogue merely to cover every theoretical material pair; first measure real constructor/reaction lookup hits, misses, and newly calculated keys after this key mismatch is fixed.

### Verification update — current chemistry change set

GitHub Actions run [38057320097](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38057320097) ran against the chemistry source changes after formatting corrections.

- Formatting, source-file-size checks, COMBINE architecture checks, focused Bob compact-storage tests, and fresh-library create/reopen checks all passed.
- The new chemistry persistence tests passed, including schema-3 load, schema-2 legacy read, malformed-tail rejection, duplicate-key rejection, manifest-count rejection, and invalid-record rejection. The three new chemistry-key tests also passed.
- Full suite: **262 passed, 75 failed, 1 ignored** in 26.38 seconds. The number of passing tests increased from 253 to 262; the existing broad failure set remains, and the actual constructor still fails with `physical bond transaction 19-3 failed` in simulation startup.
- Strict Clippy still fails. It reports staged/dead APIs and ordinary lints; this must be addressed by deciding which APIs are live, intentionally staged, or obsolete—not by blanket suppression.
- This run is evidence for the persistence and key-contract tests only. It is not a passing build/release gate and did not validate constructor viability.


## Follow-up audit — geometry catalogue reset and live dependency — 2026-10-10

This addendum supersedes the earlier references to the generated geometry catalogue as present in the current branch heads. The seven generated geometry files were deliberately deleted from both `main` and `integration/unified-source-and-catalogue`; the geometry implementation and its documentation remain on the integration branch. Git history and other branches were not rewritten. No worker was run and no replacement data was generated.

### Empty-root startup: source inspection only

- `GeometryLibrary::open(root, catalog)` creates the root directory, treats missing `formations.jsonl` as an empty formation set, accepts a missing manifest as an in-memory current-schema manifest with zero entries, and treats absent frontier/family files as empty. This indicates that opening a fresh empty root is supported by the source contract; it does **not** prove runtime startup or persistence/reopen behavior, because no executable test was run in this audit.
- `open_default_library()` creates/opens the default `geometry_library/data` root. When `EVOSIM_GEOMETRY_LIBRARY_DIR` is set, the configured path must already exist and be either empty or contain an existing store's `manifest.json` and `formations.jsonl`.
- Opening the geometry library does not seed base formations or start generation. The worker's `seed_base_catalogue` and frontier processing are the mutating operations. Do not run the worker just to prove empty-root startup.
- `Simulation::create_initial_organism` calls `initial_organism_constructor::construct_valid`; it does not open or query Bob. The current constructor therefore does not benefit from the reset library—or from a populated library—until the constructor-to-library interface is implemented.
- The simulation opens the chemistry cache separately, but the initial constructor also does not query that cache as a candidate-selection input. Chemistry's live equations and transaction authorities remain separate from cached static interface potentials.

### Documentation correction

The geometry README previously contained worker smoke-test and generation commands below the new reset warning, creating conflicting instructions. Those commands have now been removed from the active handoff. Historical benchmark data remains historical evidence only and must not be treated as a request to regenerate the discarded catalogue.

### Next implementation order

1. Add/verify a non-generative empty-library open contract using an isolated temporary root; do not invoke worker seeding or frontier processing in this test.
2. Complete the Bob query result contract: it must return the actual candidate/family information needed to propose a placement, not only classify an interface or return a projection key. Validate real runtime contact descriptors against stored family intervals and report unique, ambiguous, and unresolved results, including fluid boundaries.
3. Integrate Bob as advisory geometry knowledge into the milestone-driven constructor. Whole-structure collision/nonpenetration, exact contact, chemical interaction, bond formation, energy accounting, and genome-cavity qualification remain authoritative runtime checks.
4. Diagnose `physical bond transaction 19-3 failed` at the COMBINE transaction boundary. Current caller reduces all internal rejection causes to a generic `None`; expose a structured/internal rejection reason for diagnostics without weakening physical constraints or changing success semantics.
5. Measure usefulness with constructor-facing query hits/misses, ambiguity, candidate rejection reasons, time to a sealed qualifying cavity, required-resource acquisition, and final viability. Only then define and approve a replacement-generation run.

No build, constructor execution, or worker execution was performed for this addendum. These are source-confirmed findings and a next-step plan, not runtime acceptance evidence.


## Follow-up implementation — anchor-relative Bob edge lookup — 2026-10-10

### Correction made

The rigid-edge family generator stores `candidate_rotation_radians` in the formation's coordinate frame. The indexed live resolver was comparing the realized candidate-minus-anchor rotation against that absolute stored rotation. These quantities differ whenever the family's anchor constituent is rotated in its formation, so valid contacts could be missed or projected as distinct interfaces. The resolver now converts the stored family rotation to the anchor-relative frame before comparison and uses that same relative rotation in the canonical projection key. This preserves the existing record schema and does not change physical contact rules.

Regression test: `indexed_edge_lookup_compares_rotation_relative_to_anchor_pose` passes in GitHub Actions. The isolated empty-root open/reopen test also passes and confirms that opening an empty store initializes schema metadata but does not seed formations or family rows.

### CI and worker policy

- CI now runs `cargo test fresh_empty_library_opens_and_reopens_without_seeding_geometry` instead of running the geometry worker twice to smoke-test persistence. The active workflow no longer generates a catalogue as a startup check.
- One CI run that had started before the workflow edit still completed the previous worker smoke step in a temporary GitHub runner directory. It did not write generated data to the repository or either branch's tracked tree. Future runs use the non-generative test.
- Latest Rust run [38060589759](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38060589759): formatting, source-size, COMBINE architecture, focused Bob compact-storage tests, empty-root test, and the new relative-rotation regression passed. The full suite remains red at **264 passed, 75 failed, 1 ignored**. Strict Clippy remains red on the existing staged/dead-code and other lint backlog; this change does not claim repository stability.

### Updated next steps

1. Add a constructor-facing Bob suggestion API that returns enough local geometry data to propose actual candidate placements. The current resolver returns only a resolution classification or canonical projection string; it does not return a placement candidate.
2. Test edge-edge, line-end/edge, and vertex/edge suggestion construction, including rotated anchors, same-material ambiguity, and unsupported/fluid interfaces.
3. Wire those suggestions into the milestone-driven constructor while retaining full-structure collision/nonpenetration, live contact, chemistry, bond transaction, energy accounting, and cavity qualification as physical authority.
4. Diagnose `physical bond transaction 19-3 failed` through structured internal rejection reasons. Do not use the current fixed scaffold or weaken physical constraints as a substitute for the replacement constructor.
5. Only after constructor-facing query coverage is measured should a replacement geometry catalogue be generated. No worker was run by the current implementation change, and no generated catalogue was restored.


## Follow-up audit — rigid-edge interval semantics corrected in source — 2026-10-10

The audit found that the persisted rigid-edge interval represented candidate-edge start-position coordinates while the live resolver compared it with the realized anchor contact-point parameter. The generator also applied the same interval formula to both edge orientations. The source correction has now been committed:

- `GeometryRigidContactFamily.anchor_parameter_start/end` now mean the normalized exposed interval on the anchor edge where a realized contact point may lie, consistently in `[0, 1]`.
- `generate_rigid_contact_families` stores the exposed interval directly, independent of candidate edge length or orientation.
- Loader and insertion validation reject rigid-edge intervals outside `[0, 1]`.
- `GEOMETRY_LIBRARY_SCHEMA_VERSION` was bumped from 1 to 2 so old generated stores cannot be silently interpreted under the changed interval semantics. The old generated catalogue is absent from current branch heads, so no regeneration is needed to make this change.
- Regression test `generated_rigid_edge_families_store_exposed_contact_intervals` passed in GitHub Actions. The rotated-anchor lookup test and empty-root test also passed on the preceding CI run.

A schema bump initially also changed the live chemistry key prefix because the two version concepts shared one constant. That coupling was corrected by introducing a separate `LIVE_GEOMETRY_INTERFACE_SCHEMA_VERSION = 1`; persisted Bob records are schema 2 while the unchanged live-interface key format remains `live-v1`. CI for that decoupling commit is in progress, so final verification is still pending.

The previous “required correction” list is now historical. The remaining blocker is to return actual placement suggestions to the constructor, and to ensure canonical projections collapse equivalent local interfaces across different formation-context constituent indices without hiding genuine geometric ambiguity.
