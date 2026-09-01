# EvoSim Integration Audit & Large-File Truncation Plan

## Current status

**Phase 1 — executable runtime split: COMPLETE.**

The former monolithic `main.rs` has been decomposed into a normal Rust module tree. The binary now uses `main.rs` as its real crate entrypoint; the transitional `entry.rs` + `include!("main.rs")` arrangement has been removed. The split runtime is now organized across `state.rs`, `simulation.rs`, `perception.rs`, `memory.rs`, `movement.rs`, `transformation.rs`, `server.rs`, and `simulation_tests.rs`.

The repository's Rust CI passed `cargo test --all-targets` after the split (workflow run 49).

**Important verification note:** the GitHub file-retrieval tool and raw GitHub view produced a stale/mismatched representation of the old `main.rs` during the audit. The current repository state was therefore verified using the GitHub contents API and the post-change CI run rather than relying on the stale raw-file cache.

## Objective

Make the repository internally coherent before further feature work, while restructuring large source files so future GitHub/AI file retrieval cannot truncate the architectural context needed for safe edits.

This is an integration-first audit. No simulation rule is changed merely to make the code easier to split.

## Current repository inventory

The current `main` tree contains:

- Rust executable entry: `main.rs`
- Runtime state: `state.rs`
- Simulation orchestration: `simulation.rs`
- Perception/desirability: `perception.rs`
- Spatial memory: `memory.rs`
- Movement: `movement.rs`
- BREAK transformation lifecycle: `transformation.rs`
- HTTP/WebSocket/tick server: `server.rs`
- Simulation integration tests: `simulation_tests.rs`
- Large environment module: `environment.rs` (~38 KB)
- Large resource/chemistry module: `resources.rs` (~43 KB)
- Structural/chemistry support: `combine.rs`, `structural_combine.rs`, `structure.rs`, `contact.rs`, `connection_geometry.rs`
- Decision layer: `decision.rs`, `decision_runtime.rs`
- Supporting math/genome: `math.rs`, `genome.rs`
- Library target: `lib.rs`
- Frontend entry: `main.tsx`
- Rust CI: `.github/workflows/rust.yml`
- Two spec files: `Master Spec Sheet 3`, `Master Spec Sheet v4`

## Immediate integration findings

1. **Frontend is incomplete in the repository tree.** `main.tsx` imports `./index.css` and `./App.tsx`, but those files are not present in the current tree. The frontend therefore cannot currently be treated as an integrated build target.
2. **The decision runtime exists but is not wired into the simulation loop.** `decision_runtime.rs` exposes the bridge, while the simulation still performs direct movement and BREAK initiation. This remains an architectural integration gap, not a reason to delete the decision layer.
3. **The `entry.rs`/`include!("main.rs")` transition has now been removed.** The executable has a normal module tree and `main.rs` is a small startup boundary.
4. **`connection_geometry.rs` exists but is not declared by the executable module tree.** It remains an integration item for the structural audit.
5. **`environment.rs` and `resources.rs` remain above the practical safe-review size for repeated AI/GitHub retrieval.** They are the next large-file targets.
6. **Comments contain historical/spec references that need normalization.** Comments mentioning superseded Master Spec sections, old cloud pathways, or undecided behavior must be replaced with comments describing the current implementation and locked decisions.
7. **The current BREAK bootstrap is intentionally incomplete.** Fresh simulation material begins unbonded; BREAK requires bonded material; COMBINE is not yet connected to organism acquisition/structure formation. This is a known integration boundary and must remain explicit rather than being papered over with special-case energy creation.

## Locked rules that refactoring must preserve

- Energy is emergent from resource interactions, not a fundamental environmental resource.
- COMBINE interaction value: potential energy establishes energetic direction; reactivity and geometry modify magnitude.
- Surplus investment maps to bond strength through a capped diminishing-returns curve.
- BREAK depends on current structural/chemical state and may consume or release energy.
- Natural chemistry: compatible resources may combine; compatibility is not a hardcoded finite recipe list.
- Vents are indiscriminate transfer mechanisms; they do not prefer bonded over unbonded material.
- Layer 1 is the spatial deep reservoir; Layer 2 is the active field where organisms live and material/waste interact.
- Waste remains material in Layer 2 and is not automatically deleted or teleported to the reservoir.
- Physical structure is discrete structural units plus explicit bonds; bulk bonded material must not substitute for real structure.
- Decision history is bounded learned consequence history, not a physics cache and not fabricated prediction.
- Mechanical eligibility belongs to physical systems; decision logic chooses among mechanically eligible candidates.

## Executable sequence

### Phase 0 — Baseline integrity

