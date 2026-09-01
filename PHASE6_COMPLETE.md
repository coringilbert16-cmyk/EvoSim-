# Phase 6

Phase 6 is complete and verified for structural bond representation, chemistry integration, resource conservation, connection-point structure, runtime decision integration, and dead-code cleanup.

- Bond strength and bond energy are separate fields.
- Bond energy is explicitly serialized and validated.
- BREAK releases the exact stored structural bond energy.
- COMBINE is gated by physical connection-point contact distance.
- Resource transfer preserves material across field, reservoir, vents, and settling.
- Structural connection sites and connected components are derived from current structure state.
- COMBINE is integrated through the decision layer and runtime executor.
- The orphaned `lib.rs` target was removed; obsolete `structure_core.rs` and `energy_content` references are absent.
- CI formatting, tests, and Clippy checks pass on the verified Phase 6 integration state.

Phase 6 is closed. Subsequent work should proceed to Phase 7 only after the Phase 6 verification state remains intact.