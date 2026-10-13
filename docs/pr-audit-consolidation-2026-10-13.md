# PR audit and consolidation — 2026-10-13

## Scope and handling rules

This audit updates the 2026-10-12 lineage snapshot using PR metadata, branch relationships, changed-file scope, and Actions evidence. It covers open PRs and closed PRs directly related to active work. PR #200 was merged after the original audit text below was written; the post-merge reconciliation section at the end is the current status where it conflicts with the earlier snapshot. Older closed PRs outside the active lineage are historical work, not candidates for blind cherry-picking.

No PR was merged, force-updated, or closed as part of this audit. The integration branch `integration/unified-source-and-catalogue` remains the sole integration target and `main` remains untouched.

## Evidence-based status

### Demonstrated by focused tests (not equivalent to whole-project health)

- **Bob compact persistence / empty-library reopen / concurrent initialization:** focused gates passed on run [#7148](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38272687270), on chemistry-persistence head `728e455b`.
- **Fluid multi-contact transaction and three-piece cap:** focused gates passed on the same stacked run.
- **Bounded fluid coalescence and restoration:** focused gates passed on the same stacked run. Supported case is deliberately narrow: two single-part, same-resource fluid constituents with valid exact multi-contact boundaries sharing one complete reversed SolidContact segment. Unsupported topologies are expected to reject atomically; this is not a general polygon boolean solver.
- **Chemical BREAK focused checks:** passed on run #7148.
- **Chemistry-library transactional persistence:** targeted tests in the full-suite log passed, including stale-instance refresh/concurrent writers and manifest-failure rollback/reopen. These prove those tested cases, not all persistence failure modes.
- Formatting, source-size policy, and COMBINE architecture checks passed on run #7148.

Run #7148's full Rust suite still failed: **402 passed, 28 failed, 1 ignored**; strict Clippy also failed. This is the best recorded recent pre-constructor-rewrite comparison point, not a green build.

### Known failing / unproven

- **PR #196 — constructor rewrite:** latest exact-head run [#7338](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38438644278), commit `095fbc34`, reports **364 passed, 69 failed, 1 ignored**; full Rust suite and Clippy both fail. Constructor diagnostics report all explored placement attempts in the examined run rejected for overlap, with zero geometry coverage misses, restoration failures, COMBINE rejections, or energy rejections. This makes assembly-context collision / candidate-selection and search strategy the immediate demonstrated blocker in that run. It does not prove every rejected placement is wrong or that a successful global assembly is impossible.
- The 28-versus-69 comparison is a serious regression signal but not a controlled one-to-one test comparison: the test inventory and code revision changed. Do not claim all 41 extra failures are independent constructor defects. Earlier analysis found 52 of the 69 failures cascade from startup failing to construct an initial organism; the remaining failures need independent classification.
- **PR #190 — free-form viability contract:** remains partial and separate from #196. Its cavity/processing-space changes and contract decisions are not yet proven as part of a successful production constructor. It must not be treated as an alternate complete constructor or merged blindly because it overlaps constructor runtime and README changes.
- **PR #186 — integration target:** remains draft-only. Older green focused gates do not establish a green current full-suite/release baseline.
- **PR #179 — storage format practice:** remains experimental only. Reported size savings and exact reconstruction checks do not establish production DTO compatibility, legacy lookup equivalence, migration/reopen safety, full family coverage, or performance.
- **PRs #187–#188 — seed geometry recovery:** remain unmerged/not integrated according to the prior lineage audit. Do not reintroduce their prescribed seed topology into the constructor; any reusable fixture/regression should be assessed independently against the current scaffold-free requirement.

## PR inventory and consolidation disposition

| PR | Status / evidence | Disposition |
|---|---|---|
| #186 — source and Bob integration | Sole integration PR; full integration/release gates not green | Keep as the only integration target; draft-only |
| #193 — Bob fluid multi-contact | Focused Bob/fluid/COMBINE gates pass in stacked descendants | Retain as upstream feature history; no blind cherry-pick |
| #194 — bounded fluid coalescence | Focused coalescence/restoration gates pass in stacked descendants; intentionally bounded scope | Retain stacked on #193 |
| #195 — transactional chemistry-library persistence | Focused tests pass; full suite and Clippy red | Retain stacked on #194; do not confuse focused success with green CI |
| #196 — constructor strip/rewrite | Full suite 69 failures; Clippy red; production startup fails to find a valid assembly | Freeze feature work here pending regression recovery; keep as the single constructor integration target for now |
| #190 — free-form viability contract | Partial unique viability changes; not merged into #196; overlapping runtime/README edits | Do not merge independently. Port its still-valid user contract and tested changes into #196 deliberately, then close/retire #190 only after equivalence is verified |
| #191 — PR lineage audit | Its October 12 snapshot predates #193–#196 evidence | Treat this document as the updated evidence; keep old report historical |
| #197–#198 — constructor diagnostics | Closed without merge and explicitly superseded by #196 | Keep as history; do not reopen or merge |
| #189 — formatting-only constructor/Bob edits | Overlaps later work; no independent functional value established | Do not cherry-pick separately |
| #187–#188 — seed ring calibration | Not integrated; fixed seed topology conflicts with current scaffold-free direction | Historical evidence/fixtures only; no wholesale recovery |
| #179 — geometry storage compaction | Practice-only; no production adapter or compatibility gate proven | Keep isolated until production gates are met |
| #181–#185 and older closed work carried by integration ancestry | Prior lineage audit found those branch tips already ancestors of integration | Preserve as history; do not cherry-pick duplicates |
| #200 — sulfur parallelogram experiment | Merged into #194 at `412a2e8b`; downstream stack and integration ancestry not yet reconciled | Preserve the merged change; verify downstream exact heads before integration |

## Consolidation decision

1. Keep the #193 → #194 → #195 stack intact as the independently useful Bob/fluid/chemistry line. Focused evidence supports the listed bounded behaviors, while full-suite and Clippy status remain red.
2. Keep **one constructor workstream only**: #196 is the integration target, but it is frozen until a reliable baseline is restored. #190's unique viability contract is an input to consolidate into #196, not a second constructor to develop in parallel.
3. Before editing constructor behavior again, reproduce the known 28-failure comparison point on an exact commit, then isolate the regression boundary between that point and #196. Keep unrelated Bob/fluid/chemistry commits.
4. Classify the 69 failures into startup cascades versus independent failures. Require the scaffold-free initial organism to pass focused tests and restore the full suite to the established comparison point before expanding construction search or redesigning architecture.
5. Keep #186 draft-only and do not promote anything to `main` until repository-integrity and release gates pass.

## Audit limits

This is a status/lineage audit, not proof that every historical feature PR still behaves correctly today. Older merged PRs are not individually re-tested; current focused tests and the current full suite are the behavior evidence. The sulfur parallelogram PR #200 was not inspected or changed as part of this work.


## Exact-head follow-up — 2026-10-13

A fresh source-scope review of the current PR heads confirms the #193 → #194 → #195 stack is intact and all three full Rust suites remain red, while focused gates pass:

- **#193 current head** `f879de00d008561fcd82ce27fb2fa0349c8373c0`, run [#7143](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38270282801): **377 passed, 28 failed, 1 ignored**; Clippy failed.
- **#194 current head** `b3bc66eb17c3ae4b51c1a49bf1febbcdae7ad46d`, run [#7144](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38270352595): **400 passed, 28 failed, 1 ignored**; Clippy failed.
- **#195 current head** `728e455b780ce18e4117112c075921ec55ec88c2`, run [#7148](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38272687270): **402 passed, 28 failed, 1 ignored**; Clippy failed.

These are exact-head results for the respective PR tips, not green CI. The increasing pass count with the same 28 failing tests is encouraging but does not establish that the failures are harmless or that the branches are merge-ready.

### #193 scope review and safe consolidation boundary

Current #193 changes ten files. The central multi-contact behavior spans `src/construction/runtime.rs`, `src/geometry/contact.rs`, `src/geometry/structure.rs`, and fluid–fluid eligibility in `src/chemistry/combine.rs`; those changes are behaviorally connected and should not be split apart casually. `src/bob/library.rs` adds unique temporary manifest/frontier paths and a concurrent initialization regression; `src/bob/worker.rs` is only test formatting in the current diff.

Two additional areas are not intrinsically part of the multi-contact transaction:

1. `src/organism/juvenile.rs` changes the seed baseline from a ten-piece ring plus tight spiral to a six-piece ring plus outward chain and adjusts peripheral Sulfur/Methane anchors. This is seed/construction calibration, not the multi-contact transaction itself.
2. `src/construction/initial_organism_constructor.rs` changes a missing/unopenable persistent Bob library from a silently ignored optional lookup to a hard construction error. That may be intentional fail-closed behavior, but it changes constructor startup behavior and must be judged with constructor recovery, not assumed safe merely because the Bob gate passes.

**Do not perform a blind file-level revert or move these hunks now.** PR #196 is already stacked on #195 and edits overlapping constructor, runtime, geometry, and juvenile files. Moving hunks without a tested parent/child reconstruction risks losing fixes or creating a misleading test baseline. The safe decision is to preserve current source history, keep all three PRs draft, and prepare a separately tested scope-split only after a clean commit-level dependency map and targeted regressions are available. No source code was changed during this follow-up.

### Next safe action

Before any code movement, compare the individual commits in #193 against its parent and the overlapping changes in #196. Identify which exact commits/hunks own seed calibration and constructor startup policy, and verify whether the latter is required for the persisted-Bob path. Then isolate only independently movable changes on a new review branch and run the focused Bob/fluid gates plus constructor startup tests. If isolation cannot be demonstrated without altering physical invariants or destabilizing the stack, keep the stack intact and treat the scope issue as documented rather than forcing a split.


### Commit-lineage detail from follow-up review

The PR commit history confirms the two separable concerns are embedded in the shared ancestry, not isolated side branches:

- Seed calibration was ported into #193 by commit [`a8c00d89`](https://github.com/coringilbert16-cmyk/EvoSim-/commit/a8c00d890595c1a04dcc5039c5b7a4b68e9c67a0) (message: “Consolidate seed geometry calibration into Bob integration branch”). It changes the juvenile seed ring/chain and peripheral material anchor layout.
- Strict constructor startup behavior was added at #193's current tip by [`f879de00`](https://github.com/coringilbert16-cmyk/EvoSim-/commit/f879de00d008561fcd82ce27fb2fa0349c8373c0) (message: “Use one strict Bob library open in initial constructor”). It moves the persistent Bob open into the constructor's error path instead of treating the library as optional.
- #194 and #195 descend from #193; #196 descends from #195. Therefore both commits are in the lineage inherited by the downstream stack. A revert on #193 would change the parent history for every downstream branch and would not, by itself, cleanly isolate either concern from #196's overlapping files.

This makes a file-level split less safe than it first appears. Do not rewrite the current stack just to improve PR scope. If a narrower feature branch is still needed, reconstruct it from the integration base with an explicit allowlist of behavior commits, then prove it with the focused transaction, persistence, and constructor startup gates before changing any existing PR bases.


## Stack reconciliation — 2026-10-13

PR #200 merged into #194 at `412a2e8b795e9a2b107455eb899a25cba1a6a4ec`. The sulfur change is now represented in the downstream stack through explicit two-parent merge commits.

### Reconciled heads

- #194 head: `412a2e8b795e9a2b107455eb899a25cba1a6a4ec`.
- #195 head: `22b7da1cbc740c65e43937ea62f43d792b473db2`; parents are old #195 head `728e455b` and sulfur merge `412a2e8b`. The chemistry-library transaction code, rollback/concurrency tests, and sulfur cache-stability regression are present together.
- #196 head: `b0d51d899fd027ce609fc38d71d248968fcc9872`; parents are old constructor head `095fbc34` and reconciled #195 `22b7da1c`. The constructor branch retains its own code while incorporating the sulfur geometry and related regression coverage.
- #186 integration head before this documentation update: `a7406db5eefecd88f7c401b007d05f26e8877ab0`; it remains separate from the feature stack and draft-only.

### Exact-head evidence

- Reconciled #195 full Rust suite: **405 passed, 28 failed, 1 ignored**. Direct comparison with the pre-merge #195 run shows identical sets of 28 failing test names, with no new named failures. Focused Bob/fluid/chemistry gates passed. Clippy fails.
- Isolated two-piece Bob comparison on #195 is still in progress; no new catalogue parity claim is made.
- Reconciled #196 exact-head Rust run [#7377](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38476203144): **367 passed, 69 failed, 1 ignored**. Direct comparison with its pre-merge run shows exactly the same 69 failing test names, with no new named failures. Sulfur geometry, Bob-signature, and chemistry-cache regressions pass; Clippy fails.
- README and this audit are the authoritative current status for the stack. Older entries below are historical snapshots where their status conflicts with this section.

### Next safe action

1. Inspect the completed Bob comparison and reconciled #196 exact-head run.
2. Confirm the sulfur-specific shape, Bob-signature, and chemistry-cache regressions pass on the reconciled #196 head.
3. Only after exact-head evidence is recorded, decide whether the library line can advance toward integration. Keep constructor feature work frozen and keep #186 draft-only until the established release gates pass.
