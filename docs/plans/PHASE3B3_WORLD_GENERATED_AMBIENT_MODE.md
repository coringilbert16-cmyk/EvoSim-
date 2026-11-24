# Phase 3B.3 — World-Generated Ambient Resonance Mode

**Status:** Initial isolated reference implementation added; exact-head CI and physical review pending.

**Authority:** The Phase 1 unified wave-field specification, the resonance-field implementation roadmap, and the November 24 user decision selecting intrinsic world generation for the ambient daily energy cycle.

## 1. Selected model

The initial ambient contribution is a spatially uniform environmental resonance component over the finite simulated-world accounting area. The world intrinsically generates energy as the ambient mode's stored energy rises through the daily cycle. The mode is not powered by a represented Sun, external oscillator, or second energy reservoir.

The finite world area is used only to convert energy density into the total energy inventory. It is not a physical propagation boundary. Local disturbances continue to use the existing nondispersive scalar wave model.

The selected reference values are:

- 24,000 logical ticks per simulated day.
- 440 carrier cycles per simulated day.
- Ambient cycle-averaged energy density from 1 to 10 project-energy units per project-area unit.
- A cosine daily envelope.
- Project-native time and energy units; no wall-clock or SI conversion.

The daily phase and carrier phase offsets are explicit configuration values. The initial reference defaults use zero offsets, making the envelope maximum occur at tick zero and the minimum at half a day. The offsets are not inferred from machine time.

## 2. Mode energy and carrier

Let (P=24{,}000) ticks, (N=440) carrier cycles per day, (K>0) be the field normalization, and (A_\Omega) be the finite accounting area.

The angular carrier frequency is

\[
\omega_0=\frac{2\pi N}{P}.
\]

For daily phase offset \(\phi_d\), the target cycle-averaged ambient energy density is

\[
e_a(t)=1+\frac{9}{2}\left[1+\cos\left(2\pi\left(\frac{t}{P}+\phi_d\right)\right)\right].
\]

The total energy assigned to the ambient mode is

\[
E_a(t)=A_\Omega e_a(t).
\]

For a harmonic-mode amplitude \(a(t)\), the mode-energy relation is

\[
e_a(t)=\frac{K\omega_0^2a(t)^2}{2},
\qquad
a(t)=\sqrt{\frac{2e_a(t)}{K\omega_0^2}}.
\]

The carrier phase is

\[
\theta(t)=2\pi\left(\frac{Nt}{P}+\phi_c\right)\pmod{2\pi},
\]

and the signed carrier displacement is \(b(t)=a(t)\cos\theta(t)\). The daily envelope changes the stored mode energy; it does not change the 440-cycle carrier frequency.

Here, \(e_a\) is the cycle-averaged energy density of the resonant mode, not the instantaneous kinetic-only energy density of the massless scalar wave equation. The oscillator-mode energy relation is kept distinct from the local disturbance-field energy relation.

## 3. World-generation accounting

For each logical tick, the implementation computes the next target mode energy and its exact inventory difference:

\[
\Delta E_a^n=E_a^{n+1}-E_a^n.
\]

The signed exchange convention is:

- \(\Delta E_a^n>0\): world generation increases ambient-mode energy.
- \(\Delta E_a^n<0\): world withdrawal decreases ambient-mode energy.
- \(\Delta E_a^n=0\): no net world exchange.

The two nonnegative cumulative totals satisfy

\[
E_a^n-E_a^0
=
E_{\mathrm{generated}}^n-E_{\mathrm{withdrawn}}^n.
\]

The opening mode energy is initial inventory, not silently counted as a generated transfer. Each subsequent tick's mode state and accounting counters are staged and committed together. Invalid values or tick overflow leave the state unchanged.

There is no separately stored source budget. The world-generation term is the explicit environmental exchange in the energy balance, not an untracked increment and not a second reservoir.

## 4. Separation from local propagation

This first implementation is deliberately an isolated mode-state and energy-accounting reference. It does not change the reference wave solver, does not add a Klein–Gordon term to the spatial field equation, and does not connect the ambient mode to materials, chemistry, organisms, or production energy accounting.

The local disturbance equation remains

\[
w_{tt}=c^2\nabla^2w
\]

with the existing nondispersive propagation behavior.

The complete spatial composition and coupling of the finite-area ambient component with disturbances on the infinite-plane propagation reference remain a separate integration gate. In particular, the finite accounting footprint must not be turned into a physical wall or boundary condition. This module therefore does not claim to have solved that spatial coupling problem.

## 5. Implementation and tests

Implemented in `src/physics/world_generated_ambient_mode.rs`, registered as a private isolated module in `src/lib.rs`.

The tests cover:

- 1-to-10 cosine envelope values at quarter-day intervals over exactly 24,000 ticks.
- 440 carrier cycles per simulated day.
- Amplitude reconstruction from the oscillator-mode energy relation.
- Per-tick signed world generation/withdrawal and full-day inventory reconciliation.
- Invalid configuration rejection.
- Bit-for-bit deterministic replay.

No production source registration, spatial wave-solver modification, material coupling, chemistry, organism behavior, or GUI change is included.

## 6. Remaining gates

1. Run exact-head formatting, Rust tests, architecture checks, and Clippy.
2. Review the ambient mode's energy interpretation and the finite-area/infinite-plane composition before any field coupling.
3. Derive and test the discrete source-work mechanism if the ambient component is coupled into the spatial wave solver.
4. Validate reciprocal material interaction against the complete field inventory before any production integration.

Passing the mode-state tests does not by itself validate spatial coupling or authorize production migration.
