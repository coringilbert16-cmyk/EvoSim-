# Phase 3A.4 — Physical Area and Boundary Measures

**Status:** Audit and technical proposal for review. This document does not authorize implementation.

**Authority:** The resonance-field implementation roadmap (docs/design/resonance-field-implementation-roadmap.md), especially its geometry/coupling measures and Phase 3 requirements. The strict Phase 3A.3 geometry contract is docs/plans/PHASE3A3_VISIBILITY_TRANSITIONS.md.

## 1. Purpose and boundary

Phase 3A.3 established a strict polygon/finite-line visibility path. Phase 3A.4 must define reliable physical measures that later field/source work can consume without conflating dimensions or double-counting overlapping material.

This milestone is geometry-only. It must not introduce source injection, wave propagation, field/material coupling, material response, energy settlement, chemistry, movement, organism behavior, or production activation.

The audit finds useful existing primitives, but no single authoritative Phase 3 API for polygon material area, polygon-union area, or source-footprint area. Existing perimeter and visibility APIs are not substitutes for those missing measures.

## 2. Physical quantities and dimensions

Keep these measures distinct in names, types, documentation, and tests:

| Quantity | Dimension | Meaning | Current status |
|---|---:|---|---|
| PolygonArea | L² | Area enclosed by one validated polygon | No shared Phase 3 area API |
| PolygonUnionArea | L² | Area occupied by the union of multiple realized polygon constituents | No union-area API |
| ExposedBoundaryLength | L | Length of disjoint exposed/visible receiver-boundary intervals | Existing interval data; length can be summed |
| SourceFootprintArea | L² | Area of the physical 2D support region of an in-plane distributed source | No Phase 3 API |
| ProjectedBoundaryExtent | L | Directionally projected 2D boundary measure, e.g. integral of max(0, n·s) ds | Existing projected-extent helpers |

In a 2D world, boundary length remains L. A directional projection of a boundary is also L, even if it plays a role analogous to projected area in 3D. Neither is an area in the world plane.

A source's physical support area is a separate 2D region. It must not be inferred from receiver visibility, exposed perimeter, constituent count, or a perimeter-to-area conversion.

## 3. Existing implementation audit

### 3.1 Geometry representations

Form in src/materials/resources.rs exposes polygon_vertices() for rectangles, regular polygons, and explicit polygons. A line has finite length but zero area. Circles and fluids remain legacy geometry and are excluded from the strict Phase 3 path.

Form::is_valid() validates explicit polygons using a shoelace sum and segment-intersection checks. That validation currently uses a fixed absolute epsilon (1e-12) and accumulates the shoelace expression in vertex coordinates without translating to a local origin. It is not an adequate numerical authority for the new Phase 3 area contract across the scale and translation range already required by Phase 3.

src/geometry/phase3_numeric.rs provides scale-aware linear, cross-product, and segment-parameter tolerances for Phase 3 predicates. It does not currently provide an area accumulator or area-specific error policy. The Phase 3 adapter still calls Form::is_valid(), so the existing polygon validity predicate is a known boundary to resolve before claiming scale-independent polygon-area support.

### 3.2 Exposed and visible boundary length

ExposedBoundaryInterval in src/geometry/source_exposure.rs represents one straight boundary interval with world-space endpoints and an outward unit normal. Phase 3 visibility returns disjoint intervals with exact partition endpoints. The interval length is the Euclidean endpoint distance.

visible_material_perimeter_length() sums straight lengths and exact circular-arc lengths for the broader legacy perimeter representation. Phase 3's disjoint_polygonal_material_perimeter() adapter rejects curved pieces and restricts material perimeter to polygonal solids.

The perimeter/visibility work already supplies useful contracts:
- Visibility partitions at geometric events and returns disjoint intervals.
- The strict Phase 3 adapter accepts polygon receivers and finite line blockers, and rejects circles/fluids throughout the snapshot.
- Concave receiver targets fail closed because the current visibility algorithm does not yet resolve self-occlusion for them.
- Numerical robustness tests cover small/large scale and large translations for visibility intervals.

The exposed-length API still needs a Phase 3-specific checked measure that validates finite, non-degenerate intervals, rejects duplicate/overlapping contributions, and reports the sum in L. It should preserve interval endpoints rather than reconstructing them from rounded totals.

### 3.3 Polygon union perimeter

disjoint_source_global_material_perimeter() in src/geometry/source_exposure.rs is an existing composite-perimeter union routine. It:
- Builds straight polygon edges (and legacy circular arcs).
- Splits pieces at segment/segment and supported curved intersections.
- Classifies sub-intervals against other constituents.
- Removes internal opposing shared edges and deterministically assigns same-side duplicate ownership.
- Returns separate straight and curved pieces.

The strict Phase 3 wrapper rejects curved pieces and non-polygon forms. The current polygon union perimeter implementation requires convex polygons and rejects line/fluid forms. It has focused tests for overlapping polygon/circle unions, intersecting circles, duplicate identity, and fail-closed fluid cases; Phase 3-specific acceptance still needs polygon-only tests for shared edges, touching vertices, containment, overlapping convex polygons, and stable ownership.

