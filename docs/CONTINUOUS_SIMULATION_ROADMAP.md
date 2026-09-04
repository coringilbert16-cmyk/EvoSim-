# EvoSim Comprehensive Viable Simulation Roadmap

**Status:** Active implementation guideline  
**Purpose:** Turn the existing EvoSim codebase into a viable, continuously running artificial-life simulation in which the environment, chemistry/material system, organisms, behavior, transformations, construction, reproduction, inheritance, and evolution operate together coherently over long periods—and make that complete simulation observable through the UI.  
**This document is not the EvoSim master specification.** It is a practical engineering roadmap for implementing, integrating, testing, and validating the current simulation.  
**Working rule:** We proceed one focused change at a time. A phase is complete only when behavior is demonstrated and integration is verified.

---

# 1. Current Goal

The goal is **not** merely to make EvoSim run, animate, or display a moving world.

> **Build a viable working artificial-life simulation in which the modeled environment supplies material, organisms interact with that environment, material transformations create physical organism state, organisms maintain and develop, organisms reproduce, offspring inherit and mutate traits, populations persist and change, and the complete system can continue operating without human intervention for long periods. The UI is a faithful window into that living system.**

The intended causal loop is:

```text
ENVIRONMENT
reservoir → vents → active material field → diffusion/redistribution
                         ↓
                 material availability
                         ↓
                    perception
                         ↓
                     decision
                         ↓
                organism interaction
                         ↓
                COMBINE / BREAK
                         ↓
                    structure
                         ↓
              energy/material effects
                         ↓
             maintenance + development
                         ↓
             reproductive readiness
                         ↓
                   reproduction
                         ↓
              reproductive construction
                         ↓
                    offspring
                         ↓
              inheritance + mutation
                         ↓
                  new generation
                         └──────────→ cycle

                 authoritative state
                         ↓
                   observation API
                         ↓
                         UI
```

The UI is therefore an **observation layer**, not the objective and never a second simulation engine.

---

# 2. What This Roadmap Is — and Is Not

## This roadmap is

- An implementation sequence for making the entire simulation viable.
- A checklist for closing broken causal links.
- A guide for integrating existing subsystems instead of rewriting them unnecessarily.
- A set of verification gates.
- A long-running stability and ecosystem-validation plan.
- A guide for making the resulting simulation observable.

## This roadmap is not

- The EvoSim master specification.
- A replacement for established simulation rules.
- Permission to invent mechanics merely to create visual activity.
- A UI-first development plan.
- Permission to tune constants before determining whether code is broken.
- Permission to keep contradictory legacy authorities because tests happen to pass.

When this roadmap and an established simulation rule conflict, the established simulation rule remains authoritative.

---

# 3. Definition of a Viable Working Simulation

EvoSim is not viable merely because ticks increase, the server remains alive, unit tests pass, transformations work in isolation, reproduction works in a hand-built test, or the browser receives snapshots.

A viable system must demonstrate all of the following through the integrated simulation.

## 3.1 Continuous operation

The simulation can run for long periods without:

- panic or deadlock;
- NaN/Infinity contamination;
- stalled progression;
- duplicate stepping;
- runaway memory growth;
- accumulating stale transformations/objects;
- progressive performance collapse.

## 3.2 Environmental continuity

The environment continuously provides and redistributes material according to the model. Material must not mysteriously appear, disappear, or become permanently inaccessible.

## 3.3 Organism viability

At least some organisms can naturally progress through:

```text
birth → juvenile → material interaction → structure/growth
→ maintenance → adult → reproductive readiness
```

They must also be able to die through legitimate lifecycle rules.

## 3.4 Transformation viability

Transformations form complete lifecycles:

```text
eligible inputs → decision → begin → progress → resolve
→ correct material/structure/energy result
```

No transformation may duplicate, destroy, or strand material without an explicit rule.

## 3.5 Reproductive viability

Reproduction must be reachable through normal simulation behavior:

