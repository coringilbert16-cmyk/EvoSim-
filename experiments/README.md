# Environmental chemistry experiment runs

The dedicated GitHub Actions workflow is
`.github/workflows/chemistry-experiments.yml`. It runs only when explicitly
requested: either through **Actions → Environmental Chemistry Experiments**
or by changing `experiments/chemistry-run.json` on `main`.

## Requesting a run

Set `run` to `true` and provide unique seeds, a real simulation duration, and
the snapshot interval:

```json
{
  "run": true,
  "seeds": [42, 43, 44, 45, 46],
  "duration_seconds": 300,
  "diagnostic_interval": 100,
  "deep_diagnostics": false
}
```

The request file is the trigger for the push-based route, allowing an authorized
repository update to start an experiment without changing simulation code.
The checked-in default is inactive (`run: false`). Each request runs every seed
once, with up to five seeds in parallel. Supported limits are 1–25 unique
nonnegative seeds, 30–3,600 seconds per seed, and a diagnostic interval of
1–100,000 ticks.

## Experiment identity and provenance

For a registered research run, provide the optional identity fields in the
workflow-dispatch form or in `experiments/chemistry-run.json`:

- `experiment_id`: registry ID such as `EXP-2026-007`.
- `research_plan`: supporting plan ID such as `PLAN-006` or engineering
  observability plan `OBS-001`.
- `condition_id`: concise condition label such as `baseline` or
  `resource-feed`.

The workflow passes these values to each simulation as
`EVOSIM_EXPERIMENT_ID`, `EVOSIM_RESEARCH_PLAN`, and
`EVOSIM_CONDITION_ID`. The manifest records them as supplied; it does not
invent missing identities or treat an unregistered run as registered. Keep
the experiment registry and dedicated result record authoritative.

## Fixed experimental conditions

- Starting inventory: 1,200 independent single-constituent resources and
  80 intact two-constituent molecules (1,360 total constituents).
- The starting molecule catalog contains 18 distinct two-unit structures
  identified across the preceding five-seed experiment, distributed as evenly
  as possible (4 or 5 molecules per type).
- Active elements: Carbon, Hydrogen, Nitrogen, Oxygen, Phosphorus, and Sulfur.
- The active inventory contains only the six elemental resources; Water is not a resource or medium.
- Ordinary reaction discovery and energy rules remain enabled and unchanged.
- No forced reactions, no disabled-resonance control, and no second comparison
  condition. Seeds are independent observations of the same baseline.

## Outputs

Each seed produces a compact permanent record from the reaction/energy, molecular-organization, and summary-only opportunity analyzers. Raw JSONL, manifests, and intermediate reports stay on the runner, are used to build and validate the record, and are deleted locally; they are not uploaded. The compact comparison artifact contains only per-seed records and the comparison report. The repository result record is the permanent archive. Routine runs do not retain detailed network, geometry, contact, or temporal traces. Deep analysis is explicit opt-in and its artifacts expire after one day.

- BREAK, COMBINE, and DISPLACEMENT candidate/commit/rejection totals and
  failure reasons.
- Displacement discovery work and budget exhaustion.
- Molecular creation, destruction, lineage reuse, and COMBINE-after-reuse.
- Structural-family recurrence across independent seeds and family transitions.
- Exact formed and broken bond counts for all 21 unordered active-element pairs, separated by BREAK, COMBINE, and DISPLACEMENT.
- Elemental material incorporated into reaction-created compounds.
- Time-resolved chemical complexity is an explicitly requested deep analysis, not a routine permanent output.
- Energy-balance residuals and per-seed validation.

Raw source data and intermediate analyzer files are deleted from the runner after record generation; they are never uploaded in routine runs. Compact record transport artifacts are retained for 90 days; permanent records belong in `experiments/records/results/`. Deep diagnostic artifacts are opt-in and expire after one day. Routine Rust CI remains separate and keeps its short 60-second
seed-42 chemistry smoke baseline and focused correctness/feasibility tests.

## Time-resolved complexity analysis (deep diagnostics only)

When deep analysis is explicitly requested, the workflow runs `tools/temporal_chemical_complexity_analyzer.py` over per-seed chemistry reports. It divides each run into twelve equal-duration windows and reports newly observed compound fingerprints, cumulative compound diversity, repeated compound products, all reaction-created identities, active-compound size and topology, reaction-pattern novelty, and strongly connected components in the cumulative compound-to-compound family-transition graph. Unbonded single-resource fragments are counted as reaction-created identities but excluded from compound novelty and active-compound complexity. The analysis is offline and does not change simulation behavior.

Structural fingerprints use iterative Weisfeiler-Lehman refinement with composition and degree summaries. They are scalable descriptors, not exact graph-isomorphism proofs. A directed cycle in the family-transition graph is a candidate network cycle, not evidence by itself of autocatalysis, energy coupling, or self-sustainability. The report includes these limitations explicitly.

## Interpretation

The seed is the independent observation; individual reactions within one run
are not independent samples. Cross-seed structural matching is based on
composition and bond topology, not bond energy or endpoint geometry. Lineage
reuse is evidence of molecular reuse, not by itself proof of a productive
multistep synthesis pathway. The chemistry analyzer attributes observed bond changes from each committed reaction's
full `PhysicalMaterial` lifecycle snapshots. It maps each `internal_connections`
edge's `part_a`/`part_b` indices through the material's constituent list to obtain
the exact unordered element pair. For BREAK and DISPLACEMENT, the explicit
`broken_bond_index` is accounted for separately before comparing remaining
reactant edges with product edges; this ensures a same-element-pair replacement
is counted as one broken and one formed bond rather than cancelling to zero.
The report includes zero-count rows for all 21 active-element pairs, separate
counts by reaction type, and the initial/final live-molecule bond inventories.
It reconciles the observed inventory change against formed-minus-broken reaction
edges for every seed; missing lifecycle graphs or any mismatch are reported rather
than silently treated as zero. This is elemental-pair attribution, not endpoint-
geometry or bond-energy calibration. Committed events without complete lifecycle graph
snapshots are reported as missing rather than inferred from composition.


## Experiment research records

The durable research registry is in [`experiments/records/`](records/README.md). It tracks completed runs, historical configuration caveats, planned research, and detailed results. Update the registry and a result record whenever an experiment completes; do not overwrite historical conditions or treat family cycles as proof of catalysis.
