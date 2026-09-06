# Quantity → Physical Realization Audit

This audit records the approved direction that bulk environmental quantities may intentionally become physical structures through an explicit realization process. Quantity remains distinct from physical structure; realization is the rule that maps one into the other.

## Audit 1 — Architecture

- `ActiveMaterialField` owns aggregated ecological stock (`FieldCell.materials`).
- `PhysicalEnvironment` owns realized physical material instances.
- `Material` owns composition + internal structure; no independent bonding flag.
- `PhysicalMaterialInstance` owns a material plus constituent placements/forms derived from the immutable resource catalog.
- Exact contact, interface length, and permeability already exist as separate physical layers.
- Therefore quantity → physical must be an explicit transformation, not an implicit reinterpretation of field stock.
- The transformation must be conservative and atomic: validate the resulting physical object before withdrawing source material.
- A realization rule must not invent composition, structure, or geometry from an amount alone. It must explicitly specify how those are derived.

## Audit 2 — Existing code

The current implementation already has a low-level transaction that accepts a source field cell, material, explicit placements, and catalog, then validates and transfers the realized amount. This is a useful primitive but is not yet the environmental process that decides *when* and *what* to realize.

The spatial index is a derived broad-phase cache over `PhysicalEnvironment`; it must not become an authority for physical identity or contact.

No current interaction-capacity trait should be introduced to support realization. `processing_efficiency` has an existing biological meaning and must remain separate.

## Plan

1. Keep bulk field stock and physical objects as separate authorities.
2. Keep the existing conservative realization transaction as the atomic conversion primitive.
3. Add an explicit realization-rule layer that selects eligible bulk material and produces a physical realization specification (source cell/material, amount, structure, placements) without mutating state.
4. Execute the specification through the existing transaction so failed geometry validation cannot consume material.
5. Keep spatial indexing derived from realized physical objects and rebuild after physical mutations.
6. Add tests for deterministic rule output, conservation, insufficient quantity, invalid geometry, and coexistence of distinct physical realizations.
7. Do not connect ACQUIRE yet; realization must become reliable first.

## Audit 3 — Plan against code

The plan fits the current architecture because it adds policy above the existing transaction instead of replacing the transaction or conflating field storage with geometry. The only new authority is the realization rule itself. Its output must be data, not a second physical-material representation.

The realization rule should be deliberately minimal in this pass: a deterministic threshold/packing rule that is explicitly supplied by the caller, rather than silently choosing arbitrary environmental chemistry. This establishes the lifecycle and conservation contract without prematurely deciding EvoSim's final geochemical formation law.

Implementation therefore proceeds with a reusable realization specification + rule API, leaving the actual environmental scheduling/trigger policy for the next layer.
