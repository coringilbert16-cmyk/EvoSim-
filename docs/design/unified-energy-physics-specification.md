# Phase 1 — Unified Energy and Continuous Wave-Field Specification

**Status:** Working specification / decision register. Phase 1 is in progress. This document does not authorize production implementation.

**Controlling roadmap:** [Resonance Field — Formal Implementation Roadmap](resonance-field-implementation-roadmap.md)  
**Baseline:** [EXP-2026-008 pre-wave reference](../../experiments/records/results/EXP-2026-008-pre-wave-reference/consolidated-report.json)  
**Architecture target:** One unified energy model with a persistent, continuous-in-space, finite-speed, multi-frequency wave field as its spatial transport layer.

## 1. Purpose and design constraints

Phase 1 converts the approved architecture into explicit physical and numerical contracts. It must be completed before production solver or energy migration work begins.

The design is from first principles. Existing code is evidence about current behavior and reusable engineering, not a physical authority. In particular, the current environmental usable-energy pool, frequency-indexed retained-energy reservoir, material aura amplitude, and reaction-funding paths do not constrain the new physical ontology.

The following constraints are fixed for this work:

1. Physics and chemistry precede biology. No organism-specific reception, usable-energy budget, or biological behavior is part of this specification.
2. A continuous spatial wave field is the selected propagation architecture. The finite numerical representation is an implementation choice; a finite grid does not make the physical model discrete.
3. Energy is conserved across explicit holders and transfers. A spectrum, amplitude, phase, resonance match, or activation score is not itself an energy credit.
4. Local availability is local. A material or reaction cannot spend energy merely because a global pool contains it.
5. A material's response to a field does not automatically make that material an energy source.
6. Chemistry activation and reaction-energy settlement are separate contracts.
7. Correct physics and auditable accounting precede optimization. EXP-2026-008 is the matched pre-wave reference, not a requirement to preserve legacy physics.

## 2. Simulation coordinates and clock

### 2.1 Established simulation-time conventions

The existing project defines:
- 24,000 physics ticks per simulated day.
- 440 reference-tone cycles per simulated day.
- A separate 2,400-physics-tick orbital cycle for the external world-tone source.
- A localized hotspot configured at 880 cycles per simulated day.

These are existing model conventions, not yet a complete physical unit system. The 440-cycle carrier period and 2,400-tick orbital period are distinct quantities and must never be conflated. Phase must be derived deterministically from simulation time/state, never wall-clock execution time.

### 2.2 Time domains must remain separate

The specification must distinguish:
- **Physics time:** the simulation tick and any sub-tick numerical integration interval.
- **Experiment duration:** the requested simulated observation interval and its exact runner semantics.
- **Wall-clock time:** elapsed machine execution time used for performance measurement.

The Phase 0 run records actual elapsed execution and completed ticks. Its measured ticks/second is a performance result, not a physical conversion factor. It must not be used to infer wave speed or source power.

### 2.3 Physical world geometry and propagation speed

**User decision: the physical world is an infinite, flat, two-dimensional plane.** The visible rectangular viewer is only an observation window into that world. Its edges have no physical meaning and create no reflection, absorption, containment, or discontinuity.

The chemistry laboratory is a temporary experimental arrangement inside the plane:
- Laboratory walls are ordinary physical objects with explicit geometry and material properties. They contain and help track experimental subjects during controlled chemistry experiments.
- A wall affects matter, waves, and energy only through its defined physical interactions. It must not automatically block every kind of particle or energy.
- The laboratory is not a special simulation domain, and chemistry, energy propagation, or material behavior must never require laboratory walls to exist.
- Removing laboratory walls later must leave the same physical laws and simulation world in place.

The camera/observation window is nonphysical. It may pan, zoom, and follow subjects; crossing its edge changes only visibility. Objects and energy continue through that edge without any simulation event.

For the initial homogeneous empty-space model, propagation speed is universal and constant. It is expressed in world-distance units per physics-time unit. Do not infer it from the old movement cutoff, viewer dimensions, observed wall-clock throughput, or a desired benchmark result.

