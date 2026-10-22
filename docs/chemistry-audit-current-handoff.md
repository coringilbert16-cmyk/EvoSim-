# Chemistry Unification — Current Handoff

**Last reconciled:** 2026-10-22  
**Plan commit:** `435172dcf50fb820340257349310b5dbfafabf7a`  
**Implementation PR #205 head audited:** `12d8079230458da68a8dc3755a1d6a9b41347862`  
**Latest exact-head workflow:** [39833438679](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39833438679), complete.

This is the concise handoff. The full scientific decisions, audit register, evidence and dated findings live in `docs/chemistry-audit-unification-plan.md`, especially Sections 14–17. PR #204 is documentation-only. No source or test files have changed during the current audit series.

## Governing decisions

- Frozen baseline: `main` at `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5`; workflow [39705628427](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39705628427): 563 passed, 28 failed, 1 ignored. Compare named failures, not counts alone.
- Resource ID 6 is Oxygen. PR #196 constructor/Bob work stays separate.
- Owner-approved aggregate model (Section 17.31): the parent owns its primary structure, stored and committed material, developing structure/bonds, and associated energy until division. Core breach ends biological processing but does not transfer ownership or create a separate decomposition object. Dead organisms remain physical and continue ordinary chemistry.
- Do not infer that aggregate ownership automatically makes every holder chemically available. Do not treat a stored PhysicalMaterial connection as a funded bond: it has endpoints/topology but no allocated numeric bond energy.
- Workflow preference: one focused audit before each code change. Hard limit five minutes per command/test. Do not change source/tests until audit templates are complete.

## Exact-head verification

Workflow 39833438679 on PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`:
- Formatting and focused gates passed; chemistry-library partial-append rollback/reopen test passed.
- Full suite: 585 passed, 27 failed, 1 ignored in 283.74 seconds. All 27 failing identities match the frozen baseline's 28 failures, minus the containment-to-storage failure that remains resolved. No new failing identity observed; suite remains baseline-red.
- Strict Clippy: 34 application-target and 15 test-target diagnostics. Normalized comparison found no new or removed diagnostic identities.
- No representative performance benchmark has been run. Whole-world material/energy conservation is not proven.

## Confirmed findings from Sections 17.32–17.48

1. **Reaction reachability:** live structural chemistry visits only bonds in the parent's primary graph. Stored/committed material and developing-graph bonds are not enumerated. Their energy/topology representations differ, so do not broaden the pass blindly.
2. **Stored BREAK atomicity:** the source is held in transformation escrow. Split pieces are stored one at a time; rejected storage falls back to a field deposit whose result is ignored. Malformed completion can clear the active marker without restoring escrow. Fallback coordinates are not proven world-relative.
3. **Contained acquisition / expulsion:** acquisition removes field sources before storage and ignores failed fallback deposits. Expulsion removes the stored source and does not restore the original if the final environment deposit rejects.
4. **Movement:** geometry/occupancy/environment mutate before ledger settlement. If settlement rejects, the organism has moved without paying and movement progress is not decremented. Movement's energy cost is currently all heat and zero explicit work; reconcile that with D7 before changing equations.
5. **Stronger patterns:** COMBINE bond formation builds trial geometry, settles energy, then commits; maintenance adds stress only after successful settlement; chemical/resonance BREAK stages reservoir withdrawal and graph state, settling before commit.
6. **World tone:** every path checks incident energy = retained resonance + heat + escaped energy; frame reservoirs are staged until ledger settlement succeeds. Exact-small-world geometry/error and scale benchmarks remain open. Active transformation sources are included only after owner/active-ID checks.
7. **Receiver rule needs owner clarification:** D5 defines source budget, occlusion, frequency-shifted onward propagation and the fast sweep requirement, but not whether parent-owned non-primary surfaces can credit the parent's resonance reservoir. Section 17.38 records aggregate-wide versus primary-structure-only capture. Do not change the live mapping without a decision.
8. **Tick order:** active transformations prepare/resolve; living set is sampled; world tone settles; biological work runs; ordinary chemistry runs for all organisms; post-chemistry breach prevents later same-tick behavior; construction advances for living parents; stress damage runs near tick end. A chemistry-caused breach can still receive that tick's earlier world-tone input. Test this boundary before altering ordering.
9. **Independent oracles:** integer material inventory counts field, organism structure/storage, committed/developing construction, transformation escrow, and legacy carriers once. Energy snapshot recomputes current usable/resonant/graph-bond stores independently of cached held totals. It does not independently reconstruct every historical source event.
10. **Cache / accumulation:** chemistry cache schema 4, equation fingerprint, strict manifest/record rejection and append/manifest rollback tests are present. The fingerprint's catalog marker must be kept current by maintainers. `chemical_reaction::accumulate` returns early on bondless structures before pruning old per-organism keys; stale activation entries can persist as population grows. This is activation state, not physical energy.

## Phase status

- Phases 0–1 recorded.
- Phase 2 oracles implemented, but cross-domain failure coverage incomplete.
- Phase 3 has material identity/restore guards; holder-return atomicity remains open.
- Phase 4 cache schema/fingerprint and selected rollback/reopen tests exist; broader restart coverage remains open.
- Phase 5 ledger/initial source started; global energy proof open.
- Phases 6–9 are partial: bond settlement, tick consumers, post-death holder reachability, world-tone receiver eligibility, exact-small-world comparison and performance are not release-ready.
- Phases 10–11 remain open.

## Next actions

1. Finish mapping remaining energy/material mutation and cache/catalog invalidation call sites; close the source→validation→settlement→commit/rollback matrix.
2. Keep the confirmed candidate order for after audit closure: (a) stored BREAK all-or-nothing ownership; (b) checked contained acquisition/expulsion destination returns; (c) movement geometry/ledger atomicity; (d) bondless reaction-accumulation cleanup.
3. Ask for/record the receiver-eligibility decision before changing world-tone mapping.
4. Add focused failure-injection and independent inventory tests for each approved fix. Re-fetch PR #205 head and current CI before any implementation change.
5. Keep PR #204 handoff and phase tracker current. Do not merge or claim chemistry unified until Section 17.6 release gates have evidence.

## Separate future reproduction requirement

Internal/adjacent budding begins a second genome core at 100% of original size, grows to 160% total, then splits into two roughly 80%-size children. Current reproduction code does not yet satisfy this. It remains separate from chemistry PR #205 unless the owner explicitly changes scope.


## Latest cache dependency audit — 2026-10-22

- Plan commit: `11a7c12d5cacc6f27c78a9fed640fe2a5a0695dc`.
- Cached static potential depends on resource chemical_position values and CHEMICAL_K / CHEMICAL_D_MAX. ChemistryKey contains resource names and interface identity but not the numeric positions, so the manifest fingerprint is the invalidation boundary for catalog chemistry changes. Current fingerprint includes a catalog-version marker maintained manually; tests must prove a chemical-position change requires a version/fingerprint bump and rejects the old cache.
- Contact distance, contact radius, facing/engagement, and physical endpoints are evaluated live; the cache supplies only static potential and cannot authorize a bond. The fingerprint currently includes some live-only parameters too, causing conservative extra invalidation rather than incorrect hits.
- Next: update the handoff and PR description with the latest plan commit, then finish the remaining call-site/impact matrix before any implementation.
