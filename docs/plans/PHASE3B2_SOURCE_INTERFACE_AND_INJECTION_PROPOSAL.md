# Phase 3B.2 — Source Interface and Injection Proposal

**Status:** Design proposal only. Requires review before implementation.

**Authority:** Phase 1 unified energy/continuous wave-field specification, the resonance-field implementation roadmap, and completed Phase 3B.1 source/interface audit.

**Scope:** Define a source descriptor, one authoritative source-energy account, and an atomic source-to-field coupling boundary. Specify discrete work, physical source measures, failure behavior, and validation gates.

**Not authorized:** Production integration, live sun/hotspot migration, receiver/material settlement, material oscillators, chemistry, movement, organisms, GUI, or numerical source calibration.

## 1. Executive recommendation

Adopt a three-part boundary:

1. **Immutable source descriptor** — stable identity/provenance, physical source class and geometry, deterministic logical-tick schedule, waveform/frequency/phase, and dimensionally correct power law.
2. **Mutable source-energy account** — exactly one authoritative finite reserve per source identity, with explicit replenishment transactions if the source is not closed.
3. **Field-coupling operation** — evaluates the descriptor, constructs physical spatial forcing, applies one deterministic discrete field update, measures actual signed change in discrete field energy, and atomically settles the matching source account.

Do not merge these responsibilities into one object cloned per receiver, grid cell, or emission event. Do not make the legacy material energy reservoir or per-recipient external allowance the source-energy authority.

**Critical physics gate:** Physical incident power density q(p,t) is not field forcing f(p,t) or field amplitude u. The approved identity is ∂t e + ∇·J = K u_t f in empty space. There is no approved universal conversion from q to f. Do not invent one. A finite mathematical test source can validate the discrete accounting interface, but real sun/hotspot injection remains blocked until the work-conjugate coupling law is derived and calibrated independently.

## 2. Staged delivery

| Slice | Content | Gate |
|---|---|---|
| 3B.2a | Source descriptor/account API and finite mathematical test source in isolated reference solver | May proceed after exact discrete work/rollback contract is accepted |
| 3B.2b | Physical hotspot profile normalization and area-integration validation, independent of field coupling | Separate testable geometry/integration slice |
| 3B.2c | Hotspot-to-field coupling using approved work-conjugate law | Blocked pending coupling-law decision and calibration |
| 3B.3 | External sun geometry, retarded-time source-to-plane flux, finite budget and plane-interception accounting | Separate proposal/approval |
| Later Phase 3 | Receiver boundary coupling, material response, heat settlement | Separate gate |

The first implementation remains isolated from live simulation paths. It does not require merging PR #410 or importing its branch wholesale. Its validated equations and tests are a reference contract; solver-code consumption must be decided explicitly before implementation.

## 3. Proposed source descriptor

The descriptor is immutable during a run. It describes source identity and behavior, not mutable remaining energy. Conceptual shape; names are illustrative, not an approved API signature:

~~~rust
struct SourceDescriptor {
    id: SourceId,
    provenance: SourceProvenance,
    class: SourceClass,
    geometry: SourceGeometry,
    schedule: EmissionSchedule,
    waveform: SourceWaveform,
    power_law: SourcePowerLaw,
}
~~~

Required semantics:

- **Identity:** Stable source-instance ID, distinct from source class. Two hotspots are two identities and cannot share one mutable budget accidentally.
- **Provenance:** Identifies the physical owner/origin of energy. An external source is not an unowned grant. An internal oscillator names its owning energy holder.
- **Geometry:** Project world coordinates and source-class-specific physical representation. No camera, viewer, finite environment bounds, or solver-window coordinates.
- **Schedule:** Pure deterministic function of logical tick/substep. No wall-clock pacing or hidden timer.
- **Waveform:** Carrier frequency and phase law are distinct from source position/orbital phase. A waveform is not a spendable energy reservoir.
- **Power law:** Explicit dimensions. Total power E/tick; in-plane density E/(L²·tick); a true line emitter may use E/(L·tick). Never infer one measure from another.
- **No silent replenishment:** A source has a closed finite reserve or receives explicit, separately accounted replenishment. Constant configured power with no owner or replenishment origin is invalid.

