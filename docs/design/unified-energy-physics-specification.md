# Phase 1 — Unified Energy and Continuous Wave-Field Specification

**Status:** Phase 1 specification complete. This document defines the approved reference physics and implementation-independent contracts. It does not authorize production implementation; Phase 2 begins with an independent reference solver.

**Controlling roadmap:** [Resonance Field — Formal Implementation Roadmap](resonance-field-implementation-roadmap.md)  

> **Current-decision notice (supersedes conflicting source assumptions below):** This Phase 1 document remains the reference for the validated wave equation, field energy/flux, project-native units, numerical method, and conservation principles. Its proposed Sun/orbit, hotspot-injector, and powered-material-oscillator source models are historical candidates and are not part of the active model. The later approved model is a persistent, spatially uniform ambient energy background held by the resonance field: 24,000 ticks per day, 440 ambient-carrier cycles per day, and a cycle-averaged energy-density envelope of 10 → 1 → 10. Opening energy is initial inventory; positive change is world generation and negative change is world withdrawal. No Sun object, external injector, hotspot injector, powered material oscillator, or additional buffer/reservoir is active. See the controlling roadmap for the current implementation sequence.

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

The approved project-time convention is:
- **24,000 physics ticks = one simulated day.**
- **24,000 physics ticks = one complete solar orbital period.** One orbit and one simulated day are the same interval, not independent cycles.
- **440 carrier cycles per simulated day/orbit** for the reference sun carrier.
- **880 cycles per simulated day/orbit** for the localized hotspot carrier.

The current source audit found a legacy 2,400-tick orbital setting in the implementation. That value is inconsistent with the approved model and is a known implementation discrepancy, not an alternate physical convention. Phase and orbital position are derived deterministically from the logical simulation tick, never wall-clock execution time.

### 2.2 Time domains must remain separate

The specification must distinguish:
- **Physics time:** the simulation tick and any sub-tick numerical integration interval.
- **Experiment duration:** a requested wall-clock execution interval for controlled workload runs (the standard run is 60 real seconds); record the logical ticks completed. Wall-clock duration never defines a simulated time interval.
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

For the initial homogeneous empty-space model, propagation speed is universal and constant. It is expressed in project-native world-distance units per simulation tick. Do not infer it from the old movement cutoff, viewer dimensions, observed wall-clock throughput, or a desired benchmark result.

The physical plane is infinite even if a numerical solver uses a finite computational window. Any truncation used for computation must be handled as a numerical approximation (for example, by a validated outgoing/absorbing treatment or a sufficiently expanded domain), never as an unmodeled physical wall. Numerical truncation must not silently reflect, absorb, delete, or duplicate energy.

### 2.4 Project-native units and logical simulation time

Use a dimensionally consistent **project-native** unit system. No conversion from simulation time to wall-clock seconds or SI seconds is required or intended. No world-distance-to-meters or project-energy-to-joules conversion is required unless a future, separately approved goal explicitly calls for external physical correspondence.

| Quantity | Symbol | Project-native dimension | Required interpretation |
|---|---|---|---|
| World distance | L | world-distance unit | Euclidean distance on the infinite plane |
| Simulation time | T | tick | Logical simulation time; one tick is the base time unit |
| Energy | E | project energy unit | One consistently defined project energy unit |
| Power | P | E/tick | Energy transfer per simulation tick |
| Area | A | L² | Physical planar area in world units |
| Boundary length | ℓ | L | Physical length of a source or receiver boundary |
| Frequency | ν | cycles/tick | Oscillation cycles per logical simulation tick |
| Propagation speed | c | L/tick | Wavefront distance per simulation tick |
| Energy density | e | E/L² | Field energy per planar area |
| Energy flux | J | E/(L·tick) | Energy crossing unit boundary length per tick |
| Power density | q | E/(L²·tick) | Distributed source-to-field power per physical area |

Simulation time is an internal logical coordinate, not elapsed machine time, SI seconds, or an external clock. Runtime pacing settings such as ticks_per_second control only how quickly the program executes; they must never affect phase, orbital position, propagation, source power, reaction eligibility, or any simulated physical behavior. Wall-clock performance remains separate metadata.

One simulated day and one complete solar orbit are exactly 24,000 ticks. The sun carrier completes 440 cycles and the hotspot carrier 880 cycles during that same interval. Their tick-domain frequencies are 440/24,000 and 880/24,000 cycles per tick. The sun orbital phase completes one cycle over those same 24,000 ticks. A sub-tick solver interval, if used, is a fraction of a tick and does not introduce a second clock.

