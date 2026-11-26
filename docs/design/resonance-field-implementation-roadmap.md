# Resonance Field — Formal Implementation Roadmap

**Status:** Approved architectural direction; implementation roadmap. The continuous, time-propagating wave field is the target architecture. This document is the required reference for every future resonance-field investigation, issue, branch, pull request, implementation, and validation run.

**Architecture decision:** The resonance field is a persistent spatial wave field with phase-resolved, finite-speed propagation. Its ambient energy is held by the field as a spatially uniform background state with the approved daily envelope and explicit generation/withdrawal accounting. Spatial wave behavior is built on the already validated propagation model. Matter interacts through its actual exposed physical geometry. The field is not a molecule-to-molecule interaction cache, instantaneous redistribution, or diffusion approximation.

**Current approved ambient model:** one simulated day is 24,000 ticks; the ambient carrier completes 440 cycles per day; cycle-averaged ambient energy density follows the approved cosine envelope 10 → 1 at tick 12,000 → 10 at tick 24,000. Opening inventory is not generation. Positive stored-energy change is world generation; negative change is world withdrawal. The environment area is used only to calculate total ambient inventory, never as a propagation boundary. This is implemented in the live simulation and documented in [the Phase 3 ambient-energy report](../results/phase-3-live-ambient-energy.md).

**Explicitly not part of the active model:** no Sun object, external radiative injector, localized hotspot injector, powered material oscillator, or additional buffer/reservoir. The earlier three-source Phase 3B.3 program is superseded. Do not revive it or repeat its source-model derivations.

**Performance decision:** Correct wave physics and auditable energy accounting take precedence over an artificial benchmark win. The initial correct implementation may be slower. The engineering target is to optimize until performance at least breaks even with the existing resonance implementation on the standard workload. A temporary prototype regression is acceptable; silently weakening the physics to hide the cost is not.


