# Resonance Field — Formal Implementation Roadmap

**Status:** Approved architectural direction; implementation roadmap. The continuous, time-propagating wave field is the target architecture. This document is the required reference for every future resonance-field investigation, issue, branch, pull request, implementation, and validation run.

**November 24 user-approved ambient-mode amendment (current authority):** The initial ambient contribution is an intrinsic, spatially uniform resonance component over the finite simulated-world accounting area. Its cycle-averaged stored energy density follows the 1–10 cosine daily envelope over 24,000 ticks; its carrier completes 440 cycles per day. The world generates positive increases in the mode's stored energy and withdraws energy on decreases. No literal Sun, external oscillator, or separate source reservoir is represented. The finite area is an accounting footprint, not a physical wave boundary. Local disturbances retain the nondispersive scalar wave equation. Do not apply a global Klein–Gordon term to the entire field. The isolated mode-state implementation and its explicit limitations are recorded in [the world-generated ambient-mode plan](../plans/PHASE3B3_WORLD_GENERATED_AMBIENT_MODE.md); spatial composition and reciprocal field coupling remain gated.

**Architecture decision:** The resonance field is a continuous-in-physical-space, persistent, independently updated, multi-frequency wave field. It propagates through the environment with finite travel time and phase. Matter couples to it through its actual exposed physical geometry. The field is not a molecule-to-molecule interaction cache, an instantaneous redistribution model, or a diffusion approximation.

**Performance decision:** Correct wave physics and auditable energy accounting take precedence over an artificial benchmark win. The initial correct implementation may be slower. The engineering target is to optimize until performance at least breaks even with the existing resonance implementation on the standard workload. A temporary prototype regression is acceptable; silently weakening the physics to hide the cost is not.

## 1. Purpose and authority

This roadmap takes the repository from the existing resonance/aura implementation and the merged shape-aware geometry foundation to the intended independent wave-field architecture.

Authority order:
1. Master Technical Specification v2.4 (EVOSIM-MASTER-2.4) and later explicit user-approved decisions remain the project's design authority.
2. This document is the detailed, controlling implementation roadmap for the resonance-field work.
3. docs/design/resonance-field-architecture.md records the high-level architecture and points here.
4. README.md remains the repository's current implementation map and status record.
5. PR plans and dated handoffs are subordinate implementation/history records. If an older plan conflicts with this roadmap, stop and reconcile the conflict explicitly; do not silently revive its assumptions.

Every future resonance-field task must:
- Read this roadmap before proposing or changing code.
- Identify the current phase and the exact exit criteria being addressed.
- Link this document in its issue/PR description.
- State which physical invariants and tests it preserves.
- Update this roadmap and the README status when a phase materially advances.
- Avoid opening overlapping/stacked PRs for the same workstream without an explicit reason.

## 2. Target architecture

### 2.1 Physical layer remains authoritative for matter

The physical layer owns:
- Elements, constituents, molecules, material identity, shape, position, and motion.
- Bonds, structural relationships, collisions, chemical transformations, and displacement.
- Complete realized geometry, exposed boundaries, occlusion, and material properties.
- Source and receiver material properties supplied to the field.

The field may read versioned snapshots/events describing these facts. It must not own, reconstruct, or mutate physical geometry.

### 2.2 Resonance field is independent and exists throughout space

The resonance layer owns:
- A persistent spatial wave state, including phase and frequency-resolved state.
- Source registration and emission schedules, with provenance tied to real energy holders.
- Finite-speed propagation, attenuation, interference, reflection/scattering where physically defined, and boundary behavior.
- Field queries at arbitrary positions and along physical boundaries.
- Energy held in the field, energy flux, transfers to matter, and energy dissipated as heat.

The field exists in empty regions. Molecules do not create its underlying spatial representation merely by existing. Sources inject waves into it; receivers sample/couple to it. Propagation is environmental and must not be implemented as all-pairs molecule-to-molecule propagation.

### 2.3 Continuous physics, finite numerical representation

“Continuous field” describes the physical model, not an infinite-resolution data structure. Begin with the simplest numerically stable reference representation that can demonstrate the required wave physics. A uniform grid is acceptable for the reference solver. Adaptive cells, hierarchical representations, caching, and incremental updates are later implementation optimizations, not prerequisites for correctness.

The field representation must support:
- Spatially varying amplitude and phase.
- Multiple simultaneous frequencies (not one intensity scalar).
- Finite propagation delay and directional energy flux.
- Superposition and interference.
- Stable sampling at arbitrary points and exact physical boundary locations through controlled interpolation/integration.
- Deterministic evolution for a fixed initial state and tick sequence.

A scalar diffusion/heat equation, instantaneous global redistribution, or per-tick centroid force calculation is not an acceptable substitute for wave propagation.

## 3. Physics contracts governing production integration

Phase 1 resolves the design decisions below. They govern Phase 2 and every later implementation phase. Numerical parameter values and solver behavior must be validated through controlled project-native experiments; do not invent constants or choose equations solely for convenient performance.

