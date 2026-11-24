# Phase 3B.3 — Uniform Environmental Source Model Comparison

**Status:** Experiment definition in progress. Isolated mathematical work only; no production source registration, material coupling, chemistry, or production-ledger migration is authorized.

**Controlling roadmap:** [Resonance Field — Formal Implementation Roadmap](../design/resonance-field-implementation-roadmap.md)  
**Physical coupling checkpoint:** [Phase 3B.3 source-to-field coupling](PHASE3B3_PHYSICAL_SOURCE_COUPLING.md)

## 1. Question

For the user-approved spatially uniform environmental solar input, compare:

- **Model A — prescribed field forcing:** Candidate A's prescribed-acceleration field interface, with exact discrete signed-work settlement.
- **Model B — reciprocal source oscillator:** Candidate B's finite oscillator coupled reciprocally to the field, with its explicit coupling-potential energy and discrete total-energy invariant.

The prior PR #433 comparison is not this experiment. It replayed Candidate B's coupling-acceleration history through Candidate A and established field-response agreement under identical forcing. It did not compare the source models under a common environmental energy supply.

## 2. Shared environmental contract

Both models must use the same:

- Uniform environmental power-density schedule (q_{env}(t)), in E/(L²·tick).
- Finite source-accounting area (A_{world}), read from authoritative world configuration in any later integration; no hard-coded world dimensions.
- Total available environmental power (P_{env}(t)=A_{world}q_{env}(t)).
- Initial field state, field normalization, propagation speed, spatial domain used by the isolated numerical reference, time step, duration, and numerical resolution.
- Initial external energy inventory and exact tick/substep schedule.

The finite area is only the environmental energy-accounting footprint. It is not a reflecting, absorbing, or truncating wave boundary. The isolated solver's finite computational grid and boundary treatment must be reported separately as numerical truncation; it must not be described as the physical propagation boundary.

The solar schedule's curve, minimum, maximum, and calibration remain parameters to be selected from project evidence. Do not restore the superseded sun object, orbit, rays, 440-cycle carrier, or geometric interception model.

## 3. Critical fairness rule: power schedule is a budget, not field work

The environmental schedule specifies energy made available by the external environment. It is not equal by definition to signed field work (K u_t f).

For Model A, calculate the actual signed discrete field work using the validated Candidate A midpoint-work identity. Debit the environmental/source account for positive work and credit it for negative work. Reject any step that would overdraw the finite account; rejection must leave field and account unchanged.

For Model B, include the oscillator energy and coupling-potential energy in the discrete total-energy invariant. The first drive candidate is an explicit generalized force (F_{env,n}) applied to the source oscillator, not an after-the-fact energy increment. For the centered oscillator recurrence, the discrete external work over step (n) is

[
W_{env,n}=F_{env,n}\frac{x_{n+1}-x_{n-1}}{2},
]

where (x) is oscillator displacement. This follows from multiplying the forced centered-difference equation by the centered displacement increment; it must be verified against the actual coupled recurrence and its cross-time invariant before use. Positive work debits the external account; negative work credits it. The environmental schedule replenishes the available external account, while the deterministic drive waveform determines when/how the oscillator can accept work. Do not silently add energy to oscillator displacement, velocity, or energy after a step. If the derived discrete work identity does not close with the coupled field-plus-oscillator invariant, stop and repair the derivation before implementing the drive.

For Model A, retain Candidate A's independently prescribed field-acceleration waveform and exact discrete signed field-work settlement. Do not derive its forcing by dividing the requested positive power by instantaneous field velocity. The exact discrete work formula must be read from the validated implementation and carried into the comparison unchanged.

**Do not force either model to absorb exactly (P_{env}(t)) every tick.** Phase-dependent coupling can return energy or temporarily accept less than the available supply. Unused energy remains in the environmental account. A model that clamps negative work, divides by field velocity at nodes, or adds an unaccounted positive correction fails the experiment.

## 4. Spatially uniform source representation

The source profile must represent uniform environmental supply over the finite accounting footprint and be normalized using physical area, not cell count. For each resolution, independently verify:

[
int_{Omega_{world}}g_{env}(x,y),dA=1.
]