> **Current approved direction — 2026-11-26 (supersedes older Phase 3C integration wording below)**
>
> The resonance field remains detached from production simulation ticks while chemistry proceeds independently. The immediate field work is an isolated, independently validated finite closed-world reference field. The approved physical boundary is reflecting with zero normal energy flux. Older infinite-plane computational-outflow, Mur/Sommerfeld, and sponge-boundary proposals are not the current target.
>
> Use the contiguous dense uniform-grid solver as the initial reference candidate because the parity/performance prototype (#447) passed focused Actions validation and measured roughly 158–196× faster than the adaptive reference on the reported 250 × 250 workload. This is a storage/performance result, not proof that its existing fixed-zero edge is the approved reflecting boundary. Implement and validate the reflecting boundary separately. Keep adaptive refinement as a comparison/possible later optimization, not the first implementation.
>
> The isolated field now has a deterministic position-local query contract and an initial reciprocal transfer operator. The transfer operator solves against the exact discrete leapfrog invariant; pointwise energy density is not a withdrawable balance. The current implementation step is to adapt that operator to chemistry's staged environmental funding contract: stage against a private field copy, reserve against duplicate pending claims, then commit the field debit only when the reaction is ready or cancel it if reaction validation fails. The adapter remains isolated; it does not connect to chemistry, alter structural-energy accounting, or enter the production tick. The initial point-local bilinear interaction is a reference contract, not a chemistry geometry model. No global usable-energy pool or frequency-indexed reservoir substitutes for local field energy.
>
> **Approved sequence:** (1) consolidate the dense uniform solver as an isolated reference; (2) implement and validate the finite reflecting boundary and closed-world energy accounting; (3) expose validated local field queries; (4) validate reciprocal, atomic, position-local field/matter transactions against the discrete invariant, including repeated and overlapping interactions; (5) validate staged field funding with a single outstanding reservation and explicit commit/cancel lifecycle; (6) define and validate the chemical-structure interaction rule before any chemistry connection. Step 6 now has an isolated reference accounting kernel and contract in [resonance-chemical-structure-interaction-contract.md](resonance-chemical-structure-interaction-contract.md): incoming normal flux is integrated over the exact exposed boundary, then partitioned into reaction-work absorption, reflection, transmission, and scattering. Material coefficients and the geometry/spectral adapters remain uncalibrated/unimplemented; the field stays detached. Optimize/adapt only after correctness and matched-workload measurements.
>
> **PR disposition:** #446 and #448 are superseded and must not be merged. #447 remains isolated prototype evidence; do not merge it as-is because it is test-only and includes absorbing-boundary validation that is no longer the approved physical boundary.

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
- The persistent, spatially uniform ambient background energy state and its approved phase/envelope schedule.
- The spatially varying, phase-resolved wave state and its finite-speed evolution.
- Field energy, energy flux, and the single accounting path for ambient generation/withdrawal and any later field-to-matter transfer.
- Field queries at arbitrary positions and along physical material boundaries.

The ambient background exists throughout empty space; it is not created by molecules and is not injected by a Sun object, hotspot, material oscillator, or extra reservoir. Propagating spatial disturbances remain environmental field behavior, not all-pairs molecule-to-molecule interactions.

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
4. **Ambient phase and logical time:** Use the fixed logical clock: 24,000 ticks per simulated day. The approved ambient carrier completes 440 cycles per day. Its cycle-averaged energy density follows the validated 10 → 1 → 10 daily envelope. Runtime pacing is execution speed only and has no influence on physics.
5. **Ambient energy ownership:** Ambient energy is held by the resonance field itself. The opening ambient inventory is initial held energy, not generated energy. Each tick's signed change in held ambient energy is booked once: positive change as world generation, negative change as world withdrawal. The live ledger records opening held energy, cumulative generation, cumulative withdrawal, and current held energy. The environment area is used only to calculate total ambient inventory; it is not a physical boundary or propagation domain. No Sun object, external injector, hotspot injector, powered material oscillator, or additional buffer/reservoir is part of the approved active model. The old source classes remain historical research artifacts only.
6. **Spatial field and matter interaction:** The ambient background and spatial wave field must coexist without duplicating energy or creating a second accounting authority. The already validated finite-speed propagation model remains the spatial transport foundation. Material interaction is passive unless a separately approved physical mechanism is introduced; absorption/reflection/transmission/scattering must transfer energy through the unified accounting path and actual supported physical geometry. Do not turn resonance response into an energy source.
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
- Receiver interaction integrates over the actual exposed physical boundary using the supported geometry and material response.
- Source-footprint integration rules from the earlier Sun/hotspot/line-emitter proposal are historical only; no such source class is active or required by the approved ambient model.
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
- Logical clock: 24,000 ticks per simulated day; the approved ambient carrier completes 440 cycles per day. Runtime pacing is execution speed only.
- Infinite flat-plane geometry, nonphysical viewer/window, temporary laboratory walls as ordinary material objects, and finite computational-window requirements.
- The Phase 1 specification also recorded candidate Sun, hotspot, and powered-material-oscillator source models. Those candidates are historical and are superseded by the later approved ambient-field model; they are not active implementation requirements. Passive material response remains the next material-interaction requirement.
- Conservative source-work identity, receiver energy partition, deterministic ordering, and transaction/rollback contracts.
- Uniform Cartesian second-order reference numerics, five-point/leapfrog stability bound, second-order refinement protocol, and numerical-tolerance reporting policy.

The legacy 2,400-tick orbit and 86,400-second conversion are recorded as implementation discrepancies, not accepted model settings. Project-native parameter values remain open for controlled Phase 2 experiments; no wall-clock or SI-time calibration is required.

**Exit:** The specification and decision register are complete. Phase 2 is limited to the independent empty-space reference solver with a controlled finite-budget mathematical test source; production integration remains gated.

### Phase 2 — Independent reference wave solver

**Status: COMPLETE — formally closed 2026-11-22.**

Completed and validated as an isolated reference solver under PR #410. Its tested numerical envelope, energy-accounted computational-boundary treatment, and limitations are recorded in that PR. The solver remains a validated reference implementation; do not rebuild it or repeat its propagation, interference, stability, replay, or convergence work. Production integration is the next use of this foundation. Mathematical test forcing used by the isolated solver is not an approved production source model.

**Goal:** Prove real finite-speed wave propagation in empty space before adding molecular complexity.

Work:
- Implement a standalone field state and deterministic update loop.
- Start with the simplest stable spatial discretization that meets the physics specification; do not start with an adaptive solver.
- Support multiple frequencies and phase-resolved evolution.
- Support the controlled mathematical test forcing needed to validate finite-speed propagation; this is solver verification, not approval for a production source/injector architecture.
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

**Current status:** Phase 3A geometry/measurement slices listed below are complete where explicitly marked. The approved world-generated ambient energy envelope is now implemented in the production simulation tick and its signed generation/withdrawal inventory is recorded in the live energy ledger. This delivers the ambient-energy milestone; it does not claim material absorption or spatial coupling. Further source/material coupling research is deferred and is not a prerequisite for this ambient implementation.

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

#### Phase 3B — Source-to-field coupling research: CLOSED / SUPERSEDED

The historical Phase 3B work established reusable mathematics and accounting infrastructure:
- PRs #426–#427: normalized hotspot-profile mathematics and source-account/provenance transaction infrastructure.
- PRs #429–#432: prescribed-forcing work identity, deterministic replay, convergence/reference comparisons, and repeated multi-source reconciliation.
- PR #433: reciprocal oscillator comparator and controlled field-response comparison.

These results remain available as research and reusable code where appropriate. They do **not** authorize or require production Sun, hotspot, or powered-material-oscillator sources. The approved ambient-energy model supersedes the old three-source plan. Do not continue Phase 3B.3, derive its source classes, or repeat its completed numerical work.

#### Phase 3C — Ambient background and spatial field integration: IN PROGRESS — adaptive numerical verification gate

**Goal:** Integrate the approved ambient background and the validated finite-speed disturbance physics into the live simulation as one persistent resonance field, using the approved adaptive spatial-grid architecture.

**Implementation order:**
1. **Reuse the validated numerical core.** The reusable leapfrog reference copy is `src/physics/reference_wave_solver.rs` (extracted from PR #410 and retained on main through the Phase 3B work). Preserve its discrete energy invariant, CFL condition, and accounted computational outflow. Do not rebuild it or repeat propagation/convergence experiments.
2. **Preserve the approved ambient background.** Use the existing 24,000-tick/440-cycle schedule and 10 → 1 → 10 cycle-averaged energy envelope. The background remains spatially uniform; the 250 × 250 area is inventory bookkeeping only. Preserve opening/current/generated/withdrawn accounting. No Sun object, external injector, hotspot injector, powered material oscillator, or extra buffer is introduced.
3. **Compose ambient and disturbance state correctly.** The total displacement is the ambient carrier plus the spatial disturbance. Retain the validated cross-energy, cross-flux, and signed world-drive terms; never add the two energy inventories naïvely. The periodic amplitude correction now makes the carrier-averaged total oscillator energy match the declared envelope exactly.
4. **Implement and verify the adaptive spatial operator.** The sparse balanced quadtree, conservative regridding transactions, and fixed-mesh leapfrog evolution are implemented (PR #441); repeated refine/coarsen profiles passed (PR #442); adaptive/reference comparisons passed their stated bounds (PR #443). PR #444 replaced the inconsistent two-point coarse/fine interface flux with a symmetric variational multipoint stiffness operator assembled from face-endpoint linear triangles with massless vertex degrees of freedom statically condensed. The correction passed affine patch preservation, energy-drift, three-resolution convergence, locally refined/reference, regridding, and full repository CI checks. Uniform meshes retain the validated reference stencil. PR #444 is merged; do not repeat its completed numerical investigations.
5. **Integrate with the live simulation clock.** Advance field phase and propagation from logical ticks, independent of runtime pacing. Persist field state wherever the simulation persists its world state.
6. **Expose physical field queries.** Sample at actual positions and supported material boundaries using the existing geometry foundation. Unsupported geometry remains fail-closed; no centroid or nearest-point shortcuts.
7. **Run only focused integration checks.** Verify deterministic tick advancement, ambient schedule/accounting, adaptive-grid conservation at refinement interfaces, and agreement with the established uniform-grid reference over the existing validated envelope. Do not rerun the broad Phase 2 validation suite or launch exploratory parameter sweeps.

**Current verification gate — updated 2026-11-24:**

- PR #441 merged: adaptive quadtree, conservative fixed-mesh leapfrog evolution, automatic CFL substeps, and energy-constrained refine/coarsen transactions are implemented but remain isolated from live simulation ticks.
- PR #442 merged: repeated refine/advance/coarsen tests passed for Gaussian, anisotropic, radial-ring, and oscillatory wave profiles.
- PR #443 merged: the uniform adaptive mesh matched the uniform reference to the test tolerance; locally refined cases stayed within the explicitly tested error bound. This was bounded agreement, not a convergence-order proof.
- PR #444 merged: the coarse/fine spatial-consistency defect was corrected with a symmetric variational multipoint stiffness operator. The merged implementation passed affine patch preservation, energy-drift, three-resolution convergence, locally refined/reference, and regridding checks. The exact PR-head GitHub Actions run completed successfully, including formatting, repository architecture checks, Rust tests, experiment checks, and Clippy.
- **Adaptive numerical verification gate: COMPLETE.** Do not repeat the completed Phase 2 propagation suite or PR #444's focused numerical investigations. Proceed to Phase 3C live integration using the validated adaptive operator and its existing reference solver as the oracle.

**User-approved initial numerical scales (2026-11-24):**

- Propagation speed: `c = 1.0 L/tick`, a project-native provisional reference convention, not an SI or wall-clock conversion.
- Field normalization: `K = 1.0`, with all ambient carrier and disturbance amplitudes expressed consistently in the normalized field variable. Preserve the ambient schedule and all cross-energy/cross-flux/world-drive terms.
- Adaptive base-cell spacing: `1.0 L`; maximum refinement level: `2` (finest cell width `0.25 L`).
- Solver substeps remain automatic and must satisfy the adaptive CFL bound; do not hardcode a substep count.

**Live integration is now unblocked by the adaptive-operator gate, but remains unimplemented.** The next implementation must integrate the persistent field, ambient composition, and one reconciled energy-accounting authority. A finite adaptive-grid extent is only a computational window: the 250 × 250 ambient inventory footprint is not automatically the propagation domain. Use validated outgoing treatment or controlled domain expansion with explicit computational-outflow accounting; never introduce a physical edge or silently discard energy.

**Integration constraints confirmed by the post-PR-444 source audit:**

- `AdaptiveSpatialGrid` currently has a fixed finite extent. Its physical-position sampler returns no value outside that extent, and outer boundary cells are fixed-zero numerical nodes. The grid has no domain-expansion or recentering operation. Do not silently treat the current grid rectangle as the infinite physical plane.
- The current absorbing sponge accounts for the discrete damping loss of the disturbance-only solver. It does not yet prove the energy balance of the composed ambient-plus-disturbance field, whose total energy includes the ambient/disturbance cross term. The absorber must not be credited as complete total-field outflow accounting until that cross-term contribution is derived and validated, or the computational-domain strategy avoids that truncation.
- The current adaptive API exposes a cell's current displacement sample, but not a complete position-resolved state containing displacement, time derivative, and spatial gradient. Add a deterministic, numerically consistent query/reconstruction path before relying on local composed energy, flux, or material-boundary values. Do not substitute nearest-cell or centroid sampling.
- The existing `EnergyLedger` owns ambient inventory totals but has no spatial-field energy, ambient/disturbance cross-energy, signed cross-drive work, or total-field computational-outflow entries. Extend the existing authority and its independent inventory reconciliation; do not add a second ledger.
- The 250 × 250 ambient footprint remains an inventory convention only. Select a separate computational-window policy, with explicit expansion or a validated total-field outgoing treatment, before live integration can be considered complete.

These are implementation requirements discovered by source inspection, not requests to change the approved physical model. They do not reopen the completed PR #444 numerical validation gate.
**Phase 3C exit criteria:**
- The live simulation owns and advances a persistent adaptive spatial resonance field.
- The approved ambient background and propagating disturbance are composed with the full energy/flux terms.
- Ambient inventory, field energy, world work, and computational outflow reconcile under one accounting authority.
- The field is deterministic and independent of wall-clock pacing.
- The adaptive implementation preserves the validated propagation behavior without introducing a physical boundary.

**Not included in Phase 3C:** material absorption/chemistry settlement, organism behavior, GUI redesign, performance optimization, or new source classes.

**Phase 3D — Passive material interaction: AFTER 3C**

Once Phase 3C is complete, implement the approved passive material response through actual exposed physical geometry:
1. Complete only the documented geometry/visibility gaps required by the supported material shapes.
2. Integrate local field interaction over exposed receiver boundaries with the validated field state; no centroid shortcuts.
3. Implement absorption/reflection/transmission/scattering as passive energy transfers under the unified ledger. Resonance response is not an energy grant.
4. Make each transfer atomic with material state and energy-ledger updates; rejected interactions roll back fully.
5. Validate conservation and deterministic material interaction with focused tests.

**Phase 3D exit criteria:** supported material interactions are geometry-correct, atomic, deterministic, and energy-conserving. Unsupported geometry remains fail-closed.

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
1. Profile the correct reference/integrated implementation by ambient-background update, field propagation, boundary sampling, coupling integration, ledger settlement, and synchronization.
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

**Completed foundations:**
- Phase 0 audit and baseline.
- Phase 1 wave/energy contracts and project-native numerical specification, subject to the later ambient-model supersession stated above.
- Phase 2 independent finite-speed spatial wave solver, formally closed and validated. Do not rebuild or repeat its propagation calculations.
- Phase 3A listed geometry/measurement slices.
- Phase 3B isolated source-accounting and Candidate A/B mathematical work, retained as historical reusable research only.
- Live ambient-energy implementation: 24,000-tick day, 440-cycle carrier, 10 → 1 → 10 energy-density envelope, and signed generation/withdrawal accounting. Implemented and full exact-head CI passed in PR #439, merged as `e11ec244d9542b86bb00a3c834f300174b5489e6`.

**Phase 3C implementation started:** PR #440 adds the periodic envelope-kinetic amplitude correction and the combined ambient/disturbance displacement, energy, cross-flux, and world-drive composition API in production Rust. These use the already completed Phase 3B.3 calculations; no propagation experiment was repeated.

**Current production gap:** The ambient schedule and ambient/disturbance composition contract are implemented, and the adaptive grid/regridding foundation exists. The adaptive solver is still isolated from live simulation ticks. The original two-point coarse/fine operator failed the affine patch test; a symmetric variational multipoint replacement is under validation. The remaining Phase 3C work is to establish adaptive numerical correctness, then integrate persistent field state with logical ticks, queries, and unified accounting—not to repeat Phase 2 propagation research.

**Next action — Phase 3C:** Complete the multipoint interface-operator investigation and focused numerical gates. After those pass, integrate the persistent adaptive field with the live logical tick and reconcile ambient inventory, field energy, world work, and computational outflow under one accounting authority. Do not connect material interaction or organism behavior prematurely; do not revive Sun/hotspot/material-oscillator injection, add a buffer, or introduce a second energy ledger.

**After Phase 3C:** Complete passive material coupling through supported physical geometry (Phase 3D); integrate field effects with movement and chemistry under one energy authority (Phase 4); optimize measured bottlenecks without physics shortcuts (Phase 5); then retire the legacy resonance path (Phase 6).

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
- Ambient-energy schedule and inventory: reuse the merged live implementation and its signed generation/withdrawal accounting. Do not convert it into a Sun/hotspot source account or add a second reservoir. Historical world-tone/hotspot descriptors are not active source requirements.
- Harmonic/material-response calculations: reuse as candidate constitutive response data, but derive absorption, reflection, transmission, and energy transfer from the new field/material coupling law.
- Reaction trial builders and atomic commits: retain topology validation and rollback, but replace global-pool/reservoir funding assumptions with Phase 1's local energy-transfer rules.

**Do not retain as physical authorities:** global chemistry-usable pools or frequency-indexed reservoirs as representations of propagating energy; uniform per-constituent source grants; separate world-tone/hotspot settlement authorities; instantaneous centroid-to-centroid resonance as propagation; legacy attenuation/aura magnitude as a wave equation or energy inventory; or reaction subsidies not derived from physically available energy.

### 10.3 One conservation authority, redesigned semantics

There must be one conservation/accounting authority for the complete system. The current ledger implementation is reusable infrastructure, not a binding physical specification. Its validation, inventory, transaction, and rollback mechanics should be preserved where correct; its categories, reservoir semantics, and reaction-funding rules must change wherever they conflict with the unified ontology. The wave field must not introduce a parallel ledger, and legacy paths must not remain a second authority after migration.

### 10.4 Roadmap phase clarifications

- **Phase 0 is complete:** the code/ledger/geometry audit and reproducible baseline data are recorded. Do not reopen it as routine next work.
- **Phase 1 is complete:** the unified specification covers energy forms, units, source ownership, local availability, transfer equations, heat/thermal semantics, structural-energy limits, infinite-plane geometry, laboratory-wall/material-interface behavior, nonphysical observation-window behavior, finite computational-window treatment, numerics, determinism, and the reuse/retirement map.
- **Phase 3–4** must transfer energy through the single conservation authority, with the approved ambient inventory and spatial field reconciled rather than duplicated. Do not presume the existing usable-energy pool or resonant reservoir remains the funding mechanism.
- **Chemistry activation is not energy.** Field influence may affect activation, but reaction energy must come from physically available local transfers or validated structural-energy changes.
- **Organism-specific reception and energy budgets remain dormant** until an organism has actually been built and its physical interfaces are defined. Environmental physics must not be designed around biological budgeting.
- The old implementation may remain as a comparison/reference path during migration, but no duplicate energy authority may be active once the new path is authoritative.

**Phase 1 disposition:** COMPLETE as a historical specification milestone, with its candidate Sun/hotspot/powered-oscillator source ontology superseded by the later explicit ambient-field decision. The active logical clock is 24,000 ticks per simulated day; the ambient carrier completes 440 cycles per day. Project-native L/tick/E units are authoritative; no wall-clock or SI-time calibration is required. **Phase 2 disposition:** COMPLETE and formally closed as an isolated reference solver. Its propagation, stability, interference, replay, and convergence work is accepted; do not repeat it. EXP-2026-008 remains a comparison point, not a requirement to preserve legacy physics.

## 11. Active approved model and superseded Phase 1 candidates

The Phase 1 specification and the earlier source-coupling plans contain candidate source models that were later superseded by the user's explicit ambient-field decision. Preserve those documents as historical records, but apply the following current decisions wherever they conflict:

- **Ambient field:** persistent, spatially uniform background energy held by the resonance field. One simulated day is 24,000 ticks. The ambient carrier completes 440 cycles per day. The cycle-averaged energy density follows the approved cosine envelope: 10 at tick 0, 1 at tick 12,000, and 10 at tick 24,000.
- **Accounting:** opening ambient energy is initial inventory, not generation. Positive per-tick change in held ambient energy is world generation; negative change is world withdrawal. Track opening held, cumulative generated, cumulative withdrawn, and current held energy. The 250 × 250 environment area is used only to calculate total ambient inventory; it is not a physical boundary.
- **No source objects:** no Sun object, external radiative injector, hotspot injector, powered material oscillator, or additional buffer/reservoir. Do not implement or investigate these as prerequisites.
- **Spatial propagation:** retain the validated continuous scalar wave model, finite-speed propagation, phase/frequency state, superposition, interference, and its validated numerical reference implementation. Do not repeat its derivations or broad validation suite.
- **Matter interaction:** passive material response is the next physical interaction layer. It must use actual exposed material geometry and transfer energy through the unified accounting authority. Resonance response is not an energy source.
- **Biology:** no organism-specific energy budget, source, or reception behavior until an organism has actually been built and its physical interfaces are established.

**Current phase order:** Phase 3C integrates the validated spatial field with the live ambient background and accounting. Phase 3D implements passive material interaction. Phase 4 connects field effects to movement and chemistry. Phase 5 optimizes measured costs. Phase 6 retires the legacy resonance path. Each phase uses focused checks for its new integration boundary; completed propagation mathematics and validation are not repeated.

**Project-native units:** L, tick, E remain authoritative. Runtime pacing is execution speed only. No SI or wall-clock time calibration is required.
