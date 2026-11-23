# Phase 3B.1 — Source-to-Field Interface Audit

**Status:** Audit complete; design decisions and implementation gaps recorded. No production behavior or source/energy settlement code changed.

**Authority:** `docs/design/resonance-field-implementation-roadmap.md` and `docs/design/unified-energy-physics-specification.md`.

**Scope:** Source identity and provenance, source classes and geometry, deterministic scheduling, finite source ownership, the source-to-field transfer contract, and reuse boundaries for the isolated Phase 2 reference solver.

**Explicitly excluded:** Receiver/material settlement, material-origin oscillators, chemistry, movement, organism systems, GUI, source calibration, and production migration.

## 1. Executive result

The repository contains reusable pieces, but **does not yet contain a source-to-field interface suitable for real sources**.

- The isolated Phase 2 solver has a finite-budget mathematical source and an atomic, signed transfer based on the exact change in its discrete field-energy invariant. It accepts full-grid displacement/velocity profiles, not physical source objects, source geometry, power density, or source schedules.
- The live sun path remains a legacy rotating source with a 2,400-tick orbit and per-projected-width incident allowance. It is not the approved external 3D radiative source and has no authoritative finite world-owned source reserve.
- The live hotspot path remains a per-material-area settlement against realized material placements. Its rate depends on the retired seconds-per-day conversion; it has no normalized area-integrated power law or independent finite source budget.
- The checked Phase 3A.4d source-footprint adapter measures polygon-union area only. It does not register sources, integrate power density, or transfer energy.
- Existing energy-ledger helpers provide validation and transaction patterns, but their legacy source semantics are not a suitable source identity/budget model for the field.

**Conclusion:** Phase 3B.1 can define a small, source-class-aware contract and its tests. It should not start by adapting the live sun/hotspot settlement functions or by changing the production ledger.

## 2. Verified repository state

### 2.1 Phase 2 reference solver

Reviewed the isolated implementation in the closed, unmerged PR #410 branch, `src/physics/wave_field.rs`.

Reusable behavior already demonstrated:

- `FiniteEnergySource` holds an initial budget, remaining available energy, and cumulative signed net debit.
- `WaveField::inject_profile_at_current_time` applies a validated displacement/velocity profile at the current logical time.
- The transfer is calculated from the exact difference between the pre- and post-injection discrete leapfrog energy.
- Positive transfer debits the source; negative transfer returns energy to it.
- Field update and source-budget mutation are staged and committed atomically; insufficient funding or invalid state leaves both unchanged.
- Emission can occur after propagation begins, so propagation delay is measured from the actual activation time.

Limits relevant to Phase 3B:

- The solver source has no stable source ID, provenance, physical source class, shape, frequency schedule, phase schedule, or replenishment policy.
- The injection API accepts two arrays covering the entire numerical grid. It does not accept a spatial power-density function or integrate a physical source over area.
- The source object is a mathematical test budget, not a production energy holder or ledger transaction.
- The solver module is intentionally isolated and unmerged. This audit does not reopen Phase 2, request that PR #410 be merged, or authorize production integration. Its validated behavior is a reference contract only.

### 2.2 Existing live sun

Reviewed `src/simulation/world_tone.rs`, its runtime configuration/sample types, and the Phase 0 call-path audit.

Current implementation characteristics:

- `WorldToneRuntimeConfig` derives source orbit/position from finite rectangular world width and height.
- The default orbit period is 2,400 ticks, conflicting with the approved 24,000-tick orbit/day.
- The sample contains source position, 440-cycle/day carrier frequency, orbital phase, and intensity of 1–10 energy units per tick per normalized projected width.
- The source sample/cache stores deterministic source outputs, not a finite owned source reserve.
- The active production path settles incident input against material recipients. The Phase 0 audit records that the allowance scales with eligible constituent count and does not create a persistent propagating field.
- The approved source model instead requires an isotropic 3D external source, a vertical circular orbit above the infinite plane, retarded emission time, finite emitted power, and exact half-power interception by the plane.

**Disposition:** Keep the current implementation as legacy comparison behavior until a separately approved migration. Do not carry its finite-world orbit, 2,400-tick period, per-width rate, or per-recipient allocation into the new source interface.

