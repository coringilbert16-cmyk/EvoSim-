# Resonance Source-Geometry Foundation

## Purpose
Provide exact, reusable geometry primitives for treating a complete physical material (element or molecule) as a resonance source with a real composite boundary. This is a geometry workstream, separate from the propagation-energy integration and spatial neighbor-discovery optimization tracked in PR #377.

## Geometry ownership

This PR owns physical shape representation and geometric queries only:
- Composite material geometry built from realized physical constituents.
- Exact source-boundary and receiver-boundary primitives for supported straight and curved shapes.
- Visibility and occlusion across complete composite materials.
- Exact perimeter measures and linear allocation primitives.
- Deterministic results and fail-closed behavior for unsupported or ambiguous geometry.

It does not own live movement pair discovery, finite interaction support, propagation-energy debit, environmental heat settlement, or the independent resonance-field architecture. The field architecture is documented in [resonance-field-architecture.md](../design/resonance-field-architecture.md); this PR supplies the physical geometry interface that architecture can later consume.

## Implemented foundation

### Composite material perimeter queries
- OrganismBodyGeometry::nearest_visible_material_perimeter_point finds the closest visible point on a complete composite source material.
- Supported geometry includes circles, polygonal constituents, straight line segments, finite circular fluid arcs, multi-contact fluid boundaries, and legacy area-only fluid shapes.
- Polygon queries select actual boundary edges rather than radial approximations from constituent centers.
- Finite fluid segments include their physical contact chord; curved arcs remain exact and are not tessellated.
- Receiver coordinates are transformed into constituent-local frames; selected points and normals are returned in world coordinates.
- Third-party constituent geometry blocks hidden candidates. Equal-distance selection is deterministic.
- Invalid/empty geometry and receiver positions inside the source are rejected.

### Visibility and exposed source-boundary queries
- Receiver-relative visible source-perimeter queries return disjoint straight intervals and exact circular-arc intervals.
- Existing visibility solvers split and occlude boundary pieces against supplied bodies.
- Receiver bodies are excluded as blockers for rays that begin on their own boundary; third-party bodies remain occluders.
- Complete visible-boundary queries preserve all visible pieces rather than collapsing them to one nearest point.
- Source-global union geometry splits exact polygon edges and circular arcs at line/line, line/circle, and circle/circle intersections, then removes intervals covered by other constituents.
- Intersecting convex polygon and circle constituents are supported without tessellation; coincident/collinear boundary overlap and fluid/line union arrangements still fail closed.

### Perimeter measurement and allocation
- Exact straight-segment and circular-arc measures are available.
- Linear allocation primitives implement dE = E_source × dℓ / L_source.
- Allocation rejects invalid measures and supports proportional partitioning of a finite source budget.
- Receiver-visible subsets are measured against a source-global denominator; they must not be renormalized per receiver.

### Aura-reception integration
The geometry-aware reception path now enumerates the receiver's complete source-global perimeter rather than choosing receiver intervals from the source centroid. For each receiver-boundary sample it resolves the nearest visible point on the complete source perimeter and uses that point for distance and point-source projected extent. It retains the existing emitted spectrum, mass scaling, and continuous inverse-distance amplitude attenuation.

This integration is intentionally limited. Visibility is evaluated at each receiver-boundary sample; the boundary intervals are not yet split at every source-surface-dependent visibility transition, so partial occlusion within one sampled interval still needs exact treatment. The nearest-point query is not a continuous source-perimeter energy integral.

## Remaining geometry work
1. Complete the source-global union boundary for all supported realized constituents.
   - Current implementation handles intersecting convex polygons and circles with exact line/line, line/circle, and circle/circle splitting and midpoint interior classification.
   - Add exact handling for shared/coincident edges, touching polygon boundaries, fluid arcs/chords, multi-contact fluid boundaries, and line forms where their geometry is well-defined.
   - Preserve fail-closed behavior for ambiguous coincident or unsupported arrangements; do not substitute bounding circles or tessellation.

2. Complete perimeter-to-perimeter visibility.
   - Current receiver sampling enumerates the receiver's source-global perimeter and queries the nearest visible point on the source perimeter at each sample.
   - Split receiver intervals at exact visibility transitions as the nearest source point and third-party occlusion change along the receiver boundary.
   - Preserve third-party occlusion and exclude only the receiver's own body where rays originate on its boundary.
   - Avoid centroid-derived sample selection and avoid tessellating curved boundaries.

3. Complete supported shape coverage.
   - Integrate fluid and line boundary forms into the same exact boundary arrangement.
   - Preserve exact arc/chord and multi-contact representations.
   - Keep unsupported overlap/touch cases fail-closed until exact splitting and union classification are implemented.

4. Maintain geometry correctness and determinism.
   - Add tests for intersecting/touching polygons and circles, shared boundaries, fluid intersections, occlusion, rotated constituents, and degenerate geometry.
   - Verify exact perimeter measure, deterministic ordering, and no duplicate or missing boundary intervals.

## Explicit non-goals
- No spatial neighbor pruning or movement-pair optimization; that belongs to PR #377, which is already merged.
- No arbitrary interaction radius, physical effective-zero threshold, or attenuation-law change.
- No source-energy debit, propagation heat settlement, or new resonance-energy reservoir.
- No migration of the live movement-force authority.
- No independent resonance field/layer implementation in this PR; preserve the architecture contract in the existing design document.

## Validation
- Focused geometry, visibility, perimeter-measure, and aura-reception tests.
- Formatting and focused Clippy checks.
- Full repository architecture checks, offline analyzers, full Rust test suite, environmental chemistry smoke run, and full Clippy are required before merge.