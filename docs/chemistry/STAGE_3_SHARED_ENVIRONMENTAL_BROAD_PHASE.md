# Stage 3 — Shared environmental broad phase

Status: design/audit checkpoint. No production discovery path is switched over by this document.

## Objective

Replace three independent walks over environmental field ownership with one reusable, bounded broad-phase snapshot and candidate-pair traversal. Reaction-specific geometry and chemistry remain separate.

The shared traversal locates potentially interacting physical materials and exposes current material identities, locators, placements/bounds, and candidate relationships. It does not decide whether a reaction is chemically valid.

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
- world left/right wrapping;
- hard top/bottom boundaries;
- all target-cell and bounds-padding semantics already used by the existing paths;
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
3. Left/right wrap seam; top/bottom hard walls.
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
