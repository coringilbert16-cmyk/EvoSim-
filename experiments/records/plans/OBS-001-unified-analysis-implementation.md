# OBS-001 — Unified chemistry analysis implementation

**Status:** Active engineering/observability initiative; two-file retention implementation in PR #404, historical pilot validation pending.  
**Baseline:** PRs #398, #400, #401, #402, and #403 are merged.  
**Scope:** Offline, reproducible analysis of environmental chemistry artifacts.  
**Behavior boundary:** No changes to chemistry, physics, reaction discovery, scheduling, activation, or energy settlement.

## Objective

Turn the specialist chemistry analyzers into a coherent repository-side pipeline that validates provenance, reuses derived outputs, and produces one consolidated report per experiment/condition. The pipeline must analyze retained artifacts without rerunning simulations and must not require routine manual retrieval of raw files.

OBS-001 is engineering infrastructure, not a scientific hypothesis or experiment. It must not be assigned an EXP identifier or treated as evidence for catalysis, autocatalysis, metabolism, or spatial organization.

## Completed foundations

- **PR #398:** stable reaction identities and event-linked settlement telemetry.
- **PR #400:** reaction-specific opportunity coverage semantics; missing values and incomplete search coverage remain explicit.
- **PR #401:** versioned environmental run manifest, configuration digest, artifact checksum, and experiment metadata fields.
- **PR #402:** unified offline orchestration, manifest/raw-input validation, cached analysis stages, and report/index foundation.
- **PR #403:** compact record builder, compact comparison tooling, settlement summaries, and summary-only opportunity coverage. Its per-seed permanent-record retention model is superseded by the two-file policy in the [permanent record contract](OBS-001-permanent-experiment-record.md).
- **OBS-001-opportunity-coverage-audit.md:** discovery-domain and coverage audit, including historical EXP-2026-004 validation.
- **OBS-001-run-manifest-implementation.md:** manifest contract and known limitations.

## Required permanent outputs

Each future experiment/condition leaves exactly two permanent files in `experiments/records/results/`:

1. A setup/conditions file containing the exact source commit, configuration, seeds, duration, initial inventory, environment, enabled features, command-line/runtime settings, and other information needed to recreate the run.
2. A consolidated scientific report containing validated per-seed values, cross-seed summaries, provenance, analyzer versions, validation status, missingness, conclusions, and interpretation limits.

Do not preserve separate per-seed JSON records, standalone comparison records, analyzer indexes, or result-note files. Historical records remain unchanged. The complete contract and retention rules are defined in [OBS-001 permanent experiment record](OBS-001-permanent-experiment-record.md).

## Phases

### Phase 1 — Analyzer inventory and compatibility matrix

Completed on current main. The inventory documents CLI/API entry points, required inputs, output schemas, provenance, coverage, caching, dependencies, scaling limits, and tests. Do not count draft PR capabilities as merged production behavior. Keep matched-catalysis gates fail-closed.

### Phase 2 — Canonical analysis index and stage contracts

The temporary analysis index links manifest → raw JSONL → validated inputs → specialist outputs → derived datasets → consolidated report. It is a working artifact only and must be deleted after report validation. It must record experiment/condition/seed identity, source checksums, schema/analyzer versions, output checksums, stage status, and explicit quality warnings. It must not be committed or retained as a permanent deliverable.

### Phase 3 — Offline orchestration

The orchestrator must:

1. accept a directory or experiment artifact set;
2. validate schema, manifest/config digest, raw artifact checksum, provenance, and seed coverage;
3. run applicable specialist analyzers in dependency order;
4. record stage fingerprints, versions, status, output paths/checksums, and errors in temporary working data;
5. reuse valid derived outputs when inputs and analyzer versions match;
6. process memory-heavy analyzers per seed/stream where appropriate;
7. consolidate per-seed findings into one report with cross-seed summaries;
8. fail closed for invalid required inputs while allowing optional analyzers to report unavailable/partial status;
9. never launch or alter simulations.

### Phase 4 — Standard consolidated report

The single consolidated report must contain:

- experiment identity, source commit, setup reference, and analyzer versions;
- requested/actual duration, tick counts, initial/final inventory, and material balance;
- per-seed and cross-seed reaction totals, identities, failure reasons, and rates only when denominators are valid;
- opportunity coverage and search-domain semantics;
- energy/resonance/physical-work/heat accounting and residuals;
- event/lineage joins and molecular organization/pathways where supported;
- analyzer status, missingness, discrepancies, and interpretation limits;
- observations, derived metrics, interpretations, and hypotheses separated clearly.

Seeds are independent replicates; events within a seed are not independent samples. A family cycle or neighborhood association is not proof of catalysis.

### Phase 5 — Historical pilot and acceptance

Use retained evidence only; no simulation reruns by default:

- **EXP-2026-004:** five-seed baseline; test legacy/manifest compatibility, coverage, chemistry, lineage, and energy.
- **EXP-2026-006 / PLAN-002:** retained neighborhood artifacts and analysis-only reaggregation; test event joins, bounded sampling, geometry, and causal ineligibility.
- **EXP-2026-005 / PLAN-004:** paired resource-feed artifacts; test condition metadata, matched-seed aggregation, and filename/layout normalization.

Compare unified results with existing specialist reports. Investigate discrepancies. Historical fields that cannot be reconstructed remain unavailable; never infer them. Pilot outputs are temporary until validated; then retain only the setup and consolidated report.

### Phase 6 — Two-file retention adoption

- Update the schema/record builder to emit the consolidated experiment-level report directly; per-seed values belong inside it, not in separate permanent files.
- Generate the setup record from the validated manifest and workflow inputs.
- Validate the consolidated report against specialist analyzer outputs before publication.
- Publish exactly the setup file and consolidated report to the repository experiment records.
- Only after successful validation and publication, remove raw JSONL, manifests, per-seed specialist outputs, caches, temporary indexes, and diagnostics from the runner.
- Do not upload raw/intermediate files or retain long-lived compact-record transport artifacts. Any unavoidable cross-job transfer must be short-lived and deleted after consolidation.
- Preserve exceptional raw evidence only with explicit approval, a scientific reason, and a documented expiry.
- Require each new measurement to state its scientific question, expected data volume, consumer stage, and acceptance test.

## Acceptance criteria

- The pipeline accepts experiment directories/artifact sets and performs analysis inside the repository workflow.
- Manifest, source checksums, configuration digests, required seed coverage, and analyzer outputs are validated.
- Existing historical artifacts can be analyzed offline; no simulation reruns are required for the pilot.
- Re-running analysis on unchanged inputs reuses valid temporary outputs and does not rerun simulation.
- Reaction identities, participant/lineage joins, settlement data, and opportunity coverage remain intact or are explicitly reported missing.
- Missing, null, sampled, partial, unknown, and measured zero are never conflated.
- No rate is emitted without a valid, complete, semantically appropriate denominator.
- The consolidated report contains compact per-seed values and cross-seed statistics and agrees with specialist reports or documents discrepancies.
- Exactly two permanent files are produced per experiment/condition: setup and consolidated report.
- No separate permanent per-seed records, analysis indexes, or comparison artifacts are retained.
- Cleanup is gated on successful validation and durable publication; failed runs preserve inputs.
- No simulator behavior or chemistry/physics rules change.

## Execution log

- 2026-11-20: Registered following the merge of PR #401.
- 2026-11-20: Analyzer inventory completed; see [analyzer inventory](OBS-001-analyzer-inventory.md).
- 2026-11-20: PLAN-004 / EXP-2026-005 analysis-only recovery run #44833658626 passed; ten source observations were reused without rerunning simulations.
- 2026-11-20: PR #402 merged the unified orchestrator, manifest/raw checksums, cache fingerprints, specialist analysis stages, and portable report/index foundation.
- 2026-11-21: PR #403 merged the compact-record builder, comparison tool, settlement summaries, and summary-only opportunity coverage. The original per-seed permanent-record design is now superseded by the two-file retention contract.
- 2026-11-21: User-approved retention decision: future experiments retain only setup/conditions and one validated consolidated report. Raw and intermediate data are temporary and may be removed only after successful validation and publication.
- 2026-11-21: Two-file retention implementation is in PR #404. The workflow consolidates short-lived raw inputs in the aggregate job; deep reaction-network, geometry, and temporal stages are deferred unless explicitly requested. The finalizer validates seed coverage, condition identity, measurement compatibility, manifests, and required pipeline stages before publication.
- 2026-11-21: Review found and fixed a missing `--analysis-index` argument in the workflow's finalizer invocation. Workflow permissions were narrowed so only the aggregate publisher receives contents/actions write access.
- 2026-11-21: The setup record now stores per-seed simulator arguments and profiling environment flags. The manifest records whether movement/tick profiling was enabled, rather than relying on workflow history to reconstruct those settings.
- 2026-11-21: Historical pilot artifacts were located and confirmed retained: EXP-2026-004 source run #44582565701 (five per-seed observation artifacts plus the specialist aggregate) and EXP-2026-006 source run #44776501166 (five per-seed neighborhood observation artifacts). Both predate canonical run manifests; they must be processed in legacy-compatibility mode and must not be marked as strict manifest-validated future records. No new simulation was launched.
- **Next:** finish CI on the latest PR head, then run repository-side analysis-only reaggregation of EXP-2026-004 and EXP-2026-006, compare unified outputs with their preserved specialist reports, and record any legacy gaps/discrepancies. Do not merge or launch a new simulation as part of this validation.
