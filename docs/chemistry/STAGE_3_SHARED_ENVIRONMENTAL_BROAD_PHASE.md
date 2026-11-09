# Stage 3 — Shared environmental broad phase

Status: the read-only prototype and adapter parity described in the original Stage 3 checkpoint are complete. Production integration has since begun; see the Stage 5 update at the end of this document for the current boundary.

## Objective

Replace three independent walks over environmental field ownership with one reusable, bounded broad-phase snapshot and candidate-pair traversal. Reaction-specific geometry and chemistry remain separate.

The shared traversal locates potentially interacting physical materials and exposes current material identities, locators, placements/bounds, and candidate relationships. It does not decide whether a reaction is chemically valid.

## Implemented read-only prototype

The shared broad-phase module now has two distinct pair sources:

- The original all-pairs window remains the conservative reference oracle.
- A spatial pair window derives complete realized-material bounds by restoring each snapshot material at the same origin pose used by the live environmental paths, then calls the field's existing bounds-to-cell query with explicit padding.

The spatial window supports two contracts:

- **Unordered:** one candidate per material pair, canonicalized by stable environmental identity. The indexed cursor avoids an unbounded per-cursor seen-pair set: the lower-identity anchor emits when its own bounds reach the higher-identity target; the higher-identity anchor emits only when the lower anchor's bounds do not reach it. This preserves the union of both expanded-bounds queries while bounding deduplication memory to O(1) per cursor. This is deliberately conservative and may be a superset of the current COMBINE traversal; exact parity is still required.
- **Directed:** both anchor-to-incoming orientations, preserving DISPLACEMENT's role-sensitive search.

The window reports the total known spatial candidates plus visited and deferred counts. It does not perform endpoint geometry, chemistry, activation, scheduling, funding, or commits. The all-pairs oracle remains available for set comparisons.

Current limitations:
- This is still a read-only candidate generator; no reaction adapter or production path consumes it.
- Bounds are rebuilt from the current field snapshot. There is no persistent registry or incremental refresh yet.
- The spatial candidate set intentionally follows the existing field-cell query semantics. The current helper clamps horizontal X to the field and wraps vertical Y; this is opposite the earlier Master Spec 2.4 map description (left/right wrap, hard top/bottom). This prototype preserves current production behavior and does not silently change world geometry. Resolve that discrepancy explicitly before any geometry-policy change or production cutover.
- The spatial filter is at the material-pair layer, not proof of complete endpoint-interface parity.
- The first test fixtures exercise spatial pruning, pair orientation, and deferred-window accounting. They do not yet cover wrap seams, geometry asymmetry, or exhaustive comparison with live COMBINE/DISPLACEMENT discovery.

## Confirmed current behavior

The existing tick order is sequential and observable:

1. Transport updates the field.
2. BREAK discovers against the transported field and commits its accepted operations.
3. COMBINE discovers against the post-BREAK field and commits its accepted operations.
4. DISPLACEMENT discovers against the post-COMBINE field and commits its accepted operations.

The current discovery paths are materially different:

- BREAK uses a tick-rotated bounded nested traversal: cell → physical material → internal bond. Inspected nested entries are counted separately from yielded bond candidates.
- COMBINE uses a bounded field-anchor scan, target-cell inspection, component-pair work, and restored-structure endpoint generation.
- DISPLACEMENT traverses bonded sources and incoming materials, restores trial structures, removes one exact old bond on the trial graph, and searches replacement contacts. Directed anchor/incoming roles and both anchor/displaced orientations are meaningful.

A single immutable snapshot built once before BREAK and reused unchanged for all three reactions would be incorrect: later stages must see earlier commits.

## Proposed shared data flow

### A. Broad-phase material snapshot

Build a pass-local snapshot from the current field state. Each entry carries:

- stable environmental identity (never a cell index or vector slot);
- current material locator (cell and material indices);
- current placement/geometry bounds needed for broad-phase overlap;
- constituent count and topology signature/generation for this snapshot;
- references to existing internal bonds;
- deterministic source ordering.

Do not promote the current pass-local state_version placeholder into a persistent version. Do not infer per-material versions from coarse cell revisions.

### B. Candidate relationships

Generate deterministic candidate relationships from the snapshot, deduplicated by stable material identity pair while retaining the source locator needed for revalidation.

The broad phase must preserve:
- the exact current field-cell query semantics (horizontal X clamping and vertical Y wrapping);
- all target-cell and bounds-padding semantics already used by the existing paths (both COMBINE and DISPLACEMENT currently expand restored body bounds by 2.1 world units);
- distinct material pairs even when they share a cell;
- candidate ordering/rotation and explicit deferred coverage under bounded work;
- no self-pairs and no duplicate unordered ordinary pairs.

BREAK consumes the bond list attached to visited material entries; it does not need to reinterpret ordinary contact pairs.

### C. Reaction-specific projections

The same material snapshot/pair stream is projected independently:

- COMBINE: ordinary unordered material pairs → restored constituent pairs → all geometry-eligible endpoint interfaces. Keep the existing live COMBINE evaluator authoritative.
- BREAK: visited materials → each existing bond record. Preserve existing bond traversal semantics and separate cell/material/bond inspection counters.
- DISPLACEMENT: each bonded material and each incoming candidate material → both directed anchor/displaced orientations → trial graph with the exact old bond removed → every geometry-valid replacement interface. Do not collapse to the production best-only contact.

No shared scalar score, shared activation rule, or common reaction likelihood is introduced.

## Sequential visibility and invalidation

Do not reuse a pre-BREAK snapshot blindly after a commit.

The first safe integration strategy is a shared traversal service with explicit refresh boundaries:

1. Build/consume the BREAK view from the transported field.
2. After BREAK commits, refresh affected source/destination cells and their broad-phase neighborhoods; COMBINE consumes the refreshed view.
3. After COMBINE commits, refresh affected cells/neighborhoods again; DISPLACEMENT consumes the refreshed view.
4. Revalidate stable identities, current locators, topology signatures, exact bond references, and live geometry immediately before each operation is committed.

An incremental registry may later avoid rebuilding unaffected neighborhoods, but it is not required for the first parity-safe version. Correctness takes priority over claiming a single immutable pre-tick scan.

## Observational counters

Keep units distinct:

- broad-phase cell visits;
- material visits;
- material-pair visits;
- constituent-pair visits;
- existing-bond visits;
- raw endpoint candidates;
- geometry evaluation operations;
- registry refresh/invalidation updates;
- known, visited, and deferred candidate counts.

Counters are observational only. Do not change current discovery partitions, realization slots, or chemistry policy in this stage.

## Required parity tests before production cutover

Use deterministic small worlds and compare shared output to existing implementations as reference oracles:

1. Empty and sparse fields; multiple materials in one cell.
2. Same-cell and adjacent-cell pairs; cell-size and bounds-padding edge cases.
3. Current field boundary behavior: horizontal X hard clamping and vertical Y wrap seam. Separately track the Master Spec 2.4 discrepancy; do not silently change the field policy as part of this refactor.
4. Movement across cells and across the wrap seam.
5. Rotation and asymmetric material bounds.
6. Multi-constituent materials with multiple bonds and multiple endpoint alternatives.
7. Both displacement anchor/displaced orientations.
8. BREAK commit followed by COMBINE discovery in the changed neighborhood.
9. COMBINE commit followed by DISPLACEMENT discovery in the changed neighborhood.
10. Split/merge or topology replacement producing fresh material identities.
11. Candidate windows spanning ticks: no starvation, no lost deferred work, and no false claim that unvisited means invalid.
12. Stale locator/topology/bond revalidation immediately before commit.

