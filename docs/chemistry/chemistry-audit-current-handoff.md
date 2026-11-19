# Chemistry Unification — Current Handoff

**Last refreshed:** 2026-10-30  
**Plan:** [Chemistry Audit and Unification Plan](chemistry-audit-unification-plan.md); historical audits remain intact, and the latest dated integration checkpoint is appended at the end of that plan  
**Plan branch:** `docs/chemistry-audit-unification-plan-2026-10-22`  
**Mainline integration anchor:** PR #240 merge `bca1561c137b825814bf7dd29a3b6fd9200f3488`, followed by documentation-only merges #204, #210, #215, and #216  
**Scope:** Documentation-only handoff. No production implementation or tests belong in PR #204.

## Current state

The shared chemistry runtime/scheduler and the first compound-chemistry characterization chain are now integrated into `main`. Chemistry is **not yet fully unified**: the remaining work is to prove raw environmental material can enter ordinary chemistry naturally, establish consistent participation across all physical holders/lifecycle states, and complete the outstanding conservation and sunlight evidence gates.

### Merged foundations

- [PR #207](https://github.com/coringilbert16-cmyk/EvoSim-/pull/207) — canonical resource identity, indivisible material quantities, and independent material inventory.
- [PR #208](https://github.com/coringilbert16-cmyk/EvoSim-/pull/208) — unified energy accounting and independent world-energy inventory.
- [PR #209](https://github.com/coringilbert16-cmyk/EvoSim-/pull/209) — transactional COMBINE realization/settlement and inventory proof.
- [PR #211](https://github.com/coringilbert16-cmyk/EvoSim-/pull/211) — chemistry cache schema/fingerprint validation.
- [PR #212](https://github.com/coringilbert16-cmyk/EvoSim-/pull/212) — authoritative aggregate ownership and transactional construction/division foundations.
- [PR #213](https://github.com/coringilbert16-cmyk/EvoSim-/pull/213) — temporary zero-stress input for survival-first development; not the final stress/death contract.
- [PR #214](https://github.com/coringilbert16-cmyk/EvoSim-/pull/214) — shared chemistry runtime and reaction scheduler.
- [PR #223](https://github.com/coringilbert16-cmyk/EvoSim-/pull/223) — bounded environmental COMBINE discovery and settlement.
- [PR #226](https://github.com/coringilbert16-cmyk/EvoSim-/pull/226) — S-shaped flow, gradient-driven transport, eddies, and obstacle-aware movement.
- [PRs #229–#234](https://github.com/coringilbert16-cmyk/EvoSim-/pull/234) — canonical and controlled base-pair matrices, product characterization, compound-to-base chemistry, compound-to-compound contacts, and committed four-constituent COMBINE with energy conservation.
- [PR #240](https://github.com/coringilbert16-cmyk/EvoSim-/pull/240) — integrated the later molecular characterization chain: composition/topology, four-part COMBINE/BREAK round trip, heterogeneous constituent interfaces and reaction keys, fair rotating contact discovery, and the read-only molecular-layout geometry snapshot. The earlier #235–#239 feature-branch merges are now represented in `main` through this integration.

## Verification checkpoint and baseline policy

- The focused chemistry workflow on the integrated #240 branch passed formatting and all focused gates, including molecular-layout geometry, heterogeneous reaction-key identity, rotating contact discovery, compound/base and compound/compound transactions, the four-part round trip, molecular characterization, both base-pair matrices, environmental COMBINE/BREAK inventory, discovery budgets, stored bond-energy restoration, and settlement regressions.
- The full-suite run on the preceding #234 head completed with **842 passed, 7 failed, 2 ignored**. Its seven failure identities exactly matched the pre-characterization `main` run at `d38519d2b139ea8749c90b83fcc90cb931cd130e` (**834 passed, 7 failed, 2 ignored**). No new failing identity appeared in that comparison; eight additional tests passed.
- The #240 branch's full-suite step was cancelled when the merge triggered a new `main` workflow. The exact post-merge `main` workflow is still the authoritative follow-up; do not describe the full suite as green until it completes.
- Clippy has not been re-established on the post-#240 main head. Preserve the previously recorded repository-wide lint debt as historical context, not as evidence about this new head.
- These results do not prove whole-world conservation or full chemistry unification.

## Current implementation work and merged plans

- [PR #225](https://github.com/coringilbert16-cmyk/EvoSim-/pull/225) is the only remaining open draft. It is COMBINE-discovery-only for its first raw-reactant acceptance slice. #226 is merged; exact integer fixture, placement/randomization, flow conditions, and environmental energy source require owner approval before implementation. Do not force contacts, lower thresholds, or use organism energy as a hidden source. Natural BREAK discovery is deferred.
- [PR #210](https://github.com/coringilbert16-cmyk/EvoSim-/pull/210), [PR #215](https://github.com/coringilbert16-cmyk/EvoSim-/pull/215), and [PR #216](https://github.com/coringilbert16-cmyk/EvoSim-/pull/216) are now merged as planning documents. Their implementation work remains unstarted and must follow their dependencies and acceptance gates.
- [PR #204](https://github.com/coringilbert16-cmyk/EvoSim-/pull/204) is merged as the documentation-only roadmap/handoff.
- PR #196 constructor/Bob work and PR #179 geometry-storage practice are closed; do not reopen wholesale. Recover individual ideas only after a fresh audit against current main and the current organism definition.

## Next sequence

1. Audit #225 against current `main`; propose the exact discrete raw-material inventory, placement method, and environmental energy accounting for owner approval before implementing its fixture.
2. Continue #215 once its event contract can be tied to the merged shared runtime; preserve the default environment preset and bounded work.
3. Implement #210 as a lifecycle transition into the same ordinary chemistry path, with exact material/energy ownership through death.
4. Complete #216's exact-reference, occlusion, source-budget, duplicate-claim, onward-propagation, and performance checks.

Keep phases separate and acceptance-driven. Do not open additional PRs without explicit authorization.

## Definition of “chemistry unified”

Do not make that claim until environmental, organism/cell-held, and dead material use the same reaction policy and transaction path; rejection preserves geometry, bonds, material ownership, energy, and inventories; successful reactions reconcile integer material and independent world-energy inventories; scheduling is deterministic and bounded; and the sunlight plus whole-world conservation gates have evidence.
