# Resonance movement support cutoff

The inverse-square resonance interaction is continuous and does not reach mathematical zero at finite distance. Environmental movement therefore uses an explicit model-resolution threshold rather than a geometric radius or floating-point epsilon.

## Resolution choice

The minimum meaningful displacement from one isolated material pair is **0.001 world units**. Hydrogen's realized constituent width is 0.1 world units, so this is 1% of that smallest reference feature.

This is a modeling resolution choice, not a claim that smaller interactions are physically nonexistent. It makes the ignored contribution explicit and testable.

The existing bounded movement response is:

`r = 0.1 s / (1 + 0.1 s)`

Solving for the minimum interaction strength at `r_min = 0.001` gives:

`s_min = r_min / (0.1 (1 - r_min))`

For a pair, the existing signed cross-energy and mass scaling give the interaction numerator:

`N_ab = |cross_energy_ab| mass_scale_a mass_scale_b`

The existing attenuation is `N_ab / (1 + d)^2`. Therefore the maximum center separation at the resolution boundary is:

`d_max = sqrt(N_ab / s_min) - 1`

A negative result means that pair never reaches the configured per-pair resolution, even at zero separation.

## Efficient discovery

The spatial sweep uses a conservative amplitude-norm bound derived from Cauchy-Schwarz to avoid evaluating every possible pair. It then applies the exact frequency- and phase-aware cross-energy calculation to each broadphase candidate and retains only pairs inside their pair-specific `d_max`.

The exact force, phase, frequency, mass, attenuation, and bounded movement-response calculations are unchanged for retained pairs.

## Explicit tradeoff

The cutoff is applied per pair. Several individually sub-threshold interactions that point in the same direction are not accumulated. This is the deliberate consequence of defining a per-pair movement resolution and should be evaluated in matched simulation runs. If that loss materially changes behavior, the resolution value or cutoff semantics must be revisited rather than hidden behind a different radius.

No mechanical energy system, new resonance field, additional energy reservoir, or propagation law is introduced by this optimization.
