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
multistep synthesis pathway. The existing feasibility matrix tests cover all
21 unordered non-Water element pairs; exact observed bond-pair attribution is
a separate instrumentation improvement and should not be inferred from
composition-only reaction records.
