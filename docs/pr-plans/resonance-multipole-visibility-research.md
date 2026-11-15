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

## 14. Fixed-sample translational-gradient bounds — 2026-11-14

The source-specific boundary-sample expansion now differentiates its own
first-order scalar approximation with respect to both body reference
positions. These are not independently assigned forces: each gradient is
the analytic derivative of the same approximate scalar potential.

For fixed sample positions and weights, write `g(p)=f_a(p)f_b(p)` and
`f_i(p)=1/(1+|p-c_i|)`. The exact center gradients are

`dS/dc_a = -sum_j w_j f_b(p_j) grad(f_a)(p_j)`

and the corresponding expression with a and b exchanged.

The approximation differentiates
`S_hat=M_0 g(q)+grad(g)(q) dot M_1` while holding the exact
sample positions and signed weights fixed. Its source derivative is

`dS_hat/dc_a = -M_0 f_b grad(f_a) - f_b H_a M_1
                - grad(f_b) (grad(f_a) dot M_1)`,

with the receiver derivative obtained by exchanging a and b. The Hessian-vector
products are evaluated analytically for the radial kernel.

The force-error estimate applies Taylor's theorem to the center-gradient
integrand itself. For `h_a(p)=d[g(p)]/d(c_a)`, the implementation
uses the conservative spatial-Hessian bound

`K_a = T(d_a) F(d_b) + 3 H(d_a) G(d_b) + 3 G(d_a) H(d_b)`,

where `F(d)=1/(1+d)`, `G(d)=1/(1+d)^2`,
`H(d)=max(2/(1+d)^3,1/(d(1+d)^2))`, and

`T(d)=6/(1+d)^4 + 3(2/(d(1+d)^3)+1/(d^2(1+d)^2))`.

The receiver bound exchanges a and b. With `W=sum |w_j|` and
sample-cluster radius rho, the independent remainder estimates are

`epsilon_(grad_a) <= 0.5 W rho^2 K_a`

and

`epsilon_(grad_b) <= 0.5 W rho^2 K_b`.

The factor-three mixed-term coefficients deliberately overbound the tensor
contractions. Signed cancellation cannot reduce these bounds.

Tests compare both approximate center gradients with exact discrete
fixed-sample derivatives, including 256 deterministic randomized clusters.
The bounds are independent for source and receiver and are checked separately.

### Explicit limit

These are **fixed-sample center-translation** bounds only. Sample positions,
projected extents, signed frequency weights, visibility, and interval topology
are held fixed during differentiation. They do not bound the complete rigid
body pose derivative, sample motion, projected-extent derivative, or torque.
The complete-world finite-difference probe remains the oracle for those
omitted pose-dependent terms, but a converged finite difference is not itself
a conservative error bound. No live force or movement integration is
authorized by this milestone. The next work must separately derive and bound
the boundary-geometry/weight pose derivatives and per-body torque, with exact
fallback at visibility transitions.

## 15. Smooth boundary-pose derivative probe — 2026-11-14

A further test-only probe now isolates the geometric terms omitted by the
fixed-sample center-gradient bound. In the existing four-body asymmetric
fixture, it perturbs the source and receiver x/y/rotation coordinates
independently while remaining inside one visibility-topology region.

For every exact ordered source/receiver sample, it records central differences
of the receiver midpoint, outward normal, projected extent, and underlying
boundary representation (straight interval endpoints/normal or curved-arc
parameters). The probe requires sample counts and boundary types to remain
stable across both perturbations. A topology change is not differentiated;
it belongs to the exact/subdivision fallback path.

This separates four distinct pose-dependent quantities that the scalar
fixed-sample kernel expansion deliberately freezes: sample location, boundary
orientation, projected extent, and interval geometry. It confirms that
nonzero geometric derivatives exist in the actual production sample path and
therefore cannot be omitted from a complete force or torque evaluator.

This remains a differential observation test, not a conservative bound.
The next derivation must bound the derivatives of the interval endpoints and
their projected-extent weights inside a certified smooth visibility region.
At tangencies, edge-on transitions, interval creation/removal, or occlusion
topology changes, the evaluator must reject the derivative expansion and
retain exact evaluation or subdivide. Rigid-body torque must then be derived
from the same scalar potential, including both sample-motion and weight-motion
terms. No live integration is authorized.


## 16. Certified boundary-sample motion and weight-change envelope — 2026-11-14

