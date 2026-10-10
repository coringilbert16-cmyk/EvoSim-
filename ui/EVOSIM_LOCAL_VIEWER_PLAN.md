# EvoSim Local Viewer and Development Runtime Plan

## Status

**Purpose:** Authoritative working plan for the local continuous simulation viewer, local development runner, multi-device observation, PR execution workflow, and 30-tick observation history.

**Status:** Approved architecture / implementation plan.

**Scope:** This document defines the architecture and development sequence for the viewer/runtime system. It does not redefine EvoSim's biological, physical, energy, developmental, environmental, or behavioral rules.

**Authority:** The simulation remains authoritative for simulation state. The viewer, runner, diagnostics, and GitHub workflow are supporting infrastructure only.

---

# 1. Goals

The system must provide a continuously running local EvoSim simulation that can be observed and controlled without repeatedly closing, rebuilding, or resetting the viewer.

The system must support:

1. Continuous simulation until explicitly paused.
2. Simulation speed controls, initially including 1x, 10x, and 100x.
3. Pause, resume, and single-tick stepping.
4. A live 2D physical view of the world.
5. Microscopic inspection of an individual organism.
6. Following an organism as it moves.
7. Rendering physically represented organism structure, bonds, cavities, stored material, damage, and developmental state.
8. Rendering physically represented environmental resources using their actual established shapes and structures.
9. Observation of movement, material availability, COMBINE, BREAK, deterioration, reproduction/development, and death.
10. A small live state panel.
11. An event-oriented log rather than full-state logging every tick.
12. A rolling 30-tick simulation-state history.
13. A default viewer position 15 ticks behind the live simulation.
14. Up to 15 additional ticks of backward viewing, producing a total 30-tick observation window.
15. Rewind/restore of the actual simulation state when explicitly requested, including deterministic continuation from the restored point.
16. Multiple simultaneous viewers, including computer and phone, observing the same active simulation.
17. Independent camera/inspection state for each viewer.
18. Shared simulation controls across connected viewers.
19. A persistent browser viewer that automatically reconnects when the simulation process restarts.
20. Local execution on the user's computer and local-network access from other devices.
21. Selection of Main or a specific PR as the version to run.
22. Automatic preparation/testing of a selected PR before replacing the currently running simulation.
23. Failure isolation: an invalid PR must not destroy or replace the currently running valid simulation.
24. Diagnostics remaining optional and separate from the simulation's normal execution path.

---

# 2. Core Architectural Principle

The system is divided into four responsibilities:

1. **Simulation** — authoritative EvoSim world and lifecycle.
2. **Local Runner** — builds, tests, launches, stops, and switches simulation versions.
3. **Viewer/Server Interface** — exposes lightweight state, events, history, and controls to browser clients.
4. **Browser Viewer** — observation and control interface for computer, phone, or other local-network devices.

GitHub remains the source/version system but is not part of the simulation runtime.

The simulation must not depend on the viewer.

The simulation must not depend on GitHub.

The simulation must not emit giant diagnostic artifacts as part of normal execution.

The viewer must not calculate biological truth that belongs to the simulation.

---

# 3. Responsibility Boundaries

## 3.1 Simulation

The simulation owns:

- Environment state.
- Organism state.
- Resource/material state.
- Physical structure.
- Physical bonds.
- Positions and geometry.
- Energy and energy accounting.
- Action state and action phase.
- Development and reproduction state.
- Damage and maintenance state.
- Harmonic/internal state.
- Random state required for deterministic continuation.
- Current simulation tick.
- Simulation progression.

The simulation exposes state; it does not know why a browser is requesting it.

## 3.2 Local Runner

The runner owns:

- Available source versions.
- Main versus PR version selection.
- Local source checkout/cache management.
- Build execution.
- Required targeted validation.
- Simulation process lifecycle.
- Safe replacement of one running version with another.
- Automatic restart/reconnection support.
- Reporting build/test/run status to the viewer.

The runner does not become a second simulation authority.

## 3.3 Viewer/Server Interface

The interface owns:

- Client connections.
- Lightweight state publication.
- Event publication.
- Access to the 30-tick history.
- Viewer control commands.
- Multi-client synchronization for global simulation controls.
- Reconnection handling.

It does not reconstruct biological state or make simulation decisions.

## 3.4 Browser Viewer

The browser owns:

- Rendering.
- Camera position.
- Zoom.
- Organism selection/following.
- Timeline position for observation.
- Local presentation state.
- User interaction.

