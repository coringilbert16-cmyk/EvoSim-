# Validate Cache Versioning and Deterministic Sunlight Approximation

> **Planning scaffold only.** This file records the proposed scope and acceptance gate. No implementation changes or tests are included in this setup PR. Do not start implementation until explicitly authorized.

## Summary

Verify cache invalidation/versioning correctness and validate the performance-oriented sunlight approximation against a more exact reference. These are separate acceptance gates; split into separate implementation PRs if a single narrow diff cannot address both cleanly.

## Outline

- [ ] Audit cache keys, schema versions, fingerprints, invalidation, and restart behavior.
- [ ] Test stale-cache rejection and deterministic regeneration.
- [ ] Compare approximate sunlight exposure with exact small-world results.
- [ ] Verify occlusion, source-energy budget, and frequency-shifted onward propagation.
- [ ] Benchmark representative world sizes.

## Acceptance criteria

- [ ] Incompatible cached state cannot be silently reused.
- [ ] Approximation error is measured and bounded against the reference.
- [ ] Occlusion and source energy remain accounted for.
- [ ] Performance is measured against a documented workload.
- [ ] Cache and sunlight scopes remain separable; split implementation if needed.

## Guardrails

- Start with one focused audit of the current merged `main` before implementation.
- Use PR #205 merge commit `10521705d8f921ceb071b17240f1b7e069f52f1a` as the forward baseline; compare named test failure identities, not only totals.
- Keep each command and test run within five minutes; a timeout is incomplete evidence.
- Keep this PR limited to its primary invariant. Do not fold in unrelated cleanup.
- PR #204 is the roadmap/handoff; PR #196 constructor/Bob work is out of scope.
