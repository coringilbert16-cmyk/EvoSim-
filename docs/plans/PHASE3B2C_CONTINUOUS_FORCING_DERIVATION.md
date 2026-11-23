# Phase 3B.2c — Continuous Work-Conjugate Forcing Derivation

**Status:** Candidate A's isolated single-source forcing step is implemented in the extracted reference solver; numerical verification is pending CI. No physical source law is approved.

**Authority:** Phase 1 unified energy/continuous wave-field specification; the controlling [resonance-field implementation roadmap](../design/resonance-field-implementation-roadmap.md); the Phase 3B.2 source-interface proposal; merged PR #426 and PR #427.

## 1. Scope and completed prerequisites

This investigation begins after the following work and must not repeat it:

- **PR #426 (merged):** normalized compact-support radial-cosine hotspot profile, analytic cumulative radial power, and physical-area integration with normalization, refinement, translation, and grid-alignment tests.
- **PR #427 (merged):** stable source identity/provenance, immutable source descriptors, finite non-clonable source accounts, signed staged transfers with atomic commit, and an isolated extracted reference solver.
- **Phase 2 / PR #410:** validated isolated leapfrog reference solver remains closed and unmerged. PR #427 carries a separate extracted solver copy; do not reopen or merge #410.

Neither #426 nor #427 implements continuous field forcing. PR #427's existing profile injection changes a field state at an instant; it does not continuously drive the wave equation during propagation.

## 2. Governing continuum work identity

The approved empty-space scalar wave equation is

\[
u_{tt}=c^2\nabla^2u+f(\mathbf{x},t),
\]

with field energy density and flux

\[
e=\frac K2\left(u_t^2+c^2|\nabla u|^2\right),
\qquad
\mathbf J=-Kc^2u_t\nabla u.
\]

The local balance is

\[
\partial_t e+\nabla\cdot\mathbf J=K u_t f.
\]

For source \(s\), the continuum signed work over one interval is

\[
W_s=\int_{t_n}^{t_{n+1}}\int_\Omega K u_t f_s\,dA\,dt.
\]

This is the work conjugate to the field forcing. It is not equal by definition to a source's incident power density or to the square of field amplitude.

## 3. Exact discrete identity for the current leapfrog solver

The extracted reference solver uses the centered update (without damping)

\[
u_i^{n+1}=2u_i^n-u_i^{n-1}
+c^2\Delta t^2(L_hu^n)_i
+\Delta t^2\sum_s f_{s,i}^n,
\]

where \(L_h\) is the five-point discrete Laplacian. Its validated discrete energy is

\[
E_h^n=\frac K2\left[
\Delta x^2\sum_i\left(\frac{u_i^n-u_i^{n-1}}{\Delta t}\right)^2
+c^2\sum_{\langle i,j\rangle}
(u_i^n-u_j^n)(u_i^{n-1}-u_j^{n-1})
\right].
\]

Multiply the forced recurrence by the centered discrete velocity

\[
v_i^{n+1/2}=\frac{u_i^{n+1}-u_i^{n-1}}{2\Delta t}
\]

and sum over interior grid nodes. Discrete summation by parts makes the Laplacian contribution the difference of the cross-time gradient terms in \(E_h\). Therefore, in an undamped domain with no boundary flux crossing the measurement boundary,

\[
\boxed{
E_h^{n+1}-E_h^n
=K\Delta t\,\Delta x^2
\sum_i v_i^{n+1/2}\sum_s f_{s,i}^n
}
\]

and the additive per-source work attribution is

\[
\boxed{
W_{s,h}^n=K\Delta t\,\Delta x^2
\sum_i v_i^{n+1/2}f_{s,i}^n,
\qquad
\sum_s W_{s,h}^n=E_h^{n+1}-E_h^n.
}
\]

Equivalently, \(W_{s,h}^n=(K\Delta x^2/2)\sum_i f_{s,i}^n(u_i^{n+1}-u_i^{n-1})\). The sign convention matches PR #427: positive work increases field energy and debits the source; negative work returns energy to that source.

This identity is exact for the stated undamped leapfrog update, up to floating-point summation error. It is not merely a continuum quadrature approximation. For additive sources, the same midpoint field velocity attributes work linearly to each forcing term; the sum of source works equals the measured discrete field-energy change, provided all source settlements are staged and committed together.

