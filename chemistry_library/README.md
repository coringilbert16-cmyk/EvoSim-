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

Current persistence:
- Chemistry records carry an explicit Valid/Rejected state.
- Valid records may cache static chemical potential and bond strength.
- Rejected records retain a stable rejection reason.
- Schema versioning prevents older chemistry knowledge from being silently reused.

Bob's worker may submit newly discovered material-pair chemistry through the chemistry library, but the chemistry implementation remains authoritative. Runtime formation success still depends on the physical candidate, load, and available investment.
