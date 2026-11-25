# Phase 3C PML: exact 2D uniform-grid operator specification

**Status:** candidate spatial operators and explicit recurrence specified. The published constant-coefficient stability result is applicable to the 2D formulation, but the exact energy identity has not yet been independently reproduced for this finite-difference realization. No solver code changed.

## 1. Correct 2D PML system

For constant wave speed c and nonnegative damping coefficients ζx, ζy, define

a = ζx + ζy
b = ζx ζy
Γ1 = diag(ζx, ζy)
Γ2 = diag(ζy - ζx, ζx - ζy)

The second-order 2D PML is

u_tt + a u_t + b u - c² Δu - div(φ) = 0
φ_t + Γ1 φ = c² Γ2 grad(u)

where φ=(φx,φy). There are two auxiliary components. No scalar ψ field is needed in this 2D formulation.

Baffet et al. (2019), Section 2.1, give the continuous energy for constant damping:

E = 1/2 [ ||u_t + a u||²
          + ||grad(u)+φ||²
          + ||φ||²_(a⁻¹Γ1)
          + b ||u||² ]

with the weighted norm interpreted as the integral of φᵀ(a⁻¹Γ1)φ. The associated energy derivative is nonpositive. The formula is understood on the active PML region with the paper's boundary and interface assumptions; zero damping limits must be handled by the corresponding reduced formulation rather than blindly dividing by a.

Reference: Baffet, Grote, Imperiale & Kachanovska (2019), Sections 2.1 and 3, https://doi.org/10.1007/s10915-019-01089-9

## 2. Uniform Cartesian grid and compatible operators

Use a rectangular cell-centered scalar grid with spacing hx, hy.

- u[i,j] is stored at cell centers (i+1/2,j+1/2).
- φx[i+1/2,j] is stored on vertical faces.
- φy[i,j+1/2] is stored on horizontal faces.

Define the forward gradient:

(Gx u)[i+1/2,j] = (u[i+1,j] - u[i,j]) / hx
(Gy u)[i,j+1/2] = (u[i,j+1] - u[i,j]) / hy

Define divergence as the negative adjoint of this gradient under the matching cell and face inner products:

(D φ)[i,j] =
  (φx[i+1/2,j] - φx[i-1/2,j]) / hx
+ (φy[i,j+1/2] - φy[i,j-1/2]) / hy

With boundary terms treated consistently, D = -G* and the positive-semidefinite stiffness operator is K = -D G. Equivalently, the physical Laplacian is D G, with the conventional negative-semidefinite sign. This guarantees the discrete integration-by-parts identity; do not implement a separately chosen Laplacian stencil that breaks this relationship.

The inner products include cell/face area weights. On a uniform Cartesian mesh these are constant factors, but retaining the distinction makes the adjoint relation explicit.

Damping coefficients:
- a[i,j] and b[i,j] are cell-centered.
- ζx and the x-component of Γ2 are sampled at x-faces for the φx update.
- ζy and the y-component of Γ2 are sampled at y-faces for the φy update.
- For constant-coefficient stability tests, all these samples are identical constants. For spatially varying profiles, the coefficient interpolation is a separate consistency question and is not covered by the constant-coefficient theorem.

## 3. Explicit time update

For n ≥ 1, define

[v]^n = (v^(n+1)-v^(n-1))/(2 dt)
[[v]]^n = (v^(n+1)-2v^n+v^(n-1))/dt²
{v}^(n+1/2) = (v^(n+1)+v^n)/2
{v}^(n)_1/4 = (v^(n+1)+2v^n+v^(n-1))/4

The candidate explicit 2D update is

[[u]]^n + a [u]^n + b {u}^n_1/4
    - c² D G u^n - D φ^n = 0

(φ^(n+1)-φ^n)/dt + Γ1 (φ^(n+1)+φ^n)/2
    = c² Γ2 G ((u^(n+1)+u^n)/2)

The scalar update is explicit because the u^(n+1) coefficient is diagonal/local after moving known spatial terms to the right. The auxiliary update is a forward-in-time Crank–Nicolson step (not a centered leapfrog difference); its face solves are local scalar solves:

φx^(n+1) = ((1 - dt ζx/2) φx^n
            + dt c²(ζy-ζx) (Gx(u^(n+1)+u^n)/2))
           / (1 + dt ζx/2)

