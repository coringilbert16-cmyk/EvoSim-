# Reaction Energy Semantics — PR A

## Intention

Establish one authoritative, auditable energy-settlement model for physical chemical COMBINE and BREAK reactions. A reaction's net energy change must be derived from its actual reactant and product states, rather than assuming that every bond formation requires an external energy grant or that every bond break has a universal fixed cost.

This is the thermodynamic foundation for environment-first chemistry experiments. It reuses the existing COMBINE transaction/rollback work from PR #209 and shared reaction runtime from PR #214; it must not duplicate those systems.

## Goals

1. **One reaction-energy contract.** Define how reactant state, product state, bond/material energy, formation or separation work, and dissipation contribute to the net balance.
2. **Separate thermodynamics from kinetics.** Net reaction energy and activation barriers are distinct. A favorable net reaction is not automatically guaranteed to occur, and an activation signal is not itself an energy source.
3. **Allow favorable reactions to fund themselves.** When the modeled reaction is energetically favorable and its physical pathway is accessible, COMBINE must not require an arbitrary external usable-energy grant merely to make the transaction balance. Any released energy must be accounted for exactly once.
4. **Conserve energy for unfavorable reactions.** If the product state requires more energy than the reactants provide, the difference must be funded by a legitimate modeled energy source or the reaction must not commit.
5. **Make COMBINE and BREAK consistent.** Both directions must use the same underlying accounting principles, with direction-appropriate work and activation requirements.
6. **Preserve atomicity.** A rejected, stale, unaffordable, or otherwise failed reaction must leave geometry, bonds, material ownership, energy holders, and ledgers unchanged.
7. **Keep accounting independently checkable.** Tests must reconcile authoritative material/bond inventories and energy ledgers without relying solely on the implementation's own calculated total.

## In scope

- Audit and implement the smallest shared settlement primitive needed for COMBINE and BREAK.
- Derive energy change from authoritative pre-reaction and post-reaction physical state; do not infer stored bond energy from a resource's maximum potential energy or from activation/signal values.
- Explicitly define the status and ownership of bond energy, formation/separation work, and dissipated energy so each quantity is counted once.
- Route released energy into explicitly modeled destinations (for example, physical work, resonance/usable energy where the existing model permits it, and heat/dissipation) with conservation checks.
- Add focused unit/integration tests for favorable, unfavorable, and energy-neutral reactions, activation barriers, successful settlement, insufficient legitimate energy, stale/rejected transactions, and rollback.
- Document the energy equation and assumptions beside the implementation and in tests.

## Out of scope

- Autonomous discovery of unbonded environmental pairs, environmental COMBINE scheduling, and multi-tick candidate progression. Those belong in a later PR only if this audit demonstrates they are necessary after PR A.
- New chemistry rules chosen only to make a seed or test structure survive.
- Death/decomposition policy, sunlight-model redesign, environment-only UI/observability, organism construction changes, or unrelated Clippy/test-baseline cleanup.
- Reimplementing the COMBINE transaction or environmental BREAK scheduler already added in PRs #209 and #214.

## Baseline audit findings (main)

The first source audit confirms this is not a missing-ledger problem; it is a mismatch between the existing transaction model and reaction thermodynamics.

- `combine_runtime` currently constructs COMBINE cost as formation investment + positive structural bond energy + formation work. `EnergyTransaction::expenditure` then charges that full amount to usable energy, allocating the positive bond energy to persistent structure (or dissipation for transient fluid bonds). The transaction is internally balanced, but it has no signed reactant-to-product reaction-energy term that lets a favorable reaction's net release offset the formation cost.
- The existing BREAK path can release only energy explicitly stored in a realized graph bond. `chemical_break_energy_yield` applies the recorded bond energy to disruption work first, with any deficit charged to usable energy and surplus allocated between usable energy and heat. Environmental `PhysicalMaterialBond` values currently have no stored-energy field and are restored with `bond_energy: 0.0`; environmental BREAK therefore cannot credit an unallocated bond-energy source.
- `EnergyTransaction` already represents signed structural change and checks a transaction balance. The ledger also exposes a world inventory snapshot. These are useful foundations, but COMBINE's expenditure constructor and BREAK's release formula do not yet implement one shared thermodynamic calculation from authoritative reactant/product states.
- Chemical activation is already modeled separately from energy settlement. The reaction accumulator is a trigger, not an energy reservoir; that invariant must be preserved.
- The environmental scheduler inspected in `chemical_reaction` discovers existing environmental bonds to BREAK. It does not discover unbonded environmental pairs for COMBINE. This confirms the capability is absent in the inspected path, but PR B remains deferred until PR A is implemented and its tests establish the actual integration seam. Do not open PR B as part of this audit.

### Implementation implication

Do not simply change the sign of `bond_energy` or treat the current intrinsic bond-potential value as a reaction-energy delta. The current field is a positive structural energy store that COMBINE pays to create and BREAK can later release. A favorable chemical reaction requires an explicit, authoritative definition of reactant and product energy states and a consistent delta calculation. If the current model lacks enough state to derive that delta without guessing, the implementation must make that representation gap explicit rather than invent values from resource maximum potential or activation signal.

