# Chemistry Audit and Unification Plan

**Status:** Read-only audit register A0–A21 populated; unresolved gaps and implementation dependencies recorded. No production behavior changes have been made.  
**Baseline branch:** `main`  
**Baseline commit:** `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5`  
**Plan opened:** 2026-10-22  
**Scope:** Read-only audits A0–A21 are recorded; this does not mean the chemistry system is unified or conservation is proven. PR #204 is a durable planning/handoff artifact only: no production code or tests may change here. The current canonical owner decisions are in Section 14 and supersede earlier unresolved-question lists and provisional recommendations. A0 records exact-head counts, but the individual failing-test identities were not independently extracted from workflow logs in this pass.

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
- **No silent baseline rewriting.** Exact-head `main` workflow/commit evidence records 563 passed, 28 failed, and 1 ignored at `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5`; the commit message reports that all 28 full-suite failures match the accepted baseline. The workflow check itself concluded failure because the suite has known failures. The detailed failing-test identity list was not independently extracted from the run logs in this pass, so retain that evidence limitation rather than overstating direct verification. Do not weaken physical invariants to make CI green.
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

**Status: Gap found (exact-head baseline counts verified; detailed failure identities only commit-attested).**

- **Exact baseline:** main resolved to merge commit cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5 during this audit. PR #203 is in fact **merged** (merge commit above; PR #203 head 6104cdf952c06cc3184e32abc2b116c9512935fa), contradicting README.md, which still calls it an unmerged draft. README's maintenance/test/Clippy claims describe older commit 67bceda1672891e7e390cf242295477bfb5cf3a0 or PR #202 source, not this current merge commit.
- **CI evidence:** GitHub Actions workflow `Rust`, run `39705628427`, is attached to exact `main` SHA `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5` and completed at 2026-10-22 01:30:37Z with conclusion `failure`. Its `test` check (`119584875250`) completed at 01:30:36Z and reports four annotations. The exact-head commit message records **563 passed, 28 failed, 1 ignored**, states all 28 full-suite failures match the accepted baseline, says focused Bob/fluid/chemistry checks passed, and records strict Clippy at 34 application-target plus 15 test-target errors. Thus the count is evidenced for this exact `main` SHA; the individual failure names/assertions were not independently extracted from run logs during this pass. Do not mislabel the suite as green or present the recorded Clippy errors as chemistry regressions.
- **Scope map established:** the crate is organized through src/main.rs path attributes. Chemistry-relevant active/staged modules include src/chemistry/{chemistry,chemical_reaction,combine,combine_runtime,energy_ledger,decomposition,transformation}.rs, src/materials/resources.rs, src/perception/harmonics.rs, src/simulation/{state,world_tone,world_tone_snapshot,simulation}.rs, plus physical-material, storage/transfer, geometry/contact, movement, maintenance, reproduction, persistence, diagnostics, and UI paths. Cargo.toml identifies a single Rust 2021 package; no separate library target was declared there.
- **Historical audit lead, resolved by later A8/Section 15 inspection:** README.md said live world-tone capture remained disabled, while `src/simulation/world_tone_snapshot.rs` showed a live settlement path. The later source review found the path present; correctness and performance remain open, not whether the path exists.
- **Baseline failure identities:** exact-head evidence exists and the commit message attests that the same 28 failures match the accepted baseline. The individual failing-test list was not independently retrieved in this pass, so A0 remains a gap for independent identity-level verification, not for the existence/count of the exact-head workflow result.
- **Out of scope confirmed:** PR #196 remains separate; this audit changes no production code and does not authorize chemistry behavior changes.
- **Evidence:** README.md; PR #203 metadata/merge commit; Cargo.toml; src/main.rs; src/simulation/world_tone_snapshot.rs; current-head status/run queries (empty).
- **Dependencies / downstream audits affected:** every audit must cite the main SHA it actually inspected. A0 must be revisited after a current-head CI/test comparison becomes available. A8 must resolve the world-tone enablement discrepancy.
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

