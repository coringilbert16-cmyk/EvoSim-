# EvoSim

EvoSim is an open-ended evolutionary organism simulation. Organisms are built from physical material and physical bonds; structure and behavior should emerge from general mechanisms rather than fixed predator/prey roles or a predefined organism-size progression.

## Repository unification gate — 2026-10-10

> [!CAUTION]
> **Do not treat the current default branch (`main`) as a downloadable EvoSim source checkout yet.** A recursive GitHub tree audit found only 15 entries, rooted under `chemistry_library/`, `docs/`, and `geometry_library/`; it has no `Cargo.toml` or `src/main.rs`. It is currently a data/documentation branch, not a complete runnable Rust application.

The current unified integration target is [PR #186 — Integrate EvoSim source and Bob regeneration into main](https://github.com/coringilbert16-cmyk/EvoSim-/pull/186), branch `integration/unified-source-and-catalogue`. It restores the runnable Rust source, UI, scripts, and current project documentation. The obsolete generated geometry catalogue has since been deliberately removed from both `main` and this integration branch; geometry knowledge must be regenerated only after the generator/constructor path is validated. It remains draft-only.

### Current stabilization evidence

- The integration-branch Rust workflow [run 38055388755](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38055388755) passed formatting, source-file-size, COMBINE architecture, focused Bob compact-storage tests (4 passed), and the fresh v3 library create/reopen check. [Runner validation run 38055388762](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38055388762) passed.
- The same Rust workflow is **not green**: `cargo test --all-targets` reported **253 passed, 75 failed, 1 ignored**. Failures cross geometry/contact, cavity/genome qualification, constructor/acquisition, chemistry/COMBINE, movement/perception, observation, viability, storage, reproduction, transformation, runtime, and simulation integration.
- Clippy also fails under `-D warnings`, including unfulfilled `dead_code` expectations and ordinary lint errors. Do not suppress these wholesale; determine which APIs need integration and which are obsolete, then fix the relevant code.
- The live initial-organism constructor remains a fixed 54-unit Carbon scaffold. It does **not** yet use Bob for free-form construction, still relies on a `1.0e12` construction-energy budget, uses an unapproved farthest-contact fallback, and does not validate the full required-resource acquisition set atomically.
- Bob's focused persistence checks prove only the tested store behavior. They do not prove the live-family query contract, compatibility of the checked-in catalogue, full catalogue-generation completion, or viable organism construction.

### Required order before the next clean download

1. Keep PR #186 as the single integration target; preserve the approved biological/physical rules and geometry-library implementation, but do not treat the deleted generated catalogue as source of truth.
2. Resolve Clippy/build hygiene and shared physical-contract failures at their common authority, not by patching failing tests independently or weakening invariants.
3. Finish Bob's live-family resolution and define corruption/duplicate/manifest recovery for persistent geometry and chemistry data.
4. Replace the fixed scaffold with the approved milestone-driven constructor, including whole-structure validation, atomic transactions, analyzer-confirmed bonded genome cavity, physical acquisition of Water plus at least three other resources, and reconciled energy/material accounting.
5. Run the actual constructor as the acceptance test, then focused contracts and the full suite. A Bob worker smoke test alone is not acceptance.
6. Add a non-generative, isolated empty-library open contract. Source inspection indicates an empty root can be opened, but runtime behavior is not yet verified. Do not seed or run the worker just to test startup or refill the deleted files.
7. Only after these gates pass, merge the verified integration lineage and make it the default branch.

Until those gates pass, a fresh clone of `main` is not the right download target, and PR #186 remains a work-in-progress integration branch rather than a stable release. See [the full repository stabilization audit](docs/repository-stabilization-audit.md) for source-level findings, CI evidence, ordered implementation gates, and the definition of stable enough to download.

## Constructor and repository stabilization handoff — 2026-10-10

> [!IMPORTANT]
> This section is a dated engineering handoff, not a claim that the replacement constructor is implemented. The authoritative architecture is the **Replacement Initial Organism Constructor** plan below. Keep this handoff current as audits progress so work can resume without relying on conversation history.

- **Current integration target:** [PR #186 — Integrate EvoSim source and Bob regeneration into main](https://github.com/coringilbert16-cmyk/EvoSim-/pull/186), draft only. The prior staged PRs #181–#185 were closed as superseded; their source commits remain in the integration lineage. Do not merge or download this as a stable release while compile/lint and constructor-viability gates remain open.
- **Current live path:** `Simulation::create_initial_organism` calls `initial_organism_constructor::construct_valid`. The live constructor is still the fixed Carbon ring/support/outer-ring baseline (54 units); it does not yet use Bob to select a free-form growth sequence.
- **Geometry library is advisory candidate knowledge, not physical authority.** Revalidate each proposed placement against the whole realized structure and current contact, nonpenetration, endpoint, bonding, and energy rules. Bob's `GEOMETRY_EQUIVALENCE_TOLERANCE = 0.5` is not the approved live-contact tolerance of `0.1`; neither permits penetration.
- **Chemistry is split across explicit authorities:** catalog material properties in `materials/resources.rs`; chemical-position interaction in `chemistry/chemistry.rs`; bond work/strength in `chemistry/combine.rs`; energy transactions in `chemistry/energy_ledger.rs`; cached static interface potentials in `chemistry/chemistry_library.rs`. Do not duplicate these equations in the constructor.
- **Known specification mismatches:** the current integration branch's Phosphorus geometry is already the approved isosceles trapezoid (bottom 1.5, sides 0.5, top 1.0); do not alter it based on older audit notes. The constructor still uses `CONSTRUCTION_ENERGY = 1.0e12` and startup turns the remainder into organism energy; the inspected bond-strength path does not yet apply the approved full/half contact-feature scaling. These remaining mismatches require deliberate, separately tested corrections—not silent audit-time parameter changes.
- **Latest completed integration-branch CI:** formatting, source-size, COMBINE architecture, focused Bob compact-storage tests, and fresh-library persistence/reopen checks passed. `cargo test --all-targets` executed and reported **253 passed, 75 failed, 1 ignored**; Clippy also fails on strict dead-code expectations and other lint diagnostics. The failures span construction/geometry, chemistry, observation, viability, reproduction, and simulation integration. Do not describe the suite as green or treat this result as a reason to change unrelated rules blindly.
- **Next implementation gate:** resolve compile and strict-lint blockers without blanket suppressions; execute focused geometry, persistence, whole-structure collision, transactional accounting, cavity, and acquisition contracts; then wire the approved milestone-driven constructor to Bob. The acceptance test is the actual constructor producing a physically viable organism, followed by the full suite.
- **Lookup status (source audit updated 2026-10-10):** `GeometryLibrary::resolve_persistent_interface` has an indexed partial resolver for rigid-edge, rigid-point, and rigid-vertex queries and distinguishes unique/ambiguous/unresolved matches. The separate `classify_live_family_resolution` helper still always returns `Unresolved`, and fluid-boundary live queries are not covered by the indexed query variants. Most importantly, the live initial constructor does not open or query Bob at all; the resolver currently provides no construction benefit.
- **Chemistry-cache integrity (focused tests verified; full suite remains red):** schema 3 is now the current format; the loader accepts schema 2 as a legacy read path without relabeling keys. Invalid records, malformed final rows, duplicate keys, unknown manifest versions, and manifest count mismatches now fail with diagnostics. The runtime chemistry key builder now uses bare resource names for one-part, one-unit base materials so keys can match the checked-in cache; mixtures and non-unit amounts remain explicit. All new persistence/key-contract tests passed in CI. The same run passed formatting, source-size, COMBINE architecture, focused Bob storage, and fresh-library reopen checks, but the full suite remains at **264 passed, 75 failed, 1 ignored**, and strict Clippy fails.
- **Acquisition-set gap:** the fixed constructor chooses each resource placement separately. It does not validate the complete set together in one trial state; the replacement must do so before committing any acquisition.
- **Bond-ranking gap:** the fixed scaffold fallback chooses the farthest eligible contact when no corner-corner candidate is available. This undocumented rule must not be copied into the replacement without an explicit, tested score.
- **Recent commits:** chemistry schema/integrity fix and tests are recorded in [the chemistry fix](https://github.com/coringilbert16-cmyk/EvoSim-/commit/4df90c6f9a8ae01ba47cdcb14525dfa85113ddaf), [the integrity tests](https://github.com/coringilbert16-cmyk/EvoSim-/commit/e9dcefa6b2740cce3f292b168b2e2c1d8f2b7b98), [base-material key compatibility](https://github.com/coringilbert16-cmyk/EvoSim-/commit/692741c68536b4e40494d46457ed11a11ea0bfd3), and [the cache-key contract test](https://github.com/coringilbert16-cmyk/EvoSim-/commit/214a9b3ebb17c95e8e89d2605759cde09d0b0645). Subsequent formatting corrections are committed; a passing verification run for the current branch head is still outstanding. See `docs/constructor-library-dependency-audit.md` for the dependency map, contract observations, blockers, test evidence, and removal checklist.


> [!IMPORTANT]
> ## Authoritative constructor plan — replacement architecture
>
> **The “Replacement Initial Organism Constructor” plan below is the only current constructor architecture plan. Every earlier constructor plan or proposed construction workflow in this README is obsolete and must not be used to guide implementation.** Historical descriptions are retained only to explain the repository's migration history. They are not competing requirements.
>
> The replacement is a library-driven, milestone-based physical growth system. It uses the Geometry Reference Library and Chemistry Library as reusable physical knowledge, validates every candidate through authoritative runtime rules, commits physical changes atomically, discovers the genome from the realized bonded cavity, and stops construction when actual viability requirements are met. It does not build to a fixed piece count or prescribe a final topology.
>
> This document records the approved design, not completed implementation. Source inspection, successful compilation, and end-to-end construction are separate forms of evidence and must be reported separately.

## Historical constructor implementation snapshot — not an architectural plan

> [!WARNING]
> The implementation notes in this section describe a previously inspected transitional implementation. They are historical context only, may no longer describe the current branch head, and must not be interpreted as the approved replacement design. Use the authoritative replacement plan below for all constructor work.

## Historical constructor implementation state (superseded)

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

## OBSOLETE — Previous Intended Constructor Architecture (No Longer Valid)

> [!CAUTION]
> **OBSOLETE PLAN — DO NOT IMPLEMENT OR EXTEND THIS PLAN.** Its forward-only/local-continuation workflow, wait-on-stall behavior, and other architecture statements are superseded by the authoritative replacement plan below. Reuse valid physical invariants only where the new plan explicitly preserves them.


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

## OBSOLETE — Previous Immediate Engineering Plan (No Longer Valid)

> [!CAUTION]
> **OBSOLETE PLAN — DO NOT FOLLOW ITS CONSTRUCTOR DEFERRAL OR INTEGRATION ORDER.** The earlier instruction to defer constructor redesign and later prepare another proposal is superseded. The approved task is now to develop the replacement constructor around the geometry and chemistry libraries. The library audit and stabilization work described below may still be useful where it supports the new plan, but it is no longer a reason to postpone constructor architecture work.


The immediate engineering priority is **stabilizing Bob's geometry knowledge layer**. Constructor redesign and constructor-to-Bob integration are explicitly deferred until the library interface and lookup behavior are internally consistent and documented.

1. **Reconcile Bob's source/API contract.**
   - Confirm that the module wiring, public library functions, worker imports, and documentation describe the same API.
   - Resolve missing, renamed, duplicated, or incorrectly mapped functions before adding new layers.
   - Keep one authoritative implementation for persistent formations, contact families, canonicalization, and indexed lookup.
2. **Define and verify lookup semantics.**
   - Document which queries are supported, what their results guarantee, and how geometry/schema versions affect eligibility.
   - Treat library entries as reusable knowledge, not as permission to skip current physical validation.
   - Keep discovery/worker responsibilities separate from read-only runtime lookup responsibilities.
3. **Evaluate bounded frequency-aware priority caching.**
   - First measure existing indexed lookup cost and access patterns; do not assume a cache is needed solely because the catalogue is large.
   - If measurements justify it, add a bounded in-memory cache for frequently reused formation and contact-family query results.
   - Promote entries based on observed frequency, enforce a clear memory bound, and expose hit/miss/eviction measurements.
   - Keep persistent catalogue data authoritative for knowledge storage; the hot cache is disposable acceleration state, not a second source of truth.
4. **Verify Bob independently.**
   - Add focused tests for API consistency, canonicalized lookup, contact-family lookup, cache promotion/eviction if implemented, and cache-disabled or cold-start behavior.
   - Confirm worker persistence and read-only lookup remain separate, and that tests use isolated temporary library roots where writes are involved.
   - Run formatting, compilation, and focused tests when a runnable source checkout is available; report unrun checks explicitly.
5. **Stop at the library boundary for this phase.**
   - Do not redesign, modify, or wire the initial-organism constructor into Bob in this phase.
   - Do not build cache policy around assumptions about constructor call patterns before those patterns can be measured.
   - Once Bob's API and measured lookup behavior are stable, document a separate proposal for later constructor integration before beginning that work.

## Construction invariants

These are physical/specification invariants, not an alternative constructor algorithm. Retain them where consistent with the authoritative replacement plan below.

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


# Replacement Initial Organism Constructor
## Authoritative architecture, library integration, migration, and validation plan

**Status:** Approved design plan; implementation is not implied by this documentation.  
**Scope:** Replace the current initial-organism construction architecture and remove superseded constructor plans and implementation paths.  
**Primary objective:** Produce a physically valid, viable initial organism through the minimum necessary sequence of real construction decisions, without prescribing its final topology.  
**Core principle:** The constructor coordinates established physical authorities. It does not become another geometry engine, chemistry engine, or organism-design script.

## 1. Executive summary

EvoSim needs a constructor that can produce a viable organism from available materials. It must discover a genome through the realized physical structure, acquire required materials through valid physical interactions, and satisfy the actual biological requirements.

The replacement will not assemble toward a fixed piece count and test viability afterward. It will propose individual physical changes or reusable local arrangements, validate them before commitment, commit only complete valid transactions, and check developmental milestones as construction proceeds.

The geometry library will supply reusable knowledge about shapes, contact features, and physically plausible arrangements. The chemistry library and authoritative bond rules will determine material compatibility and chemical/energetic consequences. The constructor will coordinate these authorities, choose among viable candidates, and stop when the actual organism is viable.

The replacement must:
- Construct an initial organism without a prescribed blueprint or final shape.
- Use existing geometry and chemistry knowledge instead of repeatedly rediscovering it.
- Discover a genome cavity through the real cavity analyzer and require a qualifying bonded seal.
- Stop treating enclosure as a goal immediately after genome qualification.
- Physically acquire Water and at least three distinct non-Water resource categories.
- Enforce geometry, bonding, material, energy, and ledger invariants.
- Permit multiple viable outcomes, with deterministic reproducibility and seed-driven diversity.
- Share its physical engine with offspring construction while allowing a different policy and limited inventory.
- Report why it succeeded, stalled, or failed.
- Remove the old fixed-size and prescribed-construction architecture rather than wrapping it in another layer.

This is a design plan. It is not evidence that the implementation exists or has passed tests.

## 2. Lessons from other artificial-life simulators

### Framsticks — development versus realized body

Use explicit developmental milestones and evaluate the realized physical result, not the structure the constructor intended to create. Do not import genotype-driven predetermined body construction: EvoSim's first organism must physically discover its qualifying genome cavity.

### MABE — modular authorities

Keep geometry, chemistry, bonding, cavity analysis, acquisition, energy accounting, and construction policy separate. The constructor orchestrates these authorities; it must not duplicate their rules.

### Tierra and Avida — viability is a bootstrap condition

Initial construction establishes a viable starting point. It must not encode a permanent evolutionary trajectory, body ladder, or fixed set of organism roles.

### Polyworld — apparent success is not physical proof

A plausible-looking organism is not sufficient. Independently validate collisions, bonds, cavity qualification, resource acquisition, and accounting.

**Combined lesson:** use milestones without a prescribed body plan, modular physical authorities, minimal viability conditions, and independently measurable correctness.

## 3. Architecture and responsibilities

### 3.1 Authoritative physical knowledge

The Geometry Reference Library provides reusable geometric knowledge. The Chemistry Library and existing chemistry/bond authorities provide material and interaction knowledge. Runtime physical rules remain authoritative for the current organism.

### 3.2 Candidate query and indexing

Indexes narrow the search using material composition, exposed connection features, feature scale, arrangement size, shape, orientation, and validation status. Indexes are regenerable accelerators, not independent sources of physical truth.

### 3.3 Authoritative validation

Every candidate is checked against the actual context. Validation covers geometry and nonpenetration, physical contact, bond compatibility and strength, interactions with the existing structure, available resources, energetic consequences, and accounting.

### 3.4 Atomic construction transaction

A candidate is prepared and validated before commitment. A successful transaction updates all relevant geometry, bonds, inventory, resources, energy, ledger entries, exposed features, and analysis caches together. A failed transaction leaves the organism unchanged.

### 3.5 Developmental policy

The policy selects the next objective and chooses among legal candidates. During genome discovery, it favors feasible growth toward a qualifying cavity. Once a genome is confirmed, it stops optimizing for enclosure and pursues remaining viability requirements.

### 3.6 Verification and handoff

A final validator checks the realized organism and its ledgers. The constructor returns the validated structure in the representation expected by the simulation. It must not silently repair invalid physics at handoff.

## 4. Geometry Reference Library integration

### 4.1 Audit the existing catalogue first

Inventory the files, schemas, versions, record counts, material combinations, shape definitions, connection features, arrangement sizes, provenance, and validation status. Distinguish raw samples from validated arrangements. Record duplicates, malformed records, missing coverage, and version conflicts. Do not assume every stored sample is valid in every context.

### 4.2 Separate stored knowledge from validation guarantees

Track whether an entry is unverified, locally validated, compositionally validated, or validated in a particular context. An arrangement valid in isolation can still collide with a distant part of the organism when inserted into a larger structure.

### 4.3 Index for actual constructor queries

Index by useful physical features: material composition, shape/constituent types, exposed connection feature, feature scale, orientation, local envelope, arrangement complexity, and validation status. Each index must answer a demonstrated query and be reproducible from authoritative data.

### 4.4 Generate placements from features, not blind coordinate sweeps

When a connection feature is exposed, query plausible placements that align compatible features. Enforce the approved contact tolerance of 0.1 where applicable; touching is permitted, penetration is forbidden. Respect feature-scale compatibility and the specified bond-strength distinctions for edge, corner, and line-end contacts. Bounding boxes and spatial indexes may reject obviously irrelevant candidates, but cannot serve as final proof of contact or nonpenetration.

Resolve the existing constructor's 0.05 tolerance against the approved 0.1 value at the shared authority; do not patch only one local constant.

### 4.5 Reuse multi-piece arrangements as optional shortcuts

Validated local composites may reduce repeated search. They are candidate shortcuts, not mandatory building blocks or biological organs. Validate all external interfaces and the full contextual placement. The constructor must remain able to add individual pieces, reject a composite, and deviate from a reused arrangement later.

### 4.6 Query using current context

Candidate lookup must account for actual exposed features, available materials, unmet milestones, existing structure, and feasible resource-acquisition paths. Do not scan the entire catalogue without regard to the current construction state.

### 4.7 Preserve correctness when coverage is incomplete

A missing catalogue entry may make construction slower or reduce success probability; it must never cause invalid geometry to be accepted. Keep a correct fallback candidate-generation path where necessary and measure when it is used.

## 5. Chemistry Library integration

### 5.1 One authoritative source for material properties

Read mass, potential energy, reactivity, cohesion, and other material properties from the same authoritative definitions used by the simulation. Do not duplicate property tables in the constructor. `potential_energy` is the absolute maximum for a resource type; it is not a mutable `energy_content` value.

### 5.2 Centralize compatibility and bond consequences

For each candidate, ask the authoritative chemistry/bond system whether the material pair and contact features may bond, what strength applies, what energetic consequences follow, and which accounting entries are required. Repair an incomplete authority at its source rather than adding constructor-only exceptions.

### 5.3 Separate computation budget from physical energy

The old `ASSEMBLY_ENERGY = 1.0e12` pattern and deriving initial organism energy from the remaining assembly budget must not be carried forward. Computation/search limits are not physical energy. Account for real construction costs, transfers, reserves, and transformations according to the energy ledger; return only energy actually justified by the model.

### 5.4 Preserve physical acquisition

Environmental abundance does not permit direct insertion of logical resource labels. Resource acquisition must use the actual physical acquisition semantics and placement checks. Water and at least three distinct non-Water categories must be acquired and accounted for.

### 5.5 Distinguish availability from suitability

For each candidate, separately evaluate whether a material is available, eligible for the action, chemically compatible, geometrically placeable, and affordable under the applicable constraints. This distinction supports both abundant initial construction and constrained offspring construction.

## 6. Milestone-driven construction algorithm

### Step 1 — Initialize explicit state

Track the realized structure, constituent positions, valid bonds, exposed features, material inventory, acquired resources, energy and material ledgers, genome-analysis status, current milestone, deterministic random seed/state, and diagnostics. Do not silently insert a completed genome, unearned resources, or a prescribed final body plan.

### Step 2 — Identify the current milestone

Begin with a valid physical starting condition. Discover a qualifying genome cavity. Then satisfy the remaining organism requirements. Milestones describe biological conditions, not piece counts or exact arrangements.

### Step 3 — Generate candidates from both libraries

Generate candidate single-piece attachments, reusable local composites, compatible material combinations, and needed resource placements. Candidate generation is demand-driven: missing resource requirements should trigger relevant acquisition opportunities instead of endless unrelated growth.

### Step 4 — Validate before commitment

Geometry checks must confirm valid shapes, contact, scale compatibility, nonpenetration, and consistency with the whole relevant structure. Chemistry checks must confirm material eligibility, bond validity/strength, and energy/accounting consequences. Developmental checks must confirm the candidate serves or preserves the current milestone and does not needlessly destroy all feasible routes to outstanding requirements.

The existing `penetrates_local_neighborhood()` check against only the anchor and its direct bonded neighbors is not a sufficient whole-structure guarantee unless a proven invariant makes it complete. Use spatial indexing for efficiency, but preserve complete relevant collision detection.

### Step 5 — Use bounded, physically grounded feasibility lookahead

Greedy local growth can block later completion. During genome discovery, avoid candidates that demonstrably eliminate all feasible enclosure paths. During resource acquisition, avoid blocking all feasible placements for required resources. Prefer a candidate with plausible continuation when otherwise comparable alternatives lead to dead ends.

Lookahead estimates feasibility; it does not know or prescribe the final organism. It must not be replaced by a fixed piece count, a hardcoded ring, a mandatory scaffold, or arbitrary attempt/node caps treated as biological facts. If an operational computation budget is exhausted, report that the search effort was exhausted; do not claim physical impossibility without proof.

### Step 6 — Select without catalogue-order bias

Reject invalid candidates, compare remaining candidates according to explainable milestone and feasibility criteria, and use seeded randomness among comparable alternatives. File order and material enumeration must not silently dictate the organism. Any material or topology preference must be justified by actual constraints or an explicit approved policy.

### Step 7 — Commit atomically

Commit geometry, bonds, inventory, resource placements, energy, ledgers, exposed features, and cache updates as one transaction. If any required operation fails, roll back the entire transaction. No partial state may become visible to the simulation.

### Step 8 — Re-evaluate after relevant changes

Run cavity analysis after changes that can create or destroy a cavity or alter its seal. Cache analysis only with sound invalidation rules. Re-evaluate outstanding resource and viability requirements after relevant physical changes.

### Step 9 — Stop on actual viability

Return only when the realized organism passes all required checks. If the process stalls, report unmet requirements and blocking causes. Do not keep growing toward an arbitrary size hoping viability will emerge.

## 7. Genome discovery is a real milestone

There is no predefined genome core. A cavity qualifies only when the authoritative analyzer finds it in the realized structure and confirms the required bonded seal. An unsealed or accidental cavity is not a genome.

Run or refresh analysis when geometry or bonding changes can affect qualification. When a qualifying cavity is confirmed, record the actual result, switch milestones, and immediately stop treating enclosure as a construction objective. Do not make genome bonds indestructible or introduce special cavity-protection physics unless separately required by the physical specification.

## 8. Physical resource acquisition

Track which required categories are available, attempted, acquired, and currently represented. Use the authoritative placement and contact rules, not sampled bounding-box positions as final proof. Confirm Water and at least three distinct non-Water resource categories are physically acquired.

Respect the existing fluid rules: Water is logically abundant but physically represented as required; it remains permeable rather than a rigid wall; the genome cavity interior is exempt from water fill; and water-only sequences must not count as valid structural connections where a non-water connection is required. These rules belong to shared physical authorities, not constructor-specific exceptions.

## 9. One engine, two policies

### Initial-organism policy

Start without a required blueprint, use the abundant environment while obeying real acquisition rules, discover a qualifying genome, acquire the required resources, and stop promptly when viable.

### Offspring policy

Start from the inherited guidance and limited acquired material actually available. A blueprint is a soft preference, not a command. Permit imperfect development and valid deviations. Previously discussed similarity thresholds (>0.90 for early commitment and <0.6 for refusal/reconsideration) may be used only after the metric and semantics are documented and reconciled with the specification; they are not physical laws.

Both policies must share geometry lookup, chemistry/bond rules, validation, transactions, acquisition, ledgers, and diagnostics. Do not retain a separate blueprint renderer with different physical assumptions.

## 10. Invariants

### Geometry
- No committed change introduces forbidden penetration.
- Contact tolerance and feature-scale compatibility follow the approved rules.
- Bonds correspond to valid contact and chemistry.
- New additions cannot silently corrupt existing structure.
- Spatial indexes and caches do not weaken completeness or consistency.

### Chemistry and accounting
- Material properties and bond rules have one authoritative source.
- Every material and energy change is recorded.
- Search budget is not converted into physical energy.
- Failed transactions leave accounting unchanged.
- Ledgers reconcile with the realized state.

### Biology
- The real analyzer establishes genome qualification.
- A qualifying genome has a bonded seal.
- Enclosure stops being a goal after qualification.
- Required resource categories are physically acquired.
- Final accessibility and structural requirements are verified.

### Transaction
A candidate produces either the original unchanged state or the complete validated new state—never a partially committed intermediate state.

## 11. Required removals and cleanup

The rewrite must remove obsolete architecture, not simply wrap it.

1. **Fixed-size viability target:** remove the strategy of building to a target such as 400 pieces before checking viability. Delete dependent constants, retry logic, comments, and tests that make that target a biological requirement.
2. **Prescribed initial topology:** remove mandatory rings, scaffolds, spirals, central materials, and topology-specific branches unless an independently documented physical requirement proves they are necessary. Remove tests that equate one topology with success.
3. **Legacy seed/calibration path:** trace all uses of `confirmed_seed_baseline()`, `confirmed_seed_scale_reference()`, and `juvenile.rs`. If a calibration artifact still serves a legitimate scaling purpose, isolate and document it; remove its influence on live initial construction. Do not delete dependencies blindly.
4. **Duplicate geometry logic:** remove constructor-owned shape/contact/collision rules, duplicate tolerances, independent feature calculations, and sampled placement checks used as final validation. Route queries and final validation through the authoritative geometry system.
5. **Duplicate chemistry logic:** remove local material tables, bond-strength rules, compatibility exceptions, and energy calculations that compete with chemistry/bond authorities.
6. **Approximate resource placement as authority:** replace coarse-grid/four-rotation sampling as final proof with library-backed candidates and authoritative placement validation.
7. **Artificial assembly energy:** remove the conversion from `ASSEMBLY_ENERGY = 1.0e12` or remaining search allowance into initial organism energy. Preserve legitimate costs/reserves through the ledger.
8. **Catalogue-order commitment:** remove early-commit paths that choose the first candidate solely because it appears first. Add normalized candidate ordering and seeded, explainable selection.
9. **Arbitrary brute-force patterns:** remove repeated reconstruction of identical candidates, dead-end retries, redundant analysis, and arbitrary limits treated as biological facts. Keep operational safeguards only when they report exhaustion honestly.
10. **Duplicate construction entry points:** migrate all runtime, test, benchmark, and utility callers to one physical engine with explicit policies, then delete obsolete constructors and temporary adapters.
11. **Obsolete tests:** rewrite tests that assert old fixed size, prescribed topology, or retired reproduction rules. Do not delete valid physical requirements merely because old tests fail; retain and update tests for bonded cavities, acquisition, nonpenetration, bonding, and accounting.
12. **Obsolete documentation:** mark all prior constructor plans as superseded and remove contradictory instructions once their historical context is no longer needed.

Remove old code only after its callers and dependent invariants have been mapped. Do not remove the geometry or chemistry libraries as part of constructor cleanup.

## 12. Migration sequence

### Phase 1 — Source-of-truth audit
Identify the intended source branch; inventory constructor, simulation integration, geometry and chemistry libraries, bonding, cavity analysis, acquisition, and energy ledger. Map callers and duplicate authorities. Record what is a specification requirement versus an artifact of the old architecture.

**Deliverable:** dependency map and removal checklist.

### Phase 2 — Independent library validation
On the approved GitHub practice branch, validate representative records, contact-feature and scale coverage, candidate lookup, rejection of invalid candidates, catalogue-order independence, schema/version consistency, and test isolation.

**Deliverable:** library quality/coverage report and repeatable tests.

### Phase 3 — Candidate and validation interfaces
Implement narrow interfaces for geometry queries, chemistry eligibility, contextual validation, energy/accounting, atomic commit/rollback, and rejection diagnostics.

**Deliverable:** candidates can be proposed and rejected without mutating the organism.

### Phase 4 — Transactional growth
Prove atomic commits and rollback, bond/geometry consistency, and ledger reconciliation before full organism construction.

**Deliverable:** valid sequences of changes preserve invariants; failed changes leave state unchanged.

### Phase 5 — Genome milestone
Implement blueprint-free growth using the real cavity analyzer. Switch the objective immediately when a qualifying cavity is found.

**Deliverable:** logs identify the physical change that created the qualifying genome and confirm the policy transition.

### Phase 6 — Acquisition and viability
Integrate Water and three distinct non-Water categories through real acquisition and verify placements and accounting.

**Deliverable:** an explicit record of satisfied and outstanding viability requirements.

### Phase 7 — Comparative benchmark
Compare the prototype against the old implementation over multiple seeds, material conditions, and catalogue orderings. Measure correctness, success, time, candidate work, diversity, and failure reasons.

**Deliverable:** comparative report.

### Phase 8 — Live replacement and cleanup
Connect the new engine to live initial-organism creation, verify the simulation's physical representation and handoff, then migrate offspring policy where supported. Remove superseded code, constants, tests, and documentation. Run formatting, build, focused tests, integration tests, and the full suite; inspect the final diff.

**Deliverable:** one authoritative engine and an auditable migration record.

## 13. Benchmark and acceptance criteria

The replacement is not accepted merely because it compiles or produces one plausible organism.

### Physical correctness
Prove nonpenetration, valid bonds, analyzer-confirmed bonded cavity, physical resource acquisition, correct fluid behavior, reconciled ledgers, atomic rollback, and correct simulation handoff.

### Multiple seeds
Record success/failure per seed, failure reasons, time to cavity, time to viability, candidates generated and checked, rejection categories, and committed changes. Choose a meaningful test sample before interpreting the success rate.

### Catalogue-order independence
Reorder library records and material enumeration. Results may differ due to intentional seeded choices, but must not be systematically controlled by file order. Define deterministic candidate normalization and seeded selection.

### No hidden topology preference
Inspect outcomes across seeds and material variations. Any recurring topology must be explainable by physical constraints or an explicit policy, not a scaffold, file order, or scoring bug.

### Performance and library leverage
Measure wall-clock time, candidate count, full geometry checks, chemistry checks, cavity analyses, failed transactions, memory use where measurable, query hit rate, reusable-arrangement rate, and fallback use. The objective is fewer unnecessary decisions and earlier recognition of success.

### Simulation integration
Verify physical materials, bonds, positions, genome result, resources, and energy survive handoff and that ordinary movement, acquisition, processing, transformation, and reproduction can operate on the organism.

## 14. Failure reporting

Use explicit categories: no valid growth opportunity, no compatible geometry candidate, no permitted chemical connection, collision with existing structure, no feasible enclosure route found, blocked resource acquisition, insufficient material/energy, operational search budget exhausted, final invariant failure, or missing/unusable library coverage.

Distinguish proven impossibility under checked conditions from failure to find a solution within the available search effort. Never report the latter as proof that no viable organism exists.

## 15. Risks and mitigations

- **Greedy growth traps itself:** use bounded feasibility lookahead and maintain alternatives where useful.
- **Incomplete or stale catalogue:** validate records, track coverage, and retain a correct fallback.
- **Geometry/chemistry disagreement:** test cross-library combinations and resolve conflicts in authoritative definitions.
- **Scoring favors one body plan:** keep scoring explainable; vary seeds, catalogue order, and materials.
- **Whole-structure validation is expensive:** use spatial indexes without weakening collision completeness.
- **Two constructors remain active:** map callers, migrate one entry point, and remove obsolete paths after acceptance.
- **Focused tests hide integration failures:** test library, transaction, constructor, handoff, and full-suite layers.

## 16. Implementation status and source discipline

This plan is approved architecture, not proof of implementation. Source inspection, build success, focused tests, and full end-to-end viability are separate evidence levels. Report exactly which checks ran and which did not.

The repository has used separate branches for the geometry data and Rust source. Confirm the intended source branch and reconcile library version/schema before integration. The GitHub geometry library is the practice environment; do not alter the production library or merge the replacement merely because a prototype appears promising.

## 17. Definition of done

- [ ] Geometry and chemistry schemas, coverage, and authoritative rules audited.
- [ ] Library queries reuse valid arrangements without treating samples as universally valid.
- [ ] Candidate validation uses authoritative physical rules.
- [ ] No fixed piece-count viability target.
- [ ] No required final topology.
- [ ] Candidate commits are atomic and fully accounted for.
- [ ] Genome qualification comes from the realized structure and actual analyzer.
- [ ] Enclosure stops being a goal immediately after qualification.
- [ ] Required resources are physically acquired.
- [ ] Multiple deterministic seeds and catalogue-order variations tested.
- [ ] Hidden topology bias investigated.
- [ ] Energy and material ledgers reconcile.
- [ ] Simulation handoff and downstream behavior verified.
- [ ] Initial and offspring policies share the physical engine.
- [ ] Superseded constructor paths, duplicate authorities, obsolete tests, and conflicting documentation removed.
- [ ] Formatting, build, focused, integration, and full-suite results recorded.
- [ ] Final code diff and migration record reviewed before production integration.

## 18. Final design rule

**Use the libraries to find physically possible next steps, authoritative rules to decide which steps are valid, and biological milestones to decide when construction is finished.**

The first implementation task is the library/dependency audit, followed by a small transactional growth prototype on the practice branch. Replace live construction only after the prototype demonstrates physical correctness, real library reuse, and repeatable viable construction without a prescribed shape or fixed-size assembly.

---


### Geometry-library reset follow-up — 2026-10-10

The generated geometry data files were intentionally deleted from the current `main` and `integration/unified-source-and-catalogue` branch heads because the old snapshot was judged obsolete. Source generation rules and `src/bob/library.rs` remain on the integration branch. No replacement data was generated. Source inspection indicates an empty geometry root can be opened without seeding it, but that behavior still needs an isolated non-generative test; do not use a worker run as a smoke test. The constructor does not currently load Bob, so the next critical bridge is a tested query that returns useful placement suggestions while leaving all physical validation to the live runtime. See [the updated stabilization audit](docs/repository-stabilization-audit.md) for findings and ordered next steps.


### Bob lookup correction and current verification — 2026-10-10

A source inconsistency was found in rigid-edge lookup: generated candidate rotations were stored in the formation frame but compared against live candidate-minus-anchor rotations. The resolver now converts stored rotations to the anchor-relative frame, and the regression test for a rotated anchor passes. An isolated empty-store test also passes without seeding any geometry. CI's worker-generating smoke step has been replaced with that non-generative test; see [the latest Rust run](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38060589759) and [the stabilization audit](docs/repository-stabilization-audit.md). The full suite remains failing (**264 passed, 75 failed, 1 ignored**) and strict Clippy remains red. Bob still does not supply actual placement suggestions to the live constructor, so this is a lookup correctness fix, not completion of constructor integration.


### Rigid-edge interval correction — 2026-10-10

The persisted rigid-edge interval now means the normalized exposed interval on the anchor edge where a contact point may lie. The generator stores that interval directly, and loader/insertion validation enforce `[0, 1]`. The geometry-library schema is now version 2 so old generated records cannot be silently reinterpreted; the separate live chemistry interface-key format remains `live-v1`. The new interval, rotated-anchor, and empty-root tests passed on the preceding CI run; the latest run after decoupling the version constants is pending. Bob's canonical projection keys now omit formation-context constituent indices, with regression assertions that equivalent local interfaces collapse. Bob still needs a constructor-facing API that returns actual placement suggestions. See [the detailed interval audit](docs/repository-stabilization-audit.md).


The latest source follow-up also removes `anchor_constituent` from rigid-edge, rigid-point, and rigid-vertex canonical projection keys. It is a formation-context index, not local interface geometry, so retaining it could falsely turn equivalent contacts into ambiguous matches. Tests for this change are pending in CI.


### Bob placement-suggestion milestone — 2026-10-10

Bob now has an indexed `suggest_rigid_edge_placements` API that rebases persisted rigid-edge families to a live anchor pose and returns deduplicated candidate `Placement` suggestions. These are advisory only: the physical constructor must still validate full-structure nonpenetration, contact, bond transaction, acquisition, energy, and cavity requirements. This is a first integration primitive, not yet wired into the constructor; edge-edge is the only suggestion class implemented, and interval-wide placement exploration plus point/vertex/fluid suggestion paths remain. The regression test checks pose rebase and edge-midpoint alignment. CI is being rerun for the latest changes. See [the detailed stabilization audit](docs/repository-stabilization-audit.md).


The placement API now also supports the empty-library bootstrap case: when no matching persisted family exists, it derives only the requested rigid-edge families in memory and returns advisory placements without writing rows or starting a geometry worker. This permits local proposal generation while the checked-in catalogue remains empty. The renamed regression test verifies that an empty store remains empty and that the resulting candidate-edge midpoint is aligned to the live anchor contact point. CI is pending for this follow-up. The API remains advisory and is not yet wired into the initial constructor.


Verification checkpoint: on commit `7bfb6f21`, the empty-library bootstrap test passed and the full suite reported **266 passed, 75 failed, 1 ignored**. The existing simulation/physics/reproduction failure backlog and strict-Clippy failures remain unresolved. The newest test-only extension covers the unequal Phosphorus trapezoid edges and is still in CI. See the [stabilization audit](docs/repository-stabilization-audit.md).


Latest verification checkpoint (2026-10-10 15:23 UTC): the CI run for `b2443356107fb39a201edf772e3045b77ae8dc82` has completed. Formatting, source-size, COMBINE architecture, Bob compact-storage, and empty-store open/reopen checks pass; the Phosphorus unequal-edge suggestion regression is included. The full suite remains **266 passed, 75 failed, 1 ignored**, and strict Clippy remains red. The primary constructor blocker is now pinned to the COMBINE bond transaction for units `19-3`: candidate evaluation succeeds, but `form_selected_bond` returns no commit result and the caller currently hides the internal rejection reason. Diagnose that transaction before changing constructor geometry or wiring in Bob suggestions. The fixed scaffold is still active; the replacement constructor is not complete. The catalogue remains empty and no worker was run. See the [detailed stabilization audit](docs/repository-stabilization-audit.md).
