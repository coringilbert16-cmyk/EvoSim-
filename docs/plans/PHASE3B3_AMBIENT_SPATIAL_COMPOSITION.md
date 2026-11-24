# Phase 3B.3 — Ambient/Disturbance Spatial Composition

**Status:** Analytic packet/patch references and an isolated finite-dimensional material-local coupling harness are implemented. The latest exact-head CI is running; continuum interface/convergence validation remains open. No composition law selected and no production coupling authorized.

**Authority:** The [unified energy and continuous wave-field specification](../design/unified-energy-physics-specification.md), the [controlling resonance-field roadmap](../design/resonance-field-implementation-roadmap.md), and the user-approved intrinsic ambient-mode decision recorded in [the isolated ambient-mode plan](PHASE3B3_WORLD_GENERATED_AMBIENT_MODE.md).

## 1. Question and scope

Determine whether the world-generated ambient resonance mode can coexist with localized disturbances governed by the approved, massless, nondispersive scalar wave equation

\[
w_{tt}=c^2\nabla^2 w
\]

without changing that local propagation law, creating a physical edge at the finite accounting footprint, or double-counting energy.

This investigation is analytical and numerical-reference work only. It does not connect the ambient mode to the production solver, materials, chemistry, organisms, GUI, or production energy ledger. It does not select a physical material-coupling law.

The ambient reference schedule remains:
- 24,000 ticks per daily envelope cycle;
- 440 carrier cycles per day;
- cycle-averaged target energy density from 1 to 10 project-energy units per project-area unit;
- positive changes accounted as world generation and negative changes as world withdrawal.

## 2. Result 1: a uniform harmonic displacement is not a free mode of the approved wave equation

Suppose the ambient carrier is represented as a spatially uniform displacement

\[
b(t)=a(t)\cos(\omega_0t+\phi_c).
\]

Since \(\nabla^2 b=0\), a free solution of the massless wave equation would require

\[
b''(t)=0.
\]

A nonzero-frequency carrier does not satisfy this condition. If it is added directly to the spatial displacement, the total field

\[
U(\mathbf{x},t)=w(\mathbf{x},t)+b(t)
\]

satisfies a *forced* equation,

\[
U_{tt}=c^2\nabla^2U+f_a(t),\qquad f_a(t)=b''(t),
\]

not the free reference equation. The ambient term is then a spatially uniform forcing term. Its signed field work must be accounted using the same work-conjugate quantity as any other forcing; the prescribed ambient energy schedule cannot simply be credited as an independent positive field-energy deposit.

This is a structural result, not a numerical-stability issue. Making the carrier frequency smaller, changing the grid, or refining the time step does not turn a nonzero-frequency uniform displacement into a free solution.

## 3. Result 2: naïve energy addition is incorrect

For the approved scalar field, energy density and flux are

\[
e(U)=\frac K2\left(U_t^2+c^2|\nabla U|^2\right),
\qquad
\mathbf J(U)=-Kc^2U_t\nabla U.
\]

Substituting \(U=w+b\), with spatially uniform \(b\), gives

