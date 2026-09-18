# Phase 6.1 — Developmental Field Blueprint Specification

## Status

**Approved architectural direction; implementation boundary established.**

This specification translates the approved README blueprint rules and the owner's approved P6 recommendations into the P6 implementation boundary.

## Authority

The developmental field blueprint is the genome's inherited **developmental-intent authority**.

It is not a diagram of the organism and must not encode an exact body plan.

The authority chain is:

> Genome → Developmental Field Blueprint → Developmental Construction Solver → Physical Structure

The physical graph remains authoritative for what actually exists.

## Required developmental information

The blueprint exposes four conceptual preference domains:

1. Material-composition preference
2. Structural-density preference
3. Connectivity preference
4. Preferred developmental mass

The minimum useful developmental behavior is permitted to operate from material composition, structural density, and preferred developmental mass. Connectivity is an available developmental input and may become active when validation demonstrates that the other fields do not produce sufficient structural variation.

## Field representation

Fields are evaluated as **continuous developmental preference functions** rather than authored lists of organism parts, coordinate maps, or discrete construction diagrams.

A field evaluation describes developmental preference at a candidate location/state. It does not create or reserve a physical location.

The implementation must preserve the following distinction:

- **Blueprint:** evaluates preference.
- **Solver:** proposes and tests physical realizations.
- **Physical graph:** records realized structure.

## Explicit exclusions

The developmental field blueprint must not directly encode:

- exact material instances,
- exact unit count,
- exact coordinates,
- exact bonds,
- exact connection-point assignments,
- exact angles,
- exact rotations,
- exact silhouette,
- exact topology,
- authored branches,
- authored organs,
- or guaranteed final structure.

Any discrete placement, bond, or topology produced during construction is a **transient solver artifact** derived from developmental intent and physical constraints.

## Adult scale

The juvenile possesses the **100% adult developmental blueprint**.

Development therefore changes the degree to which the blueprint has been physically realized; it does not switch between separately authored juvenile and adult body plans.

The approximately 40% juvenile spatial realization used at birth is a construction target derived from the adult blueprint, not a separate inherited blueprint.

## Developmental mass and adulthood

Preferred developmental mass is a developmental preference.

The README explicitly states that the existing >= 90% implementation is acceptable because the governing authority is **blueprint match**, not age or an independent maturation variable.

Therefore P6 must not remove the 90% threshold merely for being a percentage. Instead, P6 must ensure that the quantity being compared against the adult target is derived from the approved blueprint realization rather than from a legacy exact structural target that has become a second body-plan authority.

The final adulthood contract must remain:

> adulthood = required match to the adult developmental blueprint.

No age, elapsed-time, reproductive-readiness, arbitrary energy threshold, or separate maturation authority may replace that rule.

## Construction boundary

The developmental solver may generate candidate construction opportunities from field evaluations, but every candidate must still pass the existing physical construction machinery:

- actual material geometry,
- connection-point compatibility,
- physical occupancy,
- existing neighboring structure,
- COMBINE admission,
- construction backtracking,
- energy/material accounting.

The blueprint cannot declare a physical bond valid by itself.

## Legacy authority migration

OrganismArchitecture and StructuralBlueprint currently encode discrete region/element placement information. They cannot remain the genome's inherited exact body-plan authority.

During P6 they must either:

1. be replaced by the developmental field blueprint, or
2. survive only as transient solver artifacts derived from developmental fields.

They must not remain a second independent inherited structural authority.

## Exact field equations and genome parameterization

The approved continuous-field representation does **not** by itself authorize arbitrary biological formulas or arbitrary new inherited parameters.

The README establishes the required developmental information but does not establish the exact equations or the minimum genome parameter set.

Existing genome traits such as environmental/resource affinities must not be silently repurposed as developmental genes merely because they are numerically convenient; their current meanings belong to other systems.

The next design step is therefore to identify the minimum new inherited information required to express the approved developmental fields and present that exact parameterization for approval before adding it to Genome.

## Non-goals

P6.1 does not redesign:

- genome identity/life definition,
- genome cavity,
- chemistry,
- COMBINE/BREAK,
- energy accounting,
- maintenance/stress,
- movement/collision/pushing,
- reproduction,
- death,
- decomposition,
- environmental material semantics,
- or evolution.