For each fixture, compare complete stable identity/interface sets, not just counts. Compare directed displacement contexts separately from unordered ordinary COMBINE pairs.

## Exit criteria

Stage 3 is complete only when:
- the shared broad phase has exhaustive small-world reference parity;
- bounded traversal reports known/visited/deferred coverage correctly;
- sequential refresh tests prove BREAK → COMBINE → DISPLACEMENT visibility;
- all three existing reaction evaluators can consume shared projections without chemistry-policy changes;
- full tests, formatting, architecture checks, and Clippy pass.

Until then, all production discovery paths remain authoritative. No merge or production cutover is implied by this design checkpoint.


## Bounded cursor design checkpoint (after full CI pass)

The current `spatial_pair_window` is not an acceptable bounded traversal: it constructs the complete candidate set (including a map/vector and deduplication) and only then applies `start` and `limit`. Its returned vector is bounded, but its discovery work and temporary memory are not.

The next implementation must be a resumable cursor, not another eager window wrapper. The cursor contract is:

1. **Persistent continuation state:** retain the source-material ordinal, target-cell window ordinal, target-material slot ordinal, and the current target-cell window position. A call resumes from that exact position; it must not restart an anchor or silently rotate past unvisited work.
2. **Bounded cell enumeration:** use the field's limited bounds-query primitive in finite windows. The cursor must know the total candidate-cell count (or an equivalent completion token) without first materializing the complete cell list. Each returned cell consumes the documented cell-inspection budget, including empty cells.
3. **Bounded material enumeration:** inspect target material slots incrementally. Each inspected slot consumes material-inspection budget, including self-pairs, stale/unassigned entries, and role/order rejections. Candidate count is not a proxy for work performed.
4. **Bounded output and accounting:** stop when the explicit work budget is exhausted, not merely when enough candidates have been emitted. Return emitted records, exact inspected-work counters, and a continuation token. Distinguish exhausted from deferred; an empty output with a continuation is not proof that no candidates exist.
5. **Stable deterministic order:** order source anchors by stable environmental identity and target cells/material slots by a documented deterministic sequence. Unordered pair emission must not depend on mutable locator ordering or require an unbounded seen-pair set. Cursor continuation must be independent of vector allocation addresses and must not skip or repeat an inspection.
6. **COMBINE unordered parity:** canonicalize each emitted pair by stable identity, never by mutable locator. Preserve the union of both endpoint-bounds queries. Deduplication must not hide an uninspected reverse query or turn a deferred query into a negative result. The implementation may use a bounded per-cursor seen set, but its memory growth must be explicit and covered by limits; alternatively prove a canonical query rule that is equivalent to the union.
7. **DISPLACEMENT directed parity:** preserve anchor → incoming direction and the reverse direction as separate records. The cursor must not canonicalize these role-sensitive pairs.
8. **Refresh semantics:** a cursor belongs to one immutable pass snapshot and captures the field's coarse mutation revision. Every page rejects a changed field revision or refreshed snapshot; this revision is an invalidation token only, never a per-material topology/state version. After any topology or ownership commit, discard affected cursors and rebuild from the refreshed field snapshot. Do not carry old locators, bounds, or bond references across BREAK/COMBINE refresh boundaries.
9. **Fairness:** if a query exceeds one tick's budget, continuation advances monotonically through its deterministic work universe. A later tick may rotate the starting anchor for fairness only if the rotation is represented explicitly and the full cycle is proven to cover every item without starvation. Do not combine a rotated restart with an old continuation.

### Required cursor tests

