# EvoSim — Master Specification Sheet V5

**STATUS: AUTHORITATIVE**  
**VERSION: 5**  
**DATE: September 2026**  
**SUPERSEDES: All previous EvoSim Master Spec Sheet documents**

---

## 0. Source-of-Truth Rule

This document is the authoritative design specification for EvoSim.

When code, comments, older specifications, audit notes, or implementation assumptions conflict with this document, this document wins unless a later explicitly authoritative specification replaces it.

A distinction is maintained throughout this document between:

- **LOCKED** — a design decision is settled and must not be reopened merely because implementation is difficult.
- **IMPLEMENTED GROUNDWORK** — code exists to support the rule, but the complete runtime behavior may not yet be integrated.
- **NOT YET DECIDED** — an issue remains deliberately open and must be resolved before implementation depends on it.

Implementation details may change. The underlying causal rules may not be changed without explicitly updating this specification.

---

# 1. Core Design Philosophy

EvoSim is a chemistry-first evolution simulator.

It is **not** a conventional resource-management or numbers-go-up game. Organisms do not receive abstract energy points that can simply be accumulated and spent. Instead, usable energy emerges from physical and chemical interactions among materials.

The simulation should expose a small set of general physical rules and allow biological organization, adaptation, and evolution to emerge from them.

The central principle is:

> Track information only when that information can causally affect another system.

Avoid duplicated derived state, arbitrary biological exceptions, hard-coded recipes, magic statistics, and special-case shortcuts that bypass the physical model.

---

# 2. Fundamental Entities

The simulation contains three fundamental domains:

1. **Environment** — the spatial physical system containing resource material.
2. **Resources / Materials** — physical matter with intrinsic properties and derived interaction behavior.
3. **Organisms** — physical structures made from materials and governed by the same underlying material rules as their environment.

Energy is **not** a fundamental entity.

Memory, perception, movement, development, reproduction, and decision-making are higher-level biological systems built on top of the physical/material substrate.

---

# 3. Resource Model

## 3.1 Base resources

The initial resource catalog is:

- Carbon
- Methane
- Hydrogen
- Sulfur
- Nitrogen
- Phosphorus
- Water

The catalog is extensible.

## 3.2 Immutable intrinsic properties

Every base resource has immutable intrinsic properties:

- mass
- potential energy
- reactivity
- cohesion
- physical form/geometry

These properties are part of the resource definition and are not mutated by ordinary simulation activity.

## 3.3 Current catalog

| Resource | Mass | Potential Energy | Reactivity | Cohesion |
|---|---:|---:|---:|---:|
| Carbon | 1.0 | 1.0 | 0.1 | 0.95 |
| Methane | 1.0 | 20.0 | 4.0 | 0.10 |
| Hydrogen | 1.0 | 12.0 | 3.0 | 0.05 |
| Sulfur | 1.0 | 8.0 | 2.0 | 0.40 |
| Nitrogen | 1.0 | 0.5 | 0.2 | 0.70 |
| Phosphorus | 1.0 | 0.8 | 0.3 | 0.60 |
| Water | 1.0 | 0.0 | 0.0 | 0.50 |

These values are the current canonical baseline catalog.

---

# 4. Potential Energy

Potential energy is an intrinsic property of material composition.

It is **not stored as a mutable organism energy pool** and should not be represented by a legacy `energy_content` field on material.

For composite materials, potential energy is derived from their constituent material properties and amounts.

Potential energy establishes the energetic direction of an interaction. It does not by itself determine the magnitude of the interaction.

---

# 5. Reactivity

Reactivity describes how strongly material tends to participate in chemical interaction.

The influence of reactivity is nonlinear/exponential rather than a simple linear multiplier.

High intrinsic reactivity can therefore create disproportionately stronger interaction tendencies.

Environmental conditions can modify effective reactivity without mutating intrinsic resource properties.

---

# 6. Water

Water is a first-class ordinary material.

Water is not a special biological resource, a magic solvent, or an abstract environmental modifier.

Water:

- can be acquired,
- can be stored,
- can be transported,
- occupies physical space,
- can form contacts,
- participates in ordinary material interactions,
- can cross material boundaries when physical conditions permit,
- can influence effective reactivity through physical dilution/interactions.

Water does **not** receive a magic permeability statistic.

Water's intrinsic reactivity and potential energy remain unchanged. Its environmental influence emerges from physical distribution and interaction.

Water should normally be distributed throughout the organism rather than represented as one abstract dedicated pool.

---

# 7. Physical Geometry

Geometry is part of the simulation's causal model.

Materials have physical forms and spatial extents. Contact, overlap, connection opportunities, packing, arrangement, and boundary formation depend on actual geometry.

The simulation must not silently replace physical geometry with abstract adjacency when geometry can determine the outcome.

Current base-resource geometry uses a common nominal unit area with resource-specific shapes. The current groundwork includes:

- Carbon — regular hexagonal geometry
- Methane — triangular geometry
- Hydrogen — circular geometry
- Sulfur — pentagonal geometry
- Nitrogen — rectangular geometry
- Phosphorus — L-shaped polygonal geometry
- Water — fluid geometry

Geometry may evolve in implementation, but resource identity and physical consequences must remain explicit.

---

# 8. Material and Composite Material Model

A material may be:

- a base resource unit, or
- a composite structural material containing multiple constituent resource identities connected by bonds.

A composite material does **not** become a new resource type merely because its constituents are bonded.

Constituent identity must remain recoverable.

The composite representation must preserve:

- constituent resource identities,
- constituent amounts/parts,
- internal bonds,
- derived mass,
- derived potential energy,
- derived reactivity,
- derived cohesion,
- physical geometry.

This allows chemistry to create physically meaningful materials without creating an arbitrary hard-coded recipe list.

---

# 9. Bonding

Bond energy is separate from resource potential energy.

A bond represents a structural/chemical relationship between material constituents.

Bond energy is stateful structural information and can change as the structure changes.

Resource potential energy is intrinsic material information and is derived from composition.

These concepts must never be collapsed into one mutable `energy_content` value.

---

# 10. Natural Chemistry

Compatible materials can interact naturally based on their continuous properties and physical circumstances.

There is no finite hard-coded list of biological recipes that defines all possible chemistry.

Compatibility and interaction depend on factors such as:

- material properties,
- potential-energy relationships,
- reactivity,
- cohesion,
- geometry,
- physical contact,
- arrangement,
- environmental conditions.

Hard-coded reactions may be used only where they represent an explicitly justified physical rule rather than a shortcut for biological behavior.

---

# 11. Environment Architecture

The environment is a two-compartment physical resource system.

## Layer 1 — Deep Reservoir

The deep reservoir is a coarse spatial mass pool representing the large underlying resource supply.

It is not the primary interaction surface for organisms.

## Layer 2 — Active Ecological Field

The active field is the spatial layer in which organisms exist and material interactions occur.

Resources in the active field can:

- move,
- diffuse,
- interact,
- be acquired,
- be transformed,
- settle toward the reservoir.

## Transfer path

The intended environmental cycle is:

**reservoir → vent → active field → diffusion/interaction → settling → reservoir**

Vents are transfer mechanisms, not chemical processors.

---

# 12. Vents

Vents draw material from the deep reservoir and introduce it into the active ecological field.

Vents are intentionally indiscriminate.

They do not preferentially select:

- bonded versus unbonded material,
- high versus low potential energy,
- particular chemical recipes.

Vent transfer does not itself perform chemistry.

Bonded/unbonded state is preserved through transfer.

---

# 13. Perception

Perception is limited and spatial.

An organism should only obtain information that could physically or biologically be available to it.

Perception must not provide omniscient knowledge of the environment.

The perception system should expose physical/material information rather than hidden simulation state.

---

# 14. Access and Acquisition

Acquisition is physical.

An organism must be able to physically access material before acquiring it.

Overlap and contact matter.

A fully overlapping material unit may be acquired when the organism can physically contain/access it.

Partial overlap can still permit access at an exposed perimeter or edge.

Semi-permeability is therefore compatible with the model.

Acquisition must not bypass geometry by simply transferring an arbitrary resource quantity into an organism.

