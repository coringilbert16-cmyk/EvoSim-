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


## Latest local-contract audit checkpoint — 2026-10-22, Section 17.65

- **Plan commit:** `1ba675a9c735557b927fd7f5a4e13ea210e2beec`. PR #204 remains documentation-only.
- PR #205 source head reviewed: `12d8079230458da68a8dc3755a1d6a9b41347862`; no source/test changes.
- Three remaining local-contract gaps were reconfirmed: bondless reaction accumulation returns before stale-key cleanup; prepared COMBINE calculates escaped-tone/resonant-held deltas but omits them from commit/rollback (current candidate should leave both zero); cache tests reject a changed fingerprint string but do not bind catalog chemical-position changes to fingerprint invalidation.
- The energy transfer primitive itself stages and validates both holder values before assignment; the construction caller's ignored boolean remains the problem.
- Required next evidence: stale-key cleanup regression; explicit zero-field COMBINE invariant or symmetric field-wise commit/rollback test; catalog chemistry fingerprint invalidation test; reserve-transfer rejection policy test.
- Section 17.64 material-loss/movement failure rows remain open. Exact-small-world sunlight proof and post-death non-primary-holder reachability remain open; receiver eligibility is still owner-decision-blocked.
- Recorded workflow 39833438679 is still the latest evidence and remains baseline-red (585 passed, 27 failed, 1 ignored; baseline identities only). It is not fresh CI.
- **Next:** consolidate remaining rows into one exit checklist and identify any uninspected production mutation paths. No implementation until all audit templates are complete.


## Latest touched-file cross-check — 2026-10-22, Section 17.66

- **Plan commit:** `3695aa8c228877b963b5c8ebf4c9c654e02fe021`. PR #204 remains documentation-only.
- PR #205 exact source head audited: `12d8079230458da68a8dc3755a1d6a9b41347862`; no source/test/document changes.
- `docs/CHEMISTRY_UNIFICATION_CONTRACT.md` labels its dependency table as the frozen-main baseline, so old schema-3 and aggregate-transfer entries are historical, not current implementation claims. Its plan-authority link is pinned to an older plan commit and should be refreshed after audit closure.
- README's “live capture stays disabled” sentence is stale: capture/settlement is live but unvalidated. Update only after audit closure and owner approval.
- CI steps use five-minute timeouts. The staged graph API in `src/geometry/structure.rs` is not evidence of live energy-settlement reachability; trace production callers.
- **Next:** produce a row-by-row exit checklist for Section 17.55, classify each transition as locally verified, test-required, or owner-decision-blocked, and reconcile with A0–A21 and Section 17.7. No implementation until every audit template is complete.


## Latest exit-checklist reconciliation — 2026-10-22, Section 17.67

- **Plan commit:** `c10cca789ca18e2c953ca344d0ae0f43dee30062`. PR #204 remains documentation-only.
- The source-to-sink matrix is now reconciled into 17 explicit transition/release-gate rows. Local transaction ordering is confirmed for several paths, but material-loss paths, movement rollback, construction failure handling, malformed resonance inputs, cache invalidation, stale activation keys, and global conservation remain open.
- **Owner-decision blocker:** whether non-primary stored/committed/developing surfaces owned by a living parent can retain resonance energy. Do not infer this from ownership alone.
- No implementation authorized: deterministic failure tests and independent inventory assertions are still missing for several rows, and the final production mutation-path search remains to be reconciled.
- Exact PR #205 source head used for this audit series: `12d8079230458da68a8dc3755a1d6a9b41347862`. No source/test changes were made.
- Latest recorded workflow 39833438679 remains historical and baseline-red (585 passed, 27 failed, 1 ignored; baseline failure identities only, normalized Clippy identities unchanged). Re-fetch current head/CI before any code change.
- **Next:** final source search for production material/energy mutation paths missing from Section 17.67, then close audit templates and owner decision before implementation.


## Latest production mutation-path checkpoint — 2026-10-22, Section 17.68

- **Plan commit:** `5197f31bb4629fe7b69efc5ff6208ff2b2ccda2b`. PR #204 remains documentation-only.
- Broad exact-head scan covered all 19 PR #205 changed paths plus movement, reproduction, world-tone, harmonics, and expulsion modules at `12d8079230458da68a8dc3755a1d6a9b41347862`. No source/test changes.
- No new distinct physical source/sink category was found beyond the existing Section 17.55 matrix. Initialization, COMBINE/BREAK, world-tone, material acquisition/expulsion, movement, construction, cache, and independent inventory paths are represented.
- This is not proof of conservation or closure: deterministic rejection tests, whole-world inventory assertions, exact-small-world sunlight comparison, cache invalidation/restart coverage, and several lifecycle checks remain open. The legacy decomposition helper stays quarantined.
- **Next:** complete audit-template sign-off by assigning each remaining row exact source/test references and a deterministic acceptance condition. Keep any row open if its proof requirement is not testable. No implementation until sign-off and the receiver-eligibility decision are complete.


## Latest acceptance-test design checkpoint — 2026-10-22, Section 17.69