### 2.3 Existing live hotspot

Reviewed `src/simulation/hotspot.rs`.

Current implementation characteristics:

- `RadialHotspot` stores center, radius, peak energy per tick per realized material area, and frequency.
- Its default rate is derived from 1,000 energy units per simulated second using the obsolete 86,400-seconds/24,000-ticks conversion.
- Settlement iterates realized environmental material placements, multiplies local intensity by each material's area and amount, then stages retained resonance/heat/escape against the legacy reservoir and ledger.
- There is no independent finite source reserve, total source power, or source replenishment transfer.
- Its current radial cosine profile is not normalized to unit integral over its physical support.

The approved model is a localized in-plane distributed source:
`q_h(p,t) = P_h(t) g(|p-p_h|)`,
where `g` is compactly supported and normalized over physical area. Integrated power must not depend on molecule count, receiver count, grid spacing, or camera scale.

**Disposition:** Reuse the intended hotspot identity/location/profile concept only where it remains valid. Retire the legacy rate conversion and material-recipient settlement as physical authorities during a later migration. Do not infer a calibrated `P_h` from the old rate.

### 2.4 Existing energy accounting primitives

Reviewed `src/chemistry/energy_ledger.rs`.

Reusable engineering patterns:

- Finite-value and nonnegative-budget validation.
- Staged mutation and atomic commit/reject behavior.
- Explicit source categories in transactions.
- Independent energy inventory and conservation residual checks.
- Rollback on failed settlement.

Not directly reusable as the field source contract:

- `ExternalEnergySource` currently distinguishes only `InitialCondition` and `WorldTone`; it does not identify individual physical source instances or their provenance.
- `EnergySourceBudget` is documented as an accounting allowance whose scope is defined by the caller. Its legacy comments describe per-interval/per-receiver source availability, not one authoritative lifetime source holder shared across emissions.
- The legacy world-tone allowance is not a finite global source reserve and does not represent energy remaining at the sun or hotspot.
- The current ledger's source/receiver transaction shape assumes immediate external input allocation. Field emission instead transfers energy into persistent field state; it must not be counted as immediate material capture.

**Disposition:** Preserve validation, staged commit, provenance, rollback, and reconciliation patterns. Define the field's source-holder and transfer semantics independently before deciding which ledger structures can be adapted.

### 2.5 Existing geometry

Reviewed PR #422 and `docs/plans/PHASE3A4D_SOURCE_FOOTPRINT.md`.

Available:

- `checked_source_footprint_area` returns checked L² area for the union of supported simple polygon pieces in common world coordinates.
- Overlapping/coincident polygons count once.
- Unsupported, concave, malformed, degenerate, non-finite, or unresolved input fails closed.

Not provided:

- Transforming a source's declared local shape/pose into world-space support polygons.
- Circle or fluid support in the strict Phase 3 geometry adapter.
- Integration of spatially varying power density over a support region.
- Grid-cell overlap/area quadrature or its convergence/error contract.
- Source scheduling, power, budget, field injection, or accounting.

Important source-specific distinction:

- The hotspot is specified as a radial compact-support source. Its natural support is circular, while the current 3A.4d adapter intentionally supports polygons only. Do not silently tessellate the circle or substitute a bounding shape. Either use a validated analytic radial integral for the approved profile, or propose a separate exact circle-support geometry slice if a general geometric measure is actually required.
- The sun is not an in-plane bounded footprint. Its radiation density is defined over the plane and integrates to half of its emitted isotropic power; do not force it through `SourceFootprintArea`.

## 3. Approved physical source classes and contracts

Phase 1 already defines the following classes. Phase 3B source injection should initially implement only the external sun and in-plane distributed hotspot.

| Class | Physical measure | Energy owner | Phase 3B.1 disposition |
|---|---|---|---|
| External radiative source (sun) | 3D isotropic power and retarded source-to-plane flux | Named external source reserve or explicitly accounted replenishment | Define interface; implementation later |
| Distributed in-plane source (hotspot) | Power density integrated over physical support area | Named finite source reserve or explicitly accounted replenishment | Define interface; implementation later |
| Boundary/line emitter | Flux/power per physical boundary length | Named source reserve | No current source; do not add speculatively |
| Powered material oscillator | Material-owned internal energy and signed field work | The material's own internal-energy holder | Explicitly out of this source-injection slice |
| Passive material response | No source emission | Receives field energy later | Receiver-side phase, not 3B.1 |