1. **Field quantity and units:** Define what the field state represents (for example, a signed oscillatory field amplitude or a set of coupled components), its units, and how physical energy density and flux are derived. Use the project-native L/tick/E basis for world length, logical simulation time, energy, power, area, boundary length, frequency, energy density, and flux. Document conversions between derived project-native quantities and keep numerical normalization separate. No SI or wall-clock time conversion is required or intended. Amplitude or spectral magnitude is not itself energy.
2. **Propagation law:** Select the governing wave equation and its dimensionality/components. State assumptions and physical interpretation. The implementation must represent finite propagation speed, not instantaneous influence.
3. **Propagation speed:** Define propagation speed in project-native world-distance units per logical simulation tick. Keep this distinct from wall-clock execution pacing.
4. **Frequency and phase:** Use the fixed logical clock: 24,000 ticks per simulated day and 24,000 ticks per complete solar orbit; they are the same interval. The sun carrier completes 440 cycles and the hotspot carrier 880 cycles during that interval. Runtime pacing is not simulated time and has no influence on physics.
5. **Source ontology and energy provenance:** Define the physical source classes (external radiative world-tone sun, localized in-plane hotspot, true boundary/line emitters if any, and material-origin oscillators). Every source needs a named authoritative energy holder or explicitly accounted external supply, a finite budget or replenishment law, deterministic schedule, and atomic source/field work transfer. Distinct source geometries retain their correct physical dimensions while sharing one source-to-field accounting contract. The sun is outside the infinite flat plane and follows a circular orbit in a plane perpendicular to it; orbital position determines illumination direction. Its orbit is defined in project-native world coordinates, independent of camera, viewport, solver window, and laboratory walls. The legacy orbit derived from finite rectangular environment dimensions must be retired; orbital radius/distance, height, orientation, and phase origin remain to be specified. The sun is an external radiative source, not a boundary emitter: define its radiation pattern, finite source-to-plane propagation delay using the same empty-space propagation speed as the field, incidence geometry, finite emitted power, and conservative coupling into the 2D field. Material-origin emitters are approved powered oscillators with explicit owned internal energy and atomic debit against emitted field work. An equivalent conservative coupled-degree-of-freedom formulation is acceptable only if it preserves that ownership and the same energy balance. Passive material response does not emit. Existing material aura amplitudes are not source energy.
6. **Source footprint and injection:** Define how each source couples energy through its actual physical geometry. For the sun, use a finite-power external radiation pattern and source-to-plane geometry/retarded time; derive incident power density over physical plane area, including incidence projection, and ensure total intercepted power never exceeds emitted power. For an in-plane distributed source, integrate power density over its physical support area. For a true line/boundary emitter, integrate flux over its complete physical boundary length. Do not use projected screen width or renormalize visible source energy independently for each receiver. Verify that total source power is invariant under grid refinement and is not scaled by material-recipient count.
7. **Material coupling:** Define frequency-dependent absorption, reflection, transmission, scattering, and response rules. Reuse established material/resonance semantics where valid; do not create molecule-specific propagation exceptions.
8. **Attenuation and dissipation:** Derive how amplitude, energy density, and flux change during propagation. Distinguish geometric spreading from true dissipative loss. The existing distance law A(d)=A0/(1+d), E(d)=E0/(1+d)^2 is legacy/reference behavior, not automatically the governing law for a finite-speed wave solver. Any change must be derived, documented, and validated rather than silently copied or discarded.
9. **Infinite-plane geometry and laboratory containment:** The physical world is an infinite flat two-dimensional plane. The rectangular display is only an observation window. There are no global environmental edges or edge boundary conditions. Define world-distance units, planar numerical representation, and continuity across storage/viewer windows. Chemistry-lab walls are ordinary physical objects with material-specific interactions, not special environment boundaries; they must be removable without changing the physics.
10. **Numerical stability:** Specify discretization, spatial/time resolution relationship, stability condition (including a CFL-like bound where applicable), interpolation, and error/convergence criteria.
11. **Determinism:** Define update ordering, source ordering, accumulation precision, and deterministic handling of parallel work.

No production migration begins while these choices are ambiguous in code or scattered across PR descriptions.

## 4. Energy and geometry invariants

These invariants are non-negotiable throughout all phases:

- No new energy source, duplicate reservoir, or untracked energy is introduced by the field.
- Source emission is debited from its authoritative energy provenance exactly once.
- Energy in the field is represented explicitly or otherwise accounted for in a rigorously equivalent ledger state.
- Energy transferred to a receiver leaves the field exactly once and enters the receiver's legitimate energy/response accounting exactly once.
- Propagation loss is not charged once per receiver. Dissipated energy is settled as environmental heat exactly once.
- Reflected, scattered, transmitted, escaped, and still-propagating energy must have explicit destinations/accounting; no silent disappearance.
- A spectrum, amplitude, phase, or perception magnitude is not itself an energy credit.
- Internal transfers cancel at world scale. The complete inventory must reconcile source input, field energy, receiver holdings/work, stored structural energy where applicable, and heat/escape.
- Failed or rejected transactions must not partially commit field debit, receiver credit, geometry changes, or ledger changes.
- The existing ledger's validation, inventory, transaction, provenance, and rollback machinery may be reused where valid; its holder categories, reservoir semantics, and reaction-funding rules must be redesigned to match the unified energy ontology. The field must not introduce a parallel accounting authority.

### Geometry and coupling measures

- Receivers use complete realized physical-material geometry, not centroids or bounding-circle/radial approximations.
- Only physically exposed, visible receiver boundary portions participate where occlusion applies.
- Receiver interaction integrates the local field over the exposed receiver boundary, with appropriate material coupling and orientation.
- Sampling must converge as boundary resolution is refined; the final method must not depend on an arbitrary centroid sample.
- A true line/boundary emitter uses its complete physical boundary length as its integration measure. This rule does not apply to the external sun.
- The sun's external radiation is integrated over physical plane area using its source-to-plane geometry, finite travel time, radiation pattern, and incidence projection. Its total intercepted power cannot exceed its emitted power.
- In-plane distributed sources integrate over their physical support area.
- No source is renormalized independently for each receiver or according to the visible camera/solver window.
- Geometry queries must remain deterministic and fail closed for unsupported/ambiguous geometry rather than inventing a substitute shape.

