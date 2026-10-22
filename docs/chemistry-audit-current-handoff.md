# Chemistry Unification — Current Handoff

**Plan (latest recorded plan commit):** `56eb760a40bde64286a0bb33d946522e06b83c98`  
**PR #205 last audited head (historical; refresh before implementation):** `12d8079230458da68a8dc3755a1d6a9b41347862`  
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

The current A0–A21 summary and Section 17.7 tracker are reconciled. Section 17.55 consolidates source→holder→sink paths; Section 17.56 maps energy APIs. Owner decision D16 / Section 17.83 now explicitly approves one authoritative physical graph: no reproductive-material tag and no separate physical `developing_structure`; any structurally attached unit is ordinary structure and participates in owner resonance, while only stored material is excluded. The source/holder and transaction-failure matrix is consolidated in Section 17.84. A targeted exact-head mutation-call-site cross-check found no additional production `EnergyLedger.transfer` caller beyond construction reserve and the quarantined recycler; it refined movement rollback to include pushed organisms, pushed physical field objects, and their indexes. Remaining audit gates include aggregate graph/core identity, deterministic failure tests, cache invalidation/restart, world-tone exact-small-world oracle, post-death holder reachability, and whole-world conservation. After audit closure, prioritize stored BREAK atomic ownership, checked acquisition/expulsion returns, complete movement rollback, bondless key cleanup, catalog fingerprint and prepared-ledger invariants, and construction-transfer rejection behavior. Re-fetch PR #205 head and CI before any code change. Do not claim chemistry unified until Section 17.6 release gates have evidence.

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


## Reproduction test-contract checkpoint — 2026-10-22, Section 17.73

- **Plan commit:** `53cfa57aee0e195a087c56998ca39983d51715ad`. No source/test files changed.
- Existing `anchor_starts_a_separate_physical_child_graph` explicitly asserts the now-rejected separate graph. Replace with a single-aggregate invariant: anchor leaves storage and enters parent structure exactly once; second core/progress remains internal.
- Existing boundary/detachment tests assume a separate child graph can leave the parent boundary or lose graph-to-graph contact. Re-evaluate under internal/adjacent budding: parent ownership remains until division; if detachment is still valid, it needs a physical partition/bond event and explicit settlement, not contact alone.
- Waiting/resume tests should preserve state while waiting and attach new material exactly once. Developmental scale should be tracked through the internal second-core region/progress, not a separate physical graph identity. Preserve the previously approved reproduction threshold unless the owner changes it.
- Required future acceptance tests cover atomic anchor/growth, no-mutation on waiting/rejection, one-time movement/spatial indexing, attached-vs-stored resonance eligibility, post-death physical ownership, atomic division, and save/load migration/fail-closed behavior.
- **Next:** finish saved-state compatibility and construction/geometry acceptance matrix; then reconcile all remaining chemistry audit rows and sign-off. No implementation until all audit templates close. PR #205 head/CI must be refreshed before first code change.


## Cache/persistence dependency checkpoint — 2026-10-22, Section 17.74

- **Plan commit:** `fec43f8b8b87462b238bf9e87bad78a72970221d`. PR #204 remains documentation-only.
- Unified attached structure will require canonical `mark_structure_changed` invalidation for cavity/developmental/harmonic caches on successful attachment/growth/partition, with no invalidation on rejected candidates. Current construction mutates its own graph separately and therefore does not invalidate parent structure caches.
- Parent cavity analysis currently sees only `organism.structure`; the aggregate model must preserve identity of the parent genome core and internal second core rather than treating the whole aggregate as one cavity. The construction anchor index currently indexes the separate graph and must be remapped/replaced.
- Serialized parent structure and serialized `ReproductiveConstruction.developing_structure` are separate. New schema must migrate active legacy construction exactly once or fail closed without losing units/bonds/energy; simply ignoring/deleting the field loses physical state. Required tests cover round-trip, legacy migration/idempotence, cache invalidation, core identity, and movement/snapshot agreement.
- **Next:** finish the canonical representation/migration acceptance matrix and reconcile it with the other open chemistry release gates. No source/test changes; PR #205 head remains historically audited at `12d8079230458da68a8dc3755a1d6a9b41347862`; re-fetch head/CI before implementation.


