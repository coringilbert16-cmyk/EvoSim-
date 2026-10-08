# Repository organization

EvoSim source is organized by responsibility rather than historical file creation order.

## Boundaries

- `src/bob/` — persistent geometry knowledge lookup, generation worker, and geometry viewer/server.
- `src/infrastructure/` — cross-cutting process infrastructure such as persistent library synchronization.
- `geometry_library/` — durable generated geometry knowledge.
- `chemistry_library/` — durable generated chemistry knowledge.
- `ui/` — browser assets.
- `docs/` — human-facing architecture, mechanics, audits, and decisions.

## Migration rule

A file is moved only after its current `main` version is fetched and its import boundary is understood. During migration, module path attributes may preserve existing `crate::...` API names so the move does not silently become a behavior rewrite.

The organization effort is intentionally incremental. Subsystems are moved one boundary at a time, with obsolete root copies removed after the new path is wired.

## Authority rule

Generated library data is not source code. The persistent library directories are runtime knowledge stores and must remain separate from executable implementation.

## Current Bob boundary

The following files now live under `src/bob/`:

- `library.rs` — persistent geometry reference library and indexed lookup.
- `worker.rs` — durable geometry expansion worker.
- `server.rs` — geometry-library viewer/server.

The existing crate module names are preserved through explicit paths in `src/main.rs` until the remaining source modules can be migrated without creating a broad import rewrite.
