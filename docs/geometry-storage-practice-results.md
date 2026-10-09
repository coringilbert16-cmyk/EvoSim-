# Geometry library compact-storage practice results

Status: **successful isolated data transformation; not yet a runtime storage migration**.

The practice run used the geometry JSONL files checked into the GitHub `main` branch. It wrote a separate compact copy as a GitHub Actions artifact and did not modify the source catalogue.

## Measured result

Validated run (safety tests + full conversion + artifact upload): [GitHub Actions run 37973218625](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/37973218625). The compact practice artifact is [available here](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/37973218625).

| Family file | Rows | Source bytes | Compact bytes | Saved bytes |
|---|---:|---:|---:|---:|
| `contact_families.jsonl` | 4,523 | 1,787,563 | 1,617,994 | 169,569 |
| `fluid_boundary_families.jsonl` | 4,523 | 2,081,909 | 1,912,340 | 169,569 |
| `rigid_contact_families.jsonl` | 307,996 | 101,825,277 | 90,726,241 | 11,099,036 |
| `rigid_vertex_contact_families.jsonl` | 153,998 | 58,430,470 | 52,880,952 | 5,549,518 |
| **Total** | **471,040** | **164,125,219** | **147,137,527** | **16,987,692** |

This is a measured reduction of approximately **10.35% across the family files** (about 16.2 MiB). Across the five checked-in JSONL files including the unchanged `formations.jsonl`, the reduction would be approximately 9.36%. This is only the checked-in GitHub snapshot, not the user's larger local catalogue.

## What the practice tool verified

- Indexed **25,807** canonical formation signatures from `formations.jsonl`.
- Generated deterministic 32-hex-character IDs from the first 128 bits of SHA-256.
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
