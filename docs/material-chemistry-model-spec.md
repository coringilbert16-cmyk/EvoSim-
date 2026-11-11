# EvoSim material and chemical interaction model

**Status:** Proposed implementation contract  
**Scope:** Six active non-Water resources; Water remains in the catalog and outside the active chemistry experiment matrix.  
**Branch:** `chemistry-material-model-spec`

## 1. Design contract

EvoSim must represent these as distinct properties:

- **Adhesion:** a symmetric, pair-specific interaction. It determines whether two materials attract or repel and the depth of their interaction well.
- **Intrinsic cohesion:** a resource-level structural property. It determines how resistant a realized connection is to disruption.
- **Bond integrity:** the local resistance of one actual bond. It is derived from the intrinsic cohesion of its endpoints, with realized contact-feature/load effects applied by the existing structural authority.
- **Emergent cohesion:** a property of a compound's actual bond graph. It measures resistance to separating the compound into disconnected pieces.
- **Reactivity:** kinetic response / activation sensitivity. It does not create energy and does not determine adhesion.
- **Resonance:** a local activation and energy-coupling state. It is not an energy source.
- **Potential energy:** a stored energy state or a type-level capacity, never an interaction score.
- **Geometry and valence/capacity:** determine which physical connections can exist. They are not substitutes for chemical compatibility.

No single scalar may silently serve as adhesion, bond integrity, compound stability, and stored energy.

## 2. Resource-level adhesion descriptors

Replace `chemical_position` as the active pair-interaction input with four explicit, bounded descriptors:

| Descriptor | Domain | Meaning |
|---|---:|---|
| Dispersion (`dispersion`) | [0, 1] | Broad, non-directional attraction available to any pair, including identical materials |
| Signed polarity (`polarity`) | [-1, 1] | Interaction orientation; opposite signs contribute attraction and like signs contribute repulsion |
| Donor propensity (`donor`) | [0, 1] | Tendency to participate in donor-like complementary interactions |
| Acceptor propensity (`acceptor`) | [0, 1] | Tendency to participate in acceptor-like complementary interactions |

These are abstract simulation descriptors, not claims that EvoSim models electrons or literal quantum chemistry. Each has an independently observable contribution. The descriptor set is deliberately small; do not add more channels unless diagnostics demonstrate a missing independent behavior.

### Initial six-resource descriptor values

These are explicit starting hypotheses for the simulation's designed chemistry, not experimentally measured elemental constants. Keep them fixed for the first validation cycle; tune only from pair-matrix and reaction evidence.

| Resource | Dispersion | Polarity | Donor | Acceptor | Existing intrinsic cohesion |
|---|---:|---:|---:|---:|---:|
| Carbon | 0.80 | 0.00 | 0.25 | 0.25 | 0.95 |
| Oxygen | 0.55 | -0.80 | 0.10 | 0.95 | 0.10 |
| Hydrogen | 0.25 | 0.35 | 0.65 | 0.10 | 0.05 |
| Sulfur | 0.90 | -0.20 | 0.50 | 0.55 | 0.45 |
| Nitrogen | 0.65 | 0.15 | 0.75 | 0.70 | 0.70 |
| Phosphorus | 0.85 | 0.55 | 0.45 | 0.35 | 0.60 |

The current cohesion values are retained as intrinsic cohesion for the first pass. They must no longer influence pair adhesion or stored bond energy. Water remains excluded from the active six-resource matrix; its fluid/capillary behavior needs a separate compatibility migration before Water is returned to active chemistry.

## 3. Symmetric pair calculation

For resources (i,j), calculate four channel contributions:

[
D_{ij}=\sqrt{d_i d_j}
]

[
P^+_{ij}=\max(0,-p_i p_j), \qquad
P^-_{ij}=\max(0,p_i p_j)
]

[
H^+_{ij}=\frac{don_i acc_j+acc_i don_j}{2}
]

[
H^-_{ij}=\frac{don_i don_j+acc_i acc_j}{2}
]

Then calculate attraction, repulsion, and signed net adhesion:

[
A_{ij}=\frac{D_{ij}+P^+_{ij}+H^+_{ij}}{3}
]

[
R_{ij}=\frac{P^-_{ij}+H^-_{ij}}{2}
]

[
J_{ij}=A_{ij}-R_{ij}
]

All values are symmetric under exchanging (i) and (j). Each channel is normalized to [0,1] and receives equal weight; there are no hidden pair-specific constants. A positive (J) is net attractive, zero is neutral, and negative (J) is net repulsive. The attractive and repulsive components remain separately available to diagnostics.

