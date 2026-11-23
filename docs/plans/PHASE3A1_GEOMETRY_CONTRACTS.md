# Phase 3A.1 — Geometry Contracts and Focused Test Plan

**Status:** Planning artifact for review. No implementation is authorized by this document.

**Parent architecture:** [Resonance Field — Formal Implementation Roadmap](../design/resonance-field-implementation-roadmap.md)  
**Geometry foundation:** [Resonance Source-Geometry Foundation](../pr-plans/resonance-source-geometry.md)  
**Current-state audit:** [Resonance Field — Phase 0 Audit](../audits/resonance-field-phase-0-audit.md)

## 1. Purpose and boundary

Phase 3A.1 converts the Phase 3A geometry audit into explicit contracts that can guide later geometry implementation without prematurely choosing algorithms or changing physical behavior.

It defines:
- Geometry ownership and input/output expectations.
- Supported geometric primitives and arrangement cases.
- Boundary-union and visibility-partition semantics.
- Distinct geometric measures for area and length.
- Determinism, numerical robustness, and fail-closed behavior.
- A focused test matrix and implementation sequence for Phase 3A.

It does **not** implement geometry, select a field-coupling law, calibrate physical parameters, or connect the field to live simulation.

The geometry layer answers geometric questions. It does not decide how much energy a source emits, how much a receiver absorbs, or where energy is accounted.

## 2. Authority and non-goals

The Master Technical Specification v2.4 and the controlling resonance-field roadmap remain authoritative. This artifact is subordinate to both.

Phase 3A is limited to exact geometry foundations and geometry-query contracts. In particular, it must not:
- Change live movement, chemistry, world-tone, hotspot, or energy-ledger behavior.
- Debit or credit source, field, receiver, material, or heat energy.
- Introduce a propagation radius, amplitude cutoff, or receiver-specific renormalization.
- Choose propagation speed, source power, field normalization, absorption coefficients, or other physical calibration values.
- Treat organism or cell geometry as a special physical category.
- Replace exact supported geometry with centroids, bounding circles, rasterization, or tessellated curves.
- Claim that a perimeter helper is an area integral or that a nearest-point query is a complete extended-source integral.

Unsupported or ambiguous arrangements must be rejected explicitly. They must not silently fall back to approximate geometry.

## 3. Geometry ownership and coordinate contract

### 3.1 Ownership

The physical/material geometry subsystem remains the sole authority for realized shape, placement, orientation, constituent membership, and physical boundary identity.

The resonance geometry interface consumes an immutable, coherent geometry snapshot. It may derive boundary arrangements, exposed intervals, visibility partitions, and geometric measures from that snapshot. It must not mutate physical geometry or reconstruct it from material counts, centroids, bounding boxes, or visual/UI state.

A query must operate against one consistent snapshot/revision. Results must not combine boundary data from different geometry revisions.

### 3.2 Coordinates and transforms

- All geometric calculations use project-native world coordinates and world-distance units.
- Constituent-local geometry is transformed into world coordinates before global arrangement and visibility classification, or through a mathematically equivalent transform-aware implementation.
- Positions, tangents, and outward normals returned to callers must be expressed in the documented coordinate frame; the contract must identify that frame explicitly.
- Rigid translation and rotation of the complete scene must preserve geometric measures and visibility relationships, with returned coordinates/normals transformed accordingly.
- The geometry contract is independent of camera, viewport, map dimensions, numerical wave-grid resolution, and display clipping.
- No implicit periodic wrapping or world-edge clipping is applied by these geometry queries.

### 3.3 Boundary identity and orientation

Every returned boundary piece must have stable ownership/provenance sufficient to identify the contributing physical material and source primitive. A composite-union boundary piece must not be counted more than once.

Boundary orientation must be consistent and documented. For closed material regions, the chosen convention must make outward-normal direction unambiguous. For open line or fluid-boundary primitives, endpoint and side semantics must be explicit; an undefined normal must be represented as undefined rather than guessed.

Canonical ordering must be deterministic and independent of input enumeration order. Stable ordering is for reproducibility, not a physical property.

## 4. Primitive and arrangement contract

The implementation must state which arrangements are fully supported. “Primitive can be queried in isolation” does not imply “all unions and intersections involving that primitive are supported.”

### 4.1 Primitive families

The contract covers the realized geometry already present in the project:
- Straight segments and polygon edges.
- Circular boundaries and finite circular arcs.
- Fluid boundaries represented by their actual supported arc/chord or multi-contact forms.
- Line forms where their geometric meaning and supported arrangement are well-defined.

