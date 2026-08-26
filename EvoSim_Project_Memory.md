# EvoSim — Project Memory / Continuation File

## Purpose of this file
This file consolidates the known project history, locked design decisions, architecture, current implementation state, and known problems so future work can resume without reconstructing the project from old chats.

---

# 1. PROJECT IDENTITY

Project: EvoSim / Evolution Simulator

Original description:
- A web/local evolutionary simulation.
- It was initially described as a "numbers go up" / evolution game, but that framing is explicitly obsolete.
- It is now an evolutionary simulation whose purpose is to produce believable emergent behavior from the minimum information necessary.
- The simulation should eventually be capable of running independently of the player.
- The long-term conceptual trajectory is simple life -> groups/colonies -> differentiated structures -> integrated organisms -> large, complex organisms.
- "Dinosaur/dino" is shorthand for the eventual conceptual endpoint, not a literal dinosaur requirement.

Core principle:
> Calculate the minimum amount of information necessary to produce the desired emergent behavior.

Do not add hard-coded behaviors merely to force an intended outcome.

---

# 2. FUNDAMENTAL ENTITIES

Exactly three fundamental entities:
1. Environment
2. Organisms
3. Resources

Everything else should emerge from interactions among these systems where practical.

---

# 3. RESOURCE MODEL — LOCKED

Every base resource has four immutable properties:
- mass
- potential energy
- reactivity
- cohesion

These properties:
- never evolve
- are intrinsic properties of resource types
- should not become mutable organism traits

Current conceptual resource catalog:
- Carbon
- Methane
- Hydrogen
- Sulfur
- Nitrogen
- Phosphorus
- Water

Current default values in the Rust resource code:

Carbon:
- mass 1.0
- potential energy 1.0
- reactivity 0.1
- cohesion 0.95

Methane:
- mass 1.0
- potential energy 20.0
- reactivity 4.0
- cohesion 0.1

Hydrogen:
- mass 1.0
- potential energy 12.0
- reactivity 3.0
- cohesion 0.05

Sulfur:
- mass 1.0
- potential energy 8.0
- reactivity 2.0
- cohesion 0.4

Nitrogen:
- mass 1.0
- potential energy 0.5
- reactivity 0.2
- cohesion 0.7

Phosphorus:
- mass 1.0
- potential energy 0.8
- reactivity 0.3
- cohesion 0.6

Water:
- mass 1.0
- potential energy 0.0
- reactivity 0.0
- cohesion 0.5

Water is intended to act as a neutral/diluting influence on reactivity rather than being an energy source.

Potential energy semantics are critical:
- Potential energy is an absolute maximum amount of energy a resource can provide through the appropriate process.
- It is NOT a consumable resource pool.
- It does NOT decrease because a resource has been processed.
- A resource with potential energy 20 has potential energy 20 per unit regardless of processing history.
- Energy itself is NOT a fundamental resource.

---

# 4. RAW VS BONDED MATERIAL — LOCKED

Raw/unbonded material cannot BREAK.

This is a hard rule and must not be weakened or removed.

Raw material:
- must first be bonded
- cannot be directly broken

COMBINE:
- creates bonded material

Environmental vents:
- may emit pre-bonded material when necessary to make the initial environment viable
- this is permissive, not mandatory

Bonded material:
- can undergo BREAK
- current implementation says BREAK is legal when bonded and total units >= 2

---

# 5. BREAK / PROCESSING — LOCKED CONCEPT

When bonded material undergoes BREAK:
- usable energy can be produced
- heat/waste effects can occur
- physical material does not simply disappear

Spent material has exactly two conceptual outcomes:
1. remains as organism structure ("build self")
2. becomes waste and is ejected from organism

Waste:
- remains physical material in environment
- can potentially be used by other organisms
- scavenging/decomposition should emerge from interactions rather than hard-coded ecological roles

CRITICAL:
- potential energy must NOT be represented as a mutable `energy_content` field on material
- potential energy is a property of resource types
- energy should emerge from the processing interaction

---

# 6. DEATH / CARCASSES — LOCKED

When an organism dies:
- bonded material must NOT be converted back into raw resources
- it must NOT simply be returned to a global raw-resource reservoir
- dead organism/carcass remains where it died
- bonded and stored material remains physically present
- other organisms can encounter and consume/use it
- scavenging/decomposition should emerge naturally

Do not create explicit "scavenger" roles.

---

# 7. ENVIRONMENT / RESOURCE POOL

Environment has a finite resource pool in implementation.

It should be sufficiently large that, at intended simulation scale, it behaves as effectively inexhaustible.

Resource clouds:
- draw material from environmental supply
- are temporary concentrations
- disperse
- may disappear once diluted beyond useful concentration
- should not continue consuming significant computation after becoming unusable
- dispersed material does not currently need to be returned to the reservoir