The extracted solver's actual discrete invariant uses the same cross-time gradient term. Its centered sponge recurrence is
\[
(1+\sigma_i\Delta t/2)u_i^{n+1}
=2u_i^n-(1-\sigma_i\Delta t/2)u_i^{n-1}
+c^2\Delta t^2(L_hu^n)_i+\Delta t^2 f_i^n.
\]
The exact combined balance is therefore
\[
\boxed{E_h^{n+1}-E_h^n
=K\Delta t\Delta x^2\sum_i v_i^{n+1/2}f_i^n
-K\Delta x^2\Delta t\sum_i\sigma_i(v_i^{n+1/2})^2.}
\]
The second term is exactly the solver's already-accounted sponge absorption. Thus source forcing and damping can operate in the same substep without conflating source work with boundary loss. The isolated implementation validates the measured field-energy change against signed source work minus the increment in boundary-absorbed energy.

This identity assumes fixed-zero outer nodes and the solver's symmetric five-point stencil; the discrete summation-by-parts boundary contribution vanishes because the boundary nodes are fixed at zero. It does not establish behavior for a physical open boundary or a different spatial operator.

## 4. Candidate A — prescribed acceleration forcing

A mathematical source can prescribe

\[
f_s(\mathbf{x},t)=a_s(t)g_s(\mathbf{x}),
\qquad
\int g_s(\mathbf{x})\,dA=1,
\]

with the spatial profile discretized using the same physical-area quadrature as the hotspot integration. The scalar \(a_s(t)\) and profile \(g_s\) define acceleration forcing; they do not directly prescribe source power.

For each substep:
1. Validate all source descriptors, accounts, profiles, and the current field state.
2. Form the additive forcing terms in stable source-ID order.
3. Stage the forced leapfrog update without mutating the field or accounts.
4. Evaluate the exact discrete invariant before and after the update.
5. Calculate each source's signed work with the discrete midpoint-velocity formula above.
6. Stage all source-account settlements; reject the complete multi-source update if any account overdraws or any value/residual is invalid.
7. Commit the field and every account atomically.

**Advantages:** Directly compatible with the current leapfrog recurrence; exact discrete work accounting; deterministic; supports positive and negative work; gives a narrow isolated numerical experiment.

**Limitations:** The field's instantaneous power is emergent:

\[
P_s^n=K\Delta x^2\sum_i v_i^{n+1/2}f_{s,i}^n.
\]

It depends on the field phase and local velocity. It can be zero at velocity nodes or negative when the field does work on the source. Therefore a prescribed \(a_s(t)\) cannot be interpreted as a prescribed positive incident power \(P_h(t)\) without an additional physical source model. Dividing desired power density by \(K u_t\) is singular at zero-velocity nodes and is rejected. Clamping negative work, forcing only when velocity has a chosen sign, or adding an arbitrary efficiency would alter the physics and is not approved.

Candidate A is suitable for validating the discrete forcing/work interface with a finite mathematical test source. It is not yet a physical hotspot law.

## 5. Candidate B — reciprocal source degree of freedom

A physically reciprocal candidate is to represent the source as an explicit powered degree of freedom coupled to the field. One illustrative continuum Hamiltonian model is:

\[
U_s(t)=\int g_s(\mathbf{x})u(\mathbf{x},t)\,dA,
\qquad \int g_s\,dA=1,
\]

\[
E_{\mathrm{source}}=\frac M2(\dot q_s^2+\omega_s^2q_s^2),
\qquad
E_{\mathrm{coupling}}=\frac\kappa2(q_s-U_s)^2.
\]

The coupled equations are

\[
u_{tt}=c^2\nabla^2u+
\frac\kappa K(q_s-U_s)g_s(\mathbf{x}),
\]

\[
M\ddot q_s+M\omega_s^2q_s+\kappa(q_s-U_s)=0.
\]

With compatible boundary conditions, the total of field energy, source-oscillator energy, and coupling potential energy is conserved. The field forcing is reciprocal: the field acts back on the source rather than treating the source waveform as an inexhaustible prescribed trajectory.

**Advantages:** A source has explicit internal energy and phase/frequency dynamics; field energy transfer is reversible; no division by local field velocity is required; energy exchange follows from one coupled model.

**Unresolved costs and risks:**
- The coupling potential is a real stored-energy term and must appear in the unified inventory; it cannot be hidden inside a field/source debit.
- The source account semantics must specify whether it owns oscillator energy only or the oscillator plus coupling energy, without double counting.
- A discrete coupled integrator must preserve or accurately reconcile the combined Hamiltonian; applying the existing field leapfrog unchanged while updating the oscillator separately is not automatically conservative.
- The mapping from the source's physical power/budget and normalized hotspot footprint to \(M,\omega_s,\kappa,q_s\) requires derivation and calibration. No parameter values are selected here.
- This is a candidate for a powered in-plane hotspot, not an approved model and not a sun model.

