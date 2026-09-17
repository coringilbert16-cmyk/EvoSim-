# Phase 3 — Physical Structure Authority Audit

## Status

Phase 3 implementation branch: `phase3-physical-structure-authority`.

This audit is deliberately conservative. It identifies remaining representations and their migration boundaries before deleting transitional state.

## Authority contract

The authoritative representation of an organism's physically existing structure is its physical constituent graph: constituent identity, constituent material, realized placement/geometry, and physical bonds. `Material` describes composition/internal material structure. `PhysicalMaterial` carries an already-realized material across Environment/Storage/COMBINE boundaries. Derived geometry must not become a second physical authority.

## Findings

### 1. `StructuralUnit`

`StructuralUnit` is the graph's constituent record and therefore remains necessary. It currently stores `Material`, placement, and optional derived `PhysicalGeometry`.

Remaining authority leak: `StructuralUnit::from_material` accepts a structured composite `Material`. A composite can therefore still be represented as one graph unit even though the intended authority model is one physical constituent per graph unit with relationships represented by graph bonds.

Migration: retain the constructor during transition, audit all callers, then replace production composite construction with constituent-level graph construction. Do not silently reject it until all active callers have migrated.

### 2. `PhysicalMaterial`

`PhysicalMaterial` is used by Environment acquisition, organism Storage, and restoration. It contains composition, intrinsic placements, and resolved internal connection endpoints. This is currently the correct transfer representation for an already-realized composite, but it must not become a second long-lived organism-structure authority after restoration.

Migration: keep it at transfer boundaries until Storage and restoration can hand the physical realization directly into the graph without retaining a parallel organism-owned copy.

### 3. `MaterialStorage`

Storage currently maintains two aligned vectors: `materials` and optional `physical_instances`. This is the clearest remaining parallel representation. The index alignment is semantic state and can allow logical and realized versions of the same material to coexist as separate representations.

Migration: replace the paired vectors with one explicit storage-entry representation that distinguishes logical material from realized physical material. Preserve existing public behavior while migrating COMBINE, reproduction, decomposition, and tests. Do not drop realized state during migration.

### 4. `PhysicalMaterialInstance`

`material_geometry.rs` still defines `PhysicalMaterialInstance`, a second physical-material wrapper containing `MaterialGeometry`. Repository search found no production consumer outside its own definition/test. `MaterialGeometry` and `PlacedMaterialPart` are still used as derived geometry helpers by structure collision/overlap code.

Migration: remove the unused `PhysicalMaterialInstance` wrapper and its test; retain the geometry algorithms themselves. This eliminates a redundant physical-material abstraction without removing active geometry functionality.

### 5. Environment storage

`FieldCell` currently contains both logical `materials` and realized `physical_materials`. These are intentionally different during the migration, but together form another parallel storage representation.

Migration: introduce a single environment-material entry abstraction only after all logical-material-only operations (diffusion/legacy transfer) and realized-material operations (acquisition/restoration) have explicit handling. Existing physical realization must never be reconstructed from composition alone.

### 6. Derived geometry

`MaterialGeometry`, `PlacedMaterialPart`, `PhysicalGeometry`, connection geometry, and collision helpers are calculations from physical state. They must remain derived/cache-like information. No migration should make one of these structures authoritative over the graph.

### 7. Context transforms

`PhysicalMaterial` already uses an intrinsic-frame realization before Storage and applies world placement during restoration. This preserves intrinsic identity while allowing context-specific placement. This rule must remain invariant during Storage consolidation.

### 8. Organism membership

The physical graph is the source for realized constituent identity and bonds. Membership must ultimately be computed from physical connectivity to the genome. Any lifecycle/decomposition code that treats a stored `Material` or derived geometry object as organism structure is a migration target.

## Safe migration order

1. Remove the unused `PhysicalMaterialInstance` wrapper while retaining `MaterialGeometry` helpers.
2. Consolidate `MaterialStorage` into one entry collection with an explicit logical/realized distinction.
3. Migrate COMBINE and restoration to consume storage entries without parallel vectors.
4. Migrate reproduction/decomposition consumers of storage.
5. Consolidate Environment logical/realized material storage.
6. Remove composite-as-one-`StructuralUnit` production paths.
7. Re-run whole-repository authority search and contract tests.

## Explicit non-goals

No lifecycle mechanics, chemistry/resource semantics, energy rules, movement, perception, blueprint redesign, or new biological rules are part of this audit.
