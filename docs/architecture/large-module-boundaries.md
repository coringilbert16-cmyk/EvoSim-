# Large-module responsibility audit

Audit checkpoint: PR #203 head `455439db2cda9a22b6e969d78221b4cee0d67ddb` (2026-10-22). This is an organization audit only. No implementation code is moved or behavior changed by this document.

## Findings

Three implementation files are materially larger than the repository's current source-size guideline and contain multiple cohesive responsibility groups. File size alone does not justify extraction; boundaries must preserve ownership, visibility, test coverage, and runtime behavior.

### `src/geometry/source_exposure.rs` (~162 KB)

Observed responsibility groups:

1. **Projection primitives** — straight-edge and circular-arc projected extent, including point-source integration.
2. **Polygonal visibility** — angular bounds, candidate filtering, line intersections, and exposed straight-boundary intervals.
3. **Circular-arc visibility** — finite circle arcs, tangent/intersection events, and exposed arc intervals.
4. **Parallel-ray geometry** — `ParallelLanePartition`, `ParallelRayFrame`, critical-lane events, and hit-depth calculations.
5. **Parallel exposure and interaction paths** — ordered first-hit widths, receiver/occluder classification, and dense-world fallback behavior.
6. **Tests** — extensive colocated regression coverage for exact geometry, shadows, translation invariance, and fail-closed cases.

**Candidate future boundary:** projection math vs. visibility/occlusion vs. parallel-ray partitioning. Before extraction, trace every caller and shared private type; keep tests close to the implementation they validate. Do not change the geometric model or precision as part of a file move.

### `src/geometry/capillary_geometry.rs` (~103 KB)

Observed responsibility groups:

1. **Exact arc descriptor** — `CircularArcBoundary` validation, transforms, point/ray queries, and intersections.
2. **Fluid boundary topology** — `FluidBoundaryPiece`, `MultiContactFluidBoundary`, validity/containment, and mixed straight/arc boundaries.
3. **Coalescence and point mapping** — shared-interface detection, boundary coalescence, survivor mapping, and placed coalescence.
4. **Equilibrium family** — `CapillaryContactFamily`, contact-angle adaptation, area-conserving solutions, and translation intervals.
5. **Polygon-boundary coalescence** — validation and merging of shared polygon edges.
6. **Tests** — conservation, topology rejection, exact arcs, transformations, and coalescence.

**Candidate future boundary:** arc geometry, multi-piece boundary topology/coalescence, and equilibrium solving. The boundary representation and conserved area/contact contracts cross these groups, so split only after dependency tracing and a test-first extraction plan.

### `src/bob/library.rs` (~289 KB)

Observed responsibility groups:

1. **Persistence codecs and schema** — compact serialization, IDs, manifest/version handling, loading, and validation.
2. **Live contact resolution** — descriptors, query classification, and resolving persistent families against live interfaces.
3. **Family data models and signatures** — rigid, point, vertex, fluid, and multi-contact family types.
4. **Library query/index layer** — lookup, projections, index building, placement suggestions, and interface resolution.
5. **Mutation/persistence API** — inserting families/formations, frontier state, and writes.
6. **Geometry generation** — rigid and fluid family generation, candidate expansion, catalogue seeding, and default-root initialization.
7. **Validation/canonicalization** — formation normalization, equivalence, exposed intervals, and physical checks.
8. **Tests and benchmarks** — persistence, lookup correctness, initialization races, physical validation, and indexed throughput.

**Candidate future boundary:** persistence/codec, live query/index/resolution, generation, and validation/canonicalization. Preserve Bob's role as persistent geometry knowledge; live construction remains authoritative for physical validity. Do not split solely by line count if it creates circular dependencies between storage, family types, and validation.

## Safe extraction protocol

For each future split:

1. Audit imports, callers, visibility, tests, and module cycles at the current head.
2. Select one responsibility group and document the exact symbols that move.
3. Move code without semantic edits; retain or relocate its tests in the same change.
4. Run formatting, the architecture/source-size guards, focused tests, then the full suite and Clippy.
5. Compare failures by test identity against the accepted baseline; never count a baseline failure as a new regression.
6. Review the diff for accidental formatting or unrelated changes before beginning another extraction.

## Decision

Do not perform these splits as part of the current Clippy cleanup. This audit records candidate boundaries for a later, separately verifiable organization pass. The immediate low-risk work is documentation discoverability and targeted lint fixes.
