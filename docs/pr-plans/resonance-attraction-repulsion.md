# PR 3 — Resonance-derived attraction and repulsion

**Goal:** Make interaction between materials emerge from their resonant fields and relative phase.

## Implement
- Derive (do not assume) an interaction potential compatible with the oscillator model.
- Derive force from the spatial gradient of the potential.
- Establish attractive, repulsive, neutral, and equilibrium cases.
- Require equal-and-opposite pair forces.
- Avoid hard-coded resource polarity, material roles, or identity-based behavior.
- Audit existing chemical attraction and define precisely where resonance becomes authoritative.

## Acceptance
- Phase relationship predicts the correct interaction direction under the derived model.
- Pair forces are equal and opposite.
- Equilibrium and neutral configurations behave consistently.
- Force varies consistently with separation.
- No arbitrary polarity assignment or material-specific exceptions.

**Dependency:** PR 2. Draft only until PR 2 is accepted.

## Existing interaction audit

The current chemistry interaction is a separate, always-nonnegative attraction model:
- `interaction_potential` derives a symmetric static magnitude from the absolute separation of the two chemical-position coordinates.
- `contact_factor` applies the spatial factor `(1 - distance / radius)^2` inside the interaction radius; it is zero outside that radius.
- `attraction` multiplies those terms by the configured maximum force. There is no relative oscillator phase, signed interaction, repulsive branch, or phase-dependent equilibrium.
- `formation_work_cost` integrates the existing chemical attraction over the remaining approach distance. The COMBINE evaluation path uses this as physical approach work, distinct from stored intrinsic bond potential.
- `combine_runtime` evaluates chemical interaction for live material interfaces; environmental COMBINE also persists/reads static chemical potentials through the chemistry library.

This means resonance-derived interaction must replace the *physical force authority*, not erase chemistry's reaction eligibility, chemical-position reaction semantics, bond potential, or transaction accounting. Keep the old mechanism until the new field-derived force and its work settlement are independently verified; then migrate its movement/work consumers explicitly. Do not count the same attraction twice during transition.
