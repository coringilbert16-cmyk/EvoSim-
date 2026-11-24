# Resonance Field Architecture — Future Design

**Status:** Architecture direction approved; implementation not yet complete. The controlling, phased implementation plan is [Resonance Field — Formal Implementation Roadmap](resonance-field-implementation-roadmap.md). Read that roadmap before every resonance-field task.

## Purpose

Define a future architecture in which resonance exists as an independently calculated field alongside the physical environment. The physical and resonance layers share a coordinate space and exchange information, but they do not repeatedly reconstruct and calculate one another's complete state as a single coupled geometry system.

The spacetime analogy describes the separation of responsibilities: physical materials occupy and change positions in the environment, while resonance is represented throughout that same coordinate space and responds to physical sources and boundaries.

This document records architectural intent. It does not authorize implementation in PR #377 or change current simulation behavior.

## Layer responsibilities

### Physical layer

The physical layer remains authoritative for:

- Materials, atoms, molecules, constituents, shapes, and positions.
- Bonds, structural relationships, collisions, and movement.
- Material creation, destruction, transformation, and displacement.
- Physical geometry, including exposed boundaries and occlusion geometry.
- Physical properties required to define resonance sources and receiver responses.

### Resonance layer

The resonance layer is independently responsible for:

- Resonance sources and their existing energy provenance.
- Frequency, harmonics, phase, and resonance signatures.
- Propagation, attenuation, and interference throughout the shared coordinate space.
- The resonance state at queryable positions and physical boundaries.
- Accounting for energy that remains in propagation, is received, or is dissipated as heat.

The resonance layer is not a second source of energy. It represents and transports the same resonance energy already defined by the simulation's physical and energy model.

## Coupling contract

The layers exchange information through explicit interfaces.

**Physical to resonance:**
- Source identity and physical position.
- Source shape and exposed physical perimeter.
- Source frequency, harmonics, phase, and authoritative available energy.
- Geometry or material changes that can affect propagation or occlusion.
- Receiver positions, shapes, and response properties.

**Resonance to physical:**
- Frequency-resolved resonance received at requested positions or exposed boundaries.
- Resulting interaction inputs for movement and material response.
- Energy transferred to receivers.
- Propagation energy dissipated as environmental heat, accounted exactly once.

The physical layer does not need to rebuild or own the resonance field. The resonance layer does not own or mutate physical geometry. Changes cross the interface as events, revisions, or snapshots with explicit validity.

## Physical invariants to preserve

The independent representation must preserve the established model:

1. Resonance originates from the actual exposed perimeter of the complete physical material, not its centroid or an arbitrary point.
2. Source energy is allocated linearly across the complete exposed source perimeter.
3. Propagation uses a physically specified finite-speed wave model. The existing distance law (amplitude `A(d) = A0 / (1 + d)` and signal energy `E(d) = E0 / (1 + d)^2`) is legacy/reference behavior; it must not be assumed to be the governing wave equation. The roadmap requires attenuation, geometric spreading, and dissipative loss to be distinguished and derived.
4. Propagation loss is accounted for as environmental heat exactly once, not once per receiver.
5. Occlusion remains physically meaningful and is derived from physical geometry.
6. No arbitrary interaction radius, fixed distance cap, or numerical-epsilon cutoff may silently remove a physically meaningful interaction.
7. The physical effective-zero rule must be defined by physical meaningfulness, not floating-point underflow or an optimization-selected threshold.
8. The resonance layer must not create duplicate energy, duplicate reservoirs, or double-charge shared energy.
9. Deterministic behavior and the existing energy ledger remain authoritative.

## Computational representation

The resonance field is a continuous conceptual field in the same coordinate space as the physical environment. Its computational representation is an implementation choice and need not be a fixed uniform grid.

Future implementations may investigate:

- Adaptive cells or field regions where resonance varies meaningfully.
- Spatial hierarchies for source and receiver queries.
- Incremental updates when sources, geometry, or propagation conditions change.
- Cached field contributions with explicit invalidation based on physical revisions.
- Reuse or aggregation of distant contributions only where the approximation is proven to preserve the required physical results and energy accounting.
- Exact evaluation at actual material boundaries when a physical receiver samples the field.

A field rebuilt globally every tick is not inherently an optimization. The intended computational advantage comes from avoiding repeated all-source/all-receiver pair calculations and updating only the field state affected by meaningful changes.

Any approximate or hierarchical method must be compared against a conservative exact reference and must not omit a physically meaningful interaction.

## Implementation roadmap

The controlling implementation sequence is maintained in [resonance-field-implementation-roadmap.md](resonance-field-implementation-roadmap.md). It defines the approved continuous, persistent, multi-frequency, finite-speed wave architecture; the physics decisions that must be resolved; energy and geometry invariants; the phased path from baseline audit through production migration; validation gates; and the performance target. The reference solver must demonstrate actual wave propagation, phase, finite travel time, and interference. A diffusion approximation or instantaneous redistribution is not acceptable. Correct physics takes priority; after profiling and optimization, performance should at least break even with the existing implementation.

## Explicitly out of scope for PR #377

PR #377 remains focused on optimizing the existing resonance movement calculation. It must not implement this independent field architecture, introduce a second resonance-energy store, or change resonance propagation physics as a side effect of optimization.

The source-geometry foundation is tracked separately from PR #377. The future field architecture can reuse that work after it has been reviewed and integrated appropriately.


## Unified energy-system clarification

The continuous wave field is the selected spatial transport architecture, but it is not the complete energy ontology. The complete design must unify external source energy, propagating field energy, material internal energy, thermal energy, validated structural/chemical potential, mechanical work, and boundary escape under one conservation authority.

The existing ledger is reusable implementation infrastructure—not binding physics. Preserve its proven validation, inventory reconciliation, staged transaction, and rollback machinery where correct. Redesign its categories, holder ownership, and reaction-funding semantics where they do not fit the unified model. Likewise, retain source configuration, material properties, frequency/phase utilities, transaction patterns, and PR #378 geometry where their contracts remain valid. Do not preserve global chemistry pools, frequency-indexed reservoirs as travelling-field state, uniform per-constituent grants, independent source settlement paths, or instantaneous centroid interactions as physical authorities.

Material resonance is a response/coupling property unless a powered source or conservative coupled degree of freedom is explicitly defined. Activation influence is not spendable energy. Organism-specific reception and energy budgets remain deferred until an organism exists; environmental physics must not be shaped around biological budgeting.

This clarification supersedes any earlier statement that the existing energy ledger's semantics or current reservoir model are inherently authoritative. The controlling reuse map and phase gates are maintained in the implementation roadmap.


## Current ambient-field decision

The active energy model is a persistent, spatially uniform ambient background held by the resonance field. Its 24,000-tick daily schedule, 440-cycle carrier, 10 → 1 → 10 cycle-averaged energy-density envelope, and signed generation/withdrawal accounting are implemented in the live simulation (PR #439). The environment area is inventory bookkeeping only, not a propagation boundary. There is no Sun object, external injector, hotspot injector, powered material oscillator, or additional buffer/reservoir in the active model.

The validated finite-speed spatial wave solver remains the propagation foundation. The immediate implementation task is to integrate that solver with the live ambient background and one accounting authority (Phase 3C), then implement passive material interaction through actual exposed physical geometry (Phase 3D). The earlier three-source coupling derivation is historical and superseded. See the [controlling implementation roadmap](resonance-field-implementation-roadmap.md).
