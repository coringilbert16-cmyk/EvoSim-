# EvoSim

EvoSim is an experimental 2D artificial-life simulation written in Rust. The project aims to let physical chemistry, energy exchange, geometry, perception, and evolution produce organism behavior through shared mechanisms rather than hard-coded organism roles.

This README is the current repository guide and implementation-status summary. Older dated handoffs are preserved verbatim in [README_HANDOFF_ARCHIVE_2026-10-22.md](README_HANDOFF_ARCHIVE_2026-10-22.md); treat them as historical evidence, not current instructions when they conflict with this page.

## Current repository state — 2026-10-22

- **Integration baseline:** PR [#202](https://github.com/coringilbert16-cmyk/EvoSim-/pull/202) is merged to `main` at `8c84978cd003738d94a74eef9114b711689c3804`.
- **Maintenance work:** [PR #203](https://github.com/coringilbert16-cmyk/EvoSim-/pull/203) is a draft based on that baseline. It contains narrowly scoped lint cleanup and documentation organization. It is not merged.
- **Known test baseline:** 563 passed, 28 failed, 1 ignored on the exact tested PR #202 source head. Comparison with the prior run found the same 28 failing test identities, with no new or resolved baseline failures. See [run #39697289682](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39697289682).
- **Strict Clippy:** `cargo clippy --locked --all-targets --all-features -- -D warnings -A clippy::unnecessary_sort_by -A clippy::unnecessary_map_or -A clippy::manual_is_multiple_of` still fails. Completed run [#39703652289](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39703652289), at code head `b6a63356179b3a03acfb9f20581f9f01e1e24843`, reports 36 application-target and 16 test-target errors, down from the earlier 42/21 checkpoint and baseline 59/38. The same 28 known full-suite test failures remain (563 passed, 28 failed, 1 ignored); their identities match the accepted baseline. Newer docs-only changes are not yet fully verified by exact-head CI. No blanket lint suppression is intended.
- **Not yet proven:** whole-world energy conservation across every storage, reaction, and lifecycle path; live sunlight/world-tone energy capture; and complete self-maintaining organism viability. Do not infer these from passing focused tests.
- **Separate work:** PR [#196](https://github.com/coringilbert16-cmyk/EvoSim-/pull/196) remains out of scope for PR #203. Do not import its constructor/Bob changes as part of maintenance cleanup.

## Design and implementation invariants

- **Organisms are more than cavities.** An enclosed genome cavity is a structural milestone, not proof of a viable organism. Long-term goals include cells, niches, populations, and later multicellularity emerging from interaction.
- **Shared mechanisms first.** Chemistry, energy, movement, stress, and post-death reactions should use consistent underlying mechanisms. “Decomposition” is chemistry after death, not a separate bond-breaking law.
- **Energy must be accounted for once.** Internal transfers cancel at world scale. Captured external input, usable energy, stored bond energy, work, and heat must reconcile. A spectrum or resonance amplitude is not itself physical energy.
- **World tone / sunlight:** source output is deterministic and directional; visibility and occlusion matter. Reflected tone means onward propagation with a potentially transformed frequency composition, not literal mirror-ray tracing. Live capture stays disabled until the allocation and conservation path is complete and verified.
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

- [x] Review exact-head CI at `1414744`: formatting, source-size and COMBINE architecture checks pass; focused Bob/fluid/chemical tests pass; full suite remains at the same 28 baseline failures (563 passed, 1 ignored).
- [ ] Continue small, audited Clippy fixes; investigate dead-code warnings before deleting or suppressing anything. Strict Clippy currently reports 44 application-target and 23 test-target errors.
- [x] Reconcile full-suite test failures by identity against the known baseline; no new or resolved failing identities.
- [ ] Improve repository documentation and source ownership without discarding historical evidence.
- [ ] Keep PR #196 separate until the stabilized baseline is understood and its exact-head behavior is independently reviewed.

## Specification authority

The project's design authority is **Master Technical Specification v2.4** (`EVOSIM-MASTER-2.4`, recall phrase “Master Spec 2.4”). This README is the repository's current implementation map and status record; it must reflect the specification and explicit later decisions, while dated handoffs remain historical evidence. If an implementation checkpoint and a design decision conflict, resolve the conflict explicitly rather than silently treating old notes as current.
