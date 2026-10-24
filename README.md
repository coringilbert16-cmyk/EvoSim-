# EvoSim

EvoSim is an experimental 2D artificial-life simulation written in Rust. The project aims to let physical chemistry, energy exchange, geometry, perception, and evolution produce organism behavior through shared mechanisms rather than hard-coded organism roles.

This README is the current repository guide and implementation-status summary. Older dated handoffs are preserved verbatim in [README_HANDOFF_ARCHIVE_2026-10-22.md](README_HANDOFF_ARCHIVE_2026-10-22.md); treat them as historical evidence, not current instructions when they conflict with this page.

## Current repository state — 2026-10-24

- **Integration baseline:** PR [#202](https://github.com/coringilbert16-cmyk/EvoSim-/pull/202) is merged to `main` at `8c84978cd003738d94a74eef9114b711689c3804`.
- **Maintenance work:** [PR #203](https://github.com/coringilbert16-cmyk/EvoSim-/pull/203) is a draft based on that baseline. It contains narrowly scoped lint cleanup and documentation organization. It is not merged.
- **Known test baseline:** 563 passed, 28 failed, 1 ignored on the exact tested PR #202 source head. Comparison with the prior run found the same 28 failing test identities, with no new or resolved baseline failures. See [run #39697289682](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39697289682).
- **Strict Clippy:** `cargo clippy --locked --all-targets --all-features -- -D warnings -A clippy::unnecessary_sort_by -A clippy::unnecessary_map_or -A clippy::manual_is_multiple_of` still fails. Exact-head run [#39705150387](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39705150387), at `67bceda1672891e7e390cf242295477bfb5cf3a0`, reports 34 application-target and 15 test-target errors, improved from the earlier 36/16 checkpoint and baseline 59/38. The full suite reports 563 passed, 28 failed, 1 ignored; all 28 failing test identities match the accepted baseline run [#39697289682](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39697289682), with no new or resolved baseline failures. Formatting, source-size, COMBINE architecture, and focused Bob/fluid/chemistry checks pass at this head. Strict Clippy remains an acknowledged follow-up; no blanket lint suppression is intended.
- **Not yet proven:** whole-world energy conservation across every storage, reaction, and lifecycle path; live sunlight/world-tone energy capture; and complete self-maintaining organism viability. Do not infer these from passing focused tests.
- **Separate work:** PR [#196](https://github.com/coringilbert16-cmyk/EvoSim-/pull/196) remains out of scope for PR #203. Do not import its constructor/Bob changes as part of maintenance cleanup.

### Current chemistry-unification implementation checkpoint

- **Implementation PR #205:** [Unify chemistry, material ownership, and energy accounting](https://github.com/coringilbert16-cmyk/EvoSim-/pull/205), branch `chemistry-unification-implementation-2026-10-22`, current source head `3a5f4d5448ce86855d9782a52b0a83834e94b7b0`. Exact-head workflow [40107767032](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/40107767032) is running; the formatting and test-assertion issues seen on the preceding division-primitive attempt have been corrected in the newer head, with the full workflow now progressing through focused gates. The canonical seed realization gate remains a known baseline failure.
- **Most recent completed full-suite identity comparison:** source head `fa04de9fe8d15ce358509cc09282f0f3ebffd631`, workflow [40103516613](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/40103516613): 692 passed, 21 failed, 1 ignored; all 21 failure identities match frozen baseline [39705628427](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39705628427), zero new identities, seven baseline failures resolved. The former current-only ordinary-tick energy-inventory residual now passes. This is not a green suite.
- **New Phase 8 holder regression:** `breached_organism_retains_storage_and_construction_without_biological_progress` passes on workflow [40104819765](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/40104819765), along with the focused energy-source, world-inventory, and transaction-rollback gates.
- **Canonical 12N realization:** the 12-piece corner-free nitrogen cavity remains the correct seed; do not restore the obsolete 16-piece ring. The full-organism realization test is still a known baseline failure, now reporting that the realized structure contains no constituents outside the cavity boundary. Do not weaken the viability contract or change the seed geometry to hide this failure. Constructor/Bob redesign remains separate from this chemistry-unification work.
- **Architecture status:** stable-ID graph append/remap, selected-subgraph extraction, atomic subgraph replacement, and the staged `partition_subgraph_for_division` primitive now have targeted regression coverage. The division primitive preserves stable constituent IDs, keeps child-internal bonds and their energy with the child, removes cross-boundary bonds from both graphs, and returns their stored energy for explicit ledger settlement. Its exact-head focused CI gate is running. This is a graph primitive, not live division: runtime still keeps a separate `developing_structure` and `developing_energy`; canonical aggregate ownership, parent-energy-funded growth, single-counted world-tone/movement geometry, and live atomic division remain unfinished. Parent-child contact currently checks geometric overlap rather than a physical bond; integration must use the shared bond-formation transaction. See [PR #204's living plan](https://github.com/coringilbert16-cmyk/EvoSim-/pull/204).
- **Documentation plan PR #204:** latest known plan update is `66447f9c0a03c7918193f370090ae294247c07ea`; it records exact-head evidence, the baseline seed failure, and the CI sequencing fix. Clippy remains failing under repository warnings-as-errors; do not report it green.

## Design and implementation invariants

- **Organisms are more than cavities.** An enclosed genome cavity is a structural milestone, not proof of a viable organism. Long-term goals include cells, niches, populations, and later multicellularity emerging from interaction.
- **Shared mechanisms first.** Chemistry, energy, movement, stress, and post-death reactions should use consistent underlying mechanisms. “Decomposition” is chemistry after death, not a separate bond-breaking law.
- **Energy must be accounted for once.** Internal transfers cancel at world scale. Captured external input, usable energy, stored bond energy, work, and heat must reconcile. A spectrum or resonance amplitude is not itself physical energy.
- **World tone / sunlight:** source output is deterministic and directional; visibility and occlusion matter. Reflected tone means onward propagation with a potentially transformed frequency composition, not literal mirror-ray tracing. Live capture is active through the precomputed world-tone frame and energy ledger; whole-world conservation across all holders, death transitions, and reaction paths is not yet proven.
- **Geometry ownership:** a structural unit represents one physical constituent. Composite materials must retain per-constituent placements and bond identity; never invent transforms from a cached composite shape.
- **Bob is a geometry library worker/viewer.** Prefer stored/reusable library results over repeated runtime geometry discovery. Preserve the corner-free 12-piece nitrogen seed; do not revert to the obsolete 16-piece ring.
- **Repository safety:** audit before each change, keep changes small, verify the exact PR head, compare test failure identities against the known baseline, and do not delete dead code until its architectural role is understood.

## Repository map

- `src/chemistry/` — chemical reactions, transactions, resource accounting, and energy ledger.
- `src/geometry/` — realized geometry, contact, cavities, material-shape reasoning, and structure authority. `material_geometry.rs` lives here; there is no separate `src/material_geometry/` directory.
- `src/bob/` — geometry-library generation, persistence, lookup, and viewer-facing support.
- `src/construction/` and `src/organism/` — construction, organism state, decisions, and reproduction.
- `src/perception/` — harmonic response, movement, and related perception logic.
- `src/materials/` — resource definitions, physical-material handling, storage, transfer, and restoration.
- `docs/README.md` — documentation index, grouped by architecture, geometry, chemistry, Bob, and organism-development topics.
- `docs/` — focused architecture, implementation, and audit documents.
- `scripts/`, `.github/workflows/` — repository checks and automation.
- `ui/` — user-interface assets.

`src/main.rs` is also the crate-level module registry: explicit `#[path]` declarations preserve existing module names while implementation files are grouped by responsibility. The `src/tests/` directory contains crate-internal contract and simulation tests; the top-level `ui/` contains browser assets, while `src/ui/` contains Rust-side visualization support. For current feature-level details, start with the [documentation index](docs/README.md), then use the focused document and most recent dated checkpoint above. The handoff archive retains earlier decisions and evidence for traceability.

## Build and verification

Rust toolchain: stable. Dependencies are pinned by the tracked `Cargo.lock`; use `--locked` so dependency resolution does not drift.

```sh
cargo fmt --all -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

The full test suite is currently **not green**. A maintenance change is acceptable only when exact-head CI has completed and the result has been compared with the known baseline; matching the baseline is not the same as clean CI. Focused workflow gates are useful but do not substitute for the full suite.

## Active maintenance checklist

- [x] Maintain exact-head CI evidence and compare failure identities against the frozen baseline before attributing regressions.
- [x] Add independent Phase 5 world-tone source-budget, strict no-creation, and ordinary-tick energy-inventory checks; the former cumulative-rounding residual is resolved on the last completed full-suite head.
- [x] Add Phase 8 regression proving a breached organism retains its material and energy holders without biological construction progress; focused gate passes.
- [ ] Complete canonical aggregate ownership migration: stable-ID membership in the parent's single physical graph, parent-held energy for construction, and no duplicate world-tone/movement geometry.
- [ ] Implement atomic aggregate division only after attachment/growth and cross-boundary bond-energy settlement are coherent; the 160% scale threshold enables but does not force division.
- [ ] Finish whole-world energy/material reconciliation and the remaining baseline failures. Latest exact-head full-suite run is pending; strict Clippy remains red.
- [ ] Keep PR #196 constructor/Bob work separate from chemistry-unification changes.

## Specification authority

The project's design authority is **Master Technical Specification v2.4** (`EVOSIM-MASTER-2.4`, recall phrase “Master Spec 2.4”). This README is the repository's current implementation map and status record; it must reflect the specification and explicit later decisions, while dated handoffs remain historical evidence. If an implementation checkpoint and a design decision conflict, resolve the conflict explicitly rather than silently treating old notes as current.
