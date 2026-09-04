# EvoSim Continuous Simulation Roadmap

**Status:** Active implementation guideline  
**Purpose:** Get EvoSim running continuously as a real, observable simulation while keeping the architecture coherent, clean, testable, and efficient.  
**This document is not the EvoSim master specification.** It is a practical engineering roadmap for the current development objective.

---

## 1. Current Goal

The immediate goal is simple:

> **Run the actual EvoSim simulation continuously and let the user watch the real simulation evolve in the browser.**

The simulation engine, environment, organisms, transformations, reproduction, and material systems should remain the authoritative system. The browser should observe and present that system rather than becoming a second simulation engine.

We are therefore not starting by adding visual effects, new gameplay mechanics, or speculative architecture. We are first creating a reliable path from the simulation core to a continuously updating visible world.

The intended end-to-end architecture is:

```text
                    ┌─────────────────────┐
                    │   Rust Simulation   │
                    │                     │
                    │ authoritative state │
                    │ + Simulation::step  │
                    └──────────┬──────────┘
                               │
                               │ snapshot / observation
                               ▼
                    ┌─────────────────────┐
                    │ Observation / API   │
                    │                     │
                    │ serialization       │
                    │ transport           │
                    └──────────┬──────────┘
                               │
                               │ live updates
                               ▼
                    ┌─────────────────────┐
                    │    React Viewer     │
                    │                     │
                    │ render only         │
                    │ inspect / visualize │
                    └─────────────────────┘
```

The exact transport may change as implementation is audited. The architectural boundary should not: **simulation state belongs to the Rust engine; presentation belongs to the viewer.**

---

# 2. What This Roadmap Is — and Is Not

## This roadmap is

- An implementation sequence.
- A checklist for making EvoSim continuously runnable.
- A guide for deciding what to build now versus later.
- A set of architectural constraints for the runtime and observation pipeline.
- A progression of concrete milestones with verification gates.
- A way to prevent UI work from driving simulation design.

## This roadmap is not

- The master simulation specification.
- A replacement for the resource, organism, material, bond, reproduction, or environment rules.
- Permission to redesign existing mechanics merely because they are difficult to visualize.
- A requirement that every subsystem be perfect before the simulator can run.
- A visual-design specification.
- A commitment to a particular networking technology before the existing code is audited.

When this roadmap conflicts with an established simulation rule, the simulation rule remains authoritative. This document governs **how we get the existing system running and observable**, not what the simulation is supposed to mean.

---

# 3. Guiding Principles

These principles apply throughout the roadmap.

## 3.1 Build the simulation once

There must be one authoritative simulation execution path.

The frontend must not independently calculate:

- organism movement,
- resource behavior,
- bond strength,
- transformation progress,
- reproduction,
- environment evolution,
- energy/material changes,
- or other simulation outcomes.

The browser displays what the engine says happened.

## 3.2 The viewer must never become the simulation

The UI may request operations such as:

- start,
- stop,
- pause,
- resume,
- reset,
- step,
- change an explicitly supported runtime setting.

Those operations should affect the authoritative simulation runtime. The UI should not implement a parallel approximation of the simulation to make the screen look alive.

## 3.3 Prefer the smallest working path

At each stage, implement the smallest change that establishes the next architectural boundary.

Do not solve future performance problems before the actual runtime path exists.

Do not build sophisticated rendering before live state reaches the browser.

Do not redesign simulation mechanics to create visual activity.

## 3.4 Verify each boundary before moving on

Every major stage must have a verification gate.

A stage is complete when we can demonstrate that its responsibility works, not merely when the code compiles.

## 3.5 Keep simulation time independent from rendering time

The simulation and browser have different jobs.

The simulation should be able to advance according to its configured simulation rate without requiring a browser render for every tick.

The browser should receive observations at a practical update rate rather than necessarily receiving every internal simulation tick.

This separation will become important for performance, but it should be introduced without prematurely complicating the first working implementation.

## 3.6 Make failures observable

When the simulator stops, stalls, produces invalid state, or behaves unexpectedly, we should be able to determine whether the problem is in:

1. simulation execution,
2. observation/snapshot creation,
3. serialization,
4. transport,
5. frontend state handling,
6. rendering.