Curved geometry remains analytic. Tessellating an arc into line segments is not an acceptable substitute for exact intersection, splitting, visibility, or measure.

### 4.2 Composite boundary union

For a composite material, the exposed boundary is the geometric union boundary of its constituent shapes, not the concatenation of every constituent perimeter.

The union operation must:
1. Find all relevant intersections and contact events between supported primitives.
2. Split primitives at those events into non-overlapping candidate intervals.
3. Classify each interval as exposed, interior/covered, or ambiguous under the union.
4. Emit each exposed interval exactly once with provenance and consistent orientation.
5. Preserve exact line and arc geometry and exact supported measures.

Required arrangement cases include:
- Polygon/polygon crossings.
- Polygon edges that partially or fully overlap.
- Polygons sharing one or more edges.
- Polygon vertex-to-edge and vertex-to-vertex contacts.
- Circle/circle crossings, tangencies, coincidence, nesting, and concentric cases.
- Polygon/circle crossings, tangencies, and boundary contacts.
- Fluid arc/chord contacts, arc/arc contacts, and supported multi-contact configurations.
- Supported line intersections and contacts.

A case may be declared unsupported if its topology cannot yet be resolved reliably, but the result must be an explicit typed failure. It must not return a partial boundary that appears complete.

### 4.3 Degeneracy policy

The geometry layer must distinguish:
- Valid zero-dimensional contacts (for example, a tangency or shared vertex).
- Valid one-dimensional shared boundary intervals.
- Invalid input geometry (non-finite coordinates, malformed primitives, impossible radii/arc parameters).
- Numerically unresolved or unsupported arrangements.

A point contact must not create or remove a finite boundary interval. A shared interval must not be double-counted. Coincident geometry must be handled according to an explicit union rule or rejected as unsupported; arbitrary tie-breaking that changes exposed measure is prohibited.

Tolerance policy must be centralized and documented in world-coordinate terms, with dimensional consistency for length, area, and angular comparisons. Tolerances may classify numerical uncertainty; they must not erase physically meaningful features based on a guessed minimum size. If a classification cannot be made safely within the declared numerical policy, fail closed.

## 5. Visibility and occlusion contract

### 5.1 Visibility is a partition, not a sample estimate

Visibility queries must return a complete partition of the queried boundary into disjoint intervals classified as visible, occluded, or unresolved. The union of returned intervals must cover the requested boundary exactly within the documented numerical tolerance, without gaps or overlap.

For curved boundaries, partitions are returned as exact arc intervals. For straight boundaries, partitions are exact segment intervals. No fixed sample count or denser sampling may be presented as exact visibility integration.

### 5.2 Receiver-relative visibility

For receiver-relative queries:
- The receiver's own body is excluded as an occluder only for the boundary-origin condition explicitly defined by the query.
- Other physical bodies remain eligible occluders.
- Self-contact and adjacent constituent geometry must not accidentally hide the receiver's own exposed boundary.
- Occlusion must be evaluated against the complete relevant blocker geometry, not only blocker centroids or bounding shapes.
- Multiple blockers and overlapping shadows must produce one deterministic partition; a boundary interval is either visible or occluded, never counted multiple times.

### 5.3 Visibility transitions

The partition must split at every supported event where visibility classification changes, including:
- A ray becomes tangent to an occluder.
- A ray crosses an occluder endpoint or boundary vertex.
- The nearest relevant source-boundary point changes where that change affects the query.
- An occluder enters or leaves the line of sight along the queried boundary.
- Multiple blockers exchange which one first intersects the line of sight.

Transitions must be derived from exact supported geometric events, not from a fixed receiver-boundary sampling grid. If the exact event arrangement is unsupported or numerically ambiguous, the entire query fails closed rather than returning a sampled approximation.

### 5.4 Complete source-to-receiver geometry

For extended source and receiver shapes, a query must preserve the complete relevant source-boundary and receiver-boundary geometry. A nearest-visible-point query is a useful primitive but is not a complete source-to-receiver integral.

The geometry layer must expose enough information to partition the supported source/receiver boundary relationship at visibility transitions and to integrate a later, caller-defined geometric kernel over the complete supported domain. It must not choose that physical kernel or its energy law.

## 6. Distinct measure contracts

Geometric measures must be strongly distinguished by physical dimension and meaning.

| Measure | Contract | Valid use |
|---|---|---|
| Area | Two-dimensional measure of a physical region or source support | Distributed in-plane source power density; physical plane-area integration |
| Boundary length | One-dimensional measure of a physical line or boundary | Genuine line-emitter flux; exposed receiver-boundary geometry where the later coupling law calls for it |
| Arc length | Exact length of a circular-arc interval | A component of boundary length |
| Segment length | Exact length of a straight interval | A component of boundary length |

