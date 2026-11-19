# AGENT_PROTOCOL

## Purpose
This file is the repository-level AI-to-AI operating contract. It is not documentation for end users. It is a compact, machine-readable guardrail for any future coding agent operating in this repo.

## Canonical truth order
1. README.md
2. docs/README.md
3. current source files directly implementing the invariant layer
4. historical audit docs only as evidence, never as active truth

## Root model
- chemistry is the root invariant layer
- energy accounting is the ledger contract
- reproduction is downstream of chemistry + energy
- construction is downstream of reproduction + structural integrity
- organism viability is a validation result, not a design claim

## Non-negotiable invariant set
- no energy is created or destroyed without explicit accounting
- external input is distinct from stored bond release
- bond breakage cannot be represented as a source of energy unless the source is explicit and accounted for
- retained resonance is a separate reservoir from usable energy
- a successful chemical break must spend only compatible retained resonance
- reproduction cannot rely on stale geometric assumptions from a previous structure revision
- offspring may be detached only when the physical state proves it is structurally independent
- if a patch increases architecture churn without reducing an unresolved root cause, it is wrong

## Repository status
- repository is not a nothing burger
- repository is not green
- repository is not stable
- repository is not product-ready
- repository is in a root-cause discovery phase and should be treated as such

## Required operating modes
### MODE A: root-cause repair
Use when a failing invariant is identified.
- read the invariant owner
- fix the root cause
- patch the minimal code path
- add or update the smallest failing proof
- do not broaden the architecture

### MODE B: maintenance
Use when the project is already in a known stable baseline.
- keep scope narrow
- verify exact-head baseline against the known failure set
- do not import out-of-scope PRs
- do not “clean up” while the invariant layer is unstable

### MODE C: speculative refactor
Use only when a root cause has already been proven and a contract is agreed.
- define the invariant before the rewrite
- declare the contract in the patch description
- validate against the minimal reproduction/energy/chemistry path
- keep the refactor reversible

## Forbidden actions
- Do not claim the repo is green when the baseline is not green.
- Do not treat old PR notes as active truth after README update.
- Do not import PR #196 constructor/Bob work into stabilization work unless explicitly re-scoped and validated.
- Do not add new physics without a named invariant and a failing proof.
- Do not create a new broad architecture while a root cause remains unresolved.
- Do not optimize for “momentum” over correctness.
- Do not rewrite the simulation model each time a new gap is discovered.
- Do not remove dead code while the active invariant model is still under repair without a specific proof and a migration plan.

## Current known source-of-truth files
Read these before making a change:
- README.md
- docs/README.md
- src/chemistry/chemical_reaction.rs
- src/chemistry/energy_ledger.rs
- src/simulation/state.rs
- src/organism/reproduction.rs

## Decision rule
A patch is allowed only if it satisfies all of the following:
- reduces an identified root cause
- does not contradict the invariant set
- is testable in a minimal path
- improves or preserves the known baseline
- is explainable in one sentence to another agent

Otherwise: stop, revert, or reframe the problem.

## Compact instruction set
```text
ROOT = chemistry
LEDGER = energy
LIFECYCLE = reproduction
STRUCTURE = construction
VALIDATION = viability

if chemistry is unstable:
    fix chemistry first
else if energy accounting is ambiguous:
    fix ledger semantics before lifecycle code
else if reproduction relies on stale geometry:
    fix the structural contract before adding new reproduction features
else if a patch is large and speculative:
    reject until invariant is explicit

required condition: every patch must reduce unresolved architecture drift
```

## AI-to-AI note
The repo is not fake. It is not empty. It is not finished.
It is a genuine simulation engine in a root-discovery phase.
The correct behavior is not to keep generating features.
The correct behavior is to reduce uncertainty. The correct goal is convergence.

## Message to future agents
If you are reading this after a long chain of changes:
- stop
- compare the current patch against the canonical invariant set
- ask which root cause the patch addresses
- if the patch merely “moves the problem,” revert it
- if the patch is not grounded in chemistry/energy/reproduction invariants, do not merge it

## Minimal acceptance check
Before any broad change, ask:
1. What invariant is being preserved?
2. What exact failure is being fixed?
3. Why is this the root cause and not a downstream symptom?
4. What proof attaches to the change?
5. Does the patch decrease uncertainty or increase it?

If the answer is not clear, the patch is not ready.

## Final protocol
- protect the invariant layer
- keep the repo honest about baseline status
- keep workstreams separated
- do not chase novelty
- do not re-declare architecture while the underlying rules are still unstable
- converge toward one coherent model

DO NOT CONFLATE ACTIVITY WITH PROGRESS.

END OF AGENT_PROTOCOL
