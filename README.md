# EvoSim

## PR #202 live sunlight settlement + geometry audit — 2026-10-20

**Current code checkpoint:** `f3fede9d9a739614a808bf2f85605e99d4fa00d1` on `feature/external-input-ledger`. CI for this exact head is pending; do not treat it as merge-clean or verified yet.

- **Live tick wiring:** after transformation resolution and living-organism identification, each tick builds one world-space sunlight snapshot and settles direct absorbed input into each living organism's existing usable-energy holder. Material response uses actual resource properties from the catalog. Captured input is recorded as external energy, split into usable energy and dissipated heat, and is not charged against a depletable sun pool.
- **Default source:** `Simulation::new` enables the 440 Hz world tone by default. The initial cycle uses signal levels 0–8, intensity 1–10 energy units per tick per projected-width unit, a 2,400-tick cosine cycle, and an orbit radius derived from the larger world dimension so the source remains outside the rectangle. These are initial model defaults based on the existing configuration fixture, not calibrated biological constants.
- **Exact geometry:** source rays originate at the source center. Polygon boundaries, circular arcs, and finite line endpoints are handled analytically rather than by curve tessellation. Curved-fluid descriptors with invalid area or conflicting boundary representations fail closed.
- **Occluder coverage:** the snapshot includes realized organism structures, environmental physical material, decomposing structures, owner-relative stored material, active-transformation material, and developing offspring plus committed physical inventory. Invalid geometry or anchors fail closed. Multi-part structural units are rejected unless each constituent is separately represented; a stale single-resource shape cannot stand in for a composite.
- **Receiver-to-holder path:** only living organism bodies are exposure receivers; other physical bodies remain eligible occluders. Per-constituent settlement uses catalog properties and the existing ledger. A frame prevents duplicate settlement of the same constituent within a tick.
- **Performance work:** the snapshot uses conservative angular bounds to omit off-angle field materials before full geometry expansion, retains every body that could shadow a living receiver, and computes exact exposure only for selected receivers. Each target edge now restricts critical points, straight-edge intersections, curved blockers, and exact ray tests to angular candidate bodies. Segment-vs-body checks also use conservative bounds before exact shape tests.
- **Transaction rollback:** failed prepared COMBINE resolution reverses staged usable-energy and ledger deltas and restores any reserved environmental source. Stress is not rolled back because the prepared stress delta is only applied after a successful physical commit.
- **Latest completed full-suite checkpoint:** workflow [39522481692](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39522481692) on pre-per-edge-filter checkpoint `99f1da2d2adabf0ce107cdff278c6b0b5b0359ae` reported **502 passed, 28 failed, 1 ignored** (169.10 seconds). A direct comparison against the agreed baseline found the exact same 28 failing test identities. The inventory-reconciliation test still exceeded 60 seconds in that run; the per-edge candidate filtering on the current head specifically addresses that repeated work, but its timing is not yet verified.
- **Known limitation:** this is direct-source capture, not full light transport. The unabsorbed portion is not credited to an organism, but reflected/secondary propagation is not yet modeled. Clippy remains a separate gate; the latest run before the current cleanup still failed on pre-existing lint issues.

**Next gates:** verify formatting and the full suite on the current head; compare failure identities with the agreed baseline; confirm the populated-world inventory test no longer incurs pathological per-tick cost; then audit and implement reflected/secondary propagation without double-crediting energy. Do not spend this workstream trying to erase the agreed 28 baseline failures unless a change introduces a regression.

## External-input transaction — 2026-10-19

First implementation slice on the dedicated `feature/external-input-ledger` branch, based on verified integration head `9b34e911c6eb98ff400ecf58dae09884d55f2ceb`.

- Added a distinct `ExternalInputTransaction` and `settle_external_input` path. External input is never recorded as released bond potential.
- The transaction carries its source, local source availability, captured input, usable-energy gain, work, and heat. It is accepted only when all values are finite/nonnegative, captured input does not exceed the supplied availability, and the energy split balances.
- Settlement updates the holder and all affected ledger counters atomically; rejected input leaves both unchanged. The new cumulative external-input counter defaults to zero when deserializing older ledgers.
- Added focused ledger tests for successful accounting, source-availability rejection, and atomic rejection of an unbalanced input.
- **Not implemented in this slice:** harmonic-response-to-capture calculation, per-tick capture scheduling, live world-tone energy capture, global inventory/conservation, and capture-efficiency inheritance. The caller must supply a valid local source budget and enforce the once-per-organism-per-tick rule; the ledger cannot infer those from a spectrum.
- Exact-head CI exposed three existing world-tone compile errors: `WorldToneSample` was compared/debug-formatted in tests and passed through the harmonic path without implementing `PartialEq`, `Debug`, or `Copy`. Added those value-type derives so the existing source model and call path compile; this does not change source behavior. Formatting-only corrections were also applied to the ledger expression and source-tone integration. Exact-head verification is still required. Do not treat this ledger primitive alone as proof of whole-world conservation or functional solar capture.


## Energy-path audit — 2026-10-20

- The shared `EnergySourceBudget` is intentionally not `Copy` or `Clone`, preventing accidental duplication of a shared remaining allowance. This is an API guard, not a live source scheduler: callers still have to create one correctly scoped budget and pass that same mutable budget to every receiver.
- Source audit confirms COMBINE/construction bond formation, movement expenditure, maintenance, realized-bond chemical BREAK, and stress BREAK use ledger transactions. Chemical and stress BREAK share a graph-validation/settlement/commit path. Reproductive usable-energy transfers conserve energy between holders. The staged legacy decomposition helper also uses the ledger but remains test-only and is not invoked by the production tick.
- The live tick still has no world-tone capture call and no source-budget scheduling across receivers. Directional exposure geometry is staged separately and does not credit energy. Keep capture disabled until the complete live energy inventory reconciles under exact-head tests.
- Verification limitation: GitHub reports no Actions run/status for the current PR head, and local test execution could not clone the repository because this environment could not resolve `github.com`. The source-budget change is source-audited but not compiled or tested here; do not infer a passing exact-head gate.

## Deterministic world-tone cycle cache — 2026-10-19

- The source sample is deterministic for a fixed world size and runtime configuration. `WorldToneCycle` precomputes one sample per tick for cycles up to 65,536 ticks and uses modulo lookup during the simulation tick.
- The cache stores only source outputs (position, frequency, signal level, energy intensity per tick per unit of exposed projected width, and phase). It does not allocate incident energy to organisms, cache live reflections, or replace energy-ledger accounting.
- The cache is rebuilt when world-tone configuration is applied. Cycles longer than the explicit 65,536-sample memory bound fall back to the existing direct sampler; they remain valid rather than being rejected.
- Regression tests compare every cached sample against direct sampling for the full 2,400-tick cycle and verify cycle wrap. A separate test checks that oversized cycles fall back safely.
- **Performance expectation:** this removes repeated trigonometric source sampling from the configured tick path, but it is not yet benchmarked and should not be presented as a major speedup. The source was already sampled only once per tick, not once per organism. Resonance evaluation and live reflection propagation are separate, likely more significant future costs.

**Future library direction (design recommendation, not implementation):** keep Bob focused on geometry. If profiling later justifies reusable harmonic responses, introduce a separate response cache/library keyed by all inputs that affect a deterministic material response (material properties, incoming frequency/composition, and relevant response parameters). Do not store position-dependent energy outcomes or live reflection paths there, and do not duplicate response records in Bob. Keep actual received/captured/reflected/dissipated energy in the live, source-backed accounting path.

## Live world energy inventory — 2026-10-20

- `EnergyLedger` now retains tick-zero baselines for usable energy and stored bond energy, alongside current world totals.
- Simulation initialization captures the initial usable-energy and structural-energy baseline. End-of-tick accounting refreshes current usable energy and sums realized graph-bond energy across the currently inventoried holders. A `DecomposingBody` remains a legacy carrier type, not a distinct chemical process.
- `Simulation::energy_inventory_snapshot()` exposes those current values with cumulative external input, explicitly non-structural energy release, work, and heat to the shared conservation check.
- Added a simulation test that checks the tick-zero inventory starts balanced, in addition to the three ledger-level reconciliation tests.
- **Important boundary:** the model currently stores bond energy only on realized graph bonds. Internal connections inside `PhysicalMaterial` have no independent energy value, so they are not separately counted as stored bond energy. That representation must be resolved before claiming the inventory covers every possible stored-material state.
- **Resolved resource-property distinction (2026-10-20):** `ResourceProperties::potential_energy` is immutable per-type maximum potential, not an amount of energy currently held by every instance. It must not be summed into the world inventory merely because a material exists; doing so would duplicate a type-level ceiling as stored energy. COMBINE/decomposition comments correctly state that constituent potential is not consumed on bond formation. The live inventory's current structural store is realized `bond_energy`; physical-material internal bonds currently derive release potential on break rather than retaining a stored numeric energy. Keep this as a representation boundary: if material bonds become explicit energy stores later, define their allocation/formation and inventory them exactly once before using them as a source.
- **Resolved stored-material BREAK accounting audit (2026-10-20):** live COMBINE debits usable energy and allocates recorded energy to realized rigid bonds or dissipates the transient fluid-bond allocation as heat. `PhysicalMaterialBond` retains endpoints but no `bond_energy`, and no energy is allocated when those pre-existing internal bonds enter storage. Stored-material BREAK therefore validates and splits the physical composite while settling a zero-energy transaction; it does not derive or credit bond potential from constituent properties. The regression `stored_material_break_does_not_credit_unallocated_bond_energy` passes. Only realized graph bonds with recorded `bond_energy` release energy on BREAK. This closes the identified free-energy path, but does not by itself prove whole-world conservation.
- **Death/decomposition architecture correction (2026-10-20):** the simulation tick must not call `process_decomposing_bodies` as a special chemistry pathway. A breached organism remains as a physical structure and continues through `chemical_reaction::accumulate`/`resolve`, the same reaction pass used by living organisms. “Decomposition” describes the chemistry observed after death; it does not define a separate bond-breaking law. The legacy `DecomposingBody` path is retained only as staged code pending a shared BREAK settlement audit, and the tick-level regression that incorrectly asserted it should advance independently has been removed.
- **Verification status superseded by the current audit at the top of this README:** exact-head Rust tests have now run, including the stored-material BREAK regression. Whole-world conservation across all chemical transitions remains unverified, and live sunlight capture remains disabled.

## World energy inventory reconciliation primitive — 2026-10-20

- Added `EnergyInventorySnapshot` in `src/chemistry/energy_ledger.rs`. It expresses the world-scale invariant: change in usable energy + change in stored structural/bond energy + work + heat = external input + explicitly non-structural released energy.
- Added focused unit tests for a balanced external-input case, a bond-breakdown case where stored bond energy decreases exactly once, and unbalanced/non-finite snapshots.
- Internal transfers are intentionally absent from this world-level equation because they cancel between holders. Bond breakdown belongs in the structural-energy decrease, not in `other_energy_released`.
- **Scope limit:** this is a tested-by-design reconciliation primitive, not yet a live whole-world inventory. The simulation does not yet independently compute tick-zero/current structural bond-energy totals and pass them through this snapshot. Tests were added but have not been run; no successful exact-head CI result is available yet. Do not treat this commit as proof of runtime conservation or functional sunlight capture.

## Unified resonance and reflected-tone energy — 2026-10-19

**Decision confirmed by the user:** an interaction may change the frequency composition of the reflected tone. Reflection is not energy-costly tone emission; it is the onward propagation of the energy left from an incoming tone. The reflected tone must be less energetic than its source interaction after capture and losses.

- Reuse the existing harmonic/resonance response rules for incoming tones and reflected tones. Do not create a solar-only reflection system or a second aura propagation mechanism.
- Material resonance and nonlinear response may reweight frequencies and produce harmonic components in the reflected spectrum. The reflected spectrum may therefore differ from the incoming spectrum.
- Keep the spectrum as a signal representation and track its physical energy budget separately. Spectral amplitude or `ResonancePerception.magnitude` is not, by itself, captured energy.
- For each interaction, enforce: incoming energy received = usable energy captured + energy reflected onward + energy lost. Any frequency transformation must remain bounded by the energy available for reflection; it cannot create energy.
- A downstream organism can harvest only reflected energy that actually reaches it. Directional propagation, attenuation, and obstruction must apply consistently to direct and reflected tones.
- The existing `material_response_to_drive` and `nonlinear_harmonic_amplitude` functions provide a starting point for spectral transformation, but the current world-tone path only computes perception. It does not create an energy-bearing reflected signal or perform capture.
- The existing external-input ledger validates one transaction against a caller-supplied availability value. It does not yet allocate a shared per-tick source budget across receivers or prevent repeated claims against the same budget.
- **Capture-model progress — 2026-10-19:** `src/perception/harmonics.rs` now defines a bounded, dimensionless damped-oscillator absorption fraction, `4ζ²r² / ((1−r²)² + 4ζ²r²)`, with `r = drive_frequency / natural_frequency`. It uses the existing damping/frequency model, is constrained to [0, 1], and is covered by unit tests for resonance, off-resonance response, and invalid inputs. This is a physical absorption-response candidate, not spectral amplitude, source energy, or an implemented capture path. It is not yet wired into live energy capture; spatial receiver weighting, shared source-budget allocation, reflection, and end-to-end conservation remain open implementation gates. Tests have been added but not run.

**Directional interception model — clarified 2026-10-19:** the world tone emits energy and signal directionally from its current rotating source position, analogous to light. Use simplified direct-path visibility rather than detailed photon/ray simulation: a clear path exposes a surface; an occluded path puts it in shadow. Only source-facing, unoccluded surfaces intercept incoming energy/signal. That surface's realized material and geometry determine inward propagation toward the genome cavity, where resonance/damping can determine absorbed energy. Do not give the entire organism an independent copy of source output, and do not equate perceived harmonic amplitude with energy. The energy ledger must credit absorbed energy once; energy not intercepted/absorbed remains uncaptured or follows an explicitly modeled reflection/loss path. Shadowing and surface exposure define the spatial capture pool, replacing the prior unresolved abstract receiver-pool framing. Exact discretization of visibility/occlusion and accounting for intercepted-but-not-absorbed energy still need implementation and tests.

**Implementation gate — updated 2026-10-19:** the bounded resonance-to-absorption response exists, but its focused tests have not been run. A geometry-level `source_path_blocked` primitive in `src/geometry/organism_geometry.rs` checks direct segments against realized constituent geometry (including circles, polygon edges, lines, and explicit fluid boundaries) without raster sampling. It handles collinear boundary overlap and excludes contact only at the destination endpoint. The `visible_source_facing_points` query reuses `surface_geometry::boundary_point_toward` to derive a world-space boundary point and outward normal for each constituent, rejects back-facing candidates, and filters points shadowed by other realized material. Those probes are useful for diagnostics but are not a surface-area or energy-interception calculation, and they are not called by simulation.