Previous architecture included environmental hotspots emitting finite resource clouds.

---

# 8. EVOLUTION — LOCKED CONCEPT

Organism genome:
- mutable
- inherited
- subject to mutation

Resource properties:
- immutable
- do not evolve

Behavior emerges from:
- genome
- organism state
- environment
- interactions

Do NOT hard-code:
- predator
- prey
- scavenger
- herbivore
- carnivore
- social roles
- colony species
- large-organism pathway

Predation/consumption should emerge from organism behavior and resource/material interactions.

Colonies should emerge from organisms interacting/cooperating rather than being an arbitrary species classification.

Intended but non-mandatory trajectory:
cell -> groups/colonies -> differentiated structures -> integrated organisms -> large/complex organisms

Do not hard-code this ladder.

---

# 9. PHYSICS / WORLD

- World should eventually be large enough for large organisms.
- Simulation may remain 2D.
- Do NOT redesign around 3D unless explicitly instructed.
- Movement and spatial interactions matter.
- Current world implementation historically used approximately 1000x1000 dimensions.
- Resource clouds and spatial sensing are part of the current architecture.

---

# 10. PERFORMANCE PHILOSOPHY

Performance matters because eventual populations and organism complexity may become very large.

Rules:
- prioritize emergent behavior using minimum necessary information
- avoid unnecessary per-material bookkeeping
- avoid storing state derivable from immutable resource properties
- do not prematurely delete information required for emergence
- optimize the architecture rather than simply making everything more abstract or detailed

The simulation should eventually support statistical/aggregate representations where appropriate, especially for colonies or lower-level systems, while preserving the information necessary for emergence.

A previously discussed future idea:
- when colonies become sufficiently established, a new "floor" could represent lower-level activity statistically
- this could allow colonies to become independent organisms
- this is a future architectural concept, not currently implemented or locked as a final mechanic

---

# 11. ORGANISM / GENOME ARCHITECTURE CURRENTLY IMPLEMENTED

Current Rust genome code uses:

TraitDef:
- name
- value
- mutation_probability
- mutation_sigma

Genome:
- Vec<TraitDef>

Genome has accessors for:

memory_strength
- default 0.5
- clamp 0..1

perception_radius
- default 100
- minimum 0

sensory_resolution
- default 0.5
- clamp 0..1

directional_resolution
- default 1.0
- clamp 0..1

mass_affinity
- default 0
- clamp -1..1

potential_energy_affinity
- default 0
- clamp -1..1

reactivity_affinity
- default 0
- clamp -1..1

cohesion_affinity
- default 0
- clamp -1..1

processing_efficiency
- default 0.8
- clamp 0.05..1

movement_efficiency
- default 0.8
- clamp 0.05..1

reproductive_investment
- default 0.5
- clamp 0.15..1

adult_mass
- default 16
- clamp 4..80

Mutation:
- each trait independently mutates based on its mutation probability
- delta is random in [-1,1] * mutation_sigma
- mutation probability itself can very rarely mutate
- mutation probability is clamped approximately 1e-6..0.1 after mutation

Initial genome values:
memory_strength = 0.5
perception_radius = 100
sensory_resolution = 0.5
directional_resolution = 1.0
mass_affinity = 0
potential_energy_affinity = 0.5
reactivity_affinity = 0
cohesion_affinity = 0
processing_efficiency = 0.8
movement_efficiency = 0.8
reproductive_investment = 0.5
adult_mass = 16

Initial trait mutation probability is 0.001 and sigma is generally 0.05 except adult_mass sigma 0.4 and perception_radius sigma 1.0.

---

# 12. RESOURCE AFFINITY DESIGN

A previously locked design decision:
- Resource affinity responds to differences between encountered resource properties and an organism's baseline.
- Baseline for the four immutable resource properties starts as the average across all base resource TYPES, not abundance.
- Affinity operates independently on:
  - mass
  - potential energy
  - reactivity
  - cohesion
- There is not one generic "resource score" underlying the system.
- Desirability is an emergent/calculated response to these property differences plus organism state and other factors.

Current implementation has `ResourceBaselines::from_catalog()` calculating the average property of each catalog type.

Current sensed output confirms this system is active.

Example observed organism:
- potential_energy_affinity = 0.5
- mass_affinity = 0
- reactivity_affinity = 0
- cohesion_affinity = 0

Therefore the initial organism is biased toward resources with higher potential energy, without explicitly having a reactivity or cohesion preference.

---

# 13. REACTIVITY MATH

Reactivity is intended to have an exponential influence.

Current shared math:

`complexity(n)`:
- 0 when n <= 1
- otherwise n * log2(n)

`exponential_influence(x)`:
- maps x >= 0 to 0..1 using `1 - exp(-x)`
- stable and bounded

