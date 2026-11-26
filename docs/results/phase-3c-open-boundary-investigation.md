

## Next candidate — outgoing characteristic with discrete flux accounting

The next candidate is an outgoing-characteristic boundary condition applied to the disturbance, with export measured from the **composed** field flux. This is an isolated numerical candidate only; it is not selected for production.

For an edge with outward unit normal \(\mathbf n\), the normal-incidence continuum characteristic is

\[
\partial_t u + c\,\partial_n u = 0.
\]

This is the Sommerfeld one-way condition for a wave leaving the domain. It is a useful baseline, but it is not an exact condition for oblique waves: a first-order normal characteristic can reflect grazing and oblique components. Corners also need a separately defined treatment; applying two independent edge updates at a corner is not automatically energy-consistent.

The physical outward flux to be accounted is the normal component of the already implemented composed flux,

\[
q_n = -Kc^2(a_t+u_t)\,\partial_n u.
\]

The sign convention is positive for energy leaving the finite world. The ambient oscillator inventory remains owned by the ambient mode. The boundary export is not an additional sink inferred from damping: it must be obtained from the discrete boundary work/flux of the actual solver update, and the same amount must be booked once in the existing world inventory.

### Candidate evaluation order

1. Implement the characteristic condition only in the isolated uniform reference solver, retaining the fixed-zero solver as a control.
2. Measure incident and returning **energy**, not just pointwise peak amplitude. Use directional/time-window separation and a probe region sufficiently far inside the edge.
3. Sweep all four edge orientations, both signs of oblique angle, near-grazing incidence, at least three well-resolved wavelengths, and corner-directed packets. Include two-packet interference with both aligned and opposing phase.
4. Repeat the same cases with the graded sponge as a control. Do not assume that adding a sponge improves the characteristic condition.
5. Independently check the solver's discrete energy change against integrated boundary work. Reject any case where the numerical boundary creates energy or where the flux ledger does not close.
6. Only if the first-order characteristic fails the declared reflection tolerance should a higher-order nonreflecting treatment or PML be investigated. Do not add PML complexity pre-emptively.

### Required measurements per case

Record solver resolution and timestep, boundary side/corner, incidence angle, wavelength, packet phase, incident energy, reflected energy, reflected/incident energy ratio, net outward boundary work, total field-energy change, and conservation residual. Record the same case twice to verify deterministic replay.

No acceptance threshold is chosen by this note. It must be declared before comparing candidate configurations, rather than adjusted after seeing the sweep. No production boundary code, live energy-ledger behavior, or Phase 3D coupling is authorized by this candidate description.

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

## Preliminary reflection sweep (2026-11-25)

An isolated reference-solver diagnostic was added and run on the CI benchmark job. It uses a right-side boundary, a single resolved packet profile, a 12-cell sponge, and measures the returning interior peak relative to the incident peak. The values below are **amplitude ratios**, not energy ratios.

| Incidence angle | Damping 0.000 | Damping 0.050 | Damping 0.150 | Damping 0.400 |
|---|---:|---:|---:|---:|
| 0° | 0.8382 | 0.6894 | 0.4659 | 0.1754 |
| 25° | 0.1911 | 0.1574 | 0.1071 | 0.0405 |
| 45° | 0.000893 | 0.000682 | 0.000412 | 0.000153 |

The corresponding measured absorbed-energy values increased with damping, but that sink is not itself proof of low reflection. The large normal-incidence return (17.5% amplitude even at maximum tested damping) is not acceptable evidence for a validated open boundary. The strong angle dependence also means the 25° and 45° results cannot be used to infer normal or near-grazing behavior.

**Disposition:** preliminary only. This sweep covers one side, one frequency/profile, and three angles; it does not include near-grazing incidence, all sides, corners, or interference. It does not select sponge parameters, establish an acceptance tolerance, validate energy export, or authorize production integration. Expand the diagnostic matrix before selecting a boundary method.

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

## Hybrid soft-plus-hard boundary experiment (2026-11-25)

The proposed hybrid is now represented by an isolated, ignored reference-solver diagnostic. It is not connected to the live solver and does not alter production physics.

The experiment applies the existing graded sponge and then explicitly clears a narrow terminal strip at the right edge after each solver substep. It sweeps two sponge widths (12 and 20 cells) and two maximum damping values (0.15 and 0.40). It reports independently:

- Reflected-to-incident **amplitude** ratio at an interior probe.
- The reference solver's exact discrete soft-layer absorbed-energy total.
- The signed change in discrete field energy caused by terminal-strip removal.
- Remaining discrete field energy.

The terminal removal is deliberately reported as a signed quantity. Clearing a strip changes not only its local kinetic state but also the discrete gradient cross-energy at the interface. If the signed change is negative, the proposed hard cleanup has increased the solver's discrete energy and cannot be interpreted as exported energy. Even if positive, it must be tested for reflection and shown to represent the complete energy change exactly once.

This first diagnostic uses a right-going packet and one probe. It is a mechanism check, not boundary validation. It does not establish an acceptance tolerance, cover other edges/corners/angles/frequencies, or validate the live ambient-composed energy balance. The broader incidence matrix remains required.

## Live-field inventory failure investigation

The exact-head CI failure is independent of boundary crossing: the failing integration test advances a 64-cell test window with domain expansion disabled and a standing-wave initial condition.

A per-tick diagnostic has been added to compare:
- Change in composed spatial-field energy.
- Incremental ambient world work currently calculated by the live adapter.
- Change in disturbance-only energy.
- The resulting per-tick and cumulative accounting residual.

This will distinguish an ambient-work integration mismatch from disturbance-energy drift. No accounting formula has been changed pending those measurements. The observed final residual on the preceding run was 183.19864464313255; this is not attributed to the boundary or counted as exported energy.