and analogously for φy, with its face-sampled coefficients.

The b-term uses the quarter-weighted time average. The φ update uses the forward midpoint pair (n,n+1), exactly as in Baffet et al. equation (3.32c). Do not replace it with a centered (n−1,n+1) difference. The earlier Fourier screen for a centered-φ candidate is not evidence for this published recurrence and must be rerun with the correct update before it is used to support a stability claim.

## 4. What the published proof covers — corrected dimensional reduction

Baffet et al. (2019) derive the continuous two-dimensional system directly in Section 2.1. Their Section 3 fully discrete proof is written for the three-dimensional formulation. The two-dimensional explicit recurrence above is obtained as an invariant dimensional reduction of that discrete system, not by copying the three-dimensional energy formula unchanged.

Set the third-direction damping to zero and restrict all fields to be independent of the third coordinate:

- Γ1 = diag(ζx, ζy, 0)
- Γ2 = diag(ζy - ζx, ζx - ζy, ζx + ζy)
- Γ3 = diag(0, 0, ζx ζy)
- det(Γ1) = 0
- φz = 0, with zero initial φz and ∂z u = 0

The three-dimensional scalar equation then reduces to the two-dimensional equation because tr(Γ1)=a and tr(Γ3)=b. The x/y auxiliary equations reduce exactly to the two-field system in Section 1: their Γ3 components vanish. The remaining third auxiliary component stays identically zero.

The discrete proof nevertheless uses analysis-only integrated variables. In this reduction:
- ψ satisfies ψ_t = u;
- Φ_t = φ for the two physical auxiliary components;
- Ψ_t = ψ;
- q = v + a u + b ψ, where v is the centered discrete velocity.

These are proof variables, not additional physical PML fields. They allow the quarter-weighted b u term to be handled in the energy argument. In particular, do not add ψ, Φ, or Ψ to the physical PML state merely because they appear in the proof.

The paper's discrete theorem uses compatible discrete operators with Δ_h = div_h grad_h and proves a CFL-conditioned positive explicit energy with a one-step non-increase identity for constant damping. The dimensional reduction above carries that algebra to 2D when the discrete spaces/operators preserve the invariant subspace and the same adjoint/compatibility assumptions. It does **not** automatically prove the result for every finite-difference boundary closure or coefficient interpolation.

Reference: Baffet et al. (2019), Section 2.1 (2D continuous system), Section 3.2 (explicit discrete scheme and energy identity), especially equations (61)–(71): https://doi.org/10.1007/s10915-019-01089-9

The earlier Fourier amplification screen remains an independent reduced check, not a substitute for verifying the exact discrete energy.

The constant-coefficient theorem does not establish monotone energy decay for the spatially varying damping profiles required in a finite PML. The paper's variable-profile experiments are empirical, not a general variable-coefficient proof.

## 5. Exact remaining proof obligation for this implementation

The key mathematical gap is now narrower: specialize the paper's discrete energy (equations 66–71) to the invariant 2D reduction, then determine whether our face-centered finite-difference realization satisfies the same hypotheses and boundary terms.

Next:
1. Write the complete reduced half-time energy, including the kinetic CFL correction and all additional time-discretization terms. Keep ψ, Φ, and Ψ explicitly labeled analysis-only.
2. Derive the one-step identity for the exact 2D recurrence using D = -G* and Δ_h = DG. Identify any terms that fail to cancel for our finite-difference boundary closure.
3. Verify the identity numerically to roundoff for random states on periodic/closed test grids with constant ζx, ζy, including both-zero, one-axis-only, and two-direction damping.
4. Check positive definiteness under the actual operator CFL bound, not just the five-point periodic estimate.
5. Only then introduce spatially varying profiles and measure reflection.

The Fourier screen's dt ≤ 1/√2 bound assumes hx=hy=c=1. The general five-point CFL condition is c dt sqrt(hx⁻² + hy⁻²) ≤ 1; the exact operator CFL should be computed from the largest eigenvalue of -DG.

## 6. Scope and decision

This is a research specification only. It does not authorize production integration, adaptive-grid changes, or changes to PR #446. The PML remains a computational layer outside the physical 250 × 250 domain. Energy exported from the physical domain, energy held in PML state, and cumulative numerical PML dissipation remain distinct quantities.

### Exact periodic-grid CFL bound

