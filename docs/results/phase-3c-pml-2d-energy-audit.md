# Phase 3C PML: 2D energy-stability audit

**Status:** literature-backed discretization identified; grid-specific proof still open. No production solver code changed.

## Correction to the earlier research note

The earlier note understated the scope of Baffet et al. (2018 preprint; 2019 journal article). Their paper establishes energy decay and L2 stability for continuous and discrete formulations in both two and three dimensions with constant damping coefficients. The fully explicit scheme is a leapfrog-based modification whose mixed damping term uses a weighted time average. The result is not limited to the 3D continuous formulation.

References:
- Baffet, Grote, Imperiale & Kachanovska (2019), *Energy Decay and Stability of a Perfectly Matched Layer for the Wave Equation*, Journal of Scientific Computing 81, 2237–2270. DOI: https://doi.org/10.1007/s10915-019-01089-9
- Preprint PDF: https://edoc.unibas.ch/70078/1/2018-13-pml-final.pdf
- Grote & Sim (2010), *Efficient PML for the Wave Equation*: https://arxiv.org/abs/1001.0319

## 2D continuous system

For damping coefficients \u03b6x, \u03b6y >= 0, define

\u03b1 = \u03b6x + \u03b6y,  \u03b2 = \u03b6x \u03b6y,
\u0393_1 = diag(\u03b6x, \u03b6y),
\u0393_2 = diag(\u03b6y - \u03b6x, \u03b6x - \u03b6y).

Then

u_tt + \u03b1 u_t + \u03b2 u - \u0394u - div(\u03c6) = 0,
\u03c6_t + \u0393_1 \u03c6 = \u0393_2 grad(u).

There are two auxiliary components in 2D. The scalar auxiliary variable used in the 3D formulation is absent.

The continuous energy used by Baffet et al. is a coupled functional involving u_t + \u03b1u, grad(u)+\u03c6, a damping-weighted norm of \u03c6, and \u03b2||u||^2. It is not the ordinary wave energy plus a simple accumulated sink. The weighting and coupling matter, especially where both damping directions are nonzero.

## Explicit time discretization to carry forward

Use the standard centered differences

[v^n]_dt = (v^(n+1) - v^(n-1))/(2 dt),
[[v^n]]_dt = (v^(n+1) - 2v^n + v^(n-1))/dt^2,
{v^n}_1/4 = (v^(n+1) + 2v^n + v^(n-1))/4.

The 2D specialization of the explicit PML update has the structure

[[u^n]]_dt + \u03b1 [u^n]_dt + \u03b2 {u^n}_1/4
    - \u0394_h u^n - div_h(\u03c6^n) = 0,

[\u03c6^n]_dt + \u0393_1 {\u03c6^n}_1/2
    = \u0393_2 grad_h({u^n}_1/2),

where {v^n}_1/2 = (v^(n+1)+v^n)/2. This is the weighted recurrence previously screened. The weighted \u03b2 term is the stability-relevant difference from the original Grote–Sim finite-difference recurrence, which evaluates the mixed term at n.

Baffet et al.'s spatial discretization is expressed through compatible discrete gradient/divergence operators satisfying the adjoint relation and \u0394_h = div_h grad_h. Their energy proof depends on this compatibility. It must not be transplanted to arbitrary collocated differences.

## Mapping to EvoSim's uniform grid

A viable finite-difference realization is:
- u at cell centers;
- grad_h u on oriented faces;
- \u03c6_x and \u03c6_y stored on their corresponding oriented faces;
- div_h is the negative adjoint of grad_h under the selected grid inner products;
- \u0394_h is defined as div_h grad_h, not independently coded with a potentially inconsistent stencil;
- damping coefficients are sampled consistently at the auxiliary component locations and at the scalar locations where \u03b1 and \u03b2 are evaluated.

This is a compatible staggered operator design, not yet a verified transcription of every interpolation detail in Grote & Sim Section 4. The original finite-difference paper places auxiliary components on staggered locations and averages them transversely before taking divergence. The 2D indexing and coefficient interpolation must be written down explicitly before coding.

## What the literature does and does not prove for this project

Established:
1. The second-order scalar PML is a legitimate perfectly matched formulation, not a sponge.
2. The 2D continuous system has an energy-decay/stability analysis.
3. Baffet et al. establish energy decay and L2 stability for discrete formulations with constant damping coefficients.
4. Their explicit leapfrog-based modification uses the weighted average for the mixed damping term and has a CFL restriction independent of damping magnitude.
5. The constant-coefficient Fourier screen previously run for EvoSim is consistent with this published design.

Not established for EvoSim:
1. The exact staggered finite-difference stencil plus its coefficient interpolation has not yet been shown to satisfy the published discrete energy identity.
2. The theorem for constant damping does not prove monotone energy decay for the spatially varying profiles required by a finite PML. The paper's variable, piecewise-constant numerical experiments show small positive energy-rate excursions near initial PML entry even while remaining bounded over long runs.
3. The outer truncation, corner profile, and chosen boundary condition still require direct reflection and long-time tests.
4. Adaptive-grid transfer is entirely out of scope until the uniform-grid version passes.

## Required next derivation

Before implementation, write the 2D grid indexing and all operators explicitly. Then either:
- specialize the published discrete energy identity to this exact staggered operator pair and constant-coefficient corner/face cases; or
- derive an independent summation-by-parts identity for the chosen finite-difference scheme.

The identity must expose the nonnegative damping terms and the CFL-dependent positive energy. Verify it numerically against the algebraic one-step energy difference for random states before running wave packets. If the exact identity does not close, revise the operators or use the paper's finite-element/mass-lumped operator framework rather than patching energy accounting.

## Scope boundary

The PML remains a computational layer outside the physical 250 x 250 world. Exported physical-domain energy, PML auxiliary-field energy, and cumulative numerical dissipation remain separate ledger quantities. This research does not authorize production integration or changes to PR #446.
