# EXP-2026-005 — Five-pair retained-artifact recovery

**Status:** Completed. All ten observations and all specialist aggregators completed from retained artifacts; no simulation reruns.

**Source run:** [GitHub Actions #44784506988](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/44784506988)  
**Recovery workflow:** paired workflow analysis-only mode, source run #44784506988; [successful recovery run #44833658626](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/44833658626). The observe matrix was skipped; no simulations were rerun.  
**Seeds:** 42–46, paired no-feed and resource-feed  
**Duration:** 3,600 wall-clock seconds per run

## Recovery and provenance

All ten observation jobs in the source workflow completed successfully, and all ten per-seed artifacts were present. The original aggregate job failed at its first batch-analysis invocation because it copied only the raw JSONL files into the condition folders. The batch analyzer also requires the per-seed chemistry and organization JSON files under normalized names (`seed-N-chemistry.json` and `seed-N-organization.json`). The original artifacts contained those reports, but the aggregation job did not copy/rename them.

This is an aggregation input-assembly defect, not a simulation failure. The retained artifacts were downloaded and inspected directly. No simulations were repeated.

The recovery workflow now:
1. Validates the source run belongs to the PLAN-004 branch.
2. Downloads the ten existing `plan004-*` artifacts from that run.
3. Verifies all 30 expected raw/chemistry/organization inputs exist.
4. Copies the raw JSONL and the two per-seed reports into the directory and filenames expected by the cross-seed analyzers.
5. Runs the existing batch, reaction-network, geometry-aware network, contact-opportunity, and temporal-complexity analyzers for both arms.
6. Uploads recovered outputs as a 90-day artifact.

The paired workflow now supports analysis-only dispatch with a numeric source run ID. Recovery run #44833658626 successfully assembled the retained artifacts and uploaded the full specialist report artifact `plan004-resource-feed-combined` (artifact ID 14676499429; 90-day retention).

## Feed-delivery verification

For each of the five resource-feed runs, raw JSONL inspection confirms:
- Exactly six `environmental_resource_feed` events.
- Exactly 72 introduced atoms per pulse.
- Exactly 12 each of Carbon, Hydrogen, Nitrogen, Oxygen, Phosphorus, and Sulfur per pulse.
- 432 distinct introduced material identities per run.
- Pulse elapsed times at 300, 900, 1,500, 2,100, 2,700, and 3,300 seconds.

Thus, the scheduled matter input was delivered in all five feed arms. No-feed arms are the matched controls.

## Paired results

| Seed | Ticks no-feed / feed | Committed reactions no-feed / feed | Committed per 1,000 ticks no-feed / feed | Recurrent families no-feed / feed | End-active families no-feed / feed | Compound constituent tally no-feed / feed |
|---:|---:|---:|---:|---:|---:|---:|
| 42 | 19,509 / 15,794 | 680 / 376 | 34.86 / 23.81 | 39 / 20 | 22 / 18 | 1,295 / 660 |
| 43 | 19,574 / 19,190 | 716 / 875 | 36.58 / 45.60 | 35 / 34 | 21 / 23 | 1,188 / 1,515 |
| 44 | 11,931 / 10,697 | 861 / 568 | 72.16 / 53.10 | 30 / 22 | 20 / 18 | 1,387 / 924 |
| 45 | 17,253 / 22,105 | 758 / 915 | 43.93 / 41.39 | 43 / 41 | 23 / 21 | 1,275 / 1,497 |
| 46 | 24,715 / 32,266 | 495 / 761 | 20.03 / 23.59 | 22 / 30 | 18 / 22 | 808 / 1,239 |

The compound-constituent tally is summed across created molecule identities; lineage generations can count transferred material more than once. It is not a conserved inventory or an independent-molecule count.

### Cross-pair summary

- Total committed reaction rate averaged **41.51 per 1,000 observed ticks** in no-feed and **37.50** in resource-feed. The mean paired difference (feed minus control) was **−4.02 per 1,000 ticks**; the median paired difference was **−2.54**.
- The feed arm had a lower committed-reaction rate in three of five seeds and a higher rate in two.
- Recurrent structural-family counts were lower in the feed arm in four of five seeds and higher in one.
- End-active family counts were lower in three pairs and higher in two.
- Compound-constituent tallies were lower in three pairs and higher in two.
- Feed runs ended with approximately 432 additional constituents, as expected from the injected matter. That extra free material must not be interpreted as increased organization.

Whole-run energy balance residuals remained small in all ten runs. The largest absolute final residual was approximately **0.0007143** (seed 43 no-feed). The retained chemistry reports contain these values; the per-pulse implementation also asserts unchanged energy inventory during matter delivery.

## Interpretation

The five-pair results do **not** show a consistent increase in committed chemistry, recurrent structural families, or persistent organization from resource feeding. The direction varies by seed, and the average committed-reaction rate per observed tick is lower in the feed arm. The data therefore do not support a positive PLAN-004 claim of resource-fed pseudo-metabolic organization.

This is a descriptive five-pair comparison, not a causal or statistical proof of no effect. The independent replicate count is five seeds. Reaction events within a seed are not independent replicates.

A committed-reaction rate per observed tick is not an opportunity-normalized rate. Existing diagnostics do not provide complete eligible-pair and exposure denominators, so no claim about intrinsic reaction probability is justified. The 1-hour runs also show different tick throughput between conditions and seeds.

## Aggregation closeout

The successful recovery artifact includes the batch chemistry report, reaction network, geometry-aware reaction network, contact-opportunity analysis, and temporal-complexity outputs for both matched arms. The original raw artifacts and per-seed reports remain unchanged. Recovered output is now available for cross-checking the recorded per-seed summaries.

The workflow correction is analysis-only. It does not alter simulation behavior, chemistry rules, energy calculations, experimental conditions, or source observations.

## Status

**Observation complete; full specialist aggregation complete; scientific result inconclusive for a positive feed effect.**
