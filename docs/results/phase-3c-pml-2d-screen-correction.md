# Phase 3C PML: discrete recurrence screening correction

This note supplements `phase-3c-pml-2d-discretization.md`.

The original Grote–Sim 2D finite-difference recurrence evaluates the mixed zeroth-order term ζxζy u at time n. I ran the same constant-coefficient Fourier amplification screen for that recurrence and for the modified weighted-average candidate.

For normalized h=c=1, sampled ζx,ζy ∈ {0.1,1,5,20,100}, wave numbers kx,ky ∈ [0,π], and dt below the 2D five-point CFL limit:

- Original unweighted ζxζy uⁿ recurrence: severe high-damping instability. At dt=0.3 and ζx=ζy=100, the largest amplification modulus in the sampled modes was about 29.0.
- Weighted-average candidate ζxζy (uⁿ⁺¹+2uⁿ+uⁿ⁻¹)/4: no sampled amplification modulus exceeded 1 for dt≤0.7. The largest sampled value was about 0.99943 at dt=0.7 and ζx=ζy=100.

At dt=0.8, above the standard 2D five-point CFL limit 1/√2, the weighted candidate had an unstable sampled mode (amplification modulus about 2.55).

This supports using the weighted average as the next derivation candidate; it does not establish stability for spatially varying damping or a finite PML. The Fourier model assumes uniform coefficients across the whole periodic grid and compatible discrete gradient/divergence symbols.

**Decision:** do not prototype the original unweighted recurrence as the default. First derive the discrete energy identity for the weighted 2D recurrence with staggered, adjoint-compatible spatial operators. If the identity cannot be made nonnegative under the CFL condition, revisit the formulation rather than tuning damping.

References:
- Grote & Sim (2010), Section 4: https://arxiv.org/abs/1001.0319
- Baffet et al. (2018), Section 3.2, especially the stability-motivated weighted time average and Theorem 5: https://edoc.unibas.ch/70078/1/2018-13-pml-final.pdf
