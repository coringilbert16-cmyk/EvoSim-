- no logical Water entering storage;
- intact composite material remaining intact while crossing a boundary;
- a high-solid/low-Water boundary remaining substantially resistant to passage;
- same topology with different composition producing different boundary behavior;
- same composition with different topology producing different boundary behavior.

This section is the authoritative reference for the interior/permeability work.



# Constructor Architecture — Authoritative Plan and Intention

**Status: ACTIVE — revised 2026-10-02. This section supersedes the earlier constructor-search experiments.**

The constructor's true job is:

> **Given a developmental blueprint and available physical material, rapidly grow one physically viable organism that is reasonably close to the blueprint without requiring exact reproduction.**

The constructor is **not** a CAD solver, global optimizer, proof engine, or exhaustive search procedure. This distinction is an architectural requirement because construction may eventually occur hundreds, thousands, or millions of times.

## A. Locked physical authorities

The following remain authoritative and are not being redesigned by this constructor realignment:

- actual physical material and inventory identity;
- actual geometry, contact, collision, and bond validity;
- ordinary COMBINE/bond admission rules;
- intact composite material geometry and internal bonds;
- cavity-derived genome qualification;
- realized-structure juvenile viability;
- Water and resource requirements;
- physical energy/ledger accounting;
- blueprint as developmental preference rather than rigid topology.

The constructor must not create a second physical bonding authority or a constructor-only biological rule.

## B. Constructor control model

The normal constructor loop is intentionally local:

```
current realized structure
        ↓
construction frontier
        ↓
small set of physically meaningful candidates
        ↓
physical filtering
        ↓
cheap developmental/viability filtering
        ↓
local blueprint preference
        ↓
commit one physical change
        ↓
update construction state
        ↓
repeat
```

Construction is therefore **growth**, not global planning.

A valid imperfect structure is preferable to an expensive attempt to prove the exact blueprint can be reproduced.

## C. Blueprint authority

The blueprint supplies developmental preference:

- preferred material;
- preferred relative location;
- preferred orientation;
- preferred connectivity;
- preferred developmental scale/density;
- anchor/developmental intent.

The blueprint does **not** require:

- exact element count;
- exact topology;
- exact closure;
- exact coordinates;
- exact bond angles;
- exact face matching;
- exact piece-by-piece realization.

If the preferred continuation is physically unavailable, the constructor may diverge while remaining viable.

## D. Candidate generation

Candidate generation must be geometry-first and bounded.

For each construction frontier, generate only physically meaningful possibilities derived from:

1. preferred blueprint direction/orientation;
2. nearby useful orientations;
3. valid shape-specific contact orientations;
4. exposed physical connection/contact opportunities.

Do not sweep arbitrary positions or orientations and do not enumerate every theoretically possible future.

Candidate generation stops once enough useful valid candidates exist for local choice.

The exact candidate count and orientation sampling are implementation parameters, not biological rules.

## E. Candidate choice

Candidates are evaluated in this order:

1. **Physical validity** — impossible contacts are rejected.
2. **Viability preservation** — reject candidates that obviously destroy the ability to reach a viable organism.
3. **Developmental progress** — prefer candidates that advance genome formation, required resource acquisition, useful frontier growth, or completion.
4. **Blueprint similarity** — among safe candidates, prefer the closest developmental realization.
5. **Geometric quality** — prefer candidates that leave useful future attachment opportunities.
6. **Controlled variation** — near-equivalent valid choices may vary so repeated construction does not deterministically produce one identical body.

These priorities are intentionally hierarchical. The constructor must not spend global search effort calculating an exact optimum.

## F. Developmental state

The constructor maintains lightweight incremental state rather than repeatedly rediscovering the same facts through full-structure analysis.

The state tracks at minimum:

- whether a qualifying genome cavity has formed;
- acquired required-resource categories;
- current physical construction frontier;
- useful remaining connection opportunities;
- blueprint/developmental progress;
- remaining construction material;
- small local recovery history.

Full authoritative validation remains available at milestones and at completion, but it is not the normal inner-loop operation.

## G. Genome-first developmental progression

Construction should reach a qualifying physical genome cavity early, without introducing a predefined core.

The progression is:

```
physical anchor
    ↓
genome-forming structure
    ↓
qualifying physical genome cavity
    ↓
outward developmental growth
    ↓
required resource diversity
    ↓
viable completion
```

The cavity remains defined entirely by realized physical geometry and graph structure.

## H. Resource progress

Resource requirements are tracked incrementally.