For dimensionless u, dimensional consistency requires [c]=L/tick, [K]=E·tick²/L², [e]=E/L², [J]=E/(L·tick), [f]=1/tick², and [K u_t f]=E/(L²·tick). If u has another project-native dimension, derive the corresponding dimensions of K and f. Integrated field energy is ∫e dA; integrated source-to-field power is ∫K u_t f dA.

Project-native values for propagation speed, field normalization K, source power, hotspot radius, material coefficients, grid spacing, and solver step remain open calibration parameters. Select and version them through dimensional consistency, source ownership, conservation, controlled reference experiments, stability, and convergence. Never derive them from wall-clock throughput, legacy reaction rates, or an assumed seconds-per-tick mapping.

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

### 4.1 Approved initial reference model

The approved initial reference model is a two-dimensional scalar, linear wave field in an isotropic homogeneous empty environment:

`u_{tt} = c^2 \Delta_{\mathbb{R}^2} u`

where:
- `u(p,t)` is a signed field state at point `p` on the infinite plane, not energy and not a resonance score.
- `\Delta_{\mathbb{R}^2}` is the ordinary two-dimensional spatial Laplacian on the flat plane.
- `c` is finite propagation speed in project-native world-distance units per simulation tick.
- Superposition follows from linearity.
- Frequency and phase are represented by the evolving field state, not by a list of independent per-material energy grants.

This is the deliberately minimal approved reference model for Phase 1 and the independent reference solver. It is a modeling default, not a claim that scalar physics is universally sufficient. One real time-domain scalar field naturally carries simultaneous frequencies and their phase relationships; frequency decomposition is an analysis/query of that field, not a set of independently spendable frequency reservoirs. The model is non-dispersive in empty space and isotropic. It is valid only if those assumptions match the intended physics. If the physical model requires polarization, direction-dependent propagation, multiple coupled components, or dispersion, the equation must be expanded before solver implementation. The project must not choose the scalar form merely because it is cheaper.

### 4.2 Approved reference-model energy density and flux

For the approved scalar reference model, introduce a positive field normalization coefficient `K`. The selected energy density and flux relations are:

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

The approved empty-space default is zero damping (`\gamma = 0`). Do not add damping merely to stabilize or quiet the simulation. Material absorption/dissipation is a separate constitutive interaction and must credit explicit local thermal storage.

Geometric spreading is not dissipation. A wave can have lower local amplitude as its energy spreads over a larger region while total field energy remains conserved. True material absorption and bulk damping are separate mechanisms with explicit local thermal destinations. The viewer edge is never an absorption boundary. Any finite solver-window treatment must be validated independently and included in numerical energy accounting.

### 4.4 Numerical candidate and stability

The approved initial reference representation is a uniform Cartesian grid with a conservative second-order wave solver. Use a second-order leapfrog integrator or an equivalent method demonstrated to preserve the discrete energy balance. Derive the stability bound for the actual stencil and time integrator; the familiar five-point-stencil CFL bound `c Δt / Δx ≤ 1/√2` is a reference check, not a substitute for that derivation. Initially target approximately 16 grid samples across the shortest modeled wavelength, then verify phase accuracy, isotropy, and convergence under refinement. For the selected five-point Cartesian Laplacian with leapfrog stepping on a square grid, the von Neumann condition is `c Δt / Δx ≤ 1/√2`: the largest discrete Laplacian eigenvalue magnitude is 8/Δx², and the leapfrog stability requirement is `c² Δt² (8/Δx²) ≤ 4`. Use a strict margin below this limit in normal runs and test both sides of the limit. If the stencil, grid, or integrator changes, re-derive the bound for that actual scheme.

A finite computational window is not the physical world boundary. Its outgoing-wave treatment must be tested for reflection error, energy flux, and convergence as the window is expanded. The numerical method must not impose laboratory containment or viewer-edge behavior on the underlying field.

Required numerical properties:
- Deterministic update and source ordering.
- Finite-value validation and fail-closed handling of invalid state.
- Explicit time step and spatial resolution.
- Measured phase and wavefront error against analytic/reference cases.
- Refinement tests for convergence.
- Energy drift tests over long empty-domain runs.
- No arbitrary propagation radius or amplitude cutoff.

