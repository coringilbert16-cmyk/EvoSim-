# EvoSim experiment records

This folder is the durable research record for completed, active, and planned EvoSim experiments. It is separate from the executable request configuration and workflow instructions in the parent `experiments/` directory.

## Navigation

- [Experiment registry](registry.md) — master index of every recorded experiment and research plan.
- [Results](results/) — immutable, dated records of completed runs and their analysis.
- [Research plans](plans/README.md) — scientific hypotheses, controls, measurements, and evidence standards.
- [Engineering/observability plan](plans/OBS-001-chemistry-observability.md) — approved compact diagnostic and analysis infrastructure plan.
- [Observability audit](observability-audit-2026-11-20.md) — current first-pass audit and persistent work checkpoint.

## Record-keeping rules

1. Assign every experiment a stable ID (`EXP-YYYY-NNN`); assign scientific research plans `PLAN-NNN`. Engineering infrastructure plans use a distinct `OBS-NNN` namespace so they cannot be confused with scientific experiments.
2. Record status explicitly: planned, approved, running, completed, inconclusive, or superseded.
3. Record the exact commit, workflow run, seeds, real wall-clock duration per seed, starting inventory, environmental conditions, and artifact links whenever available.
4. Keep raw artifacts in GitHub Actions when practical; link to the run and artifact rather than committing very large generated files.
5. Preserve historical results and their original conditions. Never silently correct old metadata; append a correction with provenance.
6. Distinguish observations from interpretations and hypotheses. A recurring family, lineage edge, or graph cycle is not automatically catalysis, autocatalysis, metabolism, or self-sustainability.
7. Do not rerun a completed experiment merely to recreate its report. Reuse and reanalyze its existing artifacts first.
8. Every planned experiment must state what evidence would support or reject its hypothesis, what controls are needed, and whether it changes simulator behavior. Chemistry changes require separate approval.
9. Keep independent seeds as the experimental replicates; reaction events within a seed are not independent samples.
10. Every new diagnostic measurement must have a named scientific question, documented coverage/limitations, and an analysis consumer. Prefer offline reconstruction; do not add redundant telemetry or full-state-per-tick logging by default.

The current environmental workflow and request configuration remain documented in [the experiment execution guide](../README.md).
