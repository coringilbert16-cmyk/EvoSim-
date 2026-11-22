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

## PLAN-007 — Environmental reaction-discovery eligibility

**Status:** Registered; EXP-2026-007 prepared, not yet run.

**Question:** At which stage do ordinary environmental BREAK and DISPLACEMENT discovery candidates stop progressing toward chemically eligible, fundable, committed reactions, and what evidence explains the absence of eligible candidates or commits?

**Purpose:** Localize the first observed bottleneck in the existing discovery funnel. This is a descriptive diagnostic experiment, not an intervention or a test of a modified chemistry rule.

**Registered experiment:** [EXP-2026-007](../registry.md#registered-experiments), condition `ordinary-discovery-baseline`.

**Design and fixed conditions:**
- Five independent seeds: 42, 43, 44, 45, and 46.
- Real simulation duration: 60 seconds per seed.
- Diagnostic interval: 100 ticks.
- Standard inventory: 1,200 independent single-constituent resources plus 80 intact two-constituent molecules (1,360 constituents total), using the existing standard six-element catalog and distribution.
- Ordinary chemistry, activation, scheduling, geometry, movement, energy, and reaction rules unchanged.
- Movement profiling: disabled. Tick profiling: disabled. Deep diagnostics: disabled.
- No forced reactions, artificial energy grants, altered thresholds, new catalyst, special-case behavior, or extended observation window.

**Measurements and interpretation:**

1. **BREAK:** use the authoritative maintained bond-index candidate population and inspected count. Follow candidates through physical restoration, geometry/endpoints, mapping/properties, chemistry eligibility, activation, scheduling, rejection, and commit. A BREAK pass is complete only when the authoritative search counts reconcile, including an explicitly measured empty domain.
2. **DISPLACEMENT:** follow anchor and target-cell/material inspection, bounded-window skips, work allowance, replacement-contact searches, activation state, trial rejection stage, energy/funding, and committed replacement transactions. Preserve `complete`, `partial`, `unknown`, and measured-zero distinctions. The bounded rotating search must not be represented as an exhaustive denominator.
3. **COMBINE:** retain its own coverage-aware measurements as environmental context only. COMBINE broad-phase pairs are not a denominator for BREAK or DISPLACEMENT.
4. **Energy and transaction joins:** where a candidate reaches a trial or commit, use stable reaction identity and linked settlement data to distinguish eligibility, funding, transaction rejection, and committed material changes. Run-level energy totals are not candidate-level causal evidence.

**Analysis rules:**
- Analyze each seed independently; the seed is the replicate. Events and repeated per-tick scans within a seed are not independent samples.
- Per-tick search counts are repeated search observations, not unique run-wide encounters.
- Do not calculate conversion rates unless numerator and denominator are both present, complete, and semantically matched.
- Missing counters remain unknown, not zero. Known bounded coverage remains partial; do not infer omitted target totals from a work counter alone.
- Report the first funnel stage where observed candidates stop progressing, separately for BREAK and DISPLACEMENT. If coverage prevents localization, state that limitation rather than inferring absence.
- Separate direct observations, derived summaries, interpretation, and follow-up hypotheses.

**Success criteria:**
- All five seeds have validated provenance and compatible measurement definitions.
- BREAK coverage and candidate-stage counts reconcile.
- DISPLACEMENT coverage is honestly classified and all available stage/rejection counters are retained.
- Measured zero, missing, and partial coverage remain distinct.
- The report identifies a supported bottleneck or explicitly identifies the missing evidence needed to do so.
- No chemistry/physics behavior is changed as part of this experiment.

**Workflow request:** dispatch `.github/workflows/chemistry-experiments.yml` from `main` with `seeds=42,43,44,45,46`, `duration_seconds=60`, `diagnostic_interval=100`, `profile_movement=false`, `profile_tick=false`, `deep_diagnostics=false`, `experiment_id=EXP-2026-007`, `research_plan=PLAN-007`, and `condition_id=ordinary-discovery-baseline`.

**Permanent record and cleanup:** after successful analysis, validate and publish exactly `setup.json` and `consolidated-report.json` under the experiment/condition result directory and update the shared registry. Delete raw observations, manifests, intermediate analyzer outputs, caches, temporary indexes, and diagnostics only after both permanent files are successfully published. Failed validation/publication must retain temporary inputs for recovery.

**Decision after the run:** do not change simulator behavior immediately. Use the first supported bottleneck to decide whether the next step is a narrowly scoped measurement correction, a source-level audit of an existing eligibility rule, or a separately designed physics/chemistry experiment requiring explicit approval.

## Research sequence

1. Complete PLAN-001 from EXP-2026-004 artifacts; do not repeat the generic baseline.
2. Select the strongest instance-level pathways and distinguish reusable substrates from possible facilitators.
3. Build the minimum controlled-comparison capability needed for PLAN-002.
4. Test autocatalysis only if a plausible self-production mechanism emerges.
5. Evaluate pseudo-metabolism after repeated pathway turnover and regeneration are demonstrated.
6. Investigate cross-catalysis after individual facilitation is operationally established.
7. Pursue spatial organization once spatially resolved opportunity and rate comparisons are available.

All future experiments must be registered before execution and receive a dedicated result record at closeout. Plans do not authorize simulator behavior changes; implementation requires a separate explicit design decision and approval.


## PLAN-008 — Pre-wave unified-energy performance baseline

**Status:** Registered; EXP-2026-008 prepared, not yet run.

**Question:** What is the reproducible performance and observable energy-accounting baseline of the current `main` implementation immediately before the unified energy / continuous spatial wave-field redesign?

**Purpose:** Establish the fixed Phase 0 comparison point for later physics implementations. This is a measurement-only experiment; it does not validate the physical completeness of the current resonance model and changes no simulator behavior.

**Registered experiment:** [EXP-2026-008](../registry.md#registered-experiments), condition `pre-wave-reference`.

**Design:** Five independent seeds (42–46), 60 seconds of real simulation time per seed, diagnostic interval 100 ticks, and the standard inventory of 1,200 independent single-constituent resources plus 80 intact two-constituent molecules (1,360 constituents total). Run from `main` and capture the exact source commit. Ordinary chemistry and all environment/energy behavior remain unchanged. Movement profiling and sampled tick profiling are enabled; deep diagnostics are disabled.

**Measurements:** Per-seed wall-clock runtime, simulated ticks and throughput, movement/resonance/placement phase timings, sampled tick-phase timings, energy/source/settlement and ledger results, material balance, and provenance. Unavailable measurements must be marked unavailable, not inferred. Profiling overhead must be kept distinct from phase costs; future comparisons must use matching instrumentation.

**Success criteria:** All five seeds complete with validated provenance and compatible measurements; per-seed and descriptive cross-seed results are consolidated; the canonical two-file permanent record is validated and published; temporary observations and intermediate analysis are deleted only after successful publication.

**Workflow request:** `seeds=42,43,44,45,46`, `duration_seconds=60`, `diagnostic_interval=100`, `profile_movement=true`, `profile_tick=true`, `deep_diagnostics=false`, `experiment_id=EXP-2026-008`, `research_plan=PLAN-008`, `condition_id=pre-wave-reference`.

**Decision:** Phase 0 remains incomplete until the consolidated report is validated and published. The report becomes the fixed pre-wave performance/accounting reference; it is not evidence that the legacy energy model is physically correct.