The boundary-sample primitive now has a separate conservative envelope for
pose-driven changes to the exact discrete samples. This does not replace the
fixed-sample Taylor remainder. It bounds the additional change between two
sample configurations when a caller can certify, for every one-to-one matched
sample j,

- eta_j >= |p'_j - p_j|, the maximum boundary-midpoint displacement; and
- omega_j >= |w'_j - w_j|, the maximum absolute signed-weight change.

The sample ordering and visibility/interval topology must remain unchanged
throughout the pose cell. The closed disk around each original midpoint with
radius eta_j must remain strictly separated from both body centers. Missing
correspondence, a topology transition, a singular distance envelope, or an
uncertified displacement/weight bound rejects this path and requires exact
evaluation or subdivision.

For g(p)=f_a(p)f_b(p), f_i(p)=1/(1+|p-c_i|), define the minimum distances
over sample j's motion disk as

d_a,j = |p_j-c_a| - eta_j,
d_b,j = |p_j-c_b| - eta_j.

Let F(d)=1/(1+d), G(d)=1/(1+d)^2, and
H(d)=max(2/(1+d)^3, 1/(d(1+d)^2)). The disk-wide bounds are

|g| <= F(d_a) F(d_b),
|grad_p g| <= G(d_a) F(d_b) + F(d_a) G(d_b).

For the fixed-sample center-gradient integrands
h_a(p)=-grad(f_a) f_b and h_b(p)=-grad(f_b) f_a, product-rule bounds give

|h_a| <= G(d_a) F(d_b),
Lip(h_a) <= H(d_a) F(d_b) + G(d_a) G(d_b),

with the receiver expression obtained by exchanging a and b.

Therefore, for the exact discrete potential and its fixed-sample center
gradients, the extra geometry/weight changes are independently bounded by

epsilon_U <= sum_j [omega_j F_a F_b + |w_j| eta_j (G_a F_b + F_a G_b)],

epsilon_(grad_a) <= sum_j [omega_j G_a F_b
                + |w_j| eta_j (H_a F_b + G_a G_b)],

and

epsilon_(grad_b) <= sum_j [omega_j G_b F_a
                + |w_j| eta_j (H_b F_a + G_b G_a)].

The implementation uses absolute signed weights and adds an outward
floating-point accumulation guard. A deterministic differential test moves
every sample by less than its supplied displacement envelope and changes
every signed weight by less than its supplied envelope; it checks the exact
potential and both exact fixed-sample center-gradient changes independently.
Invalid, incomplete, non-finite, or singular envelopes are rejected.

### What this closes, and what it does not

This closes one missing error term: the change in the discrete potential and
fixed-sample translational center gradients caused by bounded sample-midpoint
motion and bounded signed-weight/projected-extent change. It composes with the
existing fixed-sample cluster Taylor bounds; the two error contributions must
be added, not substituted for one another.

The envelope does not itself derive eta_j or omega_j from a rigid-body
pose cell. Those geometry certificates must come from analytic boundary
endpoint/intersection sensitivities and a certified projected-extent
sensitivity bound. It also does not include the chain-rule force generated by
the pose dependence of sample positions or weights, and it does not bound
body torque. Those terms require per-body pose derivatives of the same scalar
potential and independent remainder bounds. Visibility tangencies, interval
creation/removal, edge-on projection transitions, and occlusion topology
changes remain exact/subdivision cases. The evaluator remains non-live.
## 17. Rigid-pose contribution to boundary-sample displacement — 2026-11-14

A first analytic component of the missing midpoint-displacement certificate is
now implemented in `resonance_boundary_far_field.rs`. For a point attached to
a rigid receiver, let R be its maximum distance from the body's rotation
pivot, let delta_t bound reference-point translation, and let delta_theta
bound the absolute angular change. The point's displacement is bounded by

`eta_rigid <= delta_t + 2 R sin(min(delta_theta, pi)/2)`.

The angular term is the maximum chord length swept by a point at radius R.
For angular bounds at or above pi, the expression saturates at the full
diameter 2R; this remains conservative for arbitrarily large rotations.
Non-finite or negative inputs fail closed. A deterministic test checks the
bound against 128 transformed points with varied radii, point angles,
translation directions, and rotation magnitudes, and checks the large-angle
saturation and invalid-input rejection.

### Scope of this component

