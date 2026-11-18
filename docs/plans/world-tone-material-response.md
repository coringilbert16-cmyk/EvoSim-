# World-tone material-response settlement redesign

## Objective

Replace geometry-dependent world-tone propagation with direct, uniform exposure of every eligible physical material instance. Calculate harmonic response once per distinct material-property signature and reuse it for identical instances. This is both a physics correction and a system-wide performance change.

## Domain rules

- World tone is a uniform environmental source: position, orientation, intervening material, and geometric occlusion do not change whether or how a material receives it.
- Every atomic constituent receives the same source input for a given tick. A material part with an integer amount of N contributes N independent atomic recipients.
- Total source input scales with the number of eligible atomic constituents. The ledger must record that full input explicitly; no energy may be silently created, duplicated, or lost.
- Identical material-property signatures have identical responses and share one response calculation.
- Distinct molecular/material structures remain distinct when their harmonic properties differ.
- Keep harmonic response equations, resonance storage semantics, heat accounting, and ownership rules unless tests expose a necessary correction.
- This redesign is scoped to world-tone settlement. Do not remove or alter geometry/exposure behavior used by unrelated systems.

## Implementation outline

1. **Trace and isolate settlement dependencies**
   - Map all callers of `WorldToneCaptureFrame`, `WorldToneOccluderSnapshot`, and source-exposure paths.
   - Identify all living, environmental, stored, decomposing, and in-transition physical material recipients.
   - Preserve the current eligible-recipient coverage and reservoir ownership semantics.

2. **Replace geometry-based exposure**
   - Remove ray/path construction, occlusion, projected-width discovery, and downstream signal propagation from world-tone settlement.
   - Build the eligible atomic-recipient list directly from authoritative physical material state, expanding each validated integer constituent count.
   - Ensure position, orientation, and intervening bodies cannot affect world-tone receipt.

3. **Introduce shared material-response evaluation**
   - Define a stable response key from every actual property used by harmonic frequency, damping, and absorption calculations, plus source inputs that affect the response.
   - Calculate each unique response once per applicable tick/source sample.
   - Reuse it for all matching instances; do not key only by element name when molecular structure changes effective properties.
   - Avoid a persistent cache until dependency/invalidation correctness is demonstrated; a per-settlement response table is the initial design.

4. **Simplify source and energy settlement**
   - Define per-instance incident input using the existing source envelope/frequency and the agreed uniform-exposure semantics.
   - Stage all per-instance resonance/heat/escape changes.
   - Avoid repeated spectrum construction, catalog-baseline reconstruction, and per-candidate reservoir cloning where safe.
   - Commit reservoirs and ledger atomically; reject invalid/overflowing settlement without partial mutation.
   - Reconcile total external input against retained resonance, heat, and escaped energy.

5. **Regression coverage**
   - Identical property signatures reuse the same response.
   - Different property signatures produce their correct distinct responses.
   - Translation, rotation, and intervening material do not affect receipt or response.
   - Every eligible atomic constituent is settled exactly once, including environmental and nonliving material; aggregate counts expand to the correct number of recipients.
   - Stored, decomposing, attached/developing, and active-transformation material retains correct ownership/eligibility.
   - Invalid settlement does not partially commit.
   - Determinism, chemistry inventory, and full energy inventory remain valid.
   - Existing non-world-tone geometry/exposure tests remain unchanged and pass.

6. **Performance validation**
   - Use the established real-clock 60-second seed-42 experiment, 1,200 independent resources plus 80 molecules, with no tick cap.
   - Compare against the recorded baseline: 171 ticks in 60.136 seconds.
   - Report total ticks, ticks/minute, settlement time/tick, resonance movement time/tick, unique response count versus recipient count, and allocation/copy changes where available.
   - Separate local settlement savings from measured whole-simulation throughput changes.

## Merge conditions

