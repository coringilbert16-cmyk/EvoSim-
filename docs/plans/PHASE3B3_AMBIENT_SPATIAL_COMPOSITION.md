# Phase 3B.3 — Ambient/Disturbance Spatial Composition

**Status:** Initial algebraic/numerical identities implemented in an isolated Python test module; full patch-flux validation remains open. No composition law selected and no production coupling authorized.

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
