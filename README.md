# EvoSim

## Authoritative Development Guidelines

> **This document defines the currently approved design and implementation direction for EvoSim.**
>
> **Any deviation from the rules, architecture, terminology, authorities, or behavioral requirements defined here must be explicitly approved by the project owner before implementation.**
>
> When a requirement is unclear, incomplete, or not yet designed, **do not invent an implementation or make a design decision on behalf of the project. Identify the ambiguity and obtain approval.**
>
> This document supersedes assumptions, convenience implementations, inferred lifecycle rules, and previously introduced mechanisms that conflict with the rules below.

---

# 1. Project Purpose

EvoSim is an open-ended evolutionary organism simulation.

It is **not** a conventional game built around predetermined organism classes, roles, progression stages, or numerical advancement.

The objective is to create a physical environment in which organisms can emerge from relatively simple underlying rules.

The intended long-term behavior includes the possibility of:

- single cells,
- increasingly complex organisms,
- multicellular organization,
- cooperation,
- large organisms,
- structures analogous to plants and animals,
- reproduction,
- death,
- decomposition,
- and evolutionary divergence.

These outcomes must emerge from the underlying mechanisms rather than being explicitly assigned as organism categories.

There must be no hard-coded evolutionary ladder such as:

> cell → creature → predator → dinosaur

or equivalent predefined organism classes.

"Dinosaur" and similar terms may be useful as conceptual descriptions of possible emergent outcomes, but they are not simulation categories.

# 2. Fundamental Simulation Entities

The fundamental entities of EvoSim are:

1. **Environment**
2. **Organisms**
3. **Resources**

There should not be an additional hidden hierarchy of fundamental biological entities.

Organism behavior emerges from:

> **genome + internal state + environment + physical interactions**

No hard-coded predator, prey, scavenger, herbivore, social animal, colony, or equivalent behavioral role should become a fundamental simulation concept.

# 3. Core Design Philosophy

The governing design principle is:

> **Calculate the minimum amount of information necessary to produce the desired emergent behavior.**

Mechanisms should not be added solely because they appear realistic in isolation.

Whenever possible, a single mechanism should produce multiple useful emergent consequences.

For example, physical structure should simultaneously provide information about:

- what the organism is made of,
- its geometry,
- its connectivity,
- its available internal space,
- its storage,
- its ability to grow,
- its ability to repair itself,
- and its physical interactions.

The simulation should avoid creating separate variables to represent information that can already be derived from authoritative physical state.

# 4. No Unapproved Design Decisions

This is a formal project rule.

## 4.1 Implementation is not permission to design

Code must implement established EvoSim rules.

Code must **not** establish new rules merely because an implementation requires a choice.

If the specification does not determine what should happen, the correct response is:

1. identify the missing decision,
2. explain the available implementation implications,
3. request approval,
4. then implement the approved behavior.

Do not silently select a lifecycle rule, threshold, biological mechanism, geometry rule, energy rule, or architectural authority.

## 4.2 Explicit approval is required for deviations

Any proposed deviation from this document requires explicit approval from the project owner.

This includes, but is not limited to:

- new lifecycle stages,
- new maturation mechanisms,
- age-based behavior,
- new biological abstractions,
- new resource semantics,
- new geometry rules,
- new organism classes,
- new body-plan authorities,
- new genome definitions,
- new reproductive requirements,
- new energy rules,
- new damage rules,
- new environmental rules,
- new thresholds,
- new formulas,
- new decision mechanisms,
- or replacing an established authority with another mechanism.

A technically convenient implementation is **not** sufficient justification for changing the design.

# 5. Organisms and Physical Authority

The physical structure of an organism is authoritative.

The simulation should not maintain a separate abstract representation that contradicts physical reality.

The long-term architectural direction is:

> **Genome / blueprint specifies intended structure → construction system finds a physically valid realization → physical graph owns what actually exists.**

The blueprint describes what the organism is attempting to realize.

The physical structure determines what has actually been realized.

If the two disagree, the physical realization is not automatically rewritten merely because the blueprint says it should exist.

# 6. Resources and Physical Geometry

Resources are physical objects/materials.

All base resources have their assigned physical representations.

The resource geometry already established for EvoSim must be preserved.

## 6.1 Rigid shapes

Rigid resource shapes are physical objects.

They:

- cannot bend,
- cannot deform arbitrarily,
- cannot break internally merely because they are shapes,
- retain their physical geometry,
- and interact through their actual connection points and geometry.

The base resource shapes are equilateral where applicable to their assigned geometric representation.

Corners are connection points for the relevant rigid geometric forms.

## 6.2 Phosphorus

Phosphorus is intentionally represented by its **rigid L-shaped geometry**.

The L shape is not an error or temporary approximation.

The physical L geometry is authoritative.

The geometry must therefore be preserved during acquisition, storage, construction, COMBINE, BREAK, organism realization, and decomposition.

## 6.3 Water

Water is intentionally different in physical behavior.

Water is a **fluid** resource with a default circular representation.

The circle is the default shape used to represent water; it does not mean that water is fundamentally a rigid circular solid.

Water's fluid behavior must be retained.

Its physical connection capacity is determined by its available volume and physical circumstances rather than by an arbitrary fixed authored number of connection points.

The established Water behavior must not be replaced by an arbitrary rigid-shape abstraction.

# 7. Material Composition

Material is represented by its actual composition and internal physical structure.

A composite material retains:

- its constituents,
- its internal relationships,
- its physical geometry,
- and its bonds.

A standalone boolean such as `bonded` must not become the ultimate authority for whether material is structurally bonded.

Bonded state should be derivable from the actual composition and physical structure.

There must not be a mutable `energy_content` value acting as a substitute for physical/energetic state.

# 8. Potential Energy

Potential energy belongs to resource types and represents their absolute maximum potential energy.

The established resource scale is:

| Resource | Potential Energy |
|---|---:|
| Water | 0 |
| Nitrogen | 1 |
| Phosphorus | 2 |
| Carbon | 3 |
| Sulfur | 4 |
| Hydrogen | 5 |
| Methane | 6 |

Potential energy is not a mutable `energy_content` reservoir.

For composite material, potential energy is derived from its constituent resources according to the established material model.

Energy is emergent from physical/material transformations rather than being treated as a fundamental physical substance stored in organisms.

# 9. Energy Conservation

EvoSim must maintain an energy ledger.

Energy transactions must account for the relevant changes among:

- usable energy,
- structural/material changes,
- released potential,
- and heat dissipation.

The accounting system exists to prevent energy from being created or destroyed accidentally by implementation details.

The energy ledger is an accounting authority.

It is not permission to introduce a biological "energy battery" or universal storage capacity that has not been specified.

# 10. The Genome

There is **no single predefined genome core**.

In particular:

> There is no authoritative four-Nitrogen-unit genome.

Any implementation that treats a fixed collection such as four Nitrogen units as the universal genome core is legacy/incorrect authority.

The genome is defined through its actual physical construction.

A valid genome is a **combined resource structure forming the required genome cavity**.

The genome is what distinguishes life from non-life.

The physical genome is therefore central to organism identity and lifecycle.

# 11. Genome Cavity

The established genome requirement is:

> A valid genome is a combined resource structure containing an empty cavity large enough to fit three joined Carbon resources.

The exact physical realization of that requirement must be respected.

The requirement is not merely an area calculation.

A numerical approximation may be useful internally, but it must not replace the actual physical requirement where the physical geometry can determine the result.

# 12. Organism Membership

Physical graph connectivity is the authority for structural organism membership.

A physical constituent is **structurally part of the organism** when it has a valid bonded path to the genome, subject to any material-specific structural qualification defined below.

The genome itself is composed of non-fluid material. Therefore, a direct genome-to-Water bond is a valid structural connection.

The system must not maintain a separate mutable structural-membership flag that can disagree with the physical graph.

The physical graph must be able to answer:

> What physical constituents are structurally connected to this organism's genome?

Disconnected material is not structural merely because it remains inside the organism boundary.

## 12.1 Structural Water

Water is a normal physical material in the same connectivity system as every other material. It has **no special acquisition mechanism** and does not receive a special energetic bonus merely because it is Water.

Water does have one structural qualification because Water is fluid:

> **Every Water constituent that is classified as structural must have a genome-connected bonded path whose connected component contains at least one non-fluid constituent.

The genome counts as a non-fluid constituent.

Therefore Water-Water connectivity can carry structural membership onward when the same
connected component is anchored by non-fluid material. A Water chain does not become a
separate organism merely because its immediate neighbor is Water.

The structural predicate is:

> `structural(Water) = genome_connected(Water) AND component_contains_nonfluid(Water)`

For non-Water material, structural membership is determined by genome connectivity under the normal physical graph rules.

## 12.2 Internal Storage

**Storage means physical material that is completely enclosed by the organism boundary but is not structurally connected to the genome.**

Storage is therefore a derived physical classification, not a separate kind of material.

A stored object:

- already physically exists,
- remains the same physical material instance,
- retains its constituents,
- retains its bonds,
- retains its geometry and placement,
- and may be manipulated by the organism's physical transformation machinery.

A constituent that is connected to a larger physical component that still extends outside the organism is **not** converted into storage merely because part of it lies inside the boundary.

The complete connected physical component remains intact and boundary-constrained until an actual physical operation changes its connectivity.

## 12.3 Physical Containment and Acquisition — CURRENT AUTHORITY

There is **no ACQUIRE action**.

Acquisition is a physical consequence of material entering the organism's boundary. It is not a semantic transfer operation and it is not a special behavior available to the organism.

The authoritative sequence is:

`environmental physical material → physical movement/overlap → boundary interaction → containment classification → organism transformation`

The key rule is:

> **A physical material constituent becomes available to the organism when it is physically inside the organism's realized outer boundary.**

This applies equally to Water, atomic resources, composite material, and any other physically realized material.

### 12.3.1 Boundary Crossing Never Changes Connectivity

Crossing the organism boundary is **not** BREAK.

Crossing the organism boundary is **not** COMBINE.

Crossing the organism boundary does **not** create, destroy, or rearrange physical bonds.

