# EvoSim — Master Specification 4.1

**Status:** Authoritative architectural specification
**Supersedes:** Master Specification 4.0 where explicitly changed below
**Purpose:** Define the current EvoSim architecture and locked behavior without prematurely fixing unresolved mathematical equations.

---

## 1. Project Identity

EvoSim is an evolutionary simulation designed to produce believable emergent behavior from the minimum information necessary.

The simulation is not a numbers-go-up game and must not become a collection of scripted life stages or hard-coded ecological roles.

Core principle:

> Calculate the minimum amount of information necessary to produce the desired emergent behavior.

Complex behavior should arise from immutable physical properties, organism traits, environmental constraints, interactions, consequences, and selection.

---

## 2. Fundamental Systems

The simulation has three fundamental systems:

1. Environment
2. Organisms
3. Resources

Other structures should emerge from interactions among these systems where practical.

---

## 3. Resource Model — LOCKED

Every base resource has four immutable properties:

- mass
- potential energy
- reactivity
- cohesion

These are intrinsic properties of resource types. They do not evolve and are not organism traits.

Current conceptual catalog:

| Resource | Mass | Potential Energy | Reactivity | Cohesion |
|---|---:|---:|---:|---:|
| Carbon | 1.0 | 1.0 | 0.1 | 0.95 |
| Methane | 1.0 | 20.0 | 4.0 | 0.1 |
| Hydrogen | 1.0 | 12.0 | 3.0 | 0.05 |
| Sulfur | 1.0 | 8.0 | 2.0 | 0.4 |
| Nitrogen | 1.0 | 0.5 | 0.2 | 0.7 |
| Phosphorus | 1.0 | 0.8 | 0.3 | 0.6 |
| Water | 1.0 | 0.0 | 0.0 | 0.5 |

Water is a neutral/diluting influence on reactivity rather than an energy source.

### Potential Energy

Potential energy belongs to the raw resource state.

It is an intrinsic property and is derived from resource type and amount. It is not a mutable energy bank stored inside `Material`.

Processing does not reduce the intrinsic potential-energy property of a resource type.

Energy itself is not a fundamental resource.

---

## 4. Raw Material vs Bonded Structure — LOCKED

Raw/unbonded material cannot BREAK.

Raw material must first participate in a successful COMBINE operation before it can exist as bonded structure and subsequently be broken.

The current architecture uses discrete structural units and bonds inside organisms rather than treating an entire bonded compound as an indivisible bulk material.

Bulk environmental material remains a transfer/storage representation. Structural complexity exists inside the organism.

---

## 5. Structural Model — LOCKED

An organism structure consists of discrete `StructuralUnit` instances and a flat list of `Bond` edges.

Each structural unit identifies a resource type and obtains its immutable physical properties and connection geometry from the resource catalog.

A bond connects two specific connection points on two specific units.

Conceptually:

```text
OrganismStructure
├── units[]
│   └── StructuralUnit(resource type)
└── bonds[]
    └── Bond(unit_a, point_a, unit_b, point_b)
```

Existing bonds are preserved when another bond is added.

Multiple bonds may reference a connection point.

Connection count is derived from the bond list rather than stored as duplicate state.

Bond strength is bounded and is a property of the individual bond.

---

## 6. Material Properties in Structures

A `StructuralUnit` obtains:

- mass
- potential energy
- reactivity
- cohesion

from its catalog resource type.

No alternate per-unit physical-property system should be introduced.

Aggregate structural properties may use the same weighted/summed logic previously used by `Material`, applied across discrete structural units.

Existing geometry (`Shape`, `Form`, `ConnectionPoint`) remains authoritative.

---

## 7. Reactivity and Existing Chemistry Math

Reactivity has an exponential influence.

Existing mathematical building blocks remain valid:

- `complexity(n)`
- `exponential_influence(x)`
- `signed_exponential(x)` where applicable
- `effective_reactivity(reactivity, water_field)`
- `combine_work_cost()` as a formation-work building block

Water dilutes effective reactivity.

The formulas themselves are not to be replaced merely because the structural scope has changed.

`combine_work_cost()` is now understood as a building block for individual bond formation rather than a bulk-compound energy mechanism.

---

# 8. COMBINE — LOCKED ARCHITECTURE

COMBINE is the operation that creates a new bond.

Initially, the fundamental operation is binary:

```text
A + B → AB
```

The three structural cases use the same primitive:

1. unbonded + unbonded
2. bonded + unbonded
3. bonded + bonded

The difference is only where the two connection points originate.

### 8.1 Unbonded + Unbonded

Two resource units are instantiated from bulk `stored_unbonded` material and one new bond connects them.

### 8.2 Bonded + Unbonded