This certifies only rigid transport of an already-corresponding boundary
point. It can contribute to eta for a receiver pose cell when the sample is
known to remain attached to the same local boundary feature. It does not
bound changes in the source-to-boundary ray intersection, interval endpoint
creation/removal, projected extent, or signed frequency weight. Those terms
must be added separately; a visibility or correspondence transition still
forces subdivision or exact evaluation. The helper is not yet wired into the
sample-motion envelope and does not authorize runtime approximation or
movement integration.

## 18. Strict ray/edge cut-motion envelope — 2026-11-14

A conservative bound is now implemented for one visibility cut generated by a
source ray through a blocker feature and its intersection with a straight
receiver edge. Let source s, blocker critical point v, receiver-edge start a,
and edge vector e=b-a. Define q=a-s and r=v-s. The exact intersection
parameters satisfy

`t = -cross(q,r)/cross(e,r)`,
`u = -cross(q,e)/cross(e,r)`,
`p = a + t e`.

The current polygonal visibility partition retains this cut only when t is
strictly inside the receiver edge and u is strictly greater than one (the
blocker feature lies beyond the receiver edge along the source ray).

Given point-displacement bounds ds, dv, da, db, the construction uses

`dq <= da + ds`,
`dr <= dv + ds`,
`de <= da + db`.

The determinant perturbation is bounded by

`dD <= de |r| + |e| dr + de dr`.

The numerator perturbations are independently bounded by

`dN_t <= dq |r| + |q| dr + dq dr`,
`dN_u <= dq |e| + |q| de + dq de`.

If `|D|-dD > 0`, the parameter changes satisfy

`dt <= (dN_t + |t| dD)/(|D|-dD)`,
`du <= (dN_u + |u| dD)/(|D|-dD)`.

The helper rejects unless these bounds preserve the strict interior-edge and
beyond-edge conditions. For the intersection point, it returns

`dp <= da + |t| de + |e| dt + de dt`,

with an outward floating-point guard. A deterministic 256-pose perturbation
test checks the bound against independently recomputed exact intersections;
invalid, near-singular, and topology-transition envelopes fail closed.

### Scope and next missing certificates

This is a local certificate for one already-identified straight-edge cut. It
does not prove that all critical cuts were enumerated, preserve the ordering
of multiple cuts, preserve blocker classification between adjacent cuts, or
certify a complete visibility topology. It also does not construct dv for
source-dependent circle tangencies or curved-boundary intersections.
Consequently it is not yet sufficient to derive a complete boundary-sample
motion envelope or projected-extent/weight bound. The next step is to compose
these local cut bounds with certified cut-order separation and blocker-feature
motion bounds, then derive projected-extent sensitivity from the moving
interval endpoints and source direction. Any unresolved event remains an
exact/subdivision case; no live evaluator or movement integration is
authorized.

## 19. Cut-order separation certificate — 2026-11-14

The local ray/edge intersection envelope now composes with a strict ordering
check for an already-enumerated list of cuts along one receiver edge. Each
cut is represented by its base edge parameter t_i and a certified absolute
parameter displacement delta_i. The certificate requires every interval

`[t_i-delta_i, t_i+delta_i]`

to remain strictly inside the edge and strictly separated from the preceding
cut interval. This proves that the supplied cuts cannot cross, merge, or reach
an edge endpoint inside the pose cell. Empty lists are valid; malformed,
overlapping, unsorted, and endpoint-touching envelopes fail closed.

This is only an ordering certificate for the supplied cuts. It does not prove
that all blocker features or geometric events were enumerated, nor that the
blocker classification between adjacent cuts is unchanged. A complete
visibility certificate must also bound every feature's motion, retain all
candidate cuts, and prove the visibility state on each open interval. No
approximate evaluator or live movement integration is authorized.
## 20. Straight-interval projected-extent motion bound — 2026-11-14

The exact point-source projected extent of a straight exposed interval now has
a conservative change envelope under bounded source and endpoint motion. For
edge vector e=b-a, length L, outward unit normal n, and source direction u(s)
along the edge, the existing extent is

`E = L integral_0^1 max(0, n dot u(s)) ds`.

Given source displacement ds and endpoint displacements da and db, define

`de = da + db`,
`dx = ds + max(da,db)`,
`L_min = L - de`,
`d_min = distance(source, segment[a,b]) - dx`.

The path rejects unless L_min and d_min are strictly positive. The edge-normal
change is bounded by `min(2, 2 de/L_min)`. Every corresponding edge point
moves by at most `max(da,db)`, so the source-direction change is bounded by
`min(2, 2 dx/d_min)`. Since the positive-part dot product is Lipschitz in both
unit vectors and lies in [0,1], the facing-factor change is bounded by the
minimum of one and the sum of those two direction changes.