| Audit | Status | Evidence / output | Open blockers |
|---|---|---|---|
| A0 Baseline and inventory | Gap found; exact-head counts verified, named failures only commit-attested | main cbe3c5d; run 39705628427: 563 passed, 28 failed, 1 ignored; commit states all 28 match accepted baseline | Independently extract named failures if available; keep the attestation limitation explicit |
| A1 Resource vocabulary/properties | Gap found; identity choice settled by D1 | Oxygen is canonical; stale Methane references, unknown-name zero fallbacks, and composite chemical-position behavior still need tracing | Trace remaining references and enforce catalog/constituent invariants; do not reopen resource identity |
| A2 Core chemistry math/units | Gap found | formulas located; implicit units, accumulator overflow, activation/settlement alignment remain to prove | Unit contract and adversarial tests after all audits |
| A3 COMBINE and bond storage | Gap found | Trial structure + balanced ledger transaction; structure clone per attempt; fluid transient special case | Caller matrix, fluid topology and clone-cost tests after audits |
| A4 BREAK paths | Gap found | Shared realized-bond settle/commit; stored-material BREAK is zero-energy; recycling/decomposition paths identified | Stale/cancelled transformation and lifecycle material/energy tests after audits |
| A5 Ledger transaction algebra | Gap found | Local balance equation verified; hand-maintained prepared ledger delta/rollback; world snapshot use unproven | Centralize reversible deltas and prove live inventory in A6 after audits |
| A6 Whole-world energy inventory | Gap found | Snapshot is test-only; no production global balance gate; material-internal bond energy absent | Independent inventory and end-of-tick reconciliation after audits |
| A7 Resonance and reaction coupling | Gap found | Four-component cap; no explicit energy magnitude bound or general reservoir dissipation policy; epsilon pruning | Deposit/withdraw/serialization/spectrum energy invariants after audits |
| A8 World tone and capture | Gap found; runtime path observed | Live capture/settlement path exists; complete source budget, approximation error, occlusion correctness, and scaling remain unproven | Validate live path and compare the D5 approximation against an exact small-world reference; do not describe capture as disabled |
| A9 Work, heat, dissipation | Gap found | Movement/maintenance are heat-accounted; move precedes debit; stress and heat semantics need definition | Canonical sink/work semantics and failure-injection tests after audits |
| A10 Stress/death/core breach | Gap found | Death is observational; breached bodies persist and chemistry continues; same-tick gate boundary remains | Tick-order and pending-transaction tests after audits |
| A11 Post-death chemistry/material fate | Gap found | Live path is ordinary chemistry; legacy decomposition is test-only; recycling helper has no production caller found | Keep one live pathway; test/harden or quarantine legacy helper after audits |
| A12 Energy consumers/transfers | Gap found; initial-source decision settled by D2 | Initial energy is currently an implicit baseline; ignored transfer results and trial-energy allowances need explicit accounting; reproduction anchor transfer is staged separately | Implement one-time source transaction; surface transfer failures; prove deterministic trial/actual construction equivalence |
| A13 Persistence/schema/recovery | Gap found; D6 policy settled | No durable whole-simulation save/load path found in the audited source; runtime history is in-memory clones. `ChemistryLibrary` is durable and accepts schema 2 or 3; pending-operation fields have serde defaults. | Reject pre-unification chemistry caches; define a mandatory version boundary before any future durable simulation-save format; test malformed pending state and restart/corruption paths |
| A14 Chemistry cache semantics | Gap found | Persistent `ChemistryKey` uses schema version 3; loader still accepts schema 2 keys/manifests. No chemistry equation/parameter fingerprint in the key was evidenced. | Add equation/parameter identity; reject pre-unification schema versions; prove cache-hit/recompute equivalence and benchmark cold/hot lookups |
| A15 Tick ordering/determinism | Gap found | Pending transformations retain staged state and have restoration paths, but the full tick phase order and all stale/cancelled-operation rollback paths are not proven | Document authoritative phase order; test duration boundaries, save/load around commit, stale operations, and failure injection |
| A16 Material conservation | Gap found; D8 rule settled | `Material::is_valid` permits any positive finite `f64` amount and does not validate catalog membership; `Material::take` and aggregate transfer helpers can produce fractional parts. Physical storage enforces discreteness at a later boundary. | Enforce integer base-resource counts at authoritative ownership boundaries; separate fluid geometry/volume from atom count; build an independent whole-world constituent inventory oracle |
| A17 Diagnostics/observability | Gap found | Recorder is staged/opt-in; inventory snapshot is test-oriented; diagnostic header hard-codes seed 42 and current-vs-cumulative values need clear labels | Report actual run configuration; isolate I/O failures; document units and inventory residual semantics; bound output volume |
| A18 Numerical stability | Gap found | Reaction accumulation can overflow from extreme prior state; reservoir epsilon pruning and cumulative counter overflow lack a defined long-run fate/tolerance budget | Add adversarial overflow and drift tests; define numerical truncation and counter-overflow policy before shared math changes |
| A19 Scalability/profiling | Gap found | World-tone geometry/exposure is a likely hotspot; no representative runtime/memory envelope or cache-growth measurements were found | Benchmark sparse/dense worlds and cache cold/hot paths; compare first-hit approximation to a trusted oracle with explicit error bounds |
| A20 Cross-domain integration | Gap found | Local unit tests exist, but end-to-end energy/material reconciliation and cross-domain failure-injection coverage are unproven; exact-head main reports 563 passed/28 failed/1 ignored, while detailed failure identities are only commit-attested | Build independent inventory oracles and integration matrix covering full ticks, lifecycle, persistence, stale events, and failed commits |
| A21 Documentation/sequence | Gap found; design decisions reconciled | README status/capture claims are stale; historical Methane decision text is superseded by D1; exact-head named baseline failure identities remain unextracted | Use Section 16 for final call-path findings and Section 17 for the authoritative implementation sequence; preserve baseline evidence limitation |

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
- **Actual runtime call path:** the README currently states PR #203 is a draft/not merged and cites the older PR #202 source baseline, while the audited `main` SHA is `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5` and PR #203 is merged there. README also says live world-tone capture is disabled, while current `src/simulation/simulation.rs` and `src/simulation/world_tone_snapshot.rs` show configured source sampling and a live capture settlement call. The default resource catalog names Oxygen where the approved resource vocabulary names Methane; the handoff archive records the historical resource rename as Methane to Oxygen. These conflicts require a deliberate, source-backed status update and resource identity decision.
- **State mutated and transaction boundary:** this audit changed only the audit plan document. No README, source, test, library data, workflow, or runtime behavior was changed. The current PR remains a documentation/planning vehicle; its commit history contains only updates to the audit plan.
- **Energy/material source and sink:** this historical audit correctly identified unproven source/holder/sink coverage, but D2, D3, D4, and D5 now settle the relevant design direction for initial energy, stored bond energy, tolerance versus dissipation, and sunlight approximation. Implementation, independent inventory, source allocation, and full material ownership reconciliation remain open.
- **Failure/rollback behavior:** not applicable to production behavior in this documentation-only audit. Documentation updates should be evidence-gated: do not claim CI success, a clean baseline, or disabled capture unless exact-head workflows and the live call path support the statement.
- **Numerical edge cases and invalid-state behavior:** the project spec/resource identity mismatch affects persisted material names, geometry records, chemistry cache keys, tests and migrations. It must not be repaired by a global string replacement without catalog/ID/key migration coverage.
- **Complexity and scaling implications:** proposed order is: (1) close the remaining baseline-evidence detail or accept its limitation; (2) decide canonical resource identity and persistence/cache migration; (3) define units, energy inventory/source boundaries, material-holder inventory, and internal-bond policy; (4) specify tick order and transaction atomicity; (5) build independent energy/material oracle tests and failure injection before changing behavior; (6) repair numerical invalid-state paths; (7) profile the current world-tone frame and compare candidate first-hit approximations; (8) implement shared transaction/accounting primitives in small, dependency-ordered changes; (9) update README only after the relevant evidence passes. Constructor/Bob PR #196 remains separate and deferred.
- **Tests already covering it:** the repository has local unit/contract tests and historical CI runs, but A0 could not verify current-head workflow/status results for `cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5`; the README's old PR #202 counts cannot be represented as current-main results.
- **Focused tests to add or run:** (1) independently extract the exact-head failure list if workflow logs/annotations are accessible; (2) verify resource IDs/names across catalog, serialization fixtures, chemistry library and Bob library; (3) world-tone live-call-path and source-budget integration test; (4) full energy/material inventory matrix from A20; (5) equation-version cache invalidation; (6) persistence/migration tests for any resource or ledger schema change; (7) measured target-population benchmark and approximation error report.
- **Findings ranked by severity:** **P0/P1** — root README has material status contradictions about PR #203 merge state and live world-tone capture; correct the status only after source-backed reconciliation. **P1** — resource identity differs from the approved resource vocabulary and affects persistence/cache compatibility. **P1** — individual baseline failure identities were not independently extracted, although exact-head counts and the commit's baseline-match statement are available. **P1** — world energy/material conservation and live source allocation remain unproven. **P2** — implementation sequence needs to enforce dependency order and keep PR #196 separate.
- **Dependencies / downstream audits affected:** all A0–A20. This is the sequencing gate; it consolidates but does not resolve their findings.
- **Decision required from project owner, if any (historical wording):** this record predates Section 14. D1–D15 now settle the listed design choices; remaining approval is for the implementation sequence, while acceptance thresholds must be grounded in measurement.
- **Recommended smallest next change (if authorized):** the audit template is populated through A21 and exact-head baseline evidence is recorded. Next, present the dependency-ordered implementation plan for owner approval. Do not begin production code until approval. No production code change authorized now.

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

