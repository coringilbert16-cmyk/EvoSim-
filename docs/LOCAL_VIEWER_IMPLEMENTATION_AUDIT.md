# EvoSim Local Viewer / Runner Implementation Audit

## Audit status

**Date:** 2026-09-27

**Scope:** Audit the current repository against `ui/EVOSIM_LOCAL_VIEWER_PLAN.md` before implementation.

**Rule:** This audit is architectural. It does not run the long GitHub Actions suite and does not modify EvoSim biological behavior.

---

# 1. Executive finding

The repository is **partially prepared for the viewer**, but the current runtime is still a tightly coupled prototype:

- Rust already owns a continuous tick loop.
- Axum already serves the existing UI.
- The UI already consumes observation endpoints.
- Simulation state already contains most of the information needed for a rich microscope.
- Most state components are already Clone/Serde-capable.
- However, the current server owns the simulation directly, binds only to localhost, has no control API, has no historical state, has no event stream, and has no multi-client/version-control layer.
- The existing UI polls HTTP snapshots every 250 ms and currently renders a mixture of authoritative geometry and coarse visual projections.
- The current headless runner is finite and diagnostic-oriented rather than a persistent runtime.
- PR selection cannot safely be implemented inside the current single-process architecture without first separating the persistent control plane from the simulation process.

**Conclusion:** Do not throw away the existing UI/server. Refactor the runtime boundary first, then extend the existing observation layer.

---

# 2. Current runtime architecture

## 2.1 Current executable

`src/main.rs` currently has one binary.

Normal invocation calls:

`server::run()`

`--headless` calls:

`simulation_runner::run_from_args()`

The same Rust crate therefore contains:

- simulation,
- HTTP server,
- browser UI,
- headless diagnostics runner.

This is useful for the current prototype but is not sufficient for safe Main/PR switching.

## 2.2 Current server

`src/server.rs` currently:

- creates `Simulation::new(42, 10.0)`,
- wraps it in `Arc<parking_lot::Mutex<_>>`,
- starts a Tokio tick loop,
- serves the UI from embedded files,
- exposes observation HTTP endpoints,
- binds to `127.0.0.1:3000`.

Current observation endpoints:

- `/`
- `/observation/status`
- `/observation/world`
- `/observation/organism/{id}`
- `/observation/structure/{id}`
- `/observation/resources`

There is currently no WebSocket endpoint in the current `main` branch.

## 2.3 Current tick loop

The current loop:

1. checks `running`,
2. derives a sleep duration from `ticks_per_second`,
3. sleeps,
4. locks the simulation,
5. calls `step()`.

The Simulation already contains:

- `tick`,
- `ticks_per_second`,
- `running`.

However, there are currently no HTTP/UI controls that mutate these fields.

The default runtime is 10 ticks/second.

---

# 3. Current simulation state audit

## 3.1 State is already rich enough

`src/state.rs` currently contains authoritative state for:

- organism identity,
- position/developmental origin,
- occupied cells,
- genome,
- harmonic spectrum,
- memory,
- decision history,
- usable energy,
- stress,
- maintenance debt,
- stress threshold,
- stored material,
- physical structure,
- development stage,
- active transformation,
- reproductive construction,
- structural/position revisions,
- cached derived values.

`Simulation` contains:

- tick,
- speed,
- running state,
- organisms,
- environment,
- active transformations,
- decomposing bodies,
- energy ledger,
- organism/transformation ID counters,
- ChaCha8 RNG,
- decision parameters.

This is a strong foundation for snapshots and rewind.

## 3.2 Snapshot feasibility

The current `Simulation` now derives `Clone` on the runtime foundation branch; it remains intentionally non-Serde so the authoritative simulation state does not acquire a second persistence representation.

However, the audit found that the major state components are already Clone/Serde capable:

- `Environment`: Serialize/Deserialize/Clone.
- `Organism`: Serialize/Deserialize/Clone.
- `ActiveTransformation`: Serialize/Deserialize/Clone.
- `ReproductiveConstruction`: Serialize/Deserialize/Clone.
- `MaterialStorage`: Serialize/Deserialize/Clone.
- `PhysicalMaterial`: Serialize/Deserialize/Clone.
- `DecomposingBody`: Clone.
- `DecisionParameters`: Serialize/Deserialize/Clone/Copy.
- `ToneSpectrum`: Serialize/Deserialize/Clone.
- `ActiveMaterialField`: Serialize/Deserialize/Clone.
- `ChaCha8Rng` is designed as the deterministic RNG used by the simulation and is Clone-capable through its dependency API.

This means a complete-state rolling history is technically feasible without inventing a second state representation.

### Important design decision

Do **not** put the 30-tick ring buffer recursively inside `Simulation`.

Instead, keep the authoritative `Simulation` unchanged as the biological engine and introduce a runtime-owned history container around it.

Conceptually:

    RuntimeState
      ├── Simulation
      ├── History[30]
      ├── Event stream
      ├── Session metadata
      └── Control state

This preserves the rule that the simulation does not know that a viewer exists.

---

# 4. Current observation audit

## 4.1 Existing strengths

`src/observation.rs` already establishes a semantic observation layer with:

- World,
- Organism,
- Structure.

The existing structure observation preserves physical endpoints and can expose actual structural units and bonds.

The existing UI already supports:

- world view,
- organism focus,
- structure view,
- camera pan/zoom,
- mouse/touch pointer interaction,
- organism inspection,
- resource appearance lookup.

This should be reused.

## 4.2 Current weaknesses

The current browser performs HTTP polling approximately every 250 ms.

The current world observation is not yet a complete physical-world stream.

The current resource visualization contains a coarse density-field projection and explicitly comments that some world rendering is only a projection of observed bounds.

The current environment observation does not yet expose enough physical-material placement information for the desired microscope view of actual environmental material.

The current organism inspector is primarily geometry-oriented. It does not yet provide the requested compact biological/runtime status:

- energy,
- stored material,
- action,
- action phase,
- developmental/reproductive state,
- generation,
- age,
- stress/maintenance state,
- current simulation/observed tick.

Therefore, the observation contract must be extended rather than replaced.

---

# 5. Current UI audit

The existing `ui/index.html` is a usable starting point, not the final viewer.

It already has:

- a canvas,
- browser-only camera state,
- world/organism/structure observation levels,
- pointer controls,
- periodic refresh,
- responsive viewport sizing.

`ui/organism_inspector.js` already attaches an inspector to organism observations.

`ui/resource_visualization.js` already attaches resource-aware rendering.

### Required UI evolution

The existing canvas/inspection architecture should be retained where it matches the new design.

The following should be added:

1. persistent connection state,
2. simulation control panel,
3. source/version selector,
4. simulation status,
5. observed tick versus live tick,
6. timeline/history control,
7. follow-organism mode,
8. compact organism status,
9. event log,
10. explicit rewind/restore controls,
11. reconnect handling,
12. mobile layout.

Do not introduce a second unrelated frontend framework unless the current UI becomes technically incapable of meeting these requirements.

---

# 6. Current LAN audit

The server currently binds to:

`127.0.0.1:3000`

Therefore:

- desktop browser works locally,
- phone cannot currently connect,
- another LAN client cannot currently connect.

The runtime must eventually bind to the host LAN interface (or an appropriate all-interface address) rather than loopback.

The startup script currently waits for:

`http://127.0.0.1:3000/`

and opens the local browser.

It will need to become LAN-aware.

The initial implementation should remain home-network-only.

No public internet exposure is required.

---

# 7. Current continuous-runtime audit

The normal server already runs indefinitely while `running == true`.

Therefore the continuous-simulation requirement does **not** require rebuilding the tick engine from scratch.

What is missing is control and separation:

- pause endpoint,
- resume endpoint,
- single-step endpoint,
- speed endpoint,
- runtime status,
- session identity,
- observation history,
- event publication.

