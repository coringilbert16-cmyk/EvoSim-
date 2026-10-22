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
**Audit record — 2026-10-22**

**Status: Gap found (baseline partially verified; fresh CI/test identity comparison blocked).**

- **Exact baseline:** main resolved to merge commit cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5 during this audit. PR #203 is in fact **merged** (merge commit above; PR #203 head 6104cdf952c06cc3184e32abc2b116c9512935fa), contradicting README.md, which still calls it an unmerged draft. README's maintenance/test/Clippy claims describe older commit 67bceda1672891e7e390cf242295477bfb5cf3a0 or PR #202 source, not this current merge commit.
- **CI evidence:** queried combined commit status and PR-triggered workflow runs for cbe3c5dad9ec9a302a9596fe17ca957d6f6b0ab5; both returned empty results. This does **not** establish that no CI exists; it means current-head test results and the identities of the accepted 28 failures could not be verified from these connector endpoints. Do not represent the old 563/28/1 count or old Clippy counts as proven current-head results.
- **Scope map established:** the crate is organized through src/main.rs path attributes. Chemistry-relevant active/staged modules include src/chemistry/{chemistry,chemical_reaction,combine,combine_runtime,energy_ledger,decomposition,transformation}.rs, src/materials/resources.rs, src/perception/harmonics.rs, src/simulation/{state,world_tone,world_tone_snapshot,simulation}.rs, plus physical-material, storage/transfer, geometry/contact, movement, maintenance, reproduction, persistence, diagnostics, and UI paths. Cargo.toml identifies a single Rust 2021 package; no separate library target was declared there.
- **Important live-path conflict for later audit:** README.md says live world-tone capture remains disabled, but src/simulation/world_tone_snapshot.rs describes the live tick settling direct sunlight into an organism energy holder. A8 must determine the actual call path and gate; README alone is insufficient.
- **Baseline failure identities:** **unverified**. The old run links may be useful historical evidence, but they cannot certify the current tree. A0 cannot be fully closed until exact-head CI/test evidence is obtained or the limitation is explicitly accepted.
- **Out of scope confirmed:** PR #196 remains separate; this audit changes no production code and does not authorize chemistry behavior changes.
- **Evidence:** README.md; PR #203 metadata/merge commit; Cargo.toml; src/main.rs; src/simulation/world_tone_snapshot.rs; current-head status/run queries (empty).
- **Dependencies / downstream audits affected:** every audit must cite the main SHA it actually inspected. A0 must be revisited after a current-head CI/test comparison becomes available. A8 must resolve the world-tone enablement discrepancy.
- **Decision required from project owner, if any:** none to continue read-only auditing. Do not update README's claimed baseline until exact-head evidence is found.
- **Recommended smallest next change (if authorized):** after all audits, update the README's status block from verified merge and exact-head evidence; no change authorized now.

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
- **Decision required from project owner, if any:** none to continue audits. Whether internal bonds in stored physical materials should ever carry structural energy is a future model decision, not something to assume.
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
- Live capture stays disabled until all capture allocation, duplicate prevention, and conservation gates are met.

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
8. **P2 — approved approximation still unresolved:** the user's proposed radar-like single sweep/first hit may be a practical speed/accuracy trade-off, but the present algorithm is substantially richer. Compare energy allocation and shadow errors against an exact or trusted small-world reference before choosing a scalable approximation. No approximation is authorized by this audit.

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
| A0 Baseline and inventory | Gap found; CI identity comparison blocked | main cbe3c5d; README stale re PR #203 and exact-head evidence | Fix baseline evidence / revisit after CI retrieval |
| A1 Resource vocabulary/properties | Gap found | default_catalog contains Oxygen, not approved Methane; unknown-name zero fallbacks; composites have no chemical_position | Canonical resource identity and migration policy after all audits |
| A2 Core chemistry math/units | Gap found | formulas located; implicit units, accumulator overflow, activation/settlement alignment remain to prove | Unit contract and adversarial tests after all audits |
| A3 COMBINE and bond storage | Gap found | Trial structure + balanced ledger transaction; structure clone per attempt; fluid transient special case | Caller matrix, fluid topology and clone-cost tests after audits |
| A4 BREAK paths | Gap found | Shared realized-bond settle/commit; stored-material BREAK is zero-energy; recycling/decomposition paths identified | Stale/cancelled transformation and lifecycle material/energy tests after audits |
| A5 Ledger transaction algebra | Gap found | Local balance equation verified; hand-maintained prepared ledger delta/rollback; world snapshot use unproven | Centralize reversible deltas and prove live inventory in A6 after audits |
| A6 Whole-world energy inventory | Gap found | Snapshot is test-only; no production global balance gate; material-internal bond energy absent | Independent inventory and end-of-tick reconciliation after audits |
| A7 Resonance and reaction coupling | Gap found | Four-component cap; no explicit energy magnitude bound or general reservoir dissipation policy; epsilon pruning | Deposit/withdraw/serialization/spectrum energy invariants after audits |
| A8 World tone and capture | Gap found | Live capture active despite README/plan disabled claim; global source budget and scaling unproven | Resolve status, profile and compare sweep approximation, prove global budget after audits |
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
