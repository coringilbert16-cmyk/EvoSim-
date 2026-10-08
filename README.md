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

## Chemistry

This section records the current chemistry model and design decisions. Chemistry is intentionally **primitive-driven and emergent**: acidity, toxicity, corrosion, catalysis, digestion, and selective chemical behavior are not dedicated fields or hard-coded organism abilities. They should arise from physical material, local chemical interaction, geometry, topology, bond stability, and the transitions those primitives permit.

### Formation chemistry

**BREAK and COMBINE are chemistry/formation consequences, not organism actions.** Organisms do not issue Break or Combine commands and the initial cell does not contain dedicated chemistry organelles. Organisms influence contact and arrangement through physical behavior; chemistry determines whether a resulting interaction can actually change material.

The universal transition path is:

    local encounter
        -> evaluate material and interface
        -> accumulate interaction
        -> reach a transition threshold
        -> no change, bond formation, bond rupture, rearrangement,
           separation, deformation/material transfer, or another valid transition
        -> realize the resulting physical state

Favorable interaction is therefore not instantaneous. Contact can produce an interaction without immediately forming or breaking a bond.

### Foundational inputs

Local chemistry derives from:

1. **Composition** — constituent identities and quantities;
2. **Bonded topology** — which constituents are connected and how they are arranged;
3. **Exposed geometry** — actual exposed surfaces/features and physical contact;
4. **Bond strength and formation energy** — stability and energetic cost of existing/new bonds;
5. **Chemical position** — each base resource's position on the ordered chemical spectrum.

Mass, potential energy, cohesion, geometry, topology, bond energy, and chemical position remain distinct concepts. No single aggregate value should silently replace them.

### Chemical spectrum

Chemical position is an ordered **1–14 scale**, analogous to an acid/base scale but **not literal pH chemistry**:

    1 — 2 — 3 — 4 — 5 — 6 — [7 Water] — 8 — 9 — 10 — 11 — 12 — 13 — 14

There are seven base resources. The approved resource-to-position mapping is fixed as follows:

| Resource | Chemical position |
|---|---:|
| Methane | 1.5 |
| Sulfur | 3.5 |
| Hydrogen | 5.5 |
| Water | 7.0 |
| Nitrogen | 8.5 |
| Carbon | 10.5 |
| Phosphorus | 12.5 |

Water is the midpoint at **7**. Methane and Phosphorus intentionally remain inside the 1–14 scale rather than occupying its extreme endpoints. These positions are catalog data, not reactivity magnitudes.

Chemical position is **not a reactivity magnitude**. It describes position on the spectrum. For a local pair:

    D = |position_A - position_B|

D is the fundamental chemical separation. Greater separation means greater chemical opposition and therefore a stronger tendency to interact; nearby positions have less opposition.

Chemistry must never average positions across a formation. A formation containing positions 2 and 12 is not chemically equivalent to a pure position-7 material. Multi-constituent chemistry must preserve the constituent identities and local interactions.

Interaction strength is a **bounded nonlinear function of D**, not simply D. Greater opposition must not produce less interaction. The approved curve is fixed by the equations below; it is no longer an open migration-time tuning decision.

Chemical position is also separate from potential energy. Chemical separation describes interaction tendency; potential energy and bond energy describe energetic consequences.

### Local interaction, timing, and bonds

Chemistry is local. EvoSim must not globally scan every environmental material pair or maintain a registry of every active interface.

A local encounter produces an interaction that can accumulate over ticks:

    encounter -> interaction -> accumulation -> threshold -> transition

Strong interactions may cross a threshold quickly; weak interactions may require longer contact. Physical geometry and contact determine whether and how strongly an interface is engaged. Penetration is never a substitute for contact.

Existing bonds have their own stability/energy. Chemical opposition must accumulate enough effective interaction to overcome relevant structural stability before rupture or rearrangement can occur. Strong chemistry cannot teleport through a strong structure.

The approved attraction model separates static chemical potential from distance:

    D = |position_A - position_B|

    I_AB = (exp(kD) - 1) / (exp(kD_max) - 1)

    C(r) = (1 - r/R)^2       for 0 <= r <= R
    C(r) = 0                  for r > R

    F_AB = I_AB * F_max * C(r)

