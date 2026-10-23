# Chemistry Unification — Current Handoff

**Plan (latest recorded plan commit):** `aac71feadbd70926d63ef59a44b134c34494db6e`  
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


## Resume checkpoint — core identity / diagnostic test coverage (2026-10-22)

- **Latest PR #204 plan commit:** `66e293b8f0b5e66d1e052ed4616846ccd0c41ad2` (Section 17.95); handoff update follows. PR #204 remains documentation-only.
- **PR #205 exact source head inspected:** `12d8079230458da68a8dc3755a1d6a9b41347862`; refresh before implementation. No source/test changes.
- `analyze_genome_cavity` chooses the largest qualifying cavity across connected components and accepts no requested core identity. The cache then stores only that selected cavity's boundary IDs in the graph's single `genome_constituent_ids` set. This cannot support independently stable parent/second-core identities after aggregate graph unification.
- Graph-local IDs begin at 1; deserialization repairs duplicate IDs inside one graph but is not a safe graph-merge procedure. Aggregate merge must stage ID remapping, rewrite bond endpoints and core references, validate, then commit atomically.
- Existing cavity tests cover one qualifying cavity and an unbonded structure; structure tests cover per-graph IDs and ID serialization. No reviewed test covers two qualifying cavities in one connected aggregate, stable parent selection as a second cavity grows, merge collisions, exact-once division, or diagnostic projection. `diagnostics.rs` reports primary-graph counts/mass/JSON and separately serializes construction state; the inspected file has no local tests.
- **Next:** finish and classify the remaining mutation/failure rows; reconcile the full audit matrix against Sections 17.6/17.7. Test gates remain open, and no source/test changes are allowed until every audit template closes.


## Resume checkpoint — attached-structure resonance decision reconciled (2026-10-22)

- **Plan commit:** `0b071c8a47c649dd55efdfef20563bd252a51dc8` (Section 17.96). PR #204 remains documentation-only.
- **PR #205 audited source head:** `12d8079230458da68a8dc3755a1d6a9b41347862`. This is the latest head found during this resume; refresh head and CI before any source/test change.
- **Decision closed:** any structurally attached unit is ordinary organism structure and counts toward the living organism's single resonance reservoir. No reproductive-material tag and no persistent developing-structure category. Material still in storage is excluded from resonance. This supersedes the old open choice in Sections 17.37–17.38; those sections remain historical evidence of the then-current implementation.
- **Implementation mismatch:** current world-tone mapping credits only primary-structure bodies. Attached committed/developing structure must map through the eventual shared aggregate graph once; stored material remains capture-ineligible. Preserve escrow/source de-duplication, first-hit occlusion, staged settlement, and world-tone balance.
- **No source/test changes.** This was a documentation and decision-reconciliation chunk only.
- **Next:** continue the remaining production mutation-path/failure-row inventory, classify each row (source-inspection-complete, test-required, owner-decision-blocked, or release-gate-open), and reconcile independent inventories/release gates. No code until audit closure; one focused audit before each change.
- **Verification caveat:** workflow 39833438679 is historical exact-head evidence, not a fresh run. It reported 585 passed, 27 baseline-identity failures, 1 ignored; Clippy had 34 application and 15 test diagnostics with no normalized identity changes. Do not describe the suite as green.


## Resume checkpoint — prepared COMBINE rollback matrix (Section 17.97)

- Plan commit: `e6635583b9c4d6062ac33415ec588241aecd6ff5`; PR #204 remains documentation-only.
- Exact PR #205 source head inspected: `12d8079230458da68a8dc3755a1d6a9b41347862`.
- The COMBINE rollback helper omits world-tone escaped and resonant-held fields, but the current `settle_transaction` API does not mutate those fields; the gap is missing explicit zero-delta/preservation assertions, not yet a proven live accounting defect.
- Existing test `failed_prepared_combine_rolls_back_energy_and_every_ledger_delta` does not assert all ledger fields. A future focused test should initialize nonzero sentinel values and prove unmodified fields survive prepare/rollback, while proving every generated delta belongs to the transaction's mutation set.
- No source/test changes. Continue failure-row coverage and audit closure; do not code until templates close. Refresh PR #205 head and CI immediately before the first implementation change.


## Resume checkpoint — movement/acquisition/expulsion coverage (Section 17.98)

- Plan commit: `50744dc7e8e4fc04faed0cdfa95e855e4a376319`; PR #204 remains documentation-only.
- Exact PR #205 head reviewed: `12d8079230458da68a8dc3755a1d6a9b41347862`.
- Movement already has geometric push-chain atomicity tests, but not energy-ledger rejection after geometry succeeds. Containment has a successful transfer regression, but not combined storage/fallback rejection. Expulsion has no local tests and does not restore storage after final deposit rejection.
- These are test-required gates after source inspection; no source/test changes made. Continue with stored BREAK/environmental COMBINE failure coverage and reconcile the full matrix before implementation. Refresh exact head/CI before code changes.


## Resume checkpoint — BREAK/COMBINE failure coverage (Section 17.99)

- Plan commit: `0b6754f0c209e91230dfa136e1eb6b9f5bce1677`; PR #204 remains documentation-only.
- Exact PR #205 head reviewed: `12d8079230458da68a8dc3755a1d6a9b41347862`.
- Stored BREAK normal-path and one restore-helper test exist, but malformed resolver state and partial multi-piece destination failure remain uncovered. Environmental COMBINE has helper/live-candidate rollback tests, but no end-to-end exact-cell source-restoration test for malformed prepared state.
- Required assertions: exact material/source ownership, no partial ledger/energy/stress mutation, no duplicate restore, and unchanged field revision/index state on rejection. No source/test changes.
- Next: reconcile Sections 17.86–17.99 into the authoritative failure matrix and release checklist before any implementation.


## Resume checkpoint — authoritative failure matrix (Section 17.100)

