# Phase 4 — Movement and Physical Interaction

## Status

Phase 4 is prepared for implementation. This document defines the implementation boundary; it does not authorize redesign of unrelated systems.

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

### P4.0 — Audit before code

Audit every current movement entry point and every caller of movement, collision, contact, pushing, translation, and rotation.

Identify:

- current movement authority,
- current world/local coordinate assumptions,
- geometry consumers,
- collision consumers,
- existing tests,
- legacy movement representations,
- and any undefined behavior.

Stop for approval if the repository does not already define a required behavior.

### P4.1 — Canonical movement boundary

Establish one movement boundary that accepts a proposed physical displacement/transform and operates on the canonical organism structure.

It must preserve intrinsic material realization and update only contextual/world placement.

### P4.2 — Collision/contact resolution

Migrate movement collision/contact checks to derived geometry over canonical physical state.

Do not create a second occupancy authority.

### P4.3 — Pushing / displacement interactions

Preserve or migrate the existing pushing behavior so it operates on physical contact rather than abstract organism roles.

### P4.4 — Movement memory/directional integration

Only after the physical movement boundary is correct, connect already-defined movement-related genome/internal-state inputs.

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
- [ ] Movement-related genome/internal-state inputs use only already-approved semantics.
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
