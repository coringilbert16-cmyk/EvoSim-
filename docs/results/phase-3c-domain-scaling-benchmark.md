# Phase 3C — Fixed-Domain Scaling Benchmark

**Date:** 2026-11-25  
**PR:** #446  
**Source commit:** `4196daac946124780be77c4036c3adf1f8d59727`  
**Workflow:** Rust run `45626339152`, job `139247426554`  
**Command:** `bash scripts/benchmark_resonance_domain_scaling.sh`  
**Build:** Cargo release profile, locked dependencies  
**Benchmark:** ignored test `adaptive_spatial_grid::tests::benchmark_fixed_domain_domain_scaling`

## Purpose

Measure the current adaptive-grid implementation's construction and propagation cost at the candidate fixed chemistry-world size (250 × 250), and establish smaller-size comparison points. Both an empty field and a localized Gaussian/phase disturbance were measured. Each case advanced 20 logical ticks.

The benchmark is a performance characterization only. It does not change production solver behavior, establish physical boundary correctness, or validate an absorber.

## Results

All cases used two CFL substeps per logical tick.

| Case | Domain | Leaf cells | Grid build (ms) | Solver build (ms) | 20-tick advance (ms) | Relative energy drift |
|---|---:|---:|---:|---:|---:|---:|
| Empty | 32 × 32 | 1,024 | 0.118 | 0.772 | 46.310 | 0 |
| Localized wave | 32 × 32 | 1,024 | 0.064 | 0.557 | 46.358 | 3.18e-15 |
| Empty | 64 × 64 | 4,096 | 0.249 | 3.144 | 195.674 | 0 |
| Localized wave | 64 × 64 | 4,096 | 0.241 | 2.129 | 199.064 | 7.64e-15 |
| Empty | 128 × 128 | 16,384 | 1.106 | 12.893 | 888.427 | 0 |
| Localized wave | 128 × 128 | 16,384 | 1.108 | 9.256 | 881.355 | 3.40e-15 |
| Empty | 250 × 250 | 62,500 | 4.376 | 50.751 | 3,846.394 | 0 |
| Localized wave | 250 × 250 | 62,500 | 4.602 | 40.489 | 3,922.052 | 3.40e-15 |

The benchmark passed: 1 test passed, 0 failed. The dedicated CI job completed successfully.

## Findings

1. **The current representation pays for the full domain.** Empty and localized-wave propagation times are nearly identical at each size. A localized disturbance does not currently provide a meaningful runtime saving.
2. **Propagation cost scales approximately with cell count.** From 32 × 32 to 250 × 250, cell count increases by about 61× The measured 20-tick advance increases by about 83–85×. This indicates additional size-dependent overhead beyond a perfectly linear cell-count model; timings are single-run observations on a hosted runner, not stable performance guarantees.
3. **The 250 × 250 case is expensive.** Twenty logical ticks take 3.85–3.92 seconds (about 192–196 ms per logical tick) on this runner, before whole-simulation integration overhead.
4. **Numerical energy behavior is strong in this bounded test.** Localized-wave relative drift is between approximately 3.2e-15 and 7.6e-15 over 20 ticks. This is not evidence of correct open-boundary export: the benchmark's outer computational boundary is still fixed-zero, and no wave is required to leave a finite world.
5. **This benchmark does not measure memory.** Leaf count is reported, but no allocator/RSS measurement or reliable per-node memory accounting was performed.

## Engineering conclusion

Keep the approved finite 250 × 250 physical-world target. Do not shrink the physical domain, introduce an amplitude cutoff, or weaken propagation physics to improve benchmark numbers.

The present all-cells-in-`BTreeMap` representation should not be treated as the final production storage choice. The next investigation is a controlled comparison of storage layouts under the same uniform-grid wave update:

- contiguous dense arrays (the existing reference solver is the dense baseline);
- sparse active tiles/regions with an exact zero exterior and an expanding stencil halo;
- the current ordered-map representation as the comparison baseline.

The comparison must preserve the same stencil, CFL condition, deterministic update order, and full phase-resolved state. It must report construction and advance time at the same domain sizes and localized/empty profiles, plus memory accounting where it can be measured reliably. Sparse implementations must not discard small nonzero values using an arbitrary threshold. A sparse result is not acceptable unless it matches the dense reference within explicit numerical tolerances and preserves energy accounting.

This is an isolated performance investigation. It does not authorize production solver replacement, finite-boundary/absorber changes, Phase 3D material coupling, or changes to the physical model.