```text
adult → readiness → request → commitment → construction
→ offspring → independent organism
```

## 3.6 Evolutionary viability

Offspring inherit heritable traits, mutation can create valid variation, and traits can affect phenotype and therefore survival/reproduction.

## 3.7 Population viability

Long runs must not settle into an obviously pathological state such as immediate permanent extinction, unbounded population growth, immortal organisms, or reproduction disconnected from resource limits.

The roadmap does **not** prescribe a target population curve. The model must be allowed to produce its own dynamics.

## 3.8 Conservation and accounting

Material and energy accounting must remain coherent.

**Locked bond rule:**

> **Bond strength is calculated only from the constant mathematical properties of the resources participating in the bond. No age, geometry, organism state, history, stored legacy value, load, or other dynamic input may alter intrinsic bond strength.**

Dynamic quantities such as connection load may use bond strength as an input, but may not redefine it.

---

# 4. Core Engineering Principles

## 4.1 One simulation, one authority

There must be one authoritative simulation state and one authoritative progression path. No subsystem may silently maintain a contradictory second authority for fundamental state.

## 4.2 Trace causes, not symptoms

When behavior fails, trace the complete causal chain and fix the first broken link.

For example, if reproduction never occurs, do not immediately lower the reproduction threshold. Trace:

```text
environment → perception → decision → material interaction
→ transformation → structure → maintenance/development
→ readiness → reproduction
```

## 4.3 Preserve correct mechanics

Existing correct behavior should remain intact. A unit test proves a local contract, not system viability.

## 4.4 One source of truth for fundamental quantities

Audit and unify the definitions of:

- resource properties;
- material composition;
- potential energy;
- reactivity;
- cohesion;
- bond strength;
- transformation work;
- energy creation/consumption;
- material ownership;
- structural state.

Legacy fields that contradict authoritative calculations must be removed or made explicitly non-authoritative.

## 4.5 Energy remains emergent

Energy is not to become an unrelated resource that can accumulate without a causal material interaction. Every energy mutation must have an auditable source and cause.

## 4.6 No fake life

Do not add scripted movement, scripted reproduction, artificial resource motion, or decorative activity merely to make the UI look alive. Inactivity is a diagnostic result.

## 4.7 UI is observational

The browser may display and request explicitly supported runtime controls, but it must not calculate chemistry, movement, transformations, reproduction, or evolution independently.

## 4.8 Every change has a gate

For each implementation change:

1. State the invariant being repaired.
2. Trace every relevant code path.
3. Make the smallest coherent change.
4. Add/update focused tests.
5. Run tests.
6. Exercise the integrated path.
7. Re-audit adjacent lifecycle paths.
8. Only then proceed.

---

# 5. Working Method — One Step at a Time

The implementation sequence is deliberately conservative:

```text
AUDIT
  ↓
TRACE ACTUAL RUNTIME PATH
  ↓
IDENTIFY FIRST BROKEN LINK
  ↓
STATE INVARIANT
  ↓
DESIGN SMALLEST COHERENT FIX
  ↓
IMPLEMENT ONE CHANGE
  ↓
FOCUSED TESTS
  ↓
INTEGRATION TEST
  ↓
RUNTIME VERIFICATION
  ↓
RE-AUDIT
  ↓
NEXT BLOCKER
```

Do not combine unrelated fixes. Do not advance because the UI looks better. Advance because the simulation is more correct and more viable.

---

# 6. Phase 0 — Establish the Actual Current Architecture

**Objective:** Build an accurate map of what exists before changing behavior.

### Step 0.1 — Trace the runtime

Identify the exact code for:

- `Simulation` construction;
- ownership of the simulation instance;
- `Simulation::step()`;
- scheduler/tick loop;
- RNG ownership;
- environment ownership;
- organism ownership;
- transformation ownership;
- startup/shutdown/reset;
- runtime controls.

**Deliverable:** a concrete call-path from startup → tick scheduler → `Simulation::step()` → snapshot/runtime output.