This is substantially smaller than creating a new simulation loop.

---

# 8. Current diagnostics audit

The CI workflow currently performs:

1. formatting check,
2. source-size check,
3. COMBINE architecture check,
4. diagnostic headless multigeneration run,
5. artifact upload,
6. full Rust tests,
7. Clippy.

The diagnostic headless run explicitly creates a diagnostic file and is therefore not suitable as the normal live viewer runtime.

The desired viewer must not use this diagnostic path.

The existing diagnostics system should remain available for deliberate investigation but should not be called from the normal continuous server tick path.

This is a major performance boundary.

---

# 9. Current headless runner audit

`src/simulation_runner.rs` is currently a finite test/diagnostic runner.

It:

- starts a new simulation,
- runs to a tick/birth limit,
- optionally records diagnostics,
- reports progress,
- exits.

It is not a persistent local development runner.

It should not be stretched into the PR/version manager.

A separate local control-plane runner is required.

---

# 10. Required process separation

Safe PR switching creates the strongest architectural requirement discovered in this audit.

The persistent viewer/control server must survive a simulation process replacement.

If the browser/server and simulation are one process, selecting PR #X requires killing the process that is also serving the viewer.

That would violate the desired persistent viewer behavior.

Therefore the clean target is:

    Persistent control plane / runner
                 |
                 +---- serves viewer
                 |
                 +---- manages version/build state
                 |
                 +---- launches simulation process
                 |
                 +---- controls active simulation
                              |
                              v
                       EvoSim simulation

The browser remains connected to the control plane while the simulation child process is replaced.

This is the architectural change that should come before PR selection.

---

# 11. Recommended process model

Use two runtime roles.

## A. EvoSim simulation process

Responsible only for:

- authoritative Simulation,
- tick execution,
- simulation state,
- simulation controls,
- compact state/event publication to its parent/runtime interface.

It must not know about GitHub or PR management.

## B. EvoSim local runner/control plane

Responsible for:

- persistent HTTP/WebSocket server,
- browser UI hosting,
- version list,
- Git operations,
- build/test operations,
- simulation child-process lifecycle,
- safe version switching,
- client reconnect state,
- LAN exposure,
- save/load orchestration.

This control plane remains running while a simulation version is replaced.

---

# 12. Simulation communication boundary

The parent/control plane needs a small machine-readable interface to the active simulation.

The exact transport should be selected after a small implementation audit, but the boundary should support:

### State

- current tick,
- running,
- speed,
- session ID,
- compact world state,
- selected/focused organism data,
- event stream.

### Commands

- pause,
- resume,
- step,
- set speed,
- save,
- load,
- observation rewind,
- actual simulation restore.

The viewer should communicate with the control plane, not directly with arbitrary simulation processes.

---

# 13. History architecture

The 30-tick history should be maintained by the runtime layer.

At live tick N:

- history contains N-29 through N,
- default viewer position is N-15,
- oldest available position is N-29/N-30 depending on inclusive indexing convention.

The implementation must settle the exact inclusive convention before coding and then test it explicitly.

The historical state must be a complete authoritative simulation state.

The history must preserve RNG state so that actual restore can continue deterministically.

The viewer may inspect history without altering the active simulation.

Actual restore is an explicit control operation.

---

# 14. History memory audit

The current state representation makes full snapshots feasible, but the actual memory cost is unknown.

Do not guess a production memory budget.

The first implementation should measure:

1. clone time for one Simulation,
2. memory cost for one snapshot,
3. memory cost for 30 snapshots,
4. impact on tick throughput,
5. impact with one organism,
6. impact after population growth.

The measurement should be a small targeted benchmark, not the full CI workflow.

If full cloning proves too expensive at later scale, optimize the history representation then.

Do not prematurely introduce a delta-compression system.

---

# 15. Observation performance architecture

The current HTTP handlers lock the live Simulation and construct JSON while holding the mutex.

That is acceptable for the prototype but is undesirable for a continuously running high-speed simulation.

