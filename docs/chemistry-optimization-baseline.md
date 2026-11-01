# Chemistry optimization and Clippy baseline

This work starts after PR #257 merged into `chemistry-experiment-density-resonance-visuals` at `c9c30b46e7400ecffb2d6f44fc66160e56a56937`.

## Scope

- Separate reusable static chemical-potential memoization from geometry/interface identity.
- Instrument COMBINE discovery stages and chemistry-library access, including cache hits, misses, inserts, calculation time, geometry resolution time, and persistence time.
- Measure organism-owned BREAK selection/restoration where applicable.
- Remove measured redundant work without skipping live physical checks.
- Resolve the existing Clippy backlog with targeted code fixes, not broad lint suppressions.

## Confirmed starting state

- The chemistry library is opened and used by organism-owned chemical reaction evaluation and environmental COMBINE.
- Current `ChemistryKey` includes canonical material names, interface class, interface signature, and quantized chemical positions. This prevents geometry variants of the same chemical pair from sharing the static-potential entry.
- `interaction_potential()` is determined by the two chemical positions and fixed equation parameters. Contact distance, interface engagement, activation, formation, and energy settlement are separate live checks and must remain live.
- The current persistent chemistry JSONL file in the repository is empty; the manifest exists. Runtime-generated library contents must be measured in an actual run rather than inferred from the checked-in data file.

## Performance reference

The issue #258 controlled 180-second runs recorded:
- Baseline: 43,930 ticks (~244 ticks/sec), 276 COMBINE, 3 BREAK.
- Instrumented repeat: 46,534 ticks (~258.5 ticks/sec), 282 COMBINE, 3 BREAK.
- A later run showed lower average COMBINE discovery time (332 µs/tick versus 738 µs baseline) but did not improve total ticks/sec. The spike context also showed a worst tick with only one ready candidate and 163 accumulation entries.

These results are promising but noisy. Repeat same-seed runs on the exact PR head before attributing or claiming a whole-simulation speedup. Report both average and maximum timings.

## Research-informed optimization guidance

See [chemistry performance research](chemistry-performance-research.md) for comparable particle simulators and the technical references behind the following priorities:

- If repeated pair discovery is the measured bottleneck, evaluate spatial bins/stencils or another conservative neighbor structure. It may only reduce the candidate set; current live geometry, contact, interface, activation, and transaction checks must still decide eligibility.
- Choose a grid, stencil, tree, or direct scan from EvoSim's measured density, size/cutoff disparity, and object counts. LAMMPS and HOOMD-blue document that no one neighbor-search structure is best for every workload; small sparse systems may favor a direct scan.
- If allocations or copies dominate, measure temporary graph/material copying and data locality before considering packed arrays or other storage redesigns.
- Keep invariant chemical-potential memoization separate from geometry/interface identity. Do not cache live physical eligibility.
- Preserve deterministic/fair scheduling and conflict handling when reorganizing reaction stages. Defer parallelism until these boundaries are explicit and independently tested.
- Tune candidate-index parameters with repeatable same-seed runs and exact inventory/energy checks. Never accept a faster broad phase that can omit a physically eligible pair.

## Clippy baseline

The latest PR #257 Rust workflow completed all functional validation steps but failed Clippy:
- Application target: 38 errors.
- Test target: 25 errors.

The baseline includes unused/dead-code items in geometry/Bob/organism modules, type-complexity warnings, minor ownership/conversion/iteration issues, field-reassignment-after-default in tests, a useless Vec, and an unfulfilled lint expectation. Each finding should be fixed at its actual source where safe; do not add blanket `allow` attributes or delete planned functionality merely to silence the lint.

## Invariants

No chemistry equations, chemical potentials, activation thresholds, contact tolerances, reaction rules, material eligibility, geometry/contact validation, energy accounting, transaction atomicity, or deterministic/fair scheduling may change as a performance shortcut. No cache may stand in for live physical eligibility. Persistence changes require cross-process locking, duplicate detection, crash recovery, manifest consistency, and documented durability semantics.
