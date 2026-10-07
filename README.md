# EvoSim

EvoSim is an open-ended evolutionary organism simulation. Organisms are built from physical material and physical bonds; structure and behavior should emerge from general mechanisms rather than fixed predator/prey roles or a predefined organism-size progression.

## Current construction state

The initial-organism constructor is in a **transitional validation/migration phase**.

The current implementation:

- creates a deterministic physical Carbon scaffold;
- creates each physical unit through the normal material/structure machinery;
- forms permanent connections through the normal physical bond transaction;
- uses realized geometry and the realized bond graph for cavity analysis;
- requires a qualifying genome cavity;
- requires Water plus three additional acquirable resources;
- checks acquisition through physical placement in an accessible region;
- does not use an arbitrary placement-attempt budget;
- currently realizes genesis construction material from the catalog rather than waiting on environmental inventory.

The current scaffold is **not the final constructor architecture**. It is a temporary deterministic baseline retained while the final free-form constructor is integrated and verified. The genome-phase entry point is an integration seam for that replacement; it does not make a blueprint an authoritative topology.

The present scaffold is a fixed 54-unit Carbon geometry consisting of an inner ring, six radial supports, and an outer ring. Adjacent rigid units are sealed with distinct endpoint bonds at the two ends of each shared wall segment. This scaffold is a validation baseline, not the intended final organism-construction algorithm.

## Intended constructor architecture

The final constructor is intended to be forward-only and locally constructive:

1. start from available physical starting material;
2. form a qualifying genome cavity;
3. once the cavity qualifies as the genome, finish that construction phase;
4. continue building the rest of the organism using locally valid physical bonds;
5. require Water and at least three additional resources that the organism can physically acquire;
6. when ordinary construction temporarily lacks a required physical resource or a currently valid local continuation, wait rather than blocking the simulation or declaring permanent failure;
7. never solve the entire future organism as a global placement problem;
8. never backtrack already committed physical bonds.

A blueprint, if used, is only a preference. It must never become an authoritative future topology or placement command.

## Growth, development, and reproduction

EvoSim uses continuous physical growth rather than biological Juvenile, Adult, or Offspring lifecycle stages. There is no age-based maturation, adult-mass maturity gate, juvenile viability gate, or 40% reproductive threshold.

Growth is continuous and is derived from the organism's realized physical structure against its inherited developmental preferences. The genome's size preference produces a **preferred mass**; it is a soft developmental preference, not a maturity authority and not a requirement that the organism reach an exact mass.

A qualifying developmental realization above **0.90** makes budding available. This is an availability condition, not a lifecycle stage. Reproductive construction proceeds as a separate physical developing organism graph attached to the parent by physical contact. The developing graph has its own genome, structure, stress, energy, and material accounting. Its inherited energy allocation is represented by `reproductive_energy_allocation`.

The developing physical graph is retained because it is required to model actual budding and detachment. It is not a biological "offspring stage" label. When the physical relationship joining the developing graph to the parent is severed, graph separation can yield two independent organism graphs.

The intended reproduction model is **80/80**: the reproducing organism grows to approximately 160% of its preferred scale and physically separates into two approximately 80% organisms. The lifecycle gating and retired 40% rule have been removed, but the complete 160%-then-split physical implementation is **not yet complete**. The current developing construction uses an 80% preferred-scale readiness condition; that condition must not be confused with the finished 160%-then-split model.

Construction waiting is a physical construction state, not a lifecycle state. The construction runtime supports waiting when required physical material is unavailable or a valid local continuation cannot currently be made; final integration of that behavior into the catalog-backed genesis constructor remains unfinished.

### Migration status

The following retired concepts have been removed from the live organism state:

- `DevelopmentStage` and its Juvenile/Adult/Offspring variants;
- age-based maturation;
- adult-stage reproduction gating;
- the old 40% developmental/reproduction gate;
- the separate `juvenile_reserve` material concept;
- the `adult_mass()` API, replaced by `preferred_mass()`.