One existing structural unit supplies a connection point. One new unit is instantiated from `stored_unbonded`. One new bond connects them.

All existing bonds on the existing structure remain intact.

### 8.3 Bonded + Bonded

Two existing structural units supply the connection points. A new bond joins them.

This may connect previously separate connected structures or create another bond within an existing connected structure.

No special data-model mechanism is required for these cases.

### 8.4 Resource Consumption

A successful COMBINE consumes the raw resource units used to instantiate the participating structure.

Their mass does not disappear. It moves from unbonded material storage into organism structure.

A failed COMBINE normally returns the input resources unchanged.

---

# 9. COMBINE ENERGY ARCHITECTURE — NEW IN 4.1

COMBINE is the energetic interaction that converts resource potential-energy opportunity into bonded-state energy, usable organism energy, and heat/stress.

For the incoming resources:

```text
P_in = Σ P_i
```

The interaction has formation work:

```text
W_formation
```

The exact mathematical equation for `W_formation` remains unresolved and must be tested experimentally.

### 9.1 Favorable Interaction

If:

```text
P_in > W_formation
```

then the incoming potential energy can pay the formation work and leaves an interaction surplus:

```text
E_surplus = P_in - W_formation
```

The surplus may be divided among:

1. bond energy
2. usable energy
3. heat/stress

The organism's genome/processing traits may influence the partition.

### 9.2 Unfavorable Interaction

If:

```text
P_in < W_formation
```

then the organism may supply the deficit from its usable-energy reserve.

If it cannot supply the deficit, the COMBINE fails.

The exact failure/deficit handling must remain explicit in the implementation and must not create energy from nowhere.

### 9.3 Energy Conservation

COMBINE must not create energy.

The accounting boundary is:

```text
resource potential energy
        + organism-paid energy
        ↓
formation work
+ bond energy
+ usable energy
+ heat/stress
```

The exact interaction equations remain open for experimentation.

---

# 10. Bond Energy — LOCKED

Bond energy is distinct from resource potential energy.

### Resource potential energy

- belongs to the raw resource type/state
- is derived from immutable resource properties
- is not a mutable energy bank

### Bond energy

- belongs to the bonded state
- is mutable state stored on the individual bond
- is established by COMBINE
- is released by BREAK
- does not spontaneously decay in V4/V4.1

A bond therefore conceptually contains:

```text
Bond
├── endpoints
├── strength
└── bond_energy
```

### 10.1 Bond Strength

Bond strength is not simply raw potential energy.

It primarily reflects interaction compatibility/strength and resulting bond strength.

Bond strength is bounded.

Surplus investment into bond strength uses a capped diminishing-returns/soft-cap relationship.

The exact reinforcement equation remains unresolved and must be experimentally tested.

---

# 11. COMBINE ENERGY PARTITION — UNRESOLVED EQUATION

The architecture requires three possible destinations for favorable interaction surplus:

```text
interaction surplus
├── bond energy
├── usable energy
└── heat/stress
```

The exact partition equation is intentionally not locked yet.

Do not replace this with an arbitrary constant percentage and treat that percentage as final design.

The implementation should keep the partition equation isolated so it can be tested independently.

---

# 12. BREAK — LOCKED ARCHITECTURE

BREAK operates on an actual bonded structure/bond state.

It does not reconstruct energy from the original resource properties.

The fundamental energy rule is:

```text
E_break_source = Bond.bond_energy
```

### 12.1 BREAK Work

BREAK may require work:

```text
W_break
```

The exact break-work equation remains an experimental mathematical question.

Net energy is:

```text
E_net = E_bond - W_break
```

If `E_net > 0`, the positive result can become usable energy after processing efficiency.

If `E_net < 0`, the organism must pay the deficit or suffer the corresponding energetic/stress consequence.

### 12.2 BREAK Output

A successful BREAK has three conceptual outputs:

```text
bonded structure
      ↓ BREAK
bond energy → usable energy / heat-stress
resource units → unbonded material
```

The constituent resource units remain physically present.

They become unbonded material again.

The bond itself disappears.

### 12.3 Critical Rule

BREAK releases **stored bond energy only**.

It does not release:

- the original resource potential-energy values
- a recomputed estimate of the resources' potential energy
- a second copy of energy already represented by the bond

This is the primary energy-model correction in Specification 4.1.

---

# 13. BREAK ENERGY PROCESSING

For positive net BREAK energy:

```text
E_usable = E_net × processing_efficiency
```

The remainder becomes heat/stress through explicit inefficiency.

For negative net energy:

```text
E_deficit = max(0, -E_net)
```

The organism must cover the deficit where the operation is permitted to continue.