### Step 0.2 — Inventory subsystems

For each subsystem document its state, entry point, outputs, callers, tests, and whether the live tick path actually exercises it:

- environment;
- active material field;
- deep reservoir;
- vents;
- diffusion;
- settling;
- resource properties;
- materials;
- bonds and bond strength;
- connection load;
- COMBINE;
- BREAK;
- movement;
- perception;
- decision-making;
- transformations;
- energy;
- maintenance;
- development/growth;
- reproduction readiness;
- reproduction/construction;
- genetics/mutation;
- death/removal;
- snapshots/API;
- frontend.

### Step 0.3 — Find dead paths

Search for no-op handlers, permanently false eligibility checks, legacy fields still read, alternate calculations, unreachable functions, state that is written but never consumed, and state consumed without a live writer.

### Gate 0

Answer precisely:

> **What happens to the environment and initial organism during one real tick, and what exact code path causes every state transition?**

No broad implementation begins until this is known.

---

# 7. Phase 1 — Prove the Authoritative Tick Lifecycle

**Objective:** Ensure one tick is a coherent integrated lifecycle.

Audit the current `Simulation::step()` rather than assuming its existence means the system is integrated.

### Step 1.1 — Trace one real tick

For the initial organism record:

- nearby material;
- perception result;
- decision result;
- selected action;
- action handler;
- material ownership changes;
- transformations created/completed;
- energy changes;
- structure changes;
- age/development changes;
- reproduction state.

### Step 1.2 — Check ordering

Compare actual ordering with the intended causal dependencies:

```text
clock
→ environment
→ active transformations
→ resolve transformations
→ perception
→ organism state/development
→ decisions
→ actions
→ reproduction requests
→ reproductive construction
→ maintenance/capacity/accounting
→ death/removal
→ invariant checks
→ snapshot
```

Do not reorder merely because this list looks cleaner; establish which ordering the actual rules require.

### Step 1.3 — Add runtime invariants

At minimum check:

- monotonic tick;
- valid numeric state;
- unique organism IDs;
- no dead organism acting;
- transformations resolve once;
- no duplicated material ownership;
- reproduction commitments cannot be spent twice;
- snapshots do not mutate authoritative state.

### Gate 1

A controlled integration run advances multiple ticks with the expected lifecycle and no corruption.

---

# 8. Phase 2 — Unify Material, Chemistry, Bond, and Energy Authority

**Objective:** Establish a trustworthy physical/accounting foundation before diagnosing biological viability.

### Step 2.1 — Audit immutable resource properties

For every resource identify the sole authority for:

- mass;
- potential energy;
- reactivity;
- cohesion;
- other established immutable properties.

### Step 2.2 — Audit `Material`

Verify that composition and quantity are authoritative and derived properties are calculated from composition rather than stale cached values.

### Step 2.3 — Complete bond-strength migration

Repository-wide search for:

- stored `Bond.strength`;
- legacy strength fields;
- constructors accepting external strength;
- connection-load code reading stored strength;
- serialization treating strength as authoritative;
- tests encoding the obsolete authority.

All live bond-strength calculations must use intrinsic resource-property mathematics.

### Step 2.4 — Audit energy

For every energy mutation record:

```text
source → cause → amount → destination → material consequence
```

Find and remove/rework legacy independent energy authorities such as obsolete stored `energy_content` behavior.

### Step 2.5 — Audit transformation work

For COMBINE/BREAK verify input ownership, work requirement, progress, completion, output, energy consequence, and conservation.

### Step 2.6 — Add conservation tests

Cover material conservation, energy accounting, intrinsic bond-strength invariance, connection load, and duplicate ownership.

### Gate 2

There is one authoritative chemistry/material/energy model, and no known legacy authority can contradict it.

---

# 9. Phase 3 — Validate the Environment as an Ecological System

**Objective:** Prove the environment can continuously support the rest of the simulation.

### Step 3.1 — Deep reservoir

