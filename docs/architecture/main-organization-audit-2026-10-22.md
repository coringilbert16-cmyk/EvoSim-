# Main-branch organization audit — 2026-10-22

## Scope and method

Audited the current `main` commit `8c84978cd003738d94a74eef9114b711689c3804` (post-PR #202) by inspecting its tracked tree, root README, crate/module registry, Cargo manifest, CI workflows, launcher scripts, and documentation layout. This is an organization audit, not a request to change `main` directly. Changes arising from it belong on the separate draft PR #203 unless explicitly approved otherwise.

## Current shape

- 108 tracked files; the root has seven files, including Cargo metadata, the main README, and three Windows launchers.
- Rust source is already grouped under `src/{bob,chemistry,construction,geometry,infrastructure,materials,organism,perception,simulation,tests,ui}`.
- `src/main.rs` is a crate-level module registry using path attributes and a CLI dispatcher. It is not merely an entry-point function.
- Tests remain under `src/tests/` and beside implementation modules where private crate internals are needed.
- Persistent chemistry data is under `chemistry_library/data/`; there is no tracked top-level `geometry_library/` directory in this main-branch tree. Bob owns geometry-library code and runtime knowledge.
- `docs/` has 14 files at its root, covering architecture, audits, plans, and transaction notes, but no documentation index on main yet.
- The root README is approximately 336 KB because it contains extensive dated handoff/status history. PR #203's concise README plus verbatim `README_HANDOFF_ARCHIVE_2026-10-22.md` is a strong, low-risk organization improvement: current guidance becomes discoverable without destroying historical evidence.
- Largest implementation files include `src/bob/library.rs` (~289 KB), `src/geometry/source_exposure.rs` (~162 KB), `src/construction/runtime.rs` (~126 KB), `src/geometry/capillary_geometry.rs` (~103 KB), and `src/chemistry/combine_runtime.rs` (~79 KB).

## Ranked opportunities

### P0 — Keep current-state documentation canonical

**Action:** Keep the short root README as the current source of truth, the old README as a dated archive, and the documentation index as the navigation layer. Keep CI checkpoints exact-head-specific and clearly distinguish current verification from historical runs.

**Why:** Main's 336 KB README mixes current instructions with many dated checkpoints. This is the highest-impact organization issue and is already addressed in draft PR #203. Do not delete the archive or wholesale rewrite its history.

### P1 — Correct the repository organization map

**Finding:** The existing organization plan describes a top-level `geometry_library/` and root-level `tests/`, neither of which exists in the audited main tree. Actual paths are `src/bob/`, `src/tests/`, and `chemistry_library/data/`.

**Action:** Correct the map to reflect current paths and distinguish the persistent chemistry data directory from executable code. Mark unverified chemistry cache hit/miss behavior and write/refresh authority as open questions, not completed architecture.

### P1 — Make documentation migration incremental

**Finding:** The 14 root-level docs are a mix of architecture, plans, audits, and mechanics. The proposed categories (architecture, mechanics, libraries, decisions, audits) are useful, but moving all files at once creates relative-link and history risks.

**Action:** Use an index first. For future moves, select one category at a time, search all references, move the file and update every link atomically, and keep one canonical copy. Historical audit text should remain visibly historical. The first low-risk grouping candidates are dated cross-PR audits and subsystem plans; do not move until their incoming links are inventoried.

### P2 — Split oversized modules only by responsibility

**Finding:** Bob's library, source exposure, construction runtime, and capillary geometry are much larger than the rest of the source tree.

**Action:** Use the separate [large-module boundary audit](architecture/large-module-boundaries.md) for the three modules it covers. Add a similar dependency/caller audit for `src/construction/runtime.rs` before considering any extraction. Preserve module visibility and colocated tests; don't combine refactoring with behavior changes or Clippy suppression.

### P2 — Clarify the experimental workflow's lifecycle

**Finding:** `.github/workflows/sulfur-parallelogram-experiment.yml` is an 11 KB, one-PR comparison workflow that targets the dedicated fix branch and also checks the experiment head name. It appears intentionally scoped, not a general CI gate.

**Action:** Preserve it while the experiment branch or its evidence may still be needed. Once branch and artifact/history requirements are checked, either archive the workflow's rationale in `docs/` and remove an obsolete workflow, or rename/comment it so its experiment-only status is obvious. Do not silently delete it as routine cleanup.

### P3 — Keep runtime launch code cohesive

**Finding:** `run_evosim.ps1` is ~18.5 KB and contains Git/worktree, build/staging, process lifecycle, health check, install, and rollback functions. Those functions participate in one guarded update-and-launch lifecycle; size alone is not evidence they should be separate scripts.

**Action:** First add/retain focused Windows runner validation. If maintenance becomes difficult, split only after documenting shared state and rollback invariants. The current `.cmd` launchers are small and should stay thin.

### P3 — Defer crate module declaration modernization

**Finding:** `src/main.rs` has many explicit path declarations, but these preserve the existing crate-level module names during staged organization and support tests that rely on private internals.

**Action:** Do not mass-convert declarations to nested modules as part of documentation/Clippy cleanup. Treat any future move as a separate audited change with import, visibility, test, and architecture-guard verification.

## Recommended sequence

1. Correct the inaccurate organization map and link this audit from the documentation index.
2. Keep the concise README/archive/index model and refresh verification status only from completed exact-head runs.
3. Audit documentation incoming links before any physical moves.
4. Finish targeted Clippy cleanup separately from file extraction.
5. Audit `src/construction/runtime.rs` boundaries and the experimental workflow lifecycle before deciding whether either should change. The construction-runtime boundary audit is now documented; code extraction remains deferred.
6. Split only one large source responsibility at a time, with focused tests and full CI comparison against the accepted baseline.

## Non-goals

- Do not change `main` directly as part of this audit.
- Do not fold PR #196 constructor work into PR #203.
- Do not delete historical audit evidence, generated knowledge without a persistence audit, or staged APIs solely to reduce file count/lint output.
- Do not interpret the 28 known full-suite failures as new regressions without comparing test identities.