A clean boundary makes this possible.

## 3.7 Do not add mechanics just to make the screen interesting

If the initial organism does not move, reproduce, combine, break, or otherwise create visible activity, that is a simulation behavior question—not a reason to add artificial animation.

The visualizer should reveal the actual system, including periods of inactivity.

---

# 4. Current Known Starting Point

The Rust simulation already has an important foundation: `Simulation::step()` is the central per-tick execution path and returns a `Snapshot`.

The current step sequence is broadly:

1. Advance the simulation tick.
2. Step the environment.
3. Advance active transformations.
4. Resolve completed transformations.
5. Capture the current environment state for organism processing.
6. Update organism age and perception-related state.
7. Evaluate organism decisions.
8. Execute applicable actions.
9. Queue reproduction requests.
10. Begin reproduction.
11. Advance reproductive construction.
12. Apply energy-capacity rules.
13. Update the total usable-energy ledger.
14. Return a snapshot.

This is a strong starting point for an observation boundary.

However, the existence of `Simulation::step()` does **not** by itself establish continuous execution. The first implementation task is therefore an audit of the actual runtime/API/frontend path.

The current environment and material architecture should be treated as existing simulation behavior while this runtime work proceeds. Recent environment changes established a unified deep reservoir and an active-field distinction needed by current material/bond behavior. This roadmap does not reopen those design decisions.

---

# 5. Phase 0 — Runtime and Frontend Audit

**This is the first engineering step. No behavior changes should be made before this audit is complete.**

## Objective

Determine exactly how the current simulation is started, stepped, exposed, and displayed.

## Questions to answer

### Backend

- Where is `Simulation` constructed?
- Where is `Simulation::step()` called?
- Is it called by an actual continuous loop?
- Is there a timer, thread, async task, or other scheduler?
- What currently controls `ticks_per_second`?
- What does the existing `running` field actually control?
- Is there already a start/stop lifecycle?
- Is simulation state shared between requests/tasks safely?
- Is there one simulation instance or can multiple instances accidentally exist?
- Where are errors handled?
- Does the server remain alive while the simulation advances?

### API / transport

- Is there an HTTP endpoint returning a snapshot?
- Is there already WebSocket support?
- Is there SSE or another streaming mechanism?
- Does an endpoint currently call `step()` directly?
- Does the frontend polling cause simulation advancement?
- Is serialization performed from the authoritative snapshot?
- Can a client disconnect without affecting simulation execution?

### Frontend

- How does React currently obtain simulation state?
- Is it polling, streaming, or receiving a one-time response?
- Does the frontend maintain its own simulation state or calculations?
- What currently renders the world?
- Is there already a tick counter or status indicator?
- What happens when the backend stops responding?
- Can the frontend reconnect?

### Runtime lifecycle

- What happens on startup?
- What happens on shutdown?
- What happens on reset?
- Can the simulation be paused without destroying state?
- Can it resume from the same state?
- Can it advance exactly one tick for debugging?

## Deliverable

Produce a short architecture map showing the current path:

```text
startup
  -> simulation creation
  -> scheduler / step caller
  -> snapshot creation
  -> API / transport
  -> frontend data reception
  -> rendering
```

For every arrow, identify the actual file/function responsible.

## Gate

Do not implement the continuous runtime until we know whether one already partially exists and exactly where the missing link is.

---

# 6. Phase 1 — Establish One Authoritative Continuous Simulation Loop

## Objective

Make the Rust engine capable of advancing continuously without depending on the browser.

## Required behavior

A runtime should conceptually provide:

```text
initialize simulation
        ↓
start runtime
        ↓
advance Simulation::step()
        ↓
repeat while running
        ↓
stop / pause cleanly
```

The loop must own simulation progression.

The browser must not be responsible for calling `step()` once per rendered frame as the fundamental simulation scheduler.

## Design requirements

### One simulation authority

There should be one clearly identifiable simulation instance for a running world.

### Configurable simulation rate

The existing `ticks_per_second` concept should remain meaningful, but the exact scheduling mechanism should be selected after the audit.

### Start/stop semantics

