# EvoSim chemistry experiment registry

Last updated: 2026-11-20

This is the master index. Detailed completed-run results live under [results/](results/); planned research lives under [plans/](plans/README.md).

## Completed experiments

| ID | Experiment | Conditions | Status | Primary record |
|---|---|---|---|---|
| EXP-2026-001 | Seed-42 short environmental baseline | 60 seconds wall-clock; seed 42; 1,200 independent resources + 80 two-constituent molecules; 40 H–S and 40 C–H starting molecules | Completed; historical configuration | [Historical result summary](results/historical-experiments.md) |
| EXP-2026-002 | Five-seed short chemistry observation | 300 seconds per seed; seeds 42–46; historical inventory metadata is inconsistent | Completed; interpret cautiously | [Historical result summary](results/historical-experiments.md) |
| EXP-2026-003 | Five-seed extended chemistry observation | 25 minutes per seed; seeds 42–46 | Completed; reanalyzed | [Historical result summary](results/historical-experiments.md) |
| EXP-2026-004 | Five-seed one-hour chemistry observation | 3,600 seconds per seed; seeds 42–46; 1,200 independent resources + 80 two-constituent molecules; six active elements | Completed; validated | [Full result and analysis](results/2026-11-19-one-hour-five-seed.md) |

## Planned research

| ID | Research question | Status | Plan |
|---|---|---|---|
| PLAN-001 | Which recurring multistep pathways in existing data are candidates for catalytic facilitation or reusable intermediates? | Planned — artifact analysis first | [Pathway and catalysis candidate analysis](plans/README.md#plan-001-existing-pathway-analysis) |
| PLAN-002 | Can a molecule facilitate repeated reactions while remaining chemically available and not being consumed? | Planned — requires operational definition and controls | [Catalysis investigation](plans/README.md#plan-002-catalysis) |
| PLAN-003 | Can products promote production of additional instances of their own structure? | Planned — no positive evidence yet | [Autocatalysis investigation](plans/README.md#plan-003-autocatalysis) |
| PLAN-004 | Can connected reaction cycles sustain resource transformation and regenerate reusable intermediates? | Planned — build on observed reversible pathways | [Pseudo-metabolism investigation](plans/README.md#plan-004-pseudo-metabolism) |

## Interpretation guardrails

- The November 19 run is the current strongest multi-seed dataset for chemical organization.
- Its family-transition graph contains many cycles, but reversible BREAK/COMBINE paths and identity turnover can create graph cycles without a catalyst.
- Product reuse as a parent is evidence of subsequent chemical participation, not proof that the product catalyzed a reaction.
- No completed experiment currently confirms catalysis, autocatalysis, or a self-sustaining pseudo-metabolic network.
- Future experiments should begin with reanalysis of retained artifacts. Do not repeat generic baseline runs or extend observation windows solely to wait for an event.

## Historical record policy

Historical summaries retain known configuration uncertainty. The 300-second experiment's artifact metadata reported 1,280 molecules / 2,560 constituents and zero independent resources, conflicting with an earlier request description of 1,200 resources plus 80 molecules. Treat its results as a separate historical observation, not as directly comparable to EXP-2026-004.