The profile's integrated supply must remain invariant under spatial refinement and grid translation. Energy that propagates outside the accounting footprint remains in the field inventory; the footprint does not absorb it.

The current isolated reciprocal model requires a zero-valued outer grid ring and normalizes profiles over its computational grid. Before using it for this comparison, establish whether that numerical boundary treatment can represent the required unbounded-plane reference to the needed accuracy. If not, extend the isolated solver with a validated unbounded-domain reference/absorbing numerical treatment; do not interpret a finite-box boundary as a physical wall.

### 4.1 Candidate B uniform reciprocal profile — first implementation slice

For a finite rectangular source-accounting footprint with area \(A_\Omega\), use the normalized profile

\[
g_\Omega(x,y)=
\begin{cases}
1/A_\Omega,&(x,y)\in\Omega,\\
0,&\text{otherwise},
\end{cases}
\qquad
\int_\Omega g_\Omega\,dA=1.
\]

On a uniform grid, the isolated implementation represents \(\Omega\) as an interior rectangle of cell indices and sets each covered cell to \(1/(N_\Omega\Delta A)\), where \(\Delta A=\Delta x^2\) and \(N_\Omega\) is the number of covered cells. The footprint must not include the outer numerical boundary ring. Translation and refinement tests must preserve the integrated profile and physical footprint area.

This profile makes the reciprocal field acceleration spatially uniform over \(\Omega\) for a given oscillator gap. The source coordinate remains one coherent global generalized coordinate coupled to the area-weighted field average; it is **not** a set of independent local emitters. That distinction is part of the model definition, not an implementation shortcut to hide.

For the environmental account, each tick makes available

\[
\Delta E_{\mathrm{env},n}=A_\Omega q_{\mathrm{env}}(t_n)\Delta t.
\]

This replenishes the finite account. It does not prescribe the oscillator force or require the oscillator to accept all available energy. The generalized-force waveform remains an independent constitutive input; its signed discrete work is settled against the account. Do not infer force by dividing environmental power by oscillator velocity.

The first profile helper and its unit tests are isolated mathematical groundwork only. It does not establish the environmental force law, select the schedule calibration, or resolve the numerical boundary.

### 4.2 Zero-boundary reflection diagnostic

The isolated Candidate B solver's fixed-zero outer ring is a reflecting Dirichlet boundary. A domain-expansion test now compares it against the independent Candidate A leapfrog reference solver, driven with the expanded Candidate B model's exact coupling-force history.

The diagnostic uses \(\Delta x=0.2\), \(\Delta t=0.01\), propagation speed 1, and the same 1.0-by-1.0 uniform source footprint in both domains:

- Bounded Candidate B: 41×41 nodes, footprint indices [18,23) in each axis. The nearest source-footprint edge is 3.6 distance units from the numerical boundary; a reflected signal can return to the source neighborhood during the 10-unit observation.
- Expanded Candidate B / independent Candidate A reference: 121×121 nodes, footprint indices [58,63). The nearest boundary is 11.6 distance units from the footprint, so no outgoing wave can reach it during the 10-unit observation.
- Compare a 9×9 region around the source at t=4 and t=10. The independent Candidate A solver must first reproduce the expanded Candidate B field under identical forcing.

Acceptance assertions: pre-reflection regional field difference at t=4 is at most 1e-10; by t=10 the bounded-domain difference must exceed 1e-4. This is a boundary-contamination diagnostic, not evidence that the bounded solver is acceptable through t=10. It establishes that the current zero-boundary model is only usable for an observation interval shorter than its measured reflection-return horizon.

### 4.2 Numerical-domain limitation remains open

The reciprocal solver currently fixes the outer grid ring to zero. That is a finite-box Dirichlet boundary and can reflect outgoing waves. A uniform source profile does not remove this artifact and does not make the solver an infinite-plane model.

Before any source-model comparison, validate a numerical treatment against an independent unbounded-plane reference. Candidate approaches may include a sufficiently validated absorbing layer or another transparent-boundary approximation, but no method is selected by this note. Tests must quantify reflected energy, phase/amplitude error, and convergence as the boundary is moved farther from the source footprint. The finite source-accounting footprint and the numerical observation/truncation domain must remain separately named and measured.

## 5. Experiment stages

