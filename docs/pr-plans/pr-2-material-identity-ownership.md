# Enforce Resource Identity and Integer Material Inventory Invariants

> **Implementation in progress.** PR #207 is now authorized to make focused production and acceptance-test changes after the audit of merged `main`.

## Summary

Enforce canonical resource identity, indivisible integer quantities, and independently auditable material inventories across environmental stock, storage, and transaction transfers. Do not redo the parent aggregate graph/construction/division ownership migration tracked by PR #212.

## Outline

- [x] Audit remaining material representations and mutation paths against current main.
- [x] Confirm the shared `Material` validator rejects fractional per-resource quantities, unknown names, and duplicate catalog identities.
- [x] Extend the independent inventory oracle to reject ambiguous catalog identities.
- [x] Make logical and physical field deposits validate catalog identity, report rejection truthfully, and preserve state on invalid payloads or indices.
- [ ] Extend independent per-resource inventory and transaction rollback coverage for remaining owner paths.

## Acceptance criteria

- [ ] Every material unit has one authoritative physical owner.
- [ ] Transfers, storage, and consumption reconcile without duplication or disappearance.
- [ ] Rejected transactions preserve material state and inventory totals.
- [ ] Independent checks detect discrepancies without relying solely on implementation accounting helpers.

## Guardrails

- Start with one focused audit of the current merged `main` before implementation.
- Use PR #205 merge commit `10521705d8f921ceb071b17240f1b7e069f52f1a` as the forward baseline; compare named test failure identities, not only totals.
- Keep each command and test run within five minutes; a timeout is incomplete evidence.
- Keep this PR limited to its primary invariant. Do not fold in unrelated cleanup.
- PR #204 is the roadmap/handoff; PR #196 constructor/Bob work is out of scope.


## Cross-PR boundary and dependency clarification (2026-10-26)

This phase owns **material identity and ownership**, not reaction scheduling or energy policy. Its inventory must cover environmental stock, cell/organism storage, physical constituents, and temporary transaction escrow exactly once. It must not implement a second environmental material owner or reaction engine.

**Dependency:** use the PR #205 forward baseline. This is a prerequisite for the shared chemistry runtime and transactional reaction realization; it does not by itself make free environmental materials chemically active. Keep PR #206 movement proof and PR #212 aggregate-ownership migration separate.


## Current-main audit and first implementation slice (2026-10-26)

The post-#212 audit found that the independent test oracle in `src/tests/simulation_tests.rs` already walks environmental aggregates, realized environmental objects, organism structure/storage, aggregate-owned construction reservations, active transformation escrow, and decomposition carriers by resource name. It rejects unknown names and non-integer counts without reading the energy/material production ledger.

The first confirmed mutation-path defect is in `ActiveMaterialField::deposit`: its logical-material branch returned success even when `deposit_at_index` silently rejected an empty or invalid payload. The indexed deposit also lacked a bounds check and could panic for an invalid index. The first implementation slices make deposit APIs require the authoritative catalog, reject unknown/ambiguous identities for both logical stock and realized physical objects, report rejection truthfully, and preserve state on invalid payloads or indices. Focused regressions cover these rejection paths.

Remaining audit focus: confirm every transfer/escrow owner is covered by independent per-resource inventory tests, audit consumption and aggregation paths for silent loss or identity drift, and compare exact failure identities against the post-#212 baseline. This first slice does not claim the whole PR acceptance gate is complete.
