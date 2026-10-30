# PR #225 — Raw Environmental Reactants: Natural COMBINE Discovery

> **Planning-only draft.** This phase proves one natural environmental COMBINE path from discrete raw reactants. It does not implement BREAK discovery or a combined COMBINE→BREAK cycle.

## Purpose

Prove that discrete raw environmental reactants can encounter one another, accumulate chemistry, and commit a physically realized compound through ordinary simulation ticks, without an organism, injected reaction operation, forced pair contact, or weakened activation/geometry rules.

This is the first raw-resource-to-molecule experiment. The product and its constituent/bond properties must emerge from the existing chemistry rules rather than a hard-coded molecule recipe.

## Integrated dependencies

- **#214** shared chemistry runtime/scheduler — merged.
- **#223** bounded environmental COMBINE discovery and settlement — merged.
- **#226** S-shaped flow, gradient-driven transport, eddies, and obstacle-aware movement — merged.
- **#229–#234** base-resource pair matrix, controlled physical-pair classification, product characterization, compound/base chemistry, compound/compound contacts, and committed multi-constituent COMBINE — merged.
- **#235–#240** molecular composition/topology, multi-constituent COMBINE/BREAK round-trip evidence, heterogeneous interfaces/reaction keys, fair rotating contact discovery, and geometry-derived layout inputs — integrated into main through #240.

PR #215 is the merged environment-first observability/workflow plan. Its implementation should proceed as a **parallel companion workstream** to this chemistry experiment, not wait until after it. The GUI should render the same live environment being exercised here, with real resource positions, movement/flow, chemistry events, and inventory state; it must not synthesize reactions. Keep the chemistry proof and GUI implementation in separate, dependency-ordered PRs so each remains independently testable.

## Source audit to refresh before implementation

The 2026-10-29 source trace found that `FieldCell` distinguishes aggregate unstructured `materials` stock from realized `physical_materials`. Environmental COMBINE discovery enumerated realized physical objects, not aggregate stock. The existing environment seeding helpers also used precombined recipes, so they did not prove a raw-resource-to-compound path.

Re-audit these paths on current `main` before changing code. If raw reactants must be represented as physical objects to participate in contact chemistry, create them in an isolated test fixture; do not silently rewrite the production default seed.

The #240 integration adds fair rotating component-pair discovery and heterogeneous constituent identity. Do not assume this automatically solves movement/activation continuity: inspect the current encounter key and progress lifecycle under live flow. Flow must change collision opportunities naturally and may interrupt contact; chemistry must not pin materials or pause transport.

## Approved experiment fixture

- **128 individual whole-unit resources total**, not 128 of each type.
- All seven canonical base-resource types must be represented in the 128-unit population. Randomize the individual resource identities; do not prescribe a reaction recipe or fixed type ratios.
- Scatter the resources across the map in randomly sized piles of **6–20 units per pile**. Choose pile sizes and placements through a reproducible seeded randomization; the pile sizes must sum to exactly 128.
- Use a randomized spatial distribution across the available map rather than a single central cluster or predetermined pairings.
- Run dynamic environmental flow strong and varied enough to transport material between pile regions and generate recurring inter-pile encounters/contacts. Flow should create those encounters through ordinary movement, not by scripting collisions, pinning reactants together, or directly forcing a chemistry operation. Contact may be interrupted by the flow.
- Keep the fixture isolated from production environment seeding and all organism-owned energy. Identify the environmental energy source and its exact ledger/inventory path before implementation.
- Preserve the random seed and generated initial state in test output so a successful or failed run can be reproduced exactly.

## Implementation steps

1. Add an isolated environment-only fixture containing exactly 128 discrete raw reactants, all seven base types represented, partitioned into randomized piles of 6–20, with an independent tick-zero integer material inventory.
2. Enable dynamic flow at a strength/gradient that produces meaningful transport and recurring inter-pile contact opportunities. Advance ordinary `Simulation::step` ticks; do not inject COMBINE operations or call a resolver directly.
3. Prove that live discovery, activation, scheduling, geometry validation, and atomic COMBINE settlement produce at least one realized compound through naturally flow-driven encounters. If the approved flow configuration does not generate contacts, report that as a fixture/transport blocker rather than forcing a collision.
4. Record the actual constituent multiset, topology, interfaces, stored bond energy, and descriptive aggregate properties. Do not assign a hard-coded molecule identity or averaged chemical position.
5. Reconcile exact integer material inventory and independently reconcile world energy before and after each committed reaction.
6. Test insufficient environmental energy, stale source/contact, and rejected settlement; failure must preserve source geometry, ownership, material counts, energy holders, and ledger.
7. Prove deterministic repeatability from the recorded seed and bounded per-tick discovery work without requiring every possible pair to react.
8. Compare exact-head focused results and named full-suite failures with the latest accepted main baseline.

## Scope boundaries

- **COMBINE discovery only.** BREAK discovery, activation, and commit are deferred to a separate later phase. Existing BREAK transaction/round-trip tests do not prove natural environmental BREAK discovery.
- No forced pair enumeration as a requirement that every pair react, no scripted pair contact, no lowered activation thresholds, and no test-only operation injection.
- No organism or organism energy as an environmental source.
- No fractional atoms, fractional material splitting, or reinterpretation of aggregate stock as a realized molecule.
- No flow equations or duplicate transport implementation; #226 owns transport.
- No GUI implementation in this chemistry PR. Build the environment visualization and event-inspection UI in a parallel companion PR against the merged #215 plan, sharing this experiment's environment-only mode and deterministic fixture. Long-run benchmarks remain separately scoped.
- No changes to the default production seed helpers.

## Acceptance gate

A repeatable no-organism live-tick test, with dynamic environmental flow active, demonstrates:

**128 randomized individual raw resources across randomized 6–20-unit piles → flow-driven transport and natural inter-pile contact → chemistry activation → scheduled COMBINE → committed realized compound**

All seven base-resource types are present in the initial population. The independent integer material inventory reconciles exactly; the independent world-energy inventory balances without creation; rejected operations are atomic; work is bounded and deterministic; and no reaction or collision is fabricated or forced.

A resolver-only test, a manually constructed reaction operation, a predetermined recipe, scripted pair contact, or a passing inventory snapshot without a committed physical product does not satisfy this gate.