Verify authoritative quantities, valid values, resource composition, withdrawal, and replenishment behavior.

### Step 3.2 — Vents

Maintain the current rule:

> Reservoir material released by a vent enters the active field regardless of bonding state, and venting does not alter existing active material's bonding state.

Test source depletion, destination deposition, repeated venting, quantity conservation, and spatial placement.

### Step 3.3 — Active material field

Verify material can occupy organism-accessible locations and that bonded/unbonded distinction is used only where active-field mechanics require it.

### Step 3.4 — Diffusion and settling

Trace whether material can move, remain available, leave the active layer, return to the reservoir where modeled, and avoid unexplained accumulation.

### Step 3.5 — Environment-only soak test

Run without organisms and measure total quantities by resource, active/reservoir quantities, vent throughput, settling, diffusion, and numeric validity.

### Gate 3

The environment runs continuously with coherent material accounting and no unexplained creation, loss, runaway accumulation, or permanent depletion.

---

# 10. Phase 4 — Close the Environment → Organism Material Path

**Objective:** Make environmental material biologically reachable.

### Step 4.1 — Trace perception

Verify perception radius, spatial sampling, material visibility, bonding semantics, and correspondence to current environment state.

### Step 4.2 — Trace decisions

For each action determine eligibility, scoring/selection, required inputs, and whether the selected action can execute.

### Step 4.3 — Audit disabled/no-op actions

Explicitly classify current acquisition, expulsion, movement, COMBINE, and BREAK paths as either intentional or lifecycle blockers.

### Step 4.4 — Prove ownership transfer

Demonstrate:

```text
environment → perception → decision → action → organism-accessible material
```

This must use the real simulation path, not hand-built test state.

### Gate 4

At least one legitimate material interaction reaches the organism through the live runtime.

---

# 11. Phase 5 — Close the COMBINE Lifecycle

**Objective:** Prove usable material can become organism structure through the real transformation system.

### Step 5.1 — Reachability

Show that normal environmental interaction can create the material state required by COMBINE.

### Step 5.2 — Commitment

Verify material is committed exactly once and cannot remain simultaneously available elsewhere.

### Step 5.3 — Multi-tick progress

Verify work/progress advances correctly and cannot silently stall.

### Step 5.4 — Completion

Verify structural output, bond state, material ownership, energy consequence, and cleanup.

### Step 5.5 — Persistence

Demonstrate:

```text
environment material → organism → COMBINE → transformation
→ structural unit → later ticks
```

### Gate 5

A normally running organism can complete COMBINE and retain the resulting structure correctly.

---

# 12. Phase 6 — Close the BREAK Lifecycle

**Objective:** Prove structure can be broken through the authoritative transformation system.

### Step 6.1 — Target current bond

Confirm the target exists when BREAK begins.

### Step 6.2 — Audit snapshot semantics

Ensure transformation snapshots cannot later overwrite newer authoritative state or remove unrelated state.

### Step 6.3 — Enforce intrinsic bond strength

BREAK must derive strength from current material composition under the locked equation, never from obsolete stored strength.

### Step 6.4 — Completion

Verify work, bond removal, resulting material, energy release, and unrelated-bond preservation.

### Step 6.5 — Repeatability

Demonstrate:

```text
structure → BREAK → resulting material/energy → continued organism operation
```

### Gate 6

COMBINE/BREAK form a coherent transformation lifecycle wherever the established rules intend that relationship.

---

# 13. Phase 7 — Close the Organism Lifecycle

**Objective:** Make organisms genuine living entities with birth, development, maintenance, and death.

### Step 7.1 — Birth audit

Verify initial structure, accessible material, energy state where applicable, age, development stage, genome, and reproductive state.

### Step 7.2 — Growth audit

Trace how material interaction changes structural mass, geometry, capacity, and developmental state.

### Step 7.3 — Maintenance audit

Every maintenance cost must have a clear cause and accounting. It must not consume nonexistent resources or bypass material/energy rules.

