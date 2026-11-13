# Resonance multipole and guaranteed-visibility research note

**Status:** Research/design findings only; no production implementation authorized by this note.  
**PR:** [#356 — Implement physics-driven environmental bond displacement](https://github.com/coringilbert16-cmyk/EvoSim-/pull/356)  
**Branch:** `chemistry/bond-competition-physical-connection`  
**Purpose:** Preserve the mathematical findings and next-step constraints so work can resume without relying on conversation history.

## 1. Research conclusion

The strongest research direction is a **mutual, error-controlled multipole interaction method combined with a separate conservative visibility hierarchy**. This is a credible direction for scaling the existing resonance calculations without changing EvoSim's resonance law or introducing an arbitrary interaction radius.

This is not a claim that a conventional gravitational/electrostatic FMM can be transplanted unchanged. EvoSim's interaction includes extended-body boundary integration, frequency/phase information, geometry-dependent occlusion, and force/torque contributions on occluders. The actual kernel and its derivatives must be derived and bounded before a hierarchy is used in the runtime.

## 2. Existing implementation: mechanical pair potential

Relevant implementation:
- `src/perception/harmonics.rs`: `environmental_material_resonance`, `material_aura_receptions_with_geometry`, `directional_resonance_pair_potential_components_with_occluders`, `resonance_pair_potential_components_with_occluders`, `spectrum_cross_energy_components`.
- `src/geometry/source_exposure.rs` (module imported as `crate::source_exposure`): exact polygonal and circular-arc visibility interval calculations.
- `src/simulation/world_tone_snapshot.rs`: multi-body force/torque contributions and full-world correctness oracle.

### 2.1 Per-body spectral preparation

Each realized material is treated as one aggregate emitter. Its weighted material properties and the current tick determine its emitted `ToneSpectrum`; its mass produces the existing `environmental_mass_scale`. The spectrum is generated independently per body and is already subject to the existing harmonic-generation, energy-bound, and strongest-component rules.

The mechanical pair calculation can therefore prepare each participant's emitted spectrum once per tick and reuse it as frequency-resolved coefficients. Do not split a material into independent constituent emitters.

### 2.2 Directional mechanical interaction

For a source (a), receiver (b), and each exactly exposed receiver-boundary sample (p):

- The source incident spectrum is scaled by
  `mass_scale(a) * sqrt(projected_extent(p)) / (1 + distance(a,p))`.
- The receiver's own emitted spectrum is scaled by
  `mass_scale(b) * sqrt(projected_extent(p)) / (1 + distance(b,p))`.
- The mechanical potential uses the **incident** source spectrum and the receiver's own emitted spectrum. It does not apply a second receiver-response phase lag.
- Cross-energy is evaluated only between matching frequencies, with the existing phase-sensitive term
  `2 * A_a * A_b * cos(phase_a - phase_b)`.
- Contributions are accumulated by frequency over the visible samples.

Thus, after each body's emitted spectrum is prepared, the sample-level mechanical cross term is bilinear in the two bodies' frequency-resolved emitted amplitudes. The spatial weight contains the existing mass scales, projected boundary extent, and two distance attenuations. In schematic form:

```text
U_a->b(p,f) =
  2 * A_a(f) * A_b(f)
    * cos(phi_a(f) - phi_b(f))
    * mass_scale(a) * mass_scale(b) * projected_extent(p)
    / ((1 + distance(a,p)) * (1 + distance(b,p)))
```

The directional potential is summed over the exact exposed boundary samples. The symmetric pair potential combines both directional evaluations and averages their contributions by one half.

**Important qualification:** This is a discrete boundary-sample formulation in the current implementation, not yet a proven continuous surface integral. A multipole derivation must reproduce the actual sampling and projected-extent rules or prove a controlled approximation to them.

### 2.3 Why this is promising for mutual multipoles

The per-body emission calculation is separable from the pair geometry. Once spectra are prepared, frequency/phase components act as coefficients in the pair cross-energy. The remaining smooth spatial factors are based on `1/(1+d)` attenuations and receiver boundary geometry.

This suggests investigating expansions of the smooth spatial factors for well-separated spatial groups, while preserving frequency and phase in the aggregate representation. It does **not** yet establish a valid multipole expansion, a convergence domain, or an error bound for the full potential.

The approximation must account for:
- extended source and receiver geometry, not only center positions;
- receiver boundary sample positions, normals, and projected extents;
- orientation and changes in geometry;
- frequency-resolved amplitudes and phases;
- potential, translational-force, and torque errors independently.

## 3. Spectral nonlinearity: mechanical force versus activation

These are distinct computational paths and must not be collapsed into one aggregate-field approximation.

### 3.1 Mechanical pair potential

The pair potential compares the source's incident spectrum with the receiver's own emitted spectrum. It does not call the receiver's nonlinear drive-response transform for each incoming source. The emitted spectra are prepared per body, then uniformly scaled at boundary samples and compared through same-frequency cross-energy.

This gives the mechanical pair path a useful bilinear structure after per-body emission. Existing strongest-component selection remains part of the per-body spectrum definition and must be preserved exactly.

### 3.2 Reaction activation and received-field perception

The activation/perception path is different:

1. Prepare individual source emitters.
2. For each emitter and receiver boundary point, apply the receiver's `material_response_to_drive` to the scaled source spectrum.
3. Apply the existing energy bound and strongest-component selection to the response.
4. Add the per-emitter responses as complex phasors by frequency.
5. Apply strongest-component selection to the accumulated field.
6. Calculate spectral activation influence from the resulting field.

The receiver response is nonlinear in signal level through harmonic generation and energy bounding. Phasor interference also changes the final component amplitudes. Therefore, in general:

```text
TopK(sum_i response(source_i)) !=
TopK(response(sum_i source_i))
```

A single aggregate source spectrum cannot generally replace individual-emitter response without changing activation. Any accelerated activation evaluator needs its own derivation and conservative error control, or must retain direct per-emitter evaluation.

## 4. Occlusion and multi-body derivatives

The pairwise reception path builds realized body geometry, conservatively filters candidate occluders by a source-to-receiver envelope, then calls exact polygonal and circular-arc exposure routines. The visibility routines reason about angular intervals, minimum distances, geometric critical points, polygon edges, circles, lines, and curved fluid boundaries.

Consequences:

- Visibility is not a smooth distance-only attenuation. Which receiver intervals are exposed changes with relative geometry.
- A smooth spatial expansion is valid only within a region where its visibility classification and boundary correspondence remain valid, or where visibility changes are explicitly represented.
- Near silhouettes, tangencies, and occlusion transitions, a group may need subdivision or exact visibility evaluation.
- Occluders are mechanically affected bodies, not merely boolean blockers.

The current multi-body force calculation differentiates the same occlusion-aware pair potential with respect to source, receiver, and every supplied occluder, including each body's torque about its own center of mass. A hierarchical interaction must preserve these reaction contributions. A two-cell equal-and-opposite force alone is insufficient when intervening occluders participate in the potential.

The existing full-world finite-difference gradient is a correctness oracle, not an unconditional production tick-loop algorithm.

## 5. Candidate architecture to investigate

Use three cooperating layers:

### A. Mutual, error-controlled spectral multipoles

- Build a mutual spatial hierarchy / dual-tree traversal rather than independent one-way source aggregation.
- Preserve complex frequency-resolved amplitudes (or equivalent amplitude/phase pairs) and required spatial moments.
- Derive expansions from the actual `1/(1+d)` pair-sample kernel and boundary weighting.
- Derive acceptance from conservative, distribution-dependent error estimates—not a fixed radius or arbitrary opening angle.
- Bound potential, translational force, and torque separately.
- Track per-body accumulated error and outliers, not only a global average.

### B. Separate conservative visibility hierarchy

- Use bounding volumes and geometric tests to prove full visibility or full occlusion for groups when possible.
- Subdivide ambiguous groups, especially near silhouettes and tangent/critical configurations.
- Retain the exact current visibility calculation for unresolved or near-field cases.
- Preserve visibility-dependent derivatives on occluding bodies.

### C. Separate nonlinear activation evaluation

- Do not assume mechanical pair-potential linearity implies activation linearity.
- Preserve per-emitter receiver response and strongest-component ordering unless a distinct bounded approximation is proven.
- Use exact fallback when a conservative activation error bound is unavailable.

The hierarchy must also preserve linear and angular momentum. Symmetric mutual evaluation is important, but symmetric approximations can still create spurious torque. The force/torque formulation must be derived together and validated against the direct oracle.

## 6. Research precedent

These works motivate the architecture; they do not prove it for EvoSim's kernel.

- Barnes & Hut, *Error Analysis of a Tree Code* (1989): multipole approximation error analysis at cell, force, and trajectory levels. [PDF](https://isidore.co/misc/Physics%20papers%20and%20books/Zotero/storage/WQA4QPU5/Barnes%20and%20Hut%20-%201989%20-%20Error%20analysis%20of%20a%20tree%20code.pdf)
- Salmon & Warren, *Skeletons from the Treecode Closet* (1994): conventional opening-angle criteria can fail on pathological configurations; stronger distribution-dependent acceptance criteria provide better error control. [DOI](https://doi.org/10.1006/jcph.1994.1050)
- Dehnen, *A Very Fast and Momentum-Conserving Tree Code* (2000): symmetric mutual cell-cell interactions can enforce linear momentum conservation by construction. [arXiv](https://arxiv.org/abs/astro-ph/0003209)
- Dehnen, *A Fast Multipole Method for Stellar Dynamics* (2014): adaptive multipole/local expansions and error-based acceptance; also illustrates that practical performance depends on the application. [arXiv](https://arxiv.org/abs/1405.2255)
- Korobkin et al., *Conservation of Angular Momentum in the Fast Multipole Method* (2021): symmetric FMM can still introduce spurious torques; angular momentum requires separate attention. [arXiv](https://arxiv.org/abs/2107.07166)
- Leblanc & Poulin, *Guaranteed Occlusion and Visibility in Cluster Hierarchical Radiosity*: hierarchical geometric tests can prove full visibility/occlusion and subdivide ambiguous regions. [PDF](https://citeseerx.ist.psu.edu/document?doi=aeaf13d15ca5329e4ce519dc72f4171c9cc6cb64&repid=rep1&type=pdf)
- Chang & Shih, *Tighter error bounds and weighted error metrics for hierarchical radiosity* (1998): adaptive refinement can allocate more work near visibility boundaries. [DOI](https://doi.org/10.1002/(SICI)1099-1778(199804/06)9:2<53::AID-VIS175>3.0.CO;2-8)

## 7. Next research/engineering milestone

Before implementing an evaluator, write a mathematical specification that answers:

1. What is the exact discrete pair kernel, including boundary sample and projected-extent rules?
2. Which spatial and spectral moments are sufficient for a mutual expansion?
3. What is the valid expansion domain for the `1/(1+d)` factors over extended bodies?
4. How are visibility-consistent groups certified, and how are ambiguous groups subdivided?
5. How are forces and torques on source, receiver, and occluders obtained consistently?
6. What conservative remainder bounds cover potential, force, and torque?
7. How is per-body accumulated error controlled?
8. Which activation operations remain exact, and what separate bound would permit approximation?

Only after those derivations should a **test-only evaluator** be prototyped against the existing direct evaluator. Include randomized and adversarial cases: asymmetric extended bodies, aligned bodies, near and tangent occluders, narrow visibility slits, visibility transitions, strong phase cancellation, near-equal frequencies under the existing matching tolerance, and force/torque outliers.

Do not enable the approximation in the runtime until it matches the direct oracle within proven bounds and passes momentum, angular-momentum, and energy-accounting tests.

## 8. Scope and safeguards

- This note records research findings; it does not authorize changes to the resonance law or runtime behavior.
- Do not introduce an arbitrary interaction radius, unproven cutoff, or fixed opening-angle rule.
- Do not replace exact visibility with sampling that can leak or invent interactions.
- Do not aggregate nonlinear activation as though it were the mechanical pair potential.
- Do not use global-average error as the sole acceptance metric.
- Do not omit occluder forces/torques.
- Keep PR #356 unmerged until its separately approved physical/chemical implementation is complete and green.
- No extended experiments or benchmark campaign is authorized by this research note.
