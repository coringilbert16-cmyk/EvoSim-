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

# 14. Material Acquisition

The existing environment-grid acquisition mechanism is valid as an environmental mechanism.

However, acquisition of **physical materials**, especially composite materials, requires further integration with physical geometry.

The authoritative acquisition rule is:

> A resource is acquired only when the resource is fully within the organism's outer boundary.

This applies to both base and composite resources.

Acquired material must retain its physical structure.

If a composite material is acquired, the organism must not silently flatten it into unrelated independent resource units merely because the environmental representation was convenient.

The acquired material must preserve:

- constituents,
- bonds,
- geometry,
- and physical relationships.

The existing grid-cell system may continue to serve as the environment's spatial mechanism, but physical-material acquisition must ultimately respect actual containment.

# 15. Material Storage

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

The physical storage system must eventually bridge the environmental/grid representation and the actual physical material representation.

# 16. BREAK

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
- and eventual decomposition.

A physical structure may need to be broken before it can be rearranged into a configuration that better matches its blueprint.

Therefore:

> **BREAK is one of the tools of construction.**

It is not inherently destructive from the organism's perspective.

# 17. COMBINE

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

# 18. Construction and Backtracking

The construction system must be capable of finding physically valid realizations.

Where a locally valid placement prevents the rest of a structure from being realized, the constructor must be able to reconsider earlier choices.

This is why candidate search/backtracking belongs in the construction system.

The constructor should not permanently commit to the first available placement when that placement can make the overall structure impossible.

The objective is:

> Find a physically valid realization consistent with the intended blueprint and already-realized neighborhood.

# 19. The Blueprint

The blueprint is the organism's structural target.

It is not the physical organism itself.

The adult blueprint represents the intended adult structure at **100% scale**.

The blueprint provides the target layout that the physical organism is attempting to realize.

The physical structure may temporarily differ from the blueprint during:

- juvenile growth,
- construction,
- repair,
- reorganization,
- damage,
- or other physical processes.

The blueprint must not be confused with a hard-coded body-plan category.

# 20. Juvenile Blueprint Construction

The juvenile is generated from the adult blueprint.

The adult blueprint represents **100% size**.

To construct the initial juvenile:

1. Begin with the adult blueprint.
2. Reduce the overall spatial layout to approximately **40%**.
3. Do **not** require the juvenile to contain exactly 40% of the adult's constituents.
4. Do **not** require exactly 40% of the adult's bonds.
5. The important target is the reduced **overall layout**.
6. A constructor or appropriate realization mechanism then attempts to build the closest physically valid structure to that reduced layout.

The exact realization mechanism remains an implementation question to be resolved through the existing construction architecture rather than through invention of a new biological rule.

# 21. Juvenile's Blueprint After Birth

Once the juvenile is born, it possesses the **100% adult blueprint**.

It does not receive a permanently reduced 40% blueprint.

The juvenile therefore knows, through its blueprint, what adult structure it is growing toward.

Growth is the process by which physical realization moves toward that adult blueprint.

# 22. Growth Is Not a Decision

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

# 23. Growth Uses Both BREAK and COMBINE

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

# 24. Juvenile Requirements

The juvenile's fundamental requirements are:

1. **Survive**
2. **Maintain suitable energy**
3. **Construct toward adulthood**
4. **Eventually reproduce once adulthood is reached**

Growth is not a separate biological decision.

The organism does not need an invented "growth motivation" variable.

The physical/developmental state itself determines what remains necessary to reach the blueprint.

# 25. Adulthood

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

# 26. No Age-Based Lifecycle

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

# 27. Adult Transition

Once the organism reaches adulthood through the established blueprint-match requirement:

- it is considered adult,
- reproduction may begin,
- and reproductive activity becomes the primary priority.

There is no separate invented reproductive-readiness accumulation stage between adulthood and reproduction.

# 28. Reproduction

Reproduction is physically related to construction but is distinct from ordinary juvenile growth.

Reproduction begins with a **single resource designated as the anchor**.

The anchor resides within the adult organism's outer boundary.

The genome is constructed first from that anchor.

The offspring is then constructed within/through the adult's physical reproductive process until the juvenile satisfies the established birth requirements.

# 29. Reproductive Juvenile Target

The juvenile must reach:

- approximately **40% of adult structural size**, and
- approximately **90% visual/functional similarity** to the intended reduced realization,

before birth/separation.

The 40% target concerns the overall spatial realization, not a requirement that the juvenile contain exactly 40% of the adult's constituents or bonds.

The constructor is responsible for finding the closest physically valid realization of the reduced target.

The detailed visual/functional similarity system remains subject to the previously established design and must not be replaced with an invented metric.

# 30. Juvenile Separation

Once the juvenile satisfies the established birth requirements, it separates from the adult and becomes a new life.

The new juvenile then possesses its **100% adult blueprint** and proceeds through the normal developmental process.

The juvenile's physical structure at birth is therefore the reduced realization.

Its blueprint is the full adult target.

# 31. Maintenance

An organism must maintain sufficient usable energy to continue operating.

Maintenance consumes usable energy according to the current basic maintenance model.

The current basic formula is **experimental**.

It must therefore be treated as a provisional implementation rather than a permanently established biological law.

No new maintenance formula should be substituted without approval.

# 32. Heat Stress

Every energy transaction produces some heat stress.

Stress:

- accumulates,
- dissipates over time,
- and can eventually cause structural damage.

Heat stress is therefore a consequence of energy transactions rather than an unrelated damage meter.

The energy ledger and the organism's stress state must ultimately be connected so that energy transactions cannot silently bypass heat-stress consequences.

# 33. Stress Dissipation

Accumulated stress can dissipate over time.

The existing stress-decay mechanism may remain as implementation infrastructure unless and until the project specifies a different rule.

The exact implementation must not be treated as a newly invented biological law simply because a current constant exists in code.

# 34. Stress Thresholds

When accumulated stress reaches the current stress threshold:

1. structural damage occurs,
2. the threshold is reduced,
3. additional stress accumulation can therefore make subsequent damage easier,
4. and the process can cascade.

This establishes the intended cascading failure behavior.

The lowered threshold is part of the established damage model.

# 35. Stress Damage Selection

Stress damage has an important physical distinction.

When stress causes structural damage:

- a **random non-genome structural bond** is selected first,
- genome bonds are protected while non-genome structural bonds remain,
- once all non-genome structural bonds have been broken, genome bonds may become vulnerable.

This ordering is authoritative.

The current "weakest bond" implementation is therefore not equivalent to the intended rule.

# 36. Genome Damage

Genome damage occurs only after the organism's non-genome structural bonds have been exhausted through stress damage.

If genome bonds subsequently break, the genome may:

- cease functioning,
- or lose/reduce capabilities,

depending on the complexity and continued viability of the remaining genome structure.

The exact capability-reduction mechanism has not yet been fully designed.

Therefore:

> **Do not invent a genome capability-degradation system.**

The physical damage ordering can be implemented independently while the capability semantics remain pending.

# 37. Death

Death occurs when the organism can no longer remain a viable organism.

The established death conditions include:

- the genome being broken,
- inability to maintain itself,
- inability to complete necessary actions/transactions,
- and the eventual consequences of structural failure.

Death is not defined by age.

There is no maximum-age lifecycle.

There is no artificial "old age" death mechanism.

# 38. Dead Organisms Remain Physical Material

When an organism dies, it becomes resource/material rather than simply disappearing.

Its physical material must remain part of the simulation.

Death does not automatically transform all material into an abstract raw-resource pool.

The carcass remains physical material and can undergo decomposition.

# 39. Decomposition

Decomposition physically dismantles dead organism structure.

Existing decomposition infrastructure may break the structure apart and eventually return constituent material to the environment.

The precise long-term ecological rules governing every possible material transformation/reentry pathway remain incompletely designed.

Where those rules have not been specified, they must not be invented.

# 40. Colonies and Cooperation

Colonies must emerge from actual organism interactions.

There is no hard-coded "colony stage."

There is no predefined social-organism class.

Cooperation must arise from organisms actually performing cooperative behaviors.

The established neighbor concept is behavior-based rather than simple physical proximity.

Two organisms are considered neighbors in the relevant cooperative sense when they are simultaneously enacting at least two distinct cooperative behaviors.

# 41. No Universal Storage or Energy Battery

There must not be a universal genome trait that simply provides:

> "storage capacity = X"

or:

> "energy battery = X"

