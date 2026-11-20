# OBS-001 Opportunity-Coverage Audit

**Status:** implementation audit, 2026-11-20  
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

The implementation documents this discovery pass as an exhaustive scan of the maintained environmental bond index. This is a different search domain from COMBINE: the natural denominator is the bond-index population for the pass, not all material pairs. Before deriving a completeness assertion, verify at the authoritative scan call site that the inspected-entry count corresponds to the complete index length and that no separate early-exit or work budget can truncate the pass. Do not infer this solely from a zero unsearched-progress counter.

### DISPLACEMENT

The per-tick `EnvironmentalDisplacementDiscoveryDiagnostics` already records:

- anchor materials inspected and bonded anchors;
- target cells and target materials inspected;
- bonds inspected;
- replacement-contact searches and contacts found;
- activation updates, threshold-met, below-threshold, and unavailable counts;
- post-activation trial rejection stages (old bond energy, topology, replacement, candidate revalidation, formation investment, physical insertion, energy, products, other).

The scan is bounded by anchor/target-cell limits in the discovery implementation. Unlike COMBINE, the diagnostic structure does not currently expose a complete set of explicit skipped/unvisited counts or a per-tick coverage/budget status. Work-remaining counters exist at the parent tick-diagnostic level, but they must be traced to their exact scheduling semantics before treating them as opportunity denominators.

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
