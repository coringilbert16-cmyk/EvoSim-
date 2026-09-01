# EvoSim Integration Audit & Large-File Truncation Plan

## Current status

**Phase 1 — executable runtime split: COMPLETE.**

**Phase 2 — environment split: COMPLETE.**

**Phase 3 — resources/chemistry split: COMPLETE.**

**Phase 4 — structural integration: COMPLETE.**

**Phase 5 — runtime decision integration: COMPLETE.**

The repository now has a normal Rust module tree and the simulation tick is routed through the decision runtime before an organism executes an available action. Current needs come from organism state through tunable `DecisionParameters`; mechanical eligibility comes from the physical/runtime layer; bounded learned consequence history is carried by each organism; and selected actions invoke the existing MOVE/BREAK physical executors. COMBINE, ACQUIRE, and EXPEL remain mechanically unavailable until their physical executors are genuinely integrated, rather than being selected merely because a need exists.

The decision runtime does not contain chemistry, geometry, or COMBINE/BREAK equations. BREAK carries the selected context through its active transformation and records the actual resolved consequence into decision history. MOVE records its actual physical execution result immediately.

Rust CI passes `cargo test --all-targets` after the Phase 5 integration (workflow run 76).

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
2. **The decision runtime is now wired into the simulation loop.** The Phase 5 gap is closed: current needs, mechanical eligibility, candidate context, and bounded learned history now meet at the decision boundary before execution.
3. **The `entry.rs`/`include!("main.rs")` transition has now been removed.** The executable has a normal module tree and `main.rs` is a small startup boundary.
4. **`connection_geometry.rs` is integrated through the structural dependency path established in Phase 4.** It is no longer an orphaned duplicate geometry mechanism.
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

**Complete.**

Break `environment.rs` into:

- field representation and indexing
- diffusion
- reservoir representation
- vent transfer
- settling
- environment-level tests

Preserve the Layer 1/Layer 2 authority model and conservation behavior.

### Phase 3 — Split resources/chemistry

**Complete.**

Break `resources.rs` into:

- immutable resource definitions/catalog
- material representation and arithmetic
- material property aggregation
- chemistry/combination calculations
- shape/connection-site definitions
- catalog construction and validation tests

Keep resource properties immutable and keep derived properties derived rather than duplicating mutable state.

### Phase 4 — Make structural integration explicit

**Complete.**

Create one authoritative dependency path:

`resource geometry -> contact/compatibility -> formation evaluation -> bond creation -> structural state`

`connection_geometry.rs` is explicitly integrated through this path rather than remaining an orphaned duplicate implementation.

### Phase 5 — Runtime decision integration

**Complete.**

Connect `decision_runtime.rs` to organism action selection without moving chemistry/geometry into the decision layer.

The runtime now passes:

- current needs from organism state through tunable `DecisionParameters`
- mechanical eligibility from physical systems
- candidate action/context
- bounded decision history

The selected action then invokes the existing physical executor. Decision code does not reimplement COMBINE/BREAK equations.

The runtime records actual physical consequences after execution:

- MOVE records the immediate movement result.
- BREAK carries its selected context through the transformation and records the resolved energy/heat consequence.
- Unknown outcomes remain selectable; no predicted outcome is fabricated.
- Mechanically unavailable actions remain unavailable even when their associated need is active.

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