Using `|L'-L| <= de`, the resulting extent-change bound is

`|E'-E| <= de + L min(1, 2 de/L_min + 2 dx/d_min)`,

plus an outward floating-point guard. A deterministic 256-pose test moves the
source and both endpoints independently and verifies the exact projected
extent remains inside the envelope. Edge collapse, source/edge collision,
invalid normals, and non-finite inputs fail closed.

### Scope

This bounds only the geometric projected extent of one already-corresponding
straight interval. It does not prove that interval endpoints remain
corresponding, certify visibility or cut ordering, cover curved intervals, or
bound the complete signed frequency-resolved sample weight. The latter also
depends on the pose-dependent source/receiver spectra and local attenuation.
It is one composable contribution to the future sample-weight envelope, not
a complete evaluator certificate.


## 21. Composed ordered-pair pose-cell error certificate — 2026-11-14

The boundary primitive now exposes a single composition point for the
certificates established by the preceding milestones. For one exact
source-specific ordered pair, `certify_boundary_pose_cell`:

1. Rejects supplied visibility-cut envelopes unless their order remains
   strictly separated and interior to the receiver edge.
2. Evaluates the existing first-order sample-cluster expansion and retains its
   independent potential, source-center-gradient, and receiver-center-gradient
   Taylor remainders.
3. Applies the per-sample motion/weight envelope to the exact discrete
   potential and both fixed-sample center gradients.
4. Adds the two independent error contributions componentwise, with no
   cancellation between them.

The returned error tuple is therefore

`epsilon_total = epsilon_cluster + epsilon_sample_geometry`

for potential, source translation gradient, and receiver translation gradient
separately. Invalid cut order, missing sample correspondence, singular sample
motion disks, non-finite bounds, and unresolved cut envelopes reject the
certificate.

### Explicit contract boundary

This is a composition layer, not proof that the caller supplied a complete
visibility description. The caller must still establish that every blocker
feature and visibility cut was enumerated, blocker classification is stable on
every open interval, and each sample has one-to-one correspondence throughout
the pose cell. The sample displacement bounds must include rigid transport,
ray/edge intersection motion, and interval geometry. Weight-change bounds must
include projected extent and signed spectral-coefficient changes.

The composed force bounds remain fixed-sample center-translation bounds. They
do not include the chain-rule force from moving sample positions/weights with
body pose, do not include torque, and do not cover curved-boundary endpoint
certificates. A complete mutual evaluator must add those independently bounded
pose derivatives for every affected body. Until then this API remains
non-live; exact evaluation or subdivision remains mandatory whenever any
precondition cannot be established.

## 22. Generalized pose chain-rule derivative and magnitude certificate — 2026-11-14

The ordered-pair boundary primitive now evaluates the exact discrete derivative
through one caller-supplied generalized pose coordinate. For each sample,
the caller supplies the derivative of its midpoint and signed weight with
respect to that coordinate, plus certified upper bounds for those derivatives,
the sample displacement, and the absolute weight change over the pose cell.
The caller also supplies source/receiver reference-point derivatives and
their magnitude bounds.

For one sample, write g_j = f_a(p_j) f_b(p_j), where
f_i(p)=1/(1+|p-c_i|). The exact derivative contribution is

dU_j/dq =
 w_j [(-grad f_a f_b) dot (dc_a/dq)
    + (-grad f_b f_a) dot (dc_b/dq)
    + grad_p(g_j) dot (dp_j/dq)]
 + g_j (dw_j/dq).

The implementation evaluates this expression sample-by-sample, preserving
signed weights and the exact ordered-pair sample correspondence. The last two
terms are the sample-position and projected/spectral-weight chain rule. They
are not inferred from finite differences and are not folded into a center
translation derivative. The same coordinate can represent x translation,
y translation, or body angle. For an angular coordinate, the caller supplies
the rigid and intersection-induced midpoint derivative and the weight
derivative; the resulting scalar is the corresponding torque contribution
about the declared pose pivot.

A separate absolute-magnitude envelope is accumulated over the complete
caller-certified pose cell. With d_a=|p_j-c_a|-eta_j and
d_b=|p_j-c_b|-eta_j, define F(d)=1/(1+d), G(d)=1/(1+d)^2.
The sample kernel and its spatial gradient satisfy

|g_j| <= F(d_a)F(d_b),