This construction permits nonzero same-resource interactions. It does not force homonuclear pairs to attract: their outcome emerges from dispersion, polarity, and complementary-role repulsion.

### Initial pair-matrix prediction

Applying the descriptors and equations above gives the following signed net adhesion values. These are the baseline predictions to compare against the 21-pair physical contact/activation census.

| Pair | Net adhesion | Expected interaction |
|---|---:|---|
| C–C | +0.256 | attractive |
| C–O | +0.199 | attractive |
| C–H | +0.133 | attractive |
| C–S | +0.261 | attractive |
| C–N | +0.210 | attractive |
| C–P | +0.258 | attractive |
| O–O | -0.333 | repulsive |
| O–H | +0.282 | attractive |
| O–S | +0.100 | attractive |
| O–N | +0.185 | attractive |
| O–P | +0.357 | attractive |
| H–H | -0.064 | repulsive |
| H–S | +0.154 | attractive |
| H–N | +0.057 | weakly attractive |
| H–P | +0.021 | weakly attractive |
| S–S | +0.234 | attractive |
| S–N | +0.202 | attractive |
| S–P | +0.294 | attractive |
| N–N | +0.117 | attractive |
| N–P | +0.157 | attractive |
| P–P | +0.103 | attractive |

A positive matrix entry is not a promise that a bond forms. Contact geometry, available connection capacity, activation, physical work, and energy affordability remain independent gates. In particular, weakly positive pairs must remain observably different from strongly attractive pairs.

## 4. One conservative pair potential for force and stored bond energy

The current model calculates attraction, formation work, and stored bond energy through partially separate conversions. Replace those with one conservative pair-potential curve so force, work, and stored energy cannot disagree.

Let (x) be the current separation measured outward from contact, (R_c) the interaction radius, and (J_{ij}) the signed net adhesion:

[
U_{ij}(x)=
-\frac{F_{max}R_c}{3}J_{ij}
\left(1-\frac{x}{R_c}\right)^3
\quad (0\le x\le R_c)
]

Outside the interaction radius, (U=0). The force is the negative spatial derivative of this same potential; its magnitude is:

[
F_{ij}(x)=F_{max}J_{ij}
\left(1-\frac{x}{R_c}\right)^2
]

Thus the existing contact-falloff shape is retained, but the stored well depth and approach work are now mathematically consistent. At contact, the stable-bond energy magnitude is:

[
E^{bond}_{ij}=\frac{F_{max}R_c}{3}\max(J_{ij},0)
]

Use the existing approved (F_{max}) and (R_c) as the initial physical scales. Do not add a second arbitrary bond-energy multiplier. If calibration later changes the force scale, the well depth changes with it automatically.

A negative (J) produces repulsion and cannot create a stable attractive bond. A zero or negative net adhesion has zero attractive bond-energy depth. Pair repulsion is still recorded and can prevent approach/contact; it is not converted into a stable bond.

**Energy rule:** formation work is the integral of this force over the actual remaining approach distance. Stored structural potential is the well depth assigned to the realized bond. The reaction ledger compares actual pre/post structural potential and physical work once. Do not charge the same well depth again as a separate activation cost.

## 5. Intrinsic cohesion and local bond integrity

Retain the current resource cohesion values as the initial intrinsic-cohesion scale. The current first-pass local bond integrity remains:

[
I_{ij}=\sqrt{C_i C_j}
]

This reuses the existing geometric-mean relation, but changes its meaning and authority: it is a structural resistance measure only. Apply the existing realized contact-feature and load modifiers in the structural authority; do not duplicate them in chemistry.

Consequences are intentional:

- High adhesion / low cohesion: easy to form, comparatively easy to disrupt.
- Low adhesion / high cohesion: difficult or impossible to form, but durable if formed.
- High adhesion / high cohesion: favorable and durable.
- Low adhesion / low cohesion: unfavorable and fragile.

Reactivity may lower or raise the activation barrier according to the existing approved activation equation. It must not change (J), (E^{bond}), or (I).

Cohesion also currently participates in non-chemical systems (notably resonance response and capillary/contact-angle adapters). Keep those call sites explicit during migration. Where cohesion is being used as a surface-wetting proxy, replace that dependency with a pair-specific adhesion/wetting adapter rather than silently making structural cohesion control adhesion again. Preserve existing fluid geometry behavior while Water remains excluded from the active chemistry experiments.

## 6. Emergent cohesion from compound topology

Represent a compound as its actual constituent/bond graph:

- vertices are physical constituent identities;
- edges are realized bonds;
- edge weights are local bond integrity (I_e).

Define overall emergent cohesion as the weighted minimum cut:

[
C_{emergent}(G)=
\min_{\emptyset\ne S\subsetneq V}
\sum_{e\in\delta(S)} I_e
]

This measures the weakest total bond-integrity boundary separating the compound into two nonempty groups.

Expected topology behavior:

- A chain is limited by its weakest bridge.
- A ring gains redundancy because a split requires cutting multiple edges.
- A branched structure can have a weak branch attachment.
- Adding an unbonded constituent does not increase cohesion.
- Adding a redundant bond can increase emergent cohesion without changing the intrinsic cohesion of any constituent.

The minimum-cut value is a compound-level diagnostic and structural descriptor, not a substitute for local BREAK evaluation. BREAK must assess the selected bond, its local integrity, activation, resulting connected components, and the complete energy transaction. A ring bond can be removed without splitting the graph; a chain bridge may split it.

Compute from the graph. Cache only with a topology/version key that guarantees invalidation after every bond addition/removal.

## 7. Reaction responsibilities

| Mechanism | Primary rule | Independent constraints |
|---|---|---|
| COMBINE | Positive pair adhesion at realized contact | Geometry, connection capacity, activation, formation work, and available energy |
| BREAK | Local bond integrity and activation | Resulting graph topology, disruption work, and exact energy settlement |
| DISPLACEMENT | One atomic old-bond removal / replacement-bond formation trial | Incoming-contact activation, both bond integrities, resulting topology, and complete net energy budget |
| Resonance | Local activation and energy coupling | Only energy actually withdrawn from a named reservoir may fund a reaction |

### DISPLACEMENT correction

The production trial builder currently rejects any replacement whose stored bond energy is not strictly greater than the old bond's energy. Retire that rule. A replacement's favorability is determined by the complete pre/post structural-potential difference plus formation/disruption work and dissipation.

A weaker replacement may proceed when activation is sufficient and a legitimate energy holder funds the complete deficit. A stronger replacement is not automatically valid if geometry, activation, or energy accounting fails. The old bond removal and replacement formation remain one atomic transaction; never implement displacement as an independently committed BREAK followed by COMBINE.

## 8. Required migration and validation sequence

1. Add validated adhesion descriptors to catalog resources, with serialization defaults/migration behavior explicitly tested.
2. Implement the symmetric pair calculation and a diagnostic record exposing (D,P^+,P^-,H^+,H^-,A,R,J).
3. Validate all 21 unordered non-Water pairs, including the six homonuclear pairs. Assert symmetry, finite ranges, deterministic output, and descriptor independence.
4. Replace chemical-position-based COMBINE attraction, formation work, and bond-energy assignment with the single conservative pair potential.
5. Retain cohesion only for intrinsic/local structural integrity; audit every cohesion call site and separate surface wetting where required.
6. Add graph-derived emergent cohesion and topology tests for chains, rings, branches, redundant edges, and fragmentation.
7. Integrate the same bond-integrity and energy rules into BREAK and atomic DISPLACEMENT. Remove the stronger-only replacement gate.
8. Validate energy conservation, including weaker-but-funded replacement, stronger-but-unfunded replacement, failed activation, and atomic rollback.
9. Run the existing controlled pair/compound feasibility tests before any longer environmental chemistry experiment.

## 9. Non-goals and safeguards

- No organism-specific chemistry exceptions.
- No arbitrary COMBINE energy grants.
- No activation signal treated as an energy source.
- No separate BREAK then COMBINE displacement.
- No forced zero for same-element interactions.
- No compound-wide cohesion scalar used as every bond's BREAK threshold.
- No inference that a favorable pair must react without geometry, activation, and energy.
- No Water reintroduction into the active six-resource experiments in this migration.
- Do not tune descriptors and reaction thresholds simultaneously. First validate pair predictions, then validate mechanics, then run broader chemistry.

## 10. Acceptance criteria

The model is ready for implementation only when all of the following are testable:

- Pair interaction is symmetric and decomposable into independently visible attraction and repulsion channels.
- All 21 active pairs use the same equations; homonuclear pairs have calculated, not hard-coded, outcomes.
- Adhesion changes do not alter intrinsic cohesion; cohesion changes do not alter pair adhesion or stored bond potential.
- Stored bond energy, force, and formation work derive from one conservative potential.
- Local bond integrity and emergent graph cohesion respond to different inputs and answer different questions.
- BREAK evaluates the selected edge and post-break topology.
- DISPLACEMENT supports any energy-feasible replacement, not only a stronger bond, and commits atomically.
- Every transaction balances without double-counting bond energy, activation, work, heat, or external input.
