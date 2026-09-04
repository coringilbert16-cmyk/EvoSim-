# EvoSim — Comprehensive Viable Simulation Implementation Plan

**Status:** Active implementation plan  
**Last revised:** 2026-09-04  
**Authority:** Engineering roadmap; `Master Spec Sheet V5.md` remains the authoritative simulation specification.  
**Working method:** Audit first, implement one coherent change at a time, test it, integrate it, then re-audit before proceeding.

---

## 1. The Goal

The goal is to turn EvoSim into a **viable, continuously running artificial-life simulation** rather than a collection of individually functioning mechanics.

A successful simulation must be able to sustain the complete causal loop:

```text
DEEP ENVIRONMENT
      ↓
vents / diffusion / settling
      ↓
ACTIVE ECOLOGICAL FIELD
      ↓
physical material availability
      ↓
organism perception
      ↓
organism decision
      ↓
physical material interaction
      ↓
chemistry / transformations
      ↓
physical organism structure
      ↓
growth / maintenance / repair
      ↓
maturity / reproductive readiness
      ↓
reproduction
      ↓
structural + genetic inheritance
      ↓
new viable organism
      ↓
survival / variation / selection
      └──────────────→ next generation
```

The simulation must be capable of running this loop repeatedly without scripted life, fake activity, or UI-driven behavior.

The UI is an **observation layer only**. It displays authoritative simulation state; it does not become a second simulation engine.

### The actual success criterion

We are not finished when:

- the browser animates;
- ticks advance;
- unit tests pass;
- COMBINE works in isolation;
- BREAK works in isolation;
- reproduction works from hand-built test state; or
- organisms merely move around.

We are finished when the **integrated simulation itself** can naturally produce and sustain valid organism lifecycles and population dynamics under the established rules.

---

# 2. Architectural Decisions That Are Locked

These decisions are not implementation suggestions. They are the current governing rules for the implementation.

## 2.1 Chemistry is authoritative

EvoSim is chemistry/material-first. Fundamental physical properties belong to the resource/material model rather than being invented by organism behavior or UI logic.

Base resources are:

- Carbon
- Methane
- Hydrogen
- Sulfur
- Nitrogen
- Phosphorus
- Water

Resource properties are intrinsic and immutable:

- mass;
- potential energy;
- reactivity;
- cohesion;
- physical form/geometry.

Potential energy is derived from the authoritative resource/material properties. Legacy independent energy fields must not become a second authority.

Reactivity remains nonlinear/exponential according to the established chemistry model.

---

## 2.2 Bond strength has exactly one authority

**Locked rule:**

> Bond strength is calculated only from the constant mathematical properties of the resources participating in the bond. No age, geometry, organism state, history, stored legacy value, load, or other dynamic input may alter intrinsic bond strength.

Dynamic quantities such as connection load may use intrinsic bond strength as an input, but they must never redefine it.

Any remaining stored `Bond.strength` field must therefore be treated as non-authoritative or removed when the implementation reaches that cleanup point.

---

## 2.3 Bonded/unbonded is chemical state, not location

Bonded versus unbonded material describes chemical/structural state. It does not mean that a material belongs to a particular anatomical region.

There must be no permanent mappings such as:

```text
Carbon = core
Water = interior
X = membrane
```

The same resource identity can participate in different physical roles depending on inherited structure and chemistry.

---

## 2.4 Water is ordinary physical material

Water is first-class material.

It can be:

- acquired;
- stored;
- transported;
- distributed through organism material-space;
- in contact with material and membrane;
- transferred across material boundaries when physical conditions permit;
- involved in reactivity through the established chemistry model.

There is no special `organism.water` pool and no magic permeability stat.

Permeability is a **transfer capacity/rate**, not an organism preference.

The currently selected permeability model is:

```text
W = physically accessible organism water mass
WT = threshold
Wmax = full-permeability water level
Pmax = maximum transfer capacity

P(W) = 0                       when W < WT
P(W) = linear WT → Wmax       when WT ≤ W < Wmax
P(W) = Pmax                    when W ≥ Wmax
```

Water occupies physical volume like other material. Its geometry may conform to available space while preserving volume.

---

# 3. Environment Architecture

The environment has two scales:

```text
DEEP RESERVOIR
    ↓ vents
ACTIVE ECOLOGICAL FIELD
    ↓ physical interaction
ORGANISMS
```