## Multi-core cavity identity checkpoint — 2026-10-22, Section 17.75

- **Plan commit:** `b763207a5fe8b81db612db8f37c96fe735bd3ba9`. No source/test changes.
- `analyze_genome_cavity` selects the largest qualifying cavity across connected components and accepts no core identity. Parent viability/harmonics use this analyzer; the child’s cavity is currently analyzed separately because its graph is separate.
- Blindly merging attached construction units into the same graph could switch the parent’s active cavity/perception as the second core grows. The unified structure therefore needs core-specific cavity identity/query (or a non-owning analysis view referencing shared unit IDs) without duplicating physical units, bonds, energy, or receiver bodies.
- Required tests: two qualifying cavities independently discoverable; parent active cavity does not switch as second core grows; child construction readiness targets its own cavity; mutation invalidates both results; attached surfaces map to parent resonance; division partitions each core and unit exactly once.
- **Next:** define canonical single-owner physical state plus multi-core identity/query contract, then reconcile test matrix and remaining chemistry gates. Do not blind-append graphs or patch sunlight alone. No implementation yet; PR #205 head/CI refresh required before code changes.


## Current tracker reconciliation — 2026-10-22, Section 17.76

- **Plan commit:** `e14541333658fe818e2cad8acb515217d3c77440`. Section 17.7 Phase 1/8/9 statuses now reflect the owner decision; older audit checkpoints remain historical.
- Receiver policy is **decided**: attached reproductive structure is ordinary parent structure and may contribute to the living parent’s resonance; stored material is excluded. Do not keep describing this as an open owner question.
- Implementation/design blocker remains: current source serializes a separate `developing_structure` graph. Unification must preserve parent and second-core identity, avoid double-counting/movement, revise independent inventories, update cache invalidation, and define legacy save migration or fail-closed behavior.
- Other audit gates remain open: deterministic transaction rejection tests with independent inventory assertions, exact-small-world world-tone oracle, cache catalog invalidation/restart, and whole-world conservation/long-run integrity. No source/test changes in this documentation series.
- **Next:** finish single-authority aggregate/core-identity contract and serialized-state compatibility/test matrix; reconcile remaining audit rows; only then select the smallest implementation unit. Re-fetch PR #205 head/CI before code changes.


## Recommended canonical model — Section 17.77

- **Plan commit:** `85c4d141a4fd9bf4c4a00368ac9f7db43273c696`. This is a proposed design recommendation, not yet owner-approved implementation detail.
- Recommendation: `Organism.structure` becomes the only authoritative physical graph for all attached units until division. Remove `ReproductiveConstruction.developing_structure` as an independent physical graph; retain only necessary nonphysical construction metadata (child genome, second-core reference, and explicitly owned construction energy/stress/debt/progress if needed). Do not mark structural units as reproductive material. Keep un-attached material in `MaterialStorage`.
- Growth stages candidate changes against the aggregate graph and uses a second-core anchor/reference to attach new units to that core. Add a core-specific cavity query/reference so parent perception does not switch to the larger second cavity; geometry remains emergent and no fixed shape/material recipe is introduced.
- One aggregate graph must be the source for movement, collision, occlusion, cache invalidation, ordinary chemistry, and sunlight. Every attached surface may credit the living parent reservoir; stored material and transformation escrow stay separate and resonance-ineligible.
- Division remains a single atomic partition of units, bonds, storage, core identities, and energy at the already-approved condition; cross-partition bonds must settle explicitly. Legacy serialized construction graphs need exactly-once migration or fail-closed recoverable handling.
- Risks to close: stable unit/bond IDs and endpoint remapping, core-specific cavity identity, maintenance/overload semantics with two cores, migration idempotence, construction reserve-holder semantics, and atomic division. No implementation yet; audit templates remain open.


