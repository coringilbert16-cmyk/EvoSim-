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

- **Implementation PR #205:** [Unify chemistry, material ownership, and energy accounting](https://github.com/coringilbert16-cmyk/EvoSim-/pull/205), branch `chemistry-unification-implementation-2026-10-22`. Code checkpoint immediately before this documentation update: `a2fa38b2a34392bfdc2321b52e3d47cae4145d5d`; exact-head workflow [40221483333](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/40221483333) is running. Formatting passes on that head. The latest completed full suite on the same code semantics, head `c514d321f90fd3b73096c763503e9f677d455751`, reported 716 passed, 23 failed, 1 ignored. This is not a green suite; the canonical seed-realization gate and other existing geometry/viability failures remain.
- **Unified energy inventory:** the simulation snapshot now calls shared holder-accounting helpers for usable, resonant, and structural energy. Aggregate-owned construction is counted through the parent's authoritative structure and energy only; partial physical ownership invalidates the inventory instead of silently choosing a holder.
- **Material conservation checks:** the independent integer-material inventory counts environmental stock and physical objects, organism structures and storage, committed construction material, detached legacy construction, transformation escrow, and decomposing bodies. It rejects partial construction ownership. Regression coverage includes field-to-storage transfer, restoration, anchor admission into the aggregate, and rejection of ambiguous ownership.
- **Energy conservation checks:** anchor admission is tested against the actual simulation ledger, including retained usable/resonant/structural energy plus recorded work and heat. That regression now passes; it caught and corrected a test that had been using a detached ledger and therefore missed the COMBINE work/heat entries.
- **Death-transfer accounting:** the staged recycling helper validates retained resonance and transfers it into the decomposing body's energy budget before clearing the organism's reservoir. A focused regression confirms the retained energy transfers exactly once. The canonical live post-death lifecycle and whole-world conservation across all growth, movement, reaction, and division paths are not yet proven.
- **Canonical 12N realization:** the 12-piece corner-free nitrogen cavity remains the correct seed; do not restore the obsolete 16-piece ring. The full-organism realization test still reports that the realized structure contains no constituents outside the cavity boundary. Do not weaken the viability contract or change the seed geometry to hide this failure. Constructor/Bob redesign remains separate from this chemistry-unification work.
- **Architecture status:** aggregate-owned reproductive construction uses stable physical IDs as its sole physical authority; its persisted `developing_structure` stays empty and its separate `developing_energy` stays zero. Detached legacy construction remains serializable and separately counted until fully migrated. The staged `partition_subgraph_for_division` primitive preserves stable constituent IDs, keeps child-internal bonds and energy with the child, removes crossing bonds from both graphs, and returns crossing-bond energy for explicit ledger settlement. Successful growth/division conservation and the full lifecycle migration remain unfinished. Parent-child contact still requires integration with the shared physical bond-formation transaction. See [PR #204's living plan](https://github.com/coringilbert16-cmyk/EvoSim-/pull/204).
- **Documentation plan PR #204:** latest known plan update is `66447f9c0a03c7918193f370090ae294247c07ea`; it records exact-head evidence, the baseline seed failure, and the CI sequencing fix. Clippy remains failing under repository warnings-as-errors; do not report it green.

## Design and implementation invariants

- **Organisms are more than cavities.** An enclosed genome cavity is a structural milestone, not proof of a viable organism. Long-term goals include cells, niches, populations, and later multicellularity emerging from interaction.
- **Shared mechanisms first.** Chemistry, energy, movement, stress, and post-death reactions should use consistent underlying mechanisms. “Decomposition” is chemistry after death, not a separate bond-breaking law. For the survival-first organism milestone, stress inputs are temporarily scaled to zero; maintenance energy costs, chemistry, resonance, and physical damage mechanisms remain in place. Revisit stress and death after basic survival is demonstrated.
- **Energy must be accounted for once.** Internal transfers cancel at world scale. Captured external input, usable energy, stored bond energy, work, and heat must reconcile. A spectrum or resonance amplitude is not itself physical energy.
- **World tone / sunlight:** source output is deterministic and directional; visibility and occlusion matter. Reflected tone means onward propagation with a potentially transformed frequency composition, not literal mirror-ray tracing. Live capture is active through the precomputed world-tone frame and energy ledger; whole-world conservation across all holders, death transitions, and reaction paths is not yet proven.
- **Resonance time / phase:** one simulated day is 24,000 physics ticks; the reference tone is 440 oscillations per simulated day. Every oscillator starts at phase zero (peak) at tick zero, and phase is derived analytically from the shared simulation tick when a signal is evaluated. Runtime `ticks_per_second` controls execution pacing only. The 2,400-tick world-source orbital cycle is separate from carrier oscillation.
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
