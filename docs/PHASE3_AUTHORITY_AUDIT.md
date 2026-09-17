# Phase 3 — Physical Structure Authority Audit

## Status

Phase 3 implementation branch: `phase3-physical-structure-authority`.

**Implementation status: complete pending merge.** The audit distinguishes logical material descriptions from realized physical material and keeps the organism physical graph authoritative for what physically exists.

## Authority contract

The authoritative representation of an organism's physically existing structure is its physical constituent graph: constituent identity, constituent material, realized placement/geometry, and physical bonds. `Material` describes composition/internal material structure. `PhysicalMaterial` carries an already-realized material across Environment/Storage/COMBINE boundaries. Derived geometry must not become a second physical authority.

## Findings and current state

### 1. `StructuralUnit`

`StructuralUnit` is the graph's constituent record and remains necessary. It stores exactly one physical constituent `Material`, placement, and optional derived `PhysicalGeometry`.

Completed: production construction rejects multi-part or internally bonded material in `StructuralUnit::from_material`, and deserialization rejects composite material as a `StructuralUnit`. Composite physical structure is represented by constituent units and graph bonds.

### 2. `PhysicalMaterial`

`PhysicalMaterial` is used at Environment acquisition, organism Storage, COMBINE, restoration, and decomposition transfer boundaries. It contains composition, intrinsic placements, and resolved internal connection endpoints.

Completed: it is treated as a transfer representation, not organism-owned structure authority. Restoration writes its realized constituents and pre-existing bonds into the organism graph. Decomposition converts released graph constituents back into realized `PhysicalMaterial` objects before returning them to the environment, preserving physical realization instead of reducing it to composition-only `Material` values.

### 3. `MaterialStorage`

Completed: storage uses one `entries` collection with explicit `StoredMaterial::Logical` and `StoredMaterial::Physical` variants instead of parallel logical/physical vectors. Realized entries are prioritized for physical COMBINE selection so a newly acquired physical object cannot be silently replaced by a logical description.

Logical material remains valid for genuinely logical reserves and descriptions. It must not masquerade as an existing physical object.

### 4. `PhysicalMaterialInstance`

Completed: the unused `PhysicalMaterialInstance` wrapper was removed. `MaterialGeometry` and `PlacedMaterialPart` remain derived geometry helpers used by physical structure calculations.

### 5. Environment storage

`FieldCell` intentionally retains two categories because the semantics are different: aggregated logical/unstructured stock in `materials` and existing realized objects in `physical_materials`.

Completed: `ActiveMaterialField::deposit` accepts either logical `Material` or already-realized `PhysicalMaterial` and routes them to the correct representation without reconstructing geometry. Diffusion and legacy logical transfer continue to operate only on logical/unstructured stock.

This dual representation is therefore an intentional semantic distinction, not a second physical authority. No environment-entry unification is required; collapsing the distinction would obscure the logical-vs-realized boundary.

### 6. Derived geometry

`MaterialGeometry`, `PlacedMaterialPart`, `PhysicalGeometry`, connection geometry, and collision helpers remain calculations from physical state. They are not authoritative organism structure.

### 7. Context transforms

`PhysicalMaterial` uses an intrinsic-frame realization across Storage and applies contextual placement during restoration. This preserves intrinsic physical identity while allowing environment/organism placement to differ.

### 8. Organism membership and decomposition

The physical graph remains the source for realized constituent identity and bonds. BREAK removes a bond immediately. Decomposition operates on the graph and, when the final bonds are gone, exports the remaining graph constituents as realized physical material rather than composition-only values.

Completed: death storage also drains logical and realized stored entries through their respective environment paths, so physically realized stored material is not flattened during death.

### 9. Reproduction

Reproduction uses logical juvenile reserves as construction material and passes those descriptions through the shared construction runtime. This is distinct from an already-realized acquired physical composite: the reproduction reserve is a construction input, not an existing physical object.

## Completed migration order

1. Remove the unused `PhysicalMaterialInstance` wrapper while retaining `MaterialGeometry` helpers.
2. Consolidate `MaterialStorage` into one entry collection with an explicit logical/realized distinction.
3. Migrate COMBINE and restoration to consume storage entries without parallel vectors.
4. Migrate decomposition release so physically existing material returns to Environment as realized physical material.
5. Prevent composite material from masquerading as one `StructuralUnit` in construction and deserialization.
6. Re-audit Environment's logical/realized boundary and retain physical realization through transfers.
7. Close the death/storage boundary so realized stored material remains realized during recycling.

## Verification

- Full Rust test suite: **185 passed, 0 failed** after the authority migrations.
- Composite `StructuralUnit` regression is passing.
- Decomposition → Environment contract test is present and verifies released constituents remain realized physical material.
- Environment logical-vs-realized storage behavior remains covered by existing environment and acquisition tests.
- Repository workflow directory contains only the normal `rust.yml`; temporary migration workflows were removed.
- Formatting was run as part of the verified migration passes.
- Clippy warnings remain as pre-existing repository debt and are not being suppressed or reclassified as Phase 3 authority failures.

## Phase 3 conclusion

The remaining Environment dual-category representation is intentional and semantically required: logical aggregate stock and existing realized physical objects are different states. The organism physical graph is the sole authority for organism-owned physical structure; `PhysicalMaterial` is a transfer carrier; geometry is derived; logical material cannot masquerade as an existing physical object; and death/recycling preserves realized material.

Phase 3 is therefore **complete pending merge**. No lifecycle, chemistry/resource semantics, energy rules, movement, perception, blueprint redesign, or new biological rules were introduced by this phase.
