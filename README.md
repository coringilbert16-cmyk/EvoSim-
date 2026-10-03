# EvoSim

EvoSim is an open-ended evolutionary organism simulation. The organism is built from physical material and physical bonds; the simulation should produce structure and behavior from general mechanisms rather than a fixed predator/prey or organism-size progression.

## Current construction state

The initial-organism constructor is currently in a **working-validation phase**.

The current implementation:

- creates a deterministic physical Carbon scaffold;
- creates each physical unit through the normal material/structure machinery;
- forms every permanent connection through the normal physical bond transaction;
- uses realized geometry and the realized bond graph for cavity analysis;
- requires a qualifying genome cavity;
- requires Water plus three additional acquirable resources;
- checks acquisition by actual physical placement inside an accessible region;
- performs no recursive body-plan search and has no arbitrary placement-attempt budget;
- currently realizes genesis construction material from the catalog rather than waiting on environmental inventory.

The current scaffold is **not the final constructor architecture**. It is a temporary deterministic construction baseline used to prove that the physical construction, bonding, cavity, and acquisition contracts can work quickly. The newly separated genome-phase entry point is an integration seam for replacing that baseline with the final free-form constructor; it does not make the blueprint authoritative topology acceptable as the final design.

The present scaffold is a fixed 54-unit Carbon geometry consisting of an inner ring, six radial supports, and an outer ring. Adjacent rigid units are sealed with distinct endpoint bonds at the two ends of each shared wall segment. This must not be confused with the intended final free-form constructor.

## Intended constructor architecture

The final constructor should be forward-only and locally constructive:

1. start from the available physical starting material;
2. form a qualifying genome cavity;
3. once the cavity qualifies as the genome, finish that construction phase;
4. continue building the rest of the organism using locally valid physical bonds;
5. require Water and at least three additional resources that the organism can physically acquire;
6. if a required construction resource is temporarily unavailable during ordinary construction, wait without blocking the simulation;
7. never solve the entire future organism as a global placement problem;
8. never backtrack already committed physical bonds.

A blueprint, if used later, is only a preference. It must never become an authoritative future topology or placement command.

## Immediate engineering plan

The current priority is to make the initial constructor produce a valid organism in a timespan measured in milliseconds.

Work proceeds in this order:

1. **Prove the current physical scaffold.**
   - Identify the exact failing physical contract.
   - Fix only genuine implementation defects.
   - Keep construction deterministic and bounded.
2. **Make cavity formation a real construction milestone.**
   - The construction runtime now has an explicit genome-phase entry point.
   - It checks the realized physical graph immediately after committed construction/closure bonds.
   - When the cavity qualifies, that phase returns immediately instead of treating the cavity as a post-build assertion.
   - The current baseline still uses the temporary blueprint to reach that milestone; it does not yet provide the final free-form topology.
3. **Build the remainder locally.**
   - Replace the temporary fixed scaffold with the intended free-form constructive mechanism.
   - Select only from geometrically valid local continuations.
   - Commit bonds immediately; no global search or backtracking.
4. **Verify acquisition.**
   - Water plus any three additional resources must each be physically acquirable.
5. **Integrate waiting behavior where required.**
   - Resource shortage becomes a pending construction state rather than constructor failure or a simulation-thread block.
   - This is not yet part of the current catalog-backed genesis constructor.
6. **Only then broaden validation.**
   - Classify downstream failures by contract.
   - Migrate tests that still encode retired assumptions.
   - Run broader simulation/lifecycle validation after the construction layer is stable.

## Construction invariants

These are the constraints that matter to the current construction work:

- Physical geometry is authoritative.
- A permanent bond is created through the shared physical bond transaction.
- Physical contact does not by itself create a permanent bond; unbonded physical contact is a valid structural state. Permanent bonds are created only through the bond transaction.
- A qualifying genome cavity is a special case: its closed boundary must have a continuous bonded seal between the distinct material units that form that boundary. A merely coincidental geometric enclosure is not a genome.
- Intended bond contact is distinct from unintended penetration.
- Construction does not need a universal grid, 4N topology, or predefined cavity shape.
- The genome is defined by a qualifying realized cavity, not by a hard-coded core.
- Water is a physical material when instantiated; it is not logical material placed into storage merely to satisfy a test.
- Composite physical material retains its internal structure.
- Acquisition is based on physical contact/overlap, not exact coordinate identity.
- Constructor performance must come from direct construction and bounded local decisions, not from brute-force candidate enumeration.

## Verification discipline

For each change:

1. inspect the exact code path and contract being changed;
2. make the smallest isolated implementation change;
3. run formatting and compilation;
4. run focused constructor tests first;
5. inspect the actual failure before changing the next layer;
6. only broaden the test scope after the focused contract is proven.

Do not redesign the constructor in response to a downstream failure until the underlying construction contract has been verified.

## Project direction

The long-term goal remains an open-ended simulation in which organisms can develop structure, acquire resources, sense their environment, reproduce, form niches, and potentially evolve multicellular cooperation from the same general physical and behavioral mechanisms.

The current bottleneck is narrower: **constructing the first valid organism correctly and quickly.**

## Construction geometry investigation: configuration space / Minkowski methods

The constructor will investigate established computational-geometry methods before expanding the hand-built placement catalog further.

The leading approach is **configuration-space geometry using Minkowski sums/differences (commonly represented by a no-fit polygon)**. For two rigid 2D shapes, this can represent the set of relative translations that produce separation, overlap, or boundary contact. This is a geometry primitive, not a biological bonding rule.

The intended separation is:

1. computational geometry determines physically valid relative configurations;
2. EvoSim applies its own connection-scale rule (1:1, 1:0.5, and 0.5:0.25 allowed; 1:0.25 disallowed);
3. EvoSim converts selected contact configurations into explicit bond candidates through the normal bond transaction;
4. incidental physical contact remains valid without automatically becoming a bond;
5. cavity qualification continues to depend on the realized bonded seal, not geometric contact alone.

The investigation will first compare established implementations/algorithms against the actual EvoSim shape vocabulary, including the concave Phosphorus shape. Candidate implementations include Rust libraries exposing Minkowski sum/difference operations, while CGAL's 2D Minkowski-sum implementation is a reference for the underlying algorithms. Existing libraries demonstrate support for both convex and non-convex polygon Minkowski operations. No dependency is approved yet.

### Audit plan before integration

The geometry audit must establish:

- whether the current Form geometry can be converted losslessly into the polygon representation required by the chosen method;
- how relative **rotation** is represented, since a single no-fit polygon normally assumes fixed orientations;
- how many distinct orientations are actually necessary after exploiting symmetry of Carbon, Methane, Sulfur, Nitrogen, Hydrogen, and the asymmetric/concave Phosphorus shape;
- how vertex-vertex, vertex-edge, and edge-edge contact should map to EvoSim connection endpoints;
- whether the method can represent the complete useful contact locus rather than only the hand-selected endpoint matches currently cached;
- how non-convex Phosphorus behaves;
- how numerical tolerances affect touching versus penetration;
- whether generated geometry can be cached once and reused by the constructor;
- whether the resulting candidate generation materially reduces constructor runtime instead of merely moving the same search elsewhere;
- how the method interacts with multi-contact local growth and motif generation;
- and how Water remains separate as a deformable, volume-conserving material rather than being forced into the rigid polygon catalog.

The first implementation milestone is a **geometry-only proof of concept**, not a constructor rewrite. It must reproduce known valid Carbon↔Carbon, Carbon↔Nitrogen, Nitrogen↔Phosphorus, and Carbon↔Phosphorus contacts while rejecting genuine penetration. Only after those results are audited should the constructor consume the new configuration-space representation.

This investigation supersedes the assumption that the hand-written edge-pair catalog must remain the long-term geometry engine. The existing catalog remains useful as a baseline and regression oracle during the investigation.