The serialized field name `juvenile_energy_reserve` remains only as a backward-compatible alias for the renamed `reproductive_energy_allocation` field. It is not a lifecycle reserve.

The physical construction/calibration module is still named `juvenile.rs`, but its retained seed machinery is a construction-calibration artifact rather than a biological juvenile stage. Physical viability checks live in `organism_viability.rs` and describe whole-organism physical viability.

## Immediate engineering plan

The immediate priority is to replace the temporary genesis scaffold with the final local, free-form constructor while preserving the physical contracts already established.

1. **Verify the current physical baseline.**
   - Keep the deterministic scaffold only as a validation baseline.
   - Fix genuine physical-contract defects rather than redesigning around downstream symptoms.
2. **Make cavity formation a real construction milestone.**
   - The construction runtime has an explicit genome-phase entry point.
   - It checks the realized physical graph after committed construction/closure bonds.
   - When the cavity qualifies, that phase returns immediately rather than treating the cavity as a post-build assertion.
   - The current baseline still uses the temporary scaffold to reach that milestone; it does not yet provide the final free-form topology.
3. **Replace transitional endpoint search with direct local construction.**
   - Exact boundary geometry is authoritative for physical placement.
   - Face/edge/surface contact is preferred over arbitrary angular sampling.
   - Once a physically valid continuation is selected, it is committed immediately; the constructor does not continue searching for a globally "best" placement.
   - No arbitrary angular sweep, candidate cap, timeout, or backtracking should be introduced as a runtime-control mechanism.
   - The remaining endpoint-pair search is transitional machinery to be replaced by direct frontier-feature construction as the geometry reference library becomes usable by the constructor.
4. **Build the remainder locally.**
   - Replace the temporary fixed scaffold with the intended free-form constructive mechanism.
   - Select only from geometrically valid local continuations.
   - Commit bonds immediately; no global search or backtracking.
5. **Verify acquisition.**
   - Water plus any three additional resources must each be physically acquirable.
6. **Integrate waiting behavior into final genesis construction.**
   - Resource shortage becomes a pending construction state rather than constructor failure or a simulation-thread block.
   - The current catalog-backed genesis path does not yet provide this final behavior.
7. **Finish the physical 80/80 reproduction migration.**
   - Implement the full approximately 160% growth condition.
   - Physically split the parent/developing graph into two approximately 80% organism graphs.
   - Preserve physical material, energy, genome, and structural validity through detachment.
8. **Keep documentation and compatibility cleanup aligned with the current physical model.**
   - Remove remaining retired lifecycle terminology from current documentation and tests.
   - Retain historical material only where it is explicitly identified as historical.
   - Rename `juvenile.rs` only when doing so no longer obscures active constructor work.

## Construction invariants

These are the constraints that matter to current and final construction work:

- Physical geometry is authoritative.
- A permanent bond is created through the shared physical bond transaction.
- Intended bond contact is distinct from unintended penetration.
- Construction does not require a universal grid, 4N topology, or predefined cavity shape.
- The genome is defined by a qualifying realized cavity, not by a hard-coded core.
- Water is a physical material when instantiated; it is not logical material placed into storage merely to satisfy a test.
- Composite physical material retains its internal structure.
- Acquisition is based on physical contact/overlap, not exact coordinate identity.
- Construction decisions are local and forward-only; performance should come from direct geometric construction rather than global brute-force enumeration.

## Verification discipline

For each change:

1. inspect the exact code path and contract being changed;
2. make the smallest isolated implementation change;
3. run formatting and compilation;
4. run focused tests for the changed contract first;
5. inspect the actual failure before changing the next layer;
6. broaden the test scope only after the focused contract is proven.

Do not redesign the constructor in response to a downstream failure until the underlying construction contract has been verified.

## Experimental chemistry

This section is **experimental planning only**. No implementation work is committed by this section. Chemistry work is intentionally deferred until Bob/the construction system and the Geometry Reference Library are substantially established.

