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

## Experiment 2: controlled in-world displacement observation

After the standard baseline, prepare a small deterministic field fixture containing one bonded A-B molecule and one incoming C at a valid live replacement contact. Let ordinary simulation ticks accumulate C's contact activation; do not seed accumulated activation or force the resolver to commit. Record the event, both product components, their field locations, the energy settlement, and the source inventory before and after. The existing unit-level transaction test remains the control for funded commit and rollback; this field observation must use `Simulation::step` and the environmental scheduler.

If the system cannot fund a natural commit in this fixture, retain the failed observation and report the precise settlement requirement. Do not silently add energy to make it pass.

## Experiment 3: follow products through turnover

Once a field displacement commits, continue the same run without resetting the products. Check whether the bonded A-C product can participate in another ordinary reaction, whether the free B persists or reacts, and whether BREAK ever occurs as compounds move. This distinguishes a one-off topology change from useful, reusable chemistry.

## Experiment 4: organism relevance

Only after environmental products persist and participate in multi-step reactions, test whether those compounds can be acquired, retained, and used by the existing organism construction and energy rules. Track material ownership across environment, organism storage, construction, and death. Do not claim organism support from environmental molecule counts alone.