For acceptance, run paired spatial/time refinements (at minimum, halve both Δx and Δt while remaining inside the derived stability bound). Measure phase error, wavefront-position error, isotropy error, and normalized energy-balance residual against analytic/reference solutions. The observed error must decrease at the expected second-order rate in the asymptotic refinement regime for the selected second-order scheme. Record the numeric pass tolerances in each Phase 2 experiment setup before running it; they are numerical acceptance parameters, not physical constants. Test the derived stability limit from below and above, and fail the run if the stable-side solution grows without bound or violates its declared energy tolerance.

## 5. Source ontology and source-transfer contracts

### 5.1 External world-tone source (sun)

The sun is an external isotropic three-dimensional radiative source outside the infinite flat world plane. It travels on a vertical circular orbit in a plane perpendicular to the world plane. Its orbit is anchored by coordinate convention at the world origin and offset above the plane by more than its radius. The orbital period is exactly 24,000 ticks, the same interval as one simulated day. The emitted reference carrier completes 440 cycles during that same day/orbit; carrier phase and orbital position are distinct state variables derived from the same logical tick.

Let the world plane be z=0 and choose the orbit plane as x-z by coordinate convention. A parameterized source position is S(t)=(R cos θ(t), 0, H+R sin θ(t)), where θ(t)=2π(t mod 24,000)/24,000 and H>R>0. R and H are project-native world distances and remain model parameters. This orbit is independent of camera position, viewport, solver window, and laboratory walls.

For isotropic emitted power P_s(t_emit) in E/tick, planar point p=(x,y,0), source-to-point distance d=|p-S(t_emit)|, and source height h_s=H+R sin θ(t_emit), incident power density before field-coupling normalization is:

q_s(p,t) = P_s(t_emit) h_s / (4π d³)

Retarded emission time satisfies t-t_emit=d/c and uses the source position at emission. This is the inverse-square isotropic flux multiplied by the incidence projection h_s/d. The integral over the infinite plane is exactly P_s/2. The other half remains in the external radiation-accounting domain; it is not a tunable efficiency factor.

The mapping of intercepted radiation into the planar scalar field must be conservative. For the forced-wave model, signed field work density is K u_t f. Over each accounting interval, the integral of that work over area and ticks must equal the energy credited to the field. The source debit equals total emitted energy, partitioned into the planar-intercepted and non-intercepted portions without double counting. Retarded-time evaluation, field injection, source debit, and external-domain accounting are one deterministic transaction. A controlled finite-budget test source in Phase 2 must demonstrate this accounting before the live sun source is connected.

The sun's finite energy budget or explicitly accounted replenishment origin, orbit radius/height, emitted power, and coupling normalization remain project-native calibration parameters. The legacy per-constituent projected-width rate is retired; it is not converted into field amplitude or preserved as a recipient grant.

### 5.2 Localized hotspot

The hotspot is an in-plane distributed source with a radial cosine profile as the initial approved candidate. Define a compact-support radial profile g(r), zero outside support radius R_h and normalized so its integral over planar area is one. Its source density is:

q_h(p,t) = P_h(t) g(|p-p_h|)

where P_h is total source power in E/tick. Integrated hotspot power must therefore be independent of grid spacing, cell count, molecule count, receiver count, and camera scale. The source must own a finite energy budget or receive an explicitly accounted replenishment transfer. Its power, support radius, and replenishment schedule are project-native parameters. The legacy 1,000-energy-units-per-simulated-second-per-area rate and its 3,600-per-tick conversion are retired; no seconds-per-tick conversion is used.

### 5.3 Material-origin oscillators

Material-origin emitters are powered oscillators. Each oscillator owns an explicit internal-energy state/budget. Emitted field work is debited from that state in the same atomic transaction that credits the field. Absorbed field energy may replenish the oscillator only through a separately defined conservative coupling transaction. An equivalent conservative coupled-degree-of-freedom formulation is acceptable only if it preserves explicit ownership and the same total-energy balance.

Existing material aura/spectrum values alone never authorize emission or energy credit. Material emitters are disabled in the Phase 2 empty-space reference solver. The reference solver uses only a controlled mathematical test source with an explicit finite project-energy budget.

### 5.4 Source classes and dimensions

The approved source/receiver classes are:
- **External radiative source:** source outside the plane; the sun is this class.
- **Distributed in-plane source:** source with physical support area and power density; the hotspot is this class.
- **Boundary/line emitter:** only a source whose actual physical geometry is a line/boundary, with power per physical length or a derived boundary-flux condition.
- **Powered material oscillator:** material-owned internal energy coupled conservatively to the field.
- **Passive material response:** receiver that absorbs, reflects, transmits, or scatters without independently emitting energy.

