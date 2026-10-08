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

The concrete record/index layout will be added when the chemistry persistence boundary is wired; this establishes ownership without inventing storage schema prematurely.