`running` should have a clear purpose. If it represents runtime state, it should participate in actual runtime control. If it is redundant with another state machine, consolidate rather than maintaining multiple authorities.

### Clean shutdown

The runtime should be able to stop without corrupting or partially advancing simulation state.

### No accidental double stepping

The system must make it difficult or impossible for two independent loops to advance the same simulation simultaneously.

## Important non-goals

Do not yet optimize every allocation.

Do not build a complex distributed scheduler.

Do not introduce multiple simulation workers unless the current architecture actually requires them.

## Verification

The backend should be able to:

- start,
- advance repeatedly,
- remain alive indefinitely,
- stop cleanly,
- resume correctly if supported,
- and run without a browser connected.

A basic runtime test or controlled executable/test harness should demonstrate repeated advancement.

## Gate

**Milestone 1:** The Rust simulation can run continuously on its own.

At this point, the browser is not required for the simulation to exist.

---

# 7. Phase 2 — Define the Observation Boundary

## Objective

Create a clean boundary between simulation state and presentation.

The preferred conceptual pipeline is:

```text
Simulation
    ↓
Snapshot
    ↓
Observation/API layer
    ↓
Serialized snapshot
    ↓
Transport
```

The exact types and transport should follow the existing code wherever practical.

## Snapshot responsibilities

A snapshot should represent information the viewer is allowed to observe.

It should not become a second mutable simulation state.

If the existing `Snapshot` already serves this role adequately, prefer using it over creating another parallel state model.

## Avoid leaking internal implementation unnecessarily

The viewer does not need every internal field merely because it exists.

Observation should expose what is useful for:

- world visualization,
- organism inspection,
- runtime status,
- later scientific instrumentation.

The observation model can expand incrementally.

## Initial observation payload

The first useful payload should be enough to show:

- simulation tick,
- simulation running/paused state if available,
- world/environment dimensions,
- visible environment material information,
- vents,
- organisms and their positions,
- enough organism state to represent size/development where already available.

The first payload does not need every diagnostic or internal implementation detail.

## Gate

A test or manual verification should show that a snapshot can be produced from the real simulation and serialized without changing simulation state.

---

# 8. Phase 3 — Connect the Live Backend to the React Viewer

## Objective

Make the browser receive live observations from the actual running simulation.

## Desired behavior

Opening the application should establish a connection to the running backend.

The viewer should then receive updated observations as the simulation advances.

Conceptually:

```text
Rust runtime
     │
     │ continuously advances
     ▼
Snapshot generation
     │
     │ live observations
     ▼
Frontend state
     │
     ▼
React rendering
```

## Important rule

The browser should not determine whether the simulation advances.

A browser refresh, render, dropped frame, or temporary disconnect should not alter the simulation's fundamental progression.

## First UI milestone

The first screen does not need to be beautiful.

It needs to prove that the following are real:

- live tick progression,
- real organism state,
- real environment state,
- real simulation lifecycle.

A simple world canvas/view with a tick counter is preferable to an elaborate interface built on simulated placeholder data.

## Reconnection

Once the basic path works, the frontend should handle a lost backend connection gracefully rather than silently displaying stale state as if it were current.

## Gate

**Milestone 2:** Open the browser and watch the actual Rust simulation advance continuously.

This is the central goal of the first development cycle.

---

# 9. Phase 4 — Build the Simplest Useful World View

## Objective

Turn live simulation state into a readable visual world.

## First visual layer

Show only what is necessary to establish spatial reality:

- world bounds,
- environment field,
- vents,
- organisms,
- organism positions,
- organism size/shape where available,
- simulation tick/status.

## Rendering philosophy

Prefer correctness over aesthetics.

The first renderer should answer:

> "Am I looking at the real simulation right now?"

It does not need to answer:

> "Does this look like a finished game?"

## No fake animation

Do not animate organisms independently of their authoritative simulation positions.

Do not interpolate fake resource movement that contradicts snapshots.

Do not create decorative particles representing events that never occurred.

Visual interpolation can be added later if it preserves the meaning of the underlying state.

## Gate

A human observer should be able to watch the world for several minutes and confirm that the displayed state corresponds to simulation state rather than a canned animation.

---

# 10. Phase 5 — Make Organism State Meaningful and Inspectable

