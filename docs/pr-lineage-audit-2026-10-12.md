# Pull-request lineage audit — 2026-10-12

## Scope and method

Audited PR metadata, branch ancestry, changed-file lists, current integration-branch source, and available GitHub Actions results for PRs #181–#190. The active source baseline is `integration/unified-source-and-catalogue` at `d9161b0a178a1bbd58fbd8db28c3c6d662abfb7e`. This is a source/lineage audit, not a claim that the application is healthy or all features are verified.

## Findings

### Prior constructor/Bob integration sequence: #181–#185

The tips of the branches associated with #181, #182, #183, #184, and #185 are ancestors of the current integration branch. GitHub commit comparison reports zero commits ahead for those branch tips relative to the integration branch. Their work is therefore present in the integration history; these closed PRs do not require blind cherry-picking.

- **#181 — constructor dependency audit:** the dependency map, candidate/transaction contract tests, Bob lookup/persistence repairs, and associated documentation are carried forward. The live constructor algorithm was not replaced by that PR.
- **#182 — build consistency:** its narrow compile-restoration and chemistry-routing changes are carried forward. This did not make the full test suite or Clippy green.
- **#183 — persistent Bob library root:** the shared geometry-library root and viewer/worker path work are carried forward. Still verify actual runtime configuration on a clean checkout when doing release validation.
- **#184 — Bob regeneration/storage:** worker regeneration improvements and the compositional-v3 experiment are in the lineage. Production adoption of the experimental storage format is a separate decision; the practice-only #179 work must not be mistaken for production format integration.
- **#185 — source integration attempt:** its source-tree integration work is part of the current integration history. The default `main` branch remains a separate integration/release problem; #186 remains the designated integration PR.

### Seed geometry fixes: #187 and #188

Neither PR is merged, and neither fix is present in the current integration source. The current `src/organism/juvenile.rs` still declares a ten-piece ring and the tightly wound polar spiral. The README's active handoff also says the seed-ring detour is separate from the active fluid sprint, which explains why the fixes were not carried forward.

- **#187** fixes the ring to six pieces and updates attachment indices, but it leaves the tightly wound spiral that later diagnostics showed can overlap occupied space.
- **#188** adds the follow-up physically spaced outward chain and is the better recovery starting point. However, its diff still calculates the Methane attachment angle with denominator `10.0` even though the ring has six sides; the Sulfur angles use the six-side denominator. Correct that inconsistency before recovery.
- The #188 workflow run failed. Its PR notes document that the ring-closure failure moved to a later overlapping element after the first correction; the follow-up change is plausible but is not validated by a green exact-head suite. Recover the intended calibration-fixture fix in a separate focused change, fix the angle denominator, and rerun targeted geometry tests plus exact-head CI. Do not loosen live geometry tolerances.

### Storage-format experiment: #179

#179 remains open and isolated on `geometry-storage-practice`; it is not an ancestor of the integration branch. The PR reports successful practice conversion and exact recursive reconstruction of 25,807 formations, with an estimated 22.02% five-file size reduction. This is useful evidence, not production adoption. Outstanding gates remain a Rust DTO/adapter, legacy signature and lookup equivalence, restart/reopen and safe atomic migration, complete family-type coverage, and load/memory/lookup benchmarks. Keep it separate until those gates are proven.

### Formatting and viability work: #189 and #190

- **#189** contains formatting-only changes in `src/bob/worker.rs` and `src/construction/runtime.rs`.
- **#190** independently carries the same two-file formatting corrections plus the new cavity/processing-space viability changes and README contract edits. The tree-level changes overlap, but #189 is not an ancestor of #190; they were developed as separate branches from the integration base. Avoid merging both blindly or treating #189 as an independent functional feature.
- Exact-head workflow run for #190 commit `2eaf05e67bd01a75365194635c424ef5c6725ffc`, run [38190208645](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38190208645), failed. Formatting, source-size, COMBINE architecture, Bob compact-storage tests, empty-library reopen, and focused chemical BREAK tests passed. Rust tests and Clippy failed during compilation/linting. The Clippy log includes broad existing strict-lint debt, including excessive precision, clone-on-copy, needless range loop/lifetimes, type complexity, unfulfilled lint expectations, and dead-code errors. No full test pass is claimed.
- #190 is still only a partial viability-contract sprint. It does not remove the fixed Carbon scaffold, fixed category gate, or arbitrary `1e12` construction-energy budget. The constructor must not be described as free-form until those are actually removed and the functional chemistry/reserve/integrity contract is implemented and tested.

## Reconciled status

| Work | Current disposition | Required next action |
|---|---|---|
| #181–#185 | Carried in integration ancestry | Do not cherry-pick blindly; retain as source history and validate behavior at relevant gates |
| #187 | Superseded/incomplete fix | Do not recover by itself |
| #188 | Unmerged follow-up; not present in integration | Recover the six-piece ring + spaced chain, fix Methane angle denominator, add/retain regression, rerun exact-head CI |
| #179 | Separate practice-only storage experiment | Keep isolated; finish production adapter/equivalence/performance gates before adoption |
| #189 | Formatting-only; overlapping tree changes with #190 | Avoid duplicate merge; once #190's disposition is clear, close or retain #189 explicitly |
| #190 | Active partial viability implementation; CI red | Diagnose build/test blockers; continue functional chemistry, reconciled reserve, integrity period, then free-form construction replacement |
| #186 | Sole integration target | Keep draft-only until repository-integrity, fluid-realization, and release gates are met |

## Recommended sequence

1. Recover and verify the #188 seed-calibration correction as a narrow, separate change. Fix the Methane angle denominator as part of that recovery.
2. Keep the Bob fluid-realization sprint in #186 as the integration priority: prove persisted multi-contact fluid records can be realized, rebased, physically validated, and transacted with conserved area and reconciled energy before any broad generation.
3. Resolve exact-head CI failures in short scopes. Keep focused-test, full Rust suite, and Clippy results as separate status claims.
4. Continue the approved free-form viability work in #190 only after tracing the existing chemistry and energy authorities. Do not invent a new reserve constant or restore the fixed scaffold to satisfy legacy tests.
5. Keep #179's storage optimization experimental until the production compatibility and performance gates are met.

No PR was merged, closed, or force-updated as part of this audit.