## Implementation progress and remaining representation gap

The implementation now routes stable-bond COMBINE and graph-bond BREAK through the shared `ReactionEnergyBalance` calculation and `EnergyTransaction::reaction` settlement contract.

- **COMBINE:** computes signed structural potential from the current realized bond graph and the candidate product graph, then settles the product-minus-reactant difference with physical formation work. The formation threshold is used only for activation/formation eligibility; it is not charged as usable-energy investment or misclassified as heat. A favorable structural balance can credit the usable holder after work, while an unfavorable balance debits the holder and fails if unaffordable.
- **BREAK:** treats the positive bond-energy magnitude as a dissociation cost, not as an automatically available stored-energy reservoir. Because the unaffected graph is otherwise unchanged, the bond magnitude is the signed structural-potential increase. The same settlement primitive is used for chemical BREAK, decomposition, and division-boundary bond separation. Resonance, when used for disruption work, remains a separately debited holder; activation remains a gate only.
- **Inventory:** structural energy is now represented as signed potential (stable realized bonds contribute negative values). Initialization, independent world-inventory recomputation, and reproduction structural-energy accounting have been updated to use that convention.
- **Tests added/updated:** favorable, unfavorable, and neutral reaction-balance tests; zero-deficit COMBINE without an arbitrary energy grant; unaffordable BREAK atomicity; COMBINE energy/work/heat reconciliation; and the independent structural-inventory oracle.

### Latest exact-head verification — 2026-10-28

