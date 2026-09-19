# Phase 6 — Growth & Development

## Status

**P6.0 COMPLETE — P6.1/P6.2 IMPLEMENTATION IN PROGRESS**

CI validation is being used as the completion gate for the README-authorized architecture.

The governing pipeline is:

> Genome → Developmental Field Blueprint → Construction/Development Solver → Physical Structure

The physical graph remains authoritative for realized structure.

## P6.0 — Authority audit — COMPLETE

- Confirmed the repository still contains the legacy discrete OrganismArchitecture / StructuralBlueprint construction path.
- Confirmed the existing construction runtime contains reusable physical candidate generation, backtracking, COMBINE admission, and physical validation machinery.
- Confirmed adulthood currently uses the existing >=90% realization threshold; P6 will retain that threshold only when the compared quantity is derived from the developmental blueprint rather than a legacy exact body-plan authority.
- Confirmed no new viability, death, maturation, or lifecycle authority is required.

## P6.1 — Developmental-field blueprint authority — IN PROGRESS

### Approved rules implemented so far

- Added inherited `size_preference` as the developmental-size authority, normalized to [0, 1].
- Preferred developmental mass is derived from size preference using the approved logarithmic mapping.
- Size-preference mutation is bell-shaped around the parent's value and bounded to [0, 1].
- Preferred developmental mass is soft intent; actual mass remains derived from realized physical structure.
- The current `adult_mass()` API is retained only as an implementation-facing derived preferred-mass accessor; it is not an independent genome authority.
- The numerical mass bounds used by the current mapping are explicitly **experimental**.
- Removed the discrete size/count ladder and canonical juvenile-count authority.
- Preserved the confirmed-good juvenile seed only as a physically validated solver starting realization, not as a genome body-plan rule.
- Keep connectivity available as an optional field, inactive until validation demonstrates that material and density fields are insufficient for structural variation.
- Preserve discrete construction objects only as transient solver artifacts.

## P6.2 — Developmental solver — IN PROGRESS

The approved developmental equations are now documented and partially represented in code. Current numerical field widths, influence locations, candidate-score weights, and mass bounds are explicitly **experimental**.

A first transient field-driven construction candidate now exists. It derives material selection, structural density, and juvenile/adult spatial scale from developmental fields; the returned discrete blueprint is explicitly transient and is passed into the existing physical construction/COMBINE machinery. P6.2 now uses field-scored candidate growth from the confirmed-good physical seed. Remaining work is to route those transient candidates through the existing neighborhood/backtracking machinery rather than accepting a single greedy candidate.

## P6.3 — Growth integration

Route developmental field scoring into the actual ongoing COMBINE construction path. The current transient candidate solver is not yet sufficient.

Make growth an ongoing developmental process rather than a selectable action or one-time discrete target realization.

## P6.4 — Physical growth contracts

Verify preservation of existing realized material, physical construction of new structure, actual geometry and connections, COMBINE admission for new bonds, BREAK where reorganization requires it, and no rewriting of physical reality from blueprint intent.

## P6.5 — Development/adulthood audit

Audit DevelopmentStage against adult-blueprint realization. No age, reproductive-readiness, arbitrary energy, or independent maturation authority may be introduced.

## P6.6 — Contract validation

Run full tests, architecture checks, formatting, strict Clippy, and final authority audit.

## Non-goals

P6 does not redesign genome identity/life definition, genome-cavity qualification, chemistry, COMBINE/BREAK chemistry, energy accounting, maintenance/stress, movement/collision/pushing, reproduction, death, decomposition, evolution, or environmental material physics.


P6 README authority audit gate: CI must pass and the implementation must be re-audited against README authority before completion.
