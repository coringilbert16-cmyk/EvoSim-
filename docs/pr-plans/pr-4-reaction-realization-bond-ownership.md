# Make Reaction Realization and Bond Ownership Transactional

## Status

Implementation and focused verification are consolidated in this PR. Compared with the accepted baseline run for `10521705d8f921ceb071b17240f1b7e069f52f1a`, the latest completed PR run had no new failing test identities: all 10 current failures were among the baseline's 21 distinct failures, and 11 baseline failures no longer reproduced. The PR suite completed in 278.22 seconds versus 292.69 seconds at baseline, both under the seven-minute ceiling. Clippy still fails on the pre-existing repository warning backlog (34 application-target diagnostics and 14 test-target diagnostics on this run); this PR does not expand into that cleanup.

## Scope and invariant

Ensure chemistry reactions change authoritative physical material and bonds exactly once, with accounting committed consistently alongside physical realization.

This phase owns **atomic physical realization and settlement of a selected reaction**: validate live contacts/endpoints, stage material/bond changes, settle energy, then commit once or roll back completely. It does not own the world-wide candidate scheduler, reaction-progress lifetime, or environment-first UI/logging; those are separate planning scopes.

## Implemented

- Revalidate a selected COMBINE contact against live geometry before settlement; reject stale contact metrics or unavailable endpoints without mutating structure or energy.
- Stage bond insertion and fluid coalescence on a trial graph; commit the physical graph only after energy settlement succeeds.
- Preserve rollback behavior for failed COMBINE realization, including energy/ledger state and environmental-source restoration.
- Preserve transactional BREAK behavior: validate the target bond and trial graph before settlement; commit structure only after the energy transaction succeeds.
- Keep stored and environmental BREAK sources escrowed while a transformation is active; restore them on failed completion, preserving original holder/cell position when valid and retaining material explicitly when exact restoration is unavailable.
- Add a successful live-COMBINE test against the independent simulation-level world-energy inventory.
- Add a regression proving that a small geometry change after candidate selection rejects stale COMBINE metrics and preserves structure, energy, and ledger.
- Retain the existing BREAK inventory, source rollback, and bond ownership regressions.

## Acceptance criteria

- [x] Reaction bookkeeping corresponds to actual physical changes.
- [x] Bonds have unambiguous ownership and valid endpoints on committed graphs.
- [x] Rejected reactions preserve material, bonds, energy, and inventories through staged transaction/rollback paths.
- [x] Successful reactions reconcile material and energy consequences.
- [x] Formatting, architecture checks, and all focused test groups pass on the current head.
- [x] Full-suite failure identities match the accepted baseline and runtime remains under seven minutes.
- [ ] Merge readiness confirmed after final review of the current PR head.

## Verification on current head

Head: `386b04b120137a55433a2e639b55f4aa34297012`

- Formatting: passed.
- Repository architecture checks: passed.
- Energy accounting focused tests: passed.
- Chemistry/reactions focused tests: passed.
- Transactions/ownership focused tests: passed.
- Geometry/persistence focused tests: passed.
- Organism/seed acceptance focused tests: passed.
- Full Rust suite: 754 passed, 10 failed, 1 ignored in 278.22 seconds. All 10 current failing test names are a subset of the baseline's 21 distinct failing test names; no new failure identities were observed, and 11 baseline failures no longer reproduced.
- Clippy: failed on the existing warning backlog; not treated as a new transactionality regression.

## Guardrails and dependencies

- Use PR #205 merge commit `10521705d8f921ceb071b17240f1b7e069f52f1a` as the baseline; compare named test failure identities, not only totals.
- Keep this PR limited to its primary invariant. Do not fold in unrelated cleanup or optimization.
- PR #204 is the roadmap/handoff; PR #196 constructor/Bob work is out of scope.
- PR #207 owns material ownership and PR #208 owns energy accounting. The shared chemistry runtime/scheduler and cache-version boundary remain separate scopes; this PR does not add a special environmental reaction law.
- The temporary supporting changes from PRs #219 and #220 are consolidated here; those PRs are closed and are not independent merge targets.
