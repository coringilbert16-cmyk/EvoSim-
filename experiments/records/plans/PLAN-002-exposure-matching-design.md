# PLAN-002 — Exposure-matched comparison design

**Status:** Design gate; no catalytic experiment authorized or dispatched.

## Why the current placement screen is not causal

PR #392's candidate-near and candidate-remote fixtures preserve the elemental inventory, initial energy inventory, target identities, and all non-candidate starting geometries. They move one candidate molecule to change its distance from a selected target pair.

That is a useful deterministic placement screen, but it does not isolate proximity. The candidate is itself a flow obstacle; moving it can change the local flow around the target and candidate. It also changes candidate exposure to the spatial energy sources and to other molecules' resonance signals. A difference in target reaction counts could therefore arise from altered transport or energy exposure rather than a chemically available facilitator.

The present chemistry resonance implementation is the existing pairwise harmonic/aura model. It is not the approved persistent, finite-speed resonance field. Any measurements from it must be named and interpreted as legacy pairwise resonance exposure, not as a spatial wave-field measurement. See the [canonical resonance-field roadmap](../../../design/resonance-field-implementation-roadmap.md).

## Candidate selection gate

No facilitator candidate is selected by PLAN-001. The observed H–O–P ⇌ H + O–P and N–O–P ⇌ N + O–P cycles demonstrate reversible substrate/intermediate recycling. They do not identify an unchanged third molecule that facilitated either reaction.

The H–S candidate in `matched-catalysis-config.example.json` remains an illustrative software fixture only. It must not be treated as a scientific candidate.

A candidate may be nominated only with a traceable hypothesis that identifies:
- candidate composition and, where available, exact starting molecule identity;
- target reactant compositions and a specific COMBINE or DISPLACEMENT transformation;
- why the candidate is not itself a target reactant or a consumed co-reactant;
- what it means for the candidate to remain chemically available (unchanged constituent inventory and bond graph across the target event);
- evidence supporting the hypothesis, with seed, tick, event IDs, and artifact provenance.

A candidate hypothesis is not evidence of facilitation.

EXP-2026-006 adds one exact-instance follow-up trace: seed 45 H–S identity 1526 (created tick 3148; destroyed by BREAK at tick 4413) remained a distinct, non-consumed molecule identity while it was sampled near three same-pair/tick COMBINE commits at ticks 3639, 3944, and 4145. One commit formed a second H–S molecule from H+S, while the other two formed O–P and H–O. The candidate also appeared near three sampled opportunities without a same-pair/tick COMBINE commit. This is a specific, chemically available third-molecule hypothesis; it is not causal evidence. All events were in a right-boundary spatial cluster, so density, transport, and boundary effects remain plausible alternatives.

**Supplemental observational screen (EXP-2026-006):** The five-seed neighborhood data show descriptive associations for nearby H–S, H–O, S–S, O–P, and several other families among bounded COMBINE opportunity samples. These families can be prioritized for a controlled follow-up hypothesis, but the data do not select a catalyst: local density, flow, hotspot exposure, resonance, and sample selection remain confounded. H–S remains an illustrative fixture, not a validated candidate.

## Required exposure controls

The matched conditions must use the same seed, target molecule identities, target geometry/orientation, total initial material inventory, and initial energy inventory. All non-candidate starting placements remain identical. Candidate placement selection must be deterministic and fixed before observing outcomes.

At minimum, record and compare the following at tick zero and over the observation period:

1. **Target-pair exposure:** local flow velocity and shear at the target constituents; external-source/hotspot incident energy and visibility; and the background resonance signal received by the target with the candidate's own contribution reported separately.
2. **Candidate exposure:** local flow velocity and shear over its realized geometry; incident source/hotspot exposure; and incoming pairwise resonance spectrum from all other materials.
3. **Geometric side effects:** candidate-induced changes to target flow caused by obstacle/wake behavior, source shadowing/occlusion, target separation/orientation, and local molecule density.
4. **Energy availability:** usable and resonant energy available to the reaction transaction, structural and pair-potential terms, heat, and the complete ledger residual.

Use the simulator's actual physical geometry and the same production sampling/settlement paths wherever available. Do not substitute global reservoir totals, centroid-only approximations, or the bounded neighborhood sample count for local exposure measurements. If an exposure quantity cannot be measured reliably, mark the pair as unmatched; do not assume it is equal.

The target pair's non-candidate background exposure must match between arms within predeclared numerical tolerances. The candidate's own background exposure must also be matched as closely as feasible. The candidate-to-target interaction being tested is reported separately and must not be normalized away.

## Placement strategy and feasibility

Do not choose locations by looking at reaction outcomes. Generate a deterministic set of candidate-near and candidate-remote placements from the tick-zero geometry, then evaluate their exposure signatures before running chemistry.

A pair of placements is eligible only if:
- the candidate is within the configured near-distance in one arm and beyond the remote-distance in the other;
- target pair geometry and non-candidate geometry are unchanged;
- moving the candidate does not create a target-flow difference through its obstacle/wake footprint;
- source/hotspot exposure and occlusion at the target remain matched;
- the candidate's background flow and source exposure are acceptably matched;
- initial elemental, structural, usable, resonant, and pair-potential inventories reconcile.

