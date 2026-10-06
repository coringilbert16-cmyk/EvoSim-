# EvoSim

EvoSim is an open-ended evolutionary organism simulation. The organism is built from physical material and physical bonds; the simulation should produce structure and behavior from general mechanisms rather than a fixed predator/prey model or a prescribed organism-size progression.

## Current construction architecture

The initial-organism constructor now uses the local physical construction model.

The intended constructor is **free-form and cavity-driven**:

- it does not begin from a fixed carbon ring, spoke layout, spiral, or outer shell;
- it does not know in advance how many Carbon units must surround the genome;
- rigid constituents have no preferred material; each shape naturally participates in the exact contacts its geometry permits;
- the resulting topology and silhouette are consequences of local physical placement and contact, not a prescribed body plan;
- the genome is a qualifying empty cavity discovered from the realized physical graph;
- once a qualifying cavity emerges, the constructor continues local growth until the ordinary initial viability contract is satisfied;
- additional structure is added only through locally valid physical contacts;
- a qualifying cavity may have any number of boundary constituents permitted by the geometry; no number such as 10 is biologically significant.

The constructor is forward-only. A committed physical bond is permanent. There is no global future-body search and no backtracking to repair an earlier construction decision. A placement is a physical preference, not an instruction that the final graph must reproduce.
Genesis search is frontier-local: only currently exposed, unbonded physical endpoints are construction anchors, and broad-phase geometry rejects distant candidates before exact contact/penetration work. This is a structural performance rule, not an arbitrary attempt budget; constructor realization should reach the millisecond range for viable genesis. The current optimization work therefore removes speculative whole-organism cloning from the normal forward-growth path rather than treating a one-minute ceiling as the target.

## Unified bonding rule

There is no separate “forward bond” phase and “closure bond” phase.

For every newly positioned physical constituent:

1. place it using a geometrically valid local placement;
2. discover every exact physical contact created by that placement;
3. evaluate all qualifying bond candidates;
4. order qualifying candidates by **descending investment cost**;
5. form every affordable, still-valid bond through the shared physical bond transaction;
6. revalidate endpoint availability after each committed bond;
7. retain the resulting physical graph as authoritative.

Corner-to-corner, corner-to-edge, edge-to-edge, and other supported contact types are all ordinary physical bonds. The contact system owns exact contact geometry; the bond system consumes the already-resolved contact rather than rediscovering it.

The only construction invariant beyond physical validity is connectivity: a newly committed constituent must actually join the developing organism. This is a consequence of construction, not a special class of “closure” bond.

## Genome formation

Genome formation is a real construction milestone, not a post-build assertion.

The qualifying cavity is determined from the realized geometry and bond graph. The cavity analyzer independently decides whether an empty region qualifies; the constructor does not create or name a predefined genome core.

After qualification:

- the cavity itself is the genome;
- the surrounding boundary constituents remain ordinary physical material;
- the constructor continues forward from the realized organism;
- no predefined core or fixed piece count is required.

The genome is therefore an emergent structural property that can change as the organism's realized geometry changes.

## Initial viability

The first organism must establish:

- a qualifying physical genome cavity;
- Water as an acquirable resource;
- any three additional resources selected from the available catalog;
- physical ability to acquire each selected resource.

The acquisition requirement is binary per selected resource: each selected resource must be physically acquirable. The constructor does not require a particular resource trio, fixed body size, or predefined piece count.

If construction later requires a resource that is temporarily unavailable, the intended behavior is to wait and resume from the same committed graph when material becomes available. Resource waiting is a construction state, not a reason to backtrack or block the simulation.

Genesis begins with one available rigid physical constituent only to instantiate the first cell. That initial choice is not a biological material preference. Every subsequent rigid material is an equally eligible construction candidate; inherited material similarity is used only when a developmental preference actually exists.

## Geometry authority

The geometry pipeline is:

`realized shape → exact boundary features → exact contact → bond → realized graph`

Canonical boundaries expose actual physical features. Polygon vertices are represented as corners; non-vertex rigid boundary contacts are represented by exact boundary points; circular boundaries use their angular representation; fluid boundaries use realized physical points.

Exact contact discovery supports point/edge and edge/edge relationships without replacing physical geometry with an arbitrary angular grid. The selected physical contact is passed directly into bond formation.

Physical endpoint identity is based on the resolved world location and constituent identity, so equivalent physical locations cannot accidentally become duplicate bonds merely because they use different endpoint representations.

## Material and fluid rules

Physical geometry is authoritative.

Water is a normal physical material when instantiated. It is not logical material placed into storage to satisfy a viability condition. Fluid behavior belongs to the general fluid physical state rather than to Water specifically.

A fluid begins with its resource-defined default geometry, then may deform when physical contact and surrounding structure require it. Its physical amount/volume constrains the realized geometry.

The genome cavity is genuinely empty and is part of the organism rather than the surrounding environment. Structural membership follows bonded connectivity to the genome; water can participate in structure when it has a physical connection to the organism.

## Developmental preferences

A developmental blueprint may eventually provide inherited spatial and material preferences, but it is never authoritative topology.

The physical constructor has no hard-coded intrinsic material preference. When an inherited developmental material preference exists, material similarity may rank available physical constituents and enforce the minimum similarity threshold; geometry then determines whether the selected candidate can actually be placed and bonded. Material identity is therefore not globally preferred, and the constructor must not:

- require future blueprint edges to be realized;
- force a prescribed number of constituents;
- force a prescribed cavity shape;
- perform a global placement search;
- backtrack committed bonds.

This distinction is important for evolution: inherited structure preferences can influence construction while physical geometry determines the realized organism. Mutations can therefore change construction tendencies without making the genome a hard-coded body plan.

## Verification discipline

For each architectural change:

1. inspect the exact physical path and contract being changed;
2. make the smallest coherent implementation change;
3. run formatting and compilation;
4. run focused constructor/contact/cavity tests;
5. inspect the actual failure before changing another layer;
6. broaden validation only after the changed contract is proven.

Downstream failures should not be used to reintroduce retired constructor assumptions.

## Project direction

The long-term goal remains an open-ended simulation in which organisms can develop structure, acquire resources, sense their environment, reproduce, form niches, and potentially evolve multicellular cooperation from the same general physical and behavioral mechanisms.

The immediate construction goal is narrower: **build the first valid organism from physical rules rather than from a hidden body plan.**
