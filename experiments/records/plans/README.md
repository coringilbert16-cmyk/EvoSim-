# Organized-chemistry research plans

These plans define questions and evidence standards, not claims that the behaviors already exist. The objective is to discover naturally arising organization in the general chemistry engine. No plan authorizes special-case reactions, organism-specific logic, forced reactions, artificial energy grants, or threshold changes.

The EXP-2026-004 one-hour, five-seed dataset is the starting point. Analyze retained artifacts before commissioning new simulations. PLAN-001 artifact analysis is complete; no new simulation was required.

## PLAN-001 — Existing pathway and molecular-lineage analysis

**Status:** Completed for EXP-2026-004 artifact analysis; no new simulation.

**Question:** Which instance-level reaction sequences demonstrate reusable intermediates, reversible transformations, multistep synthesis, or plausible facilitation?

**Work:** Trace exact molecule IDs across parent/child events; distinguish identity persistence, consumption/recreation, and family-only similarity; reconstruct chronological pathways; separate substrate recycling from facilitation; rank candidates with seed, tick, reaction ID, parent/child IDs, and artifact provenance; document geometry and opportunity coverage limits.

**Success criterion:** A reproducible instance-level evidence table and ranked follow-up candidates. No new simulation is required.

**Second-pass result:** Exact-ID lineage walks reach 25, 31, 23, 39, and 12 connected reaction events in seeds 42–46, respectively. These are chronological identity-linked event walks, not single reactions or closed metabolic cycles. The 192 exact-pair reunifications are dominated by simple two-constituent families. A targeted audit found no event-level evidence of an unchanged third-party facilitator; current contact samples are pairwise and do not provide matched facilitator controls. See [initial findings and evidence audit](../results/2026-11-20-plan-001-initial-findings.md).

**Current candidate ranking:** (1) N–O–P ⇌ N + O–P, with four exact-fragment cycles across three seeds and complete participant geometry; (2) H–O–P ⇌ H + O–P, with one exact-fragment cycle; (3) H–O–P + S ⇌ H–O–P–S, with three exact-identity incorporation/release sequences in seed 45, one of the three initial COMBINE events lacking complete geometry. These are pathway/intermediate candidates, not facilitator candidates.

**Initial findings:** Four exact-instance fragment-reuse cycles are confirmed: three N–O–P ⇌ N + O–P and one H–O–P ⇌ H + O–P. Three additional exact-instance sequences show H–O–P temporarily incorporated into an H–O–P–S product and released again by BREAK. A broader scan identified 192 exact-pair fragment reunifications across the five seeds. See [PLAN-001 initial findings](../results/2026-11-20-plan-001-initial-findings.md). These establish reversible intermediate/substrate reuse, not catalysis.

## PLAN-002 — Catalytic facilitation

**Status:** Planned; blocked pending a defensible facilitator candidate and local-neighborhood/opportunity diagnostics. PLAN-001 found reusable pathway intermediates but no evidence identifying a molecule that facilitates a separate reaction while remaining available.

**Question:** Does a molecule increase the likelihood or rate of a reaction while remaining chemically available, rather than being consumed as a reactant?

**Required evidence:** A defined facilitator and target reaction; exact instance and graph tracking; matched facilitator-present/absent conditions; rates normalized for reactant concentration, contact/opportunity exposure, geometry, resonance, and energy availability; repeated effects across independent seeds; full material and energy accounting.

**Controls:** Match initial inventory, reactant concentrations, geometry, field exposure, and energy conditions. A molecule consumed and replaced by a same-family product is not a catalyst. Repeated participation or higher raw event counts alone are insufficient.

**Infrastructure prerequisite:** The current environmental workflow is baseline-only and has no paired facilitator control. Comparative execution and opportunity diagnostics are required before a controlled run.

**Minimum viable controlled protocol (design only; not yet implemented):**
1. Select a target reaction and a candidate molecule that is not one of its reactant substrates.
2. Add event-level local-neighborhood records: nearby molecule IDs/families, distances and relative geometry, local field/resonance state, and the exact target-reactant opportunity being evaluated. Record both successful and unsuccessful eligible opportunities, not only committed events.
3. Run paired deterministic setups with the same target-reactant inventory and environment. In the treatment, place the candidate in the target reaction zone; in the primary control, keep the same candidate and elemental inventory but place it outside the target's interaction neighborhood. If long-range field coupling makes relocation an invalid control, add a separately justified control that preserves elemental inventory while removing the candidate structure.
4. Compare target-reaction commits per eligible contact/opportunity exposure, with reactant availability, geometry, field exposure, and energy availability reported. Use independent seed pairs and uncertainty intervals.
5. Trace candidate material through all reactions. A candidate that is consumed as a target substrate is not a facilitator; a temporary intermediate must be regenerated and available for repeated cycles.
6. Require reproducible rate enhancement, correct material/energy accounting, and no special-case reaction logic before calling the result catalytic facilitation.

