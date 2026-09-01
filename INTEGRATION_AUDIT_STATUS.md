# EvoSim Integration Audit Status

## Purpose

This branch is the structural-hardening pass. The primary constraint is preserving simulation behavior while making the repository easier to audit, transmit, and extend without hitting large-file truncation.

## Current source-size risks

The current repository contains two source files that exceed the decomposition target and should not receive more unrelated responsibilities:

- `main.rs` — approximately 1,745 lines.
- `environment.rs` — approximately 1,151 lines.

The guard in `scripts/check_source_sizes.py` treats 900 lines as a decomposition warning and 1,200 lines as a hard failure. This intentionally leaves headroom below the point where code review, context transfer, or GitHub/LLM presentation becomes unreliable.

## Runtime boundary

`Cargo.toml` currently points the binary at `entry.rs`. `entry.rs` is only a thin module-loading shim and uses `include!("main.rs")`. That is a structural liability: the executable entrypoint is indirectly coupled to the entire simulation file, and the module graph is harder to reason about than a normal Rust binary/library split.

Target architecture:

```text
entrypoint
  -> runtime
      -> simulation
          -> environment
          -> organism/decision systems
          -> interaction/transformations
      -> snapshot/API/WebSocket
```

The extraction must be behavior-preserving. No chemistry or evolution rules should change as part of this refactor.

## Environment boundary

`environment.rs` currently combines at least these responsibilities:

1. active material field/grid storage;
2. diffusion;
3. deep reservoir storage and spatial mapping;
4. vents;
5. settling;
6. tests for all of the above.

These are natural extraction boundaries. The first extraction should isolate the active field from reservoir/vent orchestration because the simulation already treats the active field as the Layer 2 authoritative state.

## Important integration finding: vent semantics

The current `environment.rs` implementation still contains the older vent behavior that preferentially draws bonded reservoir material and falls back to raw material while marking the emitted material as bonded. The comments explicitly describe this behavior.

This conflicts with the current locked vent decision: the vent is an indiscriminate transfer mechanism and must not choose bonded over unbonded material or manufacture a bonded state merely because the material passed through a vent.

This is a **behavioral correction**, not a decomposition task. It must be handled as a separate audited change after the module boundary is stabilized, with conservation and bonded-status tests covering both reservoir stacks.

## Current transformation boundary

`main.rs` owns perception, memory, movement, acquisition, BREAK initiation, BREAK resolution, simulation stepping, snapshot generation, HTTP, WebSocket handling, and integration tests. This is too much responsibility for one file.

The first safe extraction sequence is:

1. runtime transport (`tick loop`, HTTP snapshot, WebSocket);
2. simulation state/snapshot types;
3. organism perception/memory/movement;
4. acquisition and transformation mechanics;
5. integration tests.

Each extraction must leave the simulation tick order unchanged.

## Locked decisions that must not be reopened during this refactor

- COMBINE interaction value: potential energy establishes direction; reactivity and geometry modify magnitude.
- Surplus-to-bond-strength: capped diminishing returns.
- BREAK energy depends on current structural/chemical state and may consume or release energy.
- Vent transfer is indiscriminate; the vent does not prefer bonded or unbonded material.
- The active field is Layer 2; the deep reservoir is Layer 1.
- Waste returns to the active field rather than disappearing or being silently restored to the abstract reservoir.

## Verification gate for every extraction

1. `cargo fmt --all -- --check`
2. `cargo check --all-targets`
3. `cargo test --all-targets`
4. Search for references to the old module/path.
5. Verify the runtime endpoint contract remains `/snapshot` and `/ws`.
6. Verify no legacy resource-cloud pathway is reintroduced.

## Truncation policy

No new responsibility should be added to a source file at or above 900 lines. Any file reaching 1,200 lines fails CI and must be decomposed before additional feature work continues.