For the stated forward-difference gradient and its negative-adjoint divergence on a periodic rectangular grid, the stiffness operator is
`K = -DG = G*G`. Its Fourier eigenvalue is

`λ(kx,ky) = 4 sin²(kx hx/2)/hx² + 4 sin²(ky hy/2)/hy²`.

Thus `λ_max ≤ 4(hx⁻² + hy⁻²)`; equality is attained when the grid supports the Nyquist mode in both directions. The paper's kinetic energy correction is positive when

`(dt²/4) λ_max < 1`,

which gives the familiar sufficient bound

`c dt sqrt(hx⁻² + hy⁻²) < 1`.

For a particular finite grid or non-periodic boundary closure, use the actual largest eigenvalue of `-DG`; do not assume the periodic Nyquist value is attained. This verifies the CFL factor for the kinetic correction only. It does not by itself verify the remaining PML energy terms or their one-step identity.


## 7. Exact normalization for general wave speed

The published discrete energy analysis assumes unit wave speed. For a candidate physical system with constant wave speed `c>0`,

`u_tt + a u_t + b u - c² Δu - div(φ) = 0`

`φ_t + Γ1 φ = c² Γ2 grad(u)`,

define normalized time and physical auxiliary field

`s=ct`, `Δs=cΔt`, `χ=φ/c²`.

The unit-speed coefficients are

`a'=a/c`, `b'=b/c²`, `Γ1'=Γ1/c`, `Γ2'=Γ2/c`, `Γ3'=Γ3/c²`.

In 2D, this is equivalent to `ζx'=ζx/c`, `ζy'=ζy/c`, with `a'=ζx'+ζy'` and `b'=ζx'ζy'`. The normalized physical equations become

`u_ss + a' u_s + b' u - Δu - div(χ) = 0`

`χ_s + Γ1'χ = Γ2' grad(u)`.

The analysis-only fields must also be transformed from their defining discrete/continuous update equations, not left unchanged:

- scalar auxiliary: `ψ'=cψ`, so `∂sψ'=u`;
- integrated vector auxiliary: `Φ'=Φ/c`, so `∂sΦ'=χ`;
- integrated scalar auxiliary: `Ψ'=c²Ψ`, so `∂sΨ'=ψ'`;
- auxiliary velocity: `v'=v/c`, preserving `(v'^(n+1)+v'^n)/2=(u^(n+1)-u^n)/Δs`;
- combined vector: `Λ'=Gψ'+Φ'=cGψ+Φ/c`;
- combined scalar: `q'=v'+a'u+b'ψ'=q/c`.

These substitutions map the full general-`c` discrete recurrence to the paper's unit-speed recurrence exactly when the timestep is changed to `Δs=cΔt`. Therefore every published energy term must be evaluated using the primed variables:

`E_k' = 1/2 ( ||q'^(n+1/2)||² - (Δs²/4)||Gq'^(n+1/2)||² )`

`E_p' = 1/2 ( ||Gu^(n+1/2)+χ^(n+1/2)||² + ||Γ1'(Gψ'^(n+1/2)+Φ'^(n+1/2))||² )`

`E_add' = (Δs²/8) ( ||Γ1'(Λ'^(n+1)-Λ'^n)/Δs||² + ||2Γ1'(Λ'^(n+1)-Λ'^n)/Δs + Γ1'^2(Λ'^(n+1)+Λ'^n)/2||² )`.

The one-step identity is then the unit-speed identity in s-time, with `Δs` and the primed damping matrix. Do not apply the unit-speed energy directly to the unscaled physical `φ,ψ,Φ,Ψ,v` fields. The normalized energy is not obtained by changing the timestep alone; all auxiliary and proof fields above must be transformed consistently.

This completes the algebraic wave-speed normalization. It does not extend the constant-coefficient theorem to spatially varying damping or prove arbitrary boundary closures.

Reference: Baffet et al. (2019), Sections 2.1 and 3.2.2, equations (3.32)–(3.41): https://edoc.unibas.ch/70078/1/2018-13-pml-final.pdf


## 8. Discrete energy specialization: equation (3.38) recovered

The explicit energy is the sum of three terms, equations (3.36)–(3.39) in Baffet et al. The auxiliary velocity sequence is defined by

`(v_h^(n+1)+v_h^n)/2 = (u_h^(n+1)-u_h^n)/Δt`.

It is not simply the centered difference of u. Under the invariant 2D reduction, the combined scalar is

