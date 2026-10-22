# Chemistry Unification — Current Handoff

**Last reconciled:** 2026-10-22  
**PR #204 documentation head:** `d88f6a03d1d697e841fd7c34acfbda4de15f0fa7`  
**PR #205 implementation head:** `12d8079230458da68a8dc3755a1d6a9b41347862`  
**Latest exact-head workflow:** [39833438679](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39833438679), complete.

This is the concise current-state handoff. The detailed audit register and canonical scientific decisions are in [chemistry-audit-unification-plan.md](chemistry-audit-unification-plan.md), especially Sections 14 and 17. PR #204 remains documentation-only; production implementation and tests belong on PR #205.

## Governing boundaries

- Frozen baseline: `main` at `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5`; workflow [39705628427](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39705628427): 563 passed, 28 failed, 1 ignored.
- Resource ID 6 is **Oxygen**.
- PR #196 constructor/Bob work remains separate.
- One focused audit before each code change. Every command/test step has a hard five-minute limit; a timeout is incomplete evidence.
- Whole-world energy/material conservation is not proven; the suite remains baseline-red.

## Latest exact-head verification

Workflow [39833438679](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39833438679) completed on PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`.

- Formatting and all focused gates passed.
- `chemistry_library::tests::insert_partial_append_failure_rolls_back_original_cache_bytes_and_reopens_cleanly` passed.
- Full suite: 585 passed, 27 failed, 1 ignored in 283.74 seconds. All 27 failing test identities match the frozen baseline; `simulation_tests::integration_tests::contained_physical_material_becomes_storage_without_an_acquire_action` remains resolved. No new failing identity observed.
- Strict Clippy remains red with 34 application-target and 15 test-target diagnostics. **Normalized diagnostic comparison is now complete:** compared with baseline workflow `39705628427`, there are no new or removed identities when matching message + source path and ignoring line-number shifts. No lint change is justified.

## Phase tracker

| Phase | Status | Remaining gate |
|---|---|---|
| 0 — Freeze baseline and scope | Recorded | Preserve exact-head named failure comparisons |
| 1 — Contract and dependency map | Recorded | Keep docs aligned with verified behavior |
| 2 — Independent inventory oracles | Implemented; incomplete | Broaden cross-domain energy/material cases |
| 3 — Resource identity and indivisible ownership | In progress | Failed transformation rollback and post-death ownership/inventory coverage |
| 4 — Chemistry cache version boundary | Implemented; gate open | Broader cache failure/restart coverage |
| 5 — Authoritative energy ledger and initial source | Started; gate open | Multi-organism, bond, transfer, and world-tone inventory cases |
| 6 — COMBINE/BREAK, bond ownership, overload | Not started as a unified phase | Exactly-once bond-energy settlement and atomic rollback |
| 7 — Energy consumers and deterministic tick phases | Not started | Source/consumer inventory and retry/cancel atomicity |
| 8 — Lifecycle and post-death chemistry | Active design audit; implementation not started | Safe one-time ownership transition for stored/developing holders; preserve all energy and active transactions without legacy decomposition |
| 9 — Sunlight correctness/performance | Not started | Exact-small-world oracle, source budget/occlusion, error and performance evidence |
| 10 — Long-run numerical integrity | Not started | Drift, overflow, and tolerance-versus-dissipation tests |
| 11 — Integrated verification | Not started | Full release-gate evidence and README consistency |

## Phase 8 audit result and next actions

The latest focused audit is recorded in Section 17.26; the Clippy comparison is recorded in Section 17.27. PR #205 comments [6348773380](https://github.com/coringilbert16-cmyk/EvoSim-/pull/205#issuecomment-6348773380) and [6348800724](https://github.com/coringilbert16-cmyk/EvoSim-/pull/205#issuecomment-6348800724) record the evidence.

- `Simulation::step` recomputes `living_ids` from core qualification. Breached organisms remain physical entries in `self.organisms`; structural-bond chemistry continues, but biological work is skipped.
- `ActiveMaterialField::deposit_physical` / `deposit_physical_at_index` transfer an already-realized physical object and report failure. `MaterialStorage::drain_entries` removes ownership, so callers must retain/restore entries if a deposit fails.
- Existing `recycle_dead_organism` is **not** a valid integration path. It uses the forbidden `DecomposingBody` carrier, ignores deposit failures after draining storage, and removes reproductive construction without preserving its developing structure as a physical holder. Existing `expel_physical_material` is geometry-aware for a single stored item, but depends on movement memory or a nonzero owner-relative offset and does not restore storage if the final environmental insertion fails; it is not a general death-release transaction.
- Current inventory tracks usable/developing energy, resonant energy, parent/developing structural bond energy, and active transformations. Active transformations intentionally settle after core breach. No approved death-transfer policy for resonant energy or intact developing-structure bonds is established yet.
- Do not activate the legacy one-bond decomposition law or claim that a material/energy release policy is settled.

**Owner-approved design (Section 17.31, 2026-10-22):** keep the parent as the authoritative aggregate owner of body structure, storage, committed material, developing structure/bonds, and associated energy until division. Core breach stops biological processing but does not transfer ownership or create a special decomposition object; ordinary chemistry should continue on owned physical holdings where physically accessible. This avoids a death-time migration, but aggregate ownership alone does not make storage/developing structures chemically available. At division, atomically partition every constituent, holder, bond, and energy store exactly once. Section 17.29's detached-holder approach is retained as a fallback if the existing world model cannot support parent-owned physical holders safely.

**Next steps:**
1. Audit the minimal world/environment home, stable ID, anchor transform, and every spatial/chemistry/persistence/inventory call site needed for a detached physical-structure collection. Define resonant-energy handling and prove atomic rollback or retained ownership for failed transfers.
2. Once that focused design audit is complete, implement and test the smallest scoped Phase 8 change on PR #205. Add independent material and energy inventory coverage across a breach and bounded live ticks.
3. Keep all phase gates open until exact-head evidence is verified. Update this handoff whenever head, evidence, or phase status changes.

## Separate reproduction requirement (not implemented here)

Internal/adjacent budding: at 100% of original organism size, begin building a second genome core immediately next to the first; continue outward growth to 160% total size; split into two roughly equal 80%-size children. Current reproduction code does not yet satisfy this lifecycle. This remains a separate future requirement, not part of this chemistry change; PR #196 remains out of scope.


Latest Phase 8 design-audit addendum: Section 17.28 documents why `expel_physical_material` is not safe to loop on core breach. PR #205 comment [6348852052](https://github.com/coringilbert16-cmyk/EvoSim-/pull/205#issuecomment-6348852052) records the source finding. No code/test change has been made; next is still the design audit, not implementation.


Recommendation added to the detailed plan in Section 17.29 (proposal only). The preferred direction is an intact generic detached physical-structure holder plus an atomic one-time ownership transition, not the legacy decomposition carrier. Next audit: chemistry enumeration/accessibility for parent-owned stored and developing physical holders, plus division partition call sites, bond-crossing settlement, energy ownership, and atomic rollback. No code change.


User proposed keeping the parent cell as complete owner until division; this is now the preferred design direction in Section 17.30. No implementation code or tests changed.


### 2026-10-22 owner approval and next chemistry gate

The owner explicitly approved the parent-aggregate ownership model: the parent remains the sole authoritative owner of its body, stored and committed material, developing structure/bonds, and associated energy until division. Core breach ends biological processing but does not trigger death-time transfer or create a special decomposition object. The adjacent second genome core remains part of unfinished internal construction until division. This is now an approved architecture, not a provisional recommendation; see detailed plan Section 17.31.

**What remains before chemistry can be called solved:** (1) ordinary reaction enumeration and physical validation must cover every applicable parent-owned holder without assuming ownership equals accessibility; (2) an independent whole-world inventory must account for each constituent and every energy category exactly once; (3) COMBINE, chemical/stress BREAK, storage, movement/work, resonance/world-tone capture, dissipation, and lifecycle paths must debit/transfer/settle atomically; (4) developing graph geometry, endpoints and bond energy must remain valid; (5) world-tone source/capture/propagation/escape/heat must reconcile with occlusion and performance; (6) bounded exact-head regressions and CI must show no new failure identities versus baseline. Atomic division at the approved 160% lifecycle is a related future gate but remains out of PR #205 unless scope is explicitly changed.

**Next focused audit:** trace reaction enumeration/physical validation for parent-owned stored material, committed material, and developing structure; compare the coverage with the independent material/energy inventory; identify the smallest missing link. No code or tests changed for this approval. Existing PR #205 exact-head evidence remains the historical evidence at 12d8079230458da68a8dc3755a1d6a9b41347862; verify the current head before any implementation work.


## Latest source-audit handoff — 2026-10-22

- PR #204 plan update: commit `47856926dacc84eacd9a0b30ab94adbfaedc832c`, Section 17.32. PR #204 remains documentation-only.
- PR #205 implementation head checked: `12d8079230458da68a8dc3755a1d6a9b41347862`. No source or test changes were made.
- **Confirmed reachability gap:** the production tick sends each organism (including breached/dead organisms retained in the collection) through `chemical_reaction::accumulate`, but that accumulator only iterates bonds in the organism's primary `structure.bonds` graph. It does not enumerate stored material, committed construction material, or the separate developing structure's bonds. The world-tone occluder snapshot includes those physical holders, but that is exposure geometry, not chemistry reachability.
- Energy snapshot includes parent usable plus developing energy, retained resonance, main-graph bond energy, developing-graph bond energy, and legacy-carrier values. This is useful coverage, not a proof of all-holder reaction settlement or atomic transitions.
- **Do not implement a broad reaction-domain change yet.** Stored `PhysicalMaterial` internal connections do not necessarily represent funded/stored numeric bond energy. Treating them as graph bonds without mapping their settlement semantics could create energy or corrupt physical topology. `MaterialStorage::drain_entries` uses `mem::take`; any eventual cross-holder operation must validate before draining or return the complete holder on failure.
- Owner-approved invariant remains: parent owns body/storage/committed/developing holdings until division; death ends biological processing but does not transfer ownership or introduce a separate decomposition law. Ownership is not automatic chemical availability.
- **Next audit:** compare `PhysicalMaterial` internal connections and stored-BREAK settlement with graph-bond settlement; trace holder take/drain/restore and energy-ledger paths. Determine which bond classes carry actual stored energy and which are eligible for ordinary chemistry, then reconcile those semantics against the independent material/energy oracle. No code or tests until the audit templates are complete.
- Existing exact-head CI evidence at `12d8079` remains as documented above; before implementation, re-fetch the PR #205 head and run bounded verification only against that exact SHA.


### Follow-up audit: stored physical-bond semantics and failure path

- Plan Sections 17.32–17.33 are now recorded; latest plan commit `adbcd7ecbc35db3474e3c34b0157f07b4dec2ae4`. PR #204 remains documentation-only.
- Exact PR #205 head audited: `12d8079230458da68a8dc3755a1d6a9b41347862`.
- `PhysicalMaterialBond` stores constituent/end-point topology but no `bond_energy`. Stored-material BREAK validates and splits topology using a zero-energy transaction; this is intentional because no energy was allocated when the pre-existing internal connection was created. Do not derive free energy from resource potential.
- Stored BREAK temporarily escrows its source on an `ActiveTransformation` linked to the parent. Preparation failure has a restoration helper. However, successful split-piece storage fallback attempts a world deposit using intrinsic-frame placement and ignores deposit failure; malformed completion with missing `prepared_energy` also clears the active ID without an evident source restore. These are failure-path candidates; they are not proof of common-path failure.
- **Next:** trace all physical-material transitions among environment, parent storage, committed storage, active-transformation escrow, and developing structure; compare every transition and rollback with the independent integer material inventory. Establish the shared bond contract (funded graph bond vs unfunded physical connection) before proposing reaction-unification code. No code/test changes until the audit templates are complete.


### Three-audit checkpoint — Sections 17.32–17.34

- Latest PR #204 plan commit: `682e085fbaa78b94289822d94bfcd67510305162`. PR #204 remains documentation-only.
- The independent `integer_material_inventory` test oracle covers field stock, organism graph/storage, committed construction storage, developing graph, active-transformation physical escrow, and legacy carrier structures; it rejects unknown/non-whole constituents and does not read ledger totals. The smaller `rollback_material_inventory` helper in `simulation.rs` covers only graph/storage/field and must not be treated as a general-world oracle.
- COMBINE mostly follows validate-on-trial then source-debit/commit ordering. Reproduction anchor transfer also uses trial copies, but full reproduction/division remains a separate lifecycle gate.
- Confirmed transition risk: environment-to-storage and stored-BREAK fallback paths ignore failed physical deposits. Stored-BREAK fallback may use intrinsic-frame placement as world coordinates. These are potential silent material-loss / placement risks under rejection, not proof of common-path failure.
- Next: build the source-removal → trial → energy settlement → destination commit → rollback matrix for every transition, especially all early returns after source removal in stored BREAK and reproductive construction. No code or test changes until the audit templates are complete.
- Exact PR #205 head for this source review: `12d8079230458da68a8dc3755a1d6a9b41347862`; re-fetch before future implementation.


### Holder transition matrix checkpoint — Section 17.35

- Latest plan commit: 17c0c4f5477c63b8a1280e339e206d71a4588875.
- The source-removal/commit matrix is now explicit. Immediate COMBINE and parent-storage-to-construction use trial copies before source debit; active BREAK escrow is counted once by the independent material oracle.
- Highest-priority failure candidate remains physical material removed from a source holder followed by a destination deposit that can fail but whose result is ignored. Stored BREAK also commits pieces one at a time, so a later piece rejection can leave a partial split; its fallback placement is not proven world-relative.
- The currently implemented construction finish boundary may emit a child on Ready, Detached, or Dead; this is existing reproduction lifecycle behavior and does not implement the owner-approved 100%→160% total growth / two ~80% children design. Do not redesign reproduction inside PR #205.
- Next audit: inspect failure-injection hooks and tests for stored BREAK, contained-material transfer, and construction transfer; separate already-testable matrix cases from cases needing a future narrow test seam. No source/test changes.


### Failure-injection coverage checkpoint — Section 17.36

- Latest plan commit: dd15f2d5d6d814511b9bf60120606d540fa2020d.
- Existing tests cover successful field-to-storage transfer, storage restoration and rejection rollback, exact-once active BREAK escrow inventory, zero-energy stored BREAK preparation, and reproduction waiting/resumption. The full independent material oracle covers the relevant authoritative holders.
- Missing tests remain for partial multi-piece stored-BREAK commit, rejected storage plus rejected field deposit, failed containment-transfer fallback, malformed stored-BREAK completion restoration, stale construction source index, and every environmental COMBINE source-cell rollback variant.
- Do not add a test seam yet. After all audit templates are complete, a narrow test-only destination-rejection seam may be needed to deterministically test partial piece rejection and fallback failure. Any regression must compare the full independent material oracle, holder/geometry snapshots, and energy/resonance ledger.
- No source/test changes. PR #205 head remains the audited 12d8079230458da68a8dc3755a1d6a9b41347862; re-fetch before implementation.
