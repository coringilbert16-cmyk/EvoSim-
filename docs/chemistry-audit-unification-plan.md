# Chemistry Audit and Unification Plan

**Status:** Read-only audit register A0–A21 populated; owner approved Section 17 and implementation is tracked separately in [PR #205](https://github.com/coringilbert16-cmyk/EvoSim-/pull/205). PR #204 remains documentation-only; no production behavior changes are made on this branch.  
**Baseline branch:** `main`  
**Baseline commit:** `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5`  
**Plan opened:** 2026-10-22  

> **Standing handoff requirement (owner instruction, 2026-10-22):** PR #204 must be handoff-ready at all times for a future ChatGPT/model instance to resume without depending on this conversation. Treat this document as the durable source of truth for decisions, rationale, exact repository/branch/commit state, current implementation phase, latest CI/test/Clippy results, unresolved risks, and next concrete actions. Update it whenever meaningful implementation or verification state changes; replace stale “queued/pending” notes when results arrive; explicitly distinguish verified facts from assumptions and historical snapshots. Keep PR #204 information-only: implementation belongs on PR #205, and this document must never imply unverified work is complete.

**Scope:** Read-only audits A0–A21 are recorded; this does not mean the chemistry system is unified or conservation is proven. PR #204 is a durable planning/handoff artifact only: no production code or tests may change here. The current canonical owner decisions are in Section 14 and supersede earlier unresolved-question lists and provisional recommendations. A0 records exact-head counts. The individual baseline failure identities were extracted later and compared by name against implementation runs; see the dated verification reconciliation in Section 17.7. The original A0 limitation is historical, not the current evidence state.

> **Handoff consistency rule (2026-10-22):** Section 14 is the canonical owner-decision log; Section 15 is the latest reconciliation of those decisions against the audited source. Earlier audit records preserve what was observed at the time, but any provisional recommendation or unresolved-question list that conflicts with Sections 14–15 is superseded. A decision being settled does **not** mean it is implemented or verified. Current unresolved work is evidence gathering and implementation coverage, not re-selection of the owner's decisions.


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
- **World-tone capture status must follow the live call path, not stale documentation.** The audited source contains a live capture-settlement path, so capture must be described as present/active but not yet conservation-validated; do not claim it is disabled. This audit PR must not toggle runtime behavior. The source model is deterministic and directional; occlusion matters. “Reflection” means onward propagation with changed frequency composition, not literal mirror-ray tracing.
- **Bob remains a geometry library worker/viewer.** Prefer persisted suggestions over repeated runtime geometric discovery. This chemistry work does not pull in deferred PR #196 constructor changes.
- **Scalability is a correctness constraint.** Do not solve conservation by cloning the whole world each tick, doing all-pairs reaction scans, tracing a ray per object, retaining unbounded event histories, or persisting redundant derived state.
- **No silent baseline rewriting.** Exact-head `main` workflow/commit evidence records 563 passed, 28 failed, and 1 ignored at `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5`; named baseline failure identities were extracted from workflow `39705628427`. At head `686bcd0`, workflow `39814502886` reported 577 passed, 28 failed, 1 ignored, with exactly the same 28 named baseline failures; the cache missing-manifest regression test passed, and normalized Clippy diagnostics matched baseline. Head `4f4b2af` reported 581 passed, 28 failed, 1 ignored with the same 28 baseline failures and all four temporary oracle tests passing. The duplicate oracle pair was then removed and a cache partial-append rollback/reopen regression test added. Head `25f1df5` fixed the cache test fixture syntax and formatting. Its workflow `39818514455` completed the suite: 580 passed, 28 failed, 1 ignored in 236.55 seconds; all 28 failure identities matched baseline, the cache append rollback test passed, and normalized Clippy messages matched baseline. Latest head `0148437ae04fc2bcd4ce173fd6749f616b4b3cff` was verified by workflow `39818747953`: 580 passed, 28 failed, 1 ignored in 237.90 seconds, exact same 28 baseline failures; the two oracle tests and cache append rollback test passed, and normalized Clippy diagnostics matched baseline. Do not weaken physical invariants or call CI green.
- **README is the current project guide.** Dated handoff archives are historical evidence and may contain superseded or contradictory snapshots. Confirm every finding against current `main`.

## 3. Baseline and claims requiring verification

This section describes evidence gaps, not open owner decisions. Where earlier audit records conflict with Section 14 or the later source reconciliation in Section 15, the later explicit decision/reconciliation controls; earlier entries remain historical audit evidence and must not be treated as current instructions.

- Whole-world energy conservation across every storage, reaction, and lifecycle path is **not yet proven**.
- A live world-tone capture/settlement path exists in the audited source. Its complete conservation correctness, occlusion/error behavior, and performance are **not yet proven**; do not describe the path as disabled or toggle it in this documentation-only PR.
- Complete self-maintaining organism viability is **not yet proven**.
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

## 5. Audit register — completed read-only record

**Status interpretation for this handoff:** A0–A21 have audit records and summary rows; the final source-path closure is recorded in Section 16. Earlier “Decision required from project owner” fields and provisional recommendations are historical snapshots, not current owner requests. Apply Section 14 for settled design direction, Section 15 for the README/main reconciliation, and Section 16 for the final death, persistence/cache, and material-ownership call-path checks. A settled decision can still have an implementation gap. No audit is marked verified merely because a helper has tests.

The following is the required record schema used when adding or revising audit evidence:

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
**Audit record — 2026-10-22**

**Status: Baseline counts verified; the initial identity-level evidence gap was subsequently closed for earlier implementation comparisons (see Section 17.7). Latest-head full-suite status remains unknown while the current CI run is in progress.**

- **Exact baseline:** main resolved to merge commit cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5 during this audit. PR #203 is in fact **merged** (merge commit above; PR #203 head 6104cdf952c06cc3184e32abc2b116c9512935fa), contradicting README.md, which still calls it an unmerged draft. README's maintenance/test/Clippy claims describe older commit 67bceda1672891e7e390cf242295477bfb5cf3a0 or PR #202 source, not this current merge commit.
- **CI evidence:** GitHub Actions workflow `Rust`, run `39705628427`, is attached to exact `main` SHA `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5` and completed at 2026-10-22 01:30:37Z with conclusion `failure`. Its `test` check (`119584875250`) completed at 01:30:36Z and reports four annotations. The exact-head commit message records **563 passed, 28 failed, 1 ignored**, states all 28 full-suite failures match the accepted baseline, says focused Bob/fluid/chemistry checks passed, and records strict Clippy at 34 application-target plus 15 test-target errors. Thus the count is evidenced for this exact `main` SHA; the individual failure names/assertions were not independently extracted from run logs during this pass. Do not mislabel the suite as green or present the recorded Clippy errors as chemistry regressions.
- **Scope map established:** the crate is organized through src/main.rs path attributes. Chemistry-relevant active/staged modules include src/chemistry/{chemistry,chemical_reaction,combine,combine_runtime,energy_ledger,decomposition,transformation}.rs, src/materials/resources.rs, src/perception/harmonics.rs, src/simulation/{state,world_tone,world_tone_snapshot,simulation}.rs, plus physical-material, storage/transfer, geometry/contact, movement, maintenance, reproduction, persistence, diagnostics, and UI paths. Cargo.toml identifies a single Rust 2021 package; no separate library target was declared there.
- **Historical audit lead, resolved by later A8/Section 15 inspection:** README.md said live world-tone capture remained disabled, while `src/simulation/world_tone_snapshot.rs` showed a live settlement path. The later source review found the path present; correctness and performance remain open, not whether the path exists.
- **Baseline failure identities:** during the initial A0 pass, the commit message attested that all 28 failures matched the accepted baseline but the named list was not independently retrieved. That limitation was subsequently closed: names were extracted from workflow `39705628427` and compared against implementation runs at `25b72bb` and `407a163` (see Section 17.7). The latest implementation head still needs its own completed full-suite result before its failures can be compared.
- **Out of scope confirmed:** PR #196 remains separate; this audit changes no production code and does not authorize chemistry behavior changes.
- **Evidence:** README.md; PR #203 metadata/merge commit; Cargo.toml; src/main.rs; src/simulation/world_tone_snapshot.rs; current-head status/run queries (empty).
- **Dependencies / downstream audits affected:** every audit must cite the exact SHA it inspected. Baseline identity comparison is documented in Section 17.7; latest-head test status remains pending.
- **Decision required from project owner, if any:** none to continue read-only auditing. Do not update README's claimed baseline until exact-head evidence is found.
- **Recommended smallest next change (if authorized):** after all audits, update the README's status block from verified merge and exact-head evidence; no change authorized now.

### A1 — Resource vocabulary, properties, and units

**Historical note:** the initial audit below correctly records that the then-approved vocabulary said Methane, but that owner decision was explicitly superseded by D1 in Section 14. Oxygen is now canonical. Treat A1's “critical catalog/spec mismatch” language as historical; the remaining audit task is to trace stale Methane references and persistence/cache compatibility, not to re-decide the resource name.

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
**Audit record — 2026-10-22**

**Status: Gap found.**

| Property / representation | Current implementation evidence | Audit interpretation |
|---|---|---|
| Resource catalog | src/materials/resources.rs::default_catalog defines seven entries: Carbon, **Oxygen**, Hydrogen, Sulfur, Nitrogen, Phosphorus, Water. | **Critical catalog/spec mismatch:** approved EvoSim resource vocabulary uses Methane (resource ID 6), not Oxygen. Oxygen currently occupies the triangular resource entry (chemical position 1.5, potential energy 20, mass 0.75). Do not silently rename it during audit; trace every serialized/catalog/library key and define migration compatibility before any fix. |
| Immutable properties | ResourceProperties holds mass, potential energy, legacy reactivity, optional chemical position, and cohesion; BaseResource adds physical state and shape. | Type-level distinction exists, but there is no dimensional type system; units are implicit f64s. Validate all catalog entries and consumers. |
| Chemical coordinate | Catalog positions observed: Oxygen 1.5, Sulfur 3.5, Hydrogen 5.5, Water 7.0, Nitrogen 8.5, Carbon 10.5, Phosphorus 12.5. | Values match the intended ordered spectrum numerically except the named first resource. Catalog ordering itself is not the approved resource-ID order. |
| Potential energy | Material::potential_energy calls fresh_energy, which multiplies catalog potential energy by amount; unknown names return 0.0. | The API is a catalog-derived fresh-material potential estimate, not a mutable wallet. Unknown resource names silently becoming zero can conceal lost or malformed material; inventory/restore/serialization paths must be audited before choosing fail-closed behavior. |
| Material identity | Material stores parts: Vec<(String, f64)> plus indexed internal_bonds; validation checks positive finite amounts and bond indices/duplicates but not whether names exist in the supplied catalog (no catalog is passed). | Constituent identity and internal bonds are retained, but catalog membership is a separate validation responsibility. Fractional amounts are accepted; physical interpretation and geometry realization need cross-audit. |
| Mixed properties | Material::weighted_properties amount-weights mass/potential energy/reactivity/cohesion, but returns chemical_position: None for both empty and nonempty composites. ResourceBaselines::from_catalog averages mass/potential energy/reactivity/cohesion and has no chemical-position field. | Deliberate avoidance of averaging chemical position preserves the need for constituent identity, but any caller using only the aggregate properties cannot derive chemistry from a composite. Audit whether callers correctly iterate constituents or fail closed. |
| Baselines and ranges | ResourceBaselines::from_catalog averages raw catalog fields; property_ranges returns property ranges but leaves chemical position None. | These are summary/calibration helpers, not authoritative per-resource properties. Verify they are not reused as physical energy or as a synthetic resource. |
| Physical geometry | Resource shapes are declared in the same catalog; Hydrogen is a 1.0×0.1 rectangle, Sulfur is a parallelogram, Nitrogen a 1×1 rectangle, Phosphorus a trapezoid, and Water is fluid with nominal area 0.5. | Geometry is catalog-coupled. Preserve approved geometry while tracing the Oxygen/Methane naming discrepancy and any saved geometry-library keys. |
| Legacy reactivity | reactivity remains alongside chemical_position; current chemistry interaction uses chemical position, while harmonics use reactivity-derived damping/response. | These are two distinct mechanisms today, not a single interchangeable chemistry scalar. Decide after A7 whether the remaining reactivity role is intentional and physically interpretable. |

**Findings / risk ranking**
1. **P0 — resource identity mismatch:** the default catalog and its tests explicitly expect Oxygen while the approved resource set specifies Methane. A unification plan cannot be considered complete until IDs, catalog names, persisted materials, chemistry-library keys, geometry library entries, tests, and migrations agree.
2. **P1 — invalid resource names can be silently assigned zero potential/mass:** fresh_energy and Material::mass default unknown names to zero. Determine whether any runtime paths can persist or transfer such entries; avoid changing this API until all callers and migration policy are known.
3. **P1 — mixed material chemical position is intentionally absent:** ensure all chemistry consumers use realized constituent identity rather than treating a composite as one invented averaged species.
4. **P2 — units are implicit:** field names do not encode units or whether values are per unit, per constituent, or aggregate; the audit must publish a canonical table before changing calculations.

**Evidence inspected:** src/materials/resources.rs (ResourceProperties, ResourceBaselines::from_catalog, Material, fresh_energy, default_catalog and catalog tests); src/chemistry/combine.rs; src/perception/harmonics.rs; README.md and the approved project resource vocabulary.

- **Dependencies / downstream audits affected:** A2, A3, A7, A8, A11, A13, A14, A16, A20, A21. The resource rename/migration decision must precede any change to persisted keys or library lookups.
- **Decision required from project owner, if any:** no immediate decision requested; preserve the discrepancy as a blocking finding for the eventual unified catalog decision.
- **Recommended smallest next change (if authorized):** after the full audit, define one canonical resource ID/name mapping and a compatibility/migration policy, then update catalog, tests, serialized references, and library keys together. No code change authorized now.

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
**Audit record — 2026-10-22**

**Status: Gap found (core formulas located; dimensional contract is not yet globally proven).**

**Equation inventory from src/chemistry/chemistry.rs**
- Chemical separation: D = min(|position_a - position_b|, Dmax); nonfinite inputs, k <= 0, and Dmax <= 0 fail with None.
- Bounded static potential: (exp(kD) - 1) / (exp(k Dmax) - 1), clamped to [0,1]. Constants are Dmax=13 and k=1/13. Current catalog positions span 1.5 to 12.5, so catalog pairs do not reach the declared full-separation endpoint at 13.
- Contact attenuation: (1 - distance/radius)^2 for distance in [0,radius]; zero outside the radius. It is bounded and nonincreasing, and is exactly zero at radius.
- Attraction: static_potential × max_force × contact_factor. CHEMICAL_MAX_FORCE=1 is documented as a force scale, not an energy deposit.
- Interface engagement clamps finite facing to [0,1].
- Reaction accumulation: next = max(0, previous + potential × contact × engagement - dissipation × previous). Runtime caller passes CHEMICAL_DISSIPATION=0.10, so the formula removes 10% of the prior accumulator each tick while adding a bounded local interaction contribution.
- Normalized reaction-to-energy conversion multiplies by CHEMICAL_ENERGY_PER_NORMALIZED_UNIT = max_force × characteristic_distance (numerically 1.0 under current constants). Bond strength is converted to an activation barrier through the same conversion function.
- BREAK eligibility compares converted accumulated reaction energy against the effective activation barrier; the reaction accumulator is not itself a separately stored physical energy reservoir. BREAK settlement then uses bond-stored energy, usable energy, compatible resonant energy, work, and heat. These are distinct quantities and must not be counted as the same energy twice.

**What is currently supported by source**
- Inputs to the primitive math helpers are checked for finiteness and invalid sign/range where appropriate; invalid cases generally fail closed with None or false.
- Potential and contact terms are bounded; interaction potential is symmetric in the two positions; contact is zero beyond radius.
- The formula constants are centralized in src/chemistry/chemistry.rs and reused by src/chemistry/chemical_reaction.rs and src/chemistry/combine.rs for the main paths inspected.
- This is **not** proof that every caller, serialized accumulator, or ledger transition obeys the same units or remains bounded over arbitrarily long runs.

**Dimensional ambiguities / follow-up findings**
1. **P1 — implicit units:** chemical positions and static potential are dimensionless, while force and energy are physical quantities. The conversion treats one normalized accumulator unit as force × distance; this is a chosen scale convention, not a derivation from an integrated force along the actual reaction trajectory. Document that contract explicitly or derive a physically grounded conversion during implementation planning.
2. **P1 — accumulator ceiling not explicit:** with input term up to 1 and 10% linear dissipation, a sustained maximum input tends toward a steady-state accumulator near 10, not 1. The conversion can therefore yield energies above the nominal one-unit barrier ceiling. This may be intended for reaction accumulation, but the API/commentary should not imply the accumulator is bounded to [0,1]; decide whether an explicit cap is physically justified after inspecting persistence and runtime behavior.
3. **P1 — reaction threshold vs disruption settlement:** activation uses effective_barrier reduced by compatible resonance, but the queued operation retains the full barrier as disruption_cost; the resolver separately withdraws compatible resonance and accounts for work. This can be coherent, but the audit must prove the trigger and the eventual energy/work transaction are exactly aligned for every case, including stale events and failed settlement.
4. **P2 — distance domain differs by subsystem:** COMBINE permits candidate distance up to 0.1 (COMBINE_CONTACT_TOLERANCE), while chemistry attenuation uses radius 1.0; do not conflate geometric contact tolerance, interaction radius, and Bob's library/deduplication tolerance.
5. **P2 — finite arithmetic at scale:** helper inputs are checked, but caller sums, accumulated values, and serialized prior state require end-to-end overflow/long-run checks. accumulate_reaction does not explicitly reject a nonfinite computed next before returning Some(next.max(0.0)); if arithmetic overflows, Some(infinity) is possible and must be caught at a boundary. This is a concrete primitive-level validation gap to cover with tests/fix only after audit completion.

**Evidence inspected:** src/chemistry/chemistry.rs (constants, interaction/contact/attraction, conversion, accumulation, activation, unit tests); src/chemistry/chemical_reaction.rs (accumulation call, resonance credit, activation, queued operation); src/chemistry/combine.rs (force/work distinction and intrinsic bond potential); src/chemistry/transformation.rs (BREAK energy allocation); src/chemistry/energy_ledger.rs (allocation balance).

- **Dependencies / downstream audits affected:** A3–A9, A15, A18, A20. The equation sheet is provisional until all callers and persisted reaction state are mapped.
- **Decision required from project owner, if any:** none before the read-only audits complete. Do not retune constants now.
- **Recommended smallest next change (if authorized):** add explicit unit/quantity documentation and adversarial overflow/long-run tests after the full audit, then decide whether the normalized-to-energy convention or accumulator range needs redesign. No code change authorized now.

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

**Audit record — 2026-10-22**

**Status: Gap found (main COMBINE path located; full caller and edge-case matrix remains for integration audits).**

**Observed production path**
- Candidate eligibility is based on live physical contact candidates. The runtime rejects endpoint distance greater than 0.1, unavailable endpoints, invalid indices, missing physical IDs/properties, and failed formation criteria. When both constituents have chemical positions, a positive chemical attraction is required at the candidate interface. Geometry/contact authority remains separate from chemistry eligibility.
- Formation threshold/investment, work, bond strength, and stored intrinsic bond energy are separate values. Formation work is calculated by integrating the documented force/contact curve over the remaining approach distance; it is not simply the static potential copied into a wallet. Intrinsic bond potential is based on chemical-position interaction when both positions exist, with a cohesion-derived fallback for materials without positions.
- Bond creation is staged on a cloned OrganismStructure. The new Bond stores strength and bond_energy. A balanced EnergyTransaction is settled before the trial structure is committed, so the physical commit after successful settlement is an assignment rather than another fallible geometry operation. Failed candidate insertion or failed settlement returns without committing the trial graph.
- Ordinary persistent bonds allocate usable-energy expenditure into stored structural energy, formation work, and dissipated investment. A fluid-fluid pair is special: current coalescence supports a narrow same-resource/single-part topology; its transient bond energy and investment are allocated as heat because that bond does not survive the merge. Unsupported coalescence topology rejects the trial.

**Findings / risk ranking**
1. **P1 — caller coverage still needs proof:** the runtime has multiple entry points (ordinary stored-unit COMBINE, selected-candidate/developmental construction, environmental sources, and prepared multi-tick transformations). Confirm all of them reach the same transaction and that no alternate path writes bonds or energy directly; A12/A20 will cross-check callers.
2. **P1 — trial-structure clone cost:** bond formation clones the whole organism structure per attempted bond. This is transactionally clear but can scale with structure size and candidate count; measure before replacing it, and preserve atomicity. No full-world clone was observed in this bond helper, but the structure clone is still a per-attempt cost.
3. **P2 — fluid transient accounting depends on exact topology:** the current special case relies on whether both first constituent resource entries are fluids, while coalescence itself later enforces same-resource/single-part constraints. Cross-test every unsupported/malformed fluid case so no transient bond is treated as persistent structural energy.
4. **P2 — contact scales are distinct:** live COMBINE tolerance is 0.1, chemical interaction radius is 1.0, and Bob/library tolerances have different semantics. Keep them separate in any unified model.

**Evidence inspected:** src/chemistry/combine.rs (chemical interaction, formation work, intrinsic bond potential); src/chemistry/combine_runtime.rs (candidate validation, trial bond insertion, energy transaction, fluid coalescence); src/chemistry/energy_ledger.rs (expenditure balance); src/geometry/contact.rs and src/geometry/structure_authority.rs discovered as the physical authority paths.

- **Dependencies / downstream audits affected:** A1–A2, A5–A6, A9, A12–A16, A19–A20. The resource identity mismatch from A1 blocks safe catalog-key changes but not this path audit.
- **Decision required from project owner, if any:** none before the remaining audits.
- **Recommended smallest next change (if authorized):** after all audits, add explicit property/unit and transaction-invariant tests for each COMBINE entry point and unsupported fluid topology; profile structure cloning before optimizing. No production code change authorized now.

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

**Audit record — 2026-10-22**

**Status: Gap found (shared realized-bond path exists; lifecycle and rollback guarantees need cross-domain proof).**

| BREAK pathway | Current behavior observed | Energy/material interpretation | Remaining audit need |
|---|---|---|---|
| Chemical reaction BREAK | Accumulation crosses an activation threshold; resolve withdraws compatible resonance on a trial reservoir and calls settle_chemical_break_energy_with_resonance. | Reaction accumulation is a trigger, not a separate energy store. Bond structural energy pays disruption first; resonance supplies compatible work; usable energy pays any remaining deficit; efficiency partitions recoverable surplus vs heat. | Prove the threshold/withdrawal/work relationship for all values and stale operations (A2/A5/A7/A20). |
| Resonance-overload BREAK | resolve_resonance_overload_break detects retained resonance above total bond-energy handling capacity, chooses an eligible non-genome bond where possible, then calls the same chemistry/resonance settlement path. | Uses retained resonance as the energy/work source; genome-boundary bonds are protected while ordinary bonds remain. | Validate no negative/duplicate reservoir debit and no unintended core breach/death (A7/A10). |
| Voluntary BREAK of stored physical material | try_start_transformation removes the exact stored physical instance and records its internal bond. Preparation validates the bond and settles a zero-energy transaction. Completion splits the compound and returns pieces to storage or environment. | Existing stored-material bonds have no prior allocated structural-energy store; this path intentionally releases no energy/heat rather than minting energy from constituent potential. | Verify rollback/restoration if a staged operation is invalidated, and preserve material amount/placement (A13/A15/A16). |
| BREAK of environmental stored material | Same prepared stored-material route, with pieces deposited back into the environment after completion. | Connectivity changes; no energy credit because the environmental bond had no recorded stored-energy allocation. | Verify field mutation/recovery and failed deposit behavior. |
| Post-death decomposition | DecomposingBody bond release calls bond_break_energy_yield and settles negative structural_delta plus usable credit/heat through the same ledger interface; constituent material is released after its final bond is gone. | Existing bond potential is removed from structural inventory once; it is not relabeled as a new external/non-structural source. | Compare with recycling and the live simulation tick; ensure all final physical materials remain accounted for (A11/A16). |

**Shared realized-bond commit contract:** settle_break_transaction clones the structure, validates/removes the matching bond on the trial graph, settles the ledger transaction, then commits the graph and applies heat as stress. The resonance reservoir is also trial-cloned and only committed after successful settlement. This is a good local atomicity pattern.

**Findings / risk ranking**
1. **P1 — staged multi-tick state:** voluntary stored-material BREAK removes a physical instance from storage at start and resolves it later. The source-restoration helper covers failed preparation, but the full cancellation/stale-event path and energy/material rollback must be tested against persistence and tick ordering.
2. **P1 — post-death path ownership must stay singular:** src/chemistry/decomposition.rs is called by simulation code for decomposing bodies; do not treat the helper as test-only based solely on old archive notes. src/chemistry/recycling.rs separately transfers usable/developing energy and physical storage to the dead body's lifecycle. A11 must establish whether every dead physical constituent and energy holder is transferred exactly once.
3. **P2 — near-zero tolerances:** bond-break math and ledger settlement use tolerances; test exact equality, zero-energy bonds, near-zero usable holders, and repeated release so epsilon handling cannot accumulate meaningful energy.
4. **P2 — one semantic distinction matters:** a stored compound whose bond never had allocated structural energy is not equivalent to a realized structural bond with bond_energy. Preserve that distinction while sharing validation/transaction infrastructure; do not force a fake energy release onto stored materials.

**Evidence inspected:** src/chemistry/transformation.rs (chemical BREAK, overload BREAK, shared settle_break_transaction, stored-material transformation prepare/resolve); src/chemistry/decomposition.rs; src/chemistry/recycling.rs; src/simulation/simulation.rs; src/chemistry/energy_ledger.rs.

- **Dependencies / downstream audits affected:** A2, A5–A7, A9–A11, A13, A15–A16, A20. Death and recycling cannot be closed by this audit alone.
- **Decision required from project owner, if any:** none before the remaining audits.
- **Recommended smallest next change (if authorized):** after all audits, create a pathway matrix test that forces each BREAK type through success, stale/invalid input, and settlement rejection while checking energy, structure, reservoir, and material state. No production code change authorized now.

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

**Audit record — 2026-10-22**

**Status: Gap found (local transaction algebra is coherent; world-wide use and prepared-delta symmetry are not yet proven).**

**Canonical local equation**

Every EnergyAllocation currently checks the signed identity:

source_energy = usable_delta + structural_delta + work_expended + heat_dissipated

Equivalently, the residual source minus all destinations must be zero within EPSILON = 1e-9. The structural delta is signed: positive when a persistent bond store is created, negative when stored bond energy is released. EnergyTransaction::expenditure constructs a debit whose negative usable_delta exactly funds positive structural storage, work, and heat. ExternalInputTransaction keeps source availability/input separate from bond potential and requires nonnegative usable credit; the input is allocated into usable energy, work, and heat.

**Observed API guarantees**
- settle_transaction validates holder finiteness/non-negativity, transaction balance, and all resulting ledger totals before mutating holder or ledger. It records non-structural source release, structural delta, usable gains, work, and heat.
- settle_external_input validates its transaction and resulting counters before mutating; settle_external_input_from_budget checks source identity and available allowance before settling, then decrements the non-Copy/non-Clone budget only after success.
- transfer checks finite/nonnegative amount, source balance, and recipient overflow before updating both holders. It deliberately does not add world energy: an internal transfer cancels at world scale.
- settle_world_tone_paths verifies incident energy equals captured resonant energy plus heat plus escaped energy within a scale-relative tolerance, then records incident external input, heat, and escaped energy. The physical allocation and reservoir mutation remain the caller's responsibility and must be reconciled in A8/A6.
- EnergyInventorySnapshot::residual independently expresses world balance across usable, resonant, and structural energy plus work, heat, escaped tone, external input, and explicit other sources. Its presence is not proof the live simulation constructs and validates it each tick.

**Findings / risk ranking**
1. **P1 — prepared-ledger rollback is manually maintained:** prepare_transformation computes per-field ledger deltas and applies them individually; rollback_prepared_combine subtracts a manually enumerated subset. The delta structure includes total_world_tone_energy_escaped and total_resonant_energy_held, while the rollback helper does not reverse those fields. Current COMBINE is not expected to alter those counters, so this is a symmetry/maintenance risk rather than proof of a present COMBINE energy loss; the invariant should be encoded rather than assumed.
2. **P1 — inventory helper versus live authority:** EnergyInventorySnapshot is defined beside the ledger, but A6 must prove which production code populates it, at what cadence, and whether missing/duplicate holders or dead/developing stores are omitted.
3. **P2 — tolerance budget:** EPSILON is applied per transaction and external source checks also use a scale-relative tolerance. Many transactions can accumulate small accepted residuals; long-run error bounds need measured tests, not only single-transaction assertions.
4. **P2 — accounting and physical commit are split by callers:** the ledger validates arithmetic, but cannot itself know whether the caller later commits a bond, reservoir, body, or material mutation. Each caller must preserve trial/settle/commit ordering and rollback on every failure path.
5. **P2 — source scope is caller-defined:** EnergySourceBudget prevents accidental cloning of a shared allowance, but comments explicitly leave its physical scope to callers. A8 must verify the sun's total incident energy is uniquely allocated, not merely that each local transaction balances.

**Evidence inspected:** src/chemistry/energy_ledger.rs (EnergyAllocation, EnergyTransaction, ExternalInputTransaction, EnergySourceBudget, EnergyInventorySnapshot, EnergyLedgerAuthority implementation); src/simulation/state.rs (serialized cumulative ledger fields); src/chemistry/transformation.rs (prepared ledger delta and rollback); src/chemistry/combine_runtime.rs (COMBINE transaction caller).

- **Dependencies / downstream audits affected:** A3–A4, A6–A9, A12–A16, A18–A20. The local algebra is reusable, but no claim of whole-world conservation is authorized yet.
- **Decision required from project owner, if any:** none before remaining audits.
- **Recommended smallest next change (if authorized):** after the complete audit, replace hand-maintained ledger delta/rollback lists with one tested delta application/reversal abstraction, and make the live world-inventory assertion explicit. No production code change authorized now.

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

**Audit record — 2026-10-22**

**Status: Gap found — a conservation snapshot and local accounting tests exist, but whole-world conservation is not enforced in production.**

**Inventory map observed**
- Usable energy is held by live/dead-but-still-present organisms and decomposing carriers; Simulation::energy_inventory_snapshot takes cumulative usable totals from the ledger, not by independently walking and summing every holder.
- Retained resonance is stored in each organism's serialized ResonantEnergyReservoir; the ledger's current total is refreshed at the end of the simulation tick via total_resonant_energy_held.
- Structural energy is stored on realized graph bonds. Simulation::structural_energy_held sums live organism graph bonds, developmental structures, and legacy decomposing-body structures. Internal bonds inside PhysicalMaterial are not separately energy-valued.
- External input, escaped world-tone energy, non-structural source release, work, and heat are cumulative ledger counters. Internal transfers cancel and should not be counted as sources.
- Tick-zero baselines are initialized from the initial organism, and later current totals are refreshed near the end of tick processing. The snapshot's residual equation includes the three physical stores, work, heat, escaped tone, external input, and other explicitly released energy.

**Critical limitation:** Simulation::energy_inventory_snapshot() is explicitly marked as used by reconciliation tests, not production telemetry. No live per-tick balance assertion was found in the inspected simulation path. Tests can prove selected constructed scenarios, but do not establish conservation for arbitrary runtime histories, source allocation, holder creation/removal, or persistence recovery.

**Findings / risk ranking**
1. **P0 — no production world-conservation gate:** local transactions validate algebra, but the live simulation does not independently recompute the inventory and reject/log a non-balanced tick. Therefore global conservation is not yet proven.
2. **P1 — current inventory totals are ledger-maintained:** a missed refresh or holder omitted from the helper can make the snapshot internally consistent while the physical world differs. A6 needs an independent enumerator over every live/dead organism, developing structure, decomposing body, and any other energy holder, compared against the ledger totals.
3. **P1 — material/structural boundary:** graph bond energy is counted; PhysicalMaterial internal bonds have no independent energy store. This may be an intentional representation rule, but document it and prove no operation treats these bonds as previously funded structural energy.
4. **P1 — lifecycle coverage:** death is observational in the current simulation tick; breached organisms remain physical and continue ordinary chemistry. Decomposing bodies remain a legacy carrier included in totals. Verify transitions into/out of those carriers, offspring construction, holder deletion, and finalization don't duplicate or omit usable/resonant/structural energy.
5. **P2 — floating-point tolerance:** per-transaction 1e-9 balance checks and scale-relative source checks do not by themselves define a long-run residual budget. Test error growth as event count increases; distinguish per-transaction acceptance from whole-world tolerance.

**Evidence inspected:** src/chemistry/energy_ledger.rs (EnergyInventorySnapshot::residual, is_balanced, ledger source/flow counters); src/simulation/simulation.rs (tick-zero initialization, end-of-tick totals, structural_energy_held, snapshot marked test-only); src/simulation/state.rs (serialized ledger fields and holders); src/reproduction.rs (usable/resonant holder sums); handoff archive only as historical corroboration, not current authority.

- **Dependencies / downstream audits affected:** A4, A8–A16, A18–A20. No global-conservation claim should be made before these paths are independently reconciled.
- **Decision required from project owner, if any:** none to continue audits. Whether stored physical-material bonds carry structural energy is now governed by D3: every realized bond that carries stored energy has explicit ownership and exactly-once settlement. The remaining gap is to inventory which bond classes carry energy and verify those rules across persistence/rollback.
- **Recommended smallest next change (if authorized):** after all audits, implement an independent world inventory and a deterministic end-of-tick conservation diagnostic first in tests/debug mode, then make any strict runtime policy decision based on measured behavior. No production code change authorized now.

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

**Audit record — 2026-10-22**

**Status: Gap found — the reservoir is a real energy store with bounded component count, but its full lifecycle and numeric bounds are not yet proven.**

**Representation and rules observed**
- ToneSpectrum represents a currently received travelling signal; its ToneComponent.amplitude is converted to energy as amplitude squared. It is not itself a persistent store.
- ResonantEnergyReservoir is serialized on each organism and stores (frequency_hz, energy) components. Deposit rejects non-finite/invalid inputs, merges nearly equal frequencies by logarithmic ratio tolerance, sorts new components, and rejects a fifth distinct component (MAX_SPECTRAL_COMPONENTS = 4). The caller is explicitly responsible for accounting rejected input as heat.
- Total reservoir energy is summed and checked for finiteness. Reaction compatibility weights each component by an exponential function of logarithmic frequency distance; it changes the accessible amount, not the total stored energy. Withdrawal subtracts the actual amount withdrawn and returns that amount for the caller's ledger transaction.
- World-tone capture stages reservoir copies for the whole source frame, checks that each accepted deposit increases stored energy by the expected retained amount, and commits staged reservoirs only after the aggregate source-path ledger settlement succeeds. If the spectral capacity rejects a deposit, the material response is routed to heat rather than silently dropped.
- Resonance-overload handling compares retained energy with bond-energy handling capacity and uses the shared BREAK settlement path. Resonance is therefore both an energy store and a chemistry interaction input; it must never also be treated as a free energy source merely because it is frequency-compatible.

**Findings / risk ranking**
1. **P1 — component count bounded, energy magnitude not domain-bounded:** there can be at most four distinct components, but no explicit physical maximum for the energy accumulated at one frequency. Finiteness checks eventually reject overflow, but an extremely large finite store can still distort thresholds and arithmetic before overflow.
2. **P1 — lifecycle dissipation/retention policy unclear:** no general per-tick reservoir dissipation was identified in the reservoir API. The approved 10% per-tick dissipation applies to the chemistry reaction accumulator; do not assume it automatically applies to retained resonance. A7/A9 must establish whether resonant energy is intentionally persistent, how it interacts with general energy dissipation, and which paths convert energy to heat.
3. **P1 — withdrawal contract needs stronger tests:** compatibility is used both to estimate accessible energy and to calculate actual withdrawals. Prove the reported withdrawn amount, remaining component energy, reaction transaction debit, and failed-settlement rollback agree across multiple frequencies and tiny/huge values.
4. **P2 — epsilon pruning changes representation:** withdrawal removes components at or below f64::EPSILON. Quantify whether this drops a non-negligible world-scale total under large populations/repeated operations, and ensure it is treated as a bounded numerical sink or retained without silent loss.
5. **P2 — serialized malformed states:** deserialization can restore invalid/duplicate/over-capacity components unless a validation path is guaranteed before use. The API fails closed when total_energy() is invalid, but recovery behavior and diagnostics must be checked in A13/A18.
6. **P2 — spectrum normalization must conserve energy:** transformation helpers create/merge/prune received spectra. Verify each spectral simplification's output energy against its input budget; an amplitude/frequency rewrite is not itself a source or sink.

**Evidence inspected:** src/perception/harmonics.rs (ResonantEnergyReservoir, ToneSpectrum, MAX_SPECTRAL_COMPONENTS, transform and material-response helpers); src/chemistry/transformation.rs (resonance-overload and chemistry BREAK); src/simulation/world_tone_snapshot.rs (staged reservoir and heat fallback); src/simulation/state.rs (serialized organism reservoir).

- **Dependencies / downstream audits affected:** A2, A4–A6, A8–A10, A13–A16, A18, A20. A bounded component count is a good scalability foundation but not a substitute for bounded values and explicit lifecycle accounting.
- **Decision required from project owner, if any:** none before the remaining audits. Do not add arbitrary resonance caps or dissipation before proving the intended energy model.
- **Recommended smallest next change (if authorized):** after all audits, add invariant/property tests for deposit, compatible withdrawal, serialization validation, and energy-preserving spectral transforms; choose explicit caps or dissipation only if the complete physical model requires them. No production code change authorized now.

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
- The audited live capture-settlement path is present; do not describe it as disabled. Treat capture as unvalidated until allocation, duplicate prevention, and conservation gates pass, and do not alter runtime behavior in this planning PR.

**Deliverable**
- Source-to-receiver flow model, complexity estimate for thousands of objects, deterministic reference tests, occlusion/energy-partition tests, and explicit enablement gate.

**Audit record — 2026-10-22**

**Status: Gap found — capture is active in the current tick path, and source allocation is transactionally staged, but this contradicts the README's disabled-capture claim and needs scalability proof.**

**Observed source and geometry path**
- WorldToneRuntimeConfig defines a single 440 Hz source, intensity 1–10 energy per tick per exposed projected width, a 2,400-tick orbit/day cycle, and an off-world radius validated against the world diagonal. WorldToneCycle precomputes source samples up to 65,536 ticks; the current 2,400-tick cycle fits. Samples describe source output, not receiver allocation.
- Simulation passes the current world-tone sample to WorldToneOccluderSnapshot::settle_living_organism_inputs during its tick. The snapshot assembles realized geometry from organisms, developing offspring, environmental physical materials, decomposing structures, stored physical materials, and in-flight transformation material. Logical stock without physical placement is not an occluder.
- The snapshot builds one world-wide exposure frame, checks that first-hit path widths reproduce direct geometric exposure widths, stages all organism resonance reservoirs, propagates a remaining signal through the ordered hit list, and partitions each path's incident energy into captured energy, heat, and escaped signal. It calls settle_world_tone_paths once for aggregate incident/captured/heat/escaped totals, then commits the staged reservoirs.
- The propagation contract is onward continuation with transformed frequency composition, not literal mirror rays. The frame's direct exposure solver is based on finite-source geometry and visibility/occlusion, with a broad phase using body angular intervals and source distances. src/geometry/source_exposure.rs also contains exact projected-extent helpers and extensive shape/visibility logic.

**Findings / risk ranking**
1. **P0 — README/runtime discrepancy:** README says live capture is disabled, but current simulation calls the live settlement path and the snapshot mutates the ledger and commits retained energy. The real current behavior must be documented accurately; do not rely on old handoff text or README alone.
2. **P1 — scalability risk remains unmeasured:** each tick constructs body geometry and a world-wide exposure frame; the visibility solver iterates bodies and parts and evaluates possible occluders for target geometry. This may be materially more expensive than a simple one-sweep/first-hit approximation at thousands of bodies. No measured large-population runtime or scaling curve was found in this audit. Profile before assuming it meets the project's performance target.
3. **P1 — source allocation is per geometric lane, not yet independently proven as world-total physical exposure:** every path is balanced and aggregate ledger settlement is atomic, but the model must establish whether the sum of all lanes is the intended finite source budget for a point source. Per-width intensity multiplied by each path's projected width can produce a world total that grows with total exposed boundary extent; if that is intended, define the physical boundary/source budget clearly. Do not mistake local path conservation for a finite global solar power limit.
4. **P1 — receiver subset parameter versus actual accounting:** from_simulation_for_organisms accepts a receiver subset, but the frame intentionally accounts every realized body as a heat receiver, even when no living organism lies behind it. Verify all bodies and all paths are included once, including overlap, multi-part bodies, fluid boundaries, and off-world/invalid geometry.
5. **P1 — reservoir failure fallback is heat:** when a living organism cannot represent the retained spectrum within its four-component reservoir, the response energy becomes heat and onward signal only carries the unresponded remainder. This is conservative locally, but it can make capture efficiency depend on spectrum capacity; that should be observable and tested.
6. **P2 — whole-frame abort policy:** invalid geometry or any inconsistent path returns without settling the entire source frame. This avoids partial credits but means one malformed unrelated physical object can suppress sunlight capture for every organism that tick. Need bounded diagnostics and a clear repair/failure policy.
7. **P2 — source sample precomputation does not precompute exposure:** source coordinates/intensity are cached, but geometry/occlusion and receiver paths are rebuilt during the live tick. This is a likely hot path; cache invalidation must follow geometry/position/material revisions if later optimized.
8. **P2 — approximation validation gap (decision settled later by D5):** the owner selected a fast first-hit/single-sweep sunlight approximation, conditional on comparison with an exact small-world reference. Compare energy allocation, occlusion/shadow error, and performance; do not reopen the high-level approximation choice.

**Evidence inspected:** src/simulation/world_tone.rs (source parameters, cycle precompute, incident energy); src/simulation/world_tone_snapshot.rs (world geometry snapshot, first-hit checks, staged capture and final settlement); src/geometry/source_exposure.rs (visibility and exposed interval computation); src/simulation/simulation.rs (live tick invocation and energy totals); src/perception/harmonics.rs (material response/reservoir).

- **Dependencies / downstream audits affected:** A5–A7, A9, A13–A15, A17–A20. A8 cannot be considered enabled-for-production-safe until the README discrepancy, finite source budget, whole-world inventory, and measured scalability are resolved.
- **Decision required from project owner, if any:** no immediate decision needed to continue audits. The existing active behavior should not be silently disabled or retuned in this planning-only PR; a later implementation decision should follow the completed audit.
- **Recommended smallest next change (if authorized):** after all audits, benchmark exact capture against the target population and a radar/first-hit approximation, record shadow and energy error, and reconcile source energy against a global budget. Until that is complete, treat live capture as active but not yet proven scalable or globally conserved. No production code change authorized now.

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

**Audit record — 2026-10-22**

**Status: Gap found — many local sinks are explicit, but heat, work, and stress are not yet a single fully specified physical contract.**

| Path | Current accounting | Audit implication |
|---|---|---|
| COMBINE | Usable-energy expenditure is split among structural bond energy, formation work, and heat/investment; work is derived from force over remaining approach distance. | Keep force, energy, work, and stored bond potential distinct; prove prior movement work is not charged again. |
| BREAK | Existing bond energy is removed from structural inventory and allocated to usable energy, work/disruption, and heat according to the shared settlement math. | Prove the threshold and final settlement draw on compatible named stores exactly once. |
| Movement | Each successful one-unit step debits usable energy and records the same amount as heat; work_expended is zero. | Current model treats locomotion cost as fully dissipated heat, not useful mechanical work. This is an explicit modeling choice but needs documentation/decision, and the physical move currently occurs before energy settlement, so an unexpected settlement failure may leave movement committed without the debit. |
| Maintenance | Paid maintenance debits usable energy and records the payment as heat, then adds stress from that payment. Unpaid demand adds maintenance debt and stress without an energy debit. | Heat and stress are separate effects of the paid cost; debt is a biological pressure, not a physical energy holder. Confirm no caller double-counts stress as energy. |
| World tone | Incident energy is partitioned into retained resonance, heat, and escaped signal; material response at nonliving/dead physical receivers becomes heat. | Local path accounting is explicit, but A6 still must prove global source budget and live inventory balance. |
| Decomposition finalization | Remaining usable budget is explicitly debited and recorded as heat before removing a finished carrier. | This is a named sink in the test-only finalization helper, not proof that ordinary living bodies are automatically decomposed in production. |
| Reaction accumulation | A fixed 10% of prior reaction accumulator is removed each tick. | The accumulator is a trigger-like chemistry quantity, not a physical energy reservoir; do not also book this mathematical decay as world heat unless the model explicitly defines it as energy. |

**Findings / risk ranking**
1. **P1 — energy-to-heat versus useful work semantics:** movement is fully counted as heat despite changing position; whether that is intentional should be decided at the model level, not fixed by relabeling counters. Formation work is tracked separately. The ledger's work counter should mean actual energy expended doing work under the chosen convention.
2. **P1 — move/settle transaction is not fully atomic:** physical movement commits before the ledger debits usable energy. Inputs normally prevalidate cost/available energy, but a later counter overflow or settlement rejection could leave position changed without the corresponding debit. A15 must trace rollback and same-tick state.
3. **P1 — heat-to-stress coupling:** paid maintenance and transaction heat can also increase biological stress. That may be a legitimate second consequence of the same dissipation, but stress must not be counted as a second energy sink or confused with stored heat.
4. **P1 — world tone is the main live external input:** its geometry work is rebuilt per tick and its local path partition is balanced, but source output and whole-world budget remain unproven (A6/A8/A19).
5. **P2 — no unified heat reservoir:** heat is recorded as a cumulative sink, not a modeled environmental temperature/energy field. That is acceptable only if the environment is explicitly an open boundary that receives irrecoverable heat; otherwise heat cannot participate in later chemistry.
6. **P2 — dissipation terminology spans different mechanisms:** reaction-accumulator decay, harmonic damping, movement cost, maintenance cost, bond-break heat, and sunlight material response should have separate named quantities/rules with a clear common energy accounting convention.

**Evidence inspected:** src/chemistry/combine.rs and combine_runtime.rs; src/chemistry/transformation.rs; src/perception/movement.rs; src/simulation/state.rs (apply_maintenance); src/simulation/world_tone_snapshot.rs; src/chemistry/chemistry.rs; src/chemistry/energy_ledger.rs; src/simulation/simulation.rs.

- **Dependencies / downstream audits affected:** A2–A8, A10, A12, A15–A20. A9's semantic decisions depend on the full source/sink and consumer inventory.
- **Decision required from project owner, if any:** no immediate decision; retain the current convention in this audit and list any semantic changes for a later explicit approval.
- **Recommended smallest next change (if authorized):** after all audits, document canonical energy/work/heat semantics and add failure-injection tests around movement commit/debit. Do not alter movement or heat behavior before the complete audit. No production code change authorized now.

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

**Audit record — 2026-10-22**

**Status: Gap found — death is currently a derived biological eligibility state, while physical persistence and ordinary chemistry continue.**

**Observed lifecycle**
- The simulation determines living IDs from the genome-core/cavity analysis. A breached organism remains in the organism collection and remains part of the physical world; the simulation comments explicitly say death is observational and that ordinary chemistry continues after breach.
- The tick's biological decision/movement/construction/survival-damage loops are gated by the living-ID set. Structural damage is not an independent random stress-break path: stress decay and resonance-overload resolution route structural change through the shared chemistry/BREAK mechanism.
- Maintenance payment is applied as part of organism state logic; A12 must verify it is only applied to eligible living organisms and cannot continue biologically after breach. Debt/stress accumulation can cause an eventual core breach but is not itself a bond-break authority.
- Invalid geometry-analysis results are not automatically treated as death in the survival-damage path; this is an important fail-closed distinction between dead and analysis failed.
- Existing in-flight transformations are intentionally retained after breach, because dropping them could discard reserved material/energy. They are allowed to settle; new biological transformations should not start from breached organisms.

**State transition table**

| Starting state | Event | Physical body | Biological processing | Chemistry |
|---|---|---|---|---|
| Intact core | Ordinary tick | Retained | Eligible if other rules permit | Active |
| Intact core | Chemical BREAK breaches core | Retained | Eligibility is recalculated according to tick-order rules; same-tick edge cases require tests | Continues |
| Intact core | Unsustainable maintenance/stress causes chemical BREAK | Retained | Stops after breach is recognized | Continues |
| Breached core | Later tick | Retained | New biological actions gated off | Continues |
| Breached core with in-flight transaction | Pending BREAK/COMBINE resolves | Retained | No new decision required for the pending operation | Must settle atomically |
| Geometry analysis failure | Unknown | Retained | Must not be declared dead solely from analysis error | Fail closed / diagnostic required |

**Findings / risk ranking**
1. **P1 — same-tick death boundary:** living eligibility is determined at a point in tick processing; a later reaction can breach a core after that set is formed. Verify whether biological actions can still occur later that tick, and define the precise transition point.
2. **P1 — maintenance invocation and lifecycle eligibility:** the state method itself is not a death gate; prove the caller gates it. Debt/stress accumulation can cause an eventual core breach but is not a bond-break authority.
3. **P1 — pending operations after death:** retaining in-flight transactions is preferable to silently dropping reservations, but test pending source removal, failed resolve, stale structure revision, and save/load across breach.
4. **P2 — physical body vs legacy decomposition:** the approved rule is that death does not automatically disassemble the body. Do not activate recycling as a special death chemistry process. The legacy DecomposingBody subsystem must remain explicitly separate unless a future design decision changes the death model.
5. **P2 — cavity predicate is a proxy for genome-core integrity:** ensure the approved core-breach predicate is stable and not conflated with general organism viability or maintenance debt alone.

**Evidence inspected:** src/simulation/simulation.rs (living-ID gating, apply_survival_damage, pending transformations); src/simulation/state.rs (maintenance/stress); src/chemistry/transformation.rs (resonance-overload BREAK); src/cavity.rs and core analysis call paths; current README and approved death rule.

- **Dependencies / downstream audits affected:** A4, A9, A11–A16, A18, A20. Exact tick ordering and maintenance callers remain open for A12/A15.
- **Decision required from project owner, if any:** none to continue. Keep death observational and do not introduce a new decomposition law.
- **Recommended smallest next change (if authorized):** after all audits, add same-tick core-breach and pending-operation tests that prove physical persistence and biological gating. No production code change authorized now.

### A11 — Post-death chemistry, decomposition/recycling, and material conservation

**Final call-path closure — Section 16:** ordinary `Simulation::step` applies the same `chemical_reaction::accumulate` / `resolve` pass to breached organisms; the legacy `process_decomposing_bodies` method is `#[cfg(test)]`, and repository search found no production caller of `recycle_dead_organism`. Do not treat the legacy path as the live death pathway.

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

**Audit record — 2026-10-22**

**Status: Gap found — the approved live death model is observational, while a legacy decomposition carrier exists behind test-only orchestration.**

**Observed current behavior**
- Ordinary simulation comments state that breached organisms remain physical and continue the ordinary chemical-reaction pass. This matches the approved model: death is not a separate chemistry process.
- A legacy DecomposingBody type can hold a structure and usable-energy budget. resolve_one_bond_with_ledger breaks its first bond, removes that bond's stored structural energy, and allocates usable credit/heat through the ledger. When the final bond is gone, material is released; process_decomposing_bodies settles any remaining usable budget as heat before removing the carrier.
- Critical reachability distinction: process_decomposing_bodies is compiled with cfg(test), so this staged carrier orchestration is not part of the ordinary production tick. recycle_dead_organism exists in src/chemistry/recycling.rs, but repository search found no production caller; its tests exercise the helper directly. Thus it must not be described as the live death pathway.
- The legacy decomposition energy accounting is internally understandable for its carrier, but it is a second bond-breaking orchestration and should not become a second law of chemistry. Current live bodies instead continue through ordinary reaction/BREAK code.
- The legacy helper releases realized constituent material only when no graph bonds remain. Physical-material internal connections have no independently allocated bond_energy, so they must not receive the realized-graph bond yield unless they were explicitly funded in that representation.

**Findings / risk ranking**
1. **P0 — production reachability/documentation must be exact:** recycling is not a demonstrated live path and decomposition orchestration is test-only. Preserve this distinction in README and implementation planning; do not wire it into the live death flow as part of unification.
2. **P1 — duplicate pathway risk:** if legacy decomposition were activated, it could compete with ordinary post-death chemistry over the same conceptual body. Any future lifecycle owner must be singular and explicit.
3. **P1 — holder/material transfer atomicity:** recycle_dead_organism drains stored material and removes reproductive construction before all subsequent energy transfers are complete. If a later transfer unexpectedly fails, the function returns None after some state has already moved. The prevalidation reduces risk but does not make all mutations atomic under ledger-counter overflow or future failure modes.
4. **P1 — energy sink at finalization:** the test-only finalizer sends any remaining carrier budget to heat because no authorized receiving path exists. That is a specific open-boundary decision, not an automatic rule that should be applied to every dead organism.
5. **P2 — material deposit return behavior:** recycling deposits physical materials into the field before it has completed all energy transfers; field deposit failure/rollback and material conservation need adversarial tests.
6. **P2 — test-only path can mislead audits:** phase/authority tests prove the helper's behavior, not that production runs it. CI results and test coverage must distinguish reachable production behavior from helper-only contracts.

**Evidence inspected:** src/chemistry/decomposition.rs; src/chemistry/recycling.rs; src/simulation/simulation.rs (cfg(test) process_decomposing_bodies and ordinary tick comments); src/tests/phase3_authority_contract_tests.rs; repository searches for recycling and decomposition callers.

- **Dependencies / downstream audits affected:** A4–A6, A9–A10, A12–A16, A18, A20. This audit corrects A4's provisional phrasing: the legacy decomposition helper is called by the test-only process function, not demonstrated as the normal production death pathway.
- **Decision required from project owner, if any:** none now. Keep ordinary chemistry as the live post-death mechanism; don't enable legacy recycling/decomposition without explicit architectural approval.
- **Recommended smallest next change (if authorized):** after all audits, test and either harden or clearly quarantine the recycling helper; keep it disconnected from production death processing unless the unified lifecycle plan explicitly chooses it. No production code change authorized now.

### A12–A21 — Remaining audit domains

The completed audit records for these domains appear in the audit log below. The original question lists are consolidated here to avoid duplicating headings and leaving apparently empty templates.

- **A12:** Energy consumers, internal transfers, initial conditions, and construction.
- **A13:** Persistence, schema evolution, concurrency, and recovery.
- **A14:** Chemistry cache authority, key semantics, fallback, and performance.
- **A15:** Tick ordering, scheduling, determinism, and stale operations.
- **A16:** Whole-world material conservation and physical representation.
- **A17:** Diagnostics, observability, UI, and scientific interpretability.
- **A18:** Numerical stability, invalid states, and long-run behavior.
- **A19:** Scalability and profiling at target populations.
- **A20:** Cross-domain invariants and integration tests.
- **A21:** Documentation/spec alignment and implementation sequencing.

## 7. Cross-cutting impact matrix

Owner/caller/test/status findings are summarized below and in the A0–A21 audit log. Section 16 closes the final cross-cutting source-path checks; any item marked Gap found remains an implementation/evidence gap, not an unfilled owner decision.

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

## 8. Earlier implementation sequence — superseded by the formal plan in Section 17

**Historical outline only:** retain the original sequence below as audit provenance. Section 17 is the authoritative plan and adds the concrete source-path findings from Section 16, the explicit cache-version boundary, the fractional-aggregate conflict with D8, independent integer constituent accounting, and evidence-based sunlight/performance gates.

This is a dependency order, not permission to begin all changes immediately.

1. **Establish baseline and inventory (A0).**
2. **Lock vocabulary, units, and equations (A1–A2).** Do not retune constants while dimensional meaning is ambiguous.
3. **Unify transaction semantics and bond accounting (A3–A5).** Establish the shared physical/energy commit primitive for COMBINE and BREAK.
4. **Build an independent world inventory (A6, A16, A18).** Prove the audit equations against isolated and integration scenarios.
5. **Unify resonance/reaction coupling (A7) and stress/death/post-death behavior (A10–A11).**
6. **Audit every existing energy consumer and transfer (A9, A12), then event ordering (A15).**
7. **Repair/verify persistence and cache semantics (A13–A14).**
8. **Validate the existing world-tone capture path (A8) against the energy inventory (A6).** The audited source contains a live capture/settlement path. Verify source budget, duplicate claims, occlusion, and performance against D5's fast first-hit/single-sweep direction and an exact small-world reference. Do not describe capture as disabled or change runtime behavior in this documentation-only PR.
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
- [ ] World-tone exposure, occlusion, source allocation, propagation, and resonance capture pass conservation and duplicate-claim tests before capture behavior is considered validated and approved for continued use.
- [ ] Performance is measured against declared target scenarios, including thousands of objects; no accidental quadratic or per-object ray-tracing regression.
- [ ] Exact-head formatting, compile, focused tests, source-size/architecture checks, baseline comparison, and bounded integration tests are reported accurately.
- [ ] README and this plan distinguish verified behavior, known baseline failures, open gaps, and unproven aspirations.

## 10. Audit log

| Audit | Status | Current evidence / output | Open blockers |
|---|---|---|---|
| A0 Baseline and inventory | Gap found; comparison evidence recorded | Frozen main run 39705628427: 563 passed, 28 failed, 1 ignored. PR #205 run 39833438679: 585 passed, 27 failed, 1 ignored; 27 identities match baseline identities minus one still-resolved failure. Normalized Clippy identities match baseline. | Suite remains baseline-red; re-fetch head and CI before any implementation. |
| A1 Resource vocabulary/properties | Owner identity settled; active identity confirmed | Oxygen is canonical for ID 6; unknown-resource queries fail closed; valid composites intentionally have no averaged chemical position. Historical archives retain Methane references intentionally. | Add invalid-catalog tests and finish cache fingerprint coverage for chemical-position changes. |
| A2 Core chemistry math/units | Gap found | Interaction, contact, activation, accumulation, and energy conversion equations/constants are located and fingerprinted. | Finish unit/range contract, adversarial overflow, and long-run accumulation tests. |
| A3 COMBINE and bond storage | Partial | Bond formation stages a trial graph and settles energy before committing; fluid-pair special handling is present. | Integrated storage/source rollback, fluid topology, and candidate-cost/performance tests. |
| A4 BREAK paths | Partial; failure gaps confirmed | Structural BREAK settles a negative structural-store delta and commits graph/resonance only after validation. Stored physical BREAK changes topology with zero energy because no bond energy was allocated. | Stored BREAK piece commits, malformed completion, stale/retry and destination-failure rollback. |
| A5 Ledger transaction algebra | Partial | Local transaction balance and internal two-holder transfer semantics are verified. Prepared COMBINE delta fields omit escaped-tone/resonant-held updates. | Assert forbidden deltas stay zero or centralize all-field delta application/rollback; test rejected transactions across every holder. |
| A6 Whole-world energy inventory | Gap found | Test-only snapshot independently recomputes current usable, resonant, and graph-bond stores; material oracle covers active escrow once. | Historical source flows still rely on ledger; cross-domain full-tick and failure-injection reconciliation remains open. |
| A7 Resonance and reaction coupling | Partial | Chemical and overload BREAK use cloned reservoir withdrawals committed only on success; Sections 17.60–17.61 trace resonance property callers. | Fail-closed malformed mass/cohesion/reactivity/baseline validation, reservoir capacity/pruning, serialization, and broader inventory tests. |
| A8 World tone and capture | Partial; validation open | Live frame balances incident energy as retained resonance + heat + escaped energy and stages reservoir commit. | Receiver eligibility clarification, independent exact-small-world exposure/first-hit comparison, approximation error and performance evidence. |
| A9 Work, heat, dissipation | Gap found | Maintenance settles before stress mutation; movement cost currently records heat and zero explicit work. | Movement commits geometry before ledger settlement; reconcile work/heat meaning with D7 and add rejection tests. |
| A10 Stress/death/core breach | Partial | Tick phases are mapped; dead structures persist and structural chemistry continues. | Test breach before versus after world-tone, stress-phase death, and pending-operation behavior. |
| A11 Post-death chemistry/material fate | Gap found | Ordinary primary-graph chemistry continues after breach; no separate decomposition law is live-integrated. | Stored/developing holders remain outside reaction enumeration; legacy recycler and all material return failures remain unsafe. |
| A12 Energy consumers/transfers | Gap found | Initial reserve uses one InitialCondition input transaction. Internal transfer primitive is atomic. | Construction reserve caller ignores transfer rejection; source/destination return and whole-step construction inventory tests. |
| A13 Persistence/schema/recovery | Partial | Chemistry cache schema 4, fingerprint/manifest validation, duplicate-key rejection, and append/manifest rollback/reopen tests exist. No durable whole-simulation save/load path was found. | Catalog fingerprint enforcement, restart/corruption coverage, and malformed pending-operation recovery. |
| A14 Chemistry cache semantics | Partial; gate open | ChemistryKey has schema/material/interface identity; static potential is computed from chemical positions and K/Dmax. | Catalog-position invalidation depends on manually bumping the manifest fingerprint; test old-cache rejection after a property change. |
| A15 Tick ordering/determinism | Partial | Active transformation, world-tone, biological, chemistry, construction, and stress phases are documented in Section 17.41. | Failure injection for stale/malformed operations, deterministic retries, and source escrow through every terminal path. |
| A16 Material conservation | Gap found | Independent integer material oracle covers field, organism holders, committed/developing construction, and active escrow. | Stored BREAK, containment acquisition, expulsion and malformed-operation destination rejection remain incompletely atomic. |
| A17 Diagnostics/observability | Gap found | Section 17.59 confirms opt-in diagnostic JSON hard-codes seed 42 and labels only usable energy as generic `energy`. | Pass actual run configuration and clearly label usable/resonant/structural quantities; keep diagnostics separate from the inventory oracle. |
| A18 Numerical stability | Gap found | Bondless reaction accumulation can retain stale keys; resonance helpers can mask NaN properties with epsilon floors; accumulate_reaction can return nonfinite output for out-of-domain finite inputs. | Cleanup/range regression, fail-closed invalid-property tests, long-run drift, reservoir pruning tolerance, and cumulative counter overflow tests. |
| A19 Scalability/profiling | Not verified | Fast first-hit/single-sweep direction is preserved; source geometry is not per-object ray traced. | No representative runtime/memory benchmark or exact-small-world approximation error measurement has been run. |
| A20 Cross-domain integration | Gap found | Source-to-holder-to-sink matrix and energy API caller map are in Sections 17.55–17.56. | Focused failure-injection tests and independent material/energy oracle checks for each gap row. |
| A21 Documentation/sequence | Current tracker/handoff reconciled; README mismatch confirmed | Section 17.57 confirms README says live capture is disabled while Simulation::step calls the live frame path; its verification checkpoint is also historical. | After audit closure and owner approval, correct current-status wording and links while preserving historical provenance; do not equate live code with validated conservation. |


## 11. Out of scope unless a direct dependency is proven

- Stabilizing or merging PR #196; constructor/Bob changes remain deferred until the chemistry model is unified.
- Reproduction redesign or movement redesign.
- Unrelated lint cleanup, repo-wide reorganization, broad formatting churn, or speculative API redesign.
- Changing resource shapes or geometry tolerances merely to satisfy chemistry tests.
- Changing the live world-tone capture path as part of this documentation-only PR; its correctness remains an audit/verification gap, not an unresolved request to enable or disable it.
- Claiming full organism viability from cavity qualification or a bounded constructor search result.

## Historical decision log — provisional entries superseded by D1–D15

Record decisions here with date, rationale, affected audits, and any superseded rule. If a design choice needs the project owner's decision, leave it open instead of inventing a constant or silently choosing behavior.

| Date | Decision | Rationale / affected audits |
|---|---|---|
| 2026-10-22 | Chemistry unification proceeds on `main`; PR #196 stays open and draft-only. | Avoid stabilizing constructor behavior against unsettled chemistry; A0/A3/A6/A12 boundary. |
| 2026-10-22 | The audited source contains a live world-tone capture/settlement path; complete conservation and approximation accuracy remain unverified. Do not describe it as disabled or change its runtime behavior in this PR. | A6/A8 and Section 15 source reconciliation. |


### A12 — All energy consumers, transfers, initialization, and construction

**Audit record — 2026-10-22**

**Status: Gap found.**

- **Exact baseline and files inspected:** `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5`; `src/organism/reproduction.rs` (begin/advance construction and energy totals); `src/construction/initial_organism_constructor.rs`; `src/organism/juvenile.rs`; `src/simulation/state.rs`; `src/chemistry/energy_ledger.rs`; `src/simulation/simulation.rs`; `src/organism/genome.rs`; search evidence for initial-creation and reproduction call sites.
- **Authoritative rule or equation found:** the ledger transaction algebra requires source energy to equal usable-energy change + persistent structural-energy change + work + heat; a transfer between internal holders should preserve the world total. Reproduction uses a parent-to-developing-offspring transfer; juvenile construction computes a construction requirement using a large trial-energy allowance, then realizes the same blueprint with that computed requirement plus a reserve.
- **Actual runtime call path:** initial organism construction flows through the fixed initial constructor / simulation creation path. Reproduction begins by removing one physical anchor from parent storage and moving it into a separate developing graph; subsequent construction attempts call the bond-driven physical-material attachment path. Each advance transfers energy from parent usable energy into `ReproductiveConstruction.developing_energy`, applies child maintenance, updates stress/energy state, and continues construction. The child's energy is included in `total_usable_energy_held` alongside parent organism energy.
- **State mutated and transaction boundary:** anchor transfer is staged through cloned parent storage before replacing parent storage and adding construction state, but the ignored ledger argument in `begin_reproduction` does not itself record the material transfer. Developing energy is moved by `ledger.transfer`; construction attachment prepares candidate structure/ledger/energy and returns a candidate only on successful attachment. Initial energy is created as an initialization condition rather than a runtime external-input event.
- **Energy/material source and sink:** the parent is the energy source for the offspring's developing reserve; parent storage is the physical anchor source. The initial organism receives a calculated energy amount sufficient for the scaffold plus inherited reproductive allocation. Construction costs flow through the bond-driven COMBINE path into bond storage/work/heat according to its ledger. This establishes a deliberate initial-condition budget, but it is not currently proven to be reconciled as an explicit world-boundary source in the same way as world tone.
- **Failure/rollback behavior:** candidate child attachment is cloned/prepared before returning the candidate, and the parent anchor is selected/removed using a trial storage before parent state replacement. However, `begin_reproduction` does not emit an energy-ledger transaction for anchor transfer, and `advance_construction` ignores the return from `ledger.transfer`; exact failure behavior if transfer validation rejects an otherwise positive finite amount is not surfaced. Initial constructor relies on a large trial budget to infer cost, then reruns construction, which requires deterministic equivalence between trial and actual realization.
- **Numerical edge cases and invalid-state behavior:** reproduction validates inherited allocation as finite and positive. Juvenile construction rejects nonfinite/nonpositive reserves and invalid/negative computed construction requirements. A transfer is bounded by the reserve shortfall and `parent_energy.max(0.0)`; however, this masks negative parent energy as zero rather than proving the stored state is valid. The initial constructor's `CONSTRUCTION_ENERGY = 1.0e12` and juvenile `TRIAL_ENERGY = 1.0e6` are algorithmic allowances, not physical sources; their roles and deterministic cost assumptions must be made explicit.
- **Complexity and scaling implications:** construction candidate ranking and attachment can test multiple available materials and placements, while initial construction and reproduction use geometry validation. The audit did not run performance tests; avoid adding whole-world clones or scans to settle these paths.
- **Tests already covering it:** source contains reproduction, juvenile realization, and bond-driven attachment tests. Exact-head `main` full-suite result is recorded in A0 (563 passed, 28 failed, 1 ignored); the existence of these tests does not prove world-scale accounting.
- **Focused tests to add or run:** (1) parent-to-child energy transfer conserves combined usable + resonant + structural inventory; (2) rejected transfer leaves both holders unchanged and reports failure; (3) reproduction anchor removal is atomic with child construction creation; (4) initialization budget is represented explicitly in baseline/source accounting; (5) trial and actual construction costs are deterministic for identical blueprint/catalog; (6) invalid negative/nonfinite parent and child energy fails closed.
- **Findings ranked by severity:** **P1** — initial energy is an implicit initial condition and is not demonstrated as a source in the common global conservation equation. **P1** — ignored transfer result means the caller cannot distinguish a rejected transfer from a successful no-op. **P2** — reproduction's physical anchor transfer has no corresponding ledger event, though material identity is transferred rather than duplicated. **P2** — trial-energy constants are large calculation allowances and must never be mistaken for energy physically injected into the world.
- **Dependencies / downstream audits affected:** A5–A6, A9–A11, A13, A15, A18, A20–A21; constructor work in PR #196 remains explicitly deferred and is not authorized by these findings.
- **Decision required from project owner, if any (historical wording):** D2 now selects an explicit one-time external-source transaction for initial energy. Implementation and exactly-once replay/restore verification remain open.
- **Recommended smallest next change (if authorized):** after all audits, make internal transfer success explicit at call sites and define the simulation's initial-energy boundary in the global inventory. No production change authorized now.

### A13 — Persistence, schema evolution, concurrency, and recovery

**Audit record — 2026-10-22**

**Status: Gap found (state compatibility mechanisms identified; full save/load recovery path not proven).**

- **Exact baseline and files/lines inspected:** `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5`; `src/simulation/state.rs` (serialized state, organism and reproductive construction fields, energy ledger); `src/organism/genome.rs` (reproductive-energy field compatibility); `src/simulation/simulation.rs` (energy inventory test hook); `src/chemistry/energy_ledger.rs`; `src/simulation/sim_process.rs` and search results for persistence/snapshot entry points.
- **Authoritative rule or equation found:** persisted authoritative physical state must survive a round trip without changing material, energy, bond, or pending-transaction ownership. Derived caches may be skipped and rebuilt; backward compatibility requires explicit defaults/aliases or a versioned migration where a missing field cannot be safely inferred.
- **Actual runtime call path:** many state types derive Serde serialization/deserialization. Organism fields include persistent structure, stored material, usable energy, resonance reservoir, active transformation/movement, and optional reproductive construction. Cache revisions/results and the harmonic cache key are skipped on serialization; genome's `reproductive_energy_allocation` has a default and the old `juvenile_energy_reserve` alias. The exact authoritative disk-save/load caller chain and schema-version/migration boundary were not proven by the retrieved excerpts, so this audit does not claim a complete persistence workflow.
- **State mutated and transaction boundary:** deserialization reconstructs authoritative state while skipped caches restart empty and should recompute on demand. Pending transformation state persists prepared structure, storage, usable energy, stress, ledger, environmental source and prepared energy. These fields represent in-flight transactions and must be restored as a coherent unit, not independently discarded.
- **Energy/material source and sink:** serialized holders preserve energy/material rather than creating or consuming it. Defaulted fields may, however, initialize absent historical data to zero or a default reservoir, which can alter inventory if the old format represented that value elsewhere or if the field was added without a migration rule. The new world inventory fields include initial/current totals and cumulative flows with Serde defaults, but defaults alone do not reconstruct a correct historical baseline.
- **Failure/rollback behavior:** Serde can reject incompatible required fields/types; defaults and aliases cover selected known changes. No evidence retrieved establishes a complete atomic snapshot write, checksum/manifest validation for simulation state, interrupted-write recovery, or migration tests covering pending energy/material transactions. Do not conflate Bob/chemistry-library persistence formats with simulation snapshot persistence.
- **Numerical edge cases and invalid-state behavior:** skipped caches avoid persisting stale derived values; persistent energy fields still require finite/nonnegative validation after load. Serde defaults such as zero for newly added cumulative counters can be structurally valid but historically wrong. A loaded active transaction whose prepared energy/ledger disagrees with prepared structure or stored material could double-spend or strand energy unless cross-field invariants are checked.
- **Complexity and scaling implications:** skipping derived caches avoids persisting redundant cavity/development/harmonic results. Full-state snapshot size and write frequency were not measured in this audit. Prefer one validated snapshot boundary over serializing caches or writing per-event full-world state.
- **Tests already covering it:** source has Serde-derived state types, selected default/alias annotations, and simulation inventory reconciliation tests. No current-head round-trip/migration suite result was verified in this audit.
- **Focused tests to add or run:** (1) round-trip each energy holder and material holder; (2) round-trip active COMBINE/BREAK and reproductive construction at each prepared phase; (3) old snapshots missing every newly defaulted field; (4) reject NaN/infinity/negative energy after load; (5) interrupted/corrupt snapshot recovery; (6) verify skipped caches recompute and produce identical authoritative outcomes; (7) verify cumulative totals and tick-zero baselines are migrated rather than silently defaulted.
- **Findings ranked by severity:** **P1** — the full simulation persistence/migration/recovery boundary remains unverified. **P1** — Serde defaults for cumulative ledger counters can produce incomplete historical accounting if loading old snapshots without a version-aware baseline migration. **P2** — pending operations persist many coupled prepared fields and need cross-field consistency validation. **P2** — skipped cache invalidation appears deliberate and appropriate, but cache rebuild behavior should be covered by round-trip tests.
- **Dependencies / downstream audits affected:** A5–A6, A10–A12, A14, A18–A21. Any resource rename or schema migration from A1 must include saved material, genome, and library-key compatibility.
- **Decision required from project owner, if any (historical wording):** D6 now selects rejection of all pre-unification saves and caches, with actionable errors. Implement and verify the mandatory version boundary and rejection behavior.
- **Recommended smallest next change (if authorized):** after the full audit, inventory the real save/load entry point and add versioned round-trip/migration tests for authoritative energy/material and pending transaction state. No production change authorized now.

### A14 — Chemistry cache authority, keys, fallback, and performance

**Audit record — 2026-10-22**

**Status: Gap found.**

- **Exact baseline and files/lines inspected:** `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5`; `src/chemistry/chemical_reaction.rs` (material identity and live lookup/fallback call); `src/chemistry/chemistry_library.rs` (key, record, persistence, get-or-insert); `src/chemistry/chemistry.rs` (authoritative static-potential calculation); `src/chemistry/combine.rs`; `src/perception/harmonics.rs` (derived cache invalidation); `src/simulation/state.rs` (skipped caches and revision invalidation).
- **Authoritative rule or equation found:** current chemistry equations remain authoritative for a cache miss; the cache stores reusable static potential, not live geometry/contact validity, realized bond strength, activation barriers, reaction accumulation, or energy settlement. Live geometry and bond identity remain authoritative. Base single-unit materials preserve the established resource-name keys; mixtures/non-unit quantities retain explicit composition/amount identity.
- **Actual runtime call path:** the reaction path builds a material identity and interface-aware chemistry key, checks the persistent chemistry library, computes the static interaction potential on a miss, and calls `get_or_insert_static_potential`. If insertion returns no value or errors, the live caller falls back to the freshly calculated value. Separately, harmonic spectrum caching uses a tuple key and invalidates it on structure/position changes or explicitly at simulation boundaries; cavity/development caches use structure revisions and are skipped by Serde.
- **State mutated and transaction boundary:** chemistry-library misses may persist a reusable record. Cache write failure does not block the physical reaction because the freshly calculated value is used. The reaction still has to pass live geometry, bond-state, activation and energy transaction validation. Derived organism caches are recomputed rather than treated as persistent truth.
- **Energy/material source and sink:** cache reads/writes do not create physical energy or material. They can change which static-potential value is supplied to reaction accumulation, so stale/incompatible records could alter future reaction eligibility despite not directly mutating energy.
- **Failure/rollback behavior:** the runtime fallback is resilient to cache persistence failure, but cache hits appear to be accepted as stored facts rather than revalidated against the current equation/parameter version. The chemistry-library schema records the data schema but no separate equation/parameter fingerprint was evidenced. The cache key quantizes interface-signature floating fields at 1e-9; this is deterministic identity normalization, not a physical tolerance or permission to bypass geometry.
- **Numerical edge cases and invalid-state behavior:** library values are validated as finite and nonnegative, and miss values derive from chemistry math. Key canonicalization and quantization require stable treatment of negative zero, near-quantization boundaries, nonfinite values, and changed equation constants. The caller's calculated fallback can be used even if persistence fails; ensure invalid computed values fail closed before reaction accumulation rather than being hidden by a cache error path.
- **Complexity and scaling implications:** persistent lookup avoids repeated static-potential calculation and miss-only persistence avoids a disk write for every cache hit. Actual hit rate, cold-load cost, disk growth, contention, and lookup latency were not measured here. Keep the library an optimization and avoid making its presence a correctness prerequisite.
- **Tests already covering it:** source includes a cache-key compatibility test for base-resource names, chemistry-library record validation and persistence tests, and a sulfur geometry-change test asserting established cached chemistry output remains stable. No current-head focused-test results were verified in this audit.
- **Focused tests to add or run:** (1) cache hit equals authoritative recomputation for a version-matched key; (2) equation-version mismatch invalidates or recomputes; (3) malformed/corrupt/missing library falls back without changing physical invariants; (4) failed cache write does not block a valid reaction; (5) unknown resource identity fails closed; (6) key canonicalization tests for pair order, mixture composition, negative zero, and quantization boundaries; (7) cold/hot lookup and growth benchmarks on representative workloads.
- **Findings ranked by severity:** **P1** — no equation/parameter fingerprint was evidenced, so cache hits can outlive the chemistry equations that produced them. **P1** — cache records can influence reaction thresholds and therefore need versioned scientific semantics, not only schema compatibility. **P2** — fallback/error behavior is resilient but needs explicit tests proving invalid calculations cannot proceed. **P2** — cache scaling metrics are not established.
- **Dependencies / downstream audits affected:** A1–A3, A7–A8, A14, A18, A20–A21; resource-name migration must be coordinated with cache keys and existing saved records.
- **Decision required from project owner, if any (historical wording):** D13 selects chemistry cache identity versioning by equations and parameters; D6 rejects pre-unification caches. Inventory every cache and verify both rules are enforced.
- **Recommended smallest next change (if authorized):** after the full audit, add a chemistry-equation fingerprint to the cache manifest/key and prove hit/miss equivalence plus failure fallback. No production code change authorized now.


### A15 — Simulation ordering, event scheduling, determinism, and stale operations

**Audit record — 2026-10-22**

**Status: Gap found.**

- **Exact baseline and files/lines inspected:** `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5`; `src/simulation/simulation.rs` (step scheduler, active transformation preparation/completion, living-population/death helpers); `src/chemistry/transformation.rs` (staged BREAK settlement); `src/organism/reproduction.rs` (construction advancement); `src/simulation/world_tone_snapshot.rs` (frame staging and settlement); `src/simulation/state.rs` (persistent active-operation state).
- **Authoritative rule or equation found:** a physical operation should validate against current state, stage all consequences, and commit geometry/material/energy together. An event whose assumptions became stale must fail without partial mutation. Simulation ordering itself determines which events can observe same-tick changes and is therefore part of the scientific model.
- **Actual runtime call path:** `Simulation::step` increments the tick, drains active transformations, decrements remaining durations, prepares operations at kind-specific boundaries, restores source material when preparation fails or the owner no longer exists, collects completed transformations, and then processes completion state. The same tick also has world-tone sampling/capture, organism behavior, movement, chemistry and lifecycle updates; the exact full ordering should be documented against the entire method before behavior changes.
- **State mutated and transaction boundary:** active transformations persist IDs, owner IDs, source material, prepared structure/storage/energy/stress/ledger and optional environmental source. The code attempts to stage preparation and restore source material on failure. World-tone capture stages reservoirs and validates the frame before settling source accounting. Reproductive construction stores a developing graph and its energy independently from the parent.
- **Energy/material source and sink:** pending COMBINE/BREAK sources may be temporarily removed from storage or environment while retained in active-operation state; this is a valid ownership transfer only if every failure/cancellation/owner-removal path restores exactly one source. World-tone input is an external source and needs one frame-level settlement; child energy is an internal transfer. A completed operation must not charge or release its bond/source twice.
- **Failure/rollback behavior:** the scheduler explicitly restores source material when preparation fails or the organism is absent. This is a useful rollback path, but recovery can deposit to the environment if storage restoration fails, changing location while retaining material. Need verify all cancellation, malformed owner IDs, stale structure revisions, and panic/error boundaries. A partially completed world-tone frame is designed to abort before credit when geometry/exposure validation fails; confirm all later settlement failures preserve that all-or-nothing property.
- **Numerical edge cases and invalid-state behavior:** duration scheduling distinguishes COMBINE from other transformations (`remaining_ticks == 0` versus `== 1` preparation). Off-by-one behavior and save/load at each duration boundary need tests. IDs and owner references must remain unique and consistent; overflow/wrapping revision counters and tick counters require explicit long-run policy.
- **Complexity and scaling implications:** active transformations are drained and rebuilt each tick; stable deterministic ordering is currently based on vector iteration. World-tone snapshot construction processes all realized bodies and per-path first-hit widths; it avoids per-organism independent full geometry setup but remains geometry-heavy. No runtime profile was run in this audit.
- **Tests already covering it:** simulation and transformation tests exist, including transaction and world-tone tests. Exact-head `main` CI counts are recorded in A0; individual failing-test identities remain only commit-attested, not independently enumerated in this pass.
- **Focused tests to add or run:** (1) exact tick-boundary tests for each operation kind; (2) same-seed repeatability with simultaneous movement, chemistry, and world tone; (3) stale owner/structure/bond and duplicate-operation tests; (4) save/load immediately before and after preparation/commit; (5) failure injection at every commit stage proving source and energy are neither duplicated nor lost; (6) compare frame settlement totals with per-path first-hit exposure under overlapping/occluded bodies.
- **Findings ranked by severity:** **P1** — complete tick ordering and event visibility contract is not yet written as one authoritative sequence. **P1** — pending operation ownership/rollback has multiple state locations and needs cross-path atomicity proof. **P2** — deterministic vector order may be an implicit tie-breaker rather than an explicit contract. **P2** — snapshot geometry and operation scheduling need profiling together at target population sizes.
- **Dependencies / downstream audits affected:** A3–A6, A8–A13, A16, A18–A20. World tone and lifecycle changes should wait until event ordering and transaction boundaries are specified.
- **Decision required from project owner, if any (historical wording):** D9 requires explicit deterministic tick phases while allowing multi-tick operations. The remaining gap is the authoritative phase table and stale/cancelled-operation semantics.
- **Recommended smallest next change (if authorized):** after all audits, document the exact tick phase order and add boundary/failure-injection tests before refactoring scheduler code. No production change authorized now.

### A16 — Whole-world material conservation and physical representation

**Audit record — 2026-10-22**

**Status: Gap found.**

- **Exact baseline and files/lines inspected:** `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5`; `src/materials/resources.rs` (`Material`, constituent amounts, internal bonds and validation); `src/materials/physical_material.rs` (realization, placements, internal connections and shape overrides); `src/simulation/state.rs` (organism storage and environment fields); `src/organism/reproduction.rs` (anchor transfer and child-held material); `src/simulation/world_tone_snapshot.rs` (physical occluder inventory); `src/chemistry/transformation.rs` (bond graph changes).
- **Authoritative rule or equation found:** physical constituents must have exactly one owner/location at a time: environmental field, organism structure, organism storage, active transformation, developing offspring, or a retained legacy carrier. Moving material between those owners must preserve constituent identity and amount. Realized graph bonds have explicit stored bond energy; material-internal connections currently preserve topology/endpoints but do not independently store a bond-energy amount.
- **Actual runtime call path:** logical `Material` composition is realized into `PhysicalMaterial` with one placement per constituent and internal connection endpoint records. Storage and environmental acquisition/expulsion move physical instances; COMBINE/BREAK moves material between the environment, storage and organism structure; reproduction transfers a physical anchor to a separate developing graph. The sunlight occluder snapshot includes environmental physical material, organism structures, decomposing structures, stored physical material and material temporarily held by active transformations, demonstrating that the implementation already recognizes multiple simultaneous physical holders.
- **State mutated and transaction boundary:** physical material realization validates composition, placement finiteness and internal bond endpoint candidates before returning an instance. Restoration and transfer code is distributed across material restoration, storage, construction and transformation paths. A world-level material inventory/reconciliation authority was not evidenced in the inspected production paths.
- **Energy/material source and sink:** constituent potential is a catalog-derived property, not a consumable wallet. Material conservation is about counts/amounts and ownership; bond energy is separate. Releasing a material from a finished physical structure should move the same constituent(s) to the environment, not mint or delete them. Internal bonds in a composite are topology only in the current representation, while realized graph bonds carry stored bond energy; this distinction must remain explicit until internal bond energetics are designed.
- **Failure/rollback behavior:** `PhysicalMaterial::realized` returns failure on invalid material/placement or when a declared internal bond cannot map to a physically valid endpoint pair. Some restoration/transfer helpers return `Option`/boolean; fallback deposition paths exist. A full proof that all caller failures roll back storage, geometry and field state atomically is not available from this audit.
- **Numerical edge cases and invalid-state behavior:** current physical material realization only accepts constituent amounts approximately equal to 1.0 and placements count equal to constituent count; aggregate logical materials may accept fractional amounts, so conversion between logical quantities and physical discrete constituents needs a declared policy. Unknown resource names can remain in a `Material` because catalog membership is not intrinsic to its validation; this interacts with A1's silent-zero property behavior.
- **Complexity and scaling implications:** a full inventory scan each tick would be expensive and risks duplicate accounting across aliases/temporarily-held sources. Prefer transaction deltas plus a periodic/debug independent audit; never count a pending transformation source both in its old storage and its active record.
- **Tests already covering it:** source has physical-material realization, restoration, storage/transfer, reproduction and transformation tests; no current-head complete material inventory test result was verified.
- **Focused tests to add or run:** (1) constituent inventory before/after every transfer, expulsion, COMBINE, BREAK, reproduction and death; (2) fractional aggregate material conversion policy; (3) internal-bond endpoint round-trip and material shape override persistence; (4) pending transformation source counted once; (5) duplicate/missing resource IDs and invalid catalog names; (6) independent slow inventory oracle compared with transaction-delta counters in debug/test builds.
- **Findings ranked by severity:** **P1** — no complete production whole-world material conservation check was evidenced. **P1** — fractional logical amounts and discrete physical realization do not yet share a documented conversion contract. **P2** — material-internal bonds have physical topology but no independent stored bond-energy account. **P2** — distributed ownership makes temporary source duplication/omission possible unless transaction boundaries are checked centrally.
- **Dependencies / downstream audits affected:** A1, A3–A6, A11–A13, A18–A21. Resource migration, composite chemistry and death/material fate must align with this inventory contract.
- **Decision required from project owner, if any (historical wording):** D8 prohibits fractional base-resource atoms; D3 defines ownership for realized bonds that carry stored energy. Implementation must enforce integer constituent accounting and inventory bond classes.
- **Recommended smallest next change (if authorized):** after the full audit, define canonical material-holder categories and build a slow test-only whole-world constituent inventory oracle before centralizing production deltas. No production change authorized now.

### A17 — Diagnostics, observability, UI, and scientific interpretability

**Audit record — 2026-10-22**

**Status: Gap found.**

- **Exact baseline and files/lines inspected:** `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5`; `src/infrastructure/diagnostics.rs`; `src/simulation/state.rs` (ledger counters and movement diagnostics); `src/chemistry/energy_ledger.rs` (inventory snapshot); `src/simulation/world_tone_snapshot.rs`; `src/perception/harmonics.rs`; README and handoff claims located by repository search.
- **Authoritative rule or equation found:** diagnostics and UI are observers, not physical authorities. Every reported value should identify whether it is a current holder inventory, a cumulative flow, a normalized reaction signal, a cached/derived quantity, a work term, or a sink such as heat/escaped signal. Instrumentation must not mutate simulation behavior or silently substitute for conservation enforcement.
- **Actual runtime call path:** `DiagnosticsRecorder` is a staged, opt-in recorder intended to write JSONL events and a summary. It captures before/after snapshots of organism state, transformations, geometry counts, stored material, resonance, movement and development. Its header currently writes a fixed seed value of 42, and the recorder module is annotated as retained for exploratory/UI integration. Energy inventory snapshot is marked as test/reconciliation support rather than production telemetry.
- **State mutated and transaction boundary:** diagnostics writes files and aggregates counters; these are observational side effects, not simulation transactions. Ledger counters persist on simulation state, while event summaries are derived. Diagnostics file I/O errors can stop or fail reporting and should not alter already-committed physical state.
- **Energy/material source and sink:** diagnostic counters are not energy sources/sinks. Report cumulative input, usable/resonant/structural held energy, work, heat and escaped tone distinctly; do not add cumulative totals to current inventory or double-count internal transfers. Material diagnostics should report location/holder counts without implying that display aggregates are authoritative.
- **Failure/rollback behavior:** the recorder uses fallible file creation and writes; an I/O error may interrupt diagnostics collection. No evidence establishes that every runtime configuration path correctly isolates such errors from simulation stepping, or that summary files are atomically finalized. The hard-coded seed in the diagnostic header can misidentify a run if the actual simulation seed differs.
- **Numerical edge cases and invalid-state behavior:** `EnergyInventorySnapshot::residual` rejects nonfinite or negative inventory/flow values; diagnostic summaries use optional extrema and counters. The UI/diagnostics must preserve “unknown/unavailable” rather than serializing NaN as a misleading zero or claiming a balanced inventory when the snapshot is incomplete.
- **Complexity and scaling implications:** detailed JSON snapshots serialize nested structures/genomes/stored material and can grow quickly. The recorder has an interval and stopped flag, but event volume, memory footprint, write throughput and summary accuracy have not been benchmarked. Prefer sparse event records and bounded aggregation for large populations.
- **Tests already covering it:** diagnostics recorder and serialization code exist; the inventory snapshot has test-oriented use; no current-head focused-test result was verified. The source annotation explicitly says the diagnostic API remains staged rather than a production guarantee.
- **Focused tests to add or run:** (1) actual seed and configuration appear in diagnostic header; (2) diagnostic-on/off simulations with same seed produce identical authoritative state; (3) file I/O failure does not corrupt or alter simulation state; (4) inventory fields are labelled and units documented; (5) unavailable/invalid metrics remain unavailable; (6) bounded output and summary consistency under long runs.
- **Findings ranked by severity:** **P1** — global conservation is not continuously observable/enforced in production, and the inventory snapshot remains test-oriented. **P2** — diagnostic header hard-codes seed 42, which can make evidence misleading. **P2** — diagnostic API is staged and lacks verified runtime wiring/performance bounds. **P3** — summaries/UI need explicit units and current-versus-cumulative distinctions to remain scientifically interpretable.
- **Dependencies / downstream audits affected:** A0, A5–A9, A13, A16, A18–A21. Diagnostics should be wired only after the underlying inventory semantics and source boundaries are stable.
- **Decision required from project owner, if any (historical wording):** D11 requires strict tests to fail on conservation violations and exploratory diagnostics to report labelled residuals without silent repair. Remaining work is implementation and clear residual/tolerance reporting.
- **Recommended smallest next change (if authorized):** after all audits, make the diagnostic header reflect the actual run configuration and expose a read-only, explicitly labelled inventory residual without allowing diagnostics to become an alternate accounting authority. No production code change authorized now.


### A18 — Numerical stability, invalid states, and long-run behavior

**Audit record — 2026-10-22**

**Status: Gap found.**

- **Exact baseline and files/lines inspected:** `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5`; `src/chemistry/chemistry.rs` (interaction math, normalized energy conversion and reaction accumulation); `src/chemistry/energy_ledger.rs` (transaction balance and inventory residual); `src/perception/harmonics.rs` (reservoir arithmetic and spectral compatibility); `src/chemistry/transformation.rs` (bond-break energy settlement); `src/simulation/state.rs` (revision/tick/ledger counters).
- **Authoritative rule or equation found:** inputs and computed outputs used as physical energy must be finite and within the documented domain. Internal transactions balance within a tolerance; world inventory residual is meaningful only when every holder and boundary flow is included. The reaction accumulator is a decaying trigger quantity, not stored energy.
- **Actual runtime call path:** chemistry helpers calculate bounded interaction/contact values and update a decaying accumulator; the result drives activation checks. Ledger transaction methods reject nonfinite/negative allocation terms and check balance. Resonance deposit and total-energy validation reject invalid state and overflow, while compatible-energy calculation multiplies stored energy by a frequency compatibility curve. World inventory snapshot computes a signed residual from initial/current usable, resonant and structural stores plus cumulative sources/sinks.
- **State mutated and transaction boundary:** accumulators and reservoir components mutate over ticks; bond and energy changes should commit through the transaction authority. Revision counters wrap with `wrapping_add`; tick and cumulative counters are long-running state. Snapshot residual is a derived diagnostic and does not itself constrain the live step.
- **Energy/material source and sink:** reservoir components hold actual retained energy; reaction accumulation is not an energy holder. Numerical underflow/pruning in the reservoir must settle removed energy to an explicit sink if it represents physical energy, rather than simply erasing it. Cumulative telemetry overflow can invalidate a residual even when local transactions remain balanced.
- **Failure/rollback behavior:** ledger transactions validate allocation and use finite arithmetic checks; reservoir deposit avoids mutation on an invalid next component energy. However, chemistry accumulation can return `Some` after arithmetic in the computed next value without a final explicit finite check, so a finite-but-extreme previous accumulator may overflow to infinity. Reservoir withdrawal removes tiny residual components using an epsilon threshold; the fate of that residual energy needs to be explicit if the pruning is physical rather than numerical normalization.
- **Numerical edge cases and invalid-state behavior:** relevant adversarial cases include NaN/infinity, negative energy, very large finite values, overflow in sums, underflow and epsilon pruning, duplicate/near-identical frequencies, quantized cache keys, zero or extreme distances, near-zero activation thresholds, tolerance accumulation over long runs, and cumulative counters exceeding finite f64 range. The use of f64 tolerances is not a proof that repeated per-event rounding remains globally bounded.
- **Complexity and scaling implications:** validating every local transaction is cheap, but an independent full-world sum every tick scales with the number of holders/bonds. Use local finite checks continuously and a slower independent oracle in tests/debug/periodic audits; use compensated or stable summation if profiling demonstrates significant aggregate error.
- **Tests already covering it:** math helpers, ledger balance/residual, reservoir validation, and transformation energy-yield functions have unit tests in source. Current-head test execution was not verified.
- **Focused tests to add or run:** (1) accumulator overflow from a maximum finite prior value; (2) long repeated additions/decays against a higher-precision oracle; (3) reservoir sum/withdraw/deposit invariants at epsilon and max-component boundaries; (4) ledger cumulative-counter overflow; (5) repeated transaction residual drift; (6) invalid serialized energy and extreme finite values; (7) frequency quantization and duplicate component edges.
- **Findings ranked by severity:** **P1** — reaction accumulation lacks a proven end-to-end finite upper bound and may overflow from extreme prior state. **P1** — epsilon pruning and long-run counter overflow have no fully documented energy/telemetry fate. **P2** — local tolerances do not yet have a measured global error budget. **P2** — wrapping revisions and long-run tick/counter semantics need a documented overflow policy.
- **Dependencies / downstream audits affected:** A2, A5–A9, A13–A14, A19–A21. The unit contract and invalid-state policy must precede changes to shared math/ledger primitives.
- **Decision required from project owner, if any (historical wording):** D4 separates numerical tolerance from physical dissipation. The implementation must define and test the tolerance policy without silently erasing stored physical energy.
- **Recommended smallest next change (if authorized):** after all audits, add explicit finite checks at accumulator boundaries and define a measured tolerance/pruning budget with adversarial tests. No production code change authorized now.

### A19 — Scalability and profiling at target populations

**Audit record — 2026-10-22**

**Status: Gap found (complexity hotspots identified; representative runtime benchmarks unavailable).**

- **Exact baseline and files/lines inspected:** `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5`; `src/simulation/world_tone.rs`; `src/simulation/world_tone_snapshot.rs`; `src/simulation/simulation.rs`; `src/chemistry/chemical_reaction.rs`; `src/chemistry/chemistry_library.rs`; `src/perception/harmonics.rs`; README and archived profiling/design notes.
- **Authoritative rule or equation found:** scalability is a correctness requirement: retain physically meaningful first-hit/occlusion behavior without a per-object ray trace or whole-world cloning per tick. Static reusable values may be cached, but position-dependent exposure and energy allocation must remain live and source-backed.
- **Actual runtime call path:** the world-tone cycle precomputes source samples for supported cycle lengths and uses indexed samples at runtime; long cycles fall back to direct sampling. A per-tick geometry snapshot assembles relevant physical bodies and computes a shared exposure frame, then verifies that first-hit path widths agree with direct projected widths before settling. Chemistry uses a persistent static-potential cache and calculates on miss. Reservoir/harmonic caches are bounded to a small spectrum, and several structure-derived caches use revisions.
- **State mutated and transaction boundary:** source-cycle precomputation is configuration-time derived state; world geometry/exposure snapshots are per-frame temporary state; chemistry library misses persist reusable static potential. Physical energy is settled only after the shared frame is validated. Cache errors should not change the authority of live geometry or equations.
- **Energy/material source and sink:** performance shortcuts must not skip incident source energy, count a shadowed lane twice, treat an unabsorbed signal as a second source, or silently discard energy because a receiver falls outside an optimization's candidate set. All realized material that can intercept a path must remain in the physical occluder/receiver representation.
- **Failure/rollback behavior:** world-tone snapshot construction and consistency checks fail closed before credit when geometry is invalid or inconsistent. The exact effect of all later settlement failure branches must remain atomic. Cache miss persistence falls back to calculated potential on error. No representative current-head performance run was available, so throughput and worst-case recovery cost are unverified.
- **Numerical edge cases and invalid-state behavior:** dense overlapping bodies, many narrow projected widths, large coordinates, near-tangent contacts, very large body counts, long source cycles and high cache cardinality may amplify floating-point error or memory use. A conservative angular broad phase must not omit an actual occluder.
- **Complexity and scaling implications:** world-tone geometry is a likely dominant hotspot because it processes realized structures/material and interaction paths; candidate/body relationships may grow quadratically in unfavorable scenes even with broad-phase filtering. Chemistry cache disk growth and cold-load latency also need measurement. No new benchmark was run and no exact target population was asserted as measured.
- **Tests already covering it:** source has precomputed-versus-direct world-tone cycle comparisons, exposure consistency checks and chemistry-library tests; these are correctness tests, not performance evidence. Historical notes are not treated as current measurements.
- **Focused tests to add or run:** (1) deterministic benchmark scenes at increasing organism/material counts; (2) separate cold-start, steady-state tick, cache-hit and cache-miss measurements; (3) sparse and worst-case dense occlusion; (4) compare optimized exposure to a slow exact oracle with bounded error; (5) memory/disk growth for persistent chemistry and Bob libraries; (6) enforce a documented runtime/memory budget on the target gaming-laptop class.
- **Findings ranked by severity:** **P1** — current source inspection does not establish that world-tone capture scales to target populations. **P1** — no measured runtime/memory envelope is available for the combined simulation and chemistry work. **P2** — cache effectiveness and cold-load/storage growth are not quantified. **P2** — any faster approximation must retain a documented error bound and energy-allocation invariant.
- **Dependencies / downstream audits affected:** A8, A14, A16, A18, A20–A21. Performance design must follow the same world inventory and first-hit semantics, not bypass them.
- **Decision required from project owner, if any (historical wording):** D14 requires representative workloads and repeatable benchmarks before choosing numeric performance/error thresholds. The high-level sunlight approximation direction is already selected by D5; thresholds must come from measurements.
- **Recommended smallest next change (if authorized):** after all audits, build a repeatable profiling harness and compare the current shared snapshot with the proposed single-sweep/first-hit approximation against an exact small-world oracle. No production code change authorized now.

### A20 — Cross-domain invariants and integration tests

**Audit record — 2026-10-22**

**Status: Gap found.**

- **Exact baseline and files/lines inspected:** `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5`; `src/chemistry/energy_ledger.rs`; `src/simulation/simulation.rs`; `src/chemistry/transformation.rs`; `src/organism/reproduction.rs`; `src/simulation/world_tone_snapshot.rs`; `src/materials/physical_material.rs`; `src/perception/harmonics.rs`; README and current-head workflow/status evidence queried in A0.
- **Authoritative rule or equation found:** cross-domain correctness requires the same event to preserve geometry, material ownership, energy allocation, resonance, biological eligibility, and persistence together. For a complete world energy inventory, `change(usable + resonant + structural) + work + heat + escaped signal = external inputs + explicitly released non-structural sources`; internal transfers cancel. For material, each constituent must be counted exactly once across all physical holders.
- **Actual runtime call path:** individual primitives have local tests for balanced ledger transactions, bond-break settlement, reservoir behavior, physical-material realization, reproduction, and world-tone exposure. `Simulation::energy_inventory_snapshot` is explicitly annotated as used by reconciliation tests rather than production telemetry. A corresponding complete material inventory oracle was not evidenced. A0 could not verify test results for the exact current baseline commit, so old PR #202 counts remain historical rather than current-head evidence.
- **State mutated and transaction boundary:** the most consequential shared commits are COMBINE/BREAK, environmental capture, movement, maintenance, reproduction transfers, storage/field movement, and load/recovery of pending operations. Unit-level balance does not prove those paths jointly conserve state through cancellation, death, failed geometry, or restart.
- **Energy/material source and sink:** known source boundaries include explicit world-tone input and initial simulation energy; known sinks include work, heat, escaped signal and any approved numerical dissipation. Bond energy is a persistent store and its release must be counted once. Material holders include world field, organism graph/storage, active transformations, developing offspring, and legacy carriers. Every source/sink and holder must be represented exactly once in integration-level reconciliation.
- **Failure/rollback behavior:** current code has multiple staged local transactions and explicit restoration helpers, but no verified integration suite injects failures across all coupled state domains. Source-frame abort, stale transformations, save/load and resource/catalog mismatch must all preserve the same invariant.
- **Numerical edge cases and invalid-state behavior:** integration tests must combine rather than isolate finite-value validation, tolerance accumulation, missing catalog identities, maximum reservoir spectrum, duplicate source claims, stale operations and pending-state restoration.
- **Complexity and scaling implications:** exact whole-world inventory scans are appropriate as a test oracle and periodic diagnostics but should not be forced into every production tick without profiling. Property-based/randomized transaction sequences can find interactions while keeping generated worlds bounded.
- **Tests already covering it:** focused unit tests exist across the listed modules; no evidence proves a single integration test currently exercises a complete tick with source capture, movement, chemistry, reproduction/material transfer, death and reconciliation. Exact-head CI remains unverified.
- **Focused tests to add or run:** (1) tick-zero inventory and source accounting; (2) multi-tick worlds with world tone on/off and explicit energy residual; (3) COMBINE then BREAK with exact structural-store delta; (4) movement + maintenance + resonance withdrawal in one tick; (5) reproduction transfer plus material anchor ownership; (6) core breach and post-death chemistry; (7) serialize/restore with pending operation; (8) injected failure before/after each commit boundary; (9) slow independent material/energy inventory oracle; (10) same seed replay and accepted-baseline failure identity comparison.
- **Findings ranked by severity:** **P0/P1** — whole-world energy and material conservation are not proven across the integrated production path. **P1** — no verified end-to-end integration matrix covers transaction failure, persistence and lifecycle together. **P1** — exact-head CI reports 28 known failures, but individual failure identities were not independently extracted in this pass. **P2** — existing focused tests can pass while source, holder, or sink paths remain disconnected.
- **Dependencies / downstream audits affected:** all previous audits; A20 is the implementation gate that must consume the findings from A0–A19 before sequencing any behavior changes.
- **Decision required from project owner, if any (historical wording):** D1–D15 now settle the design choices listed here. Remaining approval is for the implementation sequence; the invariant matrix, independent inventory oracles, and evidence-based thresholds still need to be prepared.
- **Recommended smallest next change (if authorized):** after all audits, establish an integration-test matrix with independent energy/material inventory oracles and failure injection; then implement the smallest shared transaction primitive first. No production code change authorized now.


### A21 — Documentation/spec alignment and implementation sequencing

**Historical status note:** this audit record predates the explicit owner decisions in Section 14. Its claims that resource identity, initial-energy treatment, internal-bond energy, and world-tone policy still require owner selection are superseded. The remaining tasks are implementation tracing, stale-reference cleanup, and evidence/threshold work; do not reopen D1–D15.

**Audit record — 2026-10-22**

**Status: Gap found (audit plan complete; authoritative project status and several design choices require reconciliation).**

- **Exact baseline and files/lines inspected:** `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5`; `README.md`; `docs/README.md`; `README_HANDOFF_ARCHIVE_2026-10-22.md`; `docs/chemistry-audit-unification-plan.md`; A0–A20 records and source references.
- **Authoritative rule or equation found:** Master Technical Specification v2.4 (`EVOSIM-MASTER-2.4`) is design authority; the root README is the current implementation/status guide; dated handoffs are historical evidence. Current status claims must agree with the exact current tree, and unresolved behavior must remain explicitly unresolved rather than being promoted from an old handoff to a verified fact.
- **Actual runtime call path (historical snapshot, reconciled by Sections 14–16):** the audited `main` SHA is `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5`, PR #203 is merged there, and a live world-tone capture/settlement path exists. The root README's PR/capture statements are stale. Oxygen is canonical under D1; the remaining resource work is to trace stale Methane references and enforce the cache/catalog contract, not to choose the identity again.
- **State mutated and transaction boundary:** this audit changed only the audit plan document. No README, source, test, library data, workflow, or runtime behavior was changed. The current PR remains a documentation/planning vehicle; its commit history contains only updates to the audit plan.
- **Energy/material source and sink:** this historical audit correctly identified unproven source/holder/sink coverage, but D2, D3, D4, and D5 now settle the relevant design direction for initial energy, stored bond energy, tolerance versus dissipation, and sunlight approximation. Implementation, independent inventory, source allocation, and full material ownership reconciliation remain open.
- **Failure/rollback behavior:** not applicable to production behavior in this documentation-only audit. Documentation updates should be evidence-gated: do not claim CI success, a clean baseline, or disabled capture unless exact-head workflows and the live call path support the statement.
- **Numerical edge cases and invalid-state behavior:** the project spec/resource identity mismatch affects persisted material names, geometry records, chemistry cache keys, tests and migrations. It must not be repaired by a global string replacement without catalog/ID/key migration coverage.
- **Complexity and scaling implications (updated by Section 17):** (1) preserve the named-baseline evidence limitation unless logs allow independent extraction; (2) apply D1/D6/D13 while tracing resource identity and cache invalidation; (3) define units and build independent energy/material oracles; (4) enforce integer constituent ownership; (5) unify ledger and bond transactions/tick phases; (6) preserve ordinary post-death chemistry; (7) compare the first-hit sunlight approximation against a small-world oracle; (8) profile representative workloads; (9) update README only after evidence supports the claims. Constructor/Bob PR #196 remains separate and deferred.
- **Tests already covering it:** the exact-head workflow/run and recorded counts are documented in A0. The commit message attests the 28 failures match the accepted baseline, but the individual failure identities were not independently extracted. Older PR #202 counts are not current-main evidence.
- **Focused tests to add or run:** (1) independently extract named baseline failures if logs permit; (2) test canonical Oxygen and stale-name rejection across catalog/cache/fixtures; (3) test live world-tone source allocation and exact-small-world comparison; (4) run independent energy/material inventory scenarios; (5) test equation-fingerprint cache invalidation and actionable rejection of pre-unification cache versions (no migration); (6) test future save-version rejection only if/when a durable simulation-save format is introduced; (7) measure target-population runtime/memory and approximation error.
- **Findings ranked by severity:** **P1** — root README remains stale about PR #203/capture, so status should be corrected only after evidence review. **P1** — whole-world energy/material conservation, cache equation identity, and live source allocation remain unproven. **P1** — D8 conflicts with fractional aggregate extraction still represented in source/tests. **P1** — exact-head named baseline failure identities were not independently extracted. **P2** — diagnostics, performance budgets, and staged legacy helper disposition remain to be completed.
- **Dependencies / downstream audits affected:** all A0–A20. This is the sequencing gate; it consolidates but does not resolve their findings.
- **Decision required from project owner, if any (historical wording):** this record predates Section 14. D1–D15 now settle the listed design choices; remaining approval is for the implementation sequence, while acceptance thresholds must be grounded in measurement.
- **Recommended smallest next change (if authorized):** review the authoritative implementation plan in Section 17 and explicitly approve or revise its sequencing. Do not begin production code until approval. No production code change is authorized in PR #204.

## 12. Consistency pass — 2026-10-22

**Historical snapshot note:** this consistency pass was written before the owner recorded D1–D15 in Section 14. Its statements that resource identity, initial energy, internal-bond energy, numerical pruning, and world-tone approximation are still undecided are superseded by that later decision log. The technical implementation and verification gaps remain open.

**Result: structural consistency pass completed; substantive gaps remain open.**

- Cross-checked the 22 audit headings (A0–A21) against the 22 audit-register rows. Each planned domain has a corresponding audit record and a summary row; all are correctly represented as **Gap found** or **Blocked**, not as resolved or verified.
- Reconciled the world-tone status language: source inspection indicates a live capture-settlement path despite the README's disabled-capture claim. The plan now treats capture as present but not yet conservation-validated and explicitly prohibits toggling runtime behavior in this documentation-only PR.
- Updated the baseline conclusion from exact-head GitHub Actions evidence: `main` at `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5` recorded 563 passed, 28 failed, and 1 ignored; its commit message reports the 28 failures match the accepted baseline. Individual failing-test identities were not independently enumerated from the logs in this pass.
- At the time this snapshot was written, resource identity/migration, initial-energy treatment, internal composite-bond energy, numerical truncation, and world-tone approximation bounds were still open. Section 14 subsequently records the owner's decisions; implementation and evidence gates remain.
- Confirmed the dependency order keeps independent inventory oracles and transaction/failure-injection coverage ahead of behavior changes, and keeps PR #196 constructor/Bob work separate.
- Scope remains documentation-only: no production code, tests, README, resource catalog, or persistent library data was changed. The plan is populated, but chemistry unification and whole-world conservation are not proven.

## 13. Earlier dependency-ordered outline — superseded by the formal plan in Section 17

**Historical outline only; Section 17 is authoritative.** This older table is retained to preserve the audit trail, not as a second plan. The owner has selected D1–D15 in Section 14; do not treat those decisions as open. No production change begins until the owner explicitly approves Section 17's implementation sequence. Apply one focused pre-change audit before each implementation change; keep each command/test bounded to five minutes and compare failures against the exact `main` baseline recorded in A0.

| Phase | Work and dependencies | Exit gate before advancing |
|---|---|---|
| 0. Baseline freeze | Use `main` SHA `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5` as the audited baseline. Record Rust workflow run `39705628427`, result 563 passed / 28 failed / 1 ignored, and the commit's statement that the 28 failures match the accepted baseline. If available, independently extract the named failures; otherwise preserve that limitation explicitly. | Baseline SHA/run/counts recorded; no claim that the suite is green. |
| 1. Contract implementation and vocabulary consistency | Apply D1–D4 and D7–D8 from Section 14: Oxygen is canonical; initial energy is a one-time external-source transaction; stored bond energy has explicit ownership and exactly-once settlement; numerical tolerance is distinct from physical dissipation; base resources are indivisible. Document the units contract and trace stale identity references. | Source/docs agree with settled decisions; unknown identities and fractional base-resource counts have defined fail-closed behavior. Decisions are not reopened here. |
| 2. Independent inventory oracles | Add test-only whole-world energy and material inventory oracles spanning world field, organism structure/storage, resonance, active/staged operations, developing offspring, and persistent/recovery state. Define the source/sink equation and residual tolerance. | Tests prove internal transfers cancel and each holder/source/sink is counted once; include deliberately broken/duplicate-claim cases. |
| 3. Transaction and bond accounting | Reconcile COMBINE/BREAK and related bond transitions around one validate → stage → settle → commit boundary. Ensure stored bond energy is created/released once; failures/stale events leave geometry, material, energy and resonance unchanged. | Focused transaction tests plus inventory-oracle checks pass for successful and rejected reactions. |
| 4. Resource/catalog/cache compatibility | Implement the approved resource identity migration only after phase 1; validate all material names against the catalog; version chemistry equations/parameters in cache identity and prove cache-hit results match authoritative recomputation. Cover persisted chemistry/Bob keys where affected. | Migration/round-trip tests pass; unknown identities fail explicitly; cache failures cannot change physical validity. |
| 5. Consumers, transfers and tick ordering | Repair ignored transfer outcomes, initial-source accounting, maintenance/work/heat flows, movement-associated energy, reproduction material/energy transfer (without pulling PR #196 constructor work), event ordering, cancellation and stale-operation rollback. | Cross-domain tests show conservation through a full tick and through injected failures/cancellations. |
| 6. Lifecycle and post-death chemistry | Make core breach/approved stress death transition biological eligibility only; retain body/material and route post-death bond changes through ordinary chemistry and the same transaction authority. | Death and post-death tests preserve inventory and cannot duplicate bond release or skip material ownership. |
| 7. Persistence and numerical integrity | Validate authoritative holders and pending transactions across save/load; add versioned migration/rejection policy; test finite/nonnegative state, accumulator overflow, epsilon pruning, counter overflow and cache rebuild. | Round-trip, legacy-data and corruption/failure tests preserve or explicitly reject state without silent inventory drift. |
| 8. World-tone capture and efficient exposure | Audit the live existing call path against D5: reconcile incident source energy, width allocation, duplicate claims, occlusion, damping and onward frequency-shifted propagation. Compare the proposed fast first-hit/single-sweep approximation with a slow exact small-world reference; quantify error and profile representative thousands-object workloads. | Evidence establishes whether the selected approximation meets an explicitly documented error/performance budget and whether source allocation is conservative and deterministic. Do not reopen whether capture is present; do not change runtime behavior in this PR. |
| 9. Integrated verification and status | Run bounded focused tests, the whole suite, strict Clippy tracking, persistence tests and target-scale profiles. Compare named failures to the A0 baseline. Update root README only after the current implementation state is supported by evidence. | No new unexplained baseline failures; all new invariants pass; documentation accurately reflects live source and unresolved gates. |

### Implementation evidence and approval gates

**This list is superseded by Section 14 (D1–D15). Do not reopen these choices as unanswered questions.** The owner has selected Oxygen as the canonical identity, an explicit one-time initial-energy source transaction, explicit bond-owned energy with exactly-once settlement, a distinction between numerical tolerance and physical dissipation, a conditional fast first-hit/single-sweep sunlight approximation validated against an exact small-world reference, and rejection of all pre-unification saves/caches. The remaining work is to trace implementation coverage, identify stale references, define acceptance tests/thresholds where the owner has delegated them to evidence, and document the gaps.

The suggested first implementation slice in older planning text is not authorization to change code in PR #204. This PR remains documentation-only; no production or test changes are authorized here.

## Audit completion and implementation gate

The planned audit register A0–A21 has been populated. **Completion means the planned domains have been inspected and their gaps recorded; it does not mean chemistry is unified or conservation is proven.** Owner decisions D1–D15 are settled design direction and must not be listed as pending decisions. A0's exact-head main run and counts are documented; named baseline failures were subsequently extracted and compared by identity at implementation heads `25b72bb` and `407a163`. The outstanding evidence limitation is that the latest head `f56f6f0` has not completed the full Rust suite because a focused fluid restoration test failed first. No production code, tests, README status, resource catalog, or persistent library data has been changed in this documentation workstream.

Before the first production change, require:
1. Exact-head baseline evidence and failure-identity comparison for A0; preserve the accepted baseline counts and distinguish completed comparisons at earlier implementation heads from the still-unknown full-suite result at the latest head.
2. A dependency-ordered implementation sequence that applies D1–D15 rather than asking the owner to re-decide them; define any remaining thresholds through representative measurements and tests.
3. Independent energy/material inventory oracles and failure-injection tests planned before changing production accounting paths.
4. One focused pre-change audit before each implementation change, bounded verification, and exact-head comparison against the accepted failure baseline.

The owner decisions in Section 14 are already recorded. The implementation gate is evidence and sequencing, not another decision round.


---

## 14. Owner decisions recorded — 2026-10-22

The project owner selected the following directions for the chemistry unification work. These decisions resolve the corresponding design questions in this plan. They do not mean the behavior has been implemented or verified. Production code changes remain gated on the dependency-ordered implementation plan and the project's change-approval workflow.

### D1 — Canonical resource identity: Oxygen

**Decision:** Retain Oxygen as the canonical identity for resource ID 6; do not rename it to Methane.

**Implementation implications:** Update the approved resource vocabulary/specification to Oxygen so the catalog and governing design agree. Audit all resource names, serialized values, chemistry-cache keys, Bob/library keys, tests, and documentation for stale Methane assumptions. Do not silently treat Oxygen and Methane as aliases or distinct resources. Any historical data migration must preserve this chosen identity and be explicitly tested.

### D2 — Initial energy: explicit one-time source transaction

**Decision:** Represent initialization energy as an explicit, one-time external-source transaction (Option B).

**Implementation implications:** Establish a defined pre-initialization/empty-state boundary, record initial energy exactly once, and include it in the same conservation equation as later external inputs. The tick-zero inventory must remain independently reportable. Repeated initialization, restore, or replay must not duplicate the source event.

### D3 — Composite bond energy: explicit bond-owned record

**Decision:** Each realized bond that carries stored energy has explicit, unambiguous energy ownership; a bond's stored energy can be settled/released no more than once (Option A).

**Implementation implications:** Define stable bond identity or equivalent ownership, formation settlement, break settlement, rollback, and persistence behavior. A failed or retried BREAK must not release energy twice. This decision does not assume every topology edge necessarily stores nonzero energy; the chemistry contract must define which bonds do.

### D4 — Numerical tolerance versus physical dissipation

**Decision:** Separate numerical comparison tolerance from physical dissipation (Option A).

**Implementation implications:** Epsilon comparisons may account for representation-level noise, but discarded stored energy cannot silently disappear. Any actual truncation/loss must be accounted for under an explicitly defined physical sink or avoided. Establish tolerances and a global residual budget from tests and measured long-run behavior, not arbitrary constants.

### D5 — Sunlight/world-tone computation

**Decision:** Use the fast first-hit/single-sweep approximation if it passes comparison against an exact small-world reference (Option A).

**Implementation implications:** Preserve source-energy budgeting, occlusion/shadow semantics, and the agreed onward frequency-shifted propagation model (not mirror-ray reflection). Benchmark representative workloads, measure geometry and energy-allocation error, and document an acceptable error/performance budget before selecting final thresholds. Keep the current live capture path intact unless a separately approved change is justified by audit evidence.

### D6 — Legacy persistence policy

**Decision:** Reject all pre-unification saves and caches rather than migrating or implicitly accepting them (Option C).

**Implementation implications:** Define an explicit unification-version boundary. Unsupported older saves/caches must fail with an actionable diagnostic; they must not be loaded with defaults that invent missing energy/material history. Fresh saves and caches must include the required version/schema and chemistry-equation/parameter identity. This policy applies to persisted simulation saves and chemistry caches; Bob geometry-library data must be separately assessed and only rejected if it is part of the affected legacy format, not assumed to be equivalent without evidence.

### D7 — Units and quantity semantics

**Decision:** Adopt the recommended documented, internally consistent simulation-unit contract (Option A). Keep reactivity distinct from harmonic response unless a single defensible equation proves they are the same quantity; use stronger typed representations incrementally where they prevent unit mixing.

**Implementation implications:** Explicitly define energy, work, heat/dissipation, resonance/signal quantities, material amount, and conversions between them. Keep immutable resource potential distinct from usable/stored energy. Do not impose SI calibration without a model that supports it.

### D8 — Atomic constituents and physical material representation

**Decision:** Base resources are indivisible. There are no fractional atoms, and base resources may never be split. If the current chemistry mathematics requires fractional base atoms or breaking a base resource into smaller base-resource units, fix the equations or change the model values until valid, clean outcomes are obtained.

**Implementation implications:** Replace any implicit fractional-atom semantics with explicit validation/invariants. Fractional numerical intermediates may exist only where they do not represent fractional physical atoms and where final outcomes conserve integer base-resource counts. The material inventory must count actual constituents across all holders and prove that reactions, composites, death, persistence, and rollback neither create nor destroy them. Fluid volume/deformation rules do not authorize splitting a base resource atom.

### D9 — Tick ordering and multi-tick operations

**Decision:** Define explicit deterministic tick phases and ordering (Option A), while allowing some operations to span multiple ticks to reduce per-tick computation.

**Implementation implications:** A multi-tick operation must have explicit state, ownership/reservation rules, progress semantics, and atomic final commit. It must not duplicate or lose energy/material while pending. Define stale/cancelled operation behavior, deterministic tie-breaking, and the exact same-tick biological eligibility boundary when core breach occurs. Death ends biological activity, not physical existence; valid in-flight physical operations resolve according to the declared contract.

### D10 — Resonance overload

**Decision:** Resonance overload must destabilize bonds; it is not simply a rejected deposit or an automatically dissipated excess.

**Implementation implications:** Specify which bonds are eligible, how overload magnitude selects or affects destabilization, how BREAK is scheduled/settled, and how stored bond energy and resonance energy are reconciled without double release. If destabilization spans ticks, its pending state must be deterministic, cancellable/rollback-safe, and conserve energy/material. Do not invent an arbitrary cap or dissipation rule as a substitute for overload behavior.

### D11 — Conservation diagnostics

**Decision:** Strict tests should fail on invariant violations; exploratory runtime diagnostics should report clearly labelled residuals without silently repairing state (Option A).

**Implementation implications:** Diagnostics remain observational and cannot become an alternate accounting authority. Fatal runtime policy may be tightened later based on measured residual tolerance and operational needs.

### D12 — Post-death chemistry

**Decision:** Keep the legacy decomposition/recycling helper out of normal production death processing; post-death physical changes proceed through ordinary chemistry (Option A).

**Implementation implications:** Death changes biological eligibility only. Dead organisms and their constituents remain in the world. Do not enable a separate recycling law without explicit future approval.

### D13 — Chemistry cache validity

**Decision:** Version cache identity by the chemistry equations and parameters (Option A), but reject pre-unification caches under D6.

**Implementation implications:** A cache hit is valid only for matching canonical resource identity and matching chemistry-equation/parameter version. Invalid or stale cache data must not be used to authorize a reaction threshold or physical outcome.

### D14 — Performance envelope

**Decision:** Establish a representative workload and benchmark repeatably before choosing performance/error thresholds (Option A).

**Implementation implications:** Measure runtime, memory, cache growth, and world-tone geometric/energy error. Use exact small-world behavior as an oracle where practical. Avoid unbounded all-pairs work, per-object sunlight ray tracing, or whole-world cloning per tick unless evidence justifies the cost.

### D15 — Test baseline

**Decision:** Preserve the exact baseline and compare named failures where available (Option A).

**Implementation implications:** Keep the recorded main SHA and 563 passed / 28 failed / 1 ignored result as historical baseline evidence; do not call it green. Extract named failures and compare future runs by identity, not only aggregate count. Do not pull all known baseline failures into this chemistry change unless a failure blocks a required invariant or the owner separately authorizes it.

### D16 — Physical attachment, committed structure, and resonance eligibility

**Decision (owner-confirmed 2026-10-22):** Reproductive status is not a special physical-material designator. Physical structure and committed reproductive structure are the same physical structure. Do not maintain a separate authoritative `developing_structure` graph. Any unit structurally attached to the organism is part of its aggregate physical structure and counts as a receiver for that organism's resonance. Only material still held in storage is excluded from the owner's resonance capture.

**Implementation implications:**
- Classify by actual physical state/attachment, not by a reproductive-material label. A unit attached to the organism is ordinary structure even if it was introduced during reproduction; it appears once in geometry, chemistry, structural-energy inventory, and resonance receiver mapping.
- Eliminate `ReproductiveConstruction.developing_structure` as persistent/current construction state, not merely rename or demote it. Keep only necessary nonphysical construction/core/progress metadata and explicitly justified accounting sub-holders. A transient trial graph is permissible inside an atomic candidate calculation, but it must not become a second owned, serialized, moved, sensed, or inventoried physical structure.
- Stored, unattached material remains a separate holder and does not credit the owner's resonance reservoir, although it may remain a physical occluder/receiver for onward signal propagation.
- Map all attached surfaces to the owner's one resonance reservoir, subject to the same living/biological eligibility and energy-settlement rules as other attached structure. Do not double-count attached units or structural energy.
- Update aggregate graph movement, collision/occlusion, chemistry reachability, cavity/core identity, caches, material/energy inventory oracles, and diagnostic JSON together. Preserve distinct genome-core identities as references over the shared physical graph; core identity is metadata, not a material tag.
- A distinct reserve-energy sub-holder may remain only if its accounting purpose and transfers are explicit; it must not imply a second physical graph or second organism before division.

This decision supersedes any earlier open question about whether attached reproductive structure participates in owner resonance and upgrades the one-graph direction in Sections 17.70 and 17.77 from recommendation to approved design direction. It does not close the remaining transaction-failure, multi-core cavity, diagnostic-contract, or conservation gates.

### Updated dependency order

1. Freeze the exact-head baseline and extract named failures where possible.
2. Align the canonical resource vocabulary to Oxygen and define the unification version boundary.
3. Define units, initial-source accounting, indivisible constituent accounting, bond-energy ownership, and numerical residual policy.
4. Build independent test-only energy and integer material-inventory oracles, including the tick-zero source transaction.
5. Define and test atomic COMBINE/BREAK/overload transactions, including resonance-triggered bond destabilization.
6. Specify deterministic tick phases and multi-tick operation ownership/commit/cancellation behavior.
7. Add fresh-format persistence/cache validation; reject pre-unification saves/caches explicitly.
8. Validate post-death ordinary chemistry and world-tone source/settlement against the same inventory oracles.
9. Benchmark the fast world-tone approximation against exact small-world references and set evidence-based error/performance thresholds.
10. Run cross-domain failure-injection, long-run numerical, persistence, and integration tests; update README/spec only when the implementation supports the claims.

**Implementation gate:** These owner decisions establish design direction. They do not authorize silently changing unrelated code or PR #196 constructor/Bob behavior. Begin with the smallest test-only inventory oracle and tick-zero source-accounting test, then make one audited production change at a time.


---

## 15. Follow-up audit — existing answers in README and main source (2026-10-22)

**Scope:** Read-only review of `README.md` and the chemistry/energy/material/world-tone/tick implementation on exact `main` SHA `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5`. This supplements A0–A21 by separating (a) decisions or mechanisms already present, (b) partial answers needing a contract or independent proof, and (c) unresolved gaps. It does not change runtime code or tests.

### 15.1 Important README/source mismatch

- The README says PR #203 is still a draft and cites PR #202's merged SHA `8c84978cd003738d94a74eef9114b711689c3804` as the integration baseline. The reviewed `main` SHA is instead `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5`, and A0 records PR #203 as merged at that main head. README's current-state block is stale relative to the reviewed source.
- The README says live world-tone capture stays disabled pending verification. The reviewed source constructs a default world-tone cycle in `Simulation::new`, has a live settlement path in `WorldToneSnapshot::settle_living_organism_inputs`, and includes an integration test `live_world_tone_capture_settles_and_reconciles_world_energy`. Thus the source path is active in the reviewed tree; what remains unproven is complete correctness/scale, not whether a capture path exists. Do not change the runtime gate as part of this documentation audit.
- The README correctly warns that whole-world energy conservation across every storage, reaction, and lifecycle path, and complete self-maintaining organism viability, are not proven.

### 15.2 Questions with existing answers or strong partial answers

| Design question | What README/source already establishes | Status / remaining gap |
|---|---|---|
| What is the basic chemistry scale? | `src/chemistry/chemistry.rs` defines (D_{max}=13), (k=1/13), contact radius (1), maximum force (1), characteristic energy distance (1), explicit normalized-chemistry-to-energy conversion, and 10% per-tick reaction-accumulator dissipation. The interaction potential is bounded and symmetric; force is documented as distinct from energy. | **Partially answered.** These constants and primitive equations exist. The full dimensional contract and how every caller interprets reaction accumulation, work, and dissipated energy are not yet unified. |
| How should energy transactions balance? | `src/chemistry/energy_ledger.rs` defines shared transaction shapes, source input, usable/structural changes, work, heat, and a world-tone allocation check. The inventory equation includes changes in usable, resonant, and structural stores plus external input, releases, work, heat, and escaped tone. | **Partially answered.** The ledger algebra and local checks exist. They do not prove that every live holder and every physical material is included independently or updated exactly once. |
| Is bond energy stored on realized bonds? | COMBINE computes a bond-energy value and assigns it to a realized bond; structures store `bond_energy`. BREAK uses the removed bond's stored energy first for disruption work and charges any shortfall to usable energy. Structural energy snapshots sum realized bond energy in organisms and decomposing structures. | **Partially answered.** The value has a structural owner and transaction accounting exists. The model still needs a complete definition of which bonds carry energy, stable identity/ownership across all pending and persisted states, and proof against duplicate release/retry/rollback. |
| What does reaction accumulation mean? | The chemistry comments explicitly call it an activation/trigger quantity rather than an energy reservoir. Chemical BREAK pays disruption work from stored bond energy first and usable energy for any deficit. | **Mostly answered at the conceptual level.** Audit every caller and ledger settlement to ensure no path treats the trigger as a second spendable energy store. |
| What is the broad sunlight source model? | `src/simulation/world_tone.rs` defines 440 Hz, default intensity 1–10 energy per tick per exposed width, a 2,400-tick period, and precomputed samples (bounded at 65,536). The source cycle is deterministic and directional. The snapshot builder uses physically realized geometry, including environmental and stored physical material; the live path allocates capture, heat, and escape through ledger settlement. | **Partially answered.** Source parameters, precomputation, geometric exposure, and settlement paths exist. The agreed fast first-hit approximation still needs exact-small-world comparison, duplicate-claim/occlusion validation, and representative performance/error measurements. |
| Does sunlight use literal mirror reflection? | README and world-tone design comments say no: onward propagation may change frequency composition; it is not mirror-ray tracing. | **Answered as a design rule.** Confirm every implementation path follows it; do not infer that propagation accounting is globally complete. |
| What is the broad death rule? | README says decomposition is chemistry after death. Simulation comments state a breached organism remains physical and is not eligible for new biological actions; existing in-flight physical/chemical transactions are retained rather than dropped. | **Partially answered.** Biological eligibility versus physical existence is established. The coexistence of `decomposing_bodies` and `process_decomposing_bodies` requires a call-path audit to prove the legacy helper is excluded from ordinary death processing and identify any remaining legitimate legacy use. |
| Can operations span multiple ticks? | `ActiveTransformation` has duration/remaining ticks, staged/prepared transaction state, and COMBINE is prepared/committed on its completion tick to avoid committing stale geometry. Simulation tick processing advances pending transformations. | **Partially answered.** A working pattern exists, but there is no complete documented deterministic phase table, reservation contract, or exhaustive cancellation/stale/retry/death-boundary policy. |
| Does resonance overload destabilize bonds? | `resolve_resonance_overload_break` compares retained resonant energy with total structural bond energy and can initiate physical BREAK; comments say resonance/chemistry is the structural-damage authority. | **Partially answered.** There is a concrete mechanism and threshold. Confirm selection/progression semantics, multi-bond behavior, same-tick determinism, and all energy/resonance settlement paths. |
| Are reactivity and chemical position the same? | The chemistry model uses chemical position for interaction; harmonics documents reactivity as the existing dissipative/nonlinear response scale. | **Answered as currently implemented: they are distinct inputs.** Their simulation-unit interpretation and scientific rationale still need documentation; do not merge them merely to reduce the number of properties. |
| Is there any cache versioning? | The chemistry library key has a schema version (the inspected reaction-cache tests expect schema version 3). COMBINE recipe/compatibility caches also have explicit identity structures. | **Partially answered.** Schema/interface versioning exists in places. No evidence found in this review that every chemistry cache identity fingerprints the actual equations and parameters. Inventory all cache types and prove the fingerprint is enforced before reuse. |
| Is there a numerical tolerance? | Source uses explicit tolerances in several local checks (for example (10^{-9}), (10^{-12})); physical reaction-accumulator dissipation is separately defined as 10% per tick. | **Partially answered.** There are local numeric tolerances and a physical dissipation constant, but no single documented tolerance policy or measured global residual budget. Local epsilon checks must not silently erase stored physical energy. |
| Is there an energy inventory test? | `EnergyInventorySnapshot` and tests check tick-zero and ordinary-tick energy balance, and a live world-tone capture integration test checks a balanced snapshot. | **Partially answered.** This is useful evidence, not the independent oracle required by this plan: snapshot terms draw from ledger totals and held-energy totals, so a shared omission in accounting may evade detection. No equivalent independent integer constituent inventory was identified. |

### 15.3 Questions the reviewed source does not yet answer

1. **Explicit tick-zero source transaction:** `Simulation::new` initializes initial usable/resonant/structural inventory baselines from the constructed organism. The reviewed initialization does not record initial world energy as a one-time external-source transaction. D2 settles the required design; implementation and exactly-once verification remain open.
2. **Indivisible base resources:** `Material` stores `(resource_name, f64 amount)`; `Material::is_valid` requires finite positive amounts but does not require integer base-resource counts. D8 settles that base atoms cannot be fractional or split; implementation must establish how numerical intermediates and fluid volume obey that invariant.
3. **Catalog fail-closed behavior:** `fresh_energy` returns zero for an unknown resource name, and material validity alone does not establish that every name belongs to the canonical catalog. A malformed or stale identity can therefore appear as zero energy rather than a clear catalog error. Define and test validation at load/restore/transaction boundaries.
4. **Canonical Oxygen identity across persisted data:** the default catalog contains Oxygen and D1 confirms it as canonical. Source/fixtures/docs and saved/cache keys still need a full trace to identify stale Methane assumptions and implement the explicit post-unification version rejection.
5. **Independent matter conservation:** no independent whole-world inventory of integer base constituents was identified in the reviewed paths. The existing energy snapshot does not solve this.
6. **Global atomicity and failure injection:** local staged transactions and rollback-oriented tests exist, but the review did not establish that every COMBINE/BREAK, external-source claim, transfer, persistence operation, and multi-tick cancellation is atomic across geometry, energy, resonance, and material ownership.
7. **Persistence boundary:** serde defaults exist on some evolving fields, but this review did not find proof of a single mandatory unification version that rejects all pre-unification saves/caches with actionable errors. D6 settles rejection rather than migration; the mandatory version boundary and actionable errors remain to be implemented.
8. **Benchmark/error acceptance criteria:** source sample precomputation is bounded, but no measured representative-world runtime/memory/cache-growth results or accepted geometric/energy error bounds were established by this source review.
9. **Baseline identity list:** README's older PR #202 run records a stable set of 28 failures, while the reviewed main SHA is later. Extract named failing tests from the exact reviewed-head run before claiming identity-level equivalence at that SHA.

### 15.4 Evidence reviewed

- [README.md at reviewed main SHA](https://github.com/coringilbert16-cmyk/EvoSim-/blob/cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5/README.md)
- [Primitive chemistry equations](https://github.com/coringilbert16-cmyk/EvoSim-/blob/cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5/src/chemistry/chemistry.rs)
- [Energy transaction ledger](https://github.com/coringilbert16-cmyk/EvoSim-/blob/cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5/src/chemistry/energy_ledger.rs)
- [Resource/material representation](https://github.com/coringilbert16-cmyk/EvoSim-/blob/cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5/src/materials/resources.rs)
- [COMBINE runtime energy transaction](https://github.com/coringilbert16-cmyk/EvoSim-/blob/cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5/src/chemistry/combine_runtime.rs)
- [BREAK, bond yield, and resonance overload](https://github.com/coringilbert16-cmyk/EvoSim-/blob/cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5/src/chemistry/transformation.rs)
- [Resonance reservoir and harmonic response](https://github.com/coringilbert16-cmyk/EvoSim-/blob/cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5/src/perception/harmonics.rs)
- [World-tone source, precomputation, and settlement](https://github.com/coringilbert16-cmyk/EvoSim-/blob/cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5/src/simulation/world_tone.rs)
- [Live world-tone geometry snapshot and settlement](https://github.com/coringilbert16-cmyk/EvoSim-/blob/cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5/src/simulation/world_tone_snapshot.rs)
- [Simulation state and pending transformations](https://github.com/coringilbert16-cmyk/EvoSim-/blob/cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5/src/simulation/state.rs)
- [Simulation initialization, tick processing, and energy snapshot](https://github.com/coringilbert16-cmyk/EvoSim-/blob/cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5/src/simulation/simulation.rs)

### 15.5 Updated resume point

**Current task remains documentation-only.** The final death, persistence/cache, and material-ownership source-path checks are recorded in Section 16. The next step is to turn those findings and the existing A0–A21 register into the formal, dependency-ordered implementation plan in Section 17. D1–D15 stand; do not repeat settled questions. No production code or tests may change in PR #204.


---

## 16. Final handoff closure — source-path audits (2026-10-22)

**Scope and evidence boundary:** read-only inspection of source at audited `main` SHA `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5`. This closes the three remaining source-path questions in Section 15.5. It does not prove whole-world conservation and does not authorize code/test changes. Source inspection is distinguished from runtime/test evidence throughout.

### 16.1 Death and post-death call path — verified at source level; accounting still a gap

**Files/functions inspected**

- [`src/simulation/simulation.rs::Simulation::step`](https://github.com/coringilbert16-cmyk/EvoSim-/blob/cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5/src/simulation/simulation.rs#L620-L880)
- [`src/simulation/simulation.rs::process_decomposing_bodies`](https://github.com/coringilbert16-cmyk/EvoSim-/blob/cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5/src/simulation/simulation.rs#L545-L615)
- [`src/chemistry/recycling.rs::recycle_dead_organism`](https://github.com/coringilbert16-cmyk/EvoSim-/blob/cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5/src/chemistry/recycling.rs#L1-L55)
- [`src/chemistry/chemical_reaction.rs::accumulate/resolve`](https://github.com/coringilbert16-cmyk/EvoSim-/blob/cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5/src/chemistry/chemical_reaction.rs)
- [`src/simulation/state.rs::Simulation`](https://github.com/coringilbert16-cmyk/EvoSim-/blob/cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5/src/simulation/state.rs#L465-L510)

**Findings**

1. The production tick refreshes the living set from genome-core integrity. Maintenance, harmonics, memory, reproduction, movement, and other biological actions are gated by living status.
2. The ordinary chemistry `accumulate`/`resolve` pass is called for each organism in the simulation loop, including a breached organism. Comments explicitly say physical chemistry continues after biological status ends.
3. `process_decomposing_bodies` is compiled under `#[cfg(test)]`; it is not the production death pathway. Repository search found no production caller of `recycle_dead_organism`; its observed caller is its unit test. `decomposing_bodies` remains in simulation state and in accounting/observation/snapshot support, so the staged representation must not be mistaken for a normally populated production lifecycle.
4. The recycling helper moves stored materials to the field before sequential energy transfers complete. No production defect is asserted because no production caller was found; if the helper is ever reconnected, it must first become an atomic staged transfer with rollback/failure-injection coverage.
5. The energy inventory still includes legacy decomposing-body holders. Keep those holders in the independent inventory if the type remains supported, but do not add a second death chemistry path.

**Status:** runtime architecture verified at source level; global post-death energy/material accounting and helper failure atomicity remain gaps. **Priority:** P1 for lifecycle/accounting correctness; helper hardening is gated on whether it is retained. **Dependencies:** A4–A6, A9–A13, A15–A16, A18, A20.

**Tests already identified:** `recycling_preserves_parent_and_developing_offspring_energy`; test-only finalization test in `simulation.rs`; decomposition/phase-authority tests. These exercise helpers, not normal production death. No new tests were run or created during this audit.

### 16.2 Persistence and cache identities — source-path inventory complete; version enforcement gap

**Files/functions inspected**

- [`src/simulation/state.rs`](https://github.com/coringilbert16-cmyk/EvoSim-/blob/cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5/src/simulation/state.rs)
- [`src/simulation/runtime.rs::RuntimeState`](https://github.com/coringilbert16-cmyk/EvoSim-/blob/cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5/src/simulation/runtime.rs#L1-L70)
- [`src/chemistry/chemistry_library.rs::ChemistryLibrary::open`](https://github.com/coringilbert16-cmyk/EvoSim-/blob/cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5/src/chemistry/chemistry_library.rs#L13-L285)
- [`src/bob/library.rs`](https://github.com/coringilbert16-cmyk/EvoSim-/blob/cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5/src/bob/library.rs)

**Findings**

1. No durable whole-`Simulation` save/load entry point was found in the inspected source. `Simulation` is `Clone`, not serialized as a whole; `RuntimeState` keeps a bounded history of in-memory clones (capacity 30) and restores from that history. Individual state/transaction/material types derive serde, but this is not itself proof of a persistent simulation save format. Therefore there is no existing durable simulation-save loader identified here at which to reject legacy simulation saves. Any future durable save format must be born with the unified version marker and reject older/unversioned state; do not invent a migration path.
2. `ChemistryLibrary` is a real persistent cache: `chemistry.jsonl` plus `manifest.json`, opened through `open_default_library()`. `CHEMISTRY_LIBRARY_SCHEMA_VERSION` is 3, but `ChemistryRecord::is_valid` and manifest validation accept versions 2 and 3. This conflicts with D6's reject-pre-unification-cache rule.
3. `ChemistryKey` identifies canonicalized material names, interface class/signature, and schema version. No fingerprint for the equations/parameters that produced `static_potential` was found. D13 is not satisfied merely by the existing schema number.
4. Bob's persistent geometry library has a separate schema/key contract and stores geometry formations, resource names, and formation signatures. It is not the same thing as the chemistry-potential cache. Audit its resource/catalog signature compatibility under D1; invalidate/version it only when a changed resource identity or geometry input can change the stored geometric result. Do not invalidate it merely because a chemistry equation changes if the cached result is geometry-only.
5. Current chemistry-cache loading already rejects malformed records, duplicate keys, and manifest count mismatch in the inspected source; these protections should be retained while tightening the version/equation identity contract. The source also serializes commands/responses for the local process protocol; that is not simulation-state persistence.

**Status:** current persistence/cache entry points mapped at source level; D6/D13 enforcement remains a gap. **Priority:** P1 before changing chemistry equations or trusting persistent cache hits. **Dependencies:** A1–A2, A13–A14, A20.

**Tests already identified:** chemistry-library schema/key tests, duplicate-key/manifest-integrity tests, cache-hit/miss key tests, Bob library load/signature tests, in-memory runtime history/restore tests. Their existence does not prove a unified cache-version policy. No tests were run or created during this audit.

### 16.3 Physical-material ownership and indivisible constituents — source-path inventory complete; rule enforcement gap

**Files/functions inspected**

- [`src/materials/resources.rs::Material::is_valid/take/combine_materials/merge_parts`](https://github.com/coringilbert16-cmyk/EvoSim-/blob/cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5/src/materials/resources.rs#L300-L470)
- [`src/materials/material_transfer.rs::take_whole_unstructured`](https://github.com/coringilbert16-cmyk/EvoSim-/blob/cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5/src/materials/material_transfer.rs#L1-L100)
- [`src/materials/material_storage.rs::MaterialStorage`](https://github.com/coringilbert16-cmyk/EvoSim-/blob/cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5/src/materials/material_storage.rs#L1-L120)
- [`src/materials/physical_material.rs::PhysicalMaterial::realized`](https://github.com/coringilbert16-cmyk/EvoSim-/blob/cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5/src/materials/physical_material.rs#L1-L110)
- [`src/materials/material_restoration.rs::restore_material`](https://github.com/coringilbert16-cmyk/EvoSim-/blob/cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5/src/materials/material_restoration.rs#L1-L90)
- [`src/simulation/environment.rs`](https://github.com/coringilbert16-cmyk/EvoSim-/blob/cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5/src/simulation/environment.rs)

**Findings**

1. `Material::is_valid` requires nonempty names and finite positive amounts and validates internal-bond indices/duplicates. It does not require integer amounts and does not prove resource names exist in the active catalog. `fresh_energy` and mass/property aggregation have unknown-name fallback behavior, including zero contribution; D1's canonical Oxygen choice is not enough without fail-closed identity validation.
2. `Material::take` proportionally splits each part by a floating-point fraction. The ecological `take_whole_unstructured` helper rounds the *total* aggregate transfer to whole units but allocates those units across constituent species by fractional shares and can leave fractional per-resource residuals. Its existing test explicitly transfers from a 3.5-unit aggregate and leaves 2.5. That behavior conflicts with D8 if each base-resource atom is indivisible; the test and representation must be reconciled, not preserved as the new invariant.
3. Later physical boundaries are stricter: `MaterialStorage::is_discrete` checks integer amounts, `PhysicalMaterial::realized` requires one unit per physical constituent, and `restore_material` rejects fractional or non-unit constituents. This means the invariant is inconsistently enforced at different boundaries, not absent everywhere.
4. Physical material is stored as a constituent list plus placements and internal connection endpoints. `combine_materials`/environment `merge_parts` aggregate names and amounts for ecological stock, while physical storage retains discrete realized objects. The independent inventory must count each actual constituent once across environment aggregates, field physical instances, organism structures, stored material, pending transformations, reproductive construction, and any supported legacy body.
5. Fluid geometry/area is not an atom count. Preserve the approved Water shape/volume behavior in geometry, but do not represent fluid volume as a fractional number of base atoms to bypass D8. Define separate dimensional quantities for constituent count and realized geometry.

**Status:** source paths and the main rule conflict are verified; independent whole-world constituent inventory and a consistent integer ownership boundary remain gaps. **Priority:** P1 because split/merge behavior can violate the settled model rule. **Dependencies:** A1–A2, A3–A6, A9, A11–A16, A18, A20.

**Tests already identified:** `takes_only_whole_units`, `takes_only_available_whole_units_when_request_exceeds_stock`, `transfers_whole_units_from_fractional_aggregate`, `refuses_structured_material`, storage discreteness tests, physical realization/restoration tests, and material-combine tests. The fractional-aggregate test currently documents behavior that must change under D8. No tests were run or created during this audit.

### 16.4 Handoff consistency and restart contract

- **Authoritative design decisions:** Section 14, D1–D15. Historical decision fields in A0–A21 must not be read as new owner questions.
- **Latest source reconciliation:** Section 15. **Final source-path closure:** this section, 16.1–16.3.
- **Authoritative implementation sequence:** Section 17 will replace the earlier provisional sequence in Sections 8 and 13.
- **Baseline:** `main` SHA `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5`; Rust workflow run `39705628427`; recorded 563 passed, 28 failed, 1 ignored. The commit message attests the 28 failures match the accepted baseline; named failures were not independently extracted in this pass.
- **Change scope:** this PR remains information-only and draft. No production source, tests, README, geometry/chemistry cache data, or runtime behavior has changed here.
- **Current resume point:** owner approval for Section 17 is recorded. Continue implementation only on PR #205, in phase order, with one focused audit before each change; keep PR #204 documentation-only and PR #196 constructor/Bob work separate. First reconcile the latest exact-head workflow results below, then continue Phase 3/4 verification and the independent material/energy inventory gates.

**Handoff status:** the A0–A21 audit register and owner-approved Section 17 plan are recorded. Remaining blockers are implementation and verification evidence, not missing owner decisions. PR #204 stays information-only and must track the exact PR #205 head, workflow results, known baseline failures, and next actions.


---

## 17. Formal chemistry-unification implementation plan — authoritative

**Purpose:** unify chemistry as one deterministic, conservation-auditable physical process. This plan applies owner decisions D1–D15 and the findings in A0–A21 and Section 16. It does not reopen settled choices. A decision may be settled while implementation remains absent or unverified.

**Authorization boundary:** the owner approved the scientific invariants, 12-phase implementation order, and verification gates. PR #204 remains information-only and draft; implementation and tests belong exclusively on PR #205. Keep PR #196 constructor/Bob work separate. Prefer cohesive commits in the smallest useful implementation PR; do not create separate PRs unless a real dependency or rollback boundary justifies them.

### 17.1 Target contract — invariants every phase must preserve

1. **Resource identity and units (D1, D7).** Oxygen is canonical for resource ID 6. Resource names/IDs must resolve through the authoritative catalog. Unknown/stale names fail visibly at load, restore, and transaction boundaries; they must not quietly contribute zero energy/mass or act as aliases. Reactivity and chemical position remain distinct quantities. Define units/meaning for mass, potential energy, interaction potential, force, work, heat, resonant energy, bond energy, and geometry/volume.
2. **Indivisible base constituents (D8).** A base atom is an integer count and cannot be fractionally extracted, split, or silently discarded. Geometry—especially fluid shape/area—is a separate quantity and must not be encoded as fractional atom count. Ecological aggregation may group integer counts by resource but may not fractionalize per-resource inventory.
3. **One energy accounting equation (D2–D4, D11).** Every holder and flow is counted exactly once. Initialization is a one-time external-source transaction, not an implicit free baseline also counted as external input. Internal transfers cancel at world scale. Chemical release, source input, work, heat, escaped signal energy, and changes in usable/resonant/structural stores have distinct meanings. Numerical comparison tolerance is not physical dissipation; physical energy may not disappear through epsilon pruning.
4. **Explicit bond-energy ownership (D3, D10, D12).** A realized bond that stores energy has explicit ownership/identity. Formation records energy once; BREAK/overload settles or releases it at most once; retry, cancellation, rollback, restore, or repeated processing cannot duplicate settlement. Do not infer stored energy for a bond class that has no energy record. Resonance overload must destabilize physical bonds through the shared BREAK mechanism, not merely reject a deposit or automatically dissipate it.
5. **Deterministic transactions and tick phases (D9).** Define an authoritative phase table. Operations may span multiple ticks, but each phase states what is reserved, prepared, committed, restored, or cancelled. Failed/stale operations leave geometry, material ownership, energy, and resonance unchanged. Source claims and bond releases are exactly once.
6. **Ordinary post-death chemistry (D12).** Death removes biological eligibility, not the physical body. Existing physical material remains and continues through the ordinary chemistry pass. Do not enable a parallel legacy decomposition law.
7. **Versioned persistent knowledge (D6, D13).** Reject all pre-unification chemistry caches; do not migrate them. Cache identity includes the chemistry equation/parameter fingerprint and the record/interface schema. A future durable simulation-save format must start with a mandatory unified version and reject older/unversioned data; the audit found no existing durable whole-simulation save/load path. Bob geometry records are a separate cache: invalidate them for resource/catalog/geometry changes that alter stored geometry, not merely because an energy equation changes.
8. **Measured approximations and strict evidence (D5, D11, D14, D15).** Use the fast first-hit/single-sweep sunlight approximation only if it passes exact-small-world comparison. Strict tests fail on conservation violations; exploratory diagnostics report labelled residuals without silent repair. Set runtime/error thresholds only after defining representative workloads. Compare named failures where available and preserve the A0 evidence limitation if individual failures cannot be extracted.

### 17.2 Dependency-ordered phases

| Phase | Scope and implementation work | Required evidence / exit gate | Main audit dependencies |
|---|---|---|---|
| **0 — Freeze baseline and scope** | Preserve the exact audited base SHA and CI result. Re-check current base/head before implementation; collect named failure identities if available. Inventory source, generated data, cache manifests, and test commands. Do not clean unrelated Clippy/dead-code issues unless a direct chemistry dependency is proven. | Baseline SHA, workflow run, counts, and evidence limitations recorded; no false “green” claim. Compare each new failure against the accepted baseline. | A0, A21 |
| **1 — Publish contract and dependency map** | Convert 17.1 into a compact units/resource/energy-flow table and call-site matrix. Record formulas/constants from `chemistry.rs`; document reaction accumulation as activation/trigger quantity, not energy reservoir; retain separate reactivity and chemical-position meanings. Identify every holder/source/sink before behavior changes. | Every quantity has units, range, owner, source/sink, and caller interpretation. Unjustified physical meaning remains explicitly unresolved—not silently renamed or rescaled. | A1–A2, A5–A9, A12, A17–A18 |
| **2 — Build independent test-only inventory oracles** | Add a whole-world energy oracle that independently enumerates usable, resonant, structural/bond, pending-transaction, developing-offspring, source, work, heat, and escaped-signal state. Add an integer constituent oracle covering field aggregates/physical instances, organism structure/storage, transformations, reproduction, and any retained legacy holder. Do not reuse ledger totals or production inventory helpers as the oracle. | Tick-zero, no-op tick, source input, transfer, COMBINE, BREAK, movement/work, reproduction, death, and post-death scenarios reconcile. Deliberately inject duplicate source claims, duplicate bond release, missing holders, and fractional constituents; strict tests fail. | A3–A6, A9–A12, A16, A18, A20 |
| **3 — Enforce resource identity and indivisible ownership** | Centralize catalog membership/ID/name validation; fail closed for unknown resources. Enforce integer base-constituent counts at authoritative ownership boundaries. Replace fractional proportional extraction (`Material::take`) for physical transfer and reconcile `take_whole_unstructured` so it cannot create fractional per-species counts. Revise tests that bless 3.5-unit stocks/fractional residuals. Keep atom count separate from fluid geometry/area. Trace Oxygen/Methane references across code, fixtures, docs, checked-in data, and Bob/chemistry keys. | Independent material oracle passes create, merge, transfer, storage, restore, COMBINE/BREAK, and field deposits. Invalid names/fractional atoms reject without mutation. Geometry tests preserve approved shapes and fluid deformation without changing constituent count. | A1, A11, A13–A16, A18, A20; D1, D8 |
| **4 — Establish cache-version boundary before equation changes** | At the frozen baseline, the chemistry library accepted schema 2/3. PR #205 now uses schema 4 and an equation/catalog-version fingerprint, rejecting prior schemas. Close the phase by testing catalog-property invalidation plus append/manifest restart and corruption paths. Audit Bob geometry records for resource/catalog signature dependence; change their version only when geometric meaning can change. Do not invent a simulation-save migration path: current history is in-memory. Any future durable save format starts versioned and rejects old/unversioned state. | Old chemistry cache fails with an actionable incompatibility error; no silent migration or stale hit. Fresh cache round-trips; changing an equation/parameter invalidates affected values; cache hits agree with authoritative recomputation. Geometry invalidation is scoped to geometry/catalog changes. | A1–A2, A13–A14, A21; D1, D6, D13 |
| **5 — Make the ledger authoritative and account for initialization** | Define one balance equation and transaction shape for external input, usable/resonant/structural changes, work, heat, and escaped signal. Implement D2 as a one-time initial-energy source event with a clear empty/pre-seed boundary; never count initialized holders both as initial baseline and external input. Separate numerical tolerance from physical dissipation; define finite/range validation and overflow behavior. Keep source budgets distinct from internally released potential. | Independent oracle balances initialization and ordinary ticks; initialization/restore/replay cannot double-credit. Failed transactions mutate neither holder nor cumulative totals. No energy is erased by tolerance-only pruning. | A2, A5–A6, A9, A12, A18, A20; D2, D4, D11 |
| **6 — Unify COMBINE, BREAK, bond ownership, and overload** | Trace live/staged COMBINE/BREAK variants, including fluid, stored physical material, environmental acquisition, and restoration. Establish validate → stage → settle → commit/rollback. Give each energy-bearing realized bond explicit ownership; settle once on formation and release/use once on BREAK. Stored internal physical-material bonds currently carry endpoints but no stored `bond_energy`; do not manufacture energy from them. Route overload through physical BREAK and define deterministic bond selection/progression. | Success and failure injection preserve both independent inventories. Repeated/retried/stale BREAK cannot release twice; rejected COMBINE leaves no partial geometry/debit; overload physically destabilizes bonds under the same accounting contract. | A3–A7, A15–A16, A18, A20; D3, D10 |
| **7 — Close energy consumers and deterministic tick phases** | Publish the authoritative tick phase table. Audit initial conditions, world tone, movement, maintenance, construction/reproduction transfers, reservoirs, and all internal transfers. Surface ignored transfer results. For multi-tick transformations specify reservations, prepared state, commit tick, rollback, stale geometry, death during operation, and restore/cancel behavior. Preserve in-flight operations after breach unless atomically cancelled and restored. | Seeded replay yields identical event/ledger order. Failure injection at each phase leaves no partial material, energy, resonance, or geometry mutation. Every internal transfer cancels at world scale. | A5–A7, A9, A12, A15, A18, A20; D9, D11 |
| **8 — Unify lifecycle and post-death chemistry** | Keep core breach as loss of biological eligibility. Continue ordinary chemistry on the physical organism after death. Do not call test-only `process_decomposing_bodies` from production ticks. Decide during implementation review whether uncalled `recycle_dead_organism`/`DecomposingBody` should be removed or kept quarantined; never reactivate without atomic material/energy transfer and explicit review. | Tests prove biological maintenance/reproduction/movement stop after breach while physical chemistry continues. Independent inventory accounts for the body and every constituent/energy holder through breach, reactions, and final material fate. | A4–A6, A9–A13, A15–A16, A18, A20; D12 |
| **9 — Validate and optimize world-tone exposure** | Preserve the single rotating off-screen source, frequency/intensity/day cycle, and onward frequency-shifted propagation (not mirror reflection). Compare the fast first-hit/single-sweep path against a slower exact small-world oracle. Validate one source budget, first-hit occlusion, unique projected width, no duplicate receiver claims, damping/capture/heat/escape, and no second source credit downstream. Avoid per-object ray tracing. | Deterministic small-world cases match within a measured error budget. Source energy is never over-allocated; shadows/occlusion behave as specified. Profile representative thousands-object worlds and record time/memory/error before setting thresholds. | A6–A8, A17, A19–A20; D5, D14 |
| **10 — Long-run numerical and diagnostic integrity** | Test non-finite/negative state, accumulator overflow, reservoir capacity/pruning, cumulative counter overflow, quantization, cache-key stability, and repeated-tick drift. Keep numeric tolerance separate from physical sinks. Diagnostics label units, interval/cumulative totals, residuals, actual seed/configuration, and source identity; exploratory runs report residuals but never silently repair state. | Adversarial/long-run tests preserve invariants or fail with actionable diagnostics. Tolerance budget is measured/documented. Diagnostics do not mutate state or mask invariant violations. | A2, A5–A9, A17–A20; D4, D11, D14 |
| **11 — Integrated verification and scientific status** | Run cross-domain scenarios through full ticks and failure paths: initialization, transfer, COMBINE, BREAK, overload, sunlight, movement/work, reproduction, death/post-death, cache rejection/hit/miss, and cancellation/retry. Run format/compile/focused tests/full suite/strict Clippy and representative benchmarks in bounded commands. Compare failures with A0; update README only from verified evidence. | New strict conservation/material tests pass; no unexplained new baseline failures; oracle and benchmark results recorded. The suite may remain baseline-red but is reported accurately. README, plan, and behavior agree. | A0–A21; D11, D14, D15 |

### 17.3 Required independent accounting definitions

**Energy oracle.** Define the world balance from independently enumerated holder state, not cumulative ledger fields alone.

- **Stores:** usable energy, resonant reservoir energy, and explicitly energy-bearing realized bonds.
- **Sources:** one-time initial-energy transaction and later source-backed inputs, including world tone.
- **Sinks/flows:** work, physical heat/dissipation, energy carried out of the modeled world, and explicitly permitted external releases.
- **Internal transfers:** debit and credit the same amount exactly once; they cancel at world scale.
- **Prohibited:** counting a bond release as both external energy and structural release; counting initial holders as both baseline and source input; treating reaction accumulation as stored energy; silently erasing stored energy under numeric epsilon.

**Material oracle.** Count integer constituents by canonical resource identity across every mutually exclusive owner. Physical instances, organism structures, storage, environment aggregates, pending transformations, and reproductive construction are ownership locations—not additive duplicates when an operation temporarily references the same material in a prepared copy. Distinguish an authoritative owner from a trial/prepared clone and check conservation before/after commit or rollback. Check fluid shape/area with geometry invariants separately from integer atom count.

### 17.4 Cache and persistence compatibility rules

- Chemistry cache records created before the unified contract are rejected; no schema-2/schema-3 compatibility read and no silent rewrite.
- New chemistry keys include canonical material identities/composition, interface identity, cache schema, and a stable fingerprint of every equation/parameter affecting the cached potential.
- Cache miss calculation remains authoritative. A cache hit is derived data and does not bypass live geometry, bond, catalog, or transaction validation.
- Retain strict corruption checks (invalid row, duplicate key, manifest mismatch) and add restart/failure-injection tests for record append plus manifest commit.
- No durable whole-simulation save/load path was found. Runtime history is in-memory clones. If durable saves are introduced later, they start at the unified version and reject older/unversioned saves. No migration is implemented.
- Bob geometry data is separate. Revalidate resource names/geometry signatures under D1; invalidate only for resource/catalog/geometry semantics that affect geometry, not unrelated energy-equation changes.

### 17.5 Performance and test discipline

- Keep each CI/test command within the hard five-minute limit. Timeout is incomplete evidence, not success or a fabricated failure count.
- First run focused tests for the changed invariant and rollback paths, then integration coverage, then the full suite and strict Clippy.
- Establish repeatable sparse/dense workloads with thousands of objects, stable seeds, recorded hardware/configuration, and cache-cold/cache-hot measurements before choosing numeric budgets.
- Use exact geometry only for small sunlight reference worlds; use the accepted fast sweep at scale if it meets measured error/performance limits. Do not restore per-object ray tracing as production default.
- Compare named failing tests/assertions to A0 where available. The original identity-extraction limitation has been superseded by the named-test comparison recorded in Section 17.7; retain the exact run/head context when updating that comparison.

### 17.6 Release gate — no phase may be called “unified” without evidence

- [ ] D1–D16 map to code/tests/docs; no settled decision remains an owner question. D16 is documented and owner-approved, but its one-graph representation and resonance receiver mapping are not yet implemented or verified.
- [ ] Independent energy and integer constituent oracles do not reuse production counters as their source of truth.
- [ ] Initialization is an explicit one-time source transaction without duplicate baseline/source counting.
- [ ] Every energy-bearing bond settles exactly once; COMBINE/BREAK/overload rollback is atomic.
- [ ] Unknown resource names and fractional base-atom amounts fail closed; fluid geometry remains independent.
- [ ] Chemistry cache rejects old versions and fingerprints equations/parameters; stale hits cannot change chemistry.
- [ ] Deterministic tick phases and multi-tick operation boundaries are documented and tested.
- [ ] Dead bodies stop biological behavior but continue ordinary chemistry; no parallel decomposition pathway is enabled.
- [ ] World-tone first-hit approximation passes exact-small-world comparison and measured source-budget/error/performance gates.
- [ ] Long-run numerical tolerance and physical dissipation are distinct and independently tested.
- [ ] Exact-head baseline comparison, bounded CI, failure-injection coverage, and representative performance evidence are recorded.
- [ ] README and implementation docs describe only verified behavior.

**Formal plan status:** owner approval received for the scientific invariants, 12-phase order, verification/completion gate, and D16 physical-attachment/resonance classification. D16 remains an implementation and verification obligation, not a claim of completed behavior. This document remains an information-only plan and audit register; implementation and tests are tracked in the separate [PR #205](https://github.com/coringilbert16-cmyk/EvoSim-/pull/205). The implementation base is `main` at `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5`; the recorded baseline is 563 passed, 28 failed, 1 ignored. Head `4f4b2af` reported 581 passed, 28 failed, 1 ignored with the exact 28 baseline failures. Head `25f1df5` fixed the cache-test fixture syntax and formatting; workflow `39818514455` reported 580 passed, 28 failed, 1 ignored in 236.55 seconds. Latest head `0148437` was verified by workflow `39818747953`: 580 passed, 28 failed, 1 ignored in 237.90 seconds; all 28 failure identities matched baseline, the oracle and cache append rollback tests passed, and normalized Clippy messages matched baseline. Whole-world conservation remains unproven.

### 17.7 Implementation progress tracker (reconciled 2026-10-22)

This is the current phase view; detailed dated evidence remains below in Sections 17.8 onward. PR #204 remains documentation-only. Phase status reflects source/test evidence, not just presence of code.

| Phase | Status | Current evidence / remaining gate |
|---|---|---|
| **0 — Freeze baseline and scope** | **Recorded** | Frozen main SHA cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5; workflow 39705628427: 563 passed, 28 failed, 1 ignored. Named baseline failures are available. |
| **1 — Contract and dependency map** | **Source/holder and failure maps recorded; aggregate contract not implementation-ready** | Sections 17.84–17.91 cover mutation paths, D16 impact, cache, receiver and failure rows. Remaining audit/design contracts: single graph/core identity and ID remapping; parent aggregate maintenance/stress/reserve semantics; diagnostic JSON projection; explicit failure-injection and inventory assertions. |
| **2 — Independent inventory oracles** | **Oracles exist; aggregate reconciliation and global proof open** | Current snapshots independently recompute defined material/energy holders, but construction graph/holder categories encode the rejected two-graph model. Reconcile attached units/bonds once, stored material separately, active escrow exactly once, and explicit energy sub-holders; extend rejection, post-death, long-run and cross-domain source/sink coverage. |
| **3 — Resource identity and indivisible material ownership** | **Partial; atomicity gate open** | Whole-count/unknown-resource validation and selected restoration/transfer tests exist. Source-confirmed atomicity gaps remain in stored BREAK, contained acquisition, expulsion, and movement; deterministic rejection injection plus exact physical-instance/inventory assertions are missing. |
| **4 — Chemistry cache version boundary** | **Schema/fingerprint and rollback tests present; phase gate open** | Schema 4, strict manifest/record validation, duplicate-key rejection, and append/manifest rollback/reopen tests exist. Catalog chemical-position dependency is not encoded in `ChemistryKey`; manually versioned fingerprint still lacks a property-change/restart test. |
| **5 — Authoritative energy ledger and initial source** | **Started; global proof open** | InitialCondition and world-tone frame settlement paths exist; graph BREAK/COMBINE structural deltas are tracked. Independent cumulative source/sink provenance, cross-domain conservation, and long-run residual/overflow tests remain open. |
| **6 — COMBINE/BREAK, bond ownership, overload** | **Partial; unified contract not complete** | Graph COMBINE/BREAK have staged patterns, but stored BREAK escrow can be lost on malformed completion or partial destination failure. Prepared COMBINE rollback omits escaped-tone/resonant-held fields captured in its delta; assert zero-delta or centralize symmetric handling. Failure-injection matrix remains open. |
| **7 — Energy consumers and deterministic tick phases** | **Audit mapped; transaction and accounting gates open** | Movement mutates the mover/pushed organisms/field objects before ledger settlement; construction reserve transfer ignores rejection. World-tone stages reservoirs and settles path energy. Work-versus-heat semantics, unabsorbed-throughput telemetry, and deterministic failure/retry invariants remain open. |
| **8 — Lifecycle and post-death chemistry** | **Owner architecture approved; representation/holder reachability open** | D16 requires one parent-owned graph and no second pre-division physical/biological owner. Current construction, finish, movement, inventory, diagnostics and world-tone still use `developing_structure`. Dead primary structure continues ordinary chemistry, while stored/committed holders are not enumerated by that pass; legacy recycler remains unsafe/quarantined. |
| **9 — Sunlight correctness and performance** | **Receiver policy decided; implementation/oracle/performance gates open** | Current snapshot separately maps primary/developing graphs and storage, conflicting with D16. Attached surfaces must credit the living parent's reservoir once; stored material stays capture-ineligible but can occlude/heat. No independent end-to-end small-world oracle or fresh thousand-body benchmark is recorded; multi-core cavity identity remains required. |
| **10 — Long-run numerical integrity** | **Audit findings recorded; integrated phase open** | Bondless reaction accumulation can retain stale keys; world-tone unabsorbed-throughput telemetry is not updated by the live frame path. Overflow, drift, reservoir capacity, cache restart and long-run residual tests remain required. |
| **11 — Integrated verification** | **Open** | Latest recorded exact-head workflow 39833438679 on PR #205 head 12d8079230458da68a8dc3755a1d6a9b41347862: formatting/focused gates passed; full suite 585 passed, 27 failed, 1 ignored in 283.74 seconds, all 27 failing identities matched the frozen 28-failure baseline minus one still-resolved failure. Strict Clippy had 34 application-target and 15 test-target diagnostics with no new/removed normalized identities. Suite remains baseline-red; no performance benchmark was run. |

**Current highest-priority confirmed implementation candidates (after audit closure only):**
1. Make stored BREAK output and its destination commits all-or-nothing; preserve escrow until every piece has a valid owner.
2. Make contained acquisition and expulsion restore or retain the exact source if a destination rejects.
3. Make movement geometry and energy settlement one transaction.
4. Clear obsolete reaction-accumulation keys before returning for a bondless structure.
5. Add independent failure-injection tests and whole-world inventory checks for each fix.

**Current owner decision (Sections 17.70–17.75 supersede the historical blocker above):** attached reproductive structure is ordinary parent structure and may contribute to the living parent’s resonance; stored material is excluded. The policy question is closed. Implementation remains blocked by the need to unify the physical graph safely, preserve parent/second-core identity, revise the independent inventories and persistence model, and finish the other audit templates. Do not treat the decision as proof that receiver mapping is correct.

**Verification caveat:** exact-head test results above do not mean the suite is green, and the baseline comparison does not prove conservation. Phases 2–9 remain incomplete in different respects. Do not merge or mark chemistry unified until the exit gates in Sections 17.2 and 17.6 have evidence.

### 17.8 Latest handoff addendum — 2026-10-22, PR #205 head `55e47a3fff94eb7c3cc79c23b558f5bc5a530ba7`

This addendum supersedes earlier statements above that say the production-path containment inventory test or exact-head verification is still pending.

- **Exact-head workflow:** [39819816103](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39819816103) completed with failure because the repository's full suite remains baseline-red and strict Clippy remains baseline-red.
- **Formatting and focused workflow gates:** passed: formatting, source-file-size guard, COMBINE architecture, Bob compact-storage, empty-library open/reopen, concurrent Bob initialization, focused fluid transaction/three-piece cap, focused fluid coalescence/restoration, and focused chemical BREAK.
- **Full suite:** 581 passed, 27 failed, 1 ignored in 211.52 seconds. The 27 failing test identities were compared against the frozen baseline's 28 named failures; all 27 are baseline identities. The previously baseline-failing test `simulation_tests::integration_tests::contained_physical_material_becomes_storage_without_an_acquire_action` now passes. Do not describe the suite as green.
- **Independent inventory gates:** both `integer_material_inventory_conserves_constituents_across_field_to_storage_transfer` and `integer_material_inventory_conserves_constituents_when_anchor_moves_to_offspring` passed in this run. The former directly exercises the production containment-transfer path and independently recomputes integer constituent totals before/after transfer.
- **Clippy:** failed with 34 application-target and 15 test-target diagnostics; these remain the previously observed baseline diagnostics, not a newly introduced regression. Do not treat strict Clippy as passing.
- **Phase 3 next:** continue one focused audit before each code change. The production-path transfer oracle is now evidenced; remaining gates include rollback/failure paths, physical storage/restore, and decomposition ownership. Keep the phase open.
- **Phase 4:** cache schema/fingerprint rejection, non-empty cache without manifest rejection, manifest-write rollback/reopen, partial-record-append rollback/reopen, and concurrent writer/reopen tests are present. The exact-head focused cache results are represented by the existing workflow gates; do not mark the phase complete until the broader restart/append/manifest failure-injection review is complete.
- **Current next action:** audit and add the next isolated Phase 3 ownership/rollback case, then run the narrow targeted test(s) under the five-minute limit. PR #204 remains information-only; all implementation/test edits stay on PR #205. PR #196 remains out of scope.

### 17.9 In-progress handoff — 2026-10-22, PR #205 head `d07048a85b902494a54197d2ccc3852599df5fd2`

- The exact-head run is [39821254438](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39821254438), currently in progress at the time of this update. Formatting passed after correcting the helper signature that caused the preceding commit's formatting gate to fail.
- Commit `2d059b9` added an independent material inventory assertion to `failed_stored_break_restores_material_to_organism_storage`; commit `d07048a` applies the rustfmt-required helper-signature layout. No production behavior changed in these two commits.
- The immediately prior head `1d276a0`, workflow [39820419232](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39820419232), passed formatting and all focused gates. Full suite: 582 passed, 27 failed, 1 ignored in 238.08 seconds; all 27 failure identities matched the 28 named frozen-baseline failures, and the production containment, storage-restore, and reproductive-anchor inventory tests passed. Strict Clippy remained baseline-red with 34 application-target and 15 test-target diagnostics.
- Do not claim the new failed-BREAK inventory assertion passed until run 39821254438 completes. After it completes, record the exact-head result, named failure comparison, and Clippy result here and in both PR bodies.
- Next Phase 3 action remains an independent inventory/rollback check, then continue the physical ownership and decomposition review. Do not reactivate the legacy decomposition pathway. All test/command timeouts remain five minutes.

### 17.10 Exact-head verification result — 2026-10-22, PR #205 head `d07048a85b902494a54197d2ccc3852599df5fd2`

Workflow [39821254438](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39821254438) is complete.

- Formatting, source-size guard, COMBINE architecture, Bob compact-storage, empty-library open/reopen, concurrent Bob initialization, focused fluid transaction/coalescence/restoration, and focused chemical BREAK steps all passed.
- Full suite: **582 passed, 27 failed, 1 ignored** in 282.01 seconds. Direct comparison of named failing test identities against frozen baseline workflow 39705628427 found **zero new failure identities**; 27 baseline failures remain, and `simulation_tests::integration_tests::contained_physical_material_becomes_storage_without_an_acquire_action` is resolved. The suite remains baseline-red.
- The new `failed_stored_break_restores_material_to_organism_storage` inventory assertion passed in the full suite. It snapshots independently enumerated integer constituent totals across organism structure/storage and environmental aggregate/physical holders before the failed BREAK source is restored, then asserts totals are unchanged afterward.
- Strict Clippy still fails, but normalized diagnostic identities match the frozen baseline exactly: 34 application-target and 15 test-target diagnostics. No new or missing diagnostic identities were found.
- **Phase 3 status:** rollback inventory test added and verified; continue the focused ownership audit. Successful production containment transfer, storage restore, reproductive-anchor transfer, and failed stored-BREAK rollback have independent inventory evidence. Keep Phase 3 open for broader rollback/failure, restore rejection, and decomposition ownership gates.
- **Current next action:** add a negative restoration test proving malformed stored physical material is rejected without mutating the organism structure, storage, or independent material inventory. The restoration implementation already stages changes in a trial structure and commits only after all checks; this test should verify that atomicity. Continue one focused audit before each change and keep every test/command under five minutes.

### 17.11 Current exact-head handoff — 2026-10-22, PR #205 head `3628e73cba225993c343f1915ecb1170bf7069ed`

- Workflow [39822405710](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39822405710) is in progress. Its result is not yet known.
- The new test `integer_material_inventory_preserves_state_when_restore_rejects_invalid_connections` is committed. It stores a valid Carbon–Hydrogen physical compound, corrupts only the test copy's physical connection list, then verifies restoration rejects it while the organism structure, authoritative stored material, and independently recomputed integer material inventory remain unchanged.
- The preceding exact-head result remains head `d07048a`, workflow 39821254438: 582 passed, 27 failed, 1 ignored; all 27 failure identities matched the frozen baseline, one previously failing containment-to-storage test is now passing, and the new failed-BREAK rollback inventory assertion passed. Formatting/focused gates passed; Clippy's 34 application-target and 15 test-target diagnostics matched baseline exactly.
- Do not treat the newly added restoration rejection test as verified until workflow 39822405710 completes. Next actions after verification: update this handoff with the exact-head result, then continue Phase 3 rollback/restore/decomposition ownership gates and Phase 4 cache failure-injection review.

### 17.12 Current exact-head handoff — 2026-10-22, PR #205 head `7205bae16ae70eb0b92588a4d42b32aa1bf725d8`

- Workflow [39822670092](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39822670092) is queued.
- The preceding run [39822405710](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39822405710) did not reach the full suite: the new restoration-rejection test failed to compile because `PhysicalConstituentGraph` does not implement `PartialEq`. The new test now compares serialized structure snapshots instead, which checks the full serialized structure without changing production derives. The prior run's formatting, source-size, and COMBINE architecture steps passed; later steps were skipped after the compile failure. Clippy also could not compile the test target due to that same new assertion plus its known baseline diagnostics.
- Exact-head verification for the negative restoration test is pending on `7205bae`. Do not mark the test passed until workflow 39822670092 completes.
- Previous complete evidence remains head `d07048a`, workflow 39821254438: 582 passed, 27 failed, 1 ignored; no new failure identities versus baseline, with one baseline failure resolved; formatting/focused gates passed; Clippy diagnostic identities exactly matched baseline.

### 17.13 Exact-head update — 2026-10-22, PR #205 head `2ae8d45e2ef5033438c8272786f695b328ca7c47`

This entry supersedes the earlier “workflow queued / exact-head result pending” status for head `7205bae`. The current head contains two focused commits after that run; their new topology-validation regression test has not yet been verified by CI.

- **Previous exact-head workflow:** [39822670092](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39822670092) completed on `7205bae16ae70eb0b92588a4d42b32aa1bf725d8`. Formatting, source-size guard, COMBINE architecture, Bob compact-storage, empty-library reopen, concurrent initialization, fluid transaction/coalescence/restoration, and focused BREAK gates passed.
- **Full suite on that previous head:** 583 passed, 27 failed, 1 ignored in 236.64 seconds. Named failures were extracted from baseline workflow `39705628427` and this run's logs: all 27 current failure identities are in the 28-test baseline set; no new failure identities, and the baseline failure `simulation_tests::integration_tests::contained_physical_material_becomes_storage_without_an_acquire_action` remains resolved. The suite remains baseline-red.
- **Clippy on that previous head:** failed with 34 application-target and 15 test-target diagnostics; diagnostic identities remain consistent with the accepted baseline. Do not call Clippy passing.
- **Focused audit finding:** `restore_material` validated connection count and index bounds, but did not verify that each stored physical connection still represented the corresponding `Material::internal_bonds` pair. A corrupted-but-in-range connection could therefore request a different topology. Commit `6bed90d3b8b3befc7de0123ffc757301dbfe7f2a` now rejects connection/material bond-pair mismatches before staging restoration. This preserves the invariant that stored material's declared topology is authoritative.
- **Regression test:** commit `2ae8d45e2ef5033438c827da1879111d0e9ff6ee` adds `restoration_rejects_connections_that_change_stored_bond_topology_atomically`. It redirects a stored connection from constituents 0–1 to 1–2 while keeping the indices in range, then checks rejection leaves serialized organism structure, authoritative storage, and independent integer constituent inventory unchanged.
- **Current verification:** no workflow/status result for exact head `2ae8d45` was visible at the time of this update. Do not claim either the new test or the current head passes until a run is found and inspected.
- **Next actions:** verify exact-head CI; if the fixture fails, fix only the fixture/contract issue shown by evidence. Then continue the next Phase 3 audit on decomposition ownership or restoration/rollback failure paths. Keep Phase 3 and Phase 4 open; do not activate the legacy decomposition path. All commands/tests remain limited to five minutes, PR #204 stays documentation-only, and PR #196 stays out of scope.

### 17.14 Formatting correction and current head — 2026-10-22, PR #205 head `f8a582fdd11a43492a3c8fbcbae19231c7d775d0`

- Workflow [39829802168](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39829802168) completed on head `2ae8d45e2ef5033438c827da1879111d0e9ff6ee`, but the formatting gate failed before the full test suite ran. The log identified only rustfmt layout changes in the new topology regression test's three placement literals and restoration origin. No Rust test result for the topology change was produced by that run.
- Commit `f8a582fdd11a43492a3c8fbcbae19231c7d775d0` applies the exact formatting shown by the CI diff; it does not change intended behavior.
- **Current verification state:** no workflow run for `f8a582f` was visible at the time of this update. The restoration topology guard and its regression test remain unverified on the current head.
- **Next action:** obtain the exact-head CI result. If formatting passes, inspect whether the focused/full suite executes and verify the new test by name. Then continue the next focused Phase 3 audit. Keep the suite and Clippy status accurately baseline-red until evidence says otherwise; PR #204 remains information-only and PR #196 remains out of scope.

### 17.15 Second Phase 3 restoration audit — 2026-10-22, PR #205 head `bb2a1ce26cfb887573931d775b299c69e9d5d949`

- **Audit finding:** `StructuralUnit::from_material` validates material composition but does not validate placement coordinates. `restore_material` previously transformed the caller-supplied origin and stored relative placements without checking finiteness. A single-part material has no bond/contact check that would necessarily reject a non-finite transformed placement, so the restoration boundary must validate this explicitly.
- **Implementation:** commit `ef3ccc329e6f8e87bf6137110d460dcd218406fd` adds finite-placement validation for the origin, stored relative placements, and each transformed placement (also rejecting overflow to infinity).
- **Regression test:** commit `bb2a1ce26cfb887573931d775b299c69e9d5d949` adds `restoration_rejects_non_finite_origin_without_mutating_inventory`. It supplies a NaN origin for a stored Carbon and checks restoration rejects it without changing the serialized organism structure, authoritative storage, or independently recomputed integer constituent inventory.
- **CI sequencing:** workflow [39830063592](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39830063592) was running the full suite on earlier head `f8a582f` at the time of this update. That head's formatting and all focused gates passed before the full suite started; it does not include the non-finite-origin guard/test. No workflow result for current head `bb2a1ce` was visible yet. Do not mark either new restoration regression test verified until exact-head CI runs.
- **Next action:** inspect completion of workflow 39830063592, then verify CI for current head `bb2a1ce`. Keep the next change behind a separate focused audit. Phase 3 remains open; Phase 4 failure-injection work remains open. PR #204 remains docs-only and PR #196 stays out of scope.

### 17.16 Formatting correction for finite-placement guard — 2026-10-22, PR #205 head `be1e9c2aae56c9f94ab0d899f358929bbb754915`

- Workflow [39830390116](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39830390116) checked head `bb2a1ce` and failed only at formatting before test steps. The rustfmt diff required collapsing the finite-placement helper's boolean expression to one line and wrapping the relative-placement iterator chain.
- Commit `be1e9c2aae56c9f94ab0d899f358929bbb754915` applies the exact formatting. No behavior change in this formatting-only commit.
- Workflow [39830528840](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39830528840) is queued for this current head. Exact-head test evidence for both restoration regressions remains pending.
- Continue only after recording exact-head results. The full suite and Clippy are not considered green; do not infer test success from focused gates or from an older head.


### 17.17 Exact-head verification and timeout — 2026-10-22, PR #205 head `be1e9c2aae56c9f94ab0d899f358929bbb754915`

This entry supersedes Section 17.16's queued-workflow status. The exact-head workflow has completed, but its full-suite step hit the mandated five-minute timeout; treat that gate as **incomplete**, not as a successful completed suite.

- **Exact-head workflow:** [39830528840](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39830528840), conclusion `failure`. Formatting, source-size guard, COMBINE architecture, Bob compact-storage, empty-library open/reopen, concurrent Bob initialization, focused fluid transaction/coalescence/restoration, and focused chemical BREAK steps all passed.
- **Restoration regressions:** the full-suite process did execute and pass all three relevant tests: `integer_material_inventory_preserves_state_when_restore_rejects_invalid_connections`, `restoration_rejects_connections_that_change_stored_bond_topology_atomically`, and `restoration_rejects_non_finite_origin_without_mutating_inventory`. They validate rejection and unchanged structure/storage/independent integer inventory for malformed connection topology and non-finite placement input.
- **Full-suite step:** the runner logged `585 passed, 27 failed, 1 ignored` at 304.72 seconds and then reported that `Run Rust tests` timed out after five minutes. Direct comparison of the 27 named failing identities with baseline workflow `39705628427` found **no new failure identities**; they are the baseline's 28 identities minus `simulation_tests::integration_tests::contained_physical_material_becomes_storage_without_an_acquire_action`, which remains resolved. Because the step timed out, do not call this a completed full-suite verification even though the test harness emitted a summary.
- **Clippy:** failed to compile under strict warnings with 34 application-target and 15 test-target diagnostics. These counts are consistent with the known baseline lint debt, but this update does not claim a fresh normalized diagnostic-identity comparison for this exact head.
- **Phase 3:** the topology-rejection and non-finite-placement rejection tests are now evidenced as passing. Keep the phase open for remaining failed-transformation rollback, storage/restore edge cases, and decomposition/post-death ownership.
- **Phase 4:** schema/fingerprint, missing-manifest, manifest rollback/reopen, partial-append rollback/reopen, and concurrency tests remain present; the required next gate is failure injection through the actual `ChemistryLibrary::insert` append path, not only a rollback helper.
- **Next action:** continue with a focused read-only audit of the real cache insert/append transaction and identify a minimal failure-injection seam/test. Do not edit implementation until that audit identifies the exact invariant and smallest change. Keep every command/test under five minutes, preserve PR #204 as documentation-only, and keep PR #196 out of scope.


### 17.18 Phase 4 focused audit — real cache insert/append failure path — 2026-10-22

**Audit status: Gap found; smallest test seam identified.** Read-only inspection of `src/chemistry/chemistry_library.rs` at PR #205 head `be1e9c2aae56c9f94ab0d899f358929bbb754915` found the following production transaction:

1. `ChemistryLibrary::insert` delegates to `insert_with_manifest_persistence` with the real atomic manifest persister.
2. Under the store lock, the method reloads persisted state, refreshes in-memory entries/manifest, and rejects duplicate keys.
3. It opens `chemistry.jsonl` in append mode, captures the original byte length, serializes one row, then calls `write_all` and `sync_data`.
4. If write or sync fails, it calls `rollback_appended_record` to truncate to the captured length and sync the truncation. If manifest persistence fails after append, it uses the same rollback path.
5. In-memory `entries` and `manifest` are published only after append/sync and manifest persistence both succeed.

**Coverage gap:** `partial_append_failure_rolls_back_original_cache_bytes_and_reopens_cleanly` currently writes a partial row directly to a file and calls `rollback_appended_record` itself. It proves the helper truncates bytes and that reopen succeeds, but it does **not** drive a write failure through `ChemistryLibrary::insert`; therefore it does not prove that the production append error branch invokes rollback and leaves in-memory/manifest state unchanged.

**Smallest recommended implementation/test change (PR #205 only):** factor the append-and-sync operation behind a private injectable callback used by the same insertion transaction. Production `insert` supplies the existing `write_all + sync_data` behavior; the regression test supplies a callback that writes a short prefix and then returns an injected `WriteZero`/I/O error. Exercise the actual insert transaction and assert: the original JSONL bytes are restored; manifest bytes/count are unchanged; in-memory entry count and manifest count are unchanged; the failed key is absent; and reopening the store returns exactly the original record set. Keep the current helper-level test if it still provides distinct value, or replace it only if the new end-to-end test subsumes it.

**Boundaries / follow-up:** do not change cache schema, equations, or runtime chemistry policy. A rollback-truncation failure is a distinct failure mode and may leave the store requiring operator recovery; do not claim that case solved by the injected append-failure test. Keep Phase 4 open until actual insert-path failure injection passes and broader manifest/restart failure handling is reviewed. This was one focused read-only audit; no implementation code changed during it.

**Next action:** implement the minimal private append callback and end-to-end rollback regression on PR #205, run the targeted test and formatting under the five-minute limit, then verify exact-head CI and update this handoff.


### 17.19 Phase 4 end-to-end append failure test added — exact-head verification pending

- **Current PR #205 head:** `8dbef20d13aa69c0d44393a623e2a60673460bca` (commit message: `test: exercise chemistry insert append rollback`).
- Following the focused audit in Section 17.18, `ChemistryLibrary::insert` now routes through a private `insert_with_persistence` transaction seam. Production supplies `append_record_and_sync` and the real atomic manifest persister; the existing manifest-failure test retains its narrower manifest-persistence seam.
- The prior helper-level partial-append test was replaced with `insert_partial_append_failure_rolls_back_original_cache_bytes_and_reopens_cleanly`. Its injected append callback writes and syncs a short prefix, then returns `WriteZero`, forcing the real insertion transaction's error branch to invoke `rollback_appended_record`.
- The test asserts the original JSONL bytes and manifest bytes are unchanged, in-memory entry and manifest counts remain at one, the failed key is absent, and reopening returns only the original record. This is test-only behavior coverage plus a private test seam; no intended runtime behavior or chemistry equations changed.
- **Verification:** no workflow result was returned for this exact head at the time of this handoff update. Do not claim formatting, the targeted test, full suite, or Clippy has passed until exact-head CI evidence is available. Previous head `be1e9c2` passed formatting/focused gates, passed the three restoration rejection/inventory tests, and showed no new named test failure identities, but its full-suite step timed out after five minutes and is not a completed full-suite gate.
- **Next action:** locate/inspect the exact-head workflow for `8dbef20`. If no run was created, determine why before retrying. Verify the new test by name and check formatting. Keep Phase 4 open until exact-head test evidence and broader cache failure/restart review are complete; continue Phase 3 rollback/decomposition ownership afterward.


### 17.20 Formatting-only correction and exact-head CI in progress — 2026-10-22

- Workflow [39832439035](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39832439035) ran on `8dbef20d13aa69c0d44393a623e2a60673460bca` and failed at the formatting gate. Rustfmt requested only two formatting changes in `src/chemistry/chemistry_library.rs`: collapse the production call to `insert_with_persistence` onto one line and wrap the injected test key construction. No Rust tests ran on that head; later gates were skipped. Clippy ran separately but did not substitute for the skipped test gates.
- Commit `318c9afd5b1c3e6cf81ef7498829afef90838c21` applies exactly those rustfmt changes; intended behavior is unchanged.
- **Current head:** `318c9afd5b1c3e6cf81ef7498829afef90838c21`. Exact-head workflow [39832607493](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39832607493) has started; at this handoff update, the toolchain setup step was still in progress and formatting/test outcomes were not yet available.
- **Next action:** inspect workflow 39832607493. Verify formatting, then the new insert-path rollback test by name; record the full-suite status honestly, including any five-minute timeout. Do not infer success from the previous run or formatting-only commit.


### 17.21 Phase 3/8 lifecycle ownership audit — distinguish live chemistry from legacy decomposition helpers

**Audit status: live structural chemistry verified by source inspection; post-death holder ownership remains an open audit question.** Read-only inspection of PR #205 source at head `318c9afd5b1c3e6cf81ef7498829afef90838c21` established:

- `src/chemistry/recycling.rs::recycle_dead_organism` has no observed production call site; its observed caller is its unit test. It drains organism storage and committed developmental material into the environment and transfers usable/developing energy into a `DecomposingBody`.
- `src/simulation/simulation.rs::process_decomposing_bodies` is annotated `#[cfg(test)]`. It exercises the legacy one-bond decomposition/finalization helper only in tests. The live tick contains an explicit comment: “Do not invoke the legacy DecomposingBody bond-breaking path here.” **The absence of this legacy path from runtime is consistent with the governing rule that decomposition is ordinary chemistry after death, not a parallel bond-breaking law; it is not, by itself, a defect.**
- In live `Simulation::step`, `living_ids` gates biological maintenance, harmonics/memory, decisions, movement/reproduction, and survival damage. The per-organism `chemical_reaction::accumulate` and `chemical_reaction::resolve` pass is outside that gate and is explicitly documented to continue for physical structures after organism status ends. Thus breached organisms remain in the organism vector and their structural bonds continue through the same ordinary chemical reaction mechanism.
- `chemical_reaction::accumulate` currently iterates `organism.structure.bonds`; it does not enumerate `organism.stored_material` or the reproductive construction's committed/developing holders. Those holders remain associated with the nonliving organism, and ledger inventory functions still count several associated energy/structural holders. Whether and how their physical material and energy become available to the wider environment after death is the remaining ownership question.
- Active physical/chemical transformations are deliberately retained after a core breach so a reserved material/energy transaction is not silently discarded; their exact interaction with death and any stalled developmental-construction state still needs a named inventory audit.

**Correct interpretation:** do not wire `recycle_dead_organism` or `process_decomposing_bodies` into the live tick merely because they are staged. That would risk reinstating the separate legacy decomposition law the owner rejected. The accepted path is live ordinary chemistry on the retained body, with biological processing gated off. Phase 8 remains open for the ownership/release/availability of non-structural stored and developing material/energy after death, and for proving that the full live death transition preserves inventories.

**Next audit before any lifecycle code change:** construct a source-to-holder map for a breached organism with (a) structural bonds, (b) stored physical material, (c) active transformation reservations, and (d) reproductive construction material/energy. Trace which holders ordinary chemistry can act on, which remain inaccessible, and how the independent energy/material inventory changes over several live ticks. Include core-breach timing and stress-triggered bond damage; do not implement a separate decomposition law. One focused audit before each change; PR #204 remains documentation-only and PR #196 remains out of scope.

**Parallel active work:** cache insert rollback test is on PR #205 head `318c9afd5b1c3e6cf81ef7498829afef90838c21`; workflow [39832607493](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39832607493) was running the Rust test step at the time of this audit. Verify its exact-head result before closing Phase 4 gates.


### 17.22 Post-death holder audit — structural chemistry continues; stored/developing material is not part of that reaction pass

**Audit status: structural path confirmed; material-holder availability gap needs a bounded follow-up.** Source inspection of `src/simulation/simulation.rs`, `src/chemistry/chemical_reaction.rs`, `src/organism/reproduction.rs`, and `src/chemistry/transformation.rs` establishes:

- The live per-organism loop invokes `chemical_reaction::accumulate` and resolves its returned operations regardless of `living_ids`. `accumulate` enumerates only `organism.structure.bonds`. Therefore an organism whose core was breached remains in the physical holder set and its structural bonds continue ordinary chemistry, while biological maintenance, memory, decisions, movement/reproduction progression, and survival damage are skipped.
- The independent energy snapshot sums `usable_energy` and any `reproductive_construction.developing_energy` across **all** organisms, plus resonant energy and both the organism's structural bonds and developing-structure bonds. The production aggregate `total_usable_energy_held` likewise includes developing energy. These counters do not drop the dead organism's tracked energy merely because it is no longer living.
- Active transformations are drained/resolved before the live `living_ids` set is computed. Source comments explicitly retain in-flight transformations after core breach to avoid discarding reserved material/energy; do not cancel or drop them merely because an organism is dead.
- `chemical_reaction::accumulate` has no references to `stored_material` or `reproductive_construction`. The live tick skips further reproductive-construction advancement for nonliving organisms. Thus stored physical material, committed developmental material, and developing structure/energy remain attached to the dead organism rather than participating in the structural-bond reaction pass. The staged `recycle_dead_organism` helper would release some of those holders but has no observed live call site, and its legacy decomposition path must not be activated as a substitute for ordinary chemistry.

**Remaining question / next focused audit:** the owner-approved direction says death ends biological processing, not physical existence, and resources should remain available for ordinary chemistry/repurposing. Audit whether storage and reproductive-construction holders must be released into the environment immediately at core breach, or can remain attached until another existing physical path makes them available. For either outcome, define one atomic ownership transition that preserves integer constituent inventory, usable/resonant energy, bond energy, active transformation reservations, and the developing offspring's holders. Include a dead organism with non-empty storage and active reproductive construction; verify no double counting or silent loss over multiple ticks. Do not implement the legacy `DecomposingBody` bond-breaking path.

**Current parallel verification:** workflow [39832607493](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39832607493) on head `318c9afd5b1c3e6cf81ef7498829afef90838c21` has passed formatting, source-size, COMBINE architecture, Bob, fluid, and focused BREAK gates; the full Rust test step is still running as of this audit. The end-to-end cache insert append rollback test remains unverified until that step reports it by name.


### 17.23 Exact-head cache rollback test result — passed; Clippy exposed a test-only seam warning

- **Exact head:** `318c9afd5b1c3e6cf81ef7498829afef90838c21`; workflow [39832607493](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39832607493) completed with failure.
- Formatting, source-size guard, COMBINE architecture, Bob compact-storage, empty-library open/reopen, concurrent Bob initialization, focused fluid transaction/coalescence/restoration, and focused chemical BREAK steps all passed.
- The new `chemistry_library::tests::insert_partial_append_failure_rolls_back_original_cache_bytes_and_reopens_cleanly` test **passed**. It exercised a partial write through the actual `insert_with_persistence` transaction and verified the original JSONL bytes and manifest bytes, in-memory counts, failed-key absence, and reopened record set remained unchanged.
- Full suite completed within the five-minute limit: **585 passed, 27 failed, 1 ignored** in 211.75 seconds. Comparing named failure identities against baseline workflow `39705628427` found no new failures; the 27 are the 28 baseline failures minus `simulation_tests::integration_tests::contained_physical_material_becomes_storage_without_an_acquire_action`, which remains resolved. The suite is still baseline-red.
- Strict Clippy failed with 34 application-target and 15 test-target errors. Diagnostic normalization found one new dead-code identity: `insert_with_manifest_persistence` is only called by a unit test, but is compiled into the non-test target; it was grouped with `len`/`is_empty` in the diagnostic. This is a small test-seam warning introduced by the latest change, not a chemistry behavior failure.
- **Next minimal fix:** gate the private `insert_with_manifest_persistence` helper with `#[cfg(test)]`, since its only caller is a unit test. Run formatting and exact-head CI again, then confirm the new dead-code identity disappears while the insert-path rollback test still passes. Do not claim Clippy diagnostics are baseline-identical until that comparison is repeated.
- Phase 4 remains open for broader cache failure/restart coverage; the append transaction gate is now covered by a passing end-to-end regression. Phase 8 remains open for post-death stored/developing holder availability and inventory conservation; live structural chemistry already continues after core breach.


### 17.24 Test-only annotation for manifest failure seam — exact-head verification pending

- **Current PR #205 head:** `12d8079230458da68a8dc3755a1d6a9b41347862` (commit: `fix: keep manifest failure seam test-only`).
- Following Section 17.23, `#[cfg(test)]` now guards private `insert_with_manifest_persistence`, whose only caller is the manifest-failure unit test. Production `ChemistryLibrary::insert` still uses `insert_with_persistence` with the real append/sync and manifest persistence functions; runtime behavior is unchanged. This removes the new non-test dead-code diagnostic identified on the prior head.
- Exact-head workflow [39833438679](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39833438679) is queued at the time of this update. Do not infer the annotation, test, suite, or Clippy result before that workflow completes.
- **Next:** inspect the queued workflow, verify formatting and the end-to-end insert rollback test by name, compare all 27 full-suite failure identities to baseline if the suite completes within five minutes, and compare normalized Clippy diagnostics to the baseline. If Clippy still shows a new diagnostic, audit that exact message before another code change.


### 17.25 Phase 8 inventory-oracle audit — concrete dead-holder regression scope

- **Audit status: source-to-holder inventory coverage mapped.** The independent `integer_material_inventory` helper in `src/tests/simulation_tests.rs` walks environmental aggregate/physical material; every organism's structure and stored material; each reproductive construction's committed material and developing structure; active transformations' temporarily removed stored/environmental sources; and any legacy decomposition bodies. It deliberately does not double-count prepared trial copies, prepared storage/structures, or prepared ledgers.
- This oracle includes breached organisms because it iterates all entries in `simulation.organisms`, not just `living_ids`. It is suitable for testing a death transition without relying on the production energy/material aggregate counters.
- **Smallest next Phase 8 regression:** construct an organism with non-empty physical storage and an active reproductive construction (committed material, developing structure, developing energy), breach its core through the existing physical/stress path, and snapshot the independent integer inventory plus current usable/resonant/structural energy holders. Step the live simulation for bounded ticks and assert: the organism remains a physical holder; no biological construction progress occurs after breach; ordinary structural chemistry may change bonds but not constituent counts; all energy/material holders remain counted exactly once; and no legacy `DecomposingBody` is silently introduced. The fixture should isolate external inputs and active transformations explicitly so the test distinguishes holder conservation from newly captured energy or a legitimate in-flight transaction.
- Do not assert that stored/developing material is immediately deposited into the environment until the ownership rule is established. The audit question is whether those holders remain available/repurposable or need an atomic release path, not whether to invoke the legacy recycling helper.
- **Current parallel verification:** workflow [39833438679](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39833438679) on head `12d8079230458da68a8dc3755a1d6a9b41347862` has passed formatting, source-size, architecture, Bob storage/reopen/concurrency gates; the focused Bob fluid transaction step is currently running. Cache insert rollback and Clippy results remain pending on this head.



### 17.26 Phase 8 focused ownership audit — release APIs, death observation, and legacy recycler ruled out

**Audit status: existing transfer APIs mapped; implementation design remains open.** Read-only source audit on PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`.

- The authoritative physical insertion APIs are `ActiveMaterialField::deposit_physical(x, y, PhysicalMaterial)` and `deposit_physical_at_index(index, PhysicalMaterial)`. They transfer an already-realized object without rebuilding its internal bonds and return success/failure. The coordinate variant can fail when the position is outside the field or the object is invalid.
- `MaterialStorage::drain_entries()` transfers ownership out of a storage holder, but callers must preserve each entry until deposit succeeds. The existing `recycle_dead_organism` drains entries and ignores deposit results, so it is not a safe no-loss transfer path.
- There is no explicit one-shot death transition in the live tick. The simulation recomputes `living_ids` from genome-core qualification. A breached organism leaves biological processing but remains in `self.organisms`, and its realized structural bonds continue through ordinary chemical reactions.
- Independent/current energy holders include each organism's usable energy and reproductive `developing_energy`, its resonant reservoir, bond energy in its own structure and developing structure, and any legacy decomposition body. Active transformations are drained/resolved before the living set is recomputed; their temporarily removed physical sources and prepared/reserved energy must remain owned and settle normally after breach.
- The existing `recycle_dead_organism` helper is unsuitable and must not be connected to the live tick. It introduces the forbidden `DecomposingBody` carrier; drains/deposits storage without checking deposit success; takes and drops the reproductive-construction object after depositing committed material; fails to preserve its `developing_structure` as a separate physical holder; and transfers parent/developing usable energy into the decomposition body's energy budget.
- `PhysicalMaterial` can preserve a stored composition's internal bonds, but the audited code has no established conversion that carries an entire `OrganismStructure` (unit identity, bond endpoints, and stored bond energy) into a single intact environmental physical holder. Resonant energy likewise has no established death-transfer policy/API. Do not silently drop either store or count structural bond energy as fresh constituent potential.

**Conclusion:** the post-death availability gap is confirmed, and calling the legacy recycler is ruled out. A valid implementation needs a designed representation for unfinished construction and an atomic ownership transition that handles failed deposits by rollback or retained ownership. It must conserve the parent body's physical material, committed/developing material, usable/developing/resonant energy, structural bond energy, and in-flight transformation ownership exactly once, without introducing a parallel decomposition law.

**Next gates, in order:**
1. Normalize strict-Clippy diagnostic identities on PR #205 head `12d8079` against the frozen baseline before considering any lint change.
2. Complete a narrow design audit for an atomic no-loss ownership transfer of stored and unfinished-construction material, including how developing-structure bonds remain represented and what happens to resonant energy.
3. Only after that focused audit is complete, make the smallest scoped implementation change on PR #205 and add the independent material/energy inventory regression. Keep Phase 8 open until exact-head evidence passes.

**Verification boundary:** workflow [39833438679](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39833438679) completed on `12d8079230458da68a8dc3755a1d6a9b41347862`: formatting and focused gates passed; the insert-path cache rollback test passed; full suite completed in 283.74 seconds with 585 passed, 27 baseline-matching failures, 1 ignored; Clippy had 34 app-target and 15 test-target diagnostics, with normalized identity comparison still outstanding. No source or test changes were made in this audit.


### 17.27 Exact-head Clippy diagnostic comparison — baseline identities confirmed

- Compared the normalized diagnostics in current PR #205 workflow [39833438679](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39833438679) on head `12d8079230458da68a8dc3755a1d6a9b41347862` against frozen baseline workflow [39705628427](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/39705628427).
- Both report 34 application-target and 15 test-target diagnostics. Normalized identities (diagnostic message + source path, ignoring line-number shifts) have no new and no removed entries.
- Clippy remains failing because of baseline lint debt. No lint fix is warranted by this comparison; do not make lint-only changes.
- This closes the Clippy comparison gate for head `12d8079`. Resume Phase 8 design audit from Section 17.26. Before a code change, specify how the unfinished developing structure remains a physical, chemically addressable holder with its existing bond energy and endpoints preserved, how resonant energy is owned after breach, and how failed environmental deposits roll back or retain ownership.
- PR #205 comment [6348800724](https://github.com/coringilbert16-cmyk/EvoSim-/pull/205#issuecomment-6348800724) records the comparison. No source/test change occurred.


### 17.28 Phase 8 addendum — existing expulsion helper is not a general death-release transaction

Read-only follow-up inspection of `src/chemistry/expulsion.rs` on PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862` found `expel_physical_material`, a geometry-aware voluntary path for one stored physical object.

- It correctly accounts for the owner's anchor and the stored object's owner-relative placement when computing a world destination, unlike simply depositing storage-local placements at the owner's first cell.
- It is not a guaranteed death-release mechanism: without a movement direction from experience memory, it needs a nonzero `owner_relative_origin` offset. If neither exists, it returns false and leaves the item stored.
- After removing the item from storage and calculating a destination, it returns the result of `deposit_physical_at_index` directly. If that final insertion unexpectedly fails, it does not restore the removed item. A death transition cannot rely on it without strengthening its rollback guarantee.
- It only handles stored `PhysicalMaterial`; it does not preserve/release the developing structure as an intact graph with its existing bond energies, and it does not define ownership of developing or resonant energy.

**Design consequence:** do not loop `expel_physical_material` on core breach. The Phase 8 design must use deterministic placement independent of behavioral memory, retain ownership or roll back every failed insertion, and provide an intact, chemically addressable physical holder for developing structure/bond energy. The parent body's usable/resonant energy and in-flight transformation reservations must remain accounted exactly once. No code/test changes were made.

PR #205 comment [6348852052](https://github.com/coringilbert16-cmyk/EvoSim-/pull/205#issuecomment-6348852052) records this addendum.


### 17.29 Recommended Phase 8 design — generic detached physical structure + atomic ownership transfer

**Recommendation for design review; not yet implemented or treated as a settled API contract.**

Do not convert `ReproductiveConstruction.developing_structure` into a fresh ordinary `Material`, and do not reuse legacy `DecomposingBody`. Instead, introduce (or reuse if a suitable type is found during implementation audit) a **generic detached physical-structure holder** that owns the existing realized structure as-is and participates in the same ordinary structural chemistry rules. It has no genome and performs no biological work. Its purpose is physical ownership, not a special decomposition process.

The holder must preserve:
- the existing constituent units and realized geometry;
- bond identities/endpoints and current bond-energy state;
- world anchor/placement needed for collision, contact, and chemistry;
- explicit ownership of associated usable/developing/resonant energy, keeping these energy categories distinct.

The parent organism remains physically present and continues ordinary chemistry after core breach. Stored physical material and committed construction material should transfer into authoritative environmental/physical ownership using deterministic valid placements. The developing structure should transfer as a structure graph, not be flattened into a composition unless an audited lossless representation proves it preserves geometry, bond endpoints, and energy.

**Death should be a one-time lifecycle/ownership transition**, not per-tick rediscovery and not a second decomposition law. Biological functions stop; existing chemical rules continue on every physical structural graph. Active transformations retain their existing settlement/rollback behavior and must never be orphaned by the transition. Resonant energy must remain a separately tracked reservoir until a deliberate rule determines whether and how it can be consumed or dissipated; do not silently reclassify it as usable energy.

**Atomic transfer protocol:**
1. Prepare: validate source holders, every destination/placement, geometry, and all energy ownership before draining anything.
2. Commit: transfer the original holdings exactly once, retaining original object/graph identity and energy state where possible.
3. Verify with independent material and per-category energy inventories.
4. Roll back completely on failure, or retain each untransferred holding under its existing owner for a safe retry. Never drain-and-ignore insertion failure.

**Regression design:** build a test organism with nonempty stored physical material, committed reproductive material, a nonempty developing structure with nonzero bond energy, developing energy, resonant energy, usable energy, and an in-flight transformation where feasible. Trigger an existing core-breach path, snapshot independent material and energy-category inventories, then run bounded live ticks. Assert the parent body remains physical, biological construction no longer advances, detached physical graphs and released material participate in ordinary chemistry, bonds remain intact until legitimately changed by chemistry, all holders are counted exactly once, and failed insertion preserves ownership. Do not introduce `DecomposingBody` or special one-bond decomposition.

**Why this is preferred:** it preserves the existing physical graph and its bond energy rather than reconstructing them from composition, makes death a clear ownership boundary, and allows one general chemistry engine to handle living and nonliving physical structures. Main design cost/risk is extending spatial queries, collision/contact, chemical reaction enumeration, persistence/serialization, and independent inventory to the detached-structure collection; audit those call sites before coding.

**Open decisions before implementation:** choose the minimal home for the detached-structure collection (environment/world state), define its stable ID and anchor transform, define whether resonant energy persists indefinitely or dissipates under an already approved energy law, and prove all transfer paths can fail without partial loss. This recommendation is a proposed design, not approval to start coding; complete the call-site/API audit first.


### 17.30 Refined ownership model — parent remains the aggregate owner until division

**Recommended refinement prompted by design discussion; supersedes death-time detachment as the preferred first option, pending call-site audit.**

Prefer one authoritative parent aggregate to own all material and energy involved in its current life and unfinished reproduction until a successful division transaction. The aggregate includes the parent's realized body structure, stored physical materials, committed reproductive material, developing structure and its bonds, and associated energy reservoirs. The adjacent second genome core is part of the parent's internal construction until the split; it is not a separate organism or an external bud.

Death before division does **not** transfer ownership or convert the holdings into a special decomposition object. It stops biological processing, while the dead parent continues to own its physical holdings. Ordinary chemistry may continue to act on its body and, where physically accessible and covered by the same chemistry rules, its stored/developing physical material. This avoids a death-only migration of all holdings and avoids introducing a detached structure solely because death occurred.

**Important limitation:** aggregate ownership alone does not make every owned holder chemically available. The Phase 8 audit must still determine how ordinary reactions discover and interact with stored physical material and the developing structure after biological work stops. Extend the existing chemistry path over those physical holders as appropriate; do not invent a separate decomposition law, and do not flatten a developing graph in a way that loses geometry, bond endpoints, or bond energy. Keep the parent as owner while those holdings remain within the aggregate.

At division (the reproduction requirement is 100% original size → construct adjacent second genome core → grow to 160% total → split into two approximately 80%-size children), perform a single atomic partition:
1. Assign each constituent unit and each independently owned physical material to exactly one child.
2. Partition stored/committed holdings under an explicit deterministic rule; do not duplicate inventories.
3. Assign bonds whose endpoints both belong to one child to that child. Explicitly handle any bond crossing the partition boundary through a defined chemistry/energy settlement rule; never silently clone or discard bond energy.
4. Partition usable, developing, resonant, and structural energy by their actual ownership semantics, not an arbitrary 50/50 copy.
5. Validate both resulting children and all destinations before committing; on failure, preserve the intact parent aggregate and retry safely.

This reduces lifecycle complexity by making death a biological-state change rather than a mass-transfer event. The hard remaining work is correct chemistry enumeration/accessibility for all owned physical holders and a lossless, atomic division partition. Keep the implementation gate closed until those call sites and invariants are audited. No code/test changes were made.


### 17.31 Owner approval — parent aggregate owns all holdings until division

**Decision status: APPROVED by owner in conversation on 2026-10-22.** This supersedes the provisional status language in Section 17.30. The parent cell remains the complete authoritative owner of its physical body, stored material, committed reproductive material, developing structure and bonds, and associated energy until the division transaction. Core breach ends biological processing but does not trigger a death-time ownership transfer or special decomposition object. The second genome core remains part of the parent's unfinished internal construction until division.

**This decision settles lifecycle ownership architecture, not chemistry completion.** The following must still be demonstrated:
1. Ordinary chemistry enumerates and physically validates reactions against all applicable parent-owned physical holders (body structure, stored material, committed material, and developing structure) without treating ownership alone as accessibility and without adding a separate decomposition pathway.
2. Material and energy inventories account for each constituent and each distinct energy category exactly once, including usable, developing, resonant, structural bond energy, external input, work, and heat. Every transaction has an authoritative source debit/transfer and a balanced settlement.
3. Every physical transition (COMBINE, chemical BREAK, stress BREAK, storage entry/exit, movement/work, resonance/world-tone capture, dissipation, death, and relevant lifecycle operation) has atomic validation/commit semantics and tests for failed/stale operations.
4. Developing-structure bond endpoints, geometry, and stored bond energy remain valid through ordinary chemistry and any later division partition. Cross-partition bonds have an explicit settlement rule.
5. The approved reproduction lifecycle can eventually partition the parent aggregate atomically at 160% total size into two approximately 80%-size children; every unit, holder, bond, and energy store is assigned exactly once. This is a related lifecycle gate, but implementation remains separate and does not belong in PR #205 unless the owner explicitly expands scope.
6. World-tone/sunlight input and propagation reconcile with the same energy inventory; source budget, capture, retained resonance, propagation/escape, and heat close without duplicate credit. Geometry and occlusion must be tested, and the implementation must meet the performance constraints already recorded in this plan.
7. Focused regressions and bounded full-suite CI verify the changed paths. Existing baseline failures and Clippy diagnostics must be compared by identity; never claim success solely from counts or by treating baseline failures as new regressions.

**Next focused audit:** trace the reaction-enumeration and validation path for parent-owned stored/committed/developing physical holdings, and compare it with the current independent material/energy inventory. Identify the smallest missing link before changing code. Keep PR #204 documentation-only and PR #205 as the implementation branch. No code or tests changed for this approval.

### 17.32 Phase 8/chemistry reachability audit — owner aggregate versus reaction domain — 2026-10-22, PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`

**Audit scope:** Trace the production reaction accumulator and compare its actual domain with the owner-approved aggregate model in Section 17.31. Read the exact implementation-head versions of `src/simulation/simulation.rs`, `src/chemistry/chemical_reaction.rs`, `src/simulation/state.rs`, `src/materials/material_storage.rs`, `src/simulation/world_tone_snapshot.rs`, and `src/organism/reproduction.rs`. This is a source audit only; no source or test files were changed.

**Confirmed reaction domain**
- The simulation tick calls `chemical_reaction::accumulate` once for each organism, including breached/dead organisms that remain in `simulation.organisms`. A successful reaction is resolved against that same organism immediately; a core-breach check then gates later biological processing. This is consistent with the approved no-special-decomposition rule.
- `chemical_reaction::accumulate` returns immediately when `organism.structure.bonds` is empty and otherwise iterates only a clone of that graph's `structure.bonds`. For each bond it resolves constituent indices, material properties, chemical positions, candidate contact geometry, interface compatibility, reaction potential, and activation; `resolve` settles the matching structural BREAK transaction. This is a physically validated local graph-bond path, not a general scan of all parent-owned physical holdings.
- The accumulator does **not** enumerate `organism.stored_material.entries`, `ReproductiveConstruction.committed_material.entries`, or `ReproductiveConstruction.developing_structure.bonds`. Stored compounds expose realized geometry/internal connections through `PhysicalMaterial`, while developing structure is a separate `OrganismStructure`; neither is passed into this accumulator. Therefore ownership by the parent is not sufficient to make those holdings chemically reachable. The source does not establish a generic chemistry pass over these holders.
- `MaterialStorage::drain_entries` empties the source storage using `mem::take`. That is not itself a proven defect, but any eventual multi-holder chemistry or release path must not drain before all geometry/energy checks and destination settlement can succeed; rollback or return-on-error is required to preserve the approved atomic ownership invariant.
- `ReproductiveConstruction` stores committed material, a developing graph, child genome, developmental origin, and `developing_energy`. The energy snapshot sums the parent usable energy plus developing energy, the parent's resonant reservoir, the parent graph's bond energy, and developing graph bond energy. It also includes legacy decomposition-carrier values if any are present. This confirms the test snapshot knows about several holders, but it does not prove each holder participates in ordinary reaction enumeration or that every transfer is atomic.
- The world-tone occluder snapshot independently assembles physical bodies from organism structures, developing structures, committed material, environmental physical material, stored physical material, legacy carriers, and active transformations. This is an exposure/occlusion inventory, not evidence that the ordinary chemical accumulator can react with all those holders.

**Gap / conclusion**
The approved model says the parent is the authoritative aggregate owner until division; it does not say every owned holder is automatically chemically available. The current live reaction pass is limited to bonds in the parent's main structural graph. Stored physical composites and committed material, and the developing graph/bonds and their associated energy, are not part of that pass. This is a concrete chemistry-reachability gap, but the correct fix cannot yet be selected: a stored physical bond does not necessarily have a separately funded `bond_energy`, and blindly treating internal connections as graph bonds could invent energy or break geometry incorrectly. Preserve the current invariant that potential energy is not automatically held energy.

**No implementation change authorized by this audit.** The remaining audit must map the representations and settlement semantics of (a) parent structural graph bonds, (b) stored/committed `PhysicalMaterial` internal connections, and (c) developing graph bonds; determine which bond classes carry actual stored energy, which may react, and how a failed/stale reaction restores both physical ownership and any resonance/energy debit. Then complete the all-holder independent material and energy inventory comparison and transaction failure-path matrix before proposing the smallest unified reaction-domain change. Do not add a `DecomposingBody` live path or move reproductive division into PR #205.

**Next audit:** compare `PhysicalMaterial` internal-connection representation and stored-BREAK settlement with graph-bond settlement; trace all holder take/drain/restore and energy-ledger paths at this exact head. Update the handoff with the result before any implementation.


### 17.33 Stored physical-bond audit — topology is represented; stored energy is not — 2026-10-22, PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`

**Scope / evidence:** Exact-head review of `src/materials/physical_material.rs` (`PhysicalMaterialBond`, realization, `break_internal_bond`), `src/chemistry/transformation.rs` (stored BREAK start/prepare/resolve), `src/simulation/simulation.rs` (failed preparation restoration), and `src/simulation/environment.rs` (physical deposit validation). No code or tests changed.

**Representation contract confirmed**
- `PhysicalMaterialBond` records constituent indices and concrete contact endpoints only; it has no `bond_energy` field. `PhysicalMaterial::realized_inner` derives deterministic available physical endpoints from realized geometry, and `break_internal_bond` removes one connection, computes connected components, and remaps constituent indices, internal bonds, endpoint connections, placements, and realized shapes into resulting pieces.
- The live stored-material BREAK path explicitly validates/splits topology but settles a zero-energy transaction: zero potential release, usable delta, structural delta, work, and heat. The code comment and regression state why: energy was not allocated when those pre-existing internal bonds were created, so deriving a fresh credit from resource properties would invent energy. Only realized graph bonds with recorded `bond_energy` can currently release stored structural energy.
- This is an important boundary for unification: **ordinary chemistry may not treat every `PhysicalMaterialBond` as a funded graph bond.** A future shared reaction interface must carry a bond's energy authority/provenance, or represent an explicitly unfunded connection whose BREAK only changes topology. `ResourceProperties::potential_energy` is a type-level ceiling, not per-instance stored energy.

**Transition / rollback findings**
- Starting a stored BREAK removes the source entry from `organism.stored_material` and retains it on an `ActiveTransformation` associated with `organism_id`. This is an escrow/operation holder, not a second organism. The simulation's preparation-failure branch calls `restore_failed_transformation_material`, which attempts to put the exact source back into parent storage.
- The successful stored-BREAK resolution attempts to store each split piece back in parent storage. If that fails, it attempts a world-field deposit and ignores the boolean result. The fallback uses the piece's first stored placement; stored placements are intrinsic-frame coordinates, while `owner_relative_origin` carries the owner-relative frame. Therefore this fallback is not proven to preserve parent ownership or world placement, and a failed deposit could silently lose a piece. This is a concrete failure-path candidate for the later transaction matrix, not evidence that ordinary valid stored BREAK fails.
- A malformed/incomplete pending BREAK that reaches resolution without `prepared_energy` clears `active_transformation_id` and returns; it does not visibly restore the escrowed source in that branch. Normal preparation failure has a restoration helper, but malformed persisted/incomplete-operation recovery is not proven. Audit the active-transformation material oracle and error return contract before choosing a fix.
- `restore_failed_transformation_material` itself ignores a failed environment deposit in its fallback. Any implementation should prefer returning the original holder on error over best-effort depositing at an intrinsic coordinate.

**Decision / next step**
Do not add energy to stored bonds or broaden chemistry enumeration yet. First define the common bond/reaction contract so graph bonds with recorded energy and physical-material connections without allocated energy cannot be confused. Then test stored BREAK as an atomic ownership transition: original storage → active-operation escrow → all resulting pieces back to the parent (or an explicitly approved physical transfer), with exact material inventory, valid geometry/endpoint topology, owner-relative placement, and rollback on every rejected step. Apply the same contract to committed storage only after its actual use paths are traced. This remains an audit finding; no production or test change is authorized until all audit templates are complete.

**Next audit:** trace all transitions that move `PhysicalMaterial` between environment, parent storage, committed storage, active transformation escrow, and developing structure; verify the independent material oracle counts each state once and that failed transfers retain the source.


### 17.34 Phase 8 material-holder transition audit — independent oracle coverage and transfer atomicity — 2026-10-22, PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`

**Scope / evidence:** Exact-head review of the independent test oracle in `src/tests/simulation_tests.rs`, the contained-material transfer in `src/simulation/simulation.rs` and `src/simulation/environment.rs`, stored-material restoration/COMBINE in `src/chemistry/combine_runtime.rs`, and construction transfer in `src/organism/reproduction.rs`. Source-only audit; no code or tests changed.

**Independent material oracle — confirmed coverage**
`integer_material_inventory` walks environmental logical/physical stock; each living or breached organism's structural units and storage; each construction's committed storage and developing graph; physical sources escrowed in active transformations; and legacy decomposition-carrier structures. It validates catalog identities, finite positive whole constituent amounts, and checked integer totals, without reading energy-ledger totals. Prepared/trial copies are deliberately excluded to avoid double counting. This is the correct independent material oracle for holder-level regressions. The separate local helper `rollback_material_inventory` in `simulation.rs` is narrower (main graph, parent storage, field only) and must not be reused for a scenario containing reproductive construction, active transformation escrow, or legacy carriers.

**Transition paths and current assessment**
- **Environment → parent storage:** `take_contained_physical_materials_in_regions` drains candidate cell vectors and returns fully enclosed realized objects; caller then attempts owner-anchor storage. If storage rejects the object, caller tries to deposit the original object at its original world placement. The material is not lost on the normal path, but both this fallback and the stored-BREAK fallback ignore a failed `deposit` result. This leaves an unproven failure/rollback boundary; the transition is not atomic under a failed destination deposit.
- **Parent storage → main graph (COMBINE):** candidate geometry is restored into a cloned/trial graph first. Only after candidate checks does the code take the physical source from storage and assign the trial graph plus candidate energy/ledger. This ordering is substantially safer: source debit follows validation, and the remaining commit consists of direct assignments. The delayed transformation variant additionally stages prepared structure/storage/ledger deltas and has an explicit rollback for incomplete prepared COMBINE state.
- **Parent storage → construction anchor:** reproduction clones parent storage and constructs a trial anchor/developing graph before taking the selected source from a cloned parent storage. It then publishes the parent storage and `ReproductiveConstruction` together. The transfer is accounted by the independent oracle. This remains existing reproduction behavior, not approval to change the division/reproduction lifecycle in PR #205.
- **Parent storage → active BREAK escrow → parent storage:** the independent oracle counts the escrow source exactly once. Preparation failure attempts to restore the original object. Successful split-piece storage still has the fallback risk recorded in Section 17.33. A complete failure matrix must include malformed/incomplete persisted transformation states and destination-deposit rejection, not only ordinary success.
- **Parent storage → construction committed storage/developing graph:** construction uses trial child state and transfers selected parent storage entries only after a candidate is formed; the independent oracle sees committed and developing holdings. Because this code also mutates ledger/energy and may return statuses after transfer, the exact ordering must be tested as a whole-holder transaction, especially all early-return branches.
- **Construction finish / division:** `finish_reproduction` takes the construction holder and returns a child whose graph, committed storage, usable energy, and debt come from that holder. This is a true ownership-boundary transition and remains a separate approved future lifecycle gate; do not expand PR #205 into redesigning reproduction/division without owner authorization.

**Audit conclusion**
The test oracle has the necessary holder coverage, but existing regressions do not yet demonstrate that every holder transition is atomic under destination failure. The most concrete failure class is source removal followed by a best-effort deposit whose boolean result is ignored; the stored-BREAK fallback also risks using intrinsic placement as world coordinates. Fixing this should not mean blindly making the parent own an inaccessible object or inventing a new decomposition holder. The transaction should return/retain the exact source on failure, or commit a clearly defined physical transfer only after destination validation.

**Next audit:** map each transition's source removal, trial construction, ledger settlement, final destination commit, and rollback branch into a matrix; enumerate every early return after source removal in stored BREAK and reproductive construction. Identify all holder changes with a possible ignored failure before proposing a single focused fix. No code/test changes until the audit templates are complete.


### 17.35 Holder transition matrix — commit boundaries and unresolved error returns — 2026-10-22, PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`

**Audit method:** Follow source removal, trial state, energy settlement, final publication, and every failure return in the audited paths. The independent oracle is `src/tests/simulation_tests.rs::integer_material_inventory`; it counts authoritative holders exactly once, including active transformation escrow and reproductive construction. No code/tests changed.

| Transition | Observed commit order | Audit status |
|---|---|---|
| Environment → parent storage (contained physical acquisition) | Field removes fully enclosed items first; caller attempts owner-anchor storage; on rejection it tries to deposit the original physical item at its original world placement. | **Failure gap:** destination deposit result is ignored. If both storage and field deposit reject, the source is no longer in an authoritative holder. |
| Parent storage → main structure (immediate COMBINE) | Geometry/contact and energy are prepared on trial copies; physical source is taken only after the candidate succeeds; then graph, energy, and ledger are assigned. | Ordering is mostly atomic after validation; add oracle checks for rejected/stale source and any fallible step after debit. |
| Parent storage → active stored-BREAK escrow | Source is removed from storage and held in `ActiveTransformation.stored_material`; oracle counts that escrow exactly once. | Valid temporary ownership representation. Preparation failure tries to restore the exact source; its fallback deposit result is ignored. |
| Stored-BREAK escrow → split pieces | Topology split is validated, zero-energy transaction is settled, then each piece is stored; rejected storage falls back to field deposit using the first piece placement. | **Failure gap:** partial commit is possible if some pieces store before a later piece fails; fallback can use intrinsic coordinates and ignores deposit failure. Missing prepared energy or an unexpected split failure clears the active ID and returns without an evident escrow restore. Stage all pieces and commit the entire result atomically. |
| Environmental physical source → COMBINE escrow | Trial COMBINE is evaluated first; source is removed from the real field only after trial success and stored on the transformation. Incomplete prepared COMBINE resolution has explicit ledger/source rollback. | Better staged contract. Verify exact source identity, cell validity, and rollback under corrupt persisted prepared state. |
| Parent storage → committed/developing construction | Candidate offspring and candidate ledger are prepared from clones. If parent material is selected, the parent entry is removed before candidate ledger and construction holders are published; no expected fallible operation follows those assignments. | Normal ordering is coherent; test source disappearance/stale index and reconcile graph + committed storage + developing energy as one holder transaction. |
| Construction → new organism | Tick calls `finish_reproduction` for `Ready`, `Detached`, or `Dead` status; the function takes construction and returns an organism carrying the developing graph, committed storage, and developing energy. | This is a lifecycle ownership boundary, not ordinary chemistry. The currently implemented trigger/size policy is not the owner-approved future 100% → 160% → two ~80% division design. Keep the redesign out of PR #205; preserve as a separate explicit lifecycle gate. |

**Priority finding:** the narrowest confirmed cross-domain defect candidate is failure atomicity where a removed physical source is followed by an ignored destination-deposit failure, especially multi-piece stored BREAK. The correct repair should prevalidate the destination or stage the entire destination holder and commit once; if that cannot succeed, return the original source/complete trial state to the authoritative owner. Do not silently discard a source, and do not treat a failed transfer as successful merely because its energy transaction is zero.

**Audit boundary:** this matrix is source analysis, not a claim that every scenario is reproduced by a test. Before selecting the fix, complete a targeted failure-injection matrix for (1) storage rejection, (2) field-deposit rejection, (3) later split-piece rejection after an earlier piece was accepted, (4) missing/corrupt prepared transformation state, and (5) stale parent storage index. Each case must compare the independent material oracle, geometry/topology, owner-relative placement, and energy ledger before/after.

**Next audit:** inspect the exact failure-injection hooks and current regression coverage for stored BREAK, contained-material transfer, and construction transfer. Identify which matrix cases can be tested without production changes and which require a narrow test seam after all audit templates are complete.


### 17.36 Failure-injection coverage audit — existing tests versus transition matrix — 2026-10-22, PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`

**Existing coverage verified**
- `integer_material_inventory_conserves_constituents_across_field_to_storage_transfer` checks successful environmental containment transfer, and `integer_material_inventory_conserves_constituents_across_storage_restore` checks successful storage restoration.
- Invalid restoration connection topology and non-finite placement tests assert that rejection preserves the source structure/storage and the independent inventory.
- `break_action_starts_a_transformation_before_resolution` proves that stored material moved from parent storage into active transformation escrow is counted exactly once.
- `stored_material_break_does_not_credit_unallocated_bond_energy` tests the preparation/energy contract: the source bond has no allocated energy and no usable-energy, structural-energy, work, heat, or source credit is created.
- A simulation-level failed-stored-BREAK test invokes the preparation-failure restoration helper and compares a narrow inventory before/after. Its local oracle intentionally counts only the main graph, parent storage, and field; the full independent oracle in `src/tests/simulation_tests.rs` is the appropriate oracle for cases with construction or active escrow.
- Reproductive tests cover waiting and later resumption when material becomes available, and the independent oracle checks the anchor transfer into developing construction. They do not inject stale source indices or a post-transfer failure.

**Coverage gaps — no evidence yet for these matrix rows**
1. Storage rejects a split piece after another piece has already been accepted: no atomic multi-piece rollback regression found.
2. Parent storage rejects a piece and field deposit also rejects: no regression verifies the source is retained rather than lost.
3. Containment transfer drains a field source, then both storage and fallback deposit reject: no failure-injection regression found.
4. A stored BREAK reaches resolution with missing/corrupt `prepared_energy` or an unexpected `break_internal_bond` failure: no regression verifies the original escrow is restored.
5. Construction selects a parent storage entry that becomes stale/rejected between trial and commit: no regression verifies parent storage, construction, energy, and ledger all remain unchanged.
6. Destination rejection after a COMBINE source has been reserved: rollback tests exist for incomplete prepared COMBINE state, but this audit did not establish coverage for every exact source-cell/destination failure variant.

**Test-seam assessment**
- Cases 4 and stale-source cases can be tested by constructing malformed/stale state through existing internal test access; a new production seam is not automatically necessary.
- Cases 1–3 need deterministic rejection at the storage/deposit boundary. Existing APIs reject invalid physical material or out-of-world coordinates, but they do not expose a safe deterministic way to force a later piece failure while earlier pieces are accepted. A narrow test-only injected destination callback/policy may be warranted **after all audit templates are complete**. Do not add that seam yet.
- Every failure regression should compare the full independent integer material oracle plus exact holder/geometry snapshots and the relevant energy/resonance ledger. A test that checks only total amount or a local inventory is insufficient.

**Conclusion:** Normal-path and selected rollback tests exist, but the highest-risk failure atomicity cases in Section 17.35 remain untested. The next step is not a code change: complete the remaining transition and energy-holder audits, then choose the smallest test seam and production fix together. No source or test files changed.


### 17.37 World-tone receiver mapping audit — parent-owned physical holdings versus resonance capture — 2026-10-22, PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`

**Scope / exact-head evidence:** Read the live tick at `src/simulation/simulation.rs`, the complete receiver mapping and settlement loop in `src/simulation/world_tone_snapshot.rs`, and the independent energy snapshot/ledger equation. This establishes current PR #205 behavior and supersedes older README-archive claims that live sunlight capture is disabled. No source/test changes.

**Current behavior**
- Live world-tone input is enabled in the inspected head. Each tick samples the precomputed `WorldToneCycle` when available, with direct sampling fallback, then calls `WorldToneOccluderSnapshot::settle_living_organism_inputs` for the current `living_ids`.
- The occluder snapshot contains primary organism structures, developing structures, committed construction material, environmental physical material, legacy carrier structures, parent-stored physical material, and active-transformation physical sources. This is one shared world geometry/path set; it does not trace a separate source ray per organism.
- Only primary organism structure bodies are mapped through `organism_body_indices` to a living organism's retained resonance reservoir. A breached/dead organism's primary structure is treated as a nonliving receiver. Developing structures, committed material, stored physical material, environmental objects, legacy carriers, and active transformation sources also have no organism-reservoir mapping.
- For each interaction path, response energy is removed from the travelling signal once. A mapped living receiver can retain the non-damped response in its resonance reservoir; a receiver without a living reservoir converts the full material response to heat. Only the unresponded remainder continues onward; escaped energy is counted at the path end. Reservoirs are staged and committed only after the world-tone ledger transaction succeeds.
- The independent energy snapshot counts current resonance in organism reservoirs, current usable energy including construction's developing energy, and graph bond energy in both primary and developing structures. It does not create a separate resonance reservoir for developing construction. The world-tone ledger reconciles captured resonance, heat, and escaped energy against incident external input.

**Potential contract gap**
The owner-approved Section 17.31 model makes the parent the sole authoritative owner of its entire physical aggregate until division, explicitly including stored physical material, committed reproductive material, developing structure/bonds, and associated energy. The live sunlight receiver map, however, treats all non-primary holdings as unowned-by-a-living-receiver physical objects: their response becomes heat even when they are physically parent-relative and still owned by a living parent. This may be an intentional rule for disconnected stored objects, but it is not documented as a deliberate distinction between ownership and energy harvesting, and it may prevent the approved aggregate from collecting resonance through its non-primary surfaces.

Do not automatically map every occluder to the parent reservoir: a source can be physically in the environment, can overlap another body's angular/path lane, or may be a disconnected object whose energy should become heat. The audit must distinguish **physical receiver identity**, **authoritative energy owner**, and **living resonance eligibility** rather than collapsing them into one index.

**Required resolution before claiming world-tone/chemistry unification**
1. Define whether all physical surfaces owned by a living aggregate can deposit compatible resonance into the parent's single reservoir, or whether only its primary structural graph is biologically eligible. This is a scientific contract question, not an implementation detail.
2. If non-primary parent-owned surfaces are eligible, map each such physical body to its owning organism without duplicating receiver hits or crediting environmental material to an owner. Preserve first-hit occlusion, one-time response removal, staged reservoir commits, and world energy balance.
3. Test an occluded/visible parent-owned stored object and a developing structure separately; compare captured resonance, heat, escape, and the independent energy inventory. Include breached parent behavior and verify it remains a nonliving heat receiver.
4. Keep source/path geometry shared and bounded; do not introduce per-organism ray tracing.

**Next audit:** inspect the approved sunlight/world-tone decisions in the plan and compare them to this receiver-eligibility behavior. Determine whether existing owner approval settles this question or whether a concise decision is required before the live capture map changes. No code/tests until the audit templates are complete.


### 17.38 World-tone contract reconciliation — prior approvals do not settle receiver ownership — 2026-10-22

**Status: Gap found; one narrow owner clarification is required before changing receiver mapping.** This section reconciles the approved D5 sunlight computation rule, D7 quantity semantics, D9 ownership/transaction rules, D12 post-death chemistry, and the owner-approved aggregate model in Section 17.31 against the live receiver mapping audited in Section 17.37. It is still documentation-only; no source or test files changed.

**What the existing decisions settle**
- D5 settles the computation strategy conditionally: preserve one source budget, occlusion/shadow behavior, frequency-shifted onward propagation (not mirror reflection), and adopt the fast first-hit/single-sweep approximation only after comparison against an exact small-world reference. D5 does **not** specify which physical holders are eligible to deposit energy into an organism's resonance reservoir.
- D7 says signal/resonance quantities must remain distinct from usable energy, stored bond energy, work, and heat. A body's material response is not itself proof that an organism can retain it.
- D9 and D12 establish that a pending physical holder remains accounted for across operations and that death changes biological eligibility, not physical existence. They do not state that all parent-owned surfaces share the same biological energy receiver.
- Section 17.31 explicitly assigns the parent aggregate ownership of its primary structure, stored physical material, committed reproductive material, developing structure/bonds, and associated energy until division. It explicitly warns that ownership does not automatically make every holder chemically available. It does not explicitly decide sunlight/resonance eligibility for a disconnected stored object or a developing structure.

**Current behavior from Section 17.37**
- The occlusion/exposure frame contains those non-primary holders as physical bodies. However, only a living organism's primary structure maps to its single retained resonance reservoir. All other bodies consume their material response as heat; dead primary structures also have no living receiver.
- Thus the implementation has a distinction between physical occlusion and biological capture. It may be intentional for disconnected objects, but the current decisions do not explicitly approve that distinction for parent-owned, parent-relative holders.

**Why the decision matters**
- Mapping every parent-owned body to the parent's reservoir would allow any exposed stored/developing surface to harvest compatible response. That could be scientifically appropriate for an aggregate, but it must not create duplicate receiver claims when bodies overlap, when a constituent is in active-transformation escrow, or when one physical holder is represented in more than one snapshot structure.
- Keeping capture limited to the primary graph avoids that new mapping but means a living aggregate may absorb sunlight as heat on its own stored/developing surfaces without retaining any compatible resonance. That is not prohibited by D5 or Section 17.31, but neither is it explicitly approved.
- This choice does not authorize adding a resonance reservoir to every holder. The existing approved architecture has one parent-owned aggregate and a parent resonance reservoir; any mapping must preserve one physical response debit, at most one reservoir credit, first-hit occlusion, and world-scale energy conservation.

**Narrow clarification needed**
Before modifying receiver mapping, ask the owner to choose one rule:
1. **Aggregate capture:** any physically realized surface whose unique authoritative owner is a living organism may deposit compatible response into that parent's single resonance reservoir, including stored, committed, and developing holders. Environmental material, dead aggregates, and ambiguous/duplicate ownership never capture into a living reservoir.
2. **Primary-structure capture:** only the parent's primary structural graph may deposit into the resonance reservoir; other physical holders still occlude and respond, but their material response is heat, even if parent-owned.

Do not silently infer either rule from D5's approximation decision or Section 17.31's ownership decision. Keep the current live path intact pending clarification; no runtime toggle or code change in PR #204.

**Next audit while awaiting clarification:** complete the source-to-sink energy accounting matrix independently of this eligibility choice: incident source allocation, material response, damping, reservoir credit, downstream propagation, escape, heat, and ledger settlement. Separately inspect active-transformation snapshot de-duplication and owner resolution so the later chosen rule can map each physical body to no more than one owner. No code/tests until the audit templates are complete.


### 17.39 World-tone source-to-sink ledger audit — local frame balance is present; independent global proof remains open — 2026-10-22, PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`

**Status: Local source-frame conservation guard confirmed; whole-world sunlight validation remains open.** Exact-head source review of `src/simulation/world_tone_snapshot.rs`, `src/simulation/world_tone.rs`, and `src/chemistry/energy_ledger.rs`. No source/test changes.

**Live accounting path**
1. The snapshot builds realized physical bodies for primary organism structures, developing structures, committed material, environmental physical objects, legacy carriers, owner-relative stored material, and sources escrowed by active transformations. A pending transformation is included only when its owner exists and its ID matches the owner's active transformation; ambiguous simultaneous stored/environmental source fields fail closed. This guards a class of duplicate source representations at snapshot construction.
2. Each interaction path derives incident energy from its projected width. The signal spectrum starts with amplitude (sqrt{E}); the code checks spectrum energy is finite, and a material response may not exceed incoming signal energy beyond tolerance.
3. The material response is partitioned into retained compatible resonance and damping heat for a mapped living primary receiver. If the reservoir cannot accept the full transformed spectrum, the candidate reservoir is discarded and the entire response becomes heat. A physical receiver without a living-reservoir mapping also turns its entire response into heat. The unresponded signal budget continues through the remaining path, with frequency composition transformed by each interaction; final remainder is escaped energy.
4. Every path validates `incident = captured + heat + escaped` within its stated tolerance. The frame sums the totals and calls `EnergyLedger::settle_world_tone_paths`, which independently validates nonnegative finite totals and the same balance before adding incident energy to external input, heat to cumulative heat, and escape to cumulative escaped tone. Reservoirs are staged for the whole frame and committed only after ledger settlement succeeds. This is a meaningful atomicity and local-balance guard.

**Important limits / open findings**
- The ledger's world-tone settlement is a throughput/accounting check, not an independent geometry oracle. The incident total is computed from the same snapshot's projected interaction paths that determine hit order and capture; the current direct-width cross-check validates only living primary-structure units. It does not independently prove the unique projected-width/source budget of every non-primary receiver, or that the fast sweep approximates the intended exact geometry within an accepted error bound.
- Source budget semantics are intensity per tick per unique exposed projected width, not a single finite global sun pool. Therefore the critical duplication test is not “incident energy must be below a fixed world total”; it is that the sweep partitions the exposed width exactly once, respects first-hit occlusion, and does not count a projected interval/path twice. D5 requires comparison to the exact small-world reference; this remains unverified.
- The snapshot includes parent-owned non-primary holdings as occluders/receivers but maps only primary organism bodies to a living reservoir. Section 17.38 records why this is an unresolved scientific receiver-eligibility decision. Do not classify the accounting implementation as wrong solely because it follows the current primary-only mapping.
- `total_world_tone_energy_unabsorbed` is not incremented by this frame-level `settle_world_tone_paths` call; the frame instead records total incident input, heat, and escaped energy, while retained energy lives in organism reservoirs. Its role appears to be separate diagnostic throughput in other paths; verify all writers/readers before deciding whether the omission is intentional. It is not included as a separate term in the independent inventory equation, so it must not be treated as a second energy store.
- The snapshot represents active transformation escrow bodies once in its own physical-body list, but this audit did not yet prove that no other independent inventory/snapshot counts the same physical source both in its owner storage and in escrow. The test-only material oracle and world-tone geometry snapshot must be compared on identical pending-operation scenarios.

**Next evidence needed**
1. Exact small-world comparison of projected path intervals/first hits against an independent geometric reference, including overlapping bodies, ties, boundary angles, fluid geometry, stored/developing surfaces, and pending transformation sources.
2. Frame-level tests for zero/invalid width, invalid material response, reservoir overflow, stale/ambiguous active transformations, and ledger rejection proving neither partial reservoir commits nor partial ledger totals occur.
3. Cross-check incident width and unique receiver/body identities against an independent test oracle; include owner-relative placements and escrow so one constituent cannot be represented twice.
4. Establish representative workload runtime/memory and approximation error before setting numerical gates, per D14. Keep per-object tracing out of production absent evidence that the accepted fast sweep cannot meet the required bound.

**Next audit:** compare the test-only independent material/energy inventory against active-transformation escrow and owner-relative physical-body snapshot construction; inspect every writer/reader of `total_world_tone_energy_unabsorbed` and establish whether it is diagnostic-only. Keep the receiver ownership question from Section 17.38 explicit; no implementation changes until the audit templates are complete.


### 17.40 Independent inventory / escrow cross-check — constituent source is counted once; energy snapshot has a defined scope — 2026-10-22, PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`

**Status: Test-only constituent oracle coverage confirmed; complete physical-transition conservation is still open.** Exact-head review of `src/tests/simulation_tests.rs`, `src/simulation/simulation.rs`, and `src/simulation/state.rs`. No source/test changes.

**Material oracle findings**
- `integer_material_inventory` walks environmental logical stock and realized physical material; all primary organism units and storage; committed material and developing units; active transformation escrow sources; and legacy carrier units. It validates known resource names and positive, finite, integral constituent amounts, uses checked integer addition, and does not read energy-ledger totals.
- For active transformations, the oracle intentionally counts `stored_material` and `combine_environmental_source` as the source holders while not counting prepared structures/storage/ledger fields, which are trial copies. The existing regression checks that starting a stored BREAK moves its physical source from parent storage into active-operation escrow without changing the independent constituent inventory.
- This is a sound source-of-truth split for the tested cases. It does not itself prove every completion, cancellation, failed deposit, malformed pending state, or stale owner path returns exactly one source; Sections 17.34–17.36 retain those open rollback gates.

**Energy snapshot scope**
- `EnergyInventorySnapshot` recomputes current usable energy from organism usable energy plus reproductive `developing_energy`, current resonance from organism reservoirs, and structural energy from recorded bond energy in primary and developing graphs plus any legacy carriers. It does not use cached `total_*_held` fields to derive those current totals; the test poisons those caches to demonstrate independence.
- The snapshot intentionally has no energy value for `PhysicalMaterialBond` internal connections because those connections carry topology/endpoints but no allocated `bond_energy`; see Section 17.33. This is not an omission of an existing numeric holder. It remains a contract gap to decide which physical connections are eligible for ordinary chemistry, not permission to invent stored energy.
- The inventory equation treats `total_world_tone_energy_unabsorbed` as diagnostic throughput only, not a stored-energy term. Current state documentation says so explicitly. The world-tone snapshot uses frame-level incident/captured/heat/escaped totals and does not update this telemetry field; other world-tone settlement paths do update it. Because it is not an inventory term and does not authorize spending, this difference is not currently evidence of energy loss, but telemetry consistency across capture implementations remains a separate cleanup/observability question.
- The world-tone snapshot's escrow inclusion is separately guarded by matching owner and active-transformation IDs and rejecting ambiguous dual-source state. The material oracle independently counts the same escrowed material once. Cross-checking these rules supports the no-duplicate physical source design for this representation; a pending-operation end-to-end geometry + inventory regression is still required.

**Result / remaining blockers**
The material oracle and world-tone snapshot both explicitly account for active-operation source escrow without treating prepared trial state as another physical source. The energy snapshot is independent of ledger-held cache totals for its defined holder scope. These are valuable foundations, not proof of whole-world energy conservation: exact geometry/source allocation, transaction completion/rollback, all energy consumers, and the approved receiver-eligibility rule remain open. Do not broaden the snapshot's energy terms to count telemetry or topology-only connections as energy.

**Next audit:** finish the holder-transition matrix for completed/cancelled/stale transformations and construction ownership, then reconcile the live tick phase ordering against each transaction boundary. No implementation change until the remaining audit templates are complete.


### 17.41 Live tick-order audit — physical chemistry persists after breach; eligibility boundary is phase-specific — 2026-10-22, PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`

**Status: Production order mapped; cross-phase conservation and same-tick policy coverage remain incomplete.** Exact-head review of `Simulation::step` and `apply_survival_damage` in `src/simulation/simulation.rs`. No source/test changes.

**Observed phase order**
1. Increment tick.
2. Advance active transformations; prepare due operations. On preparation failure or missing owner, call source-restoration helper. Remove failed operations from active list. Resolve completed transformations before determining current living IDs.
3. Recompute the living set from intact genome cores after completed physical transformations.
4. Sample world tone and settle the frame against that living set.
5. For each organism in collection order: if currently living, apply maintenance, update harmonics and experience memory, and possibly begin reproductive construction; then run the ordinary structural chemical accumulator/resolver regardless of living status.
6. Recheck core integrity immediately after chemistry and remove a newly breached organism from the living set before later behavior. If still intact, transfer contained environmental material and finalize pending movement experience.
7. Process movement and decision/action work only for living organisms. Existing physical/chemical transformations are retained across death; new actions are gated by the living set.
8. Advance reproductive construction only for organisms still in the living set; finish and append offspring when construction reports ready/detached/dead.
9. Apply survival/stress damage to organisms still in the living set. Stress damage may trigger resonance-overload BREAK; death is not routed through the legacy decomposition path.
10. Refresh cached held-energy totals for usable, resonant, and structural stores.

**Conclusions supported by source**
- A completed transformation can breach the core before world-tone capture; such an organism is absent from the living receiver set for that frame.
- A core breach caused by ordinary chemistry occurs after world-tone settlement and after the living organism's maintenance/harmonics/memory/reproduction-start block for that tick. The immediate post-chemistry recheck prevents later movement/actions/construction advancement/stress processing for that organism in the same tick.
- Dead/breached structures remain in the collection and continue through ordinary chemistry each tick. In-flight transformations are not silently dropped when their owner becomes breached.
- This phase order creates an intentional-or-unresolved same-tick distinction between “breached before the living set is sampled” and “breached by chemistry later in the tick.” D9 explicitly requires a defined same-tick biological eligibility boundary, but source inspection alone does not establish that tests cover every boundary.

**Remaining phase-order risks**
- A world-tone frame runs before this tick's ordinary chemistry, so a core that chemistry opens later in the tick can still receive resonance during the earlier world-tone phase. Conversely, a completed transformation that opens the core first suppresses living capture that tick. Confirm that this ordering is the intended biological eligibility contract; do not alter ordering merely to make the two cases look symmetric.
- Each organism's chemistry is resolved in collection order, and one organism's chemistry may alter its own living status before later phases; the tick does not appear to recompute all organisms' geometry or a single global chemistry event queue between every event. Stale event and cross-organism contact assumptions need test coverage.
- Construction advances after movement/actions and only for currently living parents. The developing graph and committed material remain parent-owned under Section 17.31, but their reactions/capture eligibility are not derived from this phase ordering.
- Held-energy caches are refreshed at the end of the tick; independent inventory snapshots must inspect live holders and not assume those cache totals are authoritative during intermediate phases.

**Required regression scenarios**
- Breach during a completed transformation before world-tone; breach during ordinary chemistry after world-tone; breach during end-of-tick stress processing.
- Dead parent with pending stored BREAK/COMBINE escrow through success, failure, and cancellation, preserving exact material inventory and energy snapshot.
- Multiple organisms where one organism's chemistry changes status while later organisms still have queued physical contacts; deterministic same-seed replay.
- Construction with committed/developing holdings at each boundary; no biological advancement after breach and no death-time owner transfer.

**Next audit:** trace transformation cancellation/stale-state and construction-finish transfer paths against this phase table; mark each source/holder transition as atomic, guarded, or unproven. No implementation until audit templates are complete.


### 17.42 Transformation escrow and construction ownership transitions — happy paths staged, failure recovery remains non-atomic in specific branches — 2026-10-22, PR #205 head 12d8079230458da68a8dc3755a1d6a9b41347862

**Status: Transition paths mapped; failure-atomicity gap confirmed.** Exact-head review of src/simulation/simulation.rs, src/chemistry/transformation.rs, and src/organism/reproduction.rs. No source/test changes.

**Active transformation lifecycle**
- Starting stored BREAK removes one physical instance from parent storage and holds the exact source on ActiveTransformation; the independent material oracle counts it from escrow. COMBINE can reserve an environmental source by exact cell index and records the source in the pending transformation.
- At tick start, the scheduler decrements duration and prepares at the operation-specific boundary. If preparation fails or the owner no longer exists, restore_failed_transformation_material attempts to return the exact source. For COMBINE environmental sources, the helper restores to the original cell index. For stored BREAK, it first tries parent storage; if that fails or the owner is missing, it attempts a field deposit at the source's first placement and ignores the deposit result. This fallback is not a proven world-space position because stored geometry may be owner-relative.
- COMBINE preparation builds candidate physical/energy/ledger state, and its missing-prepared-state resolution branch explicitly rolls back staged energy/ledger deltas and restores an environmental source to the exact original cell. The normal COMBINE commit copies prepared structure/storage and clears the active marker.
- Stored BREAK preparation validates the split and settles a zero-energy transaction because the stored connection has no allocated bond energy. Resolution then splits the escrowed object and attempts to return every piece to storage; on a storage rejection it attempts field deposit and ignores failure. The loop commits pieces incrementally, so a later rejected piece can leave earlier pieces stored and the remainder lost if fallback also fails. A malformed completion with missing target/prepared state clears the active marker without visibly restoring escrow. These are concrete failure-atomicity gaps, already reflected in Sections 17.34–17.36.

**Reproductive construction ownership transitions**
- begin_reproduction creates candidate anchor structure/storage from a cloned physical instance, then removes the exact source index from a cloned parent storage and commits parent storage plus the new ReproductiveConstruction together. This normal-path transition is staged, and the anchor is not intentionally duplicated.
- advance_construction transfers reserve energy from parent usable energy to the construction's developing-energy holder through the ledger; it builds candidate child state and a candidate ledger for construction work. If a parent storage source is transferred into the child candidate, it attempts to remove the original from parent storage before committing candidate ledger and construction. A failed removal returns Waiting without committing the child candidate, but the code should be covered by a stale-index/source-mutation test.
- On successful finish, finish_reproduction takes the construction out of the parent and creates a child whose usable energy, storage, and developing structure are moved from those holders. The simulation appends the returned child after iterating parents. This is a distinct ownership-transfer boundary; test inventory immediately before and after the complete simulation step, not only after the child is appended. The current reproduction completion rules do not implement the approved future 100%→160%→two-80% division specification; that work remains separate and must not be folded into chemistry PR #205 without approval.

**Risks / required coverage**
1. Inject failed storage and failed field deposit for every piece of a stored BREAK; require either all pieces returned to one valid owner/location or the original source restored, with no partial commit.
2. Exercise malformed pending BREAK state (missing target, missing prepared marker, invalid source) and prove escrow remains represented/recoverable rather than clearing its only active-operation marker.
3. Exercise missing owner and failed storage restoration with owner-relative stored placements; no silent fallback to intrinsic coordinates.
4. Exercise COMBINE prepared-state corruption and exact-cell restoration, including ledger/energy rollback; compare independent energy/material inventories.
5. Force stale construction source indices and failures after reserve transfer but before child commit; compare parent usable energy, developing energy, committed material, graph structure, and constituent inventory.
6. Compare parent+construction inventory before finish, during the same step after finish returns, and after child append. The production path should be atomic at externally observable tick boundaries; test-only intermediate probes may verify staging without changing live semantics.

**Decision:** These are implementation defects/coverage gaps, but do not patch them until the full audit register is complete. The eventual smallest fix should replace best-effort partial deposits with an all-or-nothing holder transfer that preserves exact ownership and coordinates, and provide deterministic failure injection only where existing interfaces cannot exercise rejection. Keep reproductive division itself out of this chemistry fix.


### 17.43 Transformation terminal-state matrix — terminal ownership is not total on malformed/rejected BREAK paths — 2026-10-22, PR #205 head 12d8079230458da68a8dc3755a1d6a9b41347862

**Status: Gap found.** This matrix consolidates the exact-head source paths; it is not a runtime test result. No source/test changes.

| Operation state / terminal route | Source/material disposition observed | Energy/ledger disposition | Audit status |
|---|---|---|---|
| COMBINE completes with prepared structure/storage/stress | Prepared graph and storage commit; environmental source is consumed into the prepared result; active marker cleared | Preparation's staged usable/ledger changes remain committed | Normal path staged; needs inventory integration test |
| COMBINE missing prepared physical state at resolution | If an environmental source exists, source is restored to exact cell when that cell still exists; active marker cleared | rollback_prepared_combine reverses prepared usable/ledger deltas that are still represented | Guarded, but invalid/missing source cell recovery is not proven |
| COMBINE preparation rejects or owner is absent | Restoration helper returns environmental source to exact source cell if present | Preparation failures occur before prepared ledger changes are committed; exact-head tests should assert this | Normal rejection path exists; failure injection incomplete |
| Stored BREAK preparation rejects | Exact source is put back in parent storage if possible; otherwise helper attempts field deposit using first stored placement and ignores result | No structural energy is created; stored connection has no allocated bond energy | Best-effort recovery; owner-relative fallback not proven |
| Stored BREAK succeeds and all output pieces store | All disconnected pieces return to parent storage | Zero-energy transaction; topology changes without fabricated energy | Normal path covered, but no full failure matrix |
| Stored BREAK has one or more output-piece store rejections | Each piece is handled sequentially; a failed store attempts field deposit and ignores its result. Earlier pieces may already be committed | No stored bond energy released | **Non-atomic; potential material loss/partial transfer** |
| Stored BREAK completion lacks source, target, prepared marker, or valid split | Resolver clears active marker and returns; no explicit restoration of the source is visible | No additional energy settlement at that point | **Escrow can become unowned when completed transformation is dropped** |
| Transformation owner is absent at prepare/completion boundary | Scheduler attempts the generic source-restoration helper; no owner storage is available | COMBINE source may return to exact environment cell; stored BREAK relies on best-effort placement | **Stored-source terminal ownership not guaranteed** |
| Reproduction construction waits / candidate rejected | Candidate child/ledger state is not committed; parent-held source remains authoritative | Candidate ledger is discarded | Mostly staged; stale-index failure needs explicit test |
| Reproduction construction successfully finishes | Parent construction is taken and its developing graph, committed storage, usable energy and genome become a child appended later in same step | Internal energy/material transfer; no external input intended | Whole-step inventory test required |

**Matrix conclusion:** the most severe immediately actionable gap is not a missing equation but a missing all-or-nothing terminal ownership rule for stored BREAK. The operation must not clear its only active-operation marker until it has either committed all resulting pieces to a valid destination or restored the original source. Environment fallback must use a valid world-space transform and must return a checked result; a failed destination must retain or restore the source. Similarly, a failed COMBINE source return must remain represented for recovery rather than silently disappearing. This is a focused candidate for implementation after the audit register closes, not authorization to code now.

**Required closure evidence:** for every row, a test must compare the independent constituent inventory, exact holder ownership, physical geometry/placements, usable/resonant/structural energy, cumulative ledger deltas, and active-operation markers before and after both success and injected failure. The current normal-path and selected rollback tests do not satisfy this matrix.

**Next audit:** enumerate construction transfer candidate failures and movement/maintenance/resonance energy paths in the same source→validation→commit→rollback schema. No implementation until all audit templates are complete.


### 17.44 Movement, maintenance, and resonance-break transaction audit — movement can commit geometry before energy settlement — 2026-10-22, PR #205 head 12d8079230458da68a8dc3755a1d6a9b41347862

**Status: Gap found.** Exact-head source review of src/perception/movement.rs, src/simulation/state.rs, src/chemistry/transformation.rs, and src/chemistry/energy_ledger.rs. No source/test changes.

**Movement path**
- start_movement chooses direction and distance using organism state but does not debit energy. Each actual step computes a one-unit movement cost and checks available usable energy before attempting geometry.
- advance_movement calls try_move_cell_with_reason first. That routine mutates the organism's position/occupancy and the spatial/environment state on success. Only after the physical move does advance_movement submit an EnergyTransaction with usable_delta = -cost and heat_dissipated = cost.
- If ledger settlement rejects (for example, invalid/overflowing cumulative ledger state), advance_movement returns InsufficientEnergy but does not roll back the already committed move. The code records the new position in diagnostics and leaves active.remaining_steps unchanged, so a later retry may move again without charging the failed step. The precheck makes ordinary insufficient-energy failure occur before geometry, but does not make the post-move ledger failure atomic.
- This is a concrete transaction-boundary gap. The smallest repair must either stage the movement on candidate geometry and commit only after ledger acceptance, or provide a reliable rollback of all position/occupancy/spatial-index/environment changes when settlement fails. A test-only ledger-rejection seam may be needed if a deterministic invalid-ledger fixture cannot exercise this safely.
- Movement currently classifies its full cost as heat and records zero explicit work. That is an existing model choice requiring reconciliation with D7's distinction between work and heat; do not change the split without tracing the movement cost equation and owner-approved units. At minimum, document whether the cost is intended to be dissipative locomotion cost or physical work delivered to an external body.

**Maintenance path**
- Maintenance computes demand from structural mass. It attempts a balanced transaction that debits usable energy and records the same amount as heat; stress is added only after settlement succeeds. A rejected transaction leaves the energy holder unchanged and the unpaid amount becomes maintenance debt. This path does not exhibit the same geometry-before-ledger ordering defect in the inspected function.
- Reproductive construction calls the same maintenance method on a temporary developing-organism view. The resulting energy/stress/structure values are copied back to construction state. Because this child is a temporary projection of the parent-owned developing holders, integration tests must ensure its graph energy and developing-energy value replace the old values once, not coexist as duplicated holders.

**Resonance-overload BREAK path**
- The function clones the resonance reservoir, withdraws compatible energy on the clone, validates a structural bond candidate, and settles the bond BREAK through the shared transaction path. Only after settlement succeeds does it assign the cloned reservoir back to the organism. A failed bond/ledger settlement therefore does not commit the resonance withdrawal. The structural BREAK itself clones/validates the candidate graph, settles the ledger, then commits graph mutation; this ordering is transactionally stronger than movement.
- The same BREAK helper handles stress/overload-driven physical damage; there is no independent random break path. Remaining work is to compare its ledger semantics against chemical BREAK and the independent energy oracle across successful and rejected transactions.

**Required tests / next audit**
1. Force movement geometry success followed by ledger rejection; assert position, occupancy, environment/spatial index, movement progress, usable energy, heat/work counters, and active movement are exactly unchanged.
2. Test maintenance settlement rejection leaves usable energy and stress unchanged while accruing only the specified debt.
3. Test resonance-overload BREAK settlement rejection preserves graph, reservoir, stress, and all ledger counters; success removes one bond and debits resonance exactly once.
4. Reconcile movement's heat-only cost with D7's work/heat contract and measure actual displacement semantics before any equation change.
5. Continue to audit other operations that mutate geometry before energy settlement; no implementation until the complete audit register is full.


### 17.45 Acquisition and expulsion atomicity audit — 2026-10-22, PR #205 head 12d8079230458da68a8dc3755a1d6a9b41347862

**Status: Gap found.** Exact-head review of COMBINE bond formation, contained-material acquisition, expulsion, and storage. No source/test changes.

- **COMBINE bond formation is a positive pattern:** validate and build a trial graph, settle the energy ledger, then commit the graph. Ledger rejection leaves the trial graph uncommitted. The enclosing multi-tick source/rollback issues remain separately open.
- **Contained acquisition has best-effort restoration:** field objects are removed as a batch, then stored one at a time. If storage rejects an object, the code tries to deposit it back at its first placement but ignores the result. A rejected fallback leaves no authoritative holder in this path. Partial success is safe only if every failed item is retained or restored exactly.
- **Expulsion has a final rejection gap:** it removes the exact storage entry, computes world placements, and returns the result of the final physical deposit directly. Unlike earlier failure branches, it does not restore the object if that final deposit rejects. Rollback must preserve the original intrinsic/owner-relative form, not the mutated candidate world placements.
- **instantiate_one_unit** follows a sounder pattern: restore into a trial graph, remove the matching source, then commit the graph; failed source removal does not commit the trial graph.

**Required tests:** force destination rejection for contained acquisition and expulsion; compare the independent integer material oracle, original geometry/owner-relative origin, field contents, and parent storage. Also verify multi-object partial acquisition leaves every rejected source represented exactly once. No source change authorized until all audit templates are complete.

**Next audit:** trace direct mutations of usable energy, resonant reservoirs, graph bonds, and environmental physical material that bypass a checked transaction/transfer helper.


### 17.46 Shared energy-consumer cross-check — identify reusable transaction patterns and prioritize confirmed gaps — 2026-10-22, PR #205 head 12d8079230458da68a8dc3755a1d6a9b41347862

**Status: Cross-domain ordering comparison complete for the inspected paths; integrated proof remains open.** No source/test changes.

| Path | Candidate validation / source handling | Ledger and physical commit order | Current conclusion |
|---|---|---|---|
| Structural chemical BREAK | Cloned resonance reservoir; exact live bond revalidated by trial graph | Ledger settles before graph replacement; reservoir commits only on success | Good local atomicity pattern; failure and world inventory tests remain needed |
| Resonance-overload BREAK | Cloned reservoir and selected bond; shared BREAK settlement | Ledger + graph commit first, reservoir assigned only on success | Good local pattern; reconcile identical bond/energy semantics with chemical BREAK |
| COMBINE bond formation | Trial graph and exact endpoint checks | Ledger settles against candidate energy before trial graph replaces authoritative graph | Good local pattern; enclosing storage/source transfer still has separate failure risks |
| Maintenance | Computes demand and attempted payment before mutation | Ledger debit succeeds before stress is added; unpaid balance becomes debt | No geometry-before-ledger defect observed in this function |
| Movement | Validates movement/cost and mutates live position/occupancy first | Ledger settlement follows physical movement | Confirmed order defect; a rejected settlement leaves an uncharged move |
| Contained acquisition | Removes contained environment sources into a returned batch | Storage attempt follows removal; failed storage relies on ignored-result deposit fallback | Confirmed best-effort ownership gap |
| Expulsion | Removes parent storage source and transforms candidate to world placement | Final environment deposit result is returned without restoring original storage on rejection | Confirmed best-effort ownership gap |
| Stored BREAK | Escrow source, then split into physical components | Components committed one-by-one; fallback results ignored | Confirmed partial-commit/escrow-loss risk |

**Design implication:** use one invariant across these subsystems: physical mutation and its material/energy consequences form one transaction. Validate on trial state; validate all destinations; settle accounting; then commit every source/destination/geometry change as a unit. Where a destination API can reject, either reserve/validate it before source removal or keep the exact original source in a recoverable escrow until commit. A best-effort fallback is not a transaction boundary.

**Prioritization for implementation after audit closure:** (1) fix source/destination atomicity for stored BREAK and material acquisition/expulsion; (2) fix movement geometry/ledger ordering; (3) add test-only rejection seams only if production interfaces cannot naturally exercise destination/ledger failure; (4) run cross-domain inventory tests for chemical BREAK, resonance overload, COMBINE, maintenance, and world tone. This is ordering by confirmed loss/partial-commit risk, not permission to begin implementation before the remaining audit templates are complete.

**Still not proven:** global energy conservation across all holders, the exact-small-world sunlight approximation/error bound, the receiver-ownership decision in Section 17.38, and the full baseline-comparison gate at the implementation head. No runtime or performance benchmark was run in this audit.


### 17.47 Ledger transaction-schema audit — stored bond release is represented correctly; generic source authority remains a guardrail — 2026-10-22, PR #205 head 12d8079230458da68a8dc3755a1d6a9b41347862

**Status: Partial contract verified.** Exact-head review of src/chemistry/energy_ledger.rs, src/simulation/state.rs, and live COMBINE/BREAK/maintenance/movement callers. No source/test changes.

- EnergyTransaction balance uses the named non-structural source counter (potential_released), usable-holder delta, signed structural-store delta, work, and heat. Current BREAK settlement correctly represents released stored bond energy as a negative structural delta; it does not also claim a second potential-release source. COMBINE stores bond energy through a positive structural delta. This keeps one bond-energy source debit per successful graph mutation.
- Live world-tone capture uses its separate external-input settlement path and credits retained energy to the resonance reservoir only after the source-frame balance validates. Initial-condition energy has its own one-time ExternalInputTransaction path. These source classes should remain distinct from structural bond release.
- The EnergyLedger transfer primitive validates both holders and computes both next values before assigning either; successful parent-to-developing-energy transfers are internal and should cancel at world scale. Callers must still check the boolean where subsequent logic depends on transfer success.
- The transaction type still has a generic potential_released scalar without a named source identity or an attached physical source debit. Current inspected live chemistry callers set it to zero; the independent inventory treats cumulative potential releases as a non-structural source term. This is not a confirmed current leak, but it is a future misuse hazard: any future nonzero caller must identify and validate its source, or use a dedicated source transaction, rather than making the counter itself authoritative evidence that energy existed.
- EnergyInventorySnapshot is a test-only oracle: it recomputes current usable/resonant/structural stores from holders, but its initial baselines and cumulative source/sink flows necessarily come from ledger history. Its balance check can therefore detect holder/accounting mismatch but cannot independently reconstruct every historical source event. Add cross-domain tests that compare actual holder deltas with event deltas and exact source provenance.

**Next audit:** inspect cache/reaction accumulation invalidation and any remaining energy-changing path not yet mapped; distinguish non-energy activation signals from actual energy holders. No implementation until the audit templates are complete.


### 17.48 Chemistry cache and reaction-accumulation audit — 2026-10-22

**Status: Persistent cache guards verified; accumulation cleanup gap found.** Reviewed current PR #205 head 12d8079230458da68a8dc3755a1d6a9b41347862. No source/test changes.

- Chemistry library schema 4 has an equation fingerprint. Loader rejects legacy schemas, fingerprint mismatch, a non-empty store without a manifest, duplicate keys, invalid records, and manifest entry-count mismatch. Insert refreshes under lock, rejects duplicates, appends/syncs a row, and atomically persists the manifest. Partial-append and manifest-failure rollback/reopen tests exist; the exact-head workflow passed the insert rollback test.
- The fingerprint includes the approved equations and parameters plus a catalog-version marker. Catalog/resource chemistry changes must keep that marker current; the string does not itself hash all runtime catalog properties.
- Reaction accumulation keys include organism ID, structure revision, bond endpoints, and interface signature. For non-empty structures, the accumulator removes stale revision keys before processing current bonds. Triggered events remove their own activation accumulation; a failed/stale BREAK does not commit its cloned resonance withdrawal.
- **Gap:** the accumulator returns immediately when the primary structure has no bonds, before cleaning old keys. If the final bond disappears while other keys from that revision remain, the entries can persist on later bondless ticks. Dead organisms stay in the collection, so this can retain obsolete state as population grows. This is not physical energy, but it is a memory/performance concern.
- Minimal fix candidate after audit completion: run stale-key cleanup before the empty-bond return, clearing that organism's entries when it has no bonds. Add a test where an organism with multiple accumulated keys loses its final bond, verifying obsolete keys disappear while another organism's entries remain. Do not change the activation/dissipation equations for this cleanup.

**Next audit:** inspect remaining energy/material mutations and cache/catalog invalidation call sites; update the dependency matrix. No implementation until audit templates are complete.


### 17.49 Cache/catalog invalidation note — 2026-10-22

The schema-4 chemistry manifest uses a fixed equation fingerprint with a catalog-version marker. ChemistryKey contains material names and interface identity, not the actual catalog property values; cached static potential depends on chemical position. Current tests prove mismatched fingerprints are rejected, but do not prove a catalog property change automatically changes the fingerprint. Phase 4 must document and test the version-bump contract, or derive a deterministic fingerprint from chemistry-relevant catalog fields. No source/test change.

**Next audit:** enumerate the exact inputs to interaction_potential and ChemistryKey so cache invalidation covers every value affecting the cached result.


### 17.50 Cache input dependency map — exact cached equation inputs confirmed — 2026-10-22

**Status: Dependency map complete; versioning remains manual.** Exact-head review of interaction_potential, contact_factor, ChemistryKey, and the live accumulator.

- The persisted `static_potential` is calculated only from `position_a`, `position_b`, `CHEMICAL_K`, and `CHEMICAL_D_MAX` through interaction_potential. `position_a` and `position_b` are resource `chemical_position` values. The live contact factor is computed separately from current candidate distance and `CHEMICAL_CONTACT_RADIUS`; engagement uses current candidate facing. These live values are not stored as the static potential.
- ChemistryKey is symmetric in the two material identities and contains schema version, material names, interface class, and interface signature. It does not include the numeric chemical positions. The manifest fingerprint is therefore the version boundary for changes to chemical positions or the static-potential equation. Keep that marker synchronized, or derive it from canonical relevant fields as recorded in Section 17.49.
- The current fingerprint also includes contact radius, max force/energy scale, and reaction dissipation. Some of these affect later live computations rather than the cached static-potential value. This is conservative invalidation (extra cache rebuilds), not a correctness defect.
- Geometry/interface identity is separately represented in the key, and live geometry remains authoritative before a cache lookup. A cache hit supplies only the static potential; it does not authorize a bond or replace live contact, endpoint, or energy validation.

**Closure gate:** add or document a deterministic test that changing a chemistry-position catalog value requires a changed cache fingerprint and rejection of the old store. Phase 4 remains open until the chosen manual/derived fingerprint policy is enforced. No source/test change.


### 17.51 Prepared COMBINE ledger-delta field audit — staged fields are broader than the commit list — 2026-10-22

**Status: Defensive consistency gap identified; currently omitted counters appear unaffected by the COMBINE candidate itself.** Exact-head review of prepare_transformation, rollback_prepared_combine, and try_combine. No source/test changes.

- COMBINE preparation computes a per-field EnergyLedger delta from the trial ledger, including `total_world_tone_energy_escaped` and `total_resonant_energy_held`. The commit path applies usable/structural/external/unabsorbed/gained/work/heat/usable-held deltas but does not apply the escaped-tone or resonant-held deltas. The rollback helper likewise omits those fields.
- The inspected COMBINE candidate currently has no world-tone or resonance-receiver operation, so those omitted deltas should remain zero on the current path. This is not evidence of a present energy leak, but it makes the staging contract fragile: a future candidate-side ledger effect could be silently dropped while the delta structure appears to preserve it.
- Closure options: narrow the delta struct to only fields COMBINE is allowed to mutate and assert all other deltas are zero, or commit/rollback every permitted field from one centralized delta application helper. Add a test that compares every EnergyLedger field before/after successful and rejected prepared COMBINE. Do not broaden the transaction's authority implicitly.

**Next audit:** compare all fields in the energy-ledger schema against initialization, end-of-tick refresh, prepared transformation commit/rollback, and inventory snapshot equations; record intentional diagnostics versus authoritative stores. No implementation until the audit templates are complete.


### 17.52 Energy-ledger field ownership map — inventory terms versus diagnostics/cache fields — 2026-10-22

**Status: Field semantics mapped; one telemetry inconsistency confirmed.** Exact-head review of EnergyLedger, EnergyInventorySnapshot, settle_transaction, settle_external_input, settle_world_tone_paths, and end-of-tick held-counter refresh. No source/test changes.

| Field | Intended role | In the independent inventory equation? | Finding |
|---|---|---|---|
| `total_external_energy_input` | Cumulative named external input | Yes, as external source | World-tone frame settlement adds incident input only after local balance validates |
| `total_potential_energy_released` | Non-structural source, distinct from bond energy | Yes, as other source | Inspected live chemistry transactions set it to zero |
| `total_structural_energy_delta` | Cumulative net bond-energy storage/release event | No; current bond holders are independently recomputed | Structural BREAK is a negative store delta; COMBINE is positive |
| `total_work_expended`, `total_heat_dissipated`, `total_world_tone_energy_escaped` | Cumulative sinks | Yes | Transaction paths validate finite next totals before commit |
| `total_world_tone_energy_unabsorbed` | Diagnostic throughput, not stored energy | No | Live snapshot path does not update it, although the legacy per-unit path does; its documented cumulative meaning is therefore incomplete for live-frame captures |
| `total_usable_energy_gained` | Cumulative positive usable-holder deltas | No | Diagnostic, not a source or a holder; retained resonance must not be counted as usable energy |
| `total_usable_energy_held`, `total_resonant_energy_held`, `total_structural_energy_held` | Cached current totals | No; snapshot recomputes from holders | Refreshed at end of tick; not authoritative mid-tick |
| `initial_*_energy_held` | Tick-zero baselines | Yes, as initial terms | Must not also credit the same initial grant as an untracked second source |
| `initial_condition_energy_settled` | One-time source guard | No | Prevents duplicate initial-condition grants; not itself energy |

**Telemetry decision needed for Phase 10:** either update `total_world_tone_energy_unabsorbed` consistently from the live frame's path accounting, or redefine/document it as legacy per-unit diagnostic only. Do not add it to the conservation equation; incident input, retained reservoir delta, heat, and escape already account for the physical energy exactly once. Add a test that live-frame settlement and diagnostic semantics agree.

**Next audit:** reconcile all fields in prepared COMBINE ledger deltas against this role map and close the remaining transfer/cache invalidation matrix. No implementation until the audit templates are complete.


### 17.53 Internal energy-transfer call-site audit — reproduction ignores transfer rejection; legacy recycling remains unsafe — 2026-10-22

**Status: Call sites mapped; one unchecked live transfer result found.** Exact-head review of reproduction advancement, EnergyLedger::transfer, and recycling helper. No source/test changes.

- `EnergyLedger::transfer` is atomic at the two-holder level: it validates both values and computes both next values before assigning either. It does not mutate cumulative energy-flow counters, which is appropriate for a pure internal transfer because the world total should not change.
- `advance_construction` computes a bounded reserve transfer from parent usable energy to construction developing energy but discards the boolean result (`let _ = ledger.transfer(...)`) and continues into maintenance/construction. In ordinary valid states, the amount is bounded by the parent balance and the transfer should succeed. If either holder is invalid or the transfer rejects, the code silently proceeds without establishing the intended reserve. Decide whether this is deliberately best-effort funding; otherwise return a waiting/failure status and prove parent/construction holders remain unchanged. Add a test for rejected transfer rather than assuming the boolean cannot fail.
- The other inspected transfer caller, `recycle_dead_organism`, checks the transfer result, but it is not safe to activate: it drains stored and committed material before checking every deposit result, removes reproductive construction, and clones the primary graph into a legacy DecomposingBody. Keep it quarantined and do not use it as the parent-aggregate death path.
- No other production `ledger.transfer` call sites were found by the current source search beyond construction reserve transfer and the legacy recycler. The ledger unit test confirms successful transfers preserve the sum of the two holders.

**Next audit:** inspect initialization and end-of-tick cached-holder refresh alongside prepared transformation deltas; ensure every cached/diagnostic field has one update policy and no cached value enters the independent inventory equation. No implementation until audit templates are complete.


### 17.54 Initialization and held-counter refresh audit — baseline/source split is internally consistent for current holders — 2026-10-22

**Status: Holder initialization and cache refresh mapped; world-scale historical proof remains partial.** Exact-head review of Simulation::new, Simulation::step end-of-tick refresh, structural_energy_held, and energy_inventory_snapshot. No source/test changes.

- Initial usable reserve is first removed from the seed organism and reintroduced through one balanced InitialCondition ExternalInputTransaction. The tick-zero usable baseline is deliberately zero, so the same initial reserve is not simultaneously treated as baseline energy and a new source. The one-time source guard prevents duplicate initial grants.
- Initial resonant and structural baselines are captured from the seed organism's current reservoir and realized graph bonds. At the end of each live tick, cached held totals are refreshed from current organism/developing holders and any legacy decomposition carriers. Structural totals include both primary and developing graph bonds; stored PhysicalMaterial connections contribute no structural-energy value because they carry no allocated bond energy.
- The test-only EnergyInventorySnapshot independently recomputes current usable, resonant, and structural stores from holders instead of reading the cached held totals. Its cumulative inputs/sinks and initial baselines still come from ledger history, so it can expose current-holder mismatches but cannot independently prove historical source provenance by itself.
- This confirms the current baseline/cache split for the represented holders. It does not close conservation across all event paths: movement's post-geometry ledger rejection, source/destination fallback failures, the COMBINE delta-field omission, world-tone receiver eligibility, and incomplete exact-geometry evidence remain open.

**Next audit:** reconcile all transaction and holder-transition findings into one source-to-sink dependency matrix with a concrete regression oracle per row. Do not start implementation until the matrix and all required audit templates are complete.


### 17.55 Consolidated source-to-sink matrix — 2026-10-22

This summary consolidates Sections 17.32–17.54. Local ordering being sound does not mean integrated conservation has passed. No source/test changes.

| Path | Source and destination | Current status / required evidence |
|---|---|---|
| Initial energy | Seed reserve → usable holder | One-time input transaction; tick-zero/no-op inventory tests remain required |
| COMBINE graph bond | Usable energy + validated endpoints → graph bond | Trial graph and ledger before commit; assert staged ledger fields not permitted to change stay zero |
| Chemical/resonance BREAK | Graph bond + compatible resonance → usable/work/heat | Trial graph/reservoir then settle; stale/retry/rejection inventory tests remain required |
| Stored BREAK | Escrowed physical composite → split pieces | **Gap:** piece-by-piece commit and ignored fallback results; malformed completion may clear escrow |
| Environmental COMBINE | Field object → active escrow → prepared graph/storage | Exact-cell restore exists; test missing source/cell and malformed prepared state |
| Contained acquisition | Field physical object → parent storage | **Gap:** failed storage and failed fallback deposit can leave no holder |
| Expulsion | Parent storage → environment | **Gap:** final deposit rejection does not restore original stored instance/frame |
| Movement | Old position → new position; usable energy → heat | **Gap:** geometry changes before ledger settlement; rejection can leave an uncharged move |
| Maintenance | Usable energy → heat; unpaid portion → debt | Settlement precedes stress update; rejection/debt tests remain required |
| Construction reserve | Parent usable → developing usable | Atomic transfer primitive, but live caller ignores failure result; clarify/test underfunded behavior |
| Construction growth/finish | Parent storage/construction → parent developing holders → child | Mostly staged; stale source and whole-step before/after inventory tests required |
| World-tone frame | Incident source → retained resonance + heat + escaped signal | Local balance and staged commit exist; owner eligibility, exact-small-world oracle, error/performance evidence open |
| Reaction accumulation | Per-organism revision/interface activation value | Not energy; **gap:** bondless early return can leave stale keys |
| Chemistry cache | Material/interface key → static-potential record | Strict schema/fingerprint checks; chemical-position invalidation test required |
| Legacy recycler | Parent stores → field + legacy carrier | Not a valid live death path; ignored deposit results; keep quarantined |

**Cross-cutting invariant:** identify one source owner; validate physical preconditions and destinations; stage material, geometry, and energy; settle accounting; then commit once. On rejection, preserve the original source or all outputs in valid holders. Internal transfers cancel at world scale; diagnostic/cache counters are not separate energy stores.

**Exit evidence:** every gap row needs focused failure-injection coverage plus the independent material/energy oracle. World tone also needs an independent exact-small-world exposure/first-hit comparison. The recorded full suite remains baseline-red; no source/test changes were made while consolidating this matrix.


### 17.56 Energy API call-site closure — live frame settlement is separate from staged budget API — 2026-10-22

**Status: Inspected caller set; no unlisted live energy API path found in the current search.** Exact-head call-site search plus source review; no source/test changes.

- Live `settle_transaction` consumers are COMBINE bond formation, chemical/stress/resonance BREAK, maintenance, and movement. The legacy decomposition consumer is not called by the production tick. Their ordering is covered in Sections 17.44–17.46.
- Live external-input settlement is used for the initial seed reserve. The legacy per-unit WorldToneFrame `settle_receiver` path is exercised in its own tests but is not the `Simulation::step` capture path. Live frame capture calls `settle_world_tone_paths` once after calculating the frame totals and stages reservoir commits until settlement succeeds.
- `EnergySourceBudget` and `settle_external_input_from_budget` are defined and unit-tested as staged primitives, but the live world-tone frame does not call them. They therefore provide no evidence by themselves that live projected exposure is uniquely partitioned; that remains a geometry/snapshot oracle gate. The live frame's per-path balance is its current accounting authority.
- `EnergyLedger::transfer` has two inspected production callers: reproductive reserve transfer and the unintegrated legacy recycler. The reproduction caller ignores the boolean result; the recycler checks it but is not a valid lifecycle integration path. No other live internal-transfer caller was found in the current search.
- This map does not make the uncalled legacy APIs safe to activate. A future caller must either join the approved live transaction model or remain clearly quarantined with tests that do not imply production integration.

**Next audit:** reconcile the final call-site map with the dependency/phase tracker and decide whether the remaining unfilled templates are source inspection, focused-test design, or owner decision. No implementation until audit completion.


### 17.57 Resource identity and README truth audit — active catalog uses Oxygen; README misstates live capture status — 2026-10-22

**Status: Current catalog identity confirmed; documentation mismatch confirmed.** Exact-head review of src/materials/resources.rs and README.md at PR #205 head 12d8079230458da68a8dc3755a1d6a9b41347862, plus default-branch search results. No source/test/README change.

- The active catalog defines Oxygen for resource ID 6; the catalog regression asserts Oxygen and current property data. Material validation checks catalog membership and whole finite constituent amounts. Unknown-resource mass/potential/weighted-property queries fail with NaN rather than inventing zero-valued properties. The current README and active catalog do not use Methane as a live resource identity; remaining search hits are historical audit/archive documents, which should remain historical rather than be blindly rewritten.
- README.md says “Live capture stays disabled until the allocation and conservation path is complete and verified.” The live tick in Simulation::step calls the world-tone snapshot/settlement path, and Section 17.39 confirms it stages reservoir commits and settles frame energy. Therefore the README's disabled-capture claim is factually stale. The accurate status is: capture code is live, but whole-world allocation/conservation, receiver eligibility, exact-small-world approximation accuracy, and performance remain unverified.
- README also carries the older PR #203 verification checkpoint (563/28/1 and workflow 39705150387). Current exact-head evidence for PR #205 is workflow 39833438679 and its 585/27/1 result with baseline-identity comparison. Documentation should not replace the older checkpoint without preserving it as historical context.
- **Scope decision for this plan:** record the README mismatch now, but do not edit README in this audit-plan commit. Correct the current-status sentence and verification links only after the audit register is closed and the owner approves the docs change. This keeps PR #204's present plan/handoff focus intact and avoids suggesting that “live” means “validated.”

**Next audit:** check the remaining A1 material/property fallbacks and verify whether each zero/default result is an intentional domain default or a fail-closed path. Then update the audit summary with only active-code findings, retaining historical references as historical.


### 17.58 Resource/property fallback classification — 2026-10-22

**Status: A1 follow-up completed for inspected paths.** Reviewed resources.rs, chemistry.rs, combine.rs, combine_runtime.rs, and chemical_reaction.rs at PR #205 head. No source/test changes.

- Unknown names fail catalog validation; mass/potential/weighted-property queries return NaN for invalid material. The default catalog uses Oxygen for ID 6. Current active code and README do not use Methane as an identity; remaining search hits are historical archives.
- Valid composites intentionally have no averaged chemical_position because averaging would erase constituent identity. Their weighted physical properties come from catalog-bound constituents; live chemistry operates on constituent units. Materials lacking a chemical position use the documented cohesion-based bond-potential fallback.
- A staged helper maps failed interaction_potential to zero and bond_strength maps nonfinite cohesion to zero. The live COMBINE path requires valid properties and, when both positions exist, successful chemical_interaction before commit, so these helper fallbacks are not currently shown to admit an invalid live bond. Keep adversarial tests to protect against future callers bypassing that validation.
- Empty-catalog baseline helpers return neutral values, but the production environment uses the non-empty seven-resource catalog. Test empty-catalog behavior explicitly; do not treat these values as properties for unknown resources.

A1 active identity and unknown-name behavior are substantially closed for inspected production paths. Malformed catalog-property tests and all-entry-point validation remain evidence gates. Cache invalidation remains open per Sections 17.49–17.50.


### 17.59 Diagnostic observability audit — seed header and energy label are not authoritative — 2026-10-22

**Status: Reporting mismatch confirmed; no runtime change made.** Exact-head review of src/infrastructure/diagnostics.rs and README.md at PR #205 head 12d8079230458da68a8dc3755a1d6a9b41347862.

- Diagnostics are opt-in and separate from normal headless output. The JSON header hard-codes `seed: 42` instead of reading the simulation's actual seed/configuration. A diagnostic run using another seed would therefore be mislabeled.
- The per-organism snapshot serializes only `organism.usable_energy` under the generic key `energy`. It does not represent developing usable energy, retained resonance, or stored graph-bond energy. This is useful for a local transformation trace, but it must not be interpreted as a whole-organism or world energy inventory. Rename/label it as usable energy or include separately named holder fields in a future diagnostics task.
- The recorder's opt-in file output is not the conservation oracle. Keep EnergyInventorySnapshot and integer material inventory as the independent accounting tools; do not derive world conservation from diagnostic event deltas alone.
- README.md's current statement that live sunlight capture is disabled is stale against the inspected tick path. Section 17.57 records the mismatch and the decision not to edit README until the audit register is closed and the owner approves the documentation update. The README's PR #203 test checkpoint should remain historical rather than be overwritten without preserving provenance.

**A17/A21 status:** the exact seed/quantity labels and README capture-status discrepancy are now evidenced. Fixing diagnostics and README remains a later scoped change; do not mix it into PR #204's audit-plan-only changes unless the owner authorizes it.


### 17.60 Resonance invalid-property handling — epsilon clamps can turn malformed catalog data into a valid-looking response — 2026-10-22

**Status: Adversarial numerical gap identified; default catalog is valid.** Exact-head review of natural_frequency_hz, damping_ratio, resonance_response, and world-tone body-property use. No source/test changes.

- `natural_frequency_hz` applies `max(EPSILON)` to mass, cohesion, and their baselines without first rejecting nonfinite inputs. A NaN mass/cohesion can therefore be replaced by epsilon and yield a finite, plausible frequency instead of failing closed. `damping_ratio` similarly clamps reactivity with `max(0.0)` and has no explicit finite-input guard; malformed reactivity can be interpreted as zero damping. `resonance_response` clamps damping to epsilon but does not explicitly reject NaN damping.
- The live world-tone snapshot reads catalog ResourceProperties for physical bodies and feeds them into resonance calculations. The normal seven-resource catalog has valid finite values, so this is not evidence of a default-run defect. It is an invalid-state contract gap: malformed/partially loaded catalog properties should not be silently converted into a neutral/baseline material response, particularly near the 440 Hz source where a tiny damping denominator can produce a very large amplitude response before energy bounding.
- Add adversarial tests with NaN/infinite/negative mass, cohesion, reactivity, and baselines. The preferred contract is fail-closed (`None`/invalid response) before resonance calculation, not silently clamp malformed input to a physical epsilon. Keep epsilon floors for valid near-zero physical values only after finiteness and domain checks.
- Existing downstream spectrum energy bounding prevents an unbounded amplitude from directly exceeding the incident energy budget, but it does not make malformed-property behavior scientifically valid. Keep this separate from conservation tests: it is a material-response validity issue, not by itself proof of energy creation.

**Next audit:** enumerate all entry points that call natural_frequency_hz/damping_ratio and confirm they either validate catalog properties before calling or propagate a fail-closed result. No implementation until the audit templates are complete.


### 17.61 Resonance caller cross-check — caller-side finite checks do not catch inputs masked by epsilon clamps — 2026-10-22

**Status: A7/A18 follow-up confirmed.** Callers inspected: live world-tone snapshot, legacy WorldToneFrame receiver, harmonic spectrum update, chemical BREAK accumulation, and resonance-overload BREAK candidate selection.

- `chemical_reaction.rs` and `transformation.rs` check that the returned natural frequencies are finite and positive, but `natural_frequency_hz` can turn NaN mass/cohesion into epsilon-valued inputs before returning. The caller checks therefore do not reject those malformed properties.
- `world_tone_snapshot.rs` checks damping output and bounds response energy, but a finite damping value produced from malformed reactivity is not enough to prove that the material properties were valid. The legacy per-unit capture path also relies on the same helper functions. Harmonic-spectrum generation uses these helpers and should share the same fail-closed contract.
- The narrow common boundary is to validate ResourceProperties and ResourceBaselines inside natural_frequency_hz/damping_ratio before applying epsilon floors; return a non-finite/Option failure consistently and have callers skip that body/interface without changing the source budget. Avoid independently adding divergent checks in each caller.
- Test coverage should exercise the helper directly and at least one live world-tone and one chemical-reaction call site with malformed catalog properties. Verify invalid material does not become a baseline-frequency resonator and does not change reservoir, ledger, or activation state.

This confirms an adversarial invalid-state gap across several callers; it is not evidence of a default-catalog failure or of energy exceeding the incident budget. No implementation change.


### 17.62 Accumulation helper range/overflow audit — finite inputs can still produce a nonfinite result — 2026-10-22

**Status: Defensive math-contract gap confirmed.** Exact-head review of accumulate_reaction and its live chemical-reaction caller. No source/test changes.

- `accumulate_reaction` validates that each input is finite and nonnegative, but it does not validate the output after evaluating `previous + interaction * contact * engagement - dissipation * previous`. It then returns `Some(next.max(0.0))`; finite but out-of-domain values can overflow an intermediate and return `Some(infinity)` rather than fail closed.
- The live caller's static potential, contact, engagement, and dissipation are bounded by the current equations/constants, and it rejects nonfinite `next` at the subsequent normalized-chemistry-to-energy conversion before storing a new accumulation. Therefore this is primarily a helper contract and adversarial-state gap, not evidence that ordinary valid input creates infinite accumulated energy. A stale prior value can remain if conversion rejects it.
- Add an output finiteness check before returning, and define accepted ranges for normalized potential and dissipation (dissipation is a fraction, so it should not silently accept arbitrary finite values above its intended domain). Prefer a stable expression only if it preserves the approved equation. Tests should cover maximum finite inputs, oversized interaction/dissipation, and the actual live constants.
- Keep reaction accumulation categorized as activation state, not a physical energy holder. Its overflow behavior must be deterministic and bounded, but it does not itself enter the world energy conservation equation.


### 17.63 Resonance helper entry-point and adversarial-test coverage matrix — 2026-10-22

**Status: Caller set reconciled; malformed-property test gap confirmed.** Read-only audit of `src/perception/harmonics.rs`, `src/chemistry/chemical_reaction.rs`, `src/chemistry/transformation.rs`, `src/simulation/world_tone.rs`, and `src/simulation/world_tone_snapshot.rs` at exact PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`. No source/test changes.

- The resonance helper boundary is shared by live and legacy paths. `natural_frequency_hz` and `damping_ratio` are defined in `src/perception/harmonics.rs`; `material_response_to_drive` also calls both there. Production callers found in this source pass are ordinary chemical BREAK accumulation (`chemical_reaction.rs`), resonance-overload BREAK candidate selection (`transformation.rs`), the legacy per-unit `WorldToneFrame` receiver (`world_tone.rs`), and live frame propagation (`world_tone_snapshot.rs`). The caller-side finite checks do not close the gap while helper clamps can mask malformed values.
- Existing harmonic tests cover valid cohesion/mass frequency scaling, valid relative damping, bounded absorption, and response energy. They do **not** exercise NaN/infinite/negative mass, cohesion, reactivity, or invalid baselines at the helper boundary. No explicit malformed-property regression was found in the inspected test module.
- The narrow implementation/test unit after audit closure should make helper validation consistent before epsilon floors, while preserving legitimate near-zero physical values only after domain checks. Decide whether to change return types or use an unambiguous invalid numeric sentinel based on caller ergonomics; whichever contract is chosen must make all listed callers fail closed without reservoir, ledger, or activation mutation.
- Required regression layers: (1) direct helper tests for nonfinite and out-of-domain values plus valid near-zero values; (2) at least one live world-tone snapshot test proving invalid properties do not retain resonance or alter energy settlement; (3) one chemistry-reaction test proving invalid properties do not create activation progress or trigger a BREAK; (4) confirm legacy per-unit capture and harmonic-spectrum generation inherit the same behavior.
- This is an invalid-input contract gap, not evidence that the default catalog produces invalid frequencies or that the current energy budget is exceeded. Keep it distinct from whole-world conservation and geometry accuracy.

**Next audit:** close the remaining material/energy transition failure matrix against the phase tracker, then classify each remaining item as source-inspection-complete, owner-decision-blocked, or requiring a focused failure-injection test. No implementation until every audit template is complete.


### 17.64 Transition failure matrix — source-ordering cross-check — 2026-10-22

**Status: Source ordering reconfirmed for five high-risk transitions; deterministic rejection coverage remains incomplete.** Read-only review at PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862` of `src/perception/movement.rs`, `src/chemistry/expulsion.rs`, `src/organism/reproduction.rs`, and `src/simulation/simulation.rs`. No source/test changes.

| Transition | Exact observed ordering | Failure classification / required evidence |
|---|---|---|
| Movement | `advance_movement` calls `try_move_cell_with_reason` (which changes positions/geometry) before `settle_transaction`; on settlement rejection it returns without restoring geometry or decrementing movement progress. | **Confirmed atomicity gap.** Inject ledger rejection after a successful move and compare positions, spatial index, construction anchor, energy, progress, and independent inventory. Prefer staged movement or a complete rollback. |
| Expulsion | `expel_physical_material` removes the stored instance, computes destination and mutates placements, then returns the field deposit result directly. On destination rejection, the owned physical instance is no longer in storage and is not returned to the caller. Earlier validation-failure paths attempt to restore storage but ignore restoration failure. | **Confirmed ownership gap.** Test destination rejection and restoration rejection; source must remain in storage or a valid transaction holder with unchanged world inventory. |
| Contained acquisition | `transfer_contained_environmental_material` removes all contained field objects first, then attempts storage. On storage failure it attempts a field deposit at the first placement and ignores the result. | **Confirmed ownership gap.** Force both storage and fallback-deposit rejection; assert no source disappears and the independent integer inventory is unchanged. |
| Construction reserve | `advance_construction` computes a bounded transfer and ignores the boolean from atomic `ledger.transfer`, then continues maintenance/construction. | **Failure-policy gap.** Test rejected transfer with unchanged source/destination and decide in code whether the tick waits/fails or reserve funding is explicitly best-effort; do not silently assume success. |
| Stored BREAK | Existing regression covers a failed stored-BREAK restoration path and inventory, but the live transformation still handles split pieces and fallback destinations incrementally. | **Partial coverage.** Add deterministic failure after the first split-piece commit, storage+deposit rejection, and malformed completion/escrow restoration. Validate the complete world inventory, not only storage count. |

- Movement tests for blocked/non-finite movement and atomic push chains establish geometry rejection/propagation behavior, but do not exercise a ledger rejection *after* a successful geometry move; those are distinct transaction boundaries.
- This pass does not close the remaining matrix: COMBINE prepared-ledger invariant, stale reaction keys, catalog-fingerprint invalidation/restart, maintenance debt/rejection, construction growth/finish stale-source rollback, world-tone exact-small-world oracle, and post-death stored/developing-holder reachability still require dedicated evidence.
- Receiver eligibility for parent-owned stored/committed/developing surfaces remains **owner-decision-blocked** as recorded in Section 17.38; do not infer that physical ownership automatically grants resonance retention.
- No source/test changes were made. The audit template is not yet complete, so no implementation is authorized.

**Next:** reconcile each remaining row against Section 17.7 and the Section 17.6 release gates, mark it source-inspection-complete, test-required, or owner-decision-blocked, then review whether any uninspected production mutation path remains.


### 17.65 Remaining local-contract gaps — stale activation keys, prepared COMBINE ledger, and cache invalidation — 2026-10-22

**Status: Three known gaps reconfirmed at exact PR #205 head.** Read-only review of `src/chemistry/chemical_reaction.rs`, `src/chemistry/transformation.rs`, `src/chemistry/chemistry_library.rs`, and `src/chemistry/energy_ledger.rs`. No source/test changes.

1. **Bondless reaction accumulation cleanup:** `chemical_reaction::accumulate` returns immediately when `organism.structure.bonds.is_empty()`, before removing stale accumulation entries for that organism. The entries are activation state, not energy, but can persist after the graph loses all bonds. Add a regression that seeds a valid key, removes all bonds, runs accumulation, and verifies all keys for that organism are removed; preserve other organisms' keys.
2. **Prepared COMBINE ledger delta contract:** preparation computes deltas for `total_world_tone_energy_escaped` and `total_resonant_energy_held`, but the commit path does not apply them and `rollback_prepared_combine` does not reverse them. The currently inspected COMBINE candidate should not change either field. This is a latent contract mismatch, not evidence of current energy leakage. Preferred smallest fix is an explicit invariant rejecting/asserting nonzero values for both fields, plus a test; alternatively centralize field-wise apply/rollback so every ledger field has symmetric semantics.
3. **Catalog chemistry fingerprint:** `CHEMISTRY_EQUATION_FINGERPRINT` includes a manually maintained `catalog=resource-chemistry-v1` marker. `ChemistryKey` does not carry numeric chemical positions, while cached static potential depends on those positions and K/Dmax. Existing `rejects_equation_fingerprint_mismatch` proves a changed manifest string is rejected, but does not prove catalog-property changes require a fingerprint bump. Add a deterministic test/contract tying a change to a relevant catalog chemistry field to fingerprint invalidation, or derive the fingerprint from canonical relevant fields.
4. **Transfer primitive:** `EnergyLedger::transfer` validates both holders and computes both next values before assignment, so its two-holder mutation is atomic. The remaining issue is the construction caller discarding its boolean result (Section 17.53/17.64), not a defect in the primitive itself.

**Classification:** items 1–3 require focused regression/invariant coverage; item 4 requires a caller failure-policy decision and test. These do not close the higher-risk material-loss or movement rollback gates. No source/test changes were made.

**Next:** reconcile these items and the Section 17.64 transition rows into a single exit checklist, then identify which uninspected source paths remain before audit templates can be declared complete. No implementation until that checklist is complete.


### 17.66 Touched-file closure cross-check — contract snapshot, staged graph API, and CI bounds — 2026-10-22

**Status: Three cross-cutting boundaries classified; documentation correction deferred.** Read-only review at PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862` of `docs/CHEMISTRY_UNIFICATION_CONTRACT.md`, `README.md`, `.github/workflows/rust.yml`, and `src/geometry/structure.rs`. No source/test/document changes.

- **Contract document:** `docs/CHEMISTRY_UNIFICATION_CONTRACT.md` correctly labels its dependency map as “main at the frozen commit,” so the schema-3/cache and old aggregate-transfer entries are historical baseline findings, not claims about the current implementation. However, its plan-authority link is pinned to an older plan commit (`09254d9e11fd021f42a0c8abbd43594cd7764c69`). Once the audit register is closed, update the link to the canonical current plan branch/path or a current immutable commit, while preserving the frozen-baseline distinction.
- **README status:** `README.md` still states live sunlight capture is disabled, while current `Simulation::step` executes the world-tone snapshot/settlement path. Section 17.57 already records this as a factual mismatch. Correct the statement only after audit closure and owner approval; say “live, not yet validated,” not “disabled” and not “conserved.”
- **Workflow bounds:** `.github/workflows/rust.yml` applies five-minute timeouts to formatting, source-size, architecture, focused chemistry, and full-suite steps. This matches the owner's command limit. The full suite has previously completed under five minutes on the recorded exact head, but its known baseline failures remain; time bounds do not constitute a pass.
- **Staged graph API:** `src/geometry/structure.rs` is marked as a staged API retained for subsystem integration. Its direct bond insertion/removal methods are low-level graph mutations, not proof that production chemistry settles bond energy. Continue to audit live callers rather than treating method-level tests or public visibility as runtime reachability.
- The remaining changed-file coverage is represented in Sections 17.32–17.65: material restoration/transfer, chemistry cache, energy ledger, live world tone, simulation lifecycle, and test-only inventory oracles. This cross-check did not identify a new physical source or sink, but it does not replace the open failure-injection and integrated-conservation gates.

**Next:** build the final row-by-row exit checklist, with every Section 17.55 transition classified as verified locally, test-required, or owner-decision-blocked; then compare that list against A0–A21 and Section 17.7 for omissions. No implementation until the audit templates are complete.


### 17.67 Exit checklist reconciliation — transition status and remaining proof — 2026-10-22

**Purpose:** turn the source-to-sink matrix (17.55), energy API map (17.56), source audits (17.57–17.66), A0–A21, and tracker (17.7) into one explicit closeout list. This is a reconciliation of recorded evidence, not a claim that the audit or chemistry implementation is complete.

| Transition / gate | Classification now | Evidence still required before closure |
|---|---|---|
| Initial-condition energy | **Local path inspected; test-required** | Tick-zero/no-op independent inventory; prove the one-time source guard cannot duplicate the initial grant. |
| COMBINE graph bond | **Local ordering verified; test-required** | Failure/retry inventory tests and an explicit zero invariant or symmetric handling for prepared escaped-tone/resonant-held ledger fields. |
| Chemical / resonance BREAK | **Local ordering verified; test-required** | Stale/retry/rejection inventory tests; invalid-property helper and live-call-site regressions; exactly-once bond-energy settlement. |
| Stored BREAK | **Gap confirmed; test-required** | Partial split failure, storage+deposit rejection, malformed completion, escrow restoration, and whole-world material inventory. |
| Environmental COMBINE source | **Partial; test-required** | Missing source/cell and malformed prepared state must preserve source, storage, graph, and ledger. |
| Contained acquisition | **Gap confirmed; test-required** | Inject storage rejection and fallback-deposit rejection; source must remain in a valid holder. |
| Expulsion | **Gap confirmed; test-required** | Inject final destination rejection and restoration failure; original stored instance and inventory must remain recoverable. |
| Movement | **Gap confirmed; test-required + D7 equation reconciliation** | Reject ledger settlement after geometry succeeds; rollback position/index/anchor/progress. Clarify heat-only cost versus explicit work without changing approved physics silently. |
| Maintenance | **Local settlement ordering inspected; test-required** | Insufficient energy/debt, rejected transaction, and unchanged stress/ledger assertions. |
| Construction reserve | **Atomic primitive verified; caller policy open/test-required** | Test failed transfer; choose explicit waiting/failure versus best-effort funding and assert both holders remain unchanged on rejection. |
| Construction growth / finish | **Partial; test-required** | Stale source index, failed anchor/holder transfer, malformed finish, and whole-step before/after material + energy snapshots. |
| World-tone frame | **Local frame balance/staged commit verified; owner decision + test-required** | Owner decision on non-primary parent-owned receiver eligibility; exact-small-world first-hit/allocation comparison; source-budget residual and representative runtime/memory evidence. |
| Reaction accumulation | **Gap confirmed; test-required** | Bondless stale-key cleanup and helper output range/overflow tests; prove activation state remains outside physical-energy inventory. |
| Chemistry cache | **Schema/manifest rollback locally verified; test-required** | Catalog-position-to-fingerprint invalidation, restart/reopen and failure-injection coverage beyond the existing append/manifest rollback tests. |
| Post-death lifecycle / legacy recycler | **Owner architecture approved; production integration gap + test-required** | Multi-tick dead-parent inventory/holder tests for stored, committed, and developing material; ordinary chemistry must continue without enabling the quarantined legacy recycler. |
| Independent world conservation / long-run | **Open release gate** | Independently derived energy and integer-material totals across transitions; distinguish numerical tolerance from physical dissipation; bounded long-run drift/reservoir checks. |
| README / implementation docs | **Known reporting mismatch; deferred** | After audit closure and owner approval, correct live-capture status and update stale plan-authority link while preserving frozen-baseline history. |

**Reconciliation result:** the recorded audit has no remaining ambiguity about the major known local mutation orderings, but it is **not ready to authorize implementation**. The next work is evidence design/closure for the test-required rows and the single owner-blocked receiver-eligibility rule; then perform one final source search for any production material/energy mutation path missing from this table. Do not mark any row closed merely because a helper or isolated unit test exists.

**Implementation entry condition:** every test-required row has a deterministic failure test and independent inventory assertion; the receiver rule is decided; the final mutation-path search is reconciled; the plan/handoff/PR description are synchronized; and PR #205 exact head plus CI are re-fetched before the first code change.


### 17.68 Final production mutation-path search — no new source/sink category, existing gaps remain — 2026-10-22

**Status: Broad exact-head mutation-pattern scan reconciled; no new source/sink category found.** Read-only scan of the 19 PR #205 changed paths plus movement, reproduction, world-tone, harmonic, and expulsion modules at exact head `12d8079230458da68a8dc3755a1d6a9b41347862`. No source/test changes.

- The scan re-found the already-mapped live categories: initialization source setup; COMBINE material/bond commit; chemical/resonance BREAK and retained-resonance withdrawal; staged world-tone frame settlement; storage/field acquisition and expulsion; movement; construction transfer/growth; cache persistence; independent material/energy inventory.
- Direct graph mutation methods in `src/geometry/structure.rs` include low-level insertion/removal, but production energy authority remains in higher-level COMBINE/BREAK paths. Their staged/dead-code annotation and tests do not make them a live energy source.
- `src/chemistry/decomposition.rs` remains a legacy, non-live path. Its local transaction tests do not establish that the production tick invokes it, and it must not be enabled as a parallel death-release mechanism.
- No additional distinct physical source/sink category was discovered by this pass. This does **not** close the identified rows: broad text-pattern search is not a substitute for adversarial failure injection or the independent world-level conservation oracle.
- The production mutation map is now sufficiently reconciled to stop expanding the inventory of ordinary paths. The remaining work is proof/contract closure, not another broad search: test-required rows from Section 17.67, the owner-blocked receiver rule, and final audit-template sign-off remain open.

**Next:** finish audit-template sign-off by attaching the exact source/test reference and a deterministic proof requirement to every remaining row. If any row lacks a testable acceptance condition, keep it open. Only after that may the smallest implementation unit be selected.


### 17.69 Deterministic acceptance-test design — exact failure contracts before implementation — 2026-10-22

**Status: Test-design matrix filled for the known open rows; tests are specified, not implemented.** Existing tests were enumerated at PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`. This section completes the test-design portion of the audit; it does not authorize code changes.

| Gap | Existing coverage that must be preserved | Required focused regression / acceptance condition |
|---|---|---|
| Initial source | `initial_usable_energy_enters_through_an_explicit_source_transaction`, `initial_world_energy_inventory_uses_tick_zero_baseline` | Extend/confirm duplicate source claim rejects without changing holder or ledger; independent snapshot counts the initial grant once. |
| COMBINE prepared ledger | `failed_prepared_combine_rolls_back_energy_and_every_ledger_delta` | Add nonzero sentinels for escaped-tone and resonant-held fields to the rollback fixture; add successful candidate invariant that these fields remain exactly unchanged/zero. If a future COMBINE path can alter them, centralize symmetric commit/rollback instead of silently omitting fields. |
| Chemical / resonance BREAK | `failed_stale_chemical_break_does_not_consume_retained_resonance`, `bond_break_releases_only_stored_bond_potential`, resonance overload tests | Add invalid material-property cases; after rejection assert graph, usable/resonant holders, ledger, activation map, and independent inventory are unchanged. |
| Stored BREAK | `failed_stored_break_restores_material_to_organism_storage`, `stored_material_break_does_not_credit_unallocated_energy` | Add `stored_break_partial_split_failure_preserves_inventory`, `stored_break_rejected_storage_and_deposit_preserves_escrow`, and `malformed_stored_break_completion_restores_escrow`. Inject failure after one split output; compare the independent integer inventory across field, storage, escrow, and structure. |
| Environmental COMBINE | `restore_failed_transformation_material` and existing transformation/restore checks | Add missing-cell/source and malformed-prepared-state tests; source, geometry, storage, energy, and ledger must either all commit or all remain unchanged. |
| Contained acquisition | `contained_physical_material_becomes_storage_without_an_acquire_action` | Add `contained_acquisition_rejected_storage_and_deposit_preserves_source`; force storage rejection and fallback deposit rejection, then assert the source remains in a valid holder and integer inventory is identical. |
| Expulsion | No dedicated expulsion test was found in the reviewed module | Add `expulsion_deposit_rejection_restores_original_storage_instance`; reject the final field deposit and assert the exact physical instance/placements and inventory remain recoverable. Add a second rejection test for storage restoration itself; restoration failure must be surfaced, not ignored. |
| Movement | `blocked_zero_move_and_nonfinite_move_are_rejected`, atomic push-chain tests | Add `movement_settlement_rejection_rolls_back_geometry_and_progress`; force ledger rejection after a valid move candidate and assert organism, pushed objects, spatial index, reproductive anchor, progress, energy, and ledger match pre-state. |
| Maintenance | `unpaid_maintenance_accumulates_debt_without_direct_death`, `zero_maintenance_energy_accumulates_debt_and_increasing_stress_pressure` | Add a rejected/invalid settlement case; stress, debt, holder, and ledger must not partially update beyond the documented unpaid-maintenance behavior. |
| Construction reserve | `developing_offspring_receives_persistent_energy_from_parent` | Add `construction_reserve_transfer_rejection_preserves_both_holders`; verify the parent/developing holders remain unchanged and status explicitly waits/fails rather than assuming the transfer succeeded. |
| Construction growth / finish | `construction_waits_when_all_physical_inventory_is_exhausted`, `waiting_construction_resumes_from_same_graph_when_material_arrives`, anchor-transfer inventory test | Add stale-source and malformed-finish failure tests; compare integer material inventory and all energy holders before/after, including committed/developing construction and parent storage. |
| World tone | `live_world_tone_capture_settles_and_reconciles_world_energy`, occluder and nonliving heat tests | Add an independent deterministic exact-small-world first-hit oracle; compare per-lane incident allocation, unique first-hit receiver, retained resonance, heat, and escaped signal. Add owner-approved non-primary receiver expectations before changing eligibility. |
| Reaction accumulation | `successful_chemical_break_consumes_only_compatible_retained_resonance` and stale-BREAK tests | Add `bondless_accumulation_clears_only_its_organism_keys` and helper range/overflow tests; verify no physical energy changes and other organisms' keys survive. |
| Chemistry cache | `manifest_failure_rolls_back_appended_record_and_reopens_cleanly`, `insert_partial_append_failure_rolls_back_original_cache_bytes_and_reopens_cleanly`, fingerprint mismatch and corrupt-store tests | Add `catalog_chemistry_change_requires_fingerprint_invalidation` plus restart/reopen after interrupted append/manifest state. A changed chemical-position/K/Dmax contract must not reuse old static-potential rows. |
| Post-death lifecycle | `breached_genome_core_does_not_remove_or_recycle_physical_organism`, `energy_inventory_snapshot_recomputes_current_holders_independently_of_ledger_totals` | Add `dead_parent_retains_and_accounts_for_storage_and_developing_holders_over_ticks`; dead parent stays physical, biological work stops, ordinary chemistry continues, and each holder is counted once without invoking the legacy recycler. |
| Numerical integrity | `resonant_response_and_harmonics_cannot_exceed_incident_energy`, `unrepresentable_spectral_energy_fails_closed` | Add NaN/infinity/negative material and baseline cases, accumulation overflow/domain cases, reservoir pruning/tolerance checks, and cumulative counter overflow tests. Numerical tolerance must not be recorded as physical heat. |
| Diagnostics/docs | Current opt-in diagnostics tests and Section 17.57/17.59 findings | Later, after audit closure and owner approval: verify actual seed and separately named usable/resonant/structural fields; correct README live-capture wording and refresh the contract's plan-authority link without overwriting historical baseline evidence. |

**Acceptance rule:** each failure test must use a deterministic rejection seam or malformed state that reaches the actual live transaction boundary; assert both the relevant object state and independent inventory before/after. A helper-only test does not prove production reachability. Do not weaken a test by asserting only a ledger counter, stored count, or aggregate failure total.

**Audit gate:** the test-design matrix is now explicit. Still open before implementation: owner decision on non-primary receiver eligibility; exact source references and injection strategy for each test in the implementation branch; and a final owner-facing confirmation that all audit templates are complete. No production source/test changes are authorized by this documentation update.


### 17.70 Owner decision — resonance follows attached physical structure; stored material excluded — 2026-10-22

**Status: Owner decision resolved; source-impact audit required before implementation.** The owner clarified the previously blocked non-primary receiver rule:

- “Physical structure” and what earlier notes called “committed reproductive structure” are the same physical category. Do not give attached reproductive structure a special physical-material designation merely because it is associated with reproduction.
- The separate “developing structure” category is not accepted as a desired physical model. Audit whether it can be represented as ordinary organism-owned physical structure throughout growth, with construction state tracking only the process/state that is genuinely necessary. Do not delete the field or merge state blindly before tracing construction, transfer, geometry, and division.
- Any physical structure structurally attached to an organism is eligible to contribute its material response to that owning organism’s resonance reservoir, while that organism is biologically eligible to receive resonance.
- Stored physical material is excluded from the owner’s resonance capture. It remains physical geometry and may occlude/receive signal as a nonliving physical receiver, but its response must not be credited to the owner’s resonance reservoir merely because the owner stores it.
- The same physical object must not be counted twice: attached units belong to the owner’s physical structure and resonance accounting; stored units remain stored objects. A construction-progress label alone must not create a second physical copy or energy holder.
- Death still ends biological processing without removing physical existence. Dead attached structure remains a physical receiver/occluder and cannot credit a dead organism’s resonance reservoir; ordinary chemistry remains responsible for later physical changes.

**Exact-head source impact observed at PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`:**

- `src/simulation/world_tone_snapshot.rs` builds the parent organism body from `organism.structure`, then separately adds `reproductive_construction.developing_structure` as another physical body and adds `committed_material` as separate physical bodies.
- The same snapshot also includes `organism.stored_material` and active-transformation escrow as physical occluders/receivers. The receiver-to-organism map only maps the primary `organism.structure` body to an organism. Consequently, the separate developing structure cannot retain resonance for its parent today; its response is treated as a nonliving physical receiver and becomes heat.
- `settle_living_organism_inputs` validates direct projected widths only for primary organism structure units, and commits staged resonance reservoirs to organisms after ledger settlement.
- `src/organism/reproduction.rs` has a `developing_organism` adapter that copies `developing_structure`, `committed_material`, developing energy/stress, and origin into a temporary `Organism`-shaped value. Construction/growth/finish logic therefore needs tracing before deciding whether the field can be removed or should be retained only as a transient construction workspace and never as a second physical authority.
- Existing snapshot test `developing_offspring_and_its_stored_material_remain_world_occluders` asserts current separate-body geometry behavior; its intended assertions must be revised to the owner’s unified-structure rule rather than preserved as an outdated design constraint.

**Required follow-up before implementation:**

1. Trace every read/write/transfer of `reproductive_construction.developing_structure`, `committed_material`, and associated energy/stress/debt through construction reserve, growth, waiting, ready/finish, detachment, death, and division.
2. Establish the canonical representation: attached physical units must be represented once in the owning organism structure (or prove why an interim workspace is needed without treating it as a second physical authority). Stored material remains separate and excluded from owner resonance.
3. Update the sunlight oracle and acceptance matrix so attached structure can capture into the living owner’s reservoir, stored material cannot, dead attached structure cannot capture biologically, and first-hit geometry/energy allocation remains conserved.
4. Update material/energy inventory rules and transition-failure cases to detect double counting during attach/finish/transfer and ensure the owner remains authoritative until the approved atomic division boundary.
5. Only after this source-impact audit, adjust the test design and mark the receiver-decision blocker closed. This decision resolves the policy question but does not establish implementation or conservation correctness.

No source or test files were changed in this checkpoint. PR #204 remains documentation-only; implementation stays in PR #205.


### 17.71 Reproductive structure lifecycle impact — construction currently has a second structure authority — 2026-10-22

**Status: Source-impact audit progressed; representation gap confirmed, not yet designed to closure.** Read-only trace at PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862` across `src/organism/reproduction.rs`, `src/simulation/simulation.rs`, and `src/simulation/world_tone_snapshot.rs`. No source/test changes.

| Lifecycle transition | Current behavior observed | Consequence under Section 17.70 | Required design/acceptance condition |
|---|---|---|---|
| Start reproduction | `begin_reproduction` removes a physical anchor from parent `stored_material`, builds a separate `ReproductiveConstruction.developing_structure`, and stores that structure inside the parent’s `reproductive_construction`. Parent `organism.structure` remains unchanged. | Parent is authoritative owner of the construction record, but attached physical structure is represented as a second graph instead of part of one organism structure. | Anchor must be counted exactly once as attached physical structure after transfer; no stored copy remains; parent ownership persists until division. |
| Construction state | `ReproductiveConstruction` separately holds `developing_structure`, `committed_material`, `developing_energy`, stress, maintenance debt, origin/orientation, and anchor index. | `developing_structure` is a second physical-structure category; `committed_material` is stored inventory, not structure, and stays resonance-ineligible. | Collapse physical graph identity into the organism’s authoritative structure; retain only nonphysical progress metadata that is actually required. Keep stored material separate. |
| Per-tick growth | `advance_construction` makes a temporary Organism-shaped `developing_organism` by cloning construction structure/storage and copying energy/stress/debt, applies maintenance/overload logic, then copies the resulting state back. It attaches selected material to that temporary graph and may transfer material from parent storage. | The temporary clone is a construction workspace; geometry and accounting must never treat the same units/holders as two physical authorities. | Define one authoritative state through the tick. Validate all resource/energy transfer before commit; a failed attach or transfer leaves parent-owned state and independent inventory unchanged. |
| Spatial attachment / detachment | `parent_child_in_contact`, `child_intersects_realized_parent_region`, and `parent_body_geometry` reason over separate parent and developing graphs. Status can be Waiting, Progress, Ready, Detached, or Dead. | Structural attachment must determine physical/resonance eligibility; a separate child graph cannot be silently treated as a separate organism/reservoir before division. | Re-express contact/boundary checks against attached units in the unified structure, or document the minimum temporary geometry view without a second owner/receiver identity. Detachment must not silently duplicate or lose units. |
| Sunlight snapshot | Primary `organism.structure` gets an owner index; `developing_structure` is appended as another physical body with no parent mapping. `committed_material`, parent stored material, and transformation escrow are also physical bodies. | Attached reproductive units currently cannot credit parent resonance; their response is settled as heat. Stored material must remain excluded from owner resonance even though it occludes/receives as physical material. | Snapshot geometry derives attached units from the one authoritative organism structure and maps every eligible unit to the living owner reservoir. Stored material remains a separate non-owner receiver. First-hit path energy still balances as capture + heat + escape. |
| Ready / detached / dead finish | `Simulation::tick` calls `finish_reproduction` for Ready, Detached, and Dead statuses. The helper takes the construction record and returns a new `Organism` whose structure is `developing_structure`, whose stored material is `committed_material`, and whose energy/stress/debt come from construction. | Current finish is a record-to-organism conversion, not the previously approved atomic division of one parent-owned aggregate. Status handling can end the construction path without proving exact partition. | Keep the parent sole owner until the approved division boundary; division must atomically partition every unit, stored item, bond, and energy exactly once. Do not interpret Detached/Dead as permission for silent ownership transfer. |
| Inventory and diagnostics | `total_usable_energy_held`, `total_resonant_energy_held`, and the simulation energy snapshot separately count construction energy/structure alongside organism holders. Material inventory includes developing structure and committed storage as separate holders. | The oracle may be internally consistent with today’s model while preserving a physical distinction the owner has rejected. | Revise the oracle model alongside representation: each physical unit/energy holder appears once; stored material remains separately counted; tests catch double counting across start/grow/finish/division and post-death ticks. |

**Decision clarification:** in this context, “committed reproductive structure” means the attached physical structure being built, not `committed_material` in storage. The owner’s rule is: attached structure is simply organism structure; stored material alone remains outside the organism’s resonance receiver. Do not rename `committed_material` to structure or include it in resonance.

**Required next:** trace all call sites and test contracts for `ReproductiveConstruction`, the independent material/energy inventory, movement/collision/geometry caches, and finish/division. Determine the smallest safe representation change that preserves parent ownership and exact physical inventory. Update the sunlight and transition-failure matrices after the representation contract is explicit. This does not yet authorize implementation: other audit templates and release gates remain open.


### 17.72 Cross-call-site representation audit — movement and inventories also encode the second graph — 2026-10-22

**Status: Additional dependencies confirmed at exact PR #205 head; no source/test changes.** Follow-up source inspection of `src/simulation/state.rs`, `src/perception/movement.rs`, `src/simulation/simulation.rs`, and `src/organism/reproduction.rs`.

- **Persisted schema:** `ReproductiveConstruction` serializes `committed_material`, `developing_structure`, child genome, developmental origin/orientation, developing stress, anchor index, developing energy, and maintenance debt. Removing the separate graph is a state-schema and compatibility decision, not a local sunlight-only patch. Inspect serde/default/load compatibility and existing saved-state expectations before altering the field.
- **Movement:** both successful move handling and the generic `translate_organism` path translate `organism.structure` and then separately translate `construction.developing_structure` and developmental origin. A unified graph must avoid double translation, preserve wrap/collision/spatial-index behavior, and ensure attached units participate in the same movement/occupancy/collision constraints as other organism units.
- **Geometry/eligibility:** `parent_body_geometry` derives geometry from `parent.structure` only. Construction separately uses parent-child contact and parent-region intersection checks. A unified representation must make attached structure visible to those geometry calculations while retaining the anchor/cavity invariant and avoiding accidental pre-split organism eligibility.
- **Energy inventory:** `Simulation::structural_energy_held` and test-only `energy_inventory_snapshot` separately sum `organism.structure` bonds plus `construction.developing_structure` bonds; usable-energy snapshot separately adds `construction.developing_energy`. These independent oracles need to be changed with the representation, not merely made to pass by counting both or deleting one term. The developing usable-energy holder may remain a distinct accounting holder only if its ownership/transfer semantics are explicit; “one physical structure” does not imply every energy sub-ledger must be collapsed.
- **End-of-tick ledger summaries:** cached total usable/resonant/structural values are recomputed from organism and construction fields. Once attached units share one structure, structural energy must be summed once. Resonant energy still belongs to the parent reservoir until the approved split; no second developing reservoir is created before division.
- **Tick and death:** construction advances after ordinary chemistry in the current tick ordering. It currently applies maintenance and resonance-overload BREAK to the temporary developing Organism. If the attached graph becomes parent structure, define which biological/maintenance actions apply to the aggregate versus the internal second genome core; preserve the rule that death stops biological processing while ordinary physical chemistry continues.
- **Material oracle:** transfer of the anchor and each construction material must preserve exactly one physical instance across parent storage, attached structure, construction staging, and environment. The current inventory categories that count developing graph and committed storage separately must be revised to match the canonical representation, while keeping actual stored inventory excluded from resonance.

**Audit conclusion:** the second graph is embedded in serialization, movement, geometry, construction, energy snapshots, death/tick behavior, and test fixtures. A safe implementation should first settle the canonical state/ownership representation and migration/compatibility behavior, then update the sunlight receiver mapping and independent oracles together. Do not patch only `world_tone_snapshot.rs`; that would leave conflicting physical representations and accounting.

**Next:** inspect saved-state construction and all tests that depend on boundary/detachment/anchor/cavity semantics; map each behavior to the single-structure model and specify regression tests. Other chemistry transaction audits remain open, so no implementation authorization yet.


### 17.73 Reproduction-test contract reconciliation — replace separate-child assumptions with aggregate ownership — 2026-10-22

**Status: Existing test contracts reviewed against the owner-approved physical model; tests remain unchanged pending audit closure.** Exact PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`; source/test files unchanged.

| Existing test/contract | Current assertion | Required interpretation under owner decision |
|---|---|---|
| `anchor_starts_a_separate_physical_child_graph` | Asserts parent structure remains byte-for-byte unchanged after reproduction starts and offspring anchor lives only in `developing_structure`. | Replace with an aggregate-ownership assertion: anchor is removed from storage and appears exactly once in the parent’s one authoritative physical structure; second genome/progress metadata remains internal, not a separate physical organism/graph owner. |
| `developing_offspring_receives_persistent_energy_from_parent` | Energy moves to `construction.developing_energy` and parent usable energy decreases. | Preserve explicit energy conservation but redefine holder ownership. A distinct developmental energy sub-holder may remain if it is a nonphysical sub-ledger owned by the parent aggregate and is transferred/partitioned atomically; do not create a second organism’s independent reservoir before division. Test world inventory across every tick. |
| `crossing_parent_boundary_does_not_detach_while_contact_remains`, `fully_outside_parent_boundary_is_detached`, `loss_of_parent_contact_is_detachment` | Treat the developing graph as a separate child that can leave the parent boundary or lose contact. | Re-evaluate whether these tests encode an obsolete external-bud model. Under internal/adjacent aggregate budding, physical units remain parent-owned until the approved division boundary. If detachment remains a valid failure state, define a physical bond/partition event and exact owner settlement; mere graph-to-graph contact is not an ownership rule. |
| `construction_waits_when_all_physical_inventory_is_exhausted`, `waiting_construction_resumes_from_same_graph_when_material_arrives` | Preserve a separate construction graph across waiting and later growth. | Preserve deterministic waiting/resume and exact-once material use, but assert the single authoritative aggregate graph is unchanged on waiting and extended only once when material arrives. Stored material remains stored until an atomic attachment transaction commits. |
| `developmental_scale_requires_eighty_percent_linear_target` | Scale is measured from `developing_structure` relative to a preferred target. | Preserve the approved total aggregate size/split criterion using the internal second-core region or construction progress metadata, not a separate physical graph identity. Confirm exact thresholds against the owner-approved reproduction requirement; do not let this chemistry audit silently change them. |
| Snapshot test `developing_offspring_and_its_stored_material_remain_world_occluders` | Builds separate geometry for developing graph and stored material. | Attached units should come from the parent’s one structure and map to the parent’s resonance reservoir; stored material remains a separate receiver/occluder and is not eligible for owner resonance. Assert first-hit allocation and global energy accounting. |

**Additional invariants to test once implementation is authorized:**

1. Begin construction: storage debit and attached-unit insertion are atomic; the selected anchor exists exactly once in the aggregate and contributes structural energy once.
2. Growth succeeds: candidate material leaves exactly one source holder and enters the aggregate once; failed/stale candidate leaves graph, storage, energy, and ledger unchanged.
3. Growth waits: no structure/material/energy duplication; later material arrival resumes from identical pre-wait state.
4. Movement: aggregate units translate once, collide/occlude as one organism, and update spatial indices/caches consistently.
5. Resonance: attached units may credit the living parent reservoir; stored material never credits it; dead attached units remain physical receivers but do not credit biological resonance.
6. Death: parent biological processing stops, but all attached/stored material and energy remain physically owned/accounted and ordinary chemistry can continue.
7. Division: one atomic partition moves each unit, bond, stored item, energy holder, and core identity exactly once to the two resulting organisms at the already-approved division threshold. Cross-partition bonds must settle explicitly.
8. Save/load: a persisted aggregate cannot resurrect a legacy second graph, drop attached units, or count a unit twice. Define whether legacy serialized construction records need migration or fail-closed handling before changing the serialized model.

**Audit implication:** do not preserve test names/expectations merely to minimize diff. The tests are evidence of current architecture, not higher authority than the owner-approved model. First specify the unified structure and save/load transition; then revise tests and implementation together. Chemistry audit remains open; no implementation authorized yet.


### 17.74 Cache invalidation and serialized-state implications — unified structure cannot bypass organism revisions — 2026-10-22

**Status: Dependency identified; migration policy remains open.** Source review at PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`.

- `Organism::mark_structure_changed` increments `structure_revision` and invalidates cached cavity, developmental realization, and harmonic key. The current construction path mutates `ReproductiveConstruction.developing_structure` separately, so it does not need to invalidate the parent’s structure caches today. Once attached units become part of `organism.structure`, every successful anchor/growth/rollback/partition mutation must call the canonical invalidation path exactly when committed; failed candidate changes must leave revisions/caches unchanged.
- The parent’s genome cavity is computed from `organism.structure`. Under one aggregate graph, cavity analysis must still identify the parent’s actual genome core and the internal second core without reinterpreting the whole aggregate as one cavity or changing organism viability implicitly. The current anchor-boundary rule refers to an index in the separate construction graph; migration must remap/replace that identity when units are merged.
- `ReproductiveConstruction` is serialized, while parent `structure` is serialized separately. The current deserializer can load a construction record as a separate graph. A new representation must define a versioned migration that merges each legacy construction graph exactly once into parent structure, or fail closed with a recoverable explicit error. Simply deleting/ignoring the old field would lose physical units, bonds, and bond energy; merging without an idempotence guard could duplicate them on repeated load/save.
- The current `reproductive_construction` option is `serde(default)`, but `committed_material` and `developing_structure` within the record are required fields. Legacy/malformed record behavior must be tested separately from the normal new-schema round trip.
- Geometry snapshot caches are built per frame and map organism indices to only the primary graph body. After unification, the parent mapping must include all attached structure units while stored material stays separate. Movement and structure revisions must ensure no stale cached geometry or resonance key survives a committed construction mutation.

**Required acceptance cases:**

1. New-schema save/load round-trip preserves each aggregate unit, bond, storage item, genome-core identity, and energy holder exactly once.
2. Legacy save with active construction either migrates to the aggregate once or fails closed without silently losing state; repeated save/load cannot merge twice.
3. Anchor attach and successful growth invalidate structure/cavity/developmental/harmonic caches; waiting/rejected growth leaves revisions and caches unchanged.
4. Parent cavity/genome identity remains stable where expected while second-core construction evolves inside the same aggregate; explicit tests prove the two genome cores are not accidentally conflated.
5. Movement translates attached units once and leaves stored material at owner-relative positions; snapshot geometry, collision checks, and receiver mapping agree on the same physical aggregate.

No implementation is authorized by this audit. The serialized model, cache invalidation, and core-identity semantics must be designed together before the first structure-representation change.


### 17.75 Multi-core cavity identity is a prerequisite to one-graph budding — 2026-10-22

**Status: Additional structural dependency confirmed.** Read-only inspection of `src/geometry/cavity.rs`, `src/organism/viability.rs`, and harmonic/cavity call paths at PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`.

- `analyze_genome_cavity` scans connected components and selects the qualifying cavity with the greatest area. It does not accept a core identity or an anchor index as a selection criterion. Harmonic perception and viability call this analyzer on `organism.structure`.
- Today, the developing graph is analyzed separately, so the child's second core can be found without competing with the parent's core. If both structures are merged into one aggregate graph, a simple concatenation may cause the largest-cavity rule to select the wrong core or switch which genome cavity drives the organism's harmonic spectrum, viability, and cached cavity state.
- The owner’s “one physical structure” rule does not mean the simulation can conflate two genome identities. The aggregate may contain the parent core and an internal developing second core, while the parent remains the sole organism/physical owner until division.
- A core-specific cavity query or explicit internal core identity is therefore a design dependency. It must identify the parent’s operative core for current organism perception/behavior and the second core for construction readiness, without hard-coding a material recipe or predefining cavity geometry. Physical cavity geometry remains authoritative; identity only chooses which realized cavity belongs to which genome.
- If the project chooses not to introduce core identity, the alternative is to preserve a separate *analysis view* of the attached core while still keeping one physical structure/owner. Such a view must reference shared unit IDs and geometry rather than duplicate physical units, bonds, energy, or receiver bodies. Treat this as a fallback architecture, not permission to keep a second physical authority.

**Required acceptance tests:** two qualifying cavities in one aggregate remain independently discoverable; parent active cavity/perception does not switch merely because the second core grows; the second core’s construction readiness is checked against its own cavity; structural mutation invalidates both core-specific results; world-tone receiver mapping still maps every attached surface to the parent reservoir; division assigns each core and its units to exactly one child.

**Audit implication:** the single-structure design needs an explicit physical-owner model plus a multi-core identity/query contract before implementation. Do not merge `developing_structure` into `organism.structure` as a blind vector append. No source/test changes.


### 17.76 Current tracker reconciliation after owner decision — 2026-10-22

Section 17.7's current tracker rows for Phases 1, 8, and 9 have been updated to reflect Sections 17.70–17.75. Older dated sections (including 17.38–17.39 and 17.67–17.69) retain their original historical status at the time they were written; their open receiver-policy blocker is superseded by the explicit owner decision in 17.70.

- **Decision resolved:** attached physical/reproductive structure is one parent-owned structure and eligible for living-parent resonance; stored material is excluded.
- **Current design blocker:** current source keeps a separate serialized `developing_structure` graph. Unifying it requires movement/geometry/cache invalidation changes, revised independent inventories, persistence migration/fail-closed policy, and a multi-core cavity identity contract. Do not make a sunlight-only fix or blind graph append.
- **Audit still open:** source/holder and transaction-failure matrices, deterministic rejection tests plus independent inventories, exact-small-world world-tone oracle, chemistry cache catalog invalidation/restart, and whole-world conservation/long-run integrity. No source/test changes have been made in this documentation series.
- **Next concrete task:** finish the single-authority aggregate and core-identity contract plus diagnostic/observation JSON compatibility matrix. Then reconcile remaining audit rows with Section 17.7 and the release gates. Only after all audit templates are complete may the smallest implementation unit be selected; re-fetch PR #205 head and current CI before code changes.


### 17.77 Recommended single-authority representation — one aggregate graph plus core references, not a second physical graph — 2026-10-22

**Status: Proposed design recommendation for review; not yet an owner-approved implementation detail.** Derived from the explicit rules in 17.70 and the dependency findings in 17.71–17.75.

**Recommendation:** make `Organism.structure` the only authoritative physical graph for all attached units until division. Remove `ReproductiveConstruction.developing_structure` as an independently stored physical graph. Keep only the minimum nonphysical construction state required to progress and eventually partition the aggregate, such as child genome, a stable reference to the second core’s anchor/boundary identity, construction energy/stress/debt state if those remain separately budgeted, and any genuinely necessary developmental frame/progress values. Do not tag structural units as “reproductive material”; they are ordinary physical units in the parent-owned structure. `MaterialStorage` remains the separate holder for un-attached stored material.

**Operational contract:**

1. **Start:** stage removal of one exact stored physical instance and attachment of its realized unit(s) into the parent aggregate. Commit storage debit and aggregate graph insertion together; the instance and its structural energy appear once. The child genome becomes internal construction metadata, not a second organism.
2. **Grow:** run a candidate construction step against the authoritative aggregate graph, constrained by a second-core anchor/reference so new bonds attach to the developing core rather than arbitrarily to the parent’s active core. Stage graph/material/energy changes and commit once. Waiting/rejection leaves all parent state and inventory unchanged.
3. **Identify cores geometrically:** evolve `analyze_genome_cavity` (or add a sibling query) to select the qualifying realized cavity associated with a given core reference. A core reference identifies which emergent cavity belongs to which genome; it must not prescribe cavity shape, material recipe, or fixed piece membership. The existing “largest qualifying cavity” global query remains appropriate only where there is a single operative core or where no ambiguity exists.
4. **Perceive and receive:** parent behavior/harmonics use the parent core query. The world-tone snapshot uses the same aggregate graph once; every attached unit is a potential first-hit receiver mapped to the living parent reservoir. Stored physical material and pending transformation escrow remain separate receivers and never credit the owner’s resonance.
5. **Move and mutate:** one graph moves, collides, occludes, and invalidates structure/cavity/harmonic caches once. Do not translate a second construction graph. Candidate failure must preserve graph revision, cache keys, stored material, energy holders, and ledger.
6. **Die:** biological processing stops for the parent aggregate, including reproductive construction. Attached units remain ordinary physical structure and continue ordinary chemistry. No ownership transfer or special decomposition law occurs at breach.
7. **Divide:** at the already-approved division condition, stage one partition of aggregate units, bonds, stored items, core identities, and energy into two children; settle every cross-partition bond explicitly. Commit both child states atomically or leave the parent aggregate unchanged. Do not use “construction finished” as an implicit partial ownership transfer.
8. **Persist:** version/migrate serialized construction records. For a legacy save with a separate graph, validate and merge it exactly once using stable unit/bond identity, or fail closed with a recoverable explicit error. Never silently ignore or duplicate the old graph.

**Why this is the smallest coherent model:** it aligns geometry, resonance, movement, chemistry, material ownership, and death with one physical authority; keeps reproduction-specific state limited to the information needed to track a second genome and growth process; and prevents a sunlight-only patch from masking a broader double-graph inconsistency.

**Risks that still need testable contracts:** stable physical-unit/bond IDs and bond-endpoint remapping during merge; parent/second-core cavity identity under mutations; maintenance and resonance-overload behavior while the aggregate contains two cores; save migration idempotence; construction reserve-holder semantics; and atomic division/cross-bond settlement. This recommendation does not authorize code changes until the remaining audit templates and release gates are closed. If the owner prefers a different single-authority design, revise this recommendation before implementation.


### 17.78 Persistence claim correction — construction serialization is diagnostic output, not demonstrated world save/load — 2026-10-22

**Status: Earlier migration language narrowed after call-site verification.** Exact PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`.

- `ReproductiveConstruction` and `Organism` derive Serde traits, but the current `Simulation` itself does not derive `Serialize` or `Deserialize`. No live world save/load path for restoring a whole `Simulation` was found in the reviewed runtime/process/server call sites.
- The confirmed production use of `reproductive_construction` serialization is diagnostic/observation output in `src/infrastructure/diagnostics.rs` (`serde_json::to_value`) and related observation projections. This is outward-facing JSON representation, not evidence of a simulation persistence/reload contract.
- Therefore, Sections 17.72, 17.74, and 17.77 should not present migration of persisted worlds as an existing mandatory runtime requirement. The current mandatory compatibility task is to update diagnostic/observation JSON contracts and tests so they describe the unified structure accurately and do not expose a misleading separate physical graph.
- If a world-save/load facility is introduced later, versioned migration or fail-closed handling for legacy construction graphs will be required before that feature can restore old state. That is a future compatibility constraint, not a verified current production path.
- The core representation, cavity identity, movement, cache invalidation, independent inventories, resonance mapping, and atomic division concerns remain unchanged.

**Corrected next task:** finish the canonical one-graph/core-identity contract and diagnostic/observation JSON impact matrix; do not spend effort on a nonexistent world-load migration path. No source/test changes.


### 17.79 Diagnostic JSON contract under the unified structure — 2026-10-22

**Status: Diagnostic consumer contract inspected; no dedicated module tests found in reviewed file.** Exact PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`, `src/infrastructure/diagnostics.rs`.

- The diagnostics capture stores `unit_count`, `bond_count`, `component_count`, `structural_mass`, and serialized `structure` from `organism.structure` only. It separately serializes the complete `organism.reproductive_construction` value as `reproductive_construction` JSON.
- Under the single-structure model, these primary structure metrics naturally include attached reproductive units/bonds. The diagnostic construction object should expose nonphysical progress/core metadata and any explicit construction sub-ledger values only; it must not serialize a second physical graph or imply a separate physical owner.
- The diagnostic delta/transition events compare snapshots of the organism, so changing where units are represented will change the expected shape of `structure`, counts, mass, connected components, and construction JSON. Keep event names and tick semantics stable unless an explicit version change is needed.
- No dedicated test functions were found in the reviewed `diagnostics.rs` file for the shape/semantics of the reproductive-construction JSON. Add focused projection tests before or with the implementation: during construction, aggregate counts include attached units once; stored-material counts exclude attached units; construction JSON contains no duplicate physical structure; after division, child structure receives its partition exactly once.
- This is an output compatibility contract, not a world-save/load migration requirement. Current runtime exposes diagnostic/observation serialization but no whole-world simulation persistence path was found.

**Next:** finish the owner-facing canonical aggregate/core-identity contract and diagnostic JSON acceptance cases, then complete the outstanding chemistry transition-failure matrix and release gates. No source/test changes.


### 17.80 Existing stable genome-boundary IDs can inform core identity — but cavity query ignores them — 2026-10-22

**Status: Core-identity audit refined by exact source inspection.**

- `OrganismStructure` (implemented as `PhysicalConstituentGraph`) already stores stable `PhysicalConstituentId` values and one `genome_constituent_ids` set. Juvenile realization derives that set from the qualifying cavity's actual boundary units. This is emergent geometry-derived identity metadata, not a resource/material role.
- The set currently identifies the organism's established genome constituents for structural membership. However, `analyze_genome_cavity` does not use it to select a particular cavity; it recomputes enclosed regions across connected components and returns the largest qualifying cavity. Thus it does not by itself solve a second simultaneously developing core.
- `anchor_structure` currently instantiates the second core's initial anchor in a separate graph and returns a unit index; it does not assign a second stable genome-boundary ID set. Construction later checks the cavity in that separate graph and requires the original anchor index to remain on its boundary.
- **Refined design path:** preserve the existing parent's stable core identity, introduce a minimal second-core identity/reference when needed (initially anchored to the transferred physical constituent ID; once a cavity qualifies, track its realized boundary IDs), and make cavity queries core-specific without adding a “reproductive material” tag to units. The graph owns all units/bonds; core references only disambiguate which emergent cavity belongs to which genome.
- A single global `genome_constituent_ids` set is insufficient for two cores. Prefer a small collection of core identities referencing existing physical IDs over a second structure graph. The identity must survive unit-index changes and be updated transactionally if a boundary constituent is removed or transformed.
- Required tests: preserve parent boundary IDs while adding child anchor; independently select parent and second-core cavities in one connected aggregate; ensure growth/chemistry changes don't silently switch core identity; reject dangling core references atomically; division transfers the correct core identity to each child exactly once.

This refines, rather than reverses, Section 17.77: one physical graph remains the recommendation; core identity is metadata over that graph, not a separate physical owner. No source/test changes.


### 17.81 Graph-unification mechanics — stable-ID remapping is mandatory — 2026-10-22

**Status: Graph primitive audit complete for the relevant merge concern.** Exact PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`, `src/geometry/structure.rs`.

- `PhysicalConstituentGraph::new` starts `next_constituent_id` at 1, and `add_unit` assigns IDs from that graph-local counter. A separate parent graph and construction graph can therefore both contain IDs such as 1, 2, etc.; concatenating their `units`/ `bonds` vectors would create identity collisions.
- Bonds address endpoints by `PhysicalConstituentId`, not just vector indices. Any graph merge must stage an injective old→new ID mapping for every incoming construction unit, then rewrite both endpoints of every incoming bond before validation/commit. Parent genome boundary IDs and the second-core anchor/boundary references must use the same mapping.
- Existing `add_unit` assigns a fresh ID, but the reviewed graph API does not expose a complete atomic graph-merge operation that remaps all incoming bonds and core references together. A reusable merge helper should stage the complete candidate graph, validate all endpoints and physical bonds, then return the candidate for a higher-level transaction; it must not mutate the live graph partially.
- Removing units also removes bonds that touch those physical IDs and prunes the graph's single `genome_constituent_ids` list. With two cores, the future core-reference collection must prune/reject dangling references transactionally rather than silently changing which cavity a genome represents.
- Required tests: colliding source/destination ID ranges merge without endpoint aliasing; every incoming bond retains the same physical endpoints after remap; malformed/missing endpoints reject with no live mutation; parent core IDs remain unchanged; second-core references remap exactly; independent structural-energy inventory is identical before/after a pure graph merge.

**Audit implication:** one authoritative graph is feasible in principle but requires a tested ID-remapping transaction, not vector concatenation. This helper is a candidate future implementation unit only after all audit templates are signed off. No source/test changes.


### 17.82 Construction currently runs separate biological maintenance/death before division — owner model requires reconciliation — 2026-10-22

**Status: Lifecycle contract gap confirmed at exact PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`.** No source/test changes.

- `advance_construction` builds a temporary `Organism` from `ReproductiveConstruction`, copies `developing_energy`, `developing_stress`, and `developing_maintenance_debt` into it, calls `apply_maintenance`, calls `validate_stress_state`, and—if not dead—calls resonance-overload BREAK. It then copies the temporary organism's energy/stress/debt/structure back into construction state.
- This is a separate biological-maintenance/death/overload lifecycle for the unfinished core even though the owner-approved model says the parent remains the sole organism/owner until division. It also duplicates the structural graph, while the temporary child has its own empty resonance reservoir and its own stress threshold.
- In `Simulation::tick`, a returned `ConstructionStatus::Dead` is included in the same finish branch as Ready and Detached. `finish_reproduction` takes the construction record and can still return a child when the developing structure is nonempty, even if readiness is false. Thus “Dead” does not reliably mean construction was abandoned; this needs a deterministic test and explicit intended semantics before implementation.
- **Recommended model implication:** before division, apply biological maintenance/stress/death eligibility to the parent aggregate, not to a temporary child organism. Construction should be an internal process owned by the parent; the unfinished core is physical structure and geometry, not a second living organism. Keep any construction reserve as an explicitly parent-owned sub-holder only if needed for budgeting; it must not imply independent biological agency. A parent death stops construction progress while leaving all physical units and stored material in the world for ordinary chemistry.
- Do not assume this recommendation authorizes silently deleting `developing_energy`, stress, or debt fields. First determine which are necessary accounting sub-holders versus duplicated child-organism state, then update the material/energy inventory and maintenance acceptance matrix.

**Required tests:**

1. No pre-split maintenance/death/resonance-overload processing occurs on a separate temporary organism.
2. Construction advances only while parent is biologically eligible; dead parent retains construction physical units/material/energy but performs no growth/maintenance/finish behavior.
3. Construction status cannot produce an offspring after a failed/dead construction unless an explicit, tested recovery rule permits it.
4. Parent maintenance accounts for the full attached aggregate exactly once; unpaid maintenance/debt and stress are applied to the parent owner.
5. If a separate reserve sub-holder remains, transfers are atomic and total parent-owned energy is conserved through waiting, growth, death, and division.

**Open owner-facing semantic point:** confirm whether the parent’s existing maintenance/stress model should apply to the entire aggregate during internal budding, with no independent developing-core maintenance/death state. The current owner rules strongly suggest this, but the detailed energy/stress allocation should be explicit before code changes.

### 17.83 Owner-confirmed physical-state classification — no reproductive material tag or separate developing graph — 2026-10-22

**Status: Owner decision recorded; design direction approved, implementation and verification still open.** This section records the owner's explicit clarification after Sections 17.77–17.82.

**Canonical classification**
- **Attached physical structure:** the one authoritative physical graph owned by the organism. This includes units introduced/committed during reproductive construction once they are structurally attached. They are not a separate “reproductive material” category and are not excluded from resonance because of their origin.
- **Stored material:** unattached physical units in the organism's storage holder. These do not contribute to the owner's resonance capture. They remain physically present for material accounting and may still occlude/receive onward signal according to world-tone propagation rules.
- **Construction state:** nonphysical metadata needed to track the second genome/core, progress, and any explicitly justified energy-accounting sub-holder. The persistent/current `developing_structure` physical graph should be eliminated. A transient trial graph may exist only inside an atomic candidate calculation, never as a second owned physical state.

**Consequences for implementation**
1. Change physical-state representation so attachment, not reproductive designation, determines whether a unit belongs to the organism's structure and resonance receiver set.
2. A successful construction step must atomically remove its exact source from storage/environment and attach it to the aggregate graph, including bonds and structural energy once. Waiting/rejection leaves the pre-state unchanged.
3. World-tone geometry must enumerate the aggregate structure once and map every attached surface to the organism's single resonance reservoir. Stored material is excluded from owner capture; it must not be counted again as attached structure.
4. Chemistry and ordinary post-death chemistry must operate on the physical aggregate using shared reaction rules. Stored-holder reaction reachability remains a separate open gap; do not solve it by pretending stored material is attached.
5. Preserve two genome/core identities without two physical graphs. Use stable physical IDs/core references and core-specific cavity selection; never mark units by reproductive role to make cavity or resonance decisions.
6. Revise movement, collision/occlusion, structural-energy snapshots, integer material inventory, structure/cache invalidation, and diagnostic JSON together. Diagnostic construction output should carry only nonphysical progress/core metadata and justified sub-ledgers, not a second physical graph.
7. Keep any developing-energy/reserve, stress, maintenance-debt, or progress field only if its accounting/lifecycle purpose is explicitly established. This decision rejects a second physical structure, but does not by itself settle every sub-ledger equation or the exact maintenance/stress policy.
8. The approved division operation must still partition physical units, bonds, stored material, energy holders, and core identity exactly once; cross-partition bonds require explicit settlement.

**Audit correction:** Sections 17.70–17.81 contain historical audit findings and recommendations; read them through this owner-confirmed classification. Section 17.82's separate-graph and separate-temporary-organism findings remain valid observations of current source. Its open question about exact parent maintenance/stress allocation is not resolved by this decision and must be audited without restoring a second physical graph.

**Next:** update the handoff and PR #204 description to point to this decision, then finish the source/holder and transaction-failure matrices. No source/test files have changed; implementation remains gated on audit-template closure.

### 17.84 Source/holder and transaction-failure matrix — owner decision applied to current source — 2026-10-22

**Status: Source impact for the approved physical-state classification reconciled across reproduction, world-tone, movement, ledger, and diagnostics. The failure cases are now explicitly classified; test coverage and release gates remain open.** Read-only source review at PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`. No source/test changes.

#### A. Aggregate physical ownership and resonance

| Transition / holder | Current source behavior | Classification after D16 / required work |
|---|---|---|
| Begin reproduction: stored anchor → construction | `begin_reproduction` clones a stored physical instance into an anchor workspace, calls `anchor_structure`, then removes the selected instance from a trial copy of parent storage and installs `ReproductiveConstruction { committed_material, developing_structure, ... }`. Parent's primary graph is unchanged. | **Representation gap.** The anchor is not lost, but it becomes a separate physical graph rather than parent structure. Replace with a staged transaction that inserts into the aggregate graph once and debits storage once; stable-ID remapping and parent cache invalidation are required. |
| Construction growth | `advance_construction` builds a temporary organism from the construction graph; candidate growth can consume parent storage, then writes the resulting child graph back to `developing_structure`. Waiting returns before committing candidate ledger/graph in the reviewed path. | **Representation/lifecycle gap.** Keep waiting/rejection atomic, but successful growth must extend the same aggregate graph. Preserve explicit source removal, energy settlement, and physical validation in one transaction. |
| Construction reserve / maintenance | Reserve transfer uses `ledger.transfer` but ignores its boolean result. A temporary child receives separate maintenance, stress/death validation, and resonance-overload processing; those fields are copied back. | **Failure-policy and duplicate-lifecycle gap.** Test transfer rejection and clarify whether reserve funding waits/fails. Do not create a second physical or biological owner; audit which reserve/stress/debt sub-ledgers remain necessary. Exact parent maintenance/stress allocation remains unresolved. |
| Construction finish/division | `finish_reproduction` takes the construction record and can return a child if the separate graph is nonempty even when readiness is false. Simulation routes Ready, Detached, and Dead into the finish branch. | **Lifecycle/atomic partition gap.** Define one explicit division predicate and atomic partition of units, bonds, stored material, energy holders, and core identity; failure leaves parent aggregate unchanged. A dead/failed construction must not produce a child absent an explicit recovery rule. |
| World-tone receiver geometry | `world_tone_snapshot` adds primary structure, `developing_structure`, `committed_material`, stored material, field objects, and escrow as separate bodies. Only primary organism structure maps to the owner's reservoir. | **Receiver-policy decision is settled; implementation gap confirmed.** Enumerate aggregate attached units once and map them to the owner's reservoir; stored material remains a separate non-crediting receiver/occluder. Preserve first-hit allocation and per-frame balance. |
| Energy/material inventory | Structural energy and test inventory separately sum primary graph and developing graph; usable inventory separately includes developing energy. | **Oracle reconciliation required.** Count aggregate graph/bonds once; retain a separate usable-energy sub-holder only if its parent ownership and transfers are explicit. Update integer material categories so attached units are not counted as stored or duplicated. |
| Diagnostics | Diagnostic snapshots serialize primary structure and separately serialize the whole construction record. | **Output contract update required.** Aggregate structure counts/mass include attached units once; construction JSON holds only nonphysical progress/core metadata and explicitly justified sub-ledgers. Add projection tests. |
| Core identity/cavity | Current construction analyzes the separate graph and uses `anchor_unit_index`; global cavity analysis selects the largest qualifying cavity and does not use stable core IDs to select a specific core. | **Core-specific query gap.** Use stable physical IDs/core references over the aggregate graph; remap incoming IDs/bond endpoints atomically and select parent/second-core cavities independently without prescribing shape or material recipe. |

#### B. Material/energy transition rejection matrix

| Transition | Failure point / current evidence | Classification and required proof |
|---|---|---|
| Initial-condition source | One-time external-input transaction and duplicate-claim guard exist. | **Test-required:** tick-zero/no-op inventory and duplicate initialization/replay. |
| COMBINE | Trial graph and energy transaction precede live graph commit. Prepared ledger delta helpers omit escaped-tone/resonant-held fields; current COMBINE should leave them zero. | **Test-required:** assert invariant zero or implement symmetric commit/rollback; rejection/retry must preserve graph, holders, ledger. |
| Chemical/resonance BREAK | Candidate graph/reservoir settlement is staged. | **Test-required:** stale/retry/rejection inventory, invalid resonance properties, exactly-once bond-energy release. |
| Stored BREAK | Split pieces commit incrementally; fallback deposit results are ignored; malformed completion can clear the active marker without restoring escrow. | **Confirmed atomicity gap:** failure after first piece, storage+deposit rejection, malformed completion, and whole-world integer inventory. |
| Environmental COMBINE source | Source/escrow/graph/storage paths are partially staged, with exact-cell restoration in some cases. | **Test-required:** missing source/cell and malformed prepared state preserve source, graph, storage, energy, and ledger. |
| Contained acquisition | Field objects are removed before storage; failed storage attempts a fallback deposit whose result is ignored. | **Confirmed ownership gap:** force both storage and fallback-deposit rejection; no physical unit may disappear. |
| Expulsion | Stored instance is removed before final field deposit; final deposit result is returned, and earlier restoration attempts ignore restoration failure. | **Confirmed ownership gap:** inject destination and restoration rejection; preserve a valid holder and unchanged inventory. |
| Movement | Geometry changes before ledger settlement; failed settlement returns without restoring geometry or movement progress. Movement transaction is currently heat-only with zero explicit work. | **Confirmed atomicity gap + D7 review:** `try_move_cell_with_reason` can also push other organisms and field physical materials before `advance_movement` settles the mover's energy. A later ledger rejection therefore needs to roll back the mover, every pushed organism, pushed field objects and cell indexes, construction anchors, spatial index, and movement progress—not only the mover's coordinates. Reconcile work/heat semantics without silently changing physics. |
| Maintenance | Energy settlement precedes stress update; debt can accumulate. | **Test-required:** insufficient funds/debt and transaction rejection preserve stress/ledger correctly. |
| Construction reserve/growth/finish | Transfer result ignored; separate graph; stale source and finish-state edges remain. | **Test-required:** transfer rejection, stale source, anchor/holder failure, malformed/dead finish, and before/after independent inventories. |
| World-tone frame | Frame-local incident = retained + heat + escaped balance and staged reservoir commit exist. | **Test-required:** exact small-world first-hit oracle, aggregate receiver mapping, invalid property behavior, residual and performance/memory evidence. |
| Reaction accumulation | Bondless early return can retain stale activation keys. | **Test-required:** stale-key cleanup and overflow/invalid-helper behavior; activation remains nonphysical state. |
| Chemistry cache | Schema/manifest validation and append/manifest rollback exist; catalog chemical positions are not encoded in ChemistryKey, and manual fingerprint invalidation lacks a catalog-change test. | **Test-required:** catalog-property change requires fingerprint bump and rejects old cache; reopen/restart/failure tests. |
| Post-death chemistry | Dead primary structures continue ordinary chemistry; stored and construction holders are not enumerated by the same reaction pass. Legacy recycler remains unintegrated and unsafe. | **Reachability gap:** extend ordinary chemistry to physically valid holders without a separate decomposition law or double-counted bonds/material; prove dead-parent multi-tick inventory. |
| Global conservation / long-run | Independent current-state energy and integer-material snapshots exist, but cumulative flow provenance is not independently reconstructed end-to-end. | **Open release gate:** cross-domain source/sink inventory, residual tolerance, overflow/drift, reservoir, and long-run tests. |

**Mutation-call-site cross-check:** targeted exact-head review of `material_storage.rs`, `material_transfer.rs`, `material_restoration.rs`, `physical_material.rs`, `environmental_materials.rs`, `simulation/environment.rs`, `structure_authority.rs`, `construction/runtime.rs`, COMBINE/BREAK, expulsion, recycling/decomposition, movement, reproduction, world-tone snapshot, and simulation inventory found no additional production `EnergyLedger.transfer` caller beyond the construction-reserve path and quarantined legacy recycler already mapped in 17.56. It did refine the movement failure scope to include pushed organisms and physical field objects. This was a targeted source scan, not a claim that every material mutation has passing failure-injection coverage.

**Reconciliation:** The former owner-decision blocker for attached-structure resonance is closed by D16. The implementation and exact-small-world oracle gates remain open. The matrix identifies which rows are source-inspection complete versus test-required, but the chemistry audit is not ready to authorize code: failure-injection tests, aggregate-representation design closure, and release-gate evidence remain outstanding.

**Next:** review the remaining production material-mutation call sites against this table and confirm no unlisted path bypasses the same ownership/transaction contract. Then reconcile Section 17.7 and Section 17.6 release gates. No source/test changes.

### 17.85 Targeted physical-material mutation call-site audit — no new transaction category found — 2026-10-22

**Status: Targeted source inspection complete for the high-risk holder/geometry paths listed below; failure-injection coverage remains open.** Exact PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`. No source/test changes.

- `MaterialStorage::try_store_physical_instance_at_owner_anchor` returns the physical instance on rejection, so callers can preserve ownership if they check the result. The known contained-acquisition caller does attempt a field return, but ignores a failed return; this remains the documented ownership gap.
- `ActiveMaterialField::take_contained_physical_materials_in_regions` removes each fully-contained realized object from its cell and returns it to the caller, marking changed cells. The field operation itself is a transfer into a temporary return vector; the ownership obligation then belongs to the acquisition loop, which currently can discard the returned instance if both storage and fallback deposit fail.
- `ActiveMaterialField::deposit_physical_at_index` validates that the object is realized and material-valid, then inserts it into the indexed cell and marks the cell changed. Callers must validate indexes and must not treat a rejected deposit as successful ownership transfer. Initial-world seeding is additive creation, not an ownership transfer; its ignored result is not the same loss mode as expulsion/acquisition.
- `MaterialTransfer::take_whole_unstructured` rejects invalid, fractional, empty, or structured aggregates before mutating; after validating allocations it removes whole units and returns the extracted material. Its contract is narrower than physical-object transfer and does not resolve any of the physical storage/environment transaction gaps.
- `material_restoration`, COMBINE candidate construction, and construction runtime build trial graphs before committing their live structure. Their graph `add_unit` calls are not themselves independent world-material holders; the higher-level transaction must still commit the source removal and candidate graph once.
- Environmental COMBINE preparation removes the exact source from its field cell and records the source/cell for rollback. If prepared physical state is malformed, resolver rollback attempts exact-cell restoration; a missing/invalid restoration cell remains a required failure-injection case. Prepared ledger rollback also has the previously documented escaped-tone/resonant-held field omission.
- Stored BREAK remains a live multi-output ownership gap: after `break_internal_bond` mutates the stored object into pieces, each piece is deposited/stored separately and failed destination results are ignored. The source object no longer exists as one intact holder after the split begins, so atomic rollback or an explicit escrow holder is required.
- Expulsion similarly takes the stored object before final deposit; its earlier validation-failure paths attempt restoration but ignore whether restoration succeeds. Final deposit rejection returns false without restoring the source.
- The legacy recycler drains stored and committed material before all deposits/energy transfers have succeeded and ignores field-deposit results. It remains quarantined and must not become live death processing.
- Targeted source search across the material holder, field, restoration, COMBINE/BREAK, construction, movement, reproduction, and energy-ledger files found no new production internal-energy-transfer caller beyond the two already recorded in Section 17.56: construction reserve and the quarantined recycler. This is a bounded call-site audit, not evidence that every physical mutation has passing rejection tests.

**Audit conclusion:** the known high-risk mutation paths are represented in Sections 17.55, 17.64, 17.67, and 17.84. The outstanding work is primarily deterministic failure injection and independent inventory assertions, plus the single-aggregate representation/core-identity design—not an untracked third energy-transfer API.

**Next:** reconcile these findings with the phase tracker and the Section 17.6 release checklist; then close each test-required audit template explicitly. No implementation until audit closure.



## Movement transaction and rollback audit — Section 17.86

- **Audit date:** 2026-10-22. **PR #205 exact head inspected:** `12d8079230458da68a8dc3755a1d6a9b41347862`. No source/test changes.
- `MovementEngine::advance_movement` calls `try_move_cell_with_reason` first; that path resolves a push plan, mutates pushed organisms and field physical objects, moves the primary organism and its current separate `developing_structure`, marks position changed, and refreshes the moving organism's spatial-index entry. Only after all of those physical mutations does `advance_movement` call `EnergyLedger::settle_transaction` for the movement cost.
- If settlement rejects, the function returns `InsufficientEnergy` without restoring the mover, pushed organisms, pushed field objects, field revision/cell placement, cache/position invalidation, or spatial-index state. The active movement step is not decremented, so the same operation can retry after having already changed the world. This confirms the previously recorded geometry-before-ledger gap; the rollback boundary is broader than the mover alone.
- `apply_push_plan` removes each planned field object from its old cell, translates it, and reinserts it only when the translated object's first placement resolves to a field index. There is no fallback branch if that lookup fails. Existing `can_translate_physical` may make that branch unreachable for valid inputs, but this invariant is not yet proven by a rejection test; keep it as a targeted test/audit obligation, not a confirmed normal-path loss.
- The existing movement flow stages collision feasibility in `PushPlan`, but the plan is only a feasibility description, not a full rollback snapshot or a candidate world state. The safe implementation direction remains: prepare all resulting positions/index updates and energy settlement before publishing any mutation, or capture and restore every affected state on rejection. A test-only deterministic settlement-rejection seam must assert the mover, every pushed organism, every pushed physical field object, field revision/cell contents, indexes/anchors/caches, movement progress, and independent material/energy inventories are unchanged.
- D16 conflict to preserve during future implementation: movement currently translates `ReproductiveConstruction.developing_structure` as a second graph. The canonical representation has one parent-owned graph for every attached unit; after that change, movement/collision/spatial indexing must include attached units exactly once and must not separately translate duplicate construction geometry. This is a downstream representation dependency, not permission to patch movement alone.
- **Audit status:** movement mutation ordering is confirmed; complete rejection/rollback coverage is still missing. This closes one source-inspection row only; no release gate is closed.
- **Next:** finish the remaining deterministic failure-injection rows (material acquisition/expulsion/BREAK, COMBINE restoration/ledger rollback, construction reserve rejection, stale reaction-key cleanup, cache catalog/restart, and post-death aggregate inventory), then reconcile each row against Section 17.6 and the phase tracker. Do not modify source/tests until all audit templates are complete. Before the first implementation change, refresh PR #205 head and exact-head CI.


## Contained acquisition and expulsion rejection-path audit — Section 17.87

- **Audit date:** 2026-10-22. **PR #205 exact head inspected:** `12d8079230458da68a8dc3755a1d6a9b41347862`. No source/test changes.
- Contained acquisition calls `take_contained_physical_materials_in_regions`, which removes contained objects from the field before returning them. It then attempts `try_store_physical_instance_at_owner_anchor`. On rejection, it only attempts a field fallback when the returned object has a first placement, and ignores the result of `field.deposit`; if the placement is absent, no fallback is attempted. Thus the source-to-destination transition has no authoritative rollback if both storage and fallback fail. A deterministic test must inject each failure and compare independent field+organism material inventories and exact physical-object identity/geometry.
- Expulsion calls `stored_material.take_physical_at` before validating geometry and computing the destination. Several later validation failures attempt `store_physical_instance(physical)` but ignore its boolean result. On the final path, it returns `environment.field.deposit_physical_at_index(index, physical)`; if that returns false, the already-removed storage source is not restored. These are distinct rejection points: source removal, attempted storage restoration, and final destination insertion. The test must prove exact restoration on every failure, not merely equality of aggregate material totals (which could conceal duplicate/lost geometry or changed endpoints).
- `MaterialStorage::try_store_physical_instance_at_owner_anchor` validates realized geometry, valid/discrete material, and a first placement; it then rebases into intrinsic coordinates and inserts. `Environment::deposit_physical_at_index` rejects unrealized, empty, or invalid material before mutating the field. Normal valid input may make some failures rare, but the transaction contract must still be atomic under malformed state and deterministic failure injection.
- **Required acceptance evidence:** for acquisition and expulsion, cover (1) storage rejection with successful field fallback, (2) storage rejection plus field fallback rejection, (3) malformed/missing placement, (4) expulsion validation rejection with restoration accepted, (5) restoration rejection, and (6) final destination rejection. On every rejected operation, assert exact holder membership/identity, placements and internal connections, spatial/cell index consistency, and independent whole-world material inventory; successful operations must transfer the same physical instance exactly once.
- **Audit status:** source loss paths are confirmed in code; deterministic rejection coverage and independent-inventory proof are not established. This closes a source-inspection row only.
- **Next:** continue the remaining failure matrix with stored BREAK escrow/partial split and environmental COMBINE restoration/ledger rollback, then construction reserve transfer, stale reaction keys, cache catalog/restart, and post-death aggregate inventory. No implementation until every audit template is complete; refresh PR #205 head/CI before the first source/test change.


## Stored BREAK escrow and environmental COMBINE rollback audit — Section 17.88

- **Audit date:** 2026-10-22. **PR #205 exact head inspected:** `12d8079230458da68a8dc3755a1d6a9b41347862`. No source/test changes.
- **Stored/environmental BREAK:** environmental BREAK removes the selected field object at transformation start; stored BREAK similarly carries a removed stored instance in `ActiveTransformation.stored_material`. On completion, a missing source, missing bond target, missing prepared-energy marker, or failed `break_internal_bond` clears `active_transformation_id` and returns without restoring the escrowed source. The normal split path then processes pieces one at a time. Environmental pieces call field deposit and ignore its result; stored pieces call storage insertion, and on rejection try field deposit while also ignoring that result. Therefore partial split, malformed completion, and destination rejection can lose part/all of the original physical object after the intact source has already been consumed. The earlier start/preparation failure helper does not cover malformed resolution after the transformation has entered the completion path.
- **Required BREAK contract/tests:** escrow must remain authoritative until every output destination is validated and the complete split can commit atomically. Inject (1) missing/malformed completion fields, (2) failed bond split, (3) first/middle/final piece storage rejection, (4) field rejection for any piece, and (5) source-owner disappearance. Each rejection must restore the intact original or a provably equivalent complete partition exactly once; verify connection endpoints, material inventory, energy/stress/ledger, active transformation markers, and field/storage membership.
- **Environmental COMBINE:** preparation clones the organism and local physical field into a trial state, computes a ledger delta, removes the exact environmental source, stages organism usable energy and selected ledger fields, then resolution commits the prepared structure/storage. If prepared physical state is missing, rollback reverses usable energy and the recorded ledger delta, then attempts to restore the environmental source to its original cell. This is a stronger staged pattern, but the rollback helper omits `total_world_tone_energy_escaped` and `total_resonant_energy_held` from the ledger delta reversal even though the delta struct captures both. Current COMBINE is not expected to change those fields; make that a tested zero-delta invariant or centralize complete field-wise delta handling before any future caller can mutate them.
- If the saved original cell index is missing during failed COMBINE resolution, the environmental source restoration silently does nothing. Test malformed/stale source-cell state explicitly and prove the source remains recoverable; do not assume exact-cell insertion always succeeds just because the ordinary tick path is synchronous.
- **Audit status:** the BREAK loss paths and COMBINE rollback-field omissions are source-confirmed; deterministic malformed-state/rejection tests and independent inventory assertions remain open. This is audit evidence, not a fix or release-gate closure.
- **Next:** continue with construction reserve-transfer rejection and stale bondless reaction-key cleanup; then cache catalog/restart, post-death aggregate inventory, world-tone exact-small-world oracle, and final release-gate reconciliation. No implementation until all audit templates are complete. Refresh PR #205 head and exact-head CI before first source/test change.


## Construction reserve-transfer rejection and reaction-key cleanup audit — Section 17.89

- **Audit date:** 2026-10-22. **PR #205 exact head inspected:** `12d8079230458da68a8dc3755a1d6a9b41347862`. No source/test changes.
- `advance_construction` computes a bounded reserve transfer from parent usable energy to `construction.developing_energy` and discards the boolean returned by the atomic `EnergyLedger::transfer`. It then creates the temporary developing organism and proceeds through maintenance/stress/overload and construction attempts regardless of whether reserve transfer succeeded. Under ordinary finite valid state the computed transfer should generally pass, but malformed/rejected holder values can produce progress/maintenance without the requested reserve transfer. Owner contract must decide whether reserve funding is required or best-effort; absent explicit best-effort semantics, rejection must leave both holders unchanged and return a waiting/failure state before any dependent work. Test exact holder/ledger/material invariants on rejected transfer.
- D16 makes this more than a local boolean fix: current `advance_construction` applies maintenance, stress/death validation, and resonance-overload BREAK to a temporary child organism with a separate physical graph, then copies graph and accounting values back into construction. The approved aggregate model disallows a second authoritative developing physical graph and a second pre-division organism lifecycle. Preserve only necessary nonphysical core/progress metadata and explicitly justified accounting sub-holders; parent-owned attached structure must be maintained/processed once under a still-to-be-specified aggregate energy/stress allocation. Do not patch reserve transfer in isolation.
- `chemical_reaction::accumulate` returns immediately when the primary graph has no bonds, before pruning stale keys for that organism from the shared accumulation map. The subsequent cleanup retains only entries under the current structure-revision prefix, but the early return bypasses it. A structure can become bondless after BREAK/removal while old revision activation values remain indefinitely. These entries are not physical energy, but stale activation can misrepresent future reaction readiness if a key becomes reusable; tests must verify cleanup after bond loss, revision changes, and re-formed bonds without changing the authoritative energy inventory.
- **Audit status:** ignored transfer failure and stale-key cleanup bypass are source-confirmed; rejection and stale-state regression tests remain open. No source/test changes and no phase/release gate is closed.
- **Next:** finish cache catalog-fingerprint/restart and post-death aggregate inventory audits, then world-tone exact-small-world oracle and final release-gate reconciliation. No implementation until all audit templates are complete; refresh PR #205 head/CI before the first source/test change.


## Cache invalidation and post-death/aggregate receiver audit — Section 17.90

- **Audit date:** 2026-10-22. **PR #205 exact head inspected:** `12d8079230458da68a8dc3755a1d6a9b41347862`. No source/test changes.
- Chemistry cache schema 4 validates the manifest's schema, entry count, and `CHEMISTRY_EQUATION_FINGERPRINT`; a test rejects an arbitrary mismatched fingerprint. The fingerprint is a manually maintained constant containing a catalog marker (`catalog=resource-chemistry-v1`). Static reaction potential depends on catalog chemical-position values as well as chemistry constants, while the cache key does not carry those numeric values. Current tests do not demonstrate that changing a resource's chemical position while retaining the old marker rejects the old cache. Add a catalog-dependency test that mutates a relevant position, proves the canonical fingerprint/marker must change, and confirms old persisted records are rejected; add reopen/restart coverage for mismatch and successful current-cache reuse. Prefer deriving the fingerprint from canonical relevant catalog fields if that can be done deterministically without making the cache expensive.
- The current independent energy snapshot recomputes graph-bond energy from both `organism.structure` and `ReproductiveConstruction.developing_structure`, while usable-energy inventory includes the separate construction holder. This correctly attempts to count current state once under the current representation, but directly encodes the two-graph model that D16 rejects. After unification, update the oracle to enumerate the single parent-owned graph once plus distinct accounting sub-holders; add before/after structural-energy equality tests for graph merge/attachment and exact-once partition at division.
- The world-tone receiver collector independently enumerates the parent's primary graph, `developing_structure`, committed physical material, stored material, and legacy decomposition structures. Under D16, attached developing units must be part of the parent's one receiver graph and credit the parent's resonance reservoir under the same eligibility/settlement rules; stored material remains excluded from owner capture, though it can still intercept/propagate the signal and dissipate energy according to the physical rule. Do not patch receiver mapping without simultaneously updating the authoritative graph, geometry/occlusion, movement, inventories, diagnostics, and core-specific cavity identity.
- Death handling must preserve the same physical aggregate and ordinary chemistry without transferring ownership or creating a special decomposition object. Existing production inventory paths still contain separate construction graph/accounting terms; the legacy `process_decomposing_bodies` call found in the reviewed simulation source is test-only, and the legacy recycler remains unsafe/quarantined. Required bounded-live-tick test: breach the parent core with non-empty storage and active construction, continue ordinary chemistry, and compare independent material inventory and usable/resonant/structural holder snapshots. Assert no duplicate or lost attached units, no special death-time transfer, no biological growth after breach, and no stored-material owner resonance capture.
- **Audit status:** manual cache invalidation boundary and missing catalog-position/restart test are confirmed; the aggregate inventory/receiver mismatch with D16 is confirmed at source level. No implementation or test evidence has closed these gates.
- **Next:** reconcile the remaining world-tone exact-small-world/performance oracle, independent whole-world material and energy accounting, and the exit checklist; then list every remaining gate explicitly. Do not begin source/test changes until audit templates are complete; refresh PR #205 head and exact-head CI immediately before the first implementation change.


## World-tone geometric oracle and scaling gate — Section 17.91

- **Audit date:** 2026-10-22. **PR #205 exact head inspected:** `12d8079230458da68a8dc3755a1d6a9b41347862`. No source/test changes.
- The current world-tone snapshot collects realized bodies as both occluders and possible receivers, computes an ordered set of parallel-ray interaction paths, and checks that first-hit path widths reconcile with the frame's direct projected exposure for each unit of each living primary structure. Each path then passes a single travelling spectrum through its ordered hit list; response energy is removed once, non-retained response becomes heat, and only the unresponded remainder propagates onward. The whole source frame stages reservoirs and settles incident = retained + heat + escaped before publishing reservoir changes. This is consistent with the owner rule that propagation changes frequency/energy rather than literal reflected/bouncing rays.
- Existing focused tests cover analytic isolated-circle/straight-edge projection, curved and polygonal partial/full shadows, living/nonliving receiver allocation, and frame-level conservation. The reviewed tests do not provide an independent brute-force/exhaustive small-world oracle for the entire body→path→receiver allocation pipeline, nor a reproducible scale benchmark for thousands of bodies. The path-width self-consistency check is useful but is not an independent geometry oracle because both sides use related frame/path machinery.
- **Required exact-small-world oracle:** build tiny deterministic worlds with known analytic or exhaustively sampled source lanes, overlapping/shadowed surfaces, stored material, attached second-core surfaces, and downstream receivers. Compare each receiver's exposure/path ordering within a declared tolerance, verify one source signal only, and independently recompute incident/captured/heat/escaped totals. Include degenerate geometry and zero-exposure cases; an invalid frame must abort without partial reservoir or ledger mutation.
- **Required performance gate:** benchmark the same reproducible worlds at increasing receiver/occluder counts (including thousands of physical bodies) under the five-minute command cap. Record wall time and object/path counts; compare current geometry with the approved approximation or sweep/broad-phase candidate while preserving first-hit geometry and downstream frequency-shifted propagation. No per-object sun tracing or per-tick sun recalculation should be accepted without measured justification. The prior reported 272-second geometry/ray test is historical context, not a current benchmark.
- D16 remains a prerequisite for the final receiver oracle: attached construction units must be in the single parent-owned graph and contribute to that living parent's reservoir once; stored material remains capture-ineligible but can occlude/receive heat. Avoid treating the existing separate developing graph as the intended final receiver model.
- **Audit status:** energy settlement is locally staged and path-level conservation is checked; independent whole-pipeline geometric accuracy and scale/performance gates remain open. No source/test changes and no release gate closed.
- **Next:** finish the global material/energy inventory proof and reconcile every test-required audit row with Section 17.6 and the phase tracker. Once templates are complete, select the smallest implementation unit, refresh PR #205 head/CI, and proceed one audited change at a time.


### 17.92 Consolidated exit checklist and next implementation gate — 2026-10-22

This reconciliation supersedes the stale “next” lists in older dated audit checkpoints. It does not close any unchecked release gate in Section 17.6. PR #204 remains documentation-only; no production/test changes have been made during Sections 17.86–17.91.

| Gate / audit template | Current classification | Evidence still required before source/test implementation |
|---|---|---|
| D16 single aggregate physical graph | **Design approved; implementation contract incomplete** | Canonical unit/bond ID merge and endpoint remapping; stable parent/second-core references; parent-specific cavity query; construction metadata/sub-holder definition; parent maintenance/stress/death/reserve allocation; diagnostic projection. |
| Material transaction failure matrix | **Source paths mapped; test-required** | Acquisition storage+fallback rejection; expulsion restoration/final deposit rejection; stored BREAK malformed completion/partial split; environmental COMBINE missing-source/cell rollback; movement ledger rejection including all pushed entities and indexes. Every rejection must preserve exact physical ownership and independent inventory. |
| Energy transaction failure matrix | **Source paths mapped; test-required** | COMBINE prepared-ledger zero-delta/complete rollback; construction reserve-transfer rejection semantics; maintenance rejection/debt; initial-source replay/duplicate claim; exact-once structural energy release. |
| Reaction reachability / post-death | **Architecture approved; coverage gap open** | Ordinary chemistry on dead parent aggregate; reachability/physical validation for stored or committed holders without counting topology-only connections as energy-bearing bonds; bounded multi-tick breach scenario with material/energy snapshots; no biological growth or death-time transfer. |
| Cache catalog/restart | **Test-required** | Chemical-position dependency changes require fingerprint bump; old manifest/cache rejection; current cache reopen; append/manifest failure rollback and restart. |
| World-tone receiver/oracle | **Path-level balance exists; global geometry gate open** | Single aggregate receiver mapping; stored capture exclusion; independent exact-small-world lane/path oracle; incident/captured/heat/escaped independent recomputation; invalid-frame no-partial-mutation. |
| Performance and long-run integrity | **Open release gate** | Reproducible bounded benchmark at increasing object counts up to thousands; reservoir capacity, overflow, drift, stale-key cleanup, telemetry semantics, multi-tick residual and cache-growth evidence. |
| Independent whole-world conservation | **Open release gate** | Integer material oracle includes field, attached aggregate, storage, active escrow, construction/accounting holders and any legacy carrier exactly once. Energy snapshot independently recomputes all current holders; cumulative source/sink provenance must reconcile across initialization, world tone, COMBINE/BREAK, movement, maintenance, death and long runs. |
| Exact-head verification | **Historical baseline-red result only** | Before first code change, refresh PR #205 head and workflow. For each small implementation unit, run focused gates under five minutes; compare named failures and normalized Clippy identities against frozen baseline; report actual exact-head evidence. |
| Documentation/handoff | **Maintained; verify after next chunk** | Keep this plan, current handoff and PR #204 description aligned with exact heads, new decisions, evidence, remaining gates and next action. Do not merge either PR or label chemistry unified until Section 17.6 is evidenced. |

**Next work sequence:** (1) close the aggregate/core-identity/diagnostic acceptance matrix and any remaining owner semantic gaps; (2) mark every failure row source-inspection-complete or test-required with its exact assertion contract; (3) confirm no uninspected production mutation path remains; (4) reconcile the independent inventories and release gates; (5) only then choose the smallest implementation unit, refresh PR #205 head/CI, and make one audited change at a time. No source/test changes before those steps.


## Aggregate graph, core identity, and diagnostics acceptance matrix — Section 17.93

- **Audit date:** 2026-10-22. **PR #205 exact head inspected:** `12d8079230458da68a8dc3755a1d6a9b41347862`. No source/test changes.
- `OrganismStructure` already has stable `PhysicalConstituentId` values, a `next_constituent_id` counter, and a `genome_constituent_ids` set. `add_unit` allocates from the graph-local counter; separate parent/construction graphs can therefore have overlapping ID ranges. Bonds reference constituent IDs, so a blind vector append is not a safe aggregate operation. A future merge must stage an injective incoming-ID mapping, rewrite every incoming bond endpoint, remap second-core anchor/boundary references, validate endpoints and graph invariants, and commit once; rejection must leave parent, storage, construction metadata, and inventories unchanged.
- `analyze_genome_cavity` currently searches connected components and selects the largest qualifying cavity without accepting a requested core identity. The parent’s active cavity/perception can therefore switch as the second core grows if the two attached cores share one graph. Use stable core references over shared physical IDs (parent boundary IDs and second-core anchor/boundary IDs), not a second graph or material-role tags. A core query must only analyze/select its own cavity boundary while preserving normal geometry-derived discovery; do not hard-code a shape, material recipe, or fixed cavity template.
- **Required graph/core acceptance tests:** (1) colliding local ID ranges merge with unique remapped IDs; (2) every bond endpoint maps to a real constituent and preserves endpoint location/energy; (3) malformed/dangling endpoints reject with byte-for-byte parent state/inventory unchanged; (4) parent core IDs remain stable; (5) second-core anchor and discovered boundary remap exactly once; (6) both cavities are independently discoverable in one connected aggregate; (7) parent active cavity does not switch as the second grows; (8) structural energy/material inventory is unchanged by pure attachment/remapping; (9) division partitions each unit, bond, storage item, core identity, and energy holder exactly once, with cross-partition bonds settled explicitly.
- `diagnostics.rs` currently reports `unit_count`, `bond_count`, `component_count`, `structural_mass`, and serialized `structure` from `organism.structure` only, while separately serializing the full `reproductive_construction` object, including the second physical graph. After unification, aggregate metrics and structure JSON must include attached units once; construction output must retain only nonphysical progress/core metadata and explicitly justified accounting sub-holders. Tests must assert no duplicate physical graph appears in the output, stored material remains separately represented, aggregate counts/mass match the canonical graph, and before/after diagnostic projections remain stable for a no-op tick.
- Current production source has no whole-world simulation persistence path in the audited runtime; this is a diagnostic/observation JSON compatibility task, not a requirement to invent save migration. If world persistence is introduced later, legacy graph migration/fail-closed handling becomes a prerequisite then.
- **Remaining semantic design gap:** the current temporary child gets separate maintenance, stress/death validation, overload BREAK, energy and debt state. D16 rejects a second physical graph and separate pre-division organism lifecycle, but does not by itself assign every maintenance/stress/debt/reserve term. Before implementation, define the smallest explicit accounting sub-holders needed by construction, whether maintenance is charged once against the parent aggregate, and how stress/death affects attached growth without deleting attached units or producing a child from dead construction. Do not preserve duplicate child lifecycle merely because its fields already exist.
- **Audit status:** source impact and acceptance assertions are now explicit; no aggregate implementation or diagnostic tests have been made. The semantic allocation contract and all failure-injection/release evidence remain open.
- **Next:** finalize the aggregate accounting semantics and reconcile the complete acceptance matrix against Sections 17.6/17.7. Then re-check whether any production mutation path remains uninspected. Do not change source/tests until all audit templates are complete; refresh PR #205 head and exact-head CI before the first implementation change.


## Construction lifecycle and parent-aggregate accounting audit — Section 17.94

- **Audit date:** 2026-10-22. **PR #205 exact source head inspected:** `12d8079230458da68a8dc3755a1d6a9b41347862`. **No source/test changes.**
- Re-read `src/organism/reproduction.rs`, `src/simulation/state.rs`, `src/simulation/simulation.rs`, and `src/infrastructure/diagnostics.rs` at that exact commit. The source confirms the owner-approved D16 model is not represented yet: `ReproductiveConstruction` stores a second `developing_structure` graph plus `developing_energy`, `developing_stress`, and `developing_maintenance_debt`; `developing_organism()` materializes these into a temporary full `Organism`; and `advance_construction()` runs maintenance, stress/death validation, and resonance-overload BREAK on that temporary organism before copying its state back. This is a second biological lifecycle, not merely nonphysical construction progress.
- `Simulation::tick` advances construction only for IDs in the sampled `living_ids` set, but handles `ConstructionStatus::Dead` in the same finish branch as `Ready` and `Detached`. `finish_reproduction()` can produce a child from a nonempty structure without an explicit ready-state precondition. Therefore construction-local death can create an offspring, while parent breach prevents the loop from advancing construction on subsequent ticks. These are separate from the desired rule that a dead parent retains its attached aggregate and ordinary chemistry continues.
- The current reserve transfer from parent usable energy to `developing_energy` ignores the boolean result of `EnergyLedger::transfer`, then proceeds to temporary-child maintenance and construction. This cannot be fixed as a stand-alone return check while the duplicate child lifecycle and physical graph still exist. The ledger transfer primitive itself is atomic; the caller's failure semantics are not.
- **Minimum proposed semantic contract for owner review:** (1) before division there is one organism and one physical graph; all attached construction/reproductive units participate in the parent's resonance and ordinary physical chemistry, while stored material remains excluded from resonance; (2) biological maintenance, stress accumulation/decay, and core-breach/death evaluation run once for the parent organism, not again on a temporary child; (3) parent death stops biological construction progress and forbids child creation, but does not delete attached structure, release/transfer ownership, or stop ordinary chemistry on that structure; (4) any construction reserve may survive only as a clearly justified nonphysical energy sub-holder with explicit source/transfer/release accounting, and may not create an independent stress/death lifecycle; (5) division alone partitions the aggregate, physical units/bonds, storage, core identities, and justified energy holders atomically and exactly once. This is a proposed contract derived from D16 and existing owner decisions, not yet a separately confirmed owner equation for reserve/debt.
- **Acceptance assertions added to the audit matrix:** parent maintenance is applied exactly once per tick; no temporary-child maintenance, stress decay, overload BREAK, or death validation occurs pre-split; rejected reserve transfer performs no dependent construction work and changes neither holder nor ledger; parent breach prevents growth/finish but preserves aggregate/material/energy inventories; no `Dead` construction status can create a child; attached units affect the parent's receiver map once and stored units never do; ordinary chemistry reaches valid bonds on the attached aggregate after breach; division moves each unit, bond, core reference, storage item, and energy sub-holder exactly once.
- **Audit classification:** source impact mapped; semantic proposal explicit; owner confirmation for reserve/debt representation remains open; implementation and regression tests remain gated on completion of every audit template. The previous plan/handoff checkpoint is no longer sufficient by itself because it described this allocation contract as unresolved without stating the narrowest proposed rule.
- **Next:** (1) finish the remaining mutation-path and failure-row inventory; (2) reconcile all audit-template statuses and Section 17.6/17.7 release gates; (3) refresh PR #205 head and CI immediately before any implementation; (4) only after audit closure, implement one smallest audited unit and test it. PR #204 remains documentation-only.


## Core identity and acceptance-test coverage audit — Section 17.95

- **Audit date:** 2026-10-22. **PR #205 exact source head inspected:** `12d8079230458da68a8dc3755a1d6a9b41347862`. No source/test changes.
- Re-read `src/geometry/cavity.rs`, `src/geometry/structure.rs`, `src/simulation/state.rs`, and `src/infrastructure/diagnostics.rs` at the exact recorded head. `analyze_genome_cavity` walks every connected component and chooses the qualifying cavity with greatest area. It has no core-reference parameter; `genome_cavity_cached_ref` then overwrites the graph's one `genome_constituent_ids` set with the boundary IDs of that selected cavity. That is adequate for the current single-core selection policy but cannot represent two independently stable core identities in one aggregate graph.
- `PhysicalConstituentGraph::add_unit` allocates from `next_constituent_id` starting at 1. Deserialization repairs duplicate/zero IDs within one graph, but that does not provide a merge operation for two valid graphs whose local IDs overlap. Since bond endpoints refer to physical IDs, aggregate attachment still requires an explicit old→new mapping and endpoint/core-reference rewrite before commit. Do not use vector concatenation or rely on deserialization as a merge algorithm.
- Coverage check: cavity tests cover the three-Carbon area reference, one realized qualifying cavity, and an unbonded non-genome. Structure tests cover distinct IDs within one graph, serialization round-trip of genome IDs, and bond identity. No reviewed test constructs two qualifying cavities in one connected aggregate, proves stable parent-core selection while another grows, tests collision-safe graph merge, or tests exact-once division. `diagnostics.rs` has no local test module in the inspected file; its snapshot takes counts/mass/structure JSON from `organism.structure` and separately serializes `reproductive_construction`, so no direct projection regression currently guards against duplicate physical graph reporting.
- **Acceptance matrix refined:** separate (a) graph merge/remap atomicity, (b) core-specific cavity discovery/cache identity, (c) division partition and energy/material invariants, and (d) diagnostic aggregate projection. Each is a distinct gate; passing the existing single-cavity and serialization tests is not evidence for any of these four gates. Test assertions listed in Section 17.93 remain required.
- **Audit classification:** implementation impact and missing coverage confirmed. This closes source inspection for the current core-identity/diagnostic acceptance row, but does not close its test gate or the overall audit template. The one-graph design still needs a concrete core-reference representation over stable physical IDs; do not introduce a reproductive-material tag or duplicate graph.
- **Next:** finish remaining mutation-path inventory and explicitly classify each matrix row as source-inspection-complete, test-required, owner-decision-blocked, or release-gate-open; then reconcile all rows with Sections 17.6/17.7. No source/test changes until all audit templates close.


## Owner decision reconciliation — attached structure resonance eligibility (Section 17.96, 2026-10-22)

**Decision status: SETTLED by the owner; this supersedes the unresolved-choice wording in Sections 17.37–17.38 and older handoff summaries.** Any physical structure that is structurally attached to the organism is ordinary organism structure and counts toward that organism's resonance response. There is no special reproductive-material designation and no separate developing-structure category in the target model. Material still held in storage is excluded from the organism's resonance capture. This includes committed reproductive structure and any construction units once attached to the organism's shared physical graph.

**Current implementation mismatch:** at PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`, the world-tone snapshot maps only primary-structure bodies to the living organism's resonance reservoir; separate developing/committed holders are not mapped to that reservoir, and stored material is also ineligible. The stored-material exclusion is correct. The primary-only limitation for structurally attached units is not the approved target behavior.

**Implementation contract:**
1. The canonical aggregate physical graph is the authority for attached surfaces. Each eligible attached surface can contribute compatible response to the owning living organism's single resonance reservoir exactly once.
2. Do not add a reservoir per attached unit, a reproductive-material tag, or a persistent developing physical graph. Construction/core progress is metadata over the shared graph, with only explicitly justified nonphysical accounting sub-holders.
3. Material in storage is not resonance-eligible, even though the organism remains its authoritative material owner. Storage objects can still participate in geometry/occlusion and ordinary chemistry where the physical rules allow.
4. A dead/breached organism has no living resonance receiver; its physical aggregate remains present and continues ordinary chemistry. Death does not transfer ownership or create a special decomposition process.
5. Active-transformation escrow and snapshot/trial representations must resolve to one authoritative physical source and one owner. Never double-count a surface, path response, or reservoir credit when an object is represented in escrow/prepared state.
6. Preserve first-hit occlusion, frequency-shifted onward propagation (not mirror reflection), staged reservoir commits, and incident = retained resonance + heat + escaped energy. Receiver-policy correctness does not substitute for the independent exact-small-world geometry oracle or global conservation proof.

**Audit consequence:** the owner-decision blocker is closed. The live receiver map still needs implementation changes after audit closure, plus tests for attached versus stored material, committed/attached units, escrow de-duplication, breached aggregates, and no-partial-mutation on invalid frame/ledger settlement. Sections 17.37–17.38 remain historical records of what the inspected code did and why clarification was requested; do not continue presenting the choice as unresolved.

**Next:** continue the remaining production mutation-path and failure-row audit; reconcile all audit statuses and release gates. PR #204 remains documentation-only. Refresh PR #205 head/CI immediately before any source/test change, and do not call chemistry unified until the Section 17.6 release gate has evidence.


## Prepared COMBINE ledger rollback audit — Section 17.97 (2026-10-22)

- **Exact source head inspected:** PR #205 `12d8079230458da68a8dc3755a1d6a9b41347862`. Documentation/source audit only; no source or test files changed.
- `EnergyLedger::settle_transaction` currently mutates only the holder, `total_potential_energy_released`, `total_structural_energy_delta`, `total_usable_energy_gained`, `total_work_expended`, and `total_heat_dissipated`. It does not mutate external-input, world-tone-throughput/escaped, or held-total fields. The live COMBINE candidate uses this transaction API.
- `prepare_transformation` computes a broad field-by-field ledger delta, then commits only a subset of fields; `rollback_prepared_combine` likewise reverses a subset and omits `total_world_tone_energy_escaped` and `total_resonant_energy_held`. The existing `failed_prepared_combine_rolls_back_energy_and_every_ledger_delta` test checks only selected fields and does not initialize/assert the omitted fields.
- **Classification: test-required, not a confirmed reachable production defect under the current API.** The current `settle_transaction` contract implies the omitted deltas should be exactly zero for COMBINE. Add an explicit invariant test proving every ledger field outside the transaction's mutation set remains unchanged across a prepared COMBINE and its rollback. If future COMBINE logic can legitimately change one of these fields, update commit and rollback symmetrically rather than silently widening the transaction.
- This is a focused acceptance gap in the existing COMBINE failure matrix; it does not close stored-BREAK partial commit, acquisition/expulsion rejection, movement rollback, construction reserve semantics, cache restart, post-death reachability, or world-tone/global conservation gates.
- **Next audit:** continue classifying each remaining failure row against exact source and existing test coverage. No implementation until audit templates close; PR #204 remains documentation-only, and PR #205 head/CI must be refreshed before any code change.


## Movement, acquisition, and expulsion failure-test coverage audit — Section 17.98 (2026-10-22)

- **Exact PR #205 source head inspected:** `12d8079230458da68a8dc3755a1d6a9b41347862`. No source/test changes.
- **Movement:** `advance_movement` calls the geometry/push mutation path before `EnergyLedger::settle_transaction`. If settlement rejects, the function returns an error but does not restore the mover, pushed organisms, physical field objects, cell/spatial indexes, or related revisions. Existing `failed_push_chain_is_atomic_for_all_affected_objects` covers a geometric push-chain rejection before energy settlement; it does not inject a ledger rejection after a successful move. A focused test can make the ledger reject deterministically (for example, a non-finite cumulative counter) and assert the entire pre-move snapshot and independent material/energy inventory are unchanged.
- **Contained acquisition:** existing `contained_physical_material_becomes_storage_without_an_acquire_action` covers the successful path. The source audit in Section 17.87 found that the field source is removed before the storage/fallback outcome is fully secured, and failed fallback deposit is ignored. No reviewed test injects storage rejection followed by fallback-deposit rejection while asserting exact field/storage membership and inventory.
- **Expulsion:** `src/chemistry/expulsion.rs` has no local test module in this head. The helper removes a stored object before the final environmental deposit; if the destination rejects it, the returned failure does not restore the original storage entry. Placement restoration and destination rejection need separate tests, including owner-relative/world placement, unchanged field revision/indexes on failure, and exact material inventory.
- **Classification:** all three source paths are inspected; the associated atomicity gates are **test-required**. These are not newly discovered code changes and do not authorize an isolated return-value patch: each test must exercise the complete transaction boundary and all affected holders.
- **Next:** finish exact test-coverage classification for stored BREAK and environmental COMBINE, then reconcile all matrix rows and release gates. No implementation until audit templates close; refresh PR #205 head/CI before any source/test change.


## Stored BREAK and environmental COMBINE failure-test coverage audit — Section 17.99 (2026-10-22)

- **Exact PR #205 source head inspected:** `12d8079230458da68a8dc3755a1d6a9b41347862`. No source/test changes.
- **Stored BREAK:** current tests prove the normal stored-BREAK path and test `restore_failed_transformation_material` via a malformed transformation with no stored bond. They do not exercise `resolve_transformation` with missing prepared state, malformed bond completion, or a later piece destination rejecting after an earlier piece has committed. The resolver currently clears the active marker and returns on several malformed-state branches, and split pieces are stored/deposited one at a time with failed destinations ignored. Required tests must prove escrow/source ownership survives malformed completion and every piece destination rejection, and that no partial split changes material, energy, stress, ledger, or field revision.
- **Environmental COMBINE:** `failed_live_combine_rolls_back_energy_and_ledger_when_union_is_unsupported` covers the live candidate path, while `failed_prepared_combine_rolls_back_energy_and_every_ledger_delta` unit-tests the rollback helper with synthetic deltas. Neither test proves the end-to-end staged environmental source is restored to the exact original cell when prepared physical state is malformed, nor that restoration is exactly once and does not duplicate a source if the cell/source changed. Add failure injection at the actual prepare/resolve boundary and assert exact source membership, cell revision/index state, structure/storage, ledger, and independent inventories.
- **Classification:** normal paths and some helper-level rollback are covered; malformed terminal states, partial multi-piece commit, and staged environmental-source restoration remain **test-required**. The audit does not establish that every branch is reachable in normal operation, but corrupt/stale prepared state must fail closed without losing or duplicating ownership.
- **Next:** reconcile Sections 17.86–17.99 into one authoritative failure matrix, explicitly separate source-inspection completion from test-required gates, then compare that matrix against the release checklist. No implementation until audit templates close; refresh PR #205 head/CI before any code/test change.


## Authoritative failure-matrix reconciliation — Section 17.100 (2026-10-22)

This matrix consolidates the latest bounded source/test audits in Sections 17.86–17.99. It supersedes older checkpoint wording where a source path was still described as unaudited, but does not claim a global proof that every production mutation path has been inspected.

| Area | Source audit | Current test evidence | Current classification / exit condition |
|---|---|---|---|
| Movement geometry + energy settlement | **Inspected** — geometry/push commits before ledger settlement | Push-chain geometric rejection covered; post-move ledger rejection not covered | **Test-required** — inject settlement rejection; prove mover, all pushed entities, field objects/revisions, indexes, progress, ledger and inventories unchanged |
| Contained-material acquisition | **Inspected** — source removal precedes fully secured storage/fallback outcome | Successful containment path covered | **Test-required** — inject storage and fallback-deposit rejection; preserve exact source and inventory |
| Voluntary expulsion | **Inspected** — stored object removed before final deposit; failed final insertion does not restore it | No local expulsion tests | **Test-required** — validate owner-relative placement and restoration/final-deposit rejection atomically |
| Stored BREAK | **Inspected** — escrow/active marker and per-piece destinations have malformed/partial-commit risks | Normal path and one failed-source restoration helper covered | **Test-required** — malformed resolver state, every split-piece destination rejection, stale/missing owner; no partial material/energy/stress/ledger mutation |
| Environmental COMBINE source | **Inspected** — exact-cell reservation/restoration path and prepared-state rollback mapped | Live unsupported-union and synthetic rollback helper tests exist | **Test-required** — malformed prepared state restores source to original cell exactly once, with unchanged indexes/revision and inventories |
| Prepared COMBINE ledger delta | **Inspected** — transaction API mutates a bounded field set; rollback omits fields expected to remain unchanged | Existing helper test does not assert all fields | **Test-required** — sentinel assertions prove all out-of-contract deltas are zero and untouched fields survive rollback; no reachable production defect established yet |
| Construction / D16 aggregate | **Inspected** — separate developing graph and temporary child lifecycle conflict with the approved model | Core identity, exact-once division and aggregate diagnostic tests absent | **Implementation/design gate open** — one graph and one pre-division lifecycle; explicit reserve/debt/maintenance semantics; stable core identity; dead construction cannot yield a child |
| Bondless reaction accumulation | **Inspected** — empty-bond early return bypasses stale activation-key cleanup | Targeted cleanup regression absent | **Test-required** — stale keys removed deterministically without changing physical/energy state |
| Chemistry cache/catalog | **Inspected** — manually maintained equation/catalog fingerprint is invalidation boundary | Schema/rollback coverage exists; chemical-position-change/restart case missing | **Test-required** — changing chemistry-position dependency invalidates old cache and restart rejects stale data |
| Post-death material/chemistry reachability | **Inspected** — structural bonds continue reacting; stored/committed/developing holders are not all enumerated by that pass | Independent inventory exists; bounded live-tick breached-parent scenario missing | **Implementation + test gate open** — dead parent retains aggregate ownership; ordinary chemistry can reach applicable physical holders; no biological progress, death-time transfer or legacy recycler |
| World-tone receiver mapping | **Inspected** — live path maps only primary structure; Section 17.96 settles attached-surface eligibility | Local path balance exists; independent exhaustive small-world oracle and fresh scale benchmark absent | **Implementation + release gate open** — shared attached graph maps each eligible surface once; storage excluded; escrow de-duplicated; exact-small-world and performance evidence |
| Graph/core identity and diagnostics | **Inspected** — graph-local ID collisions, global largest-cavity selection and duplicate diagnostic representation identified | Required aggregate tests absent | **Implementation + test gate open** — collision-safe remap, core-specific cavity identity, stable parent core, exact-once division, aggregate-only physical diagnostics |
| Independent whole-world conservation | **Partial evidence** — material/energy snapshots enumerate defined holders but do not yet reconcile every transition | Selected transfer/rollback oracles pass | **Release gate open** — source/sink provenance and integer material totals reconcile across initialization, reactions, movement, sunlight, maintenance, death and long runs |
| Exact-head CI / global mutation-path closure | Historical source head and workflow are recorded; no fresh run in this audit chunk | Historical workflow 39833438679 was baseline-red; no new workflow triggered | **Open** — do not claim global source inventory closed; refresh PR #205 head and CI before first code/test change and after each implementation unit |

**Decision reconciliation:** the attached-structure resonance question is closed by Section 17.96. It is no longer an owner-decision blocker. Reserve/debt/maintenance accounting still needs a concrete contract consistent with one aggregate and one biological lifecycle; do not recreate a separate developing organism to preserve old fields.

**Next work sequence:** (1) finish the remaining reserve/debt/maintenance accounting design without reopening D16; (2) complete a final call-site sweep for any production mutation/energy path absent from this matrix; (3) reconcile the independent inventory scope with every matrix transition; (4) refresh PR #205 head and exact-head CI; (5) only after the audit templates close, implement the smallest dependency-safe change with focused failure injection. PR #204 remains documentation-only.


## Construction reserve, maintenance, and stress allocation audit — Section 17.101 (2026-10-22)

- **Exact PR #205 source head inspected:** `12d8079230458da68a8dc3755a1d6a9b41347862`. No source/test changes.
- `Organism::apply_maintenance` computes demand from `self.structural_mass(catalog)`, charges one usable-energy holder, records paid maintenance as heat/stress, and accumulates debt/stress for unpaid demand. After attached structure is represented in the one canonical graph, this existing method naturally accounts for aggregate structural mass, provided the simulation calls it once per living parent per tick.
- `advance_construction` currently transfers parent usable energy into `developing_energy`, ignores the transfer result, then creates a temporary full `Organism` with its own structure, usable energy, stress, and maintenance debt. It applies a second maintenance transaction, independently validates stress/death, and can run resonance-overload BREAK on that temporary organism. This duplicates biological processing and can alter physical/energy state even when the intended reserve transfer is rejected.
- `EnergyLedger::transfer` is atomic for the two holders but does not settle an external source/sink; it is an internal ownership transfer. The source audit confirms no separate reason to count the same reserve as both parent usable energy and construction energy in the inventory.
- **Recommended minimum contract for owner confirmation (not yet treated as approved):**
  1. Run maintenance, stress accumulation/decay, and core-breach/death evaluation once on the parent organism. Once the graph is unified, maintenance demand is based on the whole attached aggregate. Remove the developing-core's independent stress and maintenance-debt lifecycle.
  2. Keep construction progress/core identity as nonphysical metadata; no persistent developing physical graph and no temporary full child organism before division.
  3. Preserve `reproductive_energy_allocation` only if its existing meaning requires an earmarked construction reserve. If retained, represent it as one explicitly named nonphysical sub-holder of the same parent aggregate; count it exactly once and define when construction may spend or release it. Do not give the reserve its own maintenance, stress, or death behavior.
  4. If reserve transfer fails, do not perform any work that assumes the transfer succeeded. The tick must leave both energy holders, physical aggregate, progress, and ledger unchanged for that attempted transaction; it must not mark the construction dead solely because energy was unavailable.
  5. Parent death stops biological construction progress and forbids offspring creation, while attached physical units remain under parent ownership and continue ordinary chemistry. Only a successful division transaction creates two child organisms and partitions all holders once.
- **Still unresolved:** whether the reserve should remain a distinct earmarked sub-holder and the exact rule for reserve spending/release. Do not silently pick a new genome-allocation equation. The minimum structural/lifecycle direction is supported by D16; the reserve's financial semantics need explicit owner approval or a narrowly justified derivation from existing contract text.
- **Required acceptance tests:** aggregate maintenance once per tick; no pre-split child maintenance/stress/death/overload; failed reserve transfer leaves exact state unchanged; no offspring from dead construction; parent-breach ordinary chemistry; reserve and all other holders reconcile exactly once through waiting, growth, death, and division.
- **Next:** use this contract to close the remaining construction semantic row, while keeping reserve semantics explicitly open. Then perform the final mutation/energy call-site sweep and reconcile the independent inventory gates. No implementation until the audit templates close.


## Bounded energy/material call-site sweep — Section 17.102 (2026-10-22)

- **Scope:** exact PR #205 head `12d8079230458da68a8dc3755a1d6a9b41347862`; reviewed the high-risk chemistry, material-transfer/restoration, energy-ledger, movement, simulation, reproduction, and world-tone modules. This is a bounded sweep, not a claim that every Rust file or every mutation path has been exhaustively proven.
- Internal `EnergyLedger::transfer` callers in the inspected production paths remain the construction-reserve transfer and the quarantined legacy recycling path, consistent with Sections 17.56/17.84. The reserve transfer ignores failure; legacy recycling drains/deposits material without a complete rollback contract and must not be reactivated as the death pathway.
- Active energy settlement paths are accounted for in the matrix: COMBINE formation, transformation BREAK/COMBINE, maintenance, movement, world-tone frame/input, and the quarantined legacy recycler. Their main unresolved risks are already classified in Sections 17.86–17.101: geometry-before-ledger commit, partial material destinations, stale/malformed escrow, construction reserve semantics, receiver mapping, and incomplete global conservation evidence.
- The sweep also found ignored deposit returns in deterministic initial-landscape/resource-cloud seeding. These calls operate on already-realized material and a previously validated field index; they are initialization/seeding paths rather than live chemistry transactions, and no new energy source/sink path was found there. Keep exact seed-count behavior separate from chemistry conservation.
- **Result:** no additional untracked live energy-transfer API caller was identified in this bounded high-risk sweep. This does not close the global source-path audit, nor does it make the known rollback paths safe.
- **Next:** finish the construction reserve semantic contract, then perform the final inventory/source-sink reconciliation. Refresh PR #205 head and exact-head CI before any source/test change. PR #204 remains documentation-only; no code or tests changed.


## Owner decision — all production costs use parent usable energy (Section 17.103, 2026-10-22)

**Decision status: SETTLED by the owner.** All production/construction costs draw directly from the parent's ordinary `usable_energy`. Remove `reproductive_energy_allocation` as a separate earmarked reserve and do not transfer energy into a separate `developing_energy` holder. There is no separate construction-energy pool and no production-specific energy budget. The ordinary energy ledger remains authoritative; each production cost is charged exactly once to the parent under the same transaction/accounting rules as other parent costs.

Consequences:
1. Construction progress and core identity remain metadata over the parent's one canonical physical aggregate. Do not instantiate a temporary full child organism to run construction maintenance, stress, debt, resonance, overload, or death.
2. Parent maintenance is applied once per tick and is based on aggregate structural mass after attached structure is unified. Stress and core-breach/death are parent lifecycle behavior, not a second developing lifecycle.
3. Before any production mutation, validate and settle the parent energy debit atomically with the corresponding construction/material mutation. If the parent cannot afford the cost or settlement rejects, do not mutate structure, progress, stress, ledger, or ownership. Do not continue after a rejected debit.
4. No reserve release/transfer path is needed. On parent death, construction progress stops and no offspring is created; attached physical material remains owned by the dead parent aggregate and continues ordinary chemistry. Only successful division creates child organisms and partitions the aggregate and all energy/material holders exactly once.
5. This decision does not authorize inventing or changing the production cost equation, energy price, or genome-allocation rule. Preserve existing documented cost formulas, but route their costs directly through the parent's usable-energy holder. If the source does not define a cost unambiguously, record that exact formula as an open question rather than creating a reserve to hide it.

**Implementation mismatch at audited PR #205 head** `12d8079230458da68a8dc3755a1d6a9b41347862`: construction transfers energy into `developing_energy`, ignores transfer rejection, then executes an independent temporary-organism lifecycle. These are now explicit migration targets, not unresolved design choices.

**Acceptance gates:** (a) no `reproductive_energy_allocation`/separate `developing_energy` reserve in the target model; (b) production debit and structural change commit or reject together; (c) insufficient-energy/rejected-ledger paths leave exact pre-operation state unchanged; (d) aggregate maintenance/stress/death occurs once; (e) dead parent cannot produce offspring; (f) division partitions all physical/material/energy ownership exactly once; (g) independent integer material and energy inventories reconcile.

**Next:** continue source/test audit of production cost formulas and division transaction boundaries; identify existing tests and exact missing failure-injection cases. Then reconcile these gates with the master contract and run a fresh PR #205 head/CI audit before source edits. PR #204 remains documentation-only.


## Production-cost audit and clarification of the parent-energy decision — Section 17.104 (2026-10-22)

**Clarification superseding any overbroad wording in Section 17.103:** the owner's decision is that every production cost is debited directly from the parent's ordinary `usable_energy`; it does not by itself decide whether the genome field `reproductive_energy_allocation` remains as a cost parameter, an offspring-energy allocation rule, or is retired. What is prohibited is using it to create a separate energy holder/reserve before division. Do not remove the genome field, its serde alias/default, or its compatibility behavior until its semantic role is established against the master contract and callers. No separate `developing_energy` holder is allowed in the target model.

**Exact-head source findings (PR #205 `12d8079230458da68a8dc3755a1d6a9b41347862`):**
1. `begin_reproduction` rejects genomes when `reproductive_energy_allocation` is non-finite or non-positive. It otherwise removes a physical anchor from parent storage and creates `ReproductiveConstruction` with a separate graph, `developing_energy = 0`, stress and maintenance debt.
2. `advance_construction` uses the genome allocation as a reserve target, transfers parent energy into `developing_energy`, ignores the transfer result, and applies a second temporary-child maintenance/stress/death/overload lifecycle. This violates the owner-approved parent-only debit/lifecycle model.
3. Each candidate construction attachment goes through the existing bond-driven COMBINE/ledger candidate path. This path already calculates work/heat/structural energy deltas; the migration must ensure those costs are charged against the parent's `usable_energy` and commit atomically with attaching the unit to the shared aggregate. The audited excerpt does not establish a separate authoritative scalar "per-piece production cost" formula, so do not invent one.
4. Initial organism setup currently computes `construction_cost = max(1.0e12 - construction.energy, 0)` and `initial_energy = construction_cost + genome.reproductive_energy_allocation`. The `1.0e12` term is a constructor trial allowance/cost-difference technique, not a physical source. This initialization formula is separate from the runtime production debit and requires reconciliation with PR #196/Master Spec 2.4; do not fold it into runtime production cost or modify constructor code under this chemistry plan.
5. `finish_reproduction` currently takes construction state and creates a child with `usable_energy = construction.developing_energy`; the simulation treats `Ready`, `Detached`, and `Dead` as finish candidates, and the finish helper can return a child for nonempty structure without a strict Ready-only precondition. The target must allow only a validated successful division to create a child, and must partition remaining parent energy/material/structure exactly once. The amount/rule for the child's post-division usable energy is **not specified by the owner decision alone**; preserve an existing master-contract rule if one exists, otherwise leave that narrow division-allocation question explicitly open rather than inventing a 50/50 split or treating production costs as a reserve.
6. Existing tests prove the separate child graph and persistent parent-to-developing energy transfer. They must be replaced, not preserved as target behavior. Coverage is missing for rejected parent debit with no mutation, aggregate attachment atomicity, parent death preventing offspring, non-Ready terminal states never creating a child, and exact-once division of every unit, bond, storage item and energy holder.

**Classification:** production mutation/cost path source-inspected; current behavior conflicts with approved parent-energy model. Candidate COMBINE provides a cost-accounting primitive but the unified aggregate transaction is not implemented. Initial constructor budget remains out of scope. Division energy allotment is the only narrow owner-contract ambiguity surfaced by this pass; the separate reserve model itself is not ambiguous and is rejected.

**Next:** inspect Master Spec 2.4 and existing reproduction/division rules for a defined post-division energy-allocation rule; then finish division rollback/test coverage and reconcile the global source/sink inventory. No source/test changes until audit closure. PR #204 remains documentation-only.


## Division transaction boundary and Master Spec reconciliation — Section 17.105 (2026-10-22)

- **Authority check:** repository README identifies Master Technical Specification v2.4 as design authority, but the exact Master Spec file is not present in the audited PR #205 repository tree. README's older reproduction prose describes a separate developing graph and an inherited energy allocation; that prose conflicts with the owner's later explicit decisions for one canonical attached aggregate and parent-funded production costs. Treat the owner decisions as controlling and flag README reproduction prose for a later documentation reconciliation, not as permission to restore the old graph/reserve model.
- **Physical target:** the established 80/80 goal remains approximately 160% preferred aggregate scale before a successful division into two approximately 80% organisms. Before division there is one organism, one aggregate graph, one parent usable-energy holder, and nonphysical progress/core metadata. The attached second core is not yet an independent organism. Existing README explicitly says full 160%-then-split implementation is incomplete; this audit does not claim it is implemented.
- **Current terminal-state defect:** simulation sends `Ready`, `Detached`, and `Dead` construction statuses into `finish_reproduction`. The helper takes the construction state and can return a child when the developing structure is nonempty even if readiness is false. This violates the settled rule that parent death stops construction and no child may be produced from a dead/failed construction. Only a fully validated division transaction may create the two resulting organisms.
- **Current energy boundary:** `finish_reproduction` assigns the child `usable_energy` from `construction.developing_energy`. That holder is rejected by the owner-approved model. All production costs must already have been charged against parent `usable_energy`; at division, remaining energy must be partitioned exactly once under the 80/80 division contract. The owner decision does not yet specify a numeric post-split allocation. Do not introduce a reserve, a 50/50 split, or a new formula. Search for an existing authoritative division rule before marking this narrow field settled.
- **Atomic division acceptance contract:**
  1. Require the canonical aggregate to meet the approved division condition and validate both resulting organisms before mutation.
  2. Partition every physical unit, bond, core reference, stored item, and remaining energy holder exactly once. Remap graph-local IDs and bond endpoints; preserve stable core identity.
  3. Settle any cross-partition bond energy/work/heat explicitly; never clone or drop bond energy.
  4. Charge no production costs to a child-side reserve. Any production cost must debit the parent's ordinary usable energy and commit atomically with its physical mutation.
  5. If any child validation, placement, ledger settlement, ID allocation, or destination insertion fails, preserve the exact intact parent aggregate and all inventories; retry must not duplicate a unit, bond, or energy.
  6. A dead/breached parent cannot advance or finish construction. It keeps ownership of the attached aggregate; ordinary chemistry may continue, but biological division does not.
- **Existing coverage vs missing coverage:** existing tests intentionally assert a separate child graph and parent-to-developing energy transfer, so those assertions describe the old model and must be replaced. The inspected test module lacks exact-once aggregate division coverage, collision-safe graph-ID remapping, cross-partition bond settlement, rollback after either child's validation fails, and proof that non-Ready terminal states never create children.
- **Classification:** master-contract repository copy unavailable; owner decision closes the reserve-holder choice; post-division remaining-energy partition formula remains a narrow open item only if no existing contract can be found. Division atomicity and tests remain implementation/release gates. No source/test changes.
- **Next:** search the current README/history for the established child energy allocation meaning; if still undefined, ask the owner one narrow question before implementing division. Then reconcile all open semantic and release rows and refresh exact-head PR #205 CI before the first source change.


## Child energy at division — existing-contract search result (Section 17.106, 2026-10-22)

- Checked the current README growth/reproduction section, the chemistry contract, handoff archive, current reproduction implementation, and repository search results for the genome allocation field/division semantics.
- The README says `reproductive_energy_allocation` is an inherited energy allocation, and the field retains a backward-compatible `juvenile_energy_reserve` serialization alias; it does **not** specify a post-division child-energy formula or an atomic parent/child usable-energy partition. The exact Master Spec v2.4 file is not in the repository tree searched. No matching commit/history result established a numeric split rule.
- Therefore preserve the genome field/alias until the owner decides its meaning under the revised model. Do not treat it as a pre-division holder, do not fund production costs from it, and do not delete it as part of this migration without a compatibility decision.
- The only unresolved owner choice from this audit is: after a successful 80/80 division, should the child receive (A) a fixed amount equal to the genome's `reproductive_energy_allocation`, debited from the parent's remaining `usable_energy` at division, or (B) a proportional share of the parent's remaining `usable_energy` under a separate explicit split rule? Neither formula is currently established in the inspected contract. This choice affects only post-division energy partition; every production cost remains charged directly to the parent's ordinary `usable_energy`.
- **Stop gate:** do not implement division-energy partition until this one choice is settled. Other audit work may continue without changing source.


## Post-division energy rule — owner choice B (Section 17.107, 2026-10-22)

- **Owner decision:** after a successful division, split the parent's remaining ordinary `usable_energy` proportionally between the two resulting organisms. Production costs remain charged directly to the parent's `usable_energy` as each construction mutation commits; this is not a production reserve and must not revive `developing_energy`.
- The choice settles fixed-allocation option A versus proportional option B. It does **not yet identify the proportional denominator**. The audited repository and README establish the approximate 160%-aggregate to two ~80%-organism physical goal, but do not define whether the energy ratio follows each resulting organism's structural mass, usable-energy demand, or another explicit division invariant. Do not silently pick a denominator or implement an arbitrary 50/50 split.
- Candidate rule for owner confirmation: distribute remaining usable energy in proportion to each validated child's post-split structural mass, so the energy allocation follows the actual physical partition. If the two resulting structures are equal in mass, this naturally produces an equal split; unequal validated partitions receive unequal shares. This candidate is not yet an approved rule.
- Division remains an all-or-nothing transaction: validate both resulting organisms and the partition before mutation; allocate all remaining usable energy exactly once; preserve energy conservation and exact rollback if any validation/placement/ledger/ID/insertion step fails. No child is created from a dead parent or non-Ready construction.
- **Implementation gate:** do not code the post-division energy partition until the proportional basis is explicit. No source/test changes in this section.
