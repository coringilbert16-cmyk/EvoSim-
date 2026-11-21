# OBS-001 — Two-file permanent experiment record contract

**Status:** Governing retention contract for future EvoSim experiments.  
**Schemas:** setup `evosim.experiment_setup` version 1; consolidated report `evosim.consolidated_experiment_report` version 1.  
**Retention principle:** permanently preserve only the reproducibility setup and the validated consolidated scientific report.

## Purpose

A completed experiment must be reproducible and scientifically reviewable after all raw simulation output and intermediate analysis files have been discarded. This two-file contract supersedes the earlier per-seed `evosim.permanent_experiment_record` version 1 model. The durable record consists of exactly two experiment-specific files:

1. **Setup record** — every condition and parameter needed to recreate the run.
2. **Consolidated report** — the validated scientific result, including seed-level measurements and cross-seed summaries.

Do not create separate permanent per-seed records, comparison records, analyzer indexes, or result-note files. Historical records are not rewritten by this policy.
The shared experiment registry remains the master index. After publication, update the registered experiment row with links to the setup and consolidated report for that condition. This shared index is not a third experiment-specific result file.

## Permanent file contract

### 1. Experiment setup

One file per experiment/condition, containing:

- stable experiment ID, research-plan ID, condition ID, and status;
- exact EvoSim source commit and repository revision;
- workflow/run identity and execution date;
- all independent seed IDs and replicate structure;
- requested and actual wall-clock duration, diagnostic interval, and relevant runtime settings;
- exact simulator command-line arguments and enabled/disabled features;
- canonical configuration and its digest;
- initial inventory counts, constituent counts, active resources, and molecule catalog where applicable;
- environmental and field parameters, including energy sources and boundary conditions;
- hardware/runtime details only where they materially affect reproducibility;
- known deviations, unavailable historical fields, and setup validation status.

The setup must be sufficient to recreate the experiment without relying on a raw diagnostic artifact or a prior conversation. Never invent missing historical values.

### 2. Consolidated scientific report

One file per experiment/condition, containing:

- experiment identity and a reference to the matching setup file;
- analyzer/tool versions and report-generation source commit;
- input validation and provenance-check results;
- per-seed measurements retained as compact rows/sections in this report;
- cross-seed descriptive statistics and variation;
- inventory/material balance, reaction identities and outcomes, energy settlements, molecular organization, and opportunity coverage where supported;
- denominator definitions and coverage completeness for every rate;
- missing, partial, sampled, unknown, and measured-zero distinctions;
- discrepancies against specialist analyzers and their resolution;
- observations, derived metrics, interpretations, hypotheses, and limitations clearly separated;
- final validation status and explicit exclusions.

The consolidated report is the sole durable scientific result. It must remain useful after every raw and intermediate file has been deleted. It must not claim completeness when a required analyzer or input is unavailable.

## Analysis and validation rules

1. OBS-001 performs analysis and consolidation inside the repository workflow; routine review must not require manually retrieving raw files.
2. Validate manifests, source checksums, configuration digests, seed coverage, required analyzer outputs, and report consistency before finalizing.
3. Retain seed-level values inside the consolidated report; seeds are independent replicates and events within a seed are not independent samples.
4. Compute rates only when numerator and denominator are present, complete, and semantically compatible.
5. Preserve null/unknown separately from measured zero and partial coverage.
6. Compare conditions only when measurement definitions and required provenance are compatible; record exclusions and reasons.
7. Validate the consolidated report against specialist analyzer outputs before cleanup.
8. If validation fails or is incomplete, preserve temporary inputs for retry/investigation and do not publish a completed report.
9. After successful validation and durable publication of both permanent files, delete all temporary raw and intermediate data.
10. Do not rerun a simulation merely to repair report assembly; reuse retained inputs when they are still available.

## Retention classes

| Class | Contents | Retention |
|---|---|---|
| Permanent | One setup file and one consolidated report per experiment/condition | Repository history |
| Temporary working data | Raw JSONL, manifests, per-seed specialist outputs, normalized inputs, caches, analyzer indexes, temporary JSON, logs, and diagnostics | Runner-local only; delete after validated publication |
| Exceptional evidence | Raw evidence retained only when explicitly approved for a specific scientific reason | Not part of the default policy; requires explicit approval and documented expiry |

Routine workflow runs must not upload raw inputs, manifests, intermediate analyzer outputs, per-seed record artifacts, or 90-day transport bundles. If temporary transfer between jobs is unavoidable, use a short-lived artifact and delete it after consolidation; it is not a permanent experiment deliverable.

## Acceptance criteria

- The permanent output is exactly two experiment-specific files: setup and consolidated report.
- No separate per-seed permanent records, analyzer indexes, or comparison artifacts are committed or retained.
- The setup file is sufficient to reconstruct the experiment conditions and execution.
- The consolidated report contains compact per-seed and cross-seed findings and remains useful without raw data.
- A validator rejects malformed/incomplete reports and preserves null/zero/partial distinctions.
- Specialist-output discrepancies are surfaced and resolved or explicitly left unresolved.
- Cleanup cannot run before successful validation and durable publication.
- Failed analysis preserves temporary inputs and cannot be marked complete.
- Future workflow runs do not upload raw or intermediate data as durable artifacts.
- No simulator, chemistry, physics, reaction discovery, scheduling, or energy-settlement behavior changes.

## Versioning

Version 2 replaces the earlier per-seed-record retention model. The schema version must increment again if field meaning or comparison semantics change. Historical v1 records remain historical and are not silently migrated or deleted.
