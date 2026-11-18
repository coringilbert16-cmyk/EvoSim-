# Optimization 01 — World-tone exposure and settlement

## Objective

Reduce the computational cost of world-tone exposure and energy settlement without changing which physical bodies participate, which geometry occludes a receiver, or how energy is conserved and credited.

This is the first implementation workstream in the computational-optimization sequence. It is intentionally isolated from the later chemistry-cache, dependency-invalidation, and resonance-mathematics workstreams.

## Current evidence

The existing whole-tick profiling measured world-tone settlement at roughly 56 ms per sampled tick in the initial profile. A later split profile identified world-tone exposure as the meaningful non-movement cost; hotspot settlement was approximately 1.0–1.1 ms/tick. These are diagnostic measurements, not clean throughput acceptance results.

The existing implementation already builds one world-wide physical snapshot and one exposure frame. All realized bodies remain part of the occluder set and all physical receivers remain eligible for energy accounting, including nonliving environmental material.

## Investigation and implementation boundaries

1. Trace the complete live path from world-tone sampling through snapshot construction, exposure discovery, material response, and energy-ledger settlement.
2. Identify independent receiver/body calculations that can safely run from immutable snapshots.
3. Determine whether geometry preparation or exposure intermediates are redundantly rebuilt within a tick.
4. If parallel work is justified, use the existing deterministic parallel execution foundation. Workers must read immutable snapshots and return local results; authoritative simulation and ledger mutation remain on the ordered reconciliation path.
5. Preserve the complete receiver and occluder sets. Do not omit bodies, rays, lanes, interactions, or energy recipients for performance.
6. Preserve exact geometry predicates, current exposure semantics, material absorption/damping behavior, heat accounting, and energy-ledger transaction rules.
7. Add differential tests against the existing sequential/reference behavior and measure clean end-to-end throughput under the standard 60-second seed-42 workload.

## Acceptance criteria

- Every physical receiver and occluder remains represented.
- Exposure and per-receiver energy results match the reference implementation under deterministic test fixtures.
- No duplicate energy credit, lost energy, or change to heat/unabsorbed accounting.
- Worker failure cannot commit partial results.
- Full Rust tests, formatting, architecture checks, and Clippy pass.
- Clean matched 60-second seed-42 benchmark shows a repeatable end-to-end improvement before any production path is selected.
- No merge until the implementation and validation are reviewed.

## Explicit non-goals

- No changes to chemistry or reaction rules.
- No changes to material geometry, physical movement, or collision semantics.
- No caching whose invalidation depends on work assigned to a later optimization.
- No approximate visibility, reduced occluder sets, ray sampling shortcuts, or altered energy physics.
- No global thread-pool introduction unless separately measured and justified.

## Sequence

This PR is only workstream 1. The following workstreams must be separate PRs, created only after this PR is completed and closed:

2. Persistent chemistry/material-property caching.
3. Dependency-driven invalidation and event-triggered recalculation.
4. Exact simplification of resonance movement pair calculations.

Each workstream must be completed, validated, and closed before starting the next. No stacked implementation branches.
