# Documentation index

This index is the entry point for repository documentation. It classifies documents by purpose without moving files or rewriting historical evidence. The repository-stabilization PR is deliberately using an index-first approach: establish ownership and discoverability, audit links, then move files only when the benefit is clear.

## Current repository and architecture

- [Repository organization plan](architecture/repository-organization.md) — source boundaries, migration rules, and ownership.
- [Repository stabilization audit](repository-stabilization-audit.md) — current-state correction at the top; older sections are historical audit evidence. Read the dated current-state correction first.
- [PR audit and consolidation (2026-10-13)](pr-audit-consolidation-2026-10-13.md) — historical cross-PR audit; verify every status against the current README before relying on it.
- [Local viewer / runner implementation audit](LOCAL_VIEWER_IMPLEMENTATION_AUDIT.md) — UI, runner, and integration audit.

## Physical geometry and structure

- [Phase 3 physical structure authority audit](PHASE3_AUTHORITY_AUDIT.md) — authority and structural ownership.
- [Fluid coalescence transaction](fluid-coalescence-transaction.md) — transaction and restoration behavior for fluid-to-fluid coalescence.
- [Phase 4 movement and physical interaction plan](PHASE4_PLAN.md) — planned movement and interaction mechanics.

## Chemistry and material handling

- Chemistry implementation and authority are documented in the source tree and current README; use the focused code/audit before making claims about conservation.
- [Fluid coalescence transaction](fluid-coalescence-transaction.md) also covers material-state changes that cross geometry and chemistry boundaries.

## Bob and persistent geometry knowledge

- [Bob geometry storage audit](bob-geometry-storage-audit.md) — earlier compact-storage proposal; historical context.
- [Bob geometry storage integration audit](bob-geometry-storage-integration-audit.md) — integration analysis; check its checkpoint date and current README before treating plans as active.
- [Bob regeneration pipeline audit](bob-regeneration-pipeline-audit.md) — regeneration and library workflow.
- [Constructor/library dependency audit](constructor-library-dependency-audit.md) — constructor-to-library dependencies. Constructor implementation work remains separate from repository stabilization.

## Organism lifecycle and development plans

- [Phase 5 single-cell maintenance and survival](PHASE5_PLAN.md) — planning document; verify completion status against the current README.
- [Phase 6 growth and development](PHASE6_PLAN.md) — development plan.
- [Phase 6.1 developmental field blueprint specification](PHASE6_BLUEPRINT_SPEC.md) — detailed blueprint specification.

## How to use this documentation

1. Start with the root [README](../README.md) for the current repository state and verified test/lint checkpoint.
2. Use this index to locate focused technical evidence.
3. Treat plans and audits as evidence of what was proposed or observed at their checkpoint, not proof that code currently implements it.
4. Before changing a subsystem, inspect the current code and the most recent relevant audit; update status and links after the change.
5. Historical handoffs remain in the root archive. Do not copy old status statements into current documentation without re-verification.

## Organization policy

- This index does not change source behavior or move existing documents.
- Future document moves should be small, audited for incoming relative links, and performed atomically with all link updates.
- Once a document has been moved, retain a clear canonical location and avoid duplicate copies that can drift.
