# Resonance Field — Phase 0 Audit and PR Delivery Plan

**Audit status:** Code and architecture audit completed against current `main`. The standard-workload performance baseline remains a required closeout item; no new benchmark or simulation run was executed as part of this repository inspection.

**Controlling architecture:** [Resonance Field — Formal Implementation Roadmap](../design/resonance-field-implementation-roadmap.md)

**Approved target:** A persistent, continuous-in-space, multi-frequency wave field with finite propagation speed, phase, interference, explicit field energy, and matter coupling through realized exposed geometry. Correct physics and energy conservation take priority; performance must ultimately reach at least break-even with the existing implementation.

## 1. Audit scope and repository starting point

Reviewed the controlling roadmap and architecture documents, the geometry and energy-integration plans, the merged PR descriptions for #377 and #378, and the current implementations in:

- `src/perception/harmonics.rs`
- `src/simulation/environment_movement.rs`
- `src/simulation/world_tone.rs`
- `src/simulation/world_tone_snapshot.rs`
- `src/simulation/hotspot.rs`
- `src/chemistry/energy_ledger.rs`
- `src/simulation/state.rs`
- `src/simulation/simulation.rs`
- `src/geometry/organism_geometry.rs`
- `src/geometry/source_exposure.rs`
- `src/materials/physical_material.rs`

### Merged foundations

- **PR #377 — merged:** Optimizes environmental resonance-movement neighbor discovery. It retains the existing pair-force evaluator and adds a conservative spatial broad phase plus an explicit 0.001-world-unit minimum per-pair movement-resolution threshold. That threshold is a movement-resolution decision only; it is not a physical propagation radius and must never become a wave-field cutoff.
- **PR #378 — merged:** Adds composite realized-material geometry, exact supported straight/curved boundary primitives, nearest-visible source-perimeter queries, receiver-visible boundary intervals, perimeter measures, and linear perimeter allocation helpers. It does not implement wave propagation, field energy, or source debit/heat settlement.

## 2. Current live resonance paths

There are four distinct mechanisms. They must be mapped separately before migration.

### 2.1 Environmental material-to-material movement

**Live path:** `Simulation::step` → ambient transport → `apply_environmental_resonance_movement` → `environmental_resonance_displacements` → atomic material-placement update.

Current behavior:

- Each realized environmental material becomes one emitter.
- The emitter position is the arithmetic mean of constituent placement coordinates (a centroid).
- Its spectrum is calculated from material response and the shared analytical oscillator phase.
- The current reference oscillator is 440 cycles per simulated day; phase is calculated from the simulation tick.
- Pair interaction uses same-frequency phasor cross-energy, material mass scaling, and continuous amplitude attenuation of (1/(1+d)). The pair strength therefore falls as (1/(1+d)^2).
- Equal-and-opposite radial pair contributions are accumulated into bounded positional displacements.
- PR #377 accelerates candidate discovery using a conservative spatial bound and pair-specific support checks. The 0.001-unit threshold is applied to the maximum displacement from an individual pair.
- The resulting position changes are committed through the physical field's whole-object movement transaction.

**What this is not:** It has no persistent spatial field state, finite travel time, propagation phase, wavefront, or environmental wave-energy inventory. The current distance law is a quasi-static interaction model, not a finite-speed wave solution. The movement update is not a field propagation step.

**Migration implication:** Preserve the current path as a reference and comparison implementation until a field-driven movement consumer is validated. Do not transfer its pair cutoff into field propagation. Do not assume its centroid source model or inverse-square strength is the correct wave law.

### 2.2 Material-origin local resonance for chemistry

**Live path:** `Simulation::step` builds one `LocalResonanceWorldSnapshot` per tick (unless the explicit A/B disable flag is set), then passes it into environmental BREAK and COMBINE discovery.

Current behavior:

- The snapshot collects realized world physical materials and prepares material emitters once per tick.
- Material emitters use centroid positions, material-response spectra, mass scaling, and the same simulation-time oscillator convention.
- Reaction endpoint positions are queried against those prepared emitters.
- The resulting frequency/phase match is a dimensionless activation influence. It is not stored energy, a source debit, or spendable reaction energy.
- Environmental BREAK/COMBINE funding remains a separate path using the existing environmental resonance reservoir and chemistry-usable energy, with the established transaction/ledger settlement.
- The local resonance heat-map helper is diagnostic sampling. Its raster is not the authoritative reaction field and must not determine reaction eligibility.

