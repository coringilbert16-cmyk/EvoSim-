# Performance research: particle chemistry and spatial simulation

**Purpose:** Record external implementations and research relevant to EvoSim's chemistry-throughput work. These sources inform optimization experiments; they do not override EvoSim's physical model, transaction rules, or validation requirements.

## Executive guidance for EvoSim

The strongest recurring pattern across particle simulators is to avoid repeatedly discovering every possible interacting pair. They use spatial bins, stencils, or bounding-volume hierarchies to produce a conservative set of nearby candidates, then evaluate the actual interaction using the current physical state.

For EvoSim, this is a *broad phase only*. A spatial index or cached neighborhood may reduce which pairs are considered, but it must never decide that a reaction is physically eligible. The live checks for geometry, contact, interface engagement, activation, reaction formation, energy settlement, and material ownership remain authoritative.

Recommended order:

1. Finish instrumentation and separate invariant chemistry data from geometry-dependent identity.
2. Profile candidate discovery, pair/contact generation, organism-owned BREAK restoration, accumulation/state management, chemistry-library access, and persistence independently.
3. Optimize only the measured dominant cost. If pair discovery dominates, prototype a conservative spatial candidate index and compare it against the existing path on identical seeded worlds.
4. If allocation/copying dominates, reduce temporary graph/material copies and improve data locality without changing ownership or transaction boundaries.
5. Consider incremental dependency updates only where dependencies are explicit and invalidation is complete.
6. Defer parallel reaction processing until candidate generation, ordering, conflicts, and transaction boundaries are deterministic and independently testable.

## Comparable simulators and what to learn

### LAMMPS — Verlet neighbor lists and spatial binning

