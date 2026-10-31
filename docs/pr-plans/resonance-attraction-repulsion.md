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

The first implementation pass now generalizes this path in `material_aura_receptions`:

- It resolves both materials through `OrganismBodyGeometry::from_physical_material`. This API requires world-resolved placements and rejects stored materials with an unresolved `owner_relative_origin`; callers must resolve the owner's transform before asking for pair interactions.
- It uses `visible_polygonal_intervals` and `visible_circular_arc_intervals` to obtain exposed straight and curved receiver boundaries, rather than one representative point per constituent.
- It uses `point_source_projected_extent` and `point_source_projected_arc_extent` to retain each interval's projected extent.
- It preserves each received spectrum and its actual receiver point/normal. Amplitude scales with the square root of projected extent, consistent with the existing amplitude-squared energy convention.
- It applies the receiving material's existing oscillator response after the source aura attenuation.

At the end of the initial reception-only pass, these samples were not yet a summed pair potential or force. The current API now has both a simple pair-reception entry point and an occluder-aware variant that retains unrelated world materials as blockers while sampling only the selected receiver. The later implementation sections below describe the symmetric pair potential and the deliberately staged force authority.

## Implement

- Generalize the existing aura source/receiver calculation so a realized physical material can receive another material's aura through its actual geometry. Reuse `OrganismBodyGeometry::from_physical_material` and its constituent placements/realized shapes to resolve receiver boundaries; do not substitute bounding circles for the actual shape.
- Preserve the existing aura's frequency-resolved amplitude and phase through reception; apply the receiver's oscillator response using the existing material-response model where appropriate.
- Derive the pair interaction potential from the physically justified interaction of the received spectra. Do not assume that phase difference alone determines a force, and do not add arbitrary coupling constants, polarity assignments, or distance laws.
- Use the existing spatial attenuation/geometric coupling as the spatial basis. Derive force from the spatial gradient of the resulting interaction potential.
- Establish attractive, repulsive, neutral, and equilibrium cases from the derived model.
- Define one symmetric pair interaction potential from the two materials' reciprocal aura reception, then derive the equal-and-opposite pair forces from that single potential. Do not calculate two unrelated one-sided forces or count the same pair twice.
- Audit existing chemical attraction and define precisely where resonance becomes authoritative.

## Phase-dependent interaction term implied by the existing model

The code already contains the two ingredients needed for a non-arbitrary phase-dependent cross term:

- `combine_component` adds same-frequency components as complex phasors.
- `wave_energy_from_amplitude` defines spectral energy as amplitude squared, (E=A^2).

For two received components at the same frequency, phasor addition gives:

`|A_i exp(i φ_i) + A_j exp(i φ_j)|² - A_i² - A_j² = 2 A_i A_j cos(φ_i - φ_j)`

This is the phase-dependent cross-energy term naturally implied by the current representation. Components at different frequencies do not contribute to the cycle-averaged cross term. The received amplitudes already include the existing aura attenuation and directional geometric coupling.

This is the leading candidate for the pair interaction-energy term, not permission to treat signal aggregation as mechanical work without proof. PR 3 must establish a symmetric pair contribution from the actual reciprocal receiver geometry, show that the spatial dependence is supplied by the existing aura path, and verify that the resulting mechanical force `F = -∇U` exchanges energy consistently rather than double-counting the incoming aura signal. Do not add a free coupling constant or manually reverse the phase polarity. Under this candidate, constructive/in-phase overlap raises the cross-energy term, destructive/anti-phase overlap lowers it, and quadrature components contribute no cross term; the force direction then follows from the spatial gradient.

## Symmetric pair potential and gradient-derived force (implementation pass 2)

The draft now computes a directional boundary-integrated interference potential:

- At each exposed receiver boundary sample, compare the **raw incident spectrum** with the receiver's own emitted spectrum at that boundary.
- Keep the receiver-responded spectrum separately for diagnostics/perception. Do not use it for the interaction cross term: its oscillator phase lag has already been applied to the receiver's own emission, and comparing against it would apply that lag twice.
- Scale the receiver's own boundary field by its existing mass scale, the same square-root projected-extent factor, and the existing continuous center-to-boundary attenuation. The incident field already carries the source's corresponding projected-extent and distance factors.
- Integrate the same-frequency phasor cross-energy over the exposed boundary samples. No extra projected-extent multiplier is applied after the square-root amplitude factors.
- Average the two reciprocal directional integrals to define one symmetric pair potential. Preserve its signed contributions by frequency as well as a scalar sum, so energy settlement can use the matching resonance reservoir components.
- Derive the pair force as the negative spatial gradient of that one potential using a centered numerical derivative. The derivative step is a numerical resolution, not a new physical constant or force law. The returned forces are exact opposites by construction.

The regression cases cover two identical Carbon materials at the same shared phase (positive cross-energy that decays with separation and therefore repels), unlike Carbon/Hydrogen oscillator phases (negative cross-energy and attraction), and a controlled quadrature pair (neutral cross-energy). They also check argument-order symmetry, translation invariance, horizontal force direction, equal/opposite force vectors, third-party occlusion, stationary-potential invariance across shared phase-clock ticks, circular Water boundaries, and contact/shallow-overlap geometry. Exact-head CI is still required to validate the latest additions.

**Still outstanding before movement can consume this result:** test visibility-transition edge cases and establish robust equilibrium behavior for spatially varying configurations; reconcile time-varying potential with the PR 4 energy ledger and physical work; audit all current chemistry-force consumers and migrate authority only after the energy path is proven. Benchmark the finite-difference gradient and cache geometry-only reception data before enabling pair interactions at world scale; do not introduce a hard interaction radius to hide performance costs. A newly added regression verifies that the stationary pair potential is invariant as the shared phase clock advances. Third-party occlusion now reduces the pair potential, but an occluded pair force deliberately returns no result: with a fixed third body, the potential is three-body and the two-body equal/opposite shortcut is not its full gradient. Deriving and applying the blocker’s reaction force belongs to a later N-body interaction step. No movement consumer is activated in this pass.

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
