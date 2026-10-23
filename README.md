# EvoSim

EvoSim is an experimental 2D artificial-life simulation written in Rust. The project aims to let physical chemistry, energy exchange, geometry, perception, and evolution produce organism behavior through shared mechanisms rather than hard-coded organism roles.

This README is the current repository guide and implementation-status summary. Older dated handoffs are preserved verbatim in [README_HANDOFF_ARCHIVE_2026-10-22.md](README_HANDOFF_ARCHIVE_2026-10-22.md); treat them as historical evidence, not current instructions when they conflict with this page.

## Current repository state — 2026-10-23

- **Integration baseline:** PR [#202](https://github.com/coringilbert16-cmyk/EvoSim-/pull/202) is merged to `main` at `8c84978cd003738d94a74eef9114b711689c3804`.
- **Chemistry plan:** [PR #204](https://github.com/coringilbert16-cmyk/EvoSim-/pull/204) remains a documentation-only draft. Its current handoff records the approved invariants, phase ordering, and exact-head verification evidence.
- **Chemistry implementation:** [PR #205](https://github.com/coringilbert16-cmyk/EvoSim-/pull/205) remains a draft at `0f9992a80131a6a6b702a25c33e13dbf3571cdf2`. Recent work fixes the post-death rollback test comparison, adds stale-instance chemistry-cache rollback coverage, rejects invalid chemical-position cache identities, bumps the cache fingerprint, updates the zero-entry manifest, and adds dedicated CI gates.
- **Current exact-head verification:** workflow [#40021029556](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/40021029556) verified formatting and focused gates on the earlier head `4872942ff1c7aa47f1e961c312af888cb69b3951`, but its full Rust test step failed. The named failures still need comparison against the frozen baseline. PR #205 has since advanced to `0f9992a80131a6a6b702a25c33e13dbf3571cdf2`; a workflow for that exact head has not yet appeared. Formatting and focused gates through late physical-restoration rollback passed on the earlier head, including the cache persistence rollback gate. The full-suite failure is not yet classified as baseline or new; no full-suite or Clippy result is claimed for the current head. The previous verified head `5b88d3585bf6d3669fe2847fc886822d2f53010d` reported **612 passed, 23 failed, 1 ignored** in 250.67 seconds, with no new failure identities against baseline [#39705628427](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39705628427); five baseline failures were resolved.
- **Strict Clippy:** the previous normalized path/diagnostic comparison found no added or removed identities (34 application-target and 15 test-target diagnostics). The current workflow has not reached Clippy yet; the two `EnergyLedger` clone-on-copy sites were removed, and the disconnected legacy recycler has a narrowly scoped expectation. Current diagnostics still need inspection.
- **Still unproven:** global material/energy conservation across all live transitions; live sunlight/world-tone energy capture; availability of stored/developing holders for ordinary chemistry after death; canonical aggregate ownership and atomic reproduction/division. Passing focused tests do not prove these.
- **Separate work:** PR [#196](https://github.com/coringilbert16-cmyk/EvoSim-/pull/196) remains out of scope for the chemistry implementation. Keep constructor/Bob changes separate.

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

- [x] Verify earlier PR #205 focused transaction gates and compare full-suite failure identities against the frozen baseline; no new failure identities, five baseline failures resolved. Current-head full-suite comparison is pending.
- [x] Verify COMBINE rollback restores the exact usable-energy and full ledger before-state; legacy serialized transactions retain the delta fallback.
- [x] Verify environmental BREAK rollback rejects corrupt source indices without mutating environmental inventory.
- [x] Verify the post-death regression preserves material/energy inventory and prevents reproductive construction progress without activating legacy `DecomposingBody`.
- [x] Stage finished-body energy settlement and commit it only with a successful atomic physical-material deposit; focused rollback gate passed on an earlier PR #205 head.
- [x] Gate late physical-restoration rollback independently; the destination-preservation regression passed on earlier verified heads. Current full Rust suite and Clippy remain pending.
- [ ] Establish an ordinary-chemistry path for stored/developing holders after death without a separate decomposition law or loss of exact ownership.
- [ ] Prove global material/energy conservation and live sunlight/world-tone allocation; do not infer completion from the current focused gates.
- [ ] Keep PR #196 constructor/Bob work separate from PR #205 chemistry changes.

## Specification authority

The project's design authority is **Master Technical Specification v2.4** (`EVOSIM-MASTER-2.4`, recall phrase “Master Spec 2.4”). This README is the repository's current implementation map and status record; it must reflect the specification and explicit later decisions, while dated handoffs remain historical evidence. If an implementation checkpoint and a design decision conflict, resolve the conflict explicitly rather than silently treating old notes as current.