### Step 7.4 — Death audit

Verify death conditions, timing, transformation cleanup, organism removal, and disposition/return of material where modeled.

### Step 7.5 — Lifecycle tests

Prove both:

```text
birth → development → maintenance → death
```

and:

```text
birth → development → maintenance → reproductive readiness
```

### Gate 7

Normal organisms can progress through lifecycle states without leaks or impossible transitions.

---

# 14. Phase 8 — Close the Reproduction Lifecycle

**Objective:** Make reproduction naturally reachable and physically real.

### Step 8.1 — Readiness

Trace every readiness prerequisite and prove normal behavior can satisfy it.

### Step 8.2 — Request and commitment

Verify valid requests, correct resource/material commitment, and no double spending.

### Step 8.3 — Construction

Verify multi-tick construction, one physical unit progressing at the intended rate, correct material consumption, and no stranded commitments.

### Step 8.4 — Geometry/contact

Verify offspring units use actual placement/contact geometry rather than arbitrary coordinates.

### Step 8.5 — Activation

At completion verify independent offspring state, valid parent/offspring relationships, independent perception/action, cleanup of construction state, and future lifecycle eligibility.

### Gate 8

A naturally running organism reaches reproduction and produces an independently functioning offspring.

---

# 15. Phase 9 — Close Genetics, Heritability, and Mutation

**Objective:** Make reproduction capable of generating evolution.

### Step 9.1 — Inheritance

Trace parent genome → offspring genome and ensure defaults do not replace inherited values.

### Step 9.2 — Phenotypic expression

For every intended heritable trait demonstrate:

```text
parent genome → offspring genome → phenotype → behavior/structure
```

### Step 9.3 — Mutation

Verify intended probability/rule, valid mutation range, parent isolation, offspring inheritance, and genome validity.

### Step 9.4 — Deterministic tests

Use controlled seeds to test both inheritance without mutation and mutation cases.

### Gate 9

Offspring inherit valid traits and mutation can introduce heritable variation without corrupting either generation.

---

# 16. Phase 10 — Demonstrate Actual Evolutionary Opportunity

**Objective:** Prove evolution is causally possible without prescribing its outcome.

### Step 10.1 — Identify selectable traits

List heritable traits that can affect survival, resource acquisition, construction, or reproduction.

### Step 10.2 — Trace causal influence

For each trait prove:

```text
trait → phenotype/behavior → survival/reproduction → offspring contribution
```

A genome field that never reaches phenotype is not yet an evolutionary trait.

### Step 10.3 — Multi-generation runs

Demonstrate multiple generations, inherited variation, population turnover, and the possibility of changing trait distributions.

Do not hard-code desired evolutionary outcomes.

### Gate 10

The simulation supports genuine generational turnover and heritable variation capable of affecting reproductive success.

---

# 17. Phase 11 — Population and Ecosystem Viability

**Objective:** Judge the integrated system as an ecosystem.

### Step 11.1 — Establish baseline runs

Run fixed seeds at progressively longer durations and record:

- population;
- births/deaths;
- age/development distribution;
- generations;
- transformations;
- material totals;
- energy totals;
- environment totals;
- active transformation count;
- tick performance.

### Step 11.2 — Classify failure modes

Investigate:

- immediate extinction;
- zero reproduction;
- runaway population;
- immortal organisms;
- runaway material/energy;
- resource depletion;
- transformation backlog;
- action lock-in;
- static environment;
- progressive slowdown.

### Step 11.3 — Trace before tuning

First classify failure as architectural, lifecycle, accounting, ordering, reachability, or parameter-driven. Only tune parameters after correctness is established.

### Step 11.4 — Define stability criteria

A successful baseline demonstrates recurring births/deaths, multiple generations, continuing environmental turnover, bounded quantities where appropriate, and no permanent subsystem stall.

### Gate 11

At least one baseline configuration sustains a functioning environment and population through repeated lifecycles for a substantial run.

---

