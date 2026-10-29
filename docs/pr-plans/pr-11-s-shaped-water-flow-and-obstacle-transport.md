# Environmental PR — S-shaped Water Flow and Obstacle-Aware Transport

## Purpose

Replace the old environment concept for the experiment with a continuous blue water field and a deterministic, spatially varying current. This PR owns environmental flow and physical transport only. It must not implement or refactor chemistry reactions; PR #225 remains responsible for proving the live COMBINE → BREAK cycle after this transport foundation is ready.

## Approved environmental behavior (user approval, 2026-10-29)

- The field is represented visually as blue water.
- The dominant current travels left to right along a sideways S-shaped path.
- Speed is highest along the S-shaped core and falls gradually as distance from the core increases.
- The bends include small recirculating eddies that feed back toward the main current. These must be represented in the velocity field rather than implemented as arbitrary particle reversals.
- Every resource and compound samples the same local flow field.
- Mass modifies material response, not the environmental velocity itself. Flow-induced mobility decreases linearly with mass and has a strictly positive floor: increasing mass must never make an object immovable solely due to mass.
- Large structures disturb the current. Dense obstacles divert flow and create a sheltered downstream wake. Porous structures permit some flow through them according to permeability.
- Start with a deterministic, one-way geometry-aware approximation. Do not implement full Navier–Stokes, pressure solving, or detailed turbulence in this PR.
- Recompute only affected flow regions when feasible after structure geometry/permeability changes. Do not recompute the entire field every tick without evidence it is necessary.

## Source trace so far

- 'src/simulation/environment.rs' defines 'ActiveMaterialField', its regular cell grid, wrapped vertical coordinates, bounded spatial queries, and 'FieldCell' holders ('materials' and 'physical_materials'). The cell grid is a spatial index; it is not itself a flow model.
- 'FieldCell.physical_materials' contains realized objects with geometry and internal connections. Transport must preserve each complete 'PhysicalMaterial' as a unit; do not split constituents or sever internal bonds during ordinary field movement.
- Environmental COMBINE and BREAK discovery currently operate on realized physical materials. Aggregate 'FieldCell.materials' is unstructured stock and must not be reinterpreted as a composite by this PR.
- 'src/materials/environmental_materials.rs' currently seeds six spatial formations and a 5,000-particle local cloud from 12 precombined material recipes. These old seed helpers should not be silently rewritten as part of implementing flow. The new environment/preset must be explicitly selectable until the new model and the chemistry experiment are validated.
- 'Simulation::step' runs world-tone sampling and environment-owned BREAK/COMBINE discovery/resolution, but the inspected excerpt does not establish a dedicated S-flow transport pass. Continue tracing for any existing material movement/transfer API before adding one; avoid a second competing movement path.
- The field has cell revisions and bounded neighborhood queries that may support cache invalidation and local flow updates. Confirm their intended mutation semantics before relying on them.

### Additional trace findings

- The only located canonical movement APIs (`Simulation::update_movement` / `Simulation::try_move_cell`, with implementation in the current movement modules) govern organism locomotion. They move an organism's connected structure and resolve physical collision/pushing; they are not an environmental-current transport system for free `PhysicalMaterial` instances.
- `ActiveMaterialField` supports deposits, atomic batch deposits, and transactional splitting. Deposits preserve an existing physical object, but there is no identified general-purpose API that moves a free environmental physical object from source cell to destination cell while preserving the exact object and atomically updating both cells. A new transport commit boundary likely belongs in the field subsystem after the full caller audit.
- `PhysicalMaterial` already stores complete material composition, per-constituent placements, internal physical bond endpoints/energy, optional owner-relative origin, and optional realized constituent shapes. Translation should update only world placements and preserve all other fields. For deformable water, preserve its authoritative realized shape instead of reconstructing it from the catalog.
- Resource types already distinguish `PhysicalState::Rigid` and `PhysicalState::Fluid`; `Form::Fluid` remains as a legacy/realized shape representation with arc and multi-contact boundary support. Water is therefore not a new material category for this work. `ResourceProperties` has no explicit permeability coefficient, but `src/materials/resources.rs` already provides `permeability_from_cohesion`: it maps Water to fully permeable and Carbon to a hard wall, with intermediate cohesion mapped continuously between them. Reuse this existing physical rule rather than adding a new permeability property unless later evidence shows it is insufficient.
- The initial obstruction approximation should derive solid blocking and open-space/permeability from actual geometry where possible. If a numeric material permeability field proves necessary, isolate that as a separately justified schema decision rather than silently equating fluid state with porosity.
- `ActiveMaterialField` has a global revision and per-cell revisions, plus bounded cell-intersection queries. These are candidate invalidation primitives for obstacle-flow caches; obstacle changes must update revisions through the authoritative mutation boundary.
- `src/perception/movement.rs` has a private `translate_physical` helper used when an organism pushes a physical environmental object. It preserves non-position fields while translating constituent placements and wrapping Y. Its existing push commit removes source indices in reverse order, increments the global field revision, and directly re-deposits objects without per-cell revision updates. Do not reuse that mutation sequence unchanged for ambient transport; a new field-level atomic move should validate destination placement before commit and update source/destination cell revisions.
- The default catalog's resource masses range from Hydrogen at 0.25 to Phosphorus at 1.75; Water is 1.00. These provide a concrete calibration range for linear mass-response tests. Compound mass should use the existing exact sum from `Material::mass(catalog)`.
- Obstacle contact already derives permeability from cohesion in `environmental_penetration_allowed`; the helper uses a 24-step binary search to bound partial penetration. This is useful precedent for permeability semantics, but the flow model should compute local velocity perturbation and wake rather than copy organism collision/push behavior.

