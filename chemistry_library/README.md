# Chemistry Library

This directory is the persistent chemistry knowledge store.

It is intentionally separate from src/chemistry/, which contains the executable chemistry implementation.

Boundary:
- src/chemistry/ — equations, reactions, combination/break/decomposition, transformation, chemistry keys, and runtime policy.
- chemistry_library/ — durable discovered chemistry records and indexes.
- src/bob/ — geometry knowledge lookup and generation.

Persistence rules:
- Records use canonical keys.
- Equivalent records deduplicate.
- Generated knowledge survives process restart.
- Tests use isolated temporary stores.
- Library data never becomes a second source of simulation rules.

Current tracked seed/cache data: `data/chemistry.jsonl` and `data/manifest.json` contain 17 schema-3 interface records. The source loader uses schema 3 as current and retains an explicit schema-2 compatibility reader. The loader rejects unknown schemas, malformed or semantically invalid records, duplicate keys, and manifest-count mismatches. These rows are static interface-cache records, not a complete chemistry ruleset; live chemistry equations and transitions remain authoritative in `src/chemistry/`. Runtime hit/miss coverage and cache usefulness must be measured before adding more records.
