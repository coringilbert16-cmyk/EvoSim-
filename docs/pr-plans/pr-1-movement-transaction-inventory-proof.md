# Prove Movement Transaction Rollback and Inventory Conservation

> **Implementation in progress.** This PR currently strengthens regression coverage only. The source audit found that movement already preflights energy settlement and resolves the push plan before applying it; no new transaction framework is planned.

## Summary

Prove that movement rejection preserves authoritative world state, energy, material inventory, and spatial indexes. Keep the existing preflight/push-plan architecture unless tests demonstrate a real production defect.

## Implementation

- [x] Expand invalid-ledger rejection coverage to include another organism and realized field material.
- [x] Snapshot the moving organism, pushed organism, complete field cells/revision, spatial-index cells/membership, ledger, active movement counters, and independent integer-material inventory.
- [x] Add a blocked push-chain case that rejects after successful energy preflight and proves no physical/energy/index side effects.
- [x] Add a successful control that proves the mover, pushed organism, and realized field material move once, indexing updates, material inventory is conserved, and energy/heat reconcile.
- [x] No production defect was exposed by the three movement transaction regressions; existing preflight/push-plan behavior passed.

## Acceptance criteria

- [x] Rejected movement preserves geometry, physical ownership, IDs, storage, revisions, indexes, movement counters, energy, ledger values, and independent inventories (excluding the expected failure diagnostic).
- [x] Failed push chains discard preflight settlement without side effects.
- [x] Successful movement updates each owner once and reconciles the expected energy debit.
- [ ] Compare failure identities against the PR #205 forward baseline.

## Verification

- [x] Targeted movement tests pass on the final PR head (GitHub Actions run 9631).
- [x] Formatting and source-size checks pass (GitHub Actions run 9631).
- [ ] Compare test failure identities with PR #205 merge baseline `10521705d8f921ceb071b17240f1b7e069f52f1a`; do not compare only failure totals.
- [ ] Confirm no production changes are needed, or make only a focused fix proven necessary by a failing regression.

## Guardrails

- Keep this PR limited to movement transaction rollback/inventory proof.
- Keep each command/test run within five minutes; a timeout is incomplete evidence.
- PR #204 remains the roadmap; PR #196 constructor/Bob work is out of scope.
- PR #212 remains the separate aggregate ownership/construction/division migration.
