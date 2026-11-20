# EXP-2026-005 — PLAN-004 paired resource-feed experiment

**Status:** Running — EXP-2026-005 was dispatched on 2026-11-20. Five of ten paired observation jobs are active; five are queued. No simulation results are available yet.

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
- Combined artifact: pending.
- GitHub Actions run: pending.

## Validation gates before launch

1. Rust tests for exact pulse size/composition, deterministic placement/identity, and unchanged energy inventory.
2. Full repository CI / Clippy for the implementation branch.
3. Validate the paired workflow and artifact aggregation paths.
4. Only after those gates pass, activate the registered run configuration and launch the ten one-hour simulations.

## Result

Pending. This document is the pre-registration; findings and interpretation will be appended after the paired run completes.