If a connected physical component is entirely inside the organism, it may become internally stored or structural according to its graph relationship to the genome.

If a connected physical component straddles the boundary, the **entire connected component remains intact and constrained**. The simulation must not split off the inside constituents simply because they crossed the membrane.

If the organism needs that material separated from the outside portion, an actual physical transformation such as BREAK must remove the relevant bond.

This is a critical invariant:

> **Membrane crossing changes physical location/containment, never physical connectivity.**

### 12.3.2 No Abstract-to-Physical Acquisition

The organism must never acquire a material description and then manufacture a new physical object from that description.

There is no intermediate "acquired material" representation.

The authoritative object is always the existing physical material instance.

Transfers and storage must preserve, where applicable:

- constituent identity,
- quantity,
- internal relationships,
- bonds,
- geometry,
- relative placement,
- orientation,
- physical state,
- and other information required to preserve physical identity.

The environment grid may be used as a spatial index for finding candidate material, but the grid is not the authority for containment, geometry, connectivity, or physical identity.

## 12.4 Current Acquisition/Storage Lifecycle

The current correct material lifecycle is:

`ENVIRONMENT
  → physical movement/overlap
  → boundary interaction
      → fully outside: remains environmental
      → straddling boundary: remains one intact, constrained physical component
      → fully enclosed: becomes internally available
  → if genome-connected and structurally qualified: structural/living
  → if fully enclosed and not structural: stored/nonliving
  → COMBINE: creates one physical bond
  → BREAK: removes one physical bond
  → EXPEL: moves an eligible stored physical component outward
`

Only physical connectivity mutations change the bond graph:

- **COMBINE creates bonds.**
- **BREAK removes bonds.**
- **Movement changes positions.**
- **Containment changes accessibility/classification.**
- **EXPEL changes location/ownership.**

No one of these operations may silently perform another operation.

## 12.5 Organism-Directed EXPEL

EXPEL is an explicit organism action.

EXPEL operates only on an existing, fully enclosed, **nonstructural physical component**.

It must not reconstruct the material from composition.

The selected material retains its:

- constituents,
- internal relationships,
- bonds,
- geometry,
- relative placement,
- orientation,
- physical state,
- and physical identity.

EXPEL does not silently BREAK, deform, dissolve, or otherwise transform material merely to make expulsion possible.

After successful expulsion, the existing physical material becomes owned by the active environmental field. The environment grid only indexes the resulting physical realization.

Structural material is not eligible for EXPEL.

If valid physical placement outside the organism cannot be achieved under the existing geometry and field topology, EXPEL fails without consuming or reconstructing the material.

## 12.6 Implementation Authority

The implementation must derive structural/storage status from authoritative physical state rather than storing independent mutable flags.

The intended authority chain is:

> **physical geometry + physical containment + physical bond graph + genome identity → structural/storage classification**

The following are prohibited as substitutes for that authority:

- abstract acquired-material objects,
- logical material stored inside organisms,
- membrane-crossing bond deletion,
- membrane-crossing bond creation,
- special Water acquisition rules,
- Water-specific environmental energy bonuses,
- arbitrary storage flags,
- or composition-only reconstruction of physical material.

# 13. Initial Seed Cell

The first organism must contain a genome.

The initial organism must have enough physical structure to:

1. fully encompass the genome,
2. provide the required genome cavity,
3. exist as a physical organism,
4. and have sufficient internal space to contain physical material from the environment.

The seed is therefore not simply a data structure marked "alive."

It must be an actual physically realized organism.

# 14. Current Acquisition and Water Implementation Plan

This section is the **current approved implementation plan** for the physical acquisition, storage, structural-membership, Water, COMBINE, BREAK, EXPEL, reproduction, and death/recycling systems.

It is subordinate to the physical rules in Sections 12–13 and is the implementation roadmap for bringing existing code into conformance with those rules.

## 14.1 Phase 1 — Establish Physical Graph Authority

In `src/structure.rs`, make the physical constituent graph the explicit authority for structural membership.

The graph must provide authoritative queries for:

- genome-connected constituents,
- direct physical neighbors,
- whether a direct neighbor is non-fluid,
- whether a constituent is structurally qualified,
- and which connected components result after BREAK.

The Water rule is:

> `structural(Water) = genome_connected(Water) AND has_direct_nonfluid_neighbor(Water)`

The genome is non-fluid and therefore qualifies as the required direct neighbor.

Do not introduce mutable `is_structural` or equivalent flags that can become stale.

## 14.2 Phase 2 — Eliminate Logical Material From Organism Storage

In `src/material_storage.rs`, organism storage contains **physical material only**.

`StoredMaterial` has no logical-material variant. All storage operations operate on existing realized `PhysicalMaterial` instances.

Abstract `Material` values may still exist in the genome and environmental aggregate field because those are recipes/aggregate environmental stock, but they are never inserted into organism storage.

Callers that currently request abstract `Material` values from storage must be audited and refactored rather than recreating physical material from descriptions.

This establishes the invariant:

> **No logical material may enter, remain in, or be generated inside organism storage.**

## 14.3 Phase 3 — Correct Boundary Containment

Audit `src/environment.rs` and `src/simulation.rs` so containment never mutates physical connectivity.

When evaluating an environmental physical component:

1. If it is fully outside, it remains environmental.
2. If the connected component straddles the organism boundary, the **entire component remains intact and constrained**.
3. If the complete connected component is inside, it becomes available to the organism.
4. Once fully enclosed, it is classified as structural or stored according to the physical graph rules.

The containment operation must never delete cross-boundary bonds.

A component may become separated only through an actual physical connectivity operation such as BREAK.

The environment's spatial grid remains an index, not physical authority.

## 14.4 Phase 4 — Remove Water-Specific Energetic Modifiers

Audit `src/combine.rs`, `src/combine_runtime.rs`, `src/resources.rs`, `src/transformation.rs`, and `src/decomposition.rs` for Water-specific energy or reactivity modifiers.

Remove rules in which environmental Water changes the energetic cost or yield of an operation merely by being nearby or present as a field quantity.

Water participates exactly like other physical material through its own physical properties when it is actually involved in a transformation.

There is no Water acquisition bonus and no Water-specific "medium energy" rule.

## 14.5 Phase 5 — Derive Classification After COMBINE and BREAK

COMBINE creates one physical bond.

BREAK removes one physical bond.

After the connectivity mutation, structural/storage classification must be derived from the resulting physical graph and containment.

Examples:

- stored Carbon + structural Carbon → the new bond can make the Carbon component genome-connected and therefore structural;
- stored Carbon + stored Hydrogen → the bonded component remains nonstructural if it is not genome-connected;
- stored Water + structural Carbon → the Water is structural only when its direct-neighbor rule is satisfied;
- stored Water + genome → Water is structural because the genome is a non-fluid direct neighbor;
- BREAK may turn previously structural material into stored material when a resulting component is no longer structurally connected to the genome.

No classification flag should be manually toggled as a side effect of these operations.

## 14.6 Phase 6 — Enforce EXPEL Eligibility

EXPEL may operate only on a fully enclosed, nonstructural, physical component.

It must:

- preserve the existing physical identity,
- preserve bonds within the expelled component,
- move the actual geometry,
- and transfer ownership back to the environmental field.

It must not:

- reconstruct material,
- silently BREAK material,
- silently COMBINE material,
- deform material,
- or expel structural material.

## 14.7 Phase 7 — Refactor Reproduction Around Physical Material

Audit `src/reproduction.rs` so offspring construction uses actual physical stored material.

Remove logical-material shortcuts in which an abstract `Material` description is converted into newly created physical material.

The child must be constructed from physically realized material. Its structural membership must then be determined by its actual physical graph and genome connectivity.

No reproduction path may bypass the physical-material authority.

## 14.8 Phase 8 — Refactor Death, Recycling, and Decomposition

Audit `src/recycling.rs`, `src/decomposition.rs`, and related death paths.

Death must release the organism's existing physical material.

No death/recycling path may:

- manufacture new physical material from abstract descriptions,
- deposit `StoredMaterial::Logical`,
- or replace existing geometry/bonds with composition-only equivalents.

Released material remains physically realized and is returned to the environment through the same physical-material authority.

## 14.9 Phase 9 — Verification Matrix

The implementation is not complete until focused tests cover at minimum:

### Containment

- free Carbon outside → environmental;
- free Carbon fully inside → stored;
- bonded Carbon-Hydrogen fully inside → whole physical component stored;
- bonded Carbon-Hydrogen straddling boundary → remains one intact constrained component;
- membrane crossing → zero bond changes.

### Genome connectivity

- G-C → C structural;
- G-C-H → C and H structural;
- G-C plus enclosed unbonded H → H stored.

### Water

- G-W → Water structural;
- G-C-W → Water structural;
- G-W-C → Water structural;
- G-C-W-C-W → both Water constituents structural;
- G-W-C-W-C → both Water constituents structural;
- G-W-W → first Water structural, second Water stored/nonstructural;
- Water-Water bond alone never satisfies the structural Water requirement;
- a Water directly bonded to genome is valid because genome is non-fluid.

### COMBINE

- stored + structural physical material can become structural when the resulting graph qualifies;
- stored + stored material remains nonstructural until connected to the genome;
- Water follows the same COMBINE mechanics as every other physical material.

### BREAK

- breaking a structural/nonstructural bond reclassifies resulting components from the resulting physical graph;
- breaking a bond does not manufacture or consume material;
- a straddling environmental component cannot be split by containment alone.

### EXPEL

- stored physical component can be expelled;
- structural component cannot be expelled;
- expulsion preserves physical identity, geometry, and internal bonds.

### Lifecycle integrity

- no logical material can enter organism storage;
- no logical material can be consumed for reproduction;
- no logical material can be generated by death;
- all transferred/released matter remains physically realized.

## 14.10 Audit and Implementation Order

The implementation must follow this order:

1. physical graph authority;
2. physical-only organism storage;
3. boundary containment correction;
4. Water-specific energetic-rule removal;
5. COMBINE/BREAK post-mutation classification;
6. EXPEL eligibility and physical transfer;
7. reproduction migration;
8. death/recycling/decomposition migration;
9. focused verification matrix;
10. long-run simulation validation.

Each phase requires an audit before implementation and a second audit after implementation.

No phase may introduce a workaround that contradicts Sections 12–14.

# 15. Active Environmental Field

The active ecological field is the complete environmental material layer.

The field is persistent and stable by default. Material changes when an organism or an explicitly defined environmental event physically acts on the affected material.

There is no vent-driven environmental source or separate deep-reservoir authority.

There is no mandatory per-tick diffusion pass. Passive redistribution is not assumed unless a future environmental process explicitly requires it.

Environmental acquisition, expulsion, decomposition, and other material transfers modify only the spatially affected field state.

# 16. Material Storage

Material storage is **physical material storage**.

Organism storage contains only existing `PhysicalMaterial` instances. There is no logical/composition-only material representation inside organism storage.

Stored material must retain its physical form, including:

- constituents,
- bonds,
- geometry,
- relative placement,
- orientation,
- physical state,
- and internal structure.

Storage is a classification of a physical object, not a separate material type.

The authoritative storage rule is:

> **Completely enclosed physical material that is not structurally connected to the genome is stored/nonliving material.**

Stored material must be available to both:

- **BREAK**
- **COMBINE**

There are no Water-specific storage or acquisition exceptions.

A physical component that remains connected to material outside the organism is not converted into stored material merely because some constituents lie inside the boundary. It remains one intact, constrained physical component until an actual connectivity operation changes it.

The storage system must never:

- accept `StoredMaterial::Logical`,
- generate physical material from an abstract `Material` description,
- delete bonds because of membrane crossing,
- or use a mutable storage flag as a substitute for physical containment and graph classification.

# 17. BREAK

BREAK is a fundamental physical operation.

BREAK operates on bonded/structured material and structural bonds.

BREAK is not simply a "damage action."

It is required for:

- growth,
- construction,
- repair,
- reorganization,
- maintenance,
- creating necessary space,
- removing unnecessary structure,
- fragmentation,
- and eventual decomposition.

A physical structure may need to be broken before it can be rearranged into a configuration that better matches its blueprint.

Therefore:

> **BREAK is one of the tools of construction.**

It is not inherently destructive from the organism's perspective.

# 18. COMBINE

COMBINE is the corresponding fundamental construction operation.

COMBINE creates new physical structure and establishes physical bonds according to the established physical rules.

COMBINE must be the authority for admitting the relevant new physical bond into the construction path.

Construction must respect:

- actual geometry,
- available connection points,
- existing neighboring structure,
- physical occupancy,
- and the physical constraints of the material.

The constructor must not simply declare a bond because the blueprint requests one.

The physical realization must actually be valid.

# 19. Construction and Backtracking

The construction system must be capable of finding physically valid realizations.

Where a locally valid placement prevents the rest of a structure from being realized, the constructor must be able to reconsider earlier choices.

This is why candidate search/backtracking belongs in the construction system.

The constructor should not permanently commit to the first available placement when that placement can make the overall structure impossible.

The objective is:

> Find a physically valid realization consistent with the intended blueprint and already-realized neighborhood.

# 20. The Developmental Blueprint

The developmental blueprint is the organism's inherited **developmental-intent authority**.

It is a compact set of continuous developmental preference fields, not the physical organism and not an exact body plan.

The adult developmental blueprint represents **100% adult developmental scale**. It does not encode exact material instances, unit counts, coordinates, bonds, angles, rotations, topology, branches, or silhouette.

The authoritative chain is:

> **Genome → Developmental Field Blueprint → Construction/Development Solver → Physical Structure**

The physical structure remains authoritative for what actually exists.

The developmental blueprint may therefore be only partially realized because of resource availability, physical geometry, connection constraints, existing structure, environmental conditions, and developmental history.

# 21. Juvenile Developmental Realization

The juvenile possesses the **same 100% adult developmental blueprint** from birth.

It does not receive a permanently reduced or separately authored juvenile blueprint.

The approved initial juvenile realization uses approximately **40% of the preferred adult linear developmental scale**. Under the approved 2-D comparable-density scaling, this corresponds to approximately (0.40^2) of preferred adult mass as a consequence of the scale relationship; it is not a requirement for exact constituent or bond counts.

The confirmed original seed realization is retained only as a physical construction and scale-calibration baseline. It is not inherited, serialized into the genome, or used as a descendant topology authority.

The solver finds a physically valid realization through normal construction/COMBINE machinery. Developmental fields rank physically valid opportunities; they do not declare exact placements or bonds.

# 22. Juvenile's Blueprint After Birth

Once the juvenile is born, it already possesses the **100% adult developmental blueprint**.

The reduced juvenile realization is a physical developmental state, not a reduced genome blueprint.

Growth is the process by which physical realization moves toward the continuous developmental preferences represented by that adult blueprint.

# 23. Growth Is Not a Decision

**GROW is not an action that an organism chooses.**

Growth is an ongoing physical/developmental process.

However, the individual actions required to advance growth may require organism decisions.

For example, the organism may need to determine:

- whether it has the required material,
- whether additional material needs to become physically available,
- whether the required space is available,
- whether an existing bond must be broken,
- whether existing material is unnecessary,
- whether stored material needs to be reorganized,
- whether COMBINE can physically attach the next required component,
- or whether BREAK is required before construction can continue.

Therefore:

> **Growth is not a decision.**
>
> **The physical operations necessary to accomplish growth may be selected through the organism's decision process.**

# 24. Growth Uses Both BREAK and COMBINE

Growth must not be reduced to COMBINE.

A growing organism may need to:

- make additional environmental material physically available,
- store material,
- BREAK existing structure,
- free physical space,
- reorganize existing structure,
- COMBINE new material,
- remove unnecessary material,
- or perform combinations of these operations.

The next step in growth depends on the current physical realization and the blueprint.

A useful conceptual loop is:

> **Adult blueprint + current physical structure → determine what is preventing the next required structural state → select the necessary physical/environmental operation → update physical structure → reassess.**

This is fundamentally different from:

> "Growth = repeatedly call COMBINE."

# 25. Juvenile Requirements

The juvenile's fundamental requirements are:

1. **Survive**
2. **Maintain suitable energy**
3. **Construct toward adulthood**
4. **Eventually reproduce once adulthood is reached**

Growth is not a separate biological decision.

The organism does not need an invented "growth motivation" variable.

The physical/developmental state itself determines what remains necessary to reach the blueprint.

# 26. Adulthood

Adulthood is determined by **blueprint realization**.

Adulthood is **not** determined by:

- age,
- elapsed time,
- an age timer,
- accumulated reproductive readiness,
- an arbitrary energy threshold,
- or an invented maturation process.

The established adulthood condition is reaching the required match to the adult blueprint.

The existing `>= 90%` implementation is acceptable and does not need to be changed merely because the conceptual wording is ">90%."

The important authority is blueprint match.

# 27. No Age-Based Lifecycle

Age-based lifecycle mechanics are not part of EvoSim.

Age was an implementation decision introduced during development and was not part of the intended design.

It must not control:

- maturation,
- adulthood,
- reproduction,
- stress thresholds,
- developmental progress,
- or lifecycle transitions.

Age-based lifecycle mechanisms should be removed rather than repurposed as hidden authority.

If a future implementation needs elapsed time for a separately approved purpose, that purpose must be explicitly defined and approved.

# 28. Adult Transition

Once the organism reaches adulthood through the established blueprint-match requirement:

- it is considered adult,
- reproduction may begin,
- and reproductive activity becomes the primary priority.

There is no separate invented reproductive-readiness accumulation stage between adulthood and reproduction.

# 29. Reproduction

Reproduction is physically related to construction but is distinct from ordinary juvenile growth.

Reproduction begins with a **single resource designated as the anchor**.

The anchor resides within the adult organism's outer boundary.

The genome is constructed first from that anchor.

The offspring is then constructed within/through the adult's physical reproductive process until the juvenile satisfies the established birth requirements.

# 30. Reproductive Juvenile Target

The juvenile birth target remains an approximately **40% linear realization of preferred adult developmental scale**.

This is a developmental-scale requirement, not an exact constituent count, exact bond count, or exact body-plan diagram. The juvenile remains free to differ physically where environmental conditions, geometry, connection constraints, available material, or developmental history require it.

The construction system finds a physically valid realization through the normal physical construction and COMBINE machinery.

Any additional birth criterion must use an already established physical requirement; P6 does not invent a separate visual-similarity body-plan metric.

# 31. Juvenile Separation

Once the juvenile satisfies the established birth requirements, it separates from the adult and becomes a new life.

The new juvenile then possesses its **100% adult developmental blueprint** and proceeds through the normal developmental process.

Its physical structure at birth is the reduced developmental realization; its inherited developmental blueprint remains the full adult field representation.

# 32. Maintenance

An organism continuously incurs maintenance demand while it exists.

Maintenance demand is derived from the realized physical structural mass:

> maintenance demand = realized structural mass × maintenance coefficient

The initial implementation uses the simulation coefficient `MAINTENANCE_ENERGY_PER_MASS`.

Maintenance consumes usable energy through the energy ledger.

An organism pays as much of its maintenance demand as its available usable energy permits. Any unpaid maintenance deficit becomes stress rather than directly causing death.

Maintenance heat remains part of the common energy-transaction/stress pathway.

This maintenance rule does not create a maintenance-specific health meter, age system, lifespan, or death countdown.

The coefficient is a simulation parameter rather than inherited biological information.

# 33. Heat Stress

Every energy transaction produces some heat stress.

Stress:

- accumulates,
- dissipates over time,
- and can eventually cause structural damage.

Heat stress is therefore a consequence of energy transactions rather than an unrelated damage meter.

The energy ledger and the organism's stress state must ultimately be connected so that energy transactions cannot silently bypass heat-stress consequences.

# 34. Stress Dissipation

Accumulated stress can dissipate over time.