`q_h^n = v_h^n + a u_h^n + b ψ_h^n`

with `a=ζx+ζy`, `b=ζxζy`, and `(ψ_h^(n+1)-ψ_h^n)/Δt=(u_h^(n+1)+u_h^n)/2`.

The kinetic contribution is

`E_k^(n+1/2) = 1/2 ( ||q_h^(n+1/2)||² - (Δt²/4)||G_h q_h^(n+1/2)||² )`.

The potential contribution is

`E_p^(n+1/2) = 1/2 ( ||G_h u_h^(n+1/2)+φ_h^(n+1/2)||² + ||Γ1(G_h ψ_h^(n+1/2)+Φ_h^(n+1/2))||² )`.

The second potential term is exactly `||Γ1 Λ_h||²`, with `Λ_h=G_h ψ_h+Φ_h`.

### Additional explicit-energy correction

The time indices in equation (3.38) are half-step indices. Cross-checking its structure against Lemma 13 gives

`E_add^(n+1/2) = (Δt²/8) ( ||Γ1(Λ_h^(n+1)-Λ_h^n)/Δt||² + ||2Γ1(Λ_h^(n+1)-Λ_h^n)/Δt + Γ1²(Λ_h^(n+1)+Λ_h^n)/2||² )`

where

- `Λ_h^n=G_h ψ_h^n+Φ_h^n`;
- both norms are ordinary compatible discrete L² norms; the first vector is explicitly multiplied by `Γ1`;
- `Γ1²` means matrix composition.

The factor **2** multiplying the first term inside the second norm is essential. It is consistent with Lemma 13, where the difference of this half-step vector produces `2Γ1[[Λ_h^n]]_Δt+Γ1²[Λ_h^n]_Δt`. The first norm is the ordinary norm of `Γ1` times the forward difference; it is **not** the `Γ1`-weighted norm of the unmultiplied difference. The two norms must not be split or simplified.

For the invariant 2D reduction, `Γ1=diag(ζx,ζy)`, giving

`E_add,2D^(n+1/2) = (Δt²/8) [ ζx²||(Λx^(n+1)-Λx^n)/Δt||² + ζy²||(Λy^(n+1)-Λy^n)/Δt||² + ||2ζx(Λx^(n+1)-Λx^n)/Δt+ζx²(Λx^(n+1)+Λx^n)/2||² + ||2ζy(Λy^(n+1)-Λy^n)/Δt+ζy²(Λy^(n+1)+Λy^n)/2||² ]`.

Both contributions are nonnegative for nonnegative damping. They are analysis-energy corrections, not additional physical PML state variables.

The full energy is `E_expl=E_k+E_p+E_add`. Theorem 5 / equation (3.41) gives

`(E_expl^(n+1/2)-E_expl^(n-1/2))/Δt = -(Δt²/2)||[G_hu_h^n+φ_h^n]_Δt||²_(Γ1) - 2||{G_hu_h^n+φ_h^n}_{1/4}||²_(Γ1)`.

The paper proves this identity for its compatible Galerkin/mass-lumped spatial operators, constant damping coefficients, and stated boundary conditions. Section 10 below independently verifies the same algebra for periodic, homogeneous Dirichlet, and homogeneous Neumann compatible finite-difference operator pairs; this verifies the energy algebra, not the reflection quality of a truncated PML boundary.

### Initial conditions for the proof fields

The explicit scheme defines `Ψ_h^0=0` and requires the damping-weighted compatibility condition from equation (3.33):

`Γ1(Γ1 Φ_h^0 + φ_h^0 - Γ2 G_h ψ_h^0)=0`.

Preserve this condition when initializing proof fields. Where a damping component is zero, the condition does not determine that component of `Φ_h^0`; do not divide by zero.

### General wave speed

The exact normalization is derived in Section 7. Apply the published energy formulas to the primed variables and normalized timestep; do not mix physical and normalized auxiliary fields.

Reference: Baffet et al. (2019), Section 3.2.2, equations (3.32)–(3.41), especially (3.33), (3.36)–(3.38), and Lemma 13: https://doi.org/10.1007/s10915-019-01089-9. Full-text preprint: https://edoc.unibas.ch/70078/1/2018-13-pml-final.pdf