The physical plane is infinite even if a numerical solver uses a finite computational window. Any truncation used for computation must be handled as a numerical approximation (for example, by a validated outgoing/absorbing treatment or a sufficiently expanded domain), never as an unmodeled physical wall. Numerical truncation must not silently reflect, absorb, delete, or duplicate energy.

### 2.4 Physical unit system and clock conversion

The model must use a dimensionally consistent physical unit system. The implementation may store scaled or normalized numerical values for precision and performance, but every stored quantity must have a documented conversion to the physical quantity it represents. Normalization is a numerical representation, not permission to mix dimensions.

The canonical dimensional basis is:

| Quantity | Symbol | Physical dimension | Required interpretation |
|---|---|---|---|
| World distance | L | length | Euclidean distance on the infinite plane |
| Physics time | T | time | Simulation time, including any sub-tick solver interval |
| Energy | E | energy | One consistently defined project energy unit, with its conversion to SI energy documented before calibration |
| Power | P | E/T | Energy transfer per physics-time interval |
| Area | A | L² | Physical planar area |
| Boundary length | ℓ | L | Physical length of a source or receiver boundary |
| Frequency | ν | 1/T | Cycles per physics-time unit |
| Propagation speed | c | L/T | Physical wavefront speed in homogeneous empty space |
| Energy density | e | E/L² | Field energy per physical planar area |
| Energy flux | J | E/(L·T) | Energy crossing unit boundary length per physics time |
| Power density | q | E/(L²·T) | Distributed source-to-field power per physical area |

The physical unit scales must be explicit configuration/specification values, not inferred from screen pixels, grid-cell count, molecular size, or machine performance. The numerical grid spacing is expressed in L; it is not itself the definition of L. Camera zoom and solver refinement must not change physical distances, source power, or total energy.

Use a single physics-time coordinate for all propagation, oscillation, and energy-transfer equations. The existing convention of 24,000 ticks per simulated day and 86,400 seconds per day implies 3.6 physical seconds per tick. Treat this as the current legacy clock conversion to verify against the runner and source implementations before adopting it as the authoritative T scale. A sub-tick solver step is a smaller interval in the same T coordinate, not a separate clock. Wall-clock execution time never enters the physical equations.

The existing 440 and 880 cycles-per-simulated-day settings convert to frequency only after the simulated-day-to-T mapping is confirmed. The 2,400-tick orbital period is a separate source-position/phase schedule; it is not the carrier period. Store frequencies in cycles/T (or angular frequency in radians/T, with the convention stated) and derive phase deterministically from physics time.

For the scalar candidate in Section 4, if u is dimensionless, dimensional consistency requires:
- [c] = L/T.
- [K] = E·T²/L².
- [e] = E/L².
- [J] = E/(L·T).
- [f] = 1/T² in the forced equation.
- [K u_t f] = E/(L²·T), a power density.

If u is assigned a physical dimension instead, derive and document the corresponding dimensions of K and f; do not retain the dimensionless-u values by convention. The integrated field energy is ∫e dA, and integrated source-to-field power for a distributed source is ∫K u_t f dA. These are the dimensional reference checks for the solver and ledger interfaces.

No numerical world-length scale, joule conversion for the project energy unit, propagation-speed value, field normalization K, or source calibration is approved by this section. Those values must be selected together using explicit physical/experimental criteria and then recorded as versioned model parameters. A value must not be selected solely to reproduce legacy reaction rates or improve benchmark throughput.

## 3. Unified energy ontology

The complete energy inventory has one conservation authority and these non-overlapping categories:

| Category | Meaning | Required ownership / transfer rule |
|---|---|---|
| External source energy | Finite energy supplied by a named external source | Source identity, initial/remaining budget, emission schedule, and debit |
| Propagating field energy | Energy currently carried in the spatial field | Local field state, energy density, flux, and boundary accounting |
| Material internal energy | Energy held in material-local physical degrees of freedom | Explicit material ownership and conservative exchange rules |
| Thermal energy | Energy represented as unresolved microscopic motion or a defined thermal state | Local storage and any modeled thermal transport |
| Structural / chemical potential | Energy associated with validated material structure and bonds | Only include terms with defined parameters and experimentally defensible meaning |
| Mechanical energy / work | Energy in motion and energy transferred through forces or constraints | Equal-and-opposite work accounting across interacting holders |
| External exchange | Energy entering from or leaving to an explicitly modeled external system | Named external source/sink, signed amount, and destination; never inferred from a map edge |