# 18. Phase 12 — Long-Run Soak Testing

**Objective:** Prove viability persists beyond demonstrations.

### Step 12.1 — Deterministic soak

Run a fixed seed for a large tick count and check invariants periodically.

### Step 12.2 — Multi-seed soak

Run multiple seeds to detect accidental seed-specific success.

### Step 12.3 — Numeric safety

Continuously check NaN, Infinity, forbidden negatives, invalid geometry, invalid genome values, impossible structures, and duplicate IDs.

### Step 12.4 — Memory stability

Monitor organisms, transformations, snapshots, historical statistics, retained references, and logs for growth unrelated to real simulated state.

### Step 12.5 — Performance stability

Measure tick time over the run. A simulation that becomes progressively unusable is not viable.

### Gate 12

Extended runs complete without progressive corruption, memory failure, or unacceptable degradation.

---

# 19. Phase 13 — Build the Observation Boundary

**Objective:** Expose authoritative state cleanly.

Preferred pipeline:

```text
Simulation → Snapshot → serialization → transport → frontend state → rendering
```

### Step 13.1 — Audit snapshots

Snapshots must be observational, authoritative, non-mutating, and free of stale duplicate authorities.

### Step 13.2 — Define observation payload

Eventually expose:

**Runtime:** tick, running/paused state, rate, health.  
**Environment:** dimensions, material distribution, resource types, vents, transformations.  
**Organisms:** identity, position, age, stage, structure/size, action, transformation, reproductive state, useful material/energy state.  
**Population:** population, births, deaths, generations, lifecycle/evolution trends.

### Gate 13

A real snapshot can be serialized and transported without changing simulation state.

---

# 20. Phase 14 — Make the UI a Scientific Window

**Objective:** Make the working simulation understandable to a human observer.

### Step 14.1 — World view

Show world bounds, real environment state, vents, real organism positions/size, and simulation tick.

### Step 14.2 — Organism inspection

Allow inspection of age, development, structure, current action, transformation, reproductive state, material/energy state, and useful genome/trait information.

### Step 14.3 — Environment inspection

Show resource distribution, active-field state, vent activity, transformations, and meaningful spatial gradients.

### Step 14.4 — Population/evolution view

Show population, births/deaths, generations, trait distributions, transformation activity, and material trends.

### Step 14.5 — Runtime controls

Expose only real backend operations such as start, pause, resume, reset, step, and explicitly supported configuration.

### Gate 14

The UI faithfully displays and helps diagnose the living simulation without becoming responsible for its progression or logic.

---

# 21. Phase 15 — Integrated Test Architecture

Tests must prove increasingly larger portions of the causal system.

## Level 1 — Unit

Resource equations, material properties, bond strength, transformation work, mutation, geometry.

## Level 2 — Subsystem

Vents, diffusion, settling, transformations, maintenance, construction.

## Level 3 — Lifecycle

```text
material → organism → COMBINE → structure
structure → BREAK → material
birth → development → reproduction → offspring
offspring → inherited trait → phenotype
```

## Level 4 — Integrated simulation

Environment + organism; organism + transformation; transformation + structure; structure + maintenance; readiness + reproduction; reproduction + genetics; multiple generations.

## Level 5 — Long-run

Large tick counts, conservation, population dynamics, numeric safety, transformation backlog, memory/performance.

### Critical rule

A test that manually constructs impossible state does not prove the live simulation can reach that state. Maintain live-path integration tests wherever practical.

---

# 22. Phase 16 — Instrumentation and Diagnostics

Instrumentation should observe, not alter, simulation behavior.

Track at minimum:

### Simulation

- tick;
- tick duration;
- population;
- active transformations;
- errors.

### Material

- total material by resource;
- environment-held material;
- organism-held material;
- transformation-held material;
- structural material.

### Energy

- created;
- consumed;
- transferred;
- total usable energy;
- unexplained changes.

### Lifecycle