\[
e(U)=
\underbrace{\frac K2(w_t^2+c^2|\nabla w|^2)}_{e_w}
+\underbrace{\frac K2(b')^2}_{e_{b,\mathrm{kin}}}
+\underbrace{K b'w_t}_{e_{\mathrm{cross}}},
\]

and

\[
\mathbf J(U)=\mathbf J(w)-Kc^2b'\nabla w.
\]

Therefore the total energy is not, in general, the sum of an independently computed disturbance energy and an independently computed ambient energy. There is a cross-energy term and a corresponding cross-flux term. Omitting either breaks the local balance for the combined field.

For an accounting patch \(\Omega\), the cross contribution is

\[
E_{\mathrm{cross},\Omega}
=K b'(t)\int_\Omega w_t\,dA.
\]

It need not vanish for a localized disturbance. Its change must reconcile with the cross-flux through the patch boundary and the ambient forcing work. A patch boundary is an accounting surface only; it must not be implemented as a reflecting, absorbing, or terminating physical boundary.

There is a second mismatch: the massless scalar-field energy contains no restoring-potential term \(K\omega_0^2b^2/2\). The harmonic-oscillator energy used by the isolated ambient-mode model is a different degree-of-freedom energy definition. It cannot be inserted into the massless wave-field inventory without explicitly defining the additional degree of freedom and its coupling/ownership.

## 4. Result 3: the slowly varying envelope has a small, nonzero oscillator-energy correction

The isolated ambient-mode plan maps the target cycle-averaged energy density to amplitude using

\[
e_a(t)=\frac{K\omega_0^2a(t)^2}{2}.
\]

For a time-varying envelope and oscillator coordinate \(b=a(t)\cos(\omega_0t+\phi_c)\), the harmonic-oscillator energy density is

\[
e_{\mathrm{osc}}(t)=\frac K2\left[(b')^2+\omega_0^2b^2\right].
\]

Averaging over the fast carrier while treating the envelope as locally constant over one carrier cycle gives

\[
\left\langle e_{\mathrm{osc}}\right\rangle
=\frac{K\omega_0^2a^2}{2}
+\frac K4(a')^2.
\]

Thus the current amplitude formula omits the nonnegative envelope-kinetic correction \(K(a')^2/4\). The correction is expected to be small because the envelope frequency is 1/440 of the carrier frequency, but it is not identically zero. The numerical reference must quantify it across the full day and decide whether the target schedule means (a) the carrier-only approximation already used, or (b) exact total oscillator energy with amplitude chosen accordingly. This choice must be explicit before the mode is coupled to anything.

## 5. Infinite-plane and accounting-area constraint

The physical domain remains the infinite plane. A spatially uniform, nonzero energy density over the entire plane has infinite total energy. The finite simulated-world area in the current ambient-mode implementation can therefore serve only as a declared accounting patch (or a normalization reference); it cannot be interpreted as the physical support of a globally uniform mode.

Any patch-level balance must report:
- its area and the ambient energy change per unit area;
- disturbance energy inside the patch;
- all disturbance and cross-flux crossing the patch boundary;
- ambient generation/withdrawal over the patch;
- numerical residual separately from physical exchange.

Changing the accounting patch must not change the local equations, carrier amplitude/density, source law, or any receiver's physical response. Patch-area scaling is an accounting check, not a physical boundary condition.

## 6. Candidate composition models — none selected

### A. Additive displacement with explicit uniform forcing

Use \(U=w+b\), retain the full combined energy and flux above, and account for the uniform forcing \(f_a=b''\) through signed work. This preserves the local disturbance equation for \(w\), but it does **not** make the ambient oscillator a free mode of that equation. The oscillator's restoring energy and world-generation mechanism still need a consistent Hamiltonian/ownership model. This is a useful diagnostic decomposition, not yet an approved physical composition.

### B. Independent ambient oscillator field

Keep the ambient oscillator as a separate spatially uniform energy-bearing degree of freedom and keep \(w\) as the independent massless disturbance field. If they do not interact, their inventories add, but the ambient mode has no effect on disturbance propagation or on matter. This is mathematically clean but may not satisfy the intended meaning of an ambient resonance field.

### C. Reciprocal coupled ambient/ disturbance degrees of freedom

Define an explicit coupled Hamiltonian with ambient and disturbance state variables, a coupling-energy term, and equal-and-opposite exchange. This can provide reciprocal interaction without treating the ambient schedule as an inexhaustible prescribed waveform. However, the coupling law must be derived. A generic local coupling can change the disturbance dispersion relation; a finite-patch coupling profile can accidentally create a physical edge. Neither is acceptable without proof and tests.

No candidate is approved by this document. In particular, do not add a global Klein–Gordon term to the disturbance equation, silently omit cross terms, or use the accounting patch as a physical support boundary.

## 7. Required isolated numerical checks

The next reference harness must be deterministic and independent of production simulation code. It should use the same project-native parameters and report formulas, input values, and normalized residuals.

1. **Uniform-mode equation residual:** Evaluate \(b''\) for the 440-cycle carrier and show that a nonzero-frequency uniform displacement is not a free solution of the massless wave equation.
2. **Combined-energy identity:** For a known localized disturbance \(w\), compute \(e(w+b)\) directly and compare it with \(e(w)+K(b')^2/2\). The difference must equal \(K b'w_t\) pointwise.
3. **Combined-flux identity:** Compute \(\mathbf J(w+b)\) and verify the cross-flux \(-Kc^2b'\nabla w\).
4. **Patch balance:** Integrate disturbance energy, cross energy, ambient forcing work, and flux across several nested accounting patches. The physical solution and local field samples must be invariant under patch changes.
5. **Envelope correction:** Evaluate \(K(a')^2/4\) and its ratio to the declared target density over all 24,000 ticks; report the maximum and integrated correction.
6. **Independent-mode control:** Verify that an uncoupled ambient oscillator plus a free disturbance field has additive inventory and no disturbance response to the ambient phase.
7. **Deterministic replay:** Repeat every numerical case and require bit-for-bit identical state/diagnostic output where the chosen arithmetic/order permits it.

The harness must include at least one analytic localized wave packet and one discrete reference case. Spatial/time refinement is required for any finite-difference result. No physical parameter is to be selected by tuning the test to pass.

## 8. Numerical-check progress

Implemented in `tests/test_ambient_spatial_composition.py`:
- Nonzero residual for a spatially uniform 440-cycle carrier under the massless wave equation.
- Pointwise additive energy identity, including the cross-energy term.
- Pointwise additive flux identity, including the cross-flux term.
- Full-day scan of the envelope correction; the maximum relative correction is approximately `1.30746e-6` (1.31 ppm) at tick 14,340 for the selected 1:440 envelope/carrier ratio.
- Accounting-patch area scaling of integrated inventory while holding local density fixed.
- Deterministic replay of the analytical diagnostics.

These tests are algebraic/reference checks, not a coupled field simulation. Still outstanding are the analytic localized wave-packet case, a discrete patch-boundary flux/work balance over nested patches, an independent-mode noninteraction control, and any reciprocal coupled-Hamiltonian candidate. The existing tests must pass the exact-head CI run before being treated as validated.

## 9. Exit criteria and next decision

This gate passes only when:
- the additive cross-energy and cross-flux identities are numerically verified;
- the uniform-carrier equation residual is measured;
- the envelope-energy correction is quantified and its interpretation explicitly decided;
- accounting-patch changes do not alter local physics;
- the independent-mode control is shown to be additive but non-interacting;
- any proposed reciprocal coupling has a stated Hamiltonian, energy inventory, and dispersion analysis before implementation.

The outcome may be that the selected ambient mode must remain a separate environmental degree of freedom rather than a component of the scalar displacement. That is an acceptable result if it is the only construction consistent with the approved nondispersive disturbance law and auditable energy ownership.

**Production integration remains blocked** until the exit criteria pass and a composition/coupling model is explicitly approved.


## 10. Localized packet and patch-balance reference (implementation added; CI pending)

The next reference increment adds `tests/test_ambient_packet_patch_balance.py` with:

- A localized Gaussian traveling packet, an exact right-moving solution of the one-dimensional massless wave equation. Its PDE residual is checked independently with centered finite differences.
- An explicit uniform harmonic carrier added to that packet only as a forced degree of freedom, with the forcing `b''(t)` retained.
- A pointwise combined-energy check that includes the ambient/disturbance cross term.
- A discrete balance over three nested accounting patches. The test evaluates combined field energy, outward boundary flux, and signed ambient forcing work using centered differences and trapezoidal quadrature at two spatial/time resolutions. It requires the normalized residual to remain below 2% at both resolutions. Monotonic residual reduction is deliberately not required because the independently discretized endpoint, boundary, and volume terms can exhibit different cancellation errors.
- A patch-invariance control showing that changing the integration patch does not alter analytic packet samples.
- An independent-mode control: oscillator inventory is additive with disturbance inventory, and changing ambient phase does not change the disturbance state.

The reference remains one-dimensional and is not a production solver. Its role is to establish the accounting identity and nested-patch behavior before extending to higher-dimensional packet geometry or proposing a reciprocal coupling law.

**Validation status:** The tests have been committed to this PR branch, but they are not considered validated until the complete exact-head CI run passes. The envelope correction interpretation and a reciprocal coupled-Hamiltonian model remain open decisions. No production integration is authorized.


## 11. First reciprocal-coupling candidate: bulk local oscillator field (analysis only)

A minimal reciprocal Hamiltonian candidate can be written with a disturbance field (w(\mathbf{x},t)) and a local ambient oscillator coordinate (q(\mathbf{x},t)):

\[
\mathcal H =
\frac{\pi_w^2}{2K}
+\frac{Kc^2}{2}|\nabla w|^2
+\frac{\pi_q^2}{2M}
+\frac{M\Omega^2}{2}q^2
+gqw.
\]

Here (K,M>0), and (g) is a reciprocal coupling coefficient. The Hamilton equations give

\[
w_{tt}=c^2\nabla^2w-\frac{g}{K}q,
\qquad
q_{tt}+\Omega^2q=-\frac{g}{M}w.
\]

For a plane-wave mode, the coupled dispersion relation is

\[
(c^2k^2-\omega^2)(\Omega^2-\omega^2)
-\frac{g^2}{KM}=0.
\]

For any nonzero (g), the disturbance branch is no longer exactly \(\omega^2=c^2k^2\). This is a direct demonstration that a bulk, pointwise reciprocal coupling between a local ambient oscillator field and the disturbance field changes the dispersion relation. It is not an acceptable coupling if exact nondispersive propagation must hold everywhere.

There is a second, stronger defect in this minimal bilinear model: its quadratic Hamiltonian is not bounded below for any nonzero \\(g\\). At zero spatial wave number, the dispersion polynomial has one negative \\(\\omega^2\\) root, corresponding to an exponentially growing mode. Thus this is a diagnostic no-go example, not a stable coupling candidate. Stabilizing it would require changing the interaction or adding further physical terms; those changes must be derived and re-tested rather than inserted as numerical fixes.

This candidate is not selected. It identifies a constraint for subsequent work:

- If the coupling exists throughout empty space, exact massless nondispersive propagation is lost for nonzero reciprocal coupling.
- If the coupling is restricted to physical material locations, empty-space propagation can remain the approved massless equation, but the material interface and local hybrid response require their own conservative derivation and tests.
- A finite accounting patch must never be used as the coupling support. Any localized coupling profile must correspond to a separately defined physical material/support, not the bookkeeping region.
- The oscillator field's energy density can be finite locally while its integral over an infinite uniform plane is infinite. Whole-plane inventory therefore needs a density-based or finite-excess formulation; the finite accounting patch remains a reporting window only.

The next viable question is therefore not “how do we couple the ambient mode everywhere without changing dispersion?” The bulk candidate shows that this cannot be done with this minimal local Hamiltonian. The next candidate to analyze is a conservative, material-local interaction, with the ambient field treated as an independently owned local degree of freedom and with exact equal-and-opposite work at the interface. This remains analytical work only; no material coupling is implemented or authorized.


## 12. Stable derivative-coupling comparison (analysis only)

A second reciprocal interaction couples the ambient coordinate to the disturbance velocity:

\[
\mathcal L =
\frac K2(w_t^2-c^2|\nabla w|^2)
+\frac M2(q_t^2-\Omega^2q^2)
+gq w_t.
\]

Its canonical Hamiltonian density is

\[
\mathcal H =
\frac{(\pi_w-gq)^2}{2K}
+\frac{\pi_q^2}{2M}
+\frac{Kc^2}{2}|\nabla w|^2
+\frac{M\Omega^2}{2}q^2.
\]

For positive (K,M), this expression is bounded below. The coupled equations are

\[
w_{tt}=c^2\nabla^2w-\frac gKq_t,
\qquad
q_{tt}+\Omega^2q=\frac gM w_t,
\]

and the plane-wave dispersion relation becomes

\[
(c^2k^2-\omega^2)(\Omega^2-\omega^2)
-\frac{g^2\omega^2}{KM}=0.
\]

This avoids the instability of the bilinear displacement coupling, but nonzero bulk coupling still changes the disturbance dispersion. It therefore also fails the requirement of exact nondispersive propagation throughout empty space.

A potentially admissible *material-local* version would set the coupling to zero in empty space and activate it only through a separately defined physical material interface. Then the empty-space equation remains massless and nondispersive, while energy can exchange reciprocally at the interface. That construction still requires a spatially varying Hamiltonian, interface matching conditions, a proof of equal-and-opposite work, and discrete conservation/convergence tests. It is a candidate for further derivation, not an approved model or implementation.


## 13. Material-local derivative coupling — conservative reference derivation (analysis only)

The stable derivative interaction from Section 12 can be localized by a physical material-support function \(g(\mathbf{x})\), with \(g=0\) everywhere in empty space. Let \(q(\mathbf{x},t)\) be an independently owned local ambient oscillator coordinate with positive inertia density \(M\) and restoring frequency \(\Omega\). Consider

\[
\mathcal L=\int \left[
\frac K2(w_t^2-c^2|\nabla w|^2)
+\frac M2(q_t^2-\Omega^2q^2)
+g(\mathbf{x})q w_t
\right]dA.
\]

The coupling profile is tied to physical material occupancy/geometry only. It must never be derived from the finite accounting patch. For a static support profile, the Euler–Lagrange equations are

\[
w_{tt}=c^2\nabla^2w-\frac{g(\mathbf{x})}{K}q_t,
\qquad
q_{tt}+\Omega^2q=\frac{g(\mathbf{x})}{M}w_t.
\]

Consequences:

- In empty space, \(g=0\), so disturbance propagation remains exactly the approved massless equation \(w_{tt}=c^2\nabla^2w\).
- The local oscillator and disturbance exchange energy only where the physical support function is nonzero.
- A static, discontinuous support profile introduces no derivative of \(g\) into either equation. With constant \(K,c\), the field displacement and normal derivative retain the ordinary continuity conditions across a finite material interface; there is no artificial wave boundary at the edge of the accounting patch.
- The local coupled response is not claimed to be dispersion-free inside material. The requirement is unchanged propagation in empty space, with material-local response treated as physical behavior to validate separately.

The canonical Hamiltonian density is

\[
\mathcal H=
\frac{(\pi_w-gq)^2}{2K}
+\frac{\pi_q^2}{2M}
+\frac{Kc^2}{2}|\nabla w|^2
+\frac{M\Omega^2}{2}q^2.
\]

For \(K,M>0\), it is bounded below. The corresponding uncoupled field-plus-oscillator energy density

\[
e=\frac K2(w_t^2+c^2|\nabla w|^2)
+\frac M2(q_t^2+\Omega^2q^2)
\]

obeys

\[
\partial_t e+\nabla\cdot\mathbf J=0,
\qquad
\mathbf J=-Kc^2w_t\nabla w,
\]

for a time-independent \(g(\mathbf{x})\) and no world-generation source. The exchange terms cancel pointwise:

\[
\partial_t e_w+\nabla\cdot\mathbf J
=-g(\mathbf{x})q_tw_t,
\qquad
\partial_t e_q=+g(\mathbf{x})q_tw_t.
\]

This is the desired equal-and-opposite local work identity. It is a stronger candidate than bulk coupling because it is stable, conservative, and identically absent from empty space. It is still only an analytical candidate.

### Mandatory unresolved source-accounting issue

The selected ambient mode is intrinsically world-generated and has a time-varying target energy-density envelope. The derivation above is source-free. Adding a world-generation drive \(s_{world}(\mathbf{x},t)\) to the oscillator equation,

\[
M(q_{tt}+\Omega^2q)=g(\mathbf{x})w_t+s_{world},
\]

adds signed source work \(s_{world}q_t\) to the ambient energy balance. The existing ambient-mode schedule currently accounts signed changes in stored mode energy; it does not yet establish a force law whose work remains correct while local material coupling is exchanging energy. Therefore the source drive, generation/withdrawal ledger, and coupling exchange must be derived together before this candidate can be considered complete. Do not simply prescribe the old amplitude envelope on top of a coupled oscillator, because that can silently inject or remove the material-exchanged energy twice.

### Required discrete reference tests before selection

1. **Empty-space invariance:** Set \(g=0\) outside material support and show the disturbance solver's empty-space update is exactly the uncoupled massless update.
2. **Localized support:** Use a compact, physically defined support profile; move or resize that profile and show only the corresponding material response changes. The accounting patch must not affect \(g\).
3. **Interface behavior:** Test smooth and sharp support profiles. Verify displacement/normal-flux matching and absence of spurious reflection attributable solely to the accounting-window edge.
4. **Equal-and-opposite work:** Measure field exchange \(-\int gq_tw_t\,dA\) and oscillator exchange \(+\int gq_tw_t\,dA\) using identical spatial quadrature; require cancellation to the reference integrator's stated tolerance.
5. **Positive energy:** Verify the canonical Hamiltonian is nonnegative for randomized finite states and positive \(K,M\).
6. **Discrete conservation:** Start with no world drive and verify total field-plus-oscillator energy conservation on a periodic numerical reference domain before a packet reaches the wrap boundary. Use an energy-consistent integrator or derive the exact discrete work pairing; do not hide integration error in a fitted correction.
7. **Convergence and controls:** Refine space and time independently; compare \(g=0\), localized \(g\ne0\), and a deliberately bulk-coupled negative control. Confirm empty-space propagation converges to the massless reference while localized exchange converges to the analytical work identity.
8. **Driven accounting:** Only after source-free exchange passes, add the world-generation drive and demonstrate that generation, withdrawal, material exchange, outgoing flux, and numerical residual reconcile without double counting.

**Decision:** This derivation identifies a viable next candidate for isolated testing, not an approved physical law. The ambient source-work issue and all discrete tests remain open. No production solver, material, chemistry, organism, or GUI integration is authorized.


### Exploratory finite-dimensional conservation spot-check (not repository CI)

A standalone one-dimensional periodic finite-difference system was formed from the source-free equations, with a compact coupling profile nonzero only on ten adjacent cells. The semi-discrete linear system was advanced for 500 steps using implicit midpoint, which preserves quadratic Hamiltonians for this linear conservative system.

- Grid: 80 cells, spacing 1; propagation speed 1.
- Oscillator frequency: 1.7; time step: 0.02.
- Coupling: 0.35 on cells 30–39, zero elsewhere.
- Initial state: a localized disturbance packet and a separate localized oscillator perturbation.
- Total energy: 0.19689581638926476 initially and 0.19689581638926898 after 500 steps.
- Relative energy residual: 2.14e-14 (absolute residual 4.22e-15).

This confirms that the proposed source-free semi-discrete equations admit a compact-support, positive-energy, conservative finite-dimensional reference under an energy-preserving integrator. It does not validate the continuum interface limit, the existing ambient world-generation drive, the production leapfrog scheme, or any repository test. The result is exploratory and must be reproduced in the isolated repository harness with spatial/time refinement before it can count toward the gate.


## 14. Candidate world-generation drive with reciprocal exchange (analysis only)

The source-work issue admits a concrete candidate that does not clamp the coupled oscillator back to its target amplitude. Let the selected, uncoupled ambient reference trajectory be

\[
q_0(t)=a(t)\cos(\Omega t+\phi).
\]

Define a predetermined world-drive term from that reference trajectory:

\[
s_0(t)=M\left(q_0''(t)+\Omega^2q_0(t)\right)
=M\left[a''(t)\cos(\Omega t+\phi)-2\Omega a'(t)\sin(\Omega t+\phi)\right].
\]

Then use the driven, material-local oscillator equation

\[
M(q_{tt}+\Omega^2q)=g(\mathbf{x})w_t+s_0(t).
\]

The drive is derived once from the approved ambient reference envelope and is not adjusted to cancel material exchange. In empty space/material-free locations, where \(g=0\), the reference trajectory \(q=q_0\) is an exact solution. Where material coupling is active, the actual \(q\) state may depart from \(q_0\); that departure is the reciprocal response and must not be erased by amplitude clamping.

The local energy identity becomes

\[
\partial_t(e_w+e_q)+\nabla\cdot\mathbf J=s_0(t)q_t.
\]

The equal-and-opposite material exchange terms still cancel. The remaining signed term is the work performed by the intrinsic world drive on the actual ambient oscillator state. Therefore the world-generation/withdrawal ledger must accumulate \(s_0q_t\), while material exchange is reported separately as equal-and-opposite internal transfer. This avoids counting material exchange as world generation.

For the uncoupled reference trajectory, the fast-carrier-averaged oscillator energy is

\[
\langle e_q\rangle=\frac{M\Omega^2a^2}{2}+\frac{M(a')^2}{4},
\]

and the averaged drive power is the time derivative of this full expression. The second term is the previously measured envelope-kinetic correction. If the declared 1–10 energy-density schedule is intended to be exact total oscillator energy, the amplitude envelope must be solved against this corrected expression; if the existing carrier-only amplitude mapping is retained, the 1.31-ppm maximum correction remains an explicit approximation. This decision must be made before calibrating the drive.

This construction is a candidate, not a validated implementation. Required follow-up checks are: (1) verify that \(q_0\) is recovered when \(g=0\); (2) verify that integrated \(s_0q_t\) equals the uncoupled oscillator-energy change; (3) enable localized \(g\) and verify the material exchange cancels while world work remains the only external term; (4) verify that coupling-induced deviations in \(q\) are not overwritten; and (5) reconcile generation, withdrawal, material exchange, outgoing disturbance flux, and numerical residual over nested accounting patches.

**No production source, solver, material, chemistry, or organism integration is authorized.**


## 15. Isolated finite-dimensional material-local coupling harness (implementation added; CI pending)

Added `tests/test_material_local_ambient_coupling.py`, a standard-library-only reference harness that does not import or modify the production solver. It discretizes the source-free material-local derivative-coupling equations on a small periodic one-dimensional grid, using a centered Laplacian and an implicit-midpoint integrator. The discrete energy uses the matching forward-difference gradient, so summation by parts makes the semi-discrete field operator energy-skew and the coupling work cancels exactly between field and oscillator.

The harness checks:

1. Nonnegative Hamiltonian energy for randomized finite states with positive field/oscillator parameters.
2. Coupling-operator support is exactly the specified compact physical-material cells; every coupling entry is zero outside that support.
3. Source-free total energy conservation over 500 implicit-midpoint steps.
4. Driven, coupled total-energy reconciliation: the only net external work is the signed world-drive integral `sum(dt * s_world * q_t_midpoint)`. Internal material exchange is not added to world generation, and the coupled oscillator is not clamped back to its uncoupled target trajectory.
5. The intrinsic drive recovers the prescribed uncoupled reference trajectory with second-order convergence under time-step halving, while its signed work closes the oscillator energy balance.
6. Three nested accounting patches close their local energy balance using the discrete outward field flux and signed world-drive work. The patch energy assigns half of each adjacent field-gradient edge to each neighboring cell; material exchange cancels pointwise and is not counted as external work.
7. Spatial refinement evolves a right-moving packet through both a grid-aligned sharp compact-support coupling region and a smooth compact-support profile. A 256-cell reference is compared at common physical coordinates against 32-, 64-, and 128-cell runs at the same final time. Both displacement and oscillator response are included in the error norm; each doubling must reduce error by more than a factor of three, consistent with second-order spatial convergence. The reference is numerical, not an exact continuum solution. A separate fixed-grid temporal refinement test compares RK4 steps with successive halvings against a finer time-step reference; the expected fourth-order reduction is tested independently of spatial refinement.

Exploratory reference measurements (same final time 5.5; spatial reference 256 cells; temporal reference step ratio 0.025) were:

| Profile / refinement | Coarse error | Next error | Fine error | Reduction ratios |
|---|---:|---:|---:|---:|
| Sharp spatial, 32/64/128 | 0.0431446 | 0.0117775 | 0.00352893 | 3.66×, 3.34× |
| Smooth spatial, 32/64/128 | 0.0439967 | 0.0118004 | 0.00349133 | 3.73×, 3.38× |
| Fixed-grid temporal, ratios 0.4/0.2/0.1 | 9.7869e-5 | 5.6756e-6 | 3.4463e-7 | 17.24×, 16.47× |

These values are exploratory outputs from the reference algorithm and are not production measurements. The committed CI assertions require spatial reductions greater than 3× and temporal reductions greater than 12×.

An additional periodic amplitude-inversion test now verifies that the declared 1–10 schedule can be matched as exact carrier-averaged total oscillator energy, including envelope-kinetic energy, to below a 1e-12 energy residual. The workflow on the parent branch now conditionally runs this test module when the file is present, so the stacked PR #438 executes it without making the parent PR #437 depend on a file that exists only in the child PR. The test result is pending exact-head CI. This is a finite-dimensional reference only: it does not establish the continuum interface limit, a production integrator, a physical coupling coefficient, or a selected world-generation law. Production integration remains blocked.


## 16. Exact average-energy envelope inversion (reference only)

The open schedule interpretation can be handled without accepting the 1.31-ppm carrier-only energy discrepancy. The recommended interpretation is that the declared 1–10 schedule is the exact *carrier-averaged total oscillator energy*, including slow-envelope kinetic energy:

\[
E_{target}(t)=\frac{M\Omega^2a(t)^2}{2}+\frac{M\dot a(t)^2}{4}.
\]

Starting from the carrier-only estimate \(a_0=\sqrt{2E_{target}/(M\Omega^2)}\), solve the small correction by periodic fixed-point iteration:

\[
a_{n+1}(t)=\sqrt{\frac{2}{M\Omega^2}\left(E_{target}(t)-\frac{M}{4}\dot a_n(t)^2\right)}.
\]

A periodic five-point derivative on 24,000 daily samples was used in the isolated reference. With \(M=1\), \(\Omega=2\pi\,440/24000\), and \(E_{target}=5.5-4.5\cos(2\pi t/24000)\), four fixed-point iterations produced:

- Maximum energy-schedule residual: approximately \(3.55\times10^{-15}\) energy units.
- Maximum relative amplitude correction from the carrier-only estimate: \(6.54\times10^{-7}\), or 0.654 ppm.
- The correction remains positive across the full 1–10 schedule.

The repository harness now includes this periodic inversion as a reference test, requiring energy residual below \(10^{-12}\) and amplitude correction below 1 ppm. It also includes a finite-propagation control: before a packet reaches a distant compact material support, localized coupling matches the vacuum reference to below \(10^{-9}\), while a deliberately incorrect bulk-coupled negative control measurably changes nominal empty-space propagation. This supports exact carrier-averaged schedule accounting without changing the selected ambient mode or adding a material-dependent source adjustment. The derived \(s_0(t)=M(q_0''+\Omega^2q_0)\) must still be integrated against the *actual* coupled \(q_t\) state, not a clamped reference amplitude. This remains isolated calibration mathematics, not production behavior.

**Recommendation:** retain the 1–10 values as exact carrier-averaged total oscillator energy and use the corrected periodic amplitude. This removes the known envelope-energy approximation instead of preserving it as a permanent tolerance. The physical coupling candidate remains unselected until its complete interface and source-work validation passes.
