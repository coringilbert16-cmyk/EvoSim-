# Time-dependent resonance implementation roadmap

This roadmap completes the existing resonance model in four dependency-ordered pull requests. It preserves the established material-frequency, damping, amplitude, harmonic, phasor, and energy-reservoir mechanisms.

## Shared conventions

- One simulated day is 24,000 simulation ticks.
- The reference world tone is 440 oscillations per simulated day.
- All material natural frequencies and reaction frequencies use the same cycles-per-day unit.
- Every oscillator begins at phase zero (its peak) at simulation tick zero.
- Phase is calculated analytically on demand; the simulation does not add oscillator substeps or change its execution tick rate.
- Runtime `ticks_per_second` remains an execution-speed control, not a physical-time definition.
- The existing 2,400-tick world-source orbital cycle is separate from the carrier oscillation period.

## PR 1 — Simulation time and phase advancement

**Goal:** Establish a coherent physics-time convention and analytical phase calculation without increasing simulation work per tick.

**Scope**
- Define 24,000 ticks per simulated day and 440 reference cycles per day.
- Make frequency units explicit as cycles per simulated day throughout resonance and reaction interfaces.
- Preserve the existing natural-frequency ratio derived from cohesion and mass.
- Preserve phase-zero-at-peak initial conditions.
- Add a validated phase-at-tick calculation with phase wrapping.
- Thread the simulation tick into active source/receiver paths so source phase is derived when needed.
- Keep orbital phase and carrier phase distinct.
- Keep runtime execution speed independent of physical phase.

**Acceptance**
- Phase is zero at tick zero.
- Reference tone advances 6.6 degrees per tick and completes 440 cycles in 24,000 ticks.
- Natural frequencies preserve their existing relative ratios.
- Phase is deterministic and calculated without per-tick oscillator state updates.
- Execution-speed changes do not affect phase at a given simulation tick.
- Existing energy and resonance behavior remains conserved and regression tests pass.

## PR 2 — Driven oscillator response and propagation

**Goal:** Complete the physical phase response of the existing spectral oscillator.

**Scope**
- Derive phase lag from the same normalized forced-oscillator equation already used for amplitude.
- Preserve existing material frequency, damping, and amplitude relationships.
- Carry coherent phase through harmonic generation and signal combination.
- Audit propagation assumptions and only introduce distance-related phase delay if a defensible propagation speed exists in the world model.
- Keep retained scalar resonance energy phase-free.

**Acceptance**
- Response phase is correct below, at, and above resonance.
- Damping affects amplitude and phase consistently.
- Harmonic phase relationships are correct.
- Propagation introduces no arbitrary offsets.
- Existing energy bounds remain satisfied.

## PR 3 — Resonance-derived attraction and repulsion

**Goal:** Derive material interaction from the phase relationship of their resonant fields.

**Scope**
- Derive an interaction potential from the oscillator model rather than adopting an unverified equation by assertion.
- Derive force from the spatial gradient of that potential.
- Establish attraction, repulsion, neutral conditions, symmetry, and equilibrium behavior.
- Avoid hard-coded resource polarity or roles.
- Reconcile or replace parallel chemical-attraction behavior only where the new physical interaction is demonstrated to be authoritative.

**Acceptance**
- Phase-dependent attraction and repulsion follow from the derived model.
- Pair forces are equal and opposite.
- Neutral/equilibrium cases are stable.
- Distance and phase changes produce consistent force changes.
- Results do not depend on arbitrary material identity.

## PR 4 — Energy conservation and environmental integration

**Goal:** Integrate resonance-derived interactions into material movement and chemistry without creating or double-counting energy.

**Scope**
- Account for interaction potential separately from retained resonance energy.
- Settle movement work, potential changes, usable energy, structural energy, and heat atomically.
- Ensure repulsion and movement against a force have a real energy source.
- Integrate with environmental material interactions and chemistry.
- Remove obsolete parallel mechanisms only after replacement behavior is verified.

**Acceptance**
- Independent energy-inventory checks balance.
- Attraction and repulsion conserve energy.
- Failed transactions leave physical and energy state unchanged.
- Existing COMBINE and BREAK behavior remains valid.
- No energy is created or counted twice.

## Dependency and merge policy

PRs are stacked in order: PR 2 depends on PR 1; PR 3 depends on PR 2; PR 4 depends on PR 3. Each is a draft until its own acceptance criteria pass. No PR is merged automatically. Existing unrelated open chemistry PRs are not modified by this roadmap.
