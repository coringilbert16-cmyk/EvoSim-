- no logical Water entering storage;
- intact composite material remaining intact while crossing a boundary;
- a high-solid/low-Water boundary remaining substantially resistant to passage;
- same topology with different composition producing different boundary behavior;
- same composition with different topology producing different boundary behavior.

This section is the authoritative reference for the interior/permeability work.


# Constructor Architecture — CURRENT PATH (Authoritative)

**Status: ACTIVE — revised 2026-10-02.**

**This is the constructor architecture we are implementing now. All previous constructor-search implementations are retired, invalid as design guidance, and must not be revived or optimized.** Their commits remain in git history only as historical/debugging context.

## A. The current path

The constructor is being rebuilt as a **local puzzle-piece assembler**, not a geometric solver.

For the **initial organism**, the constructor is **blueprint-free**. It does not know the desired genome shape, cavity topology, body plan, resource arrangement, or final piece count. Its job is simply to grow a connected physical organism from available pieces using the normal physical construction/bond authority.

The current conceptual pipeline is:

```
available physical pieces
        ↓
one physical seed / anchor
        ↓
local construction frontier
        ↓
find a small number of meaningful nearby fits
        ↓
commit one normal physical bond
        ↓
update the frontier
        ↓
repeat
        ↓
realized organism
        ↓
authoritative cavity / acquisition / viability analysis
```

The constructor **assembles structure**. Biology determines whether the realized structure qualifies.

The initial constructor therefore does **not** try to solve the final organism in advance.

## B. Initial-organism contract

The initial constructor must ultimately produce a realized organism satisfying the existing authoritative viability rules, including:

- a qualifying physical genome cavity;
- physical material outside the genome cavity;
- Water available as required by the viability contract;
- at least three distinct non-Water resource categories physically acquirable;
- normal physical geometry/contact/bond validity;
- no constructor-only biological exception.

There is currently **no required reproduction-size threshold, fixed body size, fixed topology, fixed material recipe, or fixed number of construction elements**.

A target in the range of hundreds of pieces is a construction/performance experiment, not a biological rule.

## C. Constructor does not know about the genome

The constructor must **not** be responsible for designing or proving the genome cavity.

The cavity is discovered from the realized physical structure by the existing cavity analysis.

The intended relationship is:

```
local physical growth
        ↓
emergent topology
        ↓
possible enclosed cavity
        ↓
cavity analysis determines whether it qualifies as genome
```

The constructor may use ordinary physical growth rules that make enclosed structure possible, but it must not contain a special genome-building solver or hard-coded genome geometry.

## D. Puzzle-piece assembly model

Pieces are treated as rigid physical building blocks.

A construction step should ask only approximately:

> **Can this piece physically attach to this exposed local opportunity?**

It should not ask:

> Can I find the globally optimal position for this piece?
>
> Can I prove this entire future organism will work?
>
> Which of thousands of possible future branches is best?
>
> Can I reproduce an exact blueprint topology?

Local irregularity is acceptable. Exact coordinates are not important. Exact piece ordering is not important. A valid body that differs from an intended arrangement is a normal construction outcome.

Connections should remain simple and rigid. A small local placement/contact tolerance is acceptable where needed, but this architecture must not turn bonding into a continuous flexible-body constraint solver.

## E. Construction frontier

The core data structure is a lightweight **construction frontier**.

The frontier contains exposed local opportunities where another physical piece may attach. It does **not** contain every possible point in space and does not represent every possible future organism.

For each frontier opportunity:

1. inspect the local connection/contact geometry;
2. generate only the small set of shape-specific fits that can actually work;
3. reject penetration/invalid contact;
4. commit the first suitable local attachment through the normal bond transaction;
5. add the new exposed opportunities to the frontier;
6. continue.

The critical performance invariant is:

> **Adding one piece must not require scanning or solving against the entire existing organism.**

Construction cost should therefore grow approximately with the number of pieces actually assembled, not with the number of possible arrangements.

## F. Geometry generation

Geometry is local and shape-aware.

The implementation should precompute reusable shape attachment information such as:

- exposed edges;
- vertices/corners;
- surface directions;
- connection offsets;
- compatible local transforms.

Candidate generation must **not** use:

- coordinate-grid searches;
- arbitrary whole-plane position sweeps;
- exhaustive 360° search when geometry can prune it;
- pairwise searches across every existing unit;
- global candidate ranking;
- future-organism enumeration.

Same-shape stacking, edge/edge contact, vertex/vertex contact, right-angle fits, and other genuinely available local shape relationships should emerge from the physical geometry rather than from special-case body recipes.

## G. No global search and no arbitrary attempt budget

There is no valid fallback in which the constructor keeps trying random/global placements until a viable organism happens to appear.

Likewise, an arbitrary node/attempt budget is **not** the solution to an inefficient construction algorithm.

Do not add:

- 5,000/50,000/500,000-placement budgets;
- recursive branch counts;
- large search-depth limits;
- “try until timeout” correctness;
- global backtracking;
- whole-organism candidate cloning;
- repeated complete viability analysis after every local placement.

If construction is slow, the architecture is wrong and the hot path must be fixed rather than hidden behind a larger timeout.