**Remaining validation:** test piecewise/spatially varying damping, physical/PML interface matching, angle/frequency-dependent reflection, finite layer thickness, and corner behavior. The constant-coefficient identity is verified for periodic, Dirichlet, and Neumann compatible operators; no production PML implementation is authorized by this specification.


## 9. Revised Fourier screen using the published explicit time staggering

The earlier Fourier screen used a centered-in-time update for the auxiliary field and therefore did **not** test the published scheme (62). It has been superseded by a new constant-coefficient amplification screen using the actual recurrence:

- the second-order wave update with the quarter-weighted mixed term `b{u^n}_{1/4}`;
- the forward Crank–Nicolson auxiliary update `(φ^{n+1}-φ^n)/Δt+Γ1(φ^{n+1}+φ^n)/2=Γ2G(u^{n+1}+u^n)/2`;
- the scalar proof variable update `(ψ^{n+1}-ψ^n)/Δt=(u^{n+1}+u^n)/2`.

For each Fourier mode, the amplification state was `(u^n,u^{n-1},ψ^n,φ_x^n,φ_y^n)`. The periodic forward-gradient / negative-adjoint-divergence symbols were used consistently, so `Δ_h=D_hG_h` exactly. The scan used `h_x=h_y=c=1`, 17 wave numbers per axis over `[0,π]`, damping coefficients independently sampled from `{0.1,1,5,20,100}`, and the 2D five-point CFL limit `Δt≤1/√2`.

Results:

| Δt | Maximum sampled spectral radius | Worst-case parameters |
|---:|---:|---|
| 0.2 | 1.000000 | ζx=ζy=0.1, kx=ky=0 |
| 0.4 | 1.000000 | ζx=ζy=0.1, kx=ky=0 |
| 0.6 | 1.000000 | ζx=ζy=0.1, kx=ky=0 |
| 0.7 | 1.000000 | ζx=ζy=0.1, kx=ky=0 |
| 1/√2 | 1.000000 | ζx=ζy=0.1, kx=ky=0 |
| 0.71 | 1.131240 | ζx=ζy=0.1, kx=ky=π |
| 0.8 | 2.553945 | ζx=ζy=0.1, kx=ky=π |

This supports the expected CFL threshold for the **constant-coefficient periodic Fourier model** and removes the previous centered-auxiliary screen as evidence. It is not a proof, does not test spatially varying damping or a finite PML interface/corner, and does not validate the proposed finite-difference boundary closure. The neutral spectral radius of one corresponds to the undamped/zero-frequency limit of the reduced system; it is not growth.



## 10. Periodic-grid one-step energy identity audit

A direct numerical reproduction of the published one-step identity now closes to floating-point roundoff for the proposed compatible periodic finite-difference operators.

Test setup:

- 16×16 periodic scalar grid, `h_x=h_y=1`;
- forward face gradients and their exact negative-adjoint divergence, with `Δ_h=D_hG_h`;
- `Δt=0.1`, below the operator CFL limit;
- independent random `u^0,u^1`, with `ψ^0=φ^0=Φ^0=Ψ^0=0`;
- first-order auxiliary states at n=1 initialized by applying the n=0 trapezoidal updates, rather than incorrectly setting them to zero;
- velocity sequence initialized consistently with equation (3.44);
- 5 random seeds, 5 one-step identity checks per seed, for each damping pair `(ζx,ζy)=(0,0),(0.7,0),(0,1.1),(0.7,1.1),(2,3)`.

The tested identity is

`(E_expl^(n+1/2)-E_expl^(n-1/2))/Δt = -(Δt²/2)||[G_hu_h^n+φ_h^n]_Δt||²_(Γ1) - 2||{G_hu_h^n+φ_h^n}_{1/4}||²_(Γ1)`.

Across the 125 checks, the maximum absolute residual was approximately `1.66×10⁻¹⁰` in double precision. The auxiliary identity from Lemma 12 and the energy decomposition identities were also checked independently; their residuals were at floating-point roundoff.

