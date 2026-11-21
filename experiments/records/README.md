# EvoSim experiment records

This folder is the durable research record for completed, active, and planned EvoSim experiments. It is separate from the executable request configuration and workflow instructions in the parent experiments directory.

## Navigation

- [Experiment registry](registry.md) — master index of every recorded experiment and research plan.
- [Results](results/) — permanent experiment setup and consolidated scientific reports.
- [Plans](plans/README.md) — proposed experiments, hypotheses, controls, measurements, and status.
- [Engineering/observability plan](plans/OBS-001-unified-analysis-implementation.md) — OBS-001's implementation sequence and acceptance criteria.
- [OBS-001 analyzer inventory](plans/OBS-001-analyzer-inventory.md) — analyzer capabilities, input/output contracts, and integration gaps.
- [OBS-001 permanent experiment record](plans/OBS-001-permanent-experiment-record.md) — governing two-file retention contract.

## Record-keeping rules

1. Assign every experiment a stable ID (EXP-YYYY-NNN); assign plans PLAN-NNN.
2. Record status explicitly: planned, approved, running, completed, inconclusive, or superseded.
3. For each future completed experiment, preserve exactly two experiment-specific files: the setup/conditions record and the validated consolidated report.
4. The setup must contain the exact source commit, configuration, seeds, duration, initial inventory, environmental conditions, enabled features, and all relevant run settings needed to recreate the experiment.
5. The consolidated report must contain compact per-seed results, cross-seed analysis, provenance/validation status, missingness, interpretation limits, and conclusions. Do not create separate permanent per-seed records, analyzer indexes, comparison files, or result notes.
6. Raw JSONL, manifests, specialist analyzer outputs, intermediate files, caches, and diagnostics are temporary. Delete them only after the consolidated report is validated and both permanent files are durably published.
7. If analysis or validation fails, preserve temporary inputs for investigation and do not publish the experiment as complete.
8. Preserve historical results and their original conditions. Do not rewrite old records to fit the new format; append corrections only when needed and with provenance.
9. Distinguish observations from interpretations and hypotheses. A recurring family, lineage edge, or graph cycle is not automatically catalysis, autocatalysis, metabolism, or self-sustainability.
10. Do not rerun a completed experiment merely to recreate its report. Reuse and reanalyze existing artifacts first.
11. Keep independent seeds as the experimental replicates; reaction events within a seed are not independent samples.

The current environmental workflow and request configuration remain documented in [the experiment execution guide](../README.md).