- actions attempted/completed;
- transformations started/completed;
- births;
- deaths;
- reproductive construction progress.

Temporary diagnostics may be added to answer a specific question, then removed or converted into permanent instrumentation.

---

# 23. Phase 17 — Performance and Architecture Cleanup

Optimize only after correctness and viability are demonstrated.

Audit:

- repeated environment cloning;
- snapshot creation;
- spatial queries;
- transformation allocation;
- organism traversal;
- material cloning;
- serialization frequency;
- rendering frequency;
- unbounded history/statistics;
- lock contention;
- repeated computation of immutable resource properties.

Prefer measured improvements that preserve semantics: compact snapshots, bounded histories, spatial indexing where justified, and observation throttling independent of simulation tick rate.

### Gate 17

Performance improves without changing simulation behavior.

---

# 24. Phase 18 — Final Acceptance Run

Run from a clean start with no manual intervention.

### A — Startup

Server, simulation, environment, organisms, and runtime initialize correctly.

### B — Continuous ticking

Ticks advance continuously with no duplicate stepping or stalls.

### C — Environment

Vents, active field, diffusion/redistribution, settling/recycling, and conservation operate correctly.

### D — Organisms

Perception, decisions, movement/interaction, and resource use occur through real paths.

### E — Transformations

COMBINE and BREAK start, progress, complete, and produce correct results.

### F — Lifecycle

Growth/development, maintenance, and death operate correctly.

### G — Reproduction

Readiness → request → construction → offspring → independent life.

### H — Evolution

Inheritance → mutation → phenotype → multiple generations.

### I — Long run

Population and environment remain dynamically active without runaway accounting or progressive failure.

### J — Observation

UI reflects authoritative state and disconnect/reconnect does not change simulation semantics.

---

# 25. Milestones

1. **Authoritative runtime** — one reliable simulation progression path.
2. **Integrated tick lifecycle** — environment, organisms, transformations, reproduction, maintenance, cleanup connected.
3. **Material/energy authority** — no contradictory physical/accounting authorities.
4. **Environmental viability** — material continuously circulates through the modeled environment.
5. **Organism viability** — material interaction, transformation, structure, maintenance, development, death.
6. **Reproductive viability** — natural reproduction and independent offspring.
7. **Evolutionary viability** — inheritance, mutation, heritable phenotype.
8. **Ecosystem viability** — sustainable multi-generation dynamics.
9. **Observable living simulation** — UI faithfully exposes the system.
10. **Long-run quality** — extended correctness, stability, diagnostics, and performance.

These milestones are gates, not invitations to batch unrelated changes.

---

# 26. Current High-Priority Audit Targets

## 26.1 The continuous runtime already exists

Do not rebuild a tick loop blindly. The current backend already has a continuous loop around `Simulation::step()`. The immediate question is whether that runtime produces a **viable integrated simulation**.

## 26.2 Action reachability

Current disabled/no-op action paths must be classified against the organism lifecycle. If organisms cannot obtain usable material through a legitimate path, reproduction tuning is premature.

## 26.3 Material/energy authority

Complete the repository-wide migration away from legacy stored values. Intrinsic bond strength remains the sole bond-strength authority. Legacy energy behavior must not reintroduce an independent energy model.

## 26.4 Transformation integration

COMBINE and BREAK must be exercised through normal organism behavior, not merely direct transformation tests.

## 26.5 Reproduction reachability

Reproductive construction has received substantial implementation work. The next question is whether a normal organism can reach it from environment interaction and lifecycle conditions.

## 26.6 Population viability

Once the lifecycle closes, measure long-run behavior rather than assuming it will be stable.

---

# 27. What We Will Not Do

Unless an audit proves it necessary, we will not:

- create a second simulation engine in the frontend;
- add fake life to make the UI look active;
- add scripted reproduction to manufacture population activity;
- tune constants to conceal broken causal links;
- reintroduce bonded/unbonded partitioning into the unified deep reservoir;
- reintroduce stored bond strength as an authority;
- reintroduce an independent stored-energy model;
- replace correct mechanics merely for visualization convenience;
- optimize before measuring the bottleneck;
- declare success because a short demo looks active.