Important limitation: the existing perimeter routine is not a polygon-union-area API. It does not expose a validated closed-loop/topology result or an area measure. It also contains legacy fixed tolerances in intersection and ownership logic, so it must not be assumed to inherit all Phase 3 scale-aware guarantees merely because the Phase 3 adapter calls it.

### 3.4 Existing area-like calculations

Other code computes area-like values for specialized purposes:
- Form::Fluid carries nominal_area, and capillary fluid boundaries validate enclosed area using Green's theorem.
- capillary_geometry.rs contains an area2 helper for its own fluid geometry validation.
- Other modules calculate polygon-derived quantities for inertia or connection-face validation.

These are specialized implementations, not a reusable, authoritative Phase 3 polygon area/union-area API. Fluid area semantics are out of scope for the strict polygon/line Phase 3 path.

## 4. Proposed API and ownership boundaries

Names below are conceptual proposals; final names should follow repository conventions after review.

### 4.1 Single-polygon area

Add a geometry-only operation for a validated supported polygon:
- Input: a finite, simple polygon represented by ordered vertices in one coordinate frame.
- Output: nonnegative PolygonArea in L², or failure for malformed/degenerate/unsupported input.
- Use a translated-origin triangulation/shoelace accumulation to reduce cancellation under large world translations.
- Determine winding separately; area magnitude must not depend on clockwise versus counter-clockwise ordering.
- Use scale-aware area uncertainty derived from the linear-coordinate uncertainty and characteristic polygon length/area. Do not introduce a unit-sized floor or treat tolerance as physical clearance.
- Validate self-intersections, repeated/zero-length edges, non-finite coordinates, and degeneracy before returning a measure.

The first implementation slice should support convex polygons (including rectangles and regular polygons) and explicitly fail closed for concave polygons until their validation and area contract are separately demonstrated. Concave simple polygons may be added later without changing the public measure semantics.

### 4.2 Polygon-union area

A body's material area is the area of the union of its realized polygon constituents, not the sum of each constituent's area. Overlaps and shared interfaces must not be counted twice.

Proposed reference algorithm:
1. Convert each supported convex polygon to world-space directed edges using the existing placement transform.
2. Validate the complete input set, unique constituent identity, finite transforms, and conservative bounds.
3. Build the planar edge arrangement. Split edges at every proper intersection, T-junction, shared-edge endpoint, and collinear-overlap endpoint.
4. Classify each open subsegment against the union on its two sides. Retain only true external boundary pieces; remove internal opposing shared edges and assign same-side coincident duplicates to one deterministic owner.
5. Build and validate closed oriented boundary loops. Outer loops and hole loops must have the correct opposite orientation. Any unresolved topology, ambiguous overlap, or open boundary fails closed.
6. Compute union area from the oriented boundary using a translated-origin Green/shoelace sum. The absolute area is not the sum of constituent areas or of absolute edge contributions.
7. Independently compare the resulting union boundary length with the existing polygonal perimeter result where both are supported.

The current perimeter routine is a candidate foundation for steps 1–4, not a proven implementation of all these steps. In particular, it currently requires convex polygons and does not return closed-loop topology. Before reuse, its tolerances, intersection completeness, duplicate ownership, and closure behavior must be audited and covered by Phase 3 tests.

A hole formed by a ring of material constituents is a useful topology test: its area must be excluded from material union area, while its inner boundary remains part of the union perimeter. The implementation must not silently fill holes or sum all loop magnitudes.

### 4.3 Exposed/visible boundary length

Add a checked Phase 3 measure over the output of the strict visibility/perimeter APIs:
- Sum hypot(end - start) for every accepted straight interval.
- Validate finite endpoints, positive segment length above the derived numerical uncertainty, finite unit outward normals, and consistent body/constituent ownership.
- Confirm intervals on the same physical boundary do not overlap or duplicate positive length.
- Preserve zero-measure event endpoints as partition metadata; they do not contribute length.
- Return L, not L².
- Do not merge distinct intervals across a visibility transition merely because they are close within an arbitrary fixed epsilon.

For the current Phase 3A.4 scope, curved intervals remain unsupported and must fail closed in the strict adapter.

### 4.4 Source-footprint area

Define a separate 2D region/measure interface for an in-plane distributed source:
- Input is the source's actual physical support region in world coordinates, represented by supported polygon geometry or a polygon union.
- Output is its validated SourceFootprintArea in L².
- The source footprint is independent of receiver geometry, visible boundary intervals, and camera/solver windows.
- For constant power density, total source power is density × footprint area.
- For spatially varying density, later work must integrate the density over the support region with an explicit numerical method and convergence/error report; multiplying an arbitrary representative density by area is not acceptable.
- No source is registered, activated, normalized per receiver, or connected to a field in Phase 3A.4.

This is a geometry-domain contract only. Source power, budget, temporal schedule, and energy transfer remain governed by later roadmap phases.