### Critical integration risk found in `Simulation::step` and chemistry keys

- The environment-owned BREAK and COMBINE passes run sequentially during `Simulation::step`, before organism-owned chemistry. To let flow create contacts for the same tick's discovery, the natural insertion point is before environmental chemistry discovery, after any earlier tick-start lifecycle work that may mutate the field. Confirm exact ordering against the complete step before coding.
- Environmental COMBINE accumulation keys include cell/material locations and the realized source snapshots; BREAK keys likewise identify a particular realized source and bond. Movement or geometry changes can therefore invalidate accumulated activation. That is not automatically a defect: interrupted contact should be allowed to lose progress, and flow should change which encounters happen.
- Treat flow–chemistry interaction as a first-class emergent behavior, not an interference to suppress. Do not pause flow around reacting materials, pin reactants together, preserve stale activation across changed geometry, or relax chemistry thresholds just to guarantee a chosen reaction.
- Calibrate the movement timescale against activation accumulation. If motion is so fast that essentially all otherwise-valid contacts are continually broken before reactions can activate, adjust the environmental transport scale/profile—not the chemistry rules. Tests should measure both expected movement/contact changes and whether valid reactions can emerge under normal ticks.

## Implementation status (2026-10-29)

- Added `src/simulation/environment_flow.rs` with a deterministic S-shaped velocity field, smooth outward speed falloff, two vector-field eddies, linear mass mobility with a positive floor, and cohesion-derived obstacle deflection/wakes.
- Updated the World canvas to show a blue water field, the matching S-shaped spine, and bend eddies. This is a visual aid only; the Rust velocity field remains authoritative.
- Added snapshot-based movement of complete `PhysicalMaterial` objects before environmental BREAK/COMBINE discovery. The commit removes source entries in reverse index order, inserts translated instances only after destinations are staged, and marks touched cell revisions. Constituent counts, internal bonds, bond energy, realized shapes, and rotations are retained; only world-space placements move.
- The obstacle approximation uses 5-unit spatial bins to query local geometry. Every obstacle returned by the relevant local bins contributes; there is no per-object influence cap. Dense-bin cost must be measured and optimized without dropping relevant physical influences.
- Added focused tests for deterministic finite flow, core-to-edge speed gradient, positive-floor mass response, obstacle wake behavior, catalog-derived permeability, cross-cell physical-material inventory/revision preservation, horizontal whole-object bounds, and vertical wrap conservation. Formatting, architecture checks, and all five focused energy/chemistry/transaction/geometry/organism suites passed on the prior head. The controlled main-based run from PR #227 reproduced the same seven full-suite failures as this branch (821 passed, 7 failed, 2 ignored after correcting the stale storage assertion), confirming those failures are baseline failures. PR #227's test-only correction was merged as `f777e05e3b5c7ab9bc0bb36565d012f36746d5ae`. The latest PR #226 run predates that merge and reports 827 passed, 8 failed, 2 ignored. The environmental discovery timing benchmark passes. Clippy fails on existing diagnostics in constructor, geometry, Bob-library, and environmental-combine code; captured diagnostics do not point to `environment_flow.rs`. The newly added boundary regressions are awaiting the current-head CI run.
- **Chemistry interaction finding:** COMBINE accumulation keys include source cell/index and full realized snapshots, including placements. Continuous movement can therefore reset activation even while a contact remains valid. Do not pause flow or weaken chemistry. PR #225 must decide and test encounter-progress identity so progress can accumulate while the same contact persists, and dissipate naturally when contact disappears or the relevant geometry/endpoints change.

