# Chemistry Audit and Unification Plan

**Status:** Planning outline; audit findings are to be added section by section.  
**Baseline branch:** `main`  
**Baseline commit:** `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5`  
**Plan opened:** 2026-10-22  
**Scope:** Read-only audits first; no chemistry behavior changes are authorized by this document alone.

## 1. Purpose and acceptance target

EvoSim is an open-ended artificial-life simulation. The goal is not to hard-code organism roles or declare an organism alive because it has a qualifying cavity. Chemistry, energy exchange, geometry, perception, stress, behavior, death, and evolution must interact through shared mechanisms so that cells, niches, populations, and eventually cooperative multicellularity can emerge from the world.

This workstream will establish one coherent chemistry and energy model, document the authoritative equations and ownership boundaries, identify every producer/holder/consumer/transfer of energy and matter, and produce an evidence-backed implementation sequence. The work must improve scientific consistency without making the simulation too expensive as populations and material counts grow.

**The central acceptance condition:** every physical change that creates, moves, stores, transfers, releases, transforms, or dissipates energy or material must have one explicit authority, one defined transaction boundary, and an auditable conservation consequence. Internal transfers must cancel at world scale. No cache, spectrum, reaction accumulator, resource property, construction allowance, or event counter may silently become a second source of physical energy.

A successful audit is not a claim that the system is already unified. Mark each area as **Verified**, **Gap found**, **Blocked**, or **Not applicable**, with evidence and exact source/test references. Leave unknowns explicitly unknown.

## 2. Non-negotiable design constraints

- **Shared mechanism, not parallel special cases.** Chemical BREAK, stress-triggered BREAK, and post-death bond changes should converge on the same authoritative physical validation and energy settlement. Lifecycle orchestration may differ only where necessary.
- **Decomposition is chemistry after death.** Do not create a separate decomposition law that bypasses ordinary material, bond, or energy rules.
- **Energy is accounted for once.** Separate external input, usable energy, persistent structural/bond energy, work, and dissipated heat. Internal transfers cancel in the world total. Release a stored bond energy at most once.
- **Potential energy is an immutable resource/bond property, not an untracked spendable wallet.** Distinguish resource type potential, a realized bond's stored energy, an organism's usable energy, and a signal's available energy.
- **A force is not energy.** Work must derive from force over realized displacement; attraction, reaction accumulation, resonance amplitude, and activation signals are not interchangeable with joules/energy units.
- **Geometry remains authoritative.** Actual realized material shapes, contacts, bond identities, endpoint availability, non-penetration, and fluid boundary constraints decide whether a physical reaction can commit. A chemistry cache may accelerate lookup but cannot override live geometry.
- **Transactions are atomic.** Validate against trial state, settle all energy/material consequences, then commit. Failed or stale events must not leave partial geometry, energy, resonance, or environment mutations.
- **Death changes biological eligibility, not existence.** A breached genome core or the approved unsustainable-stress condition can end biological processing; the body and its constituents remain in the physical world and may continue ordinary chemistry.
- **Sunlight/world tone remains disabled for live capture until conservation and allocation are proven.** The source is deterministic and directional; occlusion matters. “Reflection” means onward propagation with changed frequency composition, not literal mirror-ray tracing.
- **Bob remains a geometry library worker/viewer.** Prefer persisted suggestions over repeated runtime geometric discovery. This chemistry work does not pull in deferred PR #196 constructor changes.
- **Scalability is a correctness constraint.** Do not solve conservation by cloning the whole world each tick, doing all-pairs reaction scans, tracing a ray per object, retaining unbounded event histories, or persisting redundant derived state.
- **No silent baseline rewriting.** Preserve the accepted 28-failure baseline as historical context; compare exact test identities and assertions rather than treating every existing failure as a new regression. Do not weaken physical invariants to make CI green.
- **README is the current project guide.** Dated handoff archives are historical evidence and may contain superseded or contradictory snapshots. Confirm every finding against current `main`.

## 3. Baseline and known unresolved claims to verify

These are audit leads, not automatically accepted implementation facts.