- A large bounds query proves the cursor never asks the field for more than the configured cell window and never allocates the full cell list.
- Work-budget exhaustion in the middle of a target cell resumes at the next material slot, with no repeat or omission.
- Exhaustion between cell windows resumes at the next cell ordinal.
- Empty cells and rejected/self/stale material slots consume work and are counted.
- Candidate output may be empty for a page while continuation remains; subsequent pages still find later candidates.
- Concatenating all pages yields exactly the exhaustive spatial reference set for both unordered and directed modes.
- Unordered mode covers the union of both bounds queries, deduplicates exactly once, and does not emit a pair until the relevant query work is complete enough to establish it.
- Directed mode retains both orientations and exact anchor/incoming identities.
- Repeated page sizes (1, uneven sizes, and sizes larger than remaining work) produce identical final sets and coverage.
- A snapshot refresh or field mutation invalidates old continuation state; bounds-preparation and pair cursors compare the captured coarse field revision and reject stale use rather than silently resolving old locators against new ownership. The revision is not treated as a persistent per-material version.

### Implementation boundary

Do not change live COMBINE/DISPLACEMENT budgets, per-tick partitions, rotation policy, reaction thresholds, or transaction behavior while implementing the cursor. First prove cursor work accounting and parity in isolation. Only then compare the cursor against each production adapter and design the sequential refresh integration.


### Additional work-boundary finding

The cursor cannot claim bounded discovery if it eagerly calls `spatial_bounds_from_field` first. That helper restores every material and computes every body's geometry before the pair window starts. The implementation must either:
- derive and cache one material's bounds only when its source anchor is reached, charging restoration/geometry work to the same explicit budget; or
- define a separately bounded snapshot-preparation phase with its own continuation and counters.

The same accounting must include failed restorations, invalid catalog/material checks, and materials with no realized body. Otherwise large or malformed materials can consume unreported work while producing no candidate. Snapshot ownership enumeration remains a distinct one-pass cost and must be measured separately from lazy bounds construction and pair traversal.

This prevents moving the unbounded eager scan one function earlier and calling the resulting pair cursor bounded.


## First bounded pair-cursor implementation

Commit `f7b78a077fab65f031ea96e0eeeb1cf804c5ab1e` adds `SpatialPairCursor` as an isolated, read-only implementation.

- It retains only the pass ID, orientation, next pair indices, and completed pair-check count; it does not build a candidate vector, map, or per-page seen set.
- Each page evaluates no more than its explicit pair-work limit and emits no more than its output limit.
- A page may emit zero candidates while remaining incomplete. The caller must keep the cursor until `complete` is true.
- COMBINE uses stable identity ordering and tests the union of both materials' expanded-bounds cell membership.
- DISPLACEMENT walks every ordered non-self pair, preserving both directions.
- Cell membership is computed in O(1) using the exact current field query semantics (X clamped, Y wrapped), avoiding an allocated full cell-query vector per pair.
- The cursor is bound to one snapshot pass ID; a refreshed snapshot is rejected.
- Tests compare concatenated pages against the existing exhaustive spatial candidate oracle for both orientations and exercise empty intermediate pages and stale-pass rejection.

### Deliberate limitation

This is a **bounded pair-predicate cursor**, not yet the final spatial-cell traversal. It scans the exhaustive pair universe and checks whether the target cell belongs to the source bounds query; this proves deterministic bounded work and set parity but is still O(n²) in material count. It also accepts a precomputed bounds slice, so it does not yet bound the cost of constructing that slice. Do not wire it into production or describe it as the final optimized shared broad phase.

The next substage is a separately resumable, lazy material-bounds preparation cursor, with explicit work accounting for every restoration/geometry attempt. Once that is proven, we can replace the exhaustive pair scan with a genuinely indexed cell/material cursor while retaining the same parity oracle. The existing eager `spatial_bounds_from_field` and `spatial_pair_window` remain reference helpers only.


## Current implementation and parity audit (2026-11-08)

This section supersedes the earlier pair-cursor-only status and its description of lazy bounds preparation as the next substage.

### Implemented traversal layers

