# Phase 1 — Unified Energy and Continuous Wave-Field Specification

**Status:** Phase 1 specification complete. This document defines the approved reference physics and implementation-independent contracts. It does not authorize production implementation; Phase 2 begins with an independent reference solver.

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

The approved project-time convention is:
- **24,000 physics ticks = one simulated day.**
- **24,000 physics ticks = one complete solar orbital period.** One orbit and one simulated day are the same interval, not independent cycles.
- **440 carrier cycles per simulated day/orbit** for the reference sun carrier.
- **880 cycles per simulated day/orbit** for the localized hotspot carrier.

The current source audit found a legacy 2,400-tick orbital setting in the implementation. That value is inconsistent with the approved model and is a known implementation discrepancy, not an alternate physical convention. Phase and orbital position are derived deterministically from the logical simulation tick, never wall-clock execution time.

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

The approved initial reference representation is a uniform Cartesian grid with a conservative second-order wave solver. Use a second-order leapfrog integrator or an equivalent method demonstrated to preserve the discrete energy balance. Derive the stability bound for the actual stencil and time integrator; the familiar five-point-stencil CFL bound `c Δt / Δx ≤ 1/√2` is a reference check, not a substitute for that derivation. Initially target approximately 16 grid samples across the shortest modeled wavelength, then verify phase accuracy, isotropy, and convergence under refinement. The familiar five-point-stencil CFL bound `c Δt / Δx ≤ 1/√2` is only a starting reference for the corresponding idealized stencil; derive and test the actual stability bound for the selected discrete operator, spacing, and time integrator.

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

## 5. Source ontology and present-code findings

### 5.1 External world-tone source

Current source configuration describes the sun as a rotating external source, with a 440-cycles-per-simulated-day carrier, a 2,400-tick orbital cycle, and a configured incident-energy intensity of 1–10 energy units per physics tick per normalized projected width.

**Physical geometry decision:** The sun is outside the infinite flat world plane and travels on a circular orbit in a plane perpendicular to the world plane, like the sun moving around a flat world. Its orbital position determines the direction of illumination across the plane. The 24,000-tick cycle is the orbital period/schedule and equals one simulated day; it is distinct from the 440-cycle carrier frequency and phase. Define the orbit in physical world coordinates, independent of camera position, viewport, solver-window dimensions, and temporary laboratory walls. Orbit radius/distance, orientation, phase origin, and source emission geometry remain parameters to specify and calibrate; do not infer them from the old finite rectangular environment.

The source configuration and identity are reusable candidates. The current per-constituent uniform settlement is not a spatial wave source and must not remain an energy authority after migration.

The sun is not a boundary emitter of the 2D world: it is an external radiative source above the plane. Its radiation reaches the plane through a source-to-point propagation geometry with finite travel time, a declared radiation pattern, and incidence-angle projection. The incident waveform/power density is evaluated over physical plane area and coupled into the one persistent 2D field. The sun's emitted power is finite and normalized over its full radiation pattern; the infinite plane intercepts only the physically determined share. The source debit must equal the field's net received work. Do not allocate a fixed energy amount across visible materials or normalize by the camera window. A boundary-flux contract applies only to a source that is physically a line/boundary emitter.

### 5.2 Localized hotspot

The audited legacy hotspot configuration uses a radial spatial profile, 880 cycles per simulated day, and a configured peak rate derived from 1,000 legacy energy units per simulated second per unit realized material area. The current code converts this to 3,600 energy units per tick per unit area using its 24,000-tick / 86,400-second convention.

This is a volumetric/areal source-density contract, unlike the world-tone's projected-width rate. The two cannot be compared or merged into one source API until the physical dimensionality and unit conversion are explicit. The hotspot's location, frequency, profile, and source identity are reusable inputs; its current direct settlement into a shared reservoir is not.

### 5.3 Material-origin oscillators

The current material spectrum is calculated from material properties and a shared analytical oscillator convention. The code audit found no corresponding named finite source-energy debit for that material-origin amplitude. Therefore:

**Decision constraint:** Existing material aura/spectrum values are not allowed to inject field energy in the new system.

Phase 1 must choose and justify one of these interpretations before material-origin field emission is enabled:
1. **Powered oscillator:** a material has an explicitly owned internal-energy budget; emission debits it and any absorbed field energy is credited through a conservative transaction.
2. **Conservative coupled degree of freedom:** material and field are coupled degrees of freedom in one Hamiltonian/Lagrangian or equivalent conservative model; energy exchange is derived from the coupling equations, not from a source grant.
3. **Response only:** material properties determine absorption, reflection, transmission, scattering, or frequency response, but the material is not an emitter.

