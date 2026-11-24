# EvoSim

EvoSim is an experimental 2D artificial-life simulator written in Rust. It explores how physical geometry, chemistry, energy exchange, and eventually biological organization can arise from shared simulation rules.

This repository contains the simulator, its browser viewer, chemistry data, experiment workflows, and technical documentation.

## Start here

- **New to the project?** Read this page, then [the documentation index](docs/README.md).
- **Want to run the simulator?** Follow [Getting started](#getting-started).
- **Want to understand the design?** Start with [Architecture and specifications](docs/README.md#architecture-and-design).
- **Want to run chemistry experiments?** Read [the experiment guide](experiments/README.md).
- **Want a controlled chemistry workbench?** See [the Chemical Laboratory](docs/chemistry-laboratory.md).

## Getting started

### Requirements

- Git
- Rust toolchain (stable), including Cargo
- Windows users can use the included launcher; other platforms can start the server with Cargo.

### Windows

1. Clone the repository:
   ```powershell
   git clone https://github.com/coringilbert16-cmyk/EvoSim-.git
   cd EvoSim-
   ```
2. Install Rust if it is not already installed: https://www.rust-lang.org/tools/install
3. Run `run_evosim.cmd`.
4. Open http://127.0.0.1:3000/ in a browser if it does not open automatically.

The first build can take several minutes. The launcher manages the local running build and its update workflow. To create a desktop shortcut, run `install_evosim_desktop_shortcut.cmd`.

### macOS and Linux

From the repository directory:

```bash
cargo run --release
```

Then open http://127.0.0.1:3000/ in a browser.

### Chemical Laboratory

The separate controlled workbench uses the same Rust chemistry engine and starts with an empty environment:

```bash
cargo run --bin chemistry-lab
```

Open http://127.0.0.1:3030/. The workbench supports exact-coordinate element/molecule placement, live flow and established energy-source controls, molecule inspection, bounded reaction/lineage history, energy-ledger diagnostics, and reproducible setup import/export. See the [laboratory guide](docs/chemistry-laboratory.md) for controls and current API/scope.

### Build and test

```bash
cargo build --release --locked
cargo test --locked
cargo fmt --all -- --check
```

The full test suite and lint status must be judged from the latest exact-commit GitHub Actions run. Do not assume that a successful build means every test or Clippy check is green.

## Repository map

| Path | What belongs here |
|---|---|
| `src/` | Rust simulator: chemistry, materials, geometry, organisms, perception, construction, and runtime |
| `ui/` | Browser viewer assets embedded by the Rust server |
| `chemistry_library/` | Persistent chemistry-library data |
| `experiments/` | Experiment request configuration and instructions |
| `tools/` | Offline Python analysis and audit utilities |
| `tests/` | Tests for the Python analysis tools |
| `scripts/` | Repository checks and experiment helpers |
| `docs/` | Architecture, specifications, audits, plans, and historical records |
| `.github/workflows/` | CI and explicitly triggered experiment workflows |

Rust unit and contract tests are also colocated under `src/tests/` and compiled through the shared Rust library.

## Running chemistry experiments

The reproducible environmental chemistry workflow is documented in [experiments/README.md](experiments/README.md). It is manually requested through GitHub Actions or the checked-in experiment request file. The default request is inactive. Experiment runs preserve raw diagnostics and generated analysis artifacts; they do not change the simulator's ordinary runtime behavior.

## Project design and current truth

The project is developed in stages: physical simulation and chemistry first, followed by persistent chemical organization and later biological systems. A molecule, cavity, or recurring reaction pattern is not by itself proof of metabolism, an organism, or self-sustainability.

Use the documentation index to find the relevant design authority. In particular, the [resonance-field implementation roadmap](docs/design/resonance-field-implementation-roadmap.md) is the controlling detailed roadmap for resonance-field work. Historical audits and PR plans are evidence of earlier investigations, not automatically current implementation instructions.

Project status is evolving. Treat source code and the latest exact-head CI results as evidence of what currently works; treat specifications and roadmaps as intended behavior unless implementation and tests establish otherwise. Resonance-field progress is tracked in the [controlling roadmap](docs/design/resonance-field-implementation-roadmap.md). The live ambient-energy model is implemented and merged in PR #439: 24,000 ticks per day, a 440-cycle carrier, a 10 → 1 → 10 cycle-averaged energy-density envelope, and explicit opening/current/generated/withdrawn inventory accounting. Phase 3C has started in PR #440: the exact envelope-kinetic amplitude correction and combined ambient/disturbance energy, cross-flux, and world-drive composition are now implemented in Rust. The next implementation slice is the production adaptive spatial grid and live tick/query integration, reusing the validated leapfrog reference without repeating propagation calculations. Phase 3D then adds passive material interaction through physical geometry, followed by movement/chemistry integration, measured optimization, and retirement of the legacy resonance path.

## Contributing and making changes

- Keep chemistry, energy accounting, geometry, and material ownership invariants explicit.
- Separate experiments and diagnostics from production behavior.
- Avoid unrelated source refactors in chemistry or physics changes.
- Read [the documentation index](docs/README.md) before relying on an audit or plan.
- Do not describe the repository as fully green or product-ready without current verification.

## License

See the repository's license information if/when one is added. No license file is currently present.