- Current README says whole-world energy conservation across every storage, reaction, and lifecycle path is **not yet proven**.
- Current README says live sunlight/world-tone energy capture is **not yet proven** and remains disabled until allocation/conservation is complete.
- Current README says complete self-maintaining organism viability is **not yet proven**.
- `EnergyLedger` provides staged transaction/accounting primitives, but its existence does not prove a complete live world inventory.
- `EnergySourceBudget` intentionally is not `Copy` or `Clone`; that prevents one class of duplicate budget but does not itself guarantee callers share one correctly scoped allowance or prevent duplicate exposure claims.
- Reaction accumulation is a driving quantity, not by itself a named source debit.
- Resonance compatibility may lower a barrier; only actual withdrawn energy can pay for work, and the withdrawal must be committed only if the physical reaction commits.
- The harmonic reservoir has a bounded spectrum; any rejected deposition or capacity overflow must have an explicit physical fate, not disappear.
- The chemistry library is a persistent interface-keyed cache, not the authority for the whole reaction policy, resource mass, bond geometry, or global energy balance. Audit schema/manifest/record compatibility and failure handling before relying on it.
- Historical archives mention older schema mismatches, energy holders, and incomplete source wiring. Re-check current source and current manifest before promoting any archived statement to a current defect.
- The current chemistry work must not be confused with PR #196's constructor/Bob stabilization. Keep PR #196 open and draft-only while chemistry is defined on `main`.

## 4. Audit method and change control

1. Record the exact `main` SHA, current README status, workflow runs, and accepted test baseline before changes.
2. Map data ownership and call paths before changing equations or types.
3. Audit one domain at a time. For every claim, link exact file/function, relevant tests, and observed evidence.
4. Maintain a live dependency/impact matrix so changes to a shared primitive identify all callers and tests.
5. Do not mix behavior changes with broad formatting, dead-code cleanup, or unrelated repository organization.
6. For each proposed code change: perform one focused pre-change audit, state the invariant being repaired, change the smallest coherent unit, and test that unit before moving on.
7. Every audit result must distinguish source inspection, focused-test evidence, whole-suite evidence, and runtime/performance evidence.
8. Run bounded commands; keep the hard five-minute limit for each CI command/test and do not mask timeout as success. If a test suite hits the limit, report incomplete results rather than a fabricated failure count.
9. Compare failing test identities and assertions with the accepted baseline. Do not treat the known 28 failures as new regressions without evidence, and do not assume the baseline is still identical without checking.
10. Update this document as each audit completes. Update the root README only after verified decisions/status change; keep it the authoritative concise summary.

## 5. Audit register — fill in after each audit

For every audit below, fill this record:

- **Status:** Not started / In progress / Verified / Gap found / Blocked.
- **Exact baseline SHA and files/lines inspected:**
- **Authoritative rule or equation found:**
- **Actual runtime call path:**
- **State mutated and transaction boundary:**
- **Energy/material source and sink:**
- **Failure/rollback behavior:**
- **Numerical edge cases and invalid-state behavior:**
- **Complexity and scaling implications:**
- **Tests already covering it:**
- **Focused tests to add or run:**
- **Findings ranked by severity:**
- **Dependencies / downstream audits affected:**
- **Decision required from project owner, if any:**
- **Recommended smallest next change (if authorized):**

Do not mark an audit Verified solely because code compiles or a helper has unit tests. The production call path and conservation consequences must be evidenced.

## 6. Planned audits

### A0 — Baseline, current source of truth, and change inventory

**Questions**
- What is the exact current `main` SHA and what CI evidence corresponds to that exact tree?
- Which README statements are current, which archive statements are historical, and which claims require revalidation?
- Which tests make up the accepted baseline, and are the same 28 test identities still failing?
- What source files, persistent data, serialization formats, workflows, and CLI/runtime paths can chemistry changes affect?

**Inspect**
- `README.md`, `README_HANDOFF_ARCHIVE_2026-10-22.md`, `docs/README.md`.
- `src/main.rs`, `Cargo.toml`, lockfile, all relevant workflow files and scripts.
- Existing chemistry, energy, lifecycle, resource, world-tone, geometry, and simulation tests.
- Current `main` workflow runs and baseline links.

**Deliverable**
- Exact baseline manifest, confirmed baseline test identities, current source-of-truth map, and explicit exclusions. No code changes.

### A1 — Resource vocabulary, properties, and units

**Questions**
- Is there exactly one canonical resource catalog and one authority for immutable properties?
- Are mass, potential energy, reactivity, cohesion, chemical position, material composition, and resource amounts used consistently?
- Are the seven approved resources and their chemical-position ordering consistent with current README/spec?
- Is potential energy incorrectly used as mutable energy, reaction power, or an initial wallet anywhere?
- What do fractional/composite material amounts mean physically, and where are constituent identities preserved?