- **Snapshot:** one pass over field cells and physical materials, retaining stable environmental identity separately from mutable cell/material locators and projecting existing bonds.
- **Bounds preparation:** `MaterialBoundsCursor` validates a material header once, then charges each constituent placement/shape inspection as its own work item. It does not restore the whole structure or run penetration checks merely to calculate conservative bounds. The eager restore-based helper remains a reference oracle.
- **Indexed traversal:** `IndexedSpatialCursor` walks only cells intersected by each material's expanded bounds and inspects the materials actually stored in those cells. It retains the exact cell and material-slot continuation position.
- **Unordered COMBINE pairs:** stable-identity ordering emits each pair once while preserving the union of both materials' bounds queries; no candidate-sized seen-pair set is retained.
- **Directed DISPLACEMENT pairs:** both anchor/incoming orientations remain separate records.
- **Work and invalidation:** anchor initialization, cell checks, and material-slot checks are separately counted and charged. A cursor is bound to one snapshot pass ID and captured field revision. A refreshed snapshot or changed field revision rejects continuation. The field revision is only a coarse invalidation token, not a per-material state or topology version.

### Parity results

| Layer | Reference comparison | Coverage |
|---|---|---|
| Indexed broad phase | Exhaustive spatial reference using the field's current bounds-query semantics | Exact pair identities for unordered and directed modes; both bounds-reach directions; edge-crossing and same-cell fixtures; page work limits 1–5 and output limits 1–3; empty-page continuation; stale snapshot/field rejection |
| Sparse-field work | Exhaustive all-pairs count | Indexed cursor performs fewer work units than the exhaustive pair universe in a sparse 30-material fixture |
| COMBINE adapter | Existing production contact-selection helper | Exact constituent/endpoint interfaces after the same duplicate-interface normalization; chemistry evaluation is applied after discovery, not used as a shared broad-phase filter |
| BREAK adapter | Existing bounded production bond discovery | Exact material identity, bond index, constituent indices, endpoints, and stored bond energy; snapshot bond inventory and adapter inventory agree; nested cell/material/bond work counters agree |
| DISPLACEMENT adapter | All-alternatives post-removal census | Exact endpoint candidate values and interface count for both bonded-anchor orientations; exact incoming/anchor/displaced constituent identities and removed-bond reference |

The COMBINE fixture exercises a bounded component-pair window and compares the adapter output with the existing production selector for that same window. BREAK parity covers a fixture with six bonds across two physical materials. DISPLACEMENT parity covers both orientations of a two-constituent bonded source plus an incoming constituent. These are deterministic reference fixtures, not claims that all possible material/catalog combinations have been exhaustively enumerated.

### Explicit boundary: not a production cutover

The parity tests establish the shared snapshot, indexed candidate traversal, and read-only reaction adapters independently. They do **not** establish that live COMBINE, BREAK, or DISPLACEMENT now consume the shared cursor. Production discovery, reaction-specific activation, scheduling, funding, commit revalidation, and topology transactions remain unchanged.

In particular:

1. A cursor must be discarded after any topology or ownership mutation and rebuilt from a fresh field snapshot. The coarse field revision and pass ID prevent stale continuation; they do not provide persistent per-material versions.
2. BREAK, COMBINE, and DISPLACEMENT must retain their current same-tick order and refresh boundaries: BREAK commits, refresh; COMBINE commits, refresh; then DISPLACEMENT. One frozen snapshot cannot span those mutations.
3. The shared traversal currently follows the field's existing cell-query semantics (horizontal X clamped, vertical Y wrapped). The separate Master Spec 2.4 world-boundary discrepancy remains unresolved and must not be silently changed as part of this refactor.
4. The broad-phase module remains read-only and intentionally isolated. The next stage is a staged production integration with exact candidate parity at each refresh boundary—not a change to chemistry eligibility, activation, energy, budgets, or transaction semantics.

**Audit disposition:** broad-phase cursor parity and adapter-level candidate parity are covered by focused tests. The architecture is ready for a separately reviewed integration stage, but it is not yet safe to claim a single shared production discovery pass or to merge solely on the basis of these fixtures.