|grad_p g_j| <= G(d_a)F(d_b)+F(d_a)G(d_b).

Using W_j=|w_j|+the certified absolute weight-change bound, and caller-
supplied coordinate derivative bounds J_p,j and J_w,j, the chain-rule
magnitude is bounded by

sum_j {
  W_j [
    G(d_a)F(d_b) J_c,a
   +G(d_b)F(d_a) J_c,b
   +(G(d_a)F(d_b)+F(d_a)G(d_b)) J_p,j
  ] + F(d_a)F(d_b) J_w,j
},

where J_c,a and J_c,b bound the reference-point derivatives. The
implementation rejects missing samples, non-finite/negative envelopes,
understated derivative bounds, and sample motion disks that touch either
body. An outward floating-point guard is added.

### Contract and remaining work

This closes the analytic chain-rule evaluation and a conservative magnitude
bound for one generalized coordinate, including the angular/torque coordinate.
It does not derive the caller's midpoint or signed-weight Jacobians, certify
their variation over the pose cell, or prove visibility topology completeness.
Those remain upstream geometry obligations. The magnitude bound is not a
Taylor remainder for the approximate cluster derivative; this implementation
evaluates the exact discrete sample derivative and keeps it separate from the
cluster approximation. It therefore does not yet make the cluster hierarchy
scalable for pose derivatives and does not authorize live force integration.

The next step is to derive and test the missing caller-side Jacobians and
their pose-cell envelopes for the actual straight and curved boundary
constructions, including source-dependent ray/tangent intersections and
projected-extent/spectral-weight derivatives. The complete per-body x/y/angle
derivatives must then be checked against independent finite differences in
smooth visibility cells. Any topology event or unbounded sensitivity remains
an exact/subdivision fallback.

## 23. Straight-boundary pose Jacobians — 2026-11-14

The next geometry layer now has analytic first derivatives for two smooth
straight-boundary constructions.

### Ray/edge visibility cut

For a source s, critical point v, receiver-edge start a, and edge vector e=b-a,
the existing cut is defined by

D = cross(e, v-s),
N = -cross(a-s, v-s),
t = N/D,
p = a + t e.

For one generalized coordinate q, the implementation differentiates the
numerator and denominator directly:

D' = cross(e', v-s) + cross(e, v'-s'),
N' = -cross(a'-s', v-s) - cross(a-s, v'-s'),
t' = (N'D - ND')/D^2,
p' = a' + t'e + te'.

The helper preserves the same strict interior-edge and beyond-edge conditions
as the visibility cut. Singular determinants, endpoint cuts, and invalid
derivatives fail closed. The caller supplies v', which is valid for a rigid
polygon vertex and must be separately derived for source-dependent tangencies
or curved features. A central-difference regression checks the full cut-point
Jacobian under simultaneous source, critical-point, and edge motion.

### Straight interval midpoint and point-source projected extent

For an already-corresponding straight interval [a,b], the midpoint derivative
is (a'+b')/2. The interval's tangent and outward normal derivatives are
derived from e=b-a and e'=b'-a', preserving the current normal orientation.

With source offset r=s-a, tangent t, outward normal n, length L, along-edge
coordinate u=r dot t, and front-side distance d=r dot n, the exact point-source
projected extent is

E = d [asinh((L-u)/d) + asinh(u/d)].

The implementation differentiates L, u, and d through source and endpoint
motion, then applies the analytic partial derivatives of E with respect to
those three scalars. Edge-on transitions, degenerate edges, and invalid normal
orientation reject the derivative. A central-difference test exercises
simultaneous source translation and unequal endpoint motion.

### Scope boundary

These helpers provide local Jacobians, not a complete visibility certificate.
The ray/edge helper covers one known straight-edge cut; it does not enumerate
all cuts or classify every interval. The interval helper assumes the two
endpoints remain corresponding throughout the pose cell. Circle tangencies,
circle/edge intersections, curved receiver arcs, and cut-order/topology events
still need their own derivatives or exact/subdivision fallback.

The next step is to connect these local derivatives to the actual straight
visibility interval provenance without changing the public interval result,
then derive the signed sample-weight derivative from projected extent and the
frequency-resolved cross coefficient. Only after all endpoint types are
covered can the pose-coordinate chain-rule evaluator be fed by production
geometry.

## 24. Straight projected-weight derivative magnitude envelope — 2026-11-14