## H. Assembly first, viability second

The initial constructor should assemble the physical structure first.

After assembly, run the authoritative biological analysis:

1. derive the realized genome cavity;
2. determine the realized structural organization;
3. evaluate physical acquisition/accessibility;
4. apply the authoritative initial-organism viability contract.

Do not repeatedly run expensive cavity/interior/acquisition analysis while deciding every local piece placement.

If the first simple assembler does not yet produce viable organisms reliably, improve the **local assembly rules** or the developmental starting pattern. Do not reintroduce the retired global solver.

## I. Initial construction scale

The constructor is intentionally being tested with substantially more pieces than the old ~30-piece attempts.

A useful experimental range is approximately **300–500 pieces**.

This is not because an organism biologically requires 300–500 pieces. The purpose is to make the construction problem behave like actual local assembly: many simple pieces forming a continuous, irregular structure rather than a tiny structure whose every connection must be globally planned.

The implementation must remain fast enough that hundreds of local attachments are practical.

## J. Blueprint architecture is separate

The initial constructor is blueprint-free.

The **development/offspring constructor remains blueprint-driven**, but its blueprint is also a developmental preference rather than a rigid CAD specification.

The eventual blueprint should describe coarse preferences such as:

- preferred occupied regions;
- excluded/empty regions;
- broad material preferences;
- density gradients;
- developmental direction;
- anchor intent.

It should not prescribe hundreds of exact coordinates and exact bonds.

That refinement is a later step. It must not contaminate the simpler initial-assembly path.

## K. Old constructor attempts — RETIRED / INVALID

The following approaches were tried previously and are **not valid implementations of the current constructor architecture**:

- global/recursive free-form search through future construction branches;
- exhaustive candidate enumeration;
- large node or placement budgets;
- 500,000-attempt-style search;
- 5,000-attempt-style reduced search;
- arbitrary timeout/attempt limits used as correctness mechanisms;
- 10-placement lookahead;
- multi-step future-bond scoring;
- candidate cloning of whole structures;
- global candidate ranking;
- pairwise anchor × target × material closure searches;
- bridge construction whose purpose is to force a final closed topology;
- explicit genome/cavity closure solving inside the constructor;
- repeated whole-structure viability checks during growth;
- acquisition-region searches performed for every local construction choice;
- blueprint-neighbor ordering used as the primary construction algorithm;
- most-constrained blueprint closure as the primary construction algorithm;
- recursive restoration/search trees;
- any solver whose correctness depends on running for tens of minutes or hours;
- any hidden fallback to one of the above underneath the new local assembler.

These approaches are **OLD / INVALID / DO NOT EXTEND**.

They may be inspected to understand bugs or recover reusable low-level geometry/transaction code, but their control strategy is retired. A new implementation that quietly restores any of these patterns is not considered the current constructor.

## L. What may be reused

Retiring the old constructor does **not** mean discarding valid physical infrastructure.

The following remain reusable when they are truly general physical mechanisms:

- physical material representation;
- shape geometry;
- contact detection;
- penetration tests;
- endpoint/connection geometry;
- normal COMBINE;
- normal bond transaction;
- physical material restoration/transaction support;
- cavity analysis;
- resource/acquisition analysis;
- authoritative juvenile viability checks.

The distinction is:

> **Reuse physical authorities; do not reuse the retired search strategy.**

## M. Required implementation shape

The new constructor should converge toward a small architecture resembling:

```
AssemblyFrontier
    ├── local attachment opportunities
    └── lightweight construction state

ShapeGeometryCache
    ├── edges
    ├── vertices
    ├── normals
    └── local compatible transforms

LocalAssembler
    ├── select frontier opportunity
    ├── select available physical piece
    ├── generate local fits
    ├── cheap geometry/contact check
    └── normal bond transaction

RealizedStructure
    ↓
Cavity / Acquisition / Viability Analysis
```

There should be no global organism-search object hidden inside this architecture.

## N. Verification discipline

Every implementation step follows this order:

1. audit the physical authority and existing viability contract;
2. make the smallest change consistent with the current local-assembler architecture;
3. format;
4. compile/check;
5. run a **small focused construction test**;
6. measure runtime;
7. inspect failures by dependency layer;
8. only then broaden testing.

A test that takes tens of minutes or an hour before reporting a constructor failure is itself evidence that the implementation has violated the current performance architecture.

A passing test is not considered a valid fix if it was achieved by:

- weakening the viability contract;
- adding a body-specific special case;
- adding an arbitrary attempt budget;
- restoring global search;
- restoring multi-step lookahead;
- bypassing the normal physical bond authority.

## O. Non-negotiable current intention

> **The current constructor is a fast local puzzle-piece assembler.**
>
> It grows physical structure one local bond at a time.
>
> It does not solve the final organism in advance.
>
> It does not know the genome cavity.
>
> It does not search globally.
>
> It does not use arbitrary attempt budgets.
>
> It does not repeatedly prove viability during assembly.
>
> It assembles first; authoritative biology evaluates the realized result afterward.
>
> **All previous global/search-heavy constructor attempts are retired and invalid as the path forward.**
