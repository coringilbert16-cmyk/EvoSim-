# EvoSim

EvoSim is an open-ended evolutionary organism simulation. Organisms are built from physical material and physical bonds, and complexity should emerge from general physical and behavioral mechanisms rather than from a prescribed predator/prey model, fixed organism-size ladder, or hard-coded body plan.

## Construction goal

The immediate construction goal is:

> **Build a viable first organism from physical rules in the millisecond range.**

Millisecond genesis is the real performance target. A timeout, attempt budget, or one-minute ceiling is not an acceptable definition of success. If construction is slow, the architecture should be improved rather than increasing the number of placement attempts.

The first viable organism must have:

- a qualifying emergent genome cavity;
- Water;
- any three additional resource categories selected from the available catalog;
- physical ability to acquire each selected resource.

The constructor does **not** require:

- a fixed number of pieces;
- a fixed radius;
- a fixed Carbon ring;
- a spiral;
- a shell;
- a lattice;
- a prescribed silhouette;
- a particular resource trio;
- a reproduction-size requirement.

The organism may therefore be small, asymmetric, or geometrically irregular as long as it satisfies the physical viability contract.

---

# Current architecture

The constructor is forward-only and bond-driven.

A single mutable construction state owns:

- the realized organism structure;
- the construction frontier;
- endpoint occupancy;
- the spatial broad-phase index;
- cached rigid geometry;
- construction metadata;
- the resource/energy ledger.

A successful construction step must:

1. select a currently exposed construction frontier;
2. derive physically possible local candidates;
3. validate the selected geometry;
4. commit the new constituent;
5. resolve all exact physical contacts created by that placement;
6. form the resulting bonds through the shared physical bond transaction;
7. update the frontier, occupancy, spatial index, and other derived state;
8. continue from the same committed graph.

Committed bonds are permanent. Normal forward construction does not clone the organism graph, perform a global future-body search, or backtrack to repair an earlier decision.

Reproduction is different: once an organism reproduces, creating a separate organism is biologically meaningful and may use a separate construction state.

---

# Current geometry principle

The geometry of the realized shapes is the final physical authority.

The target construction pipeline is:

`live frontier → nearby geometry → configuration space → legal contact features → developmental ranking → exact placement validation → exact contact → bond transaction`

The constructor must not discover placements by trying an arbitrary angular grid.

Current resource shapes are convex, so the construction geometry should use algorithms specialized for convex polygons. General concave-polygon configuration-space machinery is unnecessary unless the physical shape model later becomes concave.

## Default construction orientation

Rigid construction has a natural preference for **face-to-face / flat-to-flat contact**.

This does **not** mean the constructor builds a circle, ring, lattice, or other predefined body plan.

For example, regular Carbon hexagons should naturally tend to stack flat-to-flat when that is a valid preferred contact. The resulting structure is whatever the local geometry produces. A cavity emerges when the accumulated physical boundary creates one with sufficient qualifying area.

All other orientations are consequences of the available geometry and physical contacts.

---

# Configuration-space / Minkowski plan

The target architecture is a feature-preserving convex configuration-space implementation. The migration has now begun: the constructor generates rigid placement candidates from convex NFP contact features when their feature representatives correspond to available construction endpoints. The NFP therefore supplies both legal contact geometry and, for supported feature pairs, the candidate placement itself. Older finite geometry-derived orientations and endpoint-derived placement remain only as fallbacks for contacts not yet represented by the NFP feature path. The NFP is not yet the sole source of all placement candidates.

For convex existing shape `A` and candidate shape `B`, translational configuration space is represented by the Minkowski construction:

`A ⊕ (-B)`

This is a placement-space representation, not a topology generator.

It answers the geometric question:

> Which relative translations put these two shapes into a physically meaningful contact relationship without penetration?

The configuration-space boundary must preserve feature provenance. Each boundary feature (segment or point, as appropriate) should retain the physical features that generated it, such as:

- edge ↔ edge;
- vertex ↔ edge;
- edge ↔ vertex;
- vertex ↔ vertex.

This allows construction preferences such as flat-to-flat contact to rank physically valid choices without hard-coding a body plan.

