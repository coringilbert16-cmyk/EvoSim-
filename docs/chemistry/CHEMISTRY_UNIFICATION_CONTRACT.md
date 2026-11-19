# Chemistry Unification Contract

Status: implementation contract approved for the chemistry-unification workstream.
Plan authority: Section 17 of [the chemistry audit and unification plan](https://github.com/coringilbert16-cmyk/EvoSim-/blob/09254d9e11fd021f42a0c8abbd43594cd7764c69/docs/chemistry-audit-unification-plan.md).

## Frozen starting point

- Base branch: `main`
- Base commit: `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5`
- Recorded Rust workflow: [run 39705628427](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39705628427), 563 passed, 28 failed, 1 ignored; workflow conclusion is failure. Do not call the suite green.
- PR #204 remains documentation-only. PR #196 constructor/Bob work is out of scope.
- Work is performed on a separate implementation branch. Keep each change preceded by a focused audit and keep commands/tests bounded to five minutes.

## Non-negotiable invariants

1. **Resource identity is canonical.** Resource ID 6 is Oxygen. Unknown resource identities fail closed; they must not silently yield zero mass or energy.
2. **Base constituents are indivisible.** Material ownership and transfers conserve nonnegative integer counts per resource. Fluid area/shape and geometry are separate quantities and do not license fractional atoms.
3. **Energy has one accounting model.** Initial energy enters through an explicit one-time external-source transaction. Internal transfers cancel at world scale. External input, usable energy, stored bond energy, work, heat, and other sinks must reconcile.
4. **Bond energy has explicit ownership.** Only realized energy-bearing bonds own stored bond energy. A bond's stored energy is settled or released no more than once, including retry, failure, and rollback paths. Do not invent stored energy for physical-material bonds that do not own it.
5. **Tolerance is not dissipation.** Numerical comparison tolerance must be separate from modeled physical dissipation.
6. **Reaction activation is not an energy reservoir.** Accumulators or thresholds may trigger chemistry but cannot be treated as stored physical energy.
7. **Overload has a physical consequence.** Resonance overload can destabilize bonds; it must not be implemented solely as rejection or silent dissipation.
8. **Tick order is deterministic.** State-changing phases have an explicit order. Work may be spread across ticks where required for performance, without violating conservation or duplicate-settlement rules.
9. **Death does not create a second chemistry system.** After death, matter remains in the environment and continues through ordinary chemistry. Legacy recycling/decomposition helpers must not be reactivated as parallel production processes.
10. **Caches are equation-specific.** Chemistry cache identity includes equation/parameter versioning. Incompatible legacy chemistry caches are rejected without migration. Bob geometry data remains separate unless its semantic inputs change.
11. **Sunlight approximation is evidence-gated.** Use the fast directional first-hit/single-sweep model only after comparison with an exact small-world reference, source-budget and occlusion checks, duplicate-claim checks, and representative performance measurements. Signal onward propagation is not literal mirror-ray reflection.
12. **No silent conservation repair.** Strict tests fail on invariant violations. Exploratory diagnostics may report explicitly labelled residuals but must not silently repair state.

## Current dependency map (main at the frozen commit)

| Concern | Primary files to audit | Known issue / integration boundary |
|---|---|---|
| Resource catalogue and amount validation | `src/materials/resources.rs` | `Material::is_valid` and `Material::take` are central boundaries; validate integer ownership and reject unknown identities consistently. |
| Physical realization and stored material | `src/materials/physical_material.rs`, `src/materials/material_storage.rs` | Physical realization/storage already has discrete constraints; align aggregate paths without conflating atom count with fluid area. |
| Aggregate transfer | `src/materials/material_transfer.rs` | Existing test `transfers_whole_units_from_fractional_aggregate` permits a state that conflicts with the indivisible-base-constituent contract. |
| Primitive chemistry math | `src/chemistry/chemistry.rs` | Preserve the approved chemistry equations while documenting unit interpretation; do not silently equate reactivity and harmonic response. |
| Reaction definition and realization | `src/chemistry/combine.rs`, `src/chemistry/combine_runtime.rs`, `src/chemistry/transformation.rs` | COMBINE assigns energy to realized bonds; BREAK consumes stored bond energy first and handles shortfall. Verify atomicity and exactly-once settlement. |
| Energy accounting | `src/chemistry/energy_ledger.rs`, `src/simulation/simulation.rs` | Shared transaction shapes exist, but tests need an independent world-level oracle. Initialization totals and external-source accounting must agree. |
| World-tone input | `src/simulation/simulation.rs` and the world-tone modules | Live capture/settlement exists; whole-world correctness, approximation error, and scale remain unproven. |
| Chemistry cache | `src/chemistry/chemistry_library.rs` | Current schema is 3 and accepts schema 2 as well; equation/parameter fingerprinting and reject-without-migration policy remain to be enforced. |
| Death/lifecycle | `src/simulation/simulation.rs`, `src/chemistry/decomposition.rs`, `src/chemistry/recycling.rs` | Ordinary post-death chemistry is the intended production path; legacy helpers stay out of the normal path unless explicitly audited and approved. |

## Required independent evidence

- An energy inventory oracle must derive total stored energy from state independently of the production ledger's own aggregate counters.
- A material inventory oracle must count authoritative integer constituents across every owner/storage/location and compare before/after transactions.
- Failed or retried operations must demonstrate atomicity: either all related state changes commit once, or none commit.
- Small-world sunlight tests compare the fast approximation against a deterministic exact reference and report error; large-world benchmarks establish actual runtime and memory cost before thresholds are chosen.
- Baseline test failures are compared by test identity when available. An unchanged aggregate failure count is not sufficient proof that no regression occurred.

## Approved phase sequence

Follow Section 17 phases 0–11 in order: freeze baseline; publish contract/dependency map; build independent oracles; enforce resource identity and integer ownership; establish cache-version boundary; make energy ledger authoritative; unify reaction/bond/overload accounting; close energy consumers and deterministic tick phases; unify lifecycle; validate sunlight; test long-run numerical integrity; complete integrated verification.

## Change discipline

- Audit the affected call paths immediately before each change.
- Keep changes cohesive and in dependency order; do not combine unrelated cleanup.
- Do not modify PR #204, pull in PR #196, or change README status claims as a side effect.
- Run focused verification first; each command/test invocation must be bounded to five minutes.
- Report the exact commit, files changed, checks run, failures and limitations. No green-suite claim while the known baseline remains red.