Candidate B is more physically complete but more involved. It should be compared against Candidate A before choosing an implementation direction.

## 6. Critical distinction: hotspot incident power versus field forcing

PR #426 establishes a normalized spatial profile \(g_h(r)\) with

\[
\int g_h(r)\,dA=1.
\]

The source-interface proposal defines a possible incident-power distribution

\[
q_h(\mathbf{x},t)=P_h(t)g_h(r).
\]

Here \(q_h\) has units \(E/(L^2\,tick)\), while the forcing \(f\) in the wave equation has units of field displacement per tick squared. They are not interchangeable.

The normalized hotspot profile can be reused as a spatial coupling/forcing shape, but this alone does not establish how much energy enters the field. A physical law must connect the source's owned energy and dynamics to signed work \(K u_t f\), including possible returned work. The total source power must emerge from or be enforced by that law, not by assigning the same energy independently to grid cells or receivers.

## 7. Required comparison and numerical experiments

No production code or real-source integration is authorized by this investigation. The next isolated experiment should compare the candidates under matched conditions.

### 7.1 First, validate Candidate A's exact discrete work identity

- **Implemented, pending CI:** an isolated one-source acceleration-forcing substep on the PR #427 extracted reference solver. It stages the forced update, computes signed work from the actual midpoint velocity, checks the exact combined source-work/sponge-loss identity, stages the finite source-account transfer, and commits only after validation. It does not touch production simulation paths.
- Focused tests cover positive work, negative work returned to a zero-budget source, simultaneous forcing and sponge absorption, insufficient-budget rollback, and rejection of nonzero forcing on fixed boundary nodes.
- Implemented and covered by tests: zero-work at a centered velocity node, force sign reversal, multiple simultaneous sources with per-source work attribution, duplicate-source rejection, and all-account rollback when any source overdraws. Deterministic replay remains to be tested.
- Added, pending CI: a paired spatial/time-refinement test compares discrete work against the analytic continuum work integral for a smooth nonseparable profile and requires second-order convergence. Further convergence studies across source profiles and longer forcing intervals remain required. \(\int\!\int K u_t f\,dA\,dt\) under paired spatial/time refinement. Report convergence order and normalized residual; do not claim continuum equivalence from a single grid.

### 7.2 Then evaluate Candidate B

- Derive the discrete coupled field/source update from one discrete total-energy function.
- Include coupling-potential energy explicitly in the inventory.
- Verify source/field energy exchange and total conservation over many periods.
- Test phase, frequency, coupling strength, spatial refinement, time refinement, and deterministic replay.
- Compare its emitted spatial field and measured average source work with Candidate A under controlled initial conditions.

### 7.3 Physical hotspot decision gate

Before selecting either model for a real hotspot, specify:
- Whether the hotspot is a prescribed active power source, a finite powered oscillator, or another explicitly modeled physical source.
- Its authoritative energy owner and any replenishment origin/schedule.
- Whether and how it can absorb returned field work.
- How the PR #426 normalized spatial profile enters the coupling law.
- How physical power, waveform, frequency, and source budget map to the model's parameters.
- The complete energy inventory, including any coupling energy.
- A refinement/convergence test demonstrating that emitted field work is independent of grid alignment and material/receiver count.

No live hotspot or sun injection is authorized until this gate passes review.

## 8. Current conclusion and next action

The exact discrete work identity for additive acceleration forcing can be derived directly from the existing leapfrog invariant. This provides a sound, narrowly scoped next numerical test and avoids repeating the already merged source-account and hotspot-normalization work.

However, Candidate A is an accounting-compatible mathematical forcing mechanism, not yet a physical incident-power law. Candidate B offers a reciprocal physical source model but introduces a coupling-energy inventory and a coupled-integrator requirement. The present evidence does not justify choosing one as the production hotspot law.

**Next action:** Deterministic replay is complete and merged in PR #430. Extend Candidate A continuum validation with sustained forcing over a fixed physical-time interval, multiple smooth profiles, and paired spatial/time refinement against analytic modal solutions. Report normalized work error and observed order. Then validate repeated multi-source attribution with mixed signed work and active sponge absorption. Only after these Candidate A gates should Candidate B's reciprocal source oscillator be implemented as an isolated comparison. Keep hotspot power mapping unresolved and do not connect either candidate to the live simulation.
