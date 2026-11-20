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

## Audit limitations and next work

This first pass inspected the main environmental recorder, chemistry analyzer, contact-opportunity analyzer, experiment execution guide, experiment registry, and plans guide; it also inspected PR metadata and changed-file inventories for #345 and #392. It did not yet audit every analyzer, event struct, workflow, or energy-ledger call site. No code was changed and no experiment was launched.

Next:
1. Complete the remaining analyzer/workflow and event-to-energy call-site audit.
2. Produce a field-level canonical schema and join map based on actual serialized event structs.
3. Reconcile #345 and #392 work against main before selecting implementation slices.
4. Reanalyze retained artifacts to test what can already be reconstructed.
5. Implement only gaps that cannot be resolved offline, with focused tests and no chemistry behavior changes.