For convex polygons, use the linear-time edge-angle merge form of the Minkowski construction rather than generating many sampled placements.

## Configuration-space responsibilities

Configuration space should:

- generate physically possible local placement/contact loci;
- preserve contact-feature provenance;
- avoid arbitrary angular sampling;
- avoid global organism search;
- remain independent of organism topology.

Configuration space should **not**:

- prescribe a Carbon ring;
- prescribe a cavity shape;
- choose a fixed number of pieces;
- force blueprint edges;
- decide the organism's topology;
- replace exact physical validation.

---

# Contact and bonding

There is one unified physical bond path.

The intended sequence is:

`configuration-space feature → world placement → exact geometry validation → exact contact → COMBINE/bond transaction`

The contact system owns exact contact geometry. The bond system consumes the resolved contact rather than rediscovering it.

Supported physical contacts are ordinary bonds, including:

- edge-to-edge;
- vertex-to-edge;
- edge-to-vertex;
- vertex-to-vertex;
- other exact boundary contacts supported by the geometry model.

There is no separate "forward bond" versus "closure bond" architecture.

A newly placed constituent must connect to the developing organism. Connectivity is a consequence of the construction frontier, not a special closure mechanism.

No duplicate physical connection-point bond may be created.

---

# Broad-phase and exact geometry

The spatial index is a derived acceleration structure, not a second physical authority.

The intended sequence is:

1. use the spatial index to find nearby realized geometry;
2. perform configuration-space reasoning only for relevant nearby shapes;
3. choose a legal contact;
4. perform exact narrow-phase validation;
5. commit the physical result.

The current `ConstructionSpatialIndex` and endpoint occupancy mechanisms are retained because they eliminate unnecessary whole-structure scans.

They are derived accelerations that already support the current constructor and are intended to remain under the final configuration-space architecture. They must not become a reason to preserve the old placement-search model.

Exact geometry remains authoritative at every stage.

SAT is the initial exact convex narrow-phase validator. GJK may be evaluated later if it provides a measurable advantage, but it should not be introduced merely for theoretical elegance.

---

# Developmental preferences

Inherited/developmental information may influence which physically valid construction choice is preferred.

It may rank:

- material similarity;
- spatial preference;
- contact preference;
- other evolved construction tendencies.

It may **not** prescribe the final topology.

The minimum construction-material similarity threshold remains:

`MIN_CONSTRUCTION_MATERIAL_MATCH = 0.60`

Developmental preference must therefore follow this rule:

> **Preference ranks valid geometry; geometry decides validity.**

The constructor must never:

- require future blueprint edges to be realized;
- force a predefined body plan;
- force a fixed cavity shape;
- force a fixed number of constituents;
- perform global placement search;
- backtrack committed bonds.

This separation is essential for evolution. The genome can evolve construction tendencies while the resulting organism remains a physical consequence of those tendencies interacting with geometry and resources.

---

# Genome formation

The genome is an **emergent physical cavity**.

The constructor does not begin with a predefined genome core. A cavity qualifies as the genome only when the realized physical geometry and bond graph satisfy the cavity qualification rules.

The temporary three-Carbon measurement scaffold remains a development/measurement authority for the minimum genome scale. It is not a permanent runtime body plan.

The scaffold consists of the minimum reference arrangement needed to establish the qualifying cavity area. Once the cavity is realized, it does not impose the final number, material, topology, or shape of the organism.

The constructor should stop the first construction phase at the milestone:

`cavity qualifies as genome`

It then continues forward only as far as needed to establish initial viability.

The genome is therefore capable of evolving with the organism's realized structure rather than being permanently tied to a fixed Carbon core.

---

# Incremental cavity detection

Cavity detection must not become a global per-step performance cost.

The intended optimization is:

1. if the current construction cannot yet contain a cycle, do not perform full cavity analysis;
2. when a new bond creates a possible enclosed region, inspect the affected local region;
3. run exact cavity qualification only when a candidate cavity exists;
4. once a qualifying cavity is found, record the genome milestone and continue normal construction.

The cavity analyzer remains the authority for whether an empty region qualifies.

