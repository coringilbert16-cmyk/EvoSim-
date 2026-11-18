# Optimization 02 — Persistent chemistry and material-property caching

## Scope

This is workstream 2 of 4 and is intentionally separate from world-tone exposure (PR #369, closed without merge). It investigates and, only where measurements support it, implements persistent caching of chemistry-derived and material-derived properties.

The goal is to avoid rebuilding invariant chemical descriptions on every simulation pass while preserving exact physical behavior. This work must not introduce organism-specific chemistry, approximate properties, reduced interaction sets, or stale cached values.

## Required investigation before implementation

1. Inventory repeated property construction and lookup in the real tick path, including world-tone snapshot assembly, environmental chemistry snapshots, reaction candidate discovery, and geometry/material preparation.
2. Classify each property by its true dependencies:
   - composition/catalog identity;
   - bond topology;
   - constituent identity and ordering;
   - realized shape / physical state;
   - placement and orientation;
   - field or global environment revisions.
3. Identify existing stable identities, topology generations, state versions, and field revisions that can serve as cache keys or invalidation signals. Do not add a parallel revision system if existing authoritative versions suffice.
4. Measure build frequency, hit potential, memory cost, and time spent constructing each candidate property. Distinguish chemistry-invariant properties from geometry and placement, which can change independently.
5. Establish differential tests proving cached and uncached results are identical, including invalidation after composition, bond, shape, and catalog changes.

## Design constraints

- Movement alone must not invalidate composition-derived chemistry properties.
- Bond changes must invalidate bond-dependent properties, but not unrelated composition-only values.
- Placement changes must not invalidate composition-only chemistry properties.
- Cache lifetime and ownership must be explicit; no global mutable cache or hidden cross-simulation sharing.
- Cache misses and invalid/stale keys fail closed to exact recomputation.
- Deterministic ordering and all existing energy-ledger semantics remain unchanged.
- No arbitrary cutoffs, skipped candidates, approximate physics, or chemistry-specific special cases.

## Acceptance criteria

- A measured, material end-to-end workload benefit, not merely a faster isolated lookup.
- Exact equivalence against the uncached reference path across deterministic tests and standard 60-second environmental chemistry runs (1,280 physical objects/resources under the established setup where applicable).
- Full formatting, architecture checks, tests, smoke experiment, and clippy green.
- Documented memory overhead and invalidation behavior.
- No merge until the user explicitly approves after review.

## Sequencing

Priority 3 (dependency-driven invalidation/event-triggered recalculation) and priority 4 (exact resonance movement math) remain separate PRs. They must not be stacked onto this branch. Complete and close this PR before starting priority 3.