For the first empty-space reference solver, use only a controlled mathematical test source with an explicit finite energy budget. It is a solver fixture, not a claim about the sun or a material emitter.

The default for the first empty-space solver is no material-origin sources. Material response is introduced only after its coupling law and energy ownership are defined.

### 5.4 Source dimensions must be reconciled

The current world-tone rate is energy/(tick·projected-width); the hotspot rate is energy/(tick·area) after conversion. These are not interchangeable. The approved physical source classes are:
- **External radiative source:** a source outside the plane whose radiation reaches and couples into the plane over a physically defined spatial distribution. The sun belongs to this class; it is not a boundary emitter.
- **Distributed in-plane source:** a source with physical support area and power density, such as the provisional hotspot model.
- **Boundary/line emitter:** only a source whose actual physical geometry is a line or boundary, with power per physical length or a derived boundary-flux condition.
- **Coupled material oscillator:** a material degree of freedom with explicit internal-energy ownership or a conservative shared field/material energy function.
- **Passive material response:** a receiver that absorbs, reflects, transmits, or scatters without independently emitting energy.

Do not silently reinterpret a source's current numerical intensity as a field amplitude. For the scalar forced-wave candidate, source power must be computed from the actual field work term (the spatial integral of `K u_t f`) or an equivalent explicitly derived coupling/flux condition. The source's finite budget is debited by the same signed transfer that the field receives; if the coupling is one-way, its missing return-work destination must be specified rather than discarded.

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

For a true boundary/line emitter, specify the boundary condition and its signed energy flux using the selected field equation's conservation law. Integrate over the complete physical emitting boundary length. A prescribed boundary amplitude is not a substitute for a power/flux contract. The field's net energy gain must equal the source's net debit, including any reflected work returned to the source. This contract does not describe the sun.

For an in-plane distributed emitter, specify the source support in physical area and power density in E/(L²·T). Integrate over the complete support using physical area. Do not multiply by the number of grid cells without the corresponding cell-area factor; refinement must not change total source power.

For an external radiative source such as the sun, define the source position in 3D coordinates relative to the embedded 2D plane, its finite emitted power, its angular radiation pattern, and the propagation speed/path to each receiving point. The source-to-plane travel time must be finite and use retarded emission time: a field contribution arriving at plane point p at time t was emitted at the corresponding earlier time determined by the source-to-p distance and the universal propagation speed through homogeneous empty space. Use the same physical propagation speed as the in-plane field for the same carrier in the same medium; introduce a different speed only if a distinct physical medium and interface are explicitly modeled. The incident power density on the plane must include the source's distance dependence and the projection/incidence geometry. Its integral over the entire plane must not exceed the source's emitted power over the corresponding interval; any un-intercepted radiation remains in the external accounting domain rather than disappearing. The approved initial radiation pattern is isotropic in three-dimensional space. An infinite plane illuminated by an isotropic point source located on one side intercepts one half of the isotropically emitted power; the other half continues into the external accounting domain. This geometric partition is not an arbitrary efficiency factor. The mapping of intercepted radiation into the planar field must be conservative and must not count the same energy in both the external radiation domain and the planar field. Source power and any non-geometric coupling loss remain calibration/constitutive questions.

The world-tone source is the sun: an external radiative source outside the infinite flat world plane, orbiting it on a circular path in a plane perpendicular to the world plane. Its changing orbital position determines the direction and incidence geometry of illumination. The 2,400-tick cycle is provisionally the orbital period/schedule and is distinct from the emitted field's carrier frequencies and phases. Define this orbit in physical coordinates independently of the camera, viewport, laboratory walls, and computational-window dimensions. The orbit is a vertical circle in a plane perpendicular to the world plane, anchored by convention at the world-coordinate origin and offset above the plane by more than its radius so the source never crosses the plane. The orbit is independent of the camera, viewport, lab walls, and solver window. The 24,000-tick orbital period equals one simulated day; the 440-cycle-per-simulated-day carrier setting remains a separate emitted carrier frequency. Orbit radius, vertical offset, orientation, phase origin, emitted power, and the conservative source-to-field coupling calibration remain open numerical parameters. The old orbit derived from a finite rectangular environment must be retired. The sun is an external radiative source coupled into the plane, not a boundary emitter and not an energy grant to each visible material. Its finite energy budget or explicitly accounted external replenishment and conversion from the legacy projected-width rate to a physically dimensioned radiation-power contract remain open. The present hotspot input is provisionally classified as an in-plane distributed source because its legacy rate is stated per realized area. Its radial cosine profile remains the initial candidate. Define a physical support area and integrate power over that area so total source power is independent of molecule count, receiver count, grid spacing, and grid refinement. Give it an explicit finite energy budget or replenishment law. Its total power and physical radius remain calibration parameters; the legacy 1,000-energy-units-per-second-per-area value is not carried forward without calibration. These classifications are hypotheses to audit against source code and intended physics.