Every implemented source needs stable identity and provenance, declared physical type and geometry, deterministic logical-tick schedule/waveform/phase, a finite authoritative energy holder or explicit replenishment origin, correctly dimensioned power, and an atomic transfer record. Source identity is not merely the source class: two distinct sources of the same class must not share or duplicate a budget accidentally.

## 4. Required source-to-field contract

The proposed interface should separate immutable source description from mutable source-energy ownership.

### 4.1 Immutable source description

A source descriptor needs:

- Stable source identity, distinct from its class.
- Provenance: external source or named internal holder.
- Source class and coordinate frame.
- Physical geometry/support definition.
- Frequency and phase law, distinct from any orbital/position schedule.
- Deterministic activation/emission schedule in logical ticks.
- A dimensionally explicit power law in E/tick, or power density in E/(L²·tick) for an in-plane distributed source.
- Explicit behavior when disabled, exhausted, underfunded, or subject to negative signed field work.

No camera, viewport, finite rectangular environment, wall-clock pacing, molecule count, or receiver count may enter source power or schedule.

### 4.2 Mutable energy ownership

The source-energy holder needs:

- Stable owner/source identity.
- Initial and remaining energy.
- Cumulative signed transfer/debit and any replenishment credits, separately identifiable.
- Explicit replenishment origin and schedule if the source is not a closed finite reserve.
- Atomic validation/commit/rollback with the field update.

A fresh budget must never be created per receiver, grid cell, or source sample. One physical source must have one authoritative budget shared by all of its emissions. Multiple source instances must have separate identities and budgets.

### 4.3 Source-to-field work

The Phase 1 forced-wave contract is authoritative. For distributed forcing, the signed power density is `K u_t f`; integrate its work over the actual physical support and accounting interval. Debit the source by exactly the net field work. Negative net work credits the source/coupled holder. A prescribed field amplitude or profile is not itself an energy grant.

The Phase 2 test solver demonstrates an exact discrete-energy settlement for a finite profile injection. It does not yet implement a general continuous source forcing law. Phase 3B must derive and test the discrete source update so its signed work equals the actual discrete field-energy change. Do not equate `P_h g` or the sun's incident `q_s` directly with field amplitude. The mapping from physical incident power to the field forcing term must preserve the Phase 1 work identity.

For a prescribed-power source independent of instantaneous field phase, Phase 1 explicitly requires a derived active-source law with a matching energy budget. Do not use an arbitrary forcing amplitude to manufacture the desired power.

### 4.4 Source-specific physical measures

**Sun:** Use the approved retarded-time geometry and incident density:
`q_s(p,t) = P_s(t_emit) h_s/(4π d³)`,
with `t - t_emit = d/c`. The integral over the infinite plane is exactly `P_s/2`; the remaining half stays in the external radiation-accounting domain. Source debit, planar interception, non-intercepted external accounting, and field work must be reconciled without double counting.

