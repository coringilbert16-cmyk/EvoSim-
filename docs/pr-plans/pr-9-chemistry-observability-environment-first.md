# Add Chemistry Event Inspection and Environment-First Experiments

> **Planning scaffold only.** This phase defines an experiment workflow; it does not implement the UI, runtime, or production code.

## Purpose

Make chemistry understandable and testable before organism design is layered on top. The user should be able to configure a world, run it with no organisms, and inspect whether and how material reactions occur. This is observability of real simulation events, not a fabricated activity feed.

## Scope

- Separate environment initialization/configuration from initial organism population while preserving the current default environment preset and existing startup behavior unless explicitly selected otherwise.
- Provide a no-organisms run mode so environment chemistry can be exercised independently of organism construction.
- Expose structured, bounded chemistry events from the authoritative runtime: candidate considered/rejected, activation/progress where useful, committed reaction, and explicit failure reason. Clearly distinguish attempted candidates from committed physical reactions.
- Include enough event context to explain participating material identities, location/contact, energy/material deltas, and transaction outcome without duplicating ownership state.
- Allow filtering/aggregating events and inspecting world inventory snapshots; avoid retaining an unbounded per-tick log.
- Add repeatable benchmark scenarios for 24,000 ticks with environment-only and mixed environment/organism material workloads, reporting runtime, candidate counts, committed reactions, and inventory residuals.

## Acceptance criteria

- User can configure and initialize an environment without creating an organism, then run and pause the simulation.
- The default environment preset remains available and is not silently changed.
- An environment-only run can show zero reactions honestly, or show committed reactions produced by the shared chemistry runtime; UI does not synthesize reactions.
- Logs distinguish candidate attempts from committed reactions and show why failed candidates did not commit.
- Material and energy inventory snapshots use independent checks and never silently repair residuals.
- Event retention is bounded and deterministic under a documented configuration.
- A repeatable 24,000-tick benchmark reports workload size, time, event/candidate volume, and conservation results; any timeout is reported as incomplete.

## Dependencies and boundaries

- Depends on PR #214 shared chemistry runtime/event contract. It must not invent its own chemistry policy or directly mutate material/energy.
- PR #207/#208 provide inventory semantics. PR #209 provides transactional realization.
- Keep this phase focused on diagnostics and experiment workflow; do not redesign organism constructor or Bob geometry-library behavior (PR #196).

## Guardrails

- Audit current UI and simulation initialization before changing the workflow.
- Do not change the default world dimensions/preset as a side effect of separating environment and organism setup.
- Do not claim viability or a complete chemistry system merely because logs are visible.
- Keep per-command/test runs under five minutes; the 24,000-tick benchmark itself must have a bounded workload and report timeout as incomplete evidence.

## Status

- [ ] Audit current initialization/UI path
- [ ] Environment-only configuration and run
- [ ] Structured event inspection
- [ ] 24,000-tick performance and conservation benchmarks
- [ ] Exact-head verification and baseline comparison

## Current dependency/status reconciliation — 2026-10-30

- PR #214 shared chemistry runtime/scheduler is merged; this plan must consume its real candidate, activation, commit, and failure paths rather than inventing a parallel event system.
- PR #223 environmental COMBINE discovery/settlement and PR #226 flow/obstacle-aware transport are merged.
- PRs #229–#240 now provide the base/compound chemistry matrices, multi-constituent transaction and round-trip evidence, heterogeneous constituent identity, fair bounded contact discovery, and molecular-layout geometry inputs.
- This PR remains planning-only. No UI, runtime instrumentation, or production code is authorized by this document.

Before implementation, audit current-main initialization, event production, and world inventory inspection. Preserve the default world preset. Environment-only mode must not create an organism or use organism energy as a hidden environmental source. Events must distinguish candidates and activation from committed physical reactions, and retention/work must remain bounded and deterministic.