**Migration implication:** Field sampling should eventually replace the instantaneous endpoint calculation, but the activation influence must remain distinct from actual energy available to fund a reaction. Reaction funding must continue through named reservoirs and the existing energy authority.

### 2.3 External world-tone input

**Live path:** `Simulation::step` samples the configured `WorldToneCycle` (or direct runtime configuration fallback) and calls `WorldToneRecipientSnapshot::settle_uniform_material_inputs`.

Current source parameters include:

- A rotating external source outside the rectangular environment.
- 440 cycles per simulated day.
- A separate 2,400-physics-tick orbital cycle.
- Configured intensity of 1–10 energy units per tick per normalized projected width.

Important implementation distinction:

- The production tick path currently uses the uniform material-recipient snapshot: one normalized unit of source width per atomic material constituent.
- The geometry-based `WorldToneOccluderSnapshot` and its ray/exposure frame exist, but its `settle_living_organism_inputs` path is not the production call used by the current tick. It must not be mistaken for the active source-settlement implementation.
- Material response and damping divide incident input into retained resonant energy, heat, and energy that escapes.
- The current uniform path does not maintain an in-world propagating wave. Its unabsorbed portion escapes rather than continuing through neighboring space and interacting with later receivers.
- Retained resonance is staged in organism reservoirs or the environmental reservoir and committed with ledger accounting.

**Migration implication:** The external world tone is a genuine named energy source and must become a field source with a finite emission budget. Its input must be debited once; energy remaining in the field must be represented; absorption, heat, reflection/transmission, and escape must have explicit destinations. The old uniform-recipient allocation must not survive as a second input authority after migration.

### 2.4 Localized hotspot input

The environmental hotspot is another independent external energy path. It is settled through `hotspot::settle_environment_inputs` and contributes to environmental resonance/energy accounting.

**Migration implication:** The hotspot must eventually inject into the same field source interface as the world tone. It must not retain a parallel propagation or energy authority. Its source geometry and emission semantics need to be documented in the physics specification.

## 3. Existing energy ownership and reusable infrastructure

### Existing stores and ledger

- `ResonantEnergyReservoir` stores nonnegative retained energy by frequency. It does not store travelling-wave phase or spatial field state and has a bounded spectral-component count.
- Organisms own their retained resonance reservoirs; the environment owns a separate reservoir for eligible environmental material.
- The `EnergyLedger` tracks external input, usable energy, retained resonance, structural energy, pair-interaction potential, work, heat, and escaped world-tone energy. Unabsorbed world-tone energy is also recorded as throughput telemetry; it is not a stored field-energy inventory.
- `EnergyInventorySnapshot` independently derives current holder totals and checks world-scale conservation. It is an important test oracle, not a live spatial field.
- Source and reaction settlement already use staged/atomic transaction patterns. These are the correct accounting patterns to preserve.

### Staged pair-potential work is not yet the live field

The repository contains useful groundwork for a signed, frequency-resolved pair-interaction potential:

- Pair-potential and gradient/force helpers exist in `harmonics.rs` and `world_tone_snapshot.rs`.
- `EnergyLedger` contains a signed pair-potential inventory term.
- `settle_pair_potential_change` provides staged settlement logic.

However, the pair-potential/gradient consumer functions and settlement helper are marked as staged/dead-code paths rather than being the active environmental movement path. The active PR #377 movement path remains the centroid-based pair-force/displacement calculation. Do not mistake the presence of ledger fields or isolated tests for a fully integrated physical energy transaction.

### Reusable components

- Simulation-time oscillator phase calculation and frequency-resolved spectrum operations.
- Material response and damping calculations, subject to the new field physics contract.
- Frequency-compatible reservoir deposit/withdraw operations.
- Existing energy-ledger transaction validation and independent inventory reconciliation.
- Atomic material movement and chemistry transaction patterns.
- Deterministic spatial broad-phase discovery from PR #377, where appropriate for field-query acceleration.
- Realized composite geometry, visibility, exact straight/arc primitives, perimeter measures, and allocation helpers from PR #378.

## 4. Geometry audit: what PR #378 does and does not finish

The merged geometry foundation is useful but is not complete perimeter-to-perimeter field coupling.

Confirmed remaining limitations:

1. Source-global union geometry still fails closed for shared/coincident edges, touching polygon boundaries, and some fluid/line arrangements.
2. Directional reception chooses one nearest visible point on the source perimeter for a source/receiver direction. It does not integrate the emitted contribution over the complete extended source perimeter.
3. Receiver geometry is partitioned into visible straight and curved intervals, but current reception evaluates representative interval samples (midpoint samples for straight intervals and arc midpoint samples for curved intervals) rather than integrating the local field over the complete receiver boundary.
4. Source-dependent visibility transitions and occlusion changes across the full source/receiver boundary pairing are not yet a complete continuous perimeter-to-perimeter integral.
5. Perimeter measure and linear allocation primitives are standalone geometry helpers. They are not connected to source-energy debit, propagation loss, or heat settlement.
6. Unsupported/ambiguous arrangements correctly fail closed. Preserve that behavior until exact support and tests exist.

**Do not rebuild these primitives inside the field.** Complete and consume the shared geometry interface. Do not substitute centroids, bounding circles, tessellated arcs, or arbitrary finite cutoffs.

## 5. Critical physics and integration gaps

The following are not implementation details that can be guessed during coding. They must be resolved in the physics specification before production integration.

1. **Material-source energy semantics:** Current material auras are calculated from material properties without a named source-energy debit. Decide whether these represent powered material oscillators, conservative coupled degrees of freedom, or another explicitly defined physical source. A wave solver must not turn a diagnostic amplitude into free energy.
2. **Field state and governing equation:** Define the field variable(s), units, dimensionality, wave equation, energy density, and energy flux. Phase and amplitude alone are not an energy inventory.
3. **Propagation speed and time units:** Define world distance per physics tick and finite travel time. Keep this separate from wall-clock runtime and ticks-per-second pacing.
4. **Frequency convention:** Preserve the established 24,000 physics ticks per simulated day, 440 reference-tone cycles per simulated day, and separate 2,400-tick source orbit unless the Master Spec explicitly changes them. Define how multiple independent sources acquire and retain phase.
5. **Boundary topology discrepancy:** The current `ActiveMaterialField::row_col_for_position` and `cells_within_radius` implementations visibly reject out-of-range x coordinates and wrap y coordinates. The approved project/world description has been recorded as horizontal wrap and vertical hard boundaries. Confirm the coordinate convention against Master Spec 2.4 and reconcile this discrepancy before choosing wave boundary conditions. Do not infer a wave reflection/absorption rule merely from a physical wall.
6. **Attenuation:** Distinguish geometric spreading from dissipative loss. The current (1/(1+d)) amplitude law and inverse-square pair strength are legacy interaction behavior, not automatically the wave equation's attenuation law.
7. **Material coupling:** Define frequency-dependent absorption, reflection, transmission, scattering, and orientation/geometry response. Decide how a finite field energy budget is shared among multiple receivers without duplicate claims.
8. **Exact surface coupling:** Define source injection and receiver absorption over complete exposed physical perimeters, including occlusion and supported curved boundaries.
9. **Numerics and determinism:** Define spatial/time resolution, stability constraints, interpolation, convergence, update ordering, accumulation precision, and deterministic replay.

## 6. Performance baseline and measurement status

The approved canonical comparison workload remains:

- 60 seconds of simulation time using real simulation-clock semantics, without an artificial tick cap.
- Seed 42.
- 1,280 starting materials: 1,200 independent resources and 80 molecules.
- Matched build mode, instrumentation, environment, and initial state.

PR #377 established a matched spatial-discovery-versus-exhaustive-discovery comparison for its own movement optimization. That is not a benchmark of the proposed wave solver.

**No new end-to-end baseline was run during this audit.** Before the solver is implemented, record the current-main runtime, simulated ticks/throughput, peak memory if available, movement phase cost, resonance snapshot cost, world-tone/hotspot settlement cost, and energy residual. Use the same command and instrumentation later for the field comparison. The first correct wave solver is allowed to be slower; the final optimized implementation must reach at least break-even without physics shortcuts.

## 7. Recommended PR delivery plan

The sequence below deliberately keeps each PR reviewable and avoids stacking multiple architectural changes together. The roadmap remains the authority; each PR should link both the roadmap and this audit.

### PR 0 — Phase 0 audit record and reproducible baseline

