# PLAN-004 — Exact-instance multistep lineage trace

**Status:** Retrospective lineage trace completed from EXP-2026-004. A connected multistage chemical sequence is identified; recurrent resource-fed persistence and pathway-specific energy coupling remain untested.

## Source and method

- Source experiment: [EXP-2026-004](2026-11-19-one-hour-five-seed.md), five one-hour runs, seeds 42–46.
- Source artifact: combined artifact ID `14564601713`, SHA-256 `e46720136d506b5e4e8a17f833ae10ede482c3facb5b7d2298bb1fd9ba03f898`.
- Instance-level sources: `geometry-aware-network/instance-reaction-events.csv` and `geometry-aware-network/molecule-instances.csv`.
- Method: link an event's child molecule identity to later events that consume that exact identity as a parent. Each displayed transition therefore has an exact identity link; co-reactants are shown separately. Family names encode composition/topology, not full geometry or energetic state.

## A connected eight-event sequence in seed 45

The strongest clear multistage example follows an oxygen–phosphorus-rich intermediate through two cycles of transformation and regeneration. It spans ticks 13,639–20,850.

| Tick / reaction ID | Event | Exact parents | Exact children | Recorded `work_required` |
|---|---|---|---|
| 13,639 / `13639-16` | DISPLACEMENT | 2357 (O–P₂), 2388 (P) | 2412 (O–P), 2413 (P₂) | 0.106793 |
| 14,649 / `14649-16` | COMBINE | 2413 (P₂), 2442 (O) | 2483 (O–P₂) | 0.350000 |
| 15,541 / `15541-16` | DISPLACEMENT | 2483 (O–P₂), 2136 (N) | 2581 (P₂), 2582 (N–O) | 0.114849 |
| 15,649 / `15649-16` | COMBINE | 2581 (P₂), 2582 (N–O) | 2589 (N–O–P₂) | 0.350000 |
| 15,679 / `15679-10` | BREAK | 2589 (N–O–P₂) | 2599 (O–P₂), 2600 (N) | 0.264575 |
| 20,037 / `20037-16` | DISPLACEMENT | 2599 (O–P₂), 2356 (C) | 2857 (P₂), 2858 (C–O) | 0.113472 |
| 20,599 / `20599-6` | BREAK | 2858 (C–O) | 2914 (O), 2915 (C) | 0.308221 |
| 20,850 / `20850-16` | COMBINE | 2857 (P₂), 2914 (O) | 2938 (O–P₂) | 0.350000 |

The eight matched raw `environmental_reaction` records report a combined `work_required` of 1.957910 simulation energy units (per-event values shown above). This is the recorded work requirement for these committed transformations, **not** a pathway energy balance: the records do not attribute actual energy withdrawals, resonance funding, stored-energy changes, heat, or losses to this lineage.

The exact lineage links are:
- 2413 → 2483 → 2581 → 2589 → 2599 → 2857 → 2938.
- In parallel, 2582 → 2589 and 2858 → 2914 → 2938.

This is more informative than a family-graph cycle alone: the same exact intermediate instance is consumed at each step, and the resulting products are tracked into later reactions. The O–P₂ family is regenerated as new molecule identity 2599 after the N–O–P₂ BREAK, then again as new identity 2938 after the C–O BREAK and subsequent COMBINE. It is therefore family/state regeneration with identity turnover, not persistence of one unchanged molecule.

## Initial free-resource identity turnover

The molecule-instance ledger also allows a limited estimate of how much of the original free-resource inventory entered chemistry at least once. Across the five seeds, **2,167 of 6,000 initial free-resource identities (36.1%)** were consumed as parents in a reaction during the one-hour run. Per seed, the count ranged from 310 to 513 of 1,200 (25.8%–42.8%).

Mean initial free-resource identities consumed per seed, by element:
- Carbon: 68.8 of 191.2 initially
- Hydrogen: 93.2 of 201.4
- Nitrogen: 42.2 of 202.2
- Oxygen: 108.8 of 203.8
- Phosphorus: 56.2 of 196.8
- Sulfur: 64.2 of 204.6


At the final snapshot, the total number of active free, unbonded single-element identities was 1,215 / 1,201 / 1,177 / 1,180 / 1,218 for seeds 42–46 respectively (mean 1,198.2), compared with 1,200 at tick zero. Thus, despite hundreds of original free-resource identities entering reactions, the aggregate free-element pool was nearly unchanged by the end, consistent with substantial internal recycling. This is a pool-count comparison, not a full atom-by-atom flux or proof of a sustained cycle.

This is an **identity-turnover count**, not net depletion or elemental consumption: an atom may be released again under a new molecule identity after BREAK, and reaction-created identities are not counted as initial free resources. It provides a rough scale for designing a feed intervention, but should not be used alone to set the feed composition or claim a resource requirement.

## Recurrent turnover evidence

The earlier exact-instance screen also found three seed-45 H–O–P / sulfur association-dissociation sequences:
- H–O–P identity 2296 + S → H–O–P–S identity 2327; BREAK later released new H–O–P identity 2350.
- H–O–P identity 2544 + S → H–O–P–S identity 2615; BREAK later released new H–O–P identity 2641.
- H–O–P identity 2772 + S → H–O–P–S identity 2794; BREAK immediately released new H–O–P identity 2795.

Together with the cross-seed H–O–P ⇌ H + O–P and N–O–P ⇌ N + O–P transformations, these observations establish repeated reversible turnover and multistage connected chemistry in the finite starting inventory.

## What this establishes — and what it does not

**Established:** exact molecule-instance histories contain connected, temporally ordered multistage transformations; several intermediate families are regenerated after consumption; this is not merely a pooled family graph.

**Not established:** pseudo-metabolism, because the experiment has no recurrent external resource input; a self-maintaining cycle; persistence under depletion; net resource conversion; catalytic or autocatalytic amplification; or energy coupling specific to this pathway.

The event/lineage artifacts do not provide a pathway-specific allocation of stored energy, environmental input, heat, and losses. The run-level energy and bond-inventory validation remains valid, but cannot be assigned to this eight-event path.

The sequence also uses a finite set of initially seeded constituents. Repeated use of exact molecule IDs across successive events demonstrates lineage continuity, not new matter entering the system. Regenerated family identities are newly created products, not proof that the environment replenished the consumed elemental inventory.

## Minimum next experiment prerequisite

The standard `--environment-chemistry-experiment` runner builds one fixed initial inventory and then observes it. The current `.github/workflows/chemistry-experiments.yml` exposes seed, duration, snapshot interval, and profiling flags, but no timed resource-feed condition or matched no-feed arm.

A valid resource-fed PLAN-004 run therefore cannot be launched through the current standard workflow. The next experiment should use identical initial conditions and seeds in paired arms:
1. **No-feed control:** existing standard environmental chemistry condition.
2. **Resource-input arm:** same initial inventory and ordinary chemistry, with a predeclared, deterministic schedule adding only free base resources to the environment. No compound, reaction, or energy is injected.

The feed schedule, elemental composition, quantity, and timing must be declared before the run. Existing diagnostics should remain unchanged. Analyze connected pathways, intermediate turnover, persistence, elemental inventory, and whole-run energy/heat/losses across the paired arms. Treat seeds—not reaction events—as independent replicates.

**Blocker:** implementing the timed environmental input and paired-arm execution is a new experiment capability, not an existing configuration option. No simulator code or chemistry rules were changed as part of this retrospective trace.