A transfer record must identify its source holder, destination holder, signed amount, simulation time, and physical mechanism. Internal transfers cancel in the whole-world balance. A holder cannot be debited or credited twice for one transfer.

The global conservation equation is an accounting identity, not a mechanism:

`E_initial + E_external,in = E_source,remaining + E_field + E_material + E_thermal + E_structural + E_mechanical + E_exported + E_residual`

The remaining source budget is included because un-emitted energy remains owned by its source. The residual is a validation result and must be within a documented numerical tolerance. It is not an energy holder and must never be silently assigned to heat.

Structural/chemical potential is provisional. Existing bond-graph energy is not assumed to be complete thermodynamics. Until validated, it must be reported separately from experimentally established energy conversions rather than used to conceal missing source energy.

## 4. Candidate reference wave model

### 4.1 Candidate, not yet an approved physical law

The simplest candidate for a first reference solver is a two-dimensional scalar, linear wave field in an isotropic homogeneous empty environment:

`u_{tt} = c^2 \Delta_{\mathbb{R}^2} u`

where:
- `u(p,t)` is a signed field state at point `p` on the infinite plane, not energy and not a resonance score.
- `\Delta_{\mathbb{R}^2}` is the ordinary two-dimensional spatial Laplacian on the flat plane.
- `c` is finite propagation speed in physical surface-distance units per physics-time unit.
- Superposition follows from linearity.
- Frequency and phase are represented by the evolving field state, not by a list of independent per-material energy grants.

This is a deliberately minimal reference candidate. One real time-domain scalar field naturally carries simultaneous frequencies and their phase relationships; frequency decomposition is an analysis/query of that field, not a set of independently spendable frequency reservoirs. The model is non-dispersive in empty space and isotropic. It is valid only if those assumptions match the intended physics. If the physical model requires polarization, direction-dependent propagation, multiple coupled components, or dispersion, the equation must be expanded before solver implementation. The project must not choose the scalar form merely because it is cheaper.

### 4.2 Energy density and flux candidate

For the candidate scalar model, introduce a positive field normalization coefficient `K`. A consistent candidate energy density and flux are:

`e = (K/2) [u_t^2 + c^2 |\nabla_{\mathbb{R}^2} u|^2]`

`J = -K c^2 u_t \nabla_{\mathbb{R}^2} u`

The units of `K` must be chosen so `e` is energy per physical area and `J` has units of energy per length per physics-time. The integral of `e` over the plane is field energy. For finite-energy localized states, global conservation is evaluated over the full plane; any finite computational window must account for flux crossing its numerical boundary without treating that boundary as a physical wall.

This relationship is essential: field amplitude or spectral magnitude alone is not an energy inventory. The coefficient and units must be fixed together with the simulation's energy unit and source calibration.

### 4.3 Damping and heat

A damped candidate equation may include `2\gamma u_t`:

`u_{tt} + 2\gamma u_t = c^2 \Delta_{\mathbb{R}^2} u + f(p,t)`

For this candidate, the local energy balance is:

`\partial_t e + \nabla_{\mathbb{R}^2} \cdot J = K u_t f - 2K\gamma u_t^2`

The first right-hand term is signed power density transferred from the source or coupled degree of freedom into the field. Its spatial integral is the source-to-field power transfer. The second term is nonnegative dissipation and must be credited to local thermal energy. If the field does work back on a source, the signed transfer reverses and the source holder is credited. Source debit and field work must be committed atomically. This relation makes source energy transfer auditable instead of equating a prescribed amplitude with an energy grant.