## 5. Starting point and known constraints

### Existing foundation

- PR #377 (merged) optimized the existing resonance movement neighbor discovery. Its pairwise movement-resolution cutoff is specific to that legacy movement path; it must not become a propagation radius or a field cutoff.
- PR #378 (merged) added shape-aware composite source/receiver geometry and perimeter primitives. The field should consume these interfaces rather than duplicate them.
- docs/pr-plans/resonance-source-geometry.md records geometry details and outstanding exact-geometry limitations.

### Geometry work still outstanding

The merged foundation is useful but not equivalent to complete continuous perimeter coupling. Its documented remaining work includes shared/coincident edges, touching boundaries, fluid/line arrangements, exact visibility transitions along receiver intervals, and complete supported-shape coverage. Current directional reception selects one nearest visible source-perimeter point for a source/receiver direction; it does not integrate contributions over the complete extended emitting perimeter. Receiver straight/curved intervals are evaluated at representative midpoint samples rather than by integrating the local field over each complete exposed interval. In addition, the material-origin emitter and chemistry snapshot paths still derive source positions from constituent-placement centroids. These are distinct approximations and must be removed from authoritative field coupling, not confused with the exact perimeter primitives PR #378 introduced.

Before relying on any unsupported arrangement, either complete its exact implementation and tests or preserve fail-closed behavior. Do not substitute tessellation, bounding circles, centroid sampling, or an arbitrary cutoff.

### Existing production behavior

The current resonance/aura and movement path remains authoritative until a staged replacement passes the acceptance criteria. Do not remove it early. Keep a reproducible baseline for correctness diagnostics and performance comparisons.

Older plans that explicitly say “reuse the existing aura; do not add a second field” describe the previous integration scope. They do not override this approved future architecture. The migration must still avoid running two competing energy authorities at once.

## 6. Phased implementation plan

### Phase 0 — Baseline and code/ledger audit

**Status: COMPLETE**

The code/ledger/geometry audit and reproducible baseline data are recorded. Do not repeat Phase 0 unless a later controlled comparison explicitly requires a new baseline.

**Goal:** Know exactly what is being replaced and what must remain invariant.

Work:
- Trace every production resonance source, propagation/reception path, movement-force consumer, reaction-energy consumer, and ledger settlement.
- Identify the authoritative energy owner for every source and receiver.
- Record existing frequency, harmonic, phase, attenuation, occlusion, movement, and energy semantics.
- Identify geometry APIs from PR #378 that are complete, incomplete, or fail-closed.
- Establish reproducible standard-workload runtime and energy/reaction diagnostics before changing behavior.

Exit criteria:
- A written call graph/data-flow map exists.
- Each energy channel has an owner and one settlement point.
- Baseline tests and benchmark commands are reproducible.
- No unresolved duplicate energy path is mistaken for active production behavior.

### Phase 1 — Formal wave-physics specification and contracts

**Status: COMPLETE**

**Goal:** Establish the implementation-independent physical and numerical contracts before building a solver.

Completed in PR #409:
- Unified energy ontology and one conservation authority, including source ownership, local availability, material/internal/thermal/structural/work holders, and atomic debit/credit/rollback rules.
- Continuous scalar wave equation, field energy density and flux, zero empty-space damping, and explicit project-native dimensions.
- Logical clock: 24,000 ticks per simulated day and per solar orbit; 440 sun-carrier cycles and 880 hotspot-carrier cycles per day. Runtime pacing is execution speed only.
- Infinite flat-plane geometry, nonphysical viewer/window, temporary laboratory walls as ordinary material objects, and finite computational-window requirements.
- External isotropic 3D sun geometry, retarded source-to-plane time, exact half-power plane interception, normalized in-plane hotspot source, powered material oscillators, and passive material response.
- Conservative source-work identity, receiver energy partition, deterministic ordering, and transaction/rollback contracts.
- Uniform Cartesian second-order reference numerics, five-point/leapfrog stability bound, second-order refinement protocol, and numerical-tolerance reporting policy.

The legacy 2,400-tick orbit and 86,400-second conversion are recorded as implementation discrepancies, not accepted model settings. Project-native parameter values remain open for controlled Phase 2 experiments; no wall-clock or SI-time calibration is required.

**Exit:** The specification and decision register are complete. Phase 2 is limited to the independent empty-space reference solver with a controlled finite-budget mathematical test source; production integration remains gated.

### Phase 2 — Independent reference wave solver

**Status: COMPLETE — formally closed 2026-11-22.**

Completed and validated as an isolated reference solver under PR #410. Its tested numerical envelope, energy-accounted computational-boundary treatment, and limitations are recorded in that PR. The solver remains separate from production and intentionally unmerged; Phase 2 closure does not authorize production integration or a rebuild of the solver.

**Goal:** Prove real finite-speed wave propagation in empty space before adding molecular complexity.