## Required implementation sequence

1. **Finish transport-path trace**
   - Find all existing field movement, physical-material transfer, boundary, and mutation/revision paths.
   - Identify the authoritative position owner for each realized object and whether a transport system already exists elsewhere.
   - Record the order in 'Simulation::step' where flow transport can safely run relative to chemistry, energy sampling, and structural mutations.
   - Establish whether environment geometry can be queried efficiently enough for obstacle approximation and how porous structure/permeability can be represented without inventing an ungrounded field.

2. **Define the deterministic flow field**
   - Add one authoritative query returning local velocity (vector and/or speed/direction) at a world position.
   - Encode the S-shaped high-speed spine, smooth outward falloff, and recirculating eddies.
   - Validate finite output, stable boundary behavior, repeatability, and nonzero background motion where required by the selected profile.
   - Keep visual blue-water presentation separate from the physical velocity calculation.

3. **Add mass-dependent transport**
   - Define and document the linear mobility mapping and strictly positive lower bound.
   - Calibrate using catalog mass values; reject NaN/infinite/negative invalid inputs safely.
   - Apply flow displacement to complete realized objects, preserving relative placements, constituent counts, internal connections, and associated bond/energy state.
   - Ensure the field's horizontal boundary and vertical wrap semantics are respected and that objects do not disappear or duplicate at boundaries.
   - Do not use flow as a source/sink of matter or energy.

4. **Add obstacle disturbance**
   - Begin with a bounded local approximation: dense geometry deflects/attenuates nearby flow and creates a downstream sheltered wake; porous geometry allows partial transmission.
   - Define the obstruction input from existing authoritative physical geometry, not decorative labels or material names alone.
   - Add tests for no obstacle, dense obstacle, porous obstacle, and downstream wake. Keep the approximation deterministic and bounded.
   - Avoid full fluid simulation and avoid full-field recalculation each tick unless measurements demonstrate need.

5. **Performance and caching**
   - Establish baseline cost for flow queries and movement before optimization.
   - Cache static/background field values where useful; invalidate obstacle-affected regions when authoritative geometry changes.
   - Prove the cache cannot serve stale flow after relevant structure changes. Keep cache behavior observationally equivalent to uncached queries.

6. **Acceptance tests**
   - S-current: strongest velocity follows the S spine; speed decreases smoothly outward.
   - Eddies: local velocity direction includes recirculation at both bends, not just a speed dip.
   - Mass: equal-position/equal-time test objects show the intended linear trend, with a positive floor for the heaviest catalog mass.
   - Conservation: material inventory is exactly unchanged by transport; internal bond/geometry identity is preserved.
   - Obstacles: dense obstacles divert/attenuate flow and create a wake; porous obstacles transmit some flow.
   - Boundaries: no material loss/duplication across horizontal limits or vertical wrap.
   - Determinism: same initial field, geometry, and tick sequence yield identical positions.
   - Bounded work and repeatable performance measurements; no unbounded geometry scans.
   - Verify both an empty environment and a field containing structures.

## Scope boundaries

- No chemistry equation, activation, COMBINE, BREAK, or energy-accounting policy changes.
- No organism behavior/roles and no organism-supplied environmental energy.
- No arbitrary new material types and no fractional constituent splitting.
- Do not silently replace the legacy default seed functions. Keep the new flow environment explicit until acceptance and user-approved rollout.
- Do not implement full fluid dynamics, turbulence solvers, or pressure fields in the first pass.
- Do not bundle PR #215's run/pause UI, structured reaction-event UI, or 24,000-tick benchmarks here.

## Dependency relationship

This environment PR is intended to land before PR #225's approved live chemistry fixture relies on flow-mediated transport. PR #225 should stay focused on discrete raw reactants, live-tick COMBINE → BREAK, independent material/energy conservation, rollback, determinism, and bounded chemistry work. It should not implement a second flow model.

## Completion gate

The PR is ready to merge when the deterministic S-shaped flow, mass-dependent movement, geometry-aware dense/porous obstacle response, conservation, cache invalidation, boundary behavior, and bounded performance are covered by tests; the existing suite is compared against the accepted baseline; and no chemistry semantics or default production seeding were changed as an accidental side effect.