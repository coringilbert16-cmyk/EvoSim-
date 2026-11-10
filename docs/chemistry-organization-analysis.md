# Chemical organization analysis

EvoSim's chemical organization analyzer is an offline, observational tool. It consumes the JSONL diagnostics produced by the environmental chemistry experiment and does not change simulation behavior.

## Run

```bash
python3 tools/chemistry_organization_analyzer.py environmental-chemistry-default-baseline.jsonl \
  --json-out environmental-chemistry-organization.json \
  --markdown-out environmental-chemistry-organization.md
```

The input must contain `molecular_lifecycle` records. Use the environmental chemistry experiment with molecular history enabled. The workflow runs one 60-second baseline on pushes to `main` and manual dispatch; a second seed is run only when an explicit side-by-side comparison is requested. Pull-request validation runs the analyzer's unit tests without launching long experiments.

## What it measures

- **Structural recurrence:** groups multi-resource or bonded materials by exact part-ordered composition and normalized part-index bond connectivity.
- **Seed separation:** reports initial `INITIAL` molecules separately from reaction-created products. Recurrence among reaction-created products is the emergence measure; initial seed repetition is never counted as emergent recurrence.
- **Persistence:** reports observed lifetimes, separating completed lifetimes from molecules still active at the end of observation.
- **Lineage pathways:** counts transitions from parent structural signatures to child signatures by reaction.
- **Resource incorporation:** reports constituent amounts in compounds, including a separate total for reaction-created compounds.
- **Integrity:** validates lifecycle IDs and parent references; malformed histories cause a nonzero exit.

## Interpretation limits

For molecules with up to seven parts, structural families use exact canonical labeling of composition plus undirected bond connectivity, so atom-part ordering does not split the same labeled graph. Larger molecules use a conservative part-ordered signature. Bond energy, geometric endpoint angles, and placements are ignored. This is structural matching, not proof of chemical equivalence.

Single-resource unbonded materials are excluded from compound-family recurrence so the many independent resource particles do not dominate the organization metrics. They remain included in the general material accounting.

Lifetimes of molecules still active at the final record are right-censored. Resource amounts are summed once per created identity, so the same conserved constituents may be counted again in later lineage generations.

Recurrence and persistence are evidence to investigate, not proof of self-maintaining organization or life. Compare independent seeds before drawing conclusions. The CI report runs the same analyzer on two independent standard-seed experiments to expose seed-sensitive patterns.