**Inspect**
- `src/materials/resources.rs`, physical-material/composition types, catalog/defaults, material restoration/storage/transfer, construction material selection, resource visualization, serialization and test fixtures.
- Every read/write of `potential_energy`, `mass`, `reactivity`, `cohesion`, chemical position, and material-part quantities.

**Deliverable**
- Property/unit table; single-authority map; invalid/missing-resource policy; list of duplicate or contradictory formulas. No arbitrary geometry/catalog changes.

### A2 — Core chemistry mathematics and dimensional consistency

**Questions**
- What are the authoritative definitions and units for chemical position, interaction potential, contact attenuation, attraction force, reaction accumulation, normalized chemistry-to-energy conversion, dissipation, and activation barrier?
- Are constants defined once and used consistently?
- Is accumulated reaction state a dimensionless/normalized quantity, a force/work proxy, or physical energy—and does code ever conflate these?
- Are distance/contact functions continuous, bounded, finite, and monotonic where intended?
- Do invalid floats, zero distance, out-of-range positions, NaN, infinity, and negative inputs fail closed?

**Inspect**
- `src/chemistry/chemistry.rs`, related constants, accumulation maps in simulation state, tests, diagnostics, and all duplicated formulas in callers.

**Deliverable**
- Equation/units sheet, invariant tests, dimensional ambiguity list, and proposed canonical API. Do not retune constants before units and callers are understood.

### A3 — Physical interaction, COMBINE, formation work, and stored bond energy

**Questions**
- How does a candidate physical interface become eligible for COMBINE?
- Is geometry/contact authority separate from chemistry eligibility?
- Is formation work derived from the actual remaining approach distance, and is work already performed by movement charged twice?
- How is intrinsic stored bond energy derived and stored?
- Are bond strength, contact-feature scale, cohesion, chemical potential, formation threshold, and stored energy distinct concepts?
- Does a failed COMBINE leave material, bond, energy, and cache state unchanged?

**Inspect**
- `src/chemistry/combine.rs`, `combine_runtime.rs`, `src/geometry/contact.rs`, connection/structure authorities, materials, movement, construction, and COMBINE architecture checks.
- Bond creation, multi-contact/fluid transactions, composite constituents, endpoint capacity, contact feature class, and trial/commit boundaries.

**Deliverable**
- COMBINE transaction state diagram; exact source of formation work and structural energy; caller inventory; atomicity tests for rigid, composite, and supported fluid cases.

### A4 — BREAK paths and structural-energy release

**Questions**
- What BREAK paths exist: voluntary/chemical, stress-triggered, overload, lifecycle/post-death, and any administrative/test-only path?
- Do they use one physical validation and settlement authority?
- Is stored energy removed from the bond before/with release, exactly once?
- Can two same-tick operations target a bond that has already been removed or changed?
- Are resonance withdrawals, work, heat, and usable-energy deltas settled only if the graph mutation commits?
- Does a failed reaction preserve both the reservoir and the original bond graph?

**Inspect**
- `src/chemistry/chemical_reaction.rs`, `transformation.rs`, stress code, `decomposition.rs`, `recycling.rs`, structure bond mutation, energy ledger, event ordering and same-tick revalidation.

**Deliverable**
- Complete BREAK call graph and path-equivalence table; identify the one authoritative commit/settlement primitive and necessary lifecycle-specific orchestration.

### A5 — Energy ledger transaction algebra and API guarantees

**Questions**
- What is the exact sign convention for usable, structural, source, work, and heat deltas?
- Does every accepted transaction balance and reject non-finite/negative/impossible values?
- Is energy created by a counter increment, a positive `potential_released`, or an unbacked caller claim without debiting a named holder/source?
- Are internal transfers represented as equal debit/credit, with zero world-total delta?
- Does rollback include every affected store?
- Are transaction reason labels complete and semantically useful without being treated as energy sources?

**Inspect**
- `src/chemistry/energy_ledger.rs`, `state::EnergyLedger`, transaction callers, source-budget code, and all transaction tests.

**Deliverable**
- Formal transaction equations, sign table, valid/invalid transaction matrix, and a list of calls that bypass or under-specify settlement.

### A6 — Whole-world energy inventory and conservation proof

