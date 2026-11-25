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

[φ]^n + Γ1 {φ}^(n+1/2)
    = c² Γ2 G {u}^(n+1/2)

The scalar update is explicit because the u^(n+1) coefficient is diagonal/local after moving known spatial terms to the right. The face auxiliary updates are local scalar solves:

φx^(n+1) = ((1 - dt ζx/2) φx^n
            + dt c²(ζy-ζx) (Gx(u^(n+1)+u^n)/2))
           / (1 + dt ζx/2)

and analogously for φy, with its face-sampled coefficients.

The b-term uses the quarter-weighted time average. Replacing it with b u^n is the older recurrence that failed the high-damping Fourier screen.

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
