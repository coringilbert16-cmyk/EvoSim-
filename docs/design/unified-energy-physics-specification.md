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

### 2.3 Spatial units and propagation speed remain unresolved

The world has numeric coordinates and cell dimensions, but Phase 1 must state what one world unit means physically within the model and how that relates to one physics tick. Propagation speed must then be expressed in world-units per physics-time unit.

No wave speed may be selected from the old movement cutoff, the grid cell size, observed wall-clock throughput, or a desired benchmark result.

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
| Boundary transfer | Energy leaving the modeled domain or supplied by an external boundary | Explicit escaped/exported or imported amount and destination |

A transfer record must identify its source holder, destination holder, signed amount, simulation time, and physical mechanism. Internal transfers cancel in the whole-world balance. A holder cannot be debited or credited twice for one transfer.

The global conservation equation is an accounting identity, not a mechanism:

`E_initial + E_external,in = E_source,remaining + E_field + E_material + E_thermal + E_structural + E_mechanical + E_exported + E_residual`

The remaining source budget is included because un-emitted energy remains owned by its source. The residual is a validation result and must be within a documented numerical tolerance. It is not an energy holder and must never be silently assigned to heat.

Structural/chemical potential is provisional. Existing bond-graph energy is not assumed to be complete thermodynamics. Until validated, it must be reported separately from experimentally established energy conversions rather than used to conceal missing source energy.

## 4. Candidate reference wave model

### 4.1 Candidate, not yet an approved physical law

The simplest candidate for a first reference solver is a two-dimensional scalar, linear wave field in an isotropic homogeneous empty environment:

`u_{tt} = c^2 \nabla^2 u`

where:
- `u(x,y,t)` is a signed field state, not energy and not a resonance score.
- `c` is finite propagation speed in world-units per physics-time unit.
- Superposition follows from linearity.
- Frequency and phase are represented by the evolving field state, not by a list of independent per-material energy grants.

This is a deliberately minimal reference candidate. It is valid only if the intended field is scalar and isotropic in the empty medium. If the physical model requires polarization, direction-dependent propagation, multiple coupled components, or a dispersive medium, the equation must be expanded before solver implementation. The project must not choose the scalar form merely because it is cheaper.

### 4.2 Energy density and flux candidate

For the candidate scalar model, introduce a positive field normalization coefficient `K`. A consistent candidate energy density and flux are:

`e = (K/2) [u_t^2 + c^2 |\nabla u|^2]`

`J = -K c^2 u_t \nabla u`

The units of `K` must be chosen so `e` is energy per world-area and `J` is energy per world-length per physics-time. The integral of `e` over the domain is field energy. The flux integral through a boundary is the energy transfer across that boundary.

This relationship is essential: field amplitude or spectral magnitude alone is not an energy inventory. The coefficient and units must be fixed together with the simulation's energy unit and source calibration.

### 4.3 Damping and heat

A damped candidate equation may include `2\gamma u_t`. If used, the associated reduction in field energy must be derived from the equation and deposited into the local thermal holder exactly once. Damping cannot be an arbitrary attenuation multiplier that makes energy disappear.

Geometric spreading is not dissipation. A wave can have lower local amplitude as its energy spreads over a larger region while total field energy remains conserved. True material absorption, bulk damping, and boundary absorption are separate mechanisms with separate heat or export destinations.

### 4.4 Numerical candidate and stability

A uniform finite-difference grid is acceptable for the independent reference solver because it is simple to test against known solutions. It is not a physical law and must not be treated as the final resolution.

For a standard two-dimensional five-point spatial stencil with an explicit second-order time update, the solver must enforce and test the applicable CFL stability bound. In the equal-spacing case, the usual bound is `c Δt / Δx ≤ 1/√2`. The actual bound must be re-derived for the selected stencil, spacing, and integration method before implementation.

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

Do not silently reinterpret a source's current numerical intensity as a field amplitude.

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

## 7. World topology and wave boundaries

The intended world topology recorded for EvoSim is horizontal wrap with hard vertical top and bottom boundaries. This is distinct from the current material-grid lookup behavior in `src/simulation/environment.rs`, which bounds x and wraps y. The source implementation therefore conflicts with the intended axis convention.

**Required blocker:** Verify the exact Master Technical Specification v2.4 coordinate convention, then reconcile the material-grid axis implementation and tests before selecting field boundary conditions. Do not build the wave solver on the reversed convention.

Physical topology and wave boundary conditions are separate decisions:
- Horizontal wrap implies periodic continuation of the field across the horizontal seam if the wave domain shares the world topology.
- A hard vertical physical wall does not, by itself, specify the wave condition. The field may reflect, absorb, or use a justified mixed boundary rule. This requires an explicit physical decision and tests.
- Any boundary absorption must debit field energy and credit heat/export exactly once.
- Periodic boundaries must preserve flux and energy across the seam without duplicate accounting.