- **Plan commit:** `977cae167cbef70b7295bfe5d1c5c44791b28928`. PR #204 remains documentation-only.
- Section 17.69 now names the existing tests to preserve and the deterministic regressions required for each open transaction/release gate: source-once initialization, COMBINE ledger symmetry, chemical/resonance BREAK, stored BREAK, environmental COMBINE, acquisition, expulsion, movement rollback, maintenance, construction reserve/growth/finish, world tone, reaction accumulation, cache invalidation/restart, post-death holder reachability, numerical integrity, and docs/diagnostics.
- Each new failure test must assert both object state and the independent material/energy oracle; helper-only tests or ledger counters alone are insufficient.
- **No implementation yet.** The owner decision on non-primary receiver eligibility remains open, and the final audit-template sign-off has not been declared complete. No source/test changes.
- PR #205 exact head reviewed in this audit series: `12d8079230458da68a8dc3755a1d6a9b41347862`. Latest recorded CI is still workflow 39833438679 and is not fresh.
- **Next:** obtain owner direction on whether parent-owned stored/committed/developing surfaces can retain resonance energy, then finalize the audit-template sign-off. Do not change receiver mapping or begin code before that decision.


## Latest owner-decision checkpoint — 2026-10-22, Section 17.70

- **Plan commit:** `f19d7c9a93e98cb92982b5571710ea4e4872a22b`. PR #204 remains documentation-only.
- **Owner decision:** physical structure and “committed reproductive structure” are one category; a separate “developing structure” identity is not desired as a physical model. Any structurally attached units count for the living owner’s resonance; stored material does not. This resolves the former owner-decision blocker but is not implementation proof.
- **Source impact at PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`:** `world_tone_snapshot.rs` represents primary structure, developing structure, committed material, stored material, and active-transformation escrow as separate physical bodies. Only primary `organism.structure` body is mapped to an organism resonance reservoir. The developing structure therefore cannot credit the parent’s reservoir and is treated as a nonliving receiver (response becomes heat). Stored material is also a physical receiver/occluder and must remain excluded from parent resonance under the owner rule.
- `reproduction.rs` adapts developing construction into a temporary `Organism` value, copying structure, committed storage, energy, stress, debt, and origin. Trace all construction/growth/wait/finish/detach/death/division paths before removing or changing this state; do not create duplicate physical authority or energy.
- Existing snapshot test `developing_offspring_and_its_stored_material_remain_world_occluders` currently encodes the old separate-body geometry. Revise its intended expectations as part of the audited design; do not preserve the old representation as a constraint.
- **Next:** complete the source-impact matrix for construction holders/transfers and attached-structure resonance; revise deterministic sunlight and inventory acceptance cases; then reconcile audit sign-off. No source/test changes until all audit templates close. Before implementation, re-fetch PR #205 head and workflow state.
- No source or test files changed. Latest recorded CI remains historical workflow `39833438679`; do not call it fresh verification.


## Latest reproductive-structure source-impact checkpoint — 2026-10-22, Section 17.71

- **Plan commit:** `e44aaa5b1da959e627080500b7cd595c295d8532`. PR #204 remains documentation-only.
- Exact PR #205 head reviewed: `12d8079230458da68a8dc3755a1d6a9b41347862`; no source/test changes.
- `begin_reproduction` removes an anchor from parent storage and places it into `ReproductiveConstruction.developing_structure`, while leaving the parent's `organism.structure` unchanged. Growth clones that construction graph/storage into a temporary Organism-shaped workspace, applies maintenance/overload/construction, and copies state back. Finish converts the construction record into a new Organism. Thus current code uses a second graph identity for attached reproductive structure.
- `world_tone_snapshot.rs` maps only the primary structure to the parent resonance reservoir; it separately includes developing structure, committed construction storage, parent storage, and active transformation escrow as physical receivers/occluders. Under the owner decision, attached reproductive structure must be ordinary parent structure and may credit the living parent’s reservoir; stored material stays separate and excluded from owner resonance.
- **Important wording:** “committed reproductive structure” means attached physical structure being built; it is not `committed_material` (stored inventory). Do not include stored material in resonance or rename it into structure.
- Current finish handles Ready, Detached, and Dead by taking the construction record and attempting to produce an offspring; this is not yet proof of the previously approved atomic division of all units, holders, bonds, and energy. Keep parent ownership until the approved division boundary.
- **Next:** audit all `ReproductiveConstruction` call sites, geometry/collision/cache assumptions, and independent material/energy oracles; define a single authoritative structure representation and revise sunlight/transaction tests. Other audit templates remain open, so no implementation yet. Historical CI remains workflow `39833438679`, not fresh verification.


## Cross-call-site representation checkpoint — 2026-10-22, Section 17.72

- **Plan commit:** `956575b99f5900b70427ec0da374f1edd431bc6e`. PR #204 remains documentation-only; PR #205 source head audited remains `12d8079230458da68a8dc3755a1d6a9b41347862`.
- `ReproductiveConstruction` is serialized and carries `developing_structure`, committed storage, genome, developmental frame, stress, anchor index, developing energy, and maintenance debt. Removing the second graph has persistence/schema implications.
- Movement translates the parent structure and then separately translates the construction graph; `parent_body_geometry` sees only the parent graph, while construction checks contact/boundary against two graphs. A unified structure must avoid double movement and include attached units in collision/occupancy/geometry.
- Production structural-energy totals and the independent test energy snapshot sum parent and developing graph separately; usable energy also includes a separate construction holder. Update authoritative totals/oracles with representation changes so bonds count once and holder ownership remains explicit.
- Tick order currently advances construction after ordinary chemistry and applies maintenance/resonance-overload BREAK to a temporary developing Organism. Decide which aggregate biological actions apply to the parent versus the internal second core while preserving death/ordinary-chemistry rules.
- **Next:** inspect serialization/load compatibility and tests for anchor/cavity/boundary/detachment semantics, then finish the canonical single-structure design and acceptance cases. Do not patch sunlight alone. Other audit templates remain open; no source/test changes. Recorded CI remains historical workflow `39833438679`.