Work:
- Implement a standalone field state and deterministic update loop.
- Start with the simplest stable spatial discretization that meets the physics specification; do not start with an adaptive solver.
- Support multiple frequencies and phase-resolved evolution.
- Support source injection with finite propagation delay.
- Implement the infinite-plane reference domain without physical environmental edges. Any finite computational window must use a validated truncation treatment and preserve continuity and auditable energy flux; it must not act as a laboratory wall or camera boundary.
- Keep field state persistent between simulation ticks.

Required tests:
- Single-frequency propagation with measured finite travel time.
- Known analytic/reference wave behavior in a simple domain.
- Two-source superposition and constructive/destructive interference.
- Standing-wave behavior and known planar reference modes without artificial reflections from computational-window or viewer edges.
- Multiple simultaneous frequencies without collapsing to one scalar.
- Phase continuity and deterministic replay.
- Grid/time-step refinement showing expected convergence.
- Stability tests at and beyond the allowed time-step bound.
- Empty-space energy/flux accounting with no unexplained creation or loss.

Exit criteria:
- The solver demonstrably propagates waves; it is not diffusion or instantaneous redistribution.
- Numerical stability and convergence are established.
- Empty-space energy accounting closes within a documented tolerance.
- The solver can run independently of molecule count and molecular collision/reaction discovery.

### Phase 3 — Sources, propagation energy, and material coupling

**Current status:** Phase 3A geometry/measurement slices listed below are complete where explicitly marked. Phase 3B has begun with isolated source-accounting and mathematical forcing work, but physical source-to-field coupling has not been established. Do not treat geometry completion or the isolated forcing prototype as live source registration, physical field injection, or production energy settlement.

#### Phase 3A — Geometry and measurement foundation: completed slices

| Slice | Status | Verified work |
|---|---|---|
| 3A.2.1 — Collinear straight-boundary union cases | **COMPLETE** | PR #412: deterministic splitting/classification of shared and overlapping straight intervals, duplicate-edge ownership, and invariance/regression tests. |
| 3A.3 — Strict Phase 3 geometry boundary | **COMPLETE** | PR #414: polygonal solid receivers/occluders and finite straight-line blockers; unsupported circles/fluids fail closed in the Phase 3 adapter without changing legacy callers. |
| 3A.4 — Physical area and boundary measures proposal | **COMPLETE** | PR #416: reviewed/documented distinct L and L² measures and the checked polygon-union approach. |
| 3A.4a — Checked polygon area | **COMPLETE** | PR #417: checked convex polygon area with uncertainty estimate and fail-closed validation. |
| 3A.4b — Checked exposed-boundary length | **COMPLETE** | PR #419: checked straight exposed-boundary length with scale/translation-aware validation. PR #418 was superseded and is not counted as a separate completion. |
| 3A.4c — Checked polygon-union measurement | **COMPLETE** | PR #420: deterministic checked polygon-union area/perimeter with fail-closed topology handling. |
| 3A.4d — Source-footprint geometry | **COMPLETE** | PR #422: checked physical source support area, reusing polygon-union measurement; overlaps/coincident pieces count once. |

**Not marked complete:** PR #411's standalone Phase 3A.1 planning artifact was closed without merge. The merged work above does not establish that every geometry arrangement or continuous perimeter-coupling requirement is solved. Preserve fail-closed behavior for unsupported arrangements and retain the remaining geometry/visibility gaps below as open work.

#### Phase 3B — Source-to-field coupling

**3B.1 — Source/interface audit: COMPLETE.** The audit is recorded in [resonance-field-phase-3b1-source-interface-audit.md](../audits/resonance-field-phase-3b1-source-interface-audit.md).

