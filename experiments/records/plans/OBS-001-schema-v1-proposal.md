# OBS-001 — Canonical experiment manifest and chemistry record schema (proposal)

**Status:** Draft for source-level review; not implemented.
**Plan:** [OBS-001](OBS-001-chemistry-observability.md)
**Baseline audited:** `main` at `fd45f2987d55623bce3ad25f1a0ededc0100ff10`.
**Scope:** Sidecar provenance, normalized event joins, opportunity coverage, energy-settlement capture, and compact analysis index. No simulation or chemistry behavior changes.

## 1. Design decisions

1. Keep the current environmental JSONL as raw archival evidence. Do not replace it or silently rewrite old schema versions.
2. Add a compact sidecar run manifest and a common analysis index. Do not force every specialist analyzer into one monolithic output schema.
3. Give every environmental reaction event a durable run-scoped ID and use that same ID in lifecycle and energy records.
4. Capture participant identities and settlement values at the authoritative event/commit boundary. Offline tools must not reconstruct or recalculate authoritative energy decisions.
5. Distinguish measured zero, missing, not applicable, not evaluated, and sampled/partial. Missing values must never be defaulted to zero.
6. Keep exhaustive opportunity counts separate from bounded samples. Only exhaustive search domains may claim complete denominators.
7. Store large diagnostic arrays once. The compact report/index contains summaries, coverage, and references; CSV exports are optional derived views.

## 2. Versioning and compatibility

Use a named schema family with explicit integer versions:
- `evosim.experiment_manifest`, version 1.
- `evosim.environmental_chemistry_jsonl`, version 3 for the complete normalized event/settlement contract (current JSONL header is version 2; old v2 remains readable).
- `evosim.opportunity_coverage`, version 1.
- `evosim.analysis_index`, version 1.

The identity-only event fields in PR #398 are additive to the current v2 envelope and remain readable by existing readers that ignore unknown fields. Move the environmental JSONL header version from 2 to 3 when the complete normalized event/settlement contract is adopted (including the event-linked settlement and coverage semantics). The raw JSONL envelope remains one JSON object per line with its existing `event` discriminator. New fields are additive, but readers must still validate the header's declared version.

Semantics:
- A numeric zero means the quantity was measured and equals zero.
- `null` means unknown/not recorded/not available, with a companion status where needed.
- `not_applicable` is distinct from `unknown`.
- A missing field in a legacy record means unavailable under that schema, not zero.
- Bounded samples always carry sample policy and coverage; they never imply exhaustive counts.

## 3. Run manifest

One `experiment-manifest.json` sidecar per run. Proposed shape:

```json
{
  "schema": "evosim.experiment_manifest",
  "schema_version": 1,
  "experiment": {
    "experiment_id": "EXP-2026-005",
    "research_plan": "PLAN-004",
    "condition_id": "resource-feed",
    "seed": 42
  },
  "run": {
    "run_id": "repository-workflow-run-attempt",
    "status": "completed",
    "started_at_utc": "ISO-8601",
    "completed_at_utc": "ISO-8601",
    "requested_duration_seconds": 3600,
    "actual_elapsed_wall_seconds": 3600.1,
    "start_tick": 0,
    "end_tick": 15794
  },
  "build": {
    "repository": "owner/repository",
    "commit_sha": "full-commit-sha",
    "executable_identity": "optional-build-id",
    "dirty": false
  },
  "conditions": {
    "configuration_schema_version": 1,
    "configuration_sha256": "hash-of-canonical-config",
    "initial_physical_material_count": 1280,
    "initial_constituent_count": 1360,
    "initial_material_inventory": {},
    "interventions": []
  },
  "diagnostics": {
    "raw_schema_versions": {"environmental_chemistry_header": 2},
    "diagnostic_interval_ticks": 100,
    "enabled_tiers": ["core", "standard"]
  },
  "analyzers": {
    "chemistry": {"version": "source-or-package-version", "schema_version": 1},
    "organization": {"version": "source-or-package-version", "schema_version": 1}
  },
  "artifacts": [
    {
      "role": "raw_observation",
      "path": "seed-42.jsonl",
      "sha256": "content-hash",
      "size_bytes": 0
    }
  ],
  "quality": {
    "observation_complete": true,
    "diagnostic_coverage": "complete",
    "warnings": []
  }
}
```

