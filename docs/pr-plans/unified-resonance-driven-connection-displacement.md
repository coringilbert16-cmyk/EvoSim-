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

### Phase A — shared geometry and approach foundation
- Audit and expose reusable exact face-pair alignment, partial occupancy, and snapped-target calculations.
- Define a deterministic approach-progress representation for a specific face pair.
- Add path-validity and multi-body collision checks for the continuous movement toward the target.
- Keep ordinary collision geometry unchanged.

**Current Phase A implementation status**
- Exact full-face and half-face pairing, stable face identity, snapped target poses, and normalized face occupancy checks are implemented in `src/geometry/connection_approach.rs`.
- Conservative swept translation and rigid-rotation path checks are implemented. They use the shared material penetration predicate and fail closed when a narrow interval cannot be proven clear.
- Collision-limited transform fractions and a geometry-only approach step planner are implemented; they return a verified movement proposal and never mutate world state.
- A shared face-pair evaluation now combines endpoint/angular progress, activation-signal gating, occupancy validation, collision-valid requested movement, and exact-snap eligibility in one result.
- These helpers are still staged: the live environmental movement/reaction loop does not yet call them. Phase A is not complete until the runtime integration and its atomic rollback tests are implemented.
- Added an occlusion-aware multi-body resonance-gradient API that differentiates the same symmetric pair potential with respect to the source, receiver, and every occluding body. It returns forces only when the complete finite-difference gradient is finite and translationally balanced. This closes the previous two-body fail-closed limitation at the force-calculation layer; it is not yet connected to the runtime movement loop.

### Phase B — resonance force and energy integration
- Integrate the existing pair potential/force into normal environmental physical movement.
  
**Persistent inertial-state foundation (implemented; runtime integration pending)**
- Added serde-compatible `InertialMotion` state to each realized `PhysicalMaterial`, holding center-of-mass linear velocity and angular velocity. Older serialized materials default to rest.
- Added a drag-free Newtonian force/torque update primitive that accepts the caller's explicit physics-time interval, authoritative mass, and moment of inertia. No timestep, force scale, drag coefficient, or damping law is invented here.
- Added translational and rotational kinetic-energy calculation, point-velocity evaluation, and coordinate-frame conversion. Intrinsic-frame conversion accounts for both the shifted reference point and rotated axes.
- Added shared geometry-derived rigid-body mass properties: total constituent mass, center of mass, and planar moment of inertia, including periodic vertical geometry and realized fluid shapes. Environmental-flow rotation now reuses the same inertia calculation, without reusing its overdamped mobility or displacement scaling for Newtonian dynamics.
- Added explicit current and tick-zero kinetic-energy stores to the energy ledger and included kinetic-energy change in the independent world-inventory conservation residual. The inventory recomputes kinetic energy from field materials, organism-owned storage/construction material, and escrowed transformation sources without double-counting staged COMBINE outputs.
- Added atomic settlement of pair-potential and kinetic-energy changes: potential can convert directly into motion, kinetic loss can fund potential increase, unretained mechanical loss becomes heat, and insufficient funding rolls back the staged ledger and holders. The environmental wrapper stages its shared usable-energy and resonance pools together.
- Extended the occlusion-aware multi-body resonance gradient to include each body's torque about its own center of mass, derived by centered finite differences of the same potential under rigid rotation. This supplies rotational force input without a separate torque law; the translational and rotational contributions are tested together for angular-momentum balance. The independent full-world finite-difference oracle now reports the same per-body torque channel and has an asymmetric-occluder regression test.
- Clone-based material transport preserves the stored state. Bond splitting transfers the rigid body's angular velocity and computes each separated piece's center-of-mass velocity from its offset in the parent's rigid motion; a regression verifies total kinetic energy is conserved across a no-impulse split. Realized-state validation rejects non-finite inertial values.
- Momentum transfer through COMBINE, environmental displacement, and storage/release transitions is still pending. Those paths must preserve or explicitly settle momentum and kinetic energy before Newtonian movement is activated.
- Regression tests cover force/torque acceleration, unforced momentum persistence, kinetic-energy calculation, invalid dynamics inputs, frame conversion, and persistence through bond splitting.
- This remains pre-runtime groundwork: resonance force is not yet applied to live material movement, and the new kinetic settlement API is not yet called by a movement transaction. The legacy overdamped material-advection pass has not yet been separated from the environmental flow field.
- Resolve occluder-aware multi-body gradients, including reaction forces on blockers; do not use the isolated-pair force in a crowded scene.
- Ensure the force is the derivative of the same signed potential recorded in the world energy inventory.
- Settle positive and negative pair-potential changes, mechanical work, resonance storage, usable energy, and heat exactly once.
- Verify the independent world-potential oracle and performance before enabling tick-loop consumers.

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

## 7.1 Runtime integration audit (Phase B entry)

