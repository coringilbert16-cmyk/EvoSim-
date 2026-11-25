# Phase 3C PML Boundary Investigation

**Status:** isolated investigation; no production boundary decision or behavior change.

## Objective

Find a numerically stable finite-domain boundary for the 250 x 250 physical world that lets outgoing waves leave with low reflection and reconciles the associated energy. The current sponge is retained only as a comparison baseline. Abrupt hard-zero removal is rejected.

## Source audit

The current reference solver is a scalar second-order leapfrog scheme on a uniform Cartesian grid. It stores previous/current scalar displacement arrays, advances with a five-point Laplacian, and holds outer grid nodes at zero. Its optional absorber applies a spatially varying centered damping term directly to the scalar recurrence. The existing damping ledger is an exact discrete energy sink for that damped recurrence, and the solver energy includes a cross-time gradient term.

A classical PML is not obtained by merely increasing the existing damping coefficient. It changes the propagation equations using coordinate stretching or an equivalent auxiliary-field formulation.

## Candidate formulation

Prototype a first-order acoustic-style split-field PML in an isolated solver, rather than modifying the existing scalar leapfrog solver in place. In the undamped interior use pressure-like scalar p and directional velocity/flux fields vx, vy:

- p_t = -K (d_x vx + d_y vy)
- vx_t = -(1/rho) d_x p
- vy_t = -(1/rho) d_y p

In the absorbing layer, introduce directional stretch/damping profiles and split pressure components (px, py) with auxiliary state. Derive the exact discrete equations from a standard split-field or unsplit PML formulation and verify stability/matching before accepting implementation. Do not assume independent damping of all state variables is a valid PML.

Use a uniform grid initially, with a staggered spatial arrangement if needed by the energy derivation. Keep physical-interior coefficients matched to the reference wave speed and normalization.

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

The reference solver's scalar second-order energy identity does not automatically apply to a first-order split-field PML. The live-field composed-energy ledger also includes ambient oscillator energy and ambient/disturbance cross-energy; those terms will eventually need explicit PML treatment. This investigation does not validate the live-field ambient-work correction staged on PR #446.