The manifest is workflow-generated because the runner knows the exact GitHub run, commit, timestamps, artifact names, and build context. The simulation recorder supplies the actual elapsed duration, start/end ticks, and initial/final conditions. If a field cannot be measured, record null plus an explicit status; do not infer it from requested settings.

The manifest is immutable after artifact upload. A later analysis run must create or update an analysis index, not rewrite the original run provenance.

## 4. Reaction event identity and participant join

Current source facts:
- `EnvironmentalChemistryEvent` has tick, reaction, source cell/material indices, source compositions, work/activation context, outcome/reason, product summaries, post-commit summaries, and nested lifecycle records.
- Lifecycle records already carry molecule IDs, parent IDs, reaction ID, reaction kind, action, and material snapshot.
- The lifecycle reaction ID is generated as tick plus the event's position in the per-tick event vector (for example, `157-2`), but the enclosing reaction event does not carry it.
- Rejected events generally have no lifecycle records, and source indices are not stable identities.

Proposed event fields:

```json
{
  "event": "environmental_reaction",
  "reaction": {
    "reaction_id": "157-2",
    "tick": 157,
    "reaction": "COMBINE",
    "source_material_ids": [1001, 1002],
    "outcome": "committed",
    "failure_reason": null,
    "product_material_ids": [1200],
    "lifecycle_join_status": "complete"
  }
}
```

Keep existing flat fields during the compatibility period; the proposed nested object is the normalized contract for new readers. Do not remove the original material snapshots in the first migration.

Identity rules:
- Promote the current tick-plus-per-tick-event-ordinal value to the enclosing reaction event's `reaction_id`; use exactly the same value in every lifecycle record created by that event.
- The run manifest scopes the ID, so `(run_id, reaction_id)` is globally unique. Do not depend on timestamp or vector index alone.
- Capture each participant's nonzero `environmental_identity` before any mutation, including rejected attempts. If the identity is unavailable, write null and `participant_identity_status: "unavailable"`; never substitute the current vector index.
- Keep cell/material indices as optional geometry/debug locators only; they are not identity.
- Record participant role and ordering explicitly for BREAK and DISPLACEMENT; do not infer role from composition.
- Product IDs come from the authoritative post-commit material identities and must join to lifecycle creation records.
- Rejected events have no product IDs and still carry their exact source participant IDs when available.
- Lifecycle creation/destruction rows retain the same `reaction_id` and molecule IDs.

## 5. Energy settlement record

Current authoritative contract:
- `EnergyTransaction` records reason, usable delta, signed structural delta, work, and heat. Its legacy `potential_released` field must remain zero and must not be represented as a real source.
- `ReactionEnergyBalance` records reactant energy, product energy, physical work, dissipated heat, signed net reaction change, required external input, and released surplus.
- Environmental paths may also withdraw frequency-compatible resonance and/or use the environmental usable-energy holder. These funding movements must be captured separately from the generic transaction.
- The recorder currently emits run-level energy snapshots but does not serialize the exact settlement beside each reaction event.

Proposed event-linked object (only values available at the authoritative commit boundary are populated):

```json
{
  "settlement_status": "settled",
  "authority": "reaction_energy_balance",
  "reactant_structural_energy": 0.0,
  "product_structural_energy": 0.0,
  "structural_delta": 0.0,
  "usable_energy_delta": 0.0,
  "resonance_energy_delta": 0.0,
  "external_input_delta": 0.0,
  "physical_work": 0.0,
  "heat_dissipated": 0.0,
  "funding": {
    "resonance_withdrawn": 0.0,
    "usable_energy_debited": 0.0,
    "usable_energy_credited": 0.0
  },
  "transaction_residual": 0.0
}
```

