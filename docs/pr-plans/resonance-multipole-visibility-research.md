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


## Implementation update — 2026-11-13

The exact environmental force evaluator now reuses the tick-local `MaterialResonanceEmitter` entries already prepared for free environmental chemistry. Pair source/receiver mass scales and emitted spectra are reused across the finite-difference samples. Center and extent are recalculated from each perturbed body's placements, so cached spectra do not freeze geometry. Receiver response to the incident spectrum, per-boundary projected extent, and exact visibility sampling remain in their original order.

A regression compares the prepared-emitter multi-body force/torque result against the uncached reference evaluator for the same three-body environment, with (10^{-10}) component tolerance. This validates the cache reuse without changing the resonance law. It is still an exact pairwise finite-difference evaluator; it is not a multipole implementation and is not suitable as the live evaluator for the standard environment.

The chemistry-only snapshot that supplies these emitters contains only free environmental atoms and molecules. No organism structures, organism-owned materials, or organism occluders enter the prepared-emitter force path.


## 9. First bounded kernel implementation — 2026-11-13

A first testable mathematical primitive is now implemented in
`src/perception/resonance_far_field.rs`. It expands only the smooth source-side
kernel for one receiver boundary sample and one frequency coefficient:

`f(r) = 1 / (1 + |r|)`.

For source positions (x_i=c+y_i), target (p), and signed coefficients
(w_i), define (R=p-c), (a=max_i |y_i|),
(M_0=sum_i w_i), (M_1=sum_i w_i y_i), and
(W_1=sum_i |w_i|). The first-order expansion is

`U ≈ M_0 f(R) - ∇f(R) · M_1`.

Its target gradient is

`∇U ≈ M_0 ∇f(R) - H_f(R) M_1`,

and the corresponding first-order force estimate on each source is
`F_i ≈ -w_i (∇f(R) - H_f(R)y_i)`. The source forces therefore sum to the
negative target gradient of this same approximate potential; they are not
assigned independently by an unrelated force rule.

The expansion is considered only when (d_-=|R|-a>0). With derivatives
evaluated at (d_-), conservative radial derivative bounds are used:

- (H_{max}=max(|f''(d_-)|, |f'(d_-)|/d_-)).
- (T_{max}=|f'''|+2|f''/d_- - f'/d_-^2|+
  2|f''-f'/d_-|/d_-).

The implemented absolute remainder bounds are:

- Potential: (epsilon_U le 	frac12 W_1 H_{max}a^2).
- Total force: (epsilon_F le 	frac12 W_1 T_{max}a^2).
- Each source's force: (epsilon_{F_i} le
  	frac12 |w_i|T_{max}a^2).
- Torque about the expansion center: sum each source's force-error bound
  multiplied by that source's distance from the center.

These bounds use absolute coefficient sums, so phase cancellation or signed
spectral cross-energy cannot make the error estimate artificially small.
Acceptance checks potential, force, and torque budgets independently.
Targets inside or touching the source cluster fail closed.

### Explicit limits of this milestone

This is a kernel primitive, not yet a complete mutual evaluator. It handles
only the source-side (1/(1+d)) factor for a fixed receiver boundary sample
and frequency coefficient. It does not approximate receiver boundary
integration, the second distance factor, projected extent, body rotation,
occlusion, or forces on occluders. No visibility group is certified by this
module; callers must retain exact visibility and use exact fallback until a
separate conservative visibility hierarchy is implemented.

The module is not connected to live movement. Its tests compare potential,
target gradient, and individual source forces against direct sums, verify
force balance for the approximate interaction, and reject unresolved
near-field geometry. This is the first derivation/prototype step; further work
must extend the same bounded construction to receiver boundary groups and
mutual cell-cell interactions, then validate torque and visibility against
the complete-world oracle before runtime use.


### Adaptive subdivision extension — 2026-11-13

The kernel primitive now also exposes `evaluate_adaptive`. It begins with the
complete source set, tests the same potential/force/torque remainder bounds,
and deterministically bisects a rejected cluster along its widest spatial
axis. Stable source indices are retained through subdivision, so per-material
forces and their error bounds return in original input order. A cluster's
error allowance is apportioned by its absolute coefficient weight; the torque
allowance is checked against the sum of each source's force-error bound times
its distance from the root center. This prevents cancellation and large
lever-arms from hiding error.

The adaptive result reports torque about the original source-set center and
converts each accepted subcluster's torque using its net force and offset from
that center. If a singleton target coincides with its source, evaluation
returns no result rather than inventing a direction for the nondifferentiable
radial kernel. Zero-weight groups contribute exactly zero.

Tests now exercise subdivision, global potential/force/torque budgets,
per-source force bounds, and stable ordering against direct sums. This makes
the smooth-kernel primitive usable as a bounded hierarchical building block.
It still does not establish identical visibility, account for source-shape
projected extent derivatives, or approximate receiver-boundary integration;
those remain explicit blockers before it can be called a complete resonance
far-field evaluator.



## 10. Exact discrete pair-kernel contract for the mutual evaluator — 2026-11-14

This section fixes the mathematical target before extending the source-only
Taylor primitive. It describes the current implementation, not a replacement
resonance law.

### 10.1 Directional sample contribution

For one directional interaction from material a to material b, let:

- A_a(f), phi_a(f) be the prepared emitted amplitude and phase of a at
  frequency f;
- A_b(f), phi_b(f) be the prepared emitted amplitude and phase of b;
- m_a and m_b be their existing environmental mass scales;
- p_j be the midpoint of one exact visible receiver-boundary interval;
- e_j be that interval's existing point-source projected extent;
- d_aj = |p_j - c_a| and d_bj = |p_j - c_b|, where c_a and c_b are
  the current environmental material reference positions.

The current directional implementation first scales each amplitude by its
own mass and square-root projected-extent/distance factor. Its same-frequency
cross-energy is therefore exactly

`u_{a->b,j}(f) =
  2 A_a(f) A_b(f) cos(phi_a(f)-phi_b(f))
  m_a m_b e_j /
  ((1+d_aj)(1+d_bj))`.

The directional potential is the sum over the exact visible intervals and
the matching frequency components. The symmetric pair potential is

`U_ab = 1/2 (U_{a->b} + U_{b->a})`.

Frequency matching retains the current 1e-9 absolute comparison rule. The
per-body emitted spectra, strongest-component selection, mass scaling,
boundary interval generation, projected-extent calculation, and visibility
semantics remain authoritative. No constituent-level emitter expansion or
spectral aggregation across bodies is permitted.

### 10.2 What the smooth kernel does and does not permit

For a fixed receiver sample p_j and fixed projected extent e_j, the spatial
factor is the product

`K(a,b,p_j) = 1 / ((1+|p_j-c_a|)(1+|p_j-c_b|))`.

The existing source-only expansion approximates only the first factor. A
complete evaluator must account for both factors in the same scalar potential.
It must not approximate the source factor for forces and then independently
invent receiver forces. Every translational force and body torque must be
obtained by differentiating the same accepted approximate scalar potential.

For a fixed p_j, define r_a=p_j-c_a and r_b=p_j-c_b. The position gradients
of the two smooth factors are

`grad_{c_a} (1+|r_a|)^-1 = r_a/(|r_a|(1+|r_a|)^2)`

and the analogous expression holds for c_b. These are only the fixed-sample
terms. They are **not** the complete receiver derivative: p_j and e_j are
functions of the realized receiver geometry and the source-dependent visible
intervals. The total derivative must also include the motion of the sample
location and projected extent, plus changes in the visible interval endpoints
where the visibility classification is differentiable.

At visibility topology changes, tangencies, interval creation/removal, or
other non-differentiable configurations, a Taylor cell is not accepted.
It must subdivide or use the existing exact finite-difference/world-potential
path. No derivative may be silently dropped to make a cell acceptable.

### 10.3 Group acceptance has two independent obligations

A mutual evaluator may accept a source/receiver cell interaction only when both
conditions hold:

1. **Visibility consistency:** every relevant source-to-receiver-boundary
   path is certified free of third-party occluders, or the cell is subdivided
   until that is proven. The visibility hierarchy's group certificate proves
   only this clear-path condition. An AABB overlap is ambiguous and requires
   exact evaluation or subdivision.
2. **Kernel approximation validity:** the full two-distance-factor,
   projected-extent-weighted, boundary-sample potential has conservative
   remainder bounds for potential, every participating body's force, and
   every participating body's torque. Visibility certification alone does not
   bound any of these errors and does not establish that different source
   bodies share identical receiver-boundary samples.

In particular, a group certificate does not authorize replacing the exact
receiver-boundary intervals by one representative sample, averaging
projected extents, or treating all members as if they shared one boundary
correspondence. The sample construction is source- and pose-dependent.

### 10.4 Required mutual approximation structure

The next evaluator prototype must approximate the complete scalar pair
potential above, not a stand-alone force kernel. Its accepted cell interaction
must retain enough information to recover:

- the frequency-resolved signed cross-energy coefficients;
- source and receiver spatial distributions;
- the exact receiver boundary/sample correspondence or a separately bounded
  approximation to it;
- derivatives of both distance factors;
- derivatives of projected extent and sample position;
- rigid-body rotational derivatives for every source and receiver;
- visibility status and the derivative validity region.

A candidate design may use tensor/product moments for the two distance factors,
but it must demonstrate a convergent expansion domain for the actual extended
body/sample geometry. It must not assume that multiplying two individually
bounded first-order expansions automatically gives a valid bound for their
product. Product remainder terms and cross terms must be included explicitly.

The exact small-world evaluator remains the oracle. Potential, per-body force,
and per-body torque errors are checked independently, including the worst
individual body rather than only aggregate error. Linear and angular momentum
balance are separate invariants. The evaluator remains test-only until all
bounds are proven and the live movement transaction can consume its output
atomically.

### 10.5 Immediate engineering gate

Before adding another approximation implementation, build adversarial
differential tests from the exact pair-potential API for:

- asymmetric multi-part source and receiver bodies;
- source and receiver rotations with nonzero torque;
- source-dependent changes in visible interval count and endpoints;
- projected-extent variation across a receiver boundary;
- clear group visibility and ambiguous third-party blockers;
- near-tangent and visibility-transition poses;
- strong positive/negative frequency cancellation and near-matched frequencies.

These tests must expose the exact potential and the complete-world per-body
finite-difference gradient. A proposed mutual evaluator must pass these cases
with reported conservative bounds before it can be connected to movement.


## 11. Receiver-sample correspondence is the next blocking derivation — 2026-11-14

The proposed bidirectional source-cluster traversal is **not yet justified by
the current exact implementation**. A critical dependency must be resolved
before implementing it.

The exact receiver boundary intervals are generated from the individual
source position. Their visible endpoints, midpoint samples, projected extents,
and even interval counts can therefore differ between source bodies aimed at
the same receiver. The source-only expansion currently accepts one fixed
receiver sample and one coefficient per source. It cannot combine sources
whose receiver samples differ by simply assigning them a common target.
Doing so would alter the discrete pair potential and could erase source-specific
visibility or projected-extent behavior.

The visibility group certificate does not solve this problem. It proves that
third-party body bounds cannot intersect the complete group-to-group segment
family. It does not prove that the exact receiver intervals or their sample
correspondence are identical across source bodies.

### 11.1 Required next engineering step

Before writing a mutual traversal, add a test-only differential probe over
small deterministic worlds that records, for every ordered source/receiver
pair:

- exact visible interval count and interval endpoints;
- exact interval midpoint positions and outward normals;
- projected extent per interval;
- per-frequency directional potential contributions;
- changes in all of the above under source and receiver translation/rotation.

The probe must include asymmetric multi-part bodies, clear paths, occluded
paths, and poses near visibility transitions. It must establish whether any
nontrivial source group has a provably common receiver-sample representation.
If not, the next mathematical task is a conservative approximation of the
source-dependent boundary integration itself, not merely a better expansion
of the distance attenuation.

### 11.2 Conditions for a future mutual traversal

A future source/receiver group evaluator may proceed only if it either:

1. preserves each source's exact sample correspondence while deriving a
   mathematically valid expansion over the varying sample locations and
   projected extents; or
2. introduces a new shared quadrature/sample representation and proves a
   conservative error bound against the current exact discrete interval
   calculation, including visibility and projected-extent errors.

For either path, potential, every body's translation force, and every body's
torque require independent bounds. All forces and torques must derive from the
same accepted approximate scalar potential. A visibility certificate alone
cannot satisfy these obligations.

Until the sample-correspondence problem is resolved, the existing
source-side Taylor primitive remains a fixed-sample kernel experiment only.
It must not be described as a mutual evaluator, report complete receiver
forces, or feed the movement accumulator. The exact world evaluator remains
authoritative and runtime integration remains blocked.


## 12. Multi-pair receiver-sample correspondence probe — 2026-11-14

A test-only probe now evaluates every ordered source/receiver pair in a
four-body deterministic world. Two independent emitters target an asymmetric,
two-part receiver while a third body acts as a possible occluder. For each
ordered pair, the probe retains the exact production reception records:

- receiver sample midpoint and outward normal;
- the underlying straight interval endpoints and normal, or exact circular
  arc center, radius, start angle, and sweep;
- point-source projected extent;
- incident frequency, amplitude, and phase components;
- frequency-resolved directional pair-potential contributions.

The probe repeats the complete ordered-pair evaluation after translating and
rotating one source and rotating the receiver. It also compares the blocked
world against the same world without the blocker. The test asserts that the
two independent sources aimed at the same receiver retain different exact
quadrature, and that pose changes and the third body affect the recorded
geometry or potential.

This is deliberately a differential observation test. It does not construct a
shared quadrature, average samples, aggregate source spectra, estimate
derivatives, or implement an approximation. It records twelve directed
interactions for the four-body world, preserving empty results as meaningful
fully occluded or non-contributing pair outcomes rather than fabricating
samples.

The implementation confirms that receiver identity alone is insufficient to
identify a common quadrature: each ordered source/receiver pair must retain its
own exact sample correspondence. The next mathematical task is therefore a
conservative approximation of source-dependent boundary integration that
preserves each pair's discrete correspondence, or a separately proven shared
quadrature with a conservative error bound against these exact records.
Potential, every participating body's force, and every body's torque still
need independent bounds before any mutual evaluator or runtime force path is
authorized.


## 13. Bounded source-specific boundary-sample expansion — 2026-11-14

The first conservative treatment of the source-dependent boundary sum is now
implemented as a testable scalar-potential primitive in
`src/perception/resonance_boundary_far_field.rs`. It does not construct a
shared quadrature. Each instance belongs to exactly one ordered source/receiver
pair and one frequency, and retains that pair's exact production interval
midpoints. The signed sample weight contains the existing spectral cross
coefficient, both environmental mass scales, and that interval's exact
projected extent.

For sample midpoint p_j, source center c_a, receiver center c_b, and signed
weight w_j, the target discrete sum is

`S = sum_j w_j f(|p_j-c_a|) f(|p_j-c_b|)`, with `f(r)=1/(1+r)`.

For a disk centered at q with radius rho containing every exact p_j, define
`delta_j=p_j-q`, `M_0=sum_j w_j`,
`M_1=sum_j w_j delta_j`, and `W=sum_j |w_j|`.
The first-order sample-cluster approximation is

`S_hat = M_0 g(q) + grad(g)(q) dot M_1`,
where `g(p)=f(|p-c_a|)f(|p-c_b|)`.

The Taylor remainder is bounded over the complete disk by

`|S-S_hat| <= 0.5 W rho^2 H_g,max`.

Let `d_a=|q-c_a|-rho` and `d_b=|q-c_b|-rho`. The evaluator rejects
unless both are strictly positive. For either radial factor, conservative
bounds throughout the disk are

`F(d)=1/(1+d)`,
`G(d)=1/(1+d)^2`,
`H(d)=max(2/(1+d)^3, 1/(d(1+d)^2))`.

The product Hessian operator norm is bounded by

`H_g,max = F(d_b)H(d_a) + F(d_a)H(d_b) + 2G(d_a)G(d_b)`.

The implementation uses absolute sample weights, so signed spectral
cancellation cannot shrink the remainder estimate. It adds a small outward
floating-point accumulation guard. Non-finite inputs, overflow, and disks
touching either body fail closed.

The test-only differential suite compares the approximation with the exact
discrete sum for mixed-sign samples, strong cancellation, and a zero-radius
cluster; it also verifies rejection when the sample disk touches either body.
The production visibility and interval construction are untouched. No sample
is moved, merged across ordered pairs, or assigned another pair's geometry.

### Explicit limit of this milestone

This is a bounded approximation of the discrete boundary-sample sum for one
ordered pair and one frequency. It is not a continuous boundary integral and
does not approximate visibility or interval endpoints. It returns a scalar
potential estimate and a conservative scalar-potential remainder only.
It does not yet return body forces or torques. Those require differentiating
the same expansion with respect to source and receiver translations and
rotations, including the pose dependence of the exact projected extents and
the validity region of the sample disk. Until independent conservative bounds
for every body's force and torque are derived and tested, this primitive must
remain test-only and must not feed the movement accumulator or mutual
hierarchy. Ambiguous visibility and visibility-topology transitions remain
exact/subdivision cases.
