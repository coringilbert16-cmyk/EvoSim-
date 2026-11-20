# EXP-2026-006 — PLAN-002 neighborhood opportunity screen

**Status:** Completed at observation level; descriptive analysis complete; pooled workflow aggregation retry pending.  
**Research plan:** PLAN-002 — catalytic facilitation.  
**Workflow run:** [Run #77](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/44776501166)  
**Source commit:** `191069deb65a9f40638e65fe93134eeadde2219c`  
**Branch:** `chemistry/plan-002-neighborhood-matched-controls`

## Conditions

- Five independent seed runs: 42–46.
- 3,600 seconds wall-clock per seed.
- Standard starting inventory: 1,200 independent free resources plus 80 intact two-constituent molecules; 1,360 constituents across C, H, N, O, P, S.
- No ambient medium; ordinary environmental chemistry; no injected matter, forced reactions, or candidate placement intervention.
- Bounded COMBINE and DISPLACEMENT opportunity samples include local-neighborhood identities, composition/topology descriptors, and geometry context.
- These were not candidate-near versus candidate-remote arms. The matched-catalysis workflow was not dispatched.

## Reaction totals

Across the five seeds, 4,477 reactions committed: 2,037 COMBINE, 1,980 BREAK, and 460 DISPLACEMENT. These are chemistry outcomes, not catalytic outcomes.

## Neighborhood analysis

A compact offline pass grouped COMBINE samples by source-molecule pair and tick, then associated them with committed reactions for the same pair and tick. This is deliberately weaker than an exact interface match.

- 49,254 bounded COMBINE source-pair/tick groups were represented.
- 1,890 groups contained at least one sampled interface marked physically valid.
- 244 of those valid groups also had a same-pair/tick COMBINE commit (12.9% descriptive association; not a reaction probability).
- 1,428 of the 2,037 COMBINE commits (70.1%) had some same-pair/tick neighborhood sample. Only 244 commits (12.0% of all COMBINE commits) coincided with a group containing a physically valid sampled interface.
- Across the 1,428 matched COMBINE event groups, 20,119 third-party neighbor-identity incidences were observed; 19,850 (98.7%) were still active at the target reaction tick. These are event-neighbor incidences, not globally unique molecules.

### Family-presence associations

Among the 1,890 groups with at least one physically valid sampled interface, the following nearby-family associations appeared in the seed-stratified descriptive comparison:

| Nearby family | Valid groups with family | Same-pair/tick commits | Commit fraction | Descriptive seed-stratified odds ratio |
|---|---:|---:|---:|---:|
| H–O | 1,279 | 201 | 15.7% | 2.52 |
| H–S | 432 | 92 | 21.3% | 2.30 |
| S–S | 862 | 153 | 17.7% | 2.34 |
| O–P | 1,233 | 190 | 15.4% | 1.97 |
| H–O–P | 170 | 45 | 26.5% | 2.23 |

For each listed family, the descriptive within-seed association was positive in all five seeds. Odds ratios are exploratory summaries only; no inferential p-values or confidence claims are made. Families co-occur, and the comparison does not control for local density, flow, hotspot exposure, resonance, or other neighborhood composition.

**Interpretation:** H–S is a reasonable provisional family to prioritize in a controlled follow-up, alongside H–O and S–S. This does not validate the illustrative H–S candidate instance or establish catalytic facilitation. The observed neighbors were generally still chemically available at the target event, but availability alone does not show that they increased the reaction rate.

## Data-quality and coverage limits

1. The neighborhood is a bounded diagnostic window, not a complete census of eligible target-pair encounters. The 1,890 valid sampled groups are not an unbiased opportunity denominator.
2. The exact transaction join uses source IDs, constituent indices, and endpoint descriptors. In seed 42 it returned zero exact sample-to-outcome matches, while the weaker source-pair-plus-tick join found 197 committed COMBINE events. For example, source IDs 879/880 at tick 67 had sampled endpoint/unit descriptors that differed from the committed event descriptors. The weaker join supports descriptive association only; it does not prove the sampled interface was the interface that committed.
3. DISPLACEMENT neighborhood coverage was especially weak: only two same-pair/tick committed displacement events were found in the five-seed sample data. No displacement facilitation inference is supported.
4. The five seeds—not individual events or pair/tick groups—are the independent experimental replicates. Repeated groups within a seed are temporally and spatially dependent.
5. Family matching uses composition/topology descriptors, not exact bond energies, full graph identity, or persistent molecule identity.
6. Neighbor survival means the identity had not been destroyed at or before the target reaction tick. It does not show that the neighbor caused the event or remained unchanged for the rest of the run.
7. The run did not compare candidate-present/absent or candidate-near/remote conditions. It cannot establish a causal catalyst effect.

## Required next work

- Repair the sample-to-transaction mapping so sampled interface descriptors can be related to the actual committed interface, or explicitly retain only the weaker pair/tick association.
- Complete time-resolved target and candidate flow/hotspot exposure, non-candidate background resonance, and complete eligible target-pair opportunity denominators.
- Use the observational associations to nominate a testable candidate hypothesis; do not promote H–S solely because it appears in the example configuration.
- Keep the paired workflow fail-closed until all exposure and denominator gates pass. No chemistry rules or energy accounting are to be changed for this plan.

## Artifacts

Five per-seed raw diagnostic artifacts were uploaded by run #77:

- Seed 42: artifact 14653927808
- Seed 43: artifact 14654771175
- Seed 44: artifact 14654077820
- Seed 45: artifact 14655400411
- Seed 46: artifact 14655325679

The initial aggregate attempt was terminated by the GitHub runner with exit code 143 after the per-seed observation jobs succeeded. Aggregate-only retry is currently running; no simulation rerun was requested or started.