The constructor must not fake a genome merely because a target number of pieces has been reached.

---

# Initial viability and acquisition

After the genome qualifies, construction continues until the initial viability contract is satisfied.

The required resources are:

- Water;
- any three additional resource categories;
- each selected resource must be physically acquirable.

The three non-water resources are not predetermined.

Acquisition is binary per selected resource: either the realized organism can physically acquire that resource or it cannot.

The constructor should use the already-realized physical geometry to evaluate acquisition rather than repeatedly rebuilding or globally searching candidate organism structures.

If material required by the construction process is temporarily unavailable, the intended future behavior is to wait and resume from the same committed graph when the material becomes available. Waiting is a construction state, not permission to backtrack.

---

# Material and water rules

Resource properties remain the physical basis of behavior:

- mass;
- potential energy;
- reactivity;
- cohesion.

`potential_energy` is the absolute maximum for a resource type, not mutable energy content.

Resource IDs remain:

| Resource | ID |
|---|---:|
| Water | 0 |
| Nitrogen | 1 |
| Phosphorus | 2 |
| Carbon | 3 |
| Sulfur | 4 |
| Hydrogen | 5 |
| Methane | 6 |

Current physical resource shapes are convex. Phosphorus is a convex trapezoid.

Water is logically ubiquitous/unlimited but is physically instantiated only when a physical representation is required. It is ordinary physical material when instantiated and is not logical material placed in storage merely to satisfy viability.

The genome cavity is genuinely empty and is part of the organism rather than the surrounding environment.

Structural membership follows bonded connectivity to the genome. Water may participate in structural/developmental tissue when physically connected according to the structural rules: a structural water path must contain a non-water physical connection to the genome-connected structure; water does not become structural merely by forming a water-only chain.

Permeability and related physical behavior should emerge from material properties and geometry rather than from special doors or resource-specific exceptions.

---

# Performance architecture

The constructor must be optimized around **bounded local geometry**, not larger search budgets.

The target hot path is approximately:

`frontier → nearby geometry → convex Minkowski/configuration space → legal contact feature → one/few exact validations → bond commit`

It must not evolve into a permanent architecture of:

`frontier → thousands of rotations → thousands of placements → whole-organism overlap scans`

The normal genesis path should not contain:

- whole-organism `structure.clone()`;
- arbitrary angular sampling;
- fixed placement-attempt budgets;
- repeated whole-structure scans;
- repeated reconstruction of immutable resource geometry;
- global cavity analysis after every placement;
- global acquisition searches after every placement.

Immutable resource geometry should be cached:

- vertices;
- edges;
- normals;
- edge angles;
- bounding radius;
- feature IDs.

Realized geometry should likewise be cached and updated incrementally where possible.

The spatial broad phase, endpoint occupancy, frontier, and geometry caches are all derived acceleration structures. None replaces exact physical geometry.

---

# Millisecond genesis implementation roadmap

This is the executable plan. The order matters.

## Phase 0 — Restore a clean baseline

- Fix the current `spatial_index` ownership/argument compile error in `grow_one_step`.
- Keep the live spatial index owned by the construction state and pass the same instance through the growth path.
- Clean incidental compiler warnings where safe.
- Run formatting and compilation.
- Run focused constructor/contact/cavity tests.

**Exit condition:** the current branch compiles and the existing architecture is measurable.

## Phase 1 — Build the convex geometry kernel

Create or restore `src/configuration_space.rs` as the dedicated configuration-space implementation.

The current historical configuration-space implementation is not automatically authoritative. Audit it before reuse.

Implement:

- canonical convex polygon representation;
- directed edges;
- edge angles;
- feature IDs;
- feature-preserving Minkowski construction.

**Exit condition:** unit tests prove correct configuration boundaries for the current convex resource shapes.

## Phase 2 — Preserve contact feature provenance

Every configuration-space boundary segment must identify the physical features that generated it.

Test:

- edge-edge;
- edge-vertex;
- vertex-edge;
- vertex-vertex;
- coincident/split boundary cases where applicable.

**Exit condition:** a configuration-space feature can be converted into the corresponding exact physical contact without rediscovering it through angular search.