- Plan commit: `dcd483fe83150324adb627d33f4c15164d945cd4`; PR #204 remains documentation-only.
- Sections 17.86–17.99 are consolidated into a current matrix distinguishing inspected source paths, existing normal/helper test evidence, test-required gaps, design/implementation gates, and release gates.
- Attached-structure resonance eligibility is settled by Section 17.96. Remaining design gap is the minimal reserve/debt/maintenance contract consistent with one aggregate and one biological lifecycle.
- No source/test changes and no fresh CI run. PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862` remains the last inspected head; refresh before any implementation.
- Next: finish reserve/debt/maintenance accounting design, perform a final production mutation/energy call-site sweep, reconcile inventory coverage, then refresh CI before coding. Do not claim global source audit or chemistry unification complete.


## Resume checkpoint — construction reserve/maintenance semantics (Section 17.101)

- Plan commit: `4b2d7175f652defe8f760e5d549f3ffc3635c6db`; PR #204 remains documentation-only.
- Exact PR #205 head inspected: `12d8079230458da68a8dc3755a1d6a9b41347862`.
- Proposed target: one parent maintenance/stress/death lifecycle based on aggregate structural mass; no temporary child organism or separate developing stress/debt. Construction progress/core identity remain metadata. A construction reserve may remain only as an explicitly accounted parent-owned sub-holder.
- **Open semantic item:** whether to retain an earmarked construction reserve and define its spend/release rule. Existing transfer is internal, but its failure result is ignored before construction proceeds. This is not being silently changed or treated as owner-approved.
- Required tests and the no-code-until-audit-closure gate are recorded in Section 17.101. No source/test changes or fresh CI run.


## Resume checkpoint — bounded energy/material call-site sweep (Section 17.102)

- Plan commit: `5f5a3719be029fd84ec859b521a1c9e7124772fb`; PR #204 remains documentation-only.
- Bounded exact-head review of high-risk chemistry/material/energy/movement/simulation/reproduction/world-tone modules found no additional live internal-energy-transfer API caller beyond construction reserve and quarantined legacy recycling.
- Ignored deposit returns in initial landscape/resource-cloud seeding were noted; these are deterministic initialization paths using realized material and validated field indices, not new live chemistry energy paths.
- This is not an exhaustive global source audit. Known rollback/ownership gaps remain open. No source/test changes or fresh CI run.
- Next: settle the construction reserve semantics, reconcile whole-world material/energy inventories, then refresh PR #205 head/CI before implementation.


## Resume checkpoint — parent-funded production costs (Section 17.103)

- Plan commit: `d2250406cebf1ef40c558608e8ec2139be7d9d94`; PR #204 remains documentation-only.
- **Owner decision settled:** all production costs draw directly from the parent's ordinary `usable_energy`. Remove the separate `reproductive_energy_allocation`/ `developing_energy` reserve path; do not invent a new cost equation.
- Construction remains metadata over one aggregate. No temporary child lifecycle. Parent maintenance/stress/death runs once; debit plus construction mutation must be atomic. Parent death stops production and prevents offspring; division partitions ownership exactly once.
- PR #205 head last inspected: `12d8079230458da68a8dc3755a1d6a9b41347862`; refresh before implementation. No source/test changes or fresh CI run.
- Next: audit production cost formulas and division transaction boundaries, then reconcile tests/inventory gates before implementation.


## Resume checkpoint — production cost/division audit (Section 17.104)

- Plan commit: `c7f93fd52cf1b09417e503865b269205c779726d`; PR #204 remains documentation-only.
- Owner decision remains: production costs are debited from parent `usable_energy`; no separate `developing_energy` reserve. Clarification: do not delete the genome field `reproductive_energy_allocation` or its compatibility alias until its role in the Master Spec is checked. It must not be a separate energy holder.
- Exact PR #205 head inspected: `12d8079230458da68a8dc3755a1d6a9b41347862`. Current construction uses that field as a reserve target, ignores transfer rejection, applies a second child lifecycle, and division creates a child with the developing holder's energy. Simulation may finish `Dead` construction; helper lacks Ready-only guard.
- Candidate attachment already uses the bond-driven COMBINE energy-accounting path, but shared-aggregate atomic commit remains unimplemented. Constructor's `1.0e12` trial allowance remains separate and out of scope.
- Narrow item to resolve from Master Spec: child usable energy after successful division. Do not invent a 50/50 split or reintroduce a production reserve. No source/test changes or fresh CI run.
- Next: inspect Master Spec 2.4 and existing division rule; then complete division failure matrix and inventory reconciliation.


## Resume checkpoint — division transaction boundary (Section 17.105)

- Plan commit: `3972ef96e66793f5ea9037d899d7e158c2d67274`; PR #204 remains documentation-only.
- README confirms the 80/80 target (roughly 160% aggregate, then two ~80% organisms), but its older separate-developing-graph prose conflicts with the owner's later decisions and must be reconciled. The Master Spec file itself was not found in the audited repository tree.
- Current simulation may finish `Dead` construction; `finish_reproduction` can return a child from a nonempty but not-ready structure. Both conflict with the owner rule that dead/failed construction cannot create offspring.
- Child currently receives `construction.developing_energy`; no separate holder is allowed. Production costs debit parent usable energy; remaining energy at division must partition exactly once, but no authoritative numeric split rule was found in the inspected README. Do not invent a 50/50 rule.
- Division tests still needed: exact-once units/bonds/storage/energy, collision-safe IDs, cross-partition bond settlement, rollback on either-child validation failure, and no child from non-Ready states.
- No source/test changes or fresh CI run. Next: check repository history for an established child-energy allocation rule; if absent, ask the owner one narrow question before implementing division.


## Resume checkpoint — proportional division energy (Section 17.107, 2026-10-22)

- **Owner choice B recorded:** at successful division, proportionally split the parent's remaining ordinary `usable_energy` between the two resulting organisms. All production costs remain debited directly from parent `usable_energy`; no separate developing-energy holder.
- The proportional denominator remains open. Repository documentation supports the physical 160%-aggregate → two ~80%-organism goal but does not define an energy ratio. Candidate for owner confirmation: allocate energy by each validated child's post-split structural mass. Do not assume this candidate is approved or implement arbitrary 50/50.
- Division stays atomic and exact-once; dead/non-Ready construction cannot create offspring. No source/test changes. Implementation gate: settle the proportional basis, then refresh exact PR #205 head and CI before first source change.


## Resume checkpoint — structural-mass proportional division (Section 17.108, 2026-10-22)

- **Owner-approved rule:** on successful division, split the parent's remaining ordinary `usable_energy` in proportion to each validated resulting organism's post-split structural mass. For valid masses `m1`, `m2` and remaining energy `E`, use `E1 = E * m1 / (m1 + m2)`, `E2 = E - E1` (or the symmetric calculation with deterministic residue assignment), after validating finite positive masses and valid finite energy. The two allocations must sum exactly to `E` within the accounting representation.
- All production costs debit the parent's `usable_energy` atomically with structural mutations. No separate developing-energy holder/reserve. Only a successful atomic division creates resulting organisms; dead/non-Ready construction cannot produce offspring.
- Do not implement the split formula against the old separate-child graph in isolation. It belongs inside the canonical aggregate/two-organism atomic partition, with exact-once material/bond/storage/energy ownership and rollback.
- Documentation update commit: `85df557c9e33f9dfd5c381d6e5c68d47fe3b6c08`. PR #204 remains docs-only. Next: refresh PR #205 head and audit its current reproduction/division/ledger/test boundary before the next source change; commands/tests ≤5 minutes, compare failure identities against baseline.


## Resume checkpoint — fresh reproduction transaction audit (Section 17.109, 2026-10-22)

- Rechecked exact PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`; no source changes or tests run.
- Confirmed current code still moves energy from parent `usable_energy` into `developing_energy`, runs a temporary child's separate maintenance/stress/death/resonance lifecycle, and funds attachment trials from that temporary child's energy even when material comes from parent storage. Candidate structure/ledger/energy are prepared on clones, but parent material removal and the later construction/ledger commit are not one canonical aggregate transaction.
- `finish_reproduction` can return a child from nonempty non-Ready construction, and the simulation routes `Dead` status toward finishing. Child receives the prohibited developing-energy holder. The approved structural-mass proportional division rule belongs after canonical aggregate partition into two validated outputs; it must not be patched into this old graph independently.
- Next coherent implementation slice: one aggregate ownership model plus parent-energy debit/physical attachment atomicity, with failure-injection tests. Division is a separate later atomic transaction; child usable-energy shares follow validated post-split structural mass. PR #196 stays out of scope. Full suite not rerun because last full-suite process hit the hard five-minute bound; compare named failures to baseline after future source change.


