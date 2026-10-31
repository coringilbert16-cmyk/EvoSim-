# PR 2 — Driven oscillator response and propagation

**Goal:** Complete the time-dependent phase response of the existing resonance model, building on PR 1's common simulation clock and on-demand phase calculation.

## Preserve
- Natural frequency derived from cohesion and mass.
- Reactivity-based damping.
- Existing amplitude response and energy bounds.
- Harmonic generation and phasor combination.
- Frequency-only retained-energy reservoir.

## Implement
- Derive oscillator phase lag from the existing amplitude-response equation, with sign convention explicitly tested.
- Apply response phase consistently to fundamental and harmonic components.
- Audit propagation delay and speed. Do not invent a propagation speed or arbitrary phase offset to make the feature appear complete.
- Keep all phase calculations analytical/on-demand.

## Acceptance
- Correct response phase below, at, and above natural resonance.
- Correct damping-dependent phase response.
- Correct harmonic phase relationships.
- No arbitrary propagation offsets.
- Existing resonance and energy tests remain valid.

**Dependency:** PR 1. Draft only until PR 1 is accepted.

## Propagation audit result

The current signal path orders geometry hits and transforms the spectrum through each material during the same simulation tick. Distance is used for geometric attenuation, but the model has no defined finite propagation speed, time-of-flight state, or distance-to-delay conversion. Therefore this PR implements oscillator response lag only. It does **not** add propagation phase delay; that remains blocked until the physical model establishes a propagation speed and its units.
