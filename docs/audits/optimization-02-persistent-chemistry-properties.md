# Optimization 02 — persistent chemistry/material-property cache audit

**Status:** source audit in progress; no production code changed.

## Executive finding

There are real repeated calculations, but the source audit does **not yet justify adding a general persistent material cache**. The clearest repeated work is property validation/aggregation for organism structural units and repeated catalog-baseline derivation. Environmental physical-material property extraction is already a direct catalog lookup and copy, not a weighted chemistry calculation. A cache must not be added until a standard-run profile establishes that these operations are material contributors rather than negligible work next to geometry and resonance calculations.

## Source-backed call-path observations

### 1. World-tone receiver snapshot

src/simulation/world_tone_snapshot.rs rebuilds the world-wide body/property snapshot inside WorldToneOccluderSnapshot::from_simulation_for_organisms. The live settlement path calls this constructor before processing the frame. It visits organism structures, committed reproductive-construction material, environmental physical materials, legacy decomposing structures, organism stored material, and material held by active transformations.

The snapshot builds body_properties anew on each invocation. Organism structure properties flow through structure_material_properties, which calls StructuralUnit::properties(catalog) for every unit. Physical environmental/stored materials flow through physical_material_properties, which resolves each constituent name against the catalog and copies the immutable catalog ResourceProperties value.

The snapshot also currently clones each per-body property vector during the final body remap (body_properties[original_index].clone()). This is allocation/copy overhead, not chemistry derivation. It is noted here because it sits directly in the same path, but it is not itself a reason to add a chemistry cache.

### 2. Structural-unit property calculation

src/geometry/structure.rs::StructuralUnit::properties first calls Material::is_valid_for_catalog, then calls Material::weighted_properties.

src/materials/resources.rs::Material::weighted_properties validates the catalog-bound material again. It then iterates the material parts and performs a catalog name lookup for each part while calculating weighted mass, potential energy, reactivity, and cohesion. Mixed-material chemical position is deliberately not averaged; it remains absent so constituent identities are not erased.

For a common one-part structural unit, the weighted aggregation is mathematically just the single catalog property's values. However, the existing validation path still checks catalog-name uniqueness and material-name resolution. This is a plausible optimization target, but changing the validation/aggregation route must preserve fail-closed behavior for duplicate catalog names, unknown resources, invalid counts, and malformed material composition.

### 3. Environmental material properties

physical_material_properties does not call weighted_properties; it maps ordered material parts to catalog properties. That path is already relatively direct. It preserves part order and returns properties indexed by the geometry constituent index.

Do not replace this with a cache keyed only by a sorted/aggregated composition: constituent order is part of physical identity and geometry indexing.

### 4. Catalog baselines and resonance calculations

ResourceBaselines::from_catalog is recomputed in the world-tone settlement path and at several harmonics/reaction call sites. It is a catalog-wide average over mass, potential energy, reactivity, and cohesion. This calculation is independent of material placement and simulation tick as long as the catalog is unchanged.

The harmonics path also recomputes material-derived values such as weighted properties and mass in environmental resonance calculations. The actual frequency, damping, and response spectra are not all cacheable: spectra and phases can depend on the current tick, incoming signal, and spatial interaction. Only the catalog/material-derived inputs should be considered for reuse.

## Dependency and invalidation assessment

| Value | True dependencies | Safe invalidation boundary |
|---|---|---|
| Catalog ResourceProperties lookup | Catalog contents and resource identity/name | Catalog replacement or mutation |
| ResourceBaselines | Complete catalog property values | Any catalog property or membership change |
| Material::weighted_properties | Ordered material parts, amounts, catalog property values | Composition or catalog change; not placement-only movement |
| Material mass | Material parts/amounts and catalog mass values | Composition or catalog change |
| Physical constituent geometry | Realized shape, placements/orientation, catalog default shape | Shape/placement/orientation changes |
| Bond-dependent descriptors | Ordered constituent identity and bond topology | Bond/topology change |
| Resonance spectra/response | Material properties plus tick/phase, input spectrum, and relevant spatial context | Per invocation/tick/input; do not persist as composition-only data |

### Existing revision infrastructure: important limitation

The environmental interaction contract in src/chemistry/environmental_interaction.rs defines stable MaterialIdentity, topology_generation, and state_version, and explicitly treats movement as a locator/state update rather than an identity change. However, the module is explicitly staged and marked not wired into production. The broad-phase snapshot currently initializes topology generation and state version to zero in its adapter. These fields therefore cannot yet be assumed to provide production-wide cache invalidation.

PhysicalMaterial::environmental_identity is stable across transport, but it is not by itself a composition or topology revision. Organism StructuralUnit has stable physical constituent IDs, but this audit has not found a single authoritative structure/catalog revision that covers all mutation paths. A persistent cache keyed only by object identity would risk stale values after composition or bond changes.

## What is justified now

1. Profile the counts and inclusive/exclusive time for StructuralUnit::properties and Material::weighted_properties; Material::is_valid_for_catalog; ResourceBaselines::from_catalog; physical_material_properties; and snapshot property-vector allocation/copy.
2. Measure under the standard 60-second, seed-42, 1,280-object / 1,360-constituent workload. Keep instrumentation out of normal production builds or feature-gate it.
3. Include a no-cache reference path and exact-output comparisons if any cache prototype is introduced.
4. Verify catalog mutation/ownership and all material/topology mutation paths before choosing cache keys. Do not treat the staged environmental interaction versions as live invalidation until their production integration is proven.
5. Only prototype the smallest measured win. A catalog-baseline cache may be simpler and safer than a general material cache, but its benefit must still be measured end-to-end.

## Explicit non-goals

- No caching of placement, orientation, current geometry, tick-dependent phase, incoming signal, or world exposure.
- No sorting/aggregating constituent identities to form a cache key.
- No skipping validation on cache hits unless the cache's authoritative dependency/version contract proves that validation remains equivalent.
- No parallel revision counter, global mutable cache, cross-simulation shared state, approximate values, or chemistry-specific reaction rules.

## Current decision

**Audit-first; no production cache implementation yet.** The code contains reusable inputs, but the largest candidate cost has not been quantified and production-wide invalidation signals are not yet established. Implementing a general persistent cache now would add stale-state risk without demonstrated end-to-end benefit.
