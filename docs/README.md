# Documentation

This directory contains the technical documentation for EvoSim. The root [README](../README.md) is the only general project and getting-started guide.

## Architecture and design

- [Repository organization](architecture/repository-organization.md) — repository boundaries and ownership.
- [Large-module boundaries](architecture/large-module-boundaries.md) — audited Rust module responsibilities.
- [Construction runtime boundaries](architecture/construction-runtime-boundaries.md) — construction subsystem responsibilities.
- [Resonance-field architecture](design/resonance-field-architecture.md) — high-level field model.
- [Resonance-field implementation roadmap](design/resonance-field-implementation-roadmap.md) — controlling detailed roadmap; read before any resonance-field work.
- [Material and chemistry model](material-chemistry-model-spec.md) — material descriptors and intended chemical interaction behavior.
- [Chemistry unification contract](CHEMISTRY_UNIFICATION_CONTRACT.md) — chemistry, ownership, and energy invariants.

## Chemistry and experiments

- [Chemistry audit and unification plan](chemistry-audit-unification-plan.md) — extensive historical and technical audit record; use the current README and source to establish present status.
- [Current chemistry handoff](chemistry-audit-current-handoff.md) — dated checkpoint; verify all PR and CI status against GitHub before relying on it.
- [Chemistry organization analysis](chemistry-organization-analysis.md) — analysis of chemical organization.
- [Reaction energy semantics](reaction-energy-semantics-plan.md) — energy-settlement design.
- [Experiment instructions](../experiments/README.md) — how to request and interpret environmental chemistry runs.

## Audits and implementation investigations

- `audits/` — focused subsystem and optimization audits.
- [Local viewer implementation audit](LOCAL_VIEWER_IMPLEMENTATION_AUDIT.md) — viewer and runner architecture audit.
- [Repository stabilization audit](repository-stabilization-audit.md) — dated repository audit; check its current-state section first.
- [Main organization audit](architecture/main-organization-audit-2026-10-22.md) — previous repository inventory.
- [Bob geometry audits](bob-geometry-storage-audit.md), [integration audit](bob-geometry-storage-integration-audit.md), and [regeneration audit](bob-regeneration-pipeline-audit.md).

## Plans and phase records

- `plans/` — subsystem implementation plans.
- `pr-plans/` — historical or scoped pull-request implementation plans. These are not automatically active work.
- Phase records: [Phase 3](PHASE3_AUTHORITY_AUDIT.md), [Phase 4](PHASE4_PLAN.md), [Phase 5](PHASE5_PLAN.md), and [Phase 6](PHASE6_PLAN.md).

## Project operating guidance

- [Agent protocol](AGENT_PROTOCOL.md) — repository-specific constraints for automated coding agents.
- [Fluid coalescence transaction](fluid-coalescence-transaction.md) — transaction and restoration notes.
- [PR audit consolidation](pr-audit-consolidation-2026-10-13.md) — historical cross-PR audit; validate current status independently.

## Historical archive

- [README handoff archive](archive/README_HANDOFF_ARCHIVE_2026-10-22.md) — preserved dated development handoffs. It is historical evidence, not the current project guide.

## Reading rules

1. Start with the root README for setup and the current high-level project map.
2. Prefer source code and exact-head CI for claims about implemented behavior.
3. Treat specifications and roadmaps as design intent, not proof of implementation.
4. Treat dated audits, handoffs, and PR plans as historical unless independently revalidated.
5. If two documents conflict, follow the applicable current design authority and explicit user-approved decisions; do not silently revive older assumptions.
