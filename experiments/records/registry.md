# EvoSim chemistry experiment registry

Last updated: 2026-11-21

This is the master index. Detailed completed-run results live under [results/](results/); planned research lives under [plans/](plans/README.md).

## Completed experiments

| ID | Experiment | Conditions | Status | Primary record |
|---|---|---|---|---|
| EXP-2026-001 | Seed-42 short environmental baseline | 60 seconds wall-clock; seed 42; 1,200 independent resources + 80 two-constituent molecules; 40 H–S and 40 C–H starting molecules | Completed; historical configuration | [Historical result summary](results/historical-experiments.md) |
| EXP-2026-002 | Five-seed short chemistry observation | 300 seconds per seed; seeds 42–46; historical inventory metadata is inconsistent | Completed; interpret cautiously | [Historical result summary](results/historical-experiments.md) |
| EXP-2026-003 | Five-seed extended chemistry observation | 25 minutes per seed; seeds 42–46 | Completed; reanalyzed | [Historical result summary](results/historical-experiments.md) |
| EXP-2026-004 | Five-seed one-hour chemistry observation | 3,600 seconds per seed; seeds 42–46; 1,200 independent resources + 80 two-constituent molecules; six active elements | Completed; scientific result validated; legacy v1 compact compatibility partial/uncertified | [Full result, analysis, and compatibility limits](results/2026-11-19-one-hour-five-seed.md) |
| EXP-2026-005 | PLAN-004 paired resource-feed comparison | Five paired seeds (42–46); 3,600 seconds per arm; no-feed control versus six pulses of 72 free atoms (12 of each active element) at 5, 15, 25, 35, 45, and 55 minutes | Completed; specialist aggregation recovered; scientific result inconclusive | [Pre-registration and protocol](results/2026-11-20-plan-004-resource-feed-paired-experiment.md) · [Five-pair recovery](results/2026-11-20-plan-004-five-pair-recovery.md) |

## Registered experiments

| ID | Experiment | Conditions | Status | Research plan |
|---|---|---|---|---|
| EXP-2026-007 | Environmental BREAK and DISPLACEMENT discovery-eligibility audit | Five independent seeds (42–46); 60 seconds per seed; diagnostic interval 100 ticks; standard six-element inventory; ordinary chemistry; deep diagnostics and profiling disabled | **Registered; not yet run** | [PLAN-007 — Environmental reaction-discovery eligibility](plans/PLAN-007-environmental-discovery-eligibility.md) |
| EXP-2026-008 | Pre-wave unified-energy performance and accounting baseline | Five independent seeds (42–46); 60 seconds per seed; diagnostic interval 100 ticks; standard six-element inventory; movement and sampled tick profiling enabled; deep diagnostics disabled | Results recorded: pre-wave-reference | [PLAN-008 — Pre-wave unified-energy performance baseline](plans/PLAN-008-pre-wave-unified-energy-baseline.md); [pre-wave-reference setup](results/EXP-2026-008-pre-wave-reference/setup.json) · [pre-wave-reference report](results/EXP-2026-008-pre-wave-reference/consolidated-report.json) |

## Planned research

