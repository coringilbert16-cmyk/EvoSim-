# EXP-2026-005 — PLAN-004 paired resource-feed experiment

**Status:** All ten observations complete. Five-pair chemistry/organization comparison recovered from retained artifacts; full specialist-analyzer aggregation pending. See the [five-pair recovery report](2026-11-20-plan-004-five-pair-recovery.md).

**GitHub Actions:** [Run #44784506988](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/44784506988).

## Research question

Does recurrent input of free environmental elements sustain connected multistage chemistry and intermediate-family turnover beyond the finite starting inventory?

## Design

- **Arms:** matched no-feed control and external free-resource feed.
- **Independent seeds:** 42, 43, 44, 45, 46; each seed appears in both arms.
- **Duration:** 3,600 seconds of wall-clock simulation time per run.
- **Total runs:** 10 (five paired seeds).
- **Initial state per run:** existing standard environmental chemistry fixture — 1,200 free base resources and 80 intact two-constituent molecules (1,360 constituents total), six active elements, no ambient medium, no organism.
- **Diagnostics:** existing environmental chemistry JSONL diagnostics at 100-tick interval; no existing diagnostic fields or measurements removed or reconfigured. New source-event records identify each external feed pulse and the exact identities introduced.
- **Chemistry:** ordinary COMBINE, BREAK, and DISPLACEMENT only. No forced reactions, compounds, or injected energy.

## Predeclared resource-feed schedule

The feed arm receives six pulses at 5, 15, 25, 35, 45, and 55 minutes of elapsed wall-clock time. Each pulse adds 72 free atoms:

| Element | Per pulse | Six-pulse total |
|---|---:|---:|
| Carbon | 12 | 72 |
| Hydrogen | 12 | 72 |
| Nitrogen | 12 | 72 |
| Oxygen | 12 | 72 |
| Phosphorus | 12 | 72 |
| Sulfur | 12 | 72 |
| **Total** | **72** | **432** |

The total feed is 432 atoms, exactly 36% of the 1,200-atom initial free-resource inventory. This is a controlled, equimolar six-element feed, not an attempt to reproduce a measured net-depletion ratio. Each pulse samples the fixed tick-zero spatial distribution of the 1,200 initial free resources, with small deterministic position jitter from a separate RNG stream derived from the run seed. This preserves the initial resource-pile distribution instead of introducing a uniform-spatial feed treatment, and does not consume the simulation's random-number stream. The actual elapsed time and simulation tick of each pulse are recorded; delivery occurs at the first simulation-loop boundary after the scheduled wall-clock time.

The no-feed arm receives no external matter after tick zero. Both arms retain the same initial seed, inventory construction, hotspot, environment, and normal physics.

## Accounting and interpretation

- Each feed atom is a free base element with no bond and no injected usable or resonant energy.
- Feed events record pulse number, scheduled interval, actual elapsed time, element counts, and the stable molecule identities created.
- The runner compares the complete energy inventory snapshot immediately before and after each pulse and requires exact equality. Thus, matter injection cannot add usable/resonant energy or alter stored energy, pair-potential accounting, work, heat, or other ledger flows. This check does not reset or require a zero residual accumulated by prior simulation steps; whole-run energy validation remains part of the existing diagnostics.
- The existing snapshots and chemistry/organization analyzers are retained for both arms.
- Compare paired outcomes by seed. Seeds, not reaction events, are the independent replicates.
- Primary interpretation: whether connected pathway activity and intermediate turnover persist or recover after resource input, compared with matched finite-inventory controls.
- Secondary interpretation: whether elemental pools and compound-family diversity differ between arms, and whether the network's whole-run energy ledger remains balanced.
- A higher reaction count alone is not evidence of pseudo-metabolism. A positive result requires temporally connected transformations, recurrent use/regeneration of intermediates, resource-derived material incorporation, and persistence across multiple feed intervals.
- This experiment does not by itself establish catalysis, autocatalysis, self-sustaining chemistry, or a biological organism.

## Implementation and run references

- Dedicated workflow: `.github/workflows/plan-004-resource-feed.yml`.
- Feed flag: `--plan004-resource-feed`.
- Feed pulse event type: `environmental_resource_feed`; each introduced material is also recorded as an `INITIAL`-classified `molecular_lifecycle` source identity with `external_source=true`, so analyzers do not misclassify feed atoms as reaction-created products.
- Run configuration: `experiments/plan-004-run.json` (will be activated only after code validation).
- Original combined artifact: not produced because the aggregation job failed on input filename/layout mismatch.
- Recovery workflow: `.github/workflows/plan-004-aggregate-existing.yml`; uses retained run #44784506988 artifacts without repeating simulations.
- Five-pair recovered comparison: [recovery report](2026-11-20-plan-004-five-pair-recovery.md).

## Validation gates before launch

1. Rust tests for exact pulse size/composition, deterministic placement/identity, and unchanged energy inventory.
2. Full repository CI / Clippy for the implementation branch.
3. Validate the paired workflow and artifact aggregation paths.
4. Only after those gates pass, activate the registered run configuration and launch the ten one-hour simulations.


## Preliminary paired observations (two of five seeds)

The first two completed matched pairs confirm that the feed protocol executed as specified, but they do not yet show a consistent chemistry response.

### Feed delivery and accounting

- Both completed feed arms contain exactly six `environmental_resource_feed` records.
- Each pulse introduced 72 free atoms: 12 each of C, H, N, O, P, and S; total external matter per feed run is 432 atoms.
- Pulse records occur at approximately 300, 900, 1,500, 2,100, 2,700, and 3,300 elapsed seconds.
- In seed 42, final constituent count is 1,792 versus 1,360 in the no-feed arm, exactly +432. Each element's final free-resource inventory is +72 versus its matched no-feed run. This confirms the supplied matter is present in the final accounting.
- Whole-run energy residuals remain small: seed 42 feed 0.0002942 versus no-feed 0.0003684; seed 43 feed 0.0006884 versus no-feed 0.0007143. The per-pulse energy-inventory invariance assertion is part of the implementation validation.

### Committed reaction outcomes

| Seed | Arm | BREAK | COMBINE | DISPLACEMENT | Total committed |
|---:|---|---:|---:|---:|---:|
| 42 | No-feed | 297 | 296 | 87 | 680 |
| 42 | Feed | 167 | 165 | 44 | 376 |
| 43 | No-feed | 320 | 322 | 74 | 716 |
| 43 | Feed | 385 | 386 | 104 | 875 |

Raw committed reaction totals move in opposite directions: feed is lower in seed 42 and higher in seed 43. The difference remains after normalizing by observed simulation ticks: seed 42 total committed reactions per 1,000 ticks are 34.86 no-feed versus 23.81 feed; seed 43 are 36.58 no-feed versus 45.60 feed. COMBINE plus DISPLACEMENT per 1,000 ticks shows the same opposite direction (19.63 vs 13.23 for seed 42; 20.23 vs 25.53 for seed 43). This is not a reproducible feed effect on the evidence currently available.

### Organization indicators

| Seed | Arm | Recurrent reaction-created families | Multi-resource or bonded identities | Families active at end |
|---:|---|---:|---:|---:|
| 42 | No-feed | 39 | 553 | 22 |
| 42 | Feed | 20 | 314 | 18 |
| 43 | No-feed | 35 | 541 | 21 |
| 43 | Feed | 34 | 667 | 23 |

The organization indicators also disagree between seeds. The extra active material in feed arms (about 1,712–1,714 objects versus about 1,278–1,281 in controls) largely reflects the 432 supplied free atoms; it must not be mistaken for an increase in organized compounds. Recurrent family counts are a more relevant, though still descriptive, indicator.

### Interim interpretation

1. Feed delivery and material conservation are working in the observed arms.
2. The current two pairs do not demonstrate sustained pseudo-metabolism. Seed 42's feed arm has fewer committed reactions and fewer recurrent families; seed 43's feed arm has more committed reactions but almost unchanged recurrent-family count.
3. A reaction-count increase alone would not satisfy PLAN-004. We still need exact-instance connected pathway traces across multiple feed intervals, intermediate regeneration/turnover, and evidence that new resource input participates in those pathways.
4. The feed arms run with a larger physical inventory, which can affect wall-clock throughput. Observed tick counts differ between arms (seed 42: 15,794 feed vs 19,509 control; seed 43: 19,190 vs 19,574). Per-tick normalization helps but does not replace the paired five-seed analysis.
5. Independent experimental replication is by seed, not by reaction event. With only two matched pairs, no statistical or causal conclusion is warranted.

**Interim conclusion:** the intervention is delivering matter without an evident energy-ledger disturbance, but the chemistry response is seed-dependent and currently inconclusive. Complete seeds 44–46 and reconstruct pathway-level turnover before deciding whether the feed supports pseudo-metabolic organization.


## Final result

Pending completion and analysis of all five matched seed pairs.


## Five-pair observation closeout

All ten observation jobs (five seeds × two arms) completed successfully. The aggregation job failed because the raw JSONL inputs were copied into the expected directories but the already-generated per-seed chemistry and organization JSON files were not. A dedicated aggregation-only recovery workflow was added to the experiment branch; it downloads the ten original artifacts, validates all 30 expected files, normalizes the report filenames, and runs the existing specialist analyzers. It does not rerun simulations.

The retained per-seed chemistry and organization reports were independently recovered and compared across all five pairs. The direction of the committed-reaction rate was lower in feed for three seeds and higher for two; the mean paired difference was -4.02 committed reactions per 1,000 observed ticks. Recurrent-family counts were lower in feed for four seeds and higher for one. These data do not support a consistent positive resource-feed effect. See the [five-pair recovery report](2026-11-20-plan-004-five-pair-recovery.md) for the full table, feed-pulse verification, limits, and recovery details.

**Updated status:** observation complete; partial five-pair analysis recovered; complete network/contact/temporal aggregation still pending. Do not repeat the ten one-hour simulations to fix the aggregation defect.
