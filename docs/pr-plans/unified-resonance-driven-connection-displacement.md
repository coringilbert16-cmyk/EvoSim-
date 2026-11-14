# Unified resonance-driven connection, reaction, and displacement plan

**Status:** Approved design foundation; implementation remains in PR #356.  
**Branch:** `chemistry/bond-competition-physical-connection`  
**Purpose:** Define one coherent physical-chemical process for approach, resonance attraction, activation, exact snapping, and bond displacement. This document is the implementation contract for the work that follows.

## 1. Governing principle

Resonance attraction, physical approach, and reaction activation are directly coupled parts of one continuous process. They must not be implemented as independent mechanisms that are joined after ordinary contact.

As two compatible materials progress toward a valid snapped connection:

- The geometry-specific resonance attraction strengthens toward the exact connection configuration.
- The physical interaction drives their actual movement and orientation toward that configuration.
- Reaction activation progresses in coordination with the same geometric approach and local resonance interaction; it advances more strongly as a valid snapped configuration is approached.
- Bond formation is permitted only when the exact snapped geometry is physically realized and the chemistry-defined activation and energy requirements are satisfied together.

The proximity measure is not center-to-center distance alone. It must represent progress toward a particular valid connection: face compatibility, separation, orientation, endpoint alignment, occupancy, and physical reachability all matter. A pair that cannot form a valid connection must not receive artificial attraction or activation merely because its centers are close.

## 2. Existing foundations to reuse

Do not create parallel systems or duplicate fields.

- **Connection geometry:** PR #355's explicit connection faces, stable face indices, endpoints, outward normals, and world transforms are the authoritative chemical interface geometry. These remain distinct from ordinary collision boundaries.
- **Resonance:** Reuse the existing frequency-resolved, phase-aware aura reception, symmetric pair potential, and potential-derived force. Do not introduce a second attraction field or scalar resonance-density energy source.
- **Energy:** Reuse the signed pair-interaction-potential settlement and existing energy ledger. Pair potential remains distinct from retained resonant energy, structural bond energy, activation, and dissipated heat.
- **Chemistry:** Reuse the existing bond-strength comparison, reaction activation model, bond potential, and energy funding rules. Bond strength determines which bond wins; energy availability determines whether the proposed transaction can be funded, not which bond is stronger.
- **Physics:** Reuse normal physical movement, collision, boundary, and push behavior. No chemistry-specific mass rule or special displacement shortcut is allowed.

The existing resonance force is staged and not yet driving ordinary material movement. Its isolated-pair implementation currently fails closed when third-party occluders are present. It must not be applied in crowded environments until a correct multi-body gradient accounts for forces on occluding bodies as well as the interacting pair.

## 2.1 Physical movement regimes — approved

The movement model has two distinct regimes:

- **Environmental flow is overdamped.** The environmental flow subsystem remains a non-inertial velocity field; it does not acquire persistent rigid-body momentum.
- **Elemental and molecular bodies are Newtonian.** Their resonance-derived forces act through persistent center-of-mass linear velocity and angular velocity. Translation follows mass-based acceleration; rotation follows torque and moment of inertia. Their kinetic energy must be represented explicitly and settled through the energy ledger.

These regimes must not be collapsed into one force-to-motion conversion. In particular, the ambient-flow mobility and `FLOW_RESPONSE_SCALE` are not Newtonian force or acceleration coefficients. The current `environment_flow::transport_field_with_singleton_center_velocity_reuse` path directly displaces realized material bodies using the overdamped field velocity; that legacy advection cannot remain the movement executor for elemental and molecular bodies once Newtonian integration is enabled. Before activation, separate environmental-flow evolution from material-body movement. The environmental field remains overdamped, while elemental and molecular trajectories come from their Newtonian state. Environmental influence on a body is permitted only through an explicitly modeled physical interaction, with its force and energy transfer accounted for; do not apply direct kinematic advection, background drag, or velocity relaxation to the body.

Use the existing canonical movement/transport commit boundaries. Do not introduce a second movement executor. Stage velocity, angular velocity, pose, collision/push consequences, pair-potential change, kinetic-energy change, and all affected energy holders together so failed movement leaves the complete state unchanged.

