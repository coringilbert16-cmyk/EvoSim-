# OBS-001 — Existing analyzer inventory (Phase 1)

**Audit date:** 2026-11-20  
**Source:** merged repository main at `ac324f0`  
**Scope:** static entry-point, input/output, and integration audit; no simulations launched.

## Capability matrix

| Component | Inputs / entry point | Outputs | Coverage / constraints | Integration state |
|---|---|---|---|---|
| Run-manifest validator | `tools/experiment_manifest.py MANIFEST`; validates manifest and referenced artifact | Validation result/status | Validates schema v1, configuration digest, completed artifact size/checksum; only checks fields captured by current manifest | Required preflight stage; manifest is sidecar produced by simulator |
| Chemistry analyzer | `tools/chemistry_analyzer.py RAW.jsonl` | JSON + Markdown via `--json-out`, `--markdown-out` | Per-run chemistry, reaction/event/failure and energy/material summaries; legacy records supported with explicit missing coverage | First per-seed stage |
| Molecular-organization analyzer | `tools/chemistry_organization_analyzer.py RAW.jsonl` | JSON + Markdown | Molecular structure, lifecycle/lineage, families and material organization; validation errors exposed | First per-seed stage; pairs with chemistry report |
| Experiment batch analyzer | `tools/chemistry_experiment_batch_analyzer.py DIR --seeds ... --duration-seconds ...` | JSON + Markdown | Requires standardized `seed-N-chemistry.json` and `seed-N-organization.json`; validates fixed inventory and lifecycle; schema v1; currently requires a single exact file per seed | Cross-seed core aggregator; filename normalization is a separate adapter need |
| Legacy paired batch analyzer | `tools/chemistry_batch_analyzer.py DIR` | JSON + Markdown | Specialized older enabled/disabled-resonance paired 300-second report; not the general current six-element experiment aggregator | Keep as historical/specialized, not the unified pipeline's default |
| Reaction-network analyzer | `tools/reaction_network_analyzer.py DIR --seeds ... --duration-seconds ...` | Output directory of network files/reports | Reads per-seed chemistry JSON; lineage/family network is descriptive, not evidence of catalysis | Derived stage after chemistry |
| Geometry-aware reaction network | `tools/geometry_aware_reaction_network.py DIR --seeds ... --duration-seconds ...` | Output directory of geometry/network artifacts | Requires per-seed chemistry JSON and raw JSONL; committed geometry only, no complete eligible-contact denominator | Derived stage; causal geometry rates remain unsupported |
| Contact-opportunity analyzer | `tools/contact_opportunity_analyzer.py DIR --seeds ... --duration-seconds ...` | CSV samples, per-tick CSV, report | Raw JSONL; coverage-aware; BREAK domain complete when reconciled, COMBINE bounded, DISPLACEMENT partial/unknown where omitted work is unavailable. CSV can become extremely large | Heavy/optional stage until compact summary mode and volume policy are in place |
| Temporal complexity analyzer | `tools/temporal_chemical_complexity_analyzer.py DIR --seeds ... --duration-seconds ... --windows 12` | CSV + Markdown in output directory | Per-seed chemistry JSON; schema v1; graph SCCs/fingerprints are descriptive and explicitly limited | Derived stage after chemistry; run per seed/controlled batch when memory dictates |

## Current input naming contract

Most multi-seed analyzers expect a normalized input directory:

- raw: `seed-N.jsonl`
- chemistry: `seed-N-chemistry.json`
- organization: `seed-N-organization.json`

Paired experiments may preserve condition prefixes in downloaded artifact names. Normalize into a separate staging directory; never rename, overwrite, or modify source artifacts.

## Reuse and provenance findings

- Manifest v1 identifies the raw observation and hashes it, but analyzer-specific source hashes/output schema versions are not yet in the manifest.
- The experiment batch analyzer emits a combined report but does not provide a canonical pipeline index linking all derived outputs and their source fingerprints.
- Most analyzers are deterministic offline scripts but do not yet share a cache protocol or stage-result contract.
- Existing report outputs can be reused only with an explicit freshness check; filenames alone are insufficient.
- The contact-opportunity analyzer's detailed CSV output must not be made unconditional in the unified routine path. Retained EXP-2026-006 contact data reached roughly 10.66 GB uncompressed; use compact summaries, streaming/per-seed execution, and opt-in detail.
- Existing workflow aggregation runs several analyzers in one job without a shared index or standardized stage status. Failures can leave partial outputs without a unified quality manifest.
- There is no independent generic cross-seed analyzer beyond the experiment batch aggregation and specialized batch tool; cross-seed descriptive statistics need seed-level output retention.

## Prioritized gaps

1. **Artifact discovery/normalization:** standardized staging and explicit condition/seed manifest mapping.
2. **Analysis index:** versioned links/checksums/status for source, per-seed, aggregate, and optional heavy outputs.
3. **Stage contracts/cache:** analyzer script version/hash + input fingerprint + output checksum; stale/missing cache must rerun, never silently reuse.
4. **Compact contact summaries:** separate summary from high-volume sample CSV; declare scope/coverage and output limits.
5. **Cross-seed semantics:** aggregate condition groups while retaining individual seeds and exclusion reasons; valid denominators only.
6. **Report composition:** one run report and one experiment report with observations, derived metrics, interpretations, and hypotheses separated.
7. **Manifest completeness follow-up:** inventory full physics/environment configuration and record analyzer/schema versions without overstating current captured-field hash.

## Work explicitly not accepted as merged capability

- PR #392 remains draft/unmerged. Its local-neighborhood, exposure-preflight, and paired-catalysis controls are not general production capabilities. Its matched-catalysis gates remain fail-closed.
- PR #345 is closed/superseded; retain its opportunity-funnel and dispersion ideas as historical context, but do not assume its proposed instrumentation is on main.
- The PLAN-004 aggregation repair is underway on its experiment branch. Its retained ten observations are successful; report assembly failed because the analyzer filename convention did not match downloaded condition-prefixed filenames. No experiment rerun is justified for this defect.

## Phase 1 exit decision

The current-main analyzer inventory is sufficient to design the initial index and orchestration contract. The pipeline must treat contact-opportunity detail as an opt-in heavy stage and must normalize artifacts in a separate workspace. The next engineering step is the canonical analysis index and stage/cache contracts.
