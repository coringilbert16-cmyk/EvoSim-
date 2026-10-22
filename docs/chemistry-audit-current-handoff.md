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


### Three-audit checkpoint — Sections 17.35–17.37

- Latest plan commit: f1c439534d62980ce253473f0e51caba4a5a5f1d.
- Failure-injection audit: normal-path tests exist, but partial multi-piece stored BREAK, storage-plus-deposit rejection, failed containment fallback, malformed completion restoration, stale construction source, and all COMBINE rollback variants are not yet covered. No test seam or source change has been added.
- Energy-holder transition matrix: COMBINE and construction mostly stage candidate state before source debit; environment/storage and stored-BREAK fallbacks remain failure-atomicity risks. Reproduction finish is a distinct ownership boundary and stays out of chemistry implementation scope.
- **New world-tone finding:** current PR #205 head has live sunlight capture enabled. Shared occlusion geometry includes stored/committed/developing material and active transformations, but only primary structures of living organisms map to resonance reservoirs. Every other physical receiver converts its material response to heat, including parent-relative non-primary holdings that the approved aggregate model says remain parent-owned until division.
- This exposes a contract question: should a living aggregate harvest compatible resonance from its owned non-primary surfaces, or is resonance eligibility intentionally limited to the primary graph? Do not change the mapping until the existing approved sunlight decisions are checked for an explicit answer.
- Next: reconcile Sections 17.31 and the earlier approved world-tone rules against receiver eligibility, and determine whether owner clarification is needed. No source/test changes.


## Latest handoff checkpoint — 2026-10-22, Section 17.38

- Plan update commit: `522872e8883df91f94ff777bc861a2f0c10cc06f`. PR #204 remains documentation-only.
- Audited PR #205 implementation head remains `12d8079230458da68a8dc3755a1d6a9b41347862`; this pass changed no implementation source or tests.
- Section 17.38 reconciles the existing approvals: D5 settles the sunlight computation approximation and source/occlusion/propagation constraints, but does not specify which physical holders can credit the organism's resonance reservoir. Section 17.31 settles parent aggregate ownership until division, but explicitly does not make every owned holder chemically available. Therefore prior decisions do not unambiguously settle whether parent-owned stored/committed/developing surfaces can harvest compatible resonance.
- The two plausible rules are recorded in Section 17.38: (1) aggregate capture by any uniquely owned physical surface of a living aggregate into the parent's one reservoir; or (2) primary-structure-only capture, with all other surfaces responding as heat. Do not infer or implement one without owner clarification. Keep the live path unchanged meanwhile.
- Work can continue without blocking on that choice: next audit is the complete source-to-sink world-tone energy matrix (incident budget → response → damping → reservoir credit → onward propagation → escape/heat → ledger), plus proof that active-transformation bodies are not double-counted and each body resolves to at most one owner.
- Existing failure-path risks remain open as recorded in Sections 17.34–17.36. No code/test change or test seam is authorized until all audit templates are complete.


## Latest handoff checkpoint — 2026-10-22, Sections 17.39–17.41

