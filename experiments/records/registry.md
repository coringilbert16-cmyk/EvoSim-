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
| PLAN-001 | Which instance-level pathways demonstrate reusable intermediates, reversible transformations, or possible facilitation? | **In progress — artifact analysis; no new run** | [Pathway analysis](plans/README.md#plan-001-existing-pathway-and-molecular-lineage-analysis) |
| PLAN-002 | Does a molecule facilitate a reaction while remaining chemically available? | Planned — depends on PLAN-001 and matched controls | [Catalytic facilitation](plans/README.md#plan-002-catalytic-facilitation) |
| PLAN-003 | Does a product promote net production of additional copies of its own structure? | Planned — requires a defined self-production pathway and seeded/unseeded controls | [Autocatalysis](plans/README.md#plan-003-autocatalysis) |
| PLAN-004 | Can a connected network repeatedly transform resources and regenerate intermediates? | Planned — requires validated pathways and turnover | [Pseudo-metabolism](plans/README.md#plan-004-pseudo-metabolism) |
| PLAN-005 | Can distinct pathways facilitate one another and form a cooperative chemical network? | Planned — depends on controlled facilitation methodology | [Cross-catalysis](plans/PLAN-005-cross-catalysis.md) |
| PLAN-006 | Do environmental fields and transport create persistent spatial reaction organization? | Planned — requires controlled spatial comparisons | [Spatially organized chemistry](plans/PLAN-006-spatial-organization.md) |

## Engineering and observability initiatives

| ID | Objective | Status | Implementation record |
|---|---|---|---|
| OBS-001 | Build compact, reproducible chemistry diagnostics and unified offline analysis without changing simulation behavior | **Active — foundations merged; integration and historical pilot in progress** | [Unified analysis implementation](plans/OBS-001-unified-analysis-implementation.md) |

OBS-001 is an engineering initiative, not a scientific experiment. It has no EXP identifier and does not itself establish a chemistry hypothesis. Its merged foundations are PR #398 (reaction identity/settlement telemetry), PR #400 (opportunity coverage semantics), and PR #401 (canonical run manifests). The opportunity-coverage and manifest implementation records remain linked in [plans/](plans/).

## Current PLAN-001 progress

The first artifact pass identified four exact-instance reversible fragment-reuse cycles across seeds 43, 44, and 45: three N–O–P ⇌ N + O–P and one H–O–P ⇌ H + O–P. These support reusable intermediate chemistry, not catalysis. See [initial findings](results/2026-11-20-plan-001-initial-findings.md).

## Interpretation guardrails

- EXP-2026-004 is the strongest validated multi-seed dataset currently recorded.
- Family-transition cycles can arise from reversible BREAK/COMBINE transformations and identity turnover; they are not proof of catalysis or autocatalysis.
- Product reuse as a parent is evidence of subsequent chemical participation, not proof that the product facilitated another reaction.
- No completed experiment currently confirms catalysis, autocatalysis, pseudo-metabolism, cross-catalysis, or self-sustaining chemistry.
- Do not repeat generic baselines or extend observation windows solely to wait for an event.
- Future experiments must be registered before execution and receive a dedicated result record at closeout.

## Historical record policy

Historical summaries retain known configuration uncertainty. The 300-second experiment's artifact metadata reported 1,280 molecules / 2,560 constituents and zero independent resources, conflicting with an earlier request description of 1,200 resources plus 80 molecules. Treat its results as a separate historical observation, not as directly comparable to EXP-2026-004.
