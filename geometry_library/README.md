# EvoSim Geometry Reference Library

This directory is the persistent home for generated geometry knowledge.

- data/ is runtime-generated and should not be committed as source code.
- The catalogue is versioned by its manifest and resource-shape signature.
- Tests must use isolated temporary roots.
- The worker will eventually populate this directory continuously as new valid formations are discovered.

The source-of-truth implementation is src/geometry_reference_library.rs.