## 5. Acceptance and analytic test matrix

### 5.1 Single polygon area

- Axis-aligned rectangle: area equals width × height.
- Rotated rectangle: area unchanged.
- Regular polygon: compare against the analytic regular-polygon area.
- Explicit convex polygon: compare against a hand-computed triangulation/shoelace result.
- Clockwise and counter-clockwise vertex order: identical nonnegative area.
- Translation by large positive/negative offsets: area remains within the derived tolerance.
- Uniform scale by factors spanning the Phase 3 test range: area scales by the square of the factor.
- Reject zero-area, repeated-edge, self-intersecting, non-finite, and numerically unresolved polygons.

### 5.2 Polygon union area and perimeter

- Disjoint rectangles: union area is the sum; perimeter is the sum.
- Overlapping rectangles: union area excludes overlap exactly once; perimeter excludes internal edges.
- One rectangle fully contained in another: union area/perimeter equal the outer rectangle.
- Edge-touching rectangles: union area is additive; shared edge is absent from external perimeter.
- Partial shared-edge overlap: the common interval is counted zero times as external perimeter.
- Vertex-only touching: area is additive; no zero-length duplicate boundary is emitted.
- Same-side coincident duplicates: one deterministic owner; no doubled perimeter or area.
- Partially overlapping convex polygons with proper crossings: analytic union area and perimeter.
- A ring of rectangles/polygons: preserve the hole, subtract its area, and retain its inner boundary length.
- Reorder constituents and reverse polygon winding: same area, perimeter, and geometric boundary.
- Unsupported concave polygon or unresolved degeneracy: explicit fail-closed result, not a guessed area.
- Compare union boundary length with the independent checked interval-length sum.

### 5.3 Visibility interval length

- Unobstructed polygon edge: sum equals exact edge length.
- One and multiple blockers: sum equals the analytically visible measure.
- Tangent/endpoint events: endpoints remain exact; point events add no length.
- No overlapping intervals or duplicate positive-length contributions.
- Translation, rigid rotation, winding reversal, and scale changes preserve normalized results.
- Malformed intervals, non-finite endpoints, zero/uncertain lengths, and unsupported geometry fail closed.

### 5.4 Source-footprint area

- Rectangle support: area equals width × height.
- Disjoint polygon support pieces: area is additive.
- Overlapping support pieces: union area, not summed individual area.
- Translation and rotation preserve area; scaling by s scales area by s².
- A spatially varying density test is specified for a later source-integration milestone, not activated here.

## 6. Numerical policy

- Reuse the Phase 3 scale-aware coordinate and predicate policy where its assumptions apply; add a dimensionally appropriate area error bound rather than comparing area directly to a length tolerance.
- Translate coordinates to a local origin before cross-product accumulation.
- Use stable summation (for example, compensated accumulation) when many edge contributions are combined.
- Keep geometric uncertainty separate from physical separation. A tolerance must never erase a resolvable gap or merge distinct boundaries as a convenience.
- Require scale/translation/rotation invariance tests and compare against analytic results.
- Reject non-finite intermediate products/sums and overflow rather than returning a partial measure.
- Report numerical uncertainty in L² for area and L for length.
- Do not use the old fixed 1e-12 polygon-area floor as the Phase 3 area criterion.

## 7. Recommended implementation slices after proposal approval

1. **3A.4a — Single-polygon measure and validity contract.** Add the checked polygon area primitive, area-specific tolerance/error handling, and analytic scale/translation tests. Do not change unrelated legacy Form::is_valid() behavior as a side effect; explicitly route the strict Phase 3 adapter through the new validated polygon contract.
2. **3A.4b — Exposed interval length contract.** Add checked, deterministic interval measurement and overlap/ownership tests using Phase 3 visibility output.
3. **3A.4c — Polygon union topology and area.** Extend or replace the current convex polygon perimeter arrangement with a result that validates boundary loops, holes, deterministic ownership, union area, and union perimeter. Keep fail-closed behavior until all supported arrangements pass.
4. **3A.4d — Source-footprint geometry adapter.** Reuse the same validated polygon/union measures for a distinct 2D support region; no source/field coupling.

Each slice should remain geometry-only, focused, and independently testable. Do not stack parallel PRs for the same geometry work.

## 8. Exit gate

Phase 3A.4 is complete only when:
- L and L² measures have distinct checked APIs and explicit contracts.
- Single-polygon area is numerically stable and analytically validated.
- Polygon-union area and perimeter count overlapping material exactly once and validate outer/hole topology.
- Visible interval length is disjoint, deterministic, and independently measurable.
- Source-footprint area is a distinct 2D support measure.
- All supported analytic, degeneracy, scale, translation, rotation, winding, and ordering tests pass.
- Unsupported geometry and unresolved arrangements fail closed.
- No source injection, wave-field coupling, energy transfer, production behavior, chemistry, movement, or organism-specific behavior has changed.

**Current disposition:** Audit and proposal only. Implementation remains gated on explicit review and authorization.