It may request simulation actions, but the simulation remains authoritative for whether those actions occur.

---

# 4. Simulation Independence

The viewer must be an observational layer over the simulation rather than another simulation system.

The simulation must be able to run:

- with the viewer connected,
- with the viewer disconnected,
- with diagnostics disabled,
- and without serializing a large full-state artifact every tick.

Viewer rendering must never block simulation progression.

If the viewer disconnects, the simulation continues.

If the browser is refreshed, the simulation continues.

If a phone disconnects, the simulation continues.

If diagnostics are disabled, the simulation remains fully functional.

---

# 5. Local Runtime Model

The user's computer is the host machine.

The actual EvoSim process runs locally on that computer and owns the active world.

The local interface is served over the home network.

Conceptual topology:

    Computer
      |
      +-- EvoSim Runner
             |
             +-- Simulation process
             |
             +-- Viewer/Server interface
                     |
                     +-- Computer browser
                     |
                     +-- Phone browser
                     |
                     +-- Other local viewers

The initial implementation is intended for the user's home network.

Internet exposure is not required and should not be implemented merely to support phone access.

---

# 6. Persistent Viewer

The browser viewer is intended to remain open during development and observation.

When the simulation process is rebuilt or restarted:

1. The simulation may disconnect.
2. The viewer displays a reconnecting state.
3. The viewer automatically reconnects.
4. The viewer resumes observation of the new running simulation.
5. The browser itself does not need to be closed.

The viewer should preserve appropriate presentation state across a backend restart where doing so is meaningful.

The viewer must clearly identify when a new simulation session has begun.

---

# 7. Version Selection

The viewer must eventually provide a version selector with at least:

- Main.
- Open PRs that are available to the local runner.

Versions should have stable local identities such as:

- `main`
- `pr-115`
- `pr-116`

The viewer may display build/test status where available.

A newly created PR must never silently replace the running simulation.

The user explicitly selects which version to run.

---

# 8. Safe PR Execution

Selecting a PR must not immediately destroy the currently running simulation.

The runner must use this sequence:

1. Fetch/update the selected source version.
2. Build the selected version.
3. Run the required targeted validation.
4. If preparation/validation fails:
   - keep the current simulation running,
   - report the failure,
   - do not switch versions.
5. If preparation succeeds:
   - stop the old simulation at the controlled transition point,
   - launch the selected version,
   - expose the new session to viewers.

The exact validation suite is an implementation decision to be established after auditing the existing repository and test infrastructure. It must not be invented prematurely.

---

# 9. Main Versus Experimental PRs

**Main** represents the stable approved source version.

A PR represents an experimental source version.

The system must allow the user to move between them deliberately.

The intended workflow is:

    Main
      -> select PR
      -> build/validate
      -> run PR
      -> observe
      -> return to Main when desired

The viewer must make the currently running source version obvious.

---

# 10. Multi-Device Observation

Multiple browser clients must be able to observe the same simulation simultaneously.

Example:

- Desktop follows Organism 17 at microscopic zoom.
- Phone views the surrounding environment.

Both are observing the same simulation tick/state.

Viewer-specific presentation state should remain independent:

- camera,
- zoom,
- selected organism,
- follow target,
- observation position.

Simulation-wide controls should be shared:

- pause,
- resume,
- step,
- simulation speed,
- save/load,
- actual simulation rewind/restore,
- version switching.

The distinction between **viewing an earlier state** and **restoring the simulation to an earlier state** must remain explicit.

---

# 11. Observation History

The simulation maintains a rolling history of the previous 30 ticks.

The history is in-memory by default.

It is not a long-term database.

Old history is discarded as new ticks arrive.

The default observation position is:

    live tick - 15

Therefore, at live tick N:

- N is the live simulation state.
- N-15 is the normal viewer state.
- N-30 is the oldest normally available history state.

This produces:

    15 ticks of intentional observation lag
    +
    15 ticks of additional rewind history
    =
    30 ticks of total observation window.

The viewer normally follows the N-15 state.

---

# 12. Observation Versus Simulation Time

The viewer's observation time is intentionally separate from simulation time.

The simulation continues to advance independently.

A viewer may:

- watch the normal N-15 state,
- move backward within the available 30-tick window,
- inspect a historical state,
- move forward again,
- return to the normal observation position.

Pausing the simulation must not destroy the history.

Resuming the simulation causes the normal observation position to continue tracking the live simulation with the intended 15-tick lag.