## Serialization-scope correction — Section 17.78

- **Plan commit:** `463b9f33c74a9f3ec42f6c06028098e1b174ef26`.
- Correction to Sections 17.72/17.74/17.77: `ReproductiveConstruction` and `Organism` derive Serde traits, but the current `Simulation` itself does not, and no whole-world save/load path was found in reviewed runtime/server/process call sites. Confirmed production serialization is diagnostic/observation output (`serde_json::to_value` in `src/infrastructure/diagnostics.rs`), not simulation persistence.
- Do not treat legacy world-save migration as a current required task. Current compatibility work is to revise diagnostic/observation JSON and tests so they represent the unified physical structure correctly. If world persistence is introduced later, migration/fail-closed handling becomes a prerequisite then.
- This correction does not change single-authority structure, core identity, movement/geometry, cache invalidation, inventory, resonance, or division requirements.
- **Next:** finish canonical aggregate/core-identity contract and diagnostic JSON impact matrix, then reconcile remaining chemistry audit rows. No source/test changes; refresh PR #205 head and CI before implementation.


## Diagnostic JSON contract checkpoint — Section 17.79

- **Plan commit:** `247f71340c8ad6d21788d694a2897f484d5c14d0`.
- `src/infrastructure/diagnostics.rs` records `unit_count`, `bond_count`, `component_count`, `structural_mass`, and serialized `structure` from `organism.structure` only, while separately serializing the entire `reproductive_construction` object. With one authoritative graph, aggregate metrics must include attached units once; construction JSON must expose progress/core metadata and explicit sub-ledger values only, not a duplicate physical graph.
- No dedicated tests for construction diagnostic JSON were found in the reviewed file. Add projection tests for aggregate counts/mass, stored-vs-attached counts, no duplicate physical graph in construction JSON, and exact-once child partition after division.
- This is diagnostic output compatibility, not world-save/load migration. No source/test changes; other chemistry transaction tests and whole-world release gates remain open.


## Core-identity refinement — Section 17.80

- **Plan commit:** `e34e52be71ee856f3b2d8be86973d6076aac4c28`.
- Existing `PhysicalConstituentGraph` has stable `PhysicalConstituentId` values and one `genome_constituent_ids` set, populated by juvenile realization from the actual cavity boundary. This is useful geometry-derived identity metadata, not a material/reproductive tag.
- `analyze_genome_cavity` ignores that set for selection and returns the largest qualifying cavity, so a second core still needs core-specific cavity selection. `anchor_structure` currently returns an index in a separate graph and does not establish a second stable core identity.
- Refined direction: one graph with a small collection of core references over shared physical IDs. Preserve parent boundary identity; anchor the second core to the transferred physical ID, then track its realized boundary IDs once a cavity qualifies. No “reproductive material” tag and no second physical graph. Reject dangling references atomically; partition core identity exactly once at division.
- No source/test changes; remaining transaction and whole-world release gates remain open.


## Graph ID-remapping checkpoint — Section 17.81

- **Plan commit:** `da37ba8d5c8b334f6e8e6b6ed4a0aae78ef777e6`.
- `PhysicalConstituentGraph::new` starts IDs at 1 and `add_unit` allocates from a graph-local counter, so parent and separate construction graphs can collide. Bonds refer to stable constituent IDs; naive vector concatenation is unsafe.
- Any future graph merge must stage an injective old→new ID mapping, rewrite both endpoints of every incoming bond, remap second-core anchor/boundary references, validate the candidate, and commit atomically. Existing graph API has no complete atomic merge helper.
- Required tests: colliding ID ranges remap correctly, bond endpoints remain valid, malformed endpoints reject without mutation, parent core IDs stay stable, second-core references remap once, and pure merge preserves structural-energy inventory.
- No source/test changes; this is a candidate future implementation unit only after audit sign-off.


## Construction lifecycle gap — Section 17.82

