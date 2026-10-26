# Complete Aggregate Ownership and Transactional Construction and Division

> **Implementation in progress.** This file tracks the aggregate-ownership migration and its acceptance evidence.

## Summary

Finish the ownership migration so an organism’s physical aggregate has one authoritative owner throughout construction, attachment, operation, and division. Preserve the distinction between a parent-owned aggregate and independent organisms after a real split.

## Outline

- [x] Audit current merged `main` for remaining duplicate ownership and lifecycle gaps.
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

## Current implementation and verification

- Added regression assertions to `finish_reproduction_readiness_ignores_stale_cached_graph`: a rejected aggregate-owned finalization attempt must preserve parent units, bonds, genome-core identity, stored material, usable energy, the energy ledger, developing-member IDs, and committed construction material.
- Refactored `finish_reproduction` to hold the staged parent graph/energy until the child object is assembled, then publish the parent graph, parent energy, and staged ledger at one commit point. The live construction stays installed during staging and is cleared only after publication.
- Refactored `commit_candidate_attachment` so parent structure, parent storage, construction metadata, parent energy, and ledger are prepared on staged copies. It publishes those holders only after contact/placement checks, aggregate replacement, and source-material removal succeed.
- Updated the waiting/resume regression to preflight the actual attachment solver and supply a physically attachable material, rather than assuming chemical preference alone guarantees a geometric bond.
- The focused rejected-reproduction rollback regression passed on an earlier head. The latest attachment transaction and revised waiting/resume test have **not yet been executed on the latest head**; no current-head workflow run is available from the repository integration, and local test execution is unavailable in this environment. Do not treat this step as verified until the focused tests run on the latest commit.
- The prior full suite had 21 failures, including the waiting/resume regression and canonical seed realization failures. The seed failure still needs comparison by named failure identity against PR #205 baseline `10521705d8f921ceb071b17240f1b7e069f52f1a`; do not weaken viability requirements to make the seed pass.
- Source audit confirms aggregate-owned construction derives transient views from stable IDs and division stages through `partition_subgraph_for_division`. Legacy detached-graph compatibility remains in production accounting/consumer paths and is still a migration gap.

## Guardrails

- Start with one focused audit of the current merged `main` before implementation.
- Use PR #205 merge commit `10521705d8f921ceb071b17240f1b7e069f52f1a` as the forward baseline; compare named test failure identities, not only totals.
- Keep each command and test run within five minutes; a timeout is incomplete evidence.
- Keep this PR limited to its primary invariant. Do not fold in unrelated cleanup.
- PR #204 is the roadmap/handoff; PR #196 constructor/Bob work is out of scope.
