# EvoSim

EvoSim is an open-ended evolutionary organism simulation. The organism is built from physical material and physical bonds; the simulation should produce structure and behavior from general mechanisms rather than a fixed predator/prey or organism-size progression.

## Current construction state

The initial-organism constructor is currently in a **working-validation phase**.

The current implementation:

- creates a deterministic physical Carbon scaffold;
- creates each physical unit through the normal material/structure machinery;
- forms every permanent connection through the normal physical bond transaction;
- uses realized geometry and the realized bond graph for cavity analysis;
- requires a qualifying genome cavity;
- requires Water plus three additional acquirable resources;
- checks acquisition by actual physical placement inside an accessible region;
- performs no recursive body-plan search and has no arbitrary placement-attempt budget;
- currently realizes genesis construction material from the catalog rather than waiting on environmental inventory.

The current scaffold is **not the final constructor architecture**. It is a temporary deterministic construction baseline used to prove that the physical construction, bonding, cavity, and acquisition contracts can work quickly. The newly separated genome-phase entry point is an integration seam for replacing that baseline with the final free-form constructor; it does not make the blueprint authoritative topology acceptable as the final design.

The present scaffold is a fixed 54-unit Carbon geometry consisting of an inner ring, six radial supports, and an outer ring. Adjacent rigid units are sealed with distinct endpoint bonds at the two ends of each shared wall segment. This must not be confused with the intended final free-form constructor.

## Intended constructor architecture

The final constructor should be forward-only and locally constructive:

1. start from the available physical starting material;
2. form a qualifying genome cavity;
3. once the cavity qualifies as the genome, finish that construction phase;
4. continue building the rest of the organism using locally valid physical bonds;
5. require Water and at least three additional resources that the organism can physically acquire;
6. if a required construction resource is temporarily unavailable during ordinary construction, wait without blocking the simulation;
7. never solve the entire future organism as a global placement problem;
8. never backtrack already committed physical bonds.

A blueprint, if used later, is only a preference. It must never become an authoritative future topology or placement command.

## Immediate engineering plan

The current priority is to make the initial constructor produce a valid organism in a timespan measured in milliseconds.

Work proceeds in this order:

1. **Prove the current physical scaffold.**
   - Identify the exact failing physical contract.
   - Fix only genuine implementation defects.
   - Keep construction deterministic and bounded.
2. **Make cavity formation a real construction milestone.**
   - The construction runtime now has an explicit genome-phase entry point.
   - It checks the realized physical graph immediately after committed construction/closure bonds.
   - When the cavity qualifies, that phase returns immediately instead of treating the cavity as a post-build assertion.
   - The current baseline still uses the temporary blueprint/scaffold to reach that milestone; it does not yet provide the final free-form topology.
3. **Make bond-driven construction local and first-valid.**
   - Exact boundary geometry is considered before declared blueprint pose.
   - Face/edge alignment is preferred over arbitrary angular sampling.
   - Once a physically valid continuation is found, it is committed immediately; the constructor does not continue searching for a "best" placement.
   - No arbitrary angular sweep, candidate cap, timeout, or backtracking is used to control runtime.
   - The remaining endpoint-pair search is transitional machinery and is the next target for replacement with direct frontier-feature construction.
4. **Build the remainder locally.**
   - Replace the temporary fixed scaffold with the intended free-form constructive mechanism.
   - Select only from geometrically valid local continuations.
   - Commit bonds immediately; no global search or backtracking.
5. **Verify acquisition.**
   - Water plus any three additional resources must each be physically acquirable.
6. **Integrate waiting behavior where required.**
   - Resource shortage becomes a pending construction state rather than constructor failure or a simulation-thread block.
   - This is not yet part of the current catalog-backed genesis constructor.
7. **Only then broaden validation.**
   - Classify downstream failures by contract.
   - Migrate tests that still encode retired assumptions.
   - Run broader simulation/lifecycle validation after the construction layer is stable.

## Construction invariants

These are the constraints that matter to the current construction work:

- Physical geometry is authoritative.
- A permanent bond is created through the shared physical bond transaction.
- Intended bond contact is distinct from unintended penetration.
- Construction does not need a universal grid, 4N topology, or predefined cavity shape.
- The genome is defined by a qualifying realized cavity, not by a hard-coded core.
- Water is a physical material when instantiated; it is not logical material placed into storage merely to satisfy a test.
- Composite physical material retains its internal structure.
- Acquisition is based on physical contact/overlap, not exact coordinate identity.
- Constructor performance must come from direct construction and bounded local decisions, not from brute-force candidate enumeration.

## Verification discipline

For each change:

1. inspect the exact code path and contract being changed;
2. make the smallest isolated implementation change;
3. run formatting and compilation;
4. run focused constructor tests first;
5. inspect the actual failure before changing the next layer;
6. only broaden the test scope after the focused contract is proven.