## Phase 3 — Replace placement enumeration

Replace the initial-organism constructor's normal endpoint/rotation candidate search with configuration-space placement.

Retire from the genesis hot path:

- normal-derived rotation enumeration;
- corner-alignment rotation enumeration;
- line-endpoint rotation enumeration;
- arbitrary angle candidates;
- repeated trial placement generation.

**Exit condition:** viable local placements are obtained from geometric contact loci rather than from a list of sampled rotations.

## Phase 4 — Rank valid contacts

Add developmental/contact ranking over the legal configuration-space features.

Preferred flat-to-flat contact should emerge here.

Do not encode:

- Carbon rings;
- circles;
- axial lattices;
- fixed cavity silhouettes;
- fixed topology.

**Exit condition:** regular Carbon naturally prefers valid flat-to-flat stacking while other shapes remain free to use whatever contact geometry is valid.

## Phase 5 — Exact narrow-phase validation

Use SAT to verify the selected convex placement.

Then resolve the exact physical contact and send it through the shared bond transaction.

**Exit condition:** configuration-space selection and exact physical geometry agree on every focused construction case.

## Phase 6 — Incremental construction state

Ensure the normal genesis path is one mutable state containing:

- structure;
- frontier;
- endpoint occupancy;
- spatial index;
- cached geometry;
- construction metadata;
- ledger.

Remove any remaining whole-graph cloning from the normal one-part construction path.

**Exit condition:** each successful placement mutates the existing construction state exactly once.

## Phase 7 — Incremental cavity milestone

Avoid global cavity analysis until a cycle/enclosure is geometrically possible.

Detect and qualify only affected cavity regions.

**Exit condition:** genome qualification remains exact while no longer dominating every construction step.

## Phase 8 — Initial viability milestone

After genome qualification, continue only until:

- Water is acquirable;
- three additional resources are acquirable.

Cache/invalidate acquisition feasibility rather than repeating global searches.

**Exit condition:** the constructor can reliably produce a physically viable organism without a fixed piece count or body plan.

## Phase 9 — Remove obsolete constructor machinery

After the new path is proven:

- remove obsolete angular candidate generators from genesis;
- remove redundant contact rediscovery;
- remove old search-budget logic;
- remove dead fixed-seed assumptions from runtime construction;
- retain legacy machinery only where it is still required by independently valid developmental/reproduction systems.

**Exit condition:** the constructor has one authoritative physical placement path.

## Phase 10 — Benchmark the actual goal

Add focused benchmarks/diagnostics for:

- constructor wall time;
- units created;
- configuration-space calculations;
- exact narrow-phase checks;
- contact resolutions;
- bonds created;
- cavity analyses;
- acquisition checks.

The primary metric is:

> **time from constructor entry to a valid organism.**

The target is milliseconds, not an arbitrary attempt count.

## Phase 11 — Diversity and evolution audit

After performance is proven, vary:

- available materials;
- developmental preferences;
- resource availability;
- geometry;
- inherited construction tendencies.

Verify that:

- geometry remains final authority;
- developmental information changes preferences rather than prescribing topology;
- no fixed Carbon body plan reappears;
- the genome remains an emergent cavity;
- construction tendencies can therefore evolve.

---

# What is explicitly rejected

The following approaches are not acceptable final solutions:

### Arbitrary search budgets

Do not solve construction failures by changing:

`5,000 → 50,000 → 500,000 attempts`

A larger budget only delays failure.

### Angular sampling

Do not solve placement by trying:

- 16 directions;
- 32 directions;
- 360 one-degree rotations;
- or any similar sampled orientation grid.

Exact feature geometry should provide the meaningful candidates.

### Fixed Carbon genesis

Do not restore:

- a Carbon ring;
- a Carbon spiral;
- Carbon spokes;
- a fixed Carbon shell;
- a fixed Carbon lattice.

Carbon is one physical material among several.

### Fixed organism geometry

Do not prescribe:

- circles;
- perfect radial symmetry;
- a target radius;
- a fixed cavity boundary count;
- a fixed piece count.

### Global speculative construction

Do not clone the entire organism graph to explore hypothetical future bodies.