**Questions**
- What energy exists at tick zero and at each subsequent tick?
- Can we independently enumerate usable energy, persistent bond/structural energy, retained resonance energy, external input, work, heat/dissipation, and any legitimate boundary output?
- Which counters are cumulative diagnostics versus actual stores?
- Does every creation/removal of a holder transfer or settle its remaining energy?
- Can an energy snapshot be reconciled without counting internal transfers twice?
- What is the tolerance policy, including floating-point accumulation over long runs?

**Inspect**
- All organism energy fields; bond energy; resonant reservoirs; `EnergyLedger`; environment/material energy if any; initial/bootstrap energy; movement/maintenance/reproduction; death/removal; world-tone inputs; save/load.
- `world_tone_snapshot.rs` and any current conservation primitive/tests.

**Deliverable**
- Holder/source/sink inventory, per-tick and cumulative conservation equations, independent snapshot/audit method, adversarial tests, and a statement of what is proven versus still unobservable.

### A7 — Resonance spectrum, reaction compatibility, and stored signal energy

**Questions**
- Are tone amplitudes/frequencies distinct from physical energy?
- Is retained resonance a finite physical energy reservoir with explicit deposits, withdrawals, capacity, and dissipation?
- How are compatible energy and reaction frequency computed? Does compatibility only weight accessible energy rather than manufacture energy?
- When spectrum capacity is full or a frequency is invalid, where does rejected input go?
- Does reservoir pruning/normalization preserve total energy?
- Can repeated reactions overspend a shared reservoir or withdraw energy before a reaction is known to commit?

**Inspect**
- `src/perception/harmonics.rs`, reaction frequency production, transformation, organism state/serialization, spectrum propagation, and tests.

**Deliverable**
- Spectrum-versus-energy contract, reservoir invariant table, compatibility/withdrawal transaction tests, and bounded-memory/per-tick cost assessment.

### A8 — World tone/sunlight source, exposure, occlusion, propagation, and capture

**Questions**
- Are source position, frequency, intensity, orbit/day-cycle, and per-width units explicit and deterministic?
- Are per-tick samples precomputable/cached by world dimensions and tick without repeated per-object sun calculations?
- Does exposure account for source direction, first-hit/occlusion, projected width, and unique exposed surfaces?
- Is source energy allocated once per exposed region and prevented from duplicate claims across receivers?
- Does material damping partition incident energy into captured energy and onward energy without creating a second source?
- Does onward propagation carry only remaining energy, permit frequency-composition changes, and obey obstruction/shadow rules without literal mirror-ray tracing?
- Can live capture be enabled only after source-backed ledger settlement and global reconciliation pass?

**Inspect**
- `src/simulation/world_tone.rs`, `world_tone_snapshot.rs`, `src/geometry/source_exposure.rs`, `EnergySourceBudget`, harmonics, environment geometry, simulation tick order, and world-tone tests.

**Design constraints to preserve**
- One off-screen rotating point source; 440 Hz at the apex; 1–10 energy units per tick per 1.0 unit of exposed projected width; 2,400-tick cycle; deterministic precomputed/cached source samples; shadows matter.
- Use an efficient sweep/index or similarly bounded exposure algorithm, not one full geometry ray trace per object. The accepted design may be slightly approximate if the error is characterized and conservation remains exact.
- Live capture stays disabled until all capture allocation, duplicate prevention, and conservation gates are met.

**Deliverable**
- Source-to-receiver flow model, complexity estimate for thousands of objects, deterministic reference tests, occlusion/energy-partition tests, and explicit enablement gate.

### A9 — Dissipation, heat, work, and environmental boundary

**Questions**
- What is the meaning and unit of dissipation at reaction, resonance, source capture, maintenance, movement, and bond break?
- Is dissipation applied once per tick/event and not duplicated by both caller and ledger?
- Is heat tracked as a world sink, a physical reservoir, or an explicitly accepted outflow?
- Are force/work/distance calculations physically and dimensionally consistent?
- Are there implicit energy sinks in cleanup, holder deletion, invalid transactions, failed deposits, or persistence recovery?

**Inspect**
- Chemistry accumulation, harmonic reservoir, world-tone response, movement cost, maintenance cost, bond formation/break, simulation tick, energy diagnostics and snapshot equations.

**Deliverable**
- Dissipation/work/sink matrix; explicit heat policy; tests for energy partition and no silent loss.

### A10 — Stress, damage, core breach, death, and biological eligibility

