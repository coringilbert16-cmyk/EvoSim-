# Phase 5 — Single-Cell Maintenance & Survival

## Status

Phase 5 design is APPROVED. The initial implementation audit is complete enough to establish the frozen authority boundary. Code changes must preserve the existing genome, physical-structure, death, and decomposition authorities.

## Objective

Implement and harden continuous maintenance and survival pressure for an organism without introducing a second life/death authority, a generic health system, age-based deterioration, or an artificial energy-battery model.

The governing chain is:

> Genome defines life → physical graph defines organism membership → maintenance creates ongoing demand → energy shortage produces stress → stress can damage physical structure → existing death authority determines when the organism ceases to remain viable → decomposition handles the resulting physical material.

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

Any unpaid maintenance deficit becomes stress.

Insufficient maintenance energy does not directly constitute death.

The resulting stress can cause physical structural damage through the already-approved stress-damage mechanism.

### P5-5 — Movement energy
Movement has no explicit movement-energy cost in Phase 5.

No speed, acceleration, friction, terrain, pushing-strength, or movement-energy equation is introduced.

### P5-6 — Stress mechanism
The existing heat/stress pathway remains authoritative:

> energy transaction → heat/stress → threshold → random eligible non-genome structural bond

Non-genome structural bonds are damaged before genome bonds. Genome capability-degradation semantics remain outside Phase 5.

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

Existing usable_energy, stress, and stress_threshold may participate in maintenance because they already have defined mechanical roles.

Genome remains the life/identity authority. Physical structure remains the physical authority. Death remains the lifecycle authority.

## Authority boundaries

| Authority | Responsibility |
|---|---|
| Genome | Distinguishes life from non-life and carries inherited structural/developmental information |
| Physical graph | Determines realized organism structure and physical membership |
| Maintenance | Creates ongoing structural maintenance demand |
| Energy ledger | Accounts for maintenance energy transactions |
| Stress | Carries accumulated heat/maintenance deficit consequences |
| Stress damage | Alters actual physical structure through eligible bond damage |
| Death | Determines when an organism can no longer remain a viable organism |
| Decomposition | Processes the physical material of a dead organism |

P5 must not duplicate or override these authorities.

## Current implementation audit

The current repository already contains a maintenance/stress pipeline:

- Organism::structural_mass() derives mass from realized structural units.
- Organism::apply_maintenance() computes demand from structural mass and MAINTENANCE_ENERGY_PER_MASS.
- Paid maintenance is settled through EnergyReason::Maintenance.
- An unpaid maintenance deficit becomes stress.
- Stress is decayed each simulation tick before stress damage is applied.
- Organism::apply_stress_damage() invokes the existing stress-break transformation.
- Simulation cleanup removes organisms for which the existing stress-damage path reports death and transfers their physical structure into DecomposingBody.
- Decomposition preserves the organism's realized structure and energy budget before releasing physical material.

### Existing implementation discrepancy to resolve

Simulation::apply_energy_capacity() currently performs stress decay and stress damage. Its name does not describe that responsibility and must not be treated as a separate energy-capacity/death authority.

Phase 5 should audit and, if appropriate, rename/refactor this boundary without changing its established semantics.

### Existing death boundary

The simulation currently treats the boolean result of the established stress-damage path as the signal to recycle the organism into decomposition.

Phase 5 must verify that this behavior exactly matches the README's death/genome/physical-structure authority before changing it.

If the existing implementation is incomplete relative to those rules, the discrepancy must be isolated rather than silently filled with a new viability rule.

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

## Implementation sequence

### P5.0 — Authority audit — COMPLETE

Trace maintenance, energy ledger, stress, structural damage, genome identity, death, and decomposition paths.

Freeze the rules above before changing code.

### P5.1 — Maintenance authority

Verify the maintenance calculation and ledger settlement against the frozen rules.

Add/adjust contract tests for:

- structural-mass-based demand,
- full maintenance payment,
- partial payment,
- zero usable energy,
- ledger accounting,
- and no maintenance-specific death shortcut.

### P5.2 — Stress integration

Verify the common heat/stress pathway and stress dissipation.

The existing physical genome-cavity authority provides the genome-bond distinction needed by stress damage. The qualifying cavity boundary identifies the physical genome bonds. P5 must not infer genome membership from fixed materials, unit counts, indices, or other legacy proxies.

Add/adjust contract tests for:

- maintenance heat,
- unpaid maintenance deficit,
- stress decay,
- threshold-triggered structural damage,
- random eligible non-genome bond selection,
- genome-bond protection while non-genome bonds remain.

### P5.3 — Survival/death boundary

Audit the exact transition from stress damage to the existing death/recycling path.

Ensure no second viability or health authority is introduced.

Add/adjust tests for:

- intact organism surviving maintenance,
- structural damage remaining physical,
- death transition through the existing authority,
- realized material preserved into decomposition,
- and dead organisms removed from the live-organism collection.

### P5.4 — Contract audit

Run full tests, formatting, architecture checks, and strict Clippy.

Audit all P5 call sites for duplicate maintenance, duplicate stress decay, duplicate death decisions, or alternate energy-capacity authorities.

## P5 completion criteria

- [ ] Continuous maintenance is implemented through one authority.
- [ ] Demand is derived from realized structural mass.
- [ ] Maintenance uses the energy ledger.
- [ ] Unpaid maintenance becomes stress rather than direct death.
- [ ] Movement remains free of an explicit energy cost.
- [ ] Stress uses the existing heat/stress mechanism.
- [ ] Stress dissipation does not repair damage.
- [ ] No age/aging/health/death-countdown authority is introduced.
- [ ] Genome remains the life/non-life authority.
- [ ] Physical graph remains the organism-structure authority.
- [ ] Existing death authority remains the death authority.
- [ ] Decomposition preserves physical material.
- [ ] Contract tests cover the complete maintenance → stress → damage → death/decomposition boundary.
- [ ] Full test suite passes.
- [ ] Formatting and architecture checks pass.
- [ ] Strict Clippy passes.