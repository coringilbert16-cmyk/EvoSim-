# EvoSim Observation Architecture

This document describes the current browser-facing observation architecture. It is an implementation architecture note, not a replacement for the authoritative simulation specification.

## Authority boundary

- The Rust simulation remains authoritative.
- Observation data is derived read-only from simulation truth.
- The browser does not maintain a second simulation state.
- Camera, selection, and focus are browser-owned view state.
- Exact physical geometry is obtained from authoritative simulation geometry; the browser does not reconstruct it from resource names.

## Observation levels

### World

World observation is deliberately coarse. Each organism exposes only:

- stable organism ID;
- representative position;
- authoritative world-space bounds.

World observation also exposes spatially aggregated field material, vents, and decomposing-body locations.

It does **not** expose exact organism forms, structure units, bonds, energy, stress, genome, decision history, or other hidden bookkeeping.

### Organism

Organism observation exposes the selected organism's authoritative physical silhouette, location, bounds, unit count, and bond count.

It remains above structural detail: internal material composition, individual placements, and individual bonds are not required to render the organism silhouette.

### Structure

Structure observation is the exact structural view. It exposes:

- structural units;
- material composition;
- authoritative unit placement;
- resolved external forms;
- bonds;
- bond endpoints derived from authoritative connection sites;
- bond strength and bond formation energy.

No browser-side reconstruction of structural geometry is performed.

## Server boundary

The browser-facing observation routes are:

- `/observation/status`
- `/observation/world`
- `/observation/organism/{id}`
- `/observation/structure/{id}`
- `/observation/resources`

The legacy `/snapshot` and `/ws` transport paths have been removed. The server no longer clones or broadcasts a complete simulation snapshot for browser rendering.

## Update behavior

The browser first checks the lightweight observation status endpoint. A larger observation payload is requested only when the simulation tick or observation target changes. Stale asynchronous responses are rejected by a browser-local request serial.

## Resolution rule

Observation level is semantic resolution, not camera zoom. Changing camera scale must never cause a lower-level observation payload to become more detailed. Moving from World to Organism to Structure is an explicit observation transition.

## Geometry rule

A visualization may simplify an observation, but it may not invent authoritative physical detail that was not observed. In particular, aggregate field material is rendered as concentration rather than fabricated into individual particles, and `Fluid` forms are never assigned an artificial finite boundary.