Do not redesign the constructor in response to a downstream failure until the underlying construction contract has been verified.

## Project direction

The long-term goal remains an open-ended simulation in which organisms can develop structure, acquire resources, sense their environment, reproduce, form niches, and potentially evolve multicellular cooperation from the same general physical and behavioral mechanisms.

The current bottleneck is narrower: **constructing the first valid organism correctly and quickly.**


## Geometry Reference Library

The geometry catalogue is being separated from the live organism constructor into a **persistent geometry reference library**. This is durable knowledge, not test state and not a per-construction search cache.

The intended catalogue grows exhaustively from the smallest physical formations upward:

1. record every valid single-resource geometry;
2. enumerate every physically valid two-constituent combination, including repeated resources;
3. derive every valid three-constituent formation from the validated smaller formations;
4. continue the same physically reachable expansion through larger composites, eventually up to 20 constituent resources;
5. canonicalize equivalent formations so the library stores each physical geometry once;
6. preserve the library across tests, process restarts, and constructor runs.

The first implementation provides the persistent store, schema/versioning, canonical formation representation, independent geometry validation, durable JSONL storage, and base-resource seeding. The store lives outside target/ and therefore is not reset by normal Rust builds or test cleanup.

### Library rules

- The library is **knowledge**, not authority for the live organism state. A constructor may later use it as a read-only source of known-valid continuations, but committed simulation bonds still go through the normal physical transaction.
- Validation is independent of discovery. A formation is stored only after resource/shape validity, bond topology, connectivity, and non-penetration checks succeed.
- Global translation and rigid rotation are canonicalized. Reflections are **not** collapsed: a mirror image is a distinct physical formation unless the geometry itself makes it identical.
- Repeated constituents are allowed. “20 constituents” means twenty physical pieces; it does not mean twenty distinct resource types.
- Water remains a real resource. Fluid formations without a finite boundary are represented without inventing rigid collision geometry; their context-fitting realization remains a separate physical-field concern.
- Fluid-to-fluid contact is not a rigid bond. Combining fluid with the same fluid produces the same fluid shape with greater volume; the geometry library must not represent that operation as two bonded fluid constituents.
- Floating-point coordinates are represented in canonical signatures with a fixed geometric quantization tolerance. The geometry schema and resource-shape catalogue are versioned so stale geometry cannot silently become current knowledge.
- Tests use isolated temporary library roots. They must never mutate the persistent production catalogue.

### Worker direction

The next layer is the non-stop catalogue worker. It will load the persistent library, select an unexplored formation/frontier, generate exact geometry candidates, validate and canonicalize them, append only new formations, and durably record progress so it can resume after interruption. It will not use the constructor's old brute-force placement loop or an arbitrary attempt budget.

The worker now expands from validated formations rather than repeatedly solving each composite from scratch. It persists a frontier record for every formation/resource expansion, resumes unfinished work after restart, and idles rather than hot-spinning when no work is currently available. It can be started with `cargo run -- --geometry-worker`. Continuous contact families are no longer treated as a problem to solve by placement sampling: the first exact fluid model uses 2-D zero-gravity Young–Laplace capillary equilibrium. For Water against a rigid edge, conserved area and the effective wetting contact angle determine a constant-curvature circular arc analytically; the remaining translational freedom is an interval along the exposed edge. The worker currently records such cases as `ContinuousFamilyPending` until that symbolic contact-family representation is persisted and can participate in later composite expansion. This makes the geometry library the long-lived reference layer that the constructor can eventually query instead of rediscovering the same geometry during every organism construction.

### Current implementation milestone

**Implemented:** persistent library core, canonical formation schema, independent validation, durable append-only storage, version manifest, base-resource seeding, and focused persistence/canonicalization tests.

**Not yet implemented:** proof of exhaustive completion for continuous contact families, constructor integration, and the final completeness audit of symmetry reduction. The non-stop worker process and durable frontier/progress records are now implemented; its general expansion path can grow validated formations through 20 constituents. Exact rigid feature-contact generation is implemented for the current finite polygon/line feature model, but continuous contact families are not yet claimed exhaustive.


### Geometry library progress

The library now includes exact feature-contact generation for two-constituent rigid formations and expansion to three constituents. Two-body seeding considers every base resource against every single-resource formation, validates candidates, canonicalizes them, and persists only new formations. Three-body expansion reuses validated two-body contacts. Every declared rigid bond must correspond to actual physical boundary contact without positive-area/interior penetration. Fluid-to-fluid combinations are deliberately not represented as rigid bonds because they merge into the same fluid geometry with increased volume. This path does not use arbitrary angular sampling or an attempt budget. Continuous contact families are intentionally not yet claimed exhaustive; they need an explicit finite feature representation before completeness can be guaranteed.