Normal construction is one forward mutable state.

### Over-specialized material rules

Do not introduce special constructor behavior merely because a material is Carbon, Water, Phosphorus, or another named resource.

Physical properties and geometry should carry the behavior.

### Premature GJK complexity

GJK is a possible future optimization/validator. It is not required before the convex Minkowski + exact SAT path is working and benchmarked.

---

# Verification discipline

Every architectural change must follow this order:

1. inspect the exact code path and contract being changed;
2. identify the smallest coherent implementation boundary;
3. make the change;
4. run formatting;
5. compile before broad testing;
6. run focused constructor/geometry/contact/cavity tests;
7. inspect actual failures;
8. only then broaden validation;
9. benchmark before declaring a performance improvement.

Do not reintroduce retired body-plan assumptions merely to make an old test pass.

When a legacy test conflicts with the current physical architecture, determine whether the test or the implementation is stale before changing either.

Never claim a CI or test suite is passing without inspecting the actual result.

---

# Current project status — 2026-10-06

## PR #175

PR #175 is the current exact-contact/construction migration branch:

`sprint/exact-contact-migration`

Current branch head after the README consistency audit:

`742cbc3f089474b5704a5073d89a8dd0dc0f80c1`

The branch has already implemented important foundations:

- forward-only bond-driven construction;
- local construction frontier;
- endpoint occupancy;
- construction spatial broad phase;
- incremental mutable one-part genesis work;
- exact physical contact direction;
- cached/hoisted geometry in the hot path;
- emergent cavity direction;
- README performance target.

The branch has **not** yet completed the final configuration-space migration. The configuration-space sections above describe the target architecture and implementation plan, not a claim that the migration is already present in the constructor.

## Current known blocker

The latest known CI compilation failed because the live `spatial_index` was not passed into `grow_one_step` even though the growth function uses it.

This is a mechanical baseline defect and remains the first implementation task before the geometry migration. The README update itself does not claim that this blocker has been fixed; a new CI result must be inspected before changing this status.

Do not interpret this compile failure as evidence that the architecture needs another search-budget increase.

## Current architectural gap

The constructor still contains the old conceptual hot path:

`frontier endpoint → candidate endpoint → rotation candidates → trial placement → overlap test → contact discovery → bond`

The next major implementation is to replace that with:

`frontier → nearby geometry → feature-preserving convex Minkowski configuration space → legal contact feature → developmental ranking → exact SAT validation → exact contact → bond`

That is the central remaining step toward millisecond genesis.

---

# Definition of done for the constructor

The constructor is not finished merely because it eventually produces an organism.

It is finished when all of the following are true:

- no fixed runtime seed/body plan is required;
- no fixed Carbon ring/spiral/lattice/shell is used;
- no arbitrary angular sampling is used in genesis;
- configuration space provides physically meaningful placement/contact candidates;
- contact feature provenance is preserved;
- flat-to-flat is a preference, not a topology rule;
- exact geometry remains final authority;
- committed bonds are permanent;
- normal genesis uses one mutable construction state;
- spatial broad phase and endpoint occupancy are incremental;
- cavity qualification is emergent and exact;
- the temporary genome measurement scaffold does not become runtime topology;
- Water plus any three acquirable non-water resources establishes initial viability;
- no reproduction-size requirement is imposed;
- no arbitrary placement-attempt budget is needed;
- viable genesis reaches the **millisecond range** on the intended local runtime;
- developmental preferences can influence construction without fixing the organism's topology;
- the resulting construction tendencies remain evolvable.

---

# Long-term simulation direction

The construction system is only the first layer.

The long-term EvoSim goal remains an open-ended evolutionary world in which organisms can:

- acquire and process physical resources;
- maintain energy and material state;
- sense environmental harmonics and resonance;
- move through physical space;
- develop memory and curiosity;
- reproduce;
- diverge through mutation;
- form ecological niches;
- potentially develop multicellular cooperation.

No hard-coded predator/prey/scavenger/social role system or large-organism progression should be required.

The central design principle remains:

> **Use the minimum information necessary. Let one physical mechanism have multiple consequences. Let evolution discover the rest.**
