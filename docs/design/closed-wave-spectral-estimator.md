# Closed wave-field spectral incident-flux estimator

**Status:** Implemented as an isolated estimator; exact-head CI and the independent analytic temporal-accuracy test pass. Material response and chemistry connection are not authorized by this document.

**Controlling documents:** [Resonance Field — Formal Implementation Roadmap](resonance-field-implementation-roadmap.md) and [Chemical-Structure / Wave-Field Interaction Contract](resonance-chemical-structure-interaction-contract.md).

## Required input

The estimator operates on synchronized boundary traces from the same solver substep:

- normal velocity component \(u_t(t)\), at the leapfrog half-step time centre;
- outward-normal displacement gradient \(\partial_n u(t)\), reconstructed at that same half-step centre;
- boundary quadrature weight and the already validated exposed-geometry interval identity.

It must not infer frequency from unsigned energy density or from a magnitude-only spectrum. Complex coefficients (amplitude and phase) are required.

## Flux reconstruction

At each boundary quadrature location, the signed normal flux is

\[
q(t)=J(t)\cdot n=-Kc^2u_t(t)\partial_nu(t).
\]

For a finite observation window, represent both synchronized traces with a complex Fourier expansion, including the conjugate negative-frequency terms required for real-valued reconstruction:

\[
u_t(t)=\sum_k V_k e^{i\omega_k t},\qquad
\partial_nu(t)=\sum_l G_l e^{i\omega_l t}.
\]

The signed flux is the full pairwise product

\[
q(t)=-Kc^2\sum_k\sum_l V_kG_l e^{i(\omega_k+\omega_l)t}.
\]

The \((k,l)\) cross terms are essential. Do not replace this expression with a sum of independent per-frequency powers, discard phase, or apply the incoming-flux clamp to individual modes. Reconstruct the complete signed flux first, then apply the physical one-sided rule

\[
P_{in}(t)=\max(0,-q(t)).
\]

Integrate that reconstructed incoming power over time and exposed boundary using the same declared spatial and temporal quadrature as the reference time-domain path. The estimator's total must reconcile to the reference incident-energy integral within a tolerance established by convergence tests.

## Accounting rules

- The spectrum is a representation of one field, not a collection of independently spendable frequency reservoirs.
- Preserve complex phase and all pairwise cross-frequency terms through reconstruction.
- Apply the positive-incoming-flux clamp only after complete signed-flux reconstruction.
- Report the total incident-energy reconciliation residual explicitly.
- Do not silently normalize a spectral result to force agreement with the time-domain total.
- Do not assign negative cross-term contributions to standalone material energy accounts.
- Frequency-dependent material response remains a later constitutive operation. This estimator does not select or calibrate material coefficients and does not authorize energy settlement.

## Validation gates

1. A single-frequency travelling-wave trace reconstructs its signed flux and incident integral.
2. Two-frequency traces reproduce constructive and destructive interference as relative phase changes; a magnitude-only or independent-bin implementation must fail these tests.
3. The complete pairwise reconstruction agrees with direct time-domain multiplication before the incoming clamp.
4. Applying the clamp after reconstruction reproduces the direct positive-incoming integral, including intervals where the signed flux changes sign.
5. Spatial and temporal refinement reduce the reconciliation residual; report the measured convergence rather than widening tolerance to hide error.
6. Repeated runs with identical samples produce bitwise-deterministic coefficient ordering and stable totals.
7. Invalid, nonuniformly sampled, or insufficiently resolved windows fail closed or return an explicit unsupported status; do not fabricate a frequency estimate.

## Implementation record

The first isolated Rust implementation is in `src/physics/closed_wave_spectral_estimator.rs`. It uses a deterministic full complex DFT, forms the complete circular pairwise coefficient convolution for signed flux, reconstructs the signed samples, and only then applies the incoming clamp. It reports the direct time-domain reference total, reconciliation residual, and maximum signed-flux reconstruction error.

The current implementation accepts uniformly sampled finite windows and a caller-declared upper physical frequency bound; it fails closed for nonuniform samples, invalid values, and windows where the doubled product bandwidth reaches Nyquist. The caller remains responsible for supplying a truthful band limit. It uses composite trapezoidal time integration for both reconstructed and direct-reference powers and applies the supplied boundary quadrature weight. The DFT assumes the finite sample window's periodic Fourier representation; the declared frequency bound is an input contract, not an inferred guarantee. It remains an isolated estimator and does not calibrate or settle material response.

Focused tests cover a single tone, phase-sensitive two-tone traces, post-reconstruction incoming clamping, invalid/nonuniform/under-resolved windows, deterministic repeatability, additive boundary weights, and temporal refinement. An additional independent analytic test uses the exact integral of the known incoming-power signal and verifies that 33-, 65-, and 129-sample estimates approach the analytic result, with the finest estimate within 2e-4. This is stronger than comparing the spectral reconstruction only with a direct calculation from the same samples. Exact-head CI passed at `f6e1eff4b6c9a9b60dcf75b518e8d275bbdb9dc0` (run `46018447526`), including formatting, architecture checks, offline analyzers, Rust tests, chemistry smoke simulation and analyses, manifest validation, and Clippy. This closes the initial isolated estimator implementation/CI gate and establishes a first temporal-accuracy result; it does not yet establish spatial quadrature convergence, robustness for non-periodic windows, or a general error bound.

## Explicit exclusions

No material coefficient calibration, curved-boundary integration, reaction-work absorption, field debit, chemistry call, or production-tick integration is included. Those remain separate gates after the estimator is validated.