If damping is not part of the selected physical model, set `\gamma = 0`; do not add it merely to stabilize or quiet the simulation. Damping cannot be an arbitrary attenuation multiplier that makes energy disappear.

Geometric spreading is not dissipation. A wave can have lower local amplitude as its energy spreads over a larger region while total field energy remains conserved. True material absorption and bulk damping are separate mechanisms with explicit local thermal destinations. The viewer edge is never an absorption boundary. Any finite solver-window treatment must be validated independently and included in numerical energy accounting.

### 4.4 Numerical candidate and stability

A regular Cartesian grid is a natural candidate for the flat-plane reference solver, but it is not automatically the final choice. Compare conservative finite-difference, finite-volume, or other suitable planar discretizations for isotropy, conservation, convergence, and cost. The familiar five-point-stencil CFL bound `c Δt / Δx ≤ 1/√2` is only a starting reference for the corresponding idealized stencil; derive and test the actual stability bound for the selected discrete operator, spacing, and time integrator.

A finite computational window is not the physical world boundary. Its outgoing-wave treatment must be tested for reflection error, energy flux, and convergence as the window is expanded. The numerical method must not impose laboratory containment or viewer-edge behavior on the underlying field.

Required numerical properties:
- Deterministic update and source ordering.
- Finite-value validation and fail-closed handling of invalid state.
- Explicit time step and spatial resolution.
- Measured phase and wavefront error against analytic/reference cases.
- Refinement tests for convergence.
- Energy drift tests over long empty-domain runs.
- No arbitrary propagation radius or amplitude cutoff.

## 5. Source ontology and present-code findings

### 5.1 External world-tone source

Current source configuration describes a rotating external source, 440 cycles per simulated day, a 2,400-tick orbital cycle, and a configured incident-energy intensity of 1–10 energy units per physics tick per normalized projected width.

The source configuration and identity are reusable candidates. The current per-constituent uniform settlement is not a spatial wave source and must not remain an energy authority after migration.

For a field model, source injection must become a finite power/flux boundary contract. The complete source boundary is the allocation denominator. Source energy is debited once when work is done on the field; un-emitted or rejected energy remains with the source. The existing intensity unit must be reconciled with world length and physics time before it is converted to a field boundary condition.

### 5.2 Localized hotspot

Current hotspot configuration uses a radial spatial profile, 880 cycles per simulated day, and a configured peak rate derived from 1,000 energy units per simulated second per unit realized material area. The current code converts this to 3,600 energy units per tick per unit area using its 24,000-tick / 86,400-second convention.

This is a volumetric/areal source-density contract, unlike the world-tone's projected-width rate. The two cannot be compared or merged into one source API until the physical dimensionality and unit conversion are explicit. The hotspot's location, frequency, profile, and source identity are reusable inputs; its current direct settlement into a shared reservoir is not.

### 5.3 Material-origin oscillators

The current material spectrum is calculated from material properties and a shared analytical oscillator convention. The code audit found no corresponding named finite source-energy debit for that material-origin amplitude. Therefore:

**Decision constraint:** Existing material aura/spectrum values are not allowed to inject field energy in the new system.

Phase 1 must choose and justify one of these interpretations before material-origin field emission is enabled:
1. **Powered oscillator:** a material has an explicitly owned internal-energy budget; emission debits it and any absorbed field energy is credited through a conservative transaction.
2. **Conservative coupled degree of freedom:** material and field are coupled degrees of freedom in one Hamiltonian/Lagrangian or equivalent conservative model; energy exchange is derived from the coupling equations, not from a source grant.
3. **Response only:** material properties determine absorption, reflection, transmission, scattering, or frequency response, but the material is not an emitter.

The default for the first empty-space solver is no material-origin sources. Material response is introduced only after its coupling law and energy ownership are defined.

### 5.4 Source dimensions must be reconciled

The current world-tone rate is energy/(tick·projected-width); the hotspot rate is energy/(tick·area) after conversion. These are not interchangeable. Phase 1 must decide whether each source is:
- A boundary emitter with a power-per-length or prescribed flux condition.
- A distributed source with power-per-area.
- A coupled material oscillator with explicit internal energy.

