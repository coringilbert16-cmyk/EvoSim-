# Phase 5 — Single-Cell Maintenance & Survival

## Status

**Phase 5 COMPLETE**

Phase 5 implements continuous maintenance and survival pressure for an organism without introducing a second life/death authority, a generic health system, age-based deterioration, or an artificial energy-battery model.

The governing chain is:

> Genome defines life → physical graph defines organism membership → maintenance creates ongoing demand → energy shortage produces stress → stress can damage physical structure → existing death authority determines when the organism ceases to remain viable → decomposition handles the resulting physical material.

## Objective

Implement and harden continuous maintenance and survival pressure for an organism while preserving the existing genome, physical-structure, death, and decomposition authorities.

## Frozen decisions

### P5-1 — Continuous maintenance
An organism continuously incurs maintenance demand while it exists.

Maintenance is not an age system and does not itself define life or death.

### P5-2 — Maintenance demand
Maintenance demand is based on realized physical structural mass:

> maintenance demand = realized structural mass × maintenance coefficient

The physical structure is authoritative. No maintenance-specific body-part taxonomy or maintenance gene is introduced.

MAINTENANCE_ENERGY_PER_MASS remains the initial simulation coefficient. Its numerical value is a tunable parameter, not inherited biological information.

### P5-3 — Maintenance energy
Maintenance consumes usable energy through the existing energy-ledger transaction system.

Maintenance must not bypass ledger accounting.

The transaction's heat consequence remains part of the common energy/stress pathway.

### P5-4 — Insufficient maintenance energy
An organism pays as much of its maintenance demand as it can.

Any unpaid maintenance deficit is accumulated as persistent maintenance debt. While current maintenance remains unpaid, the current deficit and accumulated debt become stress pressure.

Insufficient maintenance energy does not directly constitute death.

The resulting stress can cause physical structural damage through the approved stress-damage mechanism.

### P5-5 — Movement energy
Movement has no explicit movement-energy cost in Phase 5.

No speed, acceleration, friction, terrain, pushing-strength, or movement-energy equation is introduced.

### P5-6 — Stress mechanism
The existing heat/stress pathway remains authoritative:

> energy transaction → heat/stress → threshold → random eligible non-genome structural bond

Random eligible non-genome structural bonds are damaged while such bonds remain. Genome bonds become eligible only after non-genome candidates are exhausted. Genome capability-degradation semantics remain outside Phase 5.

### P5-7 — Stress dissipation
Accumulated stress dissipates over time.

Stress dissipation does not repair structural damage.

STRESS_DECAY_PER_TICK is a simulation parameter, not an aging mechanism.

### P5-8 — Stress thresholds
The existing adaptive threshold behavior remains:

1. threshold is reached,
2. eligible structural damage occurs,
3. threshold decreases,
4. threshold remains bounded by the minimum threshold.

The numerical constants remain simulation parameters pending calibration.

### P5-9 — Spontaneous degradation
No spontaneous structural degradation is introduced merely because a simulation tick passes.

Structural damage requires an established causal mechanism.

No age, aging rate, lifespan, health decay, or death countdown is introduced.

### P5-10 — Internal state
P5 does not add a generic viability/health state.

Maintenance debt is a persistent accounting state with one mechanical meaning: historical maintenance that was not paid. It is not a health score, viability state, starvation timer, or death authority.

Existing usable_energy, stress, and stress_threshold participate in maintenance because they already have defined mechanical roles.

Genome remains the life/identity authority. Physical structure remains the physical authority. Death remains the lifecycle authority.

## Authority boundaries

| Authority | Responsibility |
|---|---|
| Genome | Distinguishes life from non-life and carries inherited structural/developmental information |
| Physical graph | Determines realized organism structure and physical membership |
| Maintenance | Creates ongoing structural maintenance demand |
| Energy ledger | Accounts for maintenance energy transactions |
| Stress | Carries accumulated heat/maintenance-deficit consequences |
| Stress damage | Alters actual physical structure through eligible bond damage |
| Death | Determines when an organism can no longer remain a viable organism |
| Decomposition | Processes the physical material of a dead organism |

P5 does not duplicate or override these authorities.

## Implementation sequence

### P5.0 — Authority audit — COMPLETE
Traced maintenance, energy ledger, stress, structural damage, genome identity, death, and decomposition paths and froze the authority boundary before implementation.

### P5.1 — Maintenance authority — COMPLETE
Verified and tested:

- structural-mass-based maintenance demand,
- full maintenance payment,
- partial payment,
- zero usable energy,
- ledger accounting,
- no maintenance-specific death shortcut.

### P5.2 — Stress integration — COMPLETE
Verified and tested:

- maintenance heat enters the common stress pathway,
- unpaid maintenance deficit becomes stress,
- stress dissipates over time,
- threshold-triggered structural damage,
- random eligible non-genome bond selection,
- genome-boundary bond protection while non-genome bonds remain.

Genome-bond eligibility is derived from the existing physical genome-cavity authority rather than fixed materials, unit counts, indices, or legacy proxies.

### P5.3 — Survival/death boundary — COMPLETE
Audited and tested:

- intact organisms survive ordinary maintenance,
- structural damage remains physical,
- death transitions through the existing stress-damage/decomposition path,
- realized physical material is preserved into decomposition,
- dead organisms are removed from the live-organism collection,
- finished decomposition with no remaining bonds releases realized physical material into the environment.

No second viability or health authority was introduced.

### P5.4 — Contract audit — COMPLETE
Final audit confirms:

- one maintenance authority,
- one stress-dissipation/damage path,
- no duplicate death decision,
- no alternate energy-capacity authority,
- movement has no explicit P5 energy cost,
- physical structure remains authoritative,
- genome remains the life/non-life authority,
- decomposition preserves physical material.

## Completion criteria

- [x] Continuous maintenance is implemented through one authority.
- [x] Demand is derived from realized structural mass.
- [x] Maintenance uses the energy ledger.
- [x] Unpaid maintenance accumulates as persistent debt and contributes to stress while current maintenance remains unpaid; it does not directly cause death.
- [x] Movement remains free of an explicit energy cost.
- [x] Stress uses the existing heat/stress mechanism.
- [x] Stress dissipation does not repair damage.
- [x] No age/aging/health/death-countdown authority is introduced.
- [x] Genome remains the life/non-life authority.
- [x] Physical graph remains the organism-structure authority.
- [x] Existing death authority remains the death authority.
- [x] Decomposition preserves physical material.
- [x] Contract tests cover the complete maintenance → stress → damage → death/decomposition boundary.
- [x] Full test suite passes.
- [x] Formatting and architecture checks pass.
- [x] Strict Clippy passes.

## Explicit non-goals

Phase 5 does not redesign:

- genome definition,
- genome capability semantics,
- developmental-field blueprint,
- construction solver,
- physical-structure authority,
- movement,
- collision/pushing,
- COMBINE chemistry,
- BREAK chemistry,
- resource catalog,
- environmental reservoir/active-field rules,
- adulthood/maturation,
- reproduction,
- death semantics,
- decomposition semantics,
- or evolution.

Phase 5 establishes the maintenance/survival foundation only. Future lifecycle phases must build on these authorities rather than creating competing ones.