**3B.2 — Source interface, accounting, and hotspot profile: PARTIALLY COMPLETE.**
- **3B.2 design contract: COMPLETE.** The source descriptor/account/coupling boundary and discrete settlement contract are recorded in [PHASE3B2_SOURCE_INTERFACE_AND_INJECTION_PROPOSAL.md](../plans/PHASE3B2_SOURCE_INTERFACE_AND_INJECTION_PROPOSAL.md).
- **3B.2a source identity and finite accounting: IMPLEMENTED, MERGED.** PR #427 adds stable source identity/provenance, immutable descriptors, finite non-clonable source accounts, signed staged transfers, atomic settlement, and an isolated copy of the validated reference solver. It does not connect a real source or implement continuous forcing.
- **3B.2b normalized hotspot profile: IMPLEMENTED, MERGED.** PR #426 adds the compact-support radial-cosine profile, analytic cumulative radial power, physical-area integration, and normalization/refinement/translation/grid-alignment tests. It does not inject that profile into the field or calibrate physical hotspot power.
- **3B.2c work-conjugate continuous forcing: ISOLATED CANDIDATE A IMPLEMENTED (PR #429 MERGED); VALIDATION CONTINUES.** The extracted reference solver supports one or multiple additive prescribed-acceleration sources. Signed midpoint work is settled against finite source accounts, and the exact combined field-work/sponge-loss identity is checked before state commit. Full CI passed, including positive/negative work, zero work at a velocity node, force sign reversal, multi-source attribution, duplicate-source rejection, simultaneous sponge absorption, overdraw rollback, and fixed-boundary rejection. Deterministic replay passed across 160 multi-source substeps in PR #430. The first sustained continuum-convergence experiment exposed that its paired-refinement comparison mixed spatial and temporal error and did not establish the required order. PR #431 now adds an independent scalar discrete-modal reference, then tests spatial refinement at fixed timestep and temporal refinement at fixed grid. This remains mathematical forcing, not a physical incident-power law. Repeated multi-source attribution remains open. Details: [PHASE3B2C_CONTINUOUS_FORCING_DERIVATION.md](../plans/PHASE3B2C_CONTINUOUS_FORCING_DERIVATION.md).

**Phase 3B is IN PROGRESS.** Source identity/accounting and hotspot profile mathematics are complete. The isolated mathematical forcing prototype has passed its discrete energy/work identity and deterministic replay checks; sustained continuum convergence is still under validation and must distinguish discrete-solver agreement from spatial and temporal continuum error. No real hotspot or sun field injection, source registration in the live simulation, receiver/material settlement, or production behavior has been implemented. A physically defensible mapping from source power/profile to wave-equation forcing remains unresolved. Sun integration (3B.3), receiver/material coupling, and production migration remain separately gated.

**Goal:** Connect the independent wave field to real energy sources and physical boundaries, only after the source-to-field work law is derived and validated.

Work:
- Register existing sources with identity, position, shape, frequency spectrum, phase, schedule, and authoritative available-energy budget.
- Couple the external sun into the plane through the approved finite-power radiation pattern, retarded source-to-plane travel time, incidence projection, and physical-area integration.
- Inject in-plane distributed sources over their complete physical support area using power density.
- Use complete boundary-length integration only for sources that are physically line/boundary emitters.
- Model propagation flux, geometric spreading, attenuation, and dissipative heat according to Phase 1 physics.
- Query field state along complete receiver boundaries.
- Integrate receiver coupling over exposed receiver perimeter with occlusion and material response.
- Support multiple simultaneous sources and receivers without all-pairs propagation.
- Make field-to-material energy transfer atomic with the existing ledger.

Required tests:
- Source energy is distributed once across its complete perimeter.
- Receiver results respond to orientation, shape, exposed perimeter, distance, phase, frequency, and occlusion as specified.
- Two receivers cannot each claim the same finite field energy.
- Multiple receivers and overlapping visibility do not duplicate propagation-loss heat.
- Absorbed, reflected, transmitted, dissipated, escaped, and remaining field energy reconcile.
- Failed receiver settlement rolls back all affected transfers.
- Geometry changes invalidate only affected field/coupling data and cannot leave stale occlusion.

Exit criteria:
- Source, field, receiver, and heat accounting reconcile end-to-end.
- Perimeter coupling uses complete exposed geometry, not centroid or nearest-point shortcuts.
- No molecule-specific field propagation exceptions exist.

### Phase 4 — Controlled integration with movement and chemistry

**Goal:** Make the new field authoritative in production without changing unrelated chemistry semantics.

Work:
- Integrate field queries with the existing physical movement response through a narrow interface.
- Integrate compatible resonance transfer with reaction energy funding through the existing energy ledger.
- Preserve reaction ordering, atomic transaction semantics, and chemistry rules unless a separate approved physics change explicitly changes them.
- Keep the old path available behind a controlled comparison/feature boundary until parity and conservation are demonstrated.
- Run matched old/new diagnostics. Differences caused by finite-speed propagation and interference are expected physics changes; classify them rather than forcing false numerical equivalence.
- Ensure only one path is authoritative for any given energy transfer during a run.

Exit criteria:
- Deterministic movement and chemistry integration.
- No duplicate source, receiver, or energy authority.
- Full energy conservation and transaction rollback tests pass.
- Matched experiments explain behavior differences in terms of the newly specified wave physics.
- No regression in unrelated chemistry invariants.

### Phase 5 — Performance optimization without physics shortcuts

**Goal:** Bring the correct implementation to at least break-even performance against the current baseline.

Order of work:
1. Profile the correct reference/integrated implementation by field propagation, source injection, boundary sampling, coupling integration, ledger settlement, and synchronization.
2. Optimize the measured dominant costs only.
3. Add spatial acceleration for source influence and boundary queries where it preserves the reference solution.
4. Add incremental updates and revision-based invalidation.
5. Add caching for stable source/material/geometry coefficients with explicit invalidation.
6. Introduce adaptive spatial resolution or hierarchical approximations only after comparison against the conservative reference solver.
7. Evaluate parallel execution only with deterministic accumulation and measured benefit.

Rules:
- No arbitrary propagation radius, per-pair cutoff, dropped low-amplitude wave, phase removal, frequency collapse, or unaccounted energy truncation as a performance shortcut.
- Any approximation must have a documented error bound, a conservative reference comparison, and an experiment showing that it does not change physically meaningful outcomes.
- Keep the reference solver as a test oracle after optimized paths are introduced.

Exit criteria:
- The optimized field matches the reference solver within documented numerical tolerances.
- Energy accounting remains closed and deterministic.
- Standard-workload performance reaches at least break-even with the existing resonance implementation. If it does not, report the measured gap and continue profiling; do not weaken physics to claim success.

### Phase 6 — Production migration and retirement of legacy path

**Goal:** Make the field the single production authority.

Work:
- Enable the field by default only after Phases 0–5 pass.
- Remove duplicate legacy propagation/reception logic only after all production consumers have migrated.
- Retain only justified compatibility adapters; do not retain two energy authorities.
- Update README, design docs, diagnostics, and tests to describe the new production architecture.
- Record baseline-versus-final performance and long-duration conservation results.

Exit criteria:
- One authoritative resonance field and one authoritative energy ledger path.
- Full CI green, including formatting, architecture checks, full Rust tests, Clippy, and relevant experiments.
- Standard and long-duration runs complete without instability, energy drift, or unexplained chemistry changes.
- Performance is at least break-even on the standard comparison workload, or any remaining deficit is explicitly accepted by the user.

## 7. Validation and benchmark protocol

### Correctness gates

Every phase adds focused tests and retains all earlier gates. At minimum:
- Mathematical/reference wave tests and convergence.
- Phase, frequency, interference, and finite travel-time tests.
- Geometry, perimeter integration, and occlusion tests.
- Source/field/receiver/heat energy reconciliation.
- Determinism and replay tests.
- Transaction atomicity and rollback tests.
- Long-duration stability and energy-drift tests.
- Chemistry and movement regression tests.

### Performance comparison

Use the established standard environment as the primary matched comparison:
- 60 seconds of real wall-clock experiment execution, with no artificial tick cap; record the logical ticks completed during that interval. Wall-clock duration is a workload-control parameter, not a physical time unit.
- Seed 42 for the canonical paired run.
- 1,280 starting materials: 1,200 independent resources plus 80 molecules.
- Record wall-clock runtime, simulated ticks, throughput, peak memory where available, field update cost, boundary-coupling cost, and energy-accounting diagnostics.
- Compare old and new implementations with the same environment, seed, build mode, and instrumentation.
- Repeat enough runs to distinguish noise from a real regression; include additional seeds before accepting broad performance claims.
- Keep instrumentation overhead measured and avoid permanent hot-path counters without a demonstrated need.

The acceptance target is break-even or better after optimization. A slower early prototype is not a reason to abandon wave physics; it is a reason to measure and optimize the actual cost.

### Long-duration validation

After short reference tests pass, run progressively longer simulations. Track:
- Total emitted source energy.
- Energy currently stored in the field.
- Energy absorbed/transferred to materials.
- Energy converted to environmental heat.
- Energy reflected/transmitted/escaped under explicit rules.
- Total conservation residual and numerical drift over time.
- Runtime and stability as the field evolves.

Do not use a longer observation window as a substitute for identifying a broken eligibility, propagation, or accounting mechanism.

## 8. Explicit non-goals and prohibited shortcuts

- No instantaneous field redistribution.
- No diffusion-only model presented as wave propagation.
- No field represented solely by a per-molecule aura or centroid force.
- No propagation implemented by all-pairs molecule interactions.
- No second energy reservoir or parallel ledger.
- No energy debit/credit outside atomic accounting.
- No arbitrary distance/radius cutoff or silent low-amplitude truncation.
- No fixed uniform-grid resolution treated as a physical law.
- No adaptive-grid complexity before a correct reference solver exists.
- No tessellated or bounding-circle replacement for exact supported geometry.
- No unrelated chemistry, organism, or GUI redesign bundled into the field migration.
- No performance claim based only on isolated microbenchmarks; use matched end-to-end runs.

## 9. Current status and next action

**Current status (November 24 ambient-mode implementation):** Phase 0, Phase 1, Phase 2, Phase 3B.1, and the Phase 3B.2 isolated source-accounting/forcing/oscillator validation gates remain complete as recorded above. The selected ambient mode is intrinsically world-generated: stored energy density follows a 1–10 cosine daily envelope over 24,000 ticks, with a 440-cycle-per-day carrier. Positive changes are world generation; negative changes are world withdrawal. No literal Sun, external oscillator, or separate reservoir is part of this initial ambient model. PR #437 passed exact-head CI on commit `7d397e8c62b3aeb5a673d83bbf2146b2c8813b16`. PR #438 is the active draft for spatial composition. It now contains localized Gaussian packet, nested accounting-patch energy/flux/work, and independent-mode reference tests, plus analytical comparisons of reciprocal bulk couplings. Those new tests have not yet passed an exact-head CI run. No ambient module is connected to the production spatial solver, materials, chemistry, organisms, or production accounting. Legacy production resonance paths remain authoritative.

**Next action — Phase 3B.3:** PR #438's earlier analytic packet and nested-patch tests passed a workflow run on commit `0ca44f9`; subsequent commits added further tests and documentation and require exact-head CI. The workflow trigger gap for this stacked PR has been corrected on PR #437's branch, and fresh CI runs are now in progress. The next coupling candidate is a stable derivative interaction supported only by physical material occupancy, with zero coupling in empty space and pointwise equal-and-opposite field/oscillator work. A candidate intrinsic world-drive term has also been derived from the uncoupled target trajectory: its signed work is `s₀(t) qₜ`, while material exchange remains a separate internal transfer. Coupling-induced departures from the target oscillator state must not be clamped away. Neither the interaction nor drive is selected or implemented. The corrected envelope-energy interpretation, discrete source/exchange balance, positive Hamiltonian, empty-space invariance, interface behavior, conservation, and convergence remain validation gates. Do not use the accounting patch as physical coupling support, add a global Klein–Gordon term, connect production materials/chemistry, or migrate production energy accounting until exact-head CI and the complete source/exchange balance pass. See [PHASE3B3_AMBIENT_SPATIAL_COMPOSITION.md](../plans/PHASE3B3_AMBIENT_SPATIAL_COMPOSITION.md) and [PHASE3B3_WORLD_GENERATED_AMBIENT_MODE.md](../plans/PHASE3B3_WORLD_GENERATED_AMBIENT_MODE.md).

**Required reference for all future resonance-field work:** docs/design/resonance-field-implementation-roadmap.md


## 10. Unified energy-system design and code reuse policy

**This section is a user-approved clarification of the architecture and supersedes any earlier wording in this roadmap that treats the current energy-ledger semantics, environmental usable-energy pool, frequency-indexed resonance reservoirs, or named reaction-funding reservoirs as binding physical authorities.** The continuous spatial wave field remains the selected propagation architecture, but it is one part of a unified energy system—not a standalone replacement pool.

### 10.1 Unified physical energy ontology

Phase 1 must specify all energy forms and conversions together, with consistent units, ownership, and transfer rules:

- **External/source energy:** energy supplied by explicit external sources, with finite emission and provenance.
- **Propagating field energy:** energy carried through space, represented by local field state with energy density and flux derived from the governing equations.
- **Material internal energy:** energy retained in physical degrees of freedom or material-local modes. Resonance describes frequency-dependent response; it is not itself an energy grant.
- **Thermal energy:** energy represented as unresolved microscopic motion or explicit thermal state, with defined local storage and transport.
- **Structural/chemical potential:** energy associated with bonds and material structure only to the extent its physical definition and parameters are validated. Existing bond-graph bookkeeping is a candidate, not automatically a complete thermodynamic model.
- **Mechanical work and other physical transfers:** energy exchanged with motion or other degrees of freedom, accounted on both sides.
- **Boundary escape/external work:** explicit destinations for energy leaving the modeled world or crossing its boundary.

Every transfer identifies a source, destination, and signed amount. Internal transfers cancel in the whole-world balance. No action receives energy merely because it requires it. Local availability must follow from field flux, material ownership, or another explicitly specified physical mechanism; a global chemistry budget is not presumed to make energy locally available.

### 10.2 Reuse map

**Reuse directly where contracts remain valid:**
- PR #378 realized composite geometry, supported straight/curved boundary primitives, visibility queries, perimeter measures, and linear allocation helpers. Extend shared geometry for documented gaps; do not duplicate it in the field.
- Atomic material/chemistry transaction patterns, staged validation, commit/rollback, and deterministic state-update patterns.
- Independent energy-inventory reconciliation and conservation-test techniques, after adapting inventory coverage to the new energy forms.
- Simulation-time phase utilities and frequency-resolved spectrum operations where units and meanings remain valid.
- Material identity/properties and the physical material catalog.
- PR #377 spatial broad-phase techniques as optional query acceleration where they preserve the reference field solution. Its movement-resolution threshold is never a propagation cutoff.

**Reuse after adapting and validating:**
- Energy-ledger structures and settlement helpers: retain finite-value validation, staged allocation, provenance, residual reporting, and rollback. Redesign categories, holder ownership, and balance equations for field, material-internal, thermal, validated structural, work, and boundary energy.
- World-tone and hotspot definitions: retain source identity/configuration, location, frequency, phase schedule, and intended source characteristics where valid. Replace independent receiver-settlement paths with one field-source interface and finite emission/debit contract.
- Harmonic/material-response calculations: reuse as candidate constitutive response data, but derive absorption, reflection, transmission, and energy transfer from the new field/material coupling law.
- Reaction trial builders and atomic commits: retain topology validation and rollback, but replace global-pool/reservoir funding assumptions with Phase 1's local energy-transfer rules.

**Do not retain as physical authorities:** global chemistry-usable pools or frequency-indexed reservoirs as representations of propagating energy; uniform per-constituent source grants; separate world-tone/hotspot settlement authorities; instantaneous centroid-to-centroid resonance as propagation; legacy attenuation/aura magnitude as a wave equation or energy inventory; or reaction subsidies not derived from physically available energy.

### 10.3 One conservation authority, redesigned semantics

There must be one conservation/accounting authority for the complete system. The current ledger implementation is reusable infrastructure, not a binding physical specification. Its validation, inventory, transaction, and rollback mechanics should be preserved where correct; its categories, reservoir semantics, and reaction-funding rules must change wherever they conflict with the unified ontology. The wave field must not introduce a parallel ledger, and legacy paths must not remain a second authority after migration.

### 10.4 Roadmap phase clarifications

- **Phase 0 is complete:** the code/ledger/geometry audit and reproducible baseline data are recorded. Do not reopen it as routine next work.
- **Phase 1 is complete:** the unified specification covers energy forms, units, source ownership, local availability, transfer equations, heat/thermal semantics, structural-energy limits, infinite-plane geometry, laboratory-wall/material-interface behavior, nonphysical observation-window behavior, finite computational-window treatment, numerics, determinism, and the reuse/retirement map.
- **Phase 3–4** must transfer energy through the single redesigned conservation authority. Do not presume the existing usable-energy pool or resonant reservoir remains the funding mechanism.
- **Chemistry activation is not energy.** Field influence may affect activation, but reaction energy must come from physically available local transfers or validated structural-energy changes.
- **Organism-specific reception and energy budgets remain dormant** until an organism has actually been built and its physical interfaces are defined. Environmental physics must not be designed around biological budgeting.
- The old implementation may remain as a comparison/reference path during migration, but no duplicate energy authority may be active once the new path is authoritative.

**Phase 1 disposition:** COMPLETE. The approved logical clock is 24,000 ticks per simulated day and per solar orbit; 440 and 880 are carrier cycles per day, not additional time scales. Project-native L/tick/E units are authoritative; there is no wall-clock or SI-time calibration. The sun, hotspot, powered material oscillator, passive material response, source ownership, and atomic energy-transfer contracts are specified in the Phase 1 specification. The legacy 2,400-tick orbit and 86,400-second conversion are implementation discrepancies, not model choices. **Phase 2 disposition:** COMPLETE and formally closed as an isolated reference solver; no production integration is implied. EXP-2026-008 remains a comparison point, not a requirement to preserve legacy physics.

## 11. User-approved Phase 1 modeling defaults

The user has explicitly approved the concrete defaults below. This section supersedes earlier roadmap wording that left these architectural choices undecided. The detailed physical contracts and dimensional requirements remain in the Phase 1 specification.

- **Field:** one persistent continuous scalar field carrying simultaneous frequencies and phases; linear, isotropic, nondispersive finite-speed propagation in homogeneous empty space; superposition and interference; geometric spreading without automatic energy loss. Field amplitude is not energy.
- **Sun:** external isotropic 3D radiative source illuminating the infinite 2D plane. Its orbit is a vertical circle in a plane perpendicular to the plane, anchored by coordinate convention at world origin and offset above the plane by more than its radius. Use 24,000 ticks per orbit, exactly one simulated day; keep the 440-cycle-per-simulated-day carrier setting separate. Use finite travel time and retarded emission time at the same empty-space propagation speed as the in-plane field. An infinite plane intercepts half of isotropically emitted power; the remainder stays in the external accounting domain. Transfer into the planar field must be conservative and must not double-count energy.
- **Material response/emission:** powered material oscillators may emit only from owned internal energy, with an equal source debit. Passive absorption, reflection, transmission, and scattering redistribute energy under conservative constitutive rules. Resonance is response, not an energy source. Material emitters remain disabled in the empty-space reference solver.
- **Hotspot:** localized in-plane distributed source, retaining radial cosine profile as the initial candidate. Integrate over a defined physical support area; total power is independent of molecule count and grid refinement. Require a finite budget or explicit replenishment. Do not copy the legacy rate without calibration.
- **Units/time:** project-native L, tick, E. One simulated day and one solar orbit are both exactly 24,000 ticks. The sun carrier completes 440 cycles per day; the hotspot carrier completes 880. No wall-clock or SI-time calibration is required or intended. Runtime pacing affects execution speed only.
- **Reference numerics:** uniform Cartesian grid and conservative second-order wave solver; leapfrog or an equivalent conservative second-order integrator; initially target about 16 samples per shortest modeled wavelength and verify convergence. Derive the actual stability bound for the chosen discrete operator and time integrator.
- **Domain/damping/heat:** the physical plane remains infinite. Initial reference domain must be large enough that waves do not reach its edges during measurement; later absorbing or tiled treatment must be validated against the reference. Empty-space damping is zero. Material dissipation credits explicit local thermal storage; heat transport waits for a defined mechanism.
- **Structural energy/performance:** existing bond-graph structural energy remains provisional and separately reported. Optimize only after correctness, convergence, and conservation, with no arbitrary propagation cutoff or unaccounted energy truncation.

The world-origin anchor is a coordinate convention, not a physical center of the infinite plane. The half-power sun interception is a geometric consequence of an isotropic source and an infinite plane, not a free efficiency parameter.

**Project-native parameter values remain open:** orbit radius/height, finite source budgets and power, propagation speed in L/tick, hotspot power/support radius, material response coefficients, field normalization, grid spacing, and solver time step. Select and version them through dimensional consistency, source ownership, conservation, controlled reference experiments, stability, and convergence. No SI conversion or wall-clock calibration is required.

**Phase status:** Phase 0, Phase 1, and Phase 2 are complete. Phase 3A.2.1, 3A.3, and 3A.4a–d are complete as individually listed above. Phase 3B isolated source accounting and Candidate A mathematical forcing are underway; initial discrete-work and continuum-refinement validation passed. Physical source-to-field coupling, project-native parameter calibration, and production integration remain open and gated; no physical-time/SI conversion is required.

## 12. Ambient/disturbance spatial-composition gate

**Status (November 24, 2026):** PR #437's isolated ambient-mode implementation passed exact-head GitHub Actions on commit `7d397e8c62b3aeb5a673d83bbf2146b2c8813b16` (workflow run 45436842952, conclusion: success). This closes only the isolated mode-state implementation gate.

The next analytical gate is recorded in [Phase 3B.3 ambient/disturbance spatial composition](../plans/PHASE3B3_AMBIENT_SPATIAL_COMPOSITION.md). Initial derivation establishes:
- A nonzero-frequency spatially uniform displacement is not a free solution of the massless scalar wave equation; adding it to the disturbance displacement requires an explicit uniform forcing term.
- The energy and flux of an additive displacement contain cross-energy and cross-flux terms. Ambient and disturbance inventories cannot simply be added while ignoring those terms.
- The harmonic-oscillator energy used by the ambient-mode state is not the massless wave-field energy; the degree of freedom and its coupling/ownership must be explicit.
- The slowly varying daily envelope contributes a small but nonzero oscillator-energy correction omitted by the carrier-only amplitude mapping.
- The finite world area is an accounting patch, not the physical support boundary of a uniform mode on the infinite plane.

These are analytical constraints, not a selected composition law. The required isolated numerical checks are listed in the plan. No global Klein–Gordon term, production solver change, material coupling, chemistry integration, or organism behavior is authorized. Keep the ambient-mode and disturbance systems isolated until a candidate Hamiltonian and its discrete energy balance pass the stated tests.