Commit `077680fb96e9ccb9031325ebd67eba073ce76ec7`, workflow run [#40676440725](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/40676440725):

- Formatting and repository architecture checks passed.
- All focused groups passed: energy accounting; chemistry/reactions; transactions/ownership; geometry/persistence; organism/seed acceptance.
- Full Rust suite: **804 passed, 9 failed, 2 ignored** in 325.55 seconds.
- All nine failing test identities are a subset of the accepted baseline run #40605811843 (**789 passed, 10 failed, 2 ignored**). The previous baseline failure `exact_construction_bond_forms_the_supplied_contact` now passes. No new failing test identity was introduced by this head.
- Clippy remains red: 32 application-target and 14 test-target diagnostics, none located in files changed by PR #222. The recorded prior baseline was 34 application-target and 14 test-target diagnostics, so this is not new PR-specific lint debt.
- The environmental discovery benchmark passed: budget 1 inspected 12,852 entries / 6 candidates in 0 ms; budget 16 inspected 205,824 / 126 candidates in 3 ms; budget 256 inspected 2,576,016 / 1,608 candidates in 40 ms.

The workflow is still red overall because the full suite retains the nine known baseline failures and strict Clippy retains repository-wide lint debt. The exact-head result does not show a new regression, but it is not a green CI run and is not by itself merge approval.

### Audit findings and next steps

- The shared accounting path is internally consistent for realized graph bonds: COMBINE uses actual reactant/product graph potentials; graph BREAK, decomposition, and division-boundary separation charge the corresponding structural-potential increase plus work; failed settlements are staged and reject without committing the relevant graph/ledger changes.
- **Source-neutral funding remains explicit:** `required_external_input` describes the minimum reaction-energy deficit that must be funded, not a designated source and not an automatic draw from sunlight. A source may be captured sunlight, energy released by another reaction and actually transferred, energy invested from the organism's usable stores, or a future modeled environmental source such as a hotspot. It counts only when made available to the reaction and transferred/debited exactly once.
- **Confirmed processing-efficiency semantics (2026-10-28):** `processing_efficiency` governs how effectively an organism supplies energy for a desired reaction. It must affect the funding/conversion step, not the physical reaction-energy delta, bond potential, activation threshold, or resonance capture. For an energy-funded reaction, determine the net thermodynamic requirement first; then determine the usable energy the reaction needs and the legitimate compatible energy available to fund it. When drawing from resonance, efficiency determines the reservoir withdrawal needed to deliver that usable energy. The difference between reservoir energy withdrawn and usable energy delivered must be explicitly accounted for as conversion loss/heat (or another defined destination). Do not count the same energy as both remaining resonance and usable reaction funding.
- **Funding semantics must remain physically honest:** high processing efficiency cannot create energy, make incompatible resonance usable, or turn energy that is merely nearby into available energy. If the organism cannot provide enough usable energy from eligible holders after efficiency and conversion losses, reject the reaction atomically. If the model supports more than one legitimate funding path (such as usable stores and resonance), specify and test the allocation order/policy rather than silently double-counting or inventing a source.
- The current BREAK helper accepts `processing_efficiency` but does not use it to change energy allocation. This is now a defined implementation gap, not a decision to leave the trait inert. The implementation must trace COMBINE and BREAK funding paths and connect efficiency only at the energy-supply step, with tests proving that changing efficiency changes the cost/effectiveness of supplying a reaction but never changes the reaction's physical energy balance.
- Environmental/stored-material internal bonds still have no authoritative potential field, so their BREAK path must not claim a bond-energy credit. Fluid coalescence likewise does not claim a persistent bond-potential release. The intrinsic graph-bond potential is a model assumption, not a calibrated chemical free-energy table.

Recommended next steps: (1) trace the actual usable-energy and resonance-reservoir funding paths for COMBINE and BREAK; (2) implement the smallest shared funding/conversion step that applies organism processing efficiency only when supplying energy for a desired reaction; (3) account for reservoir withdrawal, usable energy delivered, and conversion loss exactly once; (4) add focused tests showing efficiency changes funding effectiveness without changing thermodynamic reaction deltas, plus insufficient-funding atomic rollback; (5) rerun focused conservation/rollback checks and compare full-suite failure identities to the accepted baseline. Keep PR #222 draft until these checks pass. Do not chase unrelated Clippy debt or baseline failures, and do not open PR B until this audit is closed.

### Remaining physics/representation limits

- The current stable-bond energy delta is the realized graph's signed bond-potential change. That is authoritative for the *existing bond representation*, but it does not by itself prove that `intrinsic_bond_potential` is a calibrated chemical free-energy difference for every resource pair. The interaction model's physical meaning and calibration remain a scientific assumption to document, not a measured result.
- Environmental `PhysicalMaterialBond` still has no stored bond-energy field. This PR must not infer a release from resource type maxima or activation values. Environmental BREAK with no authoritative bond-energy representation must remain unable to claim such energy.
- Fluid coalescence remains an explicit expenditure path because the merged fluid product does not yet expose authoritative internal-bond potential. Do not count its transient graph bond as persistent binding energy.
- The source-neutral deficit contract does not itself implement source discovery or delivery. Sunlight, energy released by another reaction, organism energy stores, and future environmental sources such as hotspots can fund a deficit only when their energy is actually made available to the reaction and transferred/debited exactly once. Nearby sunlight or a nearby energy-releasing reaction is not automatically available merely because it exists. If no legitimate source/holder supplies the deficit, the reaction must fail atomically rather than mint energy.
- Full verification still needs to prove all transaction rollback paths, independently reconcile material/bond and energy inventories, and compare full-suite failure identities against the accepted baseline.

## Merge requirements

PR A is mergeable only when all of the following are satisfied:

- [ ] The shared reaction-energy equation and its terms are explicit, reviewed, and used consistently by COMBINE and BREAK.
- [ ] A favorable COMBINE can commit without an arbitrary external energy grant when the modeled pathway is accessible; any net release is recorded exactly once.
- [ ] An energetically unfavorable reaction cannot commit unless a legitimate energy source covers the deficit; failed attempts create no energy.
- [ ] Organism `processing_efficiency` affects how effectively legitimate available energy is supplied to a desired reaction, without changing its thermodynamic delta, bond potential, activation threshold, or resonance capture.
- [ ] Resonance-funded reaction energy debits the compatible reservoir by the efficiency-adjusted amount, credits only the usable energy actually delivered, and accounts for conversion loss explicitly; no energy is double-counted.
- [ ] Tests demonstrate different processing efficiencies change reaction-funding effectiveness while leaving the same reaction's physical energy balance unchanged.
- [ ] Activation barriers/work are accounted for separately from net reaction energy. Activation signals do not count as energy.
- [ ] Stored bond energy has an authoritative representation or is explicitly absent from the model; resource maximum potential energy is not silently substituted for it.
- [ ] Successful COMBINE and BREAK pass independent inventory/ledger reconciliation, including material/bond changes, released energy, work, and dissipation.
- [ ] Rejected, stale, and insufficient-energy cases prove atomic rollback of all relevant physical and energy state.
- [ ] Tests cover favorable, unfavorable, and neutral balances and verify no double-counting of bond energy or dissipation.
- [ ] Focused chemistry/transaction tests, formatting, architecture checks, and the agreed full-suite verification pass without new regressions against the current baseline. Existing baseline failures must be identified by test identity, not treated as new failures.
- [ ] The PR description and implementation state clearly what remains unresolved. Do not mark ready to merge while any conservation or rollback requirement is unverified.

## Audit gate for a later environmental-COMBINE PR

Do not open PR B automatically. After PR A's code and tests are audited, decide whether environmental COMBINE discovery/scheduling is actually a separate necessary change. If the existing runtime already supports the required path after a small integration, document that finding and avoid a second PR. If a separate environmental discovery layer is still missing, propose PR B with its own bounded-scheduling and end-to-end acceptance criteria before opening it.

## Existing foundations to preserve

- PR #209: selected-COMBINE transactionality, live contact revalidation, staged geometry/bond changes, settlement-before-commit, and rollback.
- PR #214: shared chemistry runtime and bounded environmental BREAK discovery/scheduling.
- PR #215: environment-first experiments and reaction-event observability; it should consume real runtime events, not define thermodynamic semantics.