The target should be:

1. lock simulation briefly,
2. obtain/clone a compact render snapshot or immutable observation state,
3. unlock,
4. serialize/transmit outside the simulation lock.

The viewer must never hold the simulation lock while doing network I/O.

The simulation tick loop must not wait for a phone or browser.

---

# 16. Event stream architecture

Introduce a lightweight event stream generated by meaningful simulation transitions.

Events should be generated by the authoritative simulation/runtime boundary, not reconstructed by the browser.

Examples:

- action selected,
- COMBINE phase advanced,
- BREAK phase advanced,
- movement completed,
- material acquired,
- material expelled,
- reproduction started,
- offspring detached,
- organism died,
- physical material deposited,
- resource transformation occurred.

Events should be compact and bounded.

The event stream is not a substitute for state.

The state remains authoritative.

---

# 17. Rendering authority audit

The viewer must render actual physical truth.

For organisms, this means using existing structure/geometry data.

For environmental material, the current observation layer needs expansion to expose:

- physical material instances,
- their placements,
- their realized forms,
- their composition,
- internal bonds where relevant.

The existing density field can remain useful as an environmental background representation, but it cannot be the only representation when the user is inspecting actual physical material.

---

# 18. Version/PR runner audit

The version runner must not rely on the currently running working tree being clean or mutable.

The clean target is an isolated local source/build workspace for each runnable version.

Candidate model:

    local runner workspace
      |
      +-- main
      +-- pr-114
      +-- pr-115
      +-- pr-X

The runner should use Git worktrees or another isolated checkout mechanism rather than repeatedly checking branches in and out of the directory from which the control plane is executing.

This allows the control plane to remain stable while simulation versions are built.

The exact cache/worktree layout should be implemented only after checking the user's local Windows workflow and existing repository assumptions.

---

# 19. PR preparation sequence

When the viewer requests PR #X:

1. Discover/fetch the PR ref.
2. Prepare an isolated worktree.
3. Build the simulation.
4. Run the required targeted validation.
5. Keep the currently running simulation alive during all preparation.
6. If preparation fails, report failure and do not switch.
7. If preparation succeeds, start the replacement simulation.
8. Confirm the replacement becomes healthy.
9. Only then stop the old simulation.
10. Mark the new session/version active.
11. Keep all viewers connected to the persistent control plane.

This is a hard safety invariant.

---

# 20. Main/PR semantics

The viewer should display:

- Main,
- open PRs,
- build status,
- validation status,
- currently running version,
- active simulation session.

New PRs must not automatically become active.

PR selection must be explicit.

Automatic merging is outside the system's scope.

---

# 21. Existing run script audit

`run_evosim.cmd` currently:

- starts `cargo run --release`,
- waits for localhost:3000,
- opens the browser.

This should eventually become a one-command local startup entry point for the control plane.

The normal user workflow should not require PowerShell or manual terminal work.

The script should eventually:

1. start the persistent runner,
2. wait for readiness,
3. print/display the LAN viewer address,
4. open the computer browser.

The phone then uses the same LAN address.

---

# 22. Implementation order determined by audit

The clean order is now:

## Step 1 — Runtime boundary audit/implementation

Separate the persistent control plane from the simulation process.

Do not start with UI polish.

## Step 2 — Simulation process API

Create the minimal command/state boundary required by the control plane.

## Step 3 — Runtime history

Add the 30-tick complete-state ring buffer and measure its actual cost.

## Step 4 — Lightweight state/event stream

Replace periodic full HTTP polling as the primary live update mechanism.

## Step 5 — LAN binding

Make the persistent control plane accessible to the home network.

## Step 6 — Viewer controls

Add pause/resume/step/speed/timeline.

## Step 7 — Physical microscope

Expand observation contracts for actual organism and environmental material.

## Step 8 — Multi-client behavior

Verify computer and phone can observe/control the same active session.

## Step 9 — Local version manager

Add Main/PR discovery and isolated build/test.