Requirements:
- Capture the actual authoritative pre/post values or transaction fields where the commit is prepared and settled. Do not reproduce energy formulas in the recorder or analyzer.
- For a rejected/unfunded reaction, record `settlement_status: "not_settled"` and the exact available preflight reason if the resolver provides one. A planned estimate is not a committed transaction.
- Use null for a breakdown the current commit API does not expose; include a field-level availability status. Do not infer resonance withdrawal from activation, and do not infer work from the requested work field when settlement differs.
- `external_input_delta` remains zero unless a named external energy source actually participates. Matter-feed events are not energy input.
- Reconcile the event settlements against periodic/final inventory and ledger deltas in offline validation. Event rows are attribution, not a second energy ledger.

## 6. Opportunity and search coverage

Keep opportunity telemetry as tick/window summaries by default; per-candidate identity traces are Tier 3 and opt-in.

Proposed coverage object:

```json
{
  "reaction_type": "COMBINE",
  "window": {"start_tick": 100, "end_tick": 199},
  "search_domain": "documented-domain-id",
  "coverage_status": "partial",
  "counts": {
    "available": 0,
    "inspected": 0,
    "evaluated": 0,
    "geometrically_valid": 0,
    "activation_eligible": 0,
    "energy_eligible": 0,
    "committed": 0,
    "rejected": 0,
    "deferred": 0,
    "skipped_by_budget": 0,
    "unknown": 0
  },
  "truncation": {
    "budget_exhausted": false,
    "uninspected_estimate": null,
    "sampling_policy": "none"
  },
  "denominator": {
    "eligible_count": null,
    "complete": false,
    "reason": "search domain was bounded or not exhaustively enumerated"
  }
}
```

Count semantics must be documented per reaction type. Do not sum stages that represent different candidate populations as if they were one funnel. Keep separate:
- exhaustive candidate discovery and its search domain;
- candidates inspected/evaluated;
- geometric/contact validity;
- activation eligibility;
- energy affordability;
- terminal outcomes (committed/rejected);
- deferred/skipped/truncated work;
- bounded example samples.

A denominator may be marked complete only if every member of the defined search domain was evaluated under the documented eligibility rules. Otherwise it remains partial/unknown. An absent tick record is not a measured zero; use observed-tick coverage or explicit fixed-window summaries to establish whether a window was actually observed.

## 7. Environmental exposure references

Do not copy full environmental fields into every reaction row. Use compact exposure windows with IDs and references:
- `exposure_window_id`, start/end tick and wall-clock time;
- measured channels and units (for example local resonance sample, hotspot flux, flow, local density);
- spatial/temporal coverage and sampling policy;
- source/model version and any missing channels;
- whether the measurement is full, sampled, tick-zero-only, or unavailable.

The current local resonance samples and hotspot snapshots are not equivalent to a complete time-resolved persistent field. The manifest/index must state that distinction. Any future resonance-field telemetry must follow the canonical resonance-field roadmap and its phase/interference/propagation requirements; this schema proposal does not authorize a substitute field model.

## 8. Compact analysis index

One `analysis-index.json` per experiment/condition group references the run manifests and specialist outputs:

```json
{
  "schema": "evosim.analysis_index",
  "schema_version": 1,
  "experiment_id": "EXP-2026-005",
  "runs": [{"seed": 42, "condition_id": "no-feed", "manifest": "seed-42-manifest.json"}],
  "analyses": [
    {
      "name": "chemistry_batch",
      "analyzer_version": "source-or-package-version",
      "output_schema_version": 1,
      "status": "completed",
      "inputs": ["seed-42-manifest.json"],
      "outputs": ["chemistry-batch.json", "chemistry-batch.md"],
      "coverage": {"status": "partial", "warnings": []}
    }
  ],
  "validation": {
    "status": "warnings",
    "checks_passed": [],
    "checks_failed": [],
    "warnings": ["opportunity denominator incomplete"]
  }
}
```

