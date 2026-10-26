# Chemistry Unification — Current Handoff

**Last refreshed:** 2026-10-25  
**Plan:** [Chemistry Audit and Unification Plan](chemistry-audit-unification-plan.md), especially §§17.1–17.7  
**Plan branch:** `docs/chemistry-audit-unification-plan-2026-10-22`  
**Current `main` head:** `10521705d8f921ceb071b17240f1b7e069f52f1a`  
**PR #205:** [merged](https://github.com/coringilbert16-cmyk/EvoSim-/pull/205) on 2026-10-25; squash commit `10521705d8f921ceb071b17240f1b7e069f52f1a`.

## Purpose

Establish one coherent, deterministic, conservation-auditable chemistry and energy model. Every material/energy change needs one authoritative owner, one transaction boundary, and an independently checkable accounting consequence. A qualifying cavity is an early structural milestone, not proof of a viable organism.

## Current evidence

- **Forward regression baseline reset:** merged `main` at `10521705d8f921ceb071b17240f1b7e069f52f1a` (PR #205), exact-head suite **719 passed, 21 failed, 1 ignored**. All 21 failure identities were compared with the prior baseline and already existed; no new failure identity was introduced. Future work should compare against this commit and its named failure set.
- **Historical pre-migration baseline:** `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5`, workflow [39705628427](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39705628427): **563 passed, 28 failed, 1 ignored**. Retain for migration history; do not use it as the primary forward baseline after the reset.
- The reset is not a green-suite claim and does not prove whole-world conservation.
- Both corrected migration regressions passed on the final tested head: partial aggregate membership in movement and developing-offspring world-tone occlusion.
- Strict Clippy remains red (33 application-target and 15 test-target diagnostics on the last verified head). Do not claim diagnostic identity parity without a fresh normalized comparison.
- Whole-world material/energy reconciliation, complete post-death ordinary chemistry, exact-small-world sunlight accuracy, representative large-world performance, and complete lifecycle/division conservation remain unproven.

## Settled architecture — do not reopen

- One authoritative parent-owned physical aggregate before division. Stable constituent IDs and genome-core identity are metadata; no second persistent developing graph or energy holder.
- Attach and grow transactionally through the shared physical/chemistry mechanisms. Overlap alone is not a bond. Failure restores graph, storage, and ledger.
- Movement/collision, world tone, viability, inventory, death, and recycling must see the aggregate exactly once.
- A breached core stops biological work but does not erase physical ownership or create a separate decomposition law. Ordinary chemistry/recycling continues against authoritative physical ownership.
- Reproduction starts an adjacent second genome core when the original reaches 100% size, grows to 160% total, then divides into two roughly 80%-size children. Division is one atomic transaction; validate both children before publishing either.
- Sunlight is a deterministic rotating off-screen source. Use the fast first-hit/single-sweep approximation only with exact-small-world comparison and source-budget/occlusion proof. “Reflected” tone means onward frequency-shifted propagation, not mirror-ray tracing.
- Resource ID 6 is Oxygen. PR #196 constructor/Bob work stays out of this workstream.

## Direction from here

PR #204 is **documentation-only**. Implementation belongs in focused PRs, **one PR per open phase** of §17.2—not another giant PR. Start each phase from a fresh audit of current `main`, then make the smallest cohesive change, with one focused audit before each change. Keep every command/test under five minutes. Run focused gates first and compare named test failures to the reset forward baseline. A timeout is incomplete evidence.

### Recommended next action

**Selected first slice: Phase 7 — movement transaction rejection + inventory proof.** On merged `main` `10521705d8f921ceb071b17240f1b7e069f52f1a`, movement preflights ledger settlement before geometry mutation and commits that staged ledger only after the push plan succeeds. The current rejected-settlement test uses no pushed organisms or field objects and checks only a subset of mover/ledger state. Existing tests separately cover successful organism push chains and physical-material reindexing, but not a single full inventory/state assertion across rejection.

Acceptance tests for the first implementation PR:
1. With an invalid/overflowed ledger and a mover, pushed organism(s), and realized physical field material in the movement path, rejection leaves all geometry, IDs/material, storage, field ordering, revisions, spatial-index results, active-movement counters, usable energy, and ledger counters unchanged.
2. A blocked push chain after successful ledger preflight discards the preflight and leaves the same full state/inventories unchanged.
3. A successful push-chain control moves each physical owner exactly once, preserves the independent integer-material inventory, updates spatial-index queries, and reconciles energy with the expected movement heat debit.

Reuse `rejected_movement_energy_settlement_does_not_mutate_world_geometry`, `movement_pushes_another_organism_atomically`, `movement_propagates_an_organism_push_chain_atomically`, `movement_pushes_realized_physical_material_and_reindexes_it`, and `successful_movement_commits_preflight_energy_and_ledger_after_geometry`. Add only the missing cross-domain assertions; do not add a redundant transaction wrapper where no fallible operation exists after the push plan is applied. Keep each command/test under five minutes and compare named failures with the reset forward baseline.

### Completion rule

Do not call chemistry “unified” until every authoritative material/energy path reconciles, transaction rejection cannot leave partial state, the sunlight approximation is evidence-gated, and integrated verification reports exact-head results and baseline deltas. Keep historical audit detail in the main plan; this file is the short resume point.


## Chemistry planning reconciliation — 2026-10-26

This section reconciles the queued chemistry planning PRs with the approved phase order and the active migration work. It does **not** authorize implementation or supersede the current next-action note above.

### Current priority remains separate

- PR #206 remains the narrow movement rejection/rollback and inventory proof slice called out in the current handoff.
- PR #212 remains the aggregate-ownership/construction/division migration. Resource/energy inventory work must not create a competing physical graph or energy holder while that migration is in progress.
- PR #213 is a temporary stress-input setting for survival-first development. It does not settle the final death/stress contract.
- PR #196 constructor/Bob work remains out of scope.

### Chemistry planning sequence and scope boundaries

1. **PR #207 — resource identity and integer material inventory.** Limit it to canonical resource IDs, integer quantities, transfer/storage accounting, and independent material inventory. Do not redo parent-aggregate ownership, construction membership, or division handled by PR #212.
2. **PR #211 — cache versioning only.** It has been narrowed from cache-plus-sunlight. Establish equation/schema compatibility before downstream phases rely on cached results.
3. **PR #208 — unified energy accounting.** Establish the independent world-energy oracle and source/transfer/storage/work/heat settlement contract after ownership state is coherent.
4. **PR #214 — shared chemistry runtime and scheduler.** This is the missing integration layer: one reaction policy for environmental and cell/organism-held material, local candidate discovery, deterministic activation progress, and bounded work. The current organism-bond accumulator alone does not establish environment-wide chemistry.
5. **PR #209 — transactional reaction realization.** Own the atomic commit/rollback of a selected reaction; do not duplicate the scheduler from PR #214.
6. **PR #210 — death and continued chemistry.** Stop biological behavior after death while keeping matter available to the ordinary shared chemistry path; no second decomposition law.
7. **PR #216 — sunlight/world-tone validation.** Separate from cache work and gate on exact-small-world comparisons, occlusion, source budget, duplicate-claim checks, and performance.
8. **PR #215 — event inspection and environment-first experiments.** Depends on the real runtime/event contract. Separate environment setup from organism population while preserving the default preset; distinguish candidates from committed reactions; bound logs and benchmark 24,000 ticks.

These are dependency relationships, not a claim that every listed PR must merge in strict numerical order. Re-audit current main and the status of PR #206/#212 before selecting the next implementation slice. Every phase remains planning-only until separately authorized. PR #204 remains documentation-only; README is not changed as a side effect.
