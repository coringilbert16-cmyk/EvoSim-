# Phase 3B.3 — Material-local coupling on the leapfrog reference

**Status:** A separate staggered-leapfrog reference harness has been added alongside the implicit-midpoint harness. Its isolated calculations pass; exact-head repository CI remains pending. This is a finite-dimensional numerical reference, not a selected physical coupling law or production implementation.

## 1. Why this gate was added

The first material-local coupling harness used implicit midpoint for exact conservation of the finite-dimensional quadratic Hamiltonian and RK4 for spatial/time convergence. That was useful, but did not test the candidate using the second-order staggered wave update used by the isolated Phase 2 solver.

This addendum tests the same derivative coupling with a leapfrog-compatible update. It does not replace the implicit-midpoint reference; the two integrators provide independent checks.

## 2. Discrete equations

The continuum candidate remains

\[
w_{tt}=c^2\nabla^2w-\frac{g(x)}{K}q_t,
\qquad
q_{tt}+\Omega^2q=\frac{g(x)}{M}w_t+\frac{s_0(t)}{M}.
\]

Here \(g(x)\) is a static compact material-support profile, zero in empty space. The world drive \(s_0\) is prescribed from the uncoupled ambient reference trajectory; it is not adjusted to cancel material exchange.

Use positions \(w^n,q^n\) at integer times and velocities \(v_w^{n-1/2},v_q^{n-1/2}\) at staggered half times. The kick is

\[
v_w^{n+1/2}-v_w^{n-1/2}
=\Delta t\left(c^2\Delta_h w^n
-\frac{g}{K}\frac{v_q^{n+1/2}+v_q^{n-1/2}}{2}\right),
\]

\[
v_q^{n+1/2}-v_q^{n-1/2}
=\Delta t\left(-\Omega^2q^n
+\frac{g}{M}\frac{v_w^{n+1/2}+v_w^{n-1/2}}{2}
+\frac{s_0(t_n)}{M}\right).
\]

The two local velocity equations are solved together as a 2-by-2 system at each grid point. The drift is

\[
w^{n+1}=w^n+\Delta t\,v_w^{n+1/2},
\qquad
q^{n+1}=q^n+\Delta t\,v_q^{n+1/2}.
\]

The drive is evaluated at integer time \(t_n\), and its work uses the centered oscillator velocity at that same time:

\[
W_n=\Delta t\,\Delta x\sum_i
s_0(t_n)\frac{v_{q,i}^{n+1/2}+v_{q,i}^{n-1/2}}{2}.
\]

The staggered discrete energy is

\[
E^{n+1/2}=\Delta x\sum_i\left[
\frac K2\left((v_{w,i}^{n+1/2})^2
+c^2 D^+w_i^nD^+w_i^{n+1}\right)
+\frac M2\left((v_{q,i}^{n+1/2})^2
+\Omega^2q_i^nq_i^{n+1}\right)
\right].
\]

This is the leapfrog cross-time invariant, not an instantaneous continuum-energy estimate. In the source-free periodic reference, material exchange cancels and this discrete energy is conserved. With the world drive, the only net external contribution is the sum of \(W_n\).

## 3. Isolated reference settings

Harness: tests/test_material_local_ambient_leapfrog.py. It uses only the Python standard library and does not import production code.

- One-dimensional periodic grid, length 16.
- \(c=1\), \(K=1.3\), \(M=0.8\), \(\Omega=1.7\).
- Sharp support: \(g=0.35\) on \([4,8)\), zero elsewhere.
- Smooth support: compact smoothstep shoulders around the same central region.
- The packet begins near \(x=2\), with right-moving initial velocity.
- World drive uses \(a(t)=1+0.15\cos(0.4t)\) and phase \(\Omega t+0.2\).
- Periodic wrap is a numerical reference boundary, not a proposed physical world edge.

## 4. Reference measurements

These are exploratory outputs from the isolated leapfrog reference algorithm, not production measurements. Exact-head CI remains the required repository validation.

### Source-free energy and driven work

- Source-free compact-support case, 80 cells, \(\Delta t=0.02\), 500 steps: absolute discrete-energy residual approximately \(2.22\times10^{-14}\).
- Driven vacuum case: absolute \(E_f-E_i-W\) residual approximately \(1.95\times10^{-14}\).
- Driven sharp-support case: absolute \(E_f-E_i-W\) residual approximately \(1.99\times10^{-13}\).
- Positive world work and negative world work are tracked separately. Internal material exchange is not added to either world counter.
- The coupled oscillator departs from the uncoupled reference trajectory; it is not amplitude-clamped after exchanging energy with the disturbance.

### Spatial refinement

A 256-cell run is the numerical reference. Errors compare displacement and oscillator coordinates at common physical positions.

| Profile | 32-cell error | 64-cell error | 128-cell error | Reduction ratios |
|---|---:|---:|---:|---:|
| Sharp | 0.0417917 | 0.0114397 | 0.00342440 | 3.65×, 3.34× |
| Smooth | 0.0426141 | 0.0114559 | 0.00339344 | 3.72×, 3.38× |

Each spatial doubling exceeds a 3× error reduction.

### Temporal refinement

At fixed 64-cell grid, the 0.0125-step result is the numerical reference:

| Time step | Error |
|---:|---:|
| 0.2 | 0.00764119 |
| 0.1 | 0.00209293 |
| 0.05 | 0.000486231 |
| 0.025 | 0.0000968201 |

Successive error reductions are 3.65×, 4.30×, and 5.02×.

### Intrinsic drive and empty-space controls

- The uncoupled oscillator recovers its prescribed reference trajectory with second-order temporal convergence. Maximum errors at time 10 for steps 0.04, 0.02, and 0.01 are 0.00329291, 0.000822847, and 0.000205687; both reductions are approximately 4×.
- Before the packet reaches distant compact support, localized coupling differs from vacuum displacement by at most approximately \(1.03\times10^{-11}\).
- The deliberately incorrect bulk-coupling control changes nominal empty-space displacement by approximately 0.0104.

## 5. What this establishes — and what remains open

The reference supports these limited conclusions:

1. Material-local derivative coupling can be advanced with a staggered second-order update while retaining a discrete energy/work identity.
2. World-drive work can be paired with the actual coupled oscillator velocity without counting internal material exchange as world generation.
3. The uncoupled ambient reference is recovered at second order when material coupling is absent.
4. Sharp and smooth compact support both show the expected second-order refinement trend in this one-dimensional reference.
5. Empty-space propagation remains unchanged before the disturbance reaches material support; the bulk negative control detects the prohibited bulk response.

It does **not** establish the continuum interface limit, two-dimensional behavior, interaction with the Phase 2 sponge/outflow, nested regional flux closure under this coupled leapfrog update, a calibrated value for \(g\), or a physically selected coupling law.

## 6. Next validation gate

- Extend the staggered coupled update to the two-dimensional five-point wave operator and retain the approved CFL bound.
- Derive and test regional patch energy and boundary flux for the staggered cross-time invariant.
- Test sharp-interface reflection/transmission against a separately derived continuum reference, then refine interface and grid independently.
- Test the coupled update with the Phase 2 computational sponge and prove that outflow, source work, and stored energy reconcile without hidden sinks.
- Confirm deterministic replay and stability over the intended frequency/time-step envelope.
- Keep all work isolated. No production solver, material, chemistry, organism, GUI, or production-ledger changes are authorized by these results.
