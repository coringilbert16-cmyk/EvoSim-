# Phase 3A.4d — Checked Source-Footprint Geometry

**Status:** Geometry-only implementation slice. This milestone completes the distinct 2D support-area adapter; it does not activate or register a source.

**Authority:** `docs/design/resonance-field-implementation-roadmap.md` and the Phase 3A.4 physical-measures proposal. The adapter builds only on the checked polygon and polygon-union measures from 3A.4a–c.

## Goal

Provide a separately named, checked L² measure for the actual physical support region of an in-plane distributed source. The measure must not be inferred from receiver visibility, exposed boundary length, constituent count, projected screen extent, or a perimeter-to-area conversion.

## Contract

- Input is one or more simple supported polygon boundaries expressed in a common world-coordinate frame.
- The support region is the geometric union of those polygons. Overlapping and coincident pieces contribute only once.
- Output is `SourceFootprintArea`, with area and numerical uncertainty both in squared world units (L²).
- The adapter reuses the checked Phase 3 polygon-union implementation; it does not create a second polygon-area or union algorithm.
- Input vertices are already in world coordinates. Callers with placed rectangles or other forms must transform their vertices into the common frame before calling.
- Unsupported, concave, malformed, degenerate, non-finite, or numerically unresolved input fails closed.
- The adapter is distinct from `PolygonUnionArea` at the API boundary even though it reuses the same geometric result internally.

## Explicit non-goals

- No source identity, source registration, schedule, frequency, phase, power, budget, or replenishment law.
- No power-density integration or numerical quadrature of spatially varying density.
- No wave-field injection, propagation, receiver coupling, material response, energy transfer, heat, or ledger changes.
- No chemistry, movement, organism, GUI, or production behavior changes.
- No circle or fluid support in the strict Phase 3 geometry path.

For a later source-injection phase, constant power density may be multiplied by this validated footprint area. Spatially varying density requires its own explicit integration method, convergence evidence, and error report; it is not part of 3A.4d.

## Acceptance tests

- A single rectangle polygon returns width × height.
- Disjoint polygons return the sum of their areas.
- Overlapping polygons return union area, counting overlap once.
- Coincident polygons are not double-counted.
- Translation preserves area within the reported uncertainty.
- Uniform scaling by (s) scales area by (s^2).
- Empty, degenerate, malformed, unsupported, and concave input fails closed.
- The adapter exposes L² only and does not call or depend on visible-boundary intervals.
- The existing 3A.4a–c geometry tests remain unchanged and passing.

## Exit gate

Phase 3A.4d is complete when the adapter and acceptance tests pass on exact-head CI, the API remains geometry-only, and the Phase 3A.4a–c guarantees remain intact. This closes the source-footprint geometry slice only; it does not authorize Phase 3B source-to-field coupling.
