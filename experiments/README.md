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
  "diagnostic_interval": 100
}
```

The request file is the trigger for the push-based route, allowing an authorized
repository update to start an experiment without changing simulation code.
The checked-in default is inactive (`run: false`). Each request runs every seed
once, with up to five seeds in parallel. Supported limits are 1–25 unique
nonnegative seeds, 30–3,600 seconds per seed, and a diagnostic interval of
1–100,000 ticks.

## Fixed experimental conditions

- Standard environment: 1,280 physical materials (1,200 independent resources
  and 80 starting molecules).
- Active elements: Carbon, Hydrogen, Nitrogen, Oxygen, Phosphorus, and Sulfur.
- Water is excluded from the active inventory.
- Ordinary reaction discovery and energy rules remain enabled and unchanged.
- No forced reactions, no disabled-resonance control, and no second comparison
  condition. Seeds are independent observations of the same baseline.

## Outputs

Each seed uploads its raw JSONL diagnostics plus separate reaction/energy and
molecular-organization reports. A final aggregate artifact combines:

- BREAK, COMBINE, and DISPLACEMENT candidate/commit/rejection totals and
  failure reasons.
- Displacement discovery work and budget exhaustion.
- Molecular creation, destruction, lineage reuse, and COMBINE-after-reuse.
- Structural-family recurrence across independent seeds and family transitions.
- Exact formed and broken bond counts for all 21 unordered active-element pairs, separated by BREAK, COMBINE, and DISPLACEMENT.
- Elemental material incorporated into reaction-created compounds.
- Energy-balance residuals and per-seed validation.

Per-seed artifacts are retained for 30 days; the combined report is retained
for 90 days. Routine Rust CI remains separate and keeps its short 60-second
seed-42 chemistry smoke baseline and focused correctness/feasibility tests.

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