Elemental and molecular bodies experience **no background linear or angular drag** from the environmental flow. Their inertial motion persists unless a specific physical interaction transfers or dissipates mechanical energy. Environmental flow may influence their motion only through an explicitly modeled physical interaction; it must not apply a velocity-proportional damping force, erase momentum, or silently relax their velocity toward the flow field. Any dissipative interaction (for example, an explicitly modeled inelastic collision or other physically justified coupling) must identify the mechanical-energy loss and transfer it to the appropriate energy/heat inventory exactly once. Do not add generic numerical damping to stabilize the integrator. In the absence of such an interaction, Newtonian momentum and kinetic energy must be conserved subject to the modeled forces and their potential-energy accounting.

## 3. Exact snapped connection geometry

Snapping is the physically reached destination of continuous movement, never a teleport, post-reaction correction, or override of collision geometry.

A valid snapped configuration must satisfy all of the following:

- Compatible chemical connection faces with opposing outward normals.
- Exact face alignment and endpoint correspondence under the established geometry rules.
- No penetration, overlap, invalid boundary crossing, or impossible movement path.
- Available connection space, including partial face occupancy.
- Stable identity of the selected faces and occupied intervals throughout candidate evaluation and commit.

Established dimensional rules remain in force: a full 1.0 face can connect to a full 1.0 face with endpoints aligned; a 0.5 face may occupy either half of a 1.0 face without overlap; Hydrogen's 1.0-length longitudinal faces are eligible and its 0.1-length end faces are not. Do not invent new connection dimensions or relax geometry to make a reaction succeed.

Ordinary collision contact is not itself a chemical connection. No arbitrary boundary, vertex, edge, or intersection contact may bypass the exact snapped-geometry test.

## 4. Coupled approach, attraction, and activation

For each candidate pair of connection faces, evaluate a shared approach state from the realized geometry and local resonance interaction.

- The geometry component measures progress toward that candidate's exact snapped configuration, including translation, orientation, endpoint alignment, occupancy, and path validity.
- The resonance component uses the existing continuous, local, frequency-resolved and phase-aware aura calculation at the candidate interface. Occlusion must be respected.
- The physical force must be derived from the same authoritative interaction potential whose signed change is settled in the energy inventory.
- Attraction toward a valid snapped configuration strengthens as geometric progress increases. Repulsion, incompatible spectra, and other physical constraints remain effective; attraction must not overpower invalid geometry.
- Reaction activation is advanced using this same approach state. Better geometric alignment and stronger compatible local resonance bring the reaction closer to its existing chemistry-defined threshold. Center proximity alone must not advance activation.
- Keep the chemistry-defined activation threshold and bond-strength rules authoritative. Do not add a separate proximity threshold, arbitrary efficiency multiplier, or new scalar density threshold.
- The reaction event is eligible only when the exact snapped configuration is physically realized and the activation and energy requirements are satisfied. If activation is not yet sufficient at valid snapped contact, the materials may continue through the normal coupled interaction while maintaining valid geometry; no bond forms early. If activation is sufficient before exact geometry is reached, it cannot trigger a reaction until the exact geometry is reached.

The implementation must define and test the coupled progress/rate calculation explicitly. It must not simply accumulate activation at ordinary contact and snap afterward.

## 5. Displacement is the same continuous physical action

When an incoming bond is stronger than an incumbent bond occupying the required connection:

1. The incoming material continues its normal resonance-driven physical approach toward the exact snapped connection.
2. The incumbent weaker connection is displaced by that same physical movement. There is no separate BREAK phase followed by COMBINE, no switch-place behavior, no teleport, and no mass-based special case.
3. The displaced material is given an opportunity to form a different valid adjacent connection using the newly available geometry.
4. If a valid adjacent connection is physically reachable and chemically/energetically eligible, it may reconnect as part of the same staged transaction.
5. If no valid replacement connection exists, the weaker bond breaks and the displaced material remains as a physically displaced free material.
6. The stronger incoming bond wins by the existing `bond_strength` calculation. Energy funding cannot allow a weaker bond to win.

The same continuous movement, face occupancy, collision constraints, and exact snapped geometry govern both ordinary connection and displacement.

## 6. Atomic transaction and rollback

A candidate must be evaluated on staged state. The complete transaction includes:

- Physical trajectories and final placements of all affected materials.
- Collision and boundary validity along the movement path and at the final configuration.
- Connection-face selection, exact alignment, and occupancy changes.
- Resonance forces and the signed pair-potential change.
- Mechanical approach/displacement work and its authorized energy source.
- Reaction activation and chemistry-defined thresholds.
- Bond-strength competition, incumbent bond removal, and incoming bond formation.
- The displaced material's adjacent reconnection attempt or fallback bond break.
- Structural bond energy, retained resonance, usable energy, and heat settlement.
- Spatial index/cell membership and revision updates for every moved or changed material.

Nothing is committed until all physical, geometric, chemical, and energy checks pass. Any failure leaves positions, material ownership, bonds, activation, reservoirs, ledger values, and spatial bookkeeping unchanged. No partial movement, partial energy debit, or partially applied topology is permitted.

## 7. Implementation sequence

Keep all work on PR #356 unless a genuinely independent change is required and explicitly approved.

### Phase A — chemistry-only physical-body and force foundation

- Keep physical-body state generic, but make the active movement population only free environmental atoms and molecules.
- Derive resonance force and torque from the existing signed, occlusion-aware pair potential.
- Accumulate all contributions for each free environmental body before staging any pose.
- Establish an exact small-world force oracle for correctness; do not use it as the production tick evaluator.

**Current Phase A implementation status**
- `PhysicalMaterial` carries generic `InertialMotion` and exposes pure drag-free force/torque pose staging through `advanced_under_force`.
- Geometry-derived mass properties provide mass, center of mass, and planar moment of inertia. Force/torque advancement uses persistent linear/angular momentum.
- The multi-body resonance gradient includes source, receiver, and every relevant occluding body's force and torque contributions. Its geometry input is constructed only from free `PhysicalMaterial` values; it never calls the organism-structure geometry constructors.
- `LocalResonanceWorldSnapshot` contains only free environmental field materials. It excludes organism structures, organism-owned material, storage, construction, transformation escrow, and organism occluders.
- `ResonanceForceAccumulator` sums contributions by stable environmental identity and stages all proposed free-material poses atomically.
- The exact small-world oracle traverses each unordered environmental pair once. A three-body regression checks the exact pair count and near-zero total linear force and total torque.
- The read-only `EnvironmentalFlowField` can build obstacle bins and sample ambient velocity without changing material placements, inertial state, identities, or cell revisions.
- No chemistry-native face-approach controller is implemented yet. The previous approach prototype was removed because it depended on the organism bond graph.

### Phase B — environmental energy and movement-boundary foundation

- Keep the independent kinetic-energy inventory restricted to free environmental field materials. Kinetic energy is recomputed from their actual inertial state rather than added to the shared energy ledger.
- Settle signed environmental pair-potential and kinetic-energy changes through the environmental usable-energy holder, environmental resonance reservoir, and heat accounting. The settlement helper stages all holders and ledger counters before commit.
- Preserve rigid-body momentum when a free environmental molecule is split by BREAK through `break_environmental_internal_bond`. The legacy stored-material `break_internal_bond` path remains unchanged and is not used by chemistry movement.
- Keep the overdamped environmental flow field separate from the Newtonian body-momentum model. The read-only field sampler is now extracted, but the legacy transport consumer still advects materials; do not remove it until the Newtonian movement transaction is ready.
- The force/pose/energy helpers remain pre-runtime foundations. The live tick does not yet apply the accumulated resonance force, validate a continuous collision path, settle movement energy, or atomically commit new placements and field-cell revisions.
- The exact complete-pair evaluator is too expensive for the standard 1,280-material environment. A scalable mutual spectral evaluator and conservative visibility hierarchy, with explicit error bounds against the oracle, are required before live force consumption.

### Phase C — chemistry-native coupled activation and snapping
- Build approach geometry directly from free environmental atoms, molecular placements, and chemistry-owned bonds. Do not use `OrganismStructure`, `BondEndpoint`, owner-relative storage transforms, or geometry derived from organism structures. Shared physical-material geometry/visibility primitives are permitted only with free field materials.
- Calculate local spectral interaction and geometric progress for one valid molecular face pair.
- Couple resonance attraction, physical approach, and the existing chemistry activation barrier through that shared progress.
- Drive actual movement continuously toward exact snapped geometry.
- Trigger bond formation only when exact geometry, activation, chemistry, and energy conditions are all satisfied.
- Ensure no post-reaction snapping and no geometry override.

