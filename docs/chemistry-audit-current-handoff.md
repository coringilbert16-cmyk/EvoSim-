# Chemistry Unification — Current Handoff

**Last reconciled:** 2026-10-22  
**Latest PR #205 head:** `d07048a85b902494a54197d2ccc3852599df5fd2`  
**Exact-head workflow:** [#39821254438](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39821254438), in progress.

This is the concise current-state supplement to the detailed audit register in `chemistry-audit-unification-plan.md`. Use this file, the current PR #205 description, and exact-head workflow evidence for the latest implementation status; older tracker snapshots in the detailed audit register are historical until reconciled. PR #204 remains documentation-only.

## Governing decisions and boundaries

- Owner-approved scientific invariants, 12-phase order, and release gates are in Sections 14 and 17 of the [detailed chemistry audit plan](chemistry-audit-unification-plan.md).
- PR #204 is the read-only audit/handoff; implementation and tests belong on [PR #205](https://github.com/coringilbert16-cmyk/EvoSim-/pull/205).
- PR #196 constructor/Bob work remains out of scope.
- One focused audit before each code change. Every CI command/test step has a hard five-minute timeout; timeout means incomplete evidence.
- Whole-world energy/material conservation is not proven. The suite is not green.

## Frozen baseline and latest verified result

- Baseline: `main` at `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5`; workflow [39705628427](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39705628427): **563 passed, 28 failed, 1 ignored**.
- Last fully verified implementation head: `1d276a0c3defbcb3f28c9a36a0554ec1f5cffbdf`; workflow [39820419232](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39820419232): **582 passed, 27 failed, 1 ignored** in 238.08 seconds.
- The 27 failing test identities are exactly the frozen baseline identities minus `simulation_tests::integration_tests::contained_physical_material_becomes_storage_without_an_acquire_action`, which now passes. No new failing identities.
- At `1d276a0), formatting, source-size guard, COMBINE architecture, Bob compact-storage/reopen/concurrency, focused fluid transaction/coalescence/restoration, and focused chemical BREAK steps passed.
- The following tests passed at `1d276a0`: independent material inventory for production containment transfer, storage-to-structure restoration, stored-material-to-active-BREAK ownership, reproductive-anchor transfer to offspring, unknown resource identity rejection, fractional constituent rejection, and cache partial-append rollback/reopen.
- Strict Clippy remains baseline-red: 34 application-target and 15 test-target diagnostics; normalized messages match baseline exactly.

## Current phase tracker

| Phase | Status | Remaining gate |
|---|---|---|
| 0 — Freeze baseline and scope | Recorded | Preserve exact-head named failure comparisons |
| 1 — Contract and dependency map | Recorded | Keep docs aligned with verified behavior |
| 2 — Independent inventory oracles | Implemented; incomplete | Broaden cross-domain energy/material scenarios beyond current passing cases |
| 3 — Resource identity and indivisible ownership | In progress; regression gate open | Verify latest failed stored-BREAK rollback inventory assertion; add/verify failed transformation rollback, then cover decomposition/post-death ownership |
| 4 — Chemistry cache version boundary | Implemented; gate open | Existing schema/fingerprint/missing-manifest/reopen tests and helper-level partial-append rollback pass; add failure injection through the actual `ChemistryLibrary::insert` append path before closing |
| 5 — Authoritative energy ledger and initial source | Started; gate open | Broader multi-organism, bond, transfer, and world-tone conservation scenarios |
| 6 — COMBINE/BREAK, bond ownership, overload | Not started as a unified phase | Exactly-once bond-energy settlement and atomic rollback |
| 7 — Energy consumers and deterministic tick phases | Not started | Source/consumer inventory and retry/cancel atomicity |
| 8 — Lifecycle and post-death chemistry | Not started | Ordinary chemistry continues after death; no parallel decomposition law |
| 9 — Sunlight correctness/performance | Not started | Exact-small-world oracle, source budget/occlusion, error and performance evidence |
| 10 — Long-run numerical integrity | Not started | Drift, overflow, and tolerance-versus-dissipation tests |
| 11 — Integrated verification | Not started | Full release-gate evidence and README consistency |

## Changes and evidence after the last verified head

- At `c22fe33`, the production containment test passed after the inventory baseline was moved to after environmental deposit and immediately before transfer. The fixture uses a narrow rigid Hydrogen strip (1.0 × 0.1); no production behavior changed.
- At `25f1df5`, the cache partial-append rollback/reopen test passed. An earlier attempt at `a1cc6e2` had a malformed byte-string literal and failed compilation; the literal was corrected using a raw byte string.
- At `ea2754b`, rustfmt caught a line-wrap issue in the new storage-restore test; it was corrected at `1d276a0`.
- Latest head `d07048a` adds an independent material-inventory assertion to the failed stored-BREAK rollback case, according to the current implementation handoff. Exact-head workflow [39821254438](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39821254438) is in progress. The focused steps have passed so far; full Rust tests and Clippy have not returned. Do not infer success for this newest assertion.

## Next actions

1. Check exact-head workflow [39821254438](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39821254438) for `d07048a`; verify the new failed-BREAK rollback assertion, full-suite named failure identities, and Clippy.
2. After that run, audit the cache insertion path and add failure injection through the actual append/insert transaction rather than testing only the rollback helper.
3. Continue Phase 3 with failed transformation rollback and decomposition/post-death ownership coverage.
4. Expand Phase 5 cross-domain energy inventory scenarios; keep all phase gates open until their evidence is complete.
5. Update this handoff and PR #204 whenever the implementation head or meaningful verification state changes. No merge until the approved release gates have evidence.