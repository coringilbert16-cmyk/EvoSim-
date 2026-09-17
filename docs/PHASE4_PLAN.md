# Phase 4 — Movement and Physical Interaction

## Status

Phase 4 is in **P4.4 complete / P4.5 next**. P4.0 established the movement authority audit, P4.1 established the canonical movement boundary, P4.2 established the approved collision/contact policy and implementation, and P4.3 established atomic pushing/displacement interactions.

## Phase 4 objective

Implement and harden organism movement as a physical interaction with the environment and other organisms, using the already-established physical structure as the authority for what moves and what occupies space.

Movement must remain an emergent physical behavior rather than a predefined locomotion category.

## Frozen authority entering Phase 4

- The organism's realized physical graph is authoritative for what physically exists.
- Material composition and intrinsic physical realization are preserved across ownership/context changes.
- Geometry, collision, contact, and overlap calculations are derived from canonical physical state.
- Organism membership follows physical connectivity to the genome.
- ACQUIRE, Storage, COMBINE, BREAK, construction, and decomposition do not gain new rules from movement work.
- The blueprint remains structural intent; it is not a movement controller.
- Water remains fluid with deformable realization and is not converted into a rigid-circle movement object.

## Phase 4 scope

### 1. Movement as a physical transform

Movement changes the organism's world placement/context without rewriting intrinsic material realization.

The movement implementation must distinguish:

- intrinsic material geometry,
- organism-local physical structure,
- world position/orientation,
- and environmental occupancy/contact.

A world translation or rotation must not mutate the intrinsic identity of stored or realized material.

### 2. Physical collision and contact

Movement must use the existing derived geometry/contact system to determine whether a proposed displacement is physically valid.

The implementation must account for:

- organism boundary/occupancy,
- environmental material,
- other organisms,
- and actual geometry/contact constraints already established by the repository.

Do not introduce hard-coded obstacle categories.

### 3. Pushing / physical interaction

Where the existing design permits organisms or material to push one another, movement should resolve that interaction through physical occupancy and contact rather than through a biological role system.

No new predator/prey, obstacle, terrain, or locomotion classes are authorized by this phase.

### 4. Movement memory and directional resolution

Existing genome traits for movement memory and directional resolution may participate in movement behavior only according to already-established rules.

Phase 4 must not invent new memory semantics, learning rules, sensory systems, or mutation rules merely to make movement work.

If an existing trait lacks a defined behavioral rule, record the ambiguity rather than silently choosing one.

### 5. Perception boundary

Perception is adjacent infrastructure but is not automatically expanded into a new sensory system during Phase 4.

The existing bounded perception radius and directional/sensory resolution should be audited for movement consumption. Any missing behavioral definition must be surfaced for approval.

## P4.0 movement authority audit

### Audit result

The current repository has a single named movement execution entry point: `Simulation::update_movement` in `src/movement.rs`. The simulation decision loop calls that function when `ActionKind::Move` is selected. There is no second movement executor identified by the current code search.

### Current movement behavior observed

1. Movement reads `memory_strength`, `movement_efficiency`, the organism's memory points, and the existing `resource_sense` direction.
2. The first `occupied_cells` position is used as the movement anchor.
3. A normalized direction is calculated from memory and resource-sense inputs.
4. The current implementation uses a fixed `STEP_DISTANCE` of `5.0`, multiplied by the genome's `movement_efficiency`.
5. The first occupied cell is directly translated and clamped to environment bounds.
6. The same translation delta is then applied directly to every `StructuralUnit.placement` in the physical graph.
7. The current movement function does not perform physical collision/contact validation before committing the displacement.
8. The current movement function does not perform pushing.
9. The current movement function does not rotate the organism.
10. The current movement function does not consume movement energy directly.
11. Movement failure is currently represented by a boolean and can occur for an active transformation, zero direction, or zero resulting displacement.

The implementation therefore has a movement transform, but it does **not yet satisfy the Phase 4 authority boundary** for physical collision/contact and pushing.

### Coordinate/authority findings

- `Organism.occupied_cells[0]` currently acts as the locomotion/world-position anchor.
- `StructuralUnit.placement` is currently stored in world-space coordinates and is translated alongside the anchor.
- The physical graph itself is therefore already involved in movement, but the movement API is not yet expressed as a canonical physical transform over the graph.
- Intrinsic `PhysicalMaterial` realization is distinct from world placement in the Phase 3 storage/transfer model; movement must preserve that distinction.
- `ConnectionEndpoint::world_point` derives world-space endpoint positions from unit placement and intrinsic endpoint data, which provides an existing geometry basis for later collision/contact work.
- `contact.rs` already provides connection/contact candidate calculations from structural units and derived geometry, including fluid endpoints. It does not currently constitute a movement solver.