### Phase C — coupled activation and snapping
- Calculate local spectral interaction and geometric approach progress for the selected valid face pair.
- Couple attraction strength, physical approach, and activation advancement through that shared state.
- Drive actual movement continuously toward exact snapped geometry.
- Trigger bond formation only when exact geometry, activation, chemistry, and energy conditions are all satisfied.
- Ensure no post-reaction snapping and no geometry override.

### Phase D — unified physical displacement
- Extend the same approach solver to incumbent-occupied connection space.
- Resolve the weaker incumbent through physical push/displacement, not a separate chemistry operation.
- Search for a valid adjacent reconnection for the displaced material.
- Break the weaker bond only when no valid replacement connection can be made.
- Stage all affected movement and topology changes in the same atomic transaction.

### Phase E — integration and acceptance
- Integrate the shared mechanism with environmental materials, live COMBINE, BREAK, and displacement without maintaining parallel attraction/work authorities.
- Preserve existing chemistry and energy invariants except where this approved physical interaction intentionally changes approach outcomes.
- Run focused geometry, force, activation, displacement, rollback, and energy-conservation tests, then the complete CI suite.
- Do not run extended environmental experiments or benchmarks unless specifically requested; establish correctness and green CI first.

## 7.1 Runtime integration audit (chemistry-only boundary)

The current tick calls
`environment_flow::transport_field_with_singleton_center_velocity_reuse`
before environmental BREAK/COMBINE discovery. That function currently does
more than calculate the environmental flow: it directly advects and rotates
free field materials using overdamped mobility. This is a legacy material
movement path, not the approved Newtonian movement model. It must eventually
stop being the authority for elemental/molecular pose changes. The environmental
flow field itself remains overdamped and can continue to be calculated from
the surrounding geometry; it must not reset, damp, or replace a material's
persistent inertial state.

Do not simply add a second movement pass. The eventual chemistry movement
transaction must become the sole writer of free-material placements, with
field-cell membership and revisions committed atomically. A read-only `EnvironmentalFlowField` snapshot now builds obstacle bins and
samples ambient velocity without mutating material positions, cell revisions,
identities, or inertial state. The legacy transport pass consumes this field
but still performs overdamped material advection; that consumer remains to be
replaced once the Newtonian movement transaction is ready. Do not invent a
drag coefficient or an unapproved flow-force law.

The direct pair-potential gradient and the new complete unordered-pair force
sum are correctness oracles for small chemistry worlds, not production force
consumers. The exact oracle includes every unordered pair once and includes
each pair's force/torque contribution on every occluding free environmental
body. It must not be called unconditionally from the ordinary tick loop.
Production integration needs a scalable evaluator that preserves the exact
resonance law, visibility behavior, and complete reaction forces, with
controlled error against the oracle.

Energy settlement is environmental-only. The environmental settlement wrapper
stages the shared environmental usable-energy holder, the environmental
resonance reservoir, kinetic-energy inventory, and ledger together. It does
not debit organism energy or deposit into organism resonance storage. Current
kinetic-energy inventory counts only free physical materials in the
environmental field; stored material, reproductive construction, and
transformation escrow are excluded.

The previous connection-approach prototype depended on `OrganismStructure`
and persisted face bindings in the organism bond graph. That implementation
has been removed from this phase. Reintroduce approach/snap logic only as a
chemistry-native mechanism operating on free environmental atoms/molecules
and their own molecular bond geometry. Do not modify `OrganismStructure`,
`BondEndpoint`, organism storage, reproduction, or transformation code.

The next implementation sequence is:
1. Keep the active resonance population strictly to free environmental atoms
   and molecules; no organism structures, owners, storage, construction,
   transformation material, or organism occluders.
2. Separate overdamped environmental-flow-field calculation from the legacy
   overdamped material-advection behavior.
3. Build and validate a scalable mutual resonance evaluator against the exact
   complete-pair oracle, including visibility and per-body force/torque errors.
4. Stage Newtonian rigid-body movement, collision/path validation, and
   chemistry-native face approach without modifying organism code.
5. Settle before/after pair potential and kinetic energy through environmental
   holders only, then commit all moved placements and field-cell revisions once.
6. Integrate unified chemical displacement only after movement and energy
   transactions pass focused rollback and conservation tests.

Until these prerequisites pass, do not activate a live Newtonian movement
consumer, do not remove the legacy advection pass in isolation, and do not
connect any organism subsystem to the new physics.

## 8. Required regression and acceptance coverage