The live free-material movement path is `Simulation::step` ->
`environment_flow::transport_field_with_singleton_center_velocity_reuse` ->
environmental BREAK/COMBINE discovery. The flow pass already snapshots all
field materials, computes complete destination proposals, removes source
entries in reverse index order, deposits destinations, and updates affected
cell revisions. This is the correct environmental movement boundary to extend;
a second independent transport pass must not be introduced.

The current world-potential gradient in `world_tone_snapshot.rs` is a
correctness oracle, not a production force consumer. It perturbs every world
body and recomputes the full occlusion-aware pair inventory for each axis.
It must not be called unconditionally from the ordinary tick loop. Production
integration needs a cached/local interaction evaluation that preserves every
relevant occluder's contribution and returns balanced forces for all affected
bodies.

The existing `settle_pair_potential_change` helper stages its ledger and
resonance-holder changes, but its current caller contract accepts one active
mover's usable-energy holder and eligible biological resonance holders.
Environmental field materials have no independent usable-energy or retained
resonance holder. The implementation must therefore preserve the approved
ownership rule: do not debit an unrelated or distant organism, and do not
invent a nonliving material energy reserve. A proposed movement whose positive
pair-potential change has no legitimate source must be rejected before any
placement or cell-index mutation.

The first runtime slice will establish, in dependency order:
1. A production-scalable force snapshot with stable body identities and
   complete multi-body occlusion/reaction-force accounting.
2. A pure movement proposal stage that combines the existing ambient flow,
   resonance force, collision/path validation, and exact face-pair approach.
3. A staged before/after pair-potential and mechanical-work settlement that
   uses only legitimate local holders.
4. One commit boundary for all moved placements, topology/activation changes,
   energy holders, ledger counters, and field-cell revisions.

Until those prerequisites pass focused rollback and conservation tests, the
existing ambient flow remains unchanged and the full-world gradient remains
test/oracle-only.

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

## 7.2 Audited runtime gaps (2026-11-13)

The Phase B runtime audit confirmed that the existing foundations are not yet connected into the approved physical model. These findings supersede any earlier implication that the current ambient transport pass is an acceptable final movement executor for elemental and molecular bodies.

### Verified implemented foundations

- `PhysicalMaterial` persists `InertialMotion` with center-of-mass linear velocity and angular velocity.
- `InertialMotion::advanced` implements force/mass and torque/inertia acceleration without drag, and kinetic energy can be calculated from realized mass properties.
- `LocalResonanceWorldSnapshot` can calculate a pair's force/torque contributions for both participants and relevant occluders. `ResonanceForceAccumulator` stages complete multi-body accumulation atomically.
- `connection_approach.rs` validates face occupancy, approach progress, exact snapped geometry, activation gating, and swept translation/rotation paths.
- `environmental_displacement.rs` contains displacement discovery, activation/preflight, trial topology construction, and a staged topology/energy commit helper.

### Confirmed runtime gaps

1. **Newtonian state is not advanced in the live tick.** `InertialMotion::advanced` is explicitly marked as awaiting Newtonian runtime integration. The live simulation does not consume `ResonanceForceAccumulator` output or apply its force/torque results to environmental body trajectories.
2. **Ambient flow still directly moves physical materials.** `Simulation::step_with_transport_reuse` calls `transport_field_with_singleton_center_velocity_reuse`. That function computes `dx = velocity.x * mobility(mass) * FLOW_RESPONSE_SCALE`, applies the flow's `dy` and angular displacement, and commits translated/rotated material placements. This is overdamped tracer advection of the bodies, not Newtonian motion. It conflicts with the approved separation: the environmental flow remains an overdamped field; elemental and molecular bodies retain Newtonian momentum and are not kinematically advected, damped, or velocity-relaxed by that field.
3. **The face-approach solver is not the live movement controller.** The geometry helper is tested in isolation; the ordinary material transport path does not submit resonance-derived translation/torque to it, does not advance persistent inertial state, and does not couple the same approach progress to reaction activation.
4. **Displacement remains a chemistry transaction, not the same physical movement.** Environmental displacement discovery currently finds close/contacting candidates and accumulates chemistry activation from static pair potential, contact falloff, facing engagement, and optional local resonance similarity. Its trial/commit path changes topology and energy, but the live movement path does not physically carry the incoming material through the shared face-approach/collision solver while displacing the incumbent. Adjacent reconnection is therefore not yet part of one physical approach transaction.
5. **The force/energy/pose boundary is incomplete.** The force accumulator and staged energy helpers are foundations, but the live tick has no single transaction that stages force integration, inertial pose, collision/push effects, signed pair-potential change, kinetic-energy change, activation, topology, and field-cell revisions together.

### Implementation order from this audit