All powers use E/tick. Distributed power density uses E/(L²·tick). Boundary power density uses E/(L·tick) when that is the selected boundary representation. Do not reinterpret legacy source intensity as field amplitude. A common source-to-field accounting interface does not erase the different geometries or dimensions of these source classes.

### 5.5 Common source-to-field energy contract

Every source must have:
- Stable identity and provenance.
- Declared physical type, location, geometry, and support.
- Deterministic schedule and waveform/phase expressed in logical ticks.
- An authoritative finite energy holder or explicitly accounted replenishment origin.
- Target/maximum power in the correct project-native dimensions.
- Explicit exhaustion, shutoff, rejected-work, and returned-work behavior.
- An atomic transfer record linking source debit and field credit (or reverse signed work).

For the scalar forced-wave candidate, a distributed forcing f(p,t) contributes signed power density K u_t f. Its instantaneous transfer is integrated over the actual physical support. The source holder is debited by exactly the field's net received work; negative net work credits the source/coupled holder. A prescribed forcing term cannot inject energy without the matching source transfer. A prescribed-power source independent of instantaneous field phase requires a derived active-source law whose work and energy budget are explicit; it cannot overwrite field state or add arbitrary energy.

A true boundary/line emitter requires a boundary condition and signed energy-flux contract derived from the selected field equation. Integrate over its complete physical boundary length. A prescribed boundary amplitude is not a power contract. This does not describe the sun.

For the sun, use the three-dimensional source-to-plane geometry and retarded-time contract in Section 5.1. For the hotspot, integrate the normalized power density over its full support area. In both cases, total source power must remain invariant under grid refinement and independent of receiver count and viewer dimensions.

### 5.6 Audit of legacy source implementations

The following are source-audit findings, not approved physical parameters:
- **World tone:** the legacy implementation uses 1–10 energy units per physics tick per normalized projected width, a 440-cycles-per-day carrier, and a 2,400-tick orbital schedule. The 2,400-tick orbit conflicts with the approved 24,000-tick day/orbit. The cached source samples do not own/debit a finite source reserve, and the production settlement grants normalized projected width per material constituent, so total input scales with recipient count. This path is not spatial wave propagation and must be retired from the authoritative physics.
- **Hotspot:** the legacy implementation uses a radial cosine profile and a peak rate expressed per legacy simulated second per realized material area. Its 3,600-energy-units-per-tick conversion depends on the obsolete 24,000-tick/86,400-second mapping. It settles against material placements into the legacy resonance reservoir rather than propagating a spatial field or debiting a named finite source budget. These rate and settlement semantics are retired.
- **Clock:** the approved logical clock is 24,000 ticks per simulated day and per solar orbit. The sun carrier completes 440 cycles and the hotspot carrier 880 cycles in that interval. The legacy 2,400-tick orbit and 86,400-second conversion are implementation artifacts, not model choices. Runtime pacing has no physical meaning.
- **Infinite-plane incompatibility:** the legacy sun orbit is derived from finite rectangular environment dimensions. That orbit and its finite-world visibility assumptions must not be carried into the infinite-plane model.

## 6. Material coupling contract

The field and material layers remain separate authorities.

A receiver query must provide:
- Stable material identity and geometry revision.
- Complete realized shape and exposed physical boundary.
- Material response parameters with explicit units and frequency dependence.
- Boundary orientation and the visible/exposed intervals used for coupling.

The field returns:
- Local field state and phase at requested boundary locations.
- Incident energy flux over each boundary interval.
- A staged interaction result separating absorbed, reflected, transmitted, and scattered energy.
- Any energy transferred into material internal or thermal holders.

