# Complete Aggregate Ownership and Transactional Construction and Division

> **Planning scaffold only.** This file records the proposed scope and acceptance gate. No implementation changes or tests are included in this setup PR. Do not start implementation until explicitly authorized.

## Summary

Finish the ownership migration so an organism’s physical aggregate has one authoritative owner throughout construction, attachment, operation, and division. Preserve the distinction between a parent-owned aggregate and independent organisms after a real split.

## Outline

- [ ] Audit current main for remaining duplicate ownership and lifecycle gaps.
- [ ] Verify construction and attachment preserve parent authority.
- [ ] Make division an explicit, transactional ownership boundary.
- [ ] Ensure stable constituent IDs and genome-core identity do not create duplicate physical graphs or energy holders.
- [ ] Test successful division and failure rollback end to end.

## Acceptance criteria

- [ ] Each physical component has one authoritative owner before and after division.
- [ ] Failed construction or division leaves ownership and inventories unchanged.
- [ ] Successful division produces distinct, internally consistent aggregates.
- [ ] Material and energy inventories reconcile across the transition.
- [ ] The 12-piece nitrogen seed remains a cavity fixture, not a viability requirement.

## Guardrails

- Start with one focused audit of the current merged `main` before implementation.
- Use PR #205 merge commit `10521705d8f921ceb071b17240f1b7e069f52f1a` as the forward baseline; compare named test failure identities, not only totals.
- Keep each command and test run within five minutes; a timeout is incomplete evidence.
- Keep this PR limited to its primary invariant. Do not fold in unrelated cleanup.
- PR #204 is the roadmap/handoff; PR #196 constructor/Bob work is out of scope.
