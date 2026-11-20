# OBS-001 Opportunity-Coverage Audit

**Status:** implementation in progress, 2026-11-20  
**Scope:** environmental COMBINE, BREAK, and DISPLACEMENT discovery telemetry  
**Behavior boundary:** observation only; no changes to reaction rules, search order, budgets, activation, scheduling, or energy settlement.

## Purpose

Establish which existing counters can support a defensible opportunity funnel, identify missing coverage metadata, and prevent missing fields or bounded searches from being reported as measured zero or complete discovery.

## Existing authoritative measurements

### COMBINE

The per-tick `EnvironmentalCombineSearchDiagnostics` already records:

- source/target cells and materials inspected;
- forward-ordered material-pair inspections and restoration/overlap outcomes;
- constituent pairs available, inspected, and left uninspected by the window;
- truncated constituent-pair windows;
- pairs with and without valid interfaces;
- endpoint candidates generated and rejected by final evaluation;
- physically evaluable interfaces and interfaces sent to chemistry;
- accumulation keys updated, preserved because they were unsearched, invalidated, and remaining;
- `budget_exhausted` and `target_cell_windows_capped`.

This is a bounded search. `constituent_pairs_available` is a broad-phase candidate count, not a count of all possible material pairs in the world. `constituent_pairs_inspected` is the relevant examined subset. Uninspected candidates are not rejected candidates. Existing bounded opportunity samples are examples, not exhaustive event records.

### BREAK

The per-tick `EnvironmentalBreakDiscoveryDiagnostics` records:

- bond-index entries inspected and physical bond candidates yielded;
- candidates checked;
- material restoration success/rejection;
- geometry, endpoint, mapping, property, position, interface, and chemistry rejection stages;
- activation updates, below-threshold and threshold-met counts;
- stale progress invalidation and progress preservation/dissipation.

The discovery helper materializes the complete maintained environmental bond-index candidate list, records its length as inspected entries, and then inspects every yielded candidate. The loop has no candidate-inspection budget or early-exit path. This is a different search domain from COMBINE: the natural denominator is the bond-index population for the pass, not all material pairs. Coverage is complete when the two authoritative counts reconcile, including an explicitly measured empty index.

### DISPLACEMENT

The per-tick `EnvironmentalDisplacementDiscoveryDiagnostics` already records:

- anchor materials inspected and bonded anchors;
- target cells and target materials inspected;
- bonds inspected;
- replacement-contact searches and contacts found;
- activation updates, threshold-met, below-threshold, and unavailable counts;
- post-activation trial rejection stages (old bond energy, topology, replacement, candidate revalidation, formation investment, physical insertion, energy, products, other).

The scan is bounded by a 64-cell rotating anchor window, a 64-cell target window per anchor, and a shared per-tick work allowance. New diagnostics report the anchor-cell domain, cells entered, cells skipped by the fixed window, selected-window cells not reached, target-cell query count, and target queries reaching their requested limit. The parent tick record also reports remaining work. These counters expose known partial coverage but do not yet establish the exact total target-cell/material work omitted by each bounded query.

## Common reporting vocabulary

Use these states for each reaction type and each diagnostic interval:

- `complete`: the defined search domain was fully visited and the denominator is known;
- `partial`: a known subset was visited, with known or bounded unvisited work;
- `unknown`: the source record or search semantics do not establish coverage;
- `not_applicable`: the measurement does not apply to this reaction/search stage.

A numeric zero is valid only when the authoritative counter was present and measured. Missing fields, legacy records, unavailable counters, and unexecuted stages must remain null/unknown—not be coerced to zero.

## Funnel stages

Keep reaction-specific search domains while using consistent stage names:

1. available in defined search domain;
2. inspected by discovery;
3. candidate validated / evaluated;
4. geometrically valid;
5. chemically and activation eligible;
6. scheduled or deferred;
7. rejected, with stage/reason where available;
8. committed.

Do not force every reaction type to populate a stage it does not have. Use `not_applicable` with an explanation rather than fabricating a zero.

## Interpretation constraints

- Per-tick counts are repeated search observations, not necessarily unique opportunities across a run. Do not sum them and call the result unique encounters.
- A COMBINE broad-phase constituent pair is not proof of endpoint contact.
- A DISPLACEMENT replacement-contact search is not equivalent to a unique incoming molecule opportunity; the same material may be revisited across ticks.
- BREAK's bond-index candidates are a bond-domain denominator, not a molecule-pair denominator.
- Bounded samples must never be used as exhaustive counts.
- Do not calculate conversion rates when the denominator is missing, partial without a known denominator, or semantically mismatched.
- Preserve raw counters and legacy schema compatibility; normalized coverage is an additional interpretation layer.

## Implementation sequence

1. Trace the three discovery loops and record their exact search domains, early exits, and budgets.
2. Define normalized per-reaction coverage metadata without changing discovery behavior.
3. Instrument only missing authoritative quantities: especially DISPLACEMENT unvisited/budget state and BREAK full-domain confirmation if not already derivable.
4. Update the contact-opportunity analyzer so absent fields remain unknown and rates are emitted only with valid denominators.
5. Add fixtures for measured zero, absent legacy fields, partial bounded search, exhausted budget, exhaustive empty domain, and mixed reaction coverage.
6. Validate against retained historical JSONL artifacts before running new simulations.