Once the world itself is live, improve what can be understood about organisms.

## Progressive information layers

### Level 1 — Basic identity

- organism identifier,
- position,
- age,
- development stage,
- size/structure.

### Level 2 — Current activity

Where supported by the existing simulation:

- current action,
- movement state,
- active transformation,
- resource perception,
- reproductive state.

### Level 3 — Material/structural state

Eventually expose useful information such as:

- structural units,
- stored material,
- bonds,
- transformation state,
- reproductive construction progress.

### Level 4 — Scientific detail

Later, expose deeper diagnostics without cluttering the primary world view.

## Inspection model

A user should eventually be able to select an organism and answer:

- Where is it?
- How old is it?
- What stage is it in?
- What is it doing?
- What material/structure does it have?
- Is it currently transforming material?
- Is it attempting reproduction?

This is observation, not simulation control.

---

# 11. Phase 6 — Make the Environment Legible

The environment is not merely background decoration. It is part of the simulation's causal system.

The viewer should eventually make the major material cycle understandable:

```text
Deep reservoir
      ↓
    vents
      ↓
Active material field
      ↓
 diffusion / movement
      ↓
 organisms interact with material
      ↓
 material transformations
      ↓
 settling / redistribution
      ↓
Deep reservoir
```

The exact visualization should follow the actual simulation implementation.

## Initial environment visualization

Show enough information to distinguish meaningful regions and resource presence.

## Later visualization

Potential additions include:

- resource-density overlays,
- material type inspection,
- vent activity,
- transformation activity,
- settling activity,
- spatial gradients.

## Rule

Visualization must remain faithful to the current simulation rules. If a process is not actually modeled, the UI should not imply that it is.

---

# 12. Phase 7 — Close Remaining Simulation Lifecycle Gaps

Once the simulation is continuously visible, use that visibility to discover problems that ordinary unit tests may not reveal.

## Questions to investigate

### Organisms

- Do organisms actually move when their rules permit movement?
- Do they encounter usable material?
- Do they form structures?
- Do transformations complete?
- Does BREAK occur when conditions permit?
- Does reproduction proceed when readiness conditions are met?

### Environment

- Does material circulate?
- Do vents actually affect the active field over time?
- Does diffusion behave sensibly?
- Does settling return material to the reservoir as intended?
- Is material conserved except where a rule explicitly transforms it?

### Population

- Can organisms survive?
- Can they reproduce?
- Can populations collapse?
- Can populations grow without bound?
- Are births and deaths occurring through real lifecycle rules?

### Transformations

- Do active transformations get stuck?
- Are resources/materials duplicated or lost?
- Are completed transformations resolved exactly once?
- Does the system remain stable over long runs?

## Critical rule

Do not respond to visual inactivity by inventing new mechanics.

First determine whether the existing rules are behaving as designed.

---

# 13. Phase 8 — Scientific Observability

After the live world is reliable, build instrumentation that turns the viewer into a scientific tool.

## Core metrics

At minimum, plan for:

- current tick,
- population,
- births,
- deaths,
- average organism age,
- average organism size,
- environment material totals,
- active transformation count,
- reproduction activity,
- action frequencies.

## Material accounting

Where appropriate, expose totals across the relevant system compartments so long-running conservation problems can be detected.

The existing simulation already has a `total_material_in_system()` test-oriented accounting helper. A production observation metric should be designed separately if the test helper's representation is not appropriate for the UI.

## Historical data

Do not send unbounded history to the browser.

The backend can maintain compact statistics or emit periodic aggregates.

The frontend can retain a bounded history for graphs.

## Gate

The user should be able to watch the simulation and also determine whether the system is actually changing in meaningful ways.

---

# 14. Phase 9 — Performance and Efficiency Pass

Performance optimization should happen after the real runtime and observation path work.

The goal is not maximum theoretical speed. The goal is efficient, predictable simulation with a responsive viewer.

## Areas to measure

### Simulation

- organism iteration,
- environment diffusion,
- spatial queries,
- transformation processing,
- reproduction processing,
- material operations,
- allocations and cloning.

### Snapshot generation