The working direction is to make **BREAK and COMBINE consequences of formation chemistry rather than organism actions**. An organism should not need a special Break or Combine command, nor should the initial cell be designed with organelles whose purpose is to perform those operations. Structural changes should arise from the same general physical/chemical transition system that governs all material interactions.

### Foundational chemical inputs

The proposed interaction system derives local chemical behavior from four foundational inputs:

1. **Composition** — constituent identities and quantities;
2. **Bonded structure/topology** — which constituents are connected and how the formation is structurally arranged;
3. **Exposed geometry** — the actual exposed surfaces/features and their relative physical contact configuration;
4. **Bond strength / formation energy** — the energetic strength of the existing and newly formed bonds.

These are intended to describe a formation without introducing hard-coded properties such as toxicity, acidity, or food value. Such behaviors should emerge from the transitions that formations permit and the consequences of those transitions.

The desired universal transition model is approximately:

    interaction of formations
        -> evaluate composition, topology, exposed geometry, and bond strength/energy
        -> determine whether a structural transition is permitted
        -> no change, bond formation, bond rupture, rearrangement, separation, or another valid transition
        -> realize the resulting physical formation

**Natural BREAK must be supported as a chemical transition.** The current code can split a material when explicitly instructed to break an internal bond, but it does not yet provide a universal interaction-driven rule that spontaneously identifies an existing bond as unstable and ruptures it. The experimental chemistry system must support both formation and rupture; otherwise chemistry would be artificially one-directional.

Likewise, **COMBINE should not be an organism command**. Physical contact and compatible geometry may make a new formation possible, but whether a bond actually forms is determined by the universal transition rules. The organism can influence contact and arrangement through physical behavior; it cannot simply command an impossible chemical combination.

### Structural versus nonstructural material

A key experimental hypothesis is to distinguish persistent **structural material** from **nonstructural material within an organism's accessible internal volume** without creating specialized biological organelles.

Structural material participates in the organism's persistent physical graph and remains subject to formation chemistry. Nonstructural material is physically present inside the organism but is not yet part of that structural graph. The organism should be able to manipulate nonstructural material within its accessible boundary—moving it, positioning it, separating it, or bringing it into contact—while the chemistry system still determines which structural transitions are actually possible.

In other words:

    organism controls arrangement
    chemistry controls what arrangements can actually do

This is intended to provide a path for continuous growth without designing a dedicated "combine organelle," "break organelle," digestive organelle, metabolic organelle, or other pre-specified chemical machinery into the initial cell. Material could enter the organism, remain nonstructural, be physically manipulated, undergo valid chemical transitions, and potentially become structural material if the resulting formation is physically and chemically valid.

The word **freely** here means freedom of physical manipulation within the organism's accessible region, not arbitrary violation of chemistry. The organism cannot simply force an incompatible bond to form or destroy a chemically stable bond because it wants to; those outcomes remain consequences of the universal transition system.

### Permeable, stable formations

The chemistry system should permit formations whose ordinary physical consequences produce useful combinations of stability and permeability without declaring them to be membranes or organelles. For example, different compositions/topologies/geometries may naturally produce:

- dense, mechanically stable, minimally reactive structural material;
- thin or loosely bonded boundary formations that remain stable while allowing substantial material passage;
- open or porous formations with high permeability and different mechanical stability.

Permeability should therefore be derived from physical formation and interaction behavior wherever possible rather than introduced as a special biological field. The goal is for a primitive organism to be able to acquire and manipulate material across or within a boundary using the same physical rules available to everything else.

This is a hypothesis to test, not an assertion that the current resource catalogue already contains a sufficient membrane-like formation. The first chemistry experiments should determine whether the existing primitives actually produce stable, minimally reactive, sufficiently permeable formations.

### Runtime and scaling principle

Chemistry should not require scanning all environmental material pairs or maintaining a global registry of every active interface. That approach does not scale to large populations. The intended architecture is for formations/materials to carry or reference compact, reusable interaction information derived from canonical formation structure, so runtime behavior is primarily **lookup -> evaluate/apply transition -> realize physical result**, rather than repeatedly solving a chemistry problem from scratch.