The straight-interval pose derivative now has an independent conservative
magnitude bound over a smooth pose cell. Inputs include source and endpoint
displacement envelopes, plus upper bounds on their derivatives with respect
to one generalized coordinate. The certificate rejects edge collapse,
invalid normal orientation, a back-facing or edge-on base configuration, and
any pose cell whose normal-distance lower bound reaches zero.

Let e=b-a, L=|e|, t=e/L, r=s-a, u=r dot t, d=r dot n,
and delta_e=delta_a+delta_b. The edge-length lower bound is
L_min=L-delta_e. The normal-direction change is bounded by

delta_n <= min(2, 2 delta_e/L_min).

With delta_r=delta_s+delta_a and R_max=|r|+delta_r, the front-side
distance lower bound is

d_min = d-delta_r-R_max delta_n.

The path rejects unless L_min and d_min are positive. It bounds the maximum
absolute along-edge coordinate by

u_max = |u|+delta_r+R_max delta_n,

and uses L_max=L+delta_e. For the exact projected extent

E=d[asinh((L-u)/d)+asinh(u/d)],

the partial derivatives obey |E_L|<=1, |E_u|<=2, and

|E_d| <= asinh(u_max/d_min)
       + asinh((L_max+u_max)/d_min) + 2.

If J_e bounds |b'-a'| and J_r bounds |s'-a'|, then

|L'| <= J_e,
|t'| <= 2 J_e/L_min,
|u'|, |d'| <= J_r + R_max |t'|,

so the implementation returns the conservative envelope