---

# 13. Actual Simulation Rewind

Viewing an earlier state is not itself a simulation rewind.

An explicit restore operation must restore the complete authoritative simulation state required for deterministic continuation.

At minimum this includes, where applicable:

- Organisms.
- Physical structure.
- Bonds.
- Positions.
- Stored material.
- Environmental material.
- Energy state.
- Action state.
- Three-tick action phase.
- Development/reproduction state.
- Damage/maintenance state.
- Harmonic/internal state.
- Memory state.
- Random-number-generator state.

After an actual restore, the simulation continues from that historical state as a new continuation of the same deterministic history.

The system must not restore only the visual representation.

---

# 14. Three-Tick Action Visibility

The viewer should expose the established three-tick lifecycle clearly enough to observe it:

Tick 1:
- Decision system selects the action.

Tick 2:
- Candidates are found.
- Required bonds are formed/broken.
- Energy transaction is settled.

Tick 3:
- Structural mutation is committed at the beginning of the tick.

COMBINE and BREAK therefore occupy three ticks from selection through structural mutation.

The viewer displays the current action and phase rather than inventing a separate action lifecycle.

---

# 15. Microscopic Viewer

The first useful viewer should be deliberately small in scope.

It should prioritize one organism and its immediate physical surroundings.

The viewer must be able to show, where physically represented:

- organism structure,
- individual material,
- bonds,
- genome cavity,
- internal cavities,
- stored material,
- material crossing the organism boundary,
- damage,
- disconnected material,
- environmental material,
- movement,
- COMBINE,
- BREAK,
- developmental construction,
- reproduction,
- deterioration,
- death,
- resulting carcass/material.

The viewer must render the established physical resource shapes rather than replacing them with abstract biological icons.

No new biological categories should be introduced solely for visualization.

---

# 16. State Panel

The initial status panel should remain small.

At minimum, it should be capable of displaying:

- simulation tick,
- observed tick,
- organism count,
- generation,
- organism age,
- realized mass,
- stored material,
- energy,
- current action,
- action phase,
- reproduction/development state,
- current source version.

The panel is observational. It is not a second authority.

---

# 17. Event Log

The viewer should prefer meaningful event records over full-state logging.

Examples of the intended style:

    Tick 841 — COMBINE selected
    Tick 842 — Carbon + Hydrogen candidates resolved
    Tick 842 — energy transaction settled
    Tick 843 — structure mutated

Events should describe meaningful state transitions.

The system must not serialize a giant full diagnostic state every tick merely to populate the viewer.

---

# 18. Diagnostics

Diagnostics are optional and separate from normal simulation execution.

The viewer is not a diagnostic artifact generator.

The normal runtime path should remain lightweight:

    Simulation
      -> compact current-state publication
      -> event stream
      -> 30-tick history

Diagnostics may subscribe separately when required.

Full-state artifacts should be generated deliberately for investigation, not continuously by default.

This separation is specifically intended to prevent the previous diagnostic/serialization workload from becoming the simulation runtime bottleneck.

---

# 19. Performance Boundary

The simulation receives the majority of the available computational budget.

The viewer must update at a human-useful rate rather than requiring every simulation tick to be rendered.

The simulation must not wait for:

- browser rendering,
- phone rendering,
- network transmission,
- diagnostics,
- or a slow client.

The viewer may skip intermediate render states while the simulation continues.

Performance optimization should be measurement-driven rather than speculative.

---

# 20. Save/Load

Save/load is an explicit simulation feature.

The rolling 30-tick history is not a replacement for persistent saves.

A saved simulation should contain enough authoritative state to resume the world correctly.

The save system must not become a second authority or silently reconstruct missing physical state from summaries.

The exact persistent format should be audited against the existing simulation serialization before implementation.

---

# 21. Existing UI Relationship

The repository currently contains a `ui/` directory with an existing browser interface.

The new viewer architecture should be integrated into that UI rather than creating an unnecessary parallel UI tree.

Before implementation:

1. Audit the current UI.
2. Identify reusable rendering and inspection components.
3. Identify obsolete assumptions.
4. Identify what can be retained.
5. Identify what must be replaced.
6. Avoid preserving architecture merely because it already exists.

The existing UI must not be allowed to dictate simulation authority.

---

# 22. Development Sequence

Implementation should proceed in controlled phases.

## Phase 0 — Architecture Audit

Inspect:

- simulation entry points,
- current executable structure,
- current UI,
- current server/interface,
- current serialization,
- current state model,
- current save/load mechanisms,
- existing test commands,
- current CI/runtime behavior,
- existing scripts.

Do not run a new long full CI run merely to inspect architecture.

Resolve ambiguities before implementation.

## Phase 1 — Continuous Simulation Runtime

Establish a clean simulation process that can:

- run continuously,
- pause,
- resume,
- step one tick,
- change speed,
- run without diagnostics.

## Phase 2 — Runner

Add the local runner responsible for:

- source version management,
- build,
- targeted validation,
- process launch,
- process replacement,
- restart status.

## Phase 3 — Lightweight State Interface

Add:

- current state publication,
- event stream,
- client connection,
- reconnect behavior.

Do not begin with a large serialized diagnostic representation.

## Phase 4 — 30-Tick History

Add the rolling in-memory history:

- 30 ticks retained,
- 15-tick normal observation lag,
- 15 additional ticks of backward viewing.

Add complete-state restore only after the state required for deterministic continuation has been audited.

## Phase 5 — Microscopic Viewer

Implement:

- physical world rendering,
- organism structure,
- resource geometry,
- following,
- zoom,
- state panel,
- event log,
- pause/step/speed controls,
- timeline.

## Phase 6 — Multi-Client LAN Operation

Enable:

- local-network binding,
- computer browser,
- phone browser,
- simultaneous connections,
- automatic reconnect.

## Phase 7 — Version Selector

Add:

- Main,
- PR discovery,
- PR status,
- safe build/validation,
- explicit version switching.

## Phase 8 — Development Automation

Add:

- automatic detection of newly available PRs,
- controlled rebuild/restart,
- persistent viewer,
- clear session/version status,
- convenient save/load.

Automation must never silently replace a running valid simulation.

---

# 23. Safety and Failure Rules

The following are architectural invariants:

1. A viewer failure must not stop the simulation.
2. A phone disconnect must not stop the simulation.
3. A diagnostic failure must not stop the simulation.
4. A failed PR build must not replace Main.
5. A failed PR validation must not replace the currently running version.
6. A browser refresh must not reset the simulation.
7. Viewing historical state must not alter the simulation.
8. Actual simulation rewind must restore authoritative state, not merely visuals.
9. The viewer must not invent biological state.
10. The runner must not invent simulation rules.
11. GitHub source state must not become runtime simulation authority.
12. The simulation must remain usable without a viewer.

---

# 24. Minimum First Deliverable

The first working version does not need a polished application.

It needs to prove the architecture.

Minimum successful demonstration:

1. EvoSim runs continuously on the computer.
2. The browser connects without starting the simulation itself.
3. The simulation can pause/resume/step.
4. The viewer shows a real organism and its actual physical surroundings.
5. The viewer is intentionally 15 ticks behind.
6. The previous 15 ticks can be inspected.
7. The simulation continues if the browser is closed.
8. Reopening the browser reconnects to the same run.
9. A phone on the same home network can connect simultaneously.
10. Both devices observe the same active simulation.
11. A PR can be selected and prepared without destroying the currently running version.
12. A failed PR leaves the current version running.
13. A successful PR can be launched and both viewers reconnect to it.
14. Diagnostics remain off by default.

---

# 25. Explicit Non-Goals

The initial implementation should not attempt to:

- build a polished game UI,
- expose the simulation to the public internet,
- create a cloud simulation service,
- maintain unlimited history,
- generate full diagnostic artifacts every tick,
- add biological abstractions for visualization,
- add new simulation mechanics merely to make the viewer easier,
- automatically merge PRs,
- automatically switch to newly created PRs,
- make the browser authoritative for simulation state,
- or redesign EvoSim's biological rules as part of viewer development.

---

# 26. Governing Principle

The viewer/runtime system exists to make the existing EvoSim simulation observable and developable.

It must not become another layer of simulation logic.

The intended architecture is:

    GitHub
       |
       v
    Local Runner
       |
       v
    Authoritative Simulation
       |
       +---- 30-tick history
       |
       +---- lightweight state stream
       |
       +---- event stream
       |
       v
    Local Viewer Server
       |
       +---- Computer
       |
       +---- Phone
       |
       +---- Other local viewers

The simulation is the world.

The runner is the local laboratory controller.

The viewer is the microscope.

GitHub is the source/version system.

The separation between these responsibilities should be preserved throughout implementation.