The existing stress-decay mechanism may remain as implementation infrastructure unless and until the project specifies a different rule.

The exact implementation must not be treated as a newly invented biological law simply because a current constant exists in code.

# 35. Stress Thresholds

When accumulated stress reaches the current stress threshold:

1. structural damage occurs,
2. the threshold is reduced,
3. additional stress accumulation can therefore make subsequent damage easier,
4. and the process can cascade.

This establishes the intended cascading failure behavior.

The lowered threshold is part of the established damage model.

# 36. Stress Damage Selection

Stress damage has an important physical distinction.

When stress causes structural damage:

- a **random eligible non-genome structural bond** is selected first,
- genome bonds are protected while eligible non-genome structural bonds remain,
- once all eligible non-genome structural bonds have been broken, genome bonds may become vulnerable.

This ordering is authoritative.

Genome-bond eligibility is derived from the existing physical genome-cavity authority. The qualifying cavity boundary identifies the physical genome bonds; Phase 5 must not infer genome membership from a fixed material recipe, unit count, or implementation-specific index.

Random selection is intentional; bond strength does not determine which eligible bond is damaged.

# 37. Genome Damage

Genome damage occurs only after the organism's non-genome structural bonds have been exhausted through stress damage.

If genome bonds subsequently break, the genome may:

- cease functioning,
- or lose/reduce capabilities,

depending on the complexity and continued viability of the remaining genome structure.

The exact capability-reduction mechanism has not yet been fully designed.

Therefore:

> **Do not invent a genome capability-degradation system.**

The physical damage ordering can be implemented independently while the capability semantics remain pending.

# 38. Death

Death occurs when the organism can no longer remain a viable organism.

The established death conditions include:

- the genome being broken,
- inability to maintain itself,
- inability to complete necessary actions/transactions,
- and the eventual consequences of structural failure.

Death is not defined by age.

There is no maximum-age lifecycle.

There is no artificial "old age" death mechanism.

# 39. Dead Organisms Remain Physical Material

When an organism dies, it becomes resource/material rather than simply disappearing.

Its physical material must remain part of the simulation.

Death does not automatically transform all material into an abstract raw-resource pool.

The carcass remains physical material and can undergo decomposition.

# 40. Decomposition

Decomposition physically dismantles dead organism structure.

Existing decomposition infrastructure may break the structure apart and eventually return constituent material to the environment.

The precise long-term ecological rules governing every possible material transformation/reentry pathway remain incompletely designed.

Where those rules have not been specified, they must not be invented.

# 41. Colonies and Cooperation

Colonies must emerge from actual organism interactions.

There is no hard-coded "colony stage."

There is no predefined social-organism class.

Cooperation must arise from organisms actually performing cooperative behaviors.

The established neighbor concept is behavior-based rather than simple physical proximity.

Two organisms are considered neighbors in the relevant cooperative sense when they are simultaneously enacting at least two distinct cooperative behaviors.

# 42. No Universal Storage or Energy Battery

There must not be a universal genome trait that simply provides:

> "storage capacity = X"

or:

> "energy battery = X"

Physical geometry should determine available space and storage wherever that is the established mechanism.

Cavities and actual structure should provide the physical basis for storage and energy handling.

# 43. Movement, Perception, and Memory

Movement, perception, directional resolution, memory, and related systems are legitimate simulation infrastructure.

They are not, however, substitutes for the physical lifecycle rules in this document.

They must not become hidden authorities for:

- maturation,
- adulthood,
- growth,
- reproduction,
- or organism identity.

Behavioral systems and physical developmental systems must remain conceptually distinct.

# 44. Environment

The environment is a **single physical material system**.

The obsolete deep-reservoir/two-compartment model is not part of EvoSim and must not be treated as an environmental authority.

The active field is the environment's spatially organized physical-material layer. It may contain individual resources, bonded composites, larger connected structures, loose material, fluid material, accumulated material, fragmented material, and other configurations produced by established physical and environmental processes.

The active field may therefore contain material in arrangements analogous to:

- bedrock,
- rocks,
- rock fragments,
- soil,
- sediment,
- loose deposits,
- accumulated formations,
- and fluid material.

These are emergent physical descriptions, not authored environmental object categories.

The environment must not introduce special fundamental types such as `Rock`, `Bedrock`, `Soil`, `Sediment`, `Food`, or `Waste` merely to represent these arrangements.

## 44.1 Environmental Physical Authority

While material exists in the environment, the environment owns its physical realization.

A physically existing composite must retain the information necessary to determine its later physical interactions, including its composition, internal relationships, geometry, and bonds where applicable.

The governing rule is:

> **A physical material object does not lose its physical identity merely because it changes location or ownership.**

The active field is therefore not merely a numerical resource distribution.

## 44.2 Environmental Structures

Environmental material may exist as individual resource units, small composites, large connected structures, or other physical configurations.

There is no fixed environmental category or scale corresponding to "rock," "soil," "sediment," or "bedrock."

Large formations and small fragments are configurations of the same underlying resources and material rules.

## 44.3 Composition Does Not Invent Geometry

Composition alone does not authorize the simulation to choose a physical arrangement for a particular environmental material instance.

If a composite physically exists in the active field, its existing physical realization is authoritative.

If an implementation has only composition and no authoritative physical realization, it must not silently invent one solely to make physical transfer possible.

Physical realization must come from an authoritative physical process.

## 44.4 Environmental Fragmentation and Accumulation

Environmental material may fragment, move, settle, accumulate, and otherwise change physical arrangement according to established environmental and physical rules.

BREAK may participate in physical fragmentation where its rules permit.

Accumulation does not require a special `soil`, `sediment`, or `rock` state.

The resulting structure emerges from composition, geometry, connectivity, physical state, cohesion, quantity, movement, deposition, and other established mechanisms.

## 44.5 Rigid and Fluid Material

Rigid environmental material retains its established geometry and interacts through actual geometry and connection points.

Fluid environmental material retains its fluid behavior.

Water remains a fluid resource with a default circular representation; the representation does not make it a rigid circular solid.

## 44.6 Environmental Material Has No Biological Role

Environmental material is not inherently food, construction material, waste, nutrient, obstacle, shelter, useful material, or useless material.

The same material may be used differently by different organisms.

Biological significance emerges from organism behavior and physical interaction rather than from an environmental role flag.

## 44.7 Spatial Representation and Performance

The active field may use grids, cells, aggregates, or other computational structures for performance.

These are spatial implementation mechanisms, not independent physical authorities.

An aggregate representation is valid only when it preserves enough information to determine the physical interactions that must be simulated at that scale.

The governing principle remains:

> **Calculate the minimum amount of information necessary to produce the desired emergent behavior.**

## 44.8 Active Environmental Field

The active field is the complete environmental material layer.

It is persistent and remains stable unless an organism or explicitly defined environmental event changes it.

Vents, a deep-reservoir authority, and mandatory per-tick diffusion are not part of the current environment model.

# 45. Material Transformation

BREAK and COMBINE are physical transformations.

Neither should be assumed to be universally energy-positive or universally energy-negative.

The resource/material system must permit a meaningful distribution of energetic outcomes.

The existing material model should remain grounded in constituent potential energy and actual physical transformation.

# 46. Complexity

The current complexity hypothesis is:

> **n × log₂(n)**

where applicable to the established complexity model.

The current two-component duration is:

> **2 ticks**

These are project-level established parameters/hypotheses and should not be silently replaced.

# 47. Physical Construction Authority

The intended construction architecture is:

### Genome / Blueprint

Defines:

- intended composition,
- internal relationships,
- layout,
- and relevant anchors.

### Construction Solver

Determines:

- candidate physical placements,
- valid connection configurations,
- spatial feasibility,
- and a physically valid realization as close as possible to the blueprint.

### Physical Graph

Owns the actual:

- constituents,
- placements,
- geometry,
- bonds,
- and connectivity.

The physical graph is the final authority for what actually exists.

# 48. No Duplicate Structural Authorities

EvoSim should not have multiple competing systems independently deciding what the same physical bond or structure means.

In particular:

- blueprint realization should not independently create structural bonds after COMBINE has already admitted them,
- abstract architecture should not override physical structure,
- genome metadata should not contradict the physical genome,
- and convenience representations should not silently become authoritative.

The migration toward a unified construction path must continue in this direction.

# 49. Decision System

The organism decision system remains appropriate for choosing among legitimate actions where a choice is actually required.

However, it must not be used to invent a "growth action."

The organism can decide:


- whether to break,
- whether to combine,
- whether to rearrange,
- whether to perform another permitted behavior,

based on its state and environment.

But "grow" itself is the resulting developmental process.

The distinction is:

> **Growth is a goal/state transition.**
>
> **BREAK, COMBINE, and related physical operations are mechanisms that may be selected to accomplish it.**

# 50. Experimental vs Authoritative Rules

Not every existing implementation value is automatically a permanent EvoSim rule.

A value must be treated according to its actual status.

### Authoritative

Explicitly established by the project owner.

### Experimental

A provisional implementation that has been explicitly permitted but has not been finalized.

### Pending Design

A behavior that has been identified but whose exact mechanism has not yet been designed.

### Legacy / Incorrect Authority

An implementation that exists in the code but conflicts with the established project direction.

### Infrastructure

A technical mechanism that supports the simulation but does not define biological semantics.

This distinction must be maintained throughout development.

# 51. What Must Never Be Introduced Without Approval

The following are specifically prohibited unless explicitly approved:

- age-based maturation,
- age-based adulthood,
- age-based reproduction,
- age-based death,
- reproductive-readiness timers,
- invented genome cores,
- fixed four-Nitrogen genome structures,
- hard-coded organism classes,
- predator/prey role systems,
- arbitrary growth stages,
- a universal growth action,
- universal storage-capacity genes,
- universal energy batteries,
- arbitrary biological motivations,
- blueprint structures treated as physical reality,
- abstract structure overriding physical geometry,
- formulas presented as established biology when they are merely implementation guesses,
- new thresholds,
- new resource properties,
- new lifecycle transitions,
- obsolete reservoir-based environmental authority,
- or any other mechanism not already approved.