## 8. Single conservation authority and transaction contracts

Reuse the current ledger's engineering patterns where valid: finite-value checks, independent inventory reconciliation, staged transactions, provenance, rollback, and deterministic commit ordering.

Do not preserve current categories or holder semantics merely because the data structures exist. The redesigned authority must inventory the complete ontology in Section 3.

Minimum atomic operations:
1. **Source emission:** source debit and field credit commit together.
2. **Field propagation:** internal redistribution changes local field state but not total field energy except for explicitly modeled work/loss.
3. **Field-to-material coupling:** field debit and material credit commit together.
4. **Dissipation:** field debit and local thermal credit commit together.
5. **Mechanical exchange:** field/material work and mechanical-energy changes reconcile as one transaction.
6. **Boundary transfer:** exported energy is removed from the domain and recorded as export; incoming boundary energy is recorded as external input.

Rejected or invalid transactions must leave all participating holders unchanged. The field must not create a second ledger or a parallel energy authority.

## 9. Performance constraints from EXP-2026-008

The pre-wave reference used five independent seeds (42–46), 60-second experiment runs, 1,280 initial materials, movement profiling, and sampled tick profiling.

The measured pooled sampled-tick mean was approximately 159.45 ms. The resonance-movement phase averaged approximately 111.47 ms (about 70% of sampled tick time). Its subprofile split was approximately 52.18 ms resonance calculation and 57.26 ms placement validation.

These measurements are instrumented baseline observations, not a universal cost model. Future comparisons must match source commit, workload, build mode, seed set, instrumentation, and experiment semantics. The new field must be profiled by propagation, source injection, geometry/boundary coupling, material queries, and accounting separately. The initial correct reference solver may be slower; optimizations must be checked against it and cannot use arbitrary cutoffs, dropped amplitudes, frequency collapse, or weaker physics.

## 10. Phase 1 decision register

| Topic | Current status | Required resolution before Phase 1 exit |
|---|---|---|
| Unified energy categories | Defined provisionally in Section 3 | Confirm exact holder boundaries and initial/final inventory equation |
| Field variable and equation | Scalar linear wave equation proposed as reference candidate | Confirm scalar/isotropic assumptions or select a richer model |
| Energy density and flux | Candidate formulas in Section 4.2 | Fix field normalization and energy units |
| Spatial unit and propagation speed | Open | Define world-unit meaning, speed, and tick/substep relationship |
| Source time units | Existing conventions identified | Separate physics time from experiment/wall time; calibrate source power |
| World-tone source | Existing source parameters identified | Convert width-rate to a physically dimensioned source boundary contract |
| Hotspot source | Existing area-rate identified | Define distributed source coupling and unit calibration |
| Material-origin oscillators | No current source debit identified | Choose powered oscillator, conservative coupling, or response-only |
| Material coupling | Interface requirements drafted | Define constitutive response and finite-energy sharing |
| Damping / thermal state | Energy destination requirement defined | Define thermal holder, damping law, and local heat transport |
| Structural/chemical energy | Explicitly provisional | Validate parameters or keep separate from settled physical energy |
| World topology | Intended horizontal wrap / vertical hard walls; code appears reversed | Verify Master Spec 2.4 and correct/test coordinate convention |
| Wave boundary conditions | Horizontal periodic candidate; vertical open | Decide reflection/absorption/mixed vertical boundary and account for it |
| Numerics | Uniform-grid reference candidate | Select stencil/integrator, exact stability bound, precision, and convergence tolerance |
| Determinism | Required | Define ordering and deterministic parallelism contract |

## 11. Phase 1 exit gate

Phase 1 is complete only when:
- The exact Master Spec 2.4 coordinate convention has been checked and any source mismatch resolved.
- The field variable, governing equation, dimensionality, units, energy density, and flux are explicit and dimensionally consistent.
- Propagation speed and simulation-time relationship are defined independently of wall-clock performance.
- World-tone, hotspot, and material-origin source semantics have named ownership and dimensionally consistent source contracts.
- Material coupling, attenuation, heat, structural-energy limits, and boundary behavior are explicit.
- The numerical method has a derived stability condition, convergence plan, deterministic update order, and documented error tolerances.
- Atomic transfer/rollback contracts cover every energy holder.
- The design is reviewed against Master Spec 2.4 and the EXP-2026-008 baseline.
- No production solver or live chemistry/movement behavior has been changed as part of this specification phase.

**Next action:** resolve the Master Spec/topology discrepancy and settle the field's physical interpretation and units. Then lock the scalar-wave candidate (or replace it with a justified richer model), source dimensions, and material-oscillator semantics in a reviewed decision record. Only after those decisions should Phase 2 implement an independent empty-space solver.
