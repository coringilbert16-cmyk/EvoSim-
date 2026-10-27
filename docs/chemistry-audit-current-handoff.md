# Chemistry Unification — Current Handoff

**Last refreshed:** 2026-10-26  
**Plan:** [Chemistry Audit and Unification Plan](chemistry-audit-unification-plan.md), especially §17.7 for current phase status and §§17.1–17.6 for contracts/gates  
**Plan branch:** `docs/chemistry-audit-unification-plan-2026-10-22`  
**Current `main` head:** `29f0f510062eb1ec4a1db3e1f7bb3cce12ba2d08` (PR #209 squash merge; 2026-10-27 UTC)  
**Scope:** Documentation-only handoff. No implementation or tests belong in PR #204.

## Current state

The core ownership, material, energy, cache, and selected-reaction transaction slices are merged. Chemistry is **not yet unified**: the missing integration is a shared runtime that discovers and activates reactions for environmental material and cell/organism-held material under one policy.

- [PR #212](https://github.com/coringilbert16-cmyk/EvoSim-/pull/212) — aggregate ownership and transactional construction/division: merged.
- [PR #207](https://github.com/coringilbert16-cmyk/EvoSim-/pull/207) — canonical resource identity, integer material quantities, and independent material inventory: merged.
- [PR #211](https://github.com/coringilbert16-cmyk/EvoSim-/pull/211) — chemistry cache schema/fingerprint validation and deterministic regeneration: merged. Sunlight validation is separate.
- [PR #208](https://github.com/coringilbert16-cmyk/EvoSim-/pull/208) — unified energy accounting and independent world-energy inventory: merged.
- [PR #213](https://github.com/coringilbert16-cmyk/EvoSim-/pull/213) — temporary zero-stress input for survival-first work: merged. This is not the final stress/death design.
- [PR #209](https://github.com/coringilbert16-cmyk/EvoSim-/pull/209) — live COMBINE candidate revalidation, transactional realization/settlement regression, and successful-COMBINE world-energy inventory proof: merged. Its changes from temporary PRs #219/#220 were consolidated here; both support PRs are closed and must not be reopened.
- [PR #214](https://github.com/coringilbert16-cmyk/EvoSim-/pull/214) — shared chemistry runtime and scheduler: open planning-only draft; **next implementation target**.
- [PR #215](https://github.com/coringilbert16-cmyk/EvoSim-/pull/215) — bounded chemistry event inspection and environment-first experiments: open planning-only draft; depends on the runtime/event contract from #214.
- [PR #210](https://github.com/coringilbert16-cmyk/EvoSim-/pull/210) — death and continued ordinary chemistry: open planning-only draft; depends on the shared reaction path and material/energy ownership.
- [PR #216](https://github.com/coringilbert16-cmyk/EvoSim-/pull/216) — sunlight/world-tone validation: open planning-only draft; keep separate from cache versioning and validate against an exact small-world reference.
- [PR #204](https://github.com/coringilbert16-cmyk/EvoSim-/pull/204) remains documentation-only. PR #196 constructor/Bob work remains out of scope.

## Verification checkpoint and baseline policy

The latest completed full-suite run reviewed for the consolidated #209 head `4bbc0fcab167569ee19de471b70aca178a96cf61` was [workflow 40422420501](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/40422420501):

- Full Rust suite: **754 passed, 10 failed, 1 ignored** in **273.91 seconds**.
- All 10 failing test identities were present in the accepted pre-change baseline failure set; no new failing identity was found. Eleven baseline failures no longer reproduced.
- Formatting, repository architecture checks, and all focused chemistry/energy/transaction/geometry/organism acceptance groups passed.
- Clippy remains red on repository-wide diagnostics (34 app-target, 14 test-target in this run). Do not expand chemistry phases to fix unrelated lint debt.
- This was a completed run on the PR head, not a run on the post-squash merge SHA. Treat it as the latest verified code checkpoint; trigger/check exact-main CI before making claims about any later code changes.

The prior forward baseline was reset at PR #205 merge `10521705d8f921ceb071b17240f1b7e069f52f1a` with 719 passed, 21 failed, 1 ignored. The historical pre-migration reference `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5` recorded 563 passed, 28 failed, 1 ignored. Preserve these as historical comparisons, but **use the latest accepted merged-main checkpoint and named failure identities for future work**. Neither result is a green-suite or whole-world conservation claim.

## Settled architecture — do not reopen

- One authoritative parent-owned physical aggregate until division. Stable constituent IDs and genome-core identity are metadata, not a second physical graph or energy holder.
- Attachment, growth, energy settlement, and final division are transactional. Overlap alone is not a bond.
- Movement/collision, world tone, viability, inventories, death, and recycling must count each physical constituent exactly once.
- Core breach ends biological activity, not physical ownership. Ordinary chemistry continues; do not create a separate decomposition law.
- Reproduction begins an adjacent second genome core at 100% of original size, grows the parent-owned aggregate to 160%, then splits into two roughly 80%-size children; validate both before publishing either.
- Sunlight is a deterministic rotating off-screen source. The fast first-hit approximation still needs exact-small-world, source-budget, duplicate-claim/occlusion, and performance evidence. Frequency-shifted onward propagation is not mirror reflection.
- Resource ID 6 is Oxygen. The 12-piece nitrogen seed is a cavity-calibration fixture, not a required chemistry recipe or viable-organism definition.
- Stress input is temporarily scaled to zero by #213; preserve that temporary decision while avoiding claims that the final stress/death contract is settled.

## Next implementation sequence

1. **#214 — shared chemistry runtime/scheduler.** Audit the current merged `main` tick loop and every environmental, stored, organism-held, and dead-material holder. Route candidate discovery, multi-tick activation, physical realization, and committed accounting through one reaction policy and #209's existing realization/settlement boundary. Use local/indexed discovery, deterministic ordering, bounded per-tick work, and no world-wide O(N²) pair scan. First demonstrate one environment-only reaction and one equivalent organism-held reaction.
2. **#215 — event inspection/environment-first experiments.** Once #214's real event contract exists, provide environment-only setup/run and bounded structured events that distinguish candidates, activation, rejection, and committed reactions. Preserve the default world preset and expose independent inventory snapshots; do not synthesize events.
3. **#210 — death and continued chemistry.** Ensure biological activity stops after death while matter remains owned and eligible for the same ordinary chemistry. Prove conservation through death and subsequent reactions.
4. **#216 — sunlight/world-tone validation.** Compare the fast approximation to an exact small-world reference; prove occlusion, source budget, no duplicate claims, onward frequency-shifted propagation, and representative performance.

This is a dependency-informed order, not permission to combine phases into one PR. Keep implementation in the existing phase PRs; do not open additional PRs without authorization. Before each phase, audit current `main`, reuse existing tests, add only the missing acceptance assertions, compare named failures with the latest baseline, and keep commands/tests within five minutes. The full Rust suite has a separate agreed ceiling of seven minutes.

## Definition of “chemistry unified”

Do not make that claim until:
- environmental, organism/cell-held, and dead material use the same chemistry policy and transaction path;
- candidate/activation state is distinct from physical reaction realization and is not counted as stored energy;
- rejection leaves geometry, bonds, material ownership, energy, and inventories unchanged;
- successful reactions reconcile integer material counts and independent world-energy inventory;
- deterministic scheduling and bounded work are demonstrated;
- sunlight approximation and whole-world conservation gates have evidence;
- exact-head integrated verification and named baseline deltas are recorded.

Historical audit entries in the main plan are retained for traceability. They do not override this current handoff or settled owner decisions.