- Plan commit: `69b940e6c4ff43577b5e66b00232b5009a181a27`. PR #204 remains documentation-only.
- Exact PR #205 source head audited: `12d8079230458da68a8dc3755a1d6a9b41347862`. No source/test changes.
- **World-tone source-to-sink path mapped:** every path validates incident energy equals retained resonance + heat + escaped energy; the ledger validates the same frame totals before committing staged reservoirs. Active transformation sources are included as physical bodies only after owner/active-ID consistency checks, and ambiguous dual-source states fail closed. This confirms a useful local atomicity guard, not the independent geometry/whole-world proof required by D5.
- The independent material oracle counts source material in active transformation escrow once and deliberately ignores prepared trial copies. The energy snapshot independently recomputes live usable/resonant/graph-bond stores; stored physical connections carry no allocated numeric bond energy. `total_world_tone_energy_unabsorbed` is documented as throughput telemetry, not stored energy or an inventory term.
- **Tick order now documented in Section 17.41:** active transformations prepare/resolve first; living status is sampled; world tone settles; living biological processing runs; ordinary chemistry runs for all organisms; chemistry-caused breach suppresses later same-tick behavior; construction advances only for still-living parents; stress damage runs near tick end. Therefore a core breached by ordinary chemistry can still receive that tick's earlier world-tone input, while a core breached by a completed transformation before sampling cannot. Confirm this same-tick eligibility rule with explicit tests before altering ordering.
- **Open owner clarification:** Section 17.38 offers two explicit receiver rules—aggregate-wide capture from uniquely parent-owned surfaces versus primary-structure-only capture. D5 and Section 17.31 do not settle it. Do not change receiver mapping pending the owner's choice.
- **Next focused audit:** trace transformation cancellation/stale-state and reproduction construction finish/anchor transfer against the tick table, including every physical source debit, destination rejection, rollback, and owner transfer. Failure-injection gates from Sections 17.34–17.36 remain open. No implementation until all audit templates are complete.


## Latest handoff checkpoint — 2026-10-22, Section 17.42

- Plan commit: `82792e1a4d009e6f526ff49c1b55258d9d998ffd`. PR #204 remains documentation-only; exact PR #205 head audited is `12d8079230458da68a8dc3755a1d6a9b41347862`. No implementation source/tests changed.
- Section 17.42 traces transformation escrow and reproduction construction transitions. COMBINE's missing-prepared-state path rolls back staged ledger/energy and restores an environmental source to its exact cell. Stored BREAK has a concrete non-atomic failure path: it stores split pieces incrementally, then on storage rejection attempts a field deposit whose result is ignored; fallback placement may use intrinsic coordinates even when material is owner-relative. Malformed completion can clear the active marker without restoring escrow.
- Reproduction construction starts by staging an anchor copy and committing parent-storage removal with the construction. Construction progress uses candidate child/ledger state; successful finish transfers developing structure, committed storage, and developing energy from the parent construction into a child that is appended later in the same step. Add stale-source and whole-step before/after independent inventory tests; keep the approved future division specification out of chemistry PR #205.
- Next audit: map every transformation termination route (success, prepare rejection, missing owner, malformed serialized state, stale source, and commit failure) to a terminal owner for each physical source and energy debit. Then close the remaining phase-coverage matrix before proposing any code fix. No implementation until all audit templates are complete.


## Latest handoff checkpoint — 2026-10-22, Sections 17.43–17.44

- Plan commit: `b676b1bf7d34fb5e612c3bcae644e37e2a613fd2`. PR #204 remains documentation-only; PR #205 source head audited remains `12d8079230458da68a8dc3755a1d6a9b41347862`. No implementation source/tests changed.
- Section 17.43 consolidates transformation terminal paths. Highest-severity confirmed gap: stored BREAK resolves pieces incrementally and ignores field-deposit failure; malformed completion may clear the active marker without restoring its escrowed source. Stored-source fallback coordinates are not proven world-relative. Replace best-effort partial transfer with an all-or-nothing holder transaction only after audit completion.
- Section 17.44 found a separate movement atomicity gap: a successful physical move mutates position/occupancy/environment before the energy ledger settlement. If settlement rejects afterward, geometry remains moved while energy is not charged and movement progress is not decremented. Test with deterministic ledger rejection; eventual fix must stage geometry or roll it back on settlement failure.
- Maintenance settlement is better ordered: energy/stress update follows successful ledger settlement, with unpaid demand becoming debt. Resonance-overload BREAK stages reservoir withdrawal and candidate structure, settles first, and only commits the reservoir/graph on success. These paths still need independent inventory tests.
- Movement currently records its full energy cost as heat and zero explicit work. D7 requires this to be reconciled with the physical meaning of work versus dissipative locomotion cost before changing equations.
- **Next audit:** inspect remaining physical mutations and energy consumers for geometry-before-ledger or source-before-destination ordering; update the impact matrix and failure coverage. Do not implement before the audit templates are complete.
