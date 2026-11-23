# Phase 3A.3 — Exact Polygonal Visibility-Transition Partitioning

**Status:** Implementation contract and strict Phase 3 geometry boundary. This document does not claim that the visibility solver has passed the Phase 3A.3 exit gate.

**Authority:** `docs/design/resonance-field-implementation-roadmap.md`, especially its geometry invariants and Phase 3 requirements.

## Phase 3 geometry model

Phase 3 uses **polygonal solid boundaries and finite straight-line blockers only**.

- Supported receiver boundaries: rectangles, regular polygons, and explicit polygons.
- Supported solid occluders: the same polygonal forms.
- Supported zero-area occluders: finite straight-line forms.
- Unsupported in every Phase 3 geometry snapshot: circles and all fluid forms, including legacy polygon-boundary fluids, circular-segment fluids, and multi-contact fluids.
- Unsupported forms fail closed. They are not converted to polygons, approximated, or allowed to contribute visibility events.

This restriction applies only to Phase 3 reference geometry. Existing circle/fluid behavior in unrelated collision, contact, containment, chemistry, rendering, and legacy perception paths is unchanged. Removing those representations from the wider simulation is a separate future project.

The strict adapter in `src/geometry/phase3_geometry.rs` enforces this boundary before calling a dedicated polygon/finite-line visibility solver. It uses the shared material-perimeter union helper only after validating that every part is polygonal. The original legacy visibility helper and circle-perimeter behavior remain available to non-Phase-3 callers; Phase 3 must use the strict adapter and must not call the legacy visibility helper directly.

## Goal

For a finite point source, partition each supported straight polygon receiver edge at every geometric event where visibility or source-facing status can change. Classify each resulting open interval against the supported realized polygon/line geometry. Return disjoint, deterministic visible intervals, or fail closed when the arrangement is unsupported or ambiguous.

Phase 3 material perimeter is polygonal only. A finite line can occlude but cannot be a receiver boundary or contribute material perimeter.

This is geometry only. No source power, field coupling, receiver material response, energy allocation, heat, or production simulation behavior belongs in this milestone.

## Phase 3 numerical policy

- `linear_tolerance` uses 16 machine-epsilon units times the larger of participating coordinate magnitude and characteristic feature length. Including coordinate magnitude accounts for precision lost when subtracting translated world coordinates; omitting a unit floor preserves scale invariance near the origin.
- `cross_tolerance` combines coordinate uncertainty times the sum of vector lengths with floating-point product error. It is used for parallelism and collinearity decisions.
- `parameter_tolerance` converts a world-space linear tolerance into a normalized segment parameter using the actual segment length. Event endpoint exclusion and cut deduplication therefore scale with the edge rather than using a fixed normalized epsilon.
- A tolerance represents numerical uncertainty only. It is not a physical clearance or an instruction to merge distinct geometry. Events closer than the derived uncertainty cannot be reliably distinguished in the represented coordinates; events farther apart remain separate.
- The straight polygon/line Phase 3 visibility and blocker paths use this policy. Legacy circle/fluid and curved-boundary paths remain outside the strict Phase 3 contract.

## Required event completeness

A source on or inside a solid target is not a receiver: the source-facing test excludes every outward boundary interval and the query returns `Some(empty)`. This is deliberate non-reception, distinct from malformed geometry, which returns `None`.

For a straight target edge, visibility can change only at:

1. Target edge endpoints.
2. A ray from the source through a polygon vertex.
3. A ray from the source through a finite-line endpoint.
4. An intersection of the target edge with another supported straight boundary.
5. A supported coincident/collinear overlap endpoint.
6. A source-facing transition. For a straight edge the outward normal is constant, so this is validated for the complete edge rather than approximated with extra curved-target cuts.

All event calculations must respect finite segment extents. Infinite extensions of finite-line blockers must never create false events or occlusion.

## Audit findings and risks to resolve

- Candidate-body pruning uses angular bounds and AABB minimum/maximum distances. Prove that every rejection is conservative for all supported polygon/line geometry, including source-inside-bounds and angular wrap cases. Otherwise remove the rejection or fail closed.
- The Phase 3 straight-polygon visibility path now uses the shared `phase3_numeric` policy: linear tolerances scale with coordinate magnitude and local feature size; cross-product tolerances account for both vector lengths; segment-parameter tolerances derive from linear tolerance divided by segment length. The polygon area calculation is translated to a local origin before accumulation. Candidate-pruning angular and distance margins use the same scale-aware linear bound. The policy deliberately has no unit-sized floor. The policy and scale/translation regression tests are in `src/geometry/phase3_numeric.rs` and `source_exposure.rs`. Curved/legacy-only paths retain their existing tolerances and are not part of the strict Phase 3 polygon/line boundary.
- Tangencies to polygon vertices and line endpoints are zero-measure events but can change interval visibility. Retain their exact rays as partition events.
- Coincident and collinear arrangements must either be handled by canonical interval ownership or rejected explicitly. They must never produce duplicate exposure.
- The exact blocker predicate and event generator must agree on boundary-touch semantics. A ray that only touches a blocker at one point must have a defined, consistently tested treatment.
- Unsupported shapes, including every circle and fluid representation, self-occluding targets, malformed bounds, non-finite values, and ambiguous arrangements must fail closed.
- The Phase 3 adapter must validate the complete snapshot, not merely receiver bodies, so unsupported geometry cannot enter as an occluder.

## Phase 3A.3 acceptance tests

The implementation must include analytic expected results for:

- One polygon blocker creating two visible shoulders on a straight target edge.
- Two separated polygon/line blockers creating three or more visible subintervals.
- A polygon vertex or finite-line endpoint producing the correct visibility transition.
- A tangent ray through a polygon vertex or finite-line endpoint retained as an exact cut.
- Straight-boundary intersections and collinear overlaps, including endpoint ownership.
- Source-on-boundary and source-inside-target cases returning no receiving intervals (explicit non-reception rule; not a geometry error).
- Determinism under constituent reordering and polygon winding reversal.
- Translation and rotation invariance.
- Conservative candidate pruning compared against an unpruned reference path.
- No overlapping or duplicate output intervals; summed interval measure matches analytic visible measure.
- Phase 3 adapters rejecting circles and fluids as receivers **and** as blockers, while accepting finite line blockers and polygon receivers.
- Phase 3 material-perimeter adapter returning straight polygon intervals only and rejecting circles/fluids.

Curved-boundary, circle-union, and fluid-boundary tests are not Phase 3 acceptance tests. Existing legacy tests for those unrelated systems may remain in their respective legacy paths; they do not authorize those forms in Phase 3.

Tests must assert interval endpoints and interval counts where analytically available, not only total perimeter.

## Exit criteria

Phase 3A.3 is complete only when:

- Every supported polygon/line visibility transition has a corresponding exact event generator.
- Candidate pruning is proven conservative against an unpruned reference.
- Interval classification uses exact geometry and no unpartitioned sampling.
- The acceptance matrix passes, including determinism and geometric invariance.
- The strict Phase 3 adapter rejects every circle/fluid form anywhere in the geometry snapshot.
- Unsupported arrangements fail closed.
- No energy, source coupling, or production behavior has changed.

## Explicitly out of scope

- Any circle or fluid geometry calculation in Phase 3.
- Removing circle/fluid support from unrelated simulation systems.
- Physical area integration (Phase 3A.4).
- Source-to-field coupling (Phase 3B).
- Passive material reception and energy settlement (Phase 3C).
- End-to-end energy reconciliation (Phase 3D).
- Production migration, organism-specific behavior, and performance optimization.
