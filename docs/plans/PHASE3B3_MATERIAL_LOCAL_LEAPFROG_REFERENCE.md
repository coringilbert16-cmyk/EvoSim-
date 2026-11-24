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


## 7. Two-dimensional staggered-leapfrog validation

The same reciprocal local velocity solve was extended to a two-dimensional periodic grid with the five-point Laplacian. The discrete energy assigns half of each cross-time gradient-edge term to each adjacent cell. The face flux uses the centered half-step field velocity and the corresponding normal difference of the integer-time displacement.

Reference settings: 24×24 cells, unit spacing, \(c=1\), \(K=1.3\), \(M=0.8\), \(\Omega=1.7\), coupling 0.35 on the square support \([8,13)\times[8,13)\), \(\Delta t=0.1\), 60 steps. The 2D CFL number is \(c\Delta t\sqrt{\Delta x^{-2}+\Delta y^{-2}}=0.1414<1\).

Three nested accounting patches were checked: 10×10 cells, 14×14 cells, and the complete periodic domain. For each patch the residual is \(\Delta E+F_{out}-W_{world}\):

| Patch | Energy change | Integrated outward flux | World-drive work | Residual |
|---|---:|---:|---:|---:|
| 10×10 | 6.1724881567 | 0.8852192113 | 7.0577073680 | 1.78e-15 |
| 14×14 | 13.0578907235 | 0.7658849190 | 13.8237756425 | -6.39e-14 |
| Full 24×24 | 40.6061292290 | 0 | 40.6061292290 | 1.99e-13 |

The full-domain flux cancels under periodic boundaries. These results validate the discrete 2D patch identity for this reference fixture; they do not validate a production absorbing boundary.

### 2D spatial refinement

For a sharp compact square support, a right-moving localized packet, final time 3.5, and \(\Delta t=0.2\Delta x\), a 128×128 result is used as the numerical reference:

| Grid | Error |
|---:|---:|
| 16×16 | 0.0332349 |
| 32×32 | 0.00915111 |
| 64×64 | 0.00201287 |

Error reductions are 3.63× and 4.55×.

### 2D temporal refinement

At fixed 32×32 grid and final time 3.5, the \(\Delta t=0.00625\) result is the numerical reference:

| \(\Delta t/\Delta x\) | Error |
|---:|---:|
| 0.2 | 0.000492795 |
| 0.1 | 0.000121111 |
| 0.05 | 0.0000287982 |
| 0.025 | 0.00000575776 |

Successive reductions are 4.07×, 4.21×, and 5.00×.

## 8. Sharp-interface scattering reference

The continuum material-slab problem is solved independently in the frequency domain. For incident frequency \(\omega\), vacuum wavenumber \(k_0=\omega/c\), and constant material coupling \(g\), the material wavenumber is

\[
k_m^2=\frac{\omega^2}{c^2}
\left(1+\frac{g^2}{KM(\Omega^2-\omega^2)}\right).
\]

The continuum transfer matrix across a slab of length \(L\) is

\[
\begin{pmatrix}
\cos(k_mL) & \sin(k_mL)/k_m\\
-k_m\sin(k_mL) & \cos(k_mL)
\end{pmatrix}.
\]

Continuity of displacement and normal derivative at both interfaces determines the reflection and transmission amplitudes. The discrete reference independently eliminates the local oscillator from the staggered leapfrog harmonic ansatz, uses its exact temporal dispersion factors, and advances the resulting second-order spatial recurrence. Material nodes are centered between vacuum nodes, placing interfaces on half-grid faces; this avoids a first-order interface-position offset.

For \(c=1\), \(K=1.3\), \(M=0.8\), \(\Omega=1.7\), \(g=0.35\), \(\omega=1\), and \(L=4\), the continuum reference gives:

- \(k_m=1.0306900297\).
- Reflection amplitude \(R=-0.02088155925+0.01396376412i\).
- Transmission amplitude \(T=-0.55570170683-0.83100215812i\).

With \(\Delta t=0.2\Delta x\), the combined reflection-plus-transmission amplitude error against the continuum reference is:

| \(\Delta x\) | Error |
|---:|---:|
| 0.4 | 0.0301030 |
| 0.2 | 0.00741032 |
| 0.1 | 0.00184554 |
| 0.05 | 0.000460946 |
| 0.025 | 0.000115209 |

Successive error reductions are 4.06×, 4.02×, 4.004×, and 4.001×.

At fixed \(\Delta x=0.05\), temporal refinement against the semi-discrete \(\Delta t=10^{-5}\) reference gives errors 0.000548194, 0.000137060, 0.0000342656, and 0.00000856642 for time steps 0.05, 0.025, 0.0125, and 0.00625. The successive reductions are approximately 4×.

The uncoupled control gives zero reflection to roundoff and the expected vacuum propagation phase. This is an isolated one-dimensional slab result, not yet a two-dimensional scattering validation or a production-interface test.

## 9. Updated remaining gates

The reference work now covers 1D and 2D staggered energy/work balance, 2D spatial/time convergence, and 1D sharp-slab reflection/transmission with independent spatial/time refinement. The remaining gates are:

- Two-dimensional interface reflection/transmission and smooth-support/interface behavior.
- Coupled leapfrog interaction with the Phase 2 computational sponge/outflow treatment, with no hidden sinks or double-counted source work.
- Stability and deterministic replay over the intended frequency/time-step envelope.
- Only after these isolated gates pass may a coupling candidate be considered for physical selection. No production solver, source, material, chemistry, organism, GUI, or ledger changes are authorized by this note.