# 52. Handling Ambiguity

When implementation encounters a requirement that this document does not settle:

**Do not guess.**

The correct development process is:

1. Identify exactly what is unspecified.
2. Explain why the implementation requires a decision.
3. Present the relevant existing architecture and consequences.
4. Ask the project owner for the decision.
5. Implement only after approval.

The absence of a specification is not permission to create one.

# 53. Implementation Priority

The immediate architectural objective is to make one initialized cell capable of progressing through the actual physical lifecycle without violating the established rules:

> **A physically authoritative organism with a genome can make environmental material physically available through containment, incorporate material through physically valid COMBINE operations, use BREAK where necessary, maintain itself, grow toward its adult blueprint, reproduce through the established anchor/blueprint process, and eventually die and decompose without violating material or energy conservation.**

The system should be developed toward this objective incrementally.

The priority is not adding more species, behaviors, visual effects, or complexity before this fundamental loop works.

# 54. Development Standard

Every significant implementation change should be evaluated against four questions:

### 1. Is this explicitly specified?

If yes, implement it.

### 2. Is this already implemented correctly?

If yes, do not replace it unnecessarily.

### 3. Is this implementation legacy or contradictory?

If yes, identify it as such and migrate it carefully.

### 4. Is the behavior unspecified?

If yes, stop at the boundary and obtain approval.

# 55. Final Authority

The project owner's explicitly approved design decisions are the final authority for EvoSim.

Code is not authority merely because it already exists.

Tests are not authority merely because they currently pass.

A legacy abstraction is not authority merely because multiple systems depend on it.

A convenient implementation is not authority merely because it is easier to code.

The purpose of the migration is to make the implementation conform to the intended EvoSim model—not to redefine the model around whatever mechanisms happen to exist in the repository.

# 56. Non-Negotiable Principle

The central rule for future development is:

> **Implement the design that has been approved. Do not invent the design.**

If a mechanism is necessary to implement an approved behavior but its exact form has not yet been determined, that mechanism must be discussed and approved before becoming part of EvoSim's architecture.

**No deviation from this document is authorized without explicit approval from the project owner.**








amendment for blueprint design: 

EvoSim — Developmental Field Blueprint Specification

Status: Approved
Authority: Master Technical Specification 2.4
Purpose: Define the genome blueprint as developmental intent rather than an explicit construction diagram.

---

1. Core Principle

The EvoSim blueprint is not a diagram of an organism.

It describes the developmental tendencies encoded by a genome: where material is favored, where structure is favored, how connectivity is favored, and the approximate scale toward which development tends.

The blueprint does not prescribe the exact physical organism.

The authoritative developmental chain is:

Genome → Developmental Field Blueprint → Construction / Development Solver → Physical Organism Structure

The physical organism structure is the final authority.

---

2. What the Blueprint Represents

The blueprint represents a compact set of continuous developmental fields.

Initial fields are:

1. Material-composition fields
2. Structural-density field
3. Connectivity field
4. Preferred developmental mass

These are preferences, not commands.

A field may influence where the construction solver attempts to place or develop material, but it cannot require an impossible realization.

---

3. What the Blueprint Must NOT Specify

The blueprint must not directly encode:

- Exact material instances
- Exact unit count
- Exact coordinates
- Exact bonds
- Exact connection-point assignments
- Exact angles
- Exact rotations
- Exact silhouette
- Exact topology
- Explicit branches
- Explicit organs
- Species-specific body plans
- Predator/prey/scavenger roles
- A fixed evolutionary progression
- A guaranteed final structure

If the blueprint uniquely determines these properties, it has reverted to the old explicit-diagram model and violates this specification.

---

4. Developmental Coordinate System

Blueprint fields exist in an organism-local developmental coordinate system.

Origin

The developmental origin is anchored to the organism's initial seed.

This origin persists throughout development.

Translation

When the organism moves through the world, its developmental coordinate frame moves with it.

Developmental coordinates therefore remain stable relative to the organism rather than the environment.

Re-centering

The developmental frame must not continuously re-center on:

- Center of mass
- Current bounding box
- Current geometric center
- Newly grown material

Continuous re-centering would change the meaning of existing developmental fields as the organism grows and could create feedback in which growth changes the developmental target that caused the growth.

Orientation

The initial local axes are established when the organism is initialized.

The blueprint does not require a permanently world-aligned body plan.

Future orientation mechanisms may evolve if required, but they must not introduce a fixed world-space body plan.

---

5. Field Representation

Fields must be represented as compact continuous parameterizations, not literal high-resolution bitmap or voxel grids.

The initial implementation should use a small number of radial influences.

Each influence may contain:

- Local center
- Radius
- Strength
- Falloff

Initial fields may use a small number of radial influences. The exact influence count is an **experimental representation parameter**, not a permanent biological law.

This provides a compact genome representation while allowing nonuniform developmental tendencies. The approved initial radial equation is Gaussian; influence centers, widths, strengths, and numerical bounds are experimental until explicitly approved.

Deferred complexity

The initial implementation should not add aspect ratio, ellipse parameters, arbitrary orientation, or high-resolution spatial maps unless experimentation demonstrates that the simpler representation cannot produce the desired emergent behavior.

Additional parameters should be introduced only when justified by observed developmental limitations.

---

6. Material-Composition Fields

Each material-composition field represents a relative developmental preference for a material.

The field does not command the organism to contain a particular quantity of that material.

For example, a Carbon field may indicate that Carbon-rich development is favored in one region and less favored in another.

The solver remains free to produce a different composition when constrained by:

- Available environmental material
- Physical geometry
- Existing structure
- Connection constraints
- Developmental history
- Other physical limitations

The resource catalog remains authoritative for the physical properties of each resource.

Blueprint mutation changes developmental preference; it does not mutate the underlying physical properties of resources.

---

7. Structural-Density Field

The structural-density field indicates where the genome favors more or less physical structure.

It does not prescribe:

- The number of structural units
- Exact geometry
- Exact thickness
- Exact boundaries
- Specific branches

A high-density region means that additional structure is developmentally favored there.

The realized amount and arrangement of structure are determined by the construction process and physical constraints.

---

8. Connectivity Field

The connectivity field indicates where greater or lesser structural connectivity is developmentally favored.

It does not specify which particular units connect to which other units.

It does not prescribe individual bonds or connection-point assignments.

The construction solver uses the field as one input when selecting among physically valid structural realizations.

The actual bond graph remains owned by the physical structure.

---

9. Preferred Developmental Mass

The inherited **size-preference gene** is the source of developmental-size variation. Preferred developmental mass is derived from that gene using the approved logarithmic size mapping. The current `adult_mass` concept may remain as an implementation-facing preferred-mass value only if it is not maintained as a second independent inherited authority.

It means:

«The approximate structural mass toward which development tends.»

It does not mean:

«The mass at which an organism becomes an adult.»

Actual organism mass is always derived from the realized physical structure.

There must not be a second independent authoritative developmental-size gene or mass value alongside the size-preference authority.

Mass authority

Genome: preferred developmental mass
Physical structure: actual mass

The preferred mass is a soft developmental preference, not a hard boundary or guaranteed final mass.

Environmental scarcity, structural constraints, damage, developmental history, and other physical conditions may prevent the organism from reaching it.

Maturation

Maturation remains a separate lifecycle state.

Reaching preferred developmental mass does not, by itself, constitute maturation unless a future lifecycle specification explicitly establishes that relationship.

---

10. Construction Solver

The construction/development solver receives, at minimum:

- Genome developmental fields
- Current physical structure
- Available material
- Resource geometry
- Connection constraints
- Existing physical relationships
- Environmental constraints

The solver searches for a physically valid realization that is compatible with the developmental preferences.

It should select among possible realizations rather than simply translating blueprint coordinates directly into geometry.

---

11. Existing Structure Has Priority

Already-realized physical structure is authoritative.

Development must not arbitrarily rearrange existing units merely because another arrangement would produce a better blueprint-field match.

The blueprint influences future development.

It does not retroactively rewrite physical history.

This allows developmental history and environmental conditions to contribute to the resulting organism.

---

12. No Guaranteed Blueprint Realization

A blueprint is an intention, not a guarantee.

The final organism may differ from its genome's developmental preferences because of:

- Resource scarcity
- Unavailable material
- Physical collision
- Connection limitations
- Existing structural geometry
- Environmental conditions
- Developmental history
- Other physical constraints

This variation is intentional.

Two organisms with the same genome may therefore develop different valid structures under different conditions.

---

13. Authority Boundaries

The following authority hierarchy is mandatory:

Information| Authority
Resource physical properties| Resource catalog
Developmental preferences| Genome / blueprint
Actual material composition| Physical structure
Actual structural units| Physical structure
Actual geometry| Physical structure
Actual positions/orientations| Physical structure
Actual bonds| Physical structure
Actual connectivity| Physical structure
Actual mass| Physical structure
Environmental availability| Environment

No duplicated authoritative representation should be introduced.

The blueprint must never become a second source of truth for the organism's actual structure.

---

14. Mutation and Inheritance

Blueprint parameters are genome traits and therefore may:

- Be inherited
- Mutate
- Produce developmental variation

Mutation should modify field parameters smoothly where practical rather than replacing a developmental field with arbitrary spatial noise.

Resource properties themselves are not mutated through the blueprint.

---

15. Blueprint Fitness

The blueprint has no independent fitness score.

It contributes to organism development.

Natural selection operates on the organism and its consequences in the simulation rather than directly scoring whether a blueprint matches a predetermined body shape.

---

16. Performance Requirement

Blueprints must remain compact.

The initial implementation must not store a per-organism high-resolution spatial bitmap or voxel map.

Continuous parameterized influences are preferred because they:

- Reduce memory use
- Reduce mutation dimensionality
- Preserve smooth developmental variation
- Avoid encoding explicit body plans
- Allow environmental constraints to influence realization

---

17. Minimum Viable Blueprint

The minimum useful implementation consists of:

1. Material-composition preference
2. Structural-density preference
3. Preferred developmental mass