`signed_exponential(x)`:
- preserves sign while applying exponential influence to magnitude

Current resource helper:
`effective_reactivity(reactivity, water_field) = reactivity / (1 + max(water_field, 0))`

Thus water dilutes effective reactivity.

`combine_work_cost()` currently depends on:
- material amount
- weighted reactivity
- cohesion
- complexity
- water field

Conceptually:
- higher complexity increases work
- cohesion affects work
- effective reactivity is transformed exponentially

---

# 14. CURRENT MATERIAL CODE

Rust currently has:

ResourceProperties:
- mass
- potential_energy
- reactivity
- cohesion

BaseResource:
- name
- properties

ResourceBaselines:
- average properties across base resource types

Material:
- `parts: Vec<(String, f64)>`
- `energy_content: f64`
- `bonded: bool`

Material methods include:
- free_base()
- total_amount()
- is_empty()
- can_break()
- mass()
- weighted_properties()
- take()

Helpers:
- merge_parts()
- combine_materials()
- combine_work_cost()
- effective_reactivity()
- property_ranges()
- default_catalog()
- fresh_energy()

IMPORTANT CURRENT PROBLEM:
The material struct still contains:
`energy_content: f64`

This conflicts with the master spec.

The current `take()` method proportionally removes `energy_content`.

The current `combine_materials()` adds the input materials' `energy_content`.

The current `fresh_energy()` calculates:
resource potential energy * amount.

This means the codebase is in a transitional state:
the intended conceptual architecture has moved to emergent energy, but legacy mutable energy bookkeeping is still present.

This must eventually be reconciled with the authoritative spec.

---

# 15. CURRENT OBSERVED SIMULATION STATE

Most recent observed UI:

Energy Ledger:
- Potential energy released: 16700.894
- Usable energy gained: 13021.985
- Heat dissipated: 3678.909
- Usable energy currently held: 13021.985

Active transformation:
- Organism 1 — BREAK Hydrogen (4.00 units)
- Remaining: 1 / 2 ticks

Organism:
- ID: 1
- Stage: Juvenile
- Age: 1317
- Usable energy: 13021.985
- Stress: 7889155418.056
- Processing: BREAK Hydrogen — 1/2 ticks left

Sensed resource data showed:
- Carbon perceived amount around 12.5
- Methane around 7.5
- Hydrogen around 11–15
- Sulfur around 5
- Nitrogen around 5
- Phosphorus around 1
- Water around 4

Example calculated desirabilities:
- Carbon approximately -0.0100
- Methane approximately +0.0136
- Hydrogen approximately +0.0104 to +0.0137 depending on sensed amount
- Sulfur approximately +0.00174
- Nitrogen approximately -0.00468
- Phosphorus approximately -0.00093
- Water approximately -0.00408

The organism was actively choosing BREAK Hydrogen.

Sensory direction observed:
- X = -0.195
- Y = -0.981
- strength = 0.025

Current displayed mutable traits:
memory_strength = 0.5000
perception_radius = 100.0000
sensory_resolution = 0.5000
directional_resolution = 1.0000
mass_affinity = 0.0000
potential_energy_affinity = 0.5000
reactivity_affinity = 0.0000
cohesion_affinity = 0.0000
processing_efficiency = 0.8000
movement_efficiency = 0.8000
reproductive_investment = 0.5000

---

# 16. CURRENT MAJOR BUG / INVESTIGATION TARGET

The organism has accumulated absurd usable energy:
~13,022

while:
- it is still Juvenile
- age is 1317
- adult_mass is 16
- stress is ~7.9 billion

This indicates an energy accumulation/excess-energy problem or another broken constraint.

It also raises a reproduction question:
the architecture contains reproduction-related traits and lifecycle concepts, but the observed organism has not reproduced.

Likely possibilities that need code inspection, NOT assumptions:
- adulthood is gated by structural/bonded material rather than energy
- growth is not occurring
- adult condition is never reached
- reproduction condition is never called
- reproduction has another unmet resource/material/state requirement
- processing state prevents reproduction
- reproduction probability/decision logic is effectively zero
- legacy energy accounting has disconnected growth/reproduction from actual processing

Do NOT diagnose the exact reproduction failure without inspecting the relevant organism lifecycle/reproduction code.

The key conceptual distinction:
Having reproduction fields/architecture is not proof that the reproduction pathway is being reached.

---

# 17. PREVIOUS IMPLEMENTATION HISTORY

Earlier browser/CodePen prototype had:
- simulation speed around 20
- 1000x1000 world
- movement energy cost
- resource hotspots
- resource clouds
- baseline random movement
- resource attraction
- spatial memory
- crowd response
- growth pressure
- cell radius based on energy/structural carbon
- reproduction energy/carbon rules
- mutation
- evolution tracking