The construction/geometry library is the natural place to derive and cache reusable information for canonical formations and interfaces. This must not become a precomputed table of every possible formation against every other formation. Reusable interaction signatures should be compositional and based on the same four foundational inputs.

### Emergence test

The chemistry system must be tested against a deliberately simple initial organism rather than designing the organism around expected chemistry. In particular, we should ask whether a primitive qualifying-genome cell can, without predesigned chemical organelles:

1. acquire material;
2. manipulate nonstructural material inside its accessible boundary;
3. undergo valid formation transitions, including both bond formation and bond rupture;
4. incorporate useful resulting material into its structure;
5. maintain a stable boundary while permitting useful material passage;
6. grow continuously and eventually reproduce.

If this fails, the first response should be to identify which physical/chemical primitive is insufficient—not to add an organelle whose sole purpose is to make the desired chemistry happen.

If the system later produces internal specialization, compartmentalization, selective permeability, or organelle-like structures because those formations improve survival or reproduction, those structures should be treated as emergent evolutionary solutions rather than requirements baked into the initial cell.

## Project direction

The long-term goal remains an open-ended simulation in which organisms can develop structure, acquire resources, sense their environment, reproduce, form niches, and potentially evolve multicellular cooperation from the same general physical and behavioral mechanisms.

The current core bottleneck is narrower: **replacing the temporary fixed genesis scaffold with a valid, fast, local free-form constructor and then completing the physical 80/80 reproduction split.**

## Base-resource geometry

Hydrogen is a **rigid rectangle 1.0 units long × 0.1 units thick**. It is a finite-area physical strip, not a zero-thickness line. Its two primary structural contact points remain at the centers of the two longitudinal end faces, preserving line-like endpoint topology. Any geometry generation, validation, physical contact, construction, or visualization involving Hydrogen must therefore use its rectangular boundary and 0.1 thickness while preserving those two primary endpoints.

## Geometry Reference Library

The geometry catalogue is a **persistent geometry reference library** separated from live organism state. It is durable knowledge, not test state and not a per-construction search cache.

The intended catalogue grows exhaustively from the smallest physical formations upward:

1. record every valid single-resource geometry;
2. enumerate every physically valid two-constituent combination, including repeated resources;
3. derive every valid three-constituent formation from validated smaller formations;
4. continue physically reachable expansion through larger composites, eventually up to 20 constituent resources;
5. canonicalize equivalent formations so the library stores each physical geometry once;
6. preserve the library across tests, process restarts, and constructor runs.

The implementation provides the persistent store, schema/versioning, canonical formation representation, independent geometry validation, durable JSONL storage, base-resource seeding, and durable frontier/progress records. The store lives outside `target/` and therefore is not reset by normal Rust builds or test cleanup.

### Library rules

- The library is **knowledge, not authority for live organism state**. The constructor may use it as a read-only source of known-valid continuations, but committed simulation bonds still go through the normal physical transaction.
- Validation is independent of discovery. A formation is stored only after resource/shape validity, bond topology, connectivity, and non-penetration checks succeed.
- Global translation and rigid rotation are canonicalized. Reflections are **not** collapsed: a mirror image is a distinct physical formation unless the geometry itself makes it identical.
- Repeated constituents are allowed. "20 constituents" means twenty physical pieces; it does not mean twenty distinct resource types.
- Water remains a real resource. Fluid formations without a finite boundary are represented without inventing rigid collision geometry; their context-fitting realization remains a separate physical-field concern.
- Fluid-to-fluid contact is not a rigid bond. Combining fluid with the same fluid produces the same fluid shape with greater volume; the geometry library must not represent that operation as two bonded fluid constituents.
- Floating-point coordinates are represented in canonical signatures with a fixed geometric quantization tolerance. The geometry schema and resource-shape catalogue are versioned so stale geometry cannot silently become current knowledge.
- Tests use isolated temporary library roots. They must never mutate the persistent production catalogue.