**Scope**
- Commit this audit record and correct any stale roadmap statements.
- Record the canonical current-main 60-second seed-42 baseline and its exact command/artifacts.
- Confirm which existing diagnostic phases can be reused and which small instrumentation gaps remain.

**Not included:** Any physics or production behavior change.

**Acceptance**
- Audit and baseline are reproducible from the repository.
- No benchmark result is inferred from PR #377's neighbor-discovery comparison.
- Phase 0 is marked complete only after the baseline is recorded.

### PR 1 — Wave physics and source/energy contract

**Scope**
- Resolve the field variable, governing wave equation, units, dimensionality, propagation speed, phase convention, and numerical stability requirements.
- Define world boundary topology and separate wave boundary conditions.
- Define source classes and energy provenance for world tone, hotspot, and material-origin oscillators.
- Define energy density/flux and the accounting destinations for field storage, receiver absorption, heat, reflection/transmission, and escape.
- Define material coupling and complete-perimeter source/receiver contracts.

**Not included:** Production solver or changes to live chemistry/movement.

**Acceptance**
- A dimensionally consistent physics decision record exists.
- No unresolved energy source or boundary-condition ambiguity remains.
- Decisions are reviewed against Master Spec 2.4 and the current ledger semantics.

### PR 2 — Independent reference wave solver

**Scope**
- Add a persistent field state and deterministic reference update loop using the simplest stable numerical representation chosen in PR 1.
- Implement one-frequency finite-speed propagation in an empty environment.
- Keep the solver independent of molecules, geometry discovery, chemistry, and production energy holders.

**Acceptance**
- Finite travel time is measured and verified.
- A simple-domain reference/analytic solution is matched within tolerance.
- Stability, convergence, deterministic replay, and empty-space energy accounting pass.
- No diffusion-only or instantaneous redistribution substitute is used.

### PR 3 — Multi-frequency waves, phase, interference, and world boundaries

**Scope**
- Extend the reference solver to multiple simultaneous frequencies and phase-resolved superposition.
- Implement constructive/destructive interference and standing-wave behavior.
- Implement the approved horizontal/vertical wave boundary conditions.
- Add deterministic source scheduling and propagation-delay tests.

**Acceptance**
- Frequencies remain distinguishable.
- Interference and standing-wave tests pass.
- Boundary behavior and phase continuity are verified.
- Refinement tests demonstrate expected convergence.

### PR 4 — Field energy lifecycle and external source injection

**Scope**
- Add explicit field-energy accounting and atomic source injection/withdrawal.
- Migrate the external world-tone source and localized hotspot to a common field source interface.
- Replace uniform per-constituent instant settlement with field propagation and spatial reception only when the field path can account for energy exactly once.
- Preserve named source provenance and existing ledger authority.

**Acceptance**
- Source input is debited exactly once.
- Field energy is independently represented and reconciles.
- Absorbed, retained, heat, reflected/transmitted, and escaped energy balance.
- Multiple receivers cannot claim the same finite energy.
- Failed settlement rolls back all affected state.

### PR 5 — Complete exact composite source-boundary geometry

**Scope**
- Complete exact union handling for shared/coincident edges and touching supported shapes.
- Add supported fluid/line boundary arrangements where the physical representation is unambiguous.
- Preserve fail-closed behavior for unsupported or degenerate geometry.
- Add deterministic perimeter-measure and union tests.

**Acceptance**
- No duplicate or missing boundary segments in supported arrangements.
- Exact perimeter measures match analytic/reference cases.
- Rotated, touching, shared-edge, curved, and fluid cases are covered.

### PR 6 — Complete perimeter visibility and continuous field coupling

**Scope**
- Resolve source-dependent visibility transitions across receiver intervals.
- Integrate field reception over complete exposed receiver perimeters.
- Integrate source emission over the complete exposed source perimeter using the specified source boundary condition.
- Consume PR #378 geometry APIs; do not duplicate geometry solvers.

**Acceptance**
- No centroid or nearest-single-source-point approximation remains in authoritative field coupling.
- Occlusion, orientation, shape, and partial visibility are handled across complete boundaries.
- Boundary integration converges under refinement.
- Energy allocation uses the full source perimeter denominator and cannot be renormalized independently per receiver.

### PR 7 — Material-origin oscillator integration

