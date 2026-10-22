# Chemistry Unification — Current Handoff

**Last reconciled:** 2026-10-22  
**PR #204 documentation head:** `e3f3d667e0b39606de9d2f71f19f1b1191636f30`  
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
- Strict Clippy failed with 34 application-target and 15 test-target diagnostics. A normalized identity comparison against the frozen baseline has **not** yet been completed for this exact head; do not make lint changes until it establishes whether a new diagnostic exists.

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
| 8 — Lifecycle and post-death chemistry | Active audit; implementation not started | Safe, one-time ownership transition for stored/developing holders; preserve all energy and active transactions without legacy decomposition |
| 9 — Sunlight correctness/performance | Not started | Exact-small-world oracle, source budget/occlusion, error and performance evidence |
| 10 — Long-run numerical integrity | Not started | Drift, overflow, and tolerance-versus-dissipation tests |
| 11 — Integrated verification | Not started | Full release-gate evidence and README consistency |

## Phase 8 audit result and next actions

The latest focused audit is recorded in Section 17.26 of the detailed plan and PR #205 comment [6348773380](https://github.com/coringilbert16-cmyk/EvoSim-/pull/205#issuecomment-6348773380).

- `Simulation::step` recomputes `living_ids` from core qualification. Breached organisms remain physical entries in `self.organisms`; structural-bond chemistry continues, but biological work is skipped.
- `ActiveMaterialField::deposit_physical` / `deposit_physical_at_index` transfer an already-realized physical object and report failure. `MaterialStorage::drain_entries` removes ownership, so callers must retain/restore entries if a deposit fails.
- Existing `recycle_dead_organism` is **not** a valid integration path. It uses the forbidden `DecomposingBody` carrier, ignores deposit failures after draining storage, and removes reproductive construction without preserving its developing structure as a physical holder.
- Current inventory tracks usable/developing energy, resonant energy, parent/developing structural bond energy, and active transformations. Active transformations intentionally settle after core breach. No approved death-transfer policy for resonant energy or intact developing-structure bonds is established yet.
- Do not activate the legacy one-bond decomposition law or claim that a material/energy release policy is settled.

**Next steps:**
1. Normalize Clippy diagnostic identities on `12d8079` against baseline workflow `39705628427`; do not change lint code before that comparison.
2. Complete the narrow design audit for a no-loss, atomic ownership transition: stored physical material, committed material, developing structure/bond energy, usable/developing/resonant energy, and in-flight transformations. Define rollback/retained ownership for failed physical deposits.
3. Only after the focused audit is complete, implement and test the smallest scoped Phase 8 change on PR #205. Add independent material and energy inventory coverage across a breach and bounded live ticks.
4. Keep all phase gates open until exact-head evidence is verified. Update this handoff whenever head, evidence, or phase status changes.

## Separate reproduction requirement (not implemented here)

Internal/adjacent budding: at 100% of original organism size, begin building a second genome core immediately next to the first; continue outward growth to 160% total size; split into two roughly equal 80%-size children. Current reproduction code does not yet satisfy this lifecycle. This remains a separate future requirement, not part of this chemistry change; PR #196 remains out of scope.