### Caller audit

The current simulation decision path selects `ActionKind::Move` and invokes `Simulation::update_movement`. Decision selection remains outside the movement mechanics, as required. Movement-related eligibility is also checked before selection through the existing `ActionEligibility` path.

### Memory/perception audit

The movement function currently combines existing memory-derived direction with `resource_sense.direction_x/y`. Memory maintenance and perception are separate systems. The movement implementation therefore consumes existing directional state rather than implementing its own sensory search.

However, the exact behavioral semantics of `directional_resolution`, `sensory_resolution`, and the bounded `perception_radius` as movement modifiers are not established by P4.0. They must not be invented during P4.4.

### Existing physical interaction infrastructure

The repository already contains derived contact/connection geometry in `src/contact.rs`, including endpoint generation, world-point calculation, facing compatibility, distance checks, and contact candidate filtering. `src/structure.rs` owns physical constituent placement and bond data. These are the systems P4.1/P4.2 should build on rather than introducing a parallel occupancy representation.

## P4.1 — Canonical movement boundary — COMPLETE

P4.1 established one canonical movement commit boundary in `src/movement.rs`: `Simulation::try_move_cell`.

The boundary now:

1. accepts an already-computed world displacement,
2. rejects non-finite displacement without mutation,
3. uses the first occupied position as the locomotion anchor,
4. clamps the resulting anchor to existing environment bounds,
5. computes the actual applied world-space delta after clamping,
6. applies that exact delta to every realized `StructuralUnit.placement`, and
7. returns `false` when no displacement was actually committed.

`Simulation::update_movement` remains the movement-intent layer and now delegates the actual position/structure mutation to `try_move_cell`. No collision, pushing, energy, speed, turning, friction, or new sensory rule was added in P4.1.

Regression coverage was added for:

- anchor and physical-structure translation together,
- boundary clamping with no partial structure mutation, and
- rejection of non-finite displacement without mutation.

**Validation status:** code and tests are committed, but repository CI/test execution has not yet been independently verified in this environment. Do not treat P4.1 as test-passing until CI or an equivalent full local test run provides evidence.

## P4.2 — Collision/contact resolution — COMPLETE

P4.2 established the movement collision policy and implemented it at the canonical try_move_cell boundary.

Approved rules:

- Collision is actual geometric penetration/overlap; touching is allowed.
- Organism-organism penetration blocks movement at P4.2; pushing is deferred to P4.3.
- Realized environmental physical material is physical and blocking.
- Logical structured aggregate material occupies its field cell and is blocking; ordinary unstructured aggregate material is not a geometric obstacle.
- Any collision in the proposed displacement rejects the entire movement.
- No partial movement, sliding, restitution, friction, collision priority, or new force model was introduced.

The implementation uses derived material_geometry::placed_forms_penetrate geometry over the organism's realized structural units and realized environmental physical material. The canonical movement boundary remains the only movement commit point.

Regression coverage includes:

- geometric penetration blocking,
- touching without blocking,
- boundary/non-finite rejection,
- structured aggregate material blocking,
- and movement without partial mutation.

Validation: GitHub Actions run 2377 verified formatting, source-size checks, COMBINE architecture checks, and the full Rust test suite. Clippy remains failing on pre-existing repository-wide lint/dead-code findings outside this phase; it is not used as the P4.2/P4.3 acceptance gate.

## P4.3 — Pushing / displacement interactions — COMPLETE

P4.3 extends the canonical movement boundary so a proposed organism displacement can propagate through physically penetrated blockers.

Approved rules:

- Organisms and realized environmental physical material can be pushed.
- Every affected object receives the same contextual world displacement.
- Push chains propagate through further penetrated organisms or realized physical material.
- All affected objects are resolved as one atomic interaction chain.
- If any affected object cannot translate or would enter a blocking structured aggregate field cell, the entire movement fails and no object is committed.
- Touching alone does not propagate a push.
- No mass/size threshold, force calculation, friction, momentum, deformation, movement-energy cost, or new pushing trait is introduced.
- Intrinsic physical realization is preserved; pushing changes only contextual world placement.
- Realized environmental material is reindexed by its world position after a successful movement so field-cell ownership remains consistent with its physical placement.