Rules:
- Area must never be inferred from perimeter length or constituent count.
- Boundary length must never be substituted for source support area.
- The sun's physical-plane interception uses area integration, not perimeter allocation.
- The hotspot's distributed in-plane support uses physical support-area integration, not boundary-length allocation.
- Boundary-length integration applies only to a source that is physically a line/boundary emitter, or to receiver coupling when the future physical law explicitly defines boundary reception.
- Geometry helpers may calculate measures and partitions, but must not multiply them by source power or create energy transfers.
- Subdivision of a geometric region or boundary into equivalent pieces must preserve the total measure and any later integral built from a consistent kernel.

The geometry API should make dimension mismatches difficult to express—for example, distinct types or clearly named result types for area, length, and parameter intervals. The exact Rust type design is intentionally left open for implementation review.

## 7. Integration-ready query contract

The geometry foundation should provide composable operations with explicit inputs and outputs. Exact names and Rust signatures are implementation decisions; the semantic contract is not.

| Operation | Required semantic result |
|---|---|
| Build realized composite geometry | Validated snapshot with stable constituent/primitive provenance |
| Resolve composite union boundary | Complete, non-duplicated exposed straight and curved intervals |
| Measure region | Exact supported area or explicit unsupported result |
| Measure boundary | Exact supported segment/arc length |
| Partition boundary visibility | Complete visible/occluded intervals plus exact supported transition events |
| Partition source/receiver visibility relationship | Complete supported boundary-pair domains needed by a caller-defined integration kernel |
| Transform geometry | Equivalent geometry in a specified frame, preserving provenance and exact primitive form |

Every operation must return either a complete valid result or a typed failure. Partial results must not be mistaken for complete physical geometry.

A later coupling layer may consume these operations, but must own the physical kernel, source budget, field transfer, and energy settlement. Geometry must not own or duplicate those authorities.

## 8. Determinism and numerical invariants

For identical geometry snapshots and query inputs:
- Results must be bitwise deterministic where the existing numeric representation permits it; otherwise the permitted deterministic tolerance must be stated.
- Input constituent order and primitive enumeration order must not alter exposed measure or visibility classification.
- Canonical result ordering must be stable.
- Repeated evaluation and replay must return equivalent results.
- Rigid translation/rotation must preserve measures and topology.
- Equivalent subdivision of straight segments, arcs, or regions must preserve the resulting union and measure.
- Shared boundaries must be counted once; hidden/internal boundaries must be counted zero times.
- No NaN, infinity, or invalid normal may silently propagate into a successful result.

Numerical tolerances must be explicit and dimensionally appropriate. Tests must include values near classification thresholds and verify stable fail-closed behavior. Tolerances must not be tuned separately per receiver to make totals agree.

## 9. Focused test matrix

Tests should be deterministic, small, and analytic wherever possible. Each arrangement test should assert both topology (which intervals exist) and measure (their exact or tolerance-bounded lengths/areas).

### 9.1 Composite union

| ID | Geometry case | Required assertion |
|---|---|---|
| U-01 | Two polygons crossing | Correct split points; exposed union boundary has no internal edges |
| U-02 | Partial shared polygon edge | Shared interval appears zero times in exposed boundary |
| U-03 | Fully shared polygon edge | No duplicate shared edge; correct outer union |
| U-04 | Polygon vertex touches polygon edge | Point contact does not create a gap or duplicate interval |
| U-05 | Two polygons share a vertex | Correct separate/connected topology under the defined union rule |
| U-06 | Intersecting circles | Exact arc splitting and correct exposed arcs |
| U-07 | Tangent circles | Tangency is a point event; no spurious finite interval |
| U-08 | Coincident circles | Explicitly correct union or typed unsupported result |
| U-09 | Nested/concentric circles | Inner covered boundary is excluded; concentric ambiguity handled explicitly |
| U-10 | Polygon/circle crossing and tangency | Exact line/arc intersections and correct exposure |
| U-11 | Fluid arc and chord | Arc/chord contacts and physical boundary ownership are correct |
| U-12 | Multi-contact fluid boundary | All supported contacts split and classify correctly |
| U-13 | Line intersections/contact | Supported line arrangements classify correctly; unsupported forms reject |
| U-14 | Mixed multi-constituent union | No duplicate or missing exposed interval across primitive families |

### 9.2 Visibility and transitions