A separate normalization check used `c=2.5`, physical `Δt=0.04`, and normalized `Δs=0.1`, with normalized damping `(ζx',ζy')=(0.7,1.1)). Mapping the normalized solution back to physical variables gave maximum recurrence residuals of approximately `5.7×10⁻¹²` for the wave equation and `3.4×10⁻¹³` for the auxiliary equations; the normalized energy identity residual was approximately `5.0×10⁻¹¹`. This verifies the stated variable/time rescaling numerically for that case.

This validates the **constant-coefficient periodic finite-difference realization** of the published explicit energy identity, provided the operator pair is exactly adjoint-compatible and the startup fields are initialized consistently.

The same audit was then run with two non-periodic compatible operator pairs on an 8×8 scalar grid:

- homogeneous Dirichlet: interior nodal unknowns with fixed-zero boundary values; boundary-adjacent gradient edges are included;
- homogeneous Neumann: cell-centered unknowns with zero normal boundary flux; only interior gradient faces are included;
- in both cases, divergence was constructed as the exact negative adjoint of the gradient and `Δ_h=D_hG_h`.

For each boundary type, five damping pairs `(0,0),(0.7,0),(0,1.1),(0.7,1.1),(2,3)` and three random seeds were tested, with five one-step identity checks per case (150 checks total). The maximum absolute residual was approximately `4.87×10⁻¹¹`. The actual largest stiffness eigenvalues were approximately 7.75877 (Dirichlet) and 7.69552 (Neumann), giving operator CFL numbers 0.1393 and 0.1387 at `Δt=0.1`, respectively.

These tests validate the energy identity for these specific compatible homogeneous boundary closures. They do not validate spatially varying damping profiles, PML interface reflection, finite-layer performance, corners, or adaptive-grid coupling.



## 11. Preliminary 2D variable-profile packet reflection experiment

A first finite-layer experiment was run with the published 2D PML recurrence and spatially varying damping profiles. This is an empirical reflection test, not covered by the constant-coefficient energy theorem.

Setup:

- 128×128 interior nodal grid with homogeneous Dirichlet outer boundary;
- physical square bounded by the PML interface at 24 cells from each side; 24-cell PML thickness;
- `h=1`, `c=1`, `Δt=0.4`;
- independent x/y damping profiles, zero in the physical region and smoothly ramped through the PML with cubic profile power 3;
- face-sampled `ζx,ζy,Γ2x,Γ2y`, cell-sampled `a=ζx+ζy,b=ζxζy`;
- right-going Gaussian packet centered in the physical region, with propagation angles 0°, 30°, and 45°;
- reflection metric: maximum absolute field at one monitor point 12 cells inside the physical/PML interface during the first return window, divided by the outgoing peak at the same point. This is a point-monitor amplitude ratio, not a full-field reflection coefficient.

For the 24-cell, cubic profile:

| Maximum damping | 0° normal | 30° | 45° |
|---:|---:|---:|---:|
| 0 (hard boundary baseline) | 0.517 | 0.242 | 0.404 |
| 0.25 | 0.0132 | 0.00643 | 0.00253 |
| 0.5 | 0.000441 | 0.000198 | 0.0000646 |
| 1.0 | 0.000174 | 0.000102 | 0.0000568 |
| 2.0 | 0.000174 | 0.000103 | 0.0000568 |

The result shows strong attenuation of the returning packet compared with the undamped hard-boundary baseline. Increasing peak damping beyond 1.0 did not materially improve the measured ratio, suggesting the observed floor is set by discretization/profile/interface effects rather than insufficient peak damping.

A small profile/thickness sensitivity screen at peak damping 0.5 gave:

| PML thickness | Profile power | 0° ratio | 45° ratio |
|---:|---:|---:|---:|
| 16 | 2 | 0.000953 | 0.000110 |
| 16 | 3 | 0.00305 | 0.000381 |
| 16 | 4 | 0.00695 | 0.00103 |
| 24 | 3 | 0.000441 | 0.0000646 |
| 32 | 3 | 0.000118 | 0.0000394 |

The sampled solution norms remained bounded throughout these runs; this is only a finite-duration empirical check, not a stability proof. The cubic profile was not uniformly optimal at every thickness, and these results do not establish a universal damping choice.

**Interpretation:** the PML recurrence is now supported by (1) a roundoff-level constant-coefficient energy identity audit for periodic, Dirichlet, and Neumann compatible operators, and (2) a preliminary variable-profile 2D packet screen spanning multiple frequencies, angles, thicknesses, and profile powers. The next experiment should compare profile families at matched integrated damping and inspect full-field energy/flux across the physical/PML interface and corner regions. Do not select production profile parameters from these monitor-based screens.

No production solver code changed. No PR #446 changes were made.

A carrier-wavenumber sweep at the same 24-cell cubic profile and peak damping 0.5 produced:

| Carrier wavenumber k | 0° normal | 30° | 45° |
|---:|---:|---:|---:|
| 0.30 | 0.00325 | 0.00185 | 0.00106 |
| 0.55 | 0.000441 | 0.000198 | 0.0000646 |
| 0.85 | 0.000264 | 0.0000712 | 0.0000161 |
| 1.10 | 0.000177 | 0.0000199 | 0.0000137 |

At the lower wavenumber `k=0.30`, increasing layer thickness reduced the measured ratio:

| Thickness | 0° normal | 30° | 45° |
|---:|---:|---:|---:|
| 16 | 0.00736 | 0.00391 | 0.00162 |
| 24 | 0.00325 | 0.00185 | 0.00106 |
| 32 | 0.00197 | 0.00125 | 0.000722 |
| 40 | 0.00119 | 0.000853 | 0.000516 |

This indicates that the low-frequency end is the more demanding case in this packet screen, and that added thickness improves attenuation. The numbers remain monitor-based preliminary diagnostics; they are not angle/frequency-independent reflection bounds.

An extended angular sweep at `k=0.55`, peak damping 0.5, thickness 24, and cubic profile gave:

| Propagation angle | Point-monitor reflected/outgoing amplitude |
|---:|---:|
| 0° | 0.000441 |
| 30° | 0.000198 |
| 45° (corner-directed) | 0.0000646 |
| 60° | 0.000198 |
| 75° | 0.000389 |
| 90° | 0.000441 |

The near-symmetry around 45° is consistent with the symmetric x/y layer construction. It does not rule out other corner, frequency, or grid-alignment artifacts.

A fairer profile comparison was then made at matched integrated damping,
`∫₀ᵀ ζ(s) ds = ζmax T/(p+1)=3` (in grid-cell units), with `T=24`:

| Profile power p | Peak damping | k=0.55: 0° / 30° / 45° | k=0.30: 0° / 30° / 45° |
|---:|---:|---:|---:|
| 2 | 0.375 | 0.000533 / 0.000231 / 0.0000696 | 0.00324 / 0.00183 / 0.00106 |
| 3 | 0.500 | 0.000441 / 0.000198 / 0.0000646 | 0.00325 / 0.00185 / 0.00106 |
| 4 | 0.625 | 0.000369 / 0.000172 / 0.0000610 | 0.00326 / 0.00186 / 0.00106 |

At `k=0.30`, the three profiles are effectively indistinguishable at this resolution. At `k=0.55`, the steeper p=4 profile is modestly better in this particular monitor metric. This is not enough evidence to select a profile.

A matched-integral thickness sweep at `k=0.30`, p=3, and `∫ζ ds=3` used peak damping `ζmax=12/T`:

| Thickness T | Peak damping | 0° | 30° | 45° |
|---:|---:|---:|---:|---:|
| 16 | 0.750 | 0.00577 | 0.00310 | 0.00154 |
| 24 | 0.500 | 0.00325 | 0.00185 | 0.00106 |
| 32 | 0.375 | 0.00195 | 0.00115 | 0.000679 |
| 40 | 0.300 | 0.00132 | 0.000726 | 0.000508 |

The thicker layer improves the low-frequency monitor ratio even when integrated damping is held constant. That suggests propagation distance through the layer matters independently of total damping strength in this discrete configuration.


### Energy-accounting gate before any PML ledger claims

A first attempt to integrate a continuum-style interface flux from pointwise `u_t(∇u+φ)` did not reconcile with the discrete physical-domain energy change. The discrepancy is expected to be sensitive to the leapfrog time staggering and the partition of interface-face terms. That estimate is **not** accepted as exported energy.

Before reporting the three requested ledger quantities, derive the exact subdomain balance from the discrete operator pair:

1. physical-domain energy change;
2. signed energy transfer across the physical/PML interface, using the physical-side restriction of the discrete gradient and the exact time-level pairing;
3. PML auxiliary-state storage and cumulative PML damping, with any variable-profile commutator/remainder terms shown explicitly.

Raw `||φ||²` is only an auxiliary-state norm, not automatically stored physical energy. Likewise, the constant-coefficient dissipation expression is only a diagnostic under variable damping until the extra profile-dependent terms are accounted for. Do not label either as a conserved ledger category prematurely.