- [ ] All recipient categories are accounted for; no eligible physical material is silently omitted.
- [ ] No world-tone occlusion, shadowing, projected-width, or path-propagation dependency remains in the settlement path.
- [ ] Identical material-property signatures receive identical responses and share calculation work.
- [ ] Distinct property signatures remain physically distinguishable.
- [ ] Energy input scales consistently with eligible instance count and is fully represented in the ledger.
- [ ] Resonance ownership, heat, escape, and atomic commit/rollback semantics are preserved.
- [ ] Focused regression tests pass, including determinism and energy-inventory reconciliation.
- [ ] Unrelated geometry/exposure behavior is unchanged.
- [ ] Formatting, architecture checks, offline analyzers, Rust tests, environmental smoke, chemistry analysis, and Clippy all pass.
- [ ] Standard 60-second seed-42 benchmark completes; results and comparison with the 171-tick baseline are recorded.
- [ ] No chemistry equations, reaction eligibility, geometry/contact rules, or unrelated simulation behavior are changed.

This PR should remain cohesive: the uniform-source model, shared response evaluation, and atomic energy settlement are interdependent parts of the same redesign.


## Initial source audit (Nov 18, 2026)

The active tick call is `Simulation::step` -> `WorldToneOccluderSnapshot::settle_living_organism_inputs` in `src/simulation/simulation.rs`. The older lower-level receiver API in `world_tone.rs` is not the active production settlement path.

The current settlement path confirms the main cost and model mismatch:

- `WorldToneOccluderSnapshot::from_simulation_for_organisms` assembles organism, environment, decomposing, stored, reproductive-construction, and active-transformation material into geometry bodies.
- It calls `OrganismBodyGeometry` construction and `WorldToneCaptureFrame::from_geometry_for_receivers_at_tick`, which invokes directional source exposure and interaction-path discovery.
- Settlement validates first-hit projected widths against geometry exposure, then loops each ray path and each material hit.
- Each hit constructs/uses a signal response, calls `material_response_spectrum`, evaluates damping, and transforms a downstream signal.
- The settlement clones every organism reservoir once for staging, then clones a candidate reservoir again for each retaining material hit before depositing.
- `ResourceBaselines::from_catalog` is rebuilt during each settlement.
- The live ledger settles aggregate incident, retained, heat, and escaped energy through `settle_world_tone_paths`.

Important existing ownership semantics to preserve:

- Living organism structures credit that organism's resonance reservoir.
- Environmental field material may retain resonance in the environment reservoir.
- Stored material, legacy decomposing structures, committed reproductive-construction material, and active-transformation source material are physical recipients but do not currently own a resonance reservoir; their absorbed response becomes heat.
- Attached/developing structure is part of the parent's authoritative aggregate and credits the parent's reservoir.
- Invalid/ambiguous active-transformation ownership or malformed material currently fails the whole snapshot closed.
- The active settlement stages all changes and commits the ledger before replacing the staged reservoirs.

Existing tests include several that explicitly assert shadow geometry and ordered travelling-signal behavior. Those assertions must be replaced with uniform-exposure tests; they are not valid requirements under the approved model. Keep the ownership, nonliving heat, attached/developing, active-transformation, rejected-transaction, and inventory-reconciliation coverage.

Implementation should introduce a material-only recipient snapshot rather than calling the existing geometry snapshot and merely ignoring its paths. This prevents the expensive geometry construction from remaining on the live tick path. The old geometry utilities may remain for other systems or legacy isolated tests, but the production world-tone settlement path must no longer call them.


## Implementation progress

The production tick path now uses `WorldToneRecipientSnapshot`, which enumerates material constituents directly from authoritative simulation state without constructing geometry bodies or exposure paths.

- Each eligible material constituent receives one normalized source-width unit per tick; source frequency/envelope remain unchanged and total incident energy scales with recipient count.
- Exact bitwise keys cover all `ResourceProperties` fields used by harmonic response. A per-settlement table computes the material response once per unique key and reuses the already-computed spectrum for retained-energy normalization.
- Organism and environment reservoirs are staged once per holder. A complete spectrum-capacity/overflow preflight avoids cloning a reservoir for every constituent while preserving per-recipient all-or-heat behavior.
- Living organism, attached/developing, environmental field, stored, decomposing, committed construction, and active-transformation recipient categories retain their prior ownership/heat treatment.
- Production tests now exercise uniform recipient enumeration and settlement rather than requiring ordered travelling-signal paths.

Full Rust tests, environmental smoke, Clippy, and the standard 60-second seed-42 benchmark remain pending. The legacy geometry snapshot is retained only for existing diagnostic/geometry tests and is no longer called by the production tick.
