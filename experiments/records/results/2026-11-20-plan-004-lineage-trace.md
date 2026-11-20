# PLAN-004 — Exact-instance multistep lineage trace

**Status:** Retrospective lineage trace completed from EXP-2026-004. A connected multistage chemical sequence is identified; recurrent resource-fed persistence and pathway-specific energy coupling remain untested.

## Source and method

- Source experiment: [EXP-2026-004](2026-11-19-one-hour-five-seed.md), five one-hour runs, seeds 42–46.
- Source artifact: combined artifact ID `14564601713`, SHA-256 `e46720136d506b5e4e8a17f833ae10ede482c3facb5b7d2298bb1fd9ba03f898`.
- Instance-level sources: `geometry-aware-network/instance-reaction-events.csv` and `geometry-aware-network/molecule-instances.csv`.
- Method: link an event's child molecule identity to later events that consume that exact identity as a parent. Each displayed transition therefore has an exact identity link; co-reactants are shown separately. Family names encode composition/topology, not full geometry or energetic state.

## A connected eight-event sequence in seed 45

The strongest clear multistage example follows an oxygen–phosphorus-rich intermediate through two cycles of transformation and regeneration. It spans ticks 13,639–20,850.

| Tick / reaction ID | Event | Exact parents | Exact children |
|---|---|---|---|
| 13,639 / `13639-16` | DISPLACEMENT | 2357 (O–P₂), 2388 (P) | 2412 (O–P), 2413 (P₂) |
| 14,649 / `14649-16` | COMBINE | 2413 (P₂), 2442 (O) | 2483 (O–P₂) |
| 15,541 / `15541-16` | DISPLACEMENT | 2483 (O–P₂), 2136 (N) | 2581 (P₂), 2582 (N–O) |
| 15,649 / `15649-16` | COMBINE | 2581 (P₂), 2582 (N–O) | 2589 (N–O–P₂) |
| 15,679 / `15679-10` | BREAK | 2589 (N–O–P₂) | 2599 (O–P₂), 2600 (N) |
| 20,037 / `20037-16` | DISPLACEMENT | 2599 (O–P₂), 2356 (C) | 2857 (P₂), 2858 (C–O) |
| 20,599 / `20599-6` | BREAK | 2858 (C–O) | 2914 (O), 2915 (C) |
| 20,850 / `20850-16` | COMBINE | 2857 (P₂), 2914 (O) | 2938 (O–P₂) |

The exact lineage links are:
- 2413 → 2483 → 2581 → 2589 → 2599 → 2857 → 2938.
- In parallel, 2582 → 2589 and 2858 → 2914 → 2938.

This is more informative than a family-graph cycle alone: the same exact intermediate instance is consumed at each step, and the resulting products are tracked into later reactions. The O–P₂ family is regenerated as new molecule identity 2599 after the N–O–P₂ BREAK, then again as new identity 2938 after the C–O BREAK and subsequent COMBINE. It is therefore family/state regeneration with identity turnover, not persistence of one unchanged molecule.

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
