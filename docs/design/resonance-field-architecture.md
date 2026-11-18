# Resonance Field Architecture — Future Design

**Status:** Architecture direction agreed; implementation deferred until the current movement-optimization work is complete.

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
3. Propagation attenuates continuously with distance under the approved law:
   - Amplitude: `A(d) = A0 / (1 + d)`
   - Corresponding signal energy: `E(d) = E0 / (1 + d)^2`
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

## Future implementation stages

1. **Architecture and interfaces:** Define field state, source registration, receiver sampling, geometry snapshots/revisions, and energy-transfer contracts. No change to physical laws.
2. **Reference prototype:** Evaluate the existing resonance physics through the independent layer and compare received spectra, forces, movement, and energy accounting against the current implementation.
3. **Incremental computation:** Add caching, invalidation, spatial acceleration, and adaptive representation while preserving the reference results.
4. **Integration:** Replace the current coupled calculation only after equivalence, determinism, energy conservation, and measured performance improvements are demonstrated.

## Explicitly out of scope for PR #377

PR #377 remains focused on optimizing the existing resonance movement calculation. It must not implement this independent field architecture, introduce a second resonance-energy store, or change resonance propagation physics as a side effect of optimization.

The source-geometry foundation is tracked separately from PR #377. The future field architecture can reuse that work after it has been reviewed and integrated appropriately.
