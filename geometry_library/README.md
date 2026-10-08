# EvoSim Geometry Reference Library

This directory is the persistent home for generated geometry knowledge.

- `data/` is runtime-generated persistent library state, not source code. The normal `library_sync` checkpoint process intentionally mirrors this generated state into Git history; the data should therefore not be edited as source files.
- The catalogue is versioned by its manifest and resource-shape signature.
- Tests must use isolated temporary roots.
- The worker will eventually populate this directory continuously as new valid formations are discovered.

The source-of-truth implementation is src/bob/library.rs.

The library uses a locked positional equivalence tolerance of **0.5 units**. If two otherwise-identical formations differ only by a positional displacement of **≤ 0.5 units**, Bob treats them as the same geometric record rather than storing another microscopic variation. This is a geometric knowledge equivalence rule, not permission for live construction to penetrate or skip physical validation.

### Locked base-resource geometry

- **Hydrogen:** rigid rectangle, **1.0 units long × 0.1 units thick**. Hydrogen has finite area and must never be treated as a zero-thickness line segment. Its primary structural topology remains exactly two end points, located at the centers of the two longitudinal end faces.
- The generic line-segment geometry vocabulary remains available for mathematical/rigid line shapes that are explicitly defined as lines; it does not describe Hydrogen.


The current rigid-boundary vocabulary is exact polygon edges and rigid line segments. Runtime rigid surface-contact candidate generation also derives boundary contacts from exact edge intersections and endpoint-to-edge projections; it does not sample angles. Water can form symbolic capillary contact families against either boundary type; no angular or positional sampling is used.


## Fluid geometry: capillary equilibrium

Fluid geometry is not exhaustively enumerated as sampled circle placements.

For the first exact fluid case, EvoSim uses the zero-gravity Young–Laplace model in 2-D. A constant-pressure, constant-surface-tension free interface has constant curvature, so the free boundary is a circular arc. A conserved fluid area and a solid/fluid contact angle therefore determine the equilibrium radius analytically.

The persistent library should eventually store **contact families and boundary constraints** for fluid formations rather than millions of sampled placements. A translation interval along a compatible rigid edge is a continuous degree of freedom and is represented symbolically. Edge intervals use the same normalized parameter `t ∈ [0, 1]` as the library's exposed-boundary intervals; physical edge lengths are converted to that parameter space before intersections are performed.

`src/capillary_geometry.rs` currently implements the exact flat-boundary equilibrium used by both polygon edges and rigid line segments:

- conserved 2-D fluid area;
- contact angle;
- constant-curvature free arc;
- exact enclosed-area solution;
- finite contact interval on a rigid edge;
- Young–Laplace pressure jump when surface tension is supplied.

The existing resource `cohesion` values are used only as a temporary deterministic **effective wetting adapter**. This is not treated as a complete thermodynamic Young-equation model; true interfacial energies can replace that adapter later without changing the capillary geometry equations.

Water-to-water remains volume accumulation rather than a rigid geometry bond.


### Symbolic fluid contact families

Water contact is now represented as a persistent `GeometryContactFamily`, not as a sampled placement. Before a family is recorded, the worker derives the exact exposed portions of each rigid polygon edge. Fully internal shared edges are removed, and partial collinear occlusion is represented by the remaining parameter intervals. Each family then records the rigid anchor, exposed boundary edge, equilibrium contact angle, free-interface curvature/radius, contact length, and the exact interval along that edge over which the equilibrium droplet can translate.

This means a continuous family is now finite data: the mathematics describes the continuum instead of the worker pretending it can enumerate it. The worker may mark a Water frontier exhausted once the corresponding family records have been persisted. Water-to-water remains volume accumulation rather than a rigid geometry bond. A Water contact family is therefore recorded as symbolic geometry knowledge rather than pretending the nominal circle is a rigid constituent.


### Durable worker writes

The worker batches each formation-expansion result before syncing it to disk. Candidate formations are canonicalized and deduplicated in memory, then appended with one durable sync and one manifest update. Capillary contact families use the same batch boundary. This keeps persistent durability from turning every discovered geometry into a separate filesystem sync operation while preserving restart-safe committed batches.


### Running the worker safely

Use `cargo run -- --geometry-worker-once` for a bounded smoke test. It opens the same persistent library, seeds missing base formations, processes exactly one unfinished formation/resource pass, durably records its results, prints the resulting formation count, and exits. Once that succeeds, `cargo run -- --geometry-worker` runs the same worker continuously and resumes from the persisted frontier after restart.

