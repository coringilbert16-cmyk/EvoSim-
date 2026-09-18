# Phase 6 — Growth & Development

## Status

**P6.0 AUTHORITY AUDIT COMPLETE — IMPLEMENTATION NOT YET APPROVED**

Phase 6 begins the transition from the current discrete construction-target implementation to the approved developmental-field blueprint architecture.

The governing pipeline is:

> Genome → Developmental Field Blueprint → Construction/Development Solver → Physical Structure

The physical graph remains authoritative for what has actually been realized.

## P6.0 Authority Audit

### Confirmed existing authorities

- Genome owns inherited structural/developmental information.
- Physical structure owns realized organism structure.
- COMBINE remains the authority for admission of new physical bonds.
- BREAK remains available for reorganization and construction.
- Existing material restoration preserves already-realized physical material.
- Maintenance and survival are already established by Phase 5.
- Juvenile, adult, reproduction, death, and decomposition paths already exist but must not be expanded by inventing new lifecycle rules.

### Critical migration finding

The repository does **not yet implement the approved developmental-field blueprint**.

`Genome` currently owns `OrganismArchitecture`, whose `ArchitectureRegion` records explicit material, center coordinates, extents, density, region roles, relations, and target scale.

`OrganismArchitecture::construction_target()` then converts those regions into an explicit `StructuralBlueprint` containing exact material elements, exact placements, exact connection declarations, and an explicit construction anchor.

That is the legacy/discrete target representation. It conflicts with the approved blueprint direction because the approved blueprint expresses developmental tendencies rather than a diagram of exact material instances, exact coordinates, exact bonds, exact angles/rotations, exact silhouette, or exact topology.

### Current solver status

`construction_runtime.rs` already contains useful physical realization machinery: candidate placement generation, recursive/backtracking search over material constituents, external-neighbor constraints, COMBINE-based bond admission, trial structure/ledger/energy state, and physical contact validation.

This machinery should be retained and adapted rather than replaced wholesale.

### Current lifecycle status

`simulation.rs` currently constructs the initial organism from a developmental construction target, computes mature structural mass from a mature construction target, derives a growth fraction from realized structural mass divided by mature target mass, and changes `DevelopmentStage::Juvenile` to `Adult` at `ADULTHOOD_GROWTH_FRACTION = 0.90`.

This is an important unresolved authority issue for P6.

The approved blueprint design establishes an adult blueprint at 100% scale and describes growth as the process of moving physical realization toward that target. P6 must therefore audit and replace the current discrete-target/growth-fraction mechanism as required by the approved design. No new maturation threshold or alternative adulthood rule may be invented.

## P6 Non-Goals

P6 must not redesign genome identity/life definition, genome-cavity semantics, resource chemistry, COMBINE chemistry, BREAK chemistry, energy accounting, maintenance/stress, movement/collision/pushing, reproduction, death, decomposition, evolution, or environmental material physics.

## P6 Implementation Sequence

### P6.0 — Authority audit — COMPLETE

Repository and README audit performed.

### P6.1 — Developmental-field blueprint authority — NEXT

Define the code representation of the approved field blueprint without turning it into an exact body diagram.

### P6.2 — Developmental solver

Adapt the existing physical construction machinery so it samples/follows developmental fields while respecting already-realized physical structure and actual geometry.

### P6.3 — Growth integration

Make growth an ongoing developmental process rather than a selectable `GROW` action or a one-time discrete target realization.

### P6.4 — Physical growth contracts

Verify that growth preserves existing realized material, derives new structure through physical construction, respects actual geometry and connections, uses COMBINE for newly admitted bonds, can use BREAK where reorganization requires it, and never rewrites physical reality merely because blueprint intent differs.

### P6.5 — Development/adulthood audit

Audit the existing `DevelopmentStage` transitions against the approved adult-blueprint/growth semantics. Do not introduce a new maturation rule without explicit approval.

### P6.6 — Contract validation

Run full tests, architecture checks, formatting, and strict Clippy, then perform a final authority audit.

## Explicit P6 Design Constraint

The approved developmental-field blueprint is the only blueprint authority introduced by this phase.

Do not preserve the current explicit `StructuralBlueprint` as a second inherited body-plan authority merely for compatibility.

Where a discrete construction representation is still required internally, it must be a transient solver artifact derived from developmental intent, not inherited biological information.

No code should be changed merely to make an implementation choice where the approved design does not yet determine the behavior.