1. Inventory every tracked file and module relationship.
2. Audit `Cargo.toml`, crate targets, module declarations, CI, and test entrypoints.
3. Establish a clean baseline with `cargo check`, `cargo test --all-targets`, and frontend build once the missing frontend files are restored/identified.
4. Record every existing compile/test failure before changing simulation behavior.

### Phase 1 — Split the executable runtime

**Complete.**

Replace the `entry.rs`/`include!("main.rs")` transition with a normal crate layout while preserving behavior.

Current structure:

- `main.rs` — process startup only.
- `state.rs` — application/runtime state and serializable simulation data structures.
- `simulation.rs` — initialization, environment stepping, tick orchestration, snapshots, conservation accounting.
- `perception.rs` — resource sensing/desirability.
- `memory.rs` — spatial memory.
- `movement.rs` — movement decision and movement execution.
- `transformation.rs` — BREAK initiation/resolution and energy consequences.
- `server.rs` — HTTP snapshot, WebSocket streaming, tick loop, bind/startup.
- `simulation_tests.rs` — simulation integration tests.

The split contains no `include!` workaround.

### Phase 2 — Split environment

Break `environment.rs` into:

- field representation and indexing
- diffusion
- reservoir representation
- vent transfer
- settling
- environment-level tests

Preserve the Layer 1/Layer 2 authority model and conservation behavior.

### Phase 3 — Split resources/chemistry

Break `resources.rs` into:

- immutable resource definitions/catalog
- material representation and arithmetic
- material property aggregation
- chemistry/combination calculations
- shape/connection-site definitions
- catalog construction and validation tests

Keep resource properties immutable and keep derived properties derived rather than duplicating mutable state.

### Phase 4 — Make structural integration explicit

Create one authoritative dependency path:

`resource geometry -> contact/compatibility -> formation evaluation -> bond creation -> structural state`

Ensure `connection_geometry.rs` is either explicitly integrated or removed if its functionality is duplicated elsewhere. Do not leave duplicate geometry implementations.

### Phase 5 — Runtime decision integration

Connect `decision_runtime.rs` to organism action selection without moving chemistry/geometry into the decision layer.

The runtime should pass:

- current needs from organism state
- mechanical eligibility from physical systems
- candidate action/context
- bounded decision history

Then the selected action invokes the existing physical executor. Decision code must not reimplement COMBINE/BREAK equations.

### Phase 6 — Integrate COMBINE without violating emergence rules

Connect raw acquisition -> raw storage -> physical instantiation -> geometry -> compatibility -> COMBINE interaction/work -> formation threshold -> bond strength -> structural state.

Do not create energy as a side effect merely to bootstrap the simulation. If the initial world cannot naturally reach bonded material yet, that remains a testable bootstrap limitation until COMBINE is genuinely wired.

### Phase 7 — Frontend integration

Restore/verify the actual frontend file tree referenced by `main.tsx`. Establish a minimal build that connects to:

- `/snapshot`
- `/ws`

The UI must remain read-only with respect to simulation mechanics unless a deliberate command API is later added.

### Phase 8 — Cross-system invariants

Add integration tests for:

- material conservation across reservoir/field/transformation/organism storage/structure
- vent transfer conservation
- diffusion conservation
- settling conservation
- acquisition conservation and physical reach
- COMBINE mass/material conservation
- bond formation geometry/threshold/strength
- BREAK energy accounting and waste return
- decision approval vs mechanical eligibility
- snapshot serialization
- WebSocket snapshot delivery
- deterministic seeded simulation behavior

### Phase 9 — Comment/spec normalization

After behavior is stable, rewrite comments in every touched file so they describe the current architecture rather than historical decisions. Remove obsolete section numbers and superseded cloud terminology unless retained explicitly as historical notes.

### Phase 10 — Final integration gate

Before feature expansion:

- `cargo fmt -- --check`
- `cargo check --all-targets`
- `cargo test --all-targets`
- frontend typecheck/build
- repository-wide search for obsolete names and duplicate pathways
- verify no large source file remains a single AI-review unit of approximately the current 40–70 KB scale

## Large-file rule going forward

No new core simulation file should be allowed to grow into another monolith. Split when a file contains multiple independently testable responsibilities, especially when it approaches ~20–25 KB or when retrieval of the full file is no longer reliable.

When a file must temporarily remain large, audit/edit it in explicit line-bounded sections and verify cross-section symbols after every change.

## Completion criterion

The repository is considered integration-ready only when the runtime, decision layer, chemistry, structure, environment, conservation diagnostics, and frontend can be traced through one coherent dependency graph, with no orphaned duplicate mechanism and no hidden behavior depending on historical comments/spec wording.
