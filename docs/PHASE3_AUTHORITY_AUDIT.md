# Phase 3 — Physical Structure Authority Audit

## Status

Phase 3 begins with an audit-only boundary pass. No physical authority is removed or rewritten until each active consumer has a migration path.

## Frozen authority

The organism's realized physical graph is the authoritative representation of what physically exists inside the organism. `Material` describes composition/internal relationships; a physically existing material requires a complete realization. Geometry calculations are derived from physical state and must not become a competing source of truth.

## Findings

### 1. `PhysicalMaterial`

`PhysicalMaterial` is an appropriate transfer/identity wrapper for a physically realized material crossing Environment → Storage → COMBINE. Its intrinsic realization (relative placements plus pre-existing connection endpoints) must remain intact until restoration into the organism graph.

**Migration status:** retain as a transfer representation; do not treat it as an independent organism-owned structure after restoration.

### 2. `MaterialStorage::physical_instances`

Storage currently maintains parallel `materials` and optional `physical_instances` vectors. This is transitional state and is the clearest remaining duplicate representation at the Storage boundary.

The Phase 2 runtime depends on this alignment and on the newly acquired physical entry being selected first. Removing it immediately would risk silently routing realized material through the logical `Material` path.

**Migration status:** retain temporarily; next migration should replace parallel optional state with an explicitly typed storage entry that cannot silently fall back from realized to logical representation.

### 3. `PhysicalMaterialInstance` in `material_geometry.rs`

A second physical-material representation remains in `material_geometry.rs`. It overlaps conceptually with `PhysicalMaterial` and therefore remains a competing physical-material abstraction.

**Migration status:** transitional only. Before deletion, every consumer must be identified and migrated to derived geometry over canonical physical state. No new consumer should be added.

### 4. `StructuralUnit`

`StructuralUnit` is the constituent node of the physical graph and is therefore valid as a constituent-level representation. However, its `material: Material` field currently permits a structured/composite `Material` to be stored in one unit. That makes a whole composite appear as one graph constituent without carrying the constituent-level realization needed by the physical graph.

The repository already contains `structure_authority::audit_structure`, which reports `StructuredMaterialInUnit` instead of silently rewriting such state.

**Migration status:** composite-as-one-unit is transitional. New construction/restoration paths should create one physical graph constituent per realized constituent and explicit graph bonds for relationships. Existing deserialization compatibility must remain until persisted legacy state has a defined migration path.

### 5. Derived geometry

`PhysicalGeometry`, connection geometry, collision/overlap helpers, and material geometry are derived calculations. They may cache or calculate geometric facts, but they must not define constituent identity, ownership, bond existence, or material composition independently of the physical graph.

**Migration status:** audit consumers before changing or deleting any geometry abstraction.

## Phase 3 first implementation boundary

The first code migration should target the Storage representation because it is the smallest remaining authority duplication with a clear Phase 2 consumer contract:

1. Introduce an explicit storage entry type representing either logical material or a complete realized physical material.
2. Preserve exact intrinsic realization for realized entries.
3. Make logical fallback explicit rather than implicit.
4. Preserve current COMBINE selection behavior during migration.
5. Add conservation/identity regression tests.
6. Only after all consumers migrate, remove `physical_instances` parallel state.

No lifecycle, chemistry, blueprint, movement, energy, or biological rules are changed by this boundary.