Boundary/line emitters remain in the ontology only to preserve the Phase 1 distinction. No current source should be implemented as a line emitter merely because the numerical solver has a boundary.

## 4. Proposed source-energy account

Conceptual shape:

~~~rust
struct SourceEnergyAccount {
    source_id: SourceId,
    initial_energy: Energy,
    remaining_energy: Energy,
    cumulative_field_transfer: SignedEnergy,
    cumulative_replenishment: Energy,
}
~~~

Contract:

- Initial and remaining energy are finite and nonnegative.
- Remaining energy is never reconstructed from a per-tick, per-cell, or per-receiver allowance.
- Emission consumes the same account across every substep and spatial sample.
- Replenishment is a separate transaction with an identified origin, schedule, and energy debit on that origin.
- Signed field work is recorded independently from replenishment. Positive work debits the source; negative work returns energy to that same source/coupled holder.
- Exhaustion is explicit. Never clamp a negative balance to zero after the field has already changed.
- Mutable accounts are not cloned in the live transaction path. Descriptors may be shared read-only; mutable accounts require unique ownership or transaction borrowing.
- Deterministic source ordering is by stable source identity, not container iteration order.

Include source-held energy in the unified inventory:

E_initial + E_external,in = E_source,remaining + E_field + E_material + E_thermal + E_structural + E_mechanical + E_exported + E_residual.

Residual is a validation result, never a destination.

## 5. Discrete source-to-field work contract

### 5.1 Continuum authority

The approved empty-space equation is:

u_tt = c² Δu + f(p,t)

with energy density and flux:

e = (K/2)[u_t² + c²|∇u|²],    J = -Kc²u_t∇u.

The local work identity is:

∂t e + ∇·J = K u_t f.

For source region Ω and interval [t_n,t_(n+1)], signed source-to-field transfer is:

W_s→f = ∫(t_n to t_(n+1)) ∫_Ω K u_t(p,t) f_s(p,t) dA dt.

This is the energy transferred to the field, not source amplitude. It may be positive or negative. Source account and field state settle the same signed transfer.

### 5.2 Required discrete identity

For numerical source update F_n → F_(n+1), define actual discrete field transfer:

ΔE_f = E_h(F_(n+1)) - E_h(F_n),

where E_h is the solver's validated discrete energy invariant, including its exact time-centering and spatial quadrature.

The committed source update is:

E_s,n+1 = E_s,n - ΔE_f.

For an isolated source/field transaction:

(E_s,n+1 + E_h(F_(n+1))) - (E_s,n + E_h(F_n)) = 0

within declared floating-point tolerance. If damping or a computational-boundary absorber is active, its independently measured sink is included; the source transaction must not conceal it. Approved empty-space default remains zero damping.

**Exact discrete balance is mandatory.** Continuum quadrature of K u_t f is a convergence diagnostic, not a substitute for actual discrete energy change. The difference between discrete transfer and continuum work must converge under paired space/time refinement at the expected order.

### 5.3 Forcing construction is unresolved

Incident power density q has units E/(L²·tick). Forcing f has units of field-state per tick². They cannot be equated.

The tempting pointwise relation f=q/(K u_t) is singular wherever u_t=0 and unbounded near velocity nodes. It is not an acceptable general source law. Setting displacement/velocity proportional to sqrt(q) is also insufficient: it confuses state initialization with continuous emission and does not prove the work identity.

Before real source coupling, the work-conjugate law must specify:

- How source waveform and physical spatial profile determine forcing shape.
- How source power limits/schedules allowed signed work.
- Behavior at zero-velocity nodes and when field returns work to source.
- Whether the source is a prescribed-power active source or passive radiative input with field-state-dependent absorption.
- Preservation of phase, superposition, deterministic replay, and finite source budget.
- Convergence to continuum K u_t f under refinement.

