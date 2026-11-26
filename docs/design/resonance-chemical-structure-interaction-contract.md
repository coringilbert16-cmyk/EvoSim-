# Chemical-Structure / Wave-Field Interaction Contract

**Status:** Isolated reference accounting kernel; not a production coupling or calibrated material model.

**Controlling roadmap:** [Resonance Field — Formal Implementation Roadmap](resonance-field-implementation-roadmap.md).

## Rule

A chemical structure interacts with the field only over its **actual exposed physical boundary**. For a boundary element with outward unit normal \(n\), the incident energy entering the material during an interaction interval is

\[
E_{\mathrm{in}} = \int_{t_0}^{t_1}\int_{\Gamma_{\mathrm{exposed}}}
\max(0,-J\cdot n)\,d\ell\,dt.
\]

The physical geometry owner supplies the complete realized boundary and visibility/occlusion result. No centroid, bounding circle, representative point, arbitrary radius, or camera/solver-window normalization is permitted. Unsupported or ambiguous geometry fails closed.

For each frequency component \(\nu\), the material's constitutive response partitions incident energy into four nonnegative destinations:

- \(a_w(\nu)\): absorbed directly as physical work for an eligible, active chemical reaction;
- \(r(\nu)\): reflected back into the field;
- \(t(\nu)\): transmitted onward through the material;
- \(s(\nu)\): scattered back into the field.

They obey

\[
a_w(\nu)+r(\nu)+t(\nu)+s(\nu)=1,
\qquad
E_{\mathrm{work}}=\int a_w(\nu)\,dE_{\mathrm{in}}(\nu).
\]

The work channel is zero when no eligible reaction is active. Absorbed field energy performs physical work one-for-one; there is no processing-efficiency multiplier and no intermediate usable-energy reserve. The work actually supplied cannot exceed the reaction's independently calculated required_external_input. Activation remains a separate kinetic gate, never an energy source.

Reflected, transmitted, and scattered energy remains in the field. It is not credited to chemistry. The frequency decomposition is an analysis of the one time-domain field, not a set of independently spendable frequency reservoirs; its estimator must preserve total incident energy, including cross-frequency terms. Coefficients are material/state/reaction constitutive properties and must be measured or otherwise justified. **No default coefficient or arbitrary value is introduced here.**

## Atomic settlement

The future geometry/field adapter must derive local incident energy from normal field flux, apply the material response, and stage the resulting field-to-work debit through the staged funding contract. The exact field debit must equal the reaction work credited. A reaction may commit only if activation, topology, energy balance, and geometry all pass. Commit installs both staged field and chemistry state; cancellation leaves both unchanged. Overlapping receivers must share the same local field energy and cannot independently claim the same flux.

If the interaction produces more absorbed work than the current reaction needs, the excess must have an explicit modeled destination before the interaction is allowed. It must not be silently stored as nonliving usable energy or silently converted to heat.

## Isolated reference kernel

The module src/physics/chemical_structure_coupling.rs implements only the frequency-component energy partition given already-integrated incident energy and explicit response coefficients. It validates coefficient bounds and the unit-sum rule, rejects invalid inputs, and tests conservation. It does **not** yet:
- sample or integrate physical geometry;
- estimate frequency components from the time-domain field;
- apply a boundary condition to the wave solver;
- choose/calibrate material coefficients;
- connect to reaction code, chemistry, or the production tick.

These are deliberate gates, not assumed capabilities.

## Validation and exit criteria before chemistry connection

1. Exact exposed-boundary integration converges under boundary quadrature refinement and preserves occlusion; unsupported geometry fails closed.
2. The spectral estimator reconstructs the integrated incident normal flux without dropping cross-frequency energy.
3. For every tested material response, all four fractions are nonnegative and sum to one; integrated energy partitions exactly within declared numerical tolerance.
4. The work channel is zero without an active reaction and never exceeds the reaction's independently computed deficit.
5. A local field/structure transaction debits the field by exactly the physical work credited, with atomic commit/cancel and no double claims for overlapping receivers.
6. Material coefficients are justified independently of the desired reaction outcome; no coefficient is selected merely to make chemistry proceed.

Only after these checks should the field be connected to the chemistry transaction. This contract does not authorize live-tick integration.