---

# 15. Internal Physical Space

An organism is not an abstract point containing an invisible resource pool.

Its interior is physical material-space.

Material, water, storage, and structural components occupy actual space subject to geometry.

Empty space is allowed only where it is physically required or deliberately defined, such as the seed core's hollow cavity and necessary packing/clearance gaps.

---

# 16. Seed Cell — Initial Biological State

The simulation begins with one **completed viable seed cell**.

The seed does not need to evolve from inorganic matter into the first viable organism.

The seed is the initial condition. Once initialized, all subsequent maintenance, acquisition, growth, organization, chemistry, damage, development, and reproduction use the ordinary simulation mechanics.

There is no separate magical bootstrap metabolism after initialization.

---

# 17. Seed Core Architecture

The seed core is a specialized six-unit configuration.

## 17.1 Exact sequence

The six core units are arranged in this repeating sequence:

**CM → CH → CS → CM → CH → CS**

where:

- CM = Carbon–Methane composite
- CH = Carbon–Hydrogen composite
- CS = Carbon–Sulfur composite

The core therefore contains exactly:

- 2 CM units
- 2 CH units
- 2 CS units

## 17.2 Core geometry

The core forms a closed structural boundary around a central cavity.

The geometry is derived from the actual bounding geometry of the constituent resource shapes rather than assuming equal-sized units or arbitrary equal angular spacing.

Carbon faces toward the hollow center and the partner resource occupies the outward side of each paired core unit.

The exact geometry must be solved from the physical dimensions of the constituent materials.

## 17.3 Core cavity

The central cavity begins empty.

No water or other material is placed inside the cavity during seed initialization.

Water and other material may interact with core surfaces according to ordinary physical rules, but the cavity itself remains a defined hollow region unless later physical processes fill it.

## 17.4 Core integrity

The core is essential.

If any required core piece is broken such that the closed core integrity is lost, the organism dies immediately.

Core integrity requires:

- all six required units to exist,
- unique core membership,
- required closed connectivity among the six units,
- each core unit maintaining the required core connections,
- one connected core component.

External bonds to non-core material are allowed and do not by themselves invalidate the core.

---

# 18. Seed Membrane

The membrane is the physical outer boundary of the seed cell.

It is:

- significantly weaker than the core,
- exposed to the environment,
- an interface for material exchange,
- composed of physical material rather than a magic boundary statistic.

The membrane must not occupy the core's hollow cavity.

The membrane's inner boundary is defined relative to the outer boundary of the core.

### Membrane thickness

The exact final biological rule for membrane thickness is **NOT YET DECIDED**.

Current geometry code may use conservative material-envelope calculations as implementation groundwork, but a current implementation formula must not be mistaken for a locked biological law.

The final membrane construction rule must emerge from explicit physical packing/material constraints rather than an arbitrary constant.

---

# 19. Seed Interior

The cell interior is the physical region **between the outer surface of the core and the inner surface of the membrane**.

This distinction is mandatory:

> The interior is not the core cavity.

The core cavity is intentionally hollow.

The actual cell interior is the annular/intervening material region enclosed by the membrane.

The interior should contain dense material with very little unnecessary empty space, subject to physical packing and geometry.

The interior may contain:

- structural material,
- water,
- stored material,
- other physically present materials.

There is no abstract hollow cytoplasm layer.

Growth occurs by physical acquisition and organization of material.

---

# 20. Storage

Storage is physical.

Stored material occupies designated physical space within the organism.

Storage does not discriminate merely because a material is bonded or unbonded.

All compatible material types can be stored when sufficient physical space and access exist.

Storage capacity should therefore emerge from geometry and organization rather than an arbitrary biological capacity stat wherever practical.

---

# 21. Water Distribution Inside the Cell

Water is distributed through the physical cell interior rather than held in one dedicated abstract compartment.

Water:

- occupies physical space,
- contacts internal material,
- contacts/interacts with the membrane,
- participates in chemistry,
- can move through available physical pathways,
- influences effective reactivity through ordinary interaction.

Water does not have a predefined fixed number of connection sites.

