# Chemistry Unification — Current Handoff

**Plan:** `4cbf7bb6d2c7f52ad43e67ae9eec2624c8064a27`  
**PR #205 audited head:** `12d8079230458da68a8dc3755a1d6a9b41347862`  
**CI:** [39833438679](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39833438679).

PR #204 is documentation-only. Full decisions and audit records are in `docs/chemistry-audit-unification-plan.md`, Sections 14–17. No implementation source/tests changed during this audit series.

## Rules and evidence

- Baseline `main`: `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5`; workflow 39705628427: 563 passed, 28 failed, 1 ignored. Compare named failures.
- Resource ID 6 is Oxygen; PR #196 constructor/Bob remains separate. One focused audit before each change; commands/tests max five minutes.
- Parent owns body, storage, committed material, developing structure/bonds and associated energy until division. Death stops biological processing, not physical existence; no separate decomposition law. Ownership alone does not determine chemistry/sunlight eligibility.
- Workflow 39833438679: formatting/focused gates passed; 585 passed, 27 failed, 1 ignored. All 27 failures match baseline identities minus one resolved baseline failure. Clippy identities match baseline. Suite remains baseline-red; no performance or whole-world conservation proof exists.

## Confirmed gaps

- Reaction enumeration covers primary graph bonds only; stored/committed and developing holders are outside it. Stored physical connections have no allocated bond energy.
- Stored BREAK can partially commit pieces and lose escrow on malformed completion; acquisition/expulsion ignore failed destination returns.
- Movement changes geometry before ledger settlement; rejection can leave an uncharged move. Its cost is heat-only/zero explicit work pending D7 reconciliation.
- Construction reserve transfer ignores the atomic transfer result.
- World-tone local balance/staged reservoir commit exist; exact-small-world accuracy/performance and the non-primary receiver rule remain open.
- Prepared COMBINE omits escaped-tone/resonant-held ledger delta application; current candidate should leave those fields zero. Live frame settlement does not update the documented unabsorbed-throughput diagnostic.
- Cache schema 4/strict manifest checks exist. Static potential depends on chemical positions and K/Dmax, but the manual fingerprint must change with catalog chemistry. Bondless reaction accumulation can retain stale keys.
- Live world tone uses `settle_world_tone_paths`; `EnergySourceBudget` is staged/tested, not the live exposure authority.

## Next

The current A0–A21 summary and Section 17.7 tracker are reconciled. Section 17.55 consolidates source→holder→sink paths; Section 17.56 maps energy APIs. Finish remaining call-site/failure matrix before implementation. After audit closure, prioritize stored BREAK atomic ownership, checked acquisition/expulsion returns, movement geometry/ledger atomicity, bondless key cleanup, catalog fingerprint and prepared-ledger invariants, and construction-transfer rejection behavior. Re-fetch PR #205 head and CI before any code change. Do not claim chemistry unified until Section 17.6 release gates have evidence.

Future reproduction is separate: start adjacent budding at 100% original size, grow to 160% total, split into two roughly 80%-size children.


## Latest audit checkpoint — 2026-10-22, Section 17.63

- **Plan commit:** `766ef07f50b0cd243a32d66cf06864a180b20ca7`. PR #204 remains documentation-only.
- **Exact PR #205 source head audited:** `12d8079230458da68a8dc3755a1d6a9b41347862`; no source/test changes.
- Resonance helper call sites were reconciled across `harmonics.rs`, `chemical_reaction.rs`, `transformation.rs`, `world_tone.rs`, and `world_tone_snapshot.rs`. Existing tests cover valid response and energy bounds but lack adversarial malformed-property coverage. Caller finite checks do not catch NaN values masked by helper epsilon clamps.
- Section 17.63 defines the future test layers: direct helper validation, live world-tone no-mutation behavior, chemistry no-activation/BREAK behavior, and inherited legacy/spectrum behavior. This is a defensive invalid-state gap, not a demonstrated default-catalog energy leak.
- Latest recorded exact-head workflow remains [39833438679](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39833438679), conclusion failure: 585 passed, 27 failed, 1 ignored; all 27 failure identities matched the frozen baseline, with one baseline failure still resolved. Focused gates and formatting passed; normalized Clippy diagnostic identities matched baseline. Do not describe this as fresh CI.
- **Next:** finish the remaining material/energy transition failure matrix and reconcile each row with Section 17.7. No source/test changes until every audit template is complete. Before the first implementation change, re-fetch the PR #205 head and current workflow state.


## Latest transition audit checkpoint — 2026-10-22, Section 17.64

- **Plan commit:** `a34fd2f58aa708d258387dd4a8f2d524441c2930`. PR #204 remains documentation-only.
- Exact PR #205 source head reviewed: `12d8079230458da68a8dc3755a1d6a9b41347862`; no source/test changes.
- Reconfirmed five failure-path findings: movement mutates geometry before ledger settlement; expulsion removes storage before a fallible destination deposit; contained acquisition removes field sources before storage and ignores failed fallback deposits; construction reserve transfer ignores the atomic transfer result; stored BREAK remains incrementally committed and has only partial rollback coverage.
- Section 17.64 records exact required rejection tests and clarifies that existing movement geometry/push tests do not test a ledger rejection after a successful move.
- Still open: COMBINE prepared-ledger invariant, stale reaction keys, cache catalog fingerprint/restart, maintenance debt/rejection, construction growth/finish stale-source rollback, world-tone exact-small-world oracle, and post-death holder reachability. Parent-owned non-primary sunlight receiver eligibility remains owner-decision-blocked under Section 17.38.
- Latest recorded CI is still workflow 39833438679 (failure; 585 passed, 27 failed, 1 ignored; failure identities match frozen baseline, normalized Clippy identities match baseline). Historical evidence only, not fresh verification.
- **Next:** reconcile remaining rows against Section 17.7 and the Section 17.6 release gates; classify each as source-inspection-complete, test-required, or owner-decision-blocked. No implementation until every audit template is complete.
