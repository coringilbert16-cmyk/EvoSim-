# Phase 3C — Closed Reflecting Boundary Investigation

**Status:** User-approved direction for investigation (2026-11-26); candidate boundary not yet implemented or validated.
**Active PR:** #446
**Supersedes:** The 2026-11-25 requirement to export waves permanently into the void. The current user decision is to investigate a finite, closed chemistry world with hard reflecting boundaries.

## Decision and motivation

The resonance field should first be a coherent, useful chemistry environment—not a general-purpose open-domain wave simulator. A closed finite world removes the need to model outgoing energy into an untracked void and may simplify the total-field energy contract. The field's later biological role is uncertain; that is not a reason to overbuild or remove it before chemistry tests its value.

Do not infer that a hard boundary is physically neutral. Reflection is an intentional environmental behavior. It can produce standing waves, repeated exposure, and resonant accumulation. These effects should be measured in chemistry later, but are acceptable for the closed-world reference model.

## Boundary condition to evaluate

The preferred candidate is a **reflecting, zero-normal-flux boundary** for the complete physical field.

Let the total field be \(\phi=a(t)+u(x,y,t)\), where the approved ambient carrier \(a(t)\) is spatially uniform and \(u\) is the disturbance. Since \(\nabla a=0\), a homogeneous Neumann condition on the disturbance,

\[
\partial_n u=0,
\]

also gives zero normal gradient of the total field and therefore zero composed normal flux for the current flux contract,

\[
q_n=-Kc^2(a_t+u_t)\,\partial_n u=0.
\]

This is a better candidate for a closed reflecting wall than simply deleting cells or assuming the current fixed-zero disturbance edge is equivalent. A fixed-value (Dirichlet) wall and a zero-flux (Neumann) wall are distinct boundary conditions and must not be conflated.

## Isolated validation plan

1. Preserve the validated interior propagation operator and its CFL/time-stepping contract.
2. Add a test-only fixed-domain Neumann boundary candidate; do not change production behavior in this investigation.
3. Compare it against the existing fixed-zero control using identical initial packets and standing-wave cases.
4. Test all four edges, both signs of oblique incidence, multiple resolved wavelengths, near-grazing packets, corner-directed packets, and phase-aligned/opposed superpositions.
5. Measure incident and reflected **energy** in an interior observation region. For an ideal lossless closed boundary, the reflected energy should approach incident energy; no damping or disappearing energy should be counted as boundary export.
6. Verify the discrete solver energy remains bounded/reconciled over repeated reflections and deterministic replay.
7. Verify the ambient-plus-disturbance composed boundary flux is zero and that the existing world ledger records no void export. Ambient schedule generation/withdrawal remains unchanged and is not duplicated.
8. Confirm waves reflect at the actual fixed environment dimensions, independent of camera zoom; remove computational-window expansion only after the fixed-domain implementation is validated.

## Important limitations and acceptance

- This document does not authorize material coupling, chemistry settlement, organisms, or a new energy source/reservoir.
- Do not change the governing interior wave equation merely to make the boundary test pass.
- Do not weaken energy tolerances to accommodate boundary drift.
- The current live implementation's expandable window and zero-edge constraints are not yet a validated closed-world production boundary.
- The current test-only Mur/Sommerfeld and sponge comparisons are exploratory; they are no longer a production requirement under this direction. Retain their results as historical diagnostics, but do not spend further work tuning open-boundary absorption unless a future use case reopens it.
- The boundary candidate must pass focused tests and exact-head full CI before production integration or merge.

## Scope decision

First deliverable is a coherent, fixed finite-domain resonance field with reflective outer edges and a reconciled internal energy ledger. Only after that should we decide whether the resonance field materially improves molecular chemistry and whether later stages need it.