I_AB is a **static pairwise chemical interaction potential**. It does not increase or decrease as the materials move. Distance changes the amount of that potential expressed as physical attraction. Attraction therefore increases as the materials approach and reaches its maximum at contact; it does not decay toward contact. The current simulation has no universal force/acceleration integrator, so this attraction is not routed into a chemistry-specific movement instruction. When universal physical dynamics exists, chemical attraction belongs in that shared physical-force layer.

The approved reaction-accumulation direction is a bounded accumulation model with a calculated activation barrier:

    R_(t+1) = R_t + I_AB * C(r) * G - lambda * R_t

    R >= B  -> transition

where G represents the actual interface/geometry engagement, lambda represents dissipation, and B is a calculated activation barrier derived from the physical situation rather than a universal arbitrary reaction threshold.

The current parameter contract is now fixed for the first implementation pass:
- D_max = 13.0: fixed ceiling of the chemical-position coordinate system.
- k = 1 / D_max: the nonlinear exponent reaches 1 at the top of that coordinate domain.
- R = 1.0: one shared physical geometry unit for the chemistry contact radius.
- F_max = 1.0: one chemistry attraction unit; this is not resource energy.
- lambda = 0.10 per tick: ten percent of accumulated reaction dissipates each tick.
- G = calculated from the realized interface/geometry engagement, constrained to 0..1.
- B = calculated from the actual physical interface/bond state; it is not a universal reaction threshold.

These fixed values establish the first chemistry scale without turning chemistry into a collection of arbitrary per-pair constants. G and B remain calculated quantities because they must reflect actual geometry and structural requirements.

### Multiple constituents and rupture

A formation must not be reduced to whichever two materials happen to be adjacent, nor to a formation-wide average.

The first layer is **local constituent-pair interaction** at actual interfaces. The second is **recurring/coherent pattern detection** across those local interactions. When multiple local interactions repeatedly produce a coherent structural/energetic consequence, that pattern can justify a larger formation transition.

The system must not assume in advance that the weakest bond, strongest bond, or one interface bond is always the one that breaks. Whether rupture is localized or distributed is an experimental question; the affected region should follow the interaction pattern.

**Natural BREAK is required.** The chemistry layer has an energy-surplus BREAK path: a reaction must supply enough energy to meet the exact realized bond disruption requirement before that bond can rupture. Runtime reaction production, interface selection, structure-revision invalidation, and immediate physical BREAK are integrated.

COMBINE follows the same rule: physical contact and compatible geometry make a formation possible, but only the universal transition rules permit the new bond.

Products are not selected from a hard-coded product catalogue. Existing physical constituents and topology are transformed and then physically realized. For example:

    A—B -> A + B
    A + B -> A—B
    A—B—C -> A—C + B

### Energy conservation

Chemical transitions must conserve energy. Chemical interaction is not automatically usable organism energy.

Transition energy may be stored in newly formed bonds, released from broken bonds, transferred into usable organism energy, become heat/environmental energy, become kinetic movement, or remain associated with transferred/expelled material. Not every reaction must yield usable energy.

### Structural and nonstructural material

Material inside an organism may be persistent **structural material** or **nonstructural material within its accessible internal volume**.

Structural material participates in the persistent physical graph. Nonstructural material is physically present but is not yet part of that structural graph. The organism can manipulate nonstructural material by moving, positioning, separating, or contacting it; chemistry still determines which arrangements can produce structural transitions.

    organism controls arrangement
    chemistry controls what arrangements can actually do

This deliberately provides no dedicated digestive, metabolic, combine, or break organelle.

### Fluids

Fluids use the same chemistry rules as rigid material. Their distinction is physical state: fluids flow, deform, and can pass through permeable structures, rather than acquiring rigid structural bonds simply because fluid pieces touch.

Water remains a real constituent at spectrum position 7 while retaining its existing continuous-fluid physical model.

### Emergent properties

Acidity, toxicity, corrosion, catalysis, digestion, and similar effects are expected **consequences**, not fields. A material that repeatedly damages another formation through permitted transitions can therefore behave corrosively without a corrosiveness property; a material that disrupts another formation can behave toxically without a toxicity field.

