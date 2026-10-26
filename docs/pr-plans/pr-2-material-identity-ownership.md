# Enforce Resource Identity and Integer Material Inventory Invariants

> **Implementation in progress.** PR #207 is now authorized to make focused production and acceptance-test changes after the audit of merged `main`.

## Summary

Enforce canonical resource identity, indivisible integer quantities, and independently auditable material inventories across environmental stock, storage, and transaction transfers. Do not redo the parent aggregate graph/construction/division ownership migration tracked by PR #212.

## Outline

- [x] Audit remaining material representations and mutation paths against current main.
- [x] Confirm the shared `Material` validator rejects fractional per-resource quantities, unknown names, and duplicate catalog identities.
- [x] Extend the independent inventory oracle to reject ambiguous catalog identities.
- [x] Make logical and physical field deposits validate catalog identity, report rejection truthfully, and preserve state on invalid payloads or indices.
- [ ] Extend independent per-resource inventory and transaction rollback coverage for remaining owner paths.

## Acceptance criteria

- [ ] Every material unit has one authoritative physical owner.
- [ ] Transfers, storage, and consumption reconcile without duplication or disappearance.
- [ ] Rejected transactions preserve material state and inventory totals.
- [ ] Independent checks detect discrepancies without relying solely on implementation accounting helpers.

## Guardrails

- Start with one focused audit of the current merged `main` before implementation.
- Use PR #205 merge commit `10521705d8f921ceb071b17240f1b7e069f52f1a` as the forward baseline; compare named test failure identities, not only totals.
- Keep each command and test run within five minutes; a timeout is incomplete evidence.
- Keep this PR limited to its primary invariant. Do not fold in unrelated cleanup.
- PR #204 is the roadmap/handoff; PR #196 constructor/Bob work is out of scope.


## Cross-PR boundary and dependency clarification (2026-10-26)

This phase owns **material identity and ownership**, not reaction scheduling or energy policy. Its inventory must cover environmental stock, cell/organism storage, physical constituents, and temporary transaction escrow exactly once. It must not implement a second environmental material owner or reaction engine.

**Dependency:** use the PR #205 forward baseline. This is a prerequisite for the shared chemistry runtime and transactional reaction realization; it does not by itself make free environmental materials chemically active. Keep PR #206 movement proof and PR #212 aggregate-ownership migration separate.


## Current-main audit and first implementation slice (2026-10-26)

The post-#212 audit found that the independent test oracle in `src/tests/simulation_tests.rs` already walks environmental aggregates, realized environmental objects, organism structure/storage, aggregate-owned construction reservations, active transformation escrow, and decomposition carriers by resource name. It rejects unknown names and non-integer counts without reading the energy/material production ledger.

The first confirmed mutation-path defect is in `ActiveMaterialField::deposit`: its logical-material branch returned success even when `deposit_at_index` silently rejected an empty or invalid payload. The indexed deposit also lacked a bounds check and could panic for an invalid index. The first implementation slices make deposit APIs require the authoritative catalog, reject unknown/ambiguous identities for both logical stock and realized physical objects, report rejection truthfully, and preserve state on invalid payloads or indices. Focused regressions cover these rejection paths.

Remaining audit focus: confirm every transfer/escrow owner is covered by independent per-resource inventory tests, audit consumption and aggregation paths for silent loss or identity drift, and compare exact failure identities against the post-#212 baseline. This first slice does not claim the whole PR acceptance gate is complete.

## Third implementation pass — count and merge boundaries (2026-10-26)

The shared `Material` validator now rejects positive sub-unit quantities and counts at or beyond the independent inventory oracle's `u64` representation boundary. `Material::take` also rejects sub-unit extraction requests before touching its source. The independent inventory oracle independently rejects empty material records and sub-unit constituents rather than rounding them down to zero.

Environmental logical-stock deposit now stages a merged free-material value and validates the combined composition before assigning it to the cell. This closes a conservation hole where two individually valid deposits could overflow the inventory representation when merged. A regression verifies rejected merges preserve both cell contents and the field revision.

## Fourth implementation pass — expulsion source preflight (2026-10-26)

Physical expulsion now rejects an unrealized source or invalid catalog-bound composition before geometry work and before source removal. A regression verifies an unknown resource identity remains in its original storage owner and leaves environmental cells unchanged. Containment acquisition applies the same catalog-bound validation before taking a physical object out of the environmental field, with a regression for malformed persisted identity. Stored BREAK also has a regression for a fractional persisted source: reservation is rejected without changing storage, activating a transformation, or consuming an ID.

These are verified code-level safeguards, not a claim that all ownership/rollback paths or the full acceptance gate are complete. Remaining work is to finish the owner-path audit, run a clean compilation/test pass, compare failures to the post-#212 baseline, and only then decide whether the PR is ready for review.


## Second implementation pass — removal-boundary audit (2026-10-26)

The follow-up audit found that storage insertion and environmental deposits validated catalog identity, but several source-removal paths trusted a previously stored object without revalidating it. The COMBINE source take, stored BREAK reservation, and indexed physical-storage take now validate the material against the authoritative catalog and reject invalid whole-count state before removing the source. Regression tests assert that an unknown resource identity is rejected without changing the storage owner; stored BREAK also preserves the transformation ID and active-operation state on rejection.

The remaining work is to finish verification of the complete branch, reconcile exact test failures against the post-#212 baseline, and close any additional verified transfer or rollback gaps. These changes improve transaction safety but do not yet establish that PR #207 is complete or merge-ready.

## Fifth implementation pass — per-resource overflow within composites (2026-10-26)

The ownership audit found a gap between the independent inventory oracle and the shared material validator: the oracle checked overflow while aggregating repeated occurrences of the same resource, but Material::is_valid only bounded each constituent entry independently. A composite could therefore contain multiple individually representable Carbon counts whose combined per-resource count exceeded the oracle's u64 range and still pass some storage/deposit checks.

The shared validator now accumulates each resource's constituent counts with checked integer addition. Repeated resource identities remain valid when their aggregate fits; overflow is rejected before the material can pass catalog-bound validation. Regression coverage exercises both cases. The post-change workflow still needs to execute against this new head before this pass is considered verified.