## Deep reservoir

The deep reservoir is unified. It does **not** maintain separate bonded/unbonded inventories.

## Vents

Locked rule:

> Reservoir material released by a vent enters the active field regardless of bonding state. Venting does not alter the existing bonding state of active material.

The implementation must preserve material accounting through venting.

## Active field

The active field may distinguish bonded and unbonded material where ecological mechanics require that distinction, particularly for interactions involving BREAK.

The distinction must not leak backward into the unified deep reservoir.

---

# 4. The Most Important Biological Architecture Change

The organism currently has too much procedural structural authority.

The implementation is being changed from:

```text
organism behavior
    ↓
arbitrary COMBINE / BREAK / placement
    ↓
new body shape
```

to:

```text
genome
  ↓
inherited structural blueprint
  ↓
physical construction
  ↓
organism body
```

This is the central architectural change required for the next implementation stage.

---

# 5. Locked Rules for Living Structure

## 5.1 The cell cannot reorganize itself

An organism cannot intentionally redesign its body during its lifetime.

It cannot:

- freeform redesign itself;
- deliberately change its topology;
- intentionally break its own structural bonds to redesign itself;
- move existing structural pieces into a new arrangement;
- use arbitrary COMBINE to invent a new body plan.

The organism's inherited structure is therefore a constraint on lifetime construction, not a suggestion.

## 5.2 Growth is allowed

A cell can grow.

Growth means adding or replacing physical material **within the inherited structural configuration**.

Growth may:

- add permitted structural material;
- extend an inherited element/configuration;
- fill an expected missing portion of the inherited body;
- increase structural mass toward a genetically determined maximum.

Growth may not invent a new topology or reposition existing material.

The juvenile begins as a seed-sized viable organism, approximately 40% of eventual maximum size as a design target, but **40% is not a universal hard-coded rule**.

## 5.3 Cells cannot grow indefinitely

Maximum structural size is genetically determined.

The implementation must distinguish:

- maturity threshold;
- juvenile starting size;
- maximum permitted structural capacity.

The current `adult_mass` trait must not remain the sole authority for maximum body size.

## 5.4 Repair is allowed

Physical damage may break bonds or remove material.

A damaged organism may repair the damage and replace lost material.

Repair must restore the inherited structural configuration. It is not a hidden redesign mechanism.

Repair therefore follows:

```text
damage
  ↓
detect deviation from inherited blueprint
  ↓
identify required missing material / relationship
  ↓
obtain replacement material
  ↓
restore inherited configuration
```

Repair must not use unrestricted self-directed COMBINE/BREAK as a way to invent new topology.

## 5.5 Reproduction is the source of structural redesign

A genuinely different inherited body plan arises through reproduction.

The child receives a copy of the parent's structural blueprint, with a small structural mutation.

Locked scale rule:

> Structural change per reproduction should be **5% or less**, in the same broad magnitude as the permitted genome change.

Structural mutation must therefore be local/small and produce a valid viable configuration. Radical body redesign from a tiny genetic change is not permitted.

---

# 6. Structural Blueprint — New Authoritative Model

The organism needs an explicit inherited `StructuralBlueprint`.

It is part of the inherited genotype, not a runtime procedural recipe and not merely a set of phenotype traits such as compactness or branching.

At minimum the blueprint must encode:

### A. Structural elements

The physical structural elements that make up the organism, including their material/composition requirements.

### B. Inter-element topology

Which structural elements are connected and the relationship/connection information required to realize those connections.

### C. Relative geometry

Enough geometry to construct the inherited configuration physically without allowing the organism to invent arbitrary placement.

The blueprint must be serializable as part of the genome/organism state.

### Blueprint authority

The blueprint answers:

> **What body is this organism genetically permitted to build and restore?**

The environment and available material answer:

> **What physical material is currently available to build it?**

The growth/repair executor answers:

> **What permitted physical construction can happen next?**

The organism's decision system must not answer these questions by inventing topology.

---

# 7. Structural Material Model

The code already contains a useful `StructuralMaterial` abstraction supporting:

- constituent material identity;
- composite material;
- internal bonds;
- derived mass/properties;
- validation.

The current `StructuralUnit`, however, stores only a resource name and placement.

This creates two parallel structural representations:

```text
StructuralMaterial → composite-aware
StructuralUnit      → resource-name-only
```

The implementation must converge these into a coherent physical representation.