The 1–10 energy-units-per-tick-per-normalized-width world-tone setting and the 1,000 energy-units-per-simulated-second-per-area hotspot setting are legacy configuration values, not directly comparable physical quantities. Convert each through the verified tick/time and world-length/area scales before comparing or combining them. Never treat either value as a field amplitude. Source allocation must be independent of solver resolution, viewer size, camera zoom, and the number of receivers.

Material-origin emission is a distinct case. The initial empty-space reference solver has no material emitters. When material coupling is introduced, select and implement either (a) a powered oscillator that debits a named material-internal energy holder, or (b) a conservative coupled field/material degree of freedom with a derived shared energy function. Passive response may absorb, reflect, transmit, or scatter incident field energy without becoming a source. Existing aura/spectrum calculations alone authorize neither emission nor an energy credit.

### 5.6 Audit of the current source implementations

The current code confirms that the legacy source values are accounting inputs, not yet physical wave-field contracts:

- **World tone:** `WorldToneParameters` configures 1–10 energy units per physics tick per unit of exposed projected width, a 440-cycles-per-simulated-day carrier, and a 2,400-tick orbital schedule. The cached `WorldToneCycle` stores source samples; it does not own or debit a finite source-energy reserve. The staged `ExternalInputTransaction` treats `source_available` as a local incident allowance, explicitly not necessarily a global source pool. The current production `settle_uniform_material_inputs` path assigns one normalized unit of projected width per material constituent, so total incident energy scales with recipient count; it is not spatial wave propagation or a geometric source-boundary injection. The source's 2,400-tick orbital schedule is inconsistent with the approved 24,000-tick day/orbit. Older geometry/exposure APIs remain in the module but do not change that production-path finding.
- **Hotspot:** `RadialHotspot` uses a radial cosine profile with a configured peak of 1,000 energy units per simulated second per realized material area. Its code converts this to 3,600 energy units per tick per area using 24,000 ticks per 86,400 simulated seconds. Settlement evaluates material placements, multiplies local intensity by realized constituent area and amount, then records aggregate incident/captured/heat/escaped accounting. It deposits into the legacy environment resonance reservoir; it does not propagate a persistent spatial wave or debit a named finite source reserve.
- **Clock conversion:** The hotspot implementation explicitly encodes 86,400 simulated seconds per 24,000 physics ticks, which implies 3.6 simulated seconds per tick. This is a source-model convention, not wall-clock pacing. The experiment runner and all frequency/phase helpers must use the same simulated-time coordinate before this conversion is declared globally authoritative. Under that convention, 440 and 880 cycles per simulated day correspond to 440/86,400 Hz and 880/86,400 Hz respectively; the 2,400-tick orbital period corresponds to 8,640 simulated seconds and remains a separate schedule.
- **Infinite-plane incompatibility:** The legacy world-tone source position is sampled on an orbit whose radius is calculated from finite rectangular world width/height and is required to remain outside that rectangle. There is no such outside edge on the approved infinite plane. Do not carry that orbit or its finite-world visibility assumptions into the new model unchanged.

These findings constrain migration but do not prescribe the replacement physics. In particular, the new source model must not preserve per-constituent energy grants, assume the old world-tone orbit is meaningful on an infinite plane, or treat the hotspot's legacy reservoir deposit as a traveling field. Both sources must be re-expressed through the common source-to-field contract only after their physical geometry, ownership/replenishment, units, and work transfer are resolved.

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

