# EvoSim — System Integration Guiding Principles

## Purpose

This is a **directional reference**, not a specification, contract, or ultimate authority.

Its purpose is to preserve the development perspective that guides EvoSim as the implementation evolves.

The repository's current code is the source of truth for what currently exists. The Master Spec, Workshop material, roadmap, tests, and other documents are design references. When implementation experience reveals a better way to realize the intended simulation, the implementation may change and the supporting documents may become stale.

No document should be treated as permanently authoritative over the living architecture.

## Primary Goal

> **Full-system integration is the goal.**

EvoSim should become a coherent, continuously running artificial-life system in which the major subsystems participate in one causal loop and produce the intended evolutionary dynamics together.

The objective is not to maximize the number of individually completed features. It is to make the whole system work as an interconnected physical, chemical, biological, behavioral, and evolutionary process.

A useful target causal loop is:

```text
DEEP ENVIRONMENT
      ↓
environmental processes
      ↓
ACTIVE ECOLOGICAL FIELD
      ↓
physical material availability
      ↓
PERCEPTION
      ↓
DECISION
      ↓
PHYSICAL INTERACTION
      ↓
CHEMISTRY / TRANSFORMATIONS
      ↓
PHYSICAL ORGANISM STATE
      ↓
growth / maintenance / damage / repair
      ↓
maturity / reproductive readiness
      ↓
REPRODUCTION
      ↓
INHERITANCE / VARIATION
      ↓
NEW VIABLE ORGANISM
      ↓
survival / selection
      └──────────────→ next generation
```

This diagram is a conceptual target, not a mandate that every subsystem must use a particular implementation.

## How to Make Decisions

For each proposed change, ask:

1. What part of the complete causal system does this affect?
2. What existing systems should cause it, consume it, or respond to it?
3. Where is the authoritative state for the concept involved?
4. Does the change create a duplicate authority or bypass an existing physical/chemical constraint?
5. Does it strengthen or weaken the connection between subsystems?
6. What feedback loop does it create or alter?
7. Does the resulting behavior move EvoSim toward the intended evolutionary dynamics?
8. Is there a simpler implementation that preserves the same system-level behavior?

Prefer changes that improve several connected parts of the system without introducing special-case rules.

## Documents Are Guides, Not Commands

The Master Spec, Workshop, roadmap, audits, design notes, and historical documents should be read for intent, reasoning, constraints, and useful prior work.

They should **not** be treated as immutable implementation commands.

If a document conflicts with:

- a newer explicit design decision;
- a demonstrated property of the current implementation;
- a discovered dependency;
- a better physical model; or
- the requirements of full-system integration;

then the conflict should be surfaced, reasoned through, and resolved rather than blindly obeyed.

The goal is to preserve the **purpose** of a rule while allowing its implementation to evolve.

## Authority Principles

Use the narrowest appropriate authority for each concept.

Examples:

- immutable resource properties define intrinsic resource characteristics;
- actual physical structure defines current structural state;
- inherited blueprint defines what structural configuration an organism is permitted to express;
- physical geometry determines physical contact and enclosure;
- chemistry determines chemical interaction outcomes;
- decision systems choose among mechanically eligible possibilities rather than inventing physical eligibility;
- memory records bounded consequences rather than becoming a hidden physics cache;
- the environment owns environmental material state;
- UI observes simulation state rather than creating a second simulation.

These are architectural tendencies. If implementation requires a different representation, preserve the underlying causal separation rather than the particular data structure.

## Integration Over Local Completion

A subsystem is not considered truly complete merely because:

- its unit tests pass;
- its functions work in isolation;
- its data structure exists;
- its UI can display it; or
- a roadmap phase says it is complete.

A subsystem is complete enough when it is correctly connected to the rest of the simulation and its behavior participates in the intended causal loop.

When a local implementation is technically correct but disconnected from the rest of the system, integration is the higher priority.

## Emergence and Physical Causality

EvoSim should favor simple interacting rules whose consequences emerge from the system rather than procedural scripts that directly produce desired outcomes.

Whenever possible:

```text
physical state
      ↓
physical/chemical consequence
      ↓
organism state
      ↓
behavior
      ↓
new physical consequence
```

Avoid shortcuts of the form:

```text
desired biological result
      ↓
magic state change
```

This does not prohibit abstractions. It means abstractions should preserve causal relationships rather than bypass them.

## Evolutionary Test

When uncertain whether a mechanic belongs in the architecture, ask:

> **If this system ran for a very long time, what evolutionary pressure would this rule create?**

Then ask:

> **Is that pressure one we actually want?**

A mechanic can be internally consistent and still produce the wrong evolutionary dynamics. System-level evolutionary consequences therefore matter as much as local correctness.

## Living Reference

This file should remain short and stable.

It should be updated only when the **development philosophy itself** changes. It should not be expanded into a detailed implementation specification.

Implementation details belong in code, tests, focused design notes, and temporary planning documents as appropriate.

The guiding principle remains:

> **Build the whole system. Let the details change as needed to make the whole system coherent, causal, physically grounded, and evolutionarily meaningful.**
