# Phase 3 — Live Ambient Energy Implementation

**Status:** Implemented and validated by full exact-head CI.

## Contract implemented

- One simulated day is 24,000 simulation ticks.
- The ambient carrier advances 440 cycles per day.
- The cycle-averaged energy density follows the approved cosine envelope from 10 at tick zero to 1 at tick 12,000 and back to 10 at tick 24,000.
- The accounting footprint is the environment area (250 × 250 = 62,500 project-area units); it is not a propagation boundary.
- Opening ambient energy is initial inventory, not generated energy.
- For each tick, signed exchange is the change in stored ambient energy. Positive change is booked as world generation; negative change as world withdrawal. The live ledger retains cumulative generated, cumulative withdrawn, current held, and opening held values.
- Ambient phase follows the simulation tick. Runtime pacing (`ticks_per_second`) does not affect it.

## Production integration

The mode is owned by `Simulation`, initialized with the live environment, and advanced once in the environmental simulation tick. It is no longer only an isolated reference harness. Ambient accounting is distinct from chemistry credits and receiver/material settlement.

## Validation

Production-level contract tests cover the four daily envelope checkpoints, one-day generation/withdrawal closure and ending inventory, and identical ambient state at different runtime pacing values. The existing wave solver was not modified; no global Klein–Gordon term, speculative coupling, or material rule was added.

Full exact-head CI passed on implementation commit `372ee90ead9736e99d824df299a20d94357c3dd4` in [workflow run #45458137482](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/45458137482). All workflow steps passed, including formatting, architecture checks, offline analyzers, Rust tests, environmental chemistry smoke simulation, manifest validation, both experiment analyzers, artifact upload, and Clippy.

The report records the validated implementation commit and its CI result. The subsequent report-only commit must also pass exact-head CI before merge approval.
