# PR #225 — Environmental Chemistry Building-Blocks Validation

> **Planning document.** This experiment validates the chemical capabilities needed to build an organism later. It does **not** attempt to make chemistry spontaneously assemble an organism or reproduce life.

## Purpose

Determine whether EvoSim's shared physical and chemical rules provide the building blocks needed for later organism construction.

The experiment is deliberately organism-free. We are testing chemistry itself: whether ordinary material can combine, break, exchange constituents through displacement, participate in multi-step reaction pathways, and account for energy and matter correctly. A cavity, organism, or self-reproducing structure is not an acceptance criterion.

No reaction recipe, forced collision, manually injected operation, special hotspot-break rule, or weakened chemistry threshold is permitted. Controlled molecular fixtures are allowed to isolate a reaction mechanism; they are chemistry test specimens, not manually assembled organisms.

## Integrated dependencies

- **#214** shared chemistry runtime/scheduler — merged.
- **#223** bounded environmental COMBINE discovery and settlement — merged.
- **#226** S-shaped flow, gradients, eddies, and obstacle-aware movement — merged.
- **#229–#234** base-resource pair matrix, controlled physical-pair classification, product characterization, compound/base chemistry, compound/compound contacts, and committed multi-constituent COMBINE — merged.
- **#235–#240** molecular composition/topology, multi-constituent COMBINE/BREAK round-trip evidence, heterogeneous interfaces/reaction keys, fair rotating contact discovery, and geometry-derived layout inputs — integrated into main through #240.
- **#241** cohesion/reactivity-derived natural BREAK tolerance — merged.
- **#243** resonance-funded BREAK without arbitrary processing efficiency — implementation under verification.
- **#244** keyboard-only playback, stepping, and reset controls — merged.
- **#245** right-bend radial hotspot and resonance-coupled environmental energy — implementation under verification.

## Experiment structure

The experiment has several independent chemistry proofs. They share the same environmental material model, reaction rules, resonance system, and energy/material accounting. They do not require a single long run to exercise every mechanism.

### A. Natural environmental COMBINE

- Begin with exactly **128 individual whole-unit resources total**, with all seven canonical base-resource types represented.
- Randomize individual resource identities; do not prescribe fixed type ratios or a reaction recipe.
- Scatter resources across the map in reproducibly randomized piles of **6–20 units**. Pile sizes must sum to exactly 128.
- Use the existing S-shaped environmental flow and ordinary simulation ticks to produce transport and recurring contact opportunities. Flow may interrupt contact; do not pin reactants, script collisions, or inject COMBINE operations.
- Prove live discovery, activation, scheduling, geometry validation, and atomic COMBINE commit.
- Record actual product constituents, topology, interfaces, and bond-energy allocation.

### B. Natural environmental BREAK

- Use physically realized compounds as controlled chemistry specimens in an organism-free environment.
- Let ordinary environmental BREAK discovery, activation, scheduling, and settlement determine whether bonds dissociate.
- Test multiple constituent and bond types, including products created by COMBINE.
- Record the exact fragments and their physical recovery. No direct molecule damage or hotspot-specific BREAK path is allowed.

### C. Displacement and reaction chaining

- Test whether new compatible interactions can displace existing bonds when geometry and chemistry permit.
- Test whether products from one reaction can participate in later COMBINE or BREAK operations.
- Distinguish a merely possible reaction from a discovered, activated, scheduled, and committed reaction.
- Preserve exact constituent identity; do not split atoms or use fractional material.

### D. Energy coupling and the hotspot

- Enable the one right-bend radial hotspot only in the chemistry experiment.
- Its center supplies **100 energy per tick per unit of material area**, with a smooth radial cosine falloff. The default 1000×1000 world uses a 150-unit radius.
- The hotspot adds an ordinary 440 Hz resonant signal. It does not directly break or damage material.
- Verify material-specific resonance capture, usable-energy availability, damping/heat, and escaped energy.
- Verify whether both COMBINE and BREAK use the intended resonance-coupled energy pathway. Current source audit shows environmental COMBINE settlement still spends `chemistry_usable_energy`; that is a known integration question, not something the experiment should conceal or bypass. Do not count hotspot resonance as COMBINE funding until the commit path actually consumes compatible resonance.

### E. Conservation and reproducibility

- Start from a saved, reproducible tick-zero state and record the random seed and generated fixture.
- Maintain an independent integer material inventory across every reaction and transfer.
- Reconcile energy across external input, usable energy, resonance, structural potential, work, heat, and escaped signal energy.
- Verify atomic rollback for stale sources, insufficient energy, invalid geometry, and rejected settlement.
- Keep discovery and realization budgets bounded and report remaining work so an uneventful tick is distinguishable from an exhausted search budget.

## Diagnostic requirements

The run must produce machine-readable, inspectable diagnostics rather than relying on the canvas:

- Per-tick COMBINE and BREAK ready candidates, scheduled operations, committed operations, and rejected settlements.
- Reaction details: reactants, products/fragments, constituent multiset, bond/interface changes, reaction frequency, and work/energy amounts.
- Material inventory by base-resource type, physical object count, constituent count, and internal bond count.
- Energy inventory: external input, environmental usable energy, environmental resonance, structural energy, work, heat, escaped energy, and conservation residual.
- Hotspot center, radius, local intensity, and captured/heat/escaped totals.
- Reproducible seed and initial-state summary.

A failed reaction must be distinguishable from no candidate, insufficient activation, insufficient compatible resonance, stale geometry, invalid product geometry, or an atomic settlement rejection wherever the current implementation exposes that reason. If a reason is not yet observable, record that as a diagnostics gap rather than inventing a cause.

## GUI companion

Use the existing world visualization and the same experiment state; do not build a separate simulation or local setup workflow.

- Render the real hotspot as a radial gradient at its configured right-bend location.
- Keep playback controls keyboard-driven: Space pause/play, `<` one tick backward, `>` one tick forward, Escape reset to tick zero.
- Provide diagnostic readouts and event inspection sourced from simulation state, not a parallel GUI model.
- Preserve the ability to inspect historical ticks and reproduce an interesting reaction from the saved initial state.

## Explicit exclusions

- No requirement that raw chemistry spontaneously assemble an organism, cavity, genome, or reproducing life.
- No manually constructed organism or scripted biological behavior.
- No forced pair completion, scripted pair contact, lowered activation thresholds, or test-only operation injection.
- No direct hotspot damage, forced BREAK, or separate environmental-only reaction law.
- No fractional atoms or reinterpretation of aggregate stock as a realized molecule.
- No duplicate flow implementation; #226 owns transport.
- No changes to the default production seed or ordinary simulation preset.

## Acceptance gate

The chemistry building-blocks experiment is complete when it demonstrates, through ordinary chemistry rules and controlled reaction specimens where appropriate:

1. Natural COMBINE from raw environmental reactants.
2. Natural environmental BREAK and physical fragment recovery.
3. Displacement and multi-step reaction chaining.
4. Correct resonance/energy coupling for every reaction type claimed as complete.
5. Exact material conservation and balanced energy accounting.
6. Reproducible, bounded execution with diagnostics sufficient to explain what happened.

A cavity or organism is neither required nor expected in this experiment. The result should tell us whether the chemistry toolkit is ready to be used in a later organism-construction phase.