The report index must:
- validate manifest/raw artifact compatibility before running analyzers;
- run or reuse specialist analyzers by input hash and analyzer version;
- reject mixed incompatible schemas unless a versioned reader exists;
- check all required outputs and record failed/skipped analyzers;
- provide concise summaries and drill-down artifact references;
- keep one canonical large data representation per dataset. CSV is generated on demand, not duplicated by default;
- preserve independent-seed grouping for cross-seed statistics.

## 9. First implementation slices and tests

Recommended order, each as a small, separately testable change:

1. **Identity-only event join:** add `reaction_id` to reaction event and source material identity snapshots to all three event types. Keep current lifecycle IDs unchanged and assert event/lifecycle join for committed events. Rejected events must retain source IDs. Test duplicate same-tick events, multiple source cells, stale identities, and all three reaction types.
2. **Energy settlement capture:** define a serializable read-only summary populated from actual settlement values at COMBINE/BREAK/DISPLACEMENT commit boundaries. Do not change settlement arithmetic. Test conservation between captured values and authoritative transaction/inventory before/after values, plus rejected/no-settlement cases.
3. **Opportunity coverage contract:** add explicit completeness/truncation semantics to existing counters and bounded samples. Tests must distinguish measured zero from missing, exhaustive from partial search, budget exhaustion, deferred candidates, and bounded samples.
4. **Manifest generation:** produce a sidecar from the workflow and recorder. Test missing build/run metadata, duration/tick consistency, checksum and artifact references, and immutable provenance.
5. **Analysis index and compact output:** add validation/orchestration around existing analyzers. Test missing files, duplicate inputs, mixed versions, hash-based reuse, failed analyzer reporting, no duplicated large arrays, and cross-seed alignment.

## 10. Open questions requiring source-level confirmation before implementation

- Confirm exact per-reaction settlement fields and resonance/usable funding split at each current commit boundary; do not add guessed fields.
- Confirm whether all environmental participants have nonzero stable identities before any resolver mutates them, especially BREAK discovery and stale-source rejection paths.
- Confirm the intended event ordinal is allocated before or at event creation and cannot diverge from lifecycle construction.
- Confirm whether existing counters are exhaustive within their stated search domains; default all unproven denominators to partial/unknown.
- Confirm actual wall-clock duration can be measured by the headless runner without changing tick scheduling.
- Confirm analyzer output version/hash conventions and exact artifact size limits in GitHub Actions.

**No implementation is authorized by this document alone.** This is the field-level proposal for review and focused implementation planning. It changes no code or simulation behavior.


## 11. Confirmed energy commit-boundary details (2026-11-20)

The source review now confirms the following exact capture points. These are implementation design findings, not telemetry already emitted.

### Environmental BREAK

