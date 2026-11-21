# EvoSim experiment records

This folder is the durable research record for completed, active, and planned EvoSim experiments. It is separate from the executable request configuration and workflow instructions in the parent experiments directory.

## Navigation

- [Experiment registry](registry.md) — master index of every recorded experiment and research plan.
- [Results](results/) — immutable, dated records of completed runs and their analysis.
- [Plans](plans/README.md) — proposed experiments, hypotheses, controls, measurements, and status.
- [Engineering/observability plan](plans/OBS-001-unified-analysis-implementation.md) — OBS-001's implementation sequence and acceptance criteria.
- [OBS-001 analyzer inventory](plans/OBS-001-analyzer-inventory.md) — current-main analyzer capabilities, input/output contracts, and integration gaps.
- [OBS-001 permanent experiment record](plans/OBS-001-permanent-experiment-record.md) — versioned compact-record schema and retention policy.
- [Reference dataset allowlist](reference-datasets.json) — intentionally empty until a dataset is selected and approved for long-term preservation.

## Record-keeping rules

1. Assign every experiment a stable ID (EXP-YYYY-NNN); assign plans PLAN-NNN.
2. Record status explicitly: planned, approved, running, completed, inconclusive, or superseded.
3. Record the exact commit, workflow run, seeds, real wall-clock duration per seed, starting inventory, environmental conditions, and artifact links whenever available.
4. Keep compact per-seed permanent records and experiment comparison records in `experiments/records/results/`. Raw JSONL and intermediate analyzer outputs are temporary working data, not permanent archives; retain them only for an explicit scientific reason and with an expiry date.
5. Preserve historical results and their original conditions. Never silently correct old metadata; append a correction with provenance.
6. Distinguish observations from interpretations and hypotheses. A recurring family, lineage edge, or graph cycle is not automatically catalysis, autocatalysis, metabolism, or self-sustainability.
7. Do not rerun a completed experiment merely to recreate its report. Reuse and reanalyze its existing artifacts first.
8. Every planned experiment must state what evidence would support or reject its hypothesis, what controls are needed, and whether it changes simulator behavior. Chemistry changes require separate approval.
9. Keep independent seeds as the experimental replicates; reaction events within a seed are not independent samples.

The current environmental workflow and request configuration remain documented in [the experiment execution guide](../README.md).