### Worker frontier semantics

A formation's seven resource frontiers are processed as one durable pass. Water is recorded as symbolic capillary families (or exhausted when no supported rigid boundary can accept the nominal water volume), while rigid candidates are expanded and batch-persisted. Restarted workers resume any frontier not marked `Exhausted`; an interrupted final JSON record is ignored as a truncated append tail rather than destroying the durable catalogue.

The continuous worker runs each session against freshly opened geometry and chemistry stores. If a session returns an I/O error, Bob logs it, waits five seconds, and reopens both stores before retrying; it does not keep operating against stale in-memory state after a reported storage failure. An interrupted frontier remains eligible for replay. This is recoverability, not a guarantee against persistent disk/schema errors: those remain visible in the log and must be resolved before unattended use.


The library's continuous-contact model is being expanded with exact symbolic rigid boundary families; no sampled geometry is used.

### Bob worker measurements

The worker reports per-frontier-pass measurements including constituent size, generated candidate count, newly persisted formations, candidate records not newly persisted, rigid edge/point/vertex family counts, Water/fluid-boundary family counts, total catalogue size, and elapsed time. These measurements are intended to establish the real growth curve before any decision is made about changing the current 20-constituent expansion target or expansion policy.
### Rigid continuous contact families

Rigid edge-to-edge contact is also continuous: after the boundary directions are aligned, one body can translate along the shared boundary while maintaining contact. The library now records this degree of freedom as a symbolic GeometryRigidContactFamily with exact edge identifiers, relative rotation, and the complete boundary-overlap parameter interval. A family can be instantiated at a chosen parameter without angular search; the normal formation validator remains authoritative for the resulting complete structure.

This is deliberately a continuum representation, not a claim that every parameter value is automatically valid inside an already-complex formation. The family describes the exact local contact manifold; instantiation performs the full collision/contact validation.

### Continuous point-contact families

A rigid line endpoint contacting an exposed polygon edge is a genuine continuum: the contact point may move anywhere along the exposed edge, while the line may rotate through the outward half-plane without entering the solid. The library records that two-parameter family symbolically (edge parameter plus orientation interval) rather than sampling angles or positions. Full formation validation remains authoritative when a member of the family is instantiated.

A rigid line can also accept the endpoint of another line anywhere along its exposed segment. That endpoint-to-interior family is recorded exactly with the exposed line parameter interval and the candidate's full `0..2π` orientation range; no angular sampling is used.


### Continuous polygon-vertex contact families

The catalogue also records the continuous rigid manifold where a vertex of a convex polygonal candidate touches an exposed edge of an existing polygonal formation. The contact point ranges over the exact exposed edge interval, while the candidate rotation ranges over the outward half-plane that keeps its interior outside the supporting solid. Concave candidates are deliberately excluded from this shortcut because their local admissible orientation set is not a single half-plane interval; they remain subject to the ordinary exact finite-contact generator until a dedicated concave contact representation exists. No sampled angles or positions are used.


### Worker catalogue-scan cost

The worker preserves breadth-first expansion without cloning and sorting the complete catalogue on every step. It selects the smallest unfinished formation by constituent count with a single scan, clones only that formation, processes all resource frontiers, and returns. This keeps catalogue growth from multiplying full-vector allocation and sort work at every worker iteration; a persistent work cursor can be added later if catalogue-scale profiling shows the remaining single scan is significant.


### First-class fluid boundary state

Water contact is now also persisted as a `GeometryFluidBoundaryFamily`. Unlike the older contact record, this record carries the conserved fluid area and the complete solved capillary state needed to reconstruct its current free-boundary family: contact angle, curvature radius, free-arc angle, contact length, and exact translation interval along the exposed rigid edge. This is still a one-rigid-wall equilibrium; it is deliberately not presented as a multi-wall solution yet. The important architectural step is that deformable fluid geometry is now durable state rather than only an ephemeral contact calculation, so later multi-wall/corner solutions can compose from the same conserved-fluid representation.


### Intrinsic rotational symmetry

Formation canonicalization removes exact proper rotations that leave an individual constituent's physical geometry unchanged. Regular polygons use their exact rotational symmetry; rectangles and line segments are twofold symmetric; circles have no meaningful local rotation. This is only rotational equivalence, never reflection equivalence, so mirror-image formations remain distinct. Hydrogen remains a finite-area 1.0 × 0.1 rectangle with exactly two line-like primary structural endpoints.
