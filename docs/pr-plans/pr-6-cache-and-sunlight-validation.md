# Validate Chemistry Cache Versioning and Deterministic Regeneration

> **Planning scaffold only.** This phase establishes a safe cache-version boundary before later phases rely on cached reaction results. It does not implement changes.

## Summary

Establish deterministic compatibility rules for persisted chemistry-cache data. The cache accelerates calculations; it is never authoritative for live geometry, material ownership, reaction policy, or energy settlement.

## Outline

- Audit cache keys, schema versions, equation/parameter fingerprints, invalidation, and restart behavior.
- Reject incompatible legacy cache data without silently migrating or reinterpreting it.
- Test stale-cache rejection, deterministic regeneration, and failure-safe persistence.
- Ensure relevant equation/parameter changes invalidate incompatible cached results.

## Acceptance criteria

- Incompatible cached state cannot be silently reused.
- Cache identity changes when relevant equation/parameter semantics change.
- Cache load/rebuild behavior is deterministic and failure-safe.
- Focused tests cover stale schema/fingerprint and restart cases.
- This phase completes before unified energy accounting or reaction execution relies on cached results.

## Guardrails

- Audit current main before implementation; compare named test failure identities against the latest accepted baseline. PR #205 merge commit 10521705d8f921ceb071b17240f1b7e069f52f1a is the historical forward-baseline reset, not a claim that CI is green.
- Keep commands/tests under five minutes; timeout is incomplete evidence.
- Sunlight approximation, occlusion, source-budget checks, and performance benchmarks are explicitly out of scope here; they belong in a later separate validation phase.
- PR #204 remains the roadmap/handoff. PR #196 constructor/Bob work remains out of scope.