Earlier resource system included finite carbon hotspots emitting resource clouds.

The project has since moved toward:
- local development
- local server
- persistent simulation data
- running independently of Internet
- performance-focused architecture

The frontend is React/TypeScript and the simulation/backend code shown is Rust.

Current React entry point:

import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import App from './App.tsx'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)

---

# 18. PLAYER LAYER — FUTURE

The evolutionary simulation is the bottom layer and must work independently.

Eventually the player should:
- nurture an organism
- influence its environment and survival
- guide selection across generations
- attempt to push evolution toward a desired form

But:
- player does not directly design the final organism
- background environment continues evolving organically
- player influences evolution rather than directly specifying outcomes

---

# 19. COLONY / MULTICELLULAR DIRECTION

Previously proposed colony mechanic:
- at least 100 members participating to qualify as a colony
- at least 3 neighboring cells also participating in the colony

There was an important design question:
Should a colony merely be a processing/optimization qualifier, or should colony formation unlock capabilities?

The broader desired direction is:
- colonies should emerge from cooperation/interactions
- eventually differentiated structures should emerge
- colonies could eventually become independent organisms
- a new computational "floor" could represent lower-level cell activity statistically

This is NOT currently implemented and should not be treated as finalized beyond the explicitly stated colony requirements.

---

# 20. GAME THEORY / MEMORY CONCEPT

A prior architectural idea:
- interaction/consequence history should connect to memory
- memory should contribute to decision making
- game-theoretic behavior should emerge from repeated interaction consequences rather than a hard-coded strategy system

This should remain subordinate to the minimum-information/emergence principle.

---

# 21. IMPORTANT DESIGN PHILOSOPHY

Do not turn the simulation into a collection of scripted life stages.

Avoid:
- "if energy > X, become herbivore"
- "if size > X, become predator"
- "100 cells = organism" unless that is explicitly part of an agreed emergence mechanism
- hard-coded food categories
- hard-coded species roles
- hard-coded dinosaur evolution
- arbitrary complexity stats whose only purpose is making numbers increase

Prefer:
- immutable physical/resource properties
- simple organism-level heritable parameters
- environmental constraints
- interactions
- consequences
- selection
- emergence

The desired result is that complex behavior is a consequence of the system, not a list of behaviors programmed in advance.

---

# 22. CURRENT PRIORITY ORDER

When resuming development, use this priority order:

1. Inspect the actual current lifecycle/growth/reproduction code.
2. Trace why the current organism remains Juvenile despite age 1317 and enormous energy.
3. Fix the energy/material accounting so energy cannot be generated/stored in a way that violates the master spec.
4. Remove/rework mutable material `energy_content` so potential energy remains an immutable property of resource types and usable energy emerges from processing.
5. Verify COMBINE and BREAK semantics against the master spec.
6. Verify that spent material either becomes organism structure or waste.
7. Verify growth from processed material.
8. Verify adulthood and reproduction.
9. Only after the basic life loop is internally consistent, test whether mutation + selection actually produce evolution.
10. Then develop emergent cooperation/colonies.
11. Then differentiated/integrated organisms.
12. Then large organisms.

Do not jump to colonies/multicellularity while the single-organism lifecycle is broken.

---

# 23. WORKING STYLE / USER CONSTRAINT

The user has explicitly preferred:
- step-by-step guidance
- never more than one step at a time when giving implementation instructions

When debugging:
- explain what the current code is doing in plain language
- identify the smallest relevant change
- give one actionable step at a time
- wait for the result before proceeding

The user is not an experienced programmer and benefits from explanations that distinguish:
- architecture
- current implementation
- intended behavior
- bug
- future design

Do not overwhelm with a long sequence of coding instructions.

---

# 24. CURRENT STATUS SUMMARY

The project is NOT at the beginning.

The current system already has:
- immutable resource properties
- resource baselines
- property-based resource perception
- genome and mutable traits
- mutation
- spatial sensing
- directional behavior
- material combine/break concepts
- energy ledger
- active transformations
- organism lifecycle concepts
- React frontend
- Rust simulation/backend components

The major current state is:
> The core organism/resource interaction loop exists, but the transition from resource processing to material use, growth, adulthood, and reproduction is not yet internally reliable.

The most visible symptom:
> Organism 1 is Juvenile at age 1317 while holding ~13,022 usable energy and experiencing enormous excess-energy stress.

The immediate job is to trace the actual lifecycle/reproduction code and fix the fundamental accounting/lifecycle path before adding higher-level complexity.

---

# 25. AUTHORITATIVE RULE

When existing code conflicts with the master spec:
> THE CODE IS WRONG. THE MASTER SPEC WINS.

Do not silently change locked rules to accommodate existing implementation.
