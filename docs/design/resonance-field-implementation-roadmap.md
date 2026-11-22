# Resonance Field — Formal Implementation Roadmap

**Status:** Approved architectural direction; implementation roadmap. The continuous, time-propagating wave field is the target architecture. This document is the required reference for every future resonance-field investigation, issue, branch, pull request, implementation, and validation run.

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

## 3. Physics decisions required before production integration

Do not invent constants or choose equations solely for convenient performance. Record and test the following before declaring the reference solver physically specified:

1. **Field quantity and units:** Define what the field state represents (for example, a signed oscillatory field amplitude or a set of coupled components), its units, and how physical energy density and flux are derived. Amplitude or spectral magnitude is not itself energy.
2. **Propagation law:** Select the governing wave equation and its dimensionality/components. State assumptions and physical interpretation. The implementation must represent finite propagation speed, not instantaneous influence.
3. **Propagation speed:** Define the world-unit/physics-tick relationship and propagation speed. Keep this distinct from wall-clock execution pacing.
4. **Frequency and phase:** Preserve the established simulation-time oscillator convention: 24,000 physics ticks per simulated day and 440 reference-tone oscillations per simulated day. The 2,400-tick world-source orbital cycle is separate. Derive phase from simulation time/state, not wall-clock time.
5. **Source ontology and energy provenance:** Define the physical source classes (external world tone, localized hotspot, and material-origin oscillators). In particular, decide whether material-origin resonance is powered emission, a conservative coupled degree of freedom, or another explicitly funded mechanism. Existing material aura amplitudes are not automatically source energy and may not create energy without a named owner/debit or a formally conservative interaction.
6. **Source boundary condition:** Define how source energy is injected over time and across the complete exposed source perimeter. Use the geometry foundation's linear perimeter measure/allocation as the starting contract; do not renormalize visible source energy independently for each receiver.
7. **Material coupling:** Define frequency-dependent absorption, reflection, transmission, scattering, and response rules. Reuse established material/resonance semantics where valid; do not create molecule-specific propagation exceptions.
8. **Attenuation and dissipation:** Derive how amplitude, energy density, and flux change during propagation. Distinguish geometric spreading from true dissipative loss. The existing distance law A(d)=A0/(1+d), E(d)=E0/(1+d)^2 is legacy/reference behavior, not automatically the governing law for a finite-speed wave solver. Any change must be derived, documented, and validated rather than silently copied or discarded.
9. **Environment boundaries:** Implement the Master Spec 2.4 world topology (horizontal wrap and vertical hard boundaries). Separately specify the wave boundary condition at each edge (periodic continuation, reflection, absorption, or a justified combination). The Phase 0 audit found the current material-field coordinate lookup rejects out-of-range x and wraps y; reconcile the axis convention against Master Spec 2.4 before choosing wave boundary conditions. A hard physical wall does not by itself determine whether waves reflect or dissipate.
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
- Existing energy-ledger semantics remain authoritative; the field does not introduce a parallel accounting authority.

### Perimeter coupling

- Sources and receivers use complete realized physical-material geometry, not centroids or bounding-circle/radial approximations.
- Only physically exposed, visible boundary portions participate where occlusion applies.
- Source allocation uses the complete source perimeter as its denominator; it is not renormalized per receiver.
- Receiver interaction integrates the local field over the exposed receiver boundary, with appropriate material coupling and orientation.
- Sampling must converge as boundary resolution is refined; the final method must not depend on an arbitrary centroid sample.
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

**Goal:** Turn the target into explicit equations, units, state, and interfaces before building a production solver.

Work:
- Resolve every item in Section 3.
- Define field state, source descriptor, receiver descriptor, boundary-condition interface, energy-flux interface, and field query contract.
- Define geometry snapshot/version invalidation and event ordering.
- Define the relationship between simulation ticks, spatial units, propagation speed, carrier frequency, and source schedules.
- Define ledger transactions for emission, field storage/transport, absorption, and heat.

Exit criteria:
- Equations and units are documented and dimensionally consistent.
- Boundary behavior and material coupling are explicit.
- Interface contracts and failure/rollback semantics are testable.
- The specification is reviewed against Master Spec 2.4 and current energy semantics.

### Phase 2 — Independent reference wave solver

**Goal:** Prove real finite-speed wave propagation in empty space before adding molecular complexity.

Work:
- Implement a standalone field state and deterministic update loop.
- Start with the simplest stable spatial discretization that meets the physics specification; do not start with an adaptive solver.
- Support multiple frequencies and phase-resolved evolution.
- Support source injection with finite propagation delay.
- Implement specified environment edge conditions.
- Keep field state persistent between simulation ticks.

Required tests:
- Single-frequency propagation with measured finite travel time.
- Known analytic/reference wave behavior in a simple domain.
- Two-source superposition and constructive/destructive interference.
- Standing-wave formation under the specified boundary conditions.
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

**Goal:** Connect the independent wave field to real energy sources and physical boundaries.

Work:
- Register existing sources with identity, position, shape, frequency spectrum, phase, schedule, and authoritative available-energy budget.
- Inject source energy across the complete exposed source perimeter using the approved perimeter measure.
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
- 60 seconds of simulation time, measured using the simulation's real clock semantics; no artificial tick cap.
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

**Current status:** Architecture direction approved. PR #377 and PR #378 are merged. The Phase 0 code/ledger/geometry audit is recorded in [resonance-field-phase-0-audit.md](../audits/resonance-field-phase-0-audit.md). The audit confirms that the live system consists of separate material-origin movement, local chemistry activation, external world-tone input, and hotspot input paths; none is yet a persistent finite-speed wave field. The shape-aware perimeter geometry is a starting foundation, not a completed field implementation. The independent persistent wave field, finite-speed propagation, field energy lifecycle, complete perimeter coupling, and production migration remain future work.

**Next action:** Close Phase 0 by recording the reproducible 60-second seed-42, 1,280-material current-main performance baseline with phase timing and energy diagnostics. Then begin Phase 1's formal source/energy and wave-physics decisions. In particular, reconcile the observed coordinate-topology discrepancy and decide the energy semantics of material-origin oscillators before solver implementation.

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

- **Phase 0** remains open until the reproducible current-main baseline is recorded. The existing code/ledger/geometry audit is complete, but no new baseline benchmark was run during that audit.
- **Phase 1** is a unified energy-and-wave physics specification, not merely selection of a wave equation. It must include energy forms, units, source ownership, local availability, transfer equations, heat/thermal semantics, structural-energy limits, boundaries, numerics, determinism, and this reuse/retirement map.
- **Phase 3–4** must transfer energy through the single redesigned conservation authority. Do not presume the existing usable-energy pool or resonant reservoir remains the funding mechanism.
- **Chemistry activation is not energy.** Field influence may affect activation, but reaction energy must come from physically available local transfers or validated structural-energy changes.
- **Organism-specific reception and energy budgets remain dormant** until an organism has actually been built and its physical interfaces are defined. Environmental physics must not be designed around biological budgeting.
- The old implementation may remain as a comparison/reference path during migration, but no duplicate energy authority may be active once the new path is authoritative.

**Immediate sequence:** complete Phase 1's open physical decisions, starting with the Master Spec 2.4 coordinate/topology reconciliation, the field's physical interpretation and units, and the source dimensionality/material-oscillator semantics. No production energy or resonance code should be rewritten until the Phase 1 exit gate is satisfied. EXP-2026-008 is a comparison point, not a requirement to preserve legacy physics.