|E'| <= J_e + 2(J_r+R_max|t'|)
             + |E_d| (J_r+R_max|t'|),

with an outward floating-point guard. A deterministic perturbation test checks
the bound against 96 independently moved source/endpoint configurations and
their analytic directional derivatives.

This closes a local magnitude bound for the projected-extent derivative of
one already-corresponding straight interval. Multiplying by the absolute
frequency-resolved signed cross coefficient gives the corresponding projected
weight-derivative bound when that coefficient is pose-invariant. It does not
certify interval endpoint provenance, visibility completeness, spectral
coefficient variation, or curved-boundary weights. Those remain separate
inputs or exact/subdivision cases.

## 25. Straight-interval sample pose derivative composition — 2026-11-14

A composition helper now connects the straight-interval geometry derivatives to
the generalized discrete-potential chain-rule input. Given the exact source,
interval endpoints and normal; their derivatives; displacement envelopes;
derivative magnitude bounds; a scalar coordinate displacement; and a
pose-invariant signed spectral coefficient, it returns one
BoundarySamplePoseDerivative containing:

- midpoint derivative (the average of the two endpoint derivatives);
- projected-weight derivative (the signed coefficient times the exact
  projected-extent derivative);
- midpoint derivative and displacement bounds;
- projected-weight derivative and coordinate-span change bounds.

The projected-weight derivative magnitude uses the straight-interval
certificate from Section 24. Its coordinate-span change bound multiplies that
uniform derivative bound by the supplied maximum coordinate displacement.
This is valid only when the same coefficient remains constant over that
coordinate span. If the frequency-resolved coefficient changes with pose, its
derivative and change envelope must be added separately.

A focused test verifies the midpoint and signed projected-weight fields are
composed consistently and that their supplied bounds dominate the reference
derivatives.

This closes the local straight-interval-to-pose-chain-rule data path. It still
does not supply endpoint derivatives automatically from the visibility
solver, and therefore does not close production visibility provenance. The
next implementation step is to preserve endpoint source identity for each
returned interval cut, so rigid target vertices and source-ray/blocker cuts
can be differentiated without guessing which geometric feature generated an
endpoint. Line/line and circle/edge cuts, curved boundaries, and visibility
topology transitions remain separately handled or exact/subdivision cases.

## 26. Rigid-feature and circle-tangent critical-point Jacobians — 2026-11-14

Two analytic critical-point derivative primitives now cover the main point
features used by the straight visibility partition.

For a rigidly attached point p about pivot c, with translational derivative
c' and angular derivative omega, the world-point derivative is

p' = c' + omega J(p-c),

where J rotates a vector by +90 degrees. This is the exact rigid transform
derivative and is checked against central differences of combined translation
and rotation.

For a source-dependent tangent point on a circle with center c, radius R, and
external source s, let d=s-c, D=|d|, u=d/D, n=J u,
k=R/D, and h=sqrt(1-k^2). The two tangent points are

p = c + R(k u +/- h n).

Given source and circle-center derivatives, the implementation differentiates
D, u, k, h, and n analytically, then forms p'. It rejects sources on or inside
the circle, degenerate tangency (h approaching zero), invalid radii, and
malformed tangent-side selection. A two-branch finite-difference regression
checks the derivative while both source and circle move.

These primitives provide exact point derivatives for rigid polygon/line/arc
features and source-dependent circle tangencies. They do not yet retain the
feature identity in the production visibility solver, nor do they cover
circle/edge intersections, line/line intersections, or the motion of curved
receiver interval endpoints. The next step remains endpoint provenance: each
cut returned by the straight visibility partition must identify the feature
that generated it, so the appropriate derivative can be selected without
guessing. Unsupported provenance must fail closed to exact evaluation or
subdivision.

## 27. Conservative endpoint provenance for straight visibility intervals — 2026-11-14

The straight visibility solver now has an additive provenance-aware wrapper;
the existing exact interval API and its result type remain unchanged. The
wrapper pairs each returned interval with provenance for its start and end.

At this stage, an endpoint is labeled RigidTargetVertex only when its world
coordinate uniquely matches a local vertex of the target constituent after
the same rigid transform used by visibility construction. The record retains
body index, constituent index, and the exact local feature coordinate.
Every endpoint that does not meet this exact identity test—including
source-ray/occluder cuts and line/circle intersections—is labeled
UnresolvedVisibilityCut.

This deliberately conservative first pass avoids reconstructing provenance
from approximate coordinate guesses. A rigid target vertex can now be passed
to the rigid-point Jacobian from Section 26. An unresolved endpoint cannot be
differentiated through this wrapper and must remain on exact evaluation or
subdivision. Existing interval consumers are unaffected.

Two regressions cover the contract: an unoccluded polygon target yields rigid
vertex provenance for both endpoints, while an occluded target retains
unresolved provenance for at least one occlusion-generated cut.

### Next extension

The next geometry milestone is to preserve the identity of each generated
visibility cut at construction time, rather than only identifying unchanged
target vertices after the fact. Ray cuts must retain their blocker critical
feature; line/line and circle/edge intersections need explicit feature pairs
and derivative rules. Until each endpoint is either certified or explicitly
unsupported, no pose derivative may be supplied to the approximate evaluator.

## 28. Visibility-cut construction audit and event-identity contract — 2026-11-14

The first implementation audit of the straight visibility partition confirms
that endpoint provenance is currently lost in two distinct places:

1. Critical features are flattened into `(body_index, world_x, world_y)`
   tuples. Polygon and line feature identity (constituent and local feature
   coordinate) is discarded before the target-edge partition is built.
2. Each target edge accumulates scalar `t` values from source-ray/critical-
   point cuts, straight-boundary intersections, and curved-boundary
   intersections. Sorting and tolerance-based deduplication then discard which
   event or events generated each cut.

The provenance wrapper runs only after the exact intervals have been emitted.
It can uniquely match an endpoint to a target vertex, but it cannot recover a
blocker feature or distinguish a source-ray cut from a line/line or circle/edge
intersection. This confirms that the next change belongs inside construction,
not in the wrapper.

### Required construction representation

Replace the scalar cut list with a typed cut record containing:

- the target-edge parameter;
- one or more construction-event records;
- the event's exact participating feature identities and local parameters
  wherever the event is differentiable;
- an explicit unsupported/unresolved marker for event classes whose
  derivatives are not yet implemented.

The initial edge endpoints are target-vertex events and must retain the
target constituent identity and local vertex coordinate. A source ray through
a rigid polygon/line feature must retain the blocker body, constituent, and
local critical-point coordinate. Source-dependent circle tangencies must
retain circle identity and tangent branch/side, not masquerade as rigid points.
Straight/straight and circle/straight intersections must retain both feature
identities and their two intersection parameters/arc angle when available.

### Coincident-event policy

Cut deduplication must not choose an arbitrary event when two events coincide.
Merge their event sets deterministically. If the event set is incomplete,
contradictory, or cannot be represented, mark the resulting endpoint
unresolved. In particular, a target vertex coincident with an occluder cut, or
two different blocker features producing the same cut, must not inherit one
event's derivative by insertion order.

The exact interval coordinates and exposure classification must remain
unchanged by this refactor. Existing exact consumers continue to receive the
same `ExposedBoundaryInterval` values. The provenance-aware path is additive.
Tests must cover ordinary target vertices, rigid-feature ray cuts, duplicate
ray cuts from distinct blockers, ray/intersection coincidence, and unresolved
line/line and circle/edge events. The output must preserve every event or fail
closed; no coordinate-nearest fallback is permitted.

### Relation to the evaluator

This is a geometry-provenance prerequisite, not a new approximation. Once
event identity is retained, the existing rigid-point, source-dependent
circle-tangent, ray/edge-intersection, and straight-interval Jacobians can be
selected from construction data. Unsupported event classes still force exact
evaluation or subdivision. No live movement or approximate force integration
is authorized by this audit.

Separately, the full-world exact force oracle is now green with independent
finite-difference comparisons for every body's x/y force and torque, plus
whole-world linear- and angular-momentum balance including occluder reactions.
This validates the direct reference path; it does not validate a scalable
approximation or relax any of the event-provenance requirements above.


## 29. Straight/straight visibility-cut pose derivative — 2026-11-14

The strict interior intersection of a target segment and a blocker segment now
has an analytic generalized-pose derivative. For

`A + t(B-A) = C + u(D-C)`,

the helper differentiates both segment parameters and the world intersection
point from the derivatives of all four rigid endpoints. It rejects
near-parallel segments, endpoint intersections, non-finite inputs, and
non-finite derivative results. Those cases remain exact/subdivision cases;
the helper does not alter the exact visibility partition or classify an
unsupported event as differentiable.

The finite-difference regression checks both local parameters and both
coordinates of the intersection point under independent endpoint motion.
Separate regressions verify that parallel and endpoint events fail closed.

This supplies the local Jacobian primitive corresponding to the
`StraightBoundaryIntersection` provenance record from Section 28. It does
not yet wire event records to body-pose endpoint derivatives or certify
intersection ordering across a pose cell. Coincident events remain unresolved,
and circle/straight intersections plus curved receiver endpoints still require
their own derivative rules or exact/subdivision fallback. The next step is to
compose the event's retained target/blocker edge identities and local
parameters with rigid endpoint Jacobians, then validate the composed endpoint
derivative against the exact visibility output.


## 30. Rigid-body composition for straight-boundary intersections — 2026-11-14

The straight/straight intersection Jacobian now has a rigid-pose composition
helper. Given the world-space endpoints belonging to the target and blocker
edge identities retained by a `StraightBoundaryIntersection` event, plus each
body's pivot, translation rate, and angular rate, it derives all four endpoint
rates with the existing rigid-point Jacobian and feeds them into the
intersection derivative. Target and blocker may be different bodies and may
have independent rigid motions.

A finite-difference regression applies simultaneous translation and rotation
to both bodies and checks both segment-parameter derivatives and the world
intersection-point derivative. This validates the local chain-rule composition
for the supplied edge endpoints.

The helper deliberately does not look up geometry by body/unit/edge identity:
the visibility construction currently stores the edge indices and parameters,
but the exact realized world endpoints and their owning-body pivots must be
provided by the caller. It also does not certify that the same intersection
remains strict-interior or that event ordering is unchanged throughout a pose
cell. Those remain explicit caller obligations; near-parallel, endpoint,
coincident, or topology-changing events continue to require exact evaluation
or subdivision. The next milestone is a provenance-driven resolver that maps
the retained body/unit/edge identities to exact realized endpoints and composes
their body-pose Jacobians without coordinate-nearest matching.


## 31. Straight-intersection provenance retains local edge endpoints — 2026-11-14

A `StraightBoundaryIntersection` event now retains the local-frame start and
end points of both the target edge and blocker edge, in addition to their
body/unit/edge identities and intersection parameters. The endpoints are
recovered by inverse-transforming the exact world-space edge endpoints through
the uniquely matching placed constituent's current rigid pose. Duplicate or
missing constituent identities and non-finite transforms fail closed; no
nearest-coordinate reconstruction is used.

The straight-intersection provenance regression now checks that all four local
endpoints are present and finite. This makes the event record sufficient to
reconstruct the exact two rigid segments from their current pose and local
geometry, which is the missing data handoff needed by the rigid intersection
Jacobian from Sections 29–30.

This remains a geometry provenance primitive, not a complete pose-cell
certificate. The caller must still resolve each event's owning rigid poses,
compose the local endpoints into world positions, and prove that intersection
parameters stay strict-interior and event ordering remains stable over the
pose cell. Coincident cuts, collinear overlap endpoints, and near-parallel
events remain unresolved and require exact evaluation or subdivision.