**Scope**
- Define and implement material oscillator/source coupling using the PR 1 source-energy semantics.
- Replace the instantaneous material-centroid aura query with field-backed spatial samples.
- Preserve frequency-resolved material response without creating unowned energy.
- Add source/receiver material property contracts and deterministic geometry revision invalidation.

**Acceptance**
- Material-origin fields have explicit physical energy provenance or a formally defined conservative coupling.
- Field queries agree with the reference solver.
- No material-specific propagation special cases are introduced.

### PR 8 — Migrate environmental movement to field-derived interaction

**Scope**
- Replace the active centroid-pair movement authority with the approved field-derived interaction/gradient.
- Define mechanical work and energy settlement before enabling movement from field forces.
- Keep the existing PR #377 path as an A/B reference until matched validation passes.
- Apply any movement-resolution tolerance only at the movement-output layer, never as a field propagation cutoff.

**Acceptance**
- Equal-and-opposite interactions and force/energy consistency are tested.
- Movement transactions are atomic.
- No unexplained work, energy creation, or loss.
- Matched behavior differences are explained by finite-speed propagation and interference.

### PR 9 — Migrate chemistry activation and reaction energy consumers

**Scope**
- Replace `LocalResonanceWorldSnapshot` instantaneous endpoint spectra with field-backed local queries.
- Keep dimensionless activation influence separate from spendable energy.
- Route actual BREAK/COMBINE/displacement funding through named existing reservoirs and the authoritative ledger.
- Preserve reaction atomicity, chemistry rules, and source locality.

**Acceptance**
- Field sampling does not itself grant energy.
- Funding is frequency-compatible, local where required, and debited exactly once.
- BREAK, COMBINE, and displacement conservation/rollback tests pass.
- Existing chemistry behavior changes are attributed to the approved wave physics, not incidental integration changes.

### PR 10 — Migrate biological reception and retire duplicate legacy paths

**Scope**
- Migrate organism/world-tone reception and other remaining consumers to the field interface.
- Ensure only one production source, propagation, receiver, and energy authority exists.
- Remove old instant propagation/reception paths only after all consumers have migrated and passed their gates.
- Update diagnostics, documentation, and GUI data sources to read the authoritative field state.

**Acceptance**
- One field authority and one energy ledger authority.
- Long-duration energy/stability validation passes.
- No stale duplicate source or receiver paths remain.

### PR 11 — Profile and optimize to performance break-even

**Scope**
- Profile the correct integrated field by propagation, source injection, boundary integration, receiver coupling, ledger settlement, and synchronization.
- Optimize measured hot paths using deterministic spatial acceleration, caching, revision invalidation, and adaptive resolution only where validated against the reference solver.
- Keep the conservative reference solver as a test oracle.

**Acceptance**
- Optimized results match the reference solver within documented tolerances.
- Energy conservation and determinism remain intact.
- The canonical 60-second workload reaches at least break-even with the current-main baseline, or the remaining gap is explicitly reported for a user decision.
- No arbitrary propagation cutoff, dropped low-amplitude wave, phase removal, or frequency collapse is used to manufacture a performance win.

## 8. Recommended execution order and PR discipline

- Finish PR 0 before PR 1 begins; do not mix baseline instrumentation with physics changes.
- PR 1 must settle the physical decisions before PR 2/3 solver code is treated as production architecture.
- PR 2 and PR 3 remain isolated reference-solver work. Do not integrate molecules or energy holders prematurely.
- PR 4 establishes the field's own energy lifecycle before matter can absorb or reflect its energy.
- PR 5 and PR 6 complete the geometry contract needed for exact matter coupling.
- PR 7 establishes material-source semantics before movement or chemistry consumes the field.
- PR 8, PR 9, and PR 10 are separate consumer migrations. Do not combine movement, chemistry, and biological reception into one broad PR.
- PR 11 is optimization after correct integration, not a reason to weaken the physics.
- Each PR must be independently reviewable, have focused tests, preserve full CI coverage, and avoid overlapping branches. No stacked PR chain unless a true code dependency makes it necessary.

## 9. Immediate next action

**Complete PR 0's remaining baseline task:** run and record the canonical 60-second, seed-42, 1,280-material current-main benchmark with phase timing and energy diagnostics. Then begin PR 1's physics/source/energy decision record, including the coordinate-topology discrepancy and material oscillator energy semantics.

No production resonance-field code should be rewritten before those two items are resolved.
