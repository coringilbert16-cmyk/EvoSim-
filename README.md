# EvoSim

EvoSim is a Rust-based continuous evolution simulation with a browser-served UI. The simulation models an environment, resource chemistry, organism structure, and organism behavior as interacting parts of one causal system.

## Run

Build and run the simulation with Cargo:

```bash
cargo run
```

The Rust crate source lives in `src/`; `Cargo.toml` defines the `evosim` binary at `src/main.rs`. `ui/` contains the served browser UI.

## Documentation

- `Master Spec Sheet V5.md` — current authoritative specification.
- `CURRENT_STATE.md` — concise description of what the current repository implements.
- `docs/CONTINUOUS_SIMULATION_ROADMAP.md` — implementation roadmap and planned simulation lifecycle.
- `INTEGRATION_AUDIT_PLAN.md` — integration and validation plan.
- `PHASE6_COMPLETE.md` — historical Phase 6 completion record.

## Repository layout

- `src/` — Rust simulation and server source.
- `ui/` — browser UI assets.
- `docs/` — implementation roadmap and supporting documentation.
- `scripts/` — repository tooling.

## Development principle

Simulation behavior should be validated through the live lifecycle rather than only through tests that construct convenient artificial state. Changes to core simulation rules should therefore be accompanied by tests that demonstrate reachable behavior where practical.
