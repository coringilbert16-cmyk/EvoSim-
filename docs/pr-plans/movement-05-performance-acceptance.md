# PR 5: Benchmark and finalize resonance-driven movement

**Status:** Planning outline; implementation not yet started.
**Base:** `main`
**Goal:** Verify the completed movement system at the standard environment scale and close the project.

## Goal
Establish that the complete simple resonance-driven movement system works at the intended scale without making the simulation impractical.

## Outline
1. Run the standard 60-second, seed-42 environment test with 1,280 starting objects.
2. Record baseline and movement-enabled throughput using the same conditions and real elapsed time.
3. Profile only if a material regression is found; optimize the simple pair-discovery/calculation path without introducing a multipole or physics-engine architecture.
4. Verify all movement and chemistry acceptance tests together.
5. Document final behavior, measured throughput, and any remaining limitations.

## Explicit exclusions
No speculative optimization framework, hierarchy research project, expanded physics model, or unrelated chemistry/GUI work.

## Acceptance
- Full CI passes.
- Same-phase repulsion, opposing-phase attraction, quadrature neutrality, distance attenuation, multi-pair response, boundary safety, molecular integrity, and chemistry integration all pass.
- Standard 60-second, 1,280-object test completes and throughput is reported.
- Movement is considered complete; no additional physics subsystem is implied.

## Sequence
This is step 5 of 5. Complete and merge the preceding step before beginning implementation here. This PR must remain independently scoped to the objective above.
