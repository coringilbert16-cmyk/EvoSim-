# Phase 6 — Growth & Development

## Status

**P6.0–P6.6 COMPLETE — FINAL README AUTHORITY AUDIT PASSED**

CI validation passed, and the implementation has completed the required separate README authority audit.

The governing pipeline is:

> Genome → Developmental Field Blueprint → Construction/Development Solver → Physical Structure

The physical graph remains authoritative for realized structure.

## P6.0 — Authority audit — COMPLETE

- Removed the legacy discrete OrganismArchitecture / exact-body-plan construction path from inherited developmental authority.
- Confirmed the existing construction runtime contains reusable physical candidate generation, backtracking, COMBINE admission, and physical validation machinery.
- Adulthood uses the existing >=90% threshold against continuous developmental realization derived from the authoritative physical graph.
- Confirmed no new viability, death, maturation, or lifecycle authority is required.

## P6.1 — Developmental-field blueprint authority — COMPLETE

### Approved rules implemented so far

- Added inherited `size_preference` as the developmental-size authority, normalized to [0, 1].
- Preferred developmental mass is derived from size preference using the approved logarithmic mapping.
- Size-preference mutation is bell-shaped around the parent's value and bounded to [0, 1].
- Preferred developmental mass is soft intent; actual mass remains derived from realized physical structure.
- The current `adult_mass()` API is retained only as an implementation-facing derived preferred-mass accessor; it is not an independent genome authority.
- The numerical mass bounds used by the current mapping are explicitly **experimental**.
- Removed the discrete size/count ladder and canonical juvenile-count authority.
- Preserved the confirmed-good juvenile seed only as a physically validated solver starting realization, not as a genome body-plan rule.
- Connectivity is implemented as a developmental preference and physical-opportunity measurement; its numerical neighborhood coefficient remains experimental.
- The confirmed original seed realization survives only as a non-inherited construction/scale calibration baseline.

## P6.2 — Developmental solver — COMPLETE

The approved developmental equations are implemented as a physical-graph realization API and as developmental scoring inside the ongoing COMBINE construction path. Candidate generation first enforces physical validity, then developmental intent ranks the valid candidates. Current field widths, influence locations, candidate-score weights, mass bounds, and connectivity neighborhood coefficient are explicitly **experimental**.

The confirmed original seed realization is used only as the juvenile construction/calibration baseline. It is not serialized into genomes and does not define descendant topology.

## P6.2a — Developmental realization mathematics — COMPLETE

The developmental realization equations are now locked:

- material realization = developmental material-field overlap with realized physical material area;
- density realization = developmental density-field overlap with realized physical structure area;
- connectivity realization = realized physical edges versus the union of actual edges and physically admissible connection opportunities;
- connectivity neighborhood contribution is derived from realized/available connection-site occupancy;
- overall realization = arithmetic mean of active realization domains;
- inactive connectivity contributes no penalty;
- adulthood remains R >= 0.90.

No tunable realization-weight parameter is introduced. Candidate-selection weights remain separate experimental solver parameters.

The mathematical authority is:

> continuous inherited developmental fields + authoritative physical graph -> developmental realization

An authored exact body plan, target coordinate list, or transient structural blueprint cannot serve as the adulthood authority.

## P6.2b — Persistent developmental coordinates — IMPLEMENTED

Organisms now retain a persistent organism-local and persistent developmental origin and initial orientation. Movement translates that frame with the organism; it is never re-centered on center of mass, bounding box, or new growth. Offspring initialize a new frame at their own viable seed.

The realization API evaluates physical structure in that persistent frame. Finite Gaussian realization denominators are required mathematically; the current zero-falloff default fields therefore remain unavailable as finite realization domains until their experimental field parameterization is established.

## P6.3 — Growth integration — COMPLETE

Developmental field scoring is routed into the ongoing juvenile COMBINE construction path. It is a preference over physically valid construction opportunities, not a one-time authored target or selectable body-plan action.

## P6.4 — Physical growth contracts — COMPLETE

Existing realized material remains physical authority; new structure uses actual geometry and connections; bond admission remains through COMBINE; developmental intent cannot make an invalid physical construction valid or rewrite realized structure.

## P6.5 — Development/adulthood audit — COMPLETE

DevelopmentStage is driven by continuous developmental realization. No age, reproductive-readiness, arbitrary energy, preferred-mass equality, or independent maturation authority is used.

## P6.6 — Contract validation — COMPLETE

Full tests, architecture checks, formatting, strict Clippy, and the separate README authority audit all passed. P6 is complete on this branch.

## Non-goals

P6 does not redesign genome identity/life definition, genome-cavity qualification, chemistry, COMBINE/BREAK chemistry, energy accounting, maintenance/stress, movement/collision/pushing, reproduction, death, decomposition, evolution, or environmental material physics.


P6 completion gate: CI passes and the implementation is explicitly re-audited against README authority. Both gates are satisfied.


## P6 Approved Developmental-Scale Parameterization Amendment

The approved P6 developmental-field equations are now paired with the following implementation parameterization:

- The initial representation uses **4 radial influences per developmental field**. The count of four is the approved starting representation; the influence count remains **EXPERIMENTAL** and may change after validation.
- Each Gaussian influence uses the approved form \(K_i(\mathbf p)=e^{-\|\mathbf p-\mathbf c_i\|^2/(2\sigma_i^2)}\).
- Influence width is derived from preferred developmental scale: \(\sigma_i=\alpha_i L_p\). The \(\alpha_i\) values are **EXPERIMENTAL** representation parameters.
- Preferred linear scale is derived from the confirmed-good initial seed realization only as a calibration reference: \(L_p=L_{seed}\sqrt{M_p/M_{seed}}\). This is a scale relationship, not an inherited seed body plan. The comparable 2-D mass scaling is therefore \(M_J\approx0.40^2M_p\) for the approved 40% juvenile linear realization.
- Influence centers and strengths are **EXPERIMENTAL** numerical parameters. The initial centers and strengths in the default genome are implementation starting values, not biological constants.
- Developmental-field sums are normalized by their total influence strength so field shape is not confounded with absolute amplitude.
- Material realization uses composition-weighted physical area for composite units: each constituent contributes according to its fraction of the unit's material amount rather than counting the entire composite area once per constituent.
- Connectivity uses physically available endpoint opportunities and keeps the neighborhood coefficient \(\lambda\) **EXPERIMENTAL**. Candidate-selection weights \(w_M,w_D,w_K\) are also **EXPERIMENTAL** solver parameters.
- Numerical size-preference mass bounds remain **EXPERIMENTAL**.

No experimental parameter above is an additional biological authority. Changing one changes the experiment within the approved P6 architecture.