No energy should disappear without an explicit loss mechanism.

---

# 14. ENERGY IN THE ORGANISM — LOCKED

Positive usable energy generated by an interaction enters the organism's usable-energy pool.

There is no arbitrary rule that only a fixed percentage of successful interaction energy is usable merely for conservation purposes.

Conservation losses must come from explicit mechanisms such as formation work, processing inefficiency, or heat/stress.

### 14.1 Soft Physiological Capacity

Usable energy may accumulate beyond a preferred physiological capacity.

There is no arbitrary hard maximum such as `energy <= 100`.

A soft-cap/diminishing-return mechanism should reduce the benefit of excessive stored usable energy and increasingly route excess toward physiological stress/heat or equivalent explicit consequence.

The exact soft-cap equation remains unresolved.

---

# 15. ENERGY COSTS

Energy is not automatically charged for every physical action.

Do not add arbitrary energy bookkeeping to:

- sensing
- perception
- ordinary resource acquisition
- every movement calculation

Energy is mechanically meaningful where the architecture explicitly requires it:

- COMBINE
- BREAK
- maintenance
- reproduction
- growth where later implemented

Movement has the previously established baseline movement-energy consequence; the exact scaling follows the current implementation decision rather than inventing a new cost here.

---

# 16. ORGANISM PROCESSING AND DECISION ARCHITECTURE

Organisms do not symbolically plan chemical reactions.

The intended flow remains:

```text
perception
   ↓
desirability
   ↓
acquisition
   ↓
stored material
   ↓
COMBINE / BREAK
```

Genome traits bias what is acquired and how interactions are processed.

Do not introduce explicit planning such as:

> "I want methane because methane gives me 20 energy."

Complex behavior should emerge from the interaction of perception, traits, environment, and consequences.

---

# 17. COMBINE FREQUENCY

There is no arbitrary global rule such as "one COMBINE every 10 ticks."

COMBINE opportunities emerge from:

- physical availability
- contact/geometry
- stored material
- existing structure
- organism state
- processing capability

Processing complexity may determine duration.

---

# 18. COMPATIBILITY

There is no hard compatibility matrix.

Do not hard-code rules such as:

```text
Carbon + Methane = allowed
Carbon + Water = forbidden
```

unless a future design explicitly establishes such a physical law.

Compatibility should emerge from:

- reactivity
- cohesion
- physical contact
- geometry
- organism/genome processing traits

The exact geometric compatibility equation remains unresolved.

---

# 19. MATERIAL CONSERVATION

Successful COMBINE moves material from unbonded storage into structural units.

BREAK moves constituent structural units back to unbonded material storage.

Mass is conserved.

Existing structural bonds are preserved unless the operation explicitly removes them.

Environmental acquisition remains a bulk transfer mechanism. Structural graph complexity is created inside the organism.

---

# 20. ENVIRONMENTAL RESOURCE FIELDS

Resources are represented environmentally as field/distribution state rather than requiring the organism to know an abstract global resource inventory.

The main environmental resource pool is the deeper/base layer.

Vents transfer resources from the deeper resource pool into the organism-accessible environment.

Vent behavior is indiscriminate with respect to bonded/unbonded preference: the vent transfers available resource material rather than choosing material according to organism chemistry.

The organism/environment boundary should not contain structural graph logic.

---

# 21. RESOURCE ACQUISITION

Acquisition transfers bulk resource material into organism storage.

It does not instantiate structural units.

Instantiation occurs during COMBINE when material becomes part of an organism structure.

This preserves a clean boundary:

```text
Environment
    ↓ ACQUIRE
bulk unbonded material
    ↓ COMBINE
structural units + bonds
```

---

# 22. ORGANISM LIFECYCLE

The lifecycle must remain grounded in actual resource/material state and explicit energetic consequences.

Do not add scripted species roles or hard-coded evolutionary outcomes.

The single-organism lifecycle must be internally reliable before implementing higher-order systems such as colonies or multicellular differentiation.

---

# 23. REPRODUCTION

Reproduction remains a biological process separate from generic COMBINE.

Reproduction may use existing material-combination helpers for constructing offspring state, but this must not be confused with the generic raw-material COMBINE transformation.

The generic COMBINE system creates structural bonds.

Reproduction creates offspring according to the existing reproduction architecture and inherited genome rules.

Parental investment and gestation remain governed by the current single-trait dual-purpose rule where applicable.

---

# 24. MEMORY / CONSEQUENCE HISTORY

Interaction and consequence history may feed organism memory.

Memory contributes to future decision making.

Game-theoretic behavior should emerge from repeated interaction consequences rather than a hard-coded strategy system.

This remains subordinate to the minimum-information principle.