### Stage 1 — derive and freeze the common input protocol

1. Define a deterministic test solar schedule as a dimensionless shape multiplied by a symbolic/calibrated power-density scale. Keep schedule shape and absolute calibration separate.
2. Define source-account initialization, replenishment timing, tick integration, and signed transfer conventions.
3. Derive the external-drive/replenishment port for Model B and its exact discrete work identity.
4. Define how Model A's forcing waveform is prescribed independently of the instantaneous field velocity. No feedback controller or per-tick inverse of (K u_t f).
5. Reject the comparison if the two models cannot receive the same available environmental energy without violating their respective physical laws.

### Stage 2 — isolated correctness tests

For each model:

- Zero environmental supply produces no externally sourced energy.
- Zero coupling leaves the field and source oscillator/account independent.
- Positive transfer debits the unique environmental account once.
- Negative transfer returns energy to the same account once, where the model physically permits it.
- Exhausted source budget prevents overdraw and rolls back the complete step.
- Invalid inputs and failed transactions leave every state unchanged.
- Whole-system energy reconciles to numerical tolerance at every tested step.
- Repeated replay is bitwise deterministic.
- Uniform profile integral is invariant under refinement and translation.
- No field energy is created, deleted, or duplicated when waves cross the finite source-accounting footprint.

### Stage 3 — controlled comparison

Use identical initial conditions and a shared parameter set. Compare at multiple spatial and temporal resolutions, and across at least these input profiles:

1. Constant environmental supply.
2. Smooth day-to-night rise and fall.
3. Zero-input interval after a nonzero-input interval.
4. Supply exhaustion.
5. A schedule whose timing spans different source/field phase relationships.

Record, separately for each model:

- Environmental energy made available.
- Environmental energy remaining.
- Cumulative external work delivered to the model.
- Field energy and field flux leaving the numerical observation region.
- Oscillator energy (Model B only).
- Coupling-potential energy (Model B only).
- Returned work, any explicit intermediate stored energy, and numerical residual.
- Field amplitude/phase response and spatial uniformity.
- Stability, convergence, and runtime only after physical equivalence and accounting are established.

Do not compare only final field energy. Distinguish energy supplied, energy transferred, energy still held by the source, energy stored in the field, and energy transported away.

## 6. Decision criteria

A model is eligible only if it:

1. Has a dimensionally consistent environmental input law.
2. Reconciles the environmental account, source state, coupling energy, and field energy without unexplained residual.
3. Passes atomic rollback, budget exhaustion, and deterministic replay.
4. Passes independent spatial and temporal convergence checks.
5. Preserves the infinite-plane propagation interpretation independently of the finite environmental accounting area.
6. Does not use phase clamping, arbitrary efficiency, or hidden energy creation.

If both pass, compare fidelity to the intended physical meaning, robustness across phase and schedule, numerical stability, and complexity. Do not select a winner based solely on speed. If only one passes, document why. If neither passes, derive a third mechanism rather than weakening conservation.

## 7. Explicit exclusions

This experiment does not implement a real sun, hotspot, powered material oscillator in live simulation, receiver absorption, heat/material settlement, chemistry, organisms, GUI changes, or production energy-ledger migration. It does not select numerical solar minimum/maximum values without calibration evidence.

## 8. Current blocker / next implementation gates

Candidate B now has an isolated normalized rectangular uniform-profile helper, with tests for uniformity, translation, refinement, and exclusion of the outer numerical boundary ring. This establishes the spatial weighting only. It does not establish a physical source law or an infinite-plane numerical solution.

The external generalized-force port and signed-work settlement have isolated transaction tests, including positive work, returned work, zero work, invalid input, and insufficient-budget rollback. The environmental replenishment method is accounting-only; the authoritative area and schedule integration remain to be wired into a controlled experiment harness.

**Remaining gates before the head-to-head sweep:** (1) validate the uniform profile against a physically defined accounting footprint and shared area budget; (2) resolve and independently validate the finite-grid boundary treatment against an unbounded-plane reference; (3) freeze the deterministic schedule, drive waveform, account replenishment order, and matched initial conditions; (4) verify the complete energy identity under those conditions. Do not begin the A/B sweep or production integration until all gates pass.