**Geometry and snapping**
- Full-to-full 1.0 face alignment with exact endpoint correspondence.
- 0.5-to-either-half of 1.0 face occupancy without overlap.
- Hydrogen longitudinal faces accepted; 0.1 end faces rejected.
- Wrong orientation, endpoint mismatch, occupied face, collision, wall, or blocked path rejects the candidate.
- A valid target is reached by continuous physical movement, not coordinate correction.

**Coupling**
- With otherwise equivalent valid candidates, increasing geometric progress strengthens the compatible attraction and brings activation closer to threshold.
- Frequency mismatch, phase cancellation, and occlusion reduce or alter the local interaction as the existing resonance model specifies.
- Center distance without a valid compatible face pair does not create attraction/activation eligibility.
- The same potential drives force and signed energy settlement; no duplicated force or energy term.
- No reaction occurs before exact snapped geometry, and the reaction event commits only when activation and funding are sufficient.

**Displacement**
- A stronger incoming bond physically displaces a weaker incumbent through the same movement path.
- No mass-based push rule or special switch-place case.
- The displaced material reconnects only through a valid adjacent face and physically reachable placement.
- If no replacement is valid, the weaker bond breaks and the material remains physically displaced.
- A weaker incoming bond never wins because it has more available funding.

**Atomicity and conservation**
- Invalid path, invalid final geometry, insufficient energy, invalid activation, or failed reconnection rolls back every staged change.
- No partial material movement, topology change, reservoir debit, ledger update, or spatial-index update.
- Independent energy inventory balances attraction, repulsion, approach, separation, COMBINE, BREAK, and displacement without double-counting pair potential, work, structural energy, retained resonance, or heat.

## 9. Explicit non-goals and prohibitions

- No separate BREAK-then-COMBINE displacement implementation.
- No teleporting or post-reaction snapping.
- No arbitrary contact-based bonding or geometry override.
- No chemistry-specific push rules or mass-based displacement special cases.
- No second resonance field, scalar heat-map energy source, or proximity-only activation path.
- No untraced energy source, negative-cost movement, double charging, or hidden energy sink.
- No organism-specific chemistry shortcut.
- No extended experiment-window changes, tick-rate standards changes, or unrelated optimization work as part of this implementation.


## 10. Resonance multipole / visibility research handoff

The mathematical research findings for a possible scalable resonance evaluator are recorded separately in [resonance-multipole-visibility-research.md](resonance-multipole-visibility-research.md). Read that note before continuing this line of research.

Key conclusions:
- The existing mechanical pair potential has a useful frequency-resolved bilinear structure after each material's aggregate emitted spectrum is prepared. Its spatial weighting still includes the exact receiver-boundary samples, projected extent, and two `1/(1+d)` attenuation factors.
- Reaction activation is a separate nonlinear path: per-emitter receiver response, phasor accumulation, and strongest-component selection cannot generally be replaced by one aggregated source response.
- Occlusion is geometry-dependent and changes at visibility boundaries. Occluders also receive force and torque contributions from the same potential.
- The credible candidate is mutual, error-controlled spectral multipoles plus a separate conservative visibility hierarchy, with exact fallback for ambiguous visibility and unbounded error.
- Potential, translational force, torque, linear momentum, angular momentum, and per-body accumulated error all require explicit treatment.

This is a research handoff only. It does not authorize changing the resonance law, adding an arbitrary interaction radius, or enabling an approximate evaluator in the runtime. The existing direct evaluator remains the correctness oracle.

## 7.2 Chemistry-only runtime audit (2026-11-13)

This audit supersedes the earlier owner-level movement plan. Organism movement,
organism owner mapping, organism structures, reproductive construction,
organism storage, and organism transformation are outside this phase and must
not be implemented or used as force participants/occluders.

### Verified chemistry foundations

- `PhysicalMaterial` carries generic persistent inertial state and has a pure
  force/torque-driven rigid-body proposal method. The active caller is limited
  to free environmental atoms and molecules.
- `LocalResonanceWorldSnapshot` is built only from free field materials.
  Those same materials are the only emitters, receivers, and occluders.
- `ResonanceForceAccumulator` sums per-body force/torque contributions and
  stages Newtonian pose proposals against stable environmental identities.
- The exact small-world force oracle traverses every unordered environmental
  pair once and includes the force/torque contributions on all other free
  environmental occluders. Its three-body regression checks pair count and
  total linear/angular force balance.