## Resume checkpoint — canonical structure merge prerequisite (Section 17.110, 2026-10-22)

- Audited `src/geometry/structure.rs` on PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`. No atomic graph merge helper exists. Physical IDs are allocated per graph from 1; bonds refer to IDs directly and retain `bond_energy`; genome-core constituent IDs also require remapping. A naive append would risk collisions or broken bond/core identity.
- A safe merge needs an old→new physical-ID map, rewritten bond endpoints, preserved bond energy and genome IDs, geometry/connection validation, and one atomic commit with parent-energy ledger settlement and source material ownership. Current anchor placement may overlap an existing parent unit, so the anchor cannot simply be appended as another physical copy.
- No source/test changes. Next audit: inspect COMBINE prepare/commit abstractions for a reusable atomic graph+ledger boundary; choose a live caller before introducing a helper so it is tested and used immediately. PR #204 remains the handoff authority; PR #196 remains separate.


## Resume checkpoint — COMBINE transaction reuse audit (Section 17.111, 2026-10-22)

- Existing live COMBINE in `src/chemistry/transformation.rs` uses cloned candidate organism/environment/ledger state, then stores prepared structure, storage, energy/stress and ledger deltas. Resolution commits physical state; missing prepared state triggers rollback and exact-cell source restoration.
- This is a pattern, not a drop-in API: rollback manually reverses selected ledger fields and omits some fields (including world-tone escaped and resonant-held counters), and the transaction does not cover reproduction's parent structure/core-ID mutation plus parent energy/material ownership. Do not reuse it unchanged.
- Next safe step: define the exact parent-funded aggregate-attachment mutation set and implement one transaction boundary that commits/rolls back structure/bonds/core IDs, source storage, usable energy, ledger fields, and revision/cache invalidation together. Then add failure-injection tests before migrating more call sites. No source/test changes; exact PR #205 head remains `12d8079230458da68a8dc3755a1d6a9b41347862`.

## Latest transaction mutation-set audit — 2026-10-22, Section 17.112

- PR #205 branch reviewed: `chemistry-unification-implementation-2026-10-22`; last recorded source head remains `12d8079230458da68a8dc3755a1d6a9b41347862`. No production or test files changed during this audit.
- Enumerated the 15 `EnergyLedger` fields and the eight numeric fields currently reversed by `rollback_prepared_combine`. Fields not reversed (world-tone escaped, retained-resonance values, initial baselines, initial-condition guard) are not mutated by the reviewed `EnergyTransaction` settlement path; their absence from reversal is not a proven bug. Add nonzero-sentinel preservation assertions rather than reverse unrelated fields.
- COMBINE commits energy/ledger and reserves a source before prepared physical state is committed. On malformed prepared state, rollback must preserve exact source cell/index state, usable energy, all affected ledger fields, and active marker. Success commits structure/storage/stress and invalidates structure caches.
- Reproduction requires a larger atomic set: parent usable energy, aggregate graph/bonds/core IDs, stored material, construction/core metadata, ledger, structure revision/cache invalidation, and source ownership. Division later partitions all physical units/bonds/core refs/storage and remaining energy exactly once.
- Current source still represents construction as a separate graph and creates a temporary organism for its lifecycle. That cannot be repaired by a standalone energy formula or graph-merge helper. The next coherent change must have a live caller and failure-injection tests; no unused abstraction and no speculative ledger-field reversal.
- No tests were run. Next step: one focused audit of the narrowest viable live construction-attachment transaction, then implement it with focused rejection/rollback tests within the five-minute command cap.

## Latest implementation checkpoint — 2026-10-22, Section 17.113

- Latest PR #205 head: `c57926fdf551472cfa274ca1f3550d1a93b29295` on `chemistry-unification-implementation-2026-10-22`.
- One audited production change made: `finish_reproduction` now refuses to return a child unless `birth_ready_with_reference` validates readiness, restoring non-ready construction unchanged to the parent. Added focused regression `non_ready_construction_cannot_create_an_offspring`.
- This is a narrow fail-closed fix only. It does not implement canonical aggregate ownership, parent-funded production, dead-parent gating, or atomic two-child division. No energy split formula or reserve holder was introduced.
- GitHub Actions has not yet returned a workflow run for this commit in the available check; verification remains pending. No full suite was run manually.
- Latest plan commit before this checkpoint: `634f17d5fb96ddf55f3b342d7ff83bcd14a87e98`; handoff before this checkpoint: `a9c2fbcae2d9fd64145b6dba407b46b03e5d0f0d`.
- Next: verify the focused regression and formatting on exact head. Then continue one focused audit of candidate geometry/parent-energy/source-removal mutation ordering for a real aggregate attachment transaction; do not patch the old developing-energy flow in isolation.

## Construction attachment transaction audit — 2026-10-22, Section 17.114

- Audited the live construction path after the fail-closed guard. `try_child_construction` asks the attachment solver to charge `child.usable_energy` even for parent-storage candidates; the caller removes the parent-held source afterward, then commits candidate ledger and developing graph separately. Parent ordinary `usable_energy` is not the production-cost source for that attachment.
- A scalar-only switch to parent energy is unsafe: source removal and aggregate structure mutation are separate commits, and the candidate graph remains separate from the parent. It would violate both atomicity and the approved aggregate/resonance/lifecycle model.
- Next code slice must provide a live parent-owned aggregate transaction: validate candidate placement and graph/core-ID remapping, stage parent energy and ledger, stage source removal, then commit structure/storage/energy/ledger/revision together or leave all unchanged. Remove the separate developing-energy transfer and temporary-organism lifecycle within that coherent change.
- No source/test change from this audit; no additional owner decision required. Workflow `39856976428` for head `c57926fdf551472cfa274ca1f3550d1a93b29295` remained in progress at last check.


## Research-reference update — 2026-10-22

- **Latest plan commit:** `aac71feadbd70926d63ef59a44b134c34494db6e`. PR #204 remains documentation-only.
- Section 18 of `docs/chemistry-audit-unification-plan.md` now contains an annotated research bibliography and crosswalk: reaction-network conservation/thermodynamic soundness, nonequilibrium reaction-network thermodynamics, RAF/autocatalytic sets, Smoldyn, MCell/CellBlender, and artificial chemistry/Stringmol.
- Research annotations were added at the applicable core audit and implementation-plan sections. They identify potential audit/test methods and explicit limits; they do not change owner decisions or assert that recommended checks are implemented.
- Main implications: separate reaction-rule invariants from atomic runtime settlement and whole-world inventory; check energy cycles independently of material conservation; use exact-small-world oracles before spatial optimization; treat autocatalysis/emergence as later validation after physics/accounting gates pass.
- **Current PR #205 metadata was re-fetched:** head `c57926fdf551472cfa274ca1f3550d1a93b29295`. This metadata refresh is not a fresh CI verification. Continue to distinguish verified source state from historical workflow evidence, and keep PR #204 information-only.


## Latest aggregate-attachment feasibility audit — 2026-10-22, Section 17.115

- PR #204 plan update adds Section 17.115 after a focused audit of exact PR #205 head `c57926fdf551472cfa274ca1f3550d1a93b29295`. No implementation source or tests changed during the audit.
- The current split graph is enumerated separately by world-tone and energy inventory; its temporary organism has a separate energy/maintenance/stress lifecycle. The anchor begins in the parent's realized region while the parent's primary graph remains unchanged, so a naive graph merge would duplicate overlapping physical constituents.
- A safe canonical aggregate requires explicit second-core membership/identity in the parent graph, plus an atomic live growth transaction owning geometry, source material, parent usable-energy debit, ledger, structure revision/cache invalidation, and exact rollback. World-tone and independent inventory paths must move with that change.
- No scalar energy switch or unused merge helper is safe. Division and structural-mass energy partition remain downstream; no developing-energy holder or temporary-organism lifecycle should remain as a parallel model.
- **Next:** design the smallest second-core membership representation and its first live growth transaction with focused failure-injection and independent-inventory tests. Keep PR #196 out of scope; stay within the five-minute command/test cap.


## Aggregate-core representation slice — 2026-10-22, Section 17.116

- PR #205 head `2f8ae39a9b065121a174c07e60a876ac7dae59e5` adds serializable additional genome-core ID groups to the physical graph without duplicating physical units; legacy primary genome-ID serialization is retained. Added tests cover round-trip, duplicate/cross-group ID filtering, deletion cleanup, and per-core recognition.
- This is representation groundwork only. The reproductive lifecycle still stores a separate developing graph, committed material, developing energy, and temporary maintenance/stress state.
- New blocker: reproduction anchor placement currently accepts overlap with a parent unit, then instantiates the anchor in a separate graph. A blind aggregate merge could duplicate overlapping matter. Audit whether the anchor should promote/reuse a constituent or whether placement/contact semantics need a validated alternative before implementation.
- CI run `39861074268` passed formatting and structural checks at last observation; test stages were still in progress. No test-success claim yet.
- Movement remains deferred until the live construction graph is moved into canonical parent-owned structure. Then translation, collision/space checks, world-tone body enumeration, and revisions need one coordinated refactor. Notify user before starting that phase.
- Next: focused anchor placement/contact audit, then atomic parent-funded growth transaction. PR #196 remains out of scope.



## Core-specific cavity/cache audit — 2026-10-22, Section 17.117

- `Organism::genome_cavity_cached_ref` currently scans the whole structure and writes the selected cavity boundary back as the primary genome IDs. That becomes unsafe with multiple cores in one aggregate: it may select the developing core and remove its IDs from the additional-core group.
- Before aggregate migration, parent and developing genome cavity/viability checks must be core-anchored and retain separate genome/blueprint associations, even though physical units share one graph.
- Next: test core-anchored cavity lookup with two qualifying cavities, including mutation/removal/serialization. Movement remains deferred until canonical aggregation is implemented.


## Core-specific cavity lookup implementation — 2026-10-22, Section 17.118

- PR #205 head `a5b95ca21acce5ced5b4ccae47d523fc8f357472` adds core-anchored cavity analysis, uses the anchor ID for reproduction readiness, and keeps the organism cavity cache from switching to another cavity while known primary-core IDs remain.
- New two-cavity regression and all other Rust tests are not yet confirmed at the latest head; CI run `39862195685` has passed formatting, structural checks, and focused chemistry/Bob tests, while the full Rust test step remains in progress. Clippy pending.
- Aggregate ownership, atomic parent-funded growth, and movement migration remain unimplemented. Next: finish CI and audit the all-core-constituents-removed identity edge case before changing aggregate ownership. Movement is deferred until canonical graph migration and must be announced before that refactor.


## Reproduction anchor geometry and core identity — 2026-10-22, Section 17.119

- Core identity now has an explicit established flag, so deleting all known primary-core IDs cannot cause a different cavity to silently become this organism's genome. Existing nonempty serialized core IDs infer identity.
- Exact-head CI at `548fb485c5850b7ccdc0cb3a622d569cd433fbd9` exposed two missing `genome_core_identity_established` initializers and an unused `HashMap` import; fixed in subsequent commits, pending fresh exact-head CI.
- Anchor placement was using inclusive overlap as a proxy for contact, and its initial candidate exactly matched a parent part's position. It now requires boundary contact and no strict penetration against any parent part; a regression assertion was added.
- No aggregate migration or movement changes yet. Next: verify fresh CI and then design atomic anchor promotion into parent-owned physical structure.


## Core-overlap cavity ranking — 2026-10-22, Section 17.120

- Core-specific cavity lookup now prefers candidates whose boundary contains more surviving stable core IDs, with area as tie-breaker; generic lookup remains largest-area based.
- Added a focused ranking test. It does not replace a future geometric two-cavity regression fixture.
- Reproduction anchor placement now requires boundary contact without penetration, and the existing test asserts both. Initial CI exposed formatting-only diffs; formatting was corrected. Exact-head CI is still needed.
- Next: validate exact head, then implement atomic anchor promotion into the canonical parent graph. Movement remains deferred until aggregate migration.



## Latest validation checkpoint — 2026-10-22, Section 17.121

- Exact tested head `aa7750d35124a9d03952d70bd74cac99db13826c`: 589 passed, 27 failed, 1 ignored. The new core-identity initializer compiles; baseline failure pattern remains. This predates anchor-placement and cavity-ranking edits.
- Strict Clippy remains red with 34 app-target and 15 test-target diagnostics.
- Exact head `7631b67b178d26a559e6a54c1531391d6ab30962`: 590 passed, 27 baseline failures, 1 ignored. The anchor-placement and core-overlap ranking regressions passed. Clippy remains at baseline 34 app-target and 15 test-target diagnostics.
- Primary-core-loss regression was added at `c8ce07e0322b1fd880dbdc6261cfd1bc52d372c4`, formatted at `135bc4349263a80b5d3a67c8e2e40a3eeeea753f`. Exact-head CI for `135bc4349263a80b5d3a67c8e2e40a3eeeea753f` passed formatting and was still in full tests.



## Latest checkpoint — 2026-10-22, Section 17.122

- PR #205 current source head: `135bc4349263a80b5d3a67c8e2e40a3eeeea753f`.
- Exact-head workflow [39864616964](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39864616964): formatting, architecture/source-size checks, and all focused Bob/fluid/restoration/BREAK gates passed. The full Rust suite timed out at five minutes while running 619 tests; Cargo emitted no final aggregate summary, so no full-suite pass/failure counts or baseline identity claim is made. The primary-core-loss regression passed. Strict Clippy remains red at 34 application-target and 15 test-target diagnostics; no new normalized comparison was made on this head.
- Audit result: anchor placement now requires contact with the intended parent part and rejects strict penetration. `begin_reproduction` removes the selected physical anchor from parent storage and instantiates it into a separate developing graph, so the material is not duplicated in storage, but the parent structural graph remains unchanged.
- No aggregate-merge primitive was found in the inspected structure API. Graph-local physical IDs are used by units, bond endpoints, and core-identity groups; naïve append would alias IDs. A correct merge must stage an old-to-new ID map, remap all bond endpoints and core groups, preserve bond energy once, and commit failure-atomically.
- Next: design/test the staged graph-merge primitive and its rollback/identity invariants before integrating anchor promotion. Do not patch scalar energy prices or the old developing-energy reserve as a shortcut.
- Movement is still deferred. Before migrating developing structure into the parent aggregate, notify the owner that the coordinated movement phase will need translation, collision/space checks, world-tone body enumeration, and position/structure revision changes together. PR #196 remains out of scope.
- Governing plan updated in commit `22da84bcc4046100ef8e50217fc12c93f54e82ab` (Section 17.122).



## Staged physical graph merge checkpoint — 2026-10-22, Section 17.123

- Current PR #205 source head: `ed523c88d4937aa5fe55864d7218fe1db69adb32`.
- Added `append_graph_remapping_ids` and two tests. The method stages a candidate graph, remaps source constituent IDs and bond endpoints, carries source primary/additional core groups into the merged identity metadata, preserves bond energy, and rejects malformed source references before destination mutation. Geometry/contact validation is intentionally outside this utility.
- Added a dedicated CI step: `cargo test --locked append_graph_`.
- The first CI run `39866008821` failed only the formatting step; three rustfmt layout changes were corrected. Current exact-head run `39866134452` passed formatting and is running the focused graph-merge tests. Result is pending; do not claim tests passed yet.
- This is not live reproduction migration. Parent and developing graph remain separate; no energy holder or movement path was changed. After focused verification, audit overflow/invalid-destination cases and then decide the smallest coherent transaction for anchor promotion. Notify the owner before the later coordinated movement refactor.
- Plan Section 17.123 recorded in PR #204 commit `f3beaef10308148de5761ad0c42921c163ad1231`.



## Canonical aggregate / movement migration audit — 2026-10-22, Section 17.124

- Movement is now the next coordinated phase. Before canonicalizing the developing graph, the audit mapped the coupled paths: movement collision/translation currently sees only organism.structure and separately translates construction.developing_structure; world-tone snapshots enumerate the parent and developing graph as separate physical bodies; structural-energy and usable-energy inventories separately count developing holders; growth currently targets units in the separate child graph; generic viability selects a whole-graph cavity and cannot by itself validate the developing core; division must partition stable-ID components and energy atomically.
- No aggregate or movement runtime changes were made during this audit. The implementation sequence must migrate reproduction state, parent-funded atomic growth, movement translation/collision, world-tone enumeration, energy inventory, core-specific viability, and division partitioning coherently. Do not simply call the graph-append helper and leave duplicate holders/call paths behind.
- Latest source head at this checkpoint: 80b0d0b9a05093b4983be71da7536763f4071724. Formatting and focused graph-merge tests passed. Full-suite workflow 39866415942 is still running; the preceding exact-head run 39866134452 reported 593 passed, 27 failed, 1 ignored, with exactly the same 27 named failure identities as the prior verified run, and Clippy remained at baseline 34 application-target / 15 test-target diagnostics.
- Plan Section 17.124 recorded in PR #204 commit 0abd2b46690510be391a3483bada613225e981d3.



## Core-specific viability groundwork — 2026-10-22, Section 17.125

- Added validate_realized_organism_for_core to src/organism/viability.rs. It selects cavity geometry by surviving stable core IDs and measures extra structure within the selected cavity's connected component. The generic validator and live behavior remain unchanged.
- Added a focused test for the known-core success / missing-core fail-closed contract and CI gate cargo test --locked core_specific_viability_.
- Current source head: 17b2e6403703aa1cfb2e58223861103fce4194a3. Exact-head workflow 39867010714 is queued/in progress; formatting and focused test results are pending.
- This is groundwork, not live integration. Next is to finish this focused gate, then continue the canonical aggregate migration with the coordinated movement/world-tone/energy-inventory/division changes already mapped in Section 17.124.
- Plan Section 17.125 recorded in PR #204 commit ebe63022da65365ef44c208bf9c1b0f17f31fbbb.



## Latest checkpoint — 2026-10-22, Section 17.126

- PR #205 exact head: `17b2e6403703aa1cfb2e58223861103fce4194a3`. Workflow [39867010714](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39867010714): formatting, source/architecture checks, graph-merge tests, core-specific viability test, and focused Bob/fluid/restoration/BREAK gates passed. Full suite: 595 passed, 27 failed, 1 ignored (282.97s); Clippy failed. The 27 failure identities and Clippy identities have not yet been normalized against the frozen baseline on this head.
- Audit confirmed the core-specific validator fails closed for missing stable core IDs, but reproduction readiness still follows its core-anchored cavity check with a generic whole-graph viability call. That must become core-specific before shared-graph integration.
- Movement currently sees/translates the parent graph and separately translates the developing graph. World-tone and structural-energy inventory also enumerate/count developing structure separately. Those paths would double-move/double-count after a graph merge unless migrated together.
- **Updated owner contract:** attached reproductive structure is ordinary parent-owned physical structure and participates in resonance; do not retain a separate developing-structure graph or special “committed reproductive material” class. Only genuinely stored material stays in storage. This supersedes the earlier transitional separate-graph description in Section 17.31.
- No runtime source change in this audit. Next: atomically promote the reproduction anchor into the parent graph with ID/core-group remapping, then migrate movement/collision, world-tone enumeration, energy inventory, growth targeting, and core-specific viability as a coordinated slice. See plan Section 17.126.


## Implementation checkpoint — 2026-10-22, Section 17.127

- PR #205 commit `294de15f58340c006dc6817da6ad19a5e640efee` makes reproduction readiness use the core-specific viability validator anchored to the developing anchor's stable physical ID, instead of validating whichever cavity the generic whole-graph lookup selects.
- Exact-head CI for this commit is pending/not yet returned by the workflow lookup. No pass is claimed.
- This is only the first integration prerequisite. The developing graph is still separate; the next audited change must plan failure-atomic anchor promotion and all coupled movement/collision, world-tone, energy inventory, growth, viability, and revision/cache paths before shared-graph ownership is enabled. Attached reproductive structure counts for resonance; stored material alone does not. See Section 17.127.


## Verification correction — 2026-10-22, Section 17.128

- Commit `294de15f58340c006dc6817da6ad19a5e640efee` failed the exact-head workflow's formatting check; rustfmt required a multiline import. The focused logic tests did not run on that commit.
- Formatting-only fix is `080852d365dd66016985b90e0e931647313d628f`. Exact-head workflow status is not yet available from the lookup; no verification claim yet. Do not stack another source change before the focused gate passes.


## Canonical construction design audit — 2026-10-22, Section 17.129

- Target model is one parent-owned physical graph for all attached units, including the second genome's structure. Reproductive construction should retain genome/developmental metadata only—not a second graph or separate developing energy/stress/maintenance accounts. Parent usable energy funds construction atomically; parent maintenance, stress, death, and resonance operate once over the aggregate. Only unattached stored material remains in storage.
- Growth should trial against the full parent graph for collision truth but restrict attachment anchors to the developing core's stable IDs/component. Source storage, graph mutation, bond/energy settlement, and core-ID update must commit/rollback together. Do not invent an attachment bond or energy; define whether contact alone is sufficient or use the ordinary bond path.
- Movement, world-tone, energy inventory, core-specific viability, and eventual division must migrate together. Current exact-head workflow for commit `080852d365dd66016985b90e0e931647313d628f` remains in progress; no verification claim yet. See plan Section 17.129.


## Stable membership audit — 2026-10-22, Section 17.130

- Shared-graph construction needs separate identity for the child's genome-core boundary and for all child member units. Core IDs govern core integrity; full member IDs scope growth and eventual division. A connected-component-only approach fails if a bond connects child to parent. Keep the child's member IDs in construction metadata; do not let an additional core group mean full body membership or allow the child's core to substitute for the parent's core.
- Anchor promotion must atomically combine storage debit, graph merge/remapping, member/core IDs, and structure revision. Graph append does not form a parent bond; current anchor geometry only guarantees contact without penetration. Do not invent a bond or energy. Growth must collide against the whole aggregate but attach only to child member IDs and debit parent ordinary usable energy.
- Eventual division must partition child member units/bonds and preserve bond energy once, validate each child's own core, and split remaining usable energy by validated post-split structural mass. Exact-head CI for `080852d365dd66016985b90e0e931647313d628f` has passed formatting and preceding focused gates; full tests were running, Clippy pending at this checkpoint.


## Latest chemistry checkpoint — post-death holder audit, 2026-10-22

- Source audit at PR #205 head `080852d365dd66016985b90e0e931647313d628f`: biological processing is gated by core viability, while the ordinary chemistry accumulator still runs on breached organisms. Breached organisms remain physical holders; the live tick does not invoke the legacy `recycle_dead_organism` / `DecomposingBody` path.
- Added test-only regression in PR #205 commit `cac2fdcce5fc2f06a05ddbeb12099161fd80a9d6`: `src/tests/simulation_tests.rs::breached_organism_material_and_energy_holders_reconcile_over_live_ticks`. It tests the independent integer-material oracle and world-energy balance over three ticks for a breached organism holding leftover storage and active reproductive construction.
- Workflow [39882132224](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39882132224) for `cac2fdc` stopped at formatting only; no Rust tests executed. Formatting-only correction is commit `856f809f9b56a49321fc6a33622b81c3420929b4`. Its exact-head workflow [39882268330](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39882268330) has passed formatting, source-size/architecture checks, physical graph-merge and core-specific viability tests, Bob storage/open/reopen/concurrency, fluid transaction/coalescence/restoration, and focused chemical BREAK gates. Full Rust suite is running; Clippy has not reported, so the lifecycle regression is not yet verified.
- The regression rebases the fixture's structural/resonant initial inventory after directly configuring the breached starting state. It does not activate the legacy decomposition path and does not resolve the separate question of whether core repair can restore biological activity after a breach.
- Next: inspect workflow 39882132224, check whether the focused test passes, reconcile baseline failure identities and Clippy diagnostics, and then append the exact verification result. PR #204 is documentation-only; runtime/tests remain in PR #205. Movement stays coupled to the later aggregate-ownership migration.

## Resume checkpoint — universal transaction invariant and exact-index rollback (2026-10-23)

- Owner approved the universal rule now appended as Section 17.132 of the plan: every material/energy transaction either commits all source, holder, destination, physical, and ledger changes together, or restores the exact prior state. Preserving material somewhere is not enough if a failed operation changes its holder, location, or ordering without a separately accounted-for transition.
- The same plan section records the latest reproduction decision: 160% total scale permits division but does not force it; the organism chooses when, rising pressure only influences its decision. Inside-out growth can require internal bonds to break, and ordinary chemistry must perform those bond changes.
- PR #205 latest head: `75566fa05051efaec72bc4d75373fd2c9e560e19`; exact-head workflow [39954234513](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39954234513) is in progress. The focused change preserves storage order during stored-BREAK reservation and restores failed completion/preparation escrow to the original index; regressions use a middle source entry surrounded by distinct stored items. Do not claim this head verified until workflow completion.
- Last completed preceding head `c529f39ff4a06de09d8142b3a6b29e73c9010002`, workflow [39945020308](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39945020308): 601 passed, 27 failed, 1 ignored; the added malformed-preparation rollback test passed, and named failures matched the known baseline set. Strict Clippy remained red; normalized diagnostics have not been confirmed for the newer head yet.
- Next: inspect workflow 39954234513, compare named failures and normalized Clippy identities to frozen baseline, then audit environmental-source BREAK recovery because its failed-return path can fall back to organism storage instead of restoring the exact original environmental location. Continue with one focused audit before each change, commands/tests under five minutes, PR #204 documentation-only, and no PR #196 scope creep.

## Current exact-head status — 2026-10-23

- The stored-BREAK original-index rollback work is now at PR #205 head `a78d0027c9e40a6d8fe01cb82255987eecd34ebc`. It follows formatting-only correction of the four assertions reported by workflow 39954234513; that failed run stopped before focused tests.
- Exact-head workflow [39954558265](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39954558265) passed formatting and all preceding focused gates, including chemical BREAK. Full Rust suite is in progress and Clippy is pending. Wait for the actual result before classifying test identities or diagnostic changes.
- The next source audit is environmental BREAK exact restoration. Current source path removes an environmental object from a specific field cell at a known vector index, but candidate identity only captures the cell and bond index. Completion/preparation recovery does not always restore the exact cell/index. After `a78d002` verification, update the candidate identity to carry both indices and test failure rollback against multiple materials in one cell; do this as a single focused transaction change.
- PR #204 handoff/plan also records the owner-approved global all-or-nothing invariant and the organism-controlled division policy in Section 17.132; the follow-up exact-head/format correction is in Section 17.133. Reproduction implementation stays gated behind canonical aggregate/movement migration.

## Immediate verification update — 2026-10-23

- Latest PR #205 head is `5b3bba55b3accee4567f23a01c1b2b378d3bb564`; workflow [39955578198](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39955578198) is in progress.
- Previous exact-head workflow [39954558265](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39954558265) passed formatting and focused gates, but full suite hit the five-minute step timeout after reporting 599 passed, 29 failed, 1 ignored. Its two additional failures were rollback regression fixtures, not confirmed production failures: storage inserts new objects at index 0, while the tests assumed append order, so the chosen `stored:1` source was not the bonded object and the operation could not start.
- Head `5b3bba5` corrects the fixtures to reverse insert order and compare against a snapshot of normalized storage entries. Watch the exact-head focused/full-suite output before any next code changes.
- Once confirmed, add a dedicated `cargo test --locked failed_stored_break` CI step so storage rollback assertions get visible coverage before the slow full-suite gate. Then resume the environmental BREAK source-cell/material-index rollback audit; do not combine those two fixes.

## Exact-head test checkpoint — 2026-10-23

- PR #205 head `3b11a4de737a3d90017057e6f3fd2cf1a8432bca`, workflow [39956105550](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39956105550): formatting and focused chemical BREAK passed; new `cargo test --locked failed_stored_break` gate passed. The two corrected rollback tests now pass on this exact head.
- Full Rust suite is still running and may hit its five-minute cap; the prior head reported 599 passed, 29 failed, 1 ignored and timed out, but its two extra failures came from fixtures that targeted the wrong storage index. Treat only current exact-head output as authoritative for the corrected tests. Strict Clippy result/normalized comparison remains pending.
- Resume the environmental BREAK audit next: source identity needs both field cell and material-vector index in the context; exact reinsertion must be shared by preparation and completion rollback. Add regressions for both failure points, and extend the focused CI gate with `failed_environmental_break` after those tests exist.

## Current environmental BREAK rollback checkpoint — 2026-10-23

- PR #205 exact head `a2fd997703cd15dc4cca8f6eceb9707fd2f8934a`; workflow [39957367517](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39957367517) is in progress after a formatting-only correction. The preceding head's formatting failure was from five rustfmt wrapping adjustments; those are applied now.
- The runtime change updates environmental BREAK keys with the selected material-vector index, validates and selects that exact object on start, and restores failed escrow to the original cell/list index from both preparation and completion failure paths. Two focused regression tests were added and CI now runs `failed_environmental_break` alongside `failed_stored_break`.
- First inspect the two focused rollback gates on the current head, then compare full-suite failures to the frozen set. Avoid treating prior full-suite status as current-head verification; no tests have yet been confirmed on `a2fd997`.


## Current decomposition finalization checkpoint — 2026-10-23

- PR #205 head `fc2e8d5bd49422bf83757a440a0cd9b241352bc1` stages final decomposition energy settlement on a copied ledger/local holder and commits it only after `deposit_physical_batch` accepts every released physical object. The regression `invalid_finished_body_position_preserves_released_material_ownership` verifies failed deposit preserves body material, usable energy, and the live ledger.
- Exact-head workflow [40018610844](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/40018610844) has passed formatting, source-size/architecture checks, graph merge tests, chemistry accumulation, and the new focused decomposition-finalization rollback gate. The full Rust suite is running; do not call the head fully verified yet.
- A separate graph regression fixture was corrected in the same head series: it now snapshots a valid replacement subgraph before injecting a dangling destination bond, so the test exercises replacement rejection rather than failing during fixture setup.
- Next: inspect the exact-head full-suite result and Clippy diagnostics; compare named failures to the frozen baseline. Then continue the remaining phases, prioritizing the post-death ordinary-chemistry integration and world-wide material/energy inventory. Do not conflate this atomic-finalization fix with the larger “no separate decomposition law” requirement. PR #204 remains documentation-only and PR #196 remains out of scope.


## Exact-head rollback gates — 2026-10-23

- PR #205 head `1318d2f64dbe71dd57271761f40580bfabc6cd20`, workflow [40019005755](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/40019005755), passed the dedicated decomposition-finalization rollback test and the late physical-restoration rollback test. The corrected physical graph merge tests also passed.
- The full Rust suite is running on this exact head. Strict Clippy and named failure comparison are not yet available; keep the suite status pending.
- Next work remains ordinary-chemistry participation for post-death holders and the world-wide material/energy inventory. The current change only ensures a failed finished-body release cannot partially settle energy while leaving material behind.


## Resume note — Phase 8 aggregate ownership audit (2026-10-23)

The next coherent implementation step is canonical aggregate/identity migration, not activation of `recycle_dead_organism`. The live tick continues chemistry on the primary graph after core breach, but stored compounds, committed material, and the separate developing graph are not enumerated by ordinary reaction accumulation. Energy inventory already counts developing bond energy and resonant energy, so the migration must preserve those stores, stable physical IDs/endpoints, genome-core membership, and bond energy. Stored physical compounds have endpoint metadata but no allocated bond-energy store; stored BREAK must not mint energy. See plan Section 17.201. PR #196 remains out of scope.

Current PR #205 head at the time of this handoff: `83a88c887ec4476485369fa9abe6425cbf9ca899`; exact-head workflow is running. Do not treat it as verified until the stale-cache rollback test, recycling rollback compile/focused gates, full-suite named-failure comparison, and Clippy diagnostics have been inspected.
