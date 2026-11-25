# Phase 3C PML: 2D discretization research

**Status:** research and reduced stability screen only. No production solver code changed.

## Correct 2D system

Grote and Sim (2010), Eq. (2.14), gives the exact second-order scalar PML in 2D:

```
u_tt + (ζx + ζy) u_t + ζx ζy u
    = c² Δu + div(φ)

φ_t = -diag(ζx, ζy) φ
      + c² diag(ζy - ζx, ζx - ζy) grad(u)
```

There are exactly two auxiliary components. The scalar auxiliary variable ψ in their 3D formulation vanishes in 2D. Damping profiles ζx, ζy are nonnegative and vanish in the physical interior. The original paper derives this by complex coordinate stretching, so this is a genuine PML rather than a sponge approximation.

References:
- Grote & Sim (2010), *Efficient PML for the wave equation*, Sections 2, 4 and 5: https://arxiv.org/abs/1001.0319
- Baffet et al. (2018), *Energy decay and stability of a perfectly matched layer for the wave equation*: https://edoc.unibas.ch/70078/1/2018-13-pml-final.pdf

## Discretization findings

Grote & Sim provide a concrete second-order finite-difference implementation in Section 4. It uses a space-time staggered grid: displacement at cell centers, auxiliary components on staggered locations, centered leapfrog displacement, and centered-time auxiliary updates. This is a better implementation reference than a collocated central-difference sketch. Its damping term in the displacement recurrence uses ζxζy uⁿ.

Baffet et al. prove a fully discrete energy identity for a related explicit scheme in their 3D formulation (Section 3.2, Theorem 5). That proof cannot simply be cited as a proof for our 2D update. Their 3D system has additional auxiliary variables and a different energy functional. The 2D continuous energy theorem is in Section 2.1, but the paper does not present a separate 2D fully discrete theorem in the same form.

A candidate 2D explicit update worth deriving is:
```
D2(u) + (ζx+ζy) D0(u) + ζxζy A0(u)
    - c² Δh(uⁿ) - divh(φⁿ) = 0

(φⁿ⁺¹ - φⁿ)/dt
  + diag(ζx,ζy) (φⁿ⁺¹+φⁿ)/2
  = c² diag(ζy-ζx,ζx-ζy)
      gradh((uⁿ⁺¹+uⁿ)/2)
```
Here A0(u)=(uⁿ⁺¹+2uⁿ+uⁿ⁻¹)/4. The weighted average is a stability-motivated modification, not a published 2D theorem. Use compatible staggered gradient/divergence operators; do not mix collocated central gradients with a five-point Laplacian.

## Reduced Fourier stability screen

I evaluated the candidate recurrence's amplification matrix for constant ζx, ζy using a compatible discrete gradient/divergence symbol and the five-point Laplacian symbol. This is a periodic, spatially uniform coefficient screen, not a spatially varying PML experiment.

- Grid spacing and wave speed normalized to 1.
- Damping values sampled from {0.1, 1, 5, 20, 100} independently in x and y.
- Both wave-number components sampled over [0, π].
- Time steps sampled from 0.1 through 0.7.
- Largest observed amplification-factor modulus was approximately 0.99943 at dt=0.7, ζx=ζy=100, at the highest sampled wave numbers.
- At dt=0.8, above the 2D five-point CFL limit 1/√2, the scan found an unstable mode with modulus approximately 2.55.

This is a useful sanity check that the candidate does not immediately show a constant-coefficient Fourier instability below the standard CFL limit. It is **not** a discrete energy proof and does not test a PML interface, variable damping, finite layer, corners, or reflection.

## Next derivation gate

1. Reconstruct the original staggered 2D finite-difference placement and update exactly from Grote & Sim Section 4.
2. Derive the discrete 2D energy identity for the candidate update using the staggered summation-by-parts pair.
3. Test the identity on constant-coefficient single-direction and two-direction (corner) damping.
4. If the weighted-average update has no nonnegative discrete energy under CFL, test the original unweighted recurrence separately rather than hiding the defect with empirical tuning.
5. Only after the identity and interface tests pass, implement a uniform-grid prototype and measure reflection.

The physical world remains 250 × 250. The PML computational layer remains outside it. No adaptive-grid or live-field integration is authorized by this research note.
