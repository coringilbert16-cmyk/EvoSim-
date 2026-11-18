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


### 5. Environmental resonance repeatedly derives aggregate properties

A closer inspection of src/perception/harmonics.rs found a more consequential candidate call path than the world-tone snapshot's direct physical-property mapping. environmental_material_resonance validates a PhysicalMaterial, derives its centroid from current placements, recomputes ResourceBaselines::from_catalog, and calls Material::weighted_properties before building the emitted spectrum. environmental_contribution then calls environmental_material_resonance and independently recomputes ResourceBaselines::from_catalog and Material::mass for the same source material.

This work is not all reusable: centroid/extent depend on current placements, and the emitted spectrum phase depends on the current tick. But the aggregate ResourceProperties and mass are composition/catalog-derived, while ResourceBaselines are catalog-derived. Those inputs are candidates for reuse if this path is exercised frequently.

Important boundary: the source inspection establishes repeated derivation within one environmental-contribution call, but does not yet establish how often the function is invoked in the standard simulation workload. Its call frequency and inclusive time must be measured before prioritizing it.

This finding strengthens the case for a narrowly scoped profiling pass, not for immediate caching. A safe first prototype could prepare catalog baselines once per owning simulation pass and pass them down, then compare exact output and whole-run throughput. A material aggregate cache would require a proven composition/catalog key and mutation invalidation first.


## Measurement implementation

An opt-in profiler is now added on this PR branch. Set EVOSIM_PROFILE_PROPERTIES=1 to enable it; otherwise it performs no clock reads and emits no report. It records call counts and inclusive elapsed time for:

- ResourceBaselines::from_catalog;
- Material::is_valid_for_catalog;
- Material::weighted_properties;
- Material::mass;
- StructuralUnit::properties;
- world-tone physical-material property extraction;
- world-tone structure-property extraction.

The measurements are deliberately labeled **inclusive**. These paths nest (for example, StructuralUnit::properties calls catalog validation and weighted_properties, which validates again), so their times must not be summed as if they were disjoint. The call counts expose duplicated validation directly.

The PR's Rust workflow now runs one standard 60-second, seed-42 environmental chemistry experiment with profiling enabled, after ordinary CI and the existing smoke run. The log and raw diagnostic JSONL are uploaded as the material-property-profile artifact. The simulation code only reads the environment flag and updates diagnostic atomics/timers; no simulation inputs, branch decisions, candidate ordering, or energy calculations are changed.

The next decision will use both call counts and elapsed time. If the profiled functions account for negligible wall time, the cache workstream should close without a production cache. If a single reusable value dominates, prototype only that value with exact output parity and a matched end-to-end comparison.

## Validation-call audit — 2026-11-18

### What the aggregate profiler count does and does not tell us

The standard profiled run recorded **2,242,584 calls / 1,739.047 ms inclusive** in `Material::is_valid_for_catalog`. This is a real and significant repeated-work signal, but the current profiler records one aggregate counter inside the method. It does **not** attribute those calls to individual callers. We must not claim that all 2.24 million are redundant or assign them to a particular subsystem without call-site instrumentation.

The other counters expose some nested duplication:

- `Material::mass`: 666,423 calls. Each call invokes catalog validation.
- `Material::weighted_properties`: 299,196 calls. Each call invokes catalog validation.
- `StructuralUnit::properties`: 73,952 calls. It first invokes catalog validation, then calls `weighted_properties`, which validates the same material again. This path therefore performs two catalog validations per successful property calculation (147,904 validation invocations represented by those calls), absent early failure.
- `ResourceBaselines::from_catalog`: 223,282 calls. This is catalog-wide aggregation, not material validation.

These are nested counts, not disjoint work. In particular, the 2,242,584 validation count already includes the validation calls made by mass, weighted properties, and structural-unit properties.

### Confirmed source-level validation call sites

The branch source audit found validation at several different semantic boundaries:

1. **Property derivation:** `Material::mass`, `Material::weighted_properties`, `Material::potential_energy`, and `StructuralUnit::properties`. The structural-unit path explicitly validates and then calls a method that validates again. This is the clearest provable duplicate validation.
2. **Per-tick environmental movement selection:** `simulation/environment_movement.rs` validates each realized environmental material while building the movement input list.
3. **Environmental broad-phase snapshot construction:** `chemistry/environmental_broad_phase.rs` validates indexed materials before restoring geometry for bounds.
4. **Environmental field movement/snapshot and transfer boundaries:** `simulation/environment.rs` validates materials while building movement snapshots, checking moves/splits, and accepting deposits. These checks protect state mutation and ownership boundaries; they cannot simply be removed because another earlier scan happened to validate the object.
5. **Material storage operations:** `materials/material_storage.rs` validates during store, batch preparation, selection, and removal. Some iterator predicates validate candidates and then delegate to a method that validates the selected object again. This is a second likely duplicate pattern worth measuring at call-site level.
6. **COMBINE/displacement and restoration:** reaction source/product construction and material restoration validate inputs and outputs at transaction boundaries. These checks are correctness guards and should remain unless an authoritative validated-state contract spans the entire transaction.

The code search also finds validation in expulsion, transfer, physical-material construction, and transformation code. Their frequency in the standard run is not yet known.

### Why repeated validation is expensive

`Material::is_valid_for_catalog` currently performs all of the following on each call:

- `Material::is_valid`: checks nonempty names, integral positive constituent amounts, representable per-resource totals, valid bond endpoints, and duplicate/reversed bonds.
- Scans the full catalog to reject empty or duplicate resource names.
- For every material part, scans the catalog and counts matching resource names, requiring exactly one match.

The checks are intentionally fail-closed. Any optimization must preserve duplicate catalog-name rejection, unknown-name rejection, integer/count overflow protection, ordered constituent identity, and bond-graph validity. A cache keyed only by a material's object identity would be insufficient because `Material` and its vectors are mutable and equality includes both ordered parts and internal bonds.

### Audit conclusion and next measurement

The audit has identified one definite local duplication in `StructuralUnit::properties`, plus likely repeated checks in environmental movement and storage selection. However, the existing aggregate profiler cannot tell us which call sites account for the remaining validation volume.

**Next justified step:** add temporary, opt-in call-site counters around the major validation sites (movement input selection, broad-phase snapshot, environment snapshot/transfer, storage selection, and structural-unit derivation), then run the same standard 60-second seed-42 workload. Keep the existing aggregate validation counter as a reconciliation total. This will identify the dominant sources without changing validation semantics.

Only after that attribution should we remove a proven same-scope duplicate or design a validated-material/cache boundary. Do not cache successful validation globally yet: there is no authoritative composition/catalog revision covering every mutation path, and transaction-boundary checks serve distinct correctness purposes.
