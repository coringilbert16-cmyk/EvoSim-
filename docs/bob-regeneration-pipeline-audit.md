# Bob clean-regeneration pipeline audit

Status: active audit; first low-risk worker optimizations are committed on `bob-automatic-compact-storage`. An initial clean-root Windows benchmark has now been measured and recorded below. A full catalogue rebuild has not completed.

## Direction

Treat generated geometry records as a reproducible build artifact when the generation rules and runtime query contract are preserved. Prioritize making a clean rebuild correct, deterministic enough to compare, and efficient. Do not spend engineering time compacting the user's existing catalogue unless measurement shows that persistence cost materially affects runtime or regeneration.

Keep source-of-truth material definitions, geometry rules, generator code, schema/version checks, and commands. The generated `geometry_library/data/` contents may be discarded and rebuilt once a clean rebuild has been verified. Do not delete the only checkout or any uncommitted source changes as part of a data reset.

## Verified pipeline

1. `src/bob/worker.rs::run_once` opens the configured persistent library, seeds missing base formations, processes one unfinished formation/resource pass, writes results, reports metrics, and exits.
2. `--geometry-worker-once` is the bounded smoke-test CLI path. A second run tests reopening persisted data.
3. `--geometry-worker` runs continuously and sleeps when no unfinished frontier is found; it is not a bounded full-rebuild command.
4. `--geometry-worker-passes N` processes at most N formation-frontier passes in one process, retaining the open catalogue between passes. Use this for bounded local throughput measurements before attempting a full regeneration.
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

## Initial clean-root Windows benchmark

Measurements supplied from a Windows release build on an isolated temporary library root:

| Run | Passes | Total formations at end | Worker-reported elapsed |
|---|---:|---:|---:|
| First `--geometry-worker-once` | 1 | 70 | 144 ms |
| Second `--geometry-worker-once` | 1 | 144 | 125 ms |
| `--geometry-worker-passes 25` | 25 | 2,101 | 5.047 s |
| Follow-up `--geometry-worker-passes 100` | 100 | 8,409 | 20.039 s |

For the 100-pass run, PowerShell measured 20.206 seconds wall time. The data root grew from 5,730,203 bytes to 27,161,049 bytes: 21,430,846 additional bytes while the total formation count increased by 6,308. This is approximately 63 newly persisted formations and 214 KB of additional stored data per pass on average for this interval.

At the end of the run, file sizes were:

| File | Bytes |
|---|---:|
| `rigid_contact_families.jsonl` | 14,796,068 |
| `rigid_vertex_contact_families.jsonl` | 8,800,481 |
| `formations.jsonl` | 2,834,915 |
| `fluid_boundary_families.jsonl` | 280,618 |
| `contact_families.jsonl` | 234,203 |
| `frontier.json` | 213,915 |
| `manifest.json` | 672 |
| `storage_manifest.json` | 177 |

The rigid edge and vertex family files account for approximately 87% of total bytes. This is the clearest next optimization target to investigate: measure how many family records are newly inserted per pass versus merely generated, then determine whether repeated generation, record verbosity, or legitimately distinct contact manifolds dominate. Do not remove valid families or reduce geometric coverage just to shrink output.

All 100 passes reported `size=2`; therefore this benchmark does not establish the cost or completion behavior at larger constituent counts. It shows successful persistence and growth, not a completed or bounded-time full rebuild. The release build emitted 35 warnings, which should be tracked separately from the successful build and worker execution.

## Family insertion metrics and identity audit

The follow-up instrumentation reports generated and newly persisted family counts separately. In the clean-root 25-pass run, every generated rigid edge and rigid vertex family was newly persisted in each pass; point families were zero in this sample. This means storage insertion is not discarding a large share of the generated families as duplicates.

Inspection of `GeometryRigidContactFamily::signature` and `GeometryRigidVertexContactFamily::signature` explains an important caveat: each signature includes the source formation identity, candidate resource, anchor constituent/edge, candidate edge or vertex, and quantized contact parameters/rotation. Thus families from different formations are intentionally distinct lookup records even if some underlying contact geometry appears similar. Equal generated/added counts do not prove all records are geometrically unique; they show that the current signature-based deduplicator found no duplicate keys in those batches or against already stored records.

The next optimization audit should therefore focus on **generation and representation**, not blindly increasing deduplication:
- quantify family records per formation and per candidate resource;
- verify which family dimensions the live constructor actually queries and whether the stored families are all needed for those query paths;
- inspect repeated/symmetric contact cases for equivalent records that the current identity intentionally distinguishes;
- compare compact JSONL bytes per record and cold-open/index-building cost;
- preserve all geometrically meaningful lookup cases. Do not merge records across formations unless query semantics and identity can be proven equivalent.

The benchmark is still early: all measured passes were for one- and two-constituent formations. It does not estimate the cost of the full constituent range.

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
