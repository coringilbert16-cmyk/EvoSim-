# EvoSim

EvoSim is a Rust-based continuous evolution simulation with a browser-served UI. The simulation models an environment, resource chemistry, organism structure, and organism behavior as interacting parts of one causal system.

## Run

Build and run the simulation with Cargo:

```bash
cargo run
```

The current executable is defined by `Cargo.toml` and remains at the repository root for now. A later repository reorganization will move the Rust source into `src/` once the current functional work is stable.

## Documentation

- `Master Spec Sheet V5.md` — current authoritative specification.
- `docs/CONTINUOUS_SIMULATION_ROADMAP.md` — implementation roadmap and planned simulation lifecycle.
- `INTEGRATION_AUDIT_PLAN.md` — integration and validation plan.
- `PHASE6_COMPLETE.md` — historical Phase 6 completion record.

## Repository layout

The repository is intentionally being stabilized before a larger source-tree reorganization. The current Rust modules remain at the repository root; `ui/` contains the served browser UI, `docs/` contains the continuous-simulation roadmap, and `scripts/` contains repository tooling.

## Development principle

Simulation behavior should be validated through the live lifecycle rather than only through tests that construct convenient artificial state. Changes to core simulation rules should therefore be accompanied by tests that demonstrate reachable behavior where practical.

<!-- CI verification marker. -->
