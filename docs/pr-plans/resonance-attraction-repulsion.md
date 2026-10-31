# PR 3 — Resonance-derived attraction and repulsion

**Goal:** Make interaction between materials emerge from the existing resonance aura, its received spectrum, and relative phase. Do not introduce a second spatial field or parallel propagation model.

## Existing resonance-aura mechanism (reuse this)

The current implementation already provides the spatial signal path in `src/perception/harmonics.rs`:

- `environmental_material_resonance` treats each realized physical material as one source, derives its aggregate oscillator response, and records its geometric center and extent.
- `environmental_contribution` projects that source onto an actual receiver boundary segment. Receiver orientation supplies directional gain; the source material's mass supplies the existing excitation scale.
- The aura attenuates continuously with distance using `1 / (1 + distance)`; there is no hard perception radius.
- The received `ToneSpectrum` preserves each component's frequency, amplitude, and phase. The resulting `ResonancePerception` retains source position, extent, spectrum, and magnitude.

Important boundary: this is currently an environmental-material-to-genome-cavity perception path. It proves that spatial aura propagation, attenuation, and geometry-aware reception already exist, but it is not yet a general material-to-material interaction API or a force model. PR 3 should generalize/reuse this same aura path for physical material pairs, rather than create another aura, attenuation curve, or independent field.

The source extent is currently recorded as metadata; the existing distance attenuation uses center-to-receiver distance. Do not silently reinterpret extent as a new force radius.

## Implement

- Generalize the existing aura source/receiver calculation so a realized physical material can receive another material's aura through its actual geometry.
- Preserve the existing aura's frequency-resolved amplitude and phase through reception; apply the receiver's oscillator response using the existing material-response model where appropriate.
- Derive the pair interaction potential from the physically justified interaction of the received spectra. Do not assume that phase difference alone determines a force, and do not add arbitrary coupling constants, polarity assignments, or distance laws.
- Use the existing spatial attenuation/geometric coupling as the spatial basis. Derive force from the spatial gradient of the resulting interaction potential.
- Establish attractive, repulsive, neutral, and equilibrium cases from the derived model.
- Require equal-and-opposite pair forces and ensure the same pair is not counted twice.
- Audit existing chemical attraction and define precisely where resonance becomes authoritative.

## Acceptance

- Pairwise reception reuses the established aura attenuation and geometry path; no second spatial field or arbitrary hard radius is introduced.
- The interaction calculation demonstrably depends on received frequency-resolved amplitude and relative phase.
- Phase relationship predicts interaction direction under the derived physical model, with no hard-coded resource polarity or material-specific exceptions.
- Pair forces are equal and opposite.
- Equilibrium and neutral configurations behave consistently.
- Force varies consistently with separation according to the derived aura/potential relationship.
- Existing chemistry reaction eligibility, chemical-position semantics, intrinsic bond potential, and transaction rules remain intact during migration.
- Tests cover source/receiver geometry, distance attenuation, phase-dependent attraction/repulsion, neutral/equilibrium cases, symmetry, and invalid/degenerate inputs.

**Dependency:** PR 2. Draft only until PR 2 is accepted.

## Existing chemistry interaction audit

The current chemistry interaction is a separate, always-nonnegative attraction model:

- `interaction_potential` derives a symmetric static magnitude from the absolute separation of the two chemical-position coordinates.
- `contact_factor` applies the spatial factor `(1 - distance / radius)^2` inside the interaction radius; it is zero outside that radius.
- `attraction` multiplies those terms by the configured maximum force. There is no relative oscillator phase, signed interaction, repulsive branch, or phase-dependent equilibrium.
- `formation_work_cost` integrates the existing chemical attraction over the remaining approach distance. The COMBINE evaluation path uses this as physical approach work, distinct from stored intrinsic bond potential.
- `combine_runtime` evaluates chemical interaction for live material interfaces; environmental COMBINE also persists/reads static chemical potentials through the chemistry library.

Resonance-derived interaction must replace the *physical force authority*, not erase chemistry's reaction eligibility, chemical-position reaction semantics, bond potential, or transaction accounting. Keep the old mechanism until the new aura-derived force and its work settlement are independently verified; then migrate its movement/work consumers explicitly. Do not count the same attraction twice during transition.