### Worker

The geometry worker is a persistent, resumable catalogue process. It loads the persistent library, selects unexplored formation/frontier work, generates exact geometry candidates, validates and canonicalizes them, appends only new formations, and durably records progress so it can resume after interruption. It does not use the constructor's old brute-force placement loop or an arbitrary attempt budget.

The worker expands from validated formations rather than repeatedly solving each composite from scratch. It persists frontier records for formation/resource expansions, resumes unfinished work after restart, and idles rather than hot-spinning when no work is currently available. It can be started with `cargo run -- --geometry-worker`.

Continuous contact families are represented separately from rigid candidate placement. The current fluid model uses 2-D zero-gravity Young–Laplace capillary equilibrium. For Water against a rigid edge, conserved area and the effective wetting contact angle determine a constant-curvature circular arc analytically, with remaining translational freedom along the exposed edge. These cases are represented as continuous-family work rather than being approximated by arbitrary placement sampling. The representation and expansion of all continuous contact families are not yet exhaustive.

The geometry library is intended to become the long-lived reference layer that the constructor can query instead of rediscovering the same geometry during every organism construction.

### Current implementation status

**Implemented:** persistent library core, canonical formation schema, independent validation, durable append-only storage, version manifest, base-resource seeding, durable frontier/progress records, resumable worker process, exact rigid feature-contact generation for the current finite polygon/line feature model, and expansion through the currently implemented constituent range.

**Still incomplete:** exhaustive representation and expansion of every continuous contact family, the final completeness audit of symmetry reduction, and integration of the library into the live organism constructor.

### Geometry library progress

The library includes exact feature-contact generation for rigid formations and expansion from validated smaller formations. Two-body seeding considers base resources against single-resource formations, validates candidates, canonicalizes them, and persists only new formations. Three-body expansion reuses validated smaller contacts, and the worker architecture can continue the same expansion toward 20 constituents.

Every declared rigid bond must correspond to actual physical boundary contact without positive-area/interior penetration. Fluid-to-fluid combinations are deliberately not represented as rigid bonds because they merge into the same fluid geometry with increased volume. The rigid path does not use arbitrary angular sampling or an attempt budget. Runtime rigid surface-contact candidates likewise use exact polygon feature relationships rather than a sampled angular sweep.

Continuous contact families are intentionally not claimed exhaustive. The library has exact representations for the currently supported rigid and point/endpoint contact families, but additional continuous manifolds still require explicit representation before completeness can be guaranteed.

### Geometry canonicalization

The geometry library removes redundant local rotations that leave a constituent's physical shape unchanged. Regular polygons use their exact rotational symmetry, rectangles and line segments use their twofold symmetry, and circles have no meaningful local rotation. This is a proper-rotation equivalence only: mirror-image formations remain distinct. Hydrogen remains a 1.0 × 0.1 finite-area rectangle while retaining its two line-like primary endpoints.

## Geometry Library Visualizer

The geometry library has a dedicated read-only browser microscope. It reads the persistent catalogue without participating in generation or modifying library state. The browser refreshes the catalogue list every five seconds so newly recorded formations become visible; an already selected formation is not re-read or changed.

Run it with:

    cargo run -- --geometry-viewer

Then open `http://localhost:3001/geometry`. The viewer supports:

- browsing persisted formations, ordered from smallest to largest;
- filtering by constituent count or resource/signature text;
- selecting a formation and seeing its actual constituent placements and shapes;
- displaying the persisted bond topology;
- inspecting persisted rigid, point-contact, vertex-contact, and fluid contact families;
- zooming and panning the stored formation without changing it.

The visualizer is deliberately separate from the catalogue worker and the live organism constructor. It is a verification microscope: once a formation is selected, it displays that recorded formation as a static view and does not re-read, regenerate, alter, or reinterpret it. Only newly added library entries are discovered by the periodic catalogue refresh.