When a required physical resource becomes part of the developing organism, its resource category is recorded in the construction state. Missing requirements may influence candidate preference, but this does not create a hard-coded biological attraction rule.

Material substitution continues to use the established physical-material matching contract. A preferred material may be replaced only by an acceptable physical inventory candidate; below-threshold material remains unavailable rather than being silently forced.

## I. Feasibility probe — not lookahead search

The earlier 10-step lookahead experiment is **SUPERSEDED**.

The replacement is a small transactional **feasibility probe**.

A candidate may be temporarily simulated only to answer:

> Can this local choice still plausibly continue toward viability?

The probe returns a small feasibility result such as `safe`, `unsafe`, or `unknown`. It does not construct or rank an entire future organism, and it must never become an unbounded search tree.

The normal path commits one candidate immediately.

## J. Local recovery

The constructor remains forward-oriented.

If the current frontier reaches a genuine dead end, recovery is local:

```
dead end
   ↓
rollback only a small recent construction window
   ↓
try the next locally valid candidate
   ↓
resume forward growth
```

This is not global backtracking. The recovery window is a computational robustness mechanism and must remain bounded independently of organism complexity.

Committed construction should otherwise remain irreversible.

## K. Validation layers

Validation is deliberately separated by cost:

### Per candidate
Cheap physical and local-development checks.

### At developmental milestones
More substantial checks after events such as genome formation, resource completion, or major structural closure.

### At completion
The existing authoritative realized-structure viability contract.

No constructor-specific viability definition may replace the authoritative contract.

## L. Performance authority

The constructor is expected to run at evolutionary scale.

Therefore:

- normal construction cost should scale approximately with realized construction size, not with the number of possible organisms;
- candidate enumeration must remain bounded per step;
- expensive cavity/interior/whole-structure analysis must not run for every rejected candidate;
- speculative branches must remain small and transactional;
- no arbitrary large node-search budget may be used as a substitute for a good control strategy;
- profiling must target the actual hot path after correctness is established.

The desired computational shape is approximately:

```
O(realized construction steps × small local candidate set)
```

rather than combinatorial branch exploration.

## M. Implementation order

1. **Audit and isolate the old search architecture.**
   Identify exhaustive branching, recursive future search, repeated whole-structure validation, and candidate enumeration that exists only to support global search.

2. **Introduce lightweight construction state.**
   Make genome/resource/frontier progress incrementally available.

3. **Refactor candidate generation.**
   Generate a small geometry-driven candidate set rather than broad arbitrary search.

4. **Refactor candidate selection.**
   Apply physical validity → viability preservation → developmental progress → blueprint preference → geometry → controlled variation.

5. **Replace 10-step lookahead with bounded feasibility probing.**
   Preserve only the useful dead-end detection behavior.

6. **Add bounded local recovery.**
   Recover from genuine local dead ends without global search.

7. **Move expensive validation to milestones/finalization.**

8. **Delete or retire obsolete search machinery.**
   Do not leave the old solver underneath the new constructor as a hidden fallback.

9. **Build scale tests.**
   Measure 1, 100, 1,000, 10,000, and eventually 100,000+ constructions for success rate, worst-case time, candidate count, recovery count, and expensive-validation count.

10. **Only then optimize hot spots.**

## N. Explicitly superseded constructor experiments

The following are retained only as historical implementation context and must **not** be treated as current design authority:

- **Global/recursive free-form search** through many future construction branches.
- **Large node budgets** used to make exhaustive search eventually terminate.
- **10-placement lookahead as a branch-ranking/search mechanism.**
- **Blueprint-neighbor ordering as a substitute for a physical construction frontier.**
- **Most-constrained blueprint-closure ordering as a primary construction strategy.**
- **Any approach that repeatedly enumerates many candidates and commits only one while discarding all alternatives through global search.**
- **Any constructor strategy whose practical correctness depends on running for tens of minutes or an hour on a single test.**

These experiments may remain in git history for diagnosis, but they are **OLD / SUPERSEDED** and must not be extended.

## O. Verification discipline

For each realignment change:

1. audit the existing physical/viability contract;
2. make the smallest architectural step consistent with this plan;
3. format;
4. compile/check;
5. run focused constructor tests;
6. measure runtime;
7. inspect failures by dependency layer;
8. only then broaden validation.

A passing test obtained by adding a special-case geometry rule, weakening viability, or restoring an obsolete search strategy is not considered a valid fix.

**Non-negotiable intention:** The constructor grows viable organisms from physical reality. The blueprint guides development. Physics determines what can exist. Viability determines when construction has succeeded. Local decisions and bounded recovery provide reliability; global search is not the constructor.**

