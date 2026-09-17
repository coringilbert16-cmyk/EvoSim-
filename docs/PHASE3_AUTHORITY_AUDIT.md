# Phase 3 — Physical Structure Authority Audit

## Status

Phase 3 implementation branch: `phase3-physical-structure-authority`.

This audit is deliberately conservative. It distinguishes logical material descriptions from realized physical material and keeps the organism physical graph authoritative for what physically exists.

## Authority contract

The authoritative representation of an organism's physically existing structure is its physical constituent graph: constituent identity, constituent material, realized placement/geometry, and physical bonds. `Material` describes composition/internal material structure. `PhysicalMaterial` carries an already-realized material across Environment/Storage/COMBINE boundaries. Derived geometry must not become a second physical authority.

## Findings and current state

### 1. `StructuralUnit`

`StructuralUnit` is the graph's constituent record and remains necessary. It stores a single constituent `Material`, placement, and optional derived `PhysicalGeometry`.

Completed migration: production construction now rejects structured composite `Material` in `StructuralUnit::from_material`, and deserialization rejects composite material as a `StructuralUnit`. Composite physical structure must instead be represented by constituent units and graph bonds.

### 2. `PhysicalMaterial`

`PhysicalMaterial` is used at Environment acquisition, organism Storage, COMBINE, restoration, and decomposition transfer boundaries. It contains composition, intrinsic placements, and resolved internal connection endpoints.

Current rule: it is a transfer representation, not organism-owned structure authority. Restoration writes its realized constituents and pre-existing bonds into the organism graph. Decomposition now converts released graph constituents back into realized `PhysicalMaterial` objects before returning them to the environment, preserving physical realization instead of reducing them to composition-only `Material` values.

### 3. `MaterialStorage`

Completed migration: storage now uses one `entries` collection with explicit `StoredMaterial::Logical` and `StoredMaterial::Physical` variants instead of parallel logical/physical vectors. Realized entries are prioritized for physical COMBINE selection so a newly acquired physical object cannot be silently replaced by a logical description.

Logical material remains valid for genuinely logical reserves and descriptions. It must not masquerade as an existing physical object.

### 4. `PhysicalMaterialInstance`

Completed migration: the unused `PhysicalMaterialInstance` wrapper was removed. `MaterialGeometry` and `PlacedMaterialPart` remain derived geometry helpers used by physical structure calculations.

### 5. Environment storage

`FieldCell` intentionally retains two categories because the semantics are different: aggregated logical/unstructured stock in `materials` and existing realized objects in `physical_materials`.

Completed boundary work: `ActiveMaterialField::deposit` now accepts either logical `Material` or already-realized `PhysicalMaterial` and routes them to the correct representation without reconstructing geometry. Decomposition therefore returns realized physical objects directly to the physical environment path.

Remaining migration: unify these categories behind a single explicit environment-entry abstraction only if doing so reduces representation duplication without obscuring the important logical-vs-realized distinction. Diffusion and legacy logical transfer must not operate on realized objects.

### 6. Derived geometry

`MaterialGeometry`, `PlacedMaterialPart`, `PhysicalGeometry`, connection geometry, and collision helpers remain calculations from physical state. They are not authoritative organism structure.

### 7. Context transforms

`PhysicalMaterial` uses an intrinsic-frame realization across Storage and applies contextual placement during restoration. This preserves intrinsic physical identity while allowing environment/organism placement to differ.

### 8. Organism membership and decomposition

The physical graph remains the source for realized constituent identity and bonds. BREAK removes a bond immediately. Decomposition operates on the graph and, when the final bonds are gone, exports the remaining graph constituents as realized physical material rather than composition-only values.

### 9. Reproduction

Reproduction currently uses logical juvenile reserves as construction material and passes those descriptions through the shared construction runtime. This is distinct from an already-realized acquired physical composite: the reproduction reserve is a construction input, not an existing physical object. No new physical-authority rule is introduced here.

## Completed migration order

1. Remove the unused `PhysicalMaterialInstance` wrapper while retaining `MaterialGeometry` helpers.
2. Consolidate `MaterialStorage` into one entry collection with an explicit logical/realized distinction.
3. Migrate COMBINE and restoration to consume storage entries without parallel vectors.
4. Migrate decomposition release so physically existing material returns to Environment as realized physical material.
5. Prevent composite material from masquerading as one `StructuralUnit` in construction and deserialization.
6. Re-audit Environment's logical/realized boundary and retain physical realization through transfers.

## Remaining Phase 3 work

1. Add/verify focused contract coverage for decomposition → Environment physical transfer and the logical/realized Environment boundary.
2. Re-run the whole-repository authority search after the final migration changes.
3. Verify formatting and the full test suite on the resulting branch; report any pre-existing Clippy debt separately.
4. Close any remaining duplicate physical-authority path discovered by that final audit before merging Phase 3.

## Explicit non-goals

No lifecycle mechanics, chemistry/resource semantics, energy rules, movement, perception, blueprint redesign, or new biological rules are part of this audit.
