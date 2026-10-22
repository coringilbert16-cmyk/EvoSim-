# Chemistry Unification — Current Handoff

**Updated:** 2026-10-22  
**Plan commit:** `0f1d9b16d05e495bdc3e07e62415de1fa0448b87`  
**PR #205 head audited:** `12d8079230458da68a8dc3755a1d6a9b41347862`  
**CI:** [39833438679](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39833438679).

PR #204 is documentation-only. The detailed owner decisions, audit register, and evidence are in `docs/chemistry-audit-unification-plan.md`, Sections 14–17. No implementation source/tests changed during this audit series.

## Rules and verification

- Frozen baseline `main`: `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5`; run 39705628427: 563 passed, 28 failed, 1 ignored. Compare failure identities.
- Resource ID 6 is Oxygen. PR #196 constructor/Bob stays separate. One focused audit before each code change; each command/test max five minutes.
- Parent remains sole owner of body, storage, committed material, developing graph/bonds, and associated energy until division. Death stops biological processing, not physical existence; no special decomposition path. Ownership does not automatically imply reaction or sunlight eligibility.
- Workflow 39833438679 at PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`: formatting/focused gates passed; 585 passed, 27 failed, 1 ignored. All 27 failures match baseline identities minus one resolved baseline failure. Clippy diagnostic identities match baseline. Suite remains baseline-red; no performance or whole-world conservation proof exists.

## Confirmed gaps

1. Ordinary chemistry only enumerates primary graph bonds; stored/committed and developing holders are outside its reaction pass. Stored PhysicalMaterial connections have no allocated bond energy.
2. Stored BREAK can commit pieces partially, ignore failed fallback deposits, and clear escrow on malformed completion.
3. Contained acquisition and expulsion have unchecked failed-destination return paths.
4. Movement mutates geometry before ledger settlement; rejected settlement can leave an uncharged move. Movement cost is heat-only/zero explicit work pending D7 reconciliation.
5. Construction reserve transfer uses an atomic primitive but ignores the return value.
6. World-tone frame balance/staged reservoir commit are present, but exact-small-world accuracy/performance is open. Section 17.38 records unresolved receiver eligibility for parent-owned non-primary surfaces; do not change it without owner clarification.
7. Prepared COMBINE computes escaped-tone/resonant-held deltas but does not apply/rollback them; current path should leave them zero. Live-frame settlement does not update the documented unabsorbed-throughput telemetry.
8. Chemistry cache schema 4 and strict fingerprint/manifest checks exist. Static potential depends on chemical positions and K/Dmax, but ChemistryKey lacks numeric positions, so the manual fingerprint must change with catalog chemistry. Reaction accumulation can retain stale keys when an organism has no bonds.
9. Live world tone uses `settle_world_tone_paths`; `EnergySourceBudget` is staged/tested, not the live exposure authority. `EnergyLedger.transfer` has a live construction caller that ignores rejection and a legacy recycler that must remain unintegrated.

## Next

Section 17.55 consolidates source→holder→sink paths; Section 17.56 maps energy APIs. Finish the remaining call-site/failure matrix before implementation. Highest-priority fixes after audit closure: stored BREAK atomic ownership; checked acquisition/expulsion returns; movement geometry/ledger atomicity; bondless key cleanup; catalog fingerprint and prepared-ledger invariants; construction-transfer rejection behavior. Re-fetch PR #205 head and CI before any code change. Do not claim chemistry unified until Section 17.6 release gates have evidence.

Future reproduction remains separate: begin an adjacent second genome core at 100% original size, grow to 160% total, split into two roughly 80%-size children.
