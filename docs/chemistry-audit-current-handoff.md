# Chemistry Unification — Current Handoff

**Last reconciled:** 2026-10-22  
**Status:** Implementation is in progress on PR #205. This file is a concise current-state supplement to the detailed, historical audit register in `chemistry-audit-unification-plan.md`. Where the detailed register's older tracker snapshot differs from this file, use the latest exact-head evidence below and the PR #205 body until the detailed register can be reconciled. This file and PR #204 are documentation-only.

## Governing decisions and scope

- Owner-approved scientific invariants, 12-phase order, and release gates are in Section 14 and Section 17 of the [detailed chemistry audit plan](chemistry-audit-unification-plan.md).
- PR #204 is the read-only audit/handoff. Implementation and tests belong on [PR #205](https://github.com/coringilbert16-cmyk/EvoSim-/pull/205).
- PR #196 constructor/Bob work remains out of scope.
- One focused audit before each code change. Every CI command/test has a hard five-minute timeout.
- Whole-world energy/material conservation is not proven. Do not mark a phase complete because a helper or focused test passes.

## Repository and verification state

- Baseline: `main` at `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5`; baseline workflow [39705628427](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39705628427): **563 passed, 28 failed, 1 ignored**.
- Last fully verified implementation head: `686bcd0905d02c6ac4bee87e40280150765b63f9`; workflow [39814502886](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39814502886): **577 passed, 28 failed, 1 ignored**. All 28 failing test identities matched baseline exactly, with no new/missing identities. Cache missing-manifest rejection passed.
- At that head, format, source-size, architecture, Bob persistence/reopen/concurrency, focused fluid transaction/coalescence/restoration, and chemical BREAK steps passed.
- Strict Clippy failed with 34 application-target and 15 test-target diagnostics; normalized messages matched baseline.
- Latest implementation head: `c22fe33755f7c387b768543c13ec4fdf56b6e1f7`.
- Intermediate workflow [39817430683](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39817430683) on `4f4b2af` finished with **581 passed, 28 failed, 1 ignored**; the 28 failures matched baseline exactly. The containment integration test still failed at fixture setup because no full Carbon fits in the seed chamber; the independent inventory assertion was not reached. Its source fixture correction did not resolve the baseline geometry failure. No exact-head workflow result has yet been returned for `a1cc6e2`.

## Phase tracker

| Phase | Status | Next evidence required |
|---|---|---|
| 0 — Freeze baseline and scope | Recorded | None for baseline identity list; always retain exact-head comparisons |
| 1 — Contract and dependency map | Recorded | Keep contract aligned with verified behavior |
| 2 — Independent inventory oracles | Implemented; verification incomplete | Verify unknown-name and fractional-constituent rejection; broaden energy/material inventory scenarios |
| 3 — Resource identity and indivisible material ownership | In progress; regression gate open | Verify production containment transfer preserves independent inventory; then transformation rollback, physical storage/restore, and decomposition ownership |
| 4 — Chemistry cache version boundary | Implementation extended; verification pending | Verify partial-append rollback/reopen test alongside existing schema/fingerprint/missing-manifest/manifest-write tests |
| 5 — Authoritative energy ledger and initial source | Started; gate open | Broader multi-organism, bond, transfer, and world-tone inventory/conservation scenarios |
| 6 — COMBINE/BREAK, bond ownership, overload | Not started as a unified phase | Exactly-once bond-energy settlement and atomic failure/rollback evidence |
| 7 — Energy consumers and deterministic tick phases | Not started | Source/consumer inventory and retry/cancel atomicity |
| 8 — Lifecycle and post-death chemistry | Not started | Dead bodies stop biological behavior but remain available to ordinary chemistry |
| 9 — Sunlight correctness/performance | Not started | Exact-small-world oracle, source budget/occlusion, error, and performance evidence |
| 10 — Long-run numerical integrity | Not started | Drift, overflow, tolerance versus physical dissipation |
| 11 — Integrated verification | Not started | Full release-gate evidence and README consistency |

## Latest changes awaiting exact-head CI

At `0148437`:

1. Two deduplicated independent-inventory negative tests assert exact rejection of an unknown resource identity and fractional Carbon amount.
2. The production containment integration test now uses the narrow rigid Hydrogen strip (1.0 × 0.1) instead of a Carbon hexagon, because the intermediate run at `4f4b2af` showed the seed chamber could not fit a whole Carbon. It still compares independent constituent inventory before/after the live `Simulation::transfer_contained_environmental_material` call. This fixture change is test-only and awaits verification.
3. A cache partial-append rollback test appends an incomplete row, invokes the rollback primitive with an injected error, and verifies original data bytes and manifest remain unchanged and the previously committed row can be reopened. The malformed byte literal from `a1cc6e2` was corrected at `25f1df5` using a raw byte string. Workflow [39818514455](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39818514455) at `25f1df5` then reported **580 passed, 28 failed, 1 ignored**, with the same 28 named baseline failures; the new append-rollback test passed. Strict Clippy's normalized diagnostic messages matched baseline. This run predates the Hydrogen fixture change.
4. Workflow [39818747953](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39818747953) for `0148437` completed with **580 passed, 28 failed, 1 ignored**; all 28 failures matched baseline. The Hydrogen containment fixture reached the inventory assertion but failed because the baseline snapshot was taken before depositing the test material, so the deposit itself appeared as a +1 Hydrogen change. The snapshot boundary was corrected at current head `c22fe33` to occur after deposit and immediately before transfer. Exact-head workflow [39819580515](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39819580515) is in progress; no pass is inferred.

## Next actions

1. Check workflow [39819580515](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39819580515) for exact head `c22fe33`; verify all focused steps, the full suite and named baseline failure identities, both negative oracle tests, cache append rollback test, and Clippy. Confirm the inventory assertion now compares pre-transfer world state after the environmental deposit.
2. If the containment test fails, inspect its exact assertion/log first. Do not pull PR #196 constructor/Bob work into this branch.
3. Continue Phase 3 material ownership/rollback/restore/decomposition coverage and Phase 4 persistence failure-injection coverage.
4. Update this handoff and the detailed plan whenever the implementation head or meaningful verification state changes. No merge until the approved release gates have evidence.