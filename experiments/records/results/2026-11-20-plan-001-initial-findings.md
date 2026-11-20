# PLAN-001 — Initial instance-level pathway findings

**Status:** In progress; first-pass analysis of retained artifacts. No new simulation was run.

## Source dataset

- Experiment: [EXP-2026-004](2026-11-19-one-hour-five-seed.md)
- GitHub Actions run: [Run #74](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/44582565701)
- Run ID: `44582565701`
- Simulation commit: `8e7cc7d624e3131b2d99889fef02d9d5c3095318`
- Seeds: 42–46; 3,600 seconds per seed.
- Combined artifact: `chemistry-experiment-combined`, artifact ID `14564601713`.
- Artifact SHA-256: `e46720136d506b5e4e8a17f833ae10ede482c3facb5b7d2298bb1fd9ba03f898`.
- Analysis files: `geometry-aware-network/instance-reaction-events.csv`, `geometry-aware-network/molecule-instances.csv`, and `reaction-network/reaction-pathways.csv`.

## First-pass result: exact fragment reuse

Four complete instance-level cycles were found in which a parent molecule was consumed by BREAK, both exact child molecule identities remained available, and those same child IDs were later consumed together by COMBINE to produce a new molecule of the original family.

| Seed | BREAK event | Parent molecule | Exact child identities | Later COMBINE event | Product |
|---:|---|---:|---|---|---|
| 43 | `13756-4` at tick 13,756 | 2,294 N–O–P | 2,302 N + 2,303 O–P | `16293-16` at tick 16,293 | 2,469 N–O–P |
| 44 | `11341-15` at tick 11,341 | 2,455 N–O–P | 2,484 N + 2,485 O–P | `11829-16` at tick 11,829 | 2,541 N–O–P |
| 45 | `11699-5` at tick 11,699 | 2,233 N–O–P | 2,248 N + 2,249 O–P | `11772-16` at tick 11,772 | 2,262 N–O–P |
| 45 | `10816-0` at tick 10,816 | 1,982 H–O–P | 2,155 H + 2,154 O–P | `12250-16` at tick 12,250 | 2,296 H–O–P |

The table follows molecule IDs, not just composition/topology fingerprints. In each listed cycle, both BREAK products are the exact parent IDs in the later COMBINE. The final product is a new molecule identity, so this is reversible chemical interconversion and fragment reuse—not persistence of the original molecule.

## Family-level event recurrence

The exact two-edge family transformations are distributed across all five seeds:

| Transformation | Seed 42 | Seed 43 | Seed 44 | Seed 45 | Seed 46 | Total |
|---|---:|---:|---:|---:|---:|---:|
| H + O–P → H–O–P (COMBINE) | 1 | 6 | 4 | 4 | 1 | 16 |
| H–O–P → H + O–P (BREAK) | 1 | 6 | 5 | 3 | 0 | 15 |
| N + O–P → N–O–P (COMBINE) | 0 | 5 | 4 | 3 | 1 | 13 |
| N–O–P → N + O–P (BREAK) | 1 | 6 | 3 | 2 | 1 | 13 |

These are event counts for the stated exact family transformations. They are not independent statistical replicates; the five seeds are the independent runs.

## Additional first-pass pathway: H–O–P participation in sulfur-containing products

A further exact-ID trace in seed 45 found three sequences where an H–O–P instance was consumed by COMBINE with sulfur, entered an H–O–P–S product, and was then released as a new H–O–P instance by BREAK:

| Initial H–O–P ID | COMBINE with sulfur | H–O–P–S product ID | BREAK | Regenerated H–O–P ID |
|---:|---|---:|---|---:|
| 2,296 | `12558-16` at tick 12,558 | 2,327 | `12753-7` at tick 12,753 | 2,350 |
| 2,544 | `16052-16` at tick 16,052 | 2,615 | `16395-9` at tick 16,395 | 2,641 |
| 2,772 | `19457-16` at tick 19,457 | 2,794 | `19458-15` at tick 19,458 | 2,795 |

This is a repeated reversible association/dissociation pattern in one seed. The H–O–P family reappears, but each regenerated molecule has a new identity. The evidence supports reversible intermediate turnover and temporary incorporation into a larger structure; it does not show that H–O–P catalyzes an independent reaction or remains unchanged while doing so.

## Interpretation

**Established by this first pass**
- The exact O–P fragment identity can survive a BREAK event and later participate as a COMBINE parent.
- In four listed cases, both exact fragments from a BREAK event are reused together to form a new molecule of the original family.
- H–O–P / O–P and N–O–P / O–P reversible transformations recur across independent seeds.

**Not established**
- Catalysis: no facilitator has been shown to increase a separate reaction rate while remaining chemically available.
- Autocatalysis: no net self-amplifying production mechanism has been demonstrated.
- Pseudo-metabolism: four reversible fragment-reuse loops do not establish a persistent, resource-fed network.
- Any causal advantage from resonance, geometry, or spatial organization.

## Coverage limitations

The geometry-aware report contains 4,305 committed reaction events, but complete participant geometry is available for 2,811 only. The remaining 1,494 events cannot support complete geometry-conditioned comparisons. The existing artifact also does not provide a matched facilitator-absent control or an unbiased per-instance record of every eligible reaction opportunity. Do not infer catalytic rates from committed-event counts.

## Next PLAN-001 work

1. Extend the exact-ID trace to the other recurrent multistep pathways, including H–O–P participation in sulfur-containing products and the N–O–P / O–P family transitions.
2. Build a ranked event-sequence table with creation, consumption, parent/child IDs, and time gaps for candidate intermediates.
3. Identify whether any molecule instance remains unchanged while a distinct reaction occurs nearby; do not label this facilitation without opportunity and control evidence.
4. Close PLAN-001 only after the candidate set, identity transitions, and missing-data limits are documented.

No reaction rules, thresholds, energy grants, or simulator behavior were changed.

## Broader exact-pair reunification audit

A broader scan of all two-child BREAK events found **192 cases** where both exact child molecule IDs later appeared together as the exact parent IDs of a COMBINE event. Counts by independent seed were: seed 42: 25; 43: 49; 44: 51; 45: 54; 46: 13. The median BREAK-to-COMBINE interval was 599 ticks (mean 1,308 ticks; maximum 15,689 ticks). Reunited products span 22 recorded composition/topology families.

This wider set includes ordinary dissociation/recombination and re-association of fragments from larger structures; it is not equivalent to the four highlighted H–O–P/N–O–P reversals. It is evidence that exact fragment instances can remain available and later reunite, not evidence of catalysis. The four candidate H–O–P/N–O–P cycles remain the strongest targeted examples because they return to the original composition/topology family.