Do not silently reinterpret a source's current numerical intensity as a field amplitude. For the scalar forced-wave candidate, source power must be computed from the actual field work term (the spatial integral of `K u_t f`) or an equivalent explicitly derived boundary-flux condition. The source's finite budget is debited by the same signed transfer that the field receives; if the coupling is one-way, its missing return-work destination must be specified rather than discarded.

### 5.5 Common source-to-field energy contract

World-tone and hotspot inputs may retain different physical geometries, but both must use one source-to-field transfer contract and the same conservation authority. A common interface does not mean forcing boundary and distributed sources into the same dimensional parameter.

Every source must have:
- Stable source identity and provenance.
- A declared physical type: boundary emitter, distributed source, or explicitly coupled material degree of freedom.
- Physical location and geometry, expressed in world units.
- A deterministic time/phase schedule in physics time.
- A declared spectral/temporal waveform; frequency and phase are not energy units.
- An authoritative energy holder with a finite available budget, or an explicitly modeled external replenishment transfer with its own named origin and accounting.
- A maximum/target power or flux expressed in the correct dimensions.
- A rule for source shutoff, exhaustion, rejected work, and any work returned from the field.
- A transfer record linking source debit and field credit (or the reverse signed transfer) in the same atomic operation.

For the scalar forced-wave candidate, a distributed source f(p,t) contributes signed power density K u_t f. Its total instantaneous transfer is the spatial integral over the actual source support. The source holder is debited by exactly the field's net received work; if the net work is negative, the corresponding source/coupled holder is credited. A prescribed f is not allowed to inject energy without that matching transfer. If a source is intended to deliver a prescribed positive power independent of instantaneous field phase, the coupling must be formulated as a passive/active boundary or controlled-source law whose work and energy budget are explicitly derived; it must not overwrite the field state or add an arbitrary energy amount.

For a boundary emitter, specify the boundary condition and its signed energy flux using the selected field equation's conservation law. Integrate over the complete physical emitting boundary length. A prescribed boundary amplitude is not a substitute for a power/flux contract. The field's net energy gain must equal the source's net debit, including any reflected work returned to the source.

For a distributed emitter, specify the source support in physical area and power density in E/(L²·T). Integrate over the complete support using physical area. Do not multiply by the number of grid cells without the corresponding cell-area factor; refinement must not change total source power.

The present world-tone input is provisionally classified as an external boundary/incident-field source because its legacy rate is stated per normalized projected width. Its exact boundary condition, physical length measure, source budget, and conversion to power per length remain open. The present hotspot input is provisionally classified as a distributed external source because its legacy rate is stated per realized area. Its support profile, physical area measure, finite budget/replenishment, and conversion to power density remain open. These classifications are hypotheses to audit against source code and intended physics, not authorization to preserve the legacy settlement algorithms.

The 1–10 energy-units-per-tick-per-normalized-width world-tone setting and the 1,000 energy-units-per-simulated-second-per-area hotspot setting are legacy configuration values, not directly comparable physical quantities. Convert each through the verified tick/time and world-length/area scales before comparing or combining them. Never treat either value as a field amplitude. Source allocation must be independent of solver resolution, viewer size, camera zoom, and the number of receivers.

Material-origin emission is a distinct case. The initial empty-space reference solver has no material emitters. When material coupling is introduced, select and implement either (a) a powered oscillator that debits a named material-internal energy holder, or (b) a conservative coupled field/material degree of freedom with a derived shared energy function. Passive response may absorb, reflect, transmit, or scatter incident field energy without becoming a source. Existing aura/spectrum calculations alone authorize neither emission nor an energy credit.

## 6. Material coupling contract

The field and material layers remain separate authorities.

A receiver query must provide:
- Stable material identity and geometry revision.
- Complete realized shape and exposed physical boundary.
- Material response parameters with explicit units and frequency dependence.
- Boundary orientation and the visible/exposed intervals used for coupling.