A fluid region has as many meaningful connection opportunities as its actual physical contacts and geometry allow.

The current `Form::Fluid` / `ConnectionSites::Undetermined` architecture is acceptable as a starting representation, but runtime fluid contacts must eventually be derived from actual geometry/contact rather than a hard-coded connection count.

---

# 22. Connection Geometry and Contact

Connection opportunities are derived from physical material geometry.

There must not be a second independent connection-point representation that can drift away from the resource's actual geometry.

A connection is meaningful only where the participating materials can physically contact/connect.

Contact geometry can influence:

- whether interaction is possible,
- interaction magnitude,
- bond formation,
- structural integrity,
- movement constraints,
- acquisition/access.

---

# 23. COMBINE

COMBINE is a physical/chemical transformation in which compatible material participants become bonded into a composite structure.

COMBINE is not merely an inventory operation.

It must respect:

- participant identity,
- physical contact/arrangement,
- geometry,
- material properties,
- energetic direction,
- work/energy accounting,
- resulting structure.

## 23.1 Destination before transformation

The intended destination/location of the resulting material must be selected **before COMBINE occurs**.

The conceptual sequence is:

1. perceive candidate materials,
2. decide intended use and location,
3. physically arrange participants,
4. perform COMBINE.

Therefore geometry and arrangement are part of the decision-making problem.

A COMBINE result cannot simply appear in an arbitrary location after the action.

---

# 24. COMBINE Energy Model — LOCKED

The COMBINE interaction follows the settled E-model:

> **Potential energy establishes energetic direction; reactivity and geometry modify magnitude.**

Potential-energy difference establishes whether the interaction is energetically favorable or unfavorable.

Reactivity controls how strongly the materials tend to interact, using the nonlinear/exponential reactivity model.

Geometry/contact modifies the effective magnitude according to physical arrangement.

The exact implementation equation may be refined as long as it preserves these causal relationships.

COMBINE consumes energy from the interaction ledger when work is required.

There is no pre-existing organism energy pool from which an arbitrary amount is simply subtracted.

---

# 25. Surplus Investment and Bond Strength — LOCKED

When an interaction has surplus available energy after required work, surplus may be invested into the resulting bond.

The conversion from surplus investment into bond strength follows a **capped diminishing-returns curve**.

This is intentional.

Bond strength must not grow without bound linearly with surplus energy.

The precise mathematical parameterization may evolve during implementation/testing, but the following are locked:

- surplus can increase bond strength,
- returns diminish,
- bond strength is capped.

---

# 26. BREAK — LOCKED

BREAK removes or weakens an existing structural/chemical bond according to the current structural and chemical state.

BREAK energy is not universally one-directional.

Depending on current state, BREAK may:

- consume usable energy, or
- release usable energy.

The result depends on the actual current state of the material and bond rather than a fixed universal BREAK cost.

BREAK must therefore use current bond/structural/chemical information when determining its energetic outcome.

---

# 27. Energy Ledger

Usable energy is an emergent accounting quantity.

It may exist as transient/held usable energy when generated by physical interactions, but it is not a fundamental environmental resource.

Energy can arise from processes such as favorable bond-breaking or other explicitly defined transformations.

Energy can be consumed by processes such as unfavorable combination/work.

The simulation must never allow arbitrary energy creation through bookkeeping bugs, duplicated accounting, or direct resource-to-energy conversion that bypasses chemistry.

A legacy mutable `energy_content` field on materials is prohibited.

---

# 28. Material Conservation

Material is conserved except where a later explicitly defined physical process justifies a change in representation or system boundary.

COMBINE rearranges/bonds existing material; it does not create new resource mass.

BREAK separates existing material; it does not destroy constituent matter.

Vents transfer existing environmental material.

Acquisition moves material across the organism/environment boundary.

Storage moves/organizes existing material.

Derived energy values must not be confused with material mass.

---

# 29. Organism Structure

An organism is a connected physical structure composed of material units and bonds.

The structure system must support both:

- individual base-material units, and
- composite structural materials.

Structural identity must preserve constituent composition.

The architecture should not require a new biological resource type for every possible composite.