The intended direction is:

```rust
StructuralUnit {
    material: StructuralMaterial,
    placement: Placement,
}
```

The exact Rust representation may differ if the same invariants are preserved, but a structural unit must be able to physically represent composite structural material.

Internal chemical bonds within a `StructuralMaterial` must remain conceptually distinct from organism-level structural bonds between structural elements.

---

# 8. Connection Geometry Rule

Locked rule:

> **Connection points are not limited in number of possible bonds except in regards to geometry.**

A connection region/site does not have a numerical one-bond or finite-bond capacity.

Multiple bonds may originate from the same physical connection region whenever geometry permits them.

Therefore implementation logic such as:

```text
connection_count == 0
```

must not be used as a universal bond-capacity rule.

Geometry/contact determines whether a bond can exist.

This applies to:

- contact candidate generation;
- connection-site availability;
- COMBINE;
- reproduction construction;
- structural validation.

---

# 9. Core and Membrane Geometry

The existing `core_geometry.rs` and `membrane_geometry.rs` contain useful geometry mathematics, but their current six-unit anatomy must not remain the biological authority.

The current six-unit interpretation:

```text
CM → CH → CS → CM → CH → CS
```

was a guideline used to communicate an early idea, not a hard architecture.

Likewise, a six-unit core is not required.

The code must therefore transition from:

```text
hard-coded six-unit anatomy
```

to:

```text
inherited structural blueprint
        ↓
geometry realization / validation
```

Existing geometry utilities may be retained where useful, but they must become downstream utilities/checkers rather than defining universal organism anatomy.

---

# 10. Seed Organism Rule

The current initial organism is an empty juvenile. That is not acceptable for the intended architecture.

The seed must be a **completed viable cell**.

However, the seed is not required to use one universal six-unit configuration.

The system needs a source/selection mechanism for **multiple viable seed configurations**, allowing different valid starting body plans from the beginning.

Seed configurations are constrained initial conditions. Once the organism exists, its lifetime structure follows the same no-redesign rules as all other organisms.

The seed must include enough physically realized structure and water/material state to be viable under the established rules.

---

# 11. Reproduction and Structural Inheritance

The current reproduction implementation is based on:

- `CORE_UNIT_COUNT = 6`;
- `CORE_MATERIAL_AMOUNT = 6`;
- procedural placement from `construction_compactness` and `construction_branching`.

That is obsolete as the biological authority.

The replacement lifecycle is:

```text
adult organism
      ↓
reproductive readiness
      ↓
commit required material/resources
      ↓
copy parent genome
      ↓
mutate genome
      ↓
copy parent structural blueprint
      ↓
apply ≤5% valid structural mutation
      ↓
construct child from blueprint
      ↓
viable juvenile boundary
      ↓
insert NEW independent organism
```

The child's structural configuration must be inherited rather than procedurally invented from compactness/branching traits.

The child must become an actual member of `Simulation.organisms`; construction is not complete until there is a demonstrated birth boundary.

---

# 12. Genome Changes Required

The genome should continue to contain legitimate heritable physiological/behavioral traits such as perception, memory, processing, movement, resource interaction, and reproductive traits where supported by the model.

The following structural-authority traits are obsolete:

- `construction_compactness`;
- `construction_branching`.

They must not control the child's body topology.

The following concepts must be separated:

```text
juvenile starting size
maturity threshold
maximum structural capacity
```

`adult_mass` currently acts as a maturity threshold and must not silently become the universal maximum-size authority.

The genome therefore needs an explicit, coherent maximum structural capacity representation.

---

# 13. Lifetime Structural Lifecycle

The target lifecycle is:

```text
VIABLE SEED / BIRTH
      ↓
JUVENILE STRUCTURE
      ↓
MATERIAL ACQUISITION
      ↓
BLUEPRINT-CONSTRAINED GROWTH
      ↓
MATURE STRUCTURE
      ↓
MAINTENANCE
      ↓
DAMAGE ↔ REPAIR
      ↓
REPRODUCTIVE READINESS
      ↓
REPRODUCTION
```

At no point may normal lifetime behavior create an unrestricted body-construction loop.

The organism may express inherited structure more fully through growth, and restore it after damage, but it cannot decide to become a different body plan.

---

# 14. COMBINE and BREAK After the Architecture Change

The chemistry/transformation machinery remains valuable.

