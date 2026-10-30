# PR #225 — Raw Environmental Reactants: Natural COMBINE Discovery

> **Planning-only draft.** This phase proves one natural environmental COMBINE path from discrete raw reactants. It does not implement BREAK discovery or a combined COMBINE→BREAK cycle.

## Purpose

Prove that discrete raw environmental reactants can encounter one another, accumulate chemistry, and commit a physically realized compound through ordinary simulation ticks, without an organism, injected reaction operation, forced contact, or weakened activation/geometry rules.

This is the first raw-resource-to-molecule experiment. The product and its constituent/bond properties must emerge from the existing chemistry rules rather than a hard-coded molecule recipe.

## Integrated dependencies

- **#214** shared chemistry runtime/scheduler — merged.
- **#223** bounded environmental COMBINE discovery and settlement — merged.
- **#226** S-shaped flow, gradient-driven transport, eddies, and obstacle-aware movement — merged.
- **#229–#234** base-resource pair matrix, controlled physical-pair classification, product characterization, compound/base chemistry, compound/compound contacts, and committed multi-constituent COMBINE — merged.
- **#235–#240** molecular composition/topology, multi-constituent COMBINE/BREAK round-trip evidence, heterogeneous interfaces/reaction keys, fair rotating contact discovery, and geometry-derived layout inputs — integrated into main through #240.

PR #215 contains the separate environment-first event-inspection and benchmark plan. This PR must use real simulation outcomes and bounded test instrumentation; it does not implement the UI or 24,000-tick benchmarks.

## Source audit to refresh before implementation

The 2026-10-29 source trace found that `FieldCell` distinguishes aggregate unstructured `materials` stock from realized `physical_materials`. Environmental COMBINE discovery enumerated realized physical objects, not aggregate stock. The existing environment seeding helpers also used precombined recipes, so they did not prove a raw-resource-to-compound path.

Re-audit these paths on current `main` before changing code. If raw reactants must be represented as physical objects to participate in contact chemistry, create them in an isolated test fixture; do not silently rewrite the production default seed.

The #240 integration adds fair rotating component-pair discovery and heterogeneous constituent identity. Do not assume this automatically solves movement/activation continuity: inspect the current encounter key and progress lifecycle under live flow. Flow must change collision opportunities naturally and may interrupt contact; chemistry must not pin materials or pause transport.

## Fixture approval gate

Before implementation, present and obtain approval for:

- the selected canonical base resources and exact whole-unit counts;
- a varied multi-pile arrangement rather than a predetermined reaction recipe;
- deterministic placement/randomization and the flow conditions;
- the expected candidate/contact geometry, without forcing a collision;
- the environmental energy source and its exact ledger/inventory path;
- isolation from production environment seeding and any organism-owned energy.

The prior proposal used approximately twelve discrete units of each of the seven base resources spread across varied piles. Treat that as a candidate fixture, not a locked invariant, until the owner approves the exact counts and energy source.

## Implementation steps after fixture approval

1. Add an isolated environment-only fixture containing the approved discrete raw reactants and an independent tick-zero integer material inventory.
2. Enable the approved flow field and advance ordinary `Simulation::step` ticks. Do not inject COMBINE operations or call a resolver directly.
3. Prove that live discovery, activation, scheduling, geometry validation, and atomic COMBINE settlement produce at least one realized compound under the natural collision process.
4. Record the actual constituent multiset, topology, interfaces, stored bond energy, and descriptive aggregate properties. Do not assign a hard-coded molecule identity or averaged chemical position.
5. Reconcile exact integer material inventory and independently reconcile world energy before and after each committed reaction.
6. Test insufficient environmental energy, stale source/contact, and rejected settlement; failure must preserve source geometry, ownership, material counts, energy holders, and ledger.
7. Prove deterministic repeatability and bounded per-tick discovery work without requiring every possible pair to react.
8. Compare exact-head focused results and named full-suite failures with the latest accepted main baseline.

## Scope boundaries

- **COMBINE discovery only.** BREAK discovery, activation, and commit are deferred to a separate later phase. Existing BREAK transaction/round-trip tests do not prove natural environmental BREAK discovery.
- No forced pair enumeration as a requirement that every pair react, no lowered activation thresholds, and no test-only operation injection.
- No organism or organism energy as an environmental source.
- No fractional atoms, fractional material splitting, or reinterpretation of aggregate stock as a realized molecule.
- No flow equations or duplicate transport implementation; #226 owns transport.
- No UI, event-inspection UI, or long-run benchmarks; #215 owns those.
- No changes to the default production seed helpers.

## Acceptance gate

A repeatable no-organism live-tick test, with ordinary flow active, demonstrates:

**approved discrete raw reactants → natural collision/contact → chemistry activation → scheduled COMBINE → committed realized compound**

The independent integer material inventory reconciles exactly; the independent world-energy inventory balances without creation; rejected operations are atomic; work is bounded and deterministic; and no reaction is fabricated or forced.

A resolver-only test, a manually constructed reaction operation, a predetermined recipe, or a passing inventory snapshot without a committed physical product does not satisfy this gate.
