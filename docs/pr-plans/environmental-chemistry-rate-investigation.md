# Environmental chemistry rate investigation

## Purpose

Explain the low observed environmental reaction count before changing chemistry rules or work budgets. Keep this investigation on the environmental displacement PR branch until its diagnostic changes and experiment setup have been reviewed.

## Existing baseline evidence

Two independent duration-series artifacts were supplied for review. Each used the standard 1,280-object, organism-free fixture and seed 42. In the 600-second runs, one committed 65 COMBINE reactions and the other 66; both recorded zero BREAK and displacement commits. Their final material inventories matched their initial inventories, and energy residuals were small. The current logs do not establish whether few bonded reactants encountered incoming materials, whether contacts accumulated too little activation, or whether the discovery budget limited the search.

## Experiment 1: replicated standard-environment baseline

Run the unchanged standard fixture for 600 wall-clock seconds at seeds 42, 7, and 101. The first seed matches the prior observation; the other seeds test sensitivity to randomized identities and pile locations. Do not inject contacts, activation, energy, or reactions.

Run from the repository root:

```powershell
.\scripts\run_environmental_chemistry_replicates.ps1
```

The script writes one JSONL diagnostics artifact and one console log per seed under the ignored `.evosim-experiments/` directory. The default 100-tick diagnostic interval exposes progress during a run without writing a full world snapshot every tick. Use `-DurationSeconds 300` for a shorter first pass or `-Seeds 42` to reproduce the original seed. Verbose material-restoration rejection details are opt-in through `EVOSIM_DEBUG_MATERIAL_RESTORATION=1` so normal experiment output stays readable.

### Readouts

- Reaction counts and rates: COMBINE, BREAK, and displacement per 1,000 ticks and per wall-clock minute.
- Displacement discovery funnel: anchor materials, bonded anchors, neighboring target cells/materials, replacement-contact searches and valid contacts, activation updates and threshold checks, post-activation trial rejections, ready/scheduled/committed/rejected operations.
- Work pressure: displacement discovery work remaining each interval and number of ticks that exhaust the assigned share.
- State integrity: initial/final constituent inventories, physical-object counts, and energy-balance residuals.
- Product persistence: participant combinations and resulting product components for every committed reaction.

### Interpret the first limiting stage

- Few bonded anchors: COMBINE has not made enough bonded environmental material for displacement to use.
- Bonded anchors but few target materials: spatial neighborhood coverage or pile layout limits encounters.
- Many target materials but few valid replacement contacts: current geometry/contact compatibility limits candidate formation.
- Valid contacts and activation updates, but activation remains below the old-bond barrier: incoming interaction or contact persistence is limiting.
- Activation reaches the barrier but trial builds fail: inspect trial topology, product export, and energy preflight.
- Ready candidates accumulate while discovery or realization work is exhausted: investigate scheduling and work allocation separately from chemistry validity.
- Scheduled reactions reject at commit: investigate stale sources and funding/settlement reasons before changing activation.

Do not tune activation thresholds or add a displacement bonus based only on the number of commits. First identify which funnel stage is limiting and preserve matter/energy invariants.

### Replicated baseline results

The completed 600-second runs produced the following summary:

| Seed | Ticks | COMBINE | Valid replacement contacts | Activation reached | Trial rejected after activation | Main rejection stage | Displacement commits | Energy residual |
| --- | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: |
| 42 (first baseline) | 8,410 | 71 | 4 | 2 | 2 | Not yet instrumented | 0 | -3.30e-5 |
| 7 | 8,451 | 65 | 4 | 4 | 4 | Not yet instrumented | 0 | -2.69e-5 |
| 101 | 7,059 | 61 | 3 | 0 | 0 | Not applicable | 0 | -2.37e-5 |
| 42 (failure-stage follow-up) | 8,497 | 72 | 4 | 2 | 2 | Stored old-bond energy differed from the value recomputed from its endpoint properties and strength | 0 | -3.36e-5 |

The failure-stage follow-up performed 44,248 replacement-contact searches and exhausted the displacement discovery share on all 8,497 ticks. It preserved the exact constituent inventory. Both activated candidates stopped at the old-bond energy consistency check. The fixture seeds 40 H-S and 40 C-H composition-only molecules whose bonds intentionally have `bond_energy = 0.0`: these are structural bonds, not allocated energy stores. The previous displacement gate incorrectly compared that recorded zero against a recomputed pair potential and rejected the trial. The correct transaction must preserve the recorded old-bond energy (including zero), calculate the replacement bond energy through the same pair-adhesion authority as COMBINE, and settle the complete signed structural-energy change plus physical work once. No energy may be inferred or created for the old seed bond. The discovery work budget is exhausted frequently, but the observed trial failures occurred after valid contacts and activation; budget exhaustion was not the immediate rejection cause.

## Next experiments

Run these in order and keep the unchanged standard baseline as the control:

1. **Validate zero-allocation replacement.** Keep the seeded H-S/C-H bonds at zero stored energy. Confirm a trial can pass activation and replacement validation, and that the resulting new bond's energy plus disruption/formation work is funded by compatible resonance or usable energy. The ledger must settle the full signed pre/post graph difference exactly once.
2. **Exercise a generated bond in a live field.** Form A-B through ordinary COMBINE, place C at a valid replacement contact, and advance normal `Simulation::step` ticks so incoming-contact activation accumulates naturally. Verify the old stored energy is preserved and the replacement uses COMBINE's pair-adhesion bond energy.
3. **Re-run the standard displacement observation.** Use seed 42 for a 600-second diagnostic baseline only after focused tests pass; require replacement validation to be reached and, for any commit, require two product components, exact constituent inventory, and a bounded energy residual. Keep seeds 7 and 101 for a later replication pass.
4. **Follow successful products through turnover.** Continue without resetting the successful products and track subsequent COMBINE, BREAK, and displacement participation, product persistence, and bond-energy parity. Then assess acquisition and use by an organism in a separate experiment.

The immediate next measurement is a small sample of the two mismatched bonds plus a one-time parity scan of the seeded fixture. Changing the displacement threshold, reaction rate, work budget, or bond-energy tolerance before that audit would mix the likely fixture-data issue with unrelated chemistry parameters.

## Experiment 2: controlled in-world displacement observation

After the bond-energy provenance audit, prepare a small deterministic field fixture containing one bonded A-B molecule and one incoming C at a valid live replacement contact. Let ordinary simulation ticks accumulate C's contact activation; do not seed accumulated activation or force the resolver to commit. Record bond-energy parity, the event, both product components, their field locations, the energy settlement, and the source inventory before and after. The existing unit-level transaction test remains the control for funded commit and rollback; this field observation must use `Simulation::step` and the environmental scheduler.

If the system cannot fund a natural commit in this fixture, retain the failed observation and report the precise settlement requirement. Do not silently add energy to make it pass.

## Experiment 3: follow products through turnover

Once a field displacement commits, continue the same run without resetting the products. Check whether the bonded A-C product can participate in another ordinary reaction, whether the free B persists or reacts, and whether BREAK ever occurs as compounds move. This distinguishes a one-off topology change from useful, reusable chemistry.

## Experiment 4: organism relevance

Only after environmental products persist and participate in multi-step reactions, test whether those compounds can be acquired, retained, and used by the existing organism construction and energy rules. Track material ownership across environment, organism storage, construction, and death. Do not claim organism support from environmental molecule counts alone.