Material constitutive properties determine frequency-dependent coupling and response. Resonance is a response characteristic, not an energy source. Absorption, reflection, transmission, scattering, and any re-emission must follow a conservative local interaction law. Passive response may redistribute or store incoming energy but cannot create net energy. Material-origin emission remains disabled until a powered internal-energy debit or conservative coupled-degree-of-freedom law is defined.

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
| Field interpretation | Approved initial scalar, linear, isotropic, nondispersive reference model | Validate against reference experiments; expand only if evidence requires it |
| Frequency representation | User-selected single field containing multiple frequencies and phases | Validate superposition/interference and frequency-resolved response |
| Material emission/response | Approved powered oscillators plus passive absorption/reflection/transmission/scattering | Specify constitutive coefficients and conservative material-energy coupling |
| Empty-space propagation speed | User-selected universal constant in homogeneous empty space | Calibrate value and physical units; do not infer from legacy movement behavior |
| Energy density and flux | Approved scalar-reference formulas in Section 4.2 | Calibrate field normalization and energy units dimensionally |
| Dimensional basis | Project-native L, tick, E approved | Keep all equations in project-native L, tick, and E; no SI or wall-clock conversion is required |
| Scalar field dimensions | Dimensionless u is the initial reference convention | Confirm this convention in the solver contract and calibrate K consistently |
| Spatial unit and propagation speed | L and T basis approved; numerical scales remain uncalibrated | Establish world-unit-to-meter scale and propagation speed by physical criteria; define tick/substep relationship |
| Source-to-field transfer | One conservative source/field work contract approved | Complete source-specific implementation-independent transfer equations, finite budgets/replenishment, and calibrated power/flux values |
| Source time units | Existing conventions identified | Separate physics time from experiment/wall time; calibrate source power |
| World-tone source | Approved isotropic 3D external source; above-plane perpendicular circular orbit; 2,400-tick period; 440-cycle/day carrier kept separate | Calibrate orbit dimensions and source power; verify retarded-time and conservative half-plane-interception coupling |
| Hotspot source | Approved localized in-plane distributed source; radial cosine initial profile and physical support area | Calibrate power/radius and finite budget or replenishment; prove independence from molecule count and grid refinement |
| Material-origin oscillators | Powered internal-energy oscillators approved; disabled in empty-space reference solver | Define oscillator energy states, coupling law, and conservative debit/credit behavior |
| Material coupling | Passive absorption/reflection/transmission/scattering and powered emission approved | Define constitutive response coefficients and finite-energy sharing equations |
| Damping / thermal state | Zero empty-space damping approved; material dissipation credits local thermal storage | Define material dissipation coefficients and thermal holder; defer heat transport until a mechanism is selected |
| Structural/chemical energy | Explicitly provisional | Validate parameters or keep separate from settled physical energy |
| World geometry | User-resolved: infinite flat two-dimensional plane; viewer is a nonphysical observation window | Define world-distance units, planar numerical representation, and geometry tests |
| Environmental wave boundaries | Resolved: none globally; laboratory walls are ordinary physical objects | Validate finite-window numerical treatment and material-specific wall/interface behavior separately |
| Numerics | Approved uniform Cartesian grid; conservative second-order solver; leapfrog/equivalent integrator; ~16 samples per shortest wavelength initial target | Derive stability bound and demonstrate precision, isotropy, finite-window error, and convergence |
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
| Units and clock | Project-native L, T, E with explicit conversions. Verify runner and phase/frequency helpers; if all confirm 24,000 ticks per 86,400-second simulated day, adopt 3.6 seconds per tick |
| Reference numerics | Uniform Cartesian grid; conservative second-order wave solver; leapfrog or equivalent conservative second-order time integration |
| Resolution | Initially target about 16 samples per shortest modeled wavelength; establish adequacy through convergence tests |
| Computational domain | Physical plane remains infinite. Begin with a large reference domain whose edges are not reached during measurement; later use validated absorbing treatment or expanded/tiled domain |
| Empty-space damping | Zero initially |
| Material heat | Absorption/dissipation credits explicit local thermal storage; heat transport waits for a defined mechanism |
| Structural energy | Existing bond-graph structural energy remains provisional and separately reported |
| Optimization | Only after correctness, convergence, and conservation; no arbitrary propagation cutoff or unaccounted truncation |

The orbit anchor at world origin is a coordinate convention, not a physical center of the infinite plane. The vertical offset must exceed the orbit radius so the source remains above the plane for the entire orbit. For an isotropic point source, the half-power interception follows from the plane subtending one hemisphere; it is not a tunable normalization.

### 12.1 Calibration values intentionally left open

The approved defaults do not assign values to orbit radius/height, source power, world-distance-to-meter conversion, project-energy-to-joule conversion, propagation speed, hotspot power/radius, material response coefficients, field normalization, grid spacing, or solver time step. Derive these through dimensional analysis, source-code verification, physical criteria, and controlled reference experiments. Do not tune them to reproduce legacy movement or reaction behavior.

### 12.2 Phase 1 decision status

The architecture and modeling-default decisions above are resolved. Phase 1 remains a specification/contract phase until the clock convention is verified, the remaining calibration methodology and source/material transfer contracts are explicit, the numerical stability and convergence criteria are documented, and the exit-gate checklist is satisfied. No production solver or chemistry/movement behavior is authorized by this approval.