The current snapshot path clones substantial simulation state. This is acceptable as an initial correctness-first implementation, but it is an obvious candidate for measurement and later optimization.

Potential future approaches include:

- reducing copied state,
- dedicated observation structures,
- incremental/delta updates,
- bounded snapshot frequency,
- serialization from a read-only observation view.

Do not replace working straightforward code with a complex zero-copy system without measurement.

### Transport

Measure:

- snapshot size,
- serialization time,
- update frequency,
- network traffic,
- frontend processing time.

### Rendering

Measure:

- frame rate,
- number of rendered objects,
- unnecessary React updates,
- canvas/SVG/DOM costs,
- resource visualization costs.

## Target architecture

The simulation may run substantially faster than the browser needs to render.

For example:

```text
Simulation:     high internal tick rate
                    │
                    │ sampled observations
                    ▼
Viewer:         30–60 visual updates/sec
```

The actual rates should be determined experimentally.

---

# 15. Phase 10 — Long-Duration Stability

A simulator is not truly continuously runnable merely because it survives for ten seconds.

## Required tests

Run the simulation for progressively longer periods:

1. seconds,
2. minutes,
3. tens of minutes,
4. hours,
5. eventually extended unattended runs.

## Monitor for

- runaway numeric values,
- NaN/infinite values,
- memory growth,
- stalled transformations,
- population explosions,
- population extinction,
- material disappearance/creation,
- simulation clock failures,
- scheduler drift,
- deadlocks,
- stale frontend state,
- connection failures,
- corrupted restart state.

## Reproducibility

Where deterministic seeds are supported, verify that a given seed and simulation configuration can reproduce the same behavior when run under the same simulation conditions.

This is particularly important when later concurrency or performance optimizations are introduced.

## Gate

**Milestone 3:** EvoSim can run for extended periods without requiring manual intervention and without silently entering an invalid state.

---

# 16. Phase 11 — Rich Presentation Comes Last

Only after the underlying system is continuously runnable, observable, and stable should the project invest heavily in presentation.

Potential features include:

- polished world rendering,
- organism inspection panels,
- population graphs,
- material graphs,
- environment overlays,
- genealogy,
- organism filtering,
- camera controls,
- pause/resume/step controls,
- experiment controls,
- event timelines,
- historical views,
- replay/debugging tools.

These features should consume authoritative observations rather than introducing their own simulation logic.

---

# 17. Recommended Implementation Order

The actual implementation sequence should remain deliberately narrow:

```text
0. Audit current runtime/API/frontend
        ↓
1. Establish authoritative continuous Simulation loop
        ↓
2. Establish clean Snapshot/observation boundary
        ↓
3. Connect backend live state to React
        ↓
4. Render the simplest real world
        ↓
5. Verify continuous behavior
        ↓
6. Improve organism observability
        ↓
7. Improve environment observability
        ↓
8. Diagnose lifecycle problems exposed by live running
        ↓
9. Add scientific instrumentation
        ↓
10. Measure and optimize performance
        ↓
11. Prove long-duration stability
        ↓
12. Build richer presentation
```

This order is intentional.

The first visible success should happen as early as possible, but every visible layer should sit on the real simulation rather than a temporary mock that later becomes architectural debt.

---

# 18. Definition of "Running"

For this roadmap, EvoSim is considered **running continuously** only when all of the following are true:

- The Rust simulation advances without requiring a frontend render.
- There is one authoritative simulation state.
- `Simulation::step()` is driven by a real runtime loop.
- Simulation time advances according to the runtime configuration.
- The process remains alive while the simulation runs.
- The simulation can be stopped or paused cleanly where supported.
- The browser receives observations from the real simulation.
- The displayed tick/state changes because the simulation changed, not because the frontend fabricated changes.
- Disconnecting the browser does not fundamentally stop simulation progression unless that is an explicit, deliberate runtime policy.
- Reconnecting the browser can observe the current simulation state.

---

# 19. Definition of "Coherent"

As we implement this roadmap, the system should progressively satisfy these architectural properties:

### Single source of truth

Simulation state has one authoritative owner.

### Clear responsibilities

- Rust core: simulation.
- Runtime: scheduling/lifecycle.
- Observation/API: exposing state.
- React: presentation and user-facing controls.

