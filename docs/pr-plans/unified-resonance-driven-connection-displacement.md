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
