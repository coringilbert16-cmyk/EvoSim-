# Phase 3C PML Boundary Investigation

**Status:** isolated investigation; no production boundary decision or behavior change.

## Objective

Find a numerically stable finite-domain boundary for the 250 x 250 physical world that lets outgoing waves leave with low reflection and reconciles the associated energy. The current sponge is retained only as a comparison baseline. Abrupt hard-zero removal is rejected.

## Source audit

The current reference solver is a scalar second-order leapfrog scheme on a uniform Cartesian grid. It stores previous/current scalar displacement arrays, advances with a five-point Laplacian, and holds outer grid nodes at zero. Its optional absorber applies a spatially varying centered damping term directly to the scalar recurrence. The existing damping ledger is an exact discrete energy sink for that damped recurrence, and the solver energy includes a cross-time gradient term.

A classical PML is not obtained by merely increasing the existing damping coefficient. It changes the propagation equations using coordinate stretching or an equivalent auxiliary-field formulation.

## Candidate formulation

The first candidate should be a **second-order scalar-wave PML**, not a rewrite into a first-order acoustic system. This is a closer mathematical match to the existing reference solver and avoids changing the physical wave variable solely to accommodate the boundary.

A particularly relevant formulation is the 2D second-order PML analyzed by Baffet, Grote, Imperiale, and Kachanovska (2018). In compact form:

- u_tt + tr(Γ1) u_t + det(Γ1) u - Δu - div(φ) = 0
- φ_t + Γ1 φ = Γ2 grad(u)
- Γ1 = diag(σx, σy)
- Γ2 = diag(σy - σx, σx - σy)

Here φ has two auxiliary components. The damping functions are zero in the physical interior and positive in the PML. The paper derives a PML-specific energy and proves continuous and discrete energy decay/stability results for its discretization. This is materially stronger than inventing an ad hoc damping recurrence.

Implementation must use the paper's exact sign convention and complete discrete scheme; the compact equations above are a model identification, not sufficient implementation instructions. In particular, the energy functional has nontrivial auxiliary-field terms and cannot be replaced by the existing scalar solver's energy formula.

Use a uniform grid initially. Begin with piecewise-constant damping regions so the published energy analysis applies directly; only then investigate smooth grading and variable-coefficient extensions. Preserve the current scalar wave speed and normalization in the physical interior. Compare the PML's physical-interior propagation against the reference solver before measuring absorption.

Primary references:
- Baffet et al., *Energy decay and stability of a perfectly matched layer for the wave equation* (2018), especially the 2D second-order formulation and its fully discrete energy identity: https://edoc.unibas.ch/70078/1/2018-13-pml-final.pdf
- Steven G. Johnson, *Notes on Perfectly Matched Layers* (MIT, 2007), for complex-coordinate stretching, derivation, and discretization limitations: https://math.mit.edu/~stevenj/18.369/spring07/pml.pdf

## Required validation

1. Baseline parity: compare resolved wave packets against the reference solver for speed, dispersion, and interior energy behavior.
2. Reflection: measure incident and reflected energy separately for normal, oblique, and near-grazing incidence on all four sides.
3. Frequency sweep: test several resolved wavelengths, including the shortest wavelength supported by the chosen grid and time step.
4. Corners: test diagonal incidence and simultaneous arrivals at adjacent PML faces.
5. Superposition: test overlapping packets and linearity.
6. Thickness/profile sweep: compare PML thickness and polynomial grading; do not tune only one incidence angle.
7. Stability: test long runs and the maximum intended Courant ratio.
8. Energy: derive a discrete identity including physical-domain energy, PML auxiliary-field energy, and PML dissipation. Check per-step and cumulative residuals. Do not infer exported energy from a drop in physical-interior energy alone.

## Decision gates

Reject formulations with growing modes, non-finite state, unexplained energy creation, or material reflection. Establish reflection and energy-residual tolerances before selecting parameters. Transfer to the adaptive spatial grid only after the isolated uniform-grid prototype passes. The production 250 x 250 domain and open-boundary requirement remain unchanged. Phase 3D remains blocked.

## Known limitations

The reference solver's existing scalar leapfrog energy identity does not automatically apply to the PML auxiliary fields. The second-order formulation above has its own published energy functional and discrete stability analysis, which must be reproduced for the actual finite-difference implementation. The live-field composed-energy ledger also includes ambient oscillator energy and ambient/disturbance cross-energy; those terms will eventually need explicit PML treatment. This investigation does not validate the live-field ambient-work correction staged on PR #446.