**Questions**
- What exactly triggers death: genome-core breach and/or the approved unsustainable stress damage condition that opens the core?
- Is death a one-way biological state transition while preserving the physical organism/material?
- Can a dead/breached organism still receive biological stress, decisions, maintenance, reproduction, growth, or other life-only processing?
- Can physical chemistry continue after death?
- Does stress-triggered bond break share A4's physical/energy settlement?
- Is the core rechecked immediately after mutations that may breach it, including same-tick chemical operations?
- Can pending transformations execute after death and duplicate or violate energy settlement?

**Inspect**
- `src/simulation/simulation.rs`, organism state, stress/damage logic, viability/cavity analysis, transformation scheduling, decision/movement/maintenance/reproduction gates, and lifecycle tests.

**Deliverable**
- Explicit state-transition table and tick-order diagram; regressions for chemical and stress-induced breach; proof that dead bodies persist physically and ordinary chemistry remains possible.

### A11 — Post-death chemistry, decomposition/recycling, and material conservation

**Questions**
- Is any legacy decomposition/recycling path still reachable in the production tick?
- Does post-death bond removal use the same BREAK transaction and physical invariants?
- What happens to remaining usable energy when the last bond is removed or a body/carrier is deleted?
- Where do constituents go when a structure breaks, coalesces, is expelled, or is otherwise dismantled?
- Are material amounts conserved across environment ↔ organism ↔ offspring ↔ composite ↔ decomposition?
- Are there duplicate representations of the same constituent or silent resource deletion?

**Inspect**
- `src/chemistry/decomposition.rs`, `recycling.rs`, `expulsion.rs`, environmental material storage/transfer/restoration, organism removal, fluid coalescence, reproduction, and simulation lifecycle.

**Deliverable**
- Material/energy fate table for every lifecycle event; remove/disable legacy paths only after reachability is proven and tests cover the replacement path.

### A12 — Maintenance, movement, reproduction, development, and all energy consumers/transfers

**Questions**
- Are maintenance and movement costs debited once from the actual holder?
- Is movement work tied to realized movement and failure/rollback behavior?
- Are reproductive allocation, parent debit, offspring credit, and development energy transfers balanced?
- Are bootstrap/initial organism energy and constructor allowances kept distinct?
- Can growth, acquisition, or constructor code manufacture energy from a budget or resource potential?
- Do dormant/unfinished holders retain or settle energy?

**Inspect**
- `src/perception/movement.rs`, maintenance logic, `src/organism/reproduction.rs`, juvenile/development logic, constructor, resource acquisition, simulation initialization, offspring lifecycle, and all energy fields/usages.

**Boundary**
- Audit for chemistry coupling and invariants only. PR #196 constructor/Bob redesign and movement/reproduction redesign remain deferred; do not pull them into this workstream unless a directly proven conservation defect requires a narrowly scoped fix.

**Deliverable**
- Consumer/transfer inventory, per-path equations, failed-action behavior, and explicit initial-energy policy.

### A13 — Material and energy persistence, schemas, concurrency, and recovery

**Questions**
- Are serialized organism reservoirs, bond energies, ledgers, world-tone state, and materials backward-compatible and validated?
- Is the chemistry cache schema consistent with its manifest and every row?
- Are duplicate keys, duplicate IDs, malformed/truncated rows, invalid records, and count mismatches detected rather than silently overwritten/skipped?
- Can concurrent writers or crash/reopen paths lose or duplicate physical state?
- Are caches safely rebuildable from authoritative data, and do cache misses preserve the same semantics as hits?
- Does a schema migration change meaning or only representation?

**Inspect**
- `src/chemistry/chemistry_library.rs`, `chemistry_library/data/`, manifests, locks, atomic-write/rollback code, serde defaults, save/load, and persistence tests.

**Deliverable**
- Schema compatibility matrix; corruption/concurrency/recovery findings; migration plan only if needed; cache-versus-authority contract.

### A14 — Chemistry cache semantics and geometry coupling

**Questions**
- Does the cache contain reusable static knowledge only, or is it accidentally treated as live physical truth?
- Are keys canonical, symmetric where intended, versioned, and complete for relevant resource/interface properties?
- Are stale/missing records handled deterministically and safely?
- Can a cached potential override current material properties, realized geometry, live contact, or energy accounting?
- What is the fallback cost and can runtime code accidentally trigger unbounded search or persistence writes?

**Inspect**
- Chemistry library keys, records, query/miss/insert paths, geometry reference library interfaces, contact signatures, current schema/data, and callers.

**Deliverable**
- Cache authority table, correctness requirements, hit/miss equivalence tests, and runtime performance budget.