Physical geometry should determine available space and storage wherever that is the established mechanism.

Cavities and actual structure should provide the physical basis for storage and energy handling.

# 42. Movement, Perception, and Memory

Movement, perception, directional resolution, memory, and related systems are legitimate simulation infrastructure.

They are not, however, substitutes for the physical lifecycle rules in this document.

They must not become hidden authorities for:

- maturation,
- adulthood,
- growth,
- reproduction,
- or organism identity.

Behavioral systems and physical developmental systems must remain conceptually distinct.

# 43. Environment

The environment is a physical resource system.

The established environment architecture includes the distinction between:

- a reservoir,
- and an active ecological field.

Vents transfer material from the reservoir into the active field.

Vents are not inherently biased toward bonded or unbonded material.

They draw nearby reservoir material and expel it according to the established environmental mechanism.

Bonded material can settle into the reservoir.

Waste remains physical material and may be redistributed, transformed, or eventually re-enter the reservoir according to rules that have been explicitly established.

Where environmental rules remain unfinished, implementation must stop at the known boundary rather than inventing the missing rule.

# 44. Material Transformation

BREAK and COMBINE are physical transformations.

Neither should be assumed to be universally energy-positive or universally energy-negative.

The resource/material system must permit a meaningful distribution of energetic outcomes.

The existing material model should remain grounded in constituent potential energy and actual physical transformation.

# 45. Complexity

The current complexity hypothesis is:

> **n × log₂(n)**

where applicable to the established complexity model.

The current two-component duration is:

> **2 ticks**

These are project-level established parameters/hypotheses and should not be silently replaced.

# 46. Physical Construction Authority

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

# 47. No Duplicate Structural Authorities

EvoSim should not have multiple competing systems independently deciding what the same physical bond or structure means.

In particular:

- blueprint realization should not independently create structural bonds after COMBINE has already admitted them,
- abstract architecture should not override physical structure,
- genome metadata should not contradict the physical genome,
- and convenience representations should not silently become authoritative.

The migration toward a unified construction path must continue in this direction.

# 48. Decision System

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

# 49. Experimental vs Authoritative Rules

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

# 50. What Must Never Be Introduced Without Approval

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
- or any other mechanism not already approved.

# 51. Handling Ambiguity

When implementation encounters a requirement that this document does not settle:

**Do not guess.**

The correct development process is:

1. Identify exactly what is unspecified.
2. Explain why the implementation requires a decision.
3. Present the relevant existing architecture and consequences.
4. Ask the project owner for the decision.
5. Implement only after approval.

The absence of a specification is not permission to create one.

# 52. Implementation Priority

The immediate architectural objective is to make one initialized cell capable of progressing through the actual physical lifecycle without violating the established rules:

> **A physically authoritative organism with a genome can acquire environmental material, incorporate material through physically valid COMBINE operations, use BREAK where necessary, maintain itself, grow toward its adult blueprint, reproduce through the established anchor/blueprint process, and eventually die and decompose without violating material or energy conservation.**

The system should be developed toward this objective incrementally.

The priority is not adding more species, behaviors, visual effects, or complexity before this fundamental loop works.

# 53. Development Standard

Every significant implementation change should be evaluated against four questions:

### 1. Is this explicitly specified?

If yes, implement it.

### 2. Is this already implemented correctly?

If yes, do not replace it unnecessarily.

### 3. Is this implementation legacy or contradictory?

If yes, identify it as such and migrate it carefully.

### 4. Is the behavior unspecified?

If yes, stop at the boundary and obtain approval.

# 54. Final Authority

The project owner's explicitly approved design decisions are the final authority for EvoSim.

Code is not authority merely because it already exists.

Tests are not authority merely because they currently pass.

A legacy abstraction is not authority merely because multiple systems depend on it.

A convenient implementation is not authority merely because it is easier to code.

The purpose of the migration is to make the implementation conform to the intended EvoSim model—not to redefine the model around whatever mechanisms happen to exist in the repository.

# 55. Non-Negotiable Principle

The central rule for future development is:

> **Implement the design that has been approved. Do not invent the design.**

If a mechanism is necessary to implement an approved behavior but its exact form has not yet been determined, that mechanism must be discussed and approved before becoming part of EvoSim's architecture.

**No deviation from this document is authorized without explicit approval from the project owner.**
