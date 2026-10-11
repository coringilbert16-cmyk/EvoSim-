# Bob — Geometry Reference Library

> **Current verification checkpoint — 2026-10-11:** The persistent Windows runner now uses `cargo build --locked` and `cargo test --locked`, and the stale exception for an untracked `Cargo.lock` was removed. See the root README and repository stabilization audit for current CI evidence.

Bob stores reusable geometry knowledge for EvoSim. It is not live organism state and does not replace the physical validator.

## Responsibilities

- `src/bob/library.rs`: persistent formations, contact-family records, canonical identities, storage schemas, and indexed read paths.
- `src/bob/worker.rs`: discovers formations and contact families, persists results, and reports generation metrics.
- `src/bob/server.rs`: read-only geometry-library viewer.
- `src/construction/initial_organism_constructor.rs`: the active blueprint-free initial-organism path now queries Bob for advisory rigid-edge placement candidates, which still undergo live physical and COMBINE validation. The path remains transitional: Bob integration has begun, but the approved milestone-driven, free-form replacement and viable-organism acceptance gate are not yet complete.

A library result is reusable knowledge or a candidate, not permission to mutate a live organism. Live placement must still obey authoritative geometry, contact, nonpenetration, chemistry, bonding, energy, and ledger rules.

## Library location

Set `EVOSIM_GEOMETRY_LIBRARY_DIR` to choose the persistent library root. If unset, Bob uses `geometry_library/data` relative to the process working directory. An explicit override must point to a valid library; Bob must not silently replace a missing explicitly configured library with a new empty one.

The previous generated geometry catalogue was intentionally removed from both current branch heads. Do not restore that obsolete data. Preserve the source definitions, generator code, schemas, validation rules, and regeneration commands. Generate replacement data only after constructor-facing query coverage and correctness gates are validated; generated output remains a workflow artifact or local runtime state unless explicitly approved for tracking.

## Storage behavior

- Fresh empty roots use the compositional-v3 formation format, with deterministic compact formation IDs and exact one-constituent deltas where the delta encoding is smaller.
- Existing legacy roots remain readable; the on-disk encoding must not change canonical runtime identity or lookup results.
- Family rows can use compact formation references on disk while runtime signatures remain canonical.
- Frontier updates are batched at the end of a formation pass. If a pass is interrupted before checkpointing, unfinished work can be retried through idempotent insertion.
- Water contact families are generated once per formation/resource pass and reused to derive fluid-boundary records.

### Water-inclusive formation coverage — current gap

**Implementation status — first pass added, verification pending:** the worker now derives `GeometryFluidFormation` records from solved fluid-boundary families and persists them in `fluid_formations.jsonl`. These are explicit fluid-inclusive arrangement records, separate from rigid `GeometryFormation` rows because a fluid constituent is defined by conserved area and a solved boundary, not a fixed rigid pose. Each record retains its rigid parent, fluid resource, anchor interface, area, contact angle, curvature radius, free-arc angle, contact length, and contact interval. Canonical signatures deduplicate equivalent records, and the worker reports generated versus newly inserted fluid formations.

This is an initial sidecar representation, **not yet proof of complete fluid geometry coverage or constructor readiness**. Its tests and exact-head CI must pass before the library is regenerated broadly.

The generator must produce records only where the fluid solution and available exposed boundary are valid; it must report unsupported boundary classes explicitly rather than silently implying full coverage. Water–Water accumulation is not a two-constituent rigid formation and must remain excluded. Runtime construction still validates the actual live fluid shape and whole-structure interactions; Bob records reusable knowledge and does not authorize a physical mutation.

Acceptance gates for this work:

1. Generate a Water-inclusive record from a real eligible rigid formation and verify that Water is represented explicitly in the record.
2. Persist, close, reopen, and resolve that record with identical canonical identity and geometry.
3. Verify conserved area and capillary self-consistency, exposed-boundary interval bounds, valid parent references, and duplicate rejection.
4. Cover each supported fluid/solid boundary class and report unsupported classes in metrics.
5. Prove the worker resumes idempotently after interruption and that the existing rigid-only catalogue behavior remains unchanged.

Do not run broad catalogue regeneration until these gates pass. Existing `fluid_boundary_families.jsonl` rows remain useful capillary knowledge, but they are not substitutes for the new fluid-inclusive formation records.
- The worker reports generated and newly inserted family counts separately. Equal counts indicate no duplicate keys were rejected; they do not prove all records are geometrically unique.

## Worker commands

Run these commands from the repository root after building the executable:

```powershell
# Process one formation/resource frontier pass
.\target\release\evosim.exe --geometry-worker-once

# Process at most N passes in one process
.\target\release\evosim.exe --geometry-worker-passes 25

# Run continuously; the worker idles when no unfinished work is found
.\target\release\evosim.exe --geometry-worker
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

At code head `87bda9e6f9b8670ca2885edc2c15d7c257e1ecc3`, [Rust workflow #38141026450](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38141026450) completed with failure. Formatting, source-size policy, COMBINE architecture checks, Bob compact-storage tests, and the non-generative empty-library open/reopen test passed. The full suite again reported **314 passed, 29 failed, 1 ignored**. The recurring cross-subsystem seed-realization failures and other contract failures are recorded below; strict Clippy failed with 48 test-target and 58 application-target diagnostics. [Runner validation #38141026451](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/38141026451) passed. This does not prove the full Bob query path or constructor is biologically viable.

## Audit documents

- [Bob regeneration pipeline audit](../../docs/bob-regeneration-pipeline-audit.md) — worker behavior, measured growth, limits, and rebuild gates.
- [Bob storage integration audit](../../docs/bob-geometry-storage-integration-audit.md) — compact storage, identity, migration, and compatibility findings.
- [Constructor/library dependency audit](../../docs/constructor-library-dependency-audit.md) — current APIs, physical-authority gaps, and replacement-constructor gates.
