# Documentation

The root [README](../README.md) is the project entry point and getting-started guide. This index is the map to technical material; documents are grouped by purpose rather than accumulated in one directory.

## Architecture and design

- [Repository organization](architecture/repository-organization.md) — source boundaries, ownership, and integration policy.
- [Large-module boundaries](architecture/large-module-boundaries.md) — audited Rust module responsibilities.
- [Construction runtime boundaries](architecture/construction-runtime-boundaries.md) — construction subsystem responsibilities.
- [Resonance-field architecture](design/resonance-field-architecture.md) — high-level field model.
- [Resonance-field implementation roadmap](design/resonance-field-implementation-roadmap.md) — controlling detailed roadmap; read before any resonance-field work.
- [Phase 3B.1 source/interface audit](audits/resonance-field-phase-3b1-source-interface-audit.md) — verified source, budget, geometry, and reference-solver interface gaps; audit only, no production changes.
- [Phase 3B.2 source interface and injection proposal](plans/PHASE3B2_SOURCE_INTERFACE_AND_INJECTION_PROPOSAL.md) — source ownership, discrete work settlement, hotspot integration, and separate sun/receiver gates; 3B.2a and 3B.2b implementation slices are merged.
- [Phase 3B.2c continuous forcing derivation](plans/PHASE3B2C_CONTINUOUS_FORCING_DERIVATION.md) — exact discrete leapfrog work identity and comparison of prescribed forcing with reciprocal source-degree-of-freedom coupling; active mathematical investigation, not an implementation authorization.
- [Phase 3B.3 physical source-to-field coupling](plans/PHASE3B3_PHYSICAL_SOURCE_COUPLING.md) — current checkpoint after Candidate A/B validation; derives separate coupling and energy-ownership requirements for the initial uniform environmental input, localized hotspot, and powered material oscillators; analytical design only, no production integration authorization.
- [Phase 3B.3 uniform environmental source comparison](plans/PHASE3B3_UNIFORM_ENVIRONMENTAL_SOURCE_COMPARISON.md) — controlled isolated comparison of prescribed forcing and reciprocal oscillator models under one environmental energy budget; first gate is deriving the reciprocal model's conservative environmental-supply port.
- [Unified energy and continuous wave-field specification](design/unified-energy-physics-specification.md) — active Phase 1 specification and approved modeling defaults for the scalar continuous wave reference field, infinite flat world, isotropic external sun on an above-plane perpendicular orbit, conservative source coupling, powered material oscillators plus passive response, finite-area hotspot, project-native units, and uniform-grid reference solver; Phase 1 specification complete; Phase 2 is the independent empty-space reference solver, with production integration still gated.

## Chemistry and physical models

- [Material and chemistry model](chemistry/material-chemistry-model-spec.md) — material descriptors and intended chemical interaction behavior.
- [Chemistry unification contract](chemistry/CHEMISTRY_UNIFICATION_CONTRACT.md) — chemistry, ownership, and energy invariants.
- [Chemistry audit and unification plan](chemistry/chemistry-audit-unification-plan.md) — historical and technical audit register; use current source and CI for implementation status.
- [Current chemistry handoff](chemistry/chemistry-audit-current-handoff.md) — dated checkpoint; verify PR and CI status against GitHub.
- [Chemistry organization analysis](chemistry/chemistry-organization-analysis.md) — analysis of chemical organization.
- [Reaction energy semantics](chemistry/reaction-energy-semantics-plan.md) — energy-settlement design.
- [Fluid coalescence transaction](chemistry/fluid-coalescence-transaction.md) — transaction and restoration notes.
- [Experiment instructions](../experiments/README.md) — how to request and interpret environmental chemistry runs.
- [Experiment records](../experiments/records/README.md) — registry of completed experiments, results, and planned organized-chemistry research.

## Audits and implementation investigations

- [Local viewer implementation audit](audits/LOCAL_VIEWER_IMPLEMENTATION_AUDIT.md) — viewer and runner architecture.
- [Repository stabilization audit](audits/repository-stabilization-audit.md) — dated repository audit; validate its current-state section against the live repository.
- [Main organization audit](architecture/main-organization-audit-2026-10-22.md) — previous repository inventory.
- [Bob geometry storage audit](audits/bob-geometry-storage-audit.md), [integration audit](audits/bob-geometry-storage-integration-audit.md), and [regeneration audit](audits/bob-regeneration-pipeline-audit.md).
- [Constructor/library dependency audit](audits/constructor-library-dependency-audit.md).
- [Phase 3 authority audit](audits/PHASE3_AUTHORITY_AUDIT.md).
- [PR audit consolidation](audits/pr-audit-consolidation-2026-10-13.md) — historical cross-PR record.
- Additional focused audits live in `audits/`.

## Plans and phase records

- [Phase 3A.3 visibility-transition partitioning](plans/PHASE3A3_VISIBILITY_TRANSITIONS.md) — current audit, exact event taxonomy, implementation risks, and acceptance criteria.
- [Phase 3A.4 physical area and boundary measures](plans/PHASE3A4_PHYSICAL_MEASURES.md) — current geometry audit and proposed distinct L/L² measures, polygon-union topology, and analytic acceptance matrix; proposal only, implementation gated on review.
- [Phase 3A.4d source-footprint geometry](plans/PHASE3A4D_SOURCE_FOOTPRINT.md) — checked physical 2D support-area adapter; geometry only, no source activation or field coupling.
- [Phase 4](plans/PHASE4_PLAN.md), [Phase 5](plans/PHASE5_PLAN.md), and [Phase 6](plans/PHASE6_PLAN.md).
- [Phase 6 blueprint specification](plans/PHASE6_BLUEPRINT_SPEC.md).
- [World-tone material response](plans/world-tone-material-response.md).
- [Scoped PR plans](pr-plans/) — historical or scoped implementation plans; presence here does not mean work is currently approved or active.

## Project operating guidance

- [Agent protocol](operations/AGENT_PROTOCOL.md) — repository-specific constraints for automated coding agents.

## Historical archive

- [README handoff archive](archive/README_HANDOFF_ARCHIVE_2026-10-22.md) — preserved dated development handoffs. It is historical evidence, not the current project guide.

## Reading rules

1. Start with the root README for setup and the current high-level project map.
2. Prefer source code and exact-head CI for claims about implemented behavior.
3. Treat specifications and roadmaps as design intent, not proof of implementation.
4. Treat dated audits, handoffs, and PR plans as historical unless independently revalidated.
5. If documents conflict, follow the applicable current design authority and explicit user-approved decisions; do not silently revive older assumptions.
