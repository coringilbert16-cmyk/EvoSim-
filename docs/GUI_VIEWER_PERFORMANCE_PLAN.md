# GUI Viewer Performance Plan

Status: approved direction for PR #255; implementation plan, not a claim that every optimization is already implemented.

## Current implementation progress (2026-10-31)

- Implemented a deadline-based tick scheduler with a 10 TPS default, a 0.1–100 TPS control range, and independent 100 ms display publication. The scheduler avoids catch-up bursts and remains responsive to pause/speed changes at low rates.
- Restored the speed slider and report configured TPS separately from measured achieved TPS.
- Added camera scale bounds (0.02–100), removed camera-triggered world observation requests, and made world observations full-world/incremental rather than viewport-scoped.
- Replaced per-cell/per-material radial gradients and viewport-relative normalization with deterministic dominant-material cell rendering. The world render path now draws the field once, preserves hotspot/physical silhouettes, and culls off-screen field cells and geometry.
- Automated validation is still pending on the latest commit. The first check exposed rustfmt-only differences in the new scheduler; those formatting changes have been applied and need a fresh CI result.
- Remaining work: validate runtime behavior, add/verify zoom and pacing regression coverage, refine zoom-dependent field aggregation/LOD, and profile the live viewer before considering heavier batching or parallelism.

## Objective

Make the environmental chemistry experiment comfortable to observe and interact with while preserving the simulation as the sole authority for physical and chemical state. Improve responsiveness by reducing redundant work, stabilizing observation data, and making rendering proportional to what is visible.

The target is not to reproduce RimWorld's engine or adopt a new rendering framework. EvoSim has a different correctness requirement: **camera position, zoom, visibility, and rendering cadence must never change chemistry, organism behavior, energy accounting, or simulation time.**

## External references and transferable lessons

1. **RimWorld 1.7 / Waylith engine transition** — Ludeon's developer-reported performance results illustrate the gains possible from addressing the engine and whole execution pipeline rather than only drawing. Treat headline multipliers as developer-reported results, not an EvoSim target.
   - https://ludeon.com/blog/2026/10/announcing-the-waylith-engine-and-update-1-7/
   - Use: motivation to measure end-to-end costs and remove architectural bottlenecks before micro-optimizing individual draw calls.
2. **RimWorld 1.6 performance work** — Ludeon's update notes describe work such as multithreaded/batched pathfinding and lighting, and less frequent updates for selected non-critical systems.
   - https://store.steampowered.com/news/app/294100/view/498322216088241321
   - Use: batch independent work, isolate expensive subsystems, and schedule presentation/ancillary work at appropriate rates. Do not copy visibility-based throttling for EvoSim chemistry.
3. **ParaView: Displaying data** — https://docs.paraview.org/en/latest/UsersGuide/displayingData.html
   - Use: choose representations by data type and task (points, surfaces, outlines, wireframes, volume); use inexpensive overview representations while navigating and richer detail when inspecting.
4. **ParaView: Parallel data visualization** — https://docs.paraview.org/en/latest/ReferenceManual/parallelDataVisualization.html
   - Use: keep data production and visualization separable; avoid requiring the renderer to own or duplicate the authoritative simulation.
5. **deck.gl performance guide** — https://deck.gl/docs/developer-guide/performance
   - Use: preserve stable data/buffers, update only changed data, chunk incremental work, and avoid rebuilding all render inputs on every frame.
6. **deck.gl aggregation layers** — https://deck.gl/docs/api-reference/aggregation-layers/overview
   - Use: aggregate dense field data when individual primitives no longer provide useful information at the current zoom. Profile CPU versus GPU aggregation before choosing.
7. **PixiJS ParticleContainer** — https://pixijs.download/v8.14.0/docs/scene/ParticleContainer.html
   - Use: for large numbers of simple repeated visual elements, use shared geometry/material-like data and compact per-instance attributes rather than allocating complex objects and gradients for each item. This is a design reference; do not add PixiJS without evidence it solves a measured bottleneck.

## Target architecture

Keep five responsibilities explicit:

1. **Simulation runtime (authoritative)**
   - Owns ticks, chemistry, geometry, energy, and organism state.
   - Advances at a user-configurable simulation rate with a real scheduler; no tight-loop spin that ignores the configured rate.
   - Simulation cadence is independent of browser frame rate and network/display refresh.
   - Never skips or slows chemical/physical updates because an entity is off-screen.
2. **Observation/snapshot boundary**
   - Produces a consistent, read-only observation of simulation state.
   - Publishes the newest usable state rather than queuing every intermediate frame for the browser.
   - Reuses stable identifiers and unchanged data where practical; avoids cloning the entire world every tick.
   - Camera movement should not change the meaning, scale, or normalization of material quantities.
3. **Renderer**
   - Has distinct field, physical geometry, organism/selection, and UI-overlay layers.
   - Each layer has one clear owner and is drawn once per frame. Remove duplicate field rendering rather than layering another renderer over the original path.
   - Uses stable material identity/color and consistent absolute or documented reference scaling. Do not normalize colors against a camera-dependent or viewport-dependent subset.
   - Avoid per-cell/per-material radial-gradient allocation and uncontrolled translucent overdraw. Prefer cached/shared styles and simple cell/instance primitives; aggregate dense fields at overview scales.
4. **Camera and level of detail**
   - Enforce explicit minimum and maximum zoom.
   - Pan/zoom should update the camera transform and redraw locally; it should not trigger a new authoritative simulation observation merely because the camera moved.
   - Cull drawing outside the viewport from a stable snapshot.
   - Use zoom-dependent representations: aggregate field/outline at far zoom, individual material geometry at close zoom, and richer labels/selection details only when useful.
   - LOD affects presentation only, never simulation state or update cadence.
5. **Controls and telemetry**
   - Expose pause/resume, single-step, reset, and a meaningful configurable ticks-per-second control.
   - Show configured TPS separately from measured achieved TPS so throttling and performance shortfalls are distinguishable.
   - Keep display refresh independent (roughly 10–30 Hz is a starting range to measure, not a fixed requirement).

## Implementation sequence

### Stage 0 — establish a trustworthy baseline
- Record exact PR head, workload/seed, world dimensions, organism/material counts, configured TPS, achieved TPS, observation latency, renderer frame time, and memory behavior over a fixed run.
- Profile the existing viewer with camera stationary, panning, zooming, and a dense field.
- Preserve a reproducible baseline so each later phase can demonstrate improvement.

**Acceptance:** baseline measurements can be repeated; configured and achieved TPS are not conflated.

### Stage 1 — correct runtime pacing and controls
- Make the simulation loop honor the configured TPS with deadline-based pacing (account for work time and avoid accumulating drift).
- Define pause, single-step, reset, and speed-change behavior explicitly.
- Keep websocket/display publication on its own cadence.
- Start with a moderate default rate suitable for observation; expose a usable range and allow higher rates when desired.

**Acceptance:** a one-minute wall-clock run at a selected rate advances approximately the expected number of ticks within a documented tolerance; pause stops ticks; single-step advances exactly one; display refresh does not dictate simulation cadence.

### Stage 2 — stabilize observation and camera interaction
- Inspect the existing observation endpoint and viewport payload contract before changing it.
- Establish whether the server returns a full stable observation or camera-scoped data; document that contract.
- Prevent pan/zoom from requesting fresh simulation observations when only a local transform/redraw is needed.
- If server-side viewport queries are necessary for scale, use fixed world-space material normalization and cache keys independent of camera position where possible.

**Acceptance:** panning/zooming does not alter material color/intensity for unchanged world state; camera interaction does not increase simulation tick work or cause a burst of redundant observations.

### Stage 3 — make rendering layers single-owner and deterministic
- Trace the complete render call graph, including original renderWorld, renderWaterField, resource-aware overlays, structure rendering, and selection.
- Remove duplicate drawing paths; retain one field renderer and one geometry/organism renderer per layer.
- Replace overlapping per-material radial gradients with a deterministic field representation appropriate to the data (cell fill, composited material channels, or density aggregation).
- Cache palettes/styles and stable geometry; invalidate only when the corresponding source data changes.