## Step 10 — Safe PR switching

Implement prepare → validate → launch → health-check → switch.

## Step 11 — Persistent reconnect/session handling

Make simulation replacement invisible to the browser except for a clear session/version status transition.

---

# 23. Testing strategy for this work

Do not begin with the repository's 30–45+ minute full CI workflow.

Use small architectural tests first.

### Unit/contract tests

- simulation control state transitions,
- history length,
- 15-tick default observation position,
- 15-tick rewind allowance,
- restore includes RNG state,
- event ordering,
- viewer cannot mutate historical state,
- failed version preparation does not replace active version.

### Runtime tests

- simulation continues with zero clients,
- simulation continues with one client,
- simulation continues with two clients,
- slow client does not block tick execution,
- reconnect after simulation replacement,
- LAN binding.

### Version tests

- Main builds,
- a known PR builds,
- intentionally invalid version fails safely,
- failed PR leaves Main running,
- successful PR becomes active only after health check.

Full CI remains a final validation step, not the development loop.

---

# 24. Immediate implementation blockers

Before the viewer can meet the approved architecture, these concrete blockers must be addressed:

1. Current server and simulation are one process.
2. Current server binds only to loopback.
3. No persistent control API exists.
4. No pause/resume/step/speed endpoints exist.
5. No WebSocket/live event stream exists in the current branch.
6. No 30-tick history exists.
7. No actual simulation restore exists.
8. Current observation DTOs do not expose all required microscope state.
9. Current environmental observation does not expose enough physical material geometry.
10. Current UI has no version selector.
11. Current UI has no timeline/history controls.
12. Current UI has no persistent reconnect/session model.
13. Current headless runner is finite and diagnostic-oriented.
14. No isolated PR worktree/build manager exists.
15. Current startup script assumes localhost-only operation.

---

# 25. Things that are already reusable

Do not rebuild these unnecessarily:

- Axum.
- Tokio.
- Existing browser canvas/camera system.
- Existing observation contract concept.
- Existing organism structure observation.
- Existing resource appearance catalog.
- Existing simulation tick function.
- Existing `running` and `ticks_per_second` state.
- Existing serialization support throughout most simulation state.
- Existing GitHub repository and PR workflow.
- Existing Windows startup script as the basis for the control-plane launcher.

---

# 26. Audit conclusion

The project is closer to the desired viewer than it first appears.

The core simulation does not need a biological redesign.

The existing server already proves that the simulation can run continuously and that the browser can observe simulation-derived state.

The major architectural change is **process separation**:

    Persistent local runner/control plane
                  |
                  v
          Active simulation process

Once that boundary exists, the rest of the approved design becomes incremental:

- 30-tick history,
- 15-tick observation lag,
- event stream,
- LAN access,
- persistent reconnect,
- microscopic rendering,
- multi-device observation,
- Main/PR selection,
- safe PR build/test/switch.

The next implementation should therefore begin with the **runtime/control-plane boundary**, not with cosmetic UI work and not with another diagnostic system.

No biological rules should be changed as part of this work.


---

# 27. Runtime-boundary implementation progress

The runtime foundation is now implemented on the viewer branch.

The persistent HTTP server no longer owns the authoritative Simulation directly. It launches a separate simulation child process from the same executable using the `--simulation-child <port>` entrypoint.

The child owns:

- the authoritative Simulation,
- the 30-state runtime history,
- the simulation tick loop,
- pause/resume/step/speed,
- historical observation,
- actual historical restore,
- session identity.

The parent owns:

- the persistent browser HTTP server,
- the LAN listener,
- browser-facing observation/control routes,
- the child-process handle.

The parent and child communicate through a localhost-only, line-delimited JSON command boundary.

This is intentionally a small protocol rather than a second simulation model.

The browser therefore remains attached to the parent process when the simulation child is later replaced for Main/PR switching.

The current branch has **not** run the long CI suite. The next validation should be a short compile/format check before adding more runtime layers.