### No duplicated mechanics

The same simulation rule should not exist independently in backend and frontend.

### Explicit boundaries

A change in rendering should not require changing simulation rules merely to keep the UI alive.

### Testability

The simulation can be tested without the browser.

The observation layer can be tested without relying on visual rendering.

The frontend can be tested against representative observations without becoming the simulation itself.

### Replaceability

The viewer should eventually be replaceable without rewriting the simulation engine.

---

# 20. Definition of "Efficient"

Efficiency should mean:

- no unnecessary duplicate simulation work,
- no unnecessary browser-driven simulation stepping,
- no uncontrolled snapshot history,
- no needless serialization frequency,
- no needless frontend re-renders,
- no premature complexity,
- and measured optimization of actual bottlenecks.

A simple architecture that performs well enough is preferable to a sophisticated architecture whose complexity is not justified by measurement.

---

# 21. Engineering Discipline for This Roadmap

Development should proceed one focused change at a time.

For each change:

1. Identify the exact responsibility being changed.
2. Inspect the current implementation.
3. Make the smallest coherent change.
4. Run the relevant tests/checks.
5. Verify the runtime behavior when applicable.
6. Inspect the resulting architecture for accidental duplication.
7. Commit the change with a clear message.
8. Only then move to the next step.

When a test passes, that proves the tested behavior. It does not automatically prove the architecture is correct.

Likewise, a visually active screen does not prove the simulation is correct.

We should repeatedly verify both:

```text
Does it work?
    +
Is it still architecturally correct?
```

---

# 22. What We Should Avoid During This Phase

Do not:

- turn the frontend into a simulation engine;
- add fake movement or fake resource activity;
- make simulation rules depend on frame rate;
- make backend simulation progression depend on browser polling;
- introduce new mechanics solely because the first world view looks quiet;
- duplicate state models without a clear ownership reason;
- optimize snapshot copying before measuring it;
- introduce concurrency solely for theoretical performance;
- expose every internal field immediately;
- redesign established simulation mechanics while solving runtime infrastructure;
- treat the UI as the authority for simulation truth;
- let temporary demo code become the permanent runtime architecture.

---

# 23. Near-Term Working Checklist

The immediate work should stay focused on this checklist.

## Step A — Audit

- [ ] Find where `Simulation` is instantiated.
- [ ] Find every caller of `Simulation::step()`.
- [ ] Find the current server/runtime entrypoint.
- [ ] Determine whether a continuous scheduler already exists.
- [ ] Determine what `running` currently means.
- [ ] Find the existing snapshot/API path.
- [ ] Find the existing frontend data path.
- [ ] Document the current end-to-end flow.

## Step B — Runtime

- [ ] Establish one authoritative simulation loop.
- [ ] Make tick scheduling explicit.
- [ ] Make lifecycle semantics explicit.
- [ ] Verify backend-only continuous operation.

## Step C — Observation

- [ ] Reuse or refine `Snapshot` as the observation boundary.
- [ ] Define the minimum live observation payload.
- [ ] Serialize without mutating simulation state.
- [ ] Establish live transport.

## Step D — Viewer

- [ ] Connect React to live observations.
- [ ] Display live tick.
- [ ] Display world.
- [ ] Display real organisms.
- [ ] Display real environment state.
- [ ] Confirm updates are coming from the real engine.

## Step E — Stabilize

- [ ] Run continuously for minutes.
- [ ] Check lifecycle behavior.
- [ ] Check material/environment behavior.
- [ ] Check organism activity.
- [ ] Identify actual lifecycle gaps.

Only after these steps should we broaden the viewer or begin substantial optimization.

---

# 24. First Milestone to Target

The first major milestone is deliberately modest:

> **Start EvoSim, leave it running, open the browser, and watch the real simulation advance continuously in front of you.**

The world can be ugly.

The UI can be minimal.

The simulation can still be scientifically primitive.

But the state on screen must be real, continuously generated by the authoritative Rust simulation, and connected through a clean observation boundary.

Once that works, EvoSim stops being primarily a collection of simulation code and becomes a **living system that we can observe, diagnose, and iteratively improve**.

That is the foundation for everything that follows.
