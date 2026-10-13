# PR audit and consolidation — 2026-10-13

## Scope and handling rules

This audit updates the 2026-10-12 lineage snapshot using current PR metadata, branch relationships, changed-file scope, and Actions evidence. It covers every currently open PR except **PR #200**, which is intentionally excluded from audit changes and consolidation at the user's direction, plus closed PRs directly related to active work. Older closed PRs outside the active lineage are historical work, not candidates for blind cherry-picking.

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
| #200 — sulfur parallelogram experiment | Explicitly excluded from this audit/consolidation | Leave completely untouched |

## Consolidation decision

1. Keep the #193 → #194 → #195 stack intact as the independently useful Bob/fluid/chemistry line. Focused evidence supports the listed bounded behaviors, while full-suite and Clippy status remain red.
2. Keep **one constructor workstream only**: #196 is the integration target, but it is frozen until a reliable baseline is restored. #190's unique viability contract is an input to consolidate into #196, not a second constructor to develop in parallel.
3. Before editing constructor behavior again, reproduce the known 28-failure comparison point on an exact commit, then isolate the regression boundary between that point and #196. Keep unrelated Bob/fluid/chemistry commits.
4. Classify the 69 failures into startup cascades versus independent failures. Require the scaffold-free initial organism to pass focused tests and restore the full suite to the established comparison point before expanding construction search or redesigning architecture.
5. Keep #186 draft-only and do not promote anything to `main` until repository-integrity and release gates pass.

## Audit limits

This is a status/lineage audit, not proof that every historical feature PR still behaves correctly today. Older merged PRs are not individually re-tested; current focused tests and the current full suite are the behavior evidence. The sulfur parallelogram PR #200 was not inspected or changed as part of this work.
