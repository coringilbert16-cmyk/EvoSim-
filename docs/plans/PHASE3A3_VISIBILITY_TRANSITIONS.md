# Phase 3A.3 — Exact Visibility-Transition Partitioning

**Status:** Implementation audit and acceptance contract. This document does not claim that the visibility solver has passed the Phase 3A.3 exit gate.

**Authority:** `docs/design/resonance-field-implementation-roadmap.md`, especially its geometry invariants and Phase 3 requirements.

## Goal

For a finite point source or receiver, partition each supported straight or circular material boundary at every geometric event where visibility or source-facing status can change. Classify the resulting open intervals using exact realized geometry. Return disjoint, deterministic visible intervals, or fail closed when the arrangement is unsupported or ambiguous.

This is geometry only. No source power, field coupling, receiver material response, energy allocation, heat, or production simulation behavior belongs in this milestone.

## Existing implementation discovered

The current `src/geometry/source_exposure.rs` already contains two separate partitioning paths:

- `visible_polygonal_intervals_for_targets` partitions straight polygonal receiver edges.
- `visible_circular_arc_intervals_for_targets` partitions circular and circular-fluid receiver arcs.

Both collect geometric critical points, convert them into cuts on the target boundary, sort/deduplicate cuts, and classify each open interval through `source_path_blocked_for_candidates`. The midpoint is a representative point for classifying an interval, not a substitute for discovering its endpoints. It is valid only if the critical-event set is complete.

The straight path currently gathers polygon vertices, line endpoints, circle tangencies, fluid arc endpoints, straight-boundary intersections, and circle/segment intersections. The arc path gathers polygon vertices, line endpoints, circular tangencies, arc endpoints, target/straight-boundary intersections, and supported circle/circle intersections.

Existing tests cover partial and full straight-edge shadowing, line/edge intersections, invalid geometry, concave-target rejection, and several curved-boundary projection/visibility foundations.

## Required event completeness

For a straight target interval, visibility can change only at:
1. Target endpoints.
2. A ray from the source through a blocker vertex or finite-line endpoint.
3. A ray tangent to a circular blocker.
4. An intersection of the target boundary with a blocker boundary.
5. A supported coincident/collinear overlap endpoint.
6. A source-facing transition where the target normal becomes tangent to the source direction, if the target is not a straight edge with constant normal.

For a circular target arc, visibility can change only at:
1. Target arc endpoints.
2. Rays through blocker vertices or finite-line endpoints.
3. Rays tangent to circular blockers.
4. Target/blocker straight-boundary intersections.
5. Target/blocker circular-boundary intersections, including tangencies.
6. Target-circle source tangencies, which delimit front-facing and back-facing portions.
7. Endpoints and intersections of compound fluid boundary pieces.

All event calculations must respect finite segment extents and finite arc sweeps. Full-circle and wrapped-arc parameterizations must not lose or duplicate intervals.

## Audit findings and risks to resolve

- The two target paths independently assemble event sets. Their event definitions and tolerances can drift; the implementation review must compare them against one shared event taxonomy.
- Candidate-body pruning uses angular bounds and AABB minimum/maximum distances. Prove that every rejection is conservative for all supported geometry, including source-inside-bounds and angular wrap cases. Otherwise remove the rejection or fail closed.
- Fixed absolute tolerances currently appear in event intersection, cut deduplication, and classification. Establish a documented scale-aware numerical policy; do not silently treat a tolerance as a physical gap or merge distinct events.
- Tangencies are zero-measure events but can change interval visibility. They must be retained as partition events even when they do not create a finite overlap.
- Coincident and collinear arrangements must either be handled by canonical interval ownership or rejected explicitly. They must never produce duplicate exposure.
- Compound fluid boundaries must include contact chords/solid-contact pieces as occluders even when those pieces are not exposed fluid receiver surfaces.
- The exact blocker predicate and event generator must agree on boundary-touch semantics. A ray that only touches a blocker at one point must have a defined, consistently tested treatment.
- Unsupported shapes, self-occluding targets, malformed bounds, non-finite values, and ambiguous arrangements must fail closed.

## Phase 3A.3 acceptance tests

The implementation must include analytic expected results for:

- One blocker creating two visible shoulders on a straight target edge.
- Two separated blockers creating three or more visible subintervals.
- A blocker tangent to a source ray, with the tangent retained as an exact cut.
- A finite line blocker endpoint producing the correct visibility transition.
- A circle blocker tangent to a straight target edge and a circle blocker crossing it twice.
- Circular target arcs partially and fully occluded by straight segments, finite lines, circles, and other supported arcs.
- External and internal tangency, coincident boundaries, and wrapped full-circle intervals.
- Compound fluid free arcs with contact chords/solid-contact boundaries participating correctly as occluders.
- Source-on-boundary and source-inside-target cases failing closed or following an explicitly documented non-reception rule.
- Determinism under constituent reordering and polygon winding reversal.
- Translation and rotation invariance.
- Conservative candidate pruning compared against an unpruned reference path.
- No overlapping or duplicate output intervals; summed interval measure matches analytic visible measure.

Tests must assert interval endpoints and interval counts where analytically available, not only total perimeter.

## Exit criteria

Phase 3A.3 is complete only when:
- Every supported visibility transition has a corresponding exact event generator.
- The event taxonomy is consistent across straight and circular targets.
- Candidate pruning is proven conservative against an unpruned reference.
- Interval classification uses exact geometry and no unpartitioned sampling.
- The acceptance matrix passes, including determinism and geometric invariance.
- Unsupported arrangements fail closed.
- No energy, source coupling, or production behavior has changed.

## Explicitly out of scope

- Physical area integration (Phase 3A.4).
- Source-to-field coupling (Phase 3B).
- Passive material reception and energy settlement (Phase 3C).
- End-to-end energy reconciliation (Phase 3D).
- Production migration, organism-specific behavior, and performance optimization.