Material constitutive properties determine frequency-dependent coupling and response. Resonance is a response characteristic, not an energy source. For each local field/material interaction, incident field energy is partitioned into nonnegative absorbed, reflected, transmitted, and scattered portions whose sum equals the incident amount within the declared numerical tolerance. Reflected, transmitted, and scattered portions remain in the field and are not credited a second time; absorbed energy is credited exactly once to an explicitly selected material-internal or thermal holder. Any conversion from material-internal energy to heat is a separate accounted transfer. Powered material oscillators may emit only by debiting their owned internal-energy state by the exact signed field work in the same atomic transaction. Multiple receivers must share the same finite incident field energy; no receiver may independently claim overlapping flux.

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
| Unified energy categories | Defined in Section 3 with one conservation authority | Holder categories and inventory equation are fixed at specification level; implementation must map every holder without duplication |
| Field interpretation | Approved initial scalar, linear, isotropic, nondispersive reference model | Phase 2 validates against analytic/reference cases; expand only if evidence requires it |
| Frequency representation | One time-domain field carries simultaneous frequencies and phases | Phase 2 tests superposition, interference, and frequency-resolved behavior |
| Material emission/response | Powered oscillators with owned internal energy; passive absorption/reflection/transmission/scattering | Numeric coefficients are Phase 3 calibration values; ownership and atomic transfer rules are specified |
| Empty-space propagation speed | Universal constant in homogeneous empty space, expressed L/tick | Select a project-native value in Phase 2 using controlled propagation tests; never use wall-clock or legacy movement calibration |
| Energy density and flux | Approved scalar-reference formulas in Section 4.2 | Select project-native field normalization in Phase 2 and verify measured energy/flux closure |
| Dimensional basis | Project-native L, tick, E approved | Keep all equations in project-native L, tick, and E; no SI or wall-clock conversion is required |
| Scalar field dimensions | Dimensionless u is the initial reference convention | Preserve the documented project-native dimensions for K and forcing in the solver |
| Spatial unit and propagation speed | L and tick basis approved; numerical scales remain uncalibrated | Use project-native L/tick values and define any sub-tick solver step as a fraction of a tick |
| Source-to-field transfer | Signed field-work identity and atomic source debit/field credit specified | Phase 2 proves the contract with a finite-budget mathematical test source; source power values remain calibration parameters |
| Source time units | Logical tick convention resolved | Express all source schedules and powers in ticks; runtime pacing is metadata only |
| World-tone source | Isotropic 3D external sun; above-plane perpendicular circular orbit; 24,000-tick orbit/day; 440-cycle/day carrier | Orbit/source values remain project-native parameters; Phase 2 validates retarded-time and conservative half-plane interception |
| Hotspot source | Localized in-plane distributed source; normalized radial cosine profile and finite budget/replenishment | Power/radius remain project-native parameters; verify grid- and receiver-independent integrated power when implemented |
| Material-origin oscillators | Powered internal-energy oscillators approved; disabled in empty-space reference solver | Phase 3 defines numeric oscillator/constitutive parameters while preserving specified ownership and transfer rules |
| Material coupling | Local conservative partition and shared finite-field-energy rule specified | Phase 3 calibrates material response coefficients and tests no-double-claim behavior |
| Damping / thermal state | Zero empty-space damping; material dissipation credits explicit local thermal storage | Numeric dissipation coefficients are Phase 3 values; heat transport remains deferred pending a defined mechanism |
| Structural/chemical energy | Explicitly provisional | Validate parameters or keep separate from settled physical energy |
| World geometry | Infinite flat two-dimensional plane; viewer is a nonphysical observation window | Project-native Euclidean coordinates are specified; Phase 2 tests finite-window independence |
| Environmental wave boundaries | Resolved: none globally; laboratory walls are ordinary physical objects | Validate finite-window numerical treatment and material-specific wall/interface behavior separately |
| Numerics | Uniform Cartesian grid; conservative second-order wave solver; leapfrog; ~16 samples per shortest wavelength initial target | Five-point/leapfrog stability bound is derived in Section 4.4; Phase 2 measures precision, isotropy, finite-window error, and convergence |
| Determinism | Deterministic tick/source/update order required; fixed-order accumulation for reference solver | Phase 2 implements and verifies deterministic replay; parallelism is deferred until it can preserve deterministic accumulation |

## 11. Phase 1 exit gate

Phase 1 is complete only when:
- Infinite flat-plane geometry, project-native world-distance units, and the reference numerical representation are explicitly specified; Phase 2 tests camera and storage-window independence.
- The field variable, governing equation, dimensionality, units, energy density, and flux are explicit and dimensionally consistent.
- Propagation speed is defined in project-native world-distance units per tick, independently of wall-clock performance; one simulated day and one solar orbit are both exactly 24,000 ticks.
- World-tone, hotspot, and material-origin source semantics have named ownership and dimensionally consistent source contracts.
- Material coupling, attenuation, heat, structural-energy limits, and the absence of environmental edge boundaries are explicit.
- The reference method has a derived five-point/leapfrog stability condition, a second-order refinement/convergence plan, deterministic update order, and a policy requiring numeric pass tolerances to be recorded before each Phase 2 experiment.
- Atomic transfer/rollback contracts cover every energy holder.
- The design is reviewed against the user's approved infinite-plane/laboratory/camera separation and unified-energy decisions, and the EXP-2026-008 baseline.
- No production solver or live chemistry/movement behavior has been changed as part of this specification phase.