The architecture should support a connectivity field, but connectivity may be introduced as an active developmental input when testing demonstrates that material and density preferences alone cannot produce sufficient structural variation.

---

18. Required Success Test

A valid implementation must permit the following:

«The same blueprint can produce two different physically valid organisms when environmental conditions or developmental history differ.»

A blueprint that uniquely determines every material, bond, angle, coordinate, or structural relationship fails this specification.

---

19. Design Intent

The blueprint exists to provide the minimum amount of heritable information necessary to bias development toward repeatable tendencies while leaving physical realization to emergence.

The goal is not to encode an organism.

The goal is to encode how development tends to behave.

---

# 57. P6 Developmental Realization Mathematics — APPROVED

The P6 developmental realization model is defined by continuous developmental fields and the authoritative physical graph.

## 57.1 Material realization

For material m, C_m(p) is its developmental material-preference field and A_m(G) is the physical area occupied by that material in the authoritative organism graph G.

\[
V_{M,R}=\sum_m\int_{A_m(G)}C_m(\mathbf p)\,dA
\]

\[
V_{M,A}=\sum_m\int_{\mathbb R^2}C_m(\mathbf p)\,dA
\]

\[
R_M=V_{M,R}/V_{M,A}
\]

when V_{M,A}>0.

## 57.2 Structural-density realization

For density field D(p), let A_G be the physical area occupied by realized organism structure.

\[
V_{D,R}=\int_{A_G}D(\mathbf p)\,dA
\]

\[
V_{D,A}=\int_{\mathbb R^2}D(\mathbf p)\,dA
\]

\[
R_D=V_{D,R}/V_{D,A}
\]

when V_{D,A}>0.

Density is developmental preference. It does not prescribe unit count, thickness, topology, or a body boundary.

## 57.3 Connectivity realization

For an actual or physically admissible connection between endpoints a and b:

\[
S_K(a,b)=\frac{K(\mathbf a)+K(\mathbf b)}{2}+\lambda N(a,b)
\]

The neighborhood contribution is:

\[
N(a,b)=\frac12\left(\frac{q_a}{Q_a}+\frac{q_b}{Q_b}\right)
\]

where q is realized incident connectivity and Q is the number of physically available connection sites on the corresponding realized material/structure.

The physically admissible opportunity set is:

\[
O(G)=\{(a,b)\mid\text{one connection between exposed compatible physical sites a,b is physically admissible}\}
\]

and:

\[
E_{K,A}(G)=E_G\cup O(G)
\]

where E_G is the actual physical edge set.

Then:

\[
V_{K,R}=\sum_{e\in E_G}S_K(e)
\]

\[
V_{K,A}=\sum_{e\in E_{K,A}(G)}S_K(e)
\]

\[
R_K=V_{K,R}/V_{K,A}
\]

when V_{K,A}>0.

If V_{K,A}=0, connectivity is inactive and contributes no penalty.

The opportunity set is generated from physical geometry, connection compatibility, and construction rules. It is never generated from an inherited exact topology.

## 57.4 Overall realization

Only active realization domains participate. Define:

\[
\mathcal A=
\{M\mid V_{M,A}>0\}
\cup
\{D\mid V_{D,A}>0\}
\cup
\{K\mid V_{K,A}>0\}
\]

Then:

\[
\boxed{
R=\frac{1}{|\mathcal A|}\sum_{X\in\mathcal A}R_X
}
\]

The equal arithmetic mean is the fixed aggregation rule. No tunable realization-weight genes or realization-weight parameters are introduced.

The adulthood rule is:

\[
\boxed{R\ge0.90\Rightarrow\text{adult}}
\]

The 0.90 threshold is approved.

All developmental realization values must be calculated from continuous developmental fields plus the authoritative physical graph. An authored exact body plan, target coordinate list, or transient structural blueprint is never an adulthood authority.

For Gaussian realization fields:

\[
K_i(\mathbf p)=e^{-\|\mathbf p-\mathbf c_i\|^2/(2\sigma_i^2)}
\]

and:

\[
\int_{\mathbb R^2}K_i(\mathbf p)\,dA=2\pi\sigma_i^2
\]

Therefore finite realization denominators require finite positive sigma for every participating Gaussian influence. Infinite-width/zero-falloff fields remain experimental candidate-scoring representations and are not valid finite realization denominators.

### Parameter status

The following are **Approved equations/architecture**:

- material realization as developmental-field overlap with physical material area,
- density realization as developmental-field overlap with physical structure area,
- connectivity realization from actual graph edges versus physically admissible connection opportunities,
- active-domain arithmetic mean,
- adulthood at R >= 0.90.

The following remain **Experimental**:

- Gaussian influence count,
- influence centers,
- sigma values,
- field strengths,
- connectivity neighborhood coefficient lambda,
- candidate-selection weights,
- numerical size-preference mass bounds.

These experimental values must never be described as established biological constants.


## P6 Approved Developmental-Scale Parameterization Amendment

The approved P6 developmental-field equations are now paired with the following implementation parameterization:

- The initial representation uses **4 radial influences per developmental field**. The count of four is the approved starting representation; the influence count remains **EXPERIMENTAL** and may change after validation.
- Each Gaussian influence uses the approved form \(K_i(\mathbf p)=e^{-\|\mathbf p-\mathbf c_i\|^2/(2\sigma_i^2)}\).
- Influence width is derived from preferred developmental scale: \(\sigma_i=\alpha_i L_p\). The \(\alpha_i\) values are **EXPERIMENTAL** representation parameters.
- Preferred linear scale is derived from the confirmed-good initial seed realization only as a calibration reference: \(L_p=L_{seed}\sqrt{M_p/M_{seed}}\). This is a scale relationship, not an inherited seed body plan. The comparable 2-D mass scaling is therefore \(M_J\approx0.40^2M_p\) for the approved 40% juvenile linear realization.
- Influence centers and strengths are **EXPERIMENTAL** numerical parameters. The initial centers and strengths in the default genome are implementation starting values, not biological constants.
- Developmental-field sums are normalized by their total influence strength so field shape is not confounded with absolute amplitude.
- Material realization uses composition-weighted physical area for composite units: each constituent contributes according to its fraction of the unit's material amount rather than counting the entire composite area once per constituent.
- Connectivity uses physically available endpoint opportunities and keeps the neighborhood coefficient \(\lambda\) **EXPERIMENTAL**. Candidate-selection weights \(w_M,w_D,w_K\) are also **EXPERIMENTAL** solver parameters.
- Numerical size-preference mass bounds remain **EXPERIMENTAL**.

No experimental parameter above is an additional biological authority. Changing one changes the experiment within the approved P6 architecture.

# 58. Harmonics / World Tone Architecture — APPROVED

The organism does not use a separate authored resource-perception system.

The approved causal model is:

**World tone / environmental harmonic activity → physical material response → physical bond propagation → realized organism structure → physical genome cavity → organism memory / internal state**

The organism does not receive an abstract resource-desirability signal. Its physically realized structure is the interaction surface.

## 58.1 World tone

The world reference tone is **440 Hz**.

It is represented analytically as a spectral drive. The simulation does not generate 440 waveform cycles per tick.

## 58.2 Material response

Existing physical resource properties provide the harmonic behavior.

- Mass supplies inertia.
- Cohesion supplies the restoring/coupling tendency.
- Reactivity supplies damping and nonlinear harmonic response.
- No new damping-only resource property is introduced.

The intrinsic natural-frequency relationship uses the existing cohesion/mass relationship and is normalized against catalog baselines while remaining anchored to the 440 Hz world reference.

## 58.3 Spectral representation

Harmonic state uses a compact spectrum:

- frequency
- amplitude
- phase

The representation is intentionally bounded rather than a full waveform simulation.

Nonlinear material response may produce integer harmonic components. The exact spectrum is a consequence of realized material properties and structure.

## 58.4 Physical propagation

Physical bonds permit harmonic propagation.

The realized physical graph is authoritative for structural harmonic interaction. Blueprint fields, inherited body plans, and abstract sensor geometry are not used as harmonic propagation authorities.

Composite harmonic behavior is modeled as a compact coupled-oscillator approximation rather than a separate acoustic-fluid simulation.

## 58.5 Genome cavity

The actual qualifying genome cavity is the organism's physical harmonic reception surface.

Cavity membership comes from realized cavity geometry and its actual boundary units. No predefined genome core or blueprint-only receptor exists.

A non-qualifying realized structure has no qualifying genome harmonic memory surface.

## 58.6 Memory

Harmonic experiences use the existing organism memory architecture.

A remembered experience contains:

- spatial context
- memory strength
- the received spectral content
- the observed outcome

Cavity geometry determines memory capacity and persistence.

The inherited `memory_strength` trait controls memory formation/reinforcement strength. It does not determine cavity capacity or replace the physical cavity as authority.

Actual outcomes affect reinforcement. Learned harmonic memories are organism internal state and are not inherited genome traits.

## 58.7 Perception retirement

The legacy resource-perception architecture is retired.

The following are no longer part of the organism's behavioral authority:

- perception radius
- sensory resolution
- directional resolution
- resource-property affinity genes
- resource desirability calculations
- abstract resource observations
- resource-desirability directional vectors

Movement and later behavior must emerge from physically generated harmonic information and learned internal state rather than from a resource-value oracle.

The genome does not contain a replacement `harmonic preference` or `frequency preference` sensor gene. If harmonic differences produce behavioral differences, those differences must arise through realized physical structure and its interaction with the harmonic environment.

## 58.8 Authority rule

The governing principle is:

> **An organism does not perceive the world. Its physically realized structure interacts with the world, and whatever information that interaction produces becomes available to the organism.**

No mechanism may be introduced solely to restore the behavior previously supplied by the retired resource-perception oracle. If an exact harmonic-to-action relationship is not specified, implementation stops at the physical information boundary rather than inventing a behavioral shortcut.


# 59. Memory / Associative Spatial Map — NEW AUTHORITY

**Status: APPROVED architecture; numerical calibration values marked EXPERIMENTAL.**