No arbitrary efficiency, amplitude normalization, hidden energy grant, or organism-style usable-energy pool may fill this gap.

### 5.4 First implementation: mathematical test source

Use a deliberately mathematical source with stable test-source ID, explicit test-holder provenance, finite initial budget and no replenishment, deterministic compact spatial forcing template and logical-tick schedule, no material/chemistry recipient, discrete field-energy measurement before/after each update, exact signed debit/credit, and fail-closed behavior for invalid state, overdraw, or unreconciled energy.

This validates accounting/interface only. It does not validate sun/hotspot physics or calibrate source power.

## 6. Physical source integration

### 6.1 Localized hotspot

The approved class is an in-plane distributed source:

q_h(p,t) = P_h(t) g(r),   r = |p-p_h|,

with compact support 0 ≤ r ≤ R_h and:

∫g(r)dA = 2π∫₀ᴿʰ g(r)r dr = 1.

Thus total incident source power is P_h(t), units E/tick; g has units L⁻². This is not a per-material-area energy grant.

If the cosine profile is retained as candidate:

g(r) = C[1 + cos(πr/R_h)] for 0 ≤ r < R_h, and zero outside.

Normalization requires:

1 = 2π C R_h² ∫₀¹ x[1 + cos(πx)]dx
  = 2π C R_h²(1/2 - 2/π²).

Therefore:

C = 1 / [2πR_h²(1/2 - 2/π²)].

This is a mathematical normalization candidate, not a calibrated physical profile. It has compact support and zero value at the edge. Test its analytic integral independently before grid deposition.

Evaluate physical-area integration independently of molecule and receiver counts. For grid deposition, use cell/support overlap or documented quadrature with convergence/error report. Do not sample only cell centers and assume power is conserved. Under refinement, integrated profile converges to one and total power remains P_h.

The 3A.4d polygon-only adapter does not represent a circle. For this radially symmetric profile, prefer the analytic radial integral over polygonizing the circle. A general circle-union geometry feature is not justified by this single source; propose separately only if another physical use requires it.

The legacy 1,000-energy-units-per-simulated-second-per-area rate and 3,600-per-tick conversion are retired. No replacement power value is selected here.

### 6.2 External sun

The sun is a separate later source slice. It is not an in-plane source and must not use hotspot footprint area.

Approved source-to-plane density:

q_s(p,t) = P_s(t_emit) h_s/(4πd³),    t - t_emit = d/c.

Accounting partitions isotropic emitted power into exactly half intercepted by the infinite plane and half remaining in the external radiation domain. Retarded source position, finite source reserve, plane interception, numerical-window representation, and external-domain accounting are one deterministic transaction.

The finite solver window creates an additional distinction: radiation intercepted by the physical infinite plane outside the current numerical window is not the same as the half of isotropic power that never intersects the plane. Out-of-window plane interception is accounted separately; it is not silently classified as non-intercepted radiation or lost at a computational edge.

Use approved 24,000-tick orbital/day period and 440 carrier cycles/day. Orbit radius/height, propagation speed, and emitted power remain open project-native parameters. The legacy 2,400-tick orbit and per-projected-width recipient allowance are not carried forward.

## 7. Atomic transaction and failure behavior

One source update is all-or-nothing:

1. Validate descriptor, account identity, logical time, field state, geometry, and finite/nonnegative constraints.
2. Evaluate deterministic schedule and waveform at exact logical substep.
3. Construct physical forcing template without mutating field/account.
4. Stage next field state and calculate discrete energy before/after.
5. Derive signed ΔE_f from actual discrete invariant.
6. Stage source balance remaining - ΔE_f; reject if non-finite or negative.
7. Verify transaction residual and any separately modeled sinks/exports.
8. Commit field state and source account together with one deterministic transaction record.

On failure, neither field nor account changes. Reject invalid identity/provenance, duplicate source identity with duplicate account ownership, non-finite profile, unsupported geometry, schedule overflow, field instability, source overdraw, negative account balance, or residual outside tolerance.

