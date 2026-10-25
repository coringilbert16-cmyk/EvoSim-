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

Perform a fresh source audit against merged `main` to select the first still-open phase and its narrow acceptance gate. Prioritize only the confirmed dependency path; do not pick work from stale dated “next” notes. Candidate open gates include transaction rejection/rollback across movement and material transfer, full independent inventory reconciliation, post-death holder chemistry, and the exact-small-world sunlight oracle. The phase choice should be made from the current source, not assumed from this list.

### Completion rule

Do not call chemistry “unified” until every authoritative material/energy path reconciles, transaction rejection cannot leave partial state, the sunlight approximation is evidence-gated, and integrated verification reports exact-head results and baseline deltas. Keep historical audit detail in the main plan; this file is the short resume point.
