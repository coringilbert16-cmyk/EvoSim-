# EXP-2026-004 — Five-seed one-hour environmental chemistry observation

## Run identity

- Date: 2026-11-19
- GitHub Actions run: [Run #74](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/44582565701)
- Run ID: `44582565701`
- Commit: `8e7cc7d624e3131b2d99889fef02d9d5c3095318`
- Seeds: 42, 43, 44, 45, 46
- Duration: 3,600 seconds (one hour) per seed; 18,000 seconds aggregate simulated wall-clock runtime
- Condition: ordinary environmental chemistry; local resonance enabled; no forced reactions and no comparison/control condition
- Initial inventory per seed: 1,200 independent resources plus 80 established two-constituent molecules; 1,360 total constituents; Carbon, Hydrogen, Nitrogen, Oxygen, Phosphorus, Sulfur; no ambient medium
- Diagnostics: snapshots every 100 ticks; twelve equal 300-second temporal-analysis windows
- Artifact: `chemistry-experiment-combined`, artifact ID `14564601713`; six total artifacts were uploaded by the run
- Artifact retention at inspection: aggregate retained until 2027-02-17; per-seed artifacts until 2026-12-19

## Per-seed results

| Seed | Committed reactions | Unique structural families | Compound products |
|---:|---:|---:|---:|
| 42 | 639 | 73 | 422 |
| 43 | 1,153 | 86 | 767 |
| 44 | 985 | 59 | 617 |
| 45 | 1,082 | 79 | 712 |
| 46 | 446 | 40 | 267 |
| **Total / mean** | **4,305 total** | **67.4 mean per seed** | **557 mean per seed** |

## Reaction totals

| Reaction | Ready | Scheduled | Committed | Rejected |
|---|---:|---:|---:|---:|
| BREAK | 5,450,813 | 1,757,584 | 1,914 | 1,755,670 |
| COMBINE | 2,942 | 1,960 | 1,957 | 3 |
| DISPLACEMENT | 443 | 434 | 434 | 0 |

All 4,305 committed events had complete graph-delta attribution. The analyzer recorded 2,391 formed bonds and 2,348 broken bonds; initial-to-final bond inventories reconciled for every seed. No malformed bond records or missing reaction graph deltas were reported.

## Molecular reuse and network development

- Molecular identities created: 13,053.
- Molecular identities destroyed: 6,696.
- Mean active molecules per seed at the final snapshot: 1,271.4.
- Recorded lineage edges: 9,478.
- Distinct reaction-created product identities later reused as parents: 4,200.
- Parent-lineage edges from reaction-created products: 6,444.
- Of those reuse edges, 1,956 led to COMBINE products.
- Distinct composition/topology families: 158.
- Reaction-created families recurring in at least two seeds: 78.
- Distinct reaction pathway patterns: 426; 178 recurred in at least two seeds.

The 4,200 reused product identities and 6,444 lineage edges are different measures: one molecule identity can participate in more than one later event.

## Strong recurring product families

| Product family | Seeds | Created identities | Reused identities | Active at end |
|---|---:|---:|---:|---:|
| H–O | 5 | 494 | 430 | 64 |
| C–H | 5 | 285 | 282 | 3 |
| O–P | 5 | 284 | 234 | 50 |
| H–S | 5 | 242 | 232 | 10 |
| C–O | 5 | 238 | 222 | 16 |
| N–O | 5 | 238 | 229 | 9 |
| P–S | 5 | 205 | 181 | 24 |

These are composition-and-bond-topology families. The family definition does not include bond energy, endpoint geometry, placement, or environmental identity.

## Promising organized-chemistry observations

### 1. Reusable intermediates and multistep synthesis

The records show that reaction-created products frequently participate in later reactions. This is stronger evidence than a simple count of unique compounds: 4,200 distinct reaction-created identities were later reused as parents, and 1,956 reuse lineage edges led to COMBINE products.

Examples of recurrent multistep chemistry include:

- O–P plus H forms H–O–P: 16 COMBINE events across all five seeds. H–O–P also underwent BREAK to H plus O–P: 15 events across four seeds. This is a recurring reversible transformation candidate.
- N–O–P formed from N plus O–P: 13 COMBINE events across four seeds. N–O–P also broke into N plus O–P: 13 events across all five seeds. This is another recurrent reversible transformation candidate.
- H–O plus P formed H–O–P in nine COMBINE events across four seeds.
- H–O–P also appeared in further reactions with S and other molecular families, including occasional larger products.

These observations support recurring pathways and reusable intermediates. They do not establish that a molecule catalyzes its own production or remains unchanged as a facilitator.

### 2. Network recurrence

The reaction network contains 426 pathway patterns, with 178 recurring in at least two independent seeds. The pooled family-transition graph has a large strongly connected component, but this is expected to include reversible BREAK/COMBINE transformations. Graph connectivity alone cannot establish catalytic turnover or metabolism.

The temporal analyzer reported the following mean changes across the five seeds. Its separate per-seed fingerprint counts were 72, 85, 57, 78, and 38; these use a different fingerprint pipeline and should not be substituted for the exact structural-family counts above:

| Window | Cumulative structural fingerprints | Cumulative reaction patterns | Mean active-product size | Mean maximum product size |
|---:|---:|---:|---:|---:|
| 1 | 23.2 | 33.0 | 2.09 | 3.0 |
| 4 | 36.2 | 72.2 | 2.11 | 4.0 |
| 8 | 53.6 | 126.4 | 2.21 | 4.6 |
| 12 | 67.4 | 166.4 | 2.26 | 4.8 |

Maximum product size per seed at the end ranged from 5 to 7 constituents. The mean active-product size rose modestly; this is evidence of increased structural diversity and some growth in product size, not a demonstrated trend toward complex polymers.

### 3. Catalysis and autocatalysis: current evidence

**Not confirmed.** The artifacts show product reuse, reversible transformations, and candidate cycles. They do not show that a molecule remains chemically available while facilitating another reaction, that its presence increases reaction rate against a matched control, or that a product causes net production of more copies of itself.

In particular, a family-level cycle such as O–P → H–O–P → O–P can arise through ordinary COMBINE and BREAK events in which individual molecule identities are consumed and new identities are created. That is chemical recycling / reversible interconversion, not by itself autocatalysis.

### 4. Contact and geometry coverage limits

The separate contact-opportunity report pooled 109,849 diagnostic ticks. It recorded 65,017 displacement replacement-contact searches, 1,922 contacts found, 443 activation-threshold passes, and zero post-activation trial rejections. These are aggregate counters, not molecule-specific opportunity histories.

Committed participant geometry was complete for 2,811 of 4,305 reaction events. The remaining 1,494 events lack complete participant geometry. Consequently, this dataset supports descriptive geometry comparisons but not unbiased geometry-conditioned reaction probabilities.

## Validation and energy accounting

- All five requested seeds were present.
- All five runs matched the requested 3,600-second duration.
- All per-seed inventory and lifecycle checks passed.
- Maximum absolute snapshot energy residual: 0.0011785030364990234.
- Displacement discovery-budget-exhausted ticks: 109,849.

## Research conclusions

1. **Established:** the environment supports repeated multistep chemistry and widespread reuse of reaction-created molecules as later reactants.
2. **Established:** recurring pathway patterns and product families occur across independent seeds.
3. **Promising candidates:** H–O–P / O–P and N–O–P / O–P reversible transformations merit targeted follow-up as reusable-intermediate cycles.
4. **Not established:** catalysis, autocatalysis, pseudo-metabolism, net self-amplification, or self-sustainability.
5. **Next research priority:** analyze the actual instance-level lineage and event sequences for candidate intermediates, then design controlled experiments that test facilitation, regeneration, and persistence. Do not repeat the one-hour baseline merely to recreate these observations.

## Interpretation limits

- The five seeds are the independent replicates; reactions within a seed are not independent samples.
- Family matching uses composition and undirected bond topology, not bond energy or endpoint geometry.
- Lineage reuse means a product identity participates later; it does not prove a productive synthesis pathway or catalysis.
- Active lifetimes are right-censored at the final snapshot.
- Material totals across lineage generations may count conserved constituents more than once.
- Structural fingerprints use iterative Weisfeiler–Lehman refinement and composition/degree summaries; they are descriptors, not exact graph-isomorphism proofs.
- Candidate directed cycles are not evidence by themselves of autocatalysis, energy coupling, or self-sustainability.