Core, membrane, interior, and storage are physical organizational roles, not excuses to create unrelated hidden resource systems.

---

# 30. Growth

Growth is physical acquisition and organization.

The organism becomes larger or more structurally developed by acquiring and arranging matter according to ordinary access, geometry, storage, bonding, and structural rules.

Growth should not be implemented as simply increasing a radius or mass statistic while material geometry remains unchanged.

Derived organism dimensions may be calculated from actual material structure.

---

# 31. Maintenance and Damage

Maintenance must operate on the actual current organism state.

Damage can alter:

- bonds,
- geometry,
- access,
- material distribution,
- structural integrity,
- energetic state.

A structure that loses essential core integrity dies immediately.

Other structural damage does not automatically imply death unless an explicitly defined viability requirement is violated.

---

# 32. Development

Development is a higher-level biological organization process built on physical structure.

Development stages may represent recognizable organismal states, but they must not bypass physical requirements.

A developmental milestone should be reached because the underlying physical/biological conditions exist, not because a hidden counter simply crossed an arbitrary threshold.

---

# 33. Memory and Decision-Making

Memory stores information only when it can influence future behavior.

Decision-making should select among physically meaningful actions using information available through perception and internal state.

The organism does not have omniscient knowledge.

The geometry and destination of intended physical actions are part of the decision problem.

---

# 34. Movement

Movement is physical and has consequences.

Movement must account for:

- organism geometry,
- environmental contact,
- available pathways,
- energetic/work consequences,
- surrounding material/organisms.

Movement should not teleport the organism or ignore physical occupancy.

---

# 35. Reproduction

Reproduction is a physical biological process built on the organism's current structure and state.

It must ultimately require sufficient physical material, structural organization, and usable energy according to the final reproduction rules.

The initial seed is not reproduced through a special bootstrap rule.

Offspring should be constructed from real acquired material and inherited biological information.

Genetic variation must affect heritable traits without creating arbitrary new physical laws.

---

# 36. Genetics

The genome controls heritable biological traits.

Genetic variation can affect decisions, perception, movement, organization, development, and other heritable systems.

Genome values should influence physical outcomes through the ordinary biological systems they control rather than directly spawning resources or energy.

---

# 37. Order of Operations

The simulation tick should preserve causal ordering.

At a high level:

1. environment/material state is updated,
2. organisms perceive their available physical surroundings,
3. decisions are made from available information,
4. physical movement/arrangement occurs,
5. acquisition/expulsion occurs where physically valid,
6. chemical/structural interactions occur,
7. bonds and material state update,
8. energy ledger updates from actual interactions,
9. structural integrity and viability are evaluated,
10. development/reproduction are evaluated from the resulting state,
11. memory and history update.

Exact scheduling can be refined during implementation, but causal dependencies must not be reversed.

---

# 38. Biological Shortcuts That Are Prohibited

The following patterns are prohibited unless explicitly justified and added to this specification:

- arbitrary stored organism energy generated independently of chemistry,
- `energy_content` on materials,
- magic permeability statistics,
- magic water pools,
- fixed water connection counts,
- hard-coded finite chemistry recipes as the primary chemistry engine,
- teleporting acquired resources into cells,
- growth by changing only a radius/mass variable,
- arbitrary COMBINE result placement after the decision,
- treating the core cavity as ordinary interior storage,
- making the membrane stronger/equal to the core without a physical reason,
- creating a new resource identity for every bonded composite,
- omniscient perception,
- biological systems that bypass the material/geometry model merely for convenience.

---

# 39. Testing Requirements

Tests should verify causal rules, not merely implementation details.

Required categories include:

## Resource tests

- immutable intrinsic properties,
- derived composite properties,
- potential-energy derivation,
- nonlinear reactivity behavior,
- water dilution behavior.

## Geometry tests

- resource geometry validity,
- composite geometry,
- contact validity,
- core geometry closure,
- membrane boundary relationship,
- fluid contact derivation.

## Core tests

- exact six-unit composition,
- exact CM/CH/CS counts,
- closed geometry,
- positive hollow cavity,
- intact core detection,
- failure when a required core piece/bond is broken,
- survival/death integration.

