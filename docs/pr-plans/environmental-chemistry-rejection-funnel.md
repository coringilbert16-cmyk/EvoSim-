# Environmental Chemistry Rejection Funnel

## Purpose

Record the failure causes hidden by the environmental BREAK and displacement
resolvers, then compare those causes in the standard seed-42, 60-second
organism-free experiment.

## Changes

- Environmental BREAK and displacement resolvers now return a categorized
  failure reason while retaining atomic commit and accumulation preservation.
- Per-reaction JSONL events include the reason. The run summary groups rejected
  BREAK events by reason; displacement already groups its event reasons.
- BREAK distinguishes stale sources, invalid bonds, insufficient usable
  energy, invalid energy accounting, and physical split rejection.
- Displacement distinguishes stale sources, restoration and geometry failures,
  trial validation, energy funding and settlement, product validation, and
  resonance validation failures.
- Resonance conservation uses a scale-aware bound derived from the representable
  precision of the before/after reservoir totals and component count. It still
  rejects discrepancies larger than that floating-point bound.

## Verification

- Focused BREAK insufficient-energy regression passed.
- Focused displacement funding and commit regression passed.
- Resonance summation-roundoff regression passed.
- `cargo fmt --check` and `git diff --check` passed.
- Standard run command:

  ```powershell
  cargo run --locked -- --headless --environment-chemistry-experiment --seconds 60 --seed 42 --diagnostics .evosim-experiments\environment-chemistry-rejection-funnel-accepted-60s-20261107\seed-42.jsonl --diagnostic-interval 100
  ```

  Result: 794 ticks in 60.017 seconds. BREAK: 21 commits, 12,704 scheduled,
  12,683 rejected; 12,682 rejections were insufficient usable energy and one
  was a stale source. COMBINE: 22 commits, 22 scheduled, no rejects.
  Displacement: all 3 scheduled reactions committed; the earlier instrumented
  run had identified a false rejection from an overly strict absolute
  resonance-conservation tolerance. Final inventory was conserved, there were
  1,360 constituents, and energy residual was `-2.23517417907715e-6`.

- Run artifacts remain local under `.evosim-experiments/`; they are not tracked.

## Follow-up

The dominant remaining BREAK limiter is insufficient usable energy, despite
high retained environmental resonance. The next investigation should measure
compatible resonant energy and the usable-energy deficit for scheduled BREAK
operations before changing reaction rates or energy rules.
