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

- Added inherited size_preference genome trait normalized to [0, 1].
- Size-preference mutation is sampled from a bell-shaped distribution centered on the parent's inherited value, then bounded to [0, 1].
- Missing legacy size-preference data defaults to the center value rather than invalidating the genome.
- Removed fixed MAX_MEMORY_POINTS as the memory-capacity authority.
- Memory capacity is derived from the realized qualifying genome cavity's 2D enclosed interior area with square-root diminishing returns.
- Memory persistence is derived from the same cavity area with diminishing returns.
- Memories are cleared when the realized organism no longer has a qualifying cavity whose enclosed 2D area is strictly greater than the three-Carbon reference.
- Existing memory_strength remains the formation/reinforcement-strength parameter and is not repurposed as capacity or persistence.
- Transformation-driven memory reinforcement now uses the same cavity-derived capacity authority rather than a separate fixed capacity.

### Remaining P6.1 work

- Complete removal of remaining legacy architecture helpers from construction/lifecycle code.
- Define the minimum inherited information for material-composition and structural-density preference fields.
- Define how size preference maps to preferred developmental mass/scale without creating a hard size ladder.
- Keep connectivity available as an optional field, inactive until validation demonstrates that material and density fields are insufficient for structural variation.
- Preserve discrete construction objects only as transient solver artifacts.

## P6.2 — Developmental solver — IN PROGRESS

A first transient field-driven construction candidate now exists. It derives material selection, structural density, and juvenile/adult spatial scale from developmental fields; the returned discrete blueprint is explicitly transient and is passed into the existing physical construction/COMBINE machinery. Remaining P6.2 work is to replace the candidate ring heuristic with genuine candidate search/backtracking driven by field evaluation and already-realized neighborhood constraints.

## P6.3 — Growth integration

Make growth an ongoing developmental process rather than a selectable action or one-time discrete target realization.

## P6.4 — Physical growth contracts

Verify preservation of existing realized material, physical construction of new structure, actual geometry and connections, COMBINE admission for new bonds, BREAK where reorganization requires it, and no rewriting of physical reality from blueprint intent.

## P6.5 — Development/adulthood audit

Audit DevelopmentStage against adult-blueprint realization. No age, reproductive-readiness, arbitrary energy, or independent maturation authority may be introduced.

## P6.6 — Contract validation

Run full tests, architecture checks, formatting, strict Clippy, and final authority audit.

## Non-goals

P6 does not redesign genome identity/life definition, genome-cavity qualification, chemistry, COMBINE/BREAK chemistry, energy accounting, maintenance/stress, movement/collision/pushing, reproduction, death, decomposition, evolution, or environmental material physics.
