# Stage 3 — Shared environmental broad phase

Status: read-only spatial candidate-window prototype implemented; production discovery remains unchanged pending exhaustive parity.

## Objective

Replace three independent walks over environmental field ownership with one reusable, bounded broad-phase snapshot and candidate-pair traversal. Reaction-specific geometry and chemistry remain separate.

The shared traversal locates potentially interacting physical materials and exposes current material identities, locators, placements/bounds, and candidate relationships. It does not decide whether a reaction is chemically valid.

## Implemented read-only prototype

The shared broad-phase module now has two distinct pair sources:

- The original all-pairs window remains the conservative reference oracle.
- A spatial pair window derives complete realized-material bounds by restoring each snapshot material at the same origin pose used by the live environmental paths, then calls the field's existing bounds-to-cell query with explicit padding.

The spatial window supports two contracts:

- **Unordered:** one candidate per material pair, canonicalized by current field locator. It takes the union of both materials' expanded-bounds cell queries before deduplication, avoiding a direction-dependent miss when only one query reaches the other material's cell. This is deliberately conservative and may be a superset of the current COMBINE traversal; exact parity is still required.
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
5. **Stable deterministic order:** order source anchors by stable environmental identity and target cells/material slots by a documented deterministic sequence. Cursor continuation must be independent of vector allocation addresses and must not skip or repeat an inspection.
6. **COMBINE unordered parity:** canonicalize each emitted pair by stable identity, never by mutable locator. Preserve the union of both endpoint-bounds queries. Deduplication must not hide an uninspected reverse query or turn a deferred query into a negative result. The implementation may use a bounded per-cursor seen set, but its memory growth must be explicit and covered by limits; alternatively prove a canonical query rule that is equivalent to the union.
7. **DISPLACEMENT directed parity:** preserve anchor → incoming direction and the reverse direction as separate records. The cursor must not canonicalize these role-sensitive pairs.
8. **Refresh semantics:** a cursor belongs to one immutable pass snapshot. After any topology or ownership commit, discard affected cursors and rebuild from the refreshed field snapshot. Do not carry old locators, bounds, or bond references across BREAK/COMBINE refresh boundaries.
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
- A snapshot refresh invalidates old continuation state; stale cursor use is rejected rather than silently resolving old locators against new ownership.

### Implementation boundary

Do not change live COMBINE/DISPLACEMENT budgets, per-tick partitions, rotation policy, reaction thresholds, or transaction behavior while implementing the cursor. First prove cursor work accounting and parity in isolation. Only then compare the cursor against each production adapter and design the sequential refresh integration.


### Additional work-boundary finding

The cursor cannot claim bounded discovery if it eagerly calls `spatial_bounds_from_field` first. That helper restores every material and computes every body's geometry before the pair window starts. The implementation must either:
- derive and cache one material's bounds only when its source anchor is reached, charging restoration/geometry work to the same explicit budget; or
- define a separately bounded snapshot-preparation phase with its own continuation and counters.

The same accounting must include failed restorations, invalid catalog/material checks, and materials with no realized body. Otherwise large or malformed materials can consume unreported work while producing no candidate. Snapshot ownership enumeration remains a distinct one-pass cost and must be measured separately from lazy bounds construction and pair traversal.

This prevents moving the unbounded eager scan one function earlier and calling the resulting pair cursor bounded.