**Polygonal exposure discovery — 2026-10-19:** `visible_polygonal_intervals` now derives straight exposed intervals for polygonal constituents by splitting target edges at source rays through blocker vertices and classifying each interval against realized-geometry occlusion. Unsupported curved, line-only, or implicit boundaries fail closed instead of being approximated. The helper is geometry-only; it does not yet calculate curved-boundary exposure or credit energy. A sign correction to the analytic ray/edge parameter was applied during review. Focused regression tests now cover an unobstructed target edge, a partially shadowed target edge, a fully occluded target edge, and fail-closed handling of non-finite realized geometry. The helper rejects non-finite part transforms and transformed vertices before intersection math. Exact-head Rust verification is still required before relying on this path. **CI evidence audit — 2026-10-19:** the latest discoverable Rust workflow run is [#39300799075](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39300799075), checked out commit `3369b71dfaf9972fb626edb74d5e04bf68402127`, not the current PR head `f518f2daf399082bfa83efeac9d8ce8b2f0d15da`. That older run reports 412 passed, 30 failed, 1 ignored in the full test suite; Clippy also failed with 62 lint errors. The failing tests span existing constructor, geometry, cavity, harmonics, movement, reproduction, and simulation behavior, so they cannot be attributed to the new exposure tests from that run. This run predates the latest exposure-test commits and does not verify them. No successful exact-head result is available.

Energy must enter the ledger exactly once through the external-input transaction. Intercepted, absorbed, reflected, and otherwise uncaptured energy must remain distinct; raw spectral amplitude is not energy, and no arbitrary reflection/loss constants should be introduced. The new primitive and tests have not been compiled or run. Live capture, complete exposed-surface integration, whole-world conservation, and end-to-end verification remain unverified. Keep this work in PR #202; leave constructor/Bob PR #196 untouched.

## Single working authority and directory boundaries

This root `README.md` is the **only authoritative working README** for EvoSim. Subsystem READMEs are being retired; durable decisions, current status, test gates, and operational instructions belong here. Dated audit documents under `docs/` may hold detailed evidence, but they are supporting records—not competing authorities. When a detail conflicts, this README's latest explicit decision/status wins, and stale history must be labelled historical.

## Energy-system boundary and constructor baseline — 2026-10-18

This section records the read-only audit used to decide where the global energy and natural-chemistry work belongs. No source code or tests were changed/run for this audit.

### Constructor PR #196 baseline

- PR [#196](https://github.com/coringilbert16-cmyk/EvoSim-/pull/196) remains open and draft-only. Head: `baa0e2702bcb2223da3e326d9f67833c8a6d5c85` (`refactor/constructor-strip-2026-10-12`). It is a separate constructor/Bob qualifying-cavity workstream; do not use it as the energy-system base.
- Exact-head run [#38953513843](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38953513843): formatting and compilation passed; focused Bob compact-storage, persistence/reopen, fluid transaction/coalescence/restoration, chemical-BREAK, persisted-pair replay, and genome-cavity checks passed; bounded Bob generation completed; Clippy failed. The broad Rust suite was deliberately held during constructor calibration, so this run does not establish full-suite or constructor viability.
- The PR description's 338-candidate Bob result is the pre-round-robin scheduling checkpoint, not verification of the newer mutation scheduler. Constructor viability remains unresolved. Do not merge, weaken invariants, or infer life from a qualifying cavity.
- This baseline is a status snapshot, not a claim that the current head has green CI.

### Integration PR #186 audit

- PR [#186](https://github.com/coringilbert16-cmyk/EvoSim-/pull/186) remains the sole integration target and draft-only. The audited code head was `cf773b0a9784f632c0c8bf6f9cb7ae7dde7e44b9` on `integration/unified-source-and-catalogue`; this audit then added only this README record at `c9378caf6c9affc121d296f78db4bc4181a0cc82`. Base: `main`. Its tree is the current runnable source integration, while `main` is not yet a complete runnable checkout.
- Exact-head Rust workflow [#38727874484](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38727874484) failed overall: formatting, source-size, COMBINE architecture, focused Bob, fluid transaction/coalescence/restoration, and chemical-BREAK steps passed; the broad Rust tests and Clippy failed. Runner validation [#38727874795](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38727874795) passed. Do not describe #186 as green.
- The existing `src/chemistry/energy_ledger.rs` is a staged transaction API, not yet a complete simulation-wide energy inventory. It balances individual transactions and can transfer between numeric holders, but a transaction's `potential_released` can be accepted without debiting a named source. The aggregate counters do not yet account for every energy holder globally.
- `src/chemistry/chemical_reaction.rs` accumulates a reaction-driving quantity from chemistry/contact and can emit a chemical-BREAK event. That accumulator is not itself energy and currently has no explicit energy-source debit. The 440 Hz harmonic model in `src/perception/harmonics.rs` is a perception/resonance model, not an implemented physical energy-capture transaction.
- Related holders and event paths include COMBINE and both BREAK pathways, organism usable energy, structural bond energy, maintenance, movement, reproduction/developing offspring, decomposing bodies, environmental material, initial bootstrap energy, heat/dissipation, and lifecycle transfers. These require one conservation audit rather than isolated fixes.

### Implementation boundary decision

- Do not implement this work on #196. Keep #196 constructor-only and preserve its baseline.
- Do not resume old energy branches as implementation bases: `energy-architecture` is 5 commits ahead but 4,657 behind/diverged from the current integration branch; `ci/unified-energy-ledger` is 19 ahead but 3,664 behind/diverged; `energy-model-implementation` is also substantially behind. They may be mined for ideas only after hunk-level review.
- When implementation is authorized, create **one dedicated energy-and-natural-chemistry PR** from the verified current #186 integration head. Keep it as one workstream with staged commits, not a PR for every subsystem. It will necessarily touch shared files such as the ledger/state, COMBINE/BREAK, harmonics, simulation, movement, decomposition, and reproduction; keep those changes isolated from #196 and reconcile overlapping hunks deliberately later.
- First implementation gate: specify the world source, every holder, and source-to-destination conservation rules. The working recommendation is an omnipresent 440 Hz reference signal with a separately tracked, externally maintained energy source and finite/rate-limited local capture; resonance changes coupling rate, not the amount of energy available for free. Whether local environmental stores deplete and replenish remains a design decision to settle before coding.
- Then implement the global accounting authority and atomic settlement; connect existing physical/chemistry events; separately connect tone capture; audit initialization and lifecycle transfers; finally test whole-world conservation and repeated chemistry cycles. The reaction accumulator remains a reaction-driving quantity, never an implicit energy reservoir.

## PR audit and consolidation — 2026-10-13

### Stack reconciliation — 2026-10-13

PR [#200](https://github.com/coringilbert16-cmyk/EvoSim-/pull/200) is merged into the fluid-coalescence branch at `412a2e8b795e9a2b107455eb899a25cba1a6a4ec`. The merge has now been carried into the downstream stack through two explicit merge commits:

- #194 head: [`af223a90`](https://github.com/coringilbert16-cmyk/EvoSim-/commit/af223a901404b61bc4ad6de1eeebead4db0fd8fa), which corrects the experiment workflow trigger and pins its pre-sulfur baseline.
- #195: [`3418cc0c`](https://github.com/coringilbert16-cmyk/EvoSim-/commit/3418cc0c251162d4cd9e508ed9aecf8a81a13ba0), with its prior transactional-chemistry head as first parent and updated #194 as second parent.
- #196: [`d4f81741`](https://github.com/coringilbert16-cmyk/EvoSim-/commit/d4f817414fa7b2c8b8cd2dddc50994e05720f966), with the prior constructor head as first parent and reconciled #195 as second parent. The constructor changes remain intact; overlapping resource/Bob/README changes were merged selectively rather than overwritten wholesale.

**Verification status:**

- Reconciled #195 full Rust suite: **405 passed, 28 failed, 1 ignored**. Its 28 failing test names match the previous #195 run exactly; no new named failures. Focused Bob/fluid/chemistry gates passed. Strict Clippy still fails.
- Valid pre-merge sulfur experiment [run #30](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38474765699) used baseline `b3bc66eb`: 264 baseline formations / 46,112,052 bytes versus 281 parallelogram formations / 47,271,092 bytes; all 15 multi-contact fluid formations were retained and both catalogues reopened with no oversized formations or unfinished frontiers. This is a coverage signal, not proof of constructor recovery.
- Follow-up [run #31](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38475883030) was not a valid new comparison: its baseline checkout was `412a2e8b`, already containing sulfur, so both sides produced 281 formations and 47,271,092 bytes. The workflow is corrected to pin the pre-sulfur baseline and run only for the dedicated sulfur experiment branch; it is skipped on #195.
- Exact-head #194 run [#7382](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38476788537): **403 passed, 28 failed, 1 ignored**, with the same 28 failing names as the prior #194 run. Focused gates and formatting pass; Clippy fails.
- Exact-head #195 run [#7383](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38476821937): **405 passed, 28 failed, 1 ignored**, with the same 28 failing names as the prior #195 run. Focused Bob/fluid/chemistry gates and formatting pass; Clippy fails. The sulfur experiment workflow is skipped on this downstream PR as intended.
- Exact-head #196 run [#7384](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38476838600): **367 passed, 69 failed, 1 ignored**, with the same 69 failing names as the pre-merge run. Sulfur geometry, Bob-signature, and chemistry-cache tests pass; formatting passes, Clippy fails. These are no-regression results, not constructor recovery.
- Integration-head run [#7388](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38477522294) reports **405 passed, 28 failed, 1 ignored**; its 28 failing test names exactly match reconciled #195, with no additional named failures. Formatting and focused Bob/fluid/chemistry/sulfur tests pass; strict Clippy remains red. Windows runner validation passed.
- The reconciled #193 → #195 Bob/fluid/chemistry stack is now integrated into #186 at merge commit [`40201f62`](https://github.com/coringilbert16-cmyk/EvoSim-/commit/40201f6282f58f6b08c0810c47ee1cae55b87483); constructor PR #196 remains separate. Keep #186 draft-only. Do not promote to `main`, start broad catalogue generation, or weaken physical invariants.

The dated audit sections below are retained as historical evidence where they predate this reconciliation. This current section supersedes their earlier statement that #200 was excluded and their earlier description of #195/#196 as un-reconciled.

## Exact-head follow-up — 2026-10-13

Historical exact-head Rust results before the sulfur merge were **#193: 377 passed / 28 failed / 1 ignored** ([run #7143](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38270282801)); **#194: 400 / 28 / 1** ([run #7144](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38270352595)); and **#195: 402 / 28 / 1** ([run #7148](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38272687270)). After #200 merged into #194, the latest #194 run reports **403 passed / 28 failed / 1 ignored** ([run #7373](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38474817384)). Strict Clippy fails. Focused gates passing and a stable count of 28 full-suite failures are not a green build or merge approval.

The #193 source review found its multi-contact transaction spans construction runtime, contact endpoint discovery, structure endpoint mapping, and fluid–fluid candidate eligibility; those parts should not be separated casually. Its juvenile seed-ring/chain calibration and constructor's fail-closed behavior when the persistent Bob library cannot open are separate concerns. Because #196 is stacked downstream and edits overlapping files, no blind revert or file-level split is safe. Keep the stack draft and unchanged until a commit/hunk dependency map and targeted regression plan prove a clean split. No source code changed in this follow-up.


Current PR evidence and the consolidation map are recorded in [docs/pr-audit-consolidation-2026-10-13.md](docs/pr-audit-consolidation-2026-10-13.md). This current status supersedes older PR-status snapshots, while retaining their useful lineage history.

- Recent stacked Bob/fluid/chemistry focused gates pass, including tested chemistry-library rollback/concurrent-writer cases. The recorded full Rust suite still has 28 failures (402 passed, 28 failed, 1 ignored); Clippy remains red.
- Constructor PR #196 is not healthy: latest run [#7338](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38438644278) reports 364 passed, 69 failed, 1 ignored; Rust tests and Clippy fail. Freeze constructor changes until the earlier 28-failure comparison point is reproduced and the regression boundary is isolated.
- PR #190 has unique partial viability-contract changes not yet verified in the production constructor. Consolidate them deliberately into #196; do not develop two constructor PRs in parallel.
- Keep #193 → #194 → #195 stacked. Keep #186 as the sole integration target and draft-only. Keep #179 isolated as a practice-only storage experiment.
- PR #200 (sulfur parallelogram) is now merged into #194 at `412a2e8b`; the post-merge stack and integration branch still need ancestry reconciliation and exact-head verification.

### Source and data ownership

- `src/`: executable simulation, physical geometry, chemistry, organism lifecycle, construction, Bob, and tests. Live physical/energy/chemistry transactions remain authoritative.
- `src/bob/library.rs` and `src/bob/worker.rs`: geometry-library schema, validation, persistence, lookup, and generation. Bob records reusable knowledge; a record never authorizes a live bond or bypasses validation.
- `geometry_library/data/`: generated geometry state. It is intentionally absent from Git. Use isolated output roots for generation and keep generated artifacts out of commits unless explicitly approved.
- `src/chemistry/`: executable chemistry rules and transactions. `chemistry_library/data/` contains a small schema-3 interface cache (17 records at the last inventory), not the chemistry ruleset; the live chemistry implementation remains authoritative.
- `src/infrastructure/library_sync.rs`: checkpoint publication infrastructure, not part of the physical model.
- `src/` currently contains contract/unit tests that exercise private crate internals. Do not move them into external integration-test targets without an explicit API-boundary plan.
- Hydrogen is a finite-area rigid rectangle (1.0 × 0.1 units) with two primary end contact points. It is not a zero-thickness line.
- Bob's 0.5-unit positional equivalence is a **catalogue deduplication** tolerance only. It does not replace the live geometry/contact tolerance or permit penetration.
- **Hard catalogue size rule (updated 2026-10-12):** Bob may persist singleton, two-piece, and three-piece rigid formations (maximum `constituents.len() == 3`). The worker must reject configured limits above 3 and never expand beyond three pieces. Multi-contact fluid geometry may contact multiple surfaces of a parent formation; contact count is not constituent count.
- Generated geometry stays outside Git. The last measured two-piece baseline was about 48.3 MB uncompressed; three-piece size is unknown and may grow substantially. Measure counts, runtime, and storage before treating a three-piece catalogue as complete.
- Fluid knowledge is stored as symbolic capillary boundary/family records with conserved area and contact constraints—not sampled rigid poses. Current records have persistence validation and live-interface resolution; the active work is converting those records into a reproducible, area-preserving realized fluid boundary and proving it through physical validation.

### Sulfur geometry — parallelogram replacement

Sulfur's default rigid outline is a four-vertex parallelogram centered at the origin, with cyclic edge lengths **1.0, 0.5, 1.0, 0.5**, interior angles alternating **60° and 120°**, and opposite edges parallel. Its area is \\(\\sqrt{3}/4 \\approx 0.4330127\\) square units and its perimeter is 3.0 units. Canonical vertices:

```text
(-0.625, -sqrt(3)/8)
( 0.375, -sqrt(3)/8)
( 0.625,  sqrt(3)/8)
(-0.375,  sqrt(3)/8)
```

The implementation stores equivalent decimal coordinates. Keep the centroid and bounding-box center at the origin. Do not substitute a rhombus or regular polygon: the two long edges and two short edges are intentional.

This is a **geometry-only replacement**. Sulfur's mass (1.50), potential-energy capacity (8.0), reactivity (2.0), chemical position (3.5), cohesion (0.45), and rigid physical state remain unchanged. The smaller occupied area relative to the previous sulfur geometry is an expected consequence of the selected geometry; do not silently change unrelated properties to compensate.

Construction-material similarity treats `Form::RegularPolygon` and `Form::Polygon` as one broad polygon family. This avoids an artificial material-selection penalty caused solely by switching sulfur's representation; exact vertices remain authoritative for placement and contact. Bob's catalogue signature includes serialized resource shapes, so the previous-shape catalogue is invalidated and must be regenerated in an isolated output directory. Generated catalogue data must stay out of Git. The chemistry interface cache remains keyed by material pair and interface signature; the focused regression confirms the existing Hydrogen–Sulfur interface result is unchanged.

#### Replacement verification checkpoint — 2026-10-13

- Sulfur geometry assertions cover four vertices, alternating 1.0/0.5 edge lengths, parallel opposite edges, 60°/120° interior angles, centering, area, and unchanged non-geometric properties.
- Bob's catalogue signature regression confirms the shape change produces a distinct catalogue signature and prevents reuse of stale geometry records.
- Formatting passed. Valid pre-merge comparison [run #30](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38474765699) measured 264 baseline formations versus 281 parallelogram formations; both catalogues reopened, all 15 multi-contact fluid formations were retained, and no oversized formations or unfinished frontiers remained. Follow-up [run #31](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38475883030) was not a valid comparison because its baseline already contained sulfur. The workflow is now pinned to the pre-sulfur baseline and scoped to the dedicated experiment PR. Do not interpret formation count alone as proof of improved multi-piece fit.
- Isolated Bob generation completed for both variants with the same two-constituent limit, no oversized formations, no unfinished frontiers, and successful catalogue reopen. The parallelogram catalogue recorded 281 formations versus 264 baseline (+17); this is a useful coverage signal, not proof that the constructor is fixed.
- The measured run used an isolated temporary library. Do not commit its generated catalogue or treat the raw record-count difference as a normalized geometry-fit score.

This replacement is intended to improve geometric compatibility, not to claim that sulfur alone resolves multi-piece constructor failures. Keep live geometry/contact validation authoritative.

### Operational gates

1. Keep the geometry catalogue empty in the tracked tree. Never run the continuous worker or use the default catalogue directory for tests.
2. The focused persisted-fluid/query and live transactional-bond gates now pass on integration head [#7390](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38477917672). Three-piece generation may proceed only through the manual gated workflow; keep unsupported contact topologies fail-closed and treat generated records as coverage evidence, not proof that every topology realizes.
3. Next: run `.github/workflows/bob-three-piece-generation.yml` manually on `integration/unified-source-and-catalogue`. It writes only to an isolated output root outside Git. Report formation/family counts, runtime, storage size, frontier completion, and reopen verification; never commit the generated catalogue.
4. Do not divert the active geometry-library/fluid-formation sprint into the unrelated constructor-test backlog. Keep those failures visible, but prioritize fluid-record realization, interface-class coverage, and Bob's persisted-knowledge contract; never disable tests or weaken physical invariants.
5. Keep PR #186 as the single integration target. Do not merge or change the default branch as a side effect of cleanup.
6. All new decisions and handoffs go in this root README. Before adding a new audit note, update the current summary and avoid repeating obsolete checkpoints.

## Bob catalogue cap change — 2026-10-12

- Decision: raise the maximum persisted rigid formation size from 2 to 3 constituents. This changes the cap only; it does not authorize 4+ piece records or put generated catalogue data in Git.
- Worker cap, error messages, and the targeted regression are verified on exact integration head run [#7390](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38477917672): the worker accepts a three-piece limit, does not expand the three-piece frontier to four, and rejects configured limits above 3.
- The manual workflow `.github/workflows/bob-three-piece-generation.yml` runs the worker-cap regression and persisted-fluid-to-live-transaction regression before generation. Both focused tests passed on exact integration head #7390. The workflow is now eligible to run manually; no three-piece catalogue has yet been generated or measured.
- No three-piece catalogue has been generated or measured yet. The existing 48.3 MB two-piece artifact is only a baseline, not a reliable estimate for three pieces.

## Latest Bob fluid-runtime regression status — 2026-10-11

- Exact-head CI for checkpoint `b0843f0f` passed formatting, source-size policy, COMBINE architecture, Bob compact-storage, empty-library reopen, focused chemical BREAK tests, and Windows runner validation.
- The focused library regression `two_piece_multi_contact_record_survives_reopen_and_realizes_exact_area` passed.
- Runtime regression `reopened_two_piece_bob_record_maps_into_transformed_live_parent` initially failed because its assertion incorrectly expected the fluid origin to equal the rigid parent's center. The capillary solution offsets the fluid origin from that center; this was a test expectation defect, not proof of a mapping defect.
- Commit `9d8c2534` corrected the pose assertion to compare against the exact canonical fluid poses transformed into the live frame. Follow-up commit `ba35386e` extends that same regression through physical restoration and the staged multi-contact bond helper: the persisted/reopened record must produce both fluid-to-rigid bonds on a trial structure while leaving the source structure unchanged.
- Neither follow-up commit has exact-head CI results available yet. The latest completed full suite at `b0843f0f` was red (372 passed, 29 failed, 1 ignored), including the original runtime pose assertion plus broad constructor, cavity/genome, movement/contact, reproduction, COMBINE, and simulation failures. Clippy remains red on existing lint debt. Do not conflate these with the focused Bob persistence checks.
- Source-audited limitation for the next short sprint: `form_remaining_multi_contact_bonds` currently requires the number of distinct contacted rigid units to equal `boundary.contact_count()`, then forms one bond per target unit. This safely rejects a record with multiple contact segments on one rigid unit, but it does not yet support every valid multi-contact topology. Preserve fail-closed behavior; next, map each solid-contact segment to its live rigid edge/endpoint and transact the required bonds per contact, including multiple contacts on one unit. Add a regression for that topology before claiming general multi-connection support.
- Update 2026-10-13: the latest integration head's focused runtime gates passed, including `reopened_two_piece_bob_record_maps_into_transformed_live_parent`, the multi-contact fluid bond tests, coalescence/restoration tests, and the worker cap. The earlier immediate gate is satisfied for the supported path. Run the manual gated three-piece workflow next; keep unsupported multi-contact topologies fail-closed and audit the resulting artifact before drawing conclusions.

## Current repository integrity checkpoint — 2026-10-11

This checkpoint supersedes older repository-status and CI summaries later in this README. Those notes are retained as dated history, not as the current branch state.

### Authoritative branch state

- GitHub default branch `main` is **not a runnable EvoSim checkout**. Its current head is `a30c14999a95f97a644f9927e8eb4546c57e285a`; its six tree entries contain only the chemistry-library data and `docs/bob-geometry-storage-audit.md`. It has no root `README.md`, `Cargo.toml`, `Cargo.lock`, Rust source, UI, or runner scripts.
- The single integration target remains [PR #186](https://github.com/coringilbert16-cmyk/EvoSim-/pull/186), branch `integration/unified-source-and-catalogue`. Its tree contains the Rust application, UI, scripts, README, audits, geometry-library implementation/documentation, chemistry data, and a tracked `Cargo.lock`.
- The seven generated geometry data files were intentionally removed from both current branch trees. **Do not restore or commit generated geometry data as part of repository recovery.** The bounded two-piece generation run produced an artifact only; it did not repopulate the Git tree. The chemistry cache files remain tracked.
- Branch ancestry has been reconciled with current `main` by merge commit [`7041785`](https://github.com/coringilbert16-cmyk/EvoSim-/commit/7041785ded5efb6d91b3fae1f663aada38a27324). Current `main` is an ancestor of the integration branch; no force-push or history rewrite was used.
- `Cargo.lock` was generated by Cargo and is now tracked. CI and the persistent Windows runner use `--locked` for builds/tests and Clippy to prevent silent dependency resolution changes.

### Fluid-geometry lookup implementation — 2026-10-11

- Bob now has a `LiveGeometryQuery::FluidBoundary` identity containing fluid/solid material, solid edge, realized edge parameter, conserved fluid area, and capillary contact angle.
- **Water-inclusive formation implementation:** Bob now derives and persists explicit `GeometryFluidFormation` sidecar records from solved fluid-boundary families, including parent formation, Water identity, conserved area, and capillary boundary parameters. This is an initial implementation, not yet verified coverage. Exact-head CI and persistence/reopen tests must pass before broad catalogue regeneration; see the central **Geometry Reference Library** and **Bob-to-construction consumer audit** sections in this README.
- **Fluid-frontier diagnostics follow-up:** the worker counts Water frontiers that yield no boundary candidates and now reports per-anchor reason counters for unsupported anchor geometry, capillary-solver failures, anchors without exposed intervals, invalid contact intervals, and exposed intervals without valid capillary overlap. Regression tests cover a valid Water/Carbon solution, unsupported-anchor reporting, persistence/reopen, and duplicate-free resumption. These counters do not establish complete fluid coverage; broad catalogue regeneration remains gated on exact-head CI and physical validation.
- `resolve_live_contact_candidate` builds this query from the realized fluid endpoint and solid boundary; `GeometryLibrary::resolve_persistent_interface` now matches it against persisted fluid-boundary families and returns unresolved/unique/ambiguous based on canonical local-interface projections.
- Equivalent local fluid interfaces are projected independently of their parent formation signature so the same reusable relationship in multiple formations does not create a false ambiguity.
- Added a regression test that persists a fluid-boundary family, resolves it, reopens the library, and resolves it again.
- **Verification status at the preceding checkpoint:** CI on `354f284d1a4cd7f54f54ee3f6a1bb82be215b53a` passed formatting, source-size, COMBINE architecture, focused compact-storage tests (5 passed), and empty-library open/reopen. The full suite reported 316 passed, 29 failed, 1 ignored; strict Clippy failed on existing lint/staged-API issues. The reason-level diagnostic change is newer and requires its own exact-head CI before it is considered verified.
- **Still outstanding:** broader fluid-geometry discovery coverage (including applicable fluid–fluid cases), explicit coverage metrics by fluid interface class, and strict end-to-end constructor use of the persisted fluid query. This sprint closes the persistent lookup gap, not the full fluid-geometry requirement.

### Active work handoff — Bob fluid formations and geometry realization — 2026-10-11

This handoff supersedes the seed-ring debugging detour. Do not spend the active sprint chasing the element-9/element-20 constructor failures; those changes were made on a separate branch and are not part of the integration target.

**What is already present in the integration source:**
- The capillary solver calculates the 2-D Young–Laplace equilibrium against a flat rigid boundary from conserved area and effective contact angle, returning curvature radius, free-arc angle, contact length, and enclosed area.
- Bob's worker derives and persists `GeometryFluidBoundaryFamily` and `GeometryFluidFormation` sidecar records, including parent formation, Water identity, anchor constituent/edge, area, contact angle, curvature, contact length, and a feasible normalized edge-parameter interval.
- The live-interface identity includes fluid material, solid material/edge, realized edge parameter, area, and contact angle. Persistent lookup can resolve a matching local fluid-family projection.
- The prior bounded two-constituent run produced a temporary artifact and passed persistence/reopen and size/frontier checks. It did not prove realized fluid-boundary geometry or constructor consumption.

**Actual remaining gap (source-audited; supersedes older status text below):** persisted fluid records are symbolic constraints plus solved capillary parameters, and the code now has an exact circular-arc descriptor and a `GeometryFluidFormation::realize_against_parent` helper that reconstructs a shape and pose in the canonical parent's coordinate frame. `Form::Fluid` now carries `arc_boundary: Option<CircularArcBoundary>` alongside the legacy polygon boundary, validates arc/area consistency, and refuses to expose an arc as polygon vertices. The remaining bridge is not the existence of an arc descriptor: it is proving that persisted records survive reopen and become validated geometry in the *live* construction coordinate frame, and auditing every physical consumer for exact-arc behavior. `resolve_persistent_interface` still reports a resolution classification; callers that need a shape must separately retrieve the matching persisted `GeometryFluidFormation` and invoke the realization helper. Never substitute sampled polygon vertices for the exact arc.

**Next short sprint — fluid realization bridge:**
1. Map every consumer of `Form::Fluid` and polygon-boundary APIs: point containment, edge/contact discovery, overlap/nonpenetration, surface normals, rendering, and serialization. Define the smallest exact arc-capable boundary contract and ensure authoritative physical queries understand it; keep the capillary solver as calculation authority.

**Historical consumer audit — SUPERSEDED by the implementation checkpoint below. Do not use this table as current source status.** The table records the pre-arc implementation state and is retained only as audit history.

| Consumer | Current behavior / gap |
| --- | --- |
| `src/materials/resources.rs` — `Form`, validation, `polygon_vertices`, `bounding_radius` | Fluid is still `nominal_area + Option<Vec<Point>>`. Its boundary is treated as polygon vertices; a missing boundary falls back to an area-derived bounding radius. No exact arc descriptor exists. |
| `src/geometry/surface_geometry.rs` — `boundary_point_toward` | Fluid boundary normals and ray hits are computed by polygon edge intersections only. An absent polygon yields no boundary point. |
| `src/geometry/material_geometry.rs` — overlap, penetration, boundary contact, rigid contact | Fluid is intentionally not treated as a filled collision wall. If a polygon boundary exists, it is temporarily converted into a polygon for boundary-contact tests. Exact circular-arc distance/intersection is unsupported. |
| `src/geometry/interior_geometry.rs`, `src/geometry/cavity.rs`, `src/geometry/connection_geometry.rs`, `src/geometry/rigid_boundary.rs`, `src/geometry/contact.rs` | These APIs derive containment, exposed edges, corner alignment, contact endpoints, or feature counts from polygon vertices. They cannot preserve an arc as a first-class boundary; some explicitly exclude fluid from rigid vertex/edge features. |
| `src/geometry/organism_geometry.rs`, `src/construction/runtime.rs` | Runtime structural geometry and placement checks rely on polygon-derived geometry and special-case Fluid. This is a required integration surface, not an optional viewer-only concern. |
| `src/geometry/physical_geometry.rs`, `src/geometry/structure.rs` | Physical geometry can accept a valid legacy polygonal `Form::Fluid` replacement, but the API does not receive or validate a Bob formation record and does not guarantee conserved area/contact constraints. |
| `src/bob/library.rs`, `src/bob/worker.rs` | Bob persists symbolic `GeometryFluidBoundaryFamily` / `GeometryFluidFormation` records and resolves canonical interface projections. Those records do not currently produce an exact live-coordinate boundary consumed by the geometry APIs above. |
| `src/ui/resource_visualization.rs` and serde on resource/physical geometry | Rendering and serialization are downstream compatibility surfaces for any new descriptor. The serialized shape contract must remain backward-readable for existing `Form::Fluid` polygon/empty-boundary records. |

**Design gate from this audit:** do not retrofit arcs by returning sampled points from `polygon_vertices()`; that would make exact geometry appear polygonal to callers and silently reintroduce approximation into collision, normals, and contact. The next implementation should introduce an explicit, serializable circular-arc boundary descriptor alongside the legacy polygon boundary, then add shared exact arc primitives (world transform, signed sweep, point/ray intersection, closest-point/distance, normal, and arc/segment intersection) before wiring every physical consumer. Until those primitives are integrated, a new descriptor must fail closed in unsupported physical queries rather than be silently approximated.

**Latest verification update — 2026-10-11:** The Rust workflow for commit `8dc26def` completed with the targeted Bob compact-storage tests, empty-library reopen check, and focused chemical BREAK tests passing. The full Rust suite failed with 371 passed, 28 failed, and 1 ignored; failures span constructor placement, cavity/genome, contact normals, movement, COMBINE, and other existing contracts. Clippy also failed on broad lint debt. Keep those failures visible and do not turn this fluid sprint into wholesale constructor or lint cleanup.

**New targeted regression added, awaiting CI:** `two_piece_multi_contact_record_survives_reopen_and_realizes_exact_area` constructs a parent with exactly two rigid constituents, asks Bob's multi-contact generator for a two-surface fluid record, persists it, closes and reopens the library, then checks record identity, exact boundary realization, and conserved area. Formatting, source-size, architecture, Bob compact-storage, empty-library reopen, and focused chemical BREAK checks now pass on commit `7bb9cfbd`. During source review while the Rust suite was running, I found the new test's `Form::Fluid` pattern omitted `..` for the enum's other fields; corrected in `0ab70eaf`. That correction is a small compile-safety fix, and the new head still needs its own CI verification. Do not treat the persistence regression as passed until the focused test actually runs. The Rust job on `3cf6915a` had reached the full suite but did not return a final result during this handoff. A follow-on runtime regression was added in `e6efb9d8`: it builds a bonded two-piece line parent in a translated/rotated live frame, generates and persists a two-contact record, reopens Bob, and queries the record through `bob_fluid_origins_for_live_parent`, checking transformed pose and conserved area. Both new regressions require exact-head CI verification; the existing transaction test remains the separate proof for all-or-nothing bond formation. This regression intentionally narrows the stored parent to the catalogue's two-constituent cap, unlike the older three-parent-contact test which remains as broader solver coverage.

**Latest CI follow-up — 2026-10-11:** The first integration run's Bob compact-storage step did not reach the compact-storage assertions: compilation failed in `worker_rejects_constituent_limits_above_two` because `.expect_err(...)` imposed a `Debug` bound on successful `WorkerPassMetrics`. The test now uses `.err().expect(...)`, preserving the InvalidInput assertion without adding an irrelevant trait requirement. Commit: `8dc26def0323fb082f3107a7ff8ce8a08a6b8d46`. Runner Validation passed for that commit; the Rust workflow was still in progress at the time of this update. Treat the hard-cap regression and the full Rust workflow as unverified until that exact head completes.

**Current bridge audit:** `src/construction/runtime.rs::bob_fluid_origins_for_live_parent` already queries persisted Bob suggestions, calls `realize_against_parent`, maps the canonical pose into the live parent frame, and returns the exact fluid shape for live validation. The multi-contact realization helper refuses to downgrade a constrained record unless `solved_boundary` exists and matches the constraints. The remaining proof gap is a single regression that exercises the whole chain with a generated multi-contact formation: persist it, close/reopen the library, retrieve it through the runtime query, transform it into a non-origin/non-zero-rotation live frame, and verify the realized area, all contact targets, transactional bonds, and energy invariants. Existing runtime tests cover pieces of this path, but the current multi-contact transaction test constructs its fluid boundary directly rather than sourcing it from a reopened Bob record. Do not start catalogue generation before this regression passes.

**Implementation checkpoint — superseded by the source audit on 2026-10-11:** `CircularArcBoundary` is serializable and supplies exact segment area, arc length, chord length, endpoints, normals, transforms, ray intersections, closest-point queries, and finite arc/segment intersections. It is now wired into the legacy-compatible `Form::Fluid.arc_boundary` field, with area-consistency validation and arc-aware surface queries. `material_geometry.rs` has exact arc-to-rigid boundary-contact checks, and Bob's persisted `GeometryFluidFormation` can reconstruct an exact arc shape and canonical-parent pose. However, the geometry consumer audit is not complete: overlap/penetration deliberately treat fluid interiors as traversable, and arc boundary contact must not be mistaken for a complete proof of live nonpenetration or constructor acceptance. The persisted-record → canonical-parent shape helper exists; transformation/validation against a live multi-unit structure and actual constructor consumption remain unproven.
2. Add a typed realization result from a matching persisted Bob record: resolved live anchor identity, contact interval/position, solver inputs/results, conserved area, and the exact boundary descriptor in a defined coordinate frame. A missing record must return a coverage miss, not run an unrecorded fallback solve.
3. Add a dedicated adapter that re-bases that exact descriptor to the live anchor. Validate conserved area, contact-angle/edge constraints, and nonpenetration against the full current structure before commit. If the physical authorities cannot yet validate the exact descriptor, stop at the typed descriptor and do not pretend a sampled polygon is equivalent.
4. Add focused tests for persistence → reopen → live query → realization; translated/rotated anchor rebasing; invalid/infeasible interval rejection; area conservation; and rejection of a shape that overlaps another constituent. Verify that resolution uses persisted records and that no new record is silently written during a query.
5. Extend Bob's generation report to summarize fluid coverage by interface class and explicit rejection reason. Respect the model rule that fluid–fluid coalescence increases one fluid's volume; do not encode it as a rigid bond or duplicate fluid constituent.
6. Only after this bridge and its tests pass, rerun the bounded singleton/two-piece worker in an isolated output root, inspect coverage and artifact integrity, and preserve the generated data as a runtime artifact—not tracked source.

**Progress update — 2026-10-11, source and test audit:** source inspection found that the arc-aware `Form::Fluid` and canonical-parent realization helper were already implemented, despite older README statements saying otherwise. A focused regression was added on branch `sprint/bob-fluid-realization-reopen` to verify that a persisted fluid formation is still reconstructible as a valid exact-arc shape with finite pose after the Bob library is closed and reopened. This is a test change only until exact-head CI passes; it does not claim that live construction consumes or validates the realization. Next, trace where a fluid formation should enter the live construction transaction, then add an explicit adapter and tests for rebasing against a live parent arrangement and validation against all existing units. Keep fluid interior traversability distinct from rigid penetration semantics; do not change collision rules without tests that encode the intended fluid model.

**Out of scope for this sprint:** constructor failure reduction, arbitrary pose/tolerance changes, unbounded catalogue expansion, and committing generated geometry files. The acceptance target is a persisted Bob fluid record becoming a physically validated live fluid realization.

### Latest locked CI verification

At code head `87bda9e6f9b8670ca2885edc2c15d7c257e1ecc3`, [Rust workflow #38141026450](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38141026450) completed with failure. Formatting, source-size policy, COMBINE architecture checks, Bob compact-storage tests, and the non-generative empty-library open/reopen test passed. The full suite again reported **314 passed, 29 failed, 1 ignored**. The recurring cross-subsystem seed-realization failures and other contract failures are recorded below; strict Clippy failed with 48 test-target and 58 application-target diagnostics. [Runner validation #38141026451](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38141026451) passed.

### Remaining repository-integrity gates

1. Keep PR #186 as the single integration target; do not merge or switch the default branch silently.
2. Preserve the reconciled ancestry and intended tree: application source, UI, scripts, documentation, geometry implementation, chemistry data, and no generated geometry catalogue.
3. Keep the README, PR description, and active subsystem handoffs consistent with the current tree and CI evidence.
4. Verify a fresh checkout with the committed lockfile and `--locked` dependency resolution.
5. Confirm there are no tracked build outputs, local runner state, accidental generated data, duplicate executable sources, or missing essential project files.
6. Treat the remaining test and Clippy failures as explicit blockers to an application release; do not hide them with blanket suppressions.

Repository integrity and application correctness are separate gates. This branch is now structurally unified and dependency-pinned, but the application is not yet release-ready.

---



EvoSim is an open-ended evolutionary organism simulation. Organisms are built from physical material and physical bonds; structure and behavior should emerge from general mechanisms rather than fixed predator/prey roles or a predefined organism-size progression.

## Historical repository unification gate — 2026-10-10 (superseded by the checkpoint above)

> [!CAUTION]
> **Do not treat the current default branch (`main`) as a downloadable EvoSim source checkout yet.** A recursive GitHub tree audit found only 15 entries, rooted under `chemistry_library/`, `docs/`, and `geometry_library/`; it has no `Cargo.toml` or `src/main.rs`. It is currently a data/documentation branch, not a complete runnable Rust application.

The current unified integration target is [PR #186 — Integrate EvoSim source and Bob regeneration into main](https://github.com/coringilbert16-cmyk/EvoSim-/pull/186), branch `integration/unified-source-and-catalogue`. It restores the runnable Rust source, UI, scripts, and current project documentation. The obsolete generated geometry catalogue has since been deliberately removed from both `main` and this integration branch; geometry knowledge must be regenerated only after the generator/constructor path is validated. It remains draft-only.

### Current stabilization evidence

- The integration-branch Rust workflow [run 38055388755](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38055388755) passed formatting, source-file-size, COMBINE architecture, focused Bob compact-storage tests (4 passed), and the fresh v3 library create/reopen check. [Runner validation run 38055388762](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38055388762) passed.
- The same Rust workflow is **not green**: `cargo test --all-targets` reported **253 passed, 75 failed, 1 ignored**. Failures cross geometry/contact, cavity/genome qualification, constructor/acquisition, chemistry/COMBINE, movement/perception, observation, viability, storage, reproduction, transformation, runtime, and simulation integration.
- Clippy also fails under `-D warnings`, including unfulfilled `dead_code` expectations and ordinary lint errors. Do not suppress these wholesale; determine which APIs need integration and which are obsolete, then fix the relevant code.
- The live initial-organism constructor remains a fixed 54-unit Carbon scaffold. It does **not** yet use Bob for free-form construction, still relies on a `1.0e12` construction-energy budget, uses an unapproved farthest-contact fallback, and does not validate the full required-resource acquisition set atomically.
- Bob's focused persistence checks prove only the tested store behavior. They do not prove the live-family query contract, compatibility of the checked-in catalogue, full catalogue-generation completion, or viable organism construction.

### Required order before the next clean download

1. Keep PR #186 as the single integration target; preserve the approved biological/physical rules and geometry-library implementation, but do not treat the deleted generated catalogue as source of truth.
2. Resolve Clippy/build hygiene and shared physical-contract failures at their common authority, not by patching failing tests independently or weakening invariants.
3. Finish Bob's live-family resolution and define corruption/duplicate/manifest recovery for persistent geometry and chemistry data.
4. Replace the fixed scaffold with the approved milestone-driven constructor, including whole-structure validation, atomic transactions, analyzer-confirmed bonded genome cavity, physical acquisition of Water plus at least three other resources, and reconciled energy/material accounting.
5. Run the actual constructor as the acceptance test, then focused contracts and the full suite. A Bob worker smoke test alone is not acceptance.
6. Add a non-generative, isolated empty-library open contract. Source inspection indicates an empty root can be opened, but runtime behavior is not yet verified. Do not seed or run the worker just to test startup or refill the deleted files.
7. Only after these gates pass, merge the verified integration lineage and make it the default branch.

Until those gates pass, a fresh clone of `main` is not the right download target, and PR #186 remains a work-in-progress integration branch rather than a stable release. See [the full repository stabilization audit](docs/repository-stabilization-audit.md) for source-level findings, CI evidence, ordered implementation gates, and the definition of stable enough to download.

## Historical constructor and repository stabilization handoff — 2026-10-10 (superseded)

> [!WARNING]
> This is a dated handoff preserved for engineering history, not the current branch inventory. Some claims below—especially that Bob is not connected to the active constructor path and the listed CI counts—are superseded by later commits and the current checkpoint at the top of this README. Do not use this section to determine current implementation status. The **Replacement Initial Organism Constructor** plan below remains the sole approved architecture plan.

- **Current integration target:** [PR #186 — Integrate EvoSim source and Bob regeneration into main](https://github.com/coringilbert16-cmyk/EvoSim-/pull/186), draft only. The prior staged PRs #181–#185 were closed as superseded; their source commits remain in the integration lineage. Do not merge or download this as a stable release while compile/lint and constructor-viability gates remain open.
- **Current live path:** `Simulation::create_initial_organism` calls `initial_organism_constructor::construct_valid`. The live constructor is still the fixed Carbon ring/support/outer-ring baseline (54 units); it does not yet use Bob to select a free-form growth sequence.
- **Geometry library is advisory candidate knowledge, not physical authority.** Revalidate each proposed placement against the whole realized structure and current contact, nonpenetration, endpoint, bonding, and energy rules. Bob's `GEOMETRY_EQUIVALENCE_TOLERANCE = 0.5` is not the approved live-contact tolerance of `0.1`; neither permits penetration.
- **Chemistry is split across explicit authorities:** catalog material properties in `materials/resources.rs`; chemical-position interaction in `chemistry/chemistry.rs`; bond work/strength in `chemistry/combine.rs`; energy transactions in `chemistry/energy_ledger.rs`; cached static interface potentials in `chemistry/chemistry_library.rs`. Do not duplicate these equations in the constructor.
- **Known specification mismatches:** the current integration branch's Phosphorus geometry is already the approved isosceles trapezoid (bottom 1.5, sides 0.5, top 1.0); do not alter it based on older audit notes. The constructor still uses `CONSTRUCTION_ENERGY = 1.0e12` and startup turns the remainder into organism energy; the inspected bond-strength path does not yet apply the approved full/half contact-feature scaling. These remaining mismatches require deliberate, separately tested corrections—not silent audit-time parameter changes.
- **Latest completed integration-branch CI:** formatting, source-size, COMBINE architecture, focused Bob compact-storage tests, and fresh-library persistence/reopen checks passed. `cargo test --all-targets` executed and reported **253 passed, 75 failed, 1 ignored**; Clippy also fails on strict dead-code expectations and other lint diagnostics. The failures span construction/geometry, chemistry, observation, viability, reproduction, and simulation integration. Do not describe the suite as green or treat this result as a reason to change unrelated rules blindly.
- **Next implementation gate:** resolve compile and strict-lint blockers without blanket suppressions; execute focused geometry, persistence, whole-structure collision, transactional accounting, cavity, and acquisition contracts; then wire the approved milestone-driven constructor to Bob. The acceptance test is the actual constructor producing a physically viable organism, followed by the full suite.
- **Lookup status (source audit updated 2026-10-10):** `GeometryLibrary::resolve_persistent_interface` has an indexed partial resolver for rigid-edge, rigid-point, and rigid-vertex queries and distinguishes unique/ambiguous/unresolved matches. The separate `classify_live_family_resolution` helper still always returns `Unresolved`, and fluid-boundary live queries are not covered by the indexed query variants. Most importantly, the live initial constructor does not open or query Bob at all; the resolver currently provides no construction benefit.
- **Chemistry-cache integrity (focused tests verified; full suite remains red):** schema 3 is now the current format; the loader accepts schema 2 as a legacy read path without relabeling keys. Invalid records, malformed final rows, duplicate keys, unknown manifest versions, and manifest count mismatches now fail with diagnostics. The runtime chemistry key builder now uses bare resource names for one-part, one-unit base materials so keys can match the checked-in cache; mixtures and non-unit amounts remain explicit. All new persistence/key-contract tests passed in CI. The same run passed formatting, source-size, COMBINE architecture, focused Bob storage, and fresh-library reopen checks, but the full suite remains at **264 passed, 75 failed, 1 ignored**, and strict Clippy fails.
- **Acquisition-set gap:** the fixed constructor chooses each resource placement separately. It does not validate the complete set together in one trial state; the replacement must do so before committing any acquisition.
- **Bond-ranking gap:** the fixed scaffold fallback chooses the farthest eligible contact when no corner-corner candidate is available. This undocumented rule must not be copied into the replacement without an explicit, tested score.
- **Recent commits:** chemistry schema/integrity fix and tests are recorded in [the chemistry fix](https://github.com/coringilbert16-cmyk/EvoSim-/commit/4df90c6f9a8ae01ba47cdcb14525dfa85113ddaf), [the integrity tests](https://github.com/coringilbert16-cmyk/EvoSim-/commit/e9dcefa6b2740cce3f292b168b2e2c1d8f2b7b98), [base-material key compatibility](https://github.com/coringilbert16-cmyk/EvoSim-/commit/692741c68536b4e40494d46457ed11a11ea0bfd3), and [the cache-key contract test](https://github.com/coringilbert16-cmyk/EvoSim-/commit/214a9b3ebb17c95e8e89d2605759cde09d0b0645). Subsequent formatting corrections are committed; a passing verification run for the current branch head is still outstanding. See `docs/constructor-library-dependency-audit.md` for the dependency map, contract observations, blockers, test evidence, and removal checklist.


> [!IMPORTANT]
> ## Authoritative constructor plan — replacement architecture
>
> **The “Replacement Initial Organism Constructor” plan below is the only current constructor architecture plan. Every earlier constructor plan or proposed construction workflow in this README is obsolete and must not be used to guide implementation.** Historical descriptions are retained only to explain the repository's migration history. They are not competing requirements.
>
> The replacement is a library-driven, milestone-based physical growth system. It uses the Geometry Reference Library and Chemistry Library as reusable physical knowledge, validates every candidate through authoritative runtime rules, commits physical changes atomically, discovers the genome from the realized bonded cavity, and stops construction when actual viability requirements are met. It does not build to a fixed piece count or prescribe a final topology.
>
> This document records the approved design, not completed implementation. Source inspection, successful compilation, and end-to-end construction are separate forms of evidence and must be reported separately.

## Historical constructor implementation snapshot — not an architectural plan

> [!WARNING]
> The implementation notes in this section describe a previously inspected transitional implementation. They are historical context only, may no longer describe the current branch head, and must not be interpreted as the approved replacement design. Use the authoritative replacement plan below for all constructor work.

## Historical constructor implementation state (superseded)

The initial-organism constructor is in a **transitional validation/migration phase**.

The current implementation:

- creates a deterministic physical Carbon scaffold;
- creates each physical unit through the normal material/structure machinery;
- forms permanent connections through the normal physical bond transaction;
- uses realized geometry and the realized bond graph for cavity analysis;
- requires a qualifying genome cavity;
- requires Water plus three additional acquirable resources;
- checks acquisition through physical placement in an accessible region;
- does not use an arbitrary placement-attempt budget;
- currently realizes genesis construction material from the catalog rather than waiting on environmental inventory.

The current scaffold is **not the final constructor architecture**. It is a temporary deterministic baseline retained while the final free-form constructor is integrated and verified. The genome-phase entry point is an integration seam for that replacement; it does not make a blueprint an authoritative topology.

The present scaffold is a fixed 54-unit Carbon geometry consisting of an inner ring, six radial supports, and an outer ring. Adjacent rigid units are sealed with distinct endpoint bonds at the two ends of each shared wall segment. This scaffold is a validation baseline, not the intended final organism-construction algorithm.

## OBSOLETE — Previous Intended Constructor Architecture (No Longer Valid)

> [!CAUTION]
> **OBSOLETE PLAN — DO NOT IMPLEMENT OR EXTEND THIS PLAN.** Its forward-only/local-continuation workflow, wait-on-stall behavior, and other architecture statements are superseded by the authoritative replacement plan below. Reuse valid physical invariants only where the new plan explicitly preserves them.


The final constructor is intended to be forward-only and locally constructive:

1. start from available physical starting material;
2. form a qualifying genome cavity;
3. once the cavity qualifies as the genome, finish that construction phase;
4. continue building the rest of the organism using locally valid physical bonds;
5. require Water and at least three additional resources that the organism can physically acquire;
6. when ordinary construction temporarily lacks a required physical resource or a currently valid local continuation, wait rather than blocking the simulation or declaring permanent failure;
7. never solve the entire future organism as a global placement problem;
8. never backtrack already committed physical bonds.

A blueprint, if used, is only a preference. It must never become an authoritative future topology or placement command.

## Growth, development, and reproduction

EvoSim uses continuous physical growth rather than biological Juvenile, Adult, or Offspring lifecycle stages. There is no age-based maturation, adult-mass maturity gate, juvenile viability gate, or 40% reproductive threshold.

Growth is continuous and is derived from the organism's realized physical structure against its inherited developmental preferences. The genome's size preference produces a **preferred mass**; it is a soft developmental preference, not a maturity authority and not a requirement that the organism reach an exact mass.

A qualifying developmental realization above **0.90** makes budding available. This is an availability condition, not a lifecycle stage. Reproductive construction proceeds as a separate physical developing organism graph attached to the parent by physical contact. The developing graph has its own genome, structure, stress, energy, and material accounting. Its inherited energy allocation is represented by `reproductive_energy_allocation`.

The developing physical graph is retained because it is required to model actual budding and detachment. It is not a biological "offspring stage" label. When the physical relationship joining the developing graph to the parent is severed, graph separation can yield two independent organism graphs.

The intended reproduction model is **80/80**: the reproducing organism grows to approximately 160% of its preferred scale and physically separates into two approximately 80% organisms. The lifecycle gating and retired 40% rule have been removed, but the complete 160%-then-split physical implementation is **not yet complete**. The current developing construction uses an 80% preferred-scale readiness condition; that condition must not be confused with the finished 160%-then-split model.

Construction waiting is a physical construction state, not a lifecycle state. The construction runtime supports waiting when required physical material is unavailable or a valid local continuation cannot currently be made; final integration of that behavior into the catalog-backed genesis constructor remains unfinished.

### Migration status

The following retired concepts have been removed from the live organism state:

- `DevelopmentStage` and its Juvenile/Adult/Offspring variants;
- age-based maturation;
- adult-stage reproduction gating;
- the old 40% developmental/reproduction gate;
- the separate `juvenile_reserve` material concept;
- the `adult_mass()` API, replaced by `preferred_mass()`.

The serialized field name `juvenile_energy_reserve` remains only as a backward-compatible alias for the renamed `reproductive_energy_allocation` field. It is not a lifecycle reserve.

The physical construction/calibration module is still named `juvenile.rs`, but its retained seed machinery is a construction-calibration artifact rather than a biological juvenile stage. Physical viability checks live in `organism_viability.rs` and describe whole-organism physical viability.

## OBSOLETE — Previous Immediate Engineering Plan (No Longer Valid)

> [!CAUTION]
> **OBSOLETE PLAN — DO NOT FOLLOW ITS CONSTRUCTOR DEFERRAL OR INTEGRATION ORDER.** The earlier instruction to defer constructor redesign and later prepare another proposal is superseded. The approved task is now to develop the replacement constructor around the geometry and chemistry libraries. The library audit and stabilization work described below may still be useful where it supports the new plan, but it is no longer a reason to postpone constructor architecture work.


The immediate engineering priority is **stabilizing Bob's geometry knowledge layer**. Constructor redesign and constructor-to-Bob integration are explicitly deferred until the library interface and lookup behavior are internally consistent and documented.

1. **Reconcile Bob's source/API contract.**
   - Confirm that the module wiring, public library functions, worker imports, and documentation describe the same API.
   - Resolve missing, renamed, duplicated, or incorrectly mapped functions before adding new layers.
   - Keep one authoritative implementation for persistent formations, contact families, canonicalization, and indexed lookup.
2. **Define and verify lookup semantics.**
   - Document which queries are supported, what their results guarantee, and how geometry/schema versions affect eligibility.
   - Treat library entries as reusable knowledge, not as permission to skip current physical validation.
   - Keep discovery/worker responsibilities separate from read-only runtime lookup responsibilities.
3. **Evaluate bounded frequency-aware priority caching.**
   - First measure existing indexed lookup cost and access patterns; do not assume a cache is needed solely because the catalogue is large.
   - If measurements justify it, add a bounded in-memory cache for frequently reused formation and contact-family query results.
   - Promote entries based on observed frequency, enforce a clear memory bound, and expose hit/miss/eviction measurements.
   - Keep persistent catalogue data authoritative for knowledge storage; the hot cache is disposable acceleration state, not a second source of truth.
4. **Verify Bob independently.**
   - Add focused tests for API consistency, canonicalized lookup, contact-family lookup, cache promotion/eviction if implemented, and cache-disabled or cold-start behavior.
   - Confirm worker persistence and read-only lookup remain separate, and that tests use isolated temporary library roots where writes are involved.
   - Run formatting, compilation, and focused tests when a runnable source checkout is available; report unrun checks explicitly.
5. **Stop at the library boundary for this phase.**
   - Do not redesign, modify, or wire the initial-organism constructor into Bob in this phase.
   - Do not build cache policy around assumptions about constructor call patterns before those patterns can be measured.
   - Once Bob's API and measured lookup behavior are stable, document a separate proposal for later constructor integration before beginning that work.

## Construction invariants

These are physical/specification invariants, not an alternative constructor algorithm. Retain them where consistent with the authoritative replacement plan below.

These are the constraints that matter to current and final construction work:

- Physical geometry is authoritative.
- A permanent bond is created through the shared physical bond transaction.
- Intended bond contact is distinct from unintended penetration.
- Construction does not require a universal grid, 4N topology, or predefined cavity shape.
- The genome is defined by a qualifying realized cavity, not by a hard-coded core.
- Water is a physical material when instantiated; it is not logical material placed into storage merely to satisfy a test.
- Composite physical material retains its internal structure.
- Acquisition is based on physical contact/overlap, not exact coordinate identity.
- Construction decisions are local and forward-only; performance should come from direct geometric construction rather than global brute-force enumeration.

## Verification discipline

For each change:

1. inspect the exact code path and contract being changed;
2. make the smallest isolated implementation change;
3. run formatting and compilation;
4. run focused tests for the changed contract first;
5. inspect the actual failure before changing the next layer;
6. broaden the test scope only after the focused contract is proven.

Do not redesign the constructor in response to a downstream failure until the underlying construction contract has been verified.

## Chemistry

This section records the current chemistry model and design decisions. Chemistry is intentionally **primitive-driven and emergent**: acidity, toxicity, corrosion, catalysis, digestion, and selective chemical behavior are not dedicated fields or hard-coded organism abilities. They should arise from physical material, local chemical interaction, geometry, topology, bond stability, and the transitions those primitives permit.

### Formation chemistry

**BREAK and COMBINE are chemistry/formation consequences, not organism actions.** Organisms do not issue Break or Combine commands and the initial cell does not contain dedicated chemistry organelles. Organisms influence contact and arrangement through physical behavior; chemistry determines whether a resulting interaction can actually change material.

The universal transition path is:

    local encounter
        -> evaluate material and interface
        -> accumulate interaction
        -> reach a transition threshold
        -> no change, bond formation, bond rupture, rearrangement,
           separation, deformation/material transfer, or another valid transition
        -> realize the resulting physical state

Favorable interaction is therefore not instantaneous. Contact can produce an interaction without immediately forming or breaking a bond.

### Foundational inputs

Local chemistry derives from:

1. **Composition** — constituent identities and quantities;
2. **Bonded topology** — which constituents are connected and how they are arranged;
3. **Exposed geometry** — actual exposed surfaces/features and physical contact;
4. **Bond strength and formation energy** — stability and energetic cost of existing/new bonds;
5. **Chemical position** — each base resource's position on the ordered chemical spectrum.

Mass, potential energy, cohesion, geometry, topology, bond energy, and chemical position remain distinct concepts. No single aggregate value should silently replace them.

### Chemical spectrum

Chemical position is an ordered **1–14 scale**, analogous to an acid/base scale but **not literal pH chemistry**:

    1 — 2 — 3 — 4 — 5 — 6 — [7 Water] — 8 — 9 — 10 — 11 — 12 — 13 — 14

There are seven base resources. The approved resource-to-position mapping is fixed as follows:

| Resource | Chemical position |
|---|---:|
| Methane | 1.5 |
| Sulfur | 3.5 |
| Hydrogen | 5.5 |
| Water | 7.0 |
| Nitrogen | 8.5 |
| Carbon | 10.5 |
| Phosphorus | 12.5 |

Water is the midpoint at **7**. Methane and Phosphorus intentionally remain inside the 1–14 scale rather than occupying its extreme endpoints. These positions are catalog data, not reactivity magnitudes.

Chemical position is **not a reactivity magnitude**. It describes position on the spectrum. For a local pair:

    D = |position_A - position_B|

D is the fundamental chemical separation. Greater separation means greater chemical opposition and therefore a stronger tendency to interact; nearby positions have less opposition.

Chemistry must never average positions across a formation. A formation containing positions 2 and 12 is not chemically equivalent to a pure position-7 material. Multi-constituent chemistry must preserve the constituent identities and local interactions.

Interaction strength is a **bounded nonlinear function of D**, not simply D. Greater opposition must not produce less interaction. The approved curve is fixed by the equations below; it is no longer an open migration-time tuning decision.

Chemical position is also separate from potential energy. Chemical separation describes interaction tendency; potential energy and bond energy describe energetic consequences.

### Local interaction, timing, and bonds

Chemistry is local. EvoSim must not globally scan every environmental material pair or maintain a registry of every active interface.

A local encounter produces an interaction that can accumulate over ticks:

    encounter -> interaction -> accumulation -> threshold -> transition

Strong interactions may cross a threshold quickly; weak interactions may require longer contact. Physical geometry and contact determine whether and how strongly an interface is engaged. Penetration is never a substitute for contact.

Existing bonds have their own stability/energy. Chemical opposition must accumulate enough effective interaction to overcome relevant structural stability before rupture or rearrangement can occur. Strong chemistry cannot teleport through a strong structure.

The approved attraction model separates static chemical potential from distance:

    D = |position_A - position_B|

    I_AB = (exp(kD) - 1) / (exp(kD_max) - 1)

    C(r) = (1 - r/R)^2       for 0 <= r <= R
    C(r) = 0                  for r > R

    F_AB = I_AB * F_max * C(r)

I_AB is a **static pairwise chemical interaction potential**. It does not increase or decrease as the materials move. Distance changes the amount of that potential expressed as physical attraction. Attraction therefore increases as the materials approach and reaches its maximum at contact; it does not decay toward contact. The current simulation has no universal force/acceleration integrator, so this attraction is not routed into a chemistry-specific movement instruction. When universal physical dynamics exists, chemical attraction belongs in that shared physical-force layer.

The approved reaction-accumulation direction is a bounded accumulation model with a calculated activation barrier:

    R_(t+1) = R_t + I_AB * C(r) * G - lambda * R_t

    R >= B  -> transition

where G represents the actual interface/geometry engagement, lambda represents dissipation, and B is a calculated activation barrier derived from the physical situation rather than a universal arbitrary reaction threshold.

The current parameter contract is now fixed for the first implementation pass:
- D_max = 13.0: fixed ceiling of the chemical-position coordinate system.
- k = 1 / D_max: the nonlinear exponent reaches 1 at the top of that coordinate domain.
- R = 1.0: one shared physical geometry unit for the chemistry contact radius.
- F_max = 1.0: one chemistry attraction unit; this is not resource energy.
- lambda = 0.10 per tick: ten percent of accumulated reaction dissipates each tick.
- G = calculated from the realized interface/geometry engagement, constrained to 0..1.
- B = calculated from the actual physical interface/bond state; it is not a universal reaction threshold.

These fixed values establish the first chemistry scale without turning chemistry into a collection of arbitrary per-pair constants. G and B remain calculated quantities because they must reflect actual geometry and structural requirements.

### Multiple constituents and rupture

A formation must not be reduced to whichever two materials happen to be adjacent, nor to a formation-wide average.

The first layer is **local constituent-pair interaction** at actual interfaces. The second is **recurring/coherent pattern detection** across those local interactions. When multiple local interactions repeatedly produce a coherent structural/energetic consequence, that pattern can justify a larger formation transition.

The system must not assume in advance that the weakest bond, strongest bond, or one interface bond is always the one that breaks. Whether rupture is localized or distributed is an experimental question; the affected region should follow the interaction pattern.

**Natural BREAK is required.** The chemistry layer has an energy-surplus BREAK path: a reaction must supply enough energy to meet the exact realized bond disruption requirement before that bond can rupture. Runtime reaction production, interface selection, structure-revision invalidation, and immediate physical BREAK are integrated.

COMBINE follows the same rule: physical contact and compatible geometry make a formation possible, but only the universal transition rules permit the new bond.

Products are not selected from a hard-coded product catalogue. Existing physical constituents and topology are transformed and then physically realized. For example:

    A—B -> A + B
    A + B -> A—B
    A—B—C -> A—C + B

### Heat-stress structural damage selection

When accumulated heat stress reaches the current stress threshold, structural damage selects a **random eligible non-genome structural bond**.

1. Select randomly from eligible non-genome structural bonds.
2. Genome bonds remain protected while any eligible non-genome structural bond remains.
3. Genome bonds become eligible only after the non-genome set is exhausted.

Genome-bond identity must come from the existing genome-cavity authority, which identifies the actual physical cavity-boundary bonds. Do not infer genome bonds from a fixed material recipe, unit count, construction index, or other legacy proxy. This approved random-selection rule supersedes the 2026-09-16 weakest-bond amendment.

### Energy conservation

Chemical transitions must conserve energy. Chemical interaction is not automatically usable organism energy.

Transition energy may be stored in newly formed bonds, released from broken bonds, transferred into usable organism energy, become heat/environmental energy, become kinetic movement, or remain associated with transferred/expelled material. Not every reaction must yield usable energy.

### Structural and nonstructural material

Material inside an organism may be persistent **structural material** or **nonstructural material within its accessible internal volume**.

Structural material participates in the persistent physical graph. Nonstructural material is physically present but is not yet part of that structural graph. The organism can manipulate nonstructural material by moving, positioning, separating, or contacting it; chemistry still determines which arrangements can produce structural transitions.

    organism controls arrangement
    chemistry controls what arrangements can actually do

This deliberately provides no dedicated digestive, metabolic, combine, or break organelle.

### Fluids

Fluids use the same chemistry rules as rigid material. Their distinction is physical state: fluids flow, deform, and can pass through permeable structures, rather than acquiring rigid structural bonds simply because fluid pieces touch.

Water remains a real constituent at spectrum position 7 while retaining its existing continuous-fluid physical model.

### Emergent properties

Acidity, toxicity, corrosion, catalysis, digestion, and similar effects are expected **consequences**, not fields. A material that repeatedly damages another formation through permitted transitions can therefore behave corrosively without a corrosiveness property; a material that disrupts another formation can behave toxically without a toxicity field.

If an expected phenomenon cannot emerge, first identify which physical/chemical primitive is insufficient rather than adding a dedicated organelle or property.

### Geometry–chemistry division

The Geometry Reference Library is the geometry knowledge layer. Chemistry must use it rather than independently rediscovering or canonicalizing geometry.

Bob's library provides reusable information about:

- canonical formations;
- bonded topology;
- exposed surfaces/features;
- valid physical contacts/interfaces;
- contact geometry and scale;
- equivalent rigid representations;
- positional equivalence for otherwise-identical geometry is **≤ 0.5 units**. Bob records one canonical relationship rather than separate records for microscopic positional variations within that tolerance. Meaningful changes to material, topology, contact feature, orientation, or other geometry remain distinct.

The Chemistry Library will consume those canonical geometry/interface identities and combine them with material composition and chemical position to cache reusable chemical interaction information. Chemistry must not become a second geometry search engine.

The intended division is:

    Geometry Library
        -> "What physical formations and interfaces are possible?"

    Chemistry Library
        -> "Given this canonical physical interface and these materials,
           what chemical interaction/transition does it permit?"

    Runtime
        -> "Are these formations actually near/contacting each other now?"

### Chemistry Library and scaling

The Chemistry Library is a persistent, demand-driven knowledge layer rather than an exhaustive formation × formation reaction table.

A runtime encounter should approach:

    physical locality
        -> cheap candidate filter
        -> Bob geometry/interface identity
        -> canonical chemistry key
        -> chemistry-library lookup
        -> calculate only on cache miss
        -> persist reusable result
        -> runtime accumulates/executes the interaction

Absolute world coordinates, organism identity, tick number, unrelated nearby objects, and current distance should not be part of the intrinsic chemistry signature. Distance belongs to runtime. Equivalent physical representations should resolve to the same canonical chemistry interaction.

The library should become richer through actual environmental encounters rather than by inventing a predefined catalogue of reactions.

### Chemistry maturation before organisms

Once the Geometry Library and Chemistry Library are ready, the environment should be allowed to run **without organisms** for repeated cycles.

The same environmental material movement and physical encounter rules should generate chemistry naturally. New geometry and chemistry knowledge is persisted across cycles.

Each cycle should measure:

- new geometry discoveries;
- new chemistry discoveries;
- repeated/common interactions;
- newly observed transitions;
- library growth;
- computation cost.

The run should continue until discovery substantially saturates rather than stopping at an arbitrary fixed number of ticks. Only after the environment has established a useful baseline of common interactions should the viable initial organism be introduced.

### Chemistry migration status

The chemistry migration is complete at the currently approved architecture boundary. The following boundaries have been audited and migrated:

| Area | Status | Current boundary |
|---|---|---|
| Resource chemical positions | **Complete** | Seven catalog positions are explicit data: Methane 1.5, Sulfur 3.5, Hydrogen 5.5, Water 7.0, Nitrogen 8.5, Carbon 10.5, Phosphorus 12.5. |
| Core chemistry mathematics | **Complete** | Static chemical-position interaction, distance/contact expression, reaction accumulation, dissipation, and barrier checks exist with invariant tests. |
| COMBINE energy boundary | **Migrated** | Remaining formation work is derived from the approved attraction force over the actual remaining approach distance; there is no arbitrary mass/cohesion formation multiplier. The formation threshold remains a structural eligibility/investment requirement, while intrinsic bond potential is stored separately and allocated through the energy ledger. |
| BREAK energy boundary | **Migrated** | Structural BREAK releases the realized bond's stored intrinsic bond potential exactly once. Chemical BREAK uses accumulated reaction energy to pay the realized bond's activation/disruption barrier, with only the surplus becoming usable energy or heat. Constituent material potential is not recreated at bond rupture. |
| Energy conservation | **Implemented at chemical BREAK boundary** | The ledger remains authoritative. Chemical BREAK energy is settled as one conserved reaction-energy source: disruption cost + usable energy + heat = reaction energy. |
| Geometry → chemistry interface | **Live Bob resolver connected** | Runtime chemistry resolves each realized bond contact through Bob's live candidate resolver, retaining local boundary feature/parameter identity rather than topology alone. Distance and facing remain transient runtime state and are not part of the intrinsic chemistry key. |
| Chemistry Library | **Runtime lookup + calculate-on-miss + persistence integrated** | The versioned persistent library accepts material pairs plus canonical Bob interface identities. Runtime chemistry now performs lookup, calculates the static interaction potential only on a miss, and persists the reusable result. Activation barriers remain physical bond properties and are not cached as chemistry facts. |
| Natural chemistry-driven transitions | **Runtime BREAK + immediate COMBINE formation path** | Existing bonded interfaces accumulate local reaction state using the fixed chemistry contract; reaching the physical bond barrier produces an immediate local BREAK event. COMBINE now resolves immediately through the same physical formation boundary, with chemistry attraction gating real formation and developmental preference retained as solver guidance. Independent BREAK interfaces may react in the same tick; physical conflicts are resolved against the current structure. Autonomous multi-interface formation remains. |
| Retired reactivity cleanup | **Complete** | Obsolete chemistry/resource-selection uses have been removed. Remaining `reactivity` uses are owned by harmonics/nonlinear spectral response or compatibility fixtures/data and are not used as chemical position, interaction magnitude, break efficiency, or formation energy. |

**Migration rule:** chemical position is chemistry; reactivity is not a fallback chemical position, interaction magnitude, break-efficiency factor, or formation-energy source. Remaining `reactivity` code is therefore treated as transitional until its owning subsystem has an explicit replacement contract.

`potential_energy` remains an intrinsic material property and is not being silently repurposed as chemical interaction strength. The energy ledger remains the authority for all realized energy transactions.

### Chemistry completion audit

The migration completion audit is closed at the currently approved architecture boundary. Findings were checked against the live implementation before documentation was finalized. The deferred universal physics consumer is an intentional architectural seam, not a chemistry migration defect.

1. **Chemistry implementation inventory — complete**
   - `chemical_position` is the sole chemical-spectrum input.
   - `reactivity` remains only in harmonics/nonlinear spectral response and compatibility fixtures/data.
   - `potential_energy` remains an intrinsic material/energy-ledger property.
   - `cohesion` remains a physical/material property used where its own contracts require it.
   - Chemistry interaction, BREAK, COMBINE, reaction thresholds, and library persistence use the migrated paths described above.

2. **Resource-property semantics audit**
   - Verify mass, potential energy, cohesion, and chemical position for every base resource.
   - Establish the exact decimal chemical-position mapping.
   - Find every place chemical position is incorrectly averaged or treated as a reactivity/energy magnitude.

3. **Mathematical chemistry audit**
   - Translate the approved equations into exact implementation contracts.
   - Identify every remaining parameter requiring definition.
   - Verify monotonicity, boundedness, contact behavior, and limiting cases before implementation.

4. **Energy-conservation audit**
   - Trace potential energy, bond energy, work, formation, rupture, transformation, kinetic transfer, and environmental transfer.
   - Identify every current path that creates, destroys, or silently transfers energy.
   - Establish conservation invariants for each transition class.

5. **Formation/interface audit**
   - Determine how formations, bonds, exposed features, contact, contact area/coverage, and geometry are currently represented.
   - Identify where chemistry currently reconstructs geometry independently or receives incomplete geometry.

6. **Bob/Geometry Library audit**
   - Establish what the Geometry Reference Library actually provides, what remains incomplete, how canonicalization works, and what Chemistry can query.
   - Identify direct contradictions only; do not redesign working geometry infrastructure without evidence.

7. **Chemistry Library feasibility audit**
   - Define the minimum canonical chemistry key.
   - Verify that composition + Bob's canonical geometry/interface identity is sufficient.
   - Estimate combinatorial growth and ensure demand-driven caching remains bounded.

8. **Runtime locality/performance audit**
   - Trace candidate generation, spatial partitioning, neighbor lookup, formation-level checks, constituent-level checks, and chemistry invocation frequency.
   - Prove there is no hidden global pair scan.
   - Estimate worst-case chemistry work under dense local conditions.

9. **Existing test-contract audit**
   - Classify chemistry tests as authoritative, expectation-to-update, obsolete, broader physical invariants, or missing.
   - Preserve tests that enforce genuine physical contracts even when old chemistry semantics are removed.

The implementation migration is complete; remaining work belongs to downstream chemistry experiments, universal physics, or separate constructor/geometry workstreams.

### Chemistry implementation order

After the audits, the implementation sequence is:

1. establish exact chemistry/resource semantics;
2. migrate the existing system from obsolete reactivity equations to the approved chemistry model;
3. finish Bob the Builder and the Geometry Reference Library to the required chemistry interface;
4. build the persistent Chemistry Library on top of Bob;
5. run the organism-free environment in cycles until geometry/chemistry discovery substantially saturates;
6. build and verify a viable initial cell;
7. place the organism into the matured environment and troubleshoot biological behavior from there.

The guiding boundary is:

    Geometry -> Chemistry -> Environment -> Organism

This order is intended to make failures diagnosable: after the organism is introduced, a failure should be traceable either to the established physical/chemical world or to the organism's behavior, rather than both systems being unfinished simultaneously.

### Completion criteria

Chemistry is sufficiently complete for integration when:

1. local interaction derives from spectrum position rather than obsolete reactivity magnitude;
2. chemical separation, bond stability, and potential/bond energy remain distinct;
3. static pairwise interaction potential is separated from distance-dependent attraction;
4. attraction increases as interacting materials approach;
5. reaction interaction accumulates over time with dissipation;
6. transitions use calculated activation barriers;
7. natural bond rupture and bond formation use the same transition framework;
8. multi-constituent transitions can arise from recurring/coherent local patterns without a universal weakest-bond rule;
9. energy is conserved;
10. acidity/toxicity/catalysis/etc. require no dedicated fields or organelles;
11. rigid and fluid material share chemical transition logic while retaining different physical states;
12. runtime uses local lookup/evaluation rather than global pair scanning or an exhaustive formation-pair reaction table;
13. Chemistry uses Bob's canonical geometry/interface knowledge rather than duplicating geometry discovery;
14. the persistent Chemistry Library can cache reusable interaction knowledge without unbounded redundant representations;
15. the organism-free environment can populate common geometry/chemistry interactions before the organism is introduced.

This does not require predicting every chemistry before simulation. Remaining curve and transition details should be settled by focused experiments once the primitive interfaces exist.

### Current status

The chemistry model, migration, and completion audit are complete at the approved architecture boundary. Resource chemical positions, the core chemistry equations, COMBINE's energy boundary, BREAK's energy boundary, and the chemistry-driven BREAK energy-surplus gate have been migrated away from the retired interaction semantics.

The remaining `reactivity` uses are outside authoritative chemistry. The resource-helper, construction-selection, and obsolete math uses have been removed. Harmonic/nonlinear spectral response has been audited and deliberately retains `reactivity` as a separate material-response input; compatibility fixtures/data remain for separate cleanup. No remaining use may be mapped to chemical position without a subsystem-specific contract.

Natural COMBINE formation has now been brought into the same energy model. Remaining formation work is derived from the integral of the approved attraction force over the actual remaining approach distance; there is no arbitrary mass/cohesion formation multiplier. The formation threshold remains a physical eligibility/investment requirement, while intrinsic bond potential is a separate stored structural quantity and is allocated separately in the formation transaction. The chemistry-driven BREAK seam now exists: reaction energy can rupture an existing bond only when it meets that bond's supplied physical disruption requirement. Chemistry accumulation is stored in normalized chemistry units, then converted explicitly to physical energy at activation using force × characteristic contact distance (`CHEMICAL_MAX_FORCE × CHEMICAL_CONTACT_RADIUS`). The approved values make that conversion 1:1 numerically without hiding the unit conversion. Chemistry must not implement geometry mapping itself.

## Project direction

The long-term goal remains an open-ended simulation in which organisms can develop structure, acquire resources, sense their environment, reproduce, form niches, and potentially evolve multicellular cooperation from the same general physical and behavioral mechanisms.

The project execution order is now:

**Chemistry -> system migration -> Bob/Geometry Library -> Chemistry Library -> organism-free environment maturation -> viable cell -> organism in environment.**

## Base-resource geometry

Hydrogen is a **rigid rectangle 1.0 units long × 0.1 units thick**. It is a finite-area physical strip, not a zero-thickness line. Its two primary structural contact points remain at the centers of the two longitudinal end faces, preserving line-like endpoint topology. Any geometry generation, validation, physical contact, construction, or visualization involving Hydrogen must therefore use its rectangular boundary and 0.1 thickness while preserving those two primary endpoints.

## Geometry Reference Library

The geometry catalogue is a **persistent geometry reference library** separated from live organism state. It is durable knowledge, not test state and not a per-construction search cache.

The intended catalogue grows exhaustively from the smallest physical formations upward:

1. record every valid single-resource geometry;
2. enumerate every physically valid two-constituent combination, including repeated resources;
3. [SUPERSEDED] Do not persist three-constituent formations. The active catalogue limit is two constituents maximum.
4. [SUPERSEDED] Do not expand Bob's catalogue toward 20 constituents. Larger live structures must be composed from bounded records at runtime, subject to normal physical validation; they are not stored as whole-assembly catalogue entries.
5. canonicalize equivalent formations so the library stores each physical geometry once;
6. preserve the library across tests, process restarts, and constructor runs.

The implementation provides the persistent store, schema/versioning, canonical formation representation, independent geometry validation, durable JSONL storage, base-resource seeding, and durable frontier/progress records. The store lives outside `target/` and therefore is not reset by normal Rust builds or test cleanup.

### Library rules

- The library is **knowledge, not authority for live organism state**. The constructor may use it as a read-only source of known-valid continuations, but committed simulation bonds still go through the normal physical transaction.
- Validation is independent of discovery. A formation is stored only after resource/shape validity, bond topology, connectivity, and non-penetration checks succeed.
- Global translation and rigid rotation are canonicalized. Reflections are **not** collapsed: a mirror image is a distinct physical formation unless the geometry itself makes it identical.
- [SUPERSEDED] The historical 20-constituent catalogue plan is invalid. Repeated constituents are allowed within the active maximum of two physical pieces per formation.
- Water remains a real resource. Fluid formations without a finite boundary are represented without inventing rigid collision geometry; their context-fitting realization remains a separate physical-field concern.
- Fluid-to-fluid contact is not a rigid bond. Combining fluid with the same fluid produces the same fluid shape with greater volume; the geometry library must not represent that operation as two bonded fluid constituents.
- Floating-point coordinates are represented in canonical signatures with a fixed geometric quantization tolerance. The geometry schema and resource-shape catalogue are versioned so stale geometry cannot silently become current knowledge.
- Tests use isolated temporary library roots. They must never mutate the persistent production catalogue.

### Worker

The geometry worker is a persistent, resumable catalogue process. It loads the persistent library, selects unexplored formation/frontier work, generates exact geometry candidates, validates and canonicalizes them, appends only new formations, and durably records progress so it can resume after interruption. It does not use the constructor's old brute-force placement loop or an arbitrary attempt budget.

The worker expands from validated formations rather than repeatedly solving each composite from scratch. It persists frontier records for formation/resource expansions, resumes unfinished work after restart, and idles rather than hot-spinning when no work is currently available. It can be started with `cargo run -- --geometry-worker`.

Continuous contact families are represented separately from rigid candidate placement. The current fluid model uses 2-D zero-gravity Young–Laplace capillary equilibrium. For Water against a rigid edge, conserved area and the effective wetting contact angle determine a constant-curvature circular arc analytically, with remaining translational freedom along the exposed edge. These cases are represented as continuous-family work rather than being approximated by arbitrary placement sampling. The representation and expansion of all continuous contact families are not yet exhaustive.

The geometry library is intended to become the long-lived reference layer that the constructor can query instead of rediscovering the same geometry during every organism construction.

### Current implementation status

**Implemented:** persistent library core, canonical formation schema, independent validation, durable append-only storage, version manifest, base-resource seeding, durable frontier/progress records, resumable worker process, exact rigid feature-contact generation for the current finite polygon/line feature model, and expansion bounded to the active two-constituent catalogue rule.

**Still incomplete:** exhaustive representation and expansion of every continuous contact family, the final completeness audit of symmetry reduction, and integration of the library into the live organism constructor.

### Geometry library progress

The library includes exact feature-contact generation for rigid formations and expansion from validated smaller formations. Two-body seeding considers base resources against single-resource formations, validates candidates, canonicalizes them, and persists only new formations. [SUPERSEDED] Three-body expansion and the former plan to expand toward 20 constituents are prohibited by the active two-constituent catalogue cap.

Every declared rigid bond must correspond to actual physical boundary contact without positive-area/interior penetration. Fluid-to-fluid combinations are deliberately not represented as rigid bonds because they merge into the same fluid geometry with increased volume. The rigid path does not use arbitrary angular sampling or an attempt budget. Runtime rigid surface-contact candidates likewise use exact polygon feature relationships rather than a sampled angular sweep.

Continuous contact families are intentionally not claimed exhaustive. The library has exact representations for the currently supported rigid and point/endpoint contact families, but additional continuous manifolds still require explicit representation before completeness can be guaranteed.

### Geometry canonicalization

The geometry library removes redundant local rotations that leave a constituent's physical shape unchanged. Regular polygons use their exact rotational symmetry, rectangles and line segments use their twofold symmetry, and circles have no meaningful local rotation. This is a proper-rotation equivalence only: mirror-image formations remain distinct. Hydrogen remains a 1.0 × 0.1 finite-area rectangle while retaining its two line-like primary endpoints.

## Geometry Library Visualizer

The geometry library has a dedicated read-only browser microscope. It reads the persistent catalogue without participating in generation or modifying library state. The browser refreshes the catalogue list every five seconds so newly recorded formations become visible; an already selected formation is not re-read or changed.

Run it with:

    cargo run -- --geometry-viewer

Then open `http://localhost:3001/geometry`. The viewer supports:

- browsing persisted formations, ordered from smallest to largest;
- filtering by constituent count or resource/signature text;
- selecting a formation and seeing its actual constituent placements and shapes;
- displaying the persisted bond topology;
- inspecting persisted rigid, point-contact, vertex-contact, and fluid contact families;
- zooming and panning the stored formation without changing it.

The visualizer is deliberately separate from the catalogue worker and the live organism constructor. It is a verification microscope: once a formation is selected, it displays that recorded formation as a static view and does not re-read, regenerate, alter, or reinterpret it. Only newly added library entries are discovered by the periodic catalogue refresh.


# Replacement Initial Organism Constructor
## Authoritative architecture, library integration, migration, and validation plan

**Status:** Approved design plan, amended 2026-10-11 to adopt the library-backed compositional planner and separate Bob library-expansion loop; implementation is not implied by this documentation.  
**Scope:** Replace the current initial-organism construction architecture while preserving the valid physical and biological requirements in this plan. This is a marked revision of the existing plan, not a declaration that the plan itself is invalid.  
**Primary objective:** Produce a physically valid, viable initial organism by composing known geometric arrangements and making the minimum necessary sequence of real construction decisions, without prescribing its final topology.  
**Core principle:** The constructor plans over verified knowledge supplied by Bob and authoritative chemistry/physics systems. It does not invent arbitrary geometric candidates at runtime or become another geometry engine, chemistry engine, or organism-design script.

> [!IMPORTANT]
> ### Approved plan amendment — library-backed composition and separate discovery (2026-10-11)
>
> This amendment refines the existing architecture. It preserves milestone-driven construction, whole-structure validation, physical acquisition, atomic transactions, analyzer-confirmed genome discovery, and the other invariants below. It changes how geometric candidates are obtained and what happens when Bob lacks coverage.
>
> **Required direction:** the constructor may propose only placements supported by persisted, verified Bob formation/contact records. It may combine those known local arrangements into larger structures Bob has never catalogued as a whole. It must not generate arbitrary fallback poses, silently use scaffold coordinates, or treat an unrecorded in-memory suggestion as a library discovery.
>
> **Separate expansion loop:** if the planner cannot find a viable path using the current library, it must return a structured missing-coverage or contextual-blocker report. Bob's discovery worker runs separately, expands and verifies the library, and the planner retries against the resulting versioned data. Do not run open-ended geometry discovery inside a simulation tick.
>
> **Current bootstrap constraint:** the checked-in geometry catalogue is intentionally empty. Bob's persistence tests and prior bounded-generation run do not prove that this constructor works. Before strict library-backed construction can be accepted, run Bob's bounded worker to generate and validate singleton and two-constituent formations (maximum constituent count = 2), inspect the output and coverage, and make that library available to the constructor as generated runtime/artifact data. Do not commit generated catalogue files as source code or treat the temporary two-piece catalogue as complete coverage.
>
> **Evidence boundary:** successful library open/reopen, passing Bob tests, and successful two-piece generation prove library behavior only. Acceptance requires the live initial-organism constructor to build a viable organism using only known-library placements, with authoritative live physical/chemical validation and no unrecorded geometric fallback.
>
> **Marked changes in this revision:** Sections 3–6, 12, 13–17 are revised below to make candidate provenance, strict query semantics, composition, missing-coverage reporting, and bounded library expansion explicit. The existing physical and biological requirements remain in force.

> ### Approved plan amendment — fluid geometry is first-class Bob knowledge (2026-10-11)
>
> Bob's geometry library must calculate, validate, canonicalize, and persist **fluid geometry as well as rigid geometry**. Fluid handling is not a runtime-only exception, a visualization-only record, or a special case left for the constructor to invent. This is a required capability of the same durable geometry knowledge system.
>
> Fluid records must represent the *constraint and response*, not merely save one frozen outline: the fluid material and volume; the neighboring material/interface class; the boundary/contact configuration; the deformation or translation degrees of freedom; the solver inputs and relevant tolerances; and the validated result or feasible interval/region. A record must make clear which parts are invariant (for example, conserved volume) and which may vary with the contact constraints.
>
> Bob must cover fluid boundaries against rigid edges, corners/vertices, lines/points, and other fluid boundaries where the physics permits them. It must record valid fluid–rigid arrangements and fluid deformation/contact solutions, including infeasible cases as diagnostic outcomes when useful for discovery metrics—not as constructible formations. Canonicalization must avoid filling the catalogue with tiny numerical variations of the same relationship.
>
> The existing capillary/fluid solver remains the calculation authority; Bob should invoke it in the discovery worker, verify its output, and persist the reusable result and enough inputs/metadata to resolve and reproduce it. The live constructor may query and compose these persisted results, but must still revalidate the current physical state through authoritative runtime rules. A fluid query that lacks a matching persisted record is a coverage miss; it must not silently fall back to an unrecorded solve during construction.
>
> **Acceptance gate:** tests must prove fluid records are generated, persisted, reopened, canonicalized, queried by live fluid-boundary identity, and composed with rigid records; verify volume preservation, nonpenetration, boundary/contact tolerances, and rejection of invalid/infeasible solutions. The bounded two-constituent bootstrap must explicitly include fluid–rigid and applicable fluid–fluid cases and report coverage by interface class, rather than counting rigid formations alone.

## 1. Executive summary

EvoSim needs a constructor that can produce a viable organism from available materials. It must discover a genome through the realized physical structure, acquire required materials through valid physical interactions, and satisfy the actual biological requirements.

The replacement will not assemble toward a fixed piece count and test viability afterward. It will propose individual physical changes or reusable local arrangements, validate them before commitment, commit only complete valid transactions, and check developmental milestones as construction proceeds.

The geometry library will supply reusable, persisted knowledge about shapes, contact features, and verified local arrangements. The constructor will search combinations of these known arrangements; it will not manufacture an arbitrary placement when lookup fails. The chemistry library and authoritative bond rules will determine material compatibility and chemical/energetic consequences. The constructor will coordinate these authorities, choose among viable candidates, and stop when the actual organism is viable. When library coverage is insufficient, it will report the missing capability for a separate Bob expansion run.

The replacement must:
- Construct an initial organism without a prescribed blueprint or final shape.
- Use existing geometry and chemistry knowledge instead of repeatedly rediscovering it.
- Select geometric candidates only from persisted, verified Bob records; compose known local arrangements to create larger, previously unseen organisms.
- Keep geometry discovery separate from runtime construction. Missing library coverage is an explicit planner result and triggers a separately run, bounded Bob expansion process.
- Discover a genome cavity through the real cavity analyzer and require a qualifying bonded seal.
- Stop treating enclosure as a goal immediately after genome qualification.
- Physically acquire Water and at least three distinct non-Water resource categories.
- Enforce geometry, bonding, material, energy, and ledger invariants.
- Permit multiple viable outcomes, with deterministic reproducibility and seed-driven diversity.
- Share its physical engine with offspring construction while allowing a different policy and limited inventory.
- Report why it succeeded, stalled, or failed.
- Remove the old fixed-size and prescribed-construction architecture rather than wrapping it in another layer.

This is a design plan. It is not evidence that the implementation exists or has passed tests.

## 2. Lessons from other artificial-life simulators

### Framsticks — development versus realized body

Use explicit developmental milestones and evaluate the realized physical result, not the structure the constructor intended to create. Do not import genotype-driven predetermined body construction: EvoSim's first organism must physically discover its qualifying genome cavity.

### MABE — modular authorities

Keep geometry, chemistry, bonding, cavity analysis, acquisition, energy accounting, and construction policy separate. The constructor orchestrates these authorities; it must not duplicate their rules.

### Tierra and Avida — viability is a bootstrap condition

Initial construction establishes a viable starting point. It must not encode a permanent evolutionary trajectory, body ladder, or fixed set of organism roles.

### Polyworld — apparent success is not physical proof

A plausible-looking organism is not sufficient. Independently validate collisions, bonds, cavity qualification, resource acquisition, and accounting.

**Combined lesson:** use milestones without a prescribed body plan, modular physical authorities, minimal viability conditions, and independently measurable correctness.

## 3. Architecture and responsibilities

### 3.1 Authoritative physical knowledge

The Geometry Reference Library provides reusable geometric knowledge. The Chemistry Library and existing chemistry/bond authorities provide material and interaction knowledge. Runtime physical rules remain authoritative for the current organism.

### 3.2 Candidate query and indexing

Indexes narrow the search using material composition, exposed connection features, feature scale, arrangement size, shape, orientation, and validation status. Indexes are regenerable accelerators, not independent sources of physical truth. Each returned geometric candidate must trace to a persisted formation/contact record with a stable record identity and library/schema version. An in-memory candidate generated on a query miss is not a Bob discovery and is not eligible for strict construction.

### 3.3 Authoritative validation

Every candidate is checked against the actual context. Validation covers geometry and nonpenetration, physical contact, bond compatibility and strength, interactions with the existing structure, available resources, energetic consequences, and accounting.

### 3.4 Atomic construction transaction

A candidate is prepared and validated before commitment. A successful transaction updates all relevant geometry, bonds, inventory, resources, energy, ledger entries, exposed features, and analysis caches together. A failed transaction leaves the organism unchanged.

### 3.5 Developmental policy

The policy selects the next objective and chooses among legal candidates. During genome discovery, it favors feasible growth toward a qualifying cavity. Once a genome is confirmed, it stops optimizing for enclosure and pursues remaining viability requirements.

### 3.6 Verification and handoff

A final validator checks the realized organism and its ledgers. The constructor returns the validated structure in the representation expected by the simulation. It must not silently repair invalid physics at handoff.

## 4. Geometry Reference Library integration

### 4.1 Audit the existing catalogue first

Inventory the files, schemas, versions, record counts, material combinations, shape definitions, connection features, arrangement sizes, provenance, and validation status. Distinguish raw samples from validated arrangements. Record duplicates, malformed records, missing coverage, and version conflicts. Do not assume every stored sample is valid in every context.

### 4.2 Separate stored knowledge from validation guarantees

Track whether an entry is unverified, locally validated, compositionally validated, or validated in a particular context. Only records meeting the library's explicit verification contract may be used as constructor candidates. An arrangement valid in isolation can still collide with a distant part of the organism when inserted into a larger structure; library verification is candidate provenance, not permission to skip live validation.

### 4.3 Index for actual constructor queries

Index by useful physical features: material composition, shape/constituent types, exposed connection feature, feature scale, orientation, local envelope, arrangement complexity, and validation status. Each index must answer a demonstrated query and be reproducible from authoritative data.

### 4.4 Instantiate known placements; do not invent runtime geometry

When a connection feature is exposed, query Bob for persisted records that encode compatible contact features and relative placements. Instantiate/rebase those recorded arrangements against the current anchor using the record's defined coordinate frame and orientation rules. The strict constructor must not fall back to blind coordinate sweeps, locally generated unpersisted contact families, scaffold coordinates, or arbitrary pose generation when no record matches. Enforce the approved contact tolerance of 0.1 where applicable; touching is permitted, penetration is forbidden. Respect feature-scale compatibility and the specified bond-strength distinctions for edge, corner, and line-end contacts. Bounding boxes and spatial indexes may reject obviously irrelevant candidates, but cannot serve as final proof of contact or nonpenetration.

Resolve the existing constructor's 0.05 tolerance against the approved 0.1 value at the shared authority; do not patch only one local constant. Candidate diagnostics must identify the source formation/contact record and the live validation outcome.

### 4.5 Compose known arrangements into larger structures

Verified singleton and multi-constituent arrangements are reusable local building blocks, not mandatory biological organs. The planner may combine their recorded placements to create a larger structure that Bob has never seen as a complete formation. Each composition must reconcile the shared interfaces, constituent transforms, contact graph, and exposed features; validate every external interface and the full contextual placement. A composite is an optional planning shortcut: the planner can reject it, choose another known arrangement, or continue with smaller known records. Composition does not permit arbitrary new internal geometry; every constituent arrangement and placement relationship must be library-supported.

### 4.6 Query using current context

Candidate lookup must account for actual exposed features, available materials, unmet milestones, existing structure, and feasible resource-acquisition paths. Do not scan the entire catalogue without regard to the current construction state.

### 4.7 Treat missing coverage as a first-class result — no runtime fallback

A missing catalogue entry must never cause the constructor to generate an unrecorded candidate or use a hard-coded placement. Return a structured result such as `MissingLibraryCoverage`, including the query/contact class, anchor feature, material/shape requirements, rejected record IDs, and library version. Distinguish this from `KnownCandidatesRejectedByLiveContext`, chemical incompatibility, and proven absence of a feasible path under the searched conditions. Record the coverage gap for the separate Bob expansion process. After Bob discovers, verifies, persists, and versions additional records, rerun planning against that library version. Do not run open-ended discovery inside a simulation tick.

## 5. Chemistry Library integration

### 5.1 One authoritative source for material properties

Read mass, potential energy, reactivity, cohesion, and other material properties from the same authoritative definitions used by the simulation. Do not duplicate property tables in the constructor. `potential_energy` is the absolute maximum for a resource type; it is not a mutable `energy_content` value.

### 5.2 Centralize compatibility and bond consequences

For each candidate, ask the authoritative chemistry/bond system whether the material pair and contact features may bond, what strength applies, what energetic consequences follow, and which accounting entries are required. Repair an incomplete authority at its source rather than adding constructor-only exceptions.

### 5.3 Separate computation budget from physical energy

The old `ASSEMBLY_ENERGY = 1.0e12` pattern and deriving initial organism energy from the remaining assembly budget must not be carried forward. Computation/search limits are not physical energy. Account for real construction costs, transfers, reserves, and transformations according to the energy ledger; return only energy actually justified by the model.

### 5.4 Preserve physical acquisition

Environmental abundance does not permit direct insertion of logical resource labels. Resource acquisition must use the actual physical acquisition semantics and placement checks. Water and at least three distinct non-Water categories must be acquired and accounted for.

### 5.5 Distinguish availability from suitability

For each candidate, separately evaluate whether a material is available, eligible for the action, chemically compatible, geometrically placeable, and affordable under the applicable constraints. This distinction supports both abundant initial construction and constrained offspring construction.

## 6. Milestone-driven construction algorithm

### Step 1 — Initialize explicit state

Track the realized structure, constituent positions, valid bonds, exposed features, material inventory, acquired resources, energy and material ledgers, genome-analysis status, current milestone, deterministic random seed/state, and diagnostics. Do not silently insert a completed genome, unearned resources, or a prescribed final body plan.

### Step 2 — Identify the current milestone

Begin with a valid physical starting condition. Discover a qualifying genome cavity. Then satisfy the remaining organism requirements. Milestones describe biological conditions, not piece counts or exact arrangements.

### Step 3 — Query and compose only known geometry

Query Bob for persisted, verified singleton or formation records matching the current exposed features, material/shape constraints, and outstanding milestone. Instantiate the recorded relative placements against the live anchor and compose compatible known arrangements into larger candidate growth steps. The constructor may enumerate combinations of known records, but it must not invent arbitrary coordinates, rotations, contact families, or scaffold fallbacks. Chemistry eligibility and material selection are evaluated through their authoritative systems. Candidate generation is demand-driven: missing resource requirements should trigger relevant known acquisition opportunities instead of unrelated growth. If no recorded candidate matches, return a missing-coverage result for the separate expansion loop.

### Step 4 — Validate before commitment

Geometry checks must confirm valid shapes, contact, scale compatibility, nonpenetration, and consistency with the whole relevant structure. Chemistry checks must confirm material eligibility, bond validity/strength, and energy/accounting consequences. Developmental checks must confirm the candidate serves or preserves the current milestone and does not needlessly destroy all feasible routes to outstanding requirements.

The existing `penetrates_local_neighborhood()` check against only the anchor and its direct bonded neighbors is not a sufficient whole-structure guarantee unless a proven invariant makes it complete. Use spatial indexing for efficiency, but preserve complete relevant collision detection.

### Step 5 — Plan over known arrangements and backtrack before commitment

Greedy local growth can block later completion. Search a bounded, explicit planning space formed from Bob's known placement records and combinations of those records. During genome discovery, avoid candidates that demonstrably eliminate all feasible enclosure paths. During resource acquisition, avoid blocking all feasible placements for required resources. Preserve alternative branches and backtrack over uncommitted plans when a later constraint fails; do not undo already committed physical history unless a separately defined physical transaction explicitly supports rollback before visibility.

Lookahead estimates feasibility; it does not know or prescribe the final organism. It must not be replaced by a fixed piece count, a hardcoded ring, a mandatory scaffold, or arbitrary attempt/node caps treated as biological facts. If an operational computation budget is exhausted, report that the planning effort was exhausted. If the finite known-record search is exhausted, distinguish that result from proven physical impossibility and report whether the likely blocker is library coverage, context incompatibility, chemistry, or viability.

### Step 6 — Select without catalogue-order bias

Reject invalid candidates, compare remaining candidates according to explainable milestone and feasibility criteria, and use seeded randomness among comparable alternatives. File order and material enumeration must not silently dictate the organism. Any material or topology preference must be justified by actual constraints or an explicit approved policy.

### Step 7 — Commit atomically

Commit geometry, bonds, inventory, resource placements, energy, ledgers, exposed features, and cache updates as one transaction. If any required operation fails, roll back the entire transaction. No partial state may become visible to the simulation.

### Step 8 — Re-evaluate after relevant changes

Run cavity analysis after changes that can create or destroy a cavity or alter its seal. Cache analysis only with sound invalidation rules. Re-evaluate outstanding resource and viability requirements after relevant physical changes.

### Step 9 — Stop on actual viability

Return only when the realized organism passes all required checks. If the process stalls, report unmet requirements and blocking causes. Do not keep growing toward an arbitrary size hoping viability will emerge.

## 7. Genome discovery is a real milestone

There is no predefined genome core. A cavity qualifies only when the authoritative analyzer finds it in the realized structure and confirms the required bonded seal. An unsealed or accidental cavity is not a genome.

Run or refresh analysis when geometry or bonding changes can affect qualification. When a qualifying cavity is confirmed, record the actual result, switch milestones, and immediately stop treating enclosure as a construction objective. Do not make genome bonds indestructible or introduce special cavity-protection physics unless separately required by the physical specification.

## 8. Physical resource acquisition

Track which required categories are available, attempted, acquired, and currently represented. Use the authoritative placement and contact rules, not sampled bounding-box positions as final proof. Confirm Water and at least three distinct non-Water resource categories are physically acquired.

Respect the existing fluid rules: Water is logically abundant but physically represented as required; it remains permeable rather than a rigid wall; the genome cavity interior is exempt from water fill; and water-only sequences must not count as valid structural connections where a non-water connection is required. These rules belong to shared physical authorities, not constructor-specific exceptions.

## 9. One engine, two policies

### Initial-organism policy

Start without a required blueprint, use the abundant environment while obeying real acquisition rules, discover a qualifying genome, acquire the required resources, and stop promptly when viable.

### Offspring policy

Start from the inherited guidance and limited acquired material actually available. A blueprint is a soft preference, not a command. Permit imperfect development and valid deviations. Previously discussed similarity thresholds (>0.90 for early commitment and <0.6 for refusal/reconsideration) may be used only after the metric and semantics are documented and reconciled with the specification; they are not physical laws.

Both policies must share geometry lookup, chemistry/bond rules, validation, transactions, acquisition, ledgers, and diagnostics. Do not retain a separate blueprint renderer with different physical assumptions.

## 10. Invariants

### Geometry
- No committed change introduces forbidden penetration.
- Contact tolerance and feature-scale compatibility follow the approved rules.
- Bonds correspond to valid contact and chemistry.
- New additions cannot silently corrupt existing structure.
- Spatial indexes and caches do not weaken completeness or consistency.

### Chemistry and accounting
- Material properties and bond rules have one authoritative source.
- Every material and energy change is recorded.
- Search budget is not converted into physical energy.
- Failed transactions leave accounting unchanged.
- Ledgers reconcile with the realized state.

### Biology
- The real analyzer establishes genome qualification.
- A qualifying genome has a bonded seal.
- Enclosure stops being a goal after qualification.
- Required resource categories are physically acquired.
- Final accessibility and structural requirements are verified.

### Transaction
A candidate produces either the original unchanged state or the complete validated new state—never a partially committed intermediate state.

## 11. Required removals and cleanup

The rewrite must remove obsolete architecture, not simply wrap it.

1. **Fixed-size viability target:** remove the strategy of building to a target such as 400 pieces before checking viability. Delete dependent constants, retry logic, comments, and tests that make that target a biological requirement.
2. **Prescribed initial topology:** remove mandatory rings, scaffolds, spirals, central materials, and topology-specific branches unless an independently documented physical requirement proves they are necessary. Remove tests that equate one topology with success.
3. **Legacy seed/calibration path:** trace all uses of `confirmed_seed_baseline()`, `confirmed_seed_scale_reference()`, and `juvenile.rs`. If a calibration artifact still serves a legitimate scaling purpose, isolate and document it; remove its influence on live initial construction. Do not delete dependencies blindly.
4. **Duplicate geometry logic:** remove constructor-owned shape/contact/collision rules, duplicate tolerances, independent feature calculations, and sampled placement checks used as final validation. Route queries and final validation through the authoritative geometry system.
5. **Duplicate chemistry logic:** remove local material tables, bond-strength rules, compatibility exceptions, and energy calculations that compete with chemistry/bond authorities.
6. **Approximate resource placement as authority:** replace coarse-grid/four-rotation sampling as final proof with library-backed candidates and authoritative placement validation.
7. **Artificial assembly energy:** remove the conversion from `ASSEMBLY_ENERGY = 1.0e12` or remaining search allowance into initial organism energy. Preserve legitimate costs/reserves through the ledger.
8. **Catalogue-order commitment:** remove early-commit paths that choose the first candidate solely because it appears first. Add normalized candidate ordering and seeded, explainable selection.
9. **Arbitrary brute-force patterns:** remove repeated reconstruction of identical candidates, dead-end retries, redundant analysis, and arbitrary limits treated as biological facts. Keep operational safeguards only when they report exhaustion honestly.
10. **Duplicate construction entry points:** migrate all runtime, test, benchmark, and utility callers to one physical engine with explicit policies, then delete obsolete constructors and temporary adapters.
11. **Obsolete tests:** rewrite tests that assert old fixed size, prescribed topology, or retired reproduction rules. Do not delete valid physical requirements merely because old tests fail; retain and update tests for bonded cavities, acquisition, nonpenetration, bonding, and accounting.
12. **Obsolete documentation:** mark all prior constructor plans as superseded and remove contradictory instructions once their historical context is no longer needed.

Remove old code only after its callers and dependent invariants have been mapped. Do not remove the geometry or chemistry libraries as part of constructor cleanup.

## 12. Migration sequence

### Phase 1 — Source-of-truth audit
Identify the intended source branch; inventory constructor, simulation integration, geometry and chemistry libraries, bonding, cavity analysis, acquisition, and energy ledger. Map callers and duplicate authorities. Record what is a specification requirement versus an artifact of the old architecture.

**Deliverable:** dependency map and removal checklist.

### Phase 2 — Populate and independently validate a bounded bootstrap library
The checked-in geometry catalogue is intentionally empty. Run Bob's bounded generation worker for singleton and two-constituent formations only (maximum constituent count = 2) in an isolated output location. Verify completion, deduplication/canonicalization, family coverage, no records above the limit, persistence/reopen, schema/version consistency, and artifact integrity. Measure generated candidates, canonical duplicates, geometry/topology rejections, family counts, elapsed time, and storage size. Keep generated records as a versioned runtime library or workflow artifact; do not commit the generated catalogue as source data. Treat this catalogue as a bootstrap baseline, not exhaustive geometric coverage.

Then validate representative records, contact-feature and scale coverage, strict candidate lookup, rejection of invalid candidates, catalogue-order independence, and test isolation. Include fluid-specific coverage: fluid-boundary solver outputs, persisted deformation/contact descriptors, feasible parameter ranges, conserved-volume checks, fluid–rigid and applicable fluid–fluid interface classes, and reopen/query round trips.

**Deliverable:** reproducible two-constituent bootstrap artifact plus a library quality/coverage report and repeatable tests.

### Phase 3 — Strict query and planning interfaces
Implement narrow interfaces for verified-record geometry queries, composition of known arrangements, chemistry eligibility, contextual validation, energy/accounting, atomic commit/rollback, missing-coverage reporting, and rejection diagnostics. Make strict library-backed mode the only mode used by live construction; any geometry-generation APIs remain confined to Bob's separate discovery worker.

**Deliverable:** every proposed placement has traceable library provenance and can be accepted/rejected without mutating the organism.

### Phase 4 — Transactional growth
Prove atomic commits and rollback, bond/geometry consistency, and ledger reconciliation before full organism construction.

**Deliverable:** valid sequences of changes preserve invariants; failed changes leave state unchanged.

### Phase 5 — Genome milestone
Implement blueprint-free growth using the real cavity analyzer. Switch the objective immediately when a qualifying cavity is found.

**Deliverable:** logs identify the physical change that created the qualifying genome and confirm the policy transition.

### Phase 6 — Acquisition and viability
Integrate Water and three distinct non-Water categories through real acquisition and verify placements and accounting.

**Deliverable:** an explicit record of satisfied and outstanding viability requirements.

### Phase 7 — Comparative benchmark
Compare the prototype against the old implementation over multiple seeds, material conditions, and catalogue orderings. Measure correctness, success, time, candidate work, diversity, and failure reasons.

**Deliverable:** comparative report.

### Phase 8 — Live replacement and cleanup
Connect the new engine to live initial-organism creation, verify the simulation's physical representation and handoff, then migrate offspring policy where supported. Remove superseded code, constants, tests, and documentation. Run formatting, build, focused tests, integration tests, and the full suite; inspect the final diff.

**Deliverable:** one authoritative engine and an auditable migration record.

## 13. Benchmark and acceptance criteria

The replacement is not accepted merely because it compiles or produces one plausible organism.

### Physical correctness
Prove nonpenetration, valid bonds, analyzer-confirmed bonded cavity, physical resource acquisition, correct fluid behavior, reconciled ledgers, atomic rollback, and correct simulation handoff.

### Multiple seeds
Record success/failure per seed, failure reasons, time to cavity, time to viability, candidates generated and checked, rejection categories, and committed changes. Choose a meaningful test sample before interpreting the success rate.

### Catalogue-order independence
Reorder library records and material enumeration. Results may differ due to intentional seeded choices, but must not be systematically controlled by file order. Define deterministic candidate normalization and seeded selection.

### No hidden topology preference
Inspect outcomes across seeds and material variations. Any recurring topology must be explainable by physical constraints or an explicit policy, not a scaffold, file order, or scoring bug.

### Performance and library leverage
Measure wall-clock time, candidate count, full geometry checks, chemistry checks, cavity analyses, failed transactions, memory use where measurable, query hit rate, reusable-arrangement rate, and fallback use. The objective is fewer unnecessary decisions and earlier recognition of success.

### Simulation integration
Verify physical materials, bonds, positions, genome result, resources, and energy survive handoff and that ordinary movement, acquisition, processing, transformation, and reproduction can operate on the organism.

## 14. Failure reporting

Use explicit categories: no valid growth opportunity, no compatible geometry candidate, no permitted chemical connection, collision with existing structure, no feasible enclosure route found, blocked resource acquisition, insufficient material/energy, operational search budget exhausted, final invariant failure, or missing/unusable library coverage.

Distinguish proven impossibility under checked conditions from failure to find a solution within the available search effort. Never report the latter as proof that no viable organism exists.

## 15. Risks and mitigations

- **Greedy growth traps itself:** use bounded feasibility lookahead and maintain alternatives where useful.
- **Incomplete or stale catalogue:** validate records, track coverage, report missing knowledge, and expand the library in a separate bounded Bob run; do not permit runtime geometric fallback.
- **Geometry/chemistry disagreement:** test cross-library combinations and resolve conflicts in authoritative definitions.
- **Scoring favors one body plan:** keep scoring explainable; vary seeds, catalogue order, and materials.
- **Whole-structure validation is expensive:** use spatial indexes without weakening collision completeness.
- **Two constructors remain active:** map callers, migrate one entry point, and remove obsolete paths after acceptance.
- **Focused tests hide integration failures:** test library, transaction, constructor, handoff, and full-suite layers.

## 16. Implementation status and source discipline

This plan is approved architecture, not proof of implementation. Source inspection, build success, focused tests, and full end-to-end viability are separate evidence levels. Report exactly which checks ran and which did not.

The repository has used separate branches for the geometry data and Rust source. Confirm the intended source branch and reconcile library version/schema before integration. The GitHub geometry library is the practice environment; do not alter the production library or merge the replacement merely because a prototype appears promising.

## 17. Definition of done

- [ ] Geometry and chemistry schemas, coverage, and authoritative rules audited.
- [ ] Library queries reuse valid arrangements without treating samples as universally valid.
- [ ] Every construction candidate traces to a persisted, verified Bob record and its library/schema version.
- [ ] The constructor composes known arrangements into larger structures without requiring Bob to have catalogued the whole organism.
- [ ] Runtime construction has no unrecorded geometry-generation, scaffold-coordinate, or arbitrary-pose fallback.
- [ ] Missing library coverage is a structured result that can trigger separate Bob discovery and a planner retry.
- [ ] A reproducible isolated singleton/two-constituent bootstrap library has been generated and audited; its temporary nature and coverage limits are documented.
- [ ] Candidate validation uses authoritative physical rules.
- [ ] No fixed piece-count viability target.
- [ ] No required final topology.
- [ ] Candidate commits are atomic and fully accounted for.
- [ ] Genome qualification comes from the realized structure and actual analyzer.
- [ ] Enclosure stops being a goal immediately after qualification.
- [ ] Required resources are physically acquired.
- [ ] Multiple deterministic seeds and catalogue-order variations tested.
- [ ] Hidden topology bias investigated.
- [ ] Energy and material ledgers reconcile.
- [ ] Simulation handoff and downstream behavior verified.
- [ ] Initial and offspring policies share the physical engine.
- [ ] Superseded constructor paths, duplicate authorities, obsolete tests, and conflicting documentation removed.
- [ ] Formatting, build, focused, integration, and full-suite results recorded.
- [ ] Final code diff and migration record reviewed before production integration.

## 18. Final design rule

**Use Bob's verified library to propose known local arrangements, compose those arrangements into larger structures, use authoritative rules to decide which steps are physically valid, and use biological milestones to decide when construction is finished. When known coverage is insufficient, expand Bob's library separately rather than inventing geometry inside the live constructor.**

The first implementation task is the library/dependency audit, followed by a fresh bounded singleton/two-constituent Bob generation run and strict-query contract tests. Then build a small compositional, transactional growth prototype. Replace live construction only after the prototype demonstrates that every candidate comes from known records, physical invariants are preserved, missing coverage is reported honestly, and viable organisms can be built repeatably without a prescribed shape or fixed-size assembly.

---


### Geometry-library reset follow-up — 2026-10-10

The generated geometry data files were intentionally deleted from the current `main` and `integration/unified-source-and-catalogue` branch heads because the old snapshot was judged obsolete. Source generation rules and `src/bob/library.rs` remain on the integration branch. No replacement data was generated. Source inspection indicates an empty geometry root can be opened without seeding it, but that behavior still needs an isolated non-generative test; do not use a worker run as a smoke test. The constructor does not currently load Bob, so the next critical bridge is a tested query that returns useful placement suggestions while leaving all physical validation to the live runtime. See [the updated stabilization audit](docs/repository-stabilization-audit.md) for findings and ordered next steps.


### Bob lookup correction and current verification — 2026-10-10

A source inconsistency was found in rigid-edge lookup: generated candidate rotations were stored in the formation frame but compared against live candidate-minus-anchor rotations. The resolver now converts stored rotations to the anchor-relative frame, and the regression test for a rotated anchor passes. An isolated empty-store test also passes without seeding any geometry. CI's worker-generating smoke step has been replaced with that non-generative test; see [the latest Rust run](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38060589759) and [the stabilization audit](docs/repository-stabilization-audit.md). The full suite remains failing (**264 passed, 75 failed, 1 ignored**) and strict Clippy remains red. Bob still does not supply actual placement suggestions to the live constructor, so this is a lookup correctness fix, not completion of constructor integration.


### Rigid-edge interval correction — 2026-10-10

The persisted rigid-edge interval now means the normalized exposed interval on the anchor edge where a contact point may lie. The generator stores that interval directly, and loader/insertion validation enforce `[0, 1]`. The geometry-library schema is now version 2 so old generated records cannot be silently reinterpreted; the separate live chemistry interface-key format remains `live-v1`. The new interval, rotated-anchor, and empty-root tests passed on the preceding CI run; the latest run after decoupling the version constants is pending. Bob's canonical projection keys now omit formation-context constituent indices, with regression assertions that equivalent local interfaces collapse. Bob still needs a constructor-facing API that returns actual placement suggestions. See [the detailed interval audit](docs/repository-stabilization-audit.md).


The latest source follow-up also removes `anchor_constituent` from rigid-edge, rigid-point, and rigid-vertex canonical projection keys. It is a formation-context index, not local interface geometry, so retaining it could falsely turn equivalent contacts into ambiguous matches. Tests for this change are pending in CI.


### Bob placement-suggestion milestone — 2026-10-10

Bob now has an indexed `suggest_rigid_edge_placements` API that rebases persisted rigid-edge families to a live anchor pose and returns deduplicated candidate `Placement` suggestions. These are advisory only: the physical constructor must still validate full-structure nonpenetration, contact, bond transaction, acquisition, energy, and cavity requirements. This is a first integration primitive, not yet wired into the constructor; edge-edge is the only suggestion class implemented, and interval-wide placement exploration plus point/vertex/fluid suggestion paths remain. The regression test checks pose rebase and edge-midpoint alignment. CI is being rerun for the latest changes. See [the detailed stabilization audit](docs/repository-stabilization-audit.md).


The placement API now also supports the empty-library bootstrap case: when no matching persisted family exists, it derives only the requested rigid-edge families in memory and returns advisory placements without writing rows or starting a geometry worker. This permits local proposal generation while the checked-in catalogue remains empty. The renamed regression test verifies that an empty store remains empty and that the resulting candidate-edge midpoint is aligned to the live anchor contact point. CI is pending for this follow-up. The API remains advisory and is not yet wired into the initial constructor.


Verification checkpoint: on commit `7bfb6f21`, the empty-library bootstrap test passed and the full suite reported **266 passed, 75 failed, 1 ignored**. The existing simulation/physics/reproduction failure backlog and strict-Clippy failures remain unresolved. The newest test-only extension covers the unequal Phosphorus trapezoid edges and is still in CI. See the [stabilization audit](docs/repository-stabilization-audit.md).


**Superseded checkpoint (2026-10-10 15:23 UTC):** This earlier note said to diagnose transaction `19-3` before wiring Bob into the constructor and said the catalogue was empty/no worker had run. The approved two-piece generation has since completed, and Bob's rigid-edge suggestions have since been connected to the blueprint-driven runtime. See the current checkpoint below for the corrected scope; this historical note is retained only to preserve the sequence of decisions.
**Completed bounded Bob generation:** The approved one-/two-constituent generation ran successfully in [GitHub Actions run #38066184273](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38066184273). It persisted 279 formations (7 singletons, 272 two-piece), added 272 formations after deduplication from 435 candidates, and left no unfinished eligible frontiers or formations above two constituents. Persistent size was 48,316,400 bytes; compressed artifact `bob-two-piece-geometry-library` (ID `11674718881`) expires 2026-11-09. Runtime-generated data was not committed. This generation validates the bounded worker only; it does not prove constructor viability.


### Current constructor integration checkpoint — 2026-10-10

Bob's rigid-edge suggestion API is now called by `src/construction/runtime.rs` during blueprint-driven bond construction. Suggestions are candidate poses only and still pass through the runtime's live physical and COMBINE checks. However, source tracing found that the currently active blueprint-free initial-organism path is separate: `construct_valid` calls `construct_scaffold` in `src/construction/initial_organism_constructor.rs`, which fixes Carbon ring/spoke placements and calls a local `bond_units` helper without querying Bob. Consequently, the current `physical bond transaction 19-3 failed` error is **not yet testing Bob-informed placement**. The fixed scaffold remains a temporary baseline, not the approved replacement architecture.

The CI run for the Bob runtime integration passed formatting, source-size, COMBINE architecture, Bob compact-storage tests, and empty-store open/reopen checks. The Rust suite remains **266 passed, 75 failed, 1 ignored**; many failures cascade from the initial-organism construction error, and other independent failures exist. Strict Clippy fails with dozens of ordinary lint findings and stale `#[expect(dead_code)]` expectations; no blanket suppression is appropriate.

**Next implementation step:** integrate Bob-informed candidate placement into the active blueprint-free growth path (ultimately replacing the fixed scaffold), then add a focused test proving a Bob proposal enters that live path and reaches full-structure validation/COMBINE admission. Do not claim the transaction is diagnosed or fixed until that path is exercised; do not weaken physical constraints. The detailed source-path correction and readable CI findings are recorded in [the stabilization audit](docs/repository-stabilization-audit.md).

### Initial constructor integration and geometry root cause — 2026-10-10

The blueprint-free path now queries Bob for candidate placements in `src/construction/initial_organism_constructor.rs`. A focused regression confirms that an actual suggestion reaches whole-structure nonpenetration and live contact validation; Bob remains advisory, and the normal COMBINE transaction still authorizes each bond. Symmetry-equivalent Carbon rotations are canonicalized and near-equivalent proposals are snapped to the intended lattice site before being physically rechecked. A (10^{-9})-unit numerical guard is used only for floating-point boundary equality.

The former `physical bond transaction 19-3 failed` was traced to the scaffold's `hex_ring` direction sequence: it left the requested perimeter and placed spoke 19 on top of inner-ring unit 3. The axial direction sequence is corrected, with a regression test asserting unique perimeter cells for radii 1–5. That exposed a second issue: `find_enclosed_regions` inserted duplicate copies of shared polygon edges into its planar graph, preventing face traversal for the hexagonal shell. Shared undirected edges are now deduplicated before face walking.

On CI run [#38068822004](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38068822004), formatting, source-size, COMBINE-architecture, Bob storage, and empty-store checks passed; the suite improved to **311 passed, 32 failed, 1 ignored**. The initial-organism constructor tests no longer appear among the failures. This does **not** mean the full suite is green: blueprint-driven closure tests still report zero contacts in some paths, and unrelated chemistry, movement, reproduction, and other contract tests remain failing. Strict Clippy is still red. Next, audit the remaining blueprint-driven contact/closure path rather than weakening geometry or sweeping through unrelated legacy failures. See [the stabilization audit](docs/repository-stabilization-audit.md).

### Multi-neighbor closure follow-up — 2026-10-10

The blueprint-driven placement stage now rejects candidate poses that cannot make contact with every already-realized neighbor declared for the current element. This is checked against the trial structure before committing the first bond, so the constructor does not greedily lock an element into a pose that makes a later closure impossible. The existing triangle-closure and multi-constraint triangle tests now pass, and the full suite improved to **313 passed, 30 failed, 1 ignored** on [Rust run #38069059297](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38069059297). Formatting and the Bob library checks passed. Strict Clippy and the remaining 30 tests still fail; the intact-composite construction test remains a priority because it is part of the physical-material contract. This checkpoint supersedes the earlier instruction to diagnose the triangle closure path; do not weaken contact/overlap constraints to address the remaining composite case.

### Test-failure reduction investigation — 2026-10-11

The reduction from the October 9 baseline of **244 passed / 75 failed / 1 ignored** to the October 11 result of **314 passed / 29 failed / 1 ignored** is real; it is not explained by removing tests. The newer run executes 344 tests versus 320 previously, and comparison of test names shows 46 formerly failing tests now pass, with no old failures removed from the suite and no newly failing tests. The older run's 75 failures included **52 panics at `src/simulation/simulation.rs:61`**, where default simulation setup calls `construct_valid(...).expect("initial organism constructor must find a valid organism")`. Those failures were cascading initialization failures, not 52 separate movement/observation/runtime defects. No tests now panic at that simulation-initialization line.

The main historical change is geometry correctness, not the recent lint cleanup. The scaffold's axial `hex_ring` walked through the center, overlapping spoke 19 with inner-ring unit 3; correcting the direction sequence removed that overlap. This exposed a second defect in `find_enclosed_regions`: shared polygon edges were inserted twice into the planar graph, which could prevent face traversal and cavity detection. The graph now deduplicates undirected edges before adding a directed pair. At commit `4b1c6b895e`, the suite moved from **267 passed / 75 failed** at `058285ed0c` to **311 passed / 32 failed**; the initial-organism constructor tests disappeared from the failure list. Multi-neighbor blueprint placement then improved the suite to **313 / 30**, and the latest exact-head run [#38141473000](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38141473000) reports **314 passed / 29 failed / 1 ignored** at `1ea08455b6276328ace59cc454b6229ddb6332c1`.

This does **not** mean the blueprint-driven constructor is fixed. The remaining suite has a repeated, more specific failure: `no forward bond-driven placement found for blueprint element 9 after 13877 placement attempts`. That same error cascades into cavity, harmonics, viability, decomposition, and simulation tests. Treat those as one shared seed-realization blocker until proven otherwise; do not diagnose each downstream panic as an independent defect. Other remaining failures are independent candidates (including intact-composite placement, chemical-break energy threshold, contact normals, fluid boundary behavior, COMBINE heat accounting, reproduction resumption, and storage counts) and should be triaged separately after the shared seed failure is understood. Constructor-related test failures are expected during this investigation, but do not disable tests or weaken invariants.


### Bob Water-diagnostics checkpoint — 2026-10-11

Water-contact generation now exposes reason-level counters for unsupported anchor geometry, capillary-solver failures, anchors with no exposed intervals, invalid contact intervals, and exposed intervals without a valid capillary overlap. Focused regression coverage includes a valid Water–Carbon boundary and unsupported anchor geometry. On exact-head workflow [#38149992273](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38149992273), formatting, source-size policy, COMBINE architecture, Bob compact-storage tests, and empty-library open/reopen checks passed. The full Rust suite reports **317 passed, 29 failed, 1 ignored**; the remaining failures span known construction/seed, chemistry, cavity, movement, reproduction, and storage contracts and are not, by themselves, evidence of a Water-diagnostics regression. Strict Clippy also fails on existing project-wide lint findings and staged-API/dead-code expectations. Treat this as a diagnostics checkpoint, not full-suite validation or fluid-geometry acceptance. Keep broad catalogue generation gated. Next: validate each diagnostic counter against a controlled supported/unsupported fluid-boundary case, verify persisted fluid records against area/contact/boundary invariants, and only then run bounded two-constituent generation.


### Water diagnostics semantics correction — 2026-10-11

A review found that an anchor whose resource name is absent from the catalog was being counted as unsupported geometry. That conflated resource-reference resolution with a known resource shape that the generator does not support. Bob now reports unresolved anchor resources separately from unsupported known-resource geometry. A focused regression asserts this distinction. The latest code needs exact-head CI; no geometry generation was run and the catalogue remains untouched.


### Exact-head Water diagnostics CI follow-up — 2026-10-11

On exact head `1646969ebf7837186da6c7fc2496029341b7ff4d`, Rust workflow [#38150859185](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38150859185) passed formatting, source-size policy, COMBINE architecture, Bob compact-storage tests, and empty-library open/reopen without generation. The full Rust suite reports **318 passed, 29 failed, 1 ignored**. The focused Bob compact-storage and empty-store checks passed; the Water–Carbon persistence/query regression and resource-resolution/unsupported-shape diagnostics are part of the Rust suite, but overall suite failures remain across construction/seed, chemistry, COMBINE, contact normals, cavity, movement/fluid boundary, reproduction, and storage contracts. Strict Clippy remains red with 52 application-target and 41 test-target diagnostics. Runner Validation [#38150859183](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38150859183) passed. This checkpoint is not fluid-geometry acceptance and does not justify broad catalogue generation.

**Next focused step:** add controlled tests for the remaining reachable diagnostic branches and assert emitted records as well as counters. Avoid manufacturing invalid formations just to hit counters; if a branch cannot be reached through a physically valid fixture, document that as a coverage/design gap. Then validate fluid record invariants and reopen/query identity before the bounded two-constituent run.


### Malformed fluid-boundary persistence checks — 2026-10-11

At code head `4e5c922d0f5d9629f6a335fe8fb17e46a924ad3d`, fluid-boundary persistence regression coverage now explicitly rejects an edge interval ending beyond 1.0 and a non-finite contact angle, while retaining the valid Water–Carbon persistence/reopen case. Rust workflow [#38151236466](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38151236466) passed formatting, source-size policy, COMBINE architecture, Bob compact-storage tests, and empty-library open/reopen without generation. The full suite reports **318 passed, 29 failed, 1 ignored**; the fluid-boundary persistence/query test passed. Remaining suite failures span existing cross-subsystem contracts, including movement's fluid-boundary behavior, so this is not end-to-end fluid acceptance. Strict Clippy remains red on project-wide lint/staged-API findings. Runner Validation [#38151236246](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38151236246) passed. The library remains empty; no broad generation was run.

**Next short sprint:** test rejection of malformed persisted fluid-formation anchor indices and non-finite contact parameters, then rerun the focused Bob checks. Keep bounded two-constituent generation gated until persistence, identity-after-reopen, and the fluid boundary's non-wall semantics are separately understood.


### Fluid-formation rejection regression — 2026-10-11

Exact-head source commit `63942b003225ac61091bf5cddbdc6c3e436e69a2` adds focused rejection assertions for a fluid formation whose anchor edge index is out of range and whose contact angle is non-finite. Rust workflow [#38152433505](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38152433505) passed formatting, source-size policy, COMBINE architecture, Bob compact-storage tests (7 passed), and empty-library open/reopen without generation (1 passed). The fluid-boundary persistence/query regression containing the new assertions passed in the full suite. Full Rust suite: **318 passed, 29 failed, 1 ignored**; failures remain across existing cross-subsystem contracts, including fluid movement/wall semantics. Clippy remains red on broad existing lint findings. Runner Validation [#38152433516](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38152433516) passed. This verifies rejection at insertion and stable valid-record reopen, not resilience to malformed rows manually written to disk nor end-to-end fluid physics. Geometry catalogue remains empty and broad generation remains gated.

**Next:** inspect malformed-on-disk load behavior and test it only if it reveals a real storage contract gap; separately audit whether fluid boundary records feed movement as contact geometry without imposing a wall. Do not begin two-constituent generation until that semantic gap is understood.


### Malformed persisted fluid-formation load regression — 2026-10-11

At exact-head commit `98f95f2080400a750c155488101570345b9d54ee`, the fluid persistence regression now appends an invalid fluid-formation row directly to `fluid_formations.jsonl`, reopens the library, and checks that the malformed row is discarded while valid records and signatures survive. Rust workflow [#38152841443](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38152841443): formatting, source-size policy, COMBINE architecture, Bob compact-storage tests (7 passed), and empty-library open/reopen (1 passed) all passed. Full Rust suite: **318 passed, 29 failed, 1 ignored**; the 29 failures are the established cross-subsystem failures, including movement/fluid-boundary and construction/viability contracts. Clippy remains red on existing broad lint/staged-API findings. Runner Validation [#38152841312](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38152841312) passed. This confirms malformed persisted-row rejection and valid-record reopen at the storage layer only; it does not establish that Bob's fluid sidecar drives runtime movement. The geometry library remains empty and broad generation remains gated.

**Next audit:** trace whether `GeometryFluidFormation` has any runtime consumer and keep that distinct from movement's realized `Form::Fluid` boundary handling. Do not change movement semantics or start two-constituent generation until the consumer boundary is explicit.


### Bob-to-construction consumer audit — 2026-10-11

Source audit on the current integration branch establishes two distinct states:

- **Rigid Bob suggestions are already connected to construction.** `src/construction/initial_organism_constructor.rs` calls `suggest_rigid_edge_placements` and treats each pose as a proposal only; whole-structure penetration/contact checks and subsequent transaction validation remain authoritative. `src/construction/runtime.rs` also calls the same rigid suggestion API before live overlap, contact, compatibility, and energy checks.
- **Fluid formation records now have a narrow construction consumer.** `src/construction/runtime.rs` queries Bob's stored fluid formations against an exact canonical single-rigid-unit parent, reconstructs the conserved-area arc, transforms the pose into the live parent's frame, and carries the shape through `PhysicalMaterial::with_single_fluid_shape`. Both arc orientations are proposals; they still pass through the existing live overlap, neighbor-contact, bond-compatibility, and energy transaction checks. This first bridge does not yet map multi-constituent live parents to canonical Bob identities, and an end-to-end accepted construction has not yet been demonstrated. Runtime movement continues to evaluate realized `Form::Fluid` geometry directly.

The worker regression now verifies that Bob can generate Water-inclusive fluid formations for a Carbon parent, persist them, reopen the library, and realize a saved suggestion as a valid exact-arc shape with finite pose. That focused worker test passed in CI. The construction consumer was added in commit `b6caf15`; its first CI attempt caught formatting only, corrected in `48ef652`. The corrected revision passed formatting, source-size, COMBINE architecture, Bob compact-storage, and empty-library checks; the full suite remains at the known **339 passed, 29 failed, 1 ignored** baseline, and Clippy remains red on existing broad lint findings.

Next short sprint: add a focused regression for the runtime adapter that proves the stored fluid suggestion is queried and the realized override survives material restoration, while a suggestion that fails live validation is still rejected. Keep the tracked catalogue empty. Only after that regression should the bounded two-constituent generation workflow be run and its artifact metrics reviewed. Do not let a library record directly create a bond or bypass live physical/chemical checks.

### Fluid-to-construction integration contract — 2026-10-11

The follow-up source trace narrows the missing integration further. Bob's `GeometryFluidFormation` stores conserved area, the rigid parent formation signature, anchor constituent/edge, contact-angle and capillary-family scalars, and the normalized contact interval. A read-only `fluid_formations_for_parent(parent_signature, fluid_resource)` query has now been added with regression assertions for exact-parent filtering and unresolved-parent misses. The active `CapillaryContactFamily` solver still returns scalar equilibrium data; neither API currently supplies a realized world-space fluid boundary or a construction-ready fluid placement.

The runtime resource model distinguishes `PhysicalState::Fluid` from the default shape, while `Form::Fluid` is explicitly retained as a legacy serialized form. Construction's rigid suggestion path operates on poses and rigid boundary features. Therefore it would be incorrect to turn a stored fluid record into a rigid pose, or to make its contact interval directly create a bond. A correct bridge needs to preserve fluid area while deriving a boundary/pose against the current rigid arrangement, and must pass through the existing live contact, penetration, compatibility, and energy transaction checks.

**Remaining implementation gate:** extend the current single-rigid-unit parent match to multi-constituent parent arrangements only when a bounded canonical correspondence can be established without live geometry search. Add a focused construction regression proving the stored suggestion's exact arc is carried through restoration and that live physical validation can reject it. Do not let this adapter reinterpret the legacy polygon boundary or alter fluid movement's non-wall semantics. Do not run broad two-constituent generation until that regression passes. The existing dispatch workflow remains available after this gate, but has not been run.



### Single-README consolidation — 2026-10-11

The root `README.md` is now the repository's sole README and central working authority. The subordinate files `chemistry_library/README.md`, `geometry_library/README.md`, `src/bob/README.md`, `src/infrastructure/README.md`, and `tests/README.md` have been removed after their operationally relevant content was consolidated here. The separate `README_STRESS_DESIGN_AMENDMENT.md` was also retired after its approved rule was moved into the root specification. Detailed files under `docs/` remain supporting audit evidence; they do not override the current decisions and status in this README. Generated geometry data remains absent from Git.

**Current fluid integration checkpoint (2026-10-11):** Bob's worker has a focused regression for generation → persistence → reopen → exact-arc realization, and the bond-driven runtime now queries saved Water formations for exact single-rigid-unit parents. The adapter transforms the suggestion into the live frame and supplies the shape as a realized fluid-material override; existing live checks remain authoritative. The current Rust workflow confirms formatting and focused library checks, but the full suite still has the known 29 failures and Clippy remains red. No bounded generation run has been performed against this revised consumer yet, and no end-to-end construction acceptance test exists. The next gate is a focused runtime/material-restoration regression, then bounded two-constituent generation with artifact metrics. The local container could not clone GitHub because network DNS was unavailable, so validation claims refer to GitHub CI only.

## Fluid-to-construction integration audit — 2026-10-11

### Verified state

- Bob now exposes `GeometryLibrary::fluid_formations_for_parent(parent_formation_signature, fluid_resource)`. This is a read-only exact-key query; it does not itself realize a fluid placement or authorize a physical contact.
- `GeometryFluidFormation` stores the canonical rigid-parent signature, fluid resource, anchor constituent and edge, conserved area, capillary equilibrium scalars, and normalized edge-parameter interval.
- `src/geometry/capillary_geometry.rs` provides an analytic flat-edge equilibrium: conserved area, contact angle, curvature radius, free-arc angle, and contact length. Its current public solution is scalar geometry; it does **not** return a world-space boundary polygon/polyline or an already-realized physical constituent.
- Bob's rigid `GeometryFormation::canonicalized` is the appropriate canonicalization boundary for rigid parent arrangements. Do not create a signature by hashing a live structure's incidental iteration order or world translation.
- The runtime now consumes fluid suggestions for exact single-rigid-unit parents, but this is a narrow adapter rather than complete multi-constituent integration. Two-piece generation remains gated on a focused construction regression and live-validation proof.

### Current CI result for fluid-query commit

- Rust workflow run `38155085451`: formatting, source-size policy, COMBINE architecture checks, Bob compact-storage tests, and empty-library open/reopen checks passed. Rust tests completed with **318 passed, 29 failed, 1 ignored**. Clippy also failed under `-D warnings`.
- Windows runner validation run `38155085424`: passed.
- The failing Rust tests include existing constructor/developmental realization failures, geometry/contact expectation failures, and a chemical-break expectation failure. Do not attribute all failures to the fluid-query change without a baseline comparison. The focused fluid-query/persistence assertions are not reported as a standalone CI step, so their individual status cannot be inferred from the aggregate summary.
- CI confirms the repository still compiles far enough to execute the test suite, but the full test and lint gates are not green. Constructor failures remain known work; do not hide them or loosen unrelated assertions to make the gate green.

### Next implementation gate

1. Audit how a live rigid parent maps to a `GeometryFormation`, including constituent indexing, local versus world coordinates, bond topology, and canonicalization. Only query Bob after constructing and validating that exact identity.
2. Make the capillary geometry authority produce a boundary in the contacted edge's local frame from the persisted equilibrium data and interval; transform it into world coordinates using the realized anchor placement. Preserve area and reject impossible intervals. Do not infer a boundary from scalars alone without a defined coordinate convention.
3. Feed the resulting fluid shape to construction as an advisory candidate. The existing live physical geometry, contact compatibility, bond/energy transaction, and collision checks remain authoritative.
4. Add a regression proving that construction actually queries a stored fluid suggestion, preserves the fluid's conserved area, and rejects a suggestion that fails live validation.
5. Only after that regression passes, run the isolated bounded Bob workflow for formations up to two constituent pieces. Keep generated data out of the repository unless explicitly approved.


## 2026-10-11 — Validate changes proposed against the integration branch

The Rust workflow now runs for pull requests targeting both `main` and `integration/unified-source-and-catalogue`. The integration branch is the active source authority while the source-integration PR to `main` remains draft; changes targeting integration therefore need their own CI gate rather than waiting for a later main-bound PR. This changes only CI event selection, not test requirements or acceptance criteria.


## Fluid geometry realization sprint checkpoint — 2026-10-11

### Implemented in source
- `CircularArcBoundary::from_flat_contact` converts a solved `CapillaryContactFamily` into an exact circular-arc descriptor in an explicitly defined edge-local frame. The chord is centered on local `y = 0`; the free arc bulges toward positive `y`. The caller must apply the actual anchor-edge transform.
- The adapter validates persisted scalar consistency and verifies that the realized descriptor preserves the solved area and contact length. It rejects inconsistent or invalid capillary records rather than producing a guessed shape.
- Focused tests cover five contact angles, exact area/contact-length preservation, chord endpoint placement, and rejection of inconsistent stored scalars.

### Integration status — still not end-to-end
This change is a realization primitive, not a construction bridge. The descriptor is not yet represented in `Form`, exact arc-aware overlap/contact/containment are not yet wired into the physical geometry authority, and construction does not yet consume Bob's fluid formation suggestions. The legacy polygon boundary remains unchanged; do not sample the arc into polygon vertices or treat the new primitive as proof that a live fluid placement is valid.

### Validation and next gate
- Latest completed Rust workflow run `38159849420` failed; its Clippy log reports unfulfilled `dead_code` lint expectations across staged APIs under `-D warnings`. The first run for the new adapter also caught two rustfmt differences; these were corrected in commit `93ec4a649709bfc3917f4c6b8c764c97fbcb26e4`. Windows Runner Validation passed on the adjacent revisions. CI for the formatting correction is still pending at this checkpoint. This is not evidence that the full Rust test suite passed; do not conflate lint failure with the known constructor test backlog.
- Next, define a backwards-compatible exact arc boundary representation that can coexist with the legacy serialized fluid polygon, then route physical queries through exact arc operations without silently approximating the curve. Add regression coverage for collision, boundary contact, containment, translation/rotation, serialization compatibility, and area conservation.
- Only after the physical-query path and Bob-to-construction live-validation regression pass should the isolated two-constituent Bob catalogue expansion be rerun. Keep generated catalogue data out of Git.


### Follow-on implementation checkpoint — exact arc representation and physical queries — 2026-10-11

- Extended the legacy `Form::Fluid` variant with optional, serde-defaulted `arc_boundary`. Existing serialized fluids that contain only `nominal_area` and optional polygon `boundary` remain readable. A form may not specify both polygon and exact-arc boundaries.
- Fluid shape validation checks the arc descriptor and verifies that its exact circular-segment area matches `nominal_area`. The bounding radius conservatively includes the arc center offset and radius. `polygon_vertices()` explicitly returns `None` for exact-arc fluids; no arc sampling or implicit polygon approximation was introduced.
- Structural fluid realization and developmental geometry validation now accept exact arc realizations.
- `surface_geometry::boundary_point_toward` performs a ray/circular-arc query and uses the radial normal. `organism_geometry` tests containment against the actual circular segment, including rotated/translated placement. `material_geometry` now routes exact arc boundary contact against lines, circles, and polygonal boundaries through analytic arc queries; circle intersections are solved analytically and arc/segment intersections use the finite sweep. Added focused tests for exact boundary rays, local/world containment, contact under translation/rotation, and legacy/current serialization.
- The overlap and penetration predicates intentionally continue to treat fluid as traversable material, rather than as a rigid collision wall. Arc-aware boundary contact is now available separately. Further review is still needed for all contact edge cases and how this interface feeds the construction transaction.

### Validation gate
- Runner Validation passed on intermediate revisions, but that alone does not validate Rust compilation/tests.
- Rust CI had already reported rustfmt differences and a missing `arc_boundary` initializer in `src/organism/developmental_realization.rs`; the initializer and formatting differences were corrected in follow-up commits. CI for the latest source checkpoint is pending, so do not claim the current branch is green yet.
- The Rust workflow also reports existing `dead_code` failures under Clippy's `-D warnings`, including staged Bob APIs and capillary methods. Keep those visible; do not weaken lint settings as a substitute for integrating or explicitly staging the APIs.
- The persisted Bob fluid formation lookup is now connected to the initial constructor's water-acquisition candidate path. It reconstructs exact arcs from persisted scalars, transforms the parent-local pose into world coordinates, and validates region containment, rigid-anchor contact, non-anchor boundary separation, and fluid-vs-rigid containment before accepting a suggestion. A focused isolated-library regression verifies that a persisted Bob record reaches live geometry validation and preserves exact fluid area. Empty coverage and rejected suggestions are tracked separately; a library-open failure is not silently treated as an empty library.
- The latest source revision `1eaa11f8007c1e4803eb0718daf7823709509325` passes formatting and Runner Validation. Its Rust workflow is still running at this checkpoint; the previous Rust run failed Clippy on unfulfilled lint expectations, so end-to-end green is not yet established.
- Next gate: complete Rust CI, repair any failures in the exact fluid construction regression, and audit that the accepted fluid shape survives restoration into stored physical material. Then rerun the isolated two-constituent Bob catalogue expansion and report fluid interface coverage. Do not expand the catalogue before CI and restoration checks pass.


### Lint cleanup follow-up — 2026-10-11

- Removed file-level `expect(dead_code)` annotations from the physical-material, cavity, material-geometry, connection-geometry, memory, reproduction, transformation, decision-runtime, and movement modules where the latest Clippy run reported the expectation itself was unfulfilled. This removes stale staging markers rather than weakening the lint policy.
- The last completed full test run before this cleanup reported 339 passed, 29 failed, and 1 ignored. The 29 failures remain the known constructor/lifecycle backlog and are not being chased in this fluid-geometry sprint.
- Runner Validation passed on the latest code checkpoint. Rust CI is rerunning after the lint cleanup; Clippy status and any newly exposed dead-code warnings remain unconfirmed until that run completes.

### Exact-arc fluid runtime regression and CI follow-up — 2026-10-11

At commit `7e9a01f7386bf25670b6f1bdd556dcbdbe4e5769`, the focused construction runtime regression `construction_runtime::tests::bob_fluid_suggestion_transforms_and_restores_as_exact_fluid_geometry` passes. The initial-constructor regression `initial_organism_constructor::tests::persisted_bob_fluid_suggestion_reaches_live_constructor_validation` also passes. The borrowed-shape assertion compile error was corrected without changing geometry behavior.

The exact-head Rust workflow passes formatting, source-size policy, COMBINE architecture checks, Bob compact-storage tests (7 passed), and empty-library open/reopen without generation (1 passed). The complete Rust suite reports **340 passed, 29 failed, 1 ignored**. The 29 failures match the known cross-subsystem baseline; do not expand this sprint to chase them. Clippy remains red under `-D warnings` on ordinary application/test lint findings. Runner Validation passes. See [Rust workflow #38165088848](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38165088848) and [Runner Validation #38165088861](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38165088861).

**Current fluid milestone:** persisted Bob fluid formations are queried by the construction runtime for an exact single-rigid-unit parent, reconstructed as an exact circular arc, transformed into the live frame, and carried through material restoration. Both focused fluid regressions pass. This is still not proof that an end-to-end growth transaction accepts and bonds Water successfully; the live transaction remains the authority.

**Next short sprint:** extend parent identity mapping from a single rigid unit to a canonical two-constituent rigid formation, including bond topology, constituent-index remapping, and canonical-frame-to-live-frame transformation. Query Bob fluid records only when the live parent maps unambiguously to the persisted canonical parent signature. Preserve live collision/contact/energy checks and exact area. Then run the isolated `--geometry-worker-complete-up-to 2` generation workflow and report generated candidates, duplicates, rejected formations, rigid/fluid family counts, elapsed time, and artifact size. Do not commit generated catalogue data. The workflow is currently manual-dispatch-only; no new generation run is claimed in this checkpoint.

### Two-constituent parent mapping audit — 2026-10-11

Source inspection confirms the live graph uses stable `PhysicalConstituentId` values on `StructuralUnit` and `BondEndpoint`; live unit-vector indices are not stable identities. A safe two-piece parent adapter must therefore:

1. Select two rigid, externally realized units that are connected by a live bond; resolve each unit's single base-resource identity and exact shape from the catalog.
2. Express both placements in one selected unit's local frame, including relative rotation; build a two-constituent `GeometryFormation` and canonicalize it through Bob's own `canonicalized(catalog)` path.
3. Retain an explicit mapping from each live `PhysicalConstituentId` to its canonical Bob constituent index. Do not assume canonicalization preserves insertion order, particularly when both pieces share a resource name.
4. Derive the rigid transform from Bob's canonical parent frame back into live world coordinates using a matched constituent pose. Reject ambiguous or non-finite mappings rather than falling back to a singleton record.
5. Query fluid records by the canonical parent's exact signature, realize the exact arc, and still run the normal live overlap, contact, restoration, and bond/energy transaction checks.

Important schema limit: `GeometryBond` currently records constituent connectivity, not the physical bond endpoints/strength. That is sufficient only if fluid parent identity is intended to describe the rigid arrangement and connected-pair topology, not the precise endpoint-level bond graph. Confirm this scope before expanding the record schema; do not silently imply endpoint-level identity.

The latest README-only checkpoint commit is `7c2e3f844058a0b0724c30f420828a3670830fa7`. Runner Validation passed on that commit; its Rust workflow is still running. This audit does not claim multi-constituent fluid lookup is implemented.

### Multi-connection fluid support is a required capability — 2026-10-11

Scope correction from the user: fluid operations must support **multiple simultaneous connections**, and must not be designed around the current Bob limit of formations containing at most two rigid constituents. Two-constituent catalogue generation is only a bounded test stage, not a runtime or fluid-physics ceiling.

**Audit finding:** the present `GeometryFluidFormation` stores exactly one `anchor_constituent`, one `anchor_edge`, one contact interval, and one circular free arc. `realize_against_parent` validates and realizes only that one contact. `Form::Fluid` can store one exact `CircularArcBoundary` (or a legacy polygon), not a general fluid boundary with multiple solid-contact intervals. Therefore the current implementation cannot honestly claim to realize a single fluid body simultaneously constrained by multiple separate parent surfaces. Querying several single-edge records independently is not equivalent to solving one multi-connection fluid operation.

**Required design before broadening parent formation size:**

- Represent a fluid body by conserved area plus a boundary made of connected exact segments/arcs, with one or more solid-contact intervals and free-interface arcs. Keep analytic geometry exact; do not replace it with sampled polygon approximations.
- Store a list of contact constraints per fluid formation. Each constraint identifies a canonical parent constituent and boundary edge, its normalized exposed interval, and the applicable contact-angle/curvature condition. A record with one constraint is the backward-compatible single-contact case.
- Solve all active contact constraints together. Reject mutually inconsistent constraints, self-intersection, impossible intervals, invalid topology, or area mismatch; do not pick one anchor and silently ignore the rest.
- Map canonical constituent indices back to stable live `PhysicalConstituentId` values after canonicalization. Verify each intended contact against the corresponding live surface after world transformation.
- Run live multi-contact collision, containment, bond/contact, and energy transaction checks. Bob proposes geometry; it never authorizes a physical operation.
- Test one fluid body contacting two non-collinear rigid edges, three or more contacts, translated/rotated parent poses, reordered equivalent constituents, symmetric/equal-material constituents, impossible contact sets, area conservation, and regression of the current single-edge case.
- Keep the bounded two-constituent generation workflow as a diagnostic benchmark only. Once the multi-contact model is correct, extend generation by measured growth and topology, not by treating two pieces as a permanent maximum.

**Gate:** do not describe fluid support as multi-connection-capable until the boundary representation, simultaneous constraint solver, runtime mapping, and live transaction tests all pass. This is now a prerequisite to claiming general multi-constituent fluid integration.

### Multi-contact fluid geometry foundation — 2026-10-11

Implemented in `src/geometry/capillary_geometry.rs`:
- `FluidBoundaryPiece` represents either an exact straight solid-contact interval with a contact angle or an exact circular free arc.
- `MultiContactFluidBoundary` stores an ordered piecewise boundary, checks closure and valid contact angles, and verifies signed enclosed area using Green's theorem without arc tessellation.
- Rigid translation/rotation transforms every piece and revalidates closure and conserved area.
- Focused tests cover the current one-arc-plus-one-contact case, four simultaneous contact intervals, gap rejection, area mismatch rejection, and rigid-transform preservation.

**Scope limitation:** this is a geometry foundation only. It is not yet carried by `Form::Fluid`, persisted as a multi-contact Bob formation, or consumed by collision/contact/containment/restoration and live construction transactions. The boundary validator also does not yet prove global non-self-intersection or solve contact constraints from parent geometry. Do not claim the requested multi-connection fluid operations are complete until those integration and solver gates pass.

The implementation must continue beyond the current two-constituent Bob benchmark. Multi-connection tests need to involve one connected fluid boundary contacting multiple non-collinear rigid surfaces, including three or more contacts, rather than merely several independent single-contact suggestions.


### Multi-contact fluid geometry sprint — 2026-10-11

This short sprint continued the multi-contact geometry foundation rather than waiting on CI. The current branch now includes:
- Analytic containment for a closed fluid boundary composed of multiple solid-contact segments and circular free arcs.
- Normalized circular-arc angle intervals during ray-crossing containment, including arcs whose start angle lies outside the canonical interval.
- A mixed line/semicircle regression covering interior/exterior containment before and after a rigid transform, plus contact/arc counts.
- Existing multi-contact shape validation, exact-area preservation, material restoration acceptance, and rigid-surface contact dispatch remain in place from the preceding sprint.

**Not yet complete:** this is not proof of arbitrary-contact end-to-end construction. The boundary validator checks closure and signed area but does not yet establish global non-self-intersection; contact geometry must be checked against multiple distinct rigid surfaces in one integration fixture. Bob's current formation-generation cap is still a separate limit, and general multi-constituent parent matching remains unfinished. Do not report this capability as complete or run broad catalogue generation yet.

**Next short sprint:** add a physically valid fixture for one fluid body simultaneously contacting at least three non-collinear rigid surfaces; test each contact through the live geometry query, verify points inside/outside, then trace the same shape through material restoration and the live construction validation path. Keep CI polling asynchronous: continue code inspection/implementation between status checks and do not spend a sprint waiting for a workflow.


### Three-contact integration regression sprint — 2026-10-11

Follow-up work added a regression in which one closed multi-contact fluid boundary is queried against three distinct rigid line surfaces (bottom, side, and top), and a material round-trip regression that verifies a conserved-area multi-contact shape survives PhysicalMaterial serialization/restoration. The rigid-contact predicate's unbounded-fluid guard was also corrected so a fluid with multi_contact_boundary: Some(...) is not incorrectly rejected merely because its legacy polygon and single-arc fields are empty.

These are focused regressions, not a completed end-to-end organism growth proof. CI has been triggered by the recent commits and is not being treated as a reason to pause implementation. Next, check for any remaining paths that treat “no polygon and no single arc” as “no boundary” without considering the multi-contact field, then add a construction-level fixture that keeps all three live contacts and checks ordinary physical/chemical validation.

### Restored multi-contact live-query sprint — 2026-10-11

Added a construction-runtime regression that creates a conserved-area, four-sided multi-contact Water shape, stores it as a physical material, restores it at a translated/rotated world pose, and queries that same restored shape against three distinct non-collinear rigid line surfaces. Each contact must pass both the boundary-contact and rigid-contact predicates; the restored shape must remain identical and preserve its bounding geometry.

This closes a useful integration gap between the geometry/material round-trip tests and runtime-facing geometry queries. It does **not** prove that Bob can yet persist or solve a multi-constraint fluid formation, nor that a growth transaction creates all three bonds atomically. Bob's stored fluid formation schema still models one anchor edge/contact interval and one free arc; the simultaneous-contact solver and general multi-constituent live-parent mapping remain open gates. Keep the worker's two-constituent run as a bounded benchmark, not a fluid capability limit. CI should be inspected briefly after the commit, but implementation should continue independently of long-running workflows.

### Bob maximum-size parent fluid generation fix — 2026-10-11

Audit found the worker's fluid-generation path used formation.constituents.len() < max_constituents, unintentionally suppressing all fluid records for rigid parent formations exactly at the configured size limit. The limit is intended to bound rigid-parent expansion; a deformable fluid body is an additional formation record and must still be generated against a parent at that limit. The worker now generates fluid records for every processed parent, while rigid-parent expansion remains strictly bounded by the less-than max_constituents condition.

Added a focused worker regression that seeds a valid two-constituent Carbon parent, processes it with limit 2, and requires Water fluid formations to be persisted against that exact parent signature. This verifies that the bounded two-piece benchmark actually covers two-piece parents plus fluid, rather than silently testing only singleton-parent fluid records. Full multi-contact constraints and live parent mapping remain separate unfinished work.

### Multi-contact boundary topology guard — 2026-10-11

The piecewise fluid-boundary validator now rejects intersections between non-adjacent straight solid-contact segments, including a regression fixture whose signed area is positive and matches the declared area despite a self-crossing path. Intersection tolerance is scaled consistently between coordinate and cross-product units. Adjacent pieces may still share their intended closure endpoints. This was a partial topology guard at that checkpoint. Later work added analytic non-adjacent line/arc intersection checks; exact arc/arc checking was still open at the next checkpoint.

### Constructor free-interface probe audit — 2026-10-11

The initial-constructor fluid candidate validator no longer hardcodes a single arc midpoint when probing free-interface points: it now extracts the midpoint of each exact free arc in either the existing single-arc form or the piecewise multi-contact boundary, then transforms those probes into world coordinates. This prevents a multi-arc boundary from silently checking only one interface segment if such a shape reaches the validator. This is only a local validation primitive; Bob still persists single-anchor/single-arc fluid records, and the current constructor does not yet map a multi-contact record to multiple live parent surfaces or validate the resulting multi-bond transaction. Do not treat this change as end-to-end multi-contact construction support.

### Active short-sprint status — 2026-10-11

The current branch includes the maximum-size Bob fluid-generation correction, the restored three-surface multi-contact regression, a partial non-adjacent straight-segment self-intersection guard, and multi-arc free-interface probe extraction in the initial constructor. Runner Validation has passed on the latest source-changing checkpoint before the README-only update. Rust CI is still running for the constructor probe change; recent completed Rust runs are red on the existing broad Clippy -D warnings backlog and earlier formatting differences, so no full-green claim is made here. The known 29 constructor/lifecycle test failures remain out of scope. Next work stays focused on the missing simultaneous-contact solver and multi-parent live identity mapping, not on the baseline failures.


### Exact mixed-boundary intersection checks — 2026-10-11

The multi-contact fluid-boundary validator now analytically checks non-adjacent straight/arc intersections by solving the line-segment/circle equation and filtering intersection points against both the finite segment and arc sweep. The next increment adds circular-arc/circular-arc intersection checks, including concentric equal-radius arcs whose angular sweeps overlap. Adjacent pieces remain exempt so their intended shared endpoints are permitted. Regression coverage was added for line-crossing-arc, intersecting non-concentric arcs, and concentric arcs with overlapping versus disjoint sweeps.

This strengthens the boundary validity guard; it does not complete Bob's simultaneous-contact fluid solver, persistence format, or construction-time mapping from multiple live rigid constituents to a canonical parent formation. Those remain the primary end-to-end tasks. The new code is committed, but CI for this exact checkpoint is queued; do not report it green until its run completes. Continue working without waiting on CI, and do not spend time chasing the known 29 baseline Rust test failures.


### Canonical parent identity mapping — 2026-10-11

Bob's `GeometryFormation` now exposes `canonicalized_with_mapping`, which returns the canonical parent formation together with a mapping from each input constituent index to its canonical index. This is necessary for translating live stable physical constituent IDs into Bob's canonical constituent slots before querying multi-parent fluid constraints. If equally canonical results assign different canonical slots to different input constituents, the API rejects the mapping as ambiguous instead of guessing. A focused two-constituent regression checks that the returned mapping is a valid index permutation and remains consistent with canonical bond indexing.

The live construction runtime now builds a connected rigid parent formation from the physical graph's actual rigid constituents and bonds, maps each stable physical ID to its canonical constituent index, and recovers a rigid transform from Bob's canonical frame back to live space. It rejects invalid/disconnected parents, ambiguous canonical mappings, and parent transforms that do not reproduce the live constituent centers within the geometry tolerance. Fluid suggestion queries now use the full canonical rigid parent and select the stored anchor record by the current live constituent's stable ID. Bob still persists one anchor contact per fluid record, however: this is not yet simultaneous multi-contact boundary generation, nor proof that a proposed fluid body will bond to multiple surfaces in one transaction.


The arc-intersection regression tests were moved back inside their Rust test module after CI caught a misplaced module-closing brace. The non-concentric arc test fixture was also corrected so the two finite sweeps actually include a shared circle-intersection point. These corrections are committed; their new CI runs are pending.


### Live parent mapping regression — 2026-10-11

Added a construction-runtime regression that builds a two-constituent live Carbon parent, attaches its physical bond by stable IDs, canonicalizes the parent, and verifies both IDs resolve to distinct canonical slots. This covers the live graph → canonical Bob parent identity boundary. It does not yet transform suggestions back into live space or provide simultaneous multi-surface fluid boundaries; those remain the next integration steps.


### Full-parent fluid suggestion integration — 2026-10-11

Construction now queries Bob using the connected rigid parent formation rather than always manufacturing a singleton-parent signature. The live stable constituent ID is translated to its canonical anchor slot, the matching stored suggestion is realized against the complete parent arrangement, and its pose is transformed back into live coordinates. The normal live restoration, overlap, contact, and transaction checks remain authoritative. Parent canonicalization is performed only when the candidate material actually includes fluid, avoiding unnecessary work for rigid-only candidates.

This is an integration milestone, not the finished multi-contact solver. Existing Bob fluid records still describe a single anchored contact interval and one free arc. Simultaneous constraints from several rigid surfaces must still be solved and persisted as a multi-contact boundary before multi-bond fluid construction can be claimed. Latest CI should be treated as unconfirmed until the current commit's Rust workflow finishes; do not chase the known 29 baseline test failures.


### Multi-contact boundary consumer audit and realization fix — 2026-10-11

A focused source audit found two live-consumer gaps after the multi-contact shape and three-surface regressions were added:

- `StructuralUnit::realize_fluid_geometry` accepted legacy polygon and exact single-arc fluid boundaries, but rejected `multi_contact_boundary`. That meant a valid multi-contact shape could survive `PhysicalMaterial` serialization/restoration metadata and pass direct geometry predicates yet fail when installed as the constituent's realized physical geometry.
- `boundary_point_toward` handled legacy polygons and single circular arcs but ignored `MultiContactFluidBoundary`. This prevented generic surface-point/normal queries from finding a boundary on a realized multi-contact shape.

The current sprint updates both consumers. Structural realization now accepts a multi-contact boundary, with a regression asserting that the exact shape is retained. It also checks conserved area against the resource's nominal area multiplied by the constituent amount, preventing a geometrically valid but mass-inconsistent override. Surface queries now ray-intersect ordered solid-contact segments and free circular arcs analytically, choose the nearest forward boundary intersection, and return outward normals for the positive-area counter-clockwise boundary; a square multi-contact regression checks right/top intersections and normals. A non-intersecting free arc is skipped rather than terminating the search for later pieces.

**Verification:** these edits are committed on `integration/unified-source-and-catalogue` through `e11d9f5f`. Exact-head CI has not yet been confirmed for this checkpoint. The previous inspected Rust workflow failed before tests at formatting and also reported existing strict-Clippy diagnostics; runner validation passed. Do not describe these new regressions as passing until the exact-head Rust workflow reaches them.

**Remaining in this thread:** audit further consumers for assumptions that only `boundary` or `arc_boundary` can define a finite fluid surface—especially endpoint/contact generation, overlap/penetration, parent restoration, and the construction transaction. Then add a construction-level fixture that installs one multi-contact Water body, verifies at least three distinct rigid contacts after restoration, and checks the normal live transaction path. Keep this in short implementation sprints and continue work while CI runs. The multi-contact boundary validator still needs its broader contact/transaction proof; Bob's persisted formation schema and simultaneous multi-constraint solver remain separate unfinished gates. Do not run broad catalogue generation yet.

### Multi-contact consumer audit — 2026-10-11

A follow-up pass traced the current piecewise fluid boundary through the live consumers:

- `Form::Fluid` carries the optional `multi_contact_boundary`; shape validity and conserved-area restoration paths accept it.
- `boundary_point_toward` now queries exact solid-contact segments and free arcs.
- `placed_forms_boundary_contact` and `placed_forms_rigid_contact` dispatch to exact multi-contact boundary checks, including transformed boundaries.
- Material restoration retains the realized shape; the runtime regression checks three distinct rigid-surface contacts after a translated/rotated round-trip.
- Fluid interior is intentionally not treated as a rigid collision wall. This is consistent with the rule that fluid is traversable while its realized boundary remains physically queryable.

**Remaining blocker:** these consumers can query a supplied multi-contact boundary, but Bob's persisted `GeometryFluidFormation` still stores one `anchor_constituent`, one `anchor_edge`, and one circular-arc solution. The constructor therefore has no Bob record that expresses a single fluid body constrained by several parent edges at once. The current three-contact test supplies a handcrafted boundary; it does not create a multi-bond growth transaction. This is a representation/solver gap, not a reason to relax collision or bond checks.

**Next short sprint:** extend Bob's fluid-formation data model in a backward-compatible way to represent a list of contact constraints, then make validation/signatures/persistence account for the complete list before implementing simultaneous realization. Keep legacy single-anchor records readable as one-contact records. Do not claim multi-connection support or run broad catalogue generation until the simultaneous solver and live transaction gate exist.

**CI snapshot:** PR #186 remains the draft-only integration target. At the inspected checkpoint `3ff2415d51d5991b4894e07244b49e904122cd1e`, Runner Validation succeeded and Rust workflow failed; the failure is not being treated as proof that the current unmodified source edits pass. Do not chase the known 29 baseline constructor/lifecycle failures as part of this sprint.
### Bob fluid schema groundwork — 2026-10-11

The persistent `GeometryFluidFormation` model now has a `contact_constraints` list with a serde default, so older stored records that omit the field remain readable. Each entry identifies a canonical parent constituent/edge and its contact interval. The existing legacy anchor/scalar fields are retained during migration.

This is deliberately **schema groundwork, not multi-contact realization**. The current single circular-arc solver does not yet solve the constraints list. Therefore, both `realize_against_parent` and fluid-record validation fail closed when the list is non-empty; they must not silently realize only the legacy anchor while ignoring additional constraints. Current generation still emits legacy single-contact records. The next implementation step is to define and validate the simultaneous boundary solution representation, then teach the solver and persistence path to populate/consume it as one complete record.

No compile/test result is claimed for these edits yet. Keep the active focus on this bounded fluid solver/transaction work; do not run broad catalogue generation.
**Schema regression coverage added:** tests verify that a legacy serialized fluid record without `contact_constraints` deserializes to an empty list with an unchanged legacy signature, and that adding a contact constraint changes the record signature. These tests have been authored but have not yet been confirmed green on CI.
The schema tests also assert that a non-empty, unresolved constraint list is rejected by `realize_against_parent` rather than silently returning the legacy one-edge arc. This is the intended temporary behavior until a true simultaneous solver is implemented.

### Persisted solved fluid boundary — 2026-10-11

The Bob fluid-formation record now has a serde-defaulted `solved_boundary` field containing the complete ordered `MultiContactFluidBoundary` in the canonical rigid-parent coordinate frame. Legacy records remain compatible: absent `contact_constraints` and `solved_boundary` decode as empty/none, and legacy signatures are unchanged when both are absent/empty.

For multi-contact records, `realize_against_parent` now requires a persisted complete boundary, at least two constraints, a valid boundary, conserved-area agreement, matching boundary-contact/constraint counts, valid parent constituent/edge references, and finite contact interval data. It reconstructs the whole multi-contact shape at the canonical parent-frame origin; it never degrades to the single-anchor arc. The solved boundary contributes to record identity. Bob's insert/load validator now accepts only complete multi-contact records whose solid boundary pieces coincide with their referenced parent edges, match their stored contact lengths, and fall inside their allowed edge intervals. A focused regression verifies that a complete-looking but spatially misaligned boundary is rejected before persistence; valid solver-produced records are not yet available to exercise a positive multi-contact round-trip.

**Important limitation:** this is a persistence/consumption step, not yet a simultaneous equilibrium solver. Current generation still emits single-anchor records, and the validator now checks each segment against its referenced parent edge, but no simultaneous solver yet generates valid multi-contact records. The live construction transaction must still validate every actual contact, nonpenetration, self-intersection, area conservation, and bond creation atomically. Do not claim end-to-end multi-connection construction or expand catalogue generation until that gate exists.

**CI:** the inspected Rust workflow at the earlier README checkpoint failed at rustfmt before running tests; formatting corrections were committed, then the solved-boundary persistence and reconstruction changes followed. The current exact-head workflow has not yet been verified. Runner validation had passed at the earlier checkpoint. Do not treat that as a test result for these later changes.


### Multi-contact validator compile correction — 2026-10-11

The exact-head Rust workflow for `4da264c7` reached formatting successfully but failed compilation because the new parent-edge alignment validator referenced `FluidBoundaryPiece::SolidContact` without importing `FluidBoundaryPiece`. The import is now added in `src/bob/library.rs` (commit `454d2c2d`). Runner Validation had passed on `4da264c7`; that result does not validate the import correction, so inspect the new exact-head workflow before claiming compilation is green.

The implementation gate remains unchanged: current worker generation still creates only single-anchor fluid records. Do not try to pass this gate by assembling independent pairwise arcs. The next code sprint must implement one simultaneous solve over a canonical rigid parent, persist its complete boundary and constraints, and add a positive solver-produced persistence/reload regression before connecting atomic multi-bond realization in live construction.


### Exact-head verification and multi-contact audit — 2026-10-11

**Verified integration head:** `7261fa6a2bed68b5a51f31416d85282eff4be47f` on `integration/unified-source-and-catalogue` (PR #186 remains draft and unmerged).

- **Runner Validation:** passed.
- **Rust test stage:** compiled and ran the suite, then failed with 359 passed, 32 failed, and 1 ignored. The 29 previously tracked constructor/lifecycle failures remain out of scope for the Bob/fluid sprint; the three additional failures in this run must be classified separately rather than silently counted as baseline.
- **Strict Clippy stage:** failed on 62 diagnostics under warnings-as-errors, including existing unused/dead-code and style lints. Do not use this broad lint backlog to derail the current multi-contact geometry work; do not claim Rust CI is green.
- **Focused multi-contact checks passed in that run:** exact boundary transform/area preservation, line/arc containment, rejection of crossing boundaries, three distinct live rigid-surface contacts after restoration, schema compatibility for legacy fluid records, fail-closed unresolved constraints, rejection of misaligned records before persistence, material round-trip preservation, and exact surface queries/outward normals.

These results establish that the current boundary representation and runtime consumers are exercised by the suite. They do **not** establish a simultaneous Bob solver or a successful multi-bond growth transaction. The next implementation gate remains a solver-generated boundary constrained against one canonical rigid parent, with every contact and the conserved area validated as one whole boundary. Do not combine pairwise solutions, weaken contact checks, or start broad catalogue generation before that gate exists.


### Automatic exposed-edge discovery for collinear multi-contact Water — 2026-10-11

Added `generate_collinear_multi_contact_formations` to enumerate rigid boundary intervals exposed by a canonical parent, group intervals on the same line, order contiguous intervals, require at least two distinct parent constituents, check compatible equilibrium contact angles, and send complete candidate runs through the whole-boundary solver and Bob record validator. The persistence regression now also asserts that this automatic discovery path finds the three-surface test arrangement without caller-supplied segments.

**Current boundary of support:** the enumerator now clips the first and last exposed intervals to the analytic chord length, reconstructs the clipped edge parameters, and rejects gaps, repeated constituents, incompatible contact angles, or invalid complete boundaries. A focused clipping regression checks partial-edge parameter reconstruction. Non-collinear contact arrangements remain unsupported. Keep the generator gated on exact-head Rust verification and the positive automatic-discovery/persistence regression; do not expand catalogue generation broadly until the live multi-bond transaction regression passes.


### Bob worker integration for bounded multi-contact discovery — 2026-10-11

The Bob worker's Water frontier runs `generate_collinear_multi_contact_formations` alongside the existing single-anchor fluid-family path. It uses clipped exposed intervals, routes legacy and multi-contact records through one validated durable write, and reports multi-contact candidates, additions, and non-inserted counts separately. Collinear line-family deduplication avoids repeating equivalent candidate-window searches.

The three-target live transaction and clipped persistence regressions passed at checkpoint `f11d63a4`. Broad catalogue generation remains gated on the exact-head Rust run for the newest endpoint guard, rollback test, and worker metrics. Non-collinear simultaneous equilibrium remains unsupported; do not approximate it by merging pairwise solutions.


### Runtime eligibility for multi-contact suggestions — 2026-10-11

Updated `bob_fluid_origins_for_live_parent`: legacy fluid records remain keyed to their single anchor, while a record with simultaneous contact constraints can now be considered when construction is initiated from any constrained constituent in the canonical live parent. This removes the previous lookup bottleneck where a valid multi-contact record could only be proposed from its first anchor.

This lookup change is now paired with the atomic multi-contact bond staging and exact-point endpoint handling documented below. The bounded three-target transaction regression passed at checkpoint `f11d63a4`; newest endpoint hardening and worker instrumentation remain pending exact-head verification.


### Atomic multi-contact bond staging — 2026-10-11

Added `form_remaining_multi_contact_bonds` to the live construction transaction. For a realized multi-contact fluid shape it discovers the rigid units physically touched by the solved boundary, requires the number of distinct rigid targets to match the boundary's solid-contact piece count, then stages the remaining bonds on the already-isolated trial structure. Each target must have a compatible available connection pair within the normal contact tolerance, and each bond must pass the normal energy/compatibility path. Any missing target or failed bond rejects the candidate; the caller discards the trial clone, so no partial set is committed. The Bob enumerator now also rejects candidate runs with more than one contact piece for the same constituent, matching this first transaction contract.

**Verified at checkpoint `f11d63a4`:** the three-target regression passes, and the incomplete-target trial fails closed without mutating the original structure or energy. The implementation supports the bounded catalogue shape—one solid-contact piece per distinct rigid unit—and rejects a live contact set that differs. The full suite still has the documented baseline failure backlog; newest strict endpoint guard and worker counter changes remain under verification.


### Multi-contact persistence regression correction — 2026-10-11

The first full Rust test run exercised the new positive persistence regression and found a genuine signature instability: raw JSON serialization of boundary floats differed by one ULP after reload, so the restored record's signature did not match the inserted record. Replaced raw serialized-boundary signature material with a deterministic quantized signature over every SolidContact and FreeArc field. This preserves geometry-level identity at the library's existing quantization resolution while making persistence/reload stable. The corrected persistence regression passed at checkpoint `f11d63a4`, including automatic clipped discovery and a reload-stable signature.


### Exact multi-contact connection endpoints — 2026-10-11

The live three-target regression exposed a second real integration gap: ordinary fluid endpoint discovery projected target points radially from the fluid unit origin, so a line endpoint lying in the middle of a SolidContact interval could be represented by the wrong boundary point. Added exact-point resolution for `ConnectionEndpoint::Fluid` when the shape carries a `MultiContactFluidBoundary`, plus candidate generation from rigid endpoints lying on SolidContact pieces and verified surface points for circles/polygon edges. The atomic bond helper now tries alternate valid contact candidates before rejecting a target. A focused surface-geometry test verifies exact point and outward-normal preservation and passed at checkpoint `f11d63a4`. The newer strict off-boundary rejection guard is still pending exact-head verification.


### Multi-contact positive gates passed at checkpoint — 2026-10-11

The Rust test run for checkpoint `f11d63a4` passed the targeted multi-contact regressions: `construction_runtime::tests::multi_contact_fluid_commits_all_three_rigid_bonds_together`, `geometry_reference_library::fluid_contact_constraint_schema_tests::clipped_multi_contact_window_reconstructs_partial_edge_parameters`, `geometry_reference_library::fluid_contact_constraint_schema_tests::simultaneous_collinear_solution_persists_and_reloads_three_parent_contacts`, and `surface_geometry::tests::multi_contact_endpoint_resolution_preserves_exact_contact_point`. The runtime test also verifies that an incomplete target set rejects its isolated trial without changing the original structure or energy. The clipped automatic-discovery fixture proves that one solved Water boundary can contact three distinct rigid constituents even when exposed intervals exceed the analytic chord. The full Rust suite still has a broad failure backlog (366 passed, 32 failed, 1 ignored at this checkpoint); the run also fails Clippy on unrelated existing lints. The newer worker instrumentation and strict off-boundary endpoint guard still need exact-head verification.


### Multi-contact endpoint hardening — 2026-10-11

Exact fluid endpoint resolution now rejects coordinates farther than geometry tolerance from the solved boundary instead of silently projecting arbitrary points onto it. Added a negative construction regression for an incomplete target set: the multi-contact helper must fail closed without adding bonds or changing the caller's original structure/energy. The positive three-target test remains the main proof that all bonds can be staged together. Verification of these newest guards is pending.


### Multi-contact catalogue instrumentation — 2026-10-11

The Bob worker now reports multi-contact fluid formations separately from legacy fluid formations: generated candidates, persisted additions, and the difference (not inserted, including duplicates/rejections). Both per-frontier output and the bounded run-to-completion summary include these counters. This permits measuring whether exposed-edge clipping is producing useful persistent families before any broader catalogue run. The worker change is compile/test gated; do not start a large generation pass until exact-head Rust verification is available.


### Single-write multi-contact metrics — 2026-10-11

The worker's separate multi-contact counters initially required two calls to the fluid-formation persistence API, which would cause two durable file syncs per Water frontier. Replaced that with `insert_fluid_formations_with_multi_contact_counts`: legacy and multi-contact records are validated/deduplicated together, written once, and the method returns total additions plus multi-contact additions. The existing insertion API remains as a compatibility wrapper. The persistence regression now asserts the `(total_added, multi_contact_added)` pair is `(1, 1)` for the multi-contact record.


### Collinear family work deduplication — 2026-10-11

The exposed-edge generator now keys already-processed line families by quantized direction, offset, and equilibrium contact angle. Edges sharing the same line family no longer repeat the same full exposed-interval alignment and chord-window enumeration. The key uses the library's existing (10^{-9}) geometry quantization, tighter than the candidate collinearity tolerances. This is a search-work reduction only; formation signatures and validation remain unchanged.

### Energy-source contract: inexhaustible imaginary sun — 2026-10-18

**User decision: the imaginary sun is the inexhaustible external energy source.** Do not model depletion of the sun or a finite local environmental energy reservoir as a prerequisite for harvesting.

**Updated user decision — rotating, position-dependent world tone (2026-10-19):** replace the earlier assumption of a constant, omnipresent 440 Hz tone with one directional signal emitted by a source that rotates around a circular world.

- The source follows one circular orbit around the world. Its signal is directional; this is not a spatial gradient radiating from the source and not two simultaneous opposing signals.
- At the northern apex of the orbit, the signal reaches its highest frequency/pitch of **440 Hz** and its maximum signal-energy level.
- As the source moves east and then south, both frequency/pitch and signal-energy level decrease smoothly. At the southern point, both reach their minimum.
- As the source moves west and then north, both rise smoothly back to 440 Hz and the maximum signal-energy level.
- Frequency and signal-energy level are separate model quantities that co-vary over this positional cycle. Do not assume that changing frequency physically determines energy level; their shared cycle is an EvoSim environmental rule.
- **Provisional cycle calibration (2026-10-19):** one complete day/night orbit is **2,400 simulation ticks**. Tick 0 is the northern peak, tick 600 the eastern midpoint, tick 1,200 the southern valley, tick 1,800 the western midpoint, and tick 2,400 returns to the northern peak.
- **Provisional available-energy budget:** the sun makes **10 energy units per tick** available at the northern peak and **1 energy unit per tick** at the southern valley, varying smoothly between those endpoints. The sun is inexhaustible, but the amount locally available per tick is finite.
- **Provisional evolutionary calibration:** use **5% capture efficiency** as the initial-organism baseline and compare against **10%** during calibration. These are test hypotheses, not a hard-coded efficiency ceiling; evolved organisms may approach **100% of the energy available to them**, never more.
- Keep available energy, signal level, capture efficiency, and usable converted energy as distinct quantities. Signal strength or resonance perception must not itself credit energy to an organism. Captured energy must enter through an explicit, rate-limited, source-backed ledger transaction; conversion losses/work/dissipation must be accounted for separately.
- The minimum frequency, interpolation curve, and orbit radius/geometry remain **OPEN**. Cosine interpolation is the current provisional implementation choice, not a validated physical law.
- Resonance changes capture/coupling effectiveness; it does not create energy. The reaction-driving chemistry accumulator remains distinct from energy and cannot serve as an implicit energy source.
- Keep one global energy unit and account for all captured energy and subsequent transfers, conversion, work, storage, and dissipation. Conservation is tracked as initial energy plus explicitly recorded external input equals final tracked energy plus explicitly recorded external output; internal transfers cancel.
- `src/simulation/world_tone.rs` models source position, frequency, signal level, and the separately represented available-energy budget. The runtime now supports an **explicit optional configuration** and samples it each simulation tick when configured; it remains disabled by default because minimum frequency, signal-level bounds, and orbit radius are not all decided. `src/perception/harmonics.rs` merges the sampled source through realized cavity-wall material and directional geometry into the live harmonic spectrum. The harmonic cache key includes the source tick while the source is active. This is perception integration only: it does not yet capture energy into the ledger, and source-attributed perception memories are not yet passed through the action/memory path. The cosine envelope is provisional.

Implementation boundary remains unchanged: implement the source/capture contract in the dedicated energy-and-natural-chemistry workstream based on the current #186 integration foundation, keeping PR #196 constructor work independent. This decision updates the environmental source model; it does not by itself authorize or establish a complete energy-accounting implementation.



## Energy baseline, dissipation/stress, and decomposition design — 2026-10-18

**Status:** The energy rules below record established decisions. The decomposition proposal that was previously marked *needs approval* was stale and contradicted the already-established architecture; the replacement decision below supersedes it. This is a design/status correction, not authorization to change constructor/Bob PR #196.

### 1. Initial energy baseline — decision

- The starting organism receives an explicit, finite initial usable-energy endowment. It is part of the simulation's initial condition, not a reward for constructor work and not derived from the constructor's search budget.
- The exact numeric endowment remains **OPEN**. It must be a named initial-world/organism parameter chosen for the intended starting conditions and calibrated against the energy costs; do not preserve the old `1.0e12 - remaining construction energy` calculation as a physical-energy rule.
- Initialize the accounting baseline from the **realized initial state**: initial usable energy plus the energy stored in its realized bonds and any other actual initial energy holders. Do not count the constructor's search budget, genome reproductive allocation by itself, or material `potential_energy` properties as additional energy.
- If initial bond energy and initial usable energy are declared to be part of the initial state, they belong in `E_initial`; they are not also recorded as captured external input. Future energy captured from the imaginary sun is an explicit external input.
- Tick-zero reconciliation must include all actual holders. A default-zero aggregate counter is not a valid initial inventory when the organism already holds energy.

### 2. Dissipation and organism stress — decision

- **Heat is not temperature and is not a thermal reservoir.** It denotes energy spent/wasted and no longer available to the simulated biological energy economy.
- Record dissipated energy as an explicit sink/output in the energy accounting. Do not leave it available for later uptake unless a future, separately approved environmental-energy mechanism explicitly models that return path.
- When wasted energy occurs inside an organism, the same event may also increase its stress according to the stress rule. The energy flow and stress update are distinct: stress is a biological state/consequence, not an energy holder and not a second energy cost.
- Unpaid maintenance contributes to the stress mechanic; it is not energy debt by default. Any eventual damage from accumulated stress must use the approved heat-stress structural-damage selection rule in this README.
- Global reconciliation should therefore be expressed as initial tracked energy + explicit external capture = final tracked energy + explicit dissipated/output energy. Internal transfers cancel. Stress changes do not add another term to this equation.

### 3. COMBINE energy allocation — decision

For every formation, the usable-energy debit must reconcile to distinct destinations:

1. Energy retained as the realized bond's `bond_energy`.
2. Energy consumed as formation work.
3. Remaining energy dissipated as waste/output (and, when the waste occurs inside an organism, potentially contributing to stress).

Do not treat the entire formation investment as structural energy if the realized bond retains only part of it. Only introduce different allocation rules for rigid/fluid or other bond types when the modeled physical mechanism requires that difference. Every destination is counted once.

### 4. Source-specific energy conversion — decision

- Do not introduce a universal intermediate-energy pool by default.
- A realized bond may release its stored bond energy once when it breaks. The receiving destination depends on the specific modeled pathway and must be explicit.
- Chemical accumulation and resonance are reaction/coupling signals, not energy holders by themselves. A chemical reaction may legitimately release chemical potential, but the released amount must come from an explicitly defined chemical-energy change and be accounted for at the transition. An accumulated interaction signal alone is not proof that energy was released. Chemical BREAK may therefore be funded by a validated chemical-potential release, already-accounted stored energy, explicitly recorded external capture, or a valid combination; each source must be identified and credited no more than once.
- The sun's 440 Hz signal is omnipresent and inexhaustible, but capture is explicit and rate-limited. Resonance controls coupling effectiveness; chemistry determines whether/how captured energy can be converted into usable energy.

### 5. Death, bond breaking, and material return — established architecture

**Decision (already established; not an open question): decomposition is not a separate chemistry system to design.** The unified energy ledger and ordinary physical/chemical rules are the mechanism. Do not pause energy unification to invent decomposition-specific behavior, special bond-breaking rules, a nearest-organism transfer policy, or a dedicated decomposition energy pool.

- **Death is an observed state produced by ordinary chemistry, not a hard-coded destruction event.** The organism is dead when its genome core has been breached. That breach may occur directly through bond-breaking chemistry or indirectly when inability to sustain itself causes stress damage that breaks open the core. There is no separate death-specific material conversion or bond-breaking rule. The only permitted special handling is reducing computational effort after core breach; the entity's remaining structure, bonds, materials, and energy continue to obey the normal physical and chemical rules. Death ends organism status, not physical existence: matter remains available for reuse through ordinary physical/chemical interactions.
- **Existing BREAK is the structural mechanism.** A realized structural bond is broken by the same bond/chemistry rules and energy accounting used elsewhere. Bond energy may be released only once. Do not create a second decomposition-only energy yield or regenerate material potential as energy.
- **Matter is conserved.** The realized constituent materials persist through structural bond breaking and become ordinary available physical material when no structural bonds remain. Do not duplicate, delete, or silently change material identity/quantity.
- **Energy has one authority.** A death handoff moves actual usable-energy holdings (including developing-offspring energy) without creating or losing energy. BREAK settlement accounts for released bond energy, usable-energy change, work, and dissipation. Unrecovered energy may be explicitly dissipated as an accounted sink; it must not vanish merely because a lifecycle object is removed.
- **No arbitrary nearest-organism transfer.** Energy can be reused only through an actual, local, chemistry-authorized interaction or an already-defined physical transfer rule. Distance ranking alone is not an energy-transfer mechanism. Do not add a new radius or special local-capture rule solely for decomposition.
- **Do not preserve a legacy implementation merely because it exists.** The current code has a `DecomposingBody` lifecycle object and a dedicated bond-resolution path. Treat that as legacy orchestration to audit against the architecture—not as a required new physical or energy concept. Keep a temporary carrier only if the existing tick/lifecycle needs it; it must use ordinary BREAK settlement, preserve all holders/material, and cannot own a separate energy law. Remove or simplify it only after tests show the unified path covers the same lifecycle behavior.

This resolves the previous open questions: no additional user approval is needed for the decomposition architecture, energy destination policy, or nearest-organism rule. The remaining work is implementation and verification, not another design round.


### Death is chemistry-observed; simulation effort is reduced after core breach — 2026-10-19

Death is not a special-case lifecycle command that destroys, converts, or automatically disassembles an organism. It is an observational classification: the genome core is breached. A direct chemical bond break can breach it, or an organism's inability to sustain itself can cause stress damage that eventually breaches it. Both routes use ordinary chemistry and energy accounting. The only death-specific behavior permitted is reducing computational effort once the core is breached. Remaining matter, structural bonds, stored energy, and usable energy persist and continue through normal physical/chemical rules. Any `DecomposingBody` or recycling orchestration must be evaluated against this rule; it must not become a separate chemistry system or erase the body's physical existence.

**Implementation checkpoint — 2026-10-19:** the simulation step now retains breached organisms in `Simulation::organisms`, derives living status from the realized genome cavity, skips biological maintenance/memory, material acquisition, decision/movement, and reproduction for bodies whose core is breached, and still runs the ordinary chemical-reaction pass on their physical structure. Stress damage may breach the core through the existing bond-breaking path. The former automatic recycling/decomposing-body handoff is no longer called by the normal tick lifecycle. Population reporting now counts only organisms with an intact qualifying core, while the headless and browser simulations continue advancing the physical world even when that count reaches zero. Regression coverage was updated to assert that a breached body's structure remains present and no material is moved to the environment solely because of death. In-flight BREAK/COMBINE transactions are allowed to finish after core breach: they already hold reserved physical/chemical state, and dropping them would risk losing material or energy. Breached organisms cannot initiate new transformations because their decision/action loop is disabled. **Source changes have not yet been compiled or test-run; this checkpoint is not a verified green build.**

### 6. Remaining implementation gates — not design questions

1. **Finish the global energy inventory.** Reconcile explicit initial energy plus source-backed external input against every final usable-energy holder, stored bond energy, and explicitly recorded dissipation/output. Internal transfers cancel. Do not mistake per-transaction balance or a partial holder counter for whole-world conservation.
2. **Complete post-breach lifecycle verification.** Audit all biological entry points and pending transformations so a breached core receives no further biological processing, while ordinary chemistry and physical state remain active. Verify energy/material conservation and ensure legacy recycling/decomposition helpers cannot be reached from the normal death path. Add regression coverage for core breach caused by both direct bond-breaking chemistry and stress damage.
3. **Unify structural BREAK.** Compare voluntary chemical BREAK, other BREAK pathways, and the current decomposing-body bond path. Identify which can call the same authoritative physical/energy settlement and what minimal lifecycle orchestration remains necessary. Preserve the rule that a bond's stored energy is released at most once.
4. **Close the finished-body energy path.** If a temporary carrier reaches zero bonds with a positive usable-energy balance, explicitly account for the remainder before removing it. Do not silently drop the holder. Prefer an existing valid transfer/sink over a new decomposition-specific reservoir.
5. **Wire world-tone capture after the ledger boundary is sound.** The source model and cached samples provide deterministic per-tick source values, while external-input transactions and a shared-budget primitive exist. The live tick still needs one shared source allowance, receiver allocation, bounded resonance-to-capture conversion, and source-backed settlement. Reflection must carry only remaining incoming energy and obey the same propagation/obstruction rules.
6. **Then verify end-to-end.** Add tick-zero inventory, atomic death/handoff, one-time bond release, matter-return, repeated chemistry-cycle, shared-source competition, and whole-world reconciliation tests. Run formatting, compile, focused gates, full Rust tests, and Clippy on the exact resulting head; record results without calling the work green if any gate fails.

**Ordering:** inventory and holder audit first; death/BREAK path audit second; implement the shared accounting/lifecycle correction once; audit the changed paths twice; then wire live world-tone capture. Do not reopen the resolved decomposition design questions, and do not change PR #196.

### Exact-head code audit — 2026-10-19

Read-only audit of the PR #202 branch after the architecture correction above. These are concrete code gaps, not open design questions:

1. **Chemical BREAK source provenance is unresolved; chemical energy release itself is allowed.** `src/chemistry/chemical_reaction.rs` converts accumulated chemistry state with `normalized_chemistry_to_energy` and passes that amount to `settle_chemical_break_energy`; `src/chemistry/transformation.rs` records the amount as `potential_released`. The accumulator is a reaction-driving signal, not a reservoir, so this conversion alone does not demonstrate that the credited amount was physically released. A second overlap makes this especially important: `src/chemistry/combine.rs::intrinsic_bond_potential` uses the same normalized `interaction_potential` equation to set the realized bond's stored `bond_energy`. Chemical BREAK must therefore distinguish that already-accounted structural store from any additional chemical-potential change, and settle the actual change exactly once when the bond transitions. Do not prohibit legitimate chemical energy release; validate its source and prevent the same potential from being released twice.
2. **Prepared COMBINE aggregate work accounting — corrected in the code change following this audit.** `src/chemistry/transformation.rs` now adds `ledger_delta.total_work_expended` when committing the prepared transaction. The change is source-reviewed but has not yet been compiled or tested.
3. **Structural BREAK has multiple settlement paths.** Stress BREAK and the decomposing-body path release a bond's stored `bond_energy`; chemical BREAK uses a chemistry-generated quantity and a disruption-work allocation; voluntary BREAK for stored physical material derives interface potential through a separate adapter. Audit the distinctions, but converge on one authority for transaction validation and source attribution without falsely treating all three sources as identical.
4. **Death-handoff atomicity needs a failure-path proof; a partial-transfer defect is not confirmed.** `src/chemistry/recycling.rs::recycle_dead_organism` moves stored material and reproductive construction into a temporary body and transfers parent/developing energy sequentially. The transfer helper validates amounts before mutation, and the amounts are prevalidated, so the sequential transfers appear designed to succeed under current preconditions. Do not call this a confirmed defect without a reachable failure case. Add a focused failure-path test or simplify the claim if inspection shows no such case.
5. **The legacy body's arbitrary nearest-organism transfer — corrected in a follow-up code change.** `src/simulation/simulation.rs::process_decomposing_bodies` no longer transfers positive energy to the nearest organism based on distance alone. Released usable energy remains in the body's holder; any future transfer must use an already-authorized physical/chemical path.
6. **Finished-body residual energy — explicit sink added, pending tests.** Before removing a finished body, the lifecycle now settles any positive residual usable-energy budget as dissipated heat through the shared transaction ledger. If settlement fails, the body remains rather than losing its holder. Non-finite or negative residual balances also retain the body for diagnosis. This closes silent holder deletion, but the sink choice and failure behavior still need focused regression coverage and end-to-end verification.
7. **The ledger is still transaction-level, not global.** `EnergyTransaction::balanced` checks a per-transaction equation and `EnergyLedger` aggregates event counters; it does not prove that every `potential_released` value came from a debited source, that every holder is inventoried, or that final stored bond energy plus usable energy plus recorded sinks reconcile to initial energy plus external input. The external-input shared-budget primitive is not yet called by the live tick, and reflection is not an energy-bearing path.
8. **The transaction schema cannot yet express stored structural energy as a source without re-labeling it.** `EnergyAllocation::balanced` treats `source_energy` as the only source and requires `structural_delta >= 0`; `EnergyTransaction::expenditure` likewise only admits nonnegative structural additions. Meanwhile, structural BREAK paths pass the released bond potential as `potential_released` while the bond is removed from the structure. Chemical BREAK passes an independently accumulated reaction amount as `potential_released` and leaves `structural_delta = 0`, even though the realized bond also disappears. This schema cannot distinguish a true chemical-potential release from a debit of the bond's already-stored energy, or represent both cleanly. Before wiring live world-tone capture, revise the authoritative transaction model to support explicit source provenance and signed structural-store changes, then migrate COMBINE/BREAK/decomposition together and prove one-time debits.

**Audit limitation:** this pass inspected the current PR #202 source and README through GitHub. No Rust commands or tests were run locally. The README correction commit is `2ddd0857741b6a363b68cb09afadfcb464514602`; the PR remains draft, and no PR-triggered workflow run was returned for that README-only commit. Do not infer that the branch compiles or passes tests from this documentation change.

## Energy accounting implementation — transaction allocation slice — 2026-10-18

**Scope:** first implementation slice on PR #186's `integration/unified-source-and-catalogue` branch. PR #196 and constructor/Bob work remain untouched.

- `EnergyTransaction` now balances energy across four explicit quantities: change in usable energy, persistent structural energy, work expended, and dissipated energy. Work has its own aggregate counter rather than being mislabeled as heat.
- COMBINE now allocates only a surviving realized bond's `bond_energy` to persistent structure. Formation work is recorded as work; the remaining investment is recorded as dissipated energy. For fluid-fluid bonds consumed by immediate coalescence, no structural energy is retained and the transient bond energy is dissipated.
- Chemical BREAK records its disruption barrier as work, not as newly retained structural energy. Other BREAK and decomposition transactions explicitly record zero work in this schema; this change does not redesign decomposition.
- Focused regression assertions verify the allocation and conservation of the existing COMBINE debit, including the transient fluid-bond case. The serialized work counter defaults to zero when reading older ledger state.
- **Boundary not yet solved:** this is not yet a simulation-wide inventory, source-backed settlement, external sun-capture implementation, or whole-world conservation proof. Those remain subsequent stages; do not claim global conservation from per-transaction balance alone.


## Shared external-source budget settlement — 2026-10-19

**Implementation slice on PR #202:** external inputs can now settle against a mutable `EnergySourceBudget` shared by all receivers drawing from the same accounting interval.

- `EnergySourceBudget` validates that its initial allowance is finite and nonnegative.
- `settle_external_input_from_budget` checks source identity, transaction balance, and remaining budget before settlement. On success it credits the holder, updates the ledger counters, and debits the budget; on failure, neither holder, ledger, nor budget changes.
- The source allowance is an accounting primitive. The caller must define its scope (for example, a particular source during one tick) and pass the same budget instance to competing receivers. Creating a fresh budget per receiver would defeat shared-budget protection.
- The transaction's `source_available` remains the local availability bound for that capture attempt. The shared budget's `remaining` is the aggregate unclaimed allowance; these are different constraints and are checked separately.
- Regression tests cover competing claims against one budget, overspending, atomic rejection, and invalid budget initialization.

**Still not implemented:** simulation tick owns no live shared world-tone budget yet; world-tone capture is not wired into the tick; reflection is not an energy-bearing field; and this primitive does not prove whole-world energy conservation. Ordinary chemical transactions still use `EnergyTransaction`, while external input uses `ExternalInputTransaction`; both now adapt to the same `EnergyAllocation` balance rule. Aggregate reconciliation across all holders and energy sinks remains open. No decomposition behavior is changed here.

## Shared transaction allocation contract — 2026-10-19

Both `EnergyTransaction` (chemical/internal energy) and `ExternalInputTransaction` (source-backed external energy) now expose an `EnergyAllocation` view and use the same balance validation:

`source_energy = usable_delta + structural_delta + work_expended + heat_dissipated`

- For chemical transactions, `source_energy` is the explicitly released chemical/bond potential. A pure expenditure can have zero released potential and a negative `usable_delta`; the holder-funded debit balances the positive structural, work, and dissipation allocations.
- For external-input transactions, `source_energy` is the explicit external input, and `structural_delta` is zero in this allocation. External input still has its own source identity, local availability bound, and shared-budget settlement path.
- The common allocation is a validation view, not a new energy reservoir and not permission to substitute one source type for another. It does not merge the two source counters or infer missing energy.
- A focused regression checks that both transaction types satisfy the same balance primitive while retaining their distinct source and structural-allocation semantics.

**Next boundary:** aggregate reconciliation still needs to include actual initial energy, every live holder, stored bond energy, and explicit dissipated/output energy. The common transaction rule is not itself a whole-world conservation proof. The live tick still does not settle world-tone capture against one shared source budget, and no decomposition behavior is changed by this slice.


## Energy-holder audit and first reconciliation fix — 2026-10-19

Audit of PR #202 at `24fbe6b741ad29e970e02311c14509e378e3d082` found that `total_usable_energy_held` counted live organisms and their developing offspring, but omitted the usable-energy budgets held by `DecomposingBody` instances. That made the held-energy counter drop when a dying organism transferred its usable energy into decomposition, even though the transfer created or destroyed no energy. The counter was also left at its default zero until the first lifecycle refresh, despite the initial organism receiving an explicit usable-energy allocation.

The first reconciliation fix now:
- includes live-organism usable energy, developing-offspring usable energy, and decomposing-body energy budgets in the same held-usable-energy total;
- initializes that total when a simulation is created, using the initial organism's actual assigned energy;
- refreshes the same total after the lifecycle processes deaths and decomposition.

**Scope boundary:** this counter is still not a whole-world energy inventory. It measures usable energy held in these three holder classes only. It does not yet include energy retained structurally in bonds, quantify energy remaining in an active source/reflection field, or reconcile all historical flows against initial energy plus external input. Structural bond energy is a distinct store and must be inventoried without double-counting the released-potential flow counter. Initial construction also uses a very large starting allocation formula, so the initial baseline must be made explicit before a global conservation assertion is designed.

**Verification:** source changes are committed on PR #202 only. Rust formatting, compilation, focused tests, and full tests have not been run in this GitHub-only pass; do not interpret this change as verified or as proof of conservation.


## Deceased reproductive energy audit — 2026-10-19

A follow-up audit found a second holder-transfer gap: recycling a dead organism transferred its own `usable_energy` into the decomposing body, but dropped `ReproductiveConstruction` without transferring its `developing_energy`. The narrow follow-up fix validates both energy stores before recycling, then transfers both into the body budget. A regression test covers a parent with 5 usable units and 3 developing-offspring units, expecting 8 units in the decomposing body.

The decomposing-body usable-energy counter now includes that carried energy while the body exists. A follow-up lifecycle change now settles positive residual energy as an explicit dissipated-heat sink before removing a finished body; if settlement fails, the body is retained. Material release is also deferred until finalization succeeds to prevent repeated deposits. These changes still require focused and end-to-end tests. Do not claim whole-world conservation until the residual-energy path and bond/material stores are reconciled.

**Verification:** no Rust formatting, compilation, or tests have been run in this GitHub-only pass.

## Finished-body material release ordering — 2026-10-19

A second lifecycle audit found that a finished `DecomposingBody` could deposit its constituent materials before the remaining usable-energy sink succeeded. If the energy holder was invalid or settlement failed, the body was retained but its material had already entered the environment; the next tick could deposit the same material again.

The finalization order now settles residual energy first, then releases material and removes the body. If the holder is invalid or settlement fails, the body and its material remain together. A focused regression test forces an invalid residual-energy holder and checks that the finished body remains and environmental material quantity is unchanged.

**Verification status:** the code and test are committed, but they have not been compiled or run. No PR-triggered workflow run or commit status was returned for this change. This is a source-level correction, not a claim that the branch is green.

## Structural-store energy accounting slice — 2026-10-19

A follow-up audit found that ordinary structural BREAK and the legacy decomposition BREAK path removed a realized bond while recording its bond energy as a generic `potential_released` source and leaving `structural_delta` at zero. That made the transaction look like a newly released source rather than a debit of energy already held in structure.

- `EnergyAllocation` now permits a signed structural-store delta. Positive values represent energy retained in a realized bond; negative values represent removal of a previously stored structural-energy amount.
- `EnergyLedger` now accumulates `total_structural_energy_delta` separately from non-structural potential release, with a serde default for compatibility.
- Ordinary structural BREAK and the legacy decomposition BREAK path now record the released bond amount as `structural_delta = -gross` and do not relabel that stored energy as a new source. Prepared transformation ledger deltas preserve this counter.
- A focused ledger test checks that a five-unit structural debit can fund two units of usable energy and three units of dissipation without incrementing the non-structural source counter.
- Geometry regression coverage now includes an unobstructed edge, a partially shadowed edge, and a fully occluded edge.

**Chemical BREAK source boundary updated in the follow-up:** reaction accumulation now serves only as the activation signal. The transaction debits the removed bond's stored energy through a negative structural delta, spends disruption work from that store first, and charges any work shortfall to the organism's usable-energy holder. The new focused tests cover the allocation equation and work-shortfall case. This is source-reviewed only: tests, compilation, formatting, Clippy, and the full Rust suite have not been run on this branch head. Whole-world conservation and live sunlight capture remain unverified.

## Simplified unified energy rule — 2026-10-20

**Design decision:** do not add density-based energy storage or a new per-element energy subsystem yet. Keep the existing usable-energy holder, explicitly allocated structural bond energy, and shared transaction authority. Resource-type potential describes what could be available; it is not itself energy currently held.

The common rule is: every credited energy amount must have been explicitly allocated or received from a tracked source. A transformation moves energy between existing stores, spends it as work, or dissipates it as heat. It must not derive a fresh energy credit from material properties merely because a bond is broken.

**First narrow implementation:** stored environmental compounds have pre-existing internal connection endpoints, but no energy allocation was recorded when those bonds were created. Their restoration into organism structure likewise assigns zero bond energy. Stored-material BREAK now changes connectivity without crediting usable energy, structural release, or heat. Ordinary structural bonds with a recorded `bond_energy` continue to release that recorded amount through the existing transaction rule. This removes the concrete unallocated-energy credit without adding a second storage model.

**Second transition fix:** COMBINE now prepares and commits its replacement structure on the same completion tick. This removes the interval in which energy had been settled but the old structure remained live, and prevents chemical or stress-driven BREAK from mutating the live graph between preparation and commit. The inventory therefore stays simple: it counts the live committed graph and does not need to understand pending replacement graphs.

**Shared structural BREAK settlement — 2026-10-20:** ordinary chemical BREAK and stress-driven BREAK now use one commit helper. It validates the proposed bond removal against a cloned structure, settles the signed structural-energy debit and usable/work/heat allocation through the ledger, then commits the graph change and records heat stress only after settlement succeeds. Chemical BREAK still pays its disruption work from the bond's stored energy first and charges any shortfall to the usable holder; stress BREAK uses its existing no-work yield rule. This refactor aligns commit ordering without changing the distinct physical rules. It is source-audited but not yet compiled or tested.

**Regression coverage added (not yet run):** `src/tests/simulation_tests.rs::stored_material_break_does_not_credit_unallocated_energy` directly checks that stored-material BREAK changes neither usable energy nor ledger source, work, heat, or structural-energy counters. `src/simulation/simulation.rs::combine_prepares_at_completion_without_changing_break_timing` checks that COMBINE is prepared at completion while BREAK retains its existing prepare timing. `src/tests/phase3_authority_contract_tests.rs::decomposition_break_reconciles_usable_and_structural_energy` exercises the actual decomposition bond-break function and checks that the usable-energy gain is matched by a structural-store decrease, with no duplicate source credit. `src/tests/phase3_authority_contract_tests.rs::stress_break_reconciles_usable_and_structural_energy` exercises the actual stress-driven BREAK path and checks its usable-energy, structural-store, work, and heat deltas against one residual. `src/simulation/simulation.rs::simulation_step_advances_decomposition_before_inventory_refresh` checks that a simulation tick advances decomposition and refreshes the live inventory afterward. The staged-inventory-only test was removed because the staged state no longer exists in the intended lifecycle.

**Latest source-path audit — 2026-10-20:** across the current live transaction construction sites inspected in `transformation.rs`, `decomposition.rs`, `state.rs`, `simulation.rs`, and `combine_runtime.rs`, no nonzero `potential_released` value is currently credited. COMBINE allocates energy from the organism's usable holder; stress-driven BREAK, chemical BREAK, and decomposition BREAK debit the realized bond store through a negative structural delta; stored-material BREAK credits nothing because its stored internal bonds have no recorded energy allocation. This closes the known live untracked-credit paths at source level, but does not prove every tick-level transition reconciles.

**World-tone boundary:** the rotating source model is precomputed/sampled for perception, but the simulation tick does not call `settle_external_input` or `settle_external_input_from_budget`. No sunlight energy is currently credited, and no shared source allowance is yet enforced among receivers. Keep capture disabled until that integration is source-backed and a whole-world residual is tested.

 **Lifecycle correction (2026-10-20):** do not invoke a separate decomposition processor from the production tick. A breached organism remains a physical structure and continues through the ordinary `chemical_reaction::accumulate`/`resolve` pass; biological maintenance and behavior may stop independently. “Decomposition” is an observation of chemistry after death, not a distinct process or energy source. The `DecomposingBody` carrier and its dedicated bond-break helper remain staged legacy code for focused accounting tests only; they are not the live death lifecycle. The production path must not recycle a breached organism merely to force a decomposition state. Keep world-tone capture disabled until the live chemical/stress BREAK paths and whole-world energy inventory reconcile under exact-head tests.



**Verification status:** this GitHub-only pass has not run Rust commands or tests, and no CI status checks have been reported for the latest branch snapshot. The stored-material, legacy-carrier BREAK, and stress-driven BREAK regressions are added but unverified. The incorrect simulation-tick regression that treated legacy decomposition as a separate production process has been removed. The COMBINE prepare/commit timing change has been code-audited but has no dedicated integration regression yet. Whole-world conservation is not yet proven.


## User-approved solar intensity and interception rule — 2026-10-20

**Decision confirmed by the user:** the sun's 1–10 energy-unit output range is an intensity per tick per 1.0 unit of exposed projected width. The 2,400-tick cycle determines the current intensity $I_t$; the meaning of the unit is constant across the cycle.

- Incident energy for an element is $E_{\mathrm{incident},i}=I_t L_{\mathrm{exposed},i}$, where $L_{\mathrm{exposed},i}$ is its unshadowed projected width perpendicular to the directional sunlight. At intensity 10, projected exposed width 1.0 intercepts 10 energy units per tick; width 0.5 intercepts 5.
- This is an energy-flux rule, not a fixed energy pool divided equally among all visible elements. The model must not grant the same incident energy to overlapping/occluded elements or double-count the same incoming path.
- Use directional geometric visibility and projected extent rather than detailed photon/ray simulation. Existing geometry primitives are groundwork only; the full exposure-to-energy allocation is not yet live.
- Incident energy is not automatically absorbed energy. Material-specific resonance/chemistry determines the absorbed fraction; any energy not absorbed must remain unclaimed, continue onward, or enter an explicitly modeled reflection/loss path. The incoming energy balance must reconcile.
- Do not apply a second independent incidence-angle multiplier if projected geometry already includes that effect. Do not invent per-material absorption constants without a defensible derivation/calibration.
- This decision clarifies the previously recorded 1–10 source range: it is per tick per unit of exposed projected width, not the total energy budget for the entire map. The live simulation still does not perform solar capture or settlement; implementation and conservation tests remain required.

## Latest exact-head verification — 2026-10-20

Exact-head CI run [39458264248](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39458264248), commit `1fec8cdb244c7b5358d6534c76922378afa62b57`, completed with an overall **failure** result.

- **Passed:** formatting, source-file-size check, COMBINE architecture check, Bob compact-storage checks, empty-library open/reopen, concurrent Bob initialization, focused Bob/fluid transaction and three-piece-cap checks, focused fluid coalescence/restoration checks, and focused chemical BREAK checks.
- **Rust suite:** 457 passed, 28 failed, 1 ignored. The new `transformation::tests::stored_material_break_does_not_credit_unallocated_bond_energy` regression passed. The failure count remains 28, matching the earlier baseline of 28 failures; the suite is not green.
- **Clippy:** failed with 60 production-binary errors and 38 test-compilation errors, primarily warnings promoted to errors across existing Bob, constructor, and other code. These are not treated as evidence that the energy-accounting change caused those issues; isolate them against a known baseline before cleanup.
- **Energy-system boundary:** this run does not enable or verify live sunlight capture, a shared per-tick receiver scheduler, or end-to-end world conservation. Keep capture disabled until transition-level and tick-level reconciliation pass.

This entry supersedes earlier statements that no exact-head CI result was available at the time those historical notes were written. PR202 remains open and Draft; PR196 is unchanged.


## Visible world-tone capture primitive — 2026-10-20

The latest implementation adds the first explicit geometry-to-ledger solar capture path, but does not yet enable it in the live tick scheduler.

- Exposed polygon intervals now retain their owning body and constituent indices. This lets the capture path count only the receiver's visible surface instead of accidentally crediting a blocker or another body.
- `settle_world_tone_capture` computes polygonal visibility against the supplied bodies, sums the receiver's unshadowed projected extent, converts it to incident energy using the configured intensity-per-width rule, and settles absorbed energy through the external-input ledger.
- The absorbed input is split between usable energy and immediate heat using explicit caller-supplied fractions. No resource-specific absorption constants are invented. Unabsorbed energy is returned as a result but is not yet propagated/reflected to downstream bodies.
- Unsupported geometry and invalid fractions fail closed before ledger credit. The operation is receiver-local, not a finite global solar pool; the future scheduler must invoke it at most once per receiver per tick.
- Added focused tests for projected-width scaling and balanced ledger settlement, body ownership (a fully shadowed body receives no credit), and fail-closed validation.

**Still outstanding:** select absorption/usable fractions from approved material properties, integrate this primitive into the simulation tick with a per-receiver deduplication guard, account for reflected/unabsorbed energy where applicable, and prove tick/world inventory conservation. This commit does not claim live solar capture is active.


## PR #202 current audit checkpoint — 2026-10-20 (supersedes earlier status notes)

**Verified head at audit start:** `e4d0f2a495c028be8647cc6830990e9e4e00f211`. The geometry-to-ledger prototype now supports aggregate receiver settlement and per-constituent settlement using each constituent's own projected exposure and existing resonance/damping response. The frame tracks hidden constituents at zero exposure, rejects unknown constituent indices, and prevents duplicate or mixed aggregate/per-constituent credits within one frame. The caller still supplies the usable-energy fraction; no retention coefficient has been invented.

**Exact-head CI result:** formatting passed. Rust tests completed with **464 passed, 28 failed, 1 ignored**. The previous observed baseline was **457 passed, 28 failed, 1 ignored**, so this run adds seven passing tests without increasing the failure count; the 28 failures are in existing construction, geometry/contact, cavity/viability, harmonics, movement, reproduction, and storage paths, and are not claimed as fixed by PR #202. Clippy failed with broad existing production/test lint debt (60 production errors and 38 test-compilation errors in this run); it is not a green gate. Do not infer the PR-specific tests are all green solely from the aggregate comparison.

**Hard blockers before live capture:**

- Historical limitation at this earlier checkpoint: exposure discovery required polygonizable realized boundaries. The later exact point-source curved-visibility work and current mixed frame path supersede that restriction for supported circles, circular fluids, and exact fluid arcs; unsupported/invalid geometry still fails closed.
- The exposure snapshot is built from supplied organism-body geometry; live integration must audit and include every relevant physical occluder, including environmental and stored/decomposing material where physically present. Omitting blockers can create false sunlight exposure.
- The tick still does not settle world-tone capture. Do not wire the prototype into the live tick until boundary visibility and world-wide blocker coverage are sound, and usable-versus-heat allocation is defined from approved rules.
- Transaction-level balance and these focused geometry/settlement tests are not proof of whole-world conservation. Reconcile all holders, stored bond energy, work, heat, external input, and any modeled output before claiming conservation.

**Next implementation step:** audit the exact realized geometry and world-material inventory APIs, then add visibility support that preserves exact circle/fluid boundary semantics (or keep the entire capture frame fail-closed until it can). After that, connect the source to the tick and add tick-level conservation tests. Do not choose a polygonization tolerance or material retention fraction without an explicit design basis.


## PR #202 follow-up audit — 2026-10-20 (latest observed head)

- Head: `0ebb9542ebc9f735d62a1e91aef2f02d991d78b5`.
- Exact-head CI run [39468731106](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39468731106) completed with failure. Formatting, source-size, COMBINE architecture, and all focused Bob/fluid/chemical gates passed. Rust tests: **465 passed, 28 failed, 1 ignored**. The new `physical_material_geometry_preserves_realized_shapes_and_world_placements` test passed. Compared with the previously recorded 457-pass baseline, there are eight additional passes and the same 28 failures; this comparison is not a green suite and does not establish the failures are unrelated without a controlled same-revision baseline. Clippy also failed.
- The geometry helper adds no visibility behavior yet. It only converts realized physical material into placed constituent geometry, and requires the caller to resolve storage-intrinsic coordinates before use.
- Exact fluid primitives already exist in `capillary_geometry.rs` (`CircularArcBoundary` and `MultiContactFluidBoundary`), but source-exposure discovery does not consume those exact boundaries. The right next step is to adapt exposure/occlusion around these existing analytic primitives rather than introduce sampled polygons or arbitrary tolerances.
- Live capture remains disabled. The outstanding integration requirements remain: exact curved-boundary visibility, a complete world occluder inventory with correct transforms, an approved usable-energy/heat allocation rule, tick-level deduplication, and whole-world conservation tests.


## PR #202 implementation pass — 2026-10-20 (current checkpoint)

- Current PR head observed before this pass: `c2adcb783c304d28b058f2b82521f833f15d4cb6`; latest observed CI run: [39470904332](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39470904332).
- Latest full-suite result: **466 passed, 28 failed, 1 ignored**. The previous checkpoint reported 465 passed with the same 28 failures. This is one additional passing test, not a green suite. The 28 failures have not yet been proven to be inherited because no controlled CI result for the exact base revision has been established.
- Focused geometry/energy gates and formatting passed on that run. Clippy still fails with broad production and test-compilation diagnostics; separate base-branch lint debt from PR-introduced diagnostics before spending time on broad cleanup.
- **Implementation decision:** do not connect the current prototype to `Simulation::step` yet. `visible_polygonal_intervals` returns `None` when any supplied constituent cannot be represented as a polygon. Simply skipping circles or fluids would be unsafe: the same body list currently serves as both receivers and occluders, so a skipped target could be treated as receiving zero exposure while still being required to cast a shadow. The next geometry change must explicitly distinguish receiver surfaces from occluders and preserve exact boundary semantics for each supported form.
- **Pass order:** (1) inspect the exact physical-material and coordinate-transform APIs and design a complete snapshot contract; (2) extend exact visibility without polygonization; (3) connect one canonical settlement path to the live tick only after the usable-energy/heat rule is approved; (4) test tick-level conservation and duplicate-credit prevention; (5) compare full tests and Clippy against a controlled base-branch baseline.
- **Do not infer completion from the new passing-test count.** Live capture remains unimplemented; source sampling, geometry-frame construction, and ledger settlement are still separate pieces. No material retention fraction or geometry approximation tolerance is authorized by this checkpoint.


## PR #202 implementation pass 1 — source orbit validation — 2026-10-20

- Commit: `819cb14324d3dc8f6ba2604901a89f4f7c53ab39`.
- `WorldToneRuntimeConfig::sample` now rejects an orbit radius that does not clear the full rectangular world. The required radius is strictly greater than half the world diagonal; this prevents a source that is described as an external sun from travelling through the world or intersecting its corners.
- Updated runtime-configuration and precomputed-cycle test fixtures to use a radius outside the 1000×800 world, and added a negative test for an orbit inside the world. The low-level parameter sampler remains geometry-agnostic; the world-bound rule belongs at the runtime configuration layer where width and height are known.
- This is a bounded correctness fix, not live capture integration. The simulation still does not settle world-tone energy, and the exact-geometry/snapshot/energy-allocation blockers above remain.
- Verification status at documentation time: GitHub Actions was still running a workflow triggered by the preceding commit, so this latest commit's full test result is **not yet verified**. Do not treat formatting or test success from the earlier head as verification of this change.


## PR #202 implementation pass 2 — configuration activation guard — 2026-10-20

- Commit: `21723d6251c389fc5cb203c45eb1fbad70e973d5`.
- The runtime configuration entry point now validates the orbit against the actual environment dimensions before storing it. Previously, `configure_world_tone` checked only dimension-independent parameter validity, then could return success while precomputation rejected an orbit that crossed the world. It now rejects that configuration before changing the active source or cycle.
- Added a focused regression test asserting that an inside-world orbit is rejected and leaves both runtime source and precomputed cycle unset.
- Audit result: this closes the activation inconsistency introduced by the new radius invariant. It does **not** solve source integration, curved visibility, full-world occluder collection, or energy-allocation policy.
- Exact-head CI is still unverified. The currently visible in-progress workflow was triggered before these latest commits; its success/failure cannot be used as evidence for commit `21723d6`.


## PR #202 implementation pass 3 — geometry safety and formatting — 2026-10-20

- Formatting failures in `src/geometry/source_exposure.rs` and `src/simulation/world_tone.rs` were corrected in commits `08cbe3f5b1cf9ec1483804dbf2f14c00a5fa91ef` and `fa01c57919ed5853fa3b60167d6f6c9adbb6a238`. The workflow triggered after those corrections advanced past formatting and the focused gates into Rust tests; its final result was still pending at this note.
- Commit `75cef0f3cf6f96a01883fee2c1a6419996b29d63` makes polygonal visibility fail closed for non-convex target geometry. The current solver excludes a target constituent from its own occlusion test, which is safe for convex targets but could over-credit a concave target that shadows itself. A regression test covers the rejection.
- This is a conservative correctness boundary, not general concave/self-occluding visibility support. Curved receiver visibility under blockers, complete world occluder snapshots, live tick integration, and whole-world energy reconciliation remain open. The latest commit's formatting and tests still require exact-head CI verification.


## PR #202 implementation pass 4 — finite point-source curved visibility — 2026-10-20

- The source model is a finite point source at the sun's center. Rays diverge from that position; the curved-boundary query does not substitute a single parallel direction for the whole arc.
- Added `visible_circular_arc_intervals` in `src/geometry/source_exposure.rs`. It partitions supported circular receivers at critical rays through straight-boundary vertices and intersections, circular-arc endpoints and intersections, and circle tangencies, then classifies each resulting interval against realized occluder geometry. Returned intervals are exact circular arcs in world coordinates; no tessellated polygon is used.
- Added `point_source_projected_arc_extent`, which integrates the local outward-normal projection using the actual source-to-boundary direction along each arc, with adaptive Simpson integration. It is a geometry measurement only and does not allocate or settle energy.
- Supported as curved targets: circles, legacy nominal-area circular fluids, single circular-segment fluid arcs, and free arcs in valid multi-contact fluid boundaries. Straight contact edges and the implicit chord of a single circular-segment fluid are included as occluders but are not credited as exposed fluid surface. Compound-boundary self-shadowing is classified against the full realized boundary; invalid geometry fails closed.
- Added tests for unoccluded point-source circle exposure, polygon and circle shadow transitions, circle/straight-boundary intersections, point-source projected extent, and multi-contact fluid arcs with their contact chord included in self-occlusion checks. Geometry-frame tests cover complete and partial shadows between circular receivers.
- Historical status note from this pass: the point-source curved query initially fed only the exact-circular geometry-frame fallback. The current checkpoint above supersedes that limitation: the frame now calls both straight and curved visibility queries. Live tick settlement, complete world occluder inventory/transforms, approved energy/heat allocation, and whole-world conservation remain outstanding.
- Exact-head CI is pending; do not treat earlier formatting or Clippy results as verification of this pass.