### A15 — Simulation ordering, event scheduling, determinism, and stale operations

**Questions**
- What is the precise tick order for world-tone sampling, perception, accumulation, reaction resolution, stress, death checks, maintenance, movement, reproduction, material updates, and ledger reconciliation?
- Can event order affect outcomes in unintended ways?
- Are pending events revalidated against the current bond graph and energy state before settlement?
- Is reaction ordering deterministic across runs and save/load?
- Does a large number of organisms or bonds cause unbounded queues or repeated scans?
- Are source budgets scoped to a tick/region/receiver in a way that prevents duplicated claims?

**Inspect**
- `src/simulation/simulation.rs`, state and runtime, active transformations, accumulation maps, source snapshot/budget, all event queues and iteration order.

**Deliverable**
- Tick-sequence diagram, stale-event/duplicate-settlement risks, determinism policy, and asymptotic hotspot list.

### A16 — Whole-world material conservation and physical representation

**Questions**
- Does each physical constituent have one authoritative identity/location at a time?
- Do material transfer, expulsion, composite formation/splitting, fluid merge, death, and reproduction conserve constituent amounts?
- Are structural bonds and composite-internal bonds represented consistently?
- Can resource catalog properties diverge from restored physical materials?
- Are non-physical logical materials accidentally persisted as if they were physical material?

**Inspect**
- Material/resource types, environment collections, physical material, structure/composite bond graphs, material storage/transfer/restoration, fluid coalescence, expulsion, reproduction, and serialization.

**Deliverable**
- Material inventory equation and object-identity invariants, with tests that compare before/after physical totals.

### A17 — Diagnostics, observability, UI, and scientific interpretability

**Questions**
- Can we inspect where energy is stored, where it came from, where it went, and why an event failed without dumping unbounded logs?
- Do diagnostics distinguish force, reaction accumulation, usable energy, bond energy, resonance energy, source input, work, heat, and material amount?
- Are names/units consistent in UI, logs, serialized output, and tests?
- Can conservation drift and performance regressions be detected from compact counters?
- Are failure reasons sufficiently precise to separate geometry rejection, chemical ineligibility, insufficient energy, stale event, and conservation failure?

**Inspect**
- Diagnostics, resource visualization, server/UI boundaries, test helpers, errors, event summaries, and all energy-related metrics.

**Deliverable**
- Minimal instrumentation schema, unit labels, bounded aggregation strategy, and actionable error taxonomy. Avoid per-object/per-tick log spam.

### A18 — Numerical stability and long-run behavior

**Questions**
- Can repeated tiny transactions accumulate drift beyond tolerance?
- Are all calculations robust to non-finite input, overflow, underflow, extreme world size, and zero/near-zero mass or contact?
- Are rounding/quantization used only for cache identity or metadata rather than silently changing physical values?
- Do spectrum normalization and resource quantities preserve totals?
- Can long simulations develop negative energy, unbounded resonance components, overflowed counters, or stale accumulation entries?

**Inspect**
- All audited equations and state containers, floating-point tolerances, quantization, serialization, long-run tests, and metrics.

**Deliverable**
- Numerical risk register, justified tolerance policy, property-based/adversarial tests, and a bounded long-run conservation test.

### A19 — Scalability and profiling at target populations

**Questions**
- What is the complexity of each per-tick chemistry stage in organisms (N), constituents (M), bonds (B), candidate contacts (C), and spectral components (S)?
- Which stages scan all organism pairs, all bonds per organism, all materials in the world, or all source/object pairs?
- Can contact/event indexes be updated incrementally rather than rebuilt?
- Are world-tone samples cached by tick/world configuration and exposure computed once per relevant surface?
- Are full-world conservation snapshots debug/test-only, sampled, or incremental in production?
- Are caches bounded and deterministic, and does cache miss/growth trigger expensive runtime discovery?
- What happens under thousands of organisms, dense contact graphs, high reaction activity, and long lifetimes?

**Inspect**
- Tick loop, contact cache, reaction candidate generation, event accumulation, world-tone exposure, chemistry library persistence, energy accounting, and profiler/test harnesses.

**Required approach**
- Establish a measured baseline before optimization.
- Prefer O(N + B + C) or similarly sparse/local work over O(N²) all-pairs processing where physical locality permits.
- Avoid full-world clone-per-event; use narrow trial state or reversible transaction staging.
- Keep expensive independent audits available in tests/debug mode, while production uses incremental accounting if its invariants can be proven.
- Bound event queues, caches, histories, spectra, and persistence growth.
- Characterize approximation error explicitly; do not trade away conservation or physical invariants for speed.