The field returns:
- Local field state and phase at requested boundary locations.
- Incident energy flux over the boundary interval.
- A staged interaction result separating absorbed, reflected, transmitted, and scattered energy.
- Any energy transferred into material internal or thermal holders.

The coupling law must ensure that multiple receivers cannot each claim the same finite field energy. For the first reference solver there are no receivers; matter coupling is a later phase. Absorption and reflection coefficients must be bounded and energy-consistent. Unmodeled or unsupported geometry fails closed; it does not fall back to centroids or bounding circles.

Chemistry activation remains dimensionless influence only. A local field query may change an activation probability or eligibility rule only when that behavior is separately specified and validated. It does not itself fund BREAK, COMBINE, or displacement.

## 7. Infinite-plane geometry, laboratory containment, and observation

**Resolved physical decision:** The physical environment is an infinite, flat, two-dimensional plane. It has no global physical edge, enclosing sphere, periodic seam, or environmental wall.

Physical requirements:
- Physical position and distance use ordinary Euclidean two-dimensional coordinates and distance.
- The governing empty-space spatial operator is the flat-plane Laplacian; field gradients and energy flux lie in the plane.
- Field energy is integrated over physical area. Source and receiver integration use actual physical area or physical boundary length, never projected screen pixels.
- Waves and energy continue through the observation-window edge. The camera does not create a boundary condition.
- Matter also continues through the observation-window edge. Leaving the visible screen is not deletion, despawning, wrapping, or a collision.
- A chemistry laboratory is a temporary arrangement of explicit solid wall objects placed in the world to contain and track experimental subjects.
- Laboratory walls participate through ordinary geometry and material-interaction contracts. They may constrain or block particular matter and may reflect, absorb, transmit, or otherwise interact with waves according to their material properties; no universal wall behavior is presumed.
- Laboratory walls are not a prerequisite for chemistry or field propagation. They can be removed without changing the governing physics or the identity of the world.
- Future populated-world operation uses the same plane and physical laws with laboratory containment absent or selectively deployed.

Numerical implications:
- A finite solver window, storage tile, or viewer rectangle is an implementation/observation limit, not a physical boundary.
- The solver must use a validated outgoing-wave, absorbing numerical treatment, domain expansion, or another convergence-demonstrated method for computational truncation. Numerical truncation must not introduce unaccounted reflection, absorption, deletion, or duplication.
- Source injection, field propagation, and material coupling use the same flat-plane metric and physical measures.
- Numerical and visualization coordinates must preserve continuity across storage-tile and camera-window boundaries.

**Wave and wall interaction:** There are no global environmental edge conditions. Reflection, absorption, transmission, scattering, and occlusion occur only through explicit physical interactions with materials and objects. The laboratory is a convenient controlled arrangement of those objects, not a separate physics mode.

## 8. Single conservation authority and transaction contracts

Reuse the current ledger's engineering patterns where valid: finite-value checks, independent inventory reconciliation, staged transactions, provenance, rollback, and deterministic commit ordering.

Do not preserve current categories or holder semantics merely because the data structures exist. The redesigned authority must inventory the complete ontology in Section 3.

Minimum atomic operations:
1. **Source emission:** source debit and field credit commit together.
2. **Field propagation:** internal redistribution changes local field state but not total field energy except for explicitly modeled work/loss.
3. **Field-to-material coupling:** field debit and material credit commit together.
4. **Dissipation:** field debit and local thermal credit commit together.
5. **Mechanical exchange:** field/material work and mechanical-energy changes reconcile as one transaction.
6. **External exchange:** energy exchanged with an explicitly modeled external system is recorded with its source/destination and signed amount. The infinite physical plane has no global environmental edge; flux through a finite computational window is a numerical-domain accounting issue, not physical escape.

Rejected or invalid transactions must leave all participating holders unchanged. The field must not create a second ledger or a parallel energy authority.

## 9. Performance constraints from EXP-2026-008

The pre-wave reference used five independent seeds (42–46), 60-second experiment runs, 1,280 initial materials, movement profiling, and sampled tick profiling.