This protocol is a design prerequisite only. It does not authorize code changes or a simulation run.

## PLAN-003 — Autocatalysis

**Status:** Planned; depends on PLAN-001 and a defensible self-production pathway. PLAN-002 may help define the mechanism.

**Question:** Does a product promote production of additional instances of its own chemical structure?

**Required evidence:** A candidate product family and net production pathway; instance-level link between product abundance and its own production rate; matched seeded/unseeded controls; net amplification beyond recycling or the trivial effect of initial product abundance; time-resolved abundance, constituent conservation, energy input, heat/losses, and uncertainty across seeds.

**Interpretation:** A family-graph cycle, product reuse, or a larger seeded inventory is not proof. The product must causally promote additional production under controlled comparison.

**Infrastructure prerequisite:** Reproducible paired conditions and a way to seed only the candidate product without changing the rest of the initial inventory.

## PLAN-004 — Pseudo-metabolism

**Status:** Planned; depends on validated pathways and intermediate regeneration from PLAN-001, with relevant facilitation/self-production tests where applicable.

**Question:** Can a connected chemical network repeatedly transform environmental resources, turn over intermediates, and regenerate compounds needed by earlier stages?

**Required evidence:** A connected, temporally ordered network; recurrent resource input and measurable turnover; demonstrated regeneration of intermediates with identity changes explicitly tracked; persistence over repeated cycles; complete accounting of energy input, stored energy, heat, losses, and residuals; no organism-specific controller.

**Initial candidates:** H–O–P ⇌ H + O–P and N–O–P ⇌ N + O–P are reversible transformations to investigate, not evidence of metabolism.

## PLAN-005 — Cross-catalysis and cooperative chemical networks

**Status:** Planned; depends on PLAN-001 and controlled facilitation evidence from PLAN-002.

**Question:** Can distinct molecular families facilitate one another's production or turnover, creating a network whose activity depends on interactions between pathways?

**Required evidence:** At least two independently defined pathways and a proposed cross-facilitation relationship; exact identity tracking; matched A-alone, B-alone, and A+B conditions; rates/yields normalized for inventory, concentrations, geometry, contact opportunity, field exposure, and energy; reproducible effects beyond simply having more reactants or reaction opportunities; complete material and energy accounting.

**Interpretation:** Co-occurrence, shared intermediates, or higher total reaction counts in a larger mixture do not establish cooperation. The interaction must be attributable to a defined pathway and survive matched controls.

## PLAN-006 — Spatially organized chemistry

**Status:** Planned; requires controlled comparative execution and adequate local diagnostics.

**Question:** Do flow, resonance propagation, energy gradients, and transport create persistent spatial reaction zones or connected pathways not visible in well-mixed aggregate statistics?

**Required evidence:** Spatially resolved positions, concentrations, reactions, and local field/energy exposure over time; matched spatial conditions and inventories; local rates and pathway/family diversity normalized for local reactant availability and contact opportunity; zone persistence and movement; causal separation from placement artifacts or unequal material distribution; full energy/material accounting.

**Infrastructure prerequisite:** The lab exposes flow and source controls and supports deterministic tick-zero placement, but not automated matched multi-condition runs. The environmental workflow is baseline-only. Build comparison and spatial-diagnostic capability before interpreting intervention experiments.

## Research sequence

1. Complete PLAN-001 from EXP-2026-004 artifacts; do not repeat the generic baseline.
2. Select instance-level pathways and distinguish reusable substrates from possible facilitators.
3. Build the minimum controlled-comparison capability for PLAN-002.
4. Test autocatalysis only if a plausible self-production mechanism emerges.
5. Evaluate pseudo-metabolism after repeated pathway turnover and regeneration are demonstrated.
6. Investigate cross-catalysis after individual facilitation is operationally established.
7. Pursue spatial organization once spatially resolved opportunity and rate comparisons are available.

All future experiments must be registered before execution and receive a dedicated result record at closeout. Plans do not authorize simulator behavior changes; implementation requires separate design approval.
