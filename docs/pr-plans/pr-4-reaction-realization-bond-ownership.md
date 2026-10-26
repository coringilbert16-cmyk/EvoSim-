# Make Reaction Realization and Bond Ownership Transactional

> **Planning scaffold only.** This file records the proposed scope and acceptance gate. No implementation changes or tests are included in this setup PR. Do not start implementation until explicitly authorized.

## Summary

Ensure chemistry reactions change authoritative physical material and bonds exactly once, with accounting committed consistently alongside physical realization.

## Outline

- [ ] Map reaction planning to material realization and bond mutation.
- [ ] Identify duplicate ownership or non-atomic reaction paths.
- [ ] Stage and validate reaction outcomes before committing changes.
- [ ] Add success, rejection, and rollback coverage for representative reactions.

## Acceptance criteria

- [ ] Reaction bookkeeping corresponds to actual physical changes.
- [ ] Bonds have unambiguous ownership and valid endpoints.
- [ ] Rejected reactions preserve material, bonds, energy, and inventories.
- [ ] Successful reactions reconcile material and energy consequences.

## Guardrails

- Start with one focused audit of the current merged `main` before implementation.
- Use PR #205 merge commit `10521705d8f921ceb071b17240f1b7e069f52f1a` as the forward baseline; compare named test failure identities, not only totals.
- Keep each command and test run within five minutes; a timeout is incomplete evidence.
- Keep this PR limited to its primary invariant. Do not fold in unrelated cleanup.
- PR #204 is the roadmap/handoff; PR #196 constructor/Bob work is out of scope.


## Cross-PR boundary and dependency clarification (2026-10-26)

This phase owns **atomic physical realization and settlement of a selected reaction**: validate live contacts/endpoints, stage all material/bond changes, settle energy, then commit once or roll back completely. It does not own the world-wide candidate scheduler, reaction-progress lifetime, or environment-first UI/logging; those are separate planning scopes.

**Required shared-rule invariant:** the same reaction policy and settlement path must be usable for environmental material and organism/cell-held material. Do not create a special environmental reaction law. Depend on PR #207 material ownership, PR #208 energy accounting, the cache-version boundary, and the shared chemistry runtime/scheduler. Cached results never override live geometry or settlement.