The distinction is between **chemical/physical mechanics** and **organism authority**.

## COMBINE

The chemistry engine may continue to calculate whether materials can combine and what that interaction costs/produces.

But the organism may not invoke arbitrary COMBINE to redesign its body.

Organism-level structural construction must be mediated by the inherited blueprint.

## BREAK

The low-level ability to remove a bond remains necessary for physical damage and the established transformation mechanics.

But an organism may not intentionally choose BREAK for redesign.

Physical damage may cause bond failure; repair may subsequently restore the inherited structure.

The existing intrinsic bond-strength calculation and BREAK work model should be preserved unless a later chemistry audit proves a separate defect.

---

# 15. Material Acquisition Must Become Physical

The current live simulation has acquisition disabled (`can_acquire = false`) and the Acquire/Expel dispatch path is effectively a no-op.

This blocks the central environment → organism causal link.

The required path is:

```text
environment material
      ↓
physical proximity / overlap / contact
      ↓
perception
      ↓
decision
      ↓
physical transfer
      ↓
organism material-space
```

Acquisition must not be a distant arbitrary transfer or a hidden resource counter increment.

Water/material movement must use the physical organism-space model rather than magic pools.

---

# 16. Organism Physical Space

The current organism representation lacks a sufficiently explicit model for:

- internal material occupancy;
- water distribution;
- inside/outside relation;
- membrane boundary;
- interior material-space;
- physical contact between internal material and boundaries.

This must be addressed as the material-acquisition and permeability implementation proceeds.

The intended conceptual structure is:

```text
OrganismStructure
├── inherited structural elements
├── inter-element structural bonds
├── outer boundary / membrane geometry
└── interior physical material-space
       ├── structural material
       ├── water
       └── other stored/active material
```

Core, interior, and membrane are conceptual physically distinct components under one organism structural umbrella; they are not separate competing organism-level structures.

---

# 17. Required Implementation Changes

The following are the known major code changes required to reach the goal.

### Structural authority

- Introduce inherited `StructuralBlueprint`.
- Make blueprint part of serialized inherited state.
- Define structural elements, topology, and relative geometry.
- Add blueprint validation.
- Replace six-unit core authority.
- Remove structural authority from `construction_compactness` / `construction_branching`.
- Replace procedural reproduction placement with blueprint construction.

### Physical structural representation

- Make `StructuralUnit` material-bearing and composite-capable.
- Preserve internal bonds within `StructuralMaterial` separately from organism-level bonds.
- Ensure placement comes from blueprint/growth/repair rules rather than arbitrary organism decisions.

### Geometry/contact

- Remove finite connection-count assumptions.
- Allow multiple bonds from a connection region where geometry permits.
- Generalize contact/occupancy logic beyond corner-only assumptions where required.
- Preserve physical geometry as the constraint on possible bonds.

### Growth

- Implement a blueprint-driven growth executor.
- Define permitted next construction operations from the inherited blueprint.
- Enforce genetically determined maximum structural capacity.
- Ensure growth cannot reposition existing material or invent topology.

### Repair

- Detect structural deviation caused by physical damage.
- Identify missing blueprint elements/relationships.
- Obtain replacement material.
- Restore the inherited configuration.
- Prevent repair from becoming redesign.

### Reproduction

- Replace `CORE_UNIT_COUNT` / `CORE_MATERIAL_AMOUNT` construction authority.
- Clone parent blueprint.
- Apply ≤5% structural mutation.
- Validate mutated blueprint before construction.
- Construct a viable juvenile.
- Insert completed child into the live organism population.

### Initial conditions

- Replace empty initial juvenile with a completed viable seed.
- Provide multiple valid seed configurations.
- Ensure seed contains viable physical structure and sufficient water/material state.

### Environment → organism

- Enable real acquisition.
- Implement physical overlap/contact-based transfer.
- Implement organism material-space.
- Implement water distribution and permeability according to the selected linear-after-threshold model.

### Lifecycle

- Separate juvenile starting size, maturity threshold, and maximum size.
- Establish adult transition.
- Establish maintenance.
- Establish legitimate death/removal.
- Ensure no dead organism can act.

### Chemistry/energy

- Complete repository-wide bond-strength authority audit.
- Remove/relegate legacy stored bond-strength authority.
- Audit every energy mutation from material cause to destination.
- Remove obsolete `energy_content` behavior.
- Verify COMBINE/BREAK conservation and work accounting.

