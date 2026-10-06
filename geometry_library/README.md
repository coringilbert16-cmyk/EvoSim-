# EvoSim Geometry Reference Library

This directory is the persistent home for generated geometry knowledge.

- data/ is runtime-generated and should not be committed as source code.
- The catalogue is versioned by its manifest and resource-shape signature.
- Tests must use isolated temporary roots.
- The worker will eventually populate this directory continuously as new valid formations are discovered.

The source-of-truth implementation is src/geometry_reference_library.rs.


## Fluid geometry: capillary equilibrium

Fluid geometry is not exhaustively enumerated as sampled circle placements.

For the first exact fluid case, EvoSim uses the zero-gravity Young–Laplace model in 2-D. A constant-pressure, constant-surface-tension free interface has constant curvature, so the free boundary is a circular arc. A conserved fluid area and a solid/fluid contact angle therefore determine the equilibrium radius analytically.

The persistent library should eventually store **contact families and boundary constraints** for fluid formations rather than millions of sampled placements. A translation interval along a compatible rigid edge is a continuous degree of freedom and is represented symbolically.

`src/capillary_geometry.rs` currently implements the exact flat-boundary case:

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

This means a continuous family is now finite data: the mathematics describes the continuum instead of the worker pretending it can enumerate it. The worker may mark a Water frontier exhausted once the corresponding family records have been persisted. More complex cases where an existing deformed water region must itself be used as a boundary remain deferred until fluid boundary features can be composed symbolically.
