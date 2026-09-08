# EvoSim Current State

This file describes the implementation state of the repository on the current `main` line. It is a snapshot of implemented behavior, not a replacement for the authoritative specification.

## Authority

- `Master Spec Sheet V5.md` defines intended simulation behavior.
- `CURRENT_STATE.md` describes what the repository currently implements.
- Other phase/checkpoint documents are historical unless explicitly incorporated into the current implementation.

## Core architecture

- The simulation is a continuous Rust simulation with a browser-served UI.
- Environment resources are represented through an active ecological field and a deeper reservoir.
- Vents transfer material from the reservoir into the active field without selecting for bonded versus unbonded material.
- Organisms have physical structure, material composition, geometry, perception, movement, memory, decision logic, and reproduction systems.

## Material and transformation model

- Resource properties include mass, potential energy, reactivity, and cohesion.
- Material potential energy is derived from constituent resources rather than stored as an independent material energy field.
- Reactivity is represented as an exponential property.
- Transformation energy is state-dependent and may be positive or negative.
- COMBINE and BREAK are therefore not inherently energy-positive or energy-negative operations.
- `bond_strength` is a structural connection-strength concept and is distinct from bond formation energy.
- Connection load is derived from the experimental bond-strength function applied to bond formation energy rather than from the legacy stored `Bond::strength` value.

## Physical interfaces

- Boundary contact and interface geometry are separate concerns.
- Interpenetrating polygon placements are prevented from producing a false finite shared boundary when their boundaries cross transversely or one polygon is strictly contained inside another.
- Exact coincident or partially shared collinear boundaries retain their intended interface behavior.

## Organism lifecycle

- Reproduction begins with a decision to invest enough resources to construct the genome core.
- The offspring is then built toward a juvenile target of approximately 40% of adult size according to the parent's structural blueprint.
- The blueprint retains its shape while the offspring is proportionally smaller.
- Birth/detachment occurs when the juvenile stage is reached; growth is not modeled as exclusively outside-in.
- Stress decays over time.

## Repository structure

- `src/` contains Rust source.
- `ui/` contains browser UI assets.
- `docs/` contains the continuous-simulation roadmap and supporting documentation.
- `scripts/` contains repository tooling.

This document should be updated when an architectural or lifecycle change is merged into `main` and the implementation materially changes these statements.
