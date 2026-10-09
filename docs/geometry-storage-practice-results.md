# Geometry library compact-storage practice results

Status: **successful isolated data transformation; not yet a runtime storage migration**.

The practice run used the geometry JSONL files checked into the GitHub `main` branch. It wrote a separate compact copy as a GitHub Actions artifact and did not modify the source catalogue.

## Measured result

Validated full-library run (safety tests + formation-signature reconstruction + full conversion + artifact upload): [GitHub Actions run 37973923101](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/37973923101). The artifact is listed in that run's Artifacts section and expires after seven days.

| Compacted file | Rows | Source bytes | Compact bytes | Saved bytes |
|---|---:|---:|---:|---:|
| `contact_families.jsonl` | 4,523 | 1,787,563 | 1,482,304 | 305,259 |
| `fluid_boundary_families.jsonl` | 4,523 | 2,081,909 | 1,776,650 | 305,259 |
| `formations.jsonl` | 25,807 | 17,386,631 | 14,618,529 | 2,768,102 |
| `rigid_contact_families.jsonl` | 307,996 | 101,825,277 | 81,486,361 | 20,338,916 |
| `rigid_vertex_contact_families.jsonl` | 153,998 | 58,430,470 | 48,261,012 | 10,169,458 |
| **Total** | **496,847** | **181,511,850** | **147,624,856** | **33,886,994** |

The optimized format reduces the five checked-in geometry JSONL files by approximately **18.67%** (about 32.3 MiB), before counting the small sidecar manifest. This is only the checked-in GitHub snapshot, not the user's larger local catalogue.

The first practice format used 32-character hexadecimal IDs and repeated `storage_version: 2` on every family row; it saved 16,987,692 bytes across family files (10.35%). The optimized format removes the repeated row-level version field, stores version/encoding once in `storage_manifest.json`, and encodes the same 128-bit SHA-256 prefix as a 22-character unpadded base64url ID. This alone saved an additional **14,131,200 bytes** over the first family-only format. The latest pass also removed each formation's redundant persisted `signature` string, replacing it with the same compact ID; this saved another **2,768,102 bytes**.

## What the practice tool verified

- Indexed **25,807** canonical formation signatures from `formations.jsonl`.
- Recomputed each canonical formation signature from its persisted schema version, constituent resources/placements, and bond endpoints using the library's 1e-9 quantization and angle normalization; all 25,807 recomputed signatures matched their stored source signatures exactly before compaction.
- Generated deterministic 22-character unpadded base64url IDs from the first 128 bits of SHA-256; the ID still carries 128 bits of collision resistance.
- Stored format version and ID-encoding metadata once in `storage_manifest.json`, not redundantly in every family row.
- Resolved every family record's ID back to its original canonical signature.
- Preserved all family fields other than replacing `formation_signature` with `formation_id` and adding `storage_version: 2`.
- Reconstructed each source record in memory and compared it for exact Python-object equality.
- Re-read every written compact JSONL file and checked the output row count and parsed records.
- Removed the redundant `signature` field from compact formation rows only after confirming it can be reconstructed exactly from geometry; retained the canonical signature in memory for ID mapping and round-trip validation.
- Preserved all source files; the compact copy was uploaded as a short-lived artifact.

The four family files had no duplicate logical rows under the practice tool's signature-plus-fields check. The repository snapshot does not include `rigid_point_contact_families.jsonl`, so that family type was not exercised by this dataset run.

## Limits and next gates

1. The successful run validates a **data transformation**, not the Rust reader/writer integration. Current runtime structs still use `formation_signature`; they must not be switched to compact records until dedicated on-disk DTOs and ID resolution are implemented and verified.
2. The focused safety-test suite passed in the validated run. It covers exact round-trip, source preservation, unresolved references, duplicate canonical signatures, simulated ID collisions, malformed JSON, and refusal to overwrite a non-empty destination.
3. Before considering the format for the main/local library, run the same tool on the complete local catalogue, measure its actual savings, and extend the practice validation to every family type present there.
4. Do not delete or overwrite the source data. Keep the compact output separate until Rust compatibility, restart behavior, and lookup equivalence are proven.

No geometry parameters, canonical signatures, formation records, or construction behavior were changed in this practice run.

## Exact compositional-storage experiment (practice-only)

The one-constituent composition analyzer was run against the same 25,807 formations in [workflow run 37974590539](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/37974590539). Its JSON result is also available as the short-lived `geometry-compositional-storage-results` artifact.

| Measurement | Result |
|---|---:|
| Current compact formation-row bytes | 14,592,722 |
| Estimated one-constituent delta encoding | 8,510,547 |
| Additional formation-file saving | **6,082,175 bytes (41.67%)** |
| Rows with a profitable exact delta | 23,027 of 25,807 |
| Profitable rows with 2 constituents | 451 |
| Profitable rows with 3 constituents | 6,528 |
| Profitable rows with 4 constituents | 16,048 |

This experiment searches for a smaller stored formation whose constituent list and remaining bond records match the target's one-unit-removed subset exactly. A delta stores the target's own ID, the base ID, insertion index, full removed constituent, and incident bonds with their original positions. Each candidate delta is reconstructed immediately and compared against the original constituent list, bond list, and canonical signature. A canonical-signature match alone is not enough: the actual persisted subset fields must also match exactly.

**Interpretation:** if this representation is implemented in the storage layer, it suggests a further ~6.08 MB reduction from the already compacted formation rows. Applying that estimate to the previously measured full compact copy would reduce the five JSONL files from 147,624,856 bytes to approximately 141,542,681 bytes (about 22.02% below the original snapshot). This is a size estimate, not yet a generated compositional dataset; it excludes any additional decoder/index overhead and does not include a production Rust implementation.

The experiment remains isolated from the source dataset and production library. A separate compositional-v3 encoder/decoder has now been implemented on the practice branch. In [workflow run 37974766627](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/37974766627), the focused compositional round-trip test and full-dataset recursive reconstruction both passed; the workflow also produced a separate `geometry-library-compositional-v3-practice` artifact. The decoder checks unresolved/cyclic references, insertion indices, bond positions and ordering, all persisted fields, and reconstructed canonical signatures. The compact-v2 family files remain unchanged in this practice output. Runtime Rust DTO integration, restart behavior, and lookup-equivalence testing are still outstanding; do not replace the main/local library until those gates pass.