---

# 28. Definition of Done

## Runtime

- [ ] One authoritative simulation instance advances continuously.
- [ ] Browser is not required for simulation progression.
- [ ] Runtime lifecycle is coherent.
- [ ] Long runs remain stable.

## Environment

- [ ] Reservoir works.
- [ ] Vents work.
- [ ] Active field works.
- [ ] Diffusion/redistribution works.
- [ ] Settling/recycling works where modeled.
- [ ] Material accounting remains coherent.

## Chemistry/materials

- [ ] Resource properties have one authority.
- [ ] Material composition has one authority.
- [ ] Potential energy is derived consistently.
- [ ] Reactivity/cohesion are consistent.
- [ ] Bond strength uses only resource-property math.
- [ ] Legacy stored bond strength is not authoritative.
- [ ] Transformation work is coherent.
- [ ] Material conservation is demonstrated.
- [ ] Energy accounting is demonstrated.

## Organisms

- [ ] Birth state is valid.
- [ ] Perception reaches real environment state.
- [ ] Decisions select reachable actions.
- [ ] Material interaction is possible.
- [ ] COMBINE works end-to-end.
- [ ] BREAK works end-to-end.
- [ ] Structure changes correctly.
- [ ] Maintenance works.
- [ ] Development/growth works.
- [ ] Death/removal works.

## Reproduction

- [ ] Readiness is naturally reachable.
- [ ] Requests and commitments are correct.
- [ ] Construction progresses.
- [ ] Offspring is physically instantiated correctly.
- [ ] Offspring becomes independent.

## Evolution

- [ ] Genome inheritance works.
- [ ] Heritable traits affect phenotype.
- [ ] Mutation works.
- [ ] Multiple generations occur.
- [ ] Variation persists through generations.

## Population/ecosystem

- [ ] Births and deaths both occur.
- [ ] Multiple generations occur.
- [ ] Population dynamics remain bounded by model constraints.
- [ ] Environment remains active.
- [ ] No unexplained material/energy runaway occurs.
- [ ] Baseline configuration does not immediately and permanently collapse.

## Observation

- [ ] Real simulation state reaches UI.
- [ ] UI does not simulate outcomes.
- [ ] Environment is observable.
- [ ] Organisms are inspectable.
- [ ] Transformations are inspectable.
- [ ] Reproduction/population are inspectable.
- [ ] Long-run trends are observable.

## Quality

- [ ] Unit tests pass.
- [ ] Integration tests pass.
- [ ] Lifecycle tests pass.
- [ ] Long-run tests pass.
- [ ] Multi-seed tests pass the intended invariants.
- [ ] Numeric invariants hold.
- [ ] Performance is acceptable.

---

# 29. Immediate Implementation Protocol

The next implementation task is **not automatically the next numbered phase**. It is the first unresolved blocker found by auditing the current live system.

For each blocker:

1. Audit the actual runtime path.
2. Identify the first broken causal link.
3. State the invariant that should hold.
4. Find every relevant code location.
5. Choose the smallest coherent fix.
6. Implement only that fix.
7. Run focused tests.
8. Run the relevant integrated path.
9. Inspect actual runtime behavior when needed.
10. Record the result.
11. Re-audit adjacent paths.
12. Move to the next blocker only after verification.

The repair direction is:

```text
simulation authority
        ↓
material / chemistry / environment
        ↓
organism interaction
        ↓
transformations
        ↓
structure / maintenance / development
        ↓
reproduction
        ↓
genetics / evolution
        ↓
population viability
        ↓
long-run stability
        ↓
observation / UI
        ↓
optimization
```

The final product is not a continuously animated webpage.

It is a **continuously running artificial-life system whose behavior emerges from the interaction of its modeled rules, with the browser providing a faithful window into that system.**