- **Plan commit:** `af5cb77dab1842e10c699980edbd715af3b4bbda`.
- `advance_construction` creates a temporary Organism, applies its own maintenance, stress/death validation, and resonance-overload BREAK, then copies energy/stress/debt/structure back to construction. This is a separate biological lifecycle for an unfinished core before division.
- Simulation handles `ConstructionStatus::Dead` in the same finish branch as Ready/Detached. `finish_reproduction` can still return an offspring when the structure is nonempty even if readiness is false. “Dead” therefore does not reliably abandon construction; needs deterministic test/semantics.
- Recommended implication: before division, maintenance/stress/death eligibility belongs to the parent aggregate; construction is an internal process, not a second living organism. Parent death stops growth while attached units/material remain physical and ordinary chemistry continues. Do not delete reserve/stress/debt fields until separating accounting sub-holders from duplicate child-organism state.
- Required tests cover no separate pre-split maintenance/death/overload, dead-parent no-growth/finish, no offspring after dead construction without explicit recovery, parent aggregate maintenance once, and conserved parent-owned reserve across waiting/growth/death/division.
- **Open semantic confirmation:** should parent maintenance/stress apply to the entire aggregate during internal budding, with no independent developing-core maintenance/death state? Existing owner rules strongly imply yes, but exact energy/stress allocation needs explicit confirmation before code changes.

## Owner-confirmed aggregate-structure decision — D16 / Section 17.83

- **Plan commit:** `0f66f7b5ae7d15244306afc30529a110493054a3`. PR #204 remains documentation-only.
- Owner explicitly confirmed: reproductive material is not a special designation; committed reproductive structure and physical structure are the same; a separate authoritative developing physical graph should not exist; structurally attached units count for resonance; only stored material is excluded from owner capture.
- Canonical representation: one parent-owned physical graph for all attached units. Construction retains only justified nonphysical core/progress metadata and explicitly defined accounting sub-holders. No physical units should be duplicated or classified by reproductive role.
- Receiver mapping must enumerate the aggregate graph once and credit attached surfaces to the owner's resonance reservoir under the same eligibility/settlement rules as other attached structure. Stored material remains separate and does not credit the owner, though it may occlude/receive onward signal.
- Sections 17.70–17.81 are historical evidence/recommendations to be read through this decision. Section 17.82's observation that current source has a separate graph remains valid; its exact parent maintenance/stress allocation question is still open, but cannot justify restoring a second physical graph.
- No source/test changes. Audit closure remains blocked on the transition/failure matrix, core-specific cavity identity, inventory and diagnostic JSON reconciliation, world-tone exact-small-world tests, cache invalidation/restart, and whole-world conservation.
- **Next:** complete the source/holder and transaction-failure matrices; then update/reconcile the exit checklist and release gates. No implementation until every audit template is complete. Before the first implementation change, refresh PR #205 head and current CI.

## Source/holder matrix reconciliation — Section 17.84

- **Plan commit:** `5cbd1adcd1d109c16359a549bccd7a58b0e9c582`. PR #204 remains documentation-only.
- Section 17.84 applies D16 to current source and classifies the transition rows as locally inspected, confirmed gap, or test-required. The prior owner-decision blocker for attached-structure resonance is closed; implementation/oracle evidence is not.
- Exact PR #205 head reviewed remains `12d8079230458da68a8dc3755a1d6a9b41347862`; no source/test files changed.
- New refinement: movement rejection after geometry succeeds can leave pushed organisms and pushed field physical objects changed as well as the mover. Rollback/test scope includes all pushed entities, cell indexes, spatial index, construction anchors, and progress.
- Targeted source review found no additional production `EnergyLedger.transfer` caller beyond the already-mapped construction-reserve transfer and quarantined legacy recycler. This is not proof of complete material failure-injection coverage.
- **Next:** continue checking the remaining production material-mutation call sites against Section 17.84; then reconcile the phase tracker and release gates. Audit is not closed; do not change source/tests until all audit templates are complete.


## Release-gate reconciliation — D16 included, not yet verified