### Integration

- Add runtime invariants.
- Add causal integration tests.
- Add long-run/soak tests.
- Verify snapshots remain observational only.

---

# 18. Implementation Order

Implementation must proceed in this dependency order. We do **not** skip ahead because a later subsystem looks easier.

```text
1. STRUCTURAL BLUEPRINT DATA MODEL
              ↓
2. STRUCTURAL MATERIAL → PHYSICAL STRUCTURAL UNIT
              ↓
3. GENERIC GEOMETRY / CONTACT
   (including unlimited bond count subject to geometry)
              ↓
4. BLUEPRINT VALIDATION
              ↓
5. VIABLE SEED ORGANISM CONSTRUCTION
              ↓
6. BLUEPRINT-CONSTRAINED GROWTH
              ↓
7. BLUEPRINT-CONSTRAINED REPAIR
              ↓
8. REPRODUCTION + ≤5% STRUCTURAL MUTATION
              ↓
9. COMPLETED CHILD BIRTH / POPULATION INSERTION
              ↓
10. PHYSICAL ENVIRONMENT → ORGANISM ACQUISITION
              ↓
11. ORGANISM INTERNAL MATERIAL / WATER OCCUPANCY
              ↓
12. MATURITY / MAXIMUM-SIZE / MAINTENANCE / DEATH
              ↓
13. ENERGY LIFECYCLE AUDIT AND CORRECTION
              ↓
14. POPULATION VIABILITY / LONG-RUN VALIDATION
```

This order exists because each stage depends on the one before it.

For example, reproduction should not be rebuilt before the child body has an authoritative blueprint representation.

---

# 19. Detailed Phase Gates

Every phase has a proof requirement.

## Phase 1 — Structural Blueprint

**Goal:** establish the authoritative inherited body plan.

Must prove:

- blueprint serializes;
- blueprint describes real structural elements;
- topology is explicit;
- relative geometry is explicit;
- blueprint validation catches invalid structures;
- no six-unit requirement is embedded.

**Gate:** a valid blueprint can be created, serialized, validated, and inspected without relying on procedural compactness/branching.

---

## Phase 2 — Physical Structural Material

**Goal:** eliminate the split between composite `StructuralMaterial` and resource-name-only `StructuralUnit`.

Must prove:

- composite material can become a physical structural element;
- material identity and quantity remain authoritative;
- internal chemistry is preserved;
- organism-level structural bonds remain distinct.

**Gate:** a blueprint-defined composite element can be physically represented without losing material composition.

---

## Phase 3 — Geometry and Contact

**Goal:** make physical geometry, rather than connection-count bookkeeping, the bond constraint.

Must prove:

- multiple bonds can originate from a connection region where geometry permits;
- invalid geometric overlaps remain invalid;
- contact generation is physically meaningful;
- no universal `connection_count == 0` rule remains.

**Gate:** geometry alone determines whether a candidate connection is physically possible.

---

## Phase 4 — Blueprint Validation

**Goal:** guarantee that inherited body plans are constructible and viable.

Validation must cover:

- valid element references;
- topology consistency;
- geometry consistency;
- material requirements;
- required connectivity;
- absence of impossible overlaps;
- valid seed/juvenile form;
- valid maximum-growth representation.

**Gate:** invalid blueprints fail before construction rather than corrupting runtime state.

---

## Phase 5 — Seed Construction

**Goal:** start the simulation with a real viable organism.

Must prove:

```text
seed blueprint
    ↓
physical construction
    ↓
viable structure
    ↓
valid initial material/water state
    ↓
juvenile organism
```

Multiple valid seed configurations must be possible.

**Gate:** the first organism is not an empty placeholder.

---

## Phase 6 — Growth

**Goal:** allow development without self-redesign.

Must prove:

- growth follows blueprint;
- available material is consumed correctly;
- existing material is not repositioned;
- topology is not invented;
- maximum structural capacity is enforced;
- growth can progress over multiple ticks.

**Gate:** a juvenile can become a larger expression of the same inherited body plan.

---

## Phase 7 — Repair

**Goal:** recover from physical damage without redesign.

Must prove:

- damage can create a structural deficit;
- the deficit can be identified;
- replacement material can be acquired;
- the inherited configuration can be restored;
- repair cannot create a new topology.