- `break_environmental_internal_bond` is a separate free-field split path
  that preserves rigid-body momentum across molecular products and rejects
  owner-relative stored material. The legacy stored-material split method is
  unchanged.
- The independent kinetic-energy inventory and environmental settlement path
  account for free environmental field materials only.

### Confirmed runtime gaps

1. **No live Newtonian movement yet.** The force accumulator and pose proposal
   are not committed to the environmental field. Collision/path validation,
   signed energy settlement, and field-cell revisions are not in one transaction.
2. **Legacy flow still advects materials.** The current flow pass calculates
   an overdamped velocity field but also directly translates/rotates field
   materials using mobility and response scaling. This is not the approved
   Newtonian material movement model. Separate flow-field evaluation from
   material advection before replacing the legacy mover; do not invent a drag
   law or let flow reset/damp inertial state.
3. **No scalable all-pairs force consumer.** The exact complete-pair oracle is
   too expensive for the standard 1,280-material environment. A mutual
   error-controlled evaluator and conservative visibility hierarchy are
   required before live force evaluation.
4. **No chemistry-native face-approach controller.** The earlier prototype
   depended on `OrganismStructure` and its bond graph; it has been removed.
   Exact face approach, swept collision checks, and activation progress must
   be rebuilt around free molecular material geometry and chemistry-owned bonds.
5. **Displacement is not yet the same physical movement.** The existing
   environmental displacement discovery/preflight/topology transaction still
   lacks physical incoming-body approach, incumbent push, adjacent
   reconnection, and signed pair/kinetic settlement in one atomic move.

### Implementation order

1. Prove a scalable mutual force evaluator against the exact complete-pair
   oracle, including visibility and per-body force/torque error.
2. Keep the read-only overdamped environmental-flow field sampler and
   replace its legacy material-advection consumer only when the Newtonian
   movement transaction can commit complete body proposals atomically. Do not
   use sampled flow velocity as a substitute for Newtonian body momentum.
3. Connect accumulated resonance forces to one Newtonian rigid-body proposal
   and atomic environmental-field commit, including collision/path validation,
   energy settlement, and field-cell revisions.
4. Implement chemistry-native face approach and synchronize geometric
   progress, resonance attraction, and existing activation thresholds.
5. Integrate unified displacement and adjacent reconnection/fallback through
   the same physical movement and atomic transaction.
6. Run focused chemistry-only conservation/rollback tests and full CI. No
   extended experiments or benchmarks unless separately requested.

The exact force oracle is test-only. Do not call it unconditionally from the
live tick loop. Do not add organism force participants, owner mapping,
organism occluders, or organism movement state.

## 7.3 Chemistry-only physical population boundary — 2026-11-13

The resonance movement snapshot is now deliberately assembled from **free physical materials in the environmental field only**. Its emitters, receivers, and occluders all come from that same field-material population.

Explicit exclusions from this snapshot:
- organism structures and organism-owned physical structures;
- reproductive/developmental construction material;
- stored material and transformation escrow;
- decomposing organism structures;
- any organism owner-index mapping or owner-level force aggregation.

This is a phase boundary, not a permanent restriction on the generic physics primitives. `PhysicalMaterial`, inertial integration, force/torque accumulation, and staged motion remain reusable for any future physical body. The chemistry movement caller currently supplies only free environmental atoms and molecules. No organism mechanics, state, geometry, movement, or lifecycle code is changed by this boundary.

The older complete-world pair-potential inventory/oracle remains a separate diagnostic/inventory path and is not the active chemistry movement population. Do not route chemistry movement or activation through that mixed population. Future work must preserve this separation rather than broadening the chemistry snapshot to include organism blockers.

Regression coverage now constructs a simulation that also contains its normal organism state, adds three free environmental materials, and verifies that the chemistry snapshot contains exactly those three materials. It checks that all three participate in the multi-body force result and stage together, with no organism-owned force mapping.

**Next gate:** validate the isolation change, then continue auditing the chemistry-only live-tick integration boundary. Do not wire organism movement, owner aggregation, or organism geometry into this phase.


## 7.4 Complete chemistry-world force accounting oracle — 2026-11-13