| ID | Geometry case | Required assertion |
|---|---|---|
| V-01 | Fully visible boundary | One complete visible partition; no gaps |
| V-02 | Fully occluded boundary | Complete occluded partition |
| V-03 | Partially shadowed straight boundary | Exact transition endpoints and correct visible/occluded intervals |
| V-04 | Partially shadowed circular arc | Exact arc transition parameters; no tessellation |
| V-05 | Tangent visibility transition | Tangency classified consistently without an artificial interval |
| V-06 | Occluder endpoint/vertex transition | Exact split at the event |
| V-07 | Multiple blockers with overlapping shadows | Union of occlusion intervals; no duplicate accounting |
| V-08 | Blocker exchange along receiver boundary | All visibility changes represented |
| V-09 | Receiver self-contact/adjacent constituents | Own-boundary rule does not hide exposed receiver geometry |
| V-10 | Unsupported or ambiguous transition | Entire query returns typed failure, not sampled partial output |

### 9.3 Measure, invariance, and robustness

| ID | Geometry case | Required assertion |
|---|---|---|
| M-01 | Analytic polygon and circle areas | Area matches analytic reference |
| M-02 | Straight and circular-arc lengths | Exact supported length matches analytic reference |
| M-03 | Equivalent subdivision | Total area/length and union unchanged |
| M-04 | Rigid translation | Measures and topology unchanged |
| M-05 | Rigid rotation | Measures and topology unchanged; normals rotate correctly |
| M-06 | Input reordering | Canonical output and measures deterministic |
| M-07 | Repeated query/replay | Equivalent result on every run |
| M-08 | Near-tangent and near-coincident cases | Stable classification or explicit fail-closed result |
| M-09 | Zero-length/malformed/non-finite primitives | Typed validation failure |
| M-10 | Dimension mismatch | Area and length APIs cannot be silently interchanged |

### 9.4 Integration-contract tests (geometry only)

| ID | Geometry case | Required assertion |
|---|---|---|
| I-01 | Complete source geometry | No centroid/nearest-point substitution for the full extended-source domain |
| I-02 | Complete receiver geometry | Every supported exposed receiver interval is represented once |
| I-03 | Global source denominator | Visible subsets retain a source-global measure; no per-receiver renormalization helper is introduced |
| I-04 | Area vs boundary integration | Sun/hotspot support contracts use area; true line emitter contract uses length |
| I-05 | Subdivision invariance | Splitting a source/receiver primitive does not change the geometric integral domain |
| I-06 | Unsupported arrangement | Operation rejects atomically; no apparently complete partial domain escapes |

These tests validate geometric contracts only. They do not test or imply energy conservation, material absorption, source emission, or live simulation behavior.

## 10. Implementation sequence after this plan is reviewed

1. **3A.1 contract review:** Resolve naming, ownership, tolerance policy, supported arrangement declarations, and test expectations. No production code changes.
2. **3A.2 exact composite union:** Implement supported shared/coincident/contact arrangements incrementally, with focused tests after each primitive family.
3. **3A.3 exact visibility partitioning:** Derive supported visibility transition events and produce complete interval partitions; do not increase sample counts as a substitute.
4. **3A.4 complete source/receiver geometric domains:** Expose the full supported boundary-pair partitions required for later integration. Keep source-global measures and do not introduce receiver-specific normalization.
5. **3A.5 invariance and fail-closed audit:** Validate transformation, subdivision, ordering, degeneracy, and unsupported-case behavior.
6. **3A exit review:** Confirm every exit criterion before proposing Phase 3B.

Do not combine these steps into one large geometry rewrite. Each implementation increment should be independently testable and reviewable.

## 11. Phase 3A exit criteria

Phase 3A is complete only when all of the following are demonstrated:
- Every declared supported composite arrangement has exact union behavior and focused tests.
- Shared/coincident boundaries are counted once or explicitly rejected where unsupported.
- Visibility partitions cover complete supported boundaries with exact supported transition events.
- Source/receiver boundary-pair geometry is complete for all declared supported arrangements.
- Area and boundary-length contracts are distinct and dimensionally clear.
- No centroid, nearest-point, fixed-sampling, bounding-circle, raster, or tessellation approximation is used as a substitute for complete supported geometry.
- No receiver-specific source renormalization is possible through the geometry interface.
- Results are deterministic, transform-consistent, subdivision-invariant, and robust under input reordering.
- Unsupported or numerically unresolved arrangements fail closed without returning misleading partial geometry.
- Focused geometry tests and full required CI are green.

Passing Phase 3A does not authorize Phase 3B automatically. The 3A exit review must be presented separately; Phase 3B requires its own explicit gate decision.

