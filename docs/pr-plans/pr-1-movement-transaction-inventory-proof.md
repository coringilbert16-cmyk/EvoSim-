# Prove Movement Transaction Rollback and Inventory Conservation

> **Planning scaffold only.** This file records the proposed scope and acceptance gate. No implementation changes or tests are included in this setup PR. Do not start implementation until explicitly authorized.

## Summary

Strengthen movement transaction tests so rejected operations demonstrably preserve authoritative world state, energy, material inventory, and spatial indexes. Reuse existing preflight and rollback behavior; do not introduce a new transaction framework.

## Outline

- [ ] Extend rejected-movement settlement coverage to verify complete state preservation.
- [ ] Cover a blocked push chain after successful energy preflight.
- [ ] Strengthen a successful push-chain control for ownership, indexing, and energy reconciliation.
- [ ] Reuse existing movement and inventory helpers; avoid redundant architecture.

## Acceptance criteria

- [ ] Rejected movement preserves geometry, physical ownership, IDs, storage, revisions, indexes, movement counters, energy, ledger values, and independent inventories.
- [ ] Failed push chains discard preflight settlement without side effects.
- [ ] Successful movement updates each owner once and reconciles the expected energy debit.
- [ ] Compare failure identities against the PR #205 forward baseline.

## Guardrails

- Start with one focused audit of the current merged `main` before implementation.
- Use PR #205 merge commit `10521705d8f921ceb071b17240f1b7e069f52f1a` as the forward baseline; compare named test failure identities, not only totals.
- Keep each command and test run within five minutes; a timeout is incomplete evidence.
- Keep this PR limited to its primary invariant. Do not fold in unrelated cleanup.
- PR #204 is the roadmap/handoff; PR #196 constructor/Bob work is out of scope.
