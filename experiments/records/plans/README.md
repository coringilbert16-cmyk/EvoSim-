# Organized-chemistry research plans

These plans define questions and evidence standards, not claims that the behaviors already exist. The research objective is to discover naturally arising organization in the general chemistry engine. No plan authorizes special-case reactions, organism-specific logic, forced reactions, artificial energy grants, or threshold changes.

The existing EXP-2026-004 one-hour, five-seed dataset is the starting point. PLAN-001 artifact analysis is complete; subsequent work must use the archived findings rather than repeat the generic baseline.

## PLAN-001 — Existing pathway and molecular-lineage analysis

**Status:** Completed for EXP-2026-004; existing-artifact analysis only.

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

**Status:** First local-exposure telemetry increment implemented; full exposure matching and candidate selection remain blocked. No catalytic experiment is authorized.

**Question:** Does a molecule increase the likelihood or rate of a reaction while remaining chemically available, rather than being consumed as a reactant?

**Required evidence:**
- A defined facilitator candidate, target reaction, and operational definition of facilitator availability.
- Exact instance and bond-graph tracking before, during, and after facilitated events.
- Matched facilitator-present and facilitator-absent conditions.
- Reaction rates normalized for reactant concentration, contact/opportunity exposure, geometry, resonance and energy availability.
- Repeated events and uncertainty across independent seeds.
- Full material and energy accounting.

**Controls and interpretation:** Initial inventory, reactant concentrations, geometry, field exposure, and energy conditions must be matched. A molecule consumed in a reaction and replaced by a same-family product is not a catalyst. Repeated participation or higher raw event counts alone are insufficient.

**Infrastructure progress:** PR #392 adds bounded local-neighborhood telemetry, reaction-source traceability, paired candidate-near/remote fixture construction, and a manual-only workflow. A first exposure telemetry increment records per-constituent production flow velocity, hotspot intensity, the uniform world-tone source sample, and candidate-to-target legacy pairwise aura magnitudes. The analyzer compares tick-zero flow/hotspot/source samples and explicitly fails causal eligibility when telemetry is missing or unmatched. A Rust preflight command now checks both tick-zero arms and blocks the manual paired-run workflow before simulations while time-resolved flow/hotspot exposure, non-candidate background resonance, and complete eligible-opportunity denominators are unavailable. Focused flow-sampler, analyzer-gate, and preflight-blocker tests were added. Full CI is running against the current implementation. See [PLAN-002 exposure-matching design](PLAN-002-exposure-matching-design.md) for limitations. The PR remains draft and unmerged.

**Current experimental blocker:** no facilitator candidate has been scientifically selected. The checked-in example configuration is illustrative only. Current exposure measurements are tick-zero only; background resonance from other materials and full-cycle flow/hotspot exposure are not measured; complete eligible target-pair opportunity denominators are absent. The analyzer therefore cannot mark causal interpretation eligible. Do not dispatch the current workflow as a catalytic experiment.

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

## Research sequence

1. Complete PLAN-001 from EXP-2026-004 artifacts; this is complete for the current dataset, so do not repeat the generic baseline.
2. Select the strongest instance-level pathways and distinguish reusable substrates from possible facilitators.
3. With PR #392's infrastructure validation green, implement and validate the PLAN-002 exposure telemetry, matched-placement gate, and complete opportunity denominators; then select a candidate from evidence before any catalytic run.
4. Test autocatalysis only if a plausible self-production mechanism emerges.
5. Evaluate pseudo-metabolism after repeated pathway turnover and regeneration are demonstrated.
6. Investigate cross-catalysis after individual facilitation is operationally established.
7. Pursue spatial organization once spatially resolved opportunity and rate comparisons are available.

All future experiments must be registered before execution and receive a dedicated result record at closeout. Plans do not authorize simulator behavior changes; implementation requires a separate explicit design decision and approval.
