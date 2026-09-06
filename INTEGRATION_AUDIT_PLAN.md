# EvoSim Integration Audit & Large-File Truncation Plan

## Current status

This document is the **current integration-status source of truth**. Phase-completion files such as `PHASE6_COMPLETE.md` are historical records and must not be used as the current project-status authority.

**Phase 1 — executable runtime split: COMPLETE.**

**Phase 2 — environment split: COMPLETE.**

**Phase 3 — resources/chemistry split: COMPLETE.**

**Phase 4 — structural integration: COMPLETE.**

**Phase 5 — runtime decision integration: COMPLETE.**

**Phase 6 — COMBINE integration: HISTORICAL COMPLETION RECORDED; CURRENT INTEGRATION CONTINUES.**

Substantial work after the original Phase 6 completion changed the physical material, structure, geometry, boundary, and interface architecture. Therefore the old Phase 6 checklist is retained below as historical context, while the current code and the integration audits determine present implementation status.

### Current architectural priorities

1. Finish **Material → Organism Structure** authority so organism structural units are represented by actual `Material` composition + internal structure rather than a parallel resource-name representation.
2. Connect **environment → organism** through real physical acquisition.
3. Make **development/growth** consume and transform physical organism material.
4. Complete **reproduction → new organism → juvenile → adult** lifecycle integration.
5. Close the **evolution/selection** loop once viable offspring and lifecycle selection are real.

These priorities are architectural integration work, not a request to revive historical Phase 6 design decisions.

## Objective

Make the repository internally coherent before further feature work, while restructuring large source files so future GitHub/AI file retrieval cannot truncate the architectural context needed for safe edits.

This is an integration-first audit. No simulation rule is changed merely to make the code easier to split.

## Locked rules that refactoring and integration must preserve

- Energy is emergent from resource interactions, not a fundamental environmental resource.
- COMBINE interaction value: potential energy establishes energetic direction; reactivity and geometry modify magnitude.
- Surplus investment maps to bond strength through a capped diminishing-returns curve.
- COMBINE consumes energy; formation deficits may require organism payment.
- BREAK depends on current structural/chemical state and may consume or release energy.
- Natural chemistry: compatible resources may combine; compatibility is not a hardcoded finite recipe list.
- Vents are indiscriminate transfer mechanisms; they do not prefer bonded over unbonded material.
- Layer 1 is the spatial deep reservoir; Layer 2 is the active field where organisms live and material/waste interact.
- Waste remains material in Layer 2 and is not automatically deleted or teleported to the reservoir.
- Physical structure is discrete structural units plus explicit bonds; bulk bonded material must not substitute for real structure.
- Bond energy is structural state and must not be reconstructed from raw resource potential energy during BREAK.
- Decision history is bounded learned consequence history, not a physics cache and not fabricated prediction.
- Mechanical eligibility belongs to physical systems; decision logic chooses among mechanically eligible candidates.

## Historical Phase 6 implementation record

The following records what the original Phase 6 work established. It is preserved for historical context and is **not** the current project-status checklist.

### Recorded Phase 6 work

1. Added `combine_runtime.rs` as the runtime boundary between decision execution and COMBINE physics.
2. Added raw-material-to-structural-unit instantiation without introducing a second bulk bonded representation.
3. Routed structural-pair evaluation through the existing contact geometry and formation-threshold pipeline.
4. Reused the existing experimental interaction equation so potential-energy direction, reactivity, water attenuation, facing, and distance remain in one place.
5. Reused the existing experimental work-cost and capped diminishing-return bond-strength functions.
6. Added explicit organism energy payment at the runtime boundary rather than creating free energy.

### Historical Phase 6 remaining-work list

This list belonged to the earlier phase state and is retained only to preserve the historical reasoning trail. Subsequent architecture work superseded parts of it.

1. Connect ACQUIRE to actual field-to-organism raw-material transfer.
2. Connect physical instantiation to the decision/runtime path without inventing a hidden automatic construction loop.
3. Make COMBINE mechanically eligible only when an actual eligible structural pair exists.
4. Add COMBINE as a decision candidate with stable material context.
5. Record actual COMBINE outcomes in bounded decision history.
6. Resolve the structural bond-energy representation so BREAK consumes/releases stored bond energy rather than raw resource potential energy.
7. Add integration tests covering raw-material conservation, instantiation, geometry gating, threshold failure, successful bond formation, energy payment, and decision-history outcome recording.
8. Verify deterministic seeded behavior and full Rust CI before marking Phase 6 complete.

## Later integration phases

### Phase 7 — Frontend integration

Restore/verify the actual frontend file tree referenced by `main.tsx`. Establish a minimal build that connects to `/snapshot` and `/ws`.

### Phase 8 — Cross-system invariants

Add integration tests for material conservation, vent transfer, diffusion, settling, acquisition, COMBINE, bond formation, BREAK energy accounting, decision approval, snapshot serialization, WebSocket delivery, and deterministic seeded behavior.

### Phase 9 — Comment/spec normalization

Normalize comments in every touched file so they describe the current architecture rather than historical decisions.

### Phase 10 — Final integration gate

Run formatting, checks, tests, frontend build/typecheck, repository-wide obsolete-path searches, and the large-file/truncation audit.

## Large-file rule going forward

No new core simulation file should be allowed to grow into another monolith. Split when a file contains multiple independently testable responsibilities, especially when it approaches ~20–25 KB or when retrieval of the full file is no longer reliable.