**Deliverable**
- Complexity table, benchmark scenarios and measurements, hot-path budget, and proposed optimization order based on evidence.

### A20 — Cross-domain invariants and integration tests

**Questions**
- Which invariants must hold across more than one subsystem and cannot be established by isolated unit tests?
- Do COMBINE → stored bond energy → BREAK → heat/usable energy balance?
- Does world-tone input → resonance storage → compatible reaction withdrawal conserve the source input and energy remaining?
- Does stress/core breach → death → ordinary post-death chemistry preserve material and energy?
- Do parent → offspring energy transfers conserve the total?
- Do save/load and failed transaction paths preserve the same inventory?

**Inspect**
- Cross-domain integration tests and all relevant transaction callers; add no test that assumes the behavior under test instead of measuring independent state before/after.

**Deliverable**
- End-to-end invariant matrix, adversarial scenario list, and exact CI commands with bounded timeouts.

### A21 — Documentation/spec alignment and implementation sequencing

**Questions**
- Does README/spec agree with code, test expectations, and cache schemas?
- Which rules are settled, which are parameters awaiting owner decision, and which are implementation gaps?
- What is the smallest safe implementation order that minimizes rework?
- Which gates must pass before live sunlight capture or broader organism viability can be claimed?

**Deliverable**
- Updated root README status, finalized equations/terms, migration notes, staged implementation PR plan, rollback conditions, and unresolved decision list.

## 7. Cross-cutting impact matrix

Fill in the final “authoritative owner / callers / tests / status” columns as audits progress.

| Domain | Likely source-of-truth area | Downstream systems to trace | Key invariant |
|---|---|---|---|
| Resource properties and amounts | `materials/resources.rs`, material types | COMBINE, construction, environment, persistence, UI | One immutable property authority; amount conservation |
| Interaction mathematics | `chemistry/chemistry.rs` | accumulation, COMBINE, activation | Defined units; finite bounded output |
| COMBINE and bond storage | `chemistry/combine.rs`, `combine_runtime.rs`, structure | construction, organism growth, fluid contact | Atomic physical/energy commit |
| BREAK | `chemical_reaction.rs`, `transformation.rs`, structure | stress, overload, death, post-death | Stored bond energy released at most once |
| Energy accounting | `energy_ledger.rs`, `state::EnergyLedger` | all energy holders and events | Balanced transaction; global reconciliation |
| Resonance | `perception/harmonics.rs`, organism state | reaction, source propagation | Signal representation is not free energy |
| World-tone source | `simulation/world_tone.rs`, snapshot | exposure, resonance, ledger | Unique source-backed capture; shadowing |
| Exposure/geometry | `geometry/source_exposure.rs`, contact/structure | world tone, COMBINE, BREAK | Live geometry is authoritative |
| Death/lifecycle | `simulation/simulation.rs`, organism state | stress, decisions, maintenance, reproduction | Death stops biology, not physical existence |
| Post-death/material fate | decomposition/recycling/expulsion/material transfer | environment, storage, chemistry | No vanished or duplicated material/energy |
| Reproduction/development | organism reproduction/development, simulation | parent/offspring holders | Transfer debit equals credit |
| Persistence/cache | chemistry library and manifests | all chemistry lookups | Cache/schema never changes physical truth |
| Diagnostics/UI | diagnostics, visualization, runtime | operators/tests | Distinct quantities and units; bounded overhead |
| Performance | tick loop, indexes, caches, event queues | all above | Sparse/local bounded work at population scale |

## 8. Implementation sequence — provisional dependency order

This is a dependency order, not permission to begin all changes immediately.

1. **Establish baseline and inventory (A0).**
2. **Lock vocabulary, units, and equations (A1–A2).** Do not retune constants while dimensional meaning is ambiguous.
3. **Unify transaction semantics and bond accounting (A3–A5).** Establish the shared physical/energy commit primitive for COMBINE and BREAK.
4. **Build an independent world inventory (A6, A16, A18).** Prove the audit equations against isolated and integration scenarios.
5. **Unify resonance/reaction coupling (A7) and stress/death/post-death behavior (A10–A11).**
6. **Audit every existing energy consumer and transfer (A9, A12), then event ordering (A15).**
7. **Repair/verify persistence and cache semantics (A13–A14).**
8. **Complete source exposure and world-tone capture design (A8), but do not enable live capture until its tests reconcile with A6.**
9. **Complete diagnostics and scaling benchmarks (A17, A19).**
10. **Run cross-domain invariants and documentation alignment (A20–A21).**
11. Only then implement the smallest coherent production changes in focused steps, with one audit before each change and exact-head CI comparison after each.