The flow model has a finite obstacle/wake influence region. Prefer near placements outside the candidate's flow-influence footprint at the target while remaining near enough to test the existing distance-dependent resonance interaction. Verify this from the production flow calculation rather than relying on a hand-chosen radius. If no such placement exists, the simple one-candidate translation design is not suitable.

The world-tone source moves over time, so a tick-zero match is insufficient. Compare exposure over its full source cycle (or continuously during the run) and reject pairs whose cumulative or time-resolved exposure differs materially. Apply the same rule to hotspot exposure and changing obstacle geometry.

If a placement pair cannot satisfy these controls, stop and design a stronger matched control; do not relax tolerances after seeing outcomes. Any alternative control must preserve total inventory and energy without arbitrary energy grants or chemistry-specific behavior.

## Opportunity denominator and outcome analysis

The current neighborhood telemetry is bounded and deterministically sampled. It is useful for context but is not a complete, unbiased denominator. In EXP-2026-006, a strict source/constituent/endpoint transaction join returned no exact matches in seed 42 even though a weaker source-pair-plus-tick join found 197 same-pair/tick COMBINE commits. The sampled endpoint descriptors therefore do not reliably identify the interface used by the committed transaction in that artifact; treat the weaker join as descriptive only until this mapping is resolved. Before a catalytic rate comparison, add exact aggregate counters for all eligible target-pair opportunities and their physically valid/contact-ready subsets, with candidate-near status and truncation/missing-geometry coverage reported separately. Do not infer a reaction probability from commits divided by sampled opportunities.

The primary outcome is the target reaction rate per eligible target-pair opportunity, reported per independent seed with paired differences and uncertainty intervals. Also report wall-clock/tick-normalized event counts, rejection reasons, candidate survival/availability, exact parent/child identities and bond graphs, and full energy accounting. Seeds—not individual reaction events—are the independent replicates.

A positive result requires a reproducible increase across independent seeds while the candidate remains chemically available, with matched target/candidate background exposures and adequate opportunity coverage. A placement effect, a higher raw event count, a single-seed difference, or candidate consumption is not catalytic facilitation.

## Implementation and execution gates

1. Keep PR #392 as infrastructure; its existing placement workflow remains manual-only and must not be dispatched as a catalytic test.
2. Add exposure telemetry using authoritative production flow, source-energy, and resonance sampling paths. Clearly label legacy pairwise resonance measurements until the approved persistent field exists.
3. Add deterministic placement-pair selection and a fail-closed exposure-match report. No run should start if the pre-run match gate fails.
4. Add exact eligible-opportunity denominators and tests for matched/unmatched fixtures, source shadowing, flow wakes, and energy invariance.
5. Select a candidate only after reviewing PLAN-001 artifacts and writing the candidate/target hypothesis.
6. Register the experiment in the experiment registry with seeds, duration, tolerances, controls, and acceptance criteria before dispatch.
7. Dispatch only after the design and candidate are explicitly approved. Archive raw artifacts and a dedicated result document.

No reaction rules, thresholds, resonance physics, energy grants, or organism-specific logic may be changed to make a candidate pass.

## Implementation progress — first exposure telemetry increment

The current implementation branch now records tick-zero local exposure metadata for each matched fixture:
- Per-constituent flow velocity sampled through the same ambient S-flow and complete obstacle-adjusted velocity calculation used by production transport, excluding the sampled material's own obstacle.
- Per-constituent localized-hotspot intensity from the live hotspot source function.
- The tick-zero world-tone source sample. Its current production recipient semantics are spatially uniform per constituent, so moving a material does not change that source input.
- Candidate-to-target legacy pairwise aura boundary magnitudes, explicitly separated from the unmeasured background contribution.

The offline comparison analyzer compares paired candidate and target flow/hotspot samples and the world-tone source sample with a fixed predeclared absolute tolerance of 1e-9. It reports any mismatch and marks causal interpretation ineligible. A Rust `--matched-catalysis-preflight` command now builds both tick-zero arms, checks those exposure signatures, and then fails closed while temporal flow/hotspot coverage, non-candidate background resonance, or complete eligible-opportunity denominators remain unavailable. The manual workflow runs this preflight in its `prepare` job before the observation matrix, so the paired chemistry runs cannot start while the design is incomplete. Tests cover missing telemetry, candidate-induced flow mismatch, and the Rust preflight blocker.

**This increment is deliberately not a pass gate for running chemistry.** Flow and hotspot measurements are tick-zero only; source-cycle exposure is not integrated over time; background resonance from other materials is not yet measured; and complete eligible target-pair opportunity denominators are still absent. The analyzer therefore cannot mark causal interpretation eligible. The existing paired-run workflow remains unsuitable as a catalytic test and must not be dispatched.

The legacy pairwise aura is measured only as a candidate-specific diagnostic. It is not a persistent field, does not propagate at finite speed, and does not represent field energy.