**This is a proposal, not authorization to modify production code.** The owner has already selected D1–D15 in Section 14; do not treat those decisions as open. No production change begins until the owner explicitly approves the implementation sequence. Apply one focused pre-change audit before each implementation change; keep each command/test bounded to five minutes and compare failures against the exact `main` baseline recorded in A0.

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

The planned audit register A0–A21 has been populated. **Completion means the planned domains have been inspected and their gaps recorded; it does not mean chemistry is unified or conservation is proven.** Remaining implementation/evidence gaps stay marked as gaps or blocked. Owner decisions D1–D15 are settled design direction and must not be listed as pending decisions. A0's exact-head main run and recorded counts are documented; independent extraction of each failing-test identity remains a smaller evidence limitation. No production code, tests, README status, resource catalog, or persistent library data has been changed in this workstream.

Before the first production change, require:
1. Exact-head baseline evidence or an explicit owner-accepted limitation for A0; preserve the limitation that named failure identities were not independently extracted in this pass.
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
- **Next resume point:** review Section 17's formal plan against D1–D15 and the gaps in A0–A21. Implementation is still blocked until the owner explicitly approves the sequence; when implementation begins, perform one focused audit before each change, preserve PR #196 constructor/Bob work as separate, and compare bounded verification to the exact-head baseline.

**Handoff-prep status:** the three remaining source-path audits are now documented with files/functions, behavior, state ownership, tests already present, unresolved gates, priorities, and dependencies. The remaining unknowns are explicitly technical/evidentiary gaps rather than missing owner decisions.
