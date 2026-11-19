# EvoSim Chemical Laboratory

The Chemical Laboratory is a separate application backed by the same Rust engine as the main EvoSim simulator. It starts with an empty environment and no organism; it does not introduce a second chemistry implementation or laboratory-only reaction rules.

## Run

```bash
cargo run --bin chemistry-lab
```

The default address is `http://127.0.0.1:3030`. Set `EVOSIM_LAB_ADDRESS` to bind another socket address, for example `0.0.0.0:3030`.

## Current workbench controls

- Set world width and height, then reset the experiment.
- Set the deterministic random seed for a reset.
- Configure ambient flow direction, speed, S-field strength, and local vortex intensity; changes apply immediately while paused or running, and the renderer displays the configured vector field.
- Enable or disable the existing 440-cycle/day world tone and 880-cycle/day localized hotspot. Both are off by default and use the simulator's existing source settlement and energy ledger.
- Place catalog elements by selecting an element and tapping the field (initial placement is restricted to paused tick zero).
- Place a realized molecule at paused tick zero by submitting JSON with constituent resource names, bond index pairs, and one exact `[x, y, rotation_radians]` placement per constituent.
- Run, pause, step once, or advance 100 ticks.
- Select and inspect individual molecules by tapping them; inspect stable identity, constituent composition, exact coordinates, rotations, and bond graph.
- Observe bounded environmental reaction history, outcomes, work requirements, products, and molecular creation/parent lineage.
- Inspect the authoritative energy-ledger counters for external input, usable/resonant/structural holdings, pair potential, work, heat, and world-tone unabsorbed/escaped energy. History is collected from the shared engine's committed/rejected event records; no reaction is forced for display.
- Zoom with the mouse wheel/pinch and pan by dragging the field.
- Export or import a reproducible experiment setup, including world dimensions, seed, flow/source settings, and all tick-zero element/molecule placements. Import rebuilds the initial state through the engine's normal geometry and validation path.

Molecule placement is validated by the existing `PhysicalMaterial::realized` geometry and bond rules. Invalid geometry is rejected; the lab does not invent bond endpoints or force a reaction.

## HTTP API

- `GET /api/state` — current tick, environment dimensions, material inventory, identities, placements, and bond graph.
- `POST /api/reset` — replace the experiment with a new empty environment. Body: `{"width":100,"height":100,"cell_size":5,"ticks_per_second":20,"seed":42}`.
- `POST /api/place` — place one catalog element. Body: `{"resource":"Carbon","x":20,"y":30}`.
- `POST /api/molecule` — place one physically realized molecular graph. Body: `{"parts":["Carbon","Oxygen"],"bonds":[[0,1]],"placements":[[45,50,0],[47,50,0]]}`. The sample coordinates are illustrative; geometry must satisfy the engine's actual bond validation. Flow defaults preserve the existing S-shaped field exactly.
- `POST /api/run` — begin real-time stepping at the configured tick rate.
- `POST /api/pause` — pause real-time stepping.
- `POST /api/step` — step a bounded number of ticks. Body: `{"count":1}`.
- `POST /api/flow` — change the live ambient field. Body: `{"direction_degrees":90,"speed":2,"strength":0.5,"vortex_strength":1}`. Direction is measured clockwise from right; speed is a multiplier, strength blends uniform flow (0) with the S-shaped field (1), and vortex strength ranges from 0 to 2.
- `POST /api/sources` — enable or disable the established external sources. Body: `{"world_tone_enabled":true,"hotspot_enabled":true}`. Source capture is determined by existing physical material response and is settled through the authoritative energy ledger; enabling a source does not grant energy directly.
- `GET /api/state` also returns the latest 2,000 reaction events in `reaction_history`, including source materials, work, outcomes, products, and lifecycle parent IDs. The same response includes the current energy-ledger counters in `energy`.
- `GET /api/experiment` — export the reproducible tick-zero setup document.
- `POST /api/experiment` — validate and load a setup document. Invalid dimensions, placements, material names, or molecular geometry reject the entire import without replacing the current experiment.

## Scope of this milestone

This is the laboratory foundation, not the complete experimental workbench. The next milestones add configurable flow and energy sources, richer material/bond rendering and inspection, removal and editing operations, persistent experiment files, and reaction/lineage/energy diagnostics. Current energy sources are disabled by default so the empty starting state does not receive an unaccounted energy grant. Pre-placed molecular bond potential is included in the tick-zero structural-energy baseline; adding material after the first tick is currently rejected until explicit material-injection energy transactions are implemented.