- Plan commit: `452c26286afe96256adcc8e1ca3b195451e14fa2`.
- Section 17.6 now explicitly tracks D1–D16. D16 is settled and documented, but one aggregate graph, attached-unit resonance mapping, storage exclusion, core-specific cavity identity, inventory consistency, and failure atomicity remain implementation/verification obligations.
- No source/test changes or fresh CI run. Latest recorded PR #205 head remains historical `12d8079230458da68a8dc3755a1d6a9b41347862`; workflow 39833438679 remains the latest recorded verification.
- Next: reconcile Sections 17.84–17.85 with the phase tracker and release checklist, then close each test-required audit template explicitly. Re-fetch PR #205 head and CI before any code change.

## Targeted physical-material mutation audit — Section 17.85

- **Plan commit:** `e6889035c149e089c1d4fc289f88992edc832f87`.
- Targeted source inspection covered material storage/transfer, field containment and deposits, restoration, COMBINE/BREAK, expulsion, movement, construction/reproduction, and legacy recycling at PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`.
- No new production internal-energy-transfer caller surfaced beyond construction reserve and the quarantined recycler already in Section 17.56. This is a bounded call-site audit, not passing test evidence.
- Confirmed physical-material failure gaps: contained acquisition can lose a returned instance if storage and fallback deposit both reject; stored BREAK commits split pieces incrementally; expulsion can lose the removed source if destination/restoration rejects; environmental COMBINE exact-cell restoration and prepared-ledger rollback need malformed-state injection; the legacy recycler drains before deposits/transfers and stays quarantined.
- Movement failure rollback includes the mover, pushed organisms, pushed field objects, indexes, anchors, and progress.
- **Design clarification, plan commit `036ee59a7ede00938d04c55cf4b493986edadeef` (tracker reconciled at `56eb760a40bde64286a0bb33d946522e06b83c98`):** remove `ReproductiveConstruction.developing_structure` from persistent/current construction state. Retain only nonphysical core/progress metadata and justified accounting sub-holders. A transient trial graph is acceptable inside an atomic candidate calculation, but not as a second owned physical structure.
- **Next:** close each test-required audit template explicitly, beginning with the material-loss rejection tests and movement rollback; then proceed through COMBINE/BREAK, cache, world-tone, and whole-world inventory gates. No source/test changes; implementation remains gated.


## Resume checkpoint — movement transaction audit (2026-10-22)

- **PR #204 plan commit:** `abf552980984a64c689d0decc86fca9e6c611442` (Section 17.86); PR #204 remains documentation-only.
- **PR #205 exact head checked:** `12d8079230458da68a8dc3755a1d6a9b41347862`. Exact-head workflow `39833438679` is confirmed completed/failure. Formatting and focused gates passed; Rust tests and Clippy failed. Prior 585-pass/27-baseline-failure/1-ignored comparison and normalized Clippy comparison remain historical evidence from that workflow, not a fresh re-comparison here.
- **Section 17.86 finding:** movement feasibility is planned, but movement is physically committed before the movement energy transaction is settled. The path can translate pushed organisms, remove/translate/reinsert pushed physical field objects, move the active organism, update position/cache state and spatial indexing; if ledger settlement then rejects, none of those world changes are rolled back and movement progress is not decremented. A deterministic rejection test must verify the entire affected world/inventory remains unchanged.
- `apply_push_plan` reinserts pushed physical material only when the translated first placement maps to a field index; no fallback exists. `can_translate_physical` may guarantee this for valid inputs, so treat as a targeted invariant to prove, not as confirmed material loss.
- D16 still conflicts with the current movement path's separate translation of `ReproductiveConstruction.developing_structure`. Canonical future movement must cover the one authoritative parent-owned graph once, with core-specific identity and cache/spatial updates; do not patch movement in isolation.
- **No source/test changes.** Audit templates remain open. Next work is to finish deterministic failure-injection rows for acquisition/expulsion/stored BREAK, COMBINE rollback, construction reserve transfer rejection, stale reaction keys, cache catalog/restart, and post-death aggregate inventory; reconcile against Section 17.6/phase tracker. Only after all templates close should a minimal implementation unit be selected. Refresh PR #205 head and CI before the first source/test change.


## Resume checkpoint — acquisition/expulsion failure paths (2026-10-22)

- **PR #204 plan commit:** `5ff519736755e6769267b1240792665a508861b3` (Section 17.87). PR #204 remains documentation-only.
- **PR #205 audited head:** `12d8079230458da68a8dc3755a1d6a9b41347862`; workflow `39833438679` confirmed failed as recorded in the preceding checkpoint. No source/test changes.
- Contained acquisition removes each contained field object before storage. If storage rejects, fallback deposit is conditional on a first placement and its boolean result is ignored. If storage and fallback both reject—or no placement exists—the original source has no rollback path.
- Expulsion removes a stored object first. Multiple validation-failure branches attempt storage restoration but ignore the result; final field insertion rejection returns false without restoring storage. Exact instance/geometry/endpoints must be conserved, not just aggregate totals.
- Required deterministic tests: storage reject + fallback success; storage + fallback rejection; missing/malformed placement; expulsion validation rejection with restoration success/failure; final destination rejection. Assert exact holder membership, physical instance and connection identity, cell/spatial indexes, and independent material inventory.
- **Next:** audit stored BREAK escrow/partial-split and environmental COMBINE restoration/ledger rollback, then remaining construction/cache/reaction/post-death rows. Audit templates still open; do not change source/tests until all are complete. Refresh PR #205 head/CI before implementation.


## Resume checkpoint — BREAK escrow and COMBINE rollback (2026-10-22)

- **PR #204 plan commit:** `0f7c3c2ed01372b44dc16792d1af334aa453c408` (Section 17.88). PR #204 remains documentation-only.
- **PR #205 head inspected:** `12d8079230458da68a8dc3755a1d6a9b41347862`. No source/test changes.
- BREAK source confirmed: the intact stored/environmental object is removed into transformation escrow at start. Malformed completion (missing stored source, target bond, prepared energy, or failed split) clears the active marker without restoring escrow. Successful splitting deposits/stores pieces one by one; storage/deposit failures are ignored, so partial commits can lose pieces. Required: escrow remains authoritative until every output is ready and the complete split commits atomically; failure-inject malformed completion and each piece destination, asserting exact material/energy/ledger/marker recovery.
- Environmental COMBINE uses a stronger trial/stage/commit pattern, but rollback omits `total_world_tone_energy_escaped` and `total_resonant_energy_held` despite those fields being present in the prepared ledger delta. Current COMBINE should leave them zero; test that invariant or centralize complete delta reversal. Failed resolution also silently cannot restore the source if its original field-cell index is absent.
- **Next:** audit construction reserve-transfer rejection and stale bondless reaction-key cleanup, then cache catalog/restart and post-death aggregate inventory. All audit templates remain open; no implementation until closure and no source/test changes made.


## Resume checkpoint — construction transfer and reaction keys (2026-10-22)

- **PR #204 plan commit:** `aefdda3c247732f05bf2d71d460af8806d3e10dc` (Section 17.89). PR #204 remains documentation-only.
- **PR #205 exact source head:** `12d8079230458da68a8dc3755a1d6a9b41347862`. No source/test changes.
- `advance_construction` ignores the result of `EnergyLedger::transfer` and continues temporary-child maintenance/construction. Test whether reserve funding is required; if so, rejection must stop dependent work with both holders and ledger unchanged. D16 means this cannot be fixed alone: the current temporary child has a separate physical graph and lifecycle, contrary to the one parent-owned attached graph. Aggregate maintenance/stress/energy allocation remains to be designed.
- `chemical_reaction::accumulate` returns on a bondless graph before removing stale revision-prefixed accumulation keys. These are runtime activation values, not physical energy, but cleanup and bondless/reformed-graph regression tests are missing.
- **Next:** audit catalog fingerprint/restart and post-death aggregate inventory, then world-tone exact-small-world oracle and release gates. Audit templates remain open; no source/test changes until closure.


## Resume checkpoint — cache invalidation and aggregate receiver (2026-10-22)

- **PR #204 plan commit:** `118f4915c6c3a5b9562c88975df169ad9835220b` (Section 17.90). PR #204 remains documentation-only.
- **PR #205 exact head:** `12d8079230458da68a8dc3755a1d6a9b41347862`. No source/test changes.
- Cache schema 4 checks a manually versioned equation/catalog fingerprint. Existing mismatch test proves generic rejection, but there is no test proving a chemical-position change requires a fingerprint bump and invalidates/rejects old persisted records across reopen. Add catalog-dependency and restart tests; consider deterministic fingerprinting of canonical chemistry-relevant catalog fields.
- The independent energy oracle and world-tone receiver collector still enumerate `developing_structure` separately from the parent's graph. D16 requires one parent-owned attached graph, one receiver enumeration for all attached units, stored-material capture excluded, and inventory/diagnostics/movement/cavity identity updated together. Do not patch the sunlight receiver alone.
- Death remains ordinary chemistry on the parent-owned physical aggregate; no death-time ownership transfer or special decomposition object. A bounded-live-tick regression is still needed for a breached parent with non-empty storage and active construction, using independent material and energy-holder snapshots. The legacy decomposition path remains quarantined.
- **Next:** close world-tone exact-small-world/performance oracle and whole-world material/energy accounting, reconcile the exit checklist, and enumerate remaining gates. No source/test changes until all audit templates close.


## Resume checkpoint — world-tone oracle/performance (2026-10-22)

- **PR #204 plan commit:** `06ae53673430a20a7f0e7bdce9c9ff57eab173e9` (Section 17.91). PR #204 remains documentation-only.
- **PR #205 audited head:** `12d8079230458da68a8dc3755a1d6a9b41347862`. No source/test changes.
- Current world-tone path stages reservoir updates and validates path-width consistency plus incident = retained + heat + escaped. Existing tests cover analytic individual shapes, partial/full shadows, receiver allocation, and local conservation. No independent exhaustive small-world oracle for the full path/receiver pipeline or current reproducible thousand-body performance benchmark was found in the reviewed tests.
- Required gate: deterministic tiny worlds with independent geometric/exhaustive lane oracle, attached-vs-stored receiver cases, downstream frequency-shifted propagation, and independently recomputed energy totals; plus a bounded reproducible scale benchmark across increasing object counts. Historical 272-second geometry test is not fresh performance evidence.
- D16 must be implemented across the authoritative graph and receiver map, not patched into sunlight alone. **Next:** finish whole-world material/energy proof and reconcile all audit rows with Section 17.6/phase tracker; then refresh exact PR #205 head/CI before implementation.


## Consolidated handoff — exit checklist reconciliation (2026-10-22)

- **PR #204 plan commit:** `be8801a5a9e5e449530781e2e82a51b64db5bcc6` (Section 17.92; Phase 1–10 tracker rows reconciled). PR #204 remains documentation-only.
- **PR #205 exact source head audited:** `12d8079230458da68a8dc3755a1d6a9b41347862`. Latest recorded exact-head workflow `39833438679` is failed: formatting/focused gates passed, full suite and Clippy failed. The earlier 585/27/1 and normalized Clippy comparison are historical, not fresh verification.
- **Audit work completed this resume:** Sections 17.86–17.91 record movement transaction ordering, acquisition/expulsion failure paths, stored BREAK escrow and COMBINE rollback, construction reserve transfer and stale reaction keys, cache catalog/restart and post-death aggregate receiver gaps, and world-tone exact-small-world/performance gates. All are documentation/source-audit results only; no source or test files changed.
- **Current blocker:** audit templates are not yet complete, and no implementation changes are authorized. The source/holder and failure rows are mapped, but D16 aggregate/core-identity/diagnostic contracts, parent maintenance/stress/reserve semantics, deterministic rejection tests, cache restart, independent world-tone oracle/performance, and global conservation/long-run proof remain open.
- **Next:** finish the aggregate/core-identity/diagnostic acceptance matrix and any remaining owner semantic gaps; classify every failure row; confirm no uninspected production mutation path remains; reconcile independent inventories and release gates. Only then select a minimal implementation unit and refresh PR #205 head/CI immediately before coding. Do not merge or claim chemistry unified.


## Resume checkpoint — aggregate/core identity/diagnostics (2026-10-22)

- **PR #204 plan commit:** `705c9af7322c37278d62094086923251fdcdda52` (Section 17.93). PR #204 remains documentation-only.
- **PR #205 exact source head audited:** `12d8079230458da68a8dc3755a1d6a9b41347862`. No source/test changes.
- `OrganismStructure` has stable physical IDs but graph-local allocation; merging parent and construction graphs requires an injective ID map and bond endpoint/core-reference remapping. `analyze_genome_cavity` selects the largest qualifying cavity without core identity, so parent perception could switch as a second cavity grows in the aggregate. Required tests cover ID collisions, invalid endpoints/no-mutation, stable parent IDs, second-core identity, independent cavity queries, exact-once partition, and energy/material invariants.
- Diagnostics currently count/serialize only the primary graph and separately serialize the full construction graph. After unification, aggregate structure metrics must include attached units once; construction JSON must contain only nonphysical core/progress metadata and justified accounting sub-holders. No whole-world persistence path was found, so this is diagnostic JSON compatibility, not a new save migration requirement.
- **Semantic contract still open:** define necessary reserve/stress/debt sub-holders, whether maintenance is charged once against the aggregate, and how stress/death stops growth without removing attached physical units or yielding an offspring from dead construction. D16 rejects a second physical graph and separate pre-division organism lifecycle but does not assign every energy/stress equation by itself.
- **Next:** finalize aggregate accounting semantics, reconcile acceptance matrix with Sections 17.6/17.7, and verify no production mutation path remains uninspected. No source/test changes until all audit templates close; refresh PR #205 head/CI before implementation.


## Resume checkpoint — construction lifecycle / aggregate accounting (2026-10-22)

- **Latest PR #204 plan commit:** `bd7e7407722bf7f5a39c565b6096d2137e509ec3` (Section 17.94); handoff update follows. PR #204 remains documentation-only.
- **PR #205 exact source head reviewed:** `12d8079230458da68a8dc3755a1d6a9b41347862`; this is still the last recorded source head and must be refreshed before implementation. No source/test changes.
- Re-read `src/organism/reproduction.rs`, `src/simulation/state.rs`, `src/simulation/simulation.rs`, and `src/infrastructure/diagnostics.rs`. Current `ReproductiveConstruction` still owns a second physical `developing_structure`; a temporary full `Organism` receives separate maintenance, stress/death validation, and resonance-overload BREAK. The tick treats `ConstructionStatus::Dead` as finishable alongside Ready/Detached, and `finish_reproduction` does not itself enforce readiness. This conflicts with D16's one parent-owned aggregate and creates a path where construction-local death can still yield a child.
- **Proposed minimum accounting contract, not yet owner-confirmed for reserve/debt details:** one physical graph and one biological lifecycle until division; parent maintenance/stress/death runs once; parent death stops construction and forbids offspring but leaves attached units available to ordinary chemistry; stored material is resonance-ineligible; a construction reserve may remain only as a justified nonphysical energy sub-holder with explicit accounting, never a second stress/death lifecycle. See plan Section 17.94 for source findings and acceptance assertions.
- Reserve transfer rejection is ignored by `advance_construction`; do not fix it in isolation before the duplicate graph/lifecycle design is resolved. Required tests include exactly-once maintenance, no pre-split child lifecycle, no child after parent/construction death, atomic reserve rejection, post-breach aggregate chemistry, attached-vs-stored receiver eligibility, and exact-once division/inventory.
- **Next:** finish remaining production mutation-path and failure-row inventory; reconcile all audit templates with Section 17.6/17.7; only after closure refresh PR #205 head/CI and choose one smallest audited implementation unit. No source/test changes before audit closure. Historical workflow `39833438679` is not fresh verification.
