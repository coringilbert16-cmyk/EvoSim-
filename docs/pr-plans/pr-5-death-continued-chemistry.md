# Unify Death, Continued Chemistry, and Material Recycling

> **Planning scaffold only.** This file records the proposed scope and acceptance gate. No implementation changes or tests are included in this setup PR. Do not start implementation until explicitly authorized.

## Summary

Make death a transition out of biological activity, not an exception to physical ownership or chemistry. Dead organisms remain part of the environment and their material continues through ordinary chemistry.

## Outline

- [ ] Audit death triggers and all post-death update paths.
- [ ] Ensure biological functions stop after death.
- [ ] Ensure physical material remains owned and chemically available.
- [ ] Remove duplicate decomposition behavior where it conflicts with ordinary chemistry.
- [ ] Test conservation through death and subsequent reactions.

## Acceptance criteria

- [ ] Death does not destroy, duplicate, or orphan material or energy.
- [ ] Dead entities no longer perform biological functions.
- [ ] Subsequent chemistry follows the same authoritative rules as other matter.
- [ ] Core-breach and stress-related death paths preserve physical accounting.

## Guardrails

- Start with one focused audit of the current merged `main` before implementation.
- Use PR #205 merge commit `10521705d8f921ceb071b17240f1b7e069f52f1a` as the forward baseline; compare named test failure identities, not only totals.
- Keep each command and test run within five minutes; a timeout is incomplete evidence.
- Keep this PR limited to its primary invariant. Do not fold in unrelated cleanup.
- PR #204 is the roadmap/handoff; PR #196 constructor/Bob work is out of scope.


## Cross-PR boundary and dependency clarification (2026-10-26)

This phase owns the **life-to-death lifecycle transition and continued eligibility for ordinary chemistry**, not a second decomposition/recycling engine. Dead-body constituents remain represented exactly once and become candidates for the same ordinary chemistry pathway available to other matter; biological behavior stops.

**Dependency:** authoritative material ownership and energy settlement, plus shared chemistry runtime and transactional reaction realization. Keep biological death triggers distinct from chemistry execution. PR #213's temporary zero-stress setting is separate, temporary survival-first work and is not the final death contract.

## Current dependency/status reconciliation — 2026-10-30

- PR #214 shared chemistry runtime/scheduler is merged.
- PR #223 bounded environmental COMBINE discovery/settlement is merged.
- PR #226 flow/gradient/obstacle-aware transport is merged.
- PRs #229–#240 have integrated the first base/compound chemistry characterization, committed multi-constituent COMBINE/BREAK round trip, heterogeneous constituent chemistry, fair bounded contact discovery, and geometry-derived molecular-layout inputs.
- This plan remains planning-only. No death/lifecycle implementation is included or authorized by this document.

Before implementation, audit the exact current `main` death triggers, post-death tick paths, physical-holder ownership, and chemistry eligibility. Preserve the approved invariant: death ends biological activity, not physical existence or material ownership. Dead material must enter the same ordinary chemistry policy/transaction path as other material; do not add a parallel decomposition engine.

Use the latest accepted main-head workflow and named failure identities as the comparison baseline. The older PR #205 commit and failure totals above are historical references, not the current baseline. Keep this phase focused on the life-to-death transition and conservation through subsequent ordinary chemistry.
