# Bob clean-regeneration pipeline audit

Status: active audit; first low-risk worker optimizations are committed on `bob-automatic-compact-storage`. Performance has not yet been benchmarked locally, and no claim is made that a full catalogue rebuild has completed.

## Direction

Treat generated geometry records as a reproducible build artifact when the generation rules and runtime query contract are preserved. Prioritize making a clean rebuild correct, deterministic enough to compare, and efficient. Do not spend engineering time compacting the user's existing catalogue unless measurement shows that persistence cost materially affects runtime or regeneration.

Keep source-of-truth material definitions, geometry rules, generator code, schema/version checks, and commands. The generated `geometry_library/data/` contents may be discarded and rebuilt once a clean rebuild has been verified. Do not delete the only checkout or any uncommitted source changes as part of a data reset.

## Verified pipeline

1. `src/bob/worker.rs::run_once` opens the configured persistent library, seeds missing base formations, processes one unfinished formation/resource pass, writes results, reports metrics, and exits.
2. `--geometry-worker-once` is the bounded smoke-test CLI path. A second run tests reopening persisted data.
3. `--geometry-worker` runs continuously and sleeps when no unfinished frontier is found; it is not a bounded full-rebuild command.
4. `EVOSIM_GEOMETRY_LIBRARY_DIR` selects the persistent library root. The default is `geometry_library/data` relative to the current working directory. An explicit override is validated before opening.
5. New empty roots use compositional-v3 formation storage; existing legacy roots remain readable. Storage encoding is a persistence detail and must not change logical geometry or lookup results.

## Expansion bound consistency

The library README described a 20-constituent expansion target, but the inspected worker path previously called `expand_formation_candidates` without enforcing that target. That meant a continuous worker could keep creating larger formations instead of reaching a finite frontier.

The worker now enforces `MAX_LIBRARY_CONSTITUENTS = 20`: formations up to 20 constituents are eligible for expansion; formations at the limit still contribute local contact-family knowledge, but do not generate 21-constituent children. Each pass reports `expansion_limit_reached` when it encounters that boundary. This is an explicit bounded-library policy, not a claim that the catalogue will be small or that the full build will be quick; combinatorial growth below the limit still needs benchmarking.

## Waste found and changed

### Repeated Water capillary calculation

Previously the worker called `generate_water_contact_families` to persist contact families, then called `generate_fluid_boundary_families`, which called `generate_water_contact_families` again for the same formation and Water resource. That repeated the same equilibrium/boundary calculations.

The worker now computes contact families once and derives fluid-boundary records from that result. The public convenience function remains available and delegates through the same derivation helper.

### Rewriting the entire frontier file per resource transition

Previously the worker wrote the whole `frontier.json` twice for each resource it processed: once for `InProgress` and again for `Exhausted`. A formation pass processes the resource catalog together, so these repeated whole-file writes were unnecessary overhead.

The worker now accumulates completed resource states and persists them with one frontier write at the end of the formation pass. If interrupted before that checkpoint, the old unfinished states remain and generation is safely retried through the library's canonical deduplication and idempotent insertion paths. This is a durability design change, not a measured performance claim yet.

## Next measurements

On a clean, isolated data root, record:

- wall-clock duration for seeding and for one worker pass;
- selected formation constituent count;
- candidates generated versus formations newly persisted;
- rigid edge/point/vertex families and Water/fluid-boundary families produced;
- resulting formation/family row counts and total bytes by file;
- cold-open time and time to reopen after a pass;
- process peak memory if available on the target machine.

Run repeated passes at small catalogue sizes and compare throughput as the library grows. If the per-pass time rises sharply, profile before changing the expansion target or adding caches. In particular, the worker currently scans all formations to find the smallest unfinished formation each pass; measure that scan before replacing it with a persistent cursor or priority structure.

## Rebuild validation gates

1. Clean checkout builds and formatting checks pass.
2. A fresh empty root survives two `--geometry-worker-once` runs and reports compositional-v3 in `storage_manifest.json`.
3. Formation IDs, exact delta references, family formation references, and manifest/catalog signatures validate on reopen.
4. A bounded multi-pass run creates new records, and repeated processing does not create duplicate logical formations/families.
5. The complete generation run has a clear completion criterion; continuous worker idling alone is not a reproducible build result.
6. Compare fresh-build counts and representative lookup results with expected invariants. Do not treat matching byte counts as proof of equivalent geometry.
7. Only after those gates pass should generated records in an old local checkout be considered disposable.

## Scope boundaries

- Do not rewrite the user's existing catalogue in place.
- Do not lower the constituent expansion target solely to reduce time or space before measuring what the live constructor actually needs.
- Do not remove generation rules or geometry definitions along with generated output.
- Do not claim the full catalogue is complete based on a one-pass smoke test.
- Compact storage is still useful for persisted libraries, but is secondary to correct and efficient generation under the current project priority.