- [Developer documentation: neighbor lists](https://docs.lammps.org/stable/Developer_par_neigh.html)
- [Neighbor command and bin / nsq / multi choices](https://docs.lammps.org/latest/neighbor.html)

LAMMPS uses spatial bins and precomputed bin stencils to build neighbor lists rather than repeatedly scanning every possible pair. A Verlet “skin” adds a conservative margin so a list can be reused across multiple steps; it is rebuilt when displacement reaches a configured threshold. Its documentation also describes contiguous/page-based neighbor storage and spatial sorting for cache locality.

**EvoSim relevance:** A conservative contact-candidate list may be useful if profiling shows repeated spatial discovery dominates. The skin-distance technique is only safe if EvoSim can prove that no pair can enter physical contact before the list is refreshed. Movement, geometry changes, topology changes, and reaction transformations must invalidate or conservatively refresh affected entries. The current live contact/geometry checks still run before any reaction commits.

LAMMPS also documents a useful algorithm-selection lesson: binned search is not automatically fastest for every workload. A simple quadratic scan can win for small, sparse systems; multi-scale bins can help when interaction cutoffs or object sizes vary substantially. Benchmark against EvoSim's actual object counts and size distribution before choosing a structure.

### HOOMD-blue — cell lists, stencils, and bounding-volume hierarchies

- [Neighbor-list algorithm documentation](https://hoomd-blue.readthedocs.io/en/v2.5.2/nlist.html)
- [Current stencil-list API](https://hoomd-blue.readthedocs.io/en/v6.1.0/hoomd/md/nlist/stencil.html)

HOOMD-blue describes three broad approaches: ordinary cell lists, stenciled cell lists, and linear bounding-volume hierarchies (LBVH). Cell lists work well when interaction ranges are similar. Large cutoff asymmetry can make one global cell size inefficient; stencils or trees can be better for strongly asymmetric or sparse distributions.

**EvoSim relevance:** This is a reason not to commit prematurely to one spatial-index design. First measure the distribution of constituent sizes, contact ranges, density, and local clustering. A grid may be sufficient; a hierarchical or tree-based structure is a later option only if the measured workload warrants its extra complexity.

### PyRID — reacting particles, heterogeneous sizes, and data-oriented storage

- [2025 paper: The Brownian dynamics simulator PyRID for reacting and interacting particles](https://doi.org/10.1016/j.crmeth.2025.101182)
- [Open-access full text](https://pmc.ncbi.nlm.nih.gov/articles/PMC12570352/)

PyRID is a recent particle reaction/diffusion simulator supporting bimolecular reactions, pair interactions, heterogeneous molecular assemblies, and mesh-based compartments. Its design and benchmarks discuss spatial acceleration for polydisperse systems and data-oriented particle storage.

**EvoSim relevance:** Treat hierarchical grids and packed/dynamic arrays as candidate techniques, not automatic upgrades. They are most relevant if EvoSim's profiling shows size disparity, memory layout, or allocation pressure is actually expensive. Do not copy shortcuts that defer geometry or collision work if they could miss a valid contact or change a reaction outcome.

### ReaDDy 2 — reaction dynamics and explicit simulation stages

- [ReaDDy 2 paper, PLOS Computational Biology](https://doi.org/10.1371/journal.pcbi.1006830)

ReaDDy 2 combines particle interactions with reaction and topology changes. Its documented loop separates integration, neighbor-list updates, reaction handling, topology reactions, force calculation, and observables. It also explicitly addresses conflicts when candidate reactions share reactant particles.

**EvoSim relevance:** The separation of stages is a useful architectural reference for profiling and testing. EvoSim should keep candidate discovery, live eligibility evaluation, conflict/fairness policy, and atomic transaction settlement distinguishable. Reactions that share material or structural participants cannot both commit if they conflict; optimization must preserve the established deterministic/fair scheduling contract.

### Smoldyn — tune spatial partitioning empirically

- [Smoldyn simulation parameters: partitioning space](https://www.smoldyn.org/about4.html)

Smoldyn partitions space into virtual boxes to reduce candidate molecule interactions. Its documentation recommends timing multiple box sizes and warns that boxes that are too small relative to reaction binding radii can cause missed bimolecular reactions.

**EvoSim relevance:** Grid size is a benchmark parameter, not a magic constant. Every tested configuration must prove that the broad phase is conservative and cannot omit a pair that could pass the exact live contact test.

### Reaction dependency graphs — incremental updates

- [Reaction factoring and bipartite update graphs for the Gillespie algorithm](https://pmc.ncbi.nlm.nih.gov/articles/PMC2798956/)

This work uses reaction/species dependency relationships to avoid recomputing propensities that cannot have changed after a reaction event.

**EvoSim relevance:** A dependency graph may help with invariant chemistry calculations or local state updates. EvoSim's geometry-sensitive physical reactions are not equivalent to a well-mixed Gillespie propensity model: movement and geometry can change contact eligibility independently of chemical composition. Incremental updates are appropriate only for explicitly static dependencies, with complete invalidation for every relevant state change.

## Measurement and profiling references

- [Criterion.rs user guide](https://criterion-rs.github.io/book/) — statistically informed Rust microbenchmarks and regression comparisons.
- [Criterion.rs profiling guide](https://bheisler.github.io/criterion.rs/book/user_guide/profiling.html) — profiling benchmark executables without including Criterion's analysis overhead.
- [cargo-flamegraph](https://github.com/flamegraph-rs/flamegraph) — CPU flamegraphs for Rust applications.

For EvoSim, microbenchmarks must complement—not replace—same-seed fixed-duration whole-simulation runs. Record ticks/sec, reaction counts, average and maximum per-stage time, candidate counts, cache hit/miss rates, allocation/copy costs where measurable, and exact material/energy reconciliation. Repeat runs on the exact PR head; do not claim a whole-simulation speedup from one noisy run or one faster substage.

## EvoSim-specific guardrails

These are mandatory constraints when applying any technique from the references above:

- A spatial index, neighbor list, or cache may only generate a conservative candidate superset. It is never authoritative for physical contact or reaction eligibility.
- Revalidate current geometry, contact, interface engagement, activation, and transaction preconditions immediately before commit.
- Cache only data whose dependencies are truly invariant. Static chemical potential may be keyed by canonical chemical identity and fixed equation inputs; geometry/interface identity must not unnecessarily fragment that cache. Conversely, live geometry must not be hidden inside a static cache.
- Invalidate or refresh derived spatial data after movement and every geometry, topology, or material transformation that can affect candidate membership.
- Preserve deterministic/fair scheduling, conflict handling, transaction atomicity, integer material inventory, and energy settlement. Do not trade correctness for stale-list reuse or deferred checks.
- Preserve persistence locking, duplicate detection, crash recovery, manifest consistency, and documented durability if storage is optimized.
- Prefer the simplest measured improvement. Do not introduce hierarchical grids, packed-array rewrites, incremental reaction graphs, or parallelism without evidence that the current implementation's corresponding cost is material.

## Relationship to the active optimization work

This research supports [PR #259: Chemistry optimization and Clippy cleanup](https://github.com/coringilbert16-cmyk/EvoSim-/pull/259) and [roadmap issue #258](https://github.com/coringilbert16-cmyk/EvoSim-/issues/258). The active baseline and measurements remain in [chemistry-optimization-baseline.md](chemistry-optimization-baseline.md). Update this note when an experiment is accepted or rejected, including the benchmark evidence and correctness tests that justify the decision.