`src/chemistry/chemical_reaction.rs::resolve_environmental_diagnosed`:
- Clones the environment resonance reservoir and withdraws frequency-compatible `resonant_work`.
- Computes the remaining usable requirement from the stored bond energy plus disruption work not covered by resonance.
- Calls `prepare_chemical_break_energy_with_resonance`, which constructs and settles the authoritative `EnergyTransaction` on a staged ledger/usable holder.
- Tracks the actual usable-energy delta, structural delta (the removed bond's stored energy), resonance-holder decrease, and complete disruption work before committing the physical split.
- Only commits the staged resonance/ledger/usable values after the physical split succeeds.

The smallest faithful summary should be captured from the prepared settlement plus the actual `resonant_work`; do not infer resonance from requested disruption cost. The preparation object currently returns staged usable energy, staged ledger, and heat, but not the transaction itself. A small read-only settlement value can be added at this boundary without changing settlement arithmetic.

### Environmental DISPLACEMENT

`src/chemistry/environmental_displacement.rs::resolve_environmental_diagnosed`:
- Builds an authoritative `DisplacementTrial` containing `ReactionEnergyBalance` with reactant/product structural energy, physical work, heat, signed net change, required external input, and surplus.
- Withdraws actual compatible resonance funding, validates the reservoir before/after and the withdrawn amount, and computes any remaining usable-energy deficit.
- Constructs and settles the conservative `EnergyTransaction` on staged holders/ledger.
- Updates usable, resonant, structural, and work ledger holdings before committing the products.

This path already has the complete balance and actual resonance funding at the commit boundary. Return or capture a compact immutable summary from this function; do not recalculate the budget in the recorder.

### Environmental COMBINE

`src/chemistry/environmental_combine.rs::resolve_environmental` computes authoritative reactant and product structural potentials and the usable-energy delta around `combine_runtime::form_selected_bond_diagnostic`. That helper constructs and settles the actual `EnergyTransaction`, but currently returns a `CombineAttempt` without the transaction breakdown. The outer environmental resolver already has the structural pre/post values and final usable delta, but work/heat should be surfaced from the exact transaction rather than inferred from cumulative ledger differences. The minimal API change is to carry a small settlement summary out of the COMBINE helper alongside its existing attempt result; review all internal callers before choosing the exact type.

### Common rule

- Capture the settlement only after all staged validation succeeds and the physical mutation commits.
- A failed resolver returns `not_settled` plus its existing failure reason; it must not emit a planned balance as if it were a transaction.
- The legacy `EnergyTransaction::potential_released` remains zero and is never a source.
- COMBINE has no resonance withdrawal in its current environmental resolver; represent zero only if explicitly measured at that boundary.
- BREAK and DISPLACEMENT must report actual frequency-compatible resonance withdrawn separately from the usable-holder delta.
- No event-level summary may be computed by subtracting long-lived cumulative f64 ledger counters; use the exact staged transaction/pre-post values already held at the commit boundary.


## 12. Confirmed opportunity-search semantics (2026-11-20)

The three reaction types do not share one common search domain. The schema/report layer must preserve these differences.

### BREAK — exhaustive existing-bond census per tick

`chemical_reaction::accumulate_environmental_with_diagnostics` explicitly enumerates the complete maintained environmental bond index once per tick; no rotating discovery window or candidate-inspection budget applies to the BREAK bond scan. Its counters distinguish index entries inspected, bond candidates inspected/checked, material restoration, geometry/interface/chemistry rejections, activation updates, below-threshold and threshold-met candidates. The local-resonance spectrum sample rotates one candidate per tick, but that sample is separate from the exhaustive bond discovery.

Coverage may be marked exhaustive **only for the defined universe of currently indexed existing environmental bonds on that tick**. It is not a census of hypothetical future bonds or every possible molecular contact. A full eligible-energy denominator still requires connecting the checked candidates to exact identities and the energy affordability stage.

### COMBINE — bounded, rotating spatial/interface search

`environmental_combine::accumulate_environmental_with_diagnostics` consumes a shared per-tick work budget (currently initialized to one eighth of the global candidate-inspection budget). It rotates anchor cells/materials by tick, caps target-cell windows, bounds constituent-pair inspection, and records:
- available versus inspected constituent pairs;
- uninspected pairs within a candidate window and truncated windows;
- valid versus invalid interfaces;
- generated/rejected endpoint candidates;
- physically evaluable interfaces and interfaces actually submitted to chemistry;
- pairs suppressed by selection and a budget-exhausted flag;
- a bounded eight-entry opportunity sample.

The complete field-wide pair universe is not inspected on one tick. `constituent_pairs_available` means candidate pairs generated inside the visited source-pair window, not every possible pair in the world. Do not calculate a world-wide reaction probability from these counters.

### DISPLACEMENT — bounded rotating anchor/neighbor/contact search

`environmental_displacement::accumulate_environmental_with_resonance` also receives a bounded per-tick budget (currently one eighth of the global candidate-inspection budget). It rotates anchor cells/materials by tick and charges anchor, target-cell, target-material, bond, and replacement-contact inspections. It records found replacement contacts, activation updates/threshold outcomes, trial rejection stages, and a bounded eight-entry sample.

The remaining-work counter is the unspent inspection budget, not a count of uninspected opportunities. A zero remainder means the budget was depleted; by itself it does not prove that another candidate was waiting. Treat coverage as partial or unknown unless the search can explicitly prove it completed its defined domain. Do not use `replacement_contacts_found` as the number of all eligible replacements in the environment.

### Common reporting rules

- Keep per-reaction coverage separate; never add unlike stage counts into one denominator.
- Distinguish candidate discovery, interface evaluation, activation threshold, energy affordability, scheduling, and final commit/rejection.
- Existing tick rows are activity-triggered, not a guaranteed row for every tick. Use summary observed-tick count and fixed windows; absence of a row alone is not a measured zero.
- Existing COMBINE/DISPLACEMENT opportunity samples are capped at eight examples per tick. Their sample size is descriptive only; record sample policy and never substitute it for exhaustive opportunity counts.
- For a bounded search, a report must state the visited domain and known uninspected/truncated work. Where the engine cannot tell whether further work remained, use `coverage_status: "unknown"` rather than guessing complete or partial.


## 13. Concrete settlement summary mapping for implementation

The first implementation should use one shared value type carried by the resolver result and copied into `EnvironmentalChemistryEvent` only after commit. Proposed numeric fields:

| Field | BREAK source | COMBINE source | DISPLACEMENT source |
|---|---|---|---|
| `usable_energy_delta` | prepared usable holder minus pre-commit holder | staged holder minus pre-commit holder | staged holder minus pre-commit holder, including resonance transfer |
| `structural_energy_delta` | exact removed bond energy | product structural potential minus reactant structural potential | `trial.energy.net_reaction_change` |
| `physical_work` | full `operation.disruption_cost` | exact `EnergyTransaction.work_expended` | `trial.energy.physical_work` |
| `heat_dissipated` | exact prepared settlement heat (currently zero) | exact `EnergyTransaction.heat_dissipated` | `trial.energy.dissipated_heat` |
| `resonance_withdrawn` | actual `resonant_work` returned by compatible withdrawal | measured zero; no resonance reservoir is used in this resolver | actual `resonant_funding` |
| `external_energy_input` | zero | zero | zero; resonance is an internal transfer, not external input |
| `settlement_status` | settled only after split succeeds | settled only after product insertion succeeds | settled only after all products are inserted |

The usable-holder delta in BREAK is not the complete physical-work value: resonance directly funds part of the disruption work, and the usable holder pays only the remainder plus the bond separation requirement. The summary must therefore preserve the full work and resonance withdrawal as separate quantities.

For COMBINE, the exact `EnergyTransaction` is created and settled in `combine_runtime::form_bond_from_candidate_diagnostic`. Carry its work/heat values alongside `CombineAttempt`; do not infer them from `work_cost`, `net_energy_change`, or cumulative ledger counters. The environmental resolver supplies the exact structural pre/post potential and the holder pre/post values.

For DISPLACEMENT, the `ReactionEnergyBalance` and actual resonance withdrawal are already available in the resolver. The usable delta must be measured across the complete staged operation, not copied from the transaction alone, because resonance is transferred into the usable holder before transaction settlement.

### Required summary invariants

For each committed event, the captured summary must be finite and satisfy the same represented world-level accounting identity as the committed transaction:

`usable_energy_delta - resonance_withdrawn + structural_energy_delta + physical_work + heat_dissipated = 0`

This is the expected signed change for the current three environmental paths, where resonance withdrawal is an internal reservoir decrease. Use a scale-aware numerical tolerance matching the existing energy-ledger validation policy; do not introduce a new energy tolerance or change settlement arithmetic.

The invariant is a diagnostic cross-check only. It must not be used to approve or reject a physical reaction. Failed/stale/unaffordable operations have no committed settlement and must carry `not_settled`, not a synthetic zero-valued settlement. Tests should compare the captured fields to the actual transaction/pre-post holders and existing ledger deltas for each reaction type, and should include a failed physical mutation after energy preparation to prove that staged energy is not reported as committed.