If an expected phenomenon cannot emerge, first identify which physical/chemical primitive is insufficient rather than adding a dedicated organelle or property.

### Geometry–chemistry division

The Geometry Reference Library is the geometry knowledge layer. Chemistry must use it rather than independently rediscovering or canonicalizing geometry.

Bob's library provides reusable information about:

- canonical formations;
- bonded topology;
- exposed surfaces/features;
- valid physical contacts/interfaces;
- contact geometry and scale;
- equivalent rigid representations;
- positional equivalence for otherwise-identical geometry is **≤ 0.5 units**. Bob records one canonical relationship rather than separate records for microscopic positional variations within that tolerance. Meaningful changes to material, topology, contact feature, orientation, or other geometry remain distinct.

The Chemistry Library will consume those canonical geometry/interface identities and combine them with material composition and chemical position to cache reusable chemical interaction information. Chemistry must not become a second geometry search engine.

The intended division is:

    Geometry Library
        -> "What physical formations and interfaces are possible?"

    Chemistry Library
        -> "Given this canonical physical interface and these materials,
           what chemical interaction/transition does it permit?"

    Runtime
        -> "Are these formations actually near/contacting each other now?"

### Chemistry Library and scaling

The Chemistry Library is a persistent, demand-driven knowledge layer rather than an exhaustive formation × formation reaction table.

A runtime encounter should approach:

    physical locality
        -> cheap candidate filter
        -> Bob geometry/interface identity
        -> canonical chemistry key
        -> chemistry-library lookup
        -> calculate only on cache miss
        -> persist reusable result
        -> runtime accumulates/executes the interaction

Absolute world coordinates, organism identity, tick number, unrelated nearby objects, and current distance should not be part of the intrinsic chemistry signature. Distance belongs to runtime. Equivalent physical representations should resolve to the same canonical chemistry interaction.

The library should become richer through actual environmental encounters rather than by inventing a predefined catalogue of reactions.

### Chemistry maturation before organisms

Once the Geometry Library and Chemistry Library are ready, the environment should be allowed to run **without organisms** for repeated cycles.

The same environmental material movement and physical encounter rules should generate chemistry naturally. New geometry and chemistry knowledge is persisted across cycles.

Each cycle should measure:

- new geometry discoveries;
- new chemistry discoveries;
- repeated/common interactions;
- newly observed transitions;
- library growth;
- computation cost.

The run should continue until discovery substantially saturates rather than stopping at an arbitrary fixed number of ticks. Only after the environment has established a useful baseline of common interactions should the viable initial organism be introduced.

### Chemistry migration status

The chemistry migration is complete at the currently approved architecture boundary. The following boundaries have been audited and migrated:

| Area | Status | Current boundary |
|---|---|---|
| Resource chemical positions | **Complete** | Seven catalog positions are explicit data: Methane 1.5, Sulfur 3.5, Hydrogen 5.5, Water 7.0, Nitrogen 8.5, Carbon 10.5, Phosphorus 12.5. |
| Core chemistry mathematics | **Complete** | Static chemical-position interaction, distance/contact expression, reaction accumulation, dissipation, and barrier checks exist with invariant tests. |
| COMBINE energy boundary | **Migrated** | Remaining formation work is derived from the approved attraction force over the actual remaining approach distance; there is no arbitrary mass/cohesion formation multiplier. The formation threshold remains a structural eligibility/investment requirement, while intrinsic bond potential is stored separately and allocated through the energy ledger. |
| BREAK energy boundary | **Migrated** | Structural BREAK releases the realized bond's stored intrinsic bond potential exactly once. Chemical BREAK uses accumulated reaction energy to pay the realized bond's activation/disruption barrier, with only the surplus becoming usable energy or heat. Constituent material potential is not recreated at bond rupture. |
| Energy conservation | **Implemented at chemical BREAK boundary** | The ledger remains authoritative. Chemical BREAK energy is settled as one conserved reaction-energy source: disruption cost + usable energy + heat = reaction energy. |
| Geometry → chemistry interface | **Live Bob resolver connected** | Runtime chemistry resolves each realized bond contact through Bob's live candidate resolver, retaining local boundary feature/parameter identity rather than topology alone. Distance and facing remain transient runtime state and are not part of the intrinsic chemistry key. |
| Chemistry Library | **Runtime lookup + calculate-on-miss + persistence integrated** | The versioned persistent library accepts material pairs plus canonical Bob interface identities. Runtime chemistry now performs lookup, calculates the static interaction potential only on a miss, and persists the reusable result. Activation barriers remain physical bond properties and are not cached as chemistry facts. |
| Natural chemistry-driven transitions | **Runtime BREAK + immediate COMBINE formation path** | Existing bonded interfaces accumulate local reaction state using the fixed chemistry contract; reaching the physical bond barrier produces an immediate local BREAK event. COMBINE now resolves immediately through the same physical formation boundary, with chemistry attraction gating real formation and developmental preference retained as solver guidance. Independent BREAK interfaces may react in the same tick; physical conflicts are resolved against the current structure. Autonomous multi-interface formation remains. |
| Retired reactivity cleanup | **Complete** | Obsolete chemistry/resource-selection uses have been removed. Remaining `reactivity` uses are owned by harmonics/nonlinear spectral response or compatibility fixtures/data and are not used as chemical position, interaction magnitude, break efficiency, or formation energy. |