1. Establish a live Newtonian environmental-body movement stage at the existing field movement/commit boundary. It must preserve each body's inertial state through field-cell transfers and must not use `mobility` or `FLOW_RESPONSE_SCALE` as a force-to-motion rule.
2. Connect the existing multi-body resonance force/torque snapshot to that stage, evaluating each pair once and applying every participant/occluder contribution once. Keep the full-world gradient as an oracle, not a per-tick production path.
3. Route candidate connection approach through the shared exact-face and swept-path solver. Geometry progress, local resonance, and the existing chemistry activation barrier must advance together; no proximity-only activation or snapping correction.
4. Extend that same staged physical move to incumbent displacement, adjacent reconnection, and weaker-bond fallback, with one atomic commit/rollback boundary.
5. Only after the above is integrated, validate conservation, rollback, movement, alignment, activation, and displacement with focused tests and full CI. Do not extend experiment duration or introduce runtime multipole approximations during this phase.

### Staged multi-body Newtonian motion bridge (2026-11-13)

- Added `ResonanceForceAccumulator::stage_motion`, which validates every accumulated body index, stable environmental identity, and owner mapping against the immutable resonance snapshot before proposing any motion.
- The method stages all force/torque-driven rigid-body proposals as one all-or-nothing batch using `PhysicalMaterial::advanced_under_force`, and records before/after kinetic energy for each body. A single invalid mapping, mass-property failure, non-finite result, or invalid proposal rejects the entire batch.
- Added regression coverage using an identified environmental pair with an organism occluder, verifying that both free materials and the owner-mapped organism receive staged proposals and finite kinetic-energy inventories.
- This is still a pure staging bridge. It does not commit poses to the environment or organisms, does not validate collisions, and does not settle signed resonance-potential / kinetic-energy changes. It must not be described as live movement integration.

### Remaining blockers before tick integration

- A deterministic, non-arbitrary interaction traversal is still needed to enumerate the resonance interactions that exert force before face contact. The existing chemistry candidate lists are contact/discovery work queues and are not a valid substitute for the physical force interaction set.
- The world snapshot includes free field materials, organism-owned structures, and other owner-mapped bodies. A live commit must map every staged proposal back to its authoritative owner and stage all affected field-cell revisions, structure geometry, topology, and energy before any mutation.
- Collision/path validation and energy settlement must be joined to the same commit boundary. Until then, staged proposals remain uncommitted.


## 7.2 Staged-motion validation and owner-level motion audit (2026-11-13)

**Verified green CI after staged-motion regression fix**
- The multi-body staged-motion regression now passes with the complete Rust test suite and Clippy green.
- The failure exposed that organism structure proxy bodies were built with `internal_connections: None`. That state is intentionally unrealized; it is not suitable for mass-property or Newtonian motion calculations.
- `physical_material_from_structure` now uses `internal_connections: Some(Vec::new())`: the proxy is explicitly realized as one rigid physical body with no inferred internal bonds. This fixes the representation rather than weakening the mass-property checks.
- Keep the per-body mass-property and force-advance assertions in the regression. They make failures identify the exact body index, environmental identity, and owner mapping.

**New owner-level integration blocker**
- `world_physical_materials_for_pair_potential` places an organism's main structure, reproductive construction material, stored material, and transformation-held material into the world snapshot as separate physical bodies.
- Attached bodies carry `force_owner_indices = Some(organism_index)`; this is a mapping to their owner, not permission to advance them as independent rigid bodies.
- `ResonanceForceAccumulator::stage_motion` currently stages every accumulated body independently. If called directly for an owner-attached body, its proposal would move that body separately from the owning organism. That would violate the invariant that attached material follows the owner.
- `physical_material_from_structure` currently creates a proxy with default inertial motion. The organism does not yet have a demonstrated persistent owner-level inertial state that can be advanced and committed alongside its complete owned geometry.
- Before live runtime integration, introduce an explicit canonical owner-body mapping. Accumulate forces and torques from all owner-attached bodies onto that owner, including the torque shift from each body's center of mass to the owner's center of mass. Stage one owner-level rigid-body proposal, then derive all attached-body world poses from that same owner transform. Do not independently advance attached construction/storage/transformation bodies.
- Environmental field bodies with no owner remain independently movable. Decomposing bodies also need an explicit owner/motion policy rather than being accidentally treated as organisms or free field materials.
- Add regressions with one organism plus at least one owner-relative construction/storage body: force on the attached body must contribute to the owner's net force and torque; every owned placement must undergo the same rigid transform; no duplicate independent motion proposal may be emitted; any invalid member must reject the whole owner batch.

**Next implementation dependency**
1. Define and test the canonical owner rigid-body state and stable owner/body mapping.
2. Aggregate attached-body force/torque into the owner frame, preserving net force and torque about the owner's center of mass.
3. Stage one atomic pose/momentum update per independently movable entity (organism owner, free environmental material, or explicitly modeled decomposing body).
4. Only then combine those proposals with collision/path checks, energy settlement, and field-cell revisions at the live commit boundary.

The exact full-world occlusion-aware gradient remains a correctness oracle / pre-runtime source. This owner mapping work does not authorize calling it unconditionally from the tick loop or changing environmental flow's overdamped behavior.


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