The measured pooled sampled-tick mean was approximately 159.45 ms. The resonance-movement phase averaged approximately 111.47 ms (about 70% of sampled tick time). Its subprofile split was approximately 52.18 ms resonance calculation and 57.26 ms placement validation.

These measurements are instrumented baseline observations, not a universal cost model. Future comparisons must match source commit, workload, build mode, seed set, instrumentation, and experiment semantics. The new field must be profiled by propagation, source injection, geometry/boundary coupling, material queries, and accounting separately. The initial correct reference solver may be slower; optimizations must be checked against it and cannot use arbitrary cutoffs, dropped amplitudes, frequency collapse, or weaker physics.

## 10. Phase 1 decision register

| Topic | Current status | Required resolution before Phase 1 exit |
|---|---|---|
| Unified energy categories | Defined provisionally in Section 3 | Confirm exact holder boundaries and initial/final inventory equation |
| Field interpretation | User-selected coupled physical field | Confirm scalar/isotropic reference model is sufficient; expand only if physics requires it |
| Frequency representation | User-selected single field containing multiple frequencies and phases | Validate superposition/interference and frequency-resolved response |
| Material emission/response | User-selected powered emission plus passive response | Define material energy ownership and conservative emission/coupling mechanism |
| Empty-space propagation speed | User-selected universal constant in homogeneous empty space | Calibrate value and physical units; do not infer from legacy movement behavior |
| Energy density and flux | Candidate formulas in Section 4.2 | Fix field normalization and energy units |
| Spatial unit and propagation speed | Open | Define world-unit meaning, speed, and tick/substep relationship |
| Source time units | Existing conventions identified | Separate physics time from experiment/wall time; calibrate source power |
| World-tone source | Existing source parameters identified | Convert width-rate to a physically dimensioned source boundary contract |
| Hotspot source | Existing area-rate identified | Define distributed source coupling and unit calibration |
| Material-origin oscillators | No current source debit identified | Choose powered oscillator, conservative coupling, or response-only |
| Material coupling | Interface requirements drafted | Define constitutive response and finite-energy sharing |
| Damping / thermal state | Energy destination requirement defined | Define thermal holder, damping law, and local heat transport |
| Structural/chemical energy | Explicitly provisional | Validate parameters or keep separate from settled physical energy |
| World geometry | User-resolved: infinite flat two-dimensional plane; viewer is a nonphysical observation window | Define world-distance units, planar numerical representation, and geometry tests |
| Environmental wave boundaries | Resolved: none globally; laboratory walls are ordinary physical objects | Validate finite-window numerical treatment and material-specific wall/interface behavior separately |
| Numerics | Infinite-plane reference representation required; planar discretization remains open | Compare discretizations; derive stability bound, precision, isotropy, finite-window error, and convergence tolerances |
| Determinism | Required | Define ordering and deterministic parallelism contract |

## 11. Phase 1 exit gate

Phase 1 is complete only when:
- Infinite flat-plane geometry, world-distance units, and the numerical representation are explicit and tested; camera and storage-window artifacts do not affect physics.
- The field variable, governing equation, dimensionality, units, energy density, and flux are explicit and dimensionally consistent.
- Propagation speed and simulation-time relationship are defined independently of wall-clock performance.
- World-tone, hotspot, and material-origin source semantics have named ownership and dimensionally consistent source contracts.
- Material coupling, attenuation, heat, structural-energy limits, and the absence of environmental edge boundaries are explicit.
- The numerical method has a derived stability condition, convergence plan, deterministic update order, and documented error tolerances.
- Atomic transfer/rollback contracts cover every energy holder.
- The design is reviewed against the user's approved infinite-plane/laboratory/camera separation and unified-energy decisions, and the EXP-2026-008 baseline.
- No production solver or live chemistry/movement behavior has been changed as part of this specification phase.

**Next action:** compare conservative planar discretization options, define world-distance and physical energy units, validate finite computational-window treatment, and complete source-dimension and powered-material-oscillator contracts. Then lock the plane-wave equation and numerical reference method before Phase 2 implements an independent empty-space solver.