## Chemistry tests

- COMBINE energetic direction,
- reactivity magnitude influence,
- geometry magnitude influence,
- work consumption,
- surplus investment,
- capped diminishing returns,
- BREAK state-dependent energy direction.

## Conservation tests

- constituent identities preserved through COMBINE,
- BREAK preserves matter,
- vents preserve material state,
- acquisition does not duplicate matter,
- no duplicate energy creation.

## Organism tests

- seed initializes viable,
- core cavity begins empty,
- interior exists between core and membrane,
- water can occupy/interact with the interior,
- membrane is weaker than core according to the actual rule,
- core failure causes immediate death.

---

# 40. Current Implementation State

As of V5, the project has implementation groundwork for several major systems.

Implemented groundwork includes:

- composite `StructuralMaterial` representation,
- constituent identity preservation,
- internal bond representation,
- exact six-unit F-core geometry derivation,
- core integrity checking,
- membrane geometry groundwork,
- resource-specific physical geometry,
- separate bond energy state,
- two-layer environmental resource architecture.

However, groundwork is not equivalent to full integration.

Known integration gaps include:

- replacing/integrating legacy single-resource `StructuralUnit` assumptions with composite structural material support,
- constructing the actual seed organism from the completed core/membrane/interior specification,
- wiring core integrity failure to organism death,
- completing physical interior/material packing,
- deriving actual fluid contacts dynamically,
- integrating physical acquisition and storage,
- completing COMBINE runtime energy accounting,
- completing BREAK runtime energy accounting,
- integrating membrane behavior without introducing a magic permeability statistic,
- replacing any remaining legacy energy bookkeeping,
- validating the complete simulation through compilation and tests.

These are implementation tasks, not invitations to reopen the underlying locked design decisions.

---

# 41. Phase State

**Phase 6 — CLOSED.**

Phase 6 established the chemistry/energy/material foundations necessary for the next biological layer.

**Phase 7 — SEED CONSTRUCTION / BIOLOGICAL INTEGRATION — ACTIVE.**

Current checkpoint progress:

- Checkpoint 1 — composite structural material representation: implemented groundwork.
- Checkpoint 2 — exact six-unit F-core geometry: implemented groundwork.
- Checkpoint 3 — core integrity: implemented groundwork.
- Checkpoint 4 — membrane geometry: implemented groundwork; final membrane thickness law remains open.

The next implementation work is to integrate these systems into the actual viable seed organism without introducing shortcuts that violate this specification.

---

# 42. Open Boundaries

The following remain deliberately open unless and until explicitly locked:

1. exact final membrane thickness/construction rule,
2. complete physical packing algorithm for dense interior material,
3. exact dynamic fluid-contact algorithm,
4. exact final numerical form of the COMBINE equation where multiple valid formulations preserve the locked E-model,
5. exact numerical parameters for the capped diminishing-returns bond-strength curve,
6. detailed long-term reproduction thresholds and constraints where not already independently locked.

Open boundaries must not be filled with arbitrary constants merely to make an implementation convenient.

When an open boundary is reached, it should be surfaced as a design decision rather than silently converted into a permanent rule.

---

# 43. Implementation Rule

Implementation should proceed from the actual repository architecture outward.

Do not design an abstract replacement system and then force the existing code to match it without inspection.

For each checkpoint:

1. inspect the current repository implementation,
2. identify the smallest architecture change that expresses the locked rule,
3. implement it,
4. compile/test when possible,
5. inspect integration points,
6. record any genuinely unresolved design boundary separately.

A difficult implementation is not evidence that a locked design decision should be reopened.

---

# 44. Final Principle

EvoSim should become more interesting as the rules become more general, not as more exceptions are added.

The intended progression is:

**physical resources → physical interactions → chemical structure → viable cell architecture → biological organization → decision-making → evolution**

The organism should survive because its structure and chemistry work, grow because it physically acquires and organizes matter, reproduce because it reaches a physically viable state, and evolve because heritable differences alter those outcomes.

That is the governing model of EvoSim V5.
