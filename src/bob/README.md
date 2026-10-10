# Bob — Geometry Reference Library

Bob stores reusable geometry knowledge for EvoSim. It is not live organism state and does not replace the physical validator.

## Responsibilities

- `src/bob/library.rs`: persistent formations, contact-family records, canonical identities, storage schemas, and indexed read paths.
- `src/bob/worker.rs`: discovers formations and contact families, persists results, and reports generation metrics.
- `src/bob/server.rs`: read-only geometry-library viewer.
- `src/construction/initial_organism_constructor.rs`: the live initial-organism constructor. Its current scaffold-based implementation is **not yet the approved library-driven replacement**.

A library result is reusable knowledge or a candidate, not permission to mutate a live organism. Live placement must still obey authoritative geometry, contact, nonpenetration, chemistry, bonding, energy, and ledger rules.

## Library location

Set `EVOSIM_GEOMETRY_LIBRARY_DIR` to choose the persistent library root. If unset, Bob uses `geometry_library/data` relative to the process working directory. An explicit override must point to a valid library; Bob must not silently replace a missing explicitly configured library with a new empty one.

Generated data is rebuildable only when the generation rules and required lookup coverage are preserved. Keep source definitions, generator code, schemas, validation rules, and the commands needed to regenerate it. Do not delete the only useful catalogue until a clean rebuild has demonstrated the coverage the live constructor needs.

## Storage behavior

- Fresh empty roots use the compositional-v3 formation format, with deterministic compact formation IDs and exact one-constituent deltas where the delta encoding is smaller.
- Existing legacy roots remain readable; the on-disk encoding must not change canonical runtime identity or lookup results.
- Family rows can use compact formation references on disk while runtime signatures remain canonical.
- Frontier updates are batched at the end of a formation pass. If a pass is interrupted before checkpointing, unfinished work can be retried through idempotent insertion.
- Water contact families are generated once per formation/resource pass and reused to derive fluid-boundary records.
- The worker reports generated and newly inserted family counts separately. Equal counts indicate no duplicate keys were rejected; they do not prove all records are geometrically unique.

## Worker commands

Run these commands from the repository root after building the executable:

```powershell
# Process one formation/resource frontier pass
.\target\release\evosim.exe --geometry-worker-once

# Process at most N passes in one process
.\target\revosim.exe --geometry-worker-passes 25

# Run continuously; the worker idles when no unfinished work is found
.\target\evosim.exe --geometry-worker
```

The `--geometry-worker-passes N` value must be positive. A one-pass run or a successful persistence/reopen check validates only that narrow behavior; it does not establish full catalogue completion or constructor viability.

The worker enforces `MAX_LIBRARY_CONSTITUENTS = 20`: formations below the limit may be expanded, and formations at the limit may still contribute local contact-family knowledge but are not expanded into 21-constituent formations. This is a bounded-library policy, not a guarantee of small storage or quick completion. Combinatorial growth below the limit remains a known risk and must be measured.

## Constructor integration contract

The approved replacement constructor is library-driven and milestone-based, not a fixed topology or piece-count recipe. It must:

1. propose candidate attachments or reusable local arrangements from Bob;
2. validate every candidate against the whole relevant realized structure and authoritative physical rules;
3. use atomic trial/commit/rollback for geometry, bonds, material inventory, energy, and ledgers;
4. discover a genome only when the real cavity analyzer finds a qualifying bonded seal;
5. stop the genome-building phase as soon as that cavity qualifies;
6. physically acquire Water plus at least three distinct non-Water resource categories;
7. return only a physically viable organism and report specific blockers otherwise.

Initial construction and offspring construction should share one physical engine with different policies. A blueprint is a soft preference, not an authoritative final topology. The detailed dependency audit records the current API and integration blockers.

## Verification status

The latest completed source-branch workflow passed formatting, source-file-size checks, COMBINE architecture checks, focused compact-storage tests, and a fresh-library persistence/reopen check. The Rust test and Clippy steps still fail before a green full-suite result, including strict-lint failures from unfulfilled `dead_code` expectations. The previously recorded full test run was 253 passed, 75 failed, and 1 ignored. Do not describe Bob or the constructor as fully verified based on the focused storage checks.

The integration branch is the current consolidation target: [PR #186](https://github.com/coringilbert16-cmyk/EvoSim-/pull/186). It remains draft-only until compile/lint blockers are resolved, focused physical contracts execute, the full suite is recorded, and the actual constructor is shown to produce a viable organism.

## Audit documents

- [Bob regeneration pipeline audit](../../docs/bob-regeneration-pipeline-audit.md) — worker behavior, measured growth, limits, and rebuild gates.
- [Bob storage integration audit](../../docs/bob-geometry-storage-integration-audit.md) — compact storage, identity, migration, and compatibility findings.
- [Constructor/library dependency audit](../../docs/constructor-library-dependency-audit.md) — current APIs, physical-authority gaps, and replacement-constructor gates.
