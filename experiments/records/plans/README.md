# Planned organized-chemistry research

These are research plans, not approved simulator changes or claims that the behaviors already exist. Begin with analysis of retained data; do not launch redundant generic baselines.

## PLAN-001 — Existing pathway analysis

**Status:** Planned; first step before new simulation runs.

**Question:** Which instance-level reaction sequences in EXP-2026-004 show repeated use of molecular intermediates, reversible transformations, or possible facilitation?

**Starting evidence:** H–O–P / O–P and N–O–P / O–P recurring reversible pathways; 4,200 distinct reaction-created product identities reused as parents; 1,956 reuse lineage edges leading to COMBINE.

**Work:**
1. Trace exact molecule instance IDs across parent/child events, not just family fingerprints.
2. Identify whether a molecular identity persists unchanged through an event, is consumed and recreated, or merely shares a family with its successor.
3. Reconstruct chronological multistep pathways for the strongest recurrent families.
4. Separate reversible substrate recycling from a molecule facilitating another reaction.
5. Produce a ranked candidate list with exact seed, tick, reaction ID, parent IDs, child IDs, and supporting artifact rows.

**Success criterion:** a reproducible, instance-level evidence table of candidate pathways and unresolved questions. This is analysis of existing artifacts; no new run is required.

## PLAN-002 — Catalysis

**Status:** Planned; depends on PLAN-001.

**Question:** Does a molecule facilitate a reaction while remaining chemically available rather than being consumed as a reactant?

**Required evidence:**
- Identify a candidate facilitator and the reaction it may facilitate.
- Demonstrate repeated facilitated events while tracking the facilitator's exact identity and bond graph.
- Compare matched conditions with and without the candidate facilitator.
- Measure reaction frequency/rate and account for environmental exposure, contact opportunity, resonance/energy availability, and reactant concentrations.
- Confirm that any observed rate difference is not caused by different initial geometry or material inventory.

**Interpretation:** repeated participation alone is insufficient. If the candidate is consumed and replaced by a same-family product, that is not catalyst regeneration unless the relevant chemical identity/state and net reaction cycle are demonstrated.

## PLAN-003 — Autocatalysis

**Status:** Planned; depends on PLAN-001 and a defensible reaction-network candidate.

**Question:** Does a product promote production of additional copies of its own chemical structure?

**Required evidence:**
- A candidate product family and a defined net production pathway.
- Instance-level evidence that product abundance influences its own production rate.
- Matched seeded and unseeded controls with the same other conditions.
- Net amplification beyond simple recycling, repeated independent formation, or an initially larger starting inventory.
- Full constituent and energy accounting; no forced reactions or artificial energy grants.

**Interpretation:** a directed family-graph cycle is only a candidate. It is not proof of autocatalysis.

## PLAN-004 — Pseudo-metabolism

**Status:** Planned; build on validated pathways from PLAN-001 through PLAN-003.

**Question:** Can a connected chemical network repeatedly transform environmental resources, reuse intermediates, and regenerate compounds required by earlier stages?

**Required evidence:**
- A connected, temporally ordered reaction network rather than a collection of unrelated events.
- Recurrent resource input and product/intermediate turnover.
- Demonstrated regeneration of one or more network intermediates.
- Persistence over repeated cycles with measured energy input, losses, heat, and residuals.
- Evidence that network activity is maintained by its chemistry rather than an organism-specific controller.

**Initial candidates:** the recurring H–O–P ⇌ H + O–P and N–O–P ⇌ N + O–P transformations in EXP-2026-004. These are candidate reversible intermediate cycles, not yet metabolism.

## Research sequencing

1. Complete PLAN-001 using existing one-hour artifacts.
2. Choose the strongest instance-level candidate(s) and define catalyst/non-catalyst states.
3. Design a controlled catalysis experiment.
4. Pursue autocatalysis only when a plausible self-production pathway is identified.
5. Evaluate pseudo-metabolism as a network-level property after the relevant pathways and cycles are validated.

No plan authorizes changing reaction rules, thresholds, energy grants, organism-specific behavior, or baseline chemistry. Those require separate design decisions and approval.
