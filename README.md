# EvoSim

EvoSim is an open-ended evolutionary organism simulation. The organism is built from physical material and physical bonds; the simulation should produce structure and behavior from general mechanisms rather than a fixed predator/prey or organism-size progression.

## Current construction state

The initial-organism constructor is currently in a **working-validation phase**.

Current implementation:

- starts from available catalog-backed Carbon genesis material;
- grows one physical unit at a time from currently realized neighbors;
- uses configuration-space/NFP placement for rigid polygon geometry;
- admits each permanent connection through the shared physical bond transaction;
- evaluates the realized graph after each local construction step;
- stops the genome phase immediately when the realized cavity qualifies;
- continues locally after the genome milestone until the acquisition contract is satisfied;
- requires Water plus three additional acquirable resources;
- checks acquisition by actual physical placement inside an accessible region;
- performs no recursive body-plan search and has no arbitrary placement-attempt budget.

The former fixed Carbon ring/spoke scaffold has been removed from the genesis path. The constructor is now structurally free-form at genesis: there is no prescribed ring, spiral, spoke, unit count, or final topology. Genesis material is still catalog-backed rather than environmental inventory, so resource-waiting behavior remains a separate integration step.

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