**Migration rule:** chemical position is chemistry; reactivity is not a fallback chemical position, interaction magnitude, break-efficiency factor, or formation-energy source. Remaining `reactivity` code is therefore treated as transitional until its owning subsystem has an explicit replacement contract.

`potential_energy` remains an intrinsic material property and is not being silently repurposed as chemical interaction strength. The energy ledger remains the authority for all realized energy transactions.

### Chemistry completion audit

The migration completion audit is closed at the currently approved architecture boundary. Findings were checked against the live implementation before documentation was finalized. The deferred universal physics consumer is an intentional architectural seam, not a chemistry migration defect.

1. **Chemistry implementation inventory — complete**
   - `chemical_position` is the sole chemical-spectrum input.
   - `reactivity` remains only in harmonics/nonlinear spectral response and compatibility fixtures/data.
   - `potential_energy` remains an intrinsic material/energy-ledger property.
   - `cohesion` remains a physical/material property used where its own contracts require it.
   - Chemistry interaction, BREAK, COMBINE, reaction thresholds, and library persistence use the migrated paths described above.

2. **Resource-property semantics audit**
   - Verify mass, potential energy, cohesion, and chemical position for every base resource.
   - Establish the exact decimal chemical-position mapping.
   - Find every place chemical position is incorrectly averaged or treated as a reactivity/energy magnitude.

3. **Mathematical chemistry audit**
   - Translate the approved equations into exact implementation contracts.
   - Identify every remaining parameter requiring definition.
   - Verify monotonicity, boundedness, contact behavior, and limiting cases before implementation.

4. **Energy-conservation audit**
   - Trace potential energy, bond energy, work, formation, rupture, transformation, kinetic transfer, and environmental transfer.
   - Identify every current path that creates, destroys, or silently transfers energy.
   - Establish conservation invariants for each transition class.

5. **Formation/interface audit**
   - Determine how formations, bonds, exposed features, contact, contact area/coverage, and geometry are currently represented.
   - Identify where chemistry currently reconstructs geometry independently or receives incomplete geometry.

6. **Bob/Geometry Library audit**
   - Establish what the Geometry Reference Library actually provides, what remains incomplete, how canonicalization works, and what Chemistry can query.
   - Identify direct contradictions only; do not redesign working geometry infrastructure without evidence.

7. **Chemistry Library feasibility audit**
   - Define the minimum canonical chemistry key.
   - Verify that composition + Bob's canonical geometry/interface identity is sufficient.
   - Estimate combinatorial growth and ensure demand-driven caching remains bounded.

8. **Runtime locality/performance audit**
   - Trace candidate generation, spatial partitioning, neighbor lookup, formation-level checks, constituent-level checks, and chemistry invocation frequency.
   - Prove there is no hidden global pair scan.
   - Estimate worst-case chemistry work under dense local conditions.

