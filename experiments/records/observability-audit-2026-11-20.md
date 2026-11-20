# Chemistry observability audit — 2026-11-20

**Plan:** [OBS-001 — Compact chemistry observability](plans/OBS-001-chemistry-observability.md)
**Tracking:** [Issue #396](https://github.com/coringilbert16-cmyk/EvoSim-/issues/396)
**Audit status:** First-pass audit; source and open-PR inspection. Not yet a full repository-wide instrumentation audit.
**Baseline:** `main` at `fd45f2987d55623bce3ad25f1a0ededc0100ff10`.

## Executive finding

EvoSim does not need a new telemetry system built from zero. Main already records substantial chemistry evidence: tick-zero material identities and inventory, molecule lifecycle creation/destruction with material graphs, committed reaction events, per-tick discovery summaries when activity exists, periodic/final material and energy snapshots, and aggregate discovery/energy summaries. Offline analyzers already reconstruct bond changes, molecular lineages/families, temporal complexity, and sampled contact opportunities.

The main problem is **fragmentation and uneven evidence coverage**: records do not yet form one consistently versioned, provenance-rich and fully joined experiment dataset; discovery counters and bounded samples have different coverage semantics; exact event-to-transaction joins and complete opportunity denominators remain incomplete; and some environmental exposure needed for causal analysis is absent or only tick-zero/legacy sampled.

The correct next step is to consolidate and close specific gaps, not log the entire world every tick.

## First-pass capability matrix

| Capability | Existing evidence | Assessment | Gap / action |
|---|---|---|---|
| Run header and initial conditions | Environmental JSONL header includes schema version, seed, requested duration, planned ticks, biology flag, starting object/resource/molecule/constituent counts, elemental inventory, pile sizes, hotspot, diagnostic interval | **Partial** | Add exact code/build and workflow provenance, analyzer version references, actual elapsed duration and standardized environment/intervention manifest. |
| Stable material identities | Tick-zero lifecycle records require nonzero environmental IDs; creation/destruction lifecycle events preserve molecule IDs and parent/reaction references | **Substantial** | Verify identity uniqueness/lifetime semantics and exact joins for every reaction type; document legacy schema behavior. |
| Full reaction outcomes | Recorder writes each environmental reaction event and its lifecycle records | **Substantial** | Audit event fields and transaction identity across COMBINE/BREAK/DISPLACEMENT; ensure explicit stable reaction ID and normalized outcome/reason contract. |
| Molecular structure and lineage | Lifecycle snapshots contain physical material; chemistry/organization analyzers reconstruct bond-pair changes, molecule creation/destruction, parent-child lineage and structural families | **Substantial** | Validate graph completeness and join consistency; retain missingness rather than infer absent edges. |
| Discovery funnel | Main tick diagnostics and aggregate summary include BREAK/COMBINE/DISPLACEMENT readiness, scheduling, commit/reject and discovery counters | **Partial** | Counters are not a complete per-identity opportunity census. Add explicit search coverage, deferred/skipped/truncated/unknown semantics and denominator eligibility. |
| Contact opportunity geometry | Main contact analyzer aggregates broad-phase inspections, valid interfaces, endpoint candidates, truncation and bounded opportunity samples | **Partial / sampled** | Samples are explicitly not all pair encounters; identities/geometry are not complete for every evaluated opportunity. Never treat sample count as a complete denominator. |
| Displacement discovery | Main counters include bond scans, replacement-contact searches, activation and trial rejection categories | **Partial** | Need reliable identity-level joins and full eligible-target coverage before rates or candidate-specific discovery claims. |
| Energy/material accounting | Periodic and final snapshots include usable/resonant/structural energy, external input, work, heat, escaped signal, ledger totals, material inventory, and balance residual | **Substantial at run level** | Event-level transaction joins and standardized units/reconciliation need audit. Aggregate snapshots alone cannot attribute energy cost to a pathway or event. |
| Environmental exposure | Main recorder writes local resonance sample events; hotspot/source data appears in snapshots | **Partial / unverified** | Verify sample semantics and time coverage. Do not label local aura samples as the planned persistent resonance field. Need event-linked time-resolved flow/hotspot/field exposure only for comparisons that require it. |
| Time-series | Periodic inventory/energy snapshots; activity tick events emitted when relevant diagnostic activity occurs | **Partial** | Standardize time windows and distinguish absent activity from absent diagnostics. Add compact per-window summaries offline where possible. |
| Offline analysis | Chemistry, organization, temporal complexity, and contact-opportunity analyzers exist | **Substantial but fragmented** | Standardize analyzer entrypoints, output schema/version, validation, cross-seed aggregation, and a common report. Audit remaining tools before consolidation. |
| Reproducibility and artifact provenance | Experiment registry, result documents, workflow links, raw JSONL artifacts | **Partial** | A canonical manifest should bind commit, workflow run, seeds, durations, configuration, schema, analyzer versions, and artifact paths. |
| Statistical interpretation | Existing experiment records state seeds are replicates and events within a seed are not independent | **Reliable policy; implementation varies** | Carry replicate structure and uncertainty methods into standard cross-seed reports. |

## Open work inspected

### PR #345 — opportunity funnels and starting-mixture experiments

[PR #345](https://github.com/coringilbert16-cmyk/EvoSim-/pull/345) is open and unmerged at audit time; its base/head are not the same as current main and GitHub reports it is not mergeable at this check. Its changed files add:
- BREAK/COMBINE/DISPLACEMENT discovery counters and batch observation workflows.
- Randomized, uniform, and clustered dispersion conditions.
- Matched starting-molecule mixtures with inventory compensation.
- Per-seed chemistry/organization analysis and five-seed aggregate analyzers.

**Reuse decision:** Treat its funnel schema, batch analyzers, and controlled-mixture design as candidate work to reconcile with main, not as installed production capability. Before reuse, inspect exact counter definitions, denominator semantics, test coverage, and branch divergence. Avoid copying its analyzers or workflows into a second implementation.

### PR #392 — local neighborhoods and matched catalyst controls

[PR #392](https://github.com/coringilbert16-cmyk/EvoSim-/pull/392) is draft/open and unmerged at audit time; GitHub reports it is not mergeable at this check. Its changed files add:
- Bounded local-neighborhood opportunity samples with nearby molecule identity, composition, topology, geometry/distance, truncation and missing-geometry metadata.
- Exact source-material identities and endpoint descriptors on reaction outcomes.
- Contact-opportunity and paired-comparison offline analyzers.
- Tick-zero flow/hotspot/world-tone exposure telemetry and a paired exposure preflight.
- A manual matched-catalysis workflow that fails closed.

Its own PR description says the paired comparison is not eligible for causal interpretation because full-run time-resolved flow/hotspot exposure, background resonance from non-candidate materials, and complete eligible target-pair denominators are missing. Tick-zero exposure comparison is not a full-run exposure match. The local opportunity samples are bounded and are not a complete unbiased denominator.

**Reuse decision:** Preserve the fail-closed preflight. Do not weaken it or launch matched catalyst arms until its exposure and denominator requirements are satisfied. Treat its current neighborhood and exposure data as diagnostic context only.

## Highest-priority gaps

1. **Canonical provenance and schema:** one manifest with exact commit/build, workflow, seed, requested/actual duration, tick count, configuration, schema/analyzer versions, and artifact links.
2. **Identity-safe event joins:** one stable reaction/event identifier linking participants, lifecycle records, products, geometry, and authoritative energy/material transaction.
3. **Opportunity coverage semantics:** common counters for searched/available, inspected, evaluated, geometrically valid, chemically/activation eligible, rejected, deferred, committed, skipped/truncated, and unknown; explicitly state when no complete denominator exists.
4. **Event-linked environment exposure:** only for questions requiring it; distinguish time-resolved measurements from tick-zero snapshots and legacy pairwise aura proxies.
5. **Common offline reports:** reuse existing analyzers but standardize validation, output versions, cross-seed aggregation, and interpretation warnings.

## Data-volume policy

- Keep JSONL event/lifecycle records as raw evidence.
- Do not emit a full-world snapshot every tick.
- Keep periodic snapshots at the configured interval and create fixed-window summaries offline.
- Store event references to exposure windows rather than repeating large arrays.
- Make full neighborhood, field, and movement traces opt-in, bounded, and explicit about truncation.
- Reanalyze existing artifacts before commissioning any new runs.

## Deeper source and workflow findings

The field-level source pass confirmed several important details:

- `EnvironmentalChemistryEvent` carries tick, reaction kind, source cell indices, source material indices, source compositions, required work, optional frequency/activation/accumulation values, broken-bond index, committed/rejected outcome, a failure label, product summaries, and lifecycle records.
- Lifecycle records carry exact molecule IDs, parent IDs, reaction IDs, reaction type, action, tick, and optional full molecule/material snapshots. The current lifecycle reaction ID is generated from tick plus the event's position in the per-tick event vector.
- The reaction event itself does **not** carry that reaction ID. Rejected events generally have no lifecycle records, so they lack the exact persistent source-molecule identity join available for successful lifecycle transitions. Source material indices are location/vector indices, not durable identities.
- COMBINE's current failure label intentionally groups stale, invalid, and unfunded resolver failures rather than inventing a detailed cause. This is honest but limits rejection analysis.
- The per-tick COMBINE opportunity sample is explicitly bounded to eight entries. The sample records source identities, geometry/endpoints, distance, physical validity, reaction kind, and old bond energy; it is not a complete opportunity census.
- Per-tick diagnostics contain useful scan/work/budget and candidate counters, but the JSONL recorder emits a tick diagnostic record only when relevant work/activity exists. It does not write an explicit zero-activity record for every tick. The summary does retain a count of observed ticks; periodic snapshots provide a coarser timeline.
- Energy is strong at the **run inventory** level: snapshots report usable, resonant, structural, external-input, escaped-signal, work, heat, and residual values. The authoritative energy ledger's reaction transaction includes reason, usable delta, signed structural delta, work, and heat, but those transaction values are not presently serialized alongside each environmental reaction event. This prevents straightforward event/pathway-level energy attribution from the JSONL alone.
- The standard workflow already runs multiple offline analyzers: chemistry, molecular organization, reaction-network, geometry-aware network, contact-opportunity, and temporal-complexity analysis. Per-seed artifacts are retained for 30 days and combined reports for 90 days. This is useful existing infrastructure; OBS-001 should orchestrate and standardize it, not replace it wholesale.
- The JSONL header records requested duration and planned ticks, but not actual elapsed wall-clock duration, exact commit/build identity, GitHub workflow/run ID, or analyzer versions. The workflow environment can supply some provenance externally; a canonical manifest should join it to the raw data.

These findings sharpen the implementation priorities: add durable event IDs to reaction events and connect them to lifecycle/energy records; preserve exact source identities for rejected as well as committed events where feasible; formalize bounded-sample and zero-activity semantics; and add a small manifest/analysis index that binds workflow provenance to raw artifacts. Do not add full snapshots each tick.

## Event-identity and analysis-pipeline detail

The environmental event type currently includes tick, reaction name, source cell indices, source material vector indices, source compositions, work requirement, optional activation/frequency/accumulation values, broken-bond index, outcome, a failure label, product summary, post-commit source-cell compositions, and optional lifecycle transitions. Lifecycle transitions carry molecule IDs, parent IDs, reaction IDs, and full material snapshots. The lifecycle reaction ID is currently generated as a tick plus the event's current position in the tick event vector; it is not stored on the enclosing reaction event.

This means:
- A committed reaction can often be reconstructed through its lifecycle records, but the event-to-lifecycle join is indirect.
- A rejected event generally has no lifecycle records and no durable source molecule IDs in main's event payload. Source vector indices are not durable identity.
- COMBINE rejection reasons intentionally collapse stale, invalid, or unfunded resolver failures into one honest but coarse label.
- Event records contain work required and activation context, but not the exact authoritative energy settlement transaction. The ledger supports signed structural delta, usable-energy delta, work, and heat; JSONL snapshots currently expose these mainly as aggregate run totals.
- Main's contact-opportunity diagnostic stores a bounded array of up to eight opportunity samples per tick, each with source IDs and detailed endpoint geometry. The sample is useful evidence but cannot supply a full denominator.
- The current main recorder writes activity tick records only when meaningful diagnostic work/activity is present; zero-activity ticks are not represented as explicit rows. The summary retains total observed tick count, while periodic snapshots provide a coarser timeline.

The standard workflow already orchestrates six useful offline analyses: per-seed chemistry and molecular organization, then cross-seed chemistry summary, reaction network, geometry-aware reaction network, contact opportunities, and temporal chemical complexity. Per-seed artifacts have 30-day retention; aggregate artifacts have 90-day retention. OBS-001 should standardize the outputs and create one manifest/report index rather than replace this pipeline.

The environmental JSONL header includes requested duration and planned ticks but not actual elapsed wall-clock time, commit/build SHA, GitHub workflow/run ID, or analyzer versions. Those values can be attached by the workflow in a compact sidecar manifest.

### PR #392 identity join specifics

The PR adds `source_material_ids`, contact unit indices, and contact endpoints to COMBINE and DISPLACEMENT reaction events, which permits a stronger sample-to-outcome join for those event types. Its BREAK event construction currently supplies an empty source-ID vector and no contact endpoint tuple. Therefore it does not yet provide uniform exact participant identity across all three reaction types. The paired comparison analyzer correctly fails closed on missing or incomplete causal exposure/denominator requirements; preserve that behavior.

### PR #345 analysis specifics

The PR adds an observed-stage funnel to the chemistry analyzer using discovery counters already present in its branch, and adds paired-seed batch analyzers for dispersion and starting-mixture matrices. These are valuable aggregation patterns, but they must not turn stage counters into a denominator unless the search semantics prove exhaustive coverage. The experimental matrices also should remain distinct experiment records with explicit conditions rather than being silently treated as routine baseline telemetry.

## Remaining analyzer and energy-boundary audit

The current `tools/` directory contains nine relevant analysis utilities:
- `chemistry_analyzer.py`: per-run reaction/energy/bond attribution and lifecycle-derived chemistry.
- `chemistry_organization_analyzer.py`: molecular identity, structural-family recurrence, lineage, and persistence.
- `chemistry_experiment_batch_analyzer.py`: cross-seed aggregation of per-seed chemistry/organization reports.
- `reaction_network_analyzer.py`: compound families, family transitions, pathways, and CSV exports.
- `geometry_aware_reaction_network.py`: instance-level reaction network enriched with geometry descriptors.
- `contact_opportunity_analyzer.py`: bounded sampled opportunity and search funnel summaries.
- `temporal_chemical_complexity_analyzer.py`: fixed-window complexity, novelty, and candidate family-graph cycles.
- `chemistry_batch_analyzer.py` and `audit_geometry_storage.py`: specialized batch/geometry audit utilities, not part of the standard environmental workflow.

The standard workflow already calls the first seven in a staged manner (per-seed chemistry and organization, then aggregate chemistry, reaction network, geometry-aware network, contact opportunities, and temporal complexity). Each report family currently has its own schema version (mostly version 1) and its own JSON/Markdown/CSV conventions. The pipeline is capable but lacks a common top-level experiment manifest and report index that states which subreports are present, their analyzer versions, validation status, and coverage warnings.

### Energy settlement boundaries

The energy audit traced the environmental COMBINE, DISPLACEMENT, and BREAK commit paths:

- COMBINE resolves a cloned trial through the shared COMBINE authority and stages the ledger/usable-energy holder before atomically replacing source material. The resolver currently returns a boolean; the reaction event records required work but not the authoritative signed structural delta, usable-energy delta, work, heat, or exact settlement result.
- DISPLACEMENT evaluates a coupled replacement and retains a `ReactionEnergyBalance` on the trial. At commit it stages compatible resonance withdrawal, remaining usable-energy funding, the conservative settlement transaction, and ledger updates before mutating the field. The reaction event currently records activation/work context but not the transaction breakdown or resonance withdrawal.
- BREAK stages compatible resonance withdrawal and usable-energy payment, calls the chemical-break energy preparation authority, then commits the split and ledger state. The event records required work and activation but not the exact funding split or resulting transaction values.
- The authoritative `EnergyTransaction` contract represents signed structural delta, usable-energy delta, physical work, and heat; environmental reaction paths also update resonance-held totals where applicable. These should be observed at the authoritative settlement boundary, not recalculated in the diagnostics recorder or an offline analyzer.

**Implementation implication:** add a small serializable settlement summary returned or captured by the existing commit authority, and attach it to the same stable reaction event. This must be observational only: no duplicate formulas, no second ledger, and no changes to transaction decisions. Include source/frequency/resonance-withdrawal details where the authoritative path already knows them.

## Analyzer consolidation opportunity

Do not combine all domain-specific analyses into one monolithic script. Keep specialist analyzers focused, then add a thin orchestration/report-index layer:
1. Validate raw JSONL and manifest.
2. Run or reuse specialist outputs.
3. Verify that every expected output exists and has compatible seed/duration/provenance.
4. Produce one compact index/summary linking subreports and surfacing data-quality/coverage flags.
5. Preserve full JSON/CSV details as drill-down artifacts.

This is a lower-risk way to make results feel like one coherent lab notebook without discarding existing analytical depth.

## Artifact-size case study: EXP-2026-006, seed 42

A retained seed-42 artifact from workflow [#44776501166](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/44776501166) was downloaded and inspected without rerunning the simulation.

- Compressed GitHub artifact: about 28.6 MB.
- Uncompressed contents: about 526 MB total; the raw JSONL alone is **520,625,329 bytes** (about 497 MiB).
- The JSONL contained 19,355 `environmental_chemistry_tick` records (313.0 MB) and 310,060 `environmental_reaction` records (202.4 MB). Periodic snapshots, lifecycle records, and local resonance samples together were only about 5.3 MB.
- The tick records' bounded COMBINE `opportunity_samples` field accounted for about **220.4 MB** by itself. It contains at most eight samples per tick, but 8,443 ticks had all eight sample slots populated (67,532 detailed samples). “Bounded” therefore does not automatically mean small.
- The 310,060 reaction records included 309,680 BREAK attempts, exactly 16 per diagnostic tick. Of those BREAK attempts, 309,344 were rejected for `insufficient_usable_energy`, 41 for `stale_source_missing`, and 295 committed. COMBINE and DISPLACEMENT contributed 294 and 86 committed events respectively in this seed.

This is one seed from one diagnostic workflow, not a universal size estimate. It demonstrates that the largest data costs are repeated high-frequency records and detailed sampled geometry—not periodic world snapshots or molecule lineage. The compressed artifact size hides the much larger expanded JSONL footprint.

**Data-volume design consequence:**
1. Preserve every committed reaction and its exact lifecycle/energy transaction.
2. Preserve each distinct rejection/deferral mechanism and the candidate identity where scientifically meaningful, but consider compact per-tick/per-source aggregation for repeated identical high-frequency failures after validating that no unique state is lost.
3. Make detailed contact/neighborhood samples explicitly opt-in or objective-triggered; keep compact coverage counters in routine runs and retain a deterministic bounded sample only when useful.
4. Do not simply discard the 16 BREAK attempts per tick: first add stable source IDs and establish whether these are repeated attempts on the same sources, distinct candidates, or state changes that matter.
5. Keep raw artifacts compressed and avoid re-uploading the same raw JSONL into aggregate artifacts. The aggregate should contain derived reports and a manifest/index, with raw data referenced by artifact/run URL.

The first-pass case study confirms the original design goal: comprehensive records can be substantially smaller if we retain high-value events and coverage while avoiding repeated copies of detailed diagnostic structures.

A workflow review also clarified the aggregate-artifact footprint: the PR #392 reaggregation job does **not** upload the raw JSONL again in its combined artifact. It uploads derived reports. However, the contact-opportunity analyzer writes the same sampled opportunities in the main JSON report and separate JSON/CSV files, and writes tick diagnostics in both the main JSON report and a separate CSV. It does this per seed. The 236,988,261-byte aggregate artifact from workflow #78 therefore deserves a file-level size audit; the likely duplication is among derived sample/tick exports, not a second raw-data upload. Keep one canonical machine-readable representation and render CSV/Markdown on demand or as small summaries where possible.

## Open-PR focused-test review

- PR #345's added chemistry-analyzer test verifies that existing BREAK/COMBINE/DISPLACEMENT stage counters appear in the funnel. The two new batch-analyzer suites verify 15-run aggregation and reject dispersion/inventory metadata mismatches. These tests validate aggregation and metadata consistency, but they do not prove that the stage counters are complete opportunity denominators.
- PR #392's contact-analyzer test verifies one successful exact contact-key join from a sampled COMBINE opportunity to a reaction event and one neighborhood summary. It does not establish exhaustive opportunity coverage or test all reaction kinds.
- PR #392's matched-catalysis tests verify paired-arm inventory/energy mismatch rejection and fail-closed behavior when time-resolved exposure/background resonance are missing or candidate placement changes flow. The tests intentionally do not demonstrate a successful causal-eligibility gate, consistent with the implementation's current scientific blocker.

**Test reuse decision:** Keep the PR #392 fail-closed behavior and exact-key join approach, but expand tests before treating the schema as general: BREAK identity joins, DISPLACEMENT and COMBINE joins, duplicate/ambiguous matches, missing identity, truncated neighborhoods, zero-sample ticks, complete-vs-incomplete denominators, time-resolved exposure parity, and schema-version mismatch. For PR #345, preserve the funnel as a stage summary, but label rates/denominators ineligible unless search coverage semantics establish the full eligible set.

## Test inventory and coverage notes

The seven analyzers used by the standard workflow each have a dedicated Python test file on main:
- Chemistry: 12 tests.
- Molecular organization: 13 tests.
- Cross-seed chemistry batch: 2 tests.
- Reaction network: 3 tests.
- Geometry-aware network: 5 tests.
- Contact opportunities: 2 tests.
- Temporal complexity: 7 tests.

The current test inventory emphasizes lifecycle correctness, structural topology, bond attribution/reconciliation, geometry invariants, time-window analysis, and basic opportunity aggregation. The relatively small contact-opportunity and batch-analyzer suites warrant closer review for truncation/coverage semantics, denominator misuse, missing provenance, malformed/mixed-version inputs, and cross-seed consistency. No common manifest/report-index validator exists in the standard workflow.

## Audit limitations and next work

The first-pass source audit now covers the main environmental recorder, event/lifecycle and tick-diagnostic structs, chemistry and contact analyzers, all seven standard workflow analyzers and their test-file inventory, the standard experiment workflow, and environmental COMBINE/BREAK/DISPLACEMENT energy settlement paths. PR #345 and PR #392 metadata, changed-file inventories, and selected source/analyzer patches have also been inspected.

The audit is sufficient to define the core schema priorities, but not yet sufficient to start changing production diagnostics. The remaining pre-implementation work is:
1. Review the focused tests in PR #345/#392 and the weakest main analyzer suites, especially contact-opportunity and batch aggregation.
2. Reconcile branch divergence and identify which open-PR changes can be safely reused or should remain separate.
3. Reanalyze retained EXP-2026-004, EXP-2026-006/PLAN-002, and resource-feed artifacts to identify which proposed fields can already be reconstructed offline.
4. Draft the field-level schema and manifest contract against the concrete event/energy call sites.
5. Select the smallest implementation slice, add focused tests, and keep all chemistry/physics behavior unchanged.

No code was changed and no experiment was launched during this audit.


## Follow-up audit checkpoint — PR test semantics and branch reconciliation (2026-11-20)

### PR #345 batch aggregation review

The two new PR #345 batch analyzers require all five expected seeds and validate key experimental metadata. The starting-mixture aggregator also compares the full initial elemental inventory across mixtures for each seed. This is useful and reusable.

Remaining limitations found in the source/tests:
- The dispersion aggregator sums funnel-stage fields while defaulting absent fields to zero. It does not preserve a distinction between a measured zero and a missing stage field.
- Neither batch report establishes that the upstream funnel counters represent exhaustive eligible opportunities. They must remain stage/count summaries, not opportunity-normalized rates.
- The batch tests cover successful 15-run aggregation and selected metadata/inventory mismatches, but do not cover missing-vs-zero funnel fields, mixed analyzer/schema versions, missing/mismatched organization provenance, incomplete or unexpectedly short runs, duplicate run inputs, or cross-condition seed alignment.
- The run reports check requested duration and initial object counts, but the batch contract still needs actual elapsed duration, final tick count, exact source commit/workflow provenance, and analyzer version references in a common manifest.

### Main contact-opportunity and cross-seed analyzer review

The main contact-opportunity analyzer explicitly documents that broad-phase candidates, inspected pairs, valid interfaces, and interfaces submitted to chemistry are different stages; it also warns that bounded search windows leave uninspected pairs. This is scientifically appropriate. Its two tests, however, only cover one aggregated funnel example and a missing artifact. They do not test truncated-window accounting, zero-activity intervals, mixed schema versions, malformed numeric values, missing counters versus measured zero, or invalid duration/seed metadata.

The main cross-seed chemistry batch analyzer has stronger coverage for material inventory, lifecycle validation, bond-graph reconciliation, and multi-seed aggregation. Its two tests still do not validate a common provenance manifest, analyzer-version compatibility, missing-vs-zero diagnostic fields, or uncertainty/statistical reporting. These should be addressed in the common validation/report-index layer rather than duplicated in every specialist analyzer.

### PR #392 identity and test boundary

The PR adds exact source material IDs and contact descriptors to COMBINE and DISPLACEMENT paths. Its current BREAK event construction still does not provide matching participant IDs/contact descriptors, so event identity is not yet uniform across all reaction kinds. The contact analyzer's added test demonstrates one successful COMBINE sample-to-outcome join; it does not validate BREAK/DISPLACEMENT joins, duplicate/ambiguous event matches, or coverage completeness. The matched-catalysis tests deliberately validate refusal when controls are not adequate; there is no demonstrated successful causal-eligibility path. Keep the fail-closed gate.

### Branch reconciliation snapshot

A GitHub compare against current `main` reports:
- PR #345 branch `chemistry-dispersion-observability`: **36 commits ahead and 716 behind**; the comparison reports `diverged`. This is substantially stale. Do not attempt to merge it as a whole. Treat it as a source of selectively reusable experiment design, funnel fields, and batch-analysis patterns, then port only what remains useful onto current main after field-level review.
- PR #392 branch `chemistry/plan-002-neighborhood-matched-controls`: **91 commits ahead and 4 behind**; the comparison reports `diverged`. It is much closer to main, but remains a large, unmerged change set with causal-analysis blockers. Reconcile it in focused slices; do not merge wholesale or authorize the paired causal experiment.
- OBS documentation PR #397 branch `docs/chemistry-observability-plan`: **18 commits ahead and 0 behind** at this check. Its latest head `ab23256b5fadb2a05f41657a0b31adf39854dde6` has a completed successful Rust workflow run `44802636455`. This is a documentation-only PR; do not merge without the user's explicit approval.

### Updated audit decision and next work

Priority 1's focused PR/test review and branch comparison are now complete. The reusable work is:
1. PR #345: controlled dispersion/starting-mixture experiment design and per-seed aggregation patterns, subject to a modernized provenance/coverage contract.
2. PR #392: local-neighborhood sampling, exact COMBINE/DISPLACEMENT participant joins, and the fail-closed matched-control preflight. Keep sampling explicitly bounded and causal eligibility blocked until full-run exposure and denominator gates are satisfied.

Do not port the old branches wholesale. Next, reanalyze retained EXP-2026-004, EXP-2026-006/PLAN-002, and resource-feed artifacts; then draft the canonical manifest and event/opportunity/energy schema against current main. No simulator behavior or diagnostic code has been changed in this follow-up audit, and no new experiment has been launched.


## Existing-artifact reanalysis checkpoint — EXP-2026-006 report footprint (2026-11-20)

Downloaded and inspected the already-completed combined analysis artifact from workflow [#44792658175](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/44792658175), artifact `chemistry-experiment-combined`. No simulation was rerun.

- ZIP size: **236,988,261 bytes** (about 237 MB).
- ZIP entries: 56 files.
- Total uncompressed content: **10,748,880,670 bytes** (about 10.75 GB).
- The `contact-opportunities/` directory accounts for **10,660,578,639 bytes** (about 10.66 GB) across 30 files.
- Across the artifact, JSON files account for about 8.70 GB uncompressed and CSV files about 2.05 GB. Compression is strong, but extraction and downstream processing still face the expanded size.

The cause is confirmed in PR #392's `tools/contact_opportunity_analyzer.py`: the returned report already contains the complete `tick_diagnostics`, `opportunity_samples`, and `neighborhood_samples` arrays. The writer then serializes those same arrays again into separate `contact-opportunity-samples.json`, `contact-opportunity-samples.csv`, `contact-opportunity-neighborhoods.csv`, and `contact-opportunity-ticks.csv` files. The archive therefore stores the large sampled datasets in the main analysis JSON and again as standalone exports. For example, per-seed main analysis JSON files range from roughly 1.0 to 1.6 GB uncompressed, while standalone sample JSON files add roughly 386–621 MB and sample CSVs another 211–341 MB per seed; neighborhood CSVs add another 111–184 MB per seed.

**Decision:** The unified reporting layer should have one canonical machine-readable representation for each large diagnostic dataset. The compact top-level report should contain summaries, counts, coverage/truncation flags, and references to drill-down artifacts. Do not package the same full arrays in both a monolithic JSON report and duplicate JSON/CSV exports by default. CSV should be generated on demand or only for specifically requested downstream use. This is a high-value consolidation target and can reduce artifact size and extraction/analysis cost without losing scientific evidence.

### Active-run status check

Resource-feed paired workflow [#44784506988](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/44784506988) was still running when checked: 8 observation jobs had completed and 2 remained in progress. Six per-seed artifacts were visible at that check. Do not mark EXP-2026-005 complete until both remaining jobs, all ten artifacts, and the final aggregation are verified.

### Documentation PR status

After the audit update, PR #397 head is `fa7fc56fdcb72024ea84e9f239b37c76a65cec4e`. Its Rust workflow [#44803567727](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/44803567727) was still in progress at the latest check. No merge was performed.


## Source-level schema confirmation and first instrumentation slice (2026-11-20)

The exact event-construction paths in `src/simulation/simulation.rs` were reviewed after drafting OBS-001 schema v1:

- The environmental chemistry event vector is cleared at the beginning of each chemistry tick, then the tick is incremented before event resolution. Each reaction event is appended in deterministic resolution order.
- COMBINE and DISPLACEMENT operations retain cloned `source_a` and `source_b` physical materials; BREAK retains its `source_material`. Their `environmental_identity` values are available before the resolver mutates the live field. These are the right source for event participant IDs even when molecular-history recording is disabled.
- The current lifecycle ID `tick-ordinal` is generated before the event is appended. Promoting that exact value to the enclosing reaction event is a minimal compatible join. The run manifest will scope it to one run.
- The event's cell/material indices remain useful location diagnostics but must not be used as identity.
- The chemistry-lab test fixture had one source-composition entry despite two parent IDs; it was corrected to represent two source participants and now asserts the event ID, ordered participant IDs, and lifecycle ID agree.

**First implementation slice:** draft PR [#398](https://github.com/coringilbert16-cmyk/EvoSim-/pull/398) adds `reaction_id` and ordered nullable `source_material_ids` to environmental reaction events for COMBINE, DISPLACEMENT, and BREAK. It reuses the existing lifecycle ID; it does not change reaction behavior, scheduling, energy calculations, or diagnostic volume. The PR is draft and CI is running. Do not treat it as validated or merge it until all checks are green and the diff is reviewed.

The canonical schema proposal is [OBS-001-schema-v1-proposal.md](plans/OBS-001-schema-v1-proposal.md). Its next open questions are exact per-reaction energy funding capture and denominator/coverage semantics; neither should be guessed from activation or existing aggregate counters.

### Current next actions

1. Allow PR #398 full CI to finish; fix any formatting, compile, test, or Clippy failures.
2. Review analyzer compatibility: the geometry-aware network analyzer already looks for an event-level `reaction_id`, so this addition closes its previously indirect join; add/adjust regression tests if needed.
3. Finish exact energy-settlement source review at the COMBINE/BREAK/DISPLACEMENT commit boundaries before proposing a read-only settlement summary.
4. Design the opportunity coverage fields against each discovery algorithm's actual search domain; keep all incomplete denominators fail-closed.
5. Dispatch the new aggregation-only recovery workflow on PR #394's branch to generate the missing specialist reports from the ten retained PLAN-004 artifacts. No observation rerun is required.


### First identity-join slice validation — 2026-11-20

Draft [PR #398](https://github.com/coringilbert16-cmyk/EvoSim-/pull/398) now passes its full GitHub Actions validation at head `f590c446935638ab51a7f1542b027e6db5035fc4`:
- Rust formatting: success (after applying the formatter's two requested layout changes).
- Repository architecture checks: success.
- Offline analyzer tests: success.
- Full Rust tests: success.
- Shared environmental chemistry smoke simulation: success.
- Chemistry and organization analysis of the smoke output: success.
- Clippy: success.

The PR remains draft/open and unmerged pending review. The initial check failed only at formatting; the two formatter changes were applied, and the subsequent full run passed. No chemistry or energy behavior changed. This validates the first identity-only instrumentation slice but does not validate the complete OBS-001 schema or settle the remaining energy/opportunity gaps.