## Stage 4 — shared reaction-evaluation contract (2026-11-08)

The first Stage 4 increment adds `src/chemistry/environmental_evaluation.rs` as a typed result envelope around reaction-specific assessments.

### Shared vocabulary

- `EnvironmentalReactionKind` identifies COMBINE, BREAK, or DISPLACEMENT.
- `EvaluationStage` records the furthest common lifecycle stage reached: candidate validation, chemistry, activation, energy settlement, scheduling, or commit revalidation.
- `EvaluationOutcome<T>` distinguishes ready, rejected, deferred, and stale results.
- `EnvironmentalReactionEvaluation<T>` carries the inventory pass ID and reaction kind while retaining the native reaction-specific payload type.
- `EvaluationJournal` retains payload-free lifecycle records for ready, rejected, deferred, and stale assessments. `EvaluationOutcomeCounts` summarizes those records without discarding their stage/reason detail. Recording is observational: it does not enqueue, retry, settle, or commit operations.
- Bounded work that was not inspected must be represented as deferred/unexamined, never rejected. A stale record is reserved for invalidated references or snapshots, not failed chemistry.

### Deliberate non-unification

The envelope does not define a common score, activation signal, threshold, energy unit, funding authority, or commit operation. A COMBINE payload remains an `EnvironmentalCombineOperation`; BREAK retains its own disruption/activation operation; DISPLACEMENT retains its coupled replacement trial/preflight semantics. Reaction-specific rejection reasons remain caller-owned descriptions rather than being normalized into a shared eligibility policy.

The common stage ordering is descriptive lifecycle metadata, not permission to move existing production work between stages. Existing same-tick order and refresh boundaries remain BREAK commit → refresh → COMBINE commit → refresh → atomic DISPLACEMENT commit. A stale result must be discarded and rediscovered from a fresh snapshot. Deferred results remain eligible for their existing reaction-specific progress/fairness handling.

### Tests and current boundary

Focused unit tests prove:
- all three reaction kinds retain distinct native payload types through the common envelope;
- ready is the only outcome that yields a scheduler payload;
- rejected, deferred, and stale outcomes cannot be mistaken for ready work;
- lifecycle stages have a deterministic descriptive order.

This is the **contract foundation**, not yet a shared evaluator wired into production. The three existing evaluators remain authoritative, and no live reaction behavior, activation accumulation, energy settlement, discovery budget, realization slot, scheduling policy, or topology transaction has changed. The next Stage 4 increment is to adapt one reaction at a time behind this envelope and compare its full ready/deferred/rejected/stale result stream against the current production path before any cutover.


## Stage 5 — production cursor integration (2026-11-09)

The original Stage 3 “production discovery remains unchanged” statements above describe the checkpoint at which they were written. The following production paths have since been connected on the current integration branch:

- **BREAK:** creates a fresh `EnvironmentalMaterialSnapshot` before its phase and discovers bounded bond records from that snapshot. Legacy field traversal remains the fallback when environmental identities are unassigned.
- **COMBINE:** creates a new snapshot after BREAK commits and uses the indexed spatial cursor with `ForwardOrdered` traversal. The cursor preserves field-locator ordering and tick rotation; its evaluator still owns COMBINE chemistry, activation, and formation rules.
- **DISPLACEMENT:** creates another fresh snapshot after COMBINE commits and uses the same indexed cursor in `Directed` mode for each bonded anchor. The old bond and atomic replacement evaluator remain unchanged. Unassigned identities fall back to the legacy target-cell query.

The same-tick refresh sequence remains BREAK commit → fresh snapshot → COMBINE commit → fresh snapshot → DISPLACEMENT. No snapshot or pair reference is carried across a topology/ownership mutation.

