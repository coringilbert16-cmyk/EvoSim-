# OBS-001 — Unified chemistry analysis implementation

**Status:** Active engineering/observability initiative; plan registered in this change.  
**Baseline:** PRs #398, #400, and #401 are merged.  
**Scope:** Offline, reproducible analysis of environmental chemistry artifacts.  
**Behavior boundary:** No changes to chemistry, physics, reaction discovery, scheduling, activation, or energy settlement.

## Objective

Turn the existing specialist chemistry analyzers into a coherent, repeatable pipeline that validates experiment provenance, reuses derived outputs, and produces a compact run report plus a cross-seed report. The pipeline must analyze retained artifacts without rerunning simulations.

OBS-001 is engineering infrastructure, not a scientific hypothesis or experiment. It must not be assigned an EXP identifier or treated as evidence for catalysis, autocatalysis, metabolism, or spatial organization.

## Completed foundations

- **PR #398:** stable reaction identities and event-linked settlement telemetry.
- **PR #400:** reaction-specific opportunity coverage semantics; missing values and incomplete search coverage remain explicit.
- **PR #401:** versioned environmental run manifest, configuration digest, artifact checksum, and experiment metadata fields.
- **OBS-001-opportunity-coverage-audit.md:** authoritative discovery-domain and coverage audit, including historical EXP-2026-004 validation.
- **OBS-001-run-manifest-implementation.md:** manifest contract and known limitations.

These are prerequisites to integration; do not reimplement them.

## Phases

### Phase 1 — Analyzer inventory and compatibility matrix

Inspect the current main branch and active/draft PRs. For each analyzer, document:

- CLI/API entry point and required inputs;
- output artifact and schema/version;
- provenance and event/lineage keys it preserves;
- complete, partial, sampled, or unknown coverage;
- whether results can be cached/reused offline;
- input/output dependencies and known scaling limits;
- validation tests and historical artifacts already exercised.

Include the chemistry, molecular-organization, reaction-network, geometry-aware network, contact-opportunity, temporal-complexity, cross-seed, energy/material, and manifest validators that are actually present on main. Do not count draft PR capabilities as merged production behavior. Preserve potentially useful PR #345 work as historical/superseded context; assess PR #392 separately and keep its matched-catalysis gates fail-closed.

Deliverable: a concise capability matrix and prioritized gap list in this plan or a linked audit document.

### Phase 2 — Canonical analysis index and stage contracts

Define a versioned analysis index that links, rather than copies:

run manifest → raw JSONL → validated inputs → specialist outputs → derived datasets → report

The index must include experiment/condition/seed identity, source artifact URI or workflow/artifact references, source checksum, schema version, analyzer version/commit, output checksum, status, and explicit missing/quality warnings. Bind post-run uploaded artifact identifiers here; do not mutate the immutable run manifest.

Specify stable contracts for each pipeline stage. Use structured machine-readable outputs (JSON) with a small human-readable summary. Keep null/unknown distinct from measured zero. Preserve reaction-specific denominator semantics and coverage. Avoid storing huge contact arrays in the index or report.

### Phase 3 — Offline orchestration

Implement a command-line orchestrator (or extend the existing analysis workflow if the inventory shows a suitable entry point) that:

1. discovers/accepts manifest and artifact inputs;
2. validates schema, manifest/config digest, and raw artifact checksum;
3. runs applicable specialist analyzers in dependency order;
4. records each stage's input fingerprint, version, status, output path/checksum, and errors;
5. reuses valid derived outputs when inputs and analyzer versions match;
6. supports analysis-only reaggregation and per-seed processing for memory-heavy analyzers;
7. emits the analysis index and final reports;
8. exits nonzero for invalid required inputs, while allowing explicitly optional analyzers to report unavailable/partial status.

The pipeline must not launch or alter simulations. It must not silently download, overwrite, or delete source evidence.

### Phase 4 — Standard reports

Produce:

**Per-run report**
- run provenance, conditions, elapsed wall time and actual ticks;
- initial/final inventory and material balance;
- reaction totals and time-series;
- opportunity funnel by reaction type, with coverage and denominator validity;
- event/lineage joins and molecular organization/pathways;
- energy/resonance/physical-work/heat accounting and residuals;
- spatial/environment context only where supported;
- analyzer status, missingness, artifact links, and interpretation limits.

**Cross-seed report**
- condition-grouped results and independent seed count;
- per-seed values and descriptive aggregate/variation;
- no treatment of within-run events as independent replicates;
- explicit exclusion/reason for missing or incompatible seed outputs;
- no pooled conversion rate unless denominators are valid and semantically comparable.

Separate **observations**, **derived metrics**, **interpretations**, and **hypotheses**. A family cycle or neighborhood association is not proof of catalysis.

### Phase 5 — Historical pilot and acceptance

First use retained evidence; no simulation reruns by default:

- EXP-2026-004 five-seed raw artifacts: test manifest/legacy compatibility, coverage, chemistry, lineage, and energy outputs.
- EXP-2026-006 / PLAN-002 retained per-seed neighborhood artifacts and existing analysis-only reaggregation: test event joins, bounded sampling, geometry, and explicit causal ineligibility.
- PLAN-004 / EXP-2026-005 resource-feed paired artifacts: test condition metadata, nonzero-tick external material identities, matched-seed aggregation, and filename/layout normalization. Recover and aggregate the ten retained observations; do not repeat the experiment solely to repair report assembly.

Compare unified results with existing specialist reports. Investigate every discrepancy and record any non-reconstructible historical field as unavailable rather than inferred.

### Phase 6 — Adoption

After the pilot passes:

- add the unified report/index to the standard environmental chemistry workflow;
- make the compact core analysis required for registered runs;
- keep high-volume traces optional and purpose-driven;
- update experiment documentation and result-record templates;
- require each new measurement to state its scientific question, expected data volume, consumer stage, and acceptance test.

## Acceptance criteria

- A manifest and analysis index provide reproducible links from run to all source and derived artifacts.
- Raw source artifacts remain immutable and checksum-verified.
- Each analyzer's version/schema and execution status are recorded.
- Re-running analysis on unchanged inputs reuses valid outputs and does not rerun simulation.
- Existing historical artifacts can be analyzed offline; large datasets are processed per seed/streaming where appropriate.
- Reaction identities, exact participant/lineage joins, settlement data, and opportunity coverage remain intact or are explicitly reported missing.
- Missing, null, sampled, partial, unknown, and measured zero are never conflated.
- No rate is emitted without a valid, complete, semantically appropriate denominator.
- Reports distinguish observations from interpretation and retain seed-level results.
- Automated tests cover invalid manifests/checksums, legacy schema, partial coverage, missing analyzer output, stale cache, deterministic index/report assembly, and aggregation naming variants.
- No simulation behavior or chemistry/physics rules change.

## Execution log

- 2026-11-20: Registered following the merge of PR #401.
- 2026-11-20: Phase 1 current-main analyzer inventory completed; see [analyzer inventory](OBS-001-analyzer-inventory.md). The initial index/orchestration design can proceed with a deliberate heavy-contact stage boundary.
- 2026-11-20: PLAN-004 retained artifact assembly failure traced to condition-prefixed artifact names. A tested normalization helper and analysis-only workflow path were added to the PLAN-004 branch; validation and reaggregation remain pending.
- 2026-11-20: PLAN-004 / EXP-2026-005 analysis-only recovery run #44833658626 passed; its observe matrix was skipped and the full specialist aggregate artifact was uploaded. The ten source observations were not repeated. This validates the paired artifact normalization/aggregation path; historical EXP-2026-004 and EXP-2026-006 remain the next unified-pipeline pilot targets.
- 2026-11-20: Initial unified orchestrator and tests added in PR #402. It validates manifest/raw checksums, creates per-seed chemistry and organization outputs with cache fingerprints, runs batch/network/geometry/temporal stages, writes a portable versioned index/report, and keeps detailed contact CSV generation opt-in. Rust CI run #44833728704 passed on implementation commit 327b256; the latest plan-log update is documentation-only.
