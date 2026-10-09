# Geometry library compact-storage practice results

Status: **successful isolated data transformation; not yet a runtime storage migration**.

The practice run used the geometry JSONL files checked into the GitHub `main` branch. It wrote a separate compact copy as a GitHub Actions artifact and did not modify the source catalogue.

## Measured result

Validated optimized run (safety tests + full conversion + artifact upload): [GitHub Actions run 37973498112](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/37973498112). The artifact is listed in that run's Artifacts section and expires after seven days.

| Family file | Rows | Source bytes | Compact bytes | Saved bytes |
|---|---:|---:|---:|---:|
| `contact_families.jsonl` | 4,523 | 1,787,563 | 1,482,304 | 305,259 |
| `fluid_boundary_families.jsonl` | 4,523 | 2,081,909 | 1,776,650 | 305,259 |
| `rigid_contact_families.jsonl` | 307,996 | 101,825,277 | 81,486,361 | 20,338,916 |
| `rigid_vertex_contact_families.jsonl` | 153,998 | 58,430,470 | 48,261,012 | 10,169,458 |
| **Total** | **471,040** | **164,125,219** | **133,006,327** | **31,118,892** |

The optimized format reduces family-file bytes by approximately **18.96%** (about 29.7 MiB). Across the five checked-in JSONL files including unchanged `formations.jsonl`, the reduction is approximately 17.14%, before counting the small sidecar manifest. This is only the checked-in GitHub snapshot, not the user's larger local catalogue.

The first practice format used 32-character hexadecimal IDs and repeated `storage_version: 2` on every row; it saved 16,987,692 bytes (10.35%). The optimized practice format removes the repeated row-level version field and stores version/encoding once in `storage_manifest.json`, while encoding the same 128-bit SHA-256 prefix as a 22-character unpadded base64url ID. This saves an additional **14,131,200 bytes** versus the first format.

## What the practice tool verified

- Indexed **25,807** canonical formation signatures from `formations.jsonl`.
- Generated deterministic 22-character unpadded base64url IDs from the first 128 bits of SHA-256; the ID still carries 128 bits of collision resistance.
- Stored format version and ID-encoding metadata once in `storage_manifest.json`, not redundantly in every family row.
- Resolved every family record's ID back to its original canonical signature.
- Preserved all family fields other than replacing `formation_signature` with `formation_id` and adding `storage_version: 2`.
- Reconstructed each source record in memory and compared it for exact Python-object equality.
- Re-read every written compact JSONL file and checked the output row count and parsed records.
- Preserved all source files; the compact copy was uploaded as a short-lived artifact.

The four family files had no duplicate logical rows under the practice tool's signature-plus-fields check. The repository snapshot does not include `rigid_point_contact_families.jsonl`, so that family type was not exercised by this dataset run.

## Limits and next gates

1. The successful run validates a **data transformation**, not the Rust reader/writer integration. Current runtime structs still use `formation_signature`; they must not be switched to compact records until dedicated on-disk DTOs and ID resolution are implemented and verified.
2. The focused safety-test suite passed in the validated run. It covers exact round-trip, source preservation, unresolved references, duplicate canonical signatures, simulated ID collisions, malformed JSON, and refusal to overwrite a non-empty destination.
3. Before considering the format for the main/local library, run the same tool on the complete local catalogue, measure its actual savings, and extend the practice validation to every family type present there.
4. Do not delete or overwrite the source data. Keep the compact output separate until Rust compatibility, restart behavior, and lookup equivalence are proven.

No geometry parameters, canonical signatures, formation records, or construction behavior were changed in this practice run.