**Gate:** damage → repair returns the organism toward its inherited structure.

---

## Phase 8 — Reproduction and Structural Mutation

**Goal:** make reproduction the source of inherited structural variation.

Must prove:

- parent blueprint is inherited;
- structural mutation is ≤5%;
- mutation is local/small;
- mutated blueprint remains valid;
- genome and structure change on compatible scales;
- no procedural compactness/branching body generation remains.

**Gate:** offspring can differ structurally without arbitrary body-plan jumps.

---

## Phase 9 — Birth

**Goal:** complete the reproduction lifecycle.

Must prove:

```text
parent
 → reproductive commitment
 → child construction
 → viable juvenile
 → new organism entry
```

The completed child must become independently simulated.

**Gate:** reproduction naturally increases organism count when conditions allow it.

---

## Phase 10 — Environment → Organism

**Goal:** close the physical material acquisition loop.

Must prove:

- material can reach organism-accessible space;
- acquisition requires physical contact/overlap as appropriate;
- ownership changes are conserved;
- water participates as ordinary material;
- transfer capacity follows the selected permeability model.

**Gate:** a normally running organism can obtain real environmental material.

---

## Phase 11 — Internal Material and Water

**Goal:** establish physical organism-space.

Must prove:

- internal material occupies physical space;
- water can distribute through the organism;
- inside/outside relation is meaningful;
- membrane geometry constrains transfer;
- no magic water pool or magic permeability variable becomes the hidden authority.

**Gate:** internal material movement can be explained by physical state and geometry.

---

## Phase 12 — Full Organism Lifecycle

**Goal:** make organisms genuinely living entities.

Must prove:

```text
birth → juvenile → growth → mature → maintenance
→ damage/repair → reproduction or death
```

Must separately establish:

- maturity;
- maximum size;
- maintenance;
- death/removal;
- action eligibility for living organisms only.

**Gate:** organisms can naturally live, develop, reproduce, and die.

---

## Phase 13 — Energy Audit

**Goal:** prove energy remains an emergent consequence of material interactions.

For every energy mutation document:

```text
source material
 → physical/chemical cause
 → calculation
 → destination
 → resulting material/structural consequence
```

Must find and resolve any remaining independent energy authority.

**Gate:** no unexplained energy creation, accumulation, or disappearance remains.

---

## Phase 14 — Population and Long-Run Viability

**Goal:** demonstrate the complete artificial-life loop.

Run increasingly long integrated simulations and check for:

- stable ticking;
- valid numeric state;
- no runaway memory;
- no stale transformations;
- no duplicate ownership;
- viable birth/death turnover;
- material conservation;
- plausible resource limitation;
- heritable variation;
- population persistence where the model supports it;
- evolutionary change where selection pressure exists.

No particular population curve is prescribed.

**Gate:** the system can run for long periods and produce endogenous population dynamics rather than scripted activity.

---

# 20. Legacy Systems to Remove or Demote

The following are known legacy authorities that conflict with the current architecture.

### Must no longer define organism structure

- `CORE_UNIT_COUNT` as universal anatomy;
- `CORE_MATERIAL_AMOUNT` as universal body construction;
- `CorePairKind` / `F_SEQUENCE` as universal organism topology;
- six-unit `CoreIntegrity` assumptions;
- `construction_compactness` as body-plan authority;
- `construction_branching` as body-plan authority;
- arbitrary organism-driven structural COMBINE;
- organism-driven BREAK for redesign;
- arbitrary placement of new structural units.

### May remain as utilities if made non-authoritative

- core geometry calculations;
- membrane geometry calculations;
- low-level COMBINE mechanics;
- low-level BREAK mechanics;
- structural-material chemistry;
- connection/contact geometry utilities.

The principle is **not** “delete everything old.” The principle is:

> Preserve correct reusable mechanics, but remove obsolete biological authorities.

---

# 21. Testing Strategy

Tests must operate at three levels.

## Unit tests

Verify local mathematical and data-model contracts:

- resource properties;
- material composition;
- structural material;
- bond strength;
- geometry;
- blueprint validation;
- structural mutation limits.

## Integration tests

Verify causal chains:

```text
environment → organism material
material → growth
structure → damage → repair
parent → reproduction → child
child → independent organism
```

## Long-run tests

Verify system behavior over many ticks:

- no corruption;
- no stale state;
- no runaway resources;
- no unexplained energy;
- population lifecycle works;
- snapshots remain observational.