9. **Existing test-contract audit**
   - Classify chemistry tests as authoritative, expectation-to-update, obsolete, broader physical invariants, or missing.
   - Preserve tests that enforce genuine physical contracts even when old chemistry semantics are removed.

The implementation migration is complete; remaining work belongs to downstream chemistry experiments, universal physics, or separate constructor/geometry workstreams.

### Chemistry implementation order

After the audits, the implementation sequence is:

1. establish exact chemistry/resource semantics;
2. migrate the existing system from obsolete reactivity equations to the approved chemistry model;
3. finish Bob the Builder and the Geometry Reference Library to the required chemistry interface;
4. build the persistent Chemistry Library on top of Bob;
5. run the organism-free environment in cycles until geometry/chemistry discovery substantially saturates;
6. build and verify a viable initial cell;
7. place the organism into the matured environment and troubleshoot biological behavior from there.

The guiding boundary is:

    Geometry -> Chemistry -> Environment -> Organism

This order is intended to make failures diagnosable: after the organism is introduced, a failure should be traceable either to the established physical/chemical world or to the organism's behavior, rather than both systems being unfinished simultaneously.

### Completion criteria

Chemistry is sufficiently complete for integration when:

1. local interaction derives from spectrum position rather than obsolete reactivity magnitude;
2. chemical separation, bond stability, and potential/bond energy remain distinct;
3. static pairwise interaction potential is separated from distance-dependent attraction;
4. attraction increases as interacting materials approach;
5. reaction interaction accumulates over time with dissipation;
6. transitions use calculated activation barriers;
7. natural bond rupture and bond formation use the same transition framework;
8. multi-constituent transitions can arise from recurring/coherent local patterns without a universal weakest-bond rule;
9. energy is conserved;
10. acidity/toxicity/catalysis/etc. require no dedicated fields or organelles;
11. rigid and fluid material share chemical transition logic while retaining different physical states;
12. runtime uses local lookup/evaluation rather than global pair scanning or an exhaustive formation-pair reaction table;
13. Chemistry uses Bob's canonical geometry/interface knowledge rather than duplicating geometry discovery;
14. the persistent Chemistry Library can cache reusable interaction knowledge without unbounded redundant representations;
15. the organism-free environment can populate common geometry/chemistry interactions before the organism is introduced.

This does not require predicting every chemistry before simulation. Remaining curve and transition details should be settled by focused experiments once the primitive interfaces exist.

### Current status

The chemistry model, migration, and completion audit are complete at the approved architecture boundary. Resource chemical positions, the core chemistry equations, COMBINE's energy boundary, BREAK's energy boundary, and the chemistry-driven BREAK energy-surplus gate have been migrated away from the retired interaction semantics.

The remaining `reactivity` uses are outside authoritative chemistry. The resource-helper, construction-selection, and obsolete math uses have been removed. Harmonic/nonlinear spectral response has been audited and deliberately retains `reactivity` as a separate material-response input; compatibility fixtures/data remain for separate cleanup. No remaining use may be mapped to chemical position without a subsystem-specific contract.

Natural COMBINE formation has now been brought into the same energy model. Remaining formation work is derived from the integral of the approved attraction force over the actual remaining approach distance; there is no arbitrary mass/cohesion formation multiplier. The formation threshold remains a physical eligibility/investment requirement, while intrinsic bond potential is a separate stored structural quantity and is allocated separately in the formation transaction. The chemistry-driven BREAK seam now exists: reaction energy can rupture an existing bond only when it meets that bond's supplied physical disruption requirement. Chemistry accumulation is stored in normalized chemistry units, then converted explicitly to physical energy at activation using force × characteristic contact distance (`CHEMICAL_MAX_FORCE × CHEMICAL_CONTACT_RADIUS`). The approved values make that conversion 1:1 numerically without hiding the unit conversion. Chemistry must not implement geometry mapping itself.

## Project direction

The long-term goal remains an open-ended simulation in which organisms can develop structure, acquire resources, sense their environment, reproduce, form niches, and potentially evolve multicellular cooperation from the same general physical and behavioral mechanisms.

The project execution order is now:

**Chemistry -> system migration -> Bob/Geometry Library -> Chemistry Library -> organism-free environment maturation -> viable cell -> organism in environment.**

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