## Acceptance criteria

- Every reaction type reports its search domain and coverage state.
- Complete, partial, unknown, and not-applicable are distinguishable.
- Missing fields are never silently converted to zero.
- Search budget exhaustion and known unvisited work are represented explicitly.
- Rates are withheld when denominator validity or coverage is insufficient.
- Existing specialist counters and raw artifacts remain available.
- Tests demonstrate that bounded or legacy/incomplete records cannot be misreported as complete opportunity coverage.
- No chemistry behavior, reaction scheduling, activation, or energy accounting changes.

## Implementation progress — opportunity coverage slice

### Confirmed search semantics

- BREAK's discovery helper materializes the complete maintained environmental bond-index candidate list, records its length as inspected entries, then inspects every yielded candidate. The loop has no candidate-inspection budget or early-exit path. Its per-tick coverage can therefore be marked complete when the two authoritative counts reconcile, including an explicitly measured empty index.
- COMBINE retains its existing bounded broad-phase pair counts and explicit budget/truncation counters. Coverage is complete only when available = inspected + uninspected, uninspected is zero, and the budget status is explicitly false. Otherwise it is partial or unknown.
- DISPLACEMENT now records anchor-cell domain size, cells entered, cells excluded by the fixed 64-cell rotating window, selected-window cells not reached before the scan stopped, target-cell query count, and target queries that returned their requested limit. These are observation-only counters.

### Analyzer changes

- Missing, null, malformed, and negative counters remain unknown; they are no longer silently converted to zero.
- Pooled totals are null if any contributing tick lacks that field. A separate known-subtotal and missing-tick count preserve partial historical evidence without presenting it as a complete total.
- BREAK, COMBINE, and DISPLACEMENT now have independent normalized coverage states.
- Reaction-specific BREAK and DISPLACEMENT counters are namespaced in output to prevent overlapping field names (such as activation counters) from being combined.
- Conversion ratios are withheld if any required numerator or denominator is missing or the denominator is zero.
- DISPLACEMENT is never labeled complete on the basis of a remaining-work counter alone. Known skipped anchor cells or an exhausted work allowance make it partial; target-window limit hits without an exact omitted-cell count remain unknown.

### Tests added

- Reconciled complete COMBINE search, including a measured empty search.
- Partial bounded COMBINE search.
- Missing legacy counters remain unknown, not zero.
- Complete BREAK bond-index scan and mismatch fail-closed behavior.
- DISPLACEMENT budget exhaustion and skipped anchor-window coverage.
- Target-cell limit hits do not falsely claim complete coverage.
- Missing raw seed artifact error handling.

### Remaining validation

- Run the Python analyzer test suite and Rust tests/Clippy through CI.
- Validate the analyzer against retained historical JSONL artifacts, including legacy records.
- Review the exact target-cell query semantics before ever adding an omitted-target-cell denominator. A reached limit is recorded, but not treated as proof of truncation because the exact domain size is not yet measured.
- No new simulation experiment has been launched. No chemistry rules, activation thresholds, reaction ordering, scheduling, or energy settlement have been changed.

## Historical-artifact validation — EXP-2026-004

Validated the five retained raw JSONL observations from GitHub Actions run [#74](https://github.com/coringilbert16-cmyk/EvoSim-/actions/runs/44582565701), using the per-seed artifacts:

- seed 42: artifact `14564149398`
- seed 43: artifact `14563318635`
- seed 44: artifact `14563664345`
- seed 45: artifact `14562424610`
- seed 46: artifact `14562664309`

The raw files contained **109,849 environmental-chemistry diagnostic ticks** across the five seeds.

Results using the normalized coverage rules:

| Reaction | Historical tick coverage | Interpretation |
|---|---:|---|
| COMBINE | 109,849 partial | The bounded discovery budget was exhausted on every diagnostic tick. |
| BREAK | 109,849 complete | Full bond-index entries inspected matched bond candidates yielded on every tick. |
| DISPLACEMENT | 109,849 partial | The legacy per-tick remaining-work counter was zero on every tick, so the work allowance was consumed. Exact unvisited target-cell/material totals remain unavailable. |

All six newly added DISPLACEMENT anchor/target-window counters are absent from this historical schema on all 109,849 ticks. They remain unknown/null in pooled totals; the analyzer does not silently convert their absence to measured zero. The historical records therefore validate backward-compatible missingness behavior and demonstrate why DISPLACEMENT cannot claim complete coverage from the older diagnostics.

This pass parsed the raw records and evaluated coverage/missingness in a streaming check. It did not run a new simulation. It does not claim a unique-opportunity denominator across repeated ticks.

## Current slice status

- Source semantics traced.
- Reaction-specific coverage counters and normalized analyzer states implemented.
- Analyzer tests, Rust tests, smoke validation, and Clippy passed in PR CI run #14637.
- Historical EXP-2026-004 raw-artifact compatibility checked across all five seeds.
- Remaining: exact omitted-target-domain accounting for DISPLACEMENT is intentionally unresolved; the current analyzer reports partial/unknown rather than inventing an omitted-work count. Unified run manifest and cross-analyzer report index remain later OBS-001 stages.