A passing unit test never substitutes for an integrated lifecycle test.

---

# 22. Implementation Discipline

Every implementation change follows this sequence:

```text
AUDIT
  ↓
TRACE ACTUAL RUNTIME PATH
  ↓
IDENTIFY FIRST BROKEN LINK
  ↓
STATE THE INVARIANT
  ↓
DESIGN THE SMALLEST COHERENT CHANGE
  ↓
IMPLEMENT ONE CHANGE
  ↓
FOCUSED TESTS
  ↓
FULL TESTS
  ↓
INTEGRATED RUNTIME CHECK
  ↓
RE-AUDIT ADJACENT PATHS
  ↓
NEXT CHANGE
```

### Rules

1. **One step at a time.**
2. Do not bundle unrelated fixes.
3. Do not tune constants to hide an architectural failure.
4. Do not add fake activity to make the UI look alive.
5. Do not preserve a contradictory legacy authority merely because existing tests depend on it.
6. Do not declare a phase complete from unit tests alone.
7. Re-audit after every structural change because structural state is shared by chemistry, transformations, reproduction, geometry, and serialization.

---

# 23. Current Starting Point

The current codebase has useful working foundations:

- environment reservoir/active-field architecture;
- vent migration rules;
- resource-property model;
- composite `StructuralMaterial`;
- structural bonds and connection-load calculations;
- multi-tick transformations;
- corrected intrinsic bond-strength usage in important paths;
- serialized organism/genome/snapshot state;
- an existing simulation tick loop.

But several causal links remain incomplete or contradictory.

The most important current blockers are:

1. no authoritative inherited structural blueprint;
2. six-unit/procedural construction remains in reproduction and core validation;
3. `StructuralUnit` is not yet composite-material-bearing;
4. connection points still have finite-capacity logic;
5. no blueprint-driven growth lifecycle;
6. no production repair lifecycle;
7. organism-controlled structural COMBINE/BREAK can still contradict the no-redesign rule;
8. initial organism is not a completed viable seed;
9. reproduction does not yet demonstrably cross the birth boundary into a new live organism;
10. environmental acquisition is disabled/no-op;
11. internal physical material/water-space is incomplete;
12. maturity, maximum size, maintenance, and death are not yet a complete live lifecycle;
13. energy authority still requires a final repository-wide audit.

These are implementation blockers, not reasons to redesign the entire simulation.

---

# 24. Immediate Next Step

**Do not begin by rewriting reproduction, growth, or the core.**

The first implementation step is:

> **Design and implement the authoritative `StructuralBlueprint` data model and its validation contract.**

Before code is changed, audit the exact existing genome/structure/serialization interfaces that the blueprint must fit into.

Then implement only that first coherent step, add focused tests, run the full test suite, and re-audit.

The next step is not authorized merely because the first code compiles. It is authorized when the blueprint has become a demonstrably valid, serializable, inherited structural authority.

---

# 25. Final Architectural Picture

The target EvoSim architecture is:

```text
                    GENOME
                      │
                      ├───────────────┐
                      │               │
                      ↓               ↓
            STRUCTURAL BLUEPRINT   PHYSIOLOGICAL TRAITS
                      │               │
                      │               ├── perception
                      │               ├── behavior
                      │               ├── maintenance
                      │               └── reproduction
                      ↓
              BLUEPRINT VALIDATION
                      ↓
              PHYSICAL STRUCTURE
             ┌────────┼─────────┐
             │        │         │
          elements  topology  geometry
             │        │         │
             └────────┼─────────┘
                      ↓
             MATERIAL / CHEMISTRY
                      ↕
              ENVIRONMENT FIELD
                      ↕
                WATER / TRANSFER
                      ↓
                 GROWTH / REPAIR
                      ↓
                 LIFE HISTORY
                      ↓
                REPRODUCTION
                      ↓
          BLUEPRINT COPY + ≤5% MUTATION
                      ↓
                 NEXT GENERATION
```

The key invariant is:

> **An organism expresses an inherited body plan; it does not invent one during its lifetime.**

Growth and repair change the organism's physical condition without granting it freeform redesign. Reproduction is the mechanism through which structural variation enters the population. Chemistry determines material interactions, geometry determines physical possibility, and the environment determines what material is available.

That is the path from the current codebase to a continuously running artificial-life simulation.