**Acceptance:** each layer is drawn once; unchanged state produces identical colors and geometry after camera movement; draw-call/gradient counts fall measurably on the same scene.

### Stage 4 — add camera bounds, culling, and zoom-based LOD
- Add tested minimum/maximum scale limits and preserve the cursor-anchored zoom behavior.
- Compute visible world bounds once per frame and cull non-visible render primitives.
- Use coarse field aggregation and simplified organism outlines at far zoom; restore detailed physical forms at close zoom.
- Keep selection and inspection capable of requesting detail for a selected entity without forcing all entities into high detail.

**Acceptance:** zoom cannot become unusably small/large; panning a dense scene draws only visible content; far zoom reduces rendering work; close zoom restores detail; simulation outputs are invariant across camera states.

### Stage 5 — profile-driven batching and parallel work
- Only after profiling, batch field aggregation, observation serialization, and other independent work.
- Consider worker threads or GPU-backed aggregation only where measurements show a bottleneck and correctness boundaries are clear.
- Avoid introducing a graphics framework or a new threading architecture as speculative infrastructure.
- Track allocations, snapshot size, observation latency, frame time, and sustained TPS across long runs.

**Acceptance:** each optimization has before/after measurements and regression coverage; no improvement is accepted if it changes simulation results.

## Non-negotiable invariants

- Simulation rules and outcomes are independent of camera, viewport, frame rate, and visibility.
- Renderer consumes read-only observations; it never owns or mutates physical/chemical truth.
- Material colors and scales are stable for unchanged state and are not computed from whichever subset happens to be visible.
- No full-world clone on every tick or every render frame.
- No unbounded queue of obsolete observations or frames.
- User-configured TPS is honored; measured TPS is reported honestly.
- Do not add a dependency or undertake an engine rewrite without profiling evidence.

## PR #255 immediate order

1. Correct the disabled/forced-full-speed control and implement real tick pacing.
2. Add min/max zoom.
3. Remove duplicate field rendering and stabilize field appearance.
4. Stop camera motion from triggering redundant observation fetches, after confirming endpoint semantics.
5. Add viewport culling and basic zoom-dependent LOD.
6. Measure and then decide whether batching/parallelism or a rendering dependency is justified.

This sequence prioritizes correctness and immediate usability, then removes the confirmed rendering inefficiencies, and only then invests in larger performance architecture.

## Live GUI acceptance failure and corrective pass (2026-10-31)

The first manual run did not meet the core viewer acceptance criteria. The observed failures were traced to concrete integration gaps:

- Startup selected the hard-coded Structure view for organism ID 1, even though the environmental chemistry experiment intentionally starts with zero organisms. Startup now requests the World observation.
- The workspace script reparents the canvas after the original resize call. The canvas backing bitmap could therefore retain its old dimensions and be stretched into the new flex viewport. A ResizeObserver now keeps the backing dimensions synchronized with the displayed canvas.
- Geometry outlines were assigned a world-space line width with a lower bound that grew in screen pixels as zoom increased. The line width is now inversely scaled so outlines remain visually stable.
- World-field observations previously exposed only aggregate cell totals, and the renderer filled those cells as squares. Observations now include each physical constituent's resource identity, current position, rotation, and realized/catalog form. Incremental observations preserve unchanged physical geometry in the client cache.
- The field renderer now draws those physical resource forms with their material palette; unstructured stock remains a small cell marker rather than a fabricated square atom.
- Camera limits are now relative to the fitted view, and visible zoom-in, zoom-out, and fit controls supplement wheel zoom.

A regression test verifies that the environmental experiment's world observation exposes all 128 realized resource constituents, with finite placements, valid forms, and all seven resource types.

**Still required before merge:** rerun CI on the latest head and repeat live GUI acceptance. Confirm that the world field is visible on startup, resource positions visibly update during a sustained run, zoom limits work in both directions, pan/zoom no longer distort geometry, and close zoom reveals the resource-specific shapes. These code corrections are not a substitute for that manual verification.