This section supersedes the legacy `MemoryPoint` / `DecisionHistory` behavioral model and is the authoritative memory specification going forward. Older memory implementations, assumptions, and tests that conflict with this section are legacy and must be removed or adapted; they must not be used as design authority.

The approved model is:

**WORLD → existing resonance physics → physical genome cavity → spatially attributed perception → current situation → action → physical consequence → experience memory → future perception/action.**

Memory interprets current perception. It does not replace perception, override physics, or provide an abstract resource-value oracle.

## 59.1 Approved cognitive action boundary

The organism's behavioral action categories are:

- **MOVE**
- **PROCESS**
- **EXPEL**
- **IDLE**

Existing BREAK and COMBINE mechanisms remain physical processing mechanisms and may remain implementation-level operations beneath PROCESS.

There is **no ACQUIRE action.**

Acquisition is a physical consequence of an action, primarily movement/overlap with physical material. The organism does not independently choose to acquire material.

The authoritative learning sequence is:

**perception → action → physical consequence → memory reinforcement.**

## 59.2 Perception

The existing resonance/harmonic system remains the physical basis of perception. No separate sensory physics is introduced.

The resonance pipeline must retain enough source provenance for perception to distinguish simultaneously perceived signals spatially. A perceived signal conceptually contains:

- received spectrum,
- source/location information,
- spatial extent associated with the existing resonance geometry,
- perceived magnitude.

The organism only perceives what its realized physical resonance geometry permits it to receive.

The existing genome cavity and resonance geometry are therefore authoritative. No arbitrary perception radius is introduced.

The current four-component spectrum bound remains an **EXPERIMENTAL implementation compression limit**, not a biological law.

## 59.3 Spectral similarity

Spectral matching is continuous and uses no resource IDs or semantic categories.

For component frequencies \(f_a,f_b\):

\[
d_f=|\ln(f_a/f_b)|
\]

and the component similarity is:

\[
s_f=e^{-d_f/\sigma_f}
\]

The approved symmetric spectrum similarity is the mean of the two amplitude-weighted directional matches:

\[
S(A,B)=\frac{M(A\rightarrow B)+M(B\rightarrow A)}{2}
\]

where each directional match weights each source component by its non-negative amplitude and uses its best matching component in the target spectrum.

\[
0\le S(A,B)\le1
\]

An exact spectral match produces 1. Greater logarithmic frequency separation produces lower similarity.

### Experimental spectral calibration

\[
\boxed{\sigma_f=0.25}
\]

This is an **EXPERIMENTAL calibration value** and may be changed after behavioral validation without changing the approved architecture.

## 59.4 Memory layers

Memory is represented conceptually through three related layers rather than one generic memory point.

### Spatial memory

Stores:

- location/region,
- spatial extent,
- location association,
- location association strength/confidence,
- memory strength.

The spatial extent comes from the actual resonance region involved in the experience. There is no arbitrary fixed merge radius.

### Spectral/matter memory

Stores:

- a learned spectral pattern/prototype,
- matter/spectral association,
- matter/spectral association strength/confidence,
- memory strength.

The organism recognizes recurring matter through spectral similarity rather than an authored resource identity.

### Encounter memory

Stores the relationship between an actual experience's:

- spatial context,
- perceived spectrum,
- action,
- actual multidimensional consequence,
- memory strength.

Encounter memory prevents the organism from collapsing "this material is useful" and "this location is useful" into the same fact.

## 59.5 Independent spatial and spectral associations

Location and spectral/matter associations are learned independently from the same physical experience.

A positive material experience does not automatically make its location positive, and a positive location does not automatically make every material found there positive.

All combinations are valid, including:

- positive location + positive matter,
- positive location + negative matter,
- negative location + positive matter,
- unknown location + positive matter,
- positive location + unknown matter.

The same spectrum may therefore be learned positively in one context and the same location may be learned negatively because of a different experience.

## 59.6 Association representation and reinforcement

Association value is distinct from memory strength.

Association value:

\[
A\in[-1,1]
\]

answers what the remembered consequence tends to mean.

Association confidence/strength:

\[
C\in[0,1]
\]

answers how strongly the organism has learned that relationship.

Memory strength answers how strongly the experience remains represented at all.

New association estimates use weighted reinforcement rather than overwrite:

\[
A_{new}=\frac{A_{old}W_{old}+A_{exp}W_{exp}}{W_{old}+W_{exp}}
\]

\[
W_{new}=W_{old}+W_{exp}
\]

The experience weight is based on the magnitude of the actual consequence rather than merely counting events.

## 59.7 Memory strength and decay

The approved principle is:

> **memory strength = recency + accumulated experience magnitude, subject to decay and finite capacity.**

A single large experience can create a strong memory. Repeated small experiences can also create a strong memory. Unreinforced memories decay.

Memory capacity remains derived from the physical genome cavity rather than a universal abstract slot count.

The current cavity-derived capacity/decay architecture is retained conceptually. Numerical decay and normalization values are **EXPERIMENTAL calibration parameters** and must not be treated as biological constants.

The inherited `memory_strength` trait controls memory formation/reinforcement strength. It does not replace physical cavity capacity.

## 59.8 Consequence representation

Physical consequences are recorded multidimensionally before valuation. At minimum the model must retain:

- usable-energy delta,
- stress delta,
- physical-damage delta,
- developmental/blueprint-realization delta,
- material acquired,
- material consumed/used.

Additional dimensions may be retained when they are directly produced by authoritative physical state and are useful to evaluate an experience.

A consequence must not be permanently collapsed into a generic good/bad label.

Acquisition is a consequence in its own right. For example:

**MOVE/overlap → 2C material acquired → memory records acquisition.**

Later:

**PROCESS → physical transformation → energy/development/stress/etc. consequence → memory reinforces the relevant spectral and spatial associations.**

This preserves the distinction between finding/acquiring material and processing it.

## 59.9 State-dependent consequence valuation

The same physical consequence can have different present value depending on the organism's current state.

The conceptual valuation is:

\[
V(E|State)=
 w_E(State)\Delta E
+w_D(State)\Delta D
+w_R(State)\Delta R
+w_A(State)A_q
-w_S(State)\Delta S
-w_X(State)\Delta X
\]

where the terms represent, as applicable:

- usable-energy change,
- developmental/blueprint progress,
- reproduction-relevant progress,
- material acquisition,
- stress,
- physical damage.

Weights are state-dependent. Energy becomes more important when usable energy is scarce; developmental progress becomes more important when developmental needs remain; stress and damage become more costly when the organism is vulnerable.

This valuation is a current decision interpretation, not a permanent mutation of the stored physical consequence.

### Experimental valuation calibration

Initial implementation values should be treated as **EXPERIMENTAL** and calibrated against observed behavior rather than as biological constants. The initial implementation should use normalized contribution magnitudes and equal unit weights as the neutral starting point, with state multipliers supplying the first-order adaptation:

\[
w_E=1+2(1-E_n)
\]
\[
w_D=1+(1-R_n)
\]
\[
w_S=1+2S_n
\]
\[
w_X=1+2X_n
\]
\[
w_A=1
\]
\[
w_R=1+(1-R_{repro,n})
\]

where normalized state quantities are clamped to \([0,1]\). These values are deliberately experimental.

## 59.10 Curiosity

Curiosity is an inherited, mutable genome trait.

It represents the organism's baseline drive to investigate unfamiliar perceived stimuli. It is not a hard-coded behavioral category and is not a substitute for perception.

The initial trait is:

\[
\boxed{curiosity=0.5}
\]

with initial mutation sigma:

\[
\boxed{\sigma_{mutation}=0.05}
\]

These are **EXPERIMENTAL calibration values**.

When an organism has little confidence in a perceived spectral pattern or location, curiosity supplies exploratory value in proportion to its inherited trait.

## 59.11 Perception-first movement

Movement is driven primarily by what the organism is experiencing **now**.

Memory interprets current perception; it does not act as a second sensory system.

For current spectral/matter uncertainty:

\[
U_m=C_mV_m+(1-C_m)K_m
\]

For current location uncertainty:

\[
U_l=C_lV_l+(1-C_l)K_l
\]

The combined stimulus utility is based on these independent matter and location terms and the currently perceived magnitude.

For perceived stimulus \(i\):

\[
\vec D_i=\hat r_i\,M_i\,(U_{m,i}+U_{l,i})
\]

and:

\[
\vec D=\sum_i\vec D_i
\]

The organism moves according to the resulting current perceived direction, subject to physical movement constraints.

If current directional information is insufficient, curiosity provides exploratory movement rather than a remembered arbitrary destination.

Remembered locations may influence behavior through current perception of the remembered region; memory does not create omniscient perception of distant locations.

## 59.12 Movement distance

Candidate movement distances remain:

\[
d\in\{1,2,4,8\}
\]

Distance is selected from the current situation, not from a learned historical preference for a particular movement distance.

For each candidate:

\[
U(d)=E[experience\ reached\ at\ d]+E[exploration\ at\ d]-C_{move}(d)
\]

Movement cost is the existing physical movement cost. No separate "4-unit movement was good" memory is retained.

### Experimental movement calibration

The initial implementation should use the existing movement-cost function as the authoritative physical cost. The weighting between expected experience, exploration, and cost is **EXPERIMENTAL**. The neutral starting point is equal unit weighting after each term is normalized to comparable magnitude.

## 59.13 Legacy systems explicitly retired

The following are superseded by this section and must not remain behavioral authorities:

- `MemoryPoint` as the primary memory model,
- the arbitrary 40-unit memory merge radius,
- direct movement vectors toward remembered points,
- `DecisionHistory` as a separate cognitive memory system,
- non-decaying lifetime decision history,
- movement-distance-specific historical preferences,
- semantic resource desirability memories.

Actual consequence information from those systems is retained only where it belongs in the new multidimensional experience-memory model.

## 59.14 Experimental calibration registry

The following values are explicitly **EXPERIMENTAL** and may be tuned without changing the approved architecture:

| Parameter | Initial value | Purpose |
|---|---:|---|
| Spectral match sigma | 0.25 | Log-frequency similarity falloff |
| Curiosity trait | 0.50 | Baseline inherited exploration drive |
| Curiosity mutation sigma | 0.05 | Evolutionary variation in curiosity |
| Neutral consequence weights | 1.0 | Starting contribution scale |
| Energy scarcity multiplier | 2.0 | Increase energy value under scarcity |
| Stress vulnerability multiplier | 2.0 | Increase stress cost under vulnerability |
| Damage vulnerability multiplier | 2.0 | Increase damage cost under vulnerability |
| Developmental-need multiplier | 1.0 | Increase developmental value with remaining need |
| Movement term weights | 1:1:1 | Experience / exploration / movement-cost starting balance |
| Memory decay normalization | current cavity-derived model | Persistence calibration; subject to validation |

These numbers are **not** new biological laws. They are starting experimental values for the approved architecture.

## 59.15 Implementation authority

This section is now the authoritative memory design.

Before implementation, existing code must be audited against each subsection. Each migration step must preserve the approved physical authorities and remove obsolete duplicate authorities rather than layering the new model on top of the old one.

No code may reintroduce a retired memory mechanism merely because it is convenient for an intermediate implementation.

The migration must be performed incrementally with an audit before and after each change, followed by focused behavioral tests and a long-run simulation validation.

## 59.16 Changes already made in the memory-model migration

The following changes have already been made on the dedicated memory-model branch:

1. Added mutable inherited genome trait `curiosity` with initial value 0.5 and mutation sigma 0.05.
2. Added the first experimental continuous spectral-similarity implementation using logarithmic frequency distance and amplitude-weighted symmetric matching.
3. Added focused spectral-similarity tests covering exact matching, symmetry, and decreasing similarity with frequency distance.
4. Created draft PR #145 so the migration can be validated independently before any merge to `main`.

These changes are foundations only. The full memory migration described in this section is not considered complete until the legacy memory authorities have been replaced, tested, and validated by the simulation.

# 26. Interior, Water, Permeability, and Boundary Interaction

This section formalizes the approved physical model for the organism interior and
supersedes earlier descriptions that treated the union of structural material
as the organism's interior.

## 26.1 Structure is membership, not solidity

**Structural** is a membership/connectivity label.

A constituent is structural when it belongs to the organism's physical structural
graph under the established genome-connectivity rules. Structural membership does
**not** mean that the constituent is a rigid, impermeable wall.

The material itself determines physical behavior.

In particular:

- rigid materials contribute solid physical boundaries according to their actual
  realized geometry and physical properties;
- Water is a fluid medium and remains permeable;
- increasing Water content can make a structure more permeable;
- a structure can therefore be structurally connected while containing fluid,
  porous, or otherwise passable material.

No separate universal "solidness" flag is introduced merely because a constituent
is structural.

## 26.2 Organism interior is enclosed space

The organism's **interior** is the physical region enclosed by its realized
structural boundary.

It is **not** the union of the areas occupied by structural constituents.

Therefore an environmental material is not inside merely because its point lies
inside a structural-material bounding shape, and an empty cavity is not outside
merely because it contains no structural material.

The physical model must distinguish:

1. **structural geometry** — the realized material that makes up the organism;
2. **boundary geometry** — the realized outer boundary through which the
   environment can interact with the organism;
3. **interior regions** — spaces enclosed by that boundary.

A sealed cavity is an interior region. An opening to the environment is not a
sealed interior region.

Interior classification must be derived from actual realized geometry/topology,
not from a bounding box and not from a union-of-material containment test.

## 26.3 Water is everywhere logically

Water is present everywhere in the environment as an effectively unlimited
**logical** material.

This is not a finite environmental reservoir.

Water is materialized physically only when a physical representation is required
at a location, such as when an organism's structure requires interior medium.

There is therefore:

- no finite Water resource cloud,
- no Water depletion mechanic,
- no Water replenishment mechanic,
- no environmental Water stock that organisms compete to exhaust.

Only physical Water participates in physical interaction.

## 26.4 Water's physical realization

Water has a default circular physical representation when it must be represented
as a free/uncontextualized physical object.

The circle is **not** a universal rigid shape for Water.

When Water is structurally connected to an organism, its physical realization may
take the geometry necessary to fit the surrounding structural context. The
implementation may use an efficient equivalent representation when needed for
performance, provided the resulting physical behavior remains equivalent.

Thus the intended rule is:

> Free Water uses its default realization; connected Water is realized to fit
> the physical structure it belongs to.

Connected Water remains fluid even when its realized geometry conforms to a
non-circular structural region.

## 26.5 Filling the interior with Water

The approved construction sequence is:

1. construct the ordinary structural material according to the existing
   construction rules;
2. determine the resulting enclosed interior regions;
3. fill the appropriate qualifying gaps with the quantity of Water that can fit;
4. physically realize that Water using geometry appropriate to each gap;
5. connect the realized Water into the organism's structural graph under the
   existing structural qualification rules.

The automatic interior Water is therefore ordinary physical structural material,
not a separate internal Water inventory.

For the confirmed initial organism, Water is also realized as part of the outer
boundary itself. Alternating outer-shell constituents are replaced by fitted
connected Water, leaving rigid material between permeable boundary sections.
This is part of the initial realized structure, not a separate membrane organ.
The initial organism keeps that same realized structure when its developmental
stage later becomes Adult; adulthood does not rebuild the seed into a different
outer-boundary model.

Water added this way must not bypass the physical graph or become a special
logical substance stored inside the organism.

The genome cavity remains governed by the established genome rules and must not
be indiscriminately filled in a way that destroys genome qualification.

## 26.6 Acquisition and bond interaction at the boundary

Environmental material does not need to cross the boundary as a complete physical
component before the organism can interact with it.

The boundary is a constraint on **accessible connection points**:

> environmental physical material → boundary interaction → accessible connection
> point → COMBINE/BREAK through the existing action system

A bond may be combined with or broken when the relevant connection point lies inside an
accessible enclosed interior. The interacting physical material may therefore straddle
the organism boundary while the interaction occurs.

This is deliberately different from free manipulation. A physical material is freely
movable, rotatable, or otherwise manipulable as an interior object only when its entire
realized physical geometry is inside an accessible enclosed region. A straddling or
partly external component remains boundary-constrained until it becomes fully enclosed.

Passage is still supplied by the realized boundary's existing physical behavior rather
than by a special acquisition probability: rigid boundary material participates in
collision and blocks penetration, while Water remains fluid and does not create a
penetration barrier.

A material fully inside an **accessible** enclosed interior may therefore be transferred
to organism possession/storage and manipulated normally. A material that only has an
accessible connection point is not automatically transferred to storage; it can instead
participate directly in the appropriate physical bond interaction.

The genome cavity is inside the organism but is excluded from accessible interior
classification and remains genuinely empty.

Composite internal bonds remain intact unless an existing EvoSim transformation
explicitly breaks them. Logical environmental material is never promoted directly into
organism storage.

## 26.7 Permeability is derived, not authored

Permeability is an emergent physical consequence of:

- the material composition of the realized boundary,
- the actual geometry/topology of that boundary,
- the arrangement of materials along the boundary,
- and the established physical properties of those materials.

Water content is an important contributor, but **permeability is not equal to
Water occupancy or Water percentage alone**.

For example, a structure that is 98% Carbon and 2% Water must not become 100%
permeable merely because the small amount of Water happens to occupy every local
gap.

Likewise:

- the same topology with different material composition can have different
  boundary behavior;
- the same composition with different topology can have different boundary
  behavior.

No dedicated permeability gene is introduced.

No arbitrary random pass-through probability is introduced.

No general soft-body collision rewrite is introduced merely to obtain permeability.

Permeability belongs to the boundary-interaction layer and must coexist with the
existing rigid collision/movement system.

## 26.8 Required implementation invariants

The implementation must preserve these invariants:

- structural membership remains graph/connectivity-derived;
- free Water retains its default physical representation;
- connected Water can be re-realized to fit its structural context;
- Water remains fluid regardless of structural membership;
- interior classification is topological/enclosure-based;
- an open boundary does not create a sealed interior;
- nested enclosed regions remain distinguishable;
- environmental composites retain their internal bonds during passage;
- organism storage contains only realized physical material;
- logical Water is never inserted directly into organism storage;
- the finite non-Water resource cloud remains separate from infinite logical Water;
- permeability is derived from physical composition plus realized boundary geometry;
- no new gene, energy battery, or arbitrary probability is required for these rules.

## 26.9 Controlled implementation sequence

These changes are intentionally staged so that each architectural authority can
be tested independently:

1. **Realized interior topology** — generalize the existing geometric face/boundary
   machinery so enclosed regions are a reusable physical concept rather than a
   genome-only calculation.
2. **Connected Water realization** — allow connected Water to assume geometry that
   fits its structural context while preserving fluid behavior.
3. **Automatic interior Water** — after ordinary construction, materialize physical
   Water into appropriate qualifying interior gaps and connect it to the structure.
4. **Boundary/interior acquisition** — replace union-of-structural-material
   containment with boundary crossing plus enclosed-interior classification.
5. **Derived boundary behavior** — make permeability a consequence of realized
   boundary composition and topology without adding a dedicated permeability
   variable or random pass-through.

Do not change movement costs, COMBINE/BREAK timing, genome connectivity rules,
reproduction architecture, the finite non-Water resource cloud, or the Water
resource's infinite logical availability as part of these stages unless a direct
dependency is demonstrated.

## 26.10 Regression requirements

Before these changes are considered complete, tests must cover at minimum:

- material outside an organism versus material inside an enclosed cavity;
- sealed cavity versus cavity with an opening;
- nested enclosed regions;
- narrow passages;
- touching/overlapping boundary geometry;
- free Water retaining its default representation;
- connected Water receiving a context-fitting realization;
- no logical Water entering storage;
- intact composite material remaining intact while crossing a boundary;
- a high-solid/low-Water boundary remaining substantially resistant to passage;
- same topology with different composition producing different boundary behavior;
- same composition with different topology producing different boundary behavior.

This section is the authoritative reference for the interior/permeability work.