Added a small-world correctness path that traverses every unordered pair of free environmental materials exactly once. For each pair it evaluates the existing complete occlusion-aware multi-body gradient; every other free environmental body remains an occluder and receives that pair's force/torque contribution. Those contributions are summed by stable environmental identity before any body is advanced.

The three-body regression verifies the exact pair count (3), one accumulated result per body, and near-zero total linear force and total torque about the world origin. This specifically guards against omitting reaction forces on occluders, counting an unordered pair twice, or advancing a body before all of its pair contributions have been accumulated.

**Important limit:** this traversal is a correctness oracle for small test worlds, not a live-tick evaluator. It evaluates every pair and runs multi-body finite differences with all other bodies as occluders; that cost is far too high for the standard 1,280-material environment. Do not connect it to `Simulation::step`. The next required implementation is the research-backed scalable evaluator (mutual spectral multipoles plus conservative visibility handling and exact fallback), with measured error bounds against this oracle. No arbitrary interaction cutoff is allowed.

Organism mechanics and world assembly remain outside this force path. The only participants and occluders are free environmental atoms and molecules.


## 7.5 Organism subsystem isolation correction — 2026-11-13

The scope audit removed the earlier chemistry prototype's dependency on organism-owned structure and storage mechanics:

- Restored `src/geometry/structure.rs` to the base implementation. No persistent face-binding data is added to `OrganismStructure` or `BondEndpoint`.
- Removed the previous `connection_approach.rs` prototype because its occupancy model read and wrote the organism bond graph. A future approach solver must be chemistry-native.
- Restored `src/chemistry/transformation.rs` and the behavior of the legacy stored-material `break_internal_bond` path.
- Removed kinetic-energy aggregation for organism storage, reproductive construction, and transformation escrow. The current kinetic inventory is calculated from free environmental field materials only.
- Removed kinetic-energy state from the shared `EnergyLedger`; the independent inventory recomputes current environmental kinetic energy from actual free field bodies. Environmental pair/kinetic settlement accounts for the signed kinetic change without adding an organism-owned kinetic holder.
- Added a separate `PhysicalMaterial::break_environmental_internal_bond` path for free field molecules. It rejects owner-relative stored material and preserves rigid-body momentum across environmental BREAK products. The legacy stored-material split path remains unchanged.

The generic inertial primitives remain reusable, but the active movement population, force snapshot, kinetic inventory, and momentum-preserving split path are environmental chemistry only. The only residual edit in the material-storage module is a default initializer required because the shared physical-material data type now has a generic inertial-state field; no storage behavior or organism mechanics are changed.



## 7.6 Reuse prepared spectra in exact force evaluation — 2026-11-13

The tick-local `MaterialResonanceEmitter` cache is now used by the environmental pair-force path, not only by chemistry activation queries. For each pair and every finite-difference perturbation, the source and receiver reuse their already-prepared composition/tick mass scale and emitted spectrum. Their center and extent are still recomputed from the perturbed placements, so translation and rotation derivatives use current geometry rather than stale emitter coordinates. Receiver response to the incident spectrum and all exact boundary/visibility sampling remain unchanged; no emitter aggregation or activation Top-K ordering was altered.

The chemistry snapshot regression evaluates the same three-body pair both through prepared emitters and through the uncached reference path, then compares every body's force and torque to (10^{-10}). This is an exactness-preserving preparation step for the future multipole evaluator, not a multipole approximation. It does not change the force law or make the all-pairs oracle suitable for live ticks.

## 11. Live objective audit and minimal closeout plan — 2026-11-13

This audit compares the live branch against the original objective: free environmental atoms and molecules must move under the existing resonance interaction physics; physical approach, exact chemical face alignment, activation, and bond competition must advance together; displacement must be the same continuous physical event; and the complete movement/chemistry/energy/field update must commit atomically. Organism mechanics remain isolated and off limits.

### Verified current runtime gaps

