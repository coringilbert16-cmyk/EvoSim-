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