**Phase 1 disposition:** The logical clock, source geometries, source ownership, material-emission model, passive coupling contract, project-native dimensions, numerical reference method, stability requirement, convergence protocol, deterministic ordering, and energy-transfer/rollback rules are specified. The current 2,400-tick orbital setting and legacy 86,400-second conversion are implementation discrepancies to correct in a later, separately scoped implementation task. Phase 1 is complete at the specification level. Phase 2 may now begin with an independent empty-space reference solver using a controlled finite-budget mathematical test source; no production solver, live chemistry, movement, or material coupling is authorized by this completion.

## 12. User-approved Phase 1 modeling defaults

The following defaults are explicitly approved and supersede any earlier wording in this document that described these choices as undecided. They establish the initial reference model; they do not predetermine numerical calibration values or prohibit later evidence-based model expansion.

| Decision | Approved default |
|---|---|
| Field representation | One persistent continuous scalar field containing simultaneous frequencies and phases |
| Empty-space behavior | Linear, isotropic, nondispersive propagation at finite speed; constructive/destructive interference; geometric spreading without automatic energy loss |
| Field energy | Amplitude is not energy; use the field energy-density and flux relations in Section 4.2 |
| Sun | External three-dimensional isotropic radiative source illuminating the infinite two-dimensional plane |
| Sun orbit | Vertical circular orbit in a plane perpendicular to the world plane, anchored by convention at world origin and offset above the plane by more than its radius |
| Sun timing | Use 24,000 physics ticks per orbit, exactly one simulated day; keep 440 cycles per simulated day as a distinct carrier-frequency setting |
| Sun propagation | Finite source-to-plane travel time with retarded emission time; same homogeneous-empty-space propagation speed as the in-plane field for the same carrier |
| Sun energy geometry | An infinite plane intercepts half the power of an isotropic 3D point source; the other half remains in the external accounting domain. Couple intercepted energy into the planar field conservatively without double counting |
| Material-origin emission | Powered material oscillators only; emission debits owned internal energy. Passive response includes absorption, reflection, transmission, and scattering |
| Hotspot | Separate localized in-plane distributed source; retain radial cosine profile as the initial candidate, integrate over a defined physical support area, and use a finite budget or explicit replenishment |
| Units and clock | Project-native L, tick, E. No mapping to wall-clock seconds or SI time; runtime pacing cannot influence simulated behavior |
| Reference numerics | Uniform Cartesian grid; conservative second-order wave solver; leapfrog or equivalent conservative second-order time integration |
| Resolution | Initially target about 16 samples per shortest modeled wavelength; establish adequacy through convergence tests |
| Computational domain | Physical plane remains infinite. Begin with a large reference domain whose edges are not reached during measurement; later use validated absorbing treatment or expanded/tiled domain |
| Empty-space damping | Zero initially |
| Material heat | Absorption/dissipation credits explicit local thermal storage; heat transport waits for a defined mechanism |
| Structural energy | Existing bond-graph structural energy remains provisional and separately reported |
| Optimization | Only after correctness, convergence, and conservation; no arbitrary propagation cutoff or unaccounted truncation |

The orbit anchor at world origin is a coordinate convention, not a physical center of the infinite plane. The vertical offset must exceed the orbit radius so the source remains above the plane for the entire orbit. For an isotropic point source, the half-power interception follows from the plane subtending one hemisphere; it is not a tunable normalization.

### 12.1 Calibration values intentionally left open

The approved defaults do not assign values to orbit radius/height, finite source budgets/power, propagation speed in L/tick, hotspot power/radius, material response coefficients, field normalization, grid spacing, or solver time step. Select and version these through dimensional consistency, source ownership, conservation, controlled reference experiments, stability, and convergence. No SI conversion or wall-clock calibration is required. Do not tune them to reproduce legacy movement or reaction behavior.

### 12.2 Phase 1 decision status

The architecture and modeling-default decisions, source/material contracts, project-native clock and units, energy-transfer rules, numerical stability bound, and convergence/tolerance policy are specified. Phase 1 is complete at the specification level. Phase 2 will select numerical parameters and validate the reference solver; no production solver or chemistry/movement behavior is authorized by this completion.