---

# 25. COLONY / MULTICELLULAR DIRECTION — FUTURE

The long-term direction is:

```text
simple life
→ cooperation/groups
→ colonies
→ differentiated structures
→ integrated organisms
→ large complex organisms
```

Previously discussed colony requirements remain future architecture, not current implementation.

Do not implement colony mechanics until the single-organism interaction and lifecycle systems are internally consistent.

---

# 26. FIVE UNRESOLVED MATHEMATICAL SYSTEMS

The following are deliberately not finalized by Specification 4.1:

1. Exact COMBINE formation-work equation
2. Exact bond-energy equation
3. Exact surplus partition equation
4. Exact usable-energy soft-cap equation
5. Exact bond-strength reinforcement equation

These must be implemented as isolated, testable functions/parameters.

They should be evaluated experimentally before being placed into the evolutionary loop.

Do not settle these equations aesthetically or by arbitrary constants simply to make the simulation run.

---

# 27. IMPLEMENTATION PRINCIPLES

When implementation and this specification disagree:

> The implementation is wrong; the specification wins.

Prefer targeted changes over rewrites.

Preserve unrelated working systems.

Every new energetic pathway must have explicit accounting.

Every new structural pathway must preserve mass.

Do not maintain duplicate representations of the same physical state when the state can be derived from authoritative data.

Avoid mutable copies of immutable resource properties.

---

# 28. CURRENT IMPLEMENTATION STATUS — 4.1

The Rust backend currently contains groundwork for:

- immutable resource properties
- resource catalog/baselines
- property-based perception
- reactivity math
- water dilution
- material handling
- structural units
- bond edge representation
- connection geometry
- formation-threshold calculations
- bond energy state
- isolated COMBINE energy-accounting primitives
- isolated BREAK energy-accounting primitives
- organism usable-energy state
- lifecycle/reproduction systems

The implementation is transitioning from the previous bulk-material transformation model to the discrete structural bond model.

Legacy mutable `Material.energy_content` behavior must not be treated as authoritative. Potential energy must remain derived from immutable resource properties, while bond energy is stored explicitly on bonds.

---

# 29. TESTING REQUIREMENTS

The following invariants should be tested independently before evolutionary-loop testing:

### Resource

- resource properties remain immutable
- potential energy is derived rather than stored as mutable material energy
- raw material cannot BREAK

### COMBINE

- two inputs are consumed only on successful COMBINE
- failed COMBINE does not silently destroy inputs
- successful COMBINE creates the expected bond
- existing bonds remain intact
- mass is conserved
- energy accounting closes
- bond energy is stored on the resulting bond

### BREAK

- BREAK operates on an actual bond/structure
- BREAK uses stored bond energy
- BREAK does not reconstruct energy from raw resource potential energy
- the bond is removed
- constituent units return to unbonded material
- energy accounting closes
- insufficient energy produces the defined deficit/stress behavior

### Energy

- positive interaction energy can enter usable-energy storage
- explicit inefficiencies become heat/stress
- no hidden energy source exists
- no hidden energy sink exists
- soft capacity does not behave like an arbitrary hard cap

### Structure

- individual bonds can be removed without deleting unrelated bonds
- point disconnection removes all bonds at that point when explicitly requested
- connection counts remain derivable from the edge list

---

# 30. DEVELOPMENT PRIORITY

Work in this order unless an explicit design decision changes it:

1. Complete discrete structural COMBINE integration.
2. Complete structural BREAK integration.
3. Remove remaining legacy raw-material energy accounting.
4. Validate mass and energy invariants.
5. Test the five unresolved equations in isolation.
6. Repair the organism lifecycle, maturation, reproduction, maintenance, and death pathways as required by the current lifecycle specification.
7. Validate mutation and selection.
8. Only then develop colonies and higher-order organization.
9. Frontend work should reflect the stable simulation architecture rather than driving simulation design.

---

# 31. AUTHORITATIVE ENERGY MODEL SUMMARY

The entire V4.1 energy architecture can be summarized as:

```text
RAW RESOURCE
  │
  │ intrinsic potential energy
  ▼
COMBINE
  │
  ├── formation work
  ├── interaction surplus
  ├── bond energy
  ├── usable energy
  └── heat/stress
          │
          ▼
   BONDED STRUCTURE
          │
          │ stored bond energy
          ▼
        BREAK
          │
          ├── break work
          ├── usable energy
          ├── heat/stress
          └── constituent resources → unbonded storage
```

The central rule is:

> **Potential energy belongs to the resource's raw state. Bond energy belongs to the bonded state. BREAK releases bond energy, never raw resource potential energy.**

This distinction is authoritative for EvoSim 4.1.