The displacement cursor now pages one unit of target-discovery work at a time. Its single-anchor directed fixture compares the emitted target-material sequence with `cells_intersecting_bounds_limited`, including tick rotation and both a short target-cell window and a complete window. The existing BREAK and COMBINE parity fixtures remain in place.

**Important remaining boundary:** this is production integration of the common snapshot and indexed-cursor infrastructure, not yet proof of one fully materialized candidate-pair list shared by all three evaluators. COMBINE and DISPLACEMENT currently traverse the common cursor separately, while BREAK reads bond records from the same snapshot. A single shared discovery materialization, with reaction-specific projections and exact parity under the existing budgets, remains to be completed before this optimization can be considered finished.

No chemistry eligibility, activation threshold, energy settlement, atomic replacement semantics, reaction budget, or world-boundary policy was intentionally changed. The horizontal/vertical field-query semantics discrepancy with Master Spec 2.4 remains unresolved and out of scope.

The integration branch remains draft and unmerged. The full green workflow at commit `65529bb2d949565e16c8866d54d371ded50b8150` passed formatting, architecture checks, focused tests, the full Rust suite, and Clippy. Subsequent parity-test strengthening is still subject to its own full CI run.

## Stage 6 — single-snapshot reaction handoff (2026-11-09)

> This stage supersedes the earlier same-tick refresh-boundary statements in the historical sections above. Those sections describe the prior sequential-commit design.

The production tick now follows the layered snapshot model:

1. Build one environmental material snapshot after transport and environmental inputs settle.
2. Discover and schedule BREAK from that snapshot without committing.
3. Discover and schedule COMBINE from the same unchanged snapshot.
4. Discover and schedule DISPLACEMENT from the same unchanged snapshot, reusing the COMBINE candidate handoff where its coverage is complete.
5. Reserve source material identities in reaction priority order: BREAK, then COMBINE, then DISPLACEMENT. A material can participate in at most one selected environmental reaction per tick. BREAK reserves the entire source material, not just one bond.
6. Commit the selected operations only after all three discovery layers finish. Products and split pieces are therefore first discoverable on the next tick.

COMBINE and DISPLACEMENT commit validation now resolves each source by stable environmental identity. The captured cell/material locator is a fast path when it still identifies the exact source; after another disjoint commit shifts vector slots, the resolver searches for the same identity and verifies the complete source material before proceeding. BREAK already uses identity-based location recovery. This permits deferred commits without carrying stale vector indices across mutations.

When a lower-priority operation conflicts with an already-reserved material, its accumulated reaction progress is restored rather than discarded. Chemistry evaluators, activation thresholds, energy settlement, reaction-specific budgets, and the atomic displacement transaction remain in their existing resolvers.

This stage changes same-tick visibility intentionally: no reaction can consume a product created earlier in the same tick, and no individual source material can be committed through multiple environmental reaction types in one tick.

### Stage 6 validation and performance

- Exact head: `a9e05ae0e1cb7a08853c5933cc3388849802d902`.
- Full CI run [42675821503](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/42675821503) passed: formatting, architecture checks, default environmental baseline, all focused suites, full Rust tests, Clippy, and the equal-tick benchmark job.
- Equal-tick runtime benchmark: 1,800 ticks, seed 42, three alternating-order repetitions, diagnostics disabled. Baseline median: **20.810 s**; single-snapshot revision median: **21.392 s**; **2.80% slower** in this run. A prior repeat measured 3.24% slower. The implementation is not currently a runtime speedup.
- A diagnostic-enabled comparison on the same architecture recorded 124 committed environmental reactions versus 121 on baseline (+2.5%), with the reaction mix changing under the new exclusivity/priority rules. That coverage difference is useful, but it does not cancel the measured runtime regression.
- The intended gains at this stage are semantic and architectural: one immutable snapshot per tick, no same-tick reaction cascades, and at most one selected reaction per source material per tick. Do not describe this as a proven runtime optimization.
- No production cutover or merge is authorized by this milestone.
