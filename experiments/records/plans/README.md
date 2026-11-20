# Organized-chemistry research plans

These plans define questions and evidence standards, not claims that the behaviors already exist. The research objective is to discover naturally arising organization in the general chemistry engine. No plan authorizes special-case reactions, organism-specific logic, forced reactions, artificial energy grants, or threshold changes.

The existing EXP-2026-004 one-hour, five-seed dataset is the starting point. Analyze retained artifacts before commissioning new simulations. PLAN-001 is now in progress.

## PLAN-001 — Existing pathway and molecular-lineage analysis

**Status:** In progress; existing-artifact analysis only.

**Question:** Which instance-level reaction sequences demonstrate reusable intermediates, reversible transformations, multistep synthesis, or plausible facilitation?

**Starting evidence:** EXP-2026-004 recorded 4,200 distinct reaction-created product identities later reused as parents, 6,444 parent-lineage edges from reaction-created products, and 1,956 such edges leading to COMBINE. H–O–P / O–P and N–O–P / O–P are recurrent family-level transformations.

**Work:**
1. Trace exact molecule instance IDs across parent/child events.
2. Distinguish unchanged identity/state, consumption and recreation, and family-only similarity.
3. Reconstruct chronological pathways and exact intermediate reuse.
4. Separate reversible substrate recycling from a molecule facilitating a separate reaction.
5. Rank candidates with seed, tick, reaction ID, parent IDs, child IDs, and artifact provenance.
6. Record geometry and opportunity coverage limits; do not infer reaction probabilities from committed events alone.

**Success criterion:** a reproducible instance-level evidence table and a ranked set of follow-up candidates. No new simulation is required.

**Current progress:** Four complete exact-instance fragment-reuse cycles have been identified in the first pass: three N–O–P ⇌ N + O–P cycles and one H–O–P ⇌ H + O–P cycle. See [PLAN-001 initial findings](../results/2026-11-20-plan-001-initial-findings.md). These establish reversible intermediate recycling, not catalysis.

## PLAN-002 — Catalytic facilitation

**Status:** Planned; depends on PLAN-001 identifying a defensible candidate.

**Question:** Does a molecule increase the likelihood or rate of a reaction while remaining chemically available, rather than being consumed as a reactant?

**Required evidence:**
- A defined facilitator candidate, target reaction, and operational definition of facilitator availability.
- Exact instance and bond-graph tracking before, during, and after facilitated events.
- Matched facilitator-present and facilitator-absent conditions.
- Reaction rates normalized for reactant concentration, contact/opportunity exposure, geometry, resonance and energy availability.
- Repeated events and uncertainty across independent seeds.
- Full material and energy accounting.

**Controls and interpretation:** Initial inventory, reactant concentrations, geometry, field exposure, and energy conditions must be matched. A molecule consumed in a reaction and replaced by a same-family product is not a catalyst. Repeated participation or higher raw event counts alone are insufficient.

**Infrastructure prerequisite:** the current environmental workflow runs one fixed baseline condition and has no paired facilitator control. Comparative setup execution and comparable opportunity diagnostics must be implemented or supplied by a suitable laboratory workflow before a controlled run.

## PLAN-003 — Autocatalysis

**Status:** Planned; depends on PLAN-001 and a defensible self-production pathway. PLAN-002 findings may help define the mechanism.

**Question:** Does a product promote production of additional instances of its own chemical structure?

**Required evidence:**
- A candidate product family and a defined net production pathway.
- Instance-level evidence connecting product abundance to its own production rate.
- Matched seeded and unseeded controls with otherwise equivalent conditions.
- Net amplification beyond recycling, repeated independent formation, or the trivial effect of starting with more product.
- Time-resolved product abundance, constituent conservation, energy input, heat/losses, and uncertainty across seeds.

**Interpretation:** a family-graph cycle, product reuse, or an initially larger product inventory is not proof of autocatalysis. The product must causally promote additional production under controlled comparison.

**Infrastructure prerequisite:** reproducible paired conditions and a way to seed only the candidate product without changing the rest of the initial inventory.

## PLAN-004 — Pseudo-metabolism

**Status:** Planned; depends on validated pathways and intermediate regeneration from PLAN-001, with relevant facilitation/self-production tests from PLAN-002/003 where applicable.

**Question:** Can a connected chemical network repeatedly transform environmental resources, turn over intermediates, and regenerate compounds needed by earlier stages?

