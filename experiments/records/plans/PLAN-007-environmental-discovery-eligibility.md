## PLAN-007 — Environmental reaction-discovery eligibility

**Status:** Registered; EXP-2026-007 prepared, not yet run.

**Question:** At which stage do ordinary environmental BREAK and DISPLACEMENT discovery candidates stop progressing toward chemically eligible, fundable, committed reactions, and what evidence explains the absence of eligible candidates or commits?

**Purpose:** Localize the first observed bottleneck in the existing discovery funnel. This is a descriptive diagnostic experiment, not an intervention or a test of a modified chemistry rule.

**Registered experiment:** [EXP-2026-007](../registry.md#registered-experiments), condition `ordinary-discovery-baseline`.

**Design and fixed conditions:**
- Five independent seeds: 42, 43, 44, 45, and 46.
- Real simulation duration: 60 seconds per seed.
- Diagnostic interval: 100 ticks.
- Standard inventory: 1,200 independent single-constituent resources plus 80 intact two-constituent molecules (1,360 constituents total), using the existing standard six-element catalog and distribution.
- Ordinary chemistry, activation, scheduling, geometry, movement, energy, and reaction rules unchanged.
- Movement profiling: disabled. Tick profiling: disabled. Deep diagnostics: disabled.
- No forced reactions, artificial energy grants, altered thresholds, new catalyst, special-case behavior, or extended observation window.

**Measurements and interpretation:**

1. **BREAK:** use the authoritative maintained bond-index candidate population and inspected count. Follow candidates through physical restoration, geometry/endpoints, mapping/properties, chemistry eligibility, activation, scheduling, rejection, and commit. A BREAK pass is complete only when the authoritative search counts reconcile, including an explicitly measured empty domain.
2. **DISPLACEMENT:** follow anchor and target-cell/material inspection, bounded-window skips, work allowance, replacement-contact searches, activation state, trial rejection stage, energy/funding, and committed replacement transactions. Preserve `complete`, `partial`, `unknown`, and measured-zero distinctions. The bounded rotating search must not be represented as an exhaustive denominator.
3. **COMBINE:** retain its own coverage-aware measurements as environmental context only. COMBINE broad-phase pairs are not a denominator for BREAK or DISPLACEMENT.
4. **Energy and transaction joins:** where a candidate reaches a trial or commit, use stable reaction identity and linked settlement data to distinguish eligibility, funding, transaction rejection, and committed material changes. Run-level energy totals are not candidate-level causal evidence.

**Analysis rules:**
- Analyze each seed independently; the seed is the replicate. Events and repeated per-tick scans within a seed are not independent samples.
- Per-tick search counts are repeated search observations, not unique run-wide encounters.
- Do not calculate conversion rates unless numerator and denominator are both present, complete, and semantically matched.
- Missing counters remain unknown, not zero. Known bounded coverage remains partial; do not infer omitted target totals from a work counter alone.
- Report the first funnel stage where observed candidates stop progressing, separately for BREAK and DISPLACEMENT. If coverage prevents localization, state that limitation rather than inferring absence.
- Separate direct observations, derived summaries, interpretation, and follow-up hypotheses.

**Success criteria:**
- All five seeds have validated provenance and compatible measurement definitions.
- BREAK coverage and candidate-stage counts reconcile.
- DISPLACEMENT coverage is honestly classified and all available stage/rejection counters are retained.
- Measured zero, missing, and partial coverage remain distinct.
- The report identifies a supported bottleneck or explicitly identifies the missing evidence needed to do so.
- No chemistry/physics behavior is changed as part of this experiment.

**Workflow request:** dispatch `.github/workflows/chemistry-experiments.yml` from `main` with `seeds=42,43,44,45,46`, `duration_seconds=60`, `diagnostic_interval=100`, `profile_movement=false`, `profile_tick=false`, `deep_diagnostics=false`, `experiment_id=EXP-2026-007`, `research_plan=PLAN-007`, and `condition_id=ordinary-discovery-baseline`.

**Permanent record and cleanup:** after successful analysis, validate and publish exactly `setup.json` and `consolidated-report.json` under the experiment/condition result directory and update the shared registry. Delete raw observations, manifests, intermediate analyzer outputs, caches, temporary indexes, and diagnostics only after both permanent files are successfully published. Failed validation/publication must retain temporary inputs for recovery.

**Decision after the run:** do not change simulator behavior immediately. Use the first supported bottleneck to decide whether the next step is a narrowly scoped measurement correction, a source-level audit of an existing eligibility rule, or a separately designed physics/chemistry experiment requiring explicit approval.

