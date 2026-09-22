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

If a physical piece is bonded directly or indirectly to the genome, it is part of the organism.

This gives physical connectivity a central role in determining organism structure.

The system must move toward a universal physical membership rule rather than maintaining an independent abstract body list that can disagree with the actual structure.

The physical graph should ultimately be able to answer:

> What material is actually part of this organism?

based on its relationship to the genome.

# 13. Initial Seed Cell

The first organism must contain a genome.

The initial organism must have enough physical structure to:

1. fully encompass the genome,
2. provide the required genome cavity,
3. exist as a physical organism,
4. and have sufficient internal space to acquire material from the environment.

The seed is therefore not simply a data structure marked "alive."

It must be an actual physically realized organism.

# 14. Physical Material Availability

There is no ACQUIRE action.

Material availability is a consequence of physical containment rather than a separate organism action.

The environment is a physical material system. The active field may contain individual resources, bonded composites, larger connected structures, loose material, fluid material, and other physical arrangements that emerge from the underlying resource and material rules.

The authoritative rule is evaluated at the constituent scale:

> A physical constituent is available to the organism when that constituent is physically within the organism's outer boundary.

A composite material does **not** have to be entirely within the organism to interact with it. A composite may straddle the organism boundary, with some constituents inside and others outside.

This does not itself break the composite or dissolve an existing bond. The physical material remains one structure until an actual physical transformation changes its connectivity.

When a transformation acts on only the constituents within the organism, the material system may partition the existing physical material at constituent boundaries. Internal bonds are preserved only when both bonded constituents remain in the same resulting partition. A bond crossing the boundary is not silently recreated or destroyed merely because of containment.

The organism therefore does not "pick up" a composite through an ACQUIRE action. Physical containment makes constituent material available to the organism's existing transformation machinery.

This applies to both base and composite material.

No transfer operation may convert abstract composition into a newly invented physical object. Existing physical material must preserve, where applicable:

- constituents,
- quantities,
- internal relationships,
- bonds,
- geometry,
- relative placement,
- orientation,
- physical state,
- and other information required to preserve its physical identity.

COMBINE may use physically available material directly. BREAK may act on existing physical bonds, and any resulting disconnected material remains governed by its actual physical location.

The environment-grid system may remain the spatial indexing mechanism used to locate candidate material, but the grid is not itself the ultimate authority for physical geometry or containment.

## 14.1 The Active Field Is a Physical Material Layer

The active field is not merely a collection of independent resource quantities.

It is the environment's spatially organized physical-material layer.

Its contents may be analogous, at different scales and configurations, to:

- bedrock,
- rocks,
- rock fragments,
- soil,
- sediment,
- loose deposits,
- accumulated material,
- and fluid material.

These are **emergent descriptions**, not fundamental environmental object types.

The implementation must not introduce hard-coded `Rock`, `Bedrock`, `Soil`, `Sediment`, `Food`, `Waste`, or equivalent ecological categories merely to represent these arrangements.

A large environmental formation may be represented as a physically connected structure, while smaller fragments may exist as separate structures. The underlying constituents and physical relationships remain authoritative in either case.

## 14.2 Environmental Physical Authority

While material exists in the environment, the environment owns its physical realization.

A physically existing composite is therefore not permitted to collapse into composition-only state if doing so would lose information required to determine later physical interactions.

The governing rule is:

> **A physical material object does not lose its physical identity merely because it changes location or ownership.**

When material crosses from the environment into an organism, the transfer changes ownership; it does not require reconstruction of the material's existing physical structure.

## 14.3 Composition Does Not Invent Geometry

Knowing that an environmental material contains particular constituents is not sufficient authorization to choose a particular physical arrangement for that material instance.

For example, knowing that a material contains Carbon, Methane, and Water does not authorize ACQUIRE to arbitrarily choose their relative positions or orientations.

If a physically existing material instance has no authoritative physical realization, ACQUIRE must not silently invent one merely to complete the transfer.

Physical realization must instead come from an authoritative physical process.

## 14.4 Environmental Structures and Scale

The active field must support physical structures ranging from individual resource units to large connected formations.

There is no authored environmental size corresponding to a category such as rock or soil.

Scale emerges from physical arrangement, connectivity, quantity, geometry, and environmental processes.

Performance-oriented aggregation is permitted only when it preserves enough information to produce the physical interactions that the simulation needs to calculate.

An optimization must not become a second physical authority.

## 14.5 Environmental Fragmentation and Accumulation

Environmental material may fragment into smaller physical structures and may accumulate into larger structures through established physical processes.

BREAK may participate in physical fragmentation where its rules permit.

Movement, deposition, cohesion, geometry, and other established environmental interactions may produce accumulation.

Neither process requires conversion into a special environmental resource type.

## 14.6 Rigid and Fluid Environmental Material

Environmental material retains the physical-state rules of its constituents.

Rigid material retains its established geometry and interacts through actual geometry and connection points.

Fluid material retains its fluid behavior.

Water remains a fluid resource with a default circular representation; the representation is not an authored rigid solid shape.

## 14.7 No Environmental Biological Roles

Environmental material has no inherent biological purpose.

There is no fundamental distinction between food, construction material, waste, nutrient, obstacle, shelter, or useful/unused resource.

The same physical material may be used differently by different organisms.

Its biological significance emerges from organism behavior and physical interaction rather than from an environmental role flag.

# 15. Environmental Material Transfer and Vents

Vents are environmental transfer mechanisms, not separate environmental storage compartments.

Where vents are present, they redistribute material within the environment according to the established environmental process.

Vents are not inherently biased toward bonded or unbonded material and do not classify material according to biological usefulness.

Material emerging through a vent may therefore include individual resources, composite material, bonded structures, or mixtures of materials according to the actual environmental process.

No obsolete deep-reservoir model is part of EvoSim.

# 16. Material Storage

Material storage is physical material storage.

Stored material must retain its physical form.

For a composite material, this means preserving:

- constituents,
- bonds,
- geometry,
- and internal structure.

Stored material must be available to both:

- **BREAK**
- **COMBINE**

There are no special exclusions where the established rules say all stored material is available.

The storage system must preserve an acquired physical material instance rather than reducing it to composition-only state.

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
- whether it needs to acquire additional material,
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

- acquire material,
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

If an implementation has only composition and no authoritative physical realization, it must not silently invent one solely to make ACQUIRE or another transfer operation possible.

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

## 44.8 Vents

Where vents are present, they are environmental transfer mechanisms rather than separate storage compartments.

Vents do not classify material according to biological usefulness and are not inherently biased toward bonded or unbonded material.

They redistribute material according to the established environmental process.

No obsolete deep-reservoir model is part of EvoSim.

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

- whether to acquire,
- whether to break,
- whether to combine,
- whether to rearrange,
- whether to perform another permitted behavior,

based on its state and environment.

But "grow" itself is the resulting developmental process.

The distinction is:

> **Growth is a goal/state transition.**
>
> **BREAK, COMBINE, ACQUIRE, and related operations are mechanisms that may be selected to accomplish it.**

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

> **A physically authoritative organism with a genome can acquire environmental material, incorporate material through physically valid COMBINE operations, use BREAK where necessary, maintain itself, grow toward its adult blueprint, reproduce through the established anchor/blueprint process, and eventually die and decompose without violating material or energy conservation.**

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
