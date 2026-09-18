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

## Developmental mass and adulthood

Preferred developmental mass is a developmental preference, not an automatic adulthood threshold.

The existing fixed ADULTHOOD_GROWTH_FRACTION rule is therefore not retained as the final P6 authority.

Adulthood must ultimately be derived from realization of the adult developmental blueprint using information derived from the realized physical structure.

The exact realization criterion remains a separate P6 design/validation item and must not be replaced by an arbitrary percentage.

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

OrganismArchitecture and StructuralBlueprint currently encode discrete region/element placement information. They are legacy construction representations and cannot remain the genome's inherited body-plan authority.

During P6 they must either:

1. be replaced by the developmental field blueprint, or
2. survive only as transient solver artifacts derived from developmental fields.

They must not remain a second independent inherited structural authority.

## Exact field equations

This specification intentionally does not invent a particular mathematical equation or genome parameterization for the continuous fields.

The README establishes the required developmental information but does not establish a specific equation.

The implementation must not silently choose an arbitrary biological formula where the specification has not yet determined one.

The next P6 implementation step is therefore to define the **minimum genome parameterization needed to evaluate the continuous fields**, using existing genome information where possible and introducing no exact body-plan coordinates.

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