Regression coverage includes:

- single-organism pushing,
- multi-organism push-chain propagation,
- touching without pushing,
- atomic failure against structured aggregate material,
- realized physical-material pushing,
- and physical-material field reindexing.

Validation: GitHub Actions run 2377 verified formatting, source-size checks, COMBINE architecture checks, and the full Rust test suite successfully on the final P4.3 head. Clippy still fails on repository-wide pre-existing findings; the reported findings include unrelated architecture/environment dead-code and numeric-literal lint debt.

## Explicit non-goals

Phase 4 does not redesign:

- the developmental-field blueprint,
- genome definition,
- adulthood/maturation,
- reproduction,
- BREAK energy semantics,
- COMBINE chemistry/energy semantics,
- resource catalog or potential-energy scale,
- environmental reservoir/active-field rules,
- decomposition/recycling rules,
- or the organism lifecycle.

The newly approved developmental-field blueprint is a separate future migration and must not be pulled into Phase 4 unless explicitly approved.

## Phase 4 implementation sequence

### P4.0 — Audit before code — COMPLETE

Audit every current movement entry point and every caller of movement, collision, contact, pushing, translation, and rotation.

### P4.1 — Canonical movement boundary — COMPLETE

Establish one movement boundary that accepts a proposed physical displacement/transform and operates on the canonical organism structure.

It must preserve intrinsic material realization and update only contextual/world placement.

### P4.2 — Collision/contact resolution — COMPLETE

Implement the approved penetration/contact policy at the canonical movement boundary using derived physical geometry.

### P4.3 — Pushing / displacement interactions — COMPLETE

Resolve organism and realized-material push chains atomically through the canonical movement boundary while preserving intrinsic realization.

### P4.4 — Movement memory/directional integration — COMPLETE

Movement now has an explicit directional-resolution boundary: the existing memory-derived direction and existing resource-sense direction are combined exactly as previously implemented, then normalized before movement efficiency determines displacement magnitude. No new sensory, learning, turning, force, or mutation semantics were introduced. The direction calculation is covered by regression tests for combined memory/resource-sense input and the no-input rejection path.

`perception_radius` and `sensory_resolution` remain perception-layer controls, while `directional_resolution` remains applied by the existing perception system when producing `resource_sense`; movement consumes that already-produced directional state rather than reimplementing perception.

### P4.5 — Contract tests and audit

Add tests covering:

- intrinsic realization unchanged by world movement,
- translation/rotation consistency,
- collision against physical environmental material,
- organism-organism contact,
- pushing where already specified,
- no duplicate/parallel physical authority,
- and movement failure without partial mutation.

Run formatting, architecture checks, and the full test suite. Report any unavailable CI separately.

## Completion criteria

Phase 4 is complete only when:

- [ ] All movement entry points and consumers are audited.
- [ ] One canonical movement boundary is established.
- [ ] Intrinsic physical material realization survives movement unchanged.
- [ ] World placement is treated as contextual state rather than material identity.
- [ ] Collision/contact uses derived geometry from canonical physical state.
- [ ] Existing pushing behavior, where specified, uses physical interaction.
- [ ] No new biological role or obstacle category is introduced.
- [x] Movement-related genome/internal-state inputs use only already-approved semantics.
- [ ] Contract tests cover the movement authority boundary.
- [ ] Full test suite passes.
- [ ] Formatting and architecture checks pass.
- [ ] Remaining Clippy debt is reported separately rather than suppressed.

## Phase 4 stop conditions

Stop and request a design decision if implementation requires choosing any currently unspecified:

- movement force/acceleration model,
- speed formula,
- turning formula,
- friction/drag rule,
- pushing strength rule,
- collision priority rule,
- movement energy cost,
- sensory decision rule,
- memory update rule,
- or other biological/physical threshold.

The implementation must not invent these rules simply to make the code compile or produce motion.

## Relationship to the blueprint migration

The developmental-field blueprint approved for the project is intentionally outside this phase.

The long-term architecture remains:

> Genome → Developmental Field Blueprint → Construction/Development Solver → Physical Structure

Movement consumes the realized physical structure produced by that pipeline. It must not make the current explicit `StructuralBlueprint` representation more authoritative or expand it into a new body-plan system.
