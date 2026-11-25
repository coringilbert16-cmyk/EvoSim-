# Phase 3C — Open-Boundary Investigation

**Status:** Investigation in progress; no open-boundary method approved yet.
**PR:** #446
**Date:** 2026-11-25

## Purpose

Establish a numerically validated treatment for the finite chemistry-world boundary before changing the live resonance field. The approved world is finite, and outgoing waves must leave permanently without meaningful reflection. Their energy must be recorded as exported-to-void energy in the existing conservation authority.

This investigation is part of Phase 3C. It does not authorize Phase 3D material coupling, chemistry integration, organism behavior, or a production storage-layout replacement.

## Source audit

The current PR #446 implementation has the following relevant behavior:

- `LiveResonanceField::new` creates a 32 × 32 grid centered on the world, regardless of the passed world dimensions.
- `advance_one_tick` calls `advance_ticks_with_domain_expansion`; the computational domain grows before nonzero state reaches the numerical edge.
- `ensure_position_covered` can expand the grid for arbitrary positions.
- `AdaptiveWaveField` holds its outermost leaf cells at zero. This is a fixed-zero numerical boundary and is not an open physical boundary.
- The adaptive solver has an optional graded sponge. Its discrete damping sink is measured against the disturbance-only discrete energy.
- The live adapter currently reports computational outflow as zero.
- The composition model defines total energy density as disturbance energy plus ambient oscillator energy plus the ambient/disturbance cross-energy. Its total flux includes ambient/disturbance cross-flux.
- Therefore the current disturbance-only sponge loss cannot simply be copied into the live ledger as total-field exported energy. The change in cross-energy and the ambient drive work must be included.
- The existing reference solver also has a graded sponge, explicitly documented as approximate rather than perfectly matched. The source audit did not find validation of reflected energy for oblique incidence or a demonstrated finite-world total-field export balance.

## Required investigation

### 1. Establish boundary test profiles

Use reproducible, phase-resolved wave packets with known direction and frequency content. Include:

- Normal incidence on each of the four sides.
- Oblique incidence at multiple angles, including near-grazing incidence.
- Multiple resolved frequencies.
- Corner-directed propagation.
- Superposed/interfering packets with differing phases.

The initial profiles must be compactly supported and placed far enough from the boundary to establish a clean incident baseline. Do not truncate nonzero tails or introduce an amplitude cutoff.

### 2. Measure reflection independently of energy loss

For each candidate boundary configuration, measure the returning wave in an interior observation region after the incident packet reaches the boundary. Separate incident, transmitted/exported, and reflected components by direction/time window; do not infer low reflection merely from a large sponge energy sink.

Record the reflected-to-incident energy ratio and establish an explicit acceptance tolerance before selecting parameters. Repeat at the approved spatial and temporal resolution and verify deterministic replay.

### 3. Reconcile the composed field

For the live ambient-plus-disturbance field, compute the total energy change across each solver substep or a rigorously equivalent tick interval. Include:

- Disturbance energy.
- Ambient oscillator energy (owned and accounted by the ambient mode, not duplicated).
- Ambient/disturbance cross-energy.
- Signed ambient world-drive work.
- Energy exported through the open boundary.

The independent world inventory must reconcile initial field energy plus ambient world work against current field energy plus cumulative exported-to-void energy, within a declared numerical tolerance. A disturbance-only damping integral is insufficient unless its equivalence to this complete balance is derived and tested.

### 4. Select the method only after evidence

The graded sponge is a candidate, not a default. If its reflected energy cannot meet the declared tolerance across the required incidence/frequency cases, investigate a better-matched open-boundary treatment. Do not silently treat fixed-zero reflection or arbitrary damping as physical escape.

## Production integration gates

After a boundary treatment passes the isolated tests:

1. Initialize the live grid from actual environment width, height, and world origin, at the approved base spacing and refinement ceiling.
2. Remove expansion from the live tick and live position-query paths. A world resize must be an explicit environment change.
3. Apply the validated boundary treatment on all four sides and corners.
4. Record complete composed-field export in the existing EnergyLedger exactly once; do not add a second ledger.
5. Validate interior invariance before a wave reaches the boundary.
6. Validate multi-tick world-inventory reconciliation through at least one boundary-crossing event.
7. Measure full-domain runtime and memory. Keep storage optimization separate from boundary physics.
8. Require full exact-head CI green before Phase 3C advances.

## Current disposition

No production solver behavior has been changed by this investigation. PR #446 remains unmerged and does not yet satisfy the approved finite-domain model. Phase 3D remains on hold.
