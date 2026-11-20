# Historical chemistry experiment summaries

These records preserve earlier experiments without treating their configurations as interchangeable with the November 19 one-hour baseline.

## EXP-2026-001 — Seed-42, 60-second environmental baseline

- Date: 2026-11-05
- Branch/PR context: PR #311, before it was merged.
- Seed: 42.
- Real elapsed duration: 60.049 seconds; 1,054 ticks.
- Starting inventory: 1,200 independent resources plus 80 molecules; the 80 molecules were 40 H–S and 40 C–H.
- No artificial activation.
- Outcomes: 10 COMBINE commits; zero BREAK and zero DISPLACEMENT commits.
- Inventory: 1,280 materials initially, 1,270 finally; bonds 80 initially, 90 finally.
- Diagnostic artifact: `environmental-displacement-60s.jsonl` (historically attached to PR #311).
- Status: completed historical baseline. This is not the same starting molecule catalog or multi-seed setup as the later standard experiment.

An earlier 600-tick attempt is explicitly excluded: it omitted `--headless`, launched the web server, timed out, and produced no diagnostic dataset.

## EXP-2026-002 — Five-seed, 300-second observation

- Date: approximately 2026-11-12.
- Seeds: 42–46.
- Requested/recorded duration: 300 seconds per seed.
- PR #354's audit of the five-minute artifacts recorded 7,624 molecule identities and 704 committed reaction events; all recorded participants in those 704 events had complete physical geometry.
- A separate summary of the same five-minute experiment reported 361 committed BREAK events and 27 displacement failures; 98,491 of 98,832 scheduled BREAK attempts were reported as insufficient-usable-energy failures.
- **Configuration discrepancy:** the combined report header showed 1,280 molecules / 2,560 constituents and zero independent resources, while the request description said 1,200 independent resources plus 80 molecules. A stale report also described the latter inventory.
- Related analysis/implementation reference: [PR #354](https://github.com/coringilbert16-cmyk/EvoSim-/pull/354), which audited the existing five-minute geometry artifacts and integrated geometry-aware network diagnostics.
- Status: completed, but configuration provenance is inconsistent. Do not compare reaction rates or chemical outcomes directly with EXP-2026-004 until the original artifacts are reconciled.

## EXP-2026-003 — Five-seed, 25-minute observation

- Date: 2026-11-12.
- Seeds: 42–46.
- Duration: 25 minutes per seed.
- Total committed reactions reported: 2,130.
- Analysis found 47 multi-atom structures used in multiple pathways. Prominent pathway hubs included C₂H, H–O–S, H–O–P, C–O–P, and O–P–S.
- The prior analysis did not confirm catalysis or autocatalysis: participating structures were consumed in the observed events; apparent retention was same-element replacement, not a demonstrated unchanged facilitator.
- Reanalysis report filename: `evosim-25m-reanalysis.md` (historical analysis artifact); raw totals included 27,723 ticks, 706,962 endpoint candidates, and 1,280 BREAK / 657 COMBINE / 193 DISPLACEMENT commits.
- Status: completed and reanalyzed. The exact artifact/run URL was not present in the retained project handoff when this record was created.

## Comparison rule

EXP-2026-001, EXP-2026-002, EXP-2026-003, and EXP-2026-004 differ in duration, starting inventory, or provenance quality. Preserve those differences. The November 19 experiment is the current strongest validated five-seed reference dataset.
