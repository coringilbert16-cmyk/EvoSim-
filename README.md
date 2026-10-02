- no logical Water entering storage;
- intact composite material remaining intact while crossing a boundary;
- a high-solid/low-Water boundary remaining substantially resistant to passage;
- same topology with different composition producing different boundary behavior;
- same composition with different topology producing different boundary behavior.

This section is the authoritative reference for the interior/permeability work.



# Constructor Architecture — Authoritative Plan and Intention

**Status: ACTIVE — this section is the authoritative plan for the constructor work.**

**Current intention (2026-10-02):** Establish one coherent physical construction process that can produce a viable organism without requiring exact reproduction of a developmental blueprint. Construction is an irreversible physical growth process. The blueprint provides developmental guidance and preference; the realized physical organism is authoritative.

The immediate objective is not to make the existing test suite green by weakening contracts or adding special cases. The objective is to establish the physical, material, viability, and developmental rules in a coherent order, prove each layer with focused tests, and then implement the constructor around those rules.

## A. Core construction intention

The constructor must be able to:

- begin from physically available material;
- select acceptable physical material using developmental preference;
- place material using actual physical geometry;
- bond whenever legitimate physical surfaces touch;
- prefer broad, well-aligned face-to-face contact without requiring it;
- tolerate imperfect mating surfaces, unequal face lengths, partial contact, and awkward but valid orientations;
- reject unintended physical penetration;
- commit successful construction permanently;
- continue after an unsuccessful intended attachment;
- adapt through another material, surface, orientation, route, or additional structural unit;
- use additional material when the intended blueprint topology cannot be realized;
- stop only when a viable organism has been physically realized or when no physically viable continuation remains.

**Making a physically valid mistake and continuing successfully is a required behavior, not a failure mode.**

## B. Blueprint is developmental guidance, not rigid topology

The blueprint describes developmental intent, including material tendencies, spatial tendencies, density/estimated mass, preferred connectivity, and anchor information.

It does **not** require:

- exact element count;
- exact topology;
- exact closure;
- exact bond angles;
- exact face matching;
- exact spatial coordinates;
- exact reproduction of the blueprint graph.

A blueprint connection that cannot be physically realized is a preference that may be abandoned. A structurally different but physically valid and viable result is acceptable.

Construction must therefore never define failure as “an intended blueprint connection could not be made.”

## C. Physical contact authority

The physical rule is:

> **If exposed physical surfaces actually touch without unintended penetration, the contact may form a bond.**

Perfect face-to-face contact is preferred, not mandatory.

Valid construction may include:

- broad face-to-face contact;
- partial face contact;
- unequal mating surfaces;
- edge or corner contact when represented by legitimate physical contact;
- irregularly shaped mating surfaces;
- non-opposing surface normals.

Invalid construction includes unintended penetration/overlap.

There must be one physical rule for bond validity. Candidate generation may rank contacts by quality, but it must not silently turn preferred geometry into a mandatory validity condition.

## D. Construction process

The intended control flow is:

`realized physical structure
→ construction frontier
→ physically reachable candidate continuations
→ physical validity
→ preservation of viability potential
→ developmental preference
→ commit one irreversible physical change
→ recompute frontier
→ repeat`

The construction frontier is physical, not merely a list of unrealized blueprint nodes.

A candidate may be preferred because it advances the blueprint, but a physically valid alternative may be selected when the preferred continuation is unavailable.

The constructor must not backtrack committed physical construction merely because later development differs from the blueprint.

## E. Adaptive continuation

When the preferred continuation fails, the constructor must continue searching rather than terminate prematurely.

The intended adaptation order is conceptually:

1. preferred physical material;
2. acceptable physical substitute;
3. another exposed surface;
4. another valid orientation/placement;
5. another structural route;
6. additional structural material;
7. continued construction from the resulting physical frontier.

These are preferences, not a rigid retry script. The actual implementation should search the available physical continuation space rather than encode a hard-coded organism shape.

## F. Construction success and failure

**Success** is determined by the realized physical organism satisfying the authoritative viability contract.

**Failure** means that no physically valid continuation capable of reaching viability remains under the available material and construction rules.

The following are not construction failure by themselves:

- a preferred face does not mate perfectly;
- a blueprint connection cannot close;
- the realized topology differs from the blueprint;
- an additional structural unit was required;
- an acceptable substitute material was used;
- a less-preferred physical contact was necessary.

## G. Material authority

Construction operates on actual physical material.

The implementation must preserve:

- physical material identity;
- internal composite structure;
- existing constituent relationships;
- inventory ownership;
- exact consumption semantics.

Composite physical material must not be flattened merely to satisfy a blueprint preference. If a composite is restored into physical constituents and bonds, that restoration must remain physically authoritative.

No construction step may manufacture a catalog replacement when an actual physical inventory instance is required.

## H. Viability authority

Construction completion must use the existing realized-structure viability system rather than inventing a second constructor-specific definition.

The viability audit must establish exactly:

- how the realized cavity qualifies;
- what additional structure/material is required;
- how juvenile viability is determined;
- which conditions are physical invariants;
- which conditions are developmental preferences only.

A predefined core, fixed piece count, exact blueprint closure, or universal lattice topology must not become a hidden viability requirement.

## I. Focused audit and implementation order

Work proceeds in this order:

1. **Physical contact and candidate-generation audit**
   - verify that legitimate touching contacts can actually be discovered;
   - separate contact validity from preferred face mating;
   - identify and remove only genuinely conflicting geometric gates.

2. **Material restoration audit**
   - verify atomic and composite physical-material paths;
   - establish exactly how intact physical composites enter construction;
   - preserve physical identity and internal structure.

3. **Viability audit**
   - trace realized structure → cavity → genome realization → supporting structure → juvenile viability;
   - establish one authoritative construction-completion contract.

4. **Blueprint/developmental guidance audit**
   - identify every remaining place where blueprint intent is incorrectly treated as mandatory topology;
   - retain preference while removing rigid closure requirements.

5. **Constructor control-flow implementation**
   - replace blueprint-sized progression with a physical construction frontier;
   - allow adaptive divergence and additional material;
   - preserve forward-only irreversible construction.

6. **Focused adaptive-construction tests**
   - perfect construction;
   - imperfect contact;
   - unequal surfaces;
   - failed preferred connection followed by successful alternative;
   - additional structural unit;
   - substitute material;
   - valid divergent topology;
   - genuinely impossible construction.

7. **Downstream migration**
   - classify remaining failures as genuine defects, obsolete tests, or collateral regressions;
   - migrate stale tests only after the underlying contract is proven;
   - then proceed to lifecycle and full simulation validation.

## J. Verification discipline

For every implementation change:

1. inspect the exact contract being changed;
2. make the smallest isolated change;
3. format;
4. compile/check;
5. run focused tests;
6. inspect failures by dependency layer;
7. broaden validation only after the focused contract is stable.

No speculative geometry heuristic, parallel construction authority, or unrelated biological change should be introduced merely to make tests pass.

**Non-negotiable intention:** The constructor's job is to successfully build life from physical reality, not to solve a blueprint as if it were CAD. The blueprint guides development; physics determines what exists; viability determines when construction has succeeded.

