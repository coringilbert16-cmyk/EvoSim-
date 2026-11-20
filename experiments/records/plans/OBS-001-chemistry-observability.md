# OBS-001 — Compact chemistry observability and experiment analysis

**Type:** Engineering/measurement infrastructure plan (not a scientific research PLAN).
**Status:** Approved; Phase 1 audit in progress.
**Tracking:** [GitHub issue #396](https://github.com/coringilbert16-cmyk/EvoSim-/issues/396).
**Scope:** Experiment records, diagnostic schemas, offline analyzers, and standard reports only.

## Objective

Give every EvoSim chemistry experiment a compact, reproducible scientific record that can support ongoing analysis and future research questions without logging the full simulation state every tick or overwhelming users with raw output.

This plan installs a common experiment-analysis system in layers. It does not authorize changes to chemistry, reaction scheduling, geometry, energy rules, resonance physics, or organism behavior.

## Operating principles

1. Capture events and meaningful changes, not repeated full-world state.
2. Give every run a canonical manifest and every record stable identifiers.
3. Separate required measurements from bounded, opt-in traces.
4. Record coverage and omissions alongside measurements; never present a bounded sample as a complete denominator.
5. Link reaction events to exact material identities, lifecycle, geometry, environment exposure, and energy/material transactions where available.
6. Preserve raw artifacts and do expensive reconstruction offline.
7. Every added field must answer a named scientific question and have an identified analysis consumer.
8. Preserve old schema versions and historical experiment artifacts; add versioned readers rather than silently rewriting old records.
9. Keep seeds as independent replicates; events within one seed are not independent samples.

## Data tiers

### Tier 1 — Required core record for every run

- Provenance: repository commit, executable/build identity, workflow/run ID, seed, diagnostic schema version, analyzer versions, artifact references.
- Conditions: requested and actual wall-clock duration, start/end tick, starting and ending inventory, starting molecule catalog/identities, environment and field configuration, dispersion, and interventions.
- Compact outcomes: BREAK/COMBINE/DISPLACEMENT totals, molecule count and size/topology summaries, inventory change, and energy balance.
- Compact time windows: reaction activity, inventory/complexity, and energy summaries. Window resolution must be fixed and documented for comparable experiments.
- Quality: run completion, missing/invalid records, conservation checks, diagnostic coverage and analyzer status.

### Tier 2 — Standard event and opportunity records

For every committed reaction and every opportunity actually evaluated by discovery/scheduling, capture compact records containing:

- Stable event ID, seed, tick, reaction type, lifecycle outcome, and rejection/defer reason.
- Exact source material, molecule, constituent, and bond identities when available; complete reactant/product graph snapshots on lifecycle transitions; parent/child lineage links.
- Geometry/interface outcome and compact distance, orientation, endpoint/contact descriptors.
- Activation and energy decision, with the transaction's available/required/transferred/stored/heat/loss/residual values or a stable reference to the authoritative transaction.
- Discovery funnel and coverage: search domain, candidate counts, inspected/evaluated/eligible counts, accepted/rejected/deferred/skipped work, budget exhaustion, truncation, and whether the denominator is complete.
- A reference to a compact environmental exposure window, rather than copying large environment arrays into each event.

Only claim a denominator for the domain actually exhaustively evaluated. Distinguish not discovered, not inspected, inspected but geometrically invalid, eligible, rejected by chemistry/activation/energy, deferred, committed, and unknown where the engine can support those distinctions. Do not manufacture hypothetical opportunities.

### Tier 3 — Bounded, opt-in research traces

Enable only for a registered question that needs them:

- Full local-neighborhood identities, composition, geometry, and contact opportunities.
- Time-resolved flow, hotspot, resonance/field exposure, and movement context.
- Detailed candidate-discovery traces or spatial maps.
- Other high-resolution data only where the expected analysis justifies the artifact cost.

Each trace must state its time/space scope, sampling policy, limits, and omissions. It must have a size/retention policy and must not silently imply complete coverage.

## Implementation phases

### Phase 1 — Inventory and gap audit

Audit current main and relevant open PRs. Map every existing data source and analyzer to Tier 1, 2, or 3. Mark each capability as reliable, partial/sampled, reconstructible offline, missing, or unverified. Identify overlap and dependencies before proposing code. Record findings in [the dated audit](../observability-audit-2026-11-20.md).

### Phase 2 — Canonical schema and run manifest

Define a compact, versioned record contract and stable identity/join rules for run → time window → event/opportunity → material lineage → energy/environment context. Specify units, nullable/unknown semantics, schema migration, and artifact provenance.

### Phase 3 — Close decision-relevant instrumentation gaps

Prioritize:
1. Reliable event-to-lifecycle-to-energy transaction joins.
2. Explicit discovery coverage and valid opportunity denominators.
3. Event-linked geometry and source identities.
4. Time-resolved environmental exposure only where needed for the planned scientific comparison.
5. Consistent provenance, units, and run manifest.

Do not duplicate fields already recorded correctly. Do not add telemetry without a defined analysis consumer. Do not change physical behavior.

### Phase 4 — Consolidated offline analysis

Build reusable offline stages for input/schema validation, run summaries, event and lineage reconstruction, reaction-network/hypergraph reconstruction, opportunity-normalized rates, spatial/neighborhood analysis, and energy/material accounting. Reuse cached/derived reports when possible; never rerun simulations just to regenerate analysis.

### Phase 5 — Standard reports

Generate one concise report per seed and a cross-seed report containing:
- Conditions, provenance, and run quality.
- Main reaction and opportunity funnels, including coverage limitations.
- Molecular inventory, complexity, lineage, and pathways.
- Energy/material accounting.
- Time-series and spatial context when measured.
- Observations separated from interpretations and hypotheses.
- Missing evidence and explicit reasons a causal conclusion is or is not eligible.

### Phase 6 — Existing-artifact pilot and validation

First reanalyze existing EXP-2026-004, EXP-2026-006/PLAN-002, and EXP-2026-005/PLAN-004 resource-feed artifacts (use the experiment registry's canonical IDs and links as they settle). Compare outputs with existing records. Do not repeat completed simulations. Run a small validation experiment only if an essential schema field cannot be validated from existing artifacts.

### Phase 7 — Adopt and extend

Make Tier 1 and Tier 2 standard for new chemistry experiments once validated. Add Tier 3 only through a short proposal stating the scientific question, exact fields, expected artifact volume, coverage limits, analysis consumer, and acceptance test.

## Acceptance criteria

- A single manifest identifies the exact run, code/build, conditions, seed, schema/analyzer versions, and artifacts.
- Every committed reaction can be joined to exact participant/product identities, lifecycle/lineage, and energy/material accounting, or the report explicitly marks the join incomplete.
- Opportunity funnels distinguish discovered, inspected, evaluated, eligible, rejected, deferred, committed, truncated, and unknown work without treating partial samples as complete denominators.
- Environmental exposure used for causal comparisons is time-resolved and linked to the relevant participants/events; tick-zero or legacy proxies are clearly labeled.
- Routine experiments produce a compact core dataset and standard readable report without full-state-per-tick logging.
- Detailed traces are opt-in, bounded, and explicit about coverage.
- Existing raw artifacts can be reanalyzed offline without repeating simulations.
- Independent seeds remain the statistical replicates.
- No chemistry, physics, energy, scheduling, or organism behavior changes are included.

## Existing implementation to reuse

- Permanent experiment registry and result-record policy in `experiments/records/`.
- Environmental JSONL recorder in `src/infrastructure/diagnostics.rs`.
- Existing chemistry, organization, temporal-complexity, contact-opportunity, and other offline analyzers in `tools/`.
- Opportunity funnel and matched-starting-mixture work in [PR #345](https://github.com/coringilbert16-cmyk/EvoSim-/pull/345), currently open/unmerged at the time of audit.
- Neighborhood, identity, exposure-preflight, and paired-analysis work in [PR #392](https://github.com/coringilbert16-cmyk/EvoSim-/pull/392), currently draft/open and intentionally blocked from causal execution pending coverage gates.

### Audit checkpoint update — 2026-11-20

The source audit confirms that the reaction event and lifecycle identity model is not yet fully joined: lifecycle records have a generated tick-local reaction ID, but the enclosing reaction event has no matching ID; rejected events generally lack lifecycle identity records; source material indices are not durable identities. The event also lacks the authoritative per-reaction energy settlement values, which are currently visible primarily in aggregate energy snapshots. COMBINE contact samples are bounded to eight per tick. These are explicit priorities for Phases 2–3.

The current standard workflow already runs several offline analyzers (chemistry, organization, reaction network, geometry-aware network, contact opportunity, temporal complexity) and retains per-seed artifacts for 30 days and aggregate reports for 90 days. Consolidate/orchestrate these rather than replacing them. The JSONL header has requested duration but not actual elapsed time, commit/build, workflow run ID, or analyzer versions; bind those in a run manifest.

**Audit progress:** first-pass source audit and workflow inventory completed for the main environmental recorder, reaction/lifecycle structs, energy ledger contract, primary chemistry/contact analyzers, and standard experiment workflow. Remaining work: inspect the other analyzers and their schemas, trace energy transaction call sites into environmental COMBINE/BREAK/DISPLACEMENT, and inspect PR #345/#392 detailed patches/tests before defining the final field-level schema.

### Audit checkpoint update — test and workflow inventory

The main standard workflow runs seven specialist analyzers and each has a dedicated test file: chemistry (12 tests), molecular organization (13), cross-seed chemistry batch (2), reaction network (3), geometry-aware network (5), contact opportunities (2), and temporal complexity (7). Contact-opportunity and batch aggregation tests are comparatively small and need focused review for denominator/coverage semantics and malformed or mixed-version artifacts.

The source audit now covers environmental reaction event/lifecycle/tick schemas, recorder behavior, all seven standard analyzers and their test inventory, the workflow orchestration, and environmental COMBINE/BREAK/DISPLACEMENT settlement boundaries. The remaining pre-implementation work is to review the focused PR #345/#392 tests and weakest analyzer suites, reconcile branch divergence, reanalyze retained artifacts, and draft the field-level manifest/event schema. Continue from the “Audit limitations and next work” section in the dated audit; no code or experiment run has been started.

### Artifact-size checkpoint — 2026-11-20

An existing EXP-2026-006 seed-42 artifact was inspected offline (no rerun). Its compressed ZIP is about 28.6 MB, but the raw JSONL expands to 520,625,329 bytes. The largest costs are 19,355 tick records (313 MB) and 310,060 reaction records (202 MB). Bounded COMBINE opportunity samples alone account for about 220 MB; 8,443 ticks carry eight detailed samples each. Of 309,680 BREAK attempts, 309,344 were rejected for insufficient usable energy. This is one seed, not a universal estimate, but it shows where compacting can matter: high-frequency repeated rejection events and detailed sampled geometry. Preserve committed events and coverage; do not discard failed attempts until stable source IDs and repeated-state semantics are understood. Full breakdown is in the dated audit.

**Next step:** inspect the remaining PR tests and run the existing offline analyzers on retained artifacts (without rerunning simulations) to establish which outputs can be regenerated and how much raw data can be omitted from aggregate uploads. Then draft the versioned manifest/event schema and propose bounded/opt-in detailed sampling. Current CI for documentation PR #397 is running; no merge has been performed.

### Open-PR test review checkpoint

Focused tests from PR #345 and PR #392 were inspected. #345 validates funnel aggregation and matched-run metadata but does not prove opportunity-denominator completeness. #392 validates a successful exact COMBINE sample-to-event join and fail-closed paired-arm inventory/energy/exposure gates; it does not yet test a successful causal eligibility path or comprehensive BREAK/DISPLACEMENT joins. The audit lists the additional regression cases needed before adopting this schema. Continue with artifact reanalysis and field-level schema drafting; do not assume these open PRs are production-ready or duplicate their implementation.

### Active workflow checkpoint — 2026-11-20 19:29 UTC

- EXP-2026-005 resource-feed paired run [#44784506988](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/44784506988) remains `in_progress`; five per-seed artifacts were present at the last check. Do not treat it as complete until all ten arms and final aggregation are verified.
- EXP-2026-006 analysis-only reaggregation [#44792658175](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/44792658175) is `completed/success`; reuse its combined artifact (236,988,261 bytes) rather than rerunning the observations.
- Documentation PR [#397](https://github.com/coringilbert16-cmyk/EvoSim-/pull/397) is open; latest head `22ff40a02ac4a3f23093f8578f8595a2f2629984`; CI run [#44802495110](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/44802495110) was `in_progress` at the last check. Do not merge until latest-head checks are green and the documentation is reviewed.
- Tracking issue [#396](https://github.com/coringilbert16-cmyk/EvoSim-/issues/396) contains progress comments and this plan's continuation status.

**Resume order:** (1) verify #397 latest CI and merge only after green/review; (2) verify completion and record EXP-2026-005 results without changing its experiment design; (3) inspect/reuse #78 outputs and PLAN-004 artifacts; (4) finish the manifest/event schema proposal; (5) implement the smallest necessary telemetry/report-index changes with focused tests. No simulator code has been changed under OBS-001.

## Current checkpoint

- Approval received: 2026-11-20.
- GitHub tracking issue: #396.
- Documentation branch: `docs/chemistry-observability-plan`.
- Phase 1 first-pass audit: main recorder and core chemistry/contact analyzers inspected; PR #345 and PR #392 metadata and changed-file scope inspected.
- Initial findings: see [observability audit](../observability-audit-2026-11-20.md).
- Next action: complete the remaining analyzer/workflow/source audit, then finalize the schema proposal and identify the smallest non-duplicative implementation slice.
- No simulator code or diagnostics have been modified; no new experiment was launched.