| ID | Research question | Status | Plan |
|---|---|---|---|
| PLAN-001 | Which instance-level pathways demonstrate reusable intermediates, reversible transformations, or possible facilitation? | **Completed — EXP-2026-004 artifact analysis; no new run** | [Pathway analysis](plans/README.md#plan-001-existing-pathway-and-molecular-lineage-analysis) |
| PLAN-002 | Does a molecule facilitate a reaction while remaining chemically available? | Planned — depends on PLAN-001 and matched controls | [Catalytic facilitation](plans/README.md#plan-002-catalytic-facilitation) |
| PLAN-003 | Does a product promote net production of additional copies of its own structure? | Planned — requires a defined self-production pathway and seeded/unseeded controls | [Autocatalysis](plans/README.md#plan-003-autocatalysis) |
| PLAN-004 | Can a connected network repeatedly transform resources and regenerate intermediates? | **In progress — EXP-2026-005 paired resource-feed comparison complete; result inconclusive; pathway-level analysis pending** | [Pseudo-metabolism](plans/README.md#plan-004-pseudo-metabolism) |
| PLAN-005 | Can distinct pathways facilitate one another and form a cooperative chemical network? | Planned — depends on controlled facilitation methodology | [Cross-catalysis](plans/PLAN-005-cross-catalysis.md) |
| PLAN-006 | Do environmental fields and transport create persistent spatial reaction organization? | **Descriptive baseline screen complete; causal comparison blocked by incomplete opportunity coverage and unmatched conditions** | [Spatially organized chemistry](plans/PLAN-006-spatial-organization.md) |
| PLAN-007 | At which stage do ordinary environmental BREAK and DISPLACEMENT candidates fail to become eligible, funded, and committed reactions? | **Registered; EXP-2026-007 prepared, not yet run** | [Environmental reaction-discovery eligibility](plans/PLAN-007-environmental-discovery-eligibility.md) |
| PLAN-008 | What is the reproducible pre-wave performance and energy-accounting baseline? | **Registered; EXP-2026-008 prepared, not yet run** | [Pre-wave unified-energy performance baseline](plans/PLAN-008-pre-wave-unified-energy-baseline.md) |

## Engineering and observability initiatives

| ID | Objective | Status | Implementation record |
|---|---|---|---|
| OBS-001 | Build compact, reproducible chemistry diagnostics and unified offline analysis without changing simulation behavior | **Implementation complete; first canonical experiment publication pending** | [Unified analysis implementation](plans/OBS-001-unified-analysis-implementation.md); [permanent record contract](plans/OBS-001-permanent-experiment-record.md) |

OBS-001 is an engineering initiative, not a scientific experiment. It has no EXP identifier and does not itself establish a chemistry hypothesis. Its implementation is complete through PRs #398–#405: reaction identity/settlement telemetry, opportunity-coverage semantics, canonical manifests, unified analysis, compact reports, two-file retention, and the unified-analysis path fix. The two historical pilots completed offline in workflow runs [#8](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/44964063392) and [#9](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/44965815500); they did not rerun simulations and remain legacy datasets without canonical manifests. The remaining acceptance test is the first new experiment to publish exactly the two canonical permanent files through the workflow.

## Current PLAN-001 closeout

Four exact-instance reversible fragment-reuse cycles were confirmed across seeds 43–45: three N–O–P ⇌ N + O–P and one H–O–P ⇌ H + O–P. Three additional exact-instance sequences show H–O–P temporarily incorporated into H–O–P–S and released again in seed 45. A broader scan found 192 exact-pair fragment reunifications across seeds 42–46, dominated by simple two-constituent families. Longest connected identity-lineage walks ranged from 12 to 39 events across seeds. No facilitator can be identified from the available event/opportunity records. These support reusable intermediate chemistry, not catalysis. See [completed PLAN-001 findings](results/2026-11-20-plan-001-initial-findings.md).

## Interpretation guardrails

- EXP-2026-004 is the strongest validated multi-seed dataset currently recorded.
- Family-transition cycles can arise from reversible BREAK/COMBINE transformations and identity turnover; they are not proof of catalysis or autocatalysis.
- Product reuse as a parent is evidence of subsequent chemical participation, not proof that the product facilitated another reaction.
- No completed experiment currently confirms catalysis, autocatalysis, pseudo-metabolism, cross-catalysis, or self-sustaining chemistry.
- Do not repeat generic baselines or extend observation windows solely to wait for an event.
- Future experiments must be registered before execution and receive a dedicated result record at closeout.

## Historical record policy

Historical summaries retain known configuration uncertainty. The 300-second experiment's artifact metadata reported 1,280 molecules / 2,560 constituents and zero independent resources, conflicting with an earlier request description of 1,200 resources plus 80 molecules. Treat its results as a separate historical observation, not as directly comparable to EXP-2026-004. EXP-2026-004 remains scientifically validated, but its preserved result report does not contain all provenance and measurement-definition fields required for a complete v1 compact record. Those fields are explicitly unknown; do not infer them or include this legacy run in strict comparisons until verified from its original per-seed sources. See its result record for the field-by-field limitation.
