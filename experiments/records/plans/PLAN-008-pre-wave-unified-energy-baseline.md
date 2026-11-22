# PLAN-008 — Pre-wave unified-energy performance baseline

**Status:** Completed — EXP-2026-008 ran successfully for all five seeds; setup and consolidated report published and validated. Performance profiling summaries extracted from successful Actions logs and preserved in the consolidated report.

**Linked experiment:** EXP-2026-008, condition `pre-wave-reference`.

## Question

What is the reproducible performance and observable energy-accounting baseline of the current `main` implementation immediately before the unified energy / continuous spatial wave-field redesign?

## Purpose and scope

This is the Phase 0 reference experiment for the resonance energy continuous-wave update. It establishes the comparison point for later physics implementations. It is a measurement-only experiment: no source code, physics, chemistry, reaction rules, thresholds, or environment settings are changed.

This is not a test of whether the current resonance model is physically complete. It records the behavior and cost of the current implementation so later versions can be compared honestly.

## Fixed design

- Five independent seeds: 42, 43, 44, 45, and 46.
- Real simulation duration: 60 seconds per seed.
- Diagnostic interval: 100 ticks.
- Standard inventory: 1,200 independent single-constituent resources plus 80 intact two-constituent molecules (1,360 constituents total), using the existing standard six-element catalog and distribution.
- Run from the exact `main` commit captured in the setup record.
- Ordinary environmental chemistry and existing energy/source behavior unchanged.
- Movement profiling: enabled.
- Sampled tick profiling: enabled (one tick in ten).
- Deep diagnostics: disabled.
- No forced reactions, artificial energy grants, altered thresholds, special-case behavior, or extended observation window.

Profiling is deliberately enabled so the baseline includes separate movement/resonance-placement timings and sampled whole-tick phase timings. Future comparisons must use the same profiling settings. Instrumented wall-clock time must not be compared directly with unprofiled runs.

## Measurements

Capture and retain, where the current workflow and analyzers support them:

1. Exact source commit, runtime/toolchain, seed, requested and actual duration, command-line arguments, canonical configuration, and inventory provenance.
2. Simulated ticks and throughput per seed.
3. Wall-clock runtime per seed and cross-seed variation.
4. Movement phase cost, including resonance calculation and placement validation.
5. Sampled whole-tick phase timings.
6. Existing source/settlement and energy-ledger totals, residuals, and validation status.
7. Material inventory balance and reaction outcomes.
8. Missing, partial, sampled, unknown, and measured-zero distinctions.

Do not invent unavailable peak-memory or phase measurements. Record them as unavailable if the workflow does not produce them. Treat seeds as independent replicates; ticks and events within a seed are not independent replicates.

## Analysis and comparison rules

- Preserve per-seed measurements in the consolidated report.
- Report descriptive cross-seed summaries and variation; do not imply five seeds establish universal performance.
- Separate profiling overhead from simulator phase costs. All future performance comparisons must match instrumentation settings and workload.
- Compare later wave-field implementations against this exact setup and workload. Report both absolute runtime and phase-level changes.
- Do not trade away physical correctness, conservation, determinism, or required field behavior to improve the benchmark.
- This experiment establishes an empirical pre-wave reference, not a physical validation of the legacy energy model.

## Success criteria

- All five seeds complete with validated provenance and compatible measurement definitions.
- The canonical setup and exact source revision are captured.
- Tick/throughput, runtime, available phase timings, and energy/accounting results are consolidated per seed.
- The two-file permanent record validates successfully.
- Raw observations and intermediate analysis files are deleted only after both permanent files are successfully published.

## Workflow request

Dispatch `.github/workflows/chemistry-experiments.yml` from `main` with:

- `seeds=42,43,44,45,46`
- `duration_seconds=60`
- `diagnostic_interval=100`
- `profile_movement=true`
- `profile_tick=true`
- `deep_diagnostics=false`
- `experiment_id=EXP-2026-008`
- `research_plan=PLAN-008`
- `condition_id=pre-wave-reference`

## Decision after the run

Phase 0 baseline execution is complete. The two permanent files are published; the consolidated scientific report passed OBS-001 validation, and the separately derived profiling summary is explicitly labeled as log-extracted rather than OBS-001 schema-validated. Use this record as the fixed pre-wave comparison point. Review its limitations before designing Phase 1; do not interpret this empirical baseline as validation of the legacy energy physics.