1. `Simulation::step_with_transport_reuse` still calls `environment_flow::transport_field_with_singleton_center_velocity_reuse` before environmental reaction discovery. That consumer calculates overdamped flow displacement and rotation and commits changed field-cell membership/revisions. It remains the live material-movement authority.
2. `PhysicalMaterial::advanced_under_force`, `ResonanceForceAccumulator`, and `ResonanceForceAccumulator::stage_motion` provide staged Newtonian foundations, but the live tick does not call them. The force accumulator and motion staging are explicitly marked as awaiting runtime integration.
3. `LocalResonanceWorldSnapshot` is correctly built only from free environmental field materials. Its exact complete-pair force oracle evaluates every unordered pair and includes occluder reaction forces/torques, but is explicitly a small-world correctness oracle and is not called by the live tick.
4. `EnvironmentalFlowField` is a read-only ambient-flow sampler. The legacy transport consumer still uses its sampled velocity to advect material. That is not Newtonian movement and must not remain a second writer after the new mover is enabled.
5. Environmental COMBINE and displacement discovery still begin from near-contact endpoint candidates. There is no live chemistry-native approach controller that advances physical poses, exact face alignment, and activation together.
6. `environmental_displacement.rs` currently reconstructs free field materials into `OrganismStructure` via `restore_displacement_source`, then uses organism-structure geometry and bond graph helpers for candidate discovery and trial topology. This is a chemistry-path dependency on an organism-specific graph representation. Replace it with chemistry-native molecular topology/geometry before completing approach or displacement. Do not modify organism mechanics to accomplish this.
7. The current displacement resolver stages a topology/energy replacement, but it does not stage and commit the physical trajectories of the incoming, anchor, and displaced materials. It therefore does not yet implement physical displacement.

### Minimal closeout sequence

**1 — Make the chemistry movement population and data model self-contained.**
- Keep the active bodies strictly to free environmental atoms and molecules.
- Replace the `OrganismStructure` adapter in environmental displacement with chemistry-owned molecular bond/face geometry and trial topology.
- Reuse generic shape, collision, and geometry primitives only where they do not call or mutate organism systems.
- Keep the existing organism implementation untouched.

**2 — Establish the production resonance-force evaluator.**
- Preserve the existing resonance pair potential, frequency/phase behavior, visibility, and force/torque reaction on occluding environmental bodies.
- Validate any scalable mutual evaluator against the exact small-world oracle with explicit potential, force, and torque error controls.
- Do not use an arbitrary interaction radius, fixed cutoff, or unbounded approximation. Keep exact evaluation as the fallback where an approximation cannot be certified.
- Do not spend another standalone research phase on multipoles: derive and implement only what the production evaluator actually needs, with correctness tests against the oracle.

**3 — Replace the live material-advection authority with one Newtonian movement transaction.**
- Connect the production force evaluator to persistent inertial state and staged rigid-body poses.
- Validate swept collision paths, world boundaries, and final placements for every affected free material.
- Use ambient flow only according to an explicitly approved physical coupling law; do not substitute flow velocity for momentum or invent drag.
- Commit all moved placements, cell membership, identities, and field-cell revisions once. Do not run a second movement pass.

**4 — Join approach, COMBINE, BREAK, and displacement to that same physical authority.**
- Implement chemistry-native compatible-face approach. Geometric progress, resonance interaction, and existing chemistry activation must advance together.
- Form a bond only at exact valid face alignment and after the existing activation/energy rules pass.
- Preserve existing BREAK chemistry while ensuring split products retain physically correct momentum.
- For displacement, the stronger incoming bond must win by existing bond-strength rules. The incumbent must be moved by the same collision-validated physical motion; try valid adjacent reconnection, otherwise break the weaker bond. No teleport, switch-place shortcut, or separate BREAK-then-COMBINE sequence.

**5 — Make the complete event atomic and close with focused validation.**
- One staged transaction must include all affected poses/momenta, collision/path checks, face occupancy, activation, topology, signed pair-potential and kinetic-energy changes, energy-holder/heat settlement, and field-cell updates.
- Any failure must leave all source materials, positions, momentum, bonds, activation, reservoirs, ledger values, identities, and cell revisions unchanged.
- Add focused regressions for force/torque, visibility/occluder reactions, momentum, swept collision, exact face alignment, activation timing, COMBINE, BREAK, stronger/weaker displacement, adjacent reconnection/fallback, energy balance, and full rollback.
- Run the full CI suite after implementation. Do not run extended observation windows or benchmark campaigns unless explicitly requested.

### Completion definition

This PR is not complete when the force helpers compile or when a staged topology transaction passes in isolation. It is complete only when the ordinary live environmental tick uses one chemistry-only Newtonian movement authority; free materials visibly move from the existing physical interaction law; compatible approach and activation progress through real motion; COMBINE/BREAK/displacement resolve through the same atomic movement/chemistry/energy boundary; organism code remains unchanged; and full CI is green.