## 12. Deliverable and approval record

This document is a planning artifact only. Its review may refine contracts and tests. Implementation must remain on hold until:
1. The user approves the final 3A.1 contract.
2. A scoped implementation proposal identifies the exact 3A.2 work and affected modules.
3. The implementation scope is explicitly authorized.

**Current implementation status:** No geometry code, solver code, source coupling, energy accounting, or live simulation behavior is changed by this plan.


## 13. Repository-grounded first implementation proposal (3A.2.1)

This proposal is based on a follow-up inspection of the current `main` implementation. It narrows the first code increment; it does not authorize that code change.

### Current implementation facts

- `src/geometry/source_exposure.rs` contains `GlobalBoundaryPiece`, `add_global_intersection_cuts`, and `disjoint_source_global_material_perimeter`.
- The current source-global perimeter routine already handles supported non-collinear segment intersections, segment/arc intersections, and intersecting non-coincident circular arcs. Existing tests cover disjoint polygon/circle parts, an overlapping rectangle/circle, intersecting circles, and fail-closed fluid input.
- Segment/segment collinear overlap currently returns `None` when the projected intervals overlap or touch. Coincident circular arcs also return `None`. Line and fluid forms currently return `None` from the source-global routine.
- Exposed candidate intervals are currently classified by testing their midpoint against other constituent forms through `organism_geometry::form_contains_point`. That containment helper treats points on a shape boundary as contained.
- Consequently, merely adding split parameters for collinear overlap is insufficient: a shared edge between adjacent shapes must disappear from the union boundary, while duplicate same-side coincident boundaries must be emitted once rather than suppressed twice. The union classifier must distinguish local material occupancy on the two sides of a candidate interval and resolve duplicate provenance deterministically.
- The same source-exposure module also contains other visibility and projection paths. In particular, the legacy parallel-lane path quantizes curved visibility events to 0.01 world units. That path must not be reused as proof of exact transition partitioning for Phase 3A.3.

### Proposed first code increment

**3A.2.1 — Exact straight/straight contact and shared-edge union.** Limit the first implementation to polygonal straight-boundary arrangements; do not add circle coincidence, fluid, or line-form union in this increment.

Scope:
1. Extend straight/straight arrangement handling to represent proper crossings, endpoint contacts, collinear point contacts, partial shared intervals, and fully coincident intervals without losing either primitive's parameterization.
2. Classify each resulting open interval against the composite union, not by boundary-inclusive midpoint containment alone. The result must distinguish exposed outer boundary, internal shared boundary, duplicate same-side boundary, and unresolved topology.
3. Emit each exposed geometric interval once, with deterministic provenance and consistent outward orientation. Internal shared intervals emit zero times; coincident same-side intervals emit once.
4. Keep the existing exact segment/arc and arc/arc paths unchanged except for regression coverage. Do not broaden their supported arrangements in this increment.
5. Preserve atomic fail-closed behavior: if a supported straight arrangement cannot be classified consistently, return failure for the complete source-global perimeter rather than a partial perimeter.

Explicit exclusions:
- No circular coincidence/tangency/nesting changes.
- No fluid arc/chord or multi-contact union.
- No line-form union.
- No receiver visibility-transition work.
- No area integration or physical source/receiver coupling.
- No live simulation, chemistry, movement, solver, or energy-ledger changes.
- No algorithmic performance optimization; retain a correctness-first reference implementation.

### Required tests for 3A.2.1

- Two adjacent rectangles sharing a complete edge: shared edge absent; outer perimeter correct.
- Two rectangles sharing a partial edge: only shared subinterval absent; all remaining outer intervals retained.
- Two rectangles touching at one collinear endpoint: point event does not remove a finite interval or create a gap.
- Two identical polygons with the same winding: coincident boundary emitted once, not zero or twice.
- Two identical polygons with reversed winding: same union and perimeter as the same-winding case.
- Multiple collinear segments with reversed endpoint ordering: same canonical result and measure.
- Input-part reordering: same union measure and canonical geometry.
- Existing crossing polygon, overlapping rectangle/circle, intersecting-circle, and disjoint-part tests remain green.
- Translation and rotation of each fixture preserve union measure and topology.

### Acceptance boundary

3A.2.1 is complete only when the straight/straight cases above pass analytic topology and perimeter assertions, existing source-global tests remain green, and unsupported non-straight arrangements still fail closed. This increment does not complete Phase 3A and does not authorize 3A.2.2 or Phase 3B.

**Authorization status:** Proposed scope only. No implementation has been started or authorized by this planning update.
