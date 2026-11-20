# OBS-001 — Compact permanent experiment record contract

**Status:** Proposed implementation contract for routine environmental chemistry runs.  
**Schema:** `evosim.permanent_experiment_record`, version 1.  
**Retention principle:** preserve validated scientific results and provenance; treat raw diagnostics and intermediate analyzer products as temporary working data.

## Purpose

A completed experiment must remain comparable after its raw JSONL and large intermediate files expire. The permanent record is the durable scientific result. It is not a copy of the simulation trace.

A record is emitted per independent seed plus one compact experiment-level comparison record. Records are small JSON documents suitable for committing alongside the experiment result note in `experiments/records/results/`. Routine CI artifacts are transport, not the permanent archive.

## Required top-level fields

- `schema`, `schema_version`: exact schema identifier and integer version.
- `record_status`: `complete`, `partial`, or `invalid`.
- `identity`: experiment ID, research-plan ID, condition ID, seed, and independent replicate count where applicable. Unknown values remain null.
- `provenance`: source commit, workflow run ID/URL when available, simulator version, analyzer versions, manifest schema/version, configuration SHA-256, source diagnostic SHA-256, and source byte count. Raw source paths are references only and do not imply permanent retention.
- `conditions`: requested and actual wall-clock duration, start/end tick and tick count, seed, canonical configuration digest, initial inventory counts, initial constituent count, active resources, and relevant environment/field settings available from the manifest.
- `inventory`: initial/final counts by resource and material category, total initial/final physical materials and constituents, and explicit missingness.
- `reactions`: per reaction type (BREAK, COMBINE, DISPLACEMENT), candidate/ready, scheduled, committed, rejected, failure-reason counts, and discovery-work/budget counts where available. Include event counts and committed reaction counts. Do not store full event arrays.
- `opportunity_coverage`: per reaction type, denominator/domain, visited and skipped counts, coverage state, completeness, and whether a rate is valid. Preserve unknown/null separately from measured zero.
- `settlement`: compact per-reaction totals for usable-energy delta, resonance withdrawn, structural-energy delta, physical disruption/work, and heat dissipated; count of settled and unsettled events; residual summaries (count, min, max, mean, max absolute). Never substitute zero for unavailable or unsettled values.
- `molecular_development`: molecule creations/destructions, active molecules at end, lineage edge count, lineage validation status, mean observed lifetime when defined, compound count, unique/recurrent structural-family counts, family transitions summarized by reaction and family pair, and elemental material incorporation totals. Do not retain per-molecule lifecycle snapshots or full lineage edge lists.
- `organization`: compact aggregate measures needed for cross-seed comparisons, including structural family counts, composition/topology summary statistics, and time-window novelty/complexity summaries where available. Do not retain all per-window event details.
- `quality`: validation errors, warnings, missing fields, partial/unknown coverage, analyzer failures, and explicit interpretation limitations.
- `evidence`: references to exceptional retained evidence by artifact ID/path, reason for retention, expiry/review date, and checksum. Empty by default.

## Comparison rules

1. Seeds are independent replicates; events within a seed are not independent samples.
2. Compare conditions when schema, active-resource set, duration interpretation, diagnostic interval, and measurement definitions are compatible. Configuration fingerprints distinguish conditions; they do not by themselves make two intentionally different conditions incomparable.
3. Keep per-seed values alongside aggregate descriptive statistics. Never replace seed-level values with pooled event counts alone.
4. Compute rates only when numerator and denominator are present, complete, and semantically compatible.
5. Treat null as unknown/unavailable, zero as measured zero, and partial coverage as partial. Never infer missing data.
6. Record any excluded seed and the exact incompatibility reason.
7. The experiment-level record contains compact per-seed references and descriptive aggregates; it does not duplicate detailed per-seed payloads.
8. Between-condition differences are descriptive only; retain per-seed values and do not infer causality or statistical significance from event counts.

## Retention classes

| Class | Contents | Default retention |
|---|---|---|
| Permanent | Per-seed compact record, experiment comparison record, short human-readable result note, checksums/provenance | Repository experiment records; retained with project history |
| Routine temporary | Raw JSONL, per-seed specialist reports, normalized inputs, intermediate network/temporal outputs | Workflow artifacts: 7 days maximum; delete workspace copies after record validation |
| Deep temporary | Exhaustive contact opportunities, detailed local-resonance samples, expanded geometry/time traces | Explicit opt-in only; 1 day by default; no upload unless explicitly requested |
| Reference evidence | A deliberately selected baseline or exceptional diagnostic evidence required to reproduce/resolve a scientific finding | Explicitly allowlisted; reviewed individually, not retained by default |

Artifact expiration is not a substitute for cleanup: workflow jobs must remove temporary workspace files after successful compact-record validation. Failed or incomplete runs may retain temporary evidence for diagnosis, but must be clearly marked and expire automatically.

## Routine versus deep collection

Routine collection must retain enough source measurements to produce the fields above, but must not emit exhaustive per-tick local-resonance samples, full inventory snapshots at a high fixed frequency, full lifecycle graph copies beyond what is required by the analyzer, or detailed contact-opportunity CSVs.

Deep diagnostics are explicit opt-in. They may increase sampling/detail and retain selected raw evidence temporarily. A deep run must state its scientific question and retention reason. The default is off.

## Acceptance criteria

- A schema validator rejects malformed records and preserves null/zero/partial distinctions.
- The builder derives records from existing specialist analyzer outputs without inventing values.
- A comparison command accepts compact records only; it does not require JSONL, manifests, or specialist intermediate reports after the compact record has been validated.
- Comparison tests cover compatible records, differing conditions, missing fields, partial opportunity coverage, and multiple independent seeds.
- Routine workflow uploads only compact records and concise reports as its durable artifact; raw JSONL and intermediate files are temporary and expire automatically.
- Detailed contact output and deep diagnostics are opt-in and temporary.
- The permanent record remains sufficient to compare new runs after temporary artifacts have expired.
- No simulator, chemistry, physics, reaction discovery, scheduling, or energy-settlement behavior changes.

## Versioning

Changes that alter field meaning or comparison semantics require a schema-version increment and migration/compatibility policy. Additive optional fields may remain within version 1 when their missingness semantics are explicit.