The sequence may be revised if an audit uncovers a blocking dependency, but the reason and downstream consequences must be recorded here.

## 9. Release/enablement gates

Do not claim chemistry unification complete until all applicable gates have evidence.

- [ ] One authoritative unit/meaning for each energy and chemistry quantity.
- [ ] Every live COMBINE/BREAK path uses the approved physical and energy transaction boundaries.
- [ ] Failed transactions are atomic across geometry, energy, resonance, and material state.
- [ ] An independent whole-world energy inventory reconciles initial stores + external input with final stores + work + heat/output within a justified tolerance.
- [ ] Internal transfers cancel and no source is debited/credited twice.
- [ ] Structural bond energy is released at most once.
- [ ] Material quantities/identities reconcile through transfer, composite changes, reproduction, death, and post-death chemistry.
- [ ] Dead bodies stop biological processing while remaining physical and chemically interactive.
- [ ] Persistence/cache corruption and schema incompatibility fail visibly and safely.
- [ ] World-tone exposure, occlusion, source allocation, propagation, and resonance capture pass conservation and duplicate-claim tests before live capture is enabled.
- [ ] Performance is measured against declared target scenarios, including thousands of objects; no accidental quadratic or per-object ray-tracing regression.
- [ ] Exact-head formatting, compile, focused tests, source-size/architecture checks, baseline comparison, and bounded integration tests are reported accurately.
- [ ] README and this plan distinguish verified behavior, known baseline failures, open gaps, and unproven aspirations.

## 10. Audit log

| Audit | Status | Evidence / output | Open blockers |
|---|---|---|---|
| A0 Baseline and inventory | Not started | — | — |
| A1 Resource vocabulary/properties | Not started | — | — |
| A2 Core chemistry math/units | Not started | — | — |
| A3 COMBINE and bond storage | Not started | — | — |
| A4 BREAK paths | Not started | — | — |
| A5 Ledger transaction algebra | Not started | — | — |
| A6 Whole-world energy inventory | Not started | — | — |
| A7 Resonance and reaction coupling | Not started | — | — |
| A8 World tone and capture | Not started | — | — |
| A9 Work, heat, dissipation | Not started | — | — |
| A10 Stress/death/core breach | Not started | — | — |
| A11 Post-death chemistry/material fate | Not started | — | — |
| A12 Energy consumers/transfers | Not started | — | — |
| A13 Persistence/schema/recovery | Not started | — | — |
| A14 Chemistry cache semantics | Not started | — | — |
| A15 Tick ordering/determinism | Not started | — | — |
| A16 Material conservation | Not started | — | — |
| A17 Diagnostics/observability | Not started | — | — |
| A18 Numerical stability | Not started | — | — |
| A19 Scalability/profiling | Not started | — | — |
| A20 Cross-domain integration | Not started | — | — |
| A21 Documentation/sequence | Not started | — | — |

## 11. Out of scope unless a direct dependency is proven

- Stabilizing or merging PR #196; constructor/Bob changes remain deferred until the chemistry model is unified.
- Reproduction redesign or movement redesign.
- Unrelated lint cleanup, repo-wide reorganization, broad formatting churn, or speculative API redesign.
- Changing resource shapes or geometry tolerances merely to satisfy chemistry tests.
- Enabling live world-tone capture before the source-backed conservation gate passes.
- Claiming full organism viability from cavity qualification or a bounded constructor search result.

## 12. Decision log

Record decisions here with date, rationale, affected audits, and any superseded rule. If a design choice needs the project owner's decision, leave it open instead of inventing a constant or silently choosing behavior.

| Date | Decision | Rationale / affected audits |
|---|---|---|
| 2026-10-22 | Chemistry unification proceeds on `main`; PR #196 stays open and draft-only. | Avoid stabilizing constructor behavior against unsettled chemistry; A0/A3/A6/A12 boundary. |
| 2026-10-22 | Live world-tone capture remains disabled until source allocation and whole-world conservation are proven. | Prevent unbacked energy creation and duplicate source claims; A6/A8. |