**Required evidence:**
- A connected, temporally ordered reaction network, not merely a collection of unrelated events.
- Recurrent resource input and measurable product/intermediate turnover.
- Demonstrated regeneration of intermediate identities or chemically equivalent states with explicit accounting of identity changes.
- Persistence over repeated cycles.
- Energy input, stored energy, heat, losses, and residuals accounted for across the network.
- Evidence that persistence follows from general chemistry and environmental physics, not an organism-specific controller.

**Initial candidates:** H–O–P ⇌ H + O–P and N–O–P ⇌ N + O–P are candidate reversible intermediate transformations. They are not yet metabolism.

**Current status (2026-11-20):** Initial five-seed artifact screen completed using EXP-2026-004, without a new simulation. Recurrent family-level turnover and three H–O–P / sulfur association-dissociation sequences were identified, but no persistent resource-fed multi-step network has been demonstrated. See [PLAN-004 initial screen](../results/2026-11-20-plan-004-006-initial-screens.md). Continue exact-instance tracing of longer connected sequences; a separate recurrent-resource-input experiment is required to test persistence under replenishment.

## PLAN-005 — Cross-catalysis and cooperative chemical networks

**Status:** Planned; depends on PLAN-001 and controlled facilitation evidence from PLAN-002.

**Question:** Can two or more distinct molecular families facilitate one another's production or turnover, creating a network whose activity depends on interactions between pathways?

**Required evidence:**
- At least two independently defined candidate pathways and a proposed cross-facilitation relationship.
- Exact molecule-instance tracking to distinguish facilitators from consumed substrates and regenerated products.
- Matched conditions for pathway A alone, pathway B alone, and A+B together.
- Rates and product yields normalized for total inventory, reactant concentration, geometry, contact opportunity, field exposure, and energy availability.
- Evidence that the combined network has a reproducible effect beyond simply containing more reactants or more reaction opportunities.
- Material and energy accounting across the connected network.

**Interpretation:** co-occurrence, shared intermediates, or higher total reaction counts in a larger mixture do not establish cooperation. The interaction must be attributable to a defined chemical pathway and survive matched controls.

## PLAN-006 — Spatially organized chemistry

**Status:** Planned; can be designed from existing spatial/field capabilities, but requires controlled comparative execution and adequate local diagnostics.

**Question:** Do flow, resonance propagation, energy gradients, and transport create persistent spatial reaction zones or connected chemical pathways that would not be apparent from well-mixed aggregate statistics?

**Required evidence:**
- Reproducible spatially resolved molecular positions, concentrations, reaction events, and local field/energy exposure over time.
- Defined spatial conditions (for example, baseline versus altered flow or field geometry) with matched inventories and seeds.
- Local reaction rates and pathway/family diversity normalized for local reactant availability and contact opportunity.
- Persistence, movement, formation, and disappearance of reaction zones.
- Evidence that observed organization is caused by environmental physics rather than a placement artifact or unequal material distribution.
- Full energy and material accounting.

**Infrastructure prerequisite:** the current lab exposes flow and source controls and supports deterministic tick-zero placement, but does not provide automated matched multi-condition runs. The existing environmental workflow is baseline-only. Build the comparison and spatial-diagnostic capability before interpreting a spatial intervention experiment.

**Current status (2026-11-20):** A descriptive baseline screen of EXP-2026-004 geometry is complete. Of 4,305 committed events, 2,811 have complete participant geometry; those events show a possible spatial concentration, but substantial seed-to-seed variation. The data lack complete local opportunity denominators and matched environmental conditions, so causal spatial organization and exposure-normalized rates cannot be inferred. See [PLAN-006 initial screen](../results/2026-11-20-plan-004-006-initial-screens.md).

## Research sequence

1. Complete PLAN-001 from EXP-2026-004 artifacts; do not repeat the generic baseline.
2. Select the strongest instance-level pathways and distinguish reusable substrates from possible facilitators.
3. Build the minimum controlled-comparison capability needed for PLAN-002.
4. Test autocatalysis only if a plausible self-production mechanism emerges.
5. Continue PLAN-004 by tracing longer exact-instance pathway sequences; design recurrent resource-input conditions only after a defensible network is identified.
6. Investigate cross-catalysis after individual facilitation is operationally established.
7. Use the PLAN-006 baseline spatial concentration as a hypothesis only; pursue causal spatial comparisons only when matched conditions and adequate local opportunity denominators are available.

All future experiments must be registered before execution and receive a dedicated result record at closeout. Plans do not authorize simulator behavior changes; implementation requires a separate explicit design decision and approval.