Process multiple sources in stable source-ID order. Each source has its own account; field updates are sequential and deterministic. Combined energy change equals sum of signed transfers. No budget is copied per grid cell or source sample.

## 8. Solver-consumption boundary

PR #410 is a completed isolated reference-solver milestone but remains closed and unmerged. Do not reopen it or treat its branch as production dependency.

Before 3B.2a code, choose one explicit path:

- **Preferred if architecture permits:** Extract/reuse validated numerical solver implementation through a narrowly scoped, separately reviewed dependency/module change while preserving standalone tests and exact energy invariant.
- **Alternative:** Reimplement only the minimal source-account/interface adapter around the same validated solver contract, with direct parity tests against independent reference results. Do not silently fork/change the wave equation or integrator.
- **Not acceptable:** Merge PR #410 merely to access its types, copy implementation into production without provenance/parity tests, or rewrite numerical core as part of source-interface work.

The choice must assign solver-test ownership, deterministic behavior, and maintenance responsibility while minimizing duplicate numerical authorities.

## 9. Validation matrix

### Source/account
- Unique source IDs and explicit provenance.
- Two same-class sources own distinct accounts.
- One account persists across emissions/substeps.
- No mutable-budget cloning.
- Exhaustion and explicit replenishment behavior.
- Deterministic source ordering and replay.

### Discrete field-work transaction
- Positive field work debits source exactly once.
- Negative field work credits correct source holder.
- Zero-work update does not alter source energy.
- Insufficient budget rejects with exact field/account rollback.
- Invalid/non-finite forcing rejects with exact rollback.
- Residual reconciles to discrete energy invariant.
- Multiple source transfers sum to total field-energy change.
- Source-free propagation remains unchanged.

### Physical hotspot profile
- Analytic normalization integral equals one.
- Total power equals P_h independent of radius after normalization.
- Translation preserves integrated power.
- Grid refinement converges to analytic area integral.
- Grid alignment and sub-cell translation do not create material power changes.
- Integrated power independent of molecule count, receiver count, and viewer size.
- No center-sample-only approximation without convergence evidence.

### Continuum/discrete convergence
- Paired space/time refinement within derived CFL margin.
- Compare discrete signed transfer with converged quadrature of ∫∫K u_t f dA dt.
- Report convergence order and normalized residual.
- Validate phase/frequency behavior and deterministic replay.
- No arbitrary amplitude cutoff or empty-space damping.

### Later sun-specific tests
- Retarded-time position and emission schedule deterministic.
- Integral over entire plane is P_s/2.
- Non-intercepted external radiation is P_s/2.
- Finite-window plane interception accounted separately.
- Orbit/carrier phases remain distinct.
- Results independent of viewer and material/receiver count.

## 10. Explicit non-goals

No live sun/hotspot migration; source-to-material or receiver-boundary coupling; absorption, reflection, transmission, scattering, occlusion, or thermal settlement; material-origin oscillators; chemistry, environmental movement, organism logic, or energy budgets; new global usable-energy/frequency-reservoir authorities; calibration of K, c, source power, orbit radius/height, hotspot radius, or replenishment schedule; GUI, performance optimization, or changes to infinite-plane physical boundary model.

## 11. Review decisions required

Before implementation, approve or revise:

1. Three-part descriptor/account/coupling boundary.
2. Exact discrete identity: source balance change equals negative discrete field-energy change, with atomic commit/rollback.
3. Solver-consumption path without reopening or merging PR #410.
4. Normalized cosine hotspot profile as a mathematical candidate, not calibrated physical law.
5. Real hotspot/sun coupling remains blocked until work-conjugate physical forcing law is derived; incident power density is not field amplitude.
6. Sun implementation remains a separate 3B.3 slice; receiver/material settlement remains separately gated.
7. Authorize only 3B.2a and 3B.2b after design acceptance; no production integration implied.

## 12. Recommended next action

Review this proposal as a design gate. If accepted, implement only the finite mathematical source/account transaction and independent analytic hotspot normalization/integration tests. Do not connect real hotspot or sun to live simulation until the field-coupling law is separately derived, reviewed, and validated.