**Hotspot:** Use a compact-support normalized radial profile `g(r)), with `∫g dA = 1`, and `q_h = P_h(t)g(r)`. The total integrated power is `P_h(t)`. Its integral must be independent of discretization and recipients. If the cosine profile is retained, its normalization and exact radial integral must be stated and tested; spatial discretization/quadrature must report convergence and error.

No source power or geometry parameter is calibrated by this audit.

## 5. Interface gap matrix

| Requirement | Existing support | Gap / action |
|---|---|---|
| Finite-budget exact profile injection | Isolated Phase 2 solver | Mathematical reference only; no physical source identity or general forcing API |
| Stable source identity/provenance | Partial source enums in legacy ledger | Add source-instance identity and explicit provenance contract |
| One authoritative finite budget | Test-only source budget; legacy per-interval allowance | Define lifetime holder, replenishment, and shared debit semantics |
| Deterministic logical schedule | Existing phase/tick utilities and legacy source samples | Define source activation/emission events independently of legacy orbit/cache |
| External sun geometry | Phase 1 equations; legacy finite-world sampler | Implement retarded-time isotropic source-to-plane calculation; retire legacy orbit/rate later |
| Hotspot support and density | Legacy radial profile; checked polygon support area | Normalize radial profile and integrate physical density; avoid per-material allocation |
| Physical source measures | Checked polygon-union L² for polygons | Add world-shape transform and exact/validated circle support treatment as required |
| Conservative field injection | Phase 2 exact discrete profile settlement | Derive discrete continuous-forcing/source-work update; prove equality to field-energy change |
| Multi-source accounting | Phase 2 multiple mathematical pulse sources | Verify distinct source budgets, deterministic ordering, superposition, and no duplicate debit |
| Production integration | None in Phase 2 | Remains gated; no changes in this slice |

## 6. Recommended implementation boundary

The smallest safe source interface should have three conceptually distinct pieces:

1. **Source descriptor (immutable):** identity, provenance/class, physical geometry, deterministic schedule, frequency/phase, and dimensional power law.
2. **Source energy account (mutable):** the one authoritative finite reserve or explicitly accounted replenishment source, with signed transfer history and atomic debit/credit.
3. **Field coupling operation:** evaluates the source at a logical time, computes the physically defined spatial forcing over its support, applies the discrete field update, derives actual signed field work from the discrete field-energy change, and atomically settles the matching source account.

Keep source geometry evaluation separate from energy ownership and from numerical grid deposition. That separation makes it possible to test physical integrals independently of solver discretization and makes it impossible for grid-cell or receiver iteration to create new budgets.

This is a design recommendation, not an approved API signature or permission to begin implementation. The next proposal should compare this boundary with the actual Phase 2 solver types and specify the minimum changes needed without reopening its completed numerical validation.

## 7. Phase 3B.1 acceptance and open decisions

### Acceptance for this audit

- Existing source implementations and solver interfaces have been traced and classified.
- The approved Phase 1 source physics and units are restated without changing them.
- Existing code reuse and non-reuse boundaries are explicit.
- Remaining interface gaps and source-specific geometry requirements are identified.
- No production behavior, source settlement, solver, chemistry, movement, or organism code was changed.

### Decisions still required before 3B.2 implementation

1. **Reference solver consumption boundary:** PR #410 is closed, isolated, and unmerged. Decide in the 3B implementation proposal how the validated solver code/API is consumed without reopening Phase 2 or silently importing its branch as production. This audit does not merge PR #410.
2. **Discrete continuous-forcing law:** Specify how the sun/hotspot physical power densities map into the solver's discrete field update while preserving exact signed work. The Phase 2 profile injection is not yet this general law.
3. **Source-account ownership/replenishment:** Define whether initial sun/hotspot reserves are finite closed budgets or are funded by explicit external replenishment transactions. No values are selected here.
4. **Hotspot radial normalization:** State the exact normalization of the compact-support cosine profile and its analytic integral; define how the discrete grid integral converges to it.
5. **Hotspot support geometry:** Choose analytic circle integration for this radial profile or propose a separate exact circle-measure geometry slice. Do not polygonize without explicit approval.
6. **Source timing and parameter set:** Use the approved 24,000-tick day/orbit, 440 sun carrier cycles/day, and 880 hotspot cycles/day. Orbit radius/height, propagation speed, source power/budget, hotspot radius, and field normalization remain open calibrated parameters; do not infer them from legacy rates or wall-clock execution.

### Not part of this audit's acceptance

- Receiver perimeter coupling, occlusion integration, material absorption/reflection/transmission, heat settlement, or material-origin oscillators.
- Live sun/hotspot migration or retirement of the old paths.
- Any production ledger schema change.
- Performance optimization or GUI work.

## 8. Recommended next step

Prepare a focused **Phase 3B.2 implementation proposal** around the three-part source descriptor / source-energy account / field-coupling boundary. Resolve the solver-consumption boundary and discrete continuous-forcing work identity first. Then implement a controlled finite-budget source test and normalized hotspot injection in isolation. Keep the live sun, live hotspot, receivers, chemistry, movement, and production ledger unchanged.

The 3B.2 proposal must include the exact source API, discrete work equation, failure/rollback behavior, physical-area integration method, test matrix, and explicit non-goals before implementation begins.
