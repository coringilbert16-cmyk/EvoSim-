#!/usr/bin/env python3
"""Summarize environmental contact-opportunity diagnostics without inventing zeroes."""
from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path
from typing import Any

SEARCH_FIELDS = (
    "anchor_cells_inspected", "anchor_materials_inspected", "target_cells_inspected",
    "target_materials_inspected", "forward_ordered_target_material_pair_inspections",
    "target_material_pairs_restored", "target_material_pairs_restore_failed",
    "target_material_pairs_rejected_for_overlap", "constituent_pairs_available",
    "constituent_pairs_inspected", "constituent_pairs_uninspected_by_window",
    "constituent_pair_windows_truncated", "constituent_pairs_with_valid_interfaces",
    "constituent_pairs_without_valid_interfaces", "physically_evaluable_interfaces",
    "endpoint_pair_candidates_generated", "endpoint_pair_candidates_rejected_by_evaluation",
    "interfaces_sent_to_chemistry", "accumulation_keys_updated",
    "accumulation_keys_preserved_unsearched", "accumulation_keys_invalidated",
)
BREAK_FIELDS = (
    "scan_entries_inspected", "bond_candidates_inspected", "bond_candidates_checked",
    "material_restorations_succeeded", "material_restoration_rejections",
    "candidate_geometry_rejections", "candidate_endpoint_rejections",
    "candidate_mapping_rejections", "candidate_property_rejections",
    "candidate_position_rejections", "candidate_interface_rejections",
    "candidate_chemistry_rejections", "activation_updates",
    "activation_below_threshold", "activation_threshold_met",
    "stale_progress_invalidated", "progress_preserved_unsearched",
    "progress_dissipated_checked",
)
DISPLACEMENT_FIELDS = (
    "anchor_cells_available", "anchor_cells_inspected",
    "anchor_cells_unvisited_by_window", "anchor_cells_not_reached_in_selected_window",
    "target_cell_windows_inspected", "target_cell_window_limit_reached",
    "anchor_materials_inspected", "bonded_anchor_materials", "target_cells_inspected",
    "target_materials_inspected", "bonds_inspected", "replacement_contact_searches",
    "replacement_contacts_found", "activation_updates", "activation_threshold_met",
    "activation_below_threshold", "activation_status_unavailable",
    "trial_build_rejections_after_activation", "trial_rejections_old_bond_energy",
    "trial_rejections_topology", "trial_rejections_replacement",
    "trial_rejections_candidate_revalidation", "trial_rejections_formation_investment",
    "trial_rejections_physical_insertion", "trial_rejections_energy",
    "trial_rejections_products", "trial_rejections_other",
)
REACTION_FIELDS = (
    "combine_ready_candidates", "combine_scheduled", "combine_committed", "combine_rejected",
    "displacement_ready_candidates", "displacement_scheduled",
    "displacement_committed", "displacement_rejected",
)
ALL_FIELDS = (
    *SEARCH_FIELDS,
    *(f"break_{field}" for field in BREAK_FIELDS),
    *(f"displacement_{field}" for field in DISPLACEMENT_FIELDS),
    *REACTION_FIELDS,
)


def _counter(source: dict[str, Any], field: str) -> int | None:
    """Return a measured non-negative integer; absent, null, or malformed stays unknown."""
    if field not in source or source[field] is None or isinstance(source[field], bool):
        return None
    value = source[field]
    if isinstance(value, float) and not value.is_integer():
        return None
    try:
        parsed = int(value)
    except (TypeError, ValueError, OverflowError):
        return None
    return parsed if parsed >= 0 else None


def _coverage(state: str, domain: str, denominator: int | None, reason: str) -> dict[str, Any]:
    return {
        "state": state,
        "search_domain": domain,
        "domain_count": denominator,
        "reason": reason,
    }


def _combine_coverage(search: dict[str, Any]) -> dict[str, Any]:
    available = _counter(search, "constituent_pairs_available")
    inspected = _counter(search, "constituent_pairs_inspected")
    uninspected = _counter(search, "constituent_pairs_uninspected_by_window")
    exhausted = search.get("budget_exhausted")
    if None in (available, inspected, uninspected):
        return _coverage("unknown", "broad_phase_constituent_pairs", available,
                         "One or more authoritative pair-coverage counters are absent.")
    if inspected + uninspected != available:
        return _coverage("unknown", "broad_phase_constituent_pairs", available,
                         "Available, inspected, and uninspected counts do not reconcile.")
    if exhausted is True or uninspected > 0:
        return _coverage("partial", "broad_phase_constituent_pairs", available,
                         "The bounded search left available candidates uninspected or exhausted its budget.")
    if exhausted is False and uninspected == 0:
        return _coverage("complete", "broad_phase_constituent_pairs", available,
                         "All counted broad-phase candidates were inspected.")
    return _coverage("unknown", "broad_phase_constituent_pairs", available,
                     "The search budget status is absent; completeness cannot be asserted.")


def _break_coverage(discovery: dict[str, Any]) -> dict[str, Any]:
    scanned = _counter(discovery, "scan_entries_inspected")
    candidates = _counter(discovery, "bond_candidates_inspected")
    if scanned is None or candidates is None:
        return _coverage("unknown", "maintained_environmental_bond_index", scanned,
                         "Legacy or incomplete record lacks the full-index scan counters.")
    if scanned != candidates:
        return _coverage("unknown", "maintained_environmental_bond_index", scanned,
                         "Index entries inspected do not match bond candidates yielded.")
    return _coverage("complete", "maintained_environmental_bond_index", scanned,
                     "BREAK traverses the complete maintained bond index without a candidate budget.")


def _displacement_coverage(discovery: dict[str, Any], work: dict[str, Any]) -> dict[str, Any]:
    remaining = _counter(work, "displacement_discovery_work_remaining")
    anchor_window_unvisited = _counter(discovery, "anchor_cells_unvisited_by_window")
    anchor_selected_unvisited = _counter(discovery, "anchor_cells_not_reached_in_selected_window")
    target_limit_reached = _counter(discovery, "target_cell_window_limit_reached")
    if remaining == 0:
        return _coverage("partial", "bounded_anchor_target_and_contact_search", None,
                         "The per-tick discovery work allowance was fully consumed; total unvisited target work is not measured.")
    if anchor_window_unvisited is None or anchor_selected_unvisited is None or target_limit_reached is None:
        return _coverage("unknown", "bounded_anchor_target_and_contact_search", None,
                         "Coverage counters are absent in this record or legacy schema.")
    if anchor_window_unvisited > 0 or anchor_selected_unvisited > 0:
        return _coverage("partial", "bounded_anchor_target_and_contact_search", None,
                         "Known anchor cells were skipped by a fixed window; total unvisited target work is not measured.")
    if target_limit_reached > 0:
        return _coverage("unknown", "bounded_anchor_target_and_contact_search", None,
                         "At least one target-cell query reached its requested limit, but the exact omitted-cell count is not measured.")
    if remaining is None:
        return _coverage("unknown", "bounded_anchor_target_and_contact_search", None,
                         "No per-tick remaining-work value is available.")
    return _coverage("unknown", "bounded_anchor_target_and_contact_search", None,
                     "Anchor cells were visited, but total target materials and candidate work are not established as a complete domain.")


def _sum_known(values: list[int | None]) -> tuple[int, int]:
    return sum(value for value in values if value is not None), sum(value is None for value in values)


def analyze(root: Path, seeds: list[int], duration: int) -> dict[str, Any]:
    per_seed: list[dict[str, Any]] = []
    all_ticks: list[dict[str, Any]] = []
    opportunity_samples: list[dict[str, Any]] = []
    observed: dict[str, list[int | None]] = {field: [] for field in ALL_FIELDS}
    coverage_ticks = {"combine": [], "break": [], "displacement": []}

    for seed in seeds:
        path = root / f"seed-{seed}.jsonl"
        if not path.is_file():
            raise ValueError(f"seed {seed}: missing raw observation {path}")
        seed_values: dict[str, list[int | None]] = {field: [] for field in ALL_FIELDS}
        ticks = 0
        with path.open(encoding="utf-8") as stream:
            for line_number, line in enumerate(stream, 1):
                if not line.strip():
                    continue
                try:
                    record = json.loads(line)
                except json.JSONDecodeError as exc:
                    raise ValueError(f"{path}:{line_number}: invalid JSON: {exc}") from exc
                if record.get("event") != "environmental_chemistry_tick":
                    continue
                work = record.get("reaction_work") or {}
                search = work.get("combine_search") or {}
                breaking = work.get("break_discovery") or {}
                displacement = work.get("displacement_discovery") or {}
                row: dict[str, Any] = {"seed": seed, "tick": record.get("tick")}

                for field in SEARCH_FIELDS:
                    value = _counter(search, field)
                    row[field] = value
                    observed[field].append(value)
                    seed_values[field].append(value)
                for field in BREAK_FIELDS:
                    key = f"break_{field}"
                    value = _counter(breaking, field)
                    row[key] = value
                    observed[key].append(value)
                    seed_values[key].append(value)
                for field in DISPLACEMENT_FIELDS:
                    key = f"displacement_{field}"
                    value = _counter(displacement, field)
                    row[key] = value
                    observed[key].append(value)
                    seed_values[key].append(value)
                for field in REACTION_FIELDS:
                    value = _counter(work, field)
                    row[field] = value
                    observed[field].append(value)
                    seed_values[field].append(value)

                for sample in search.get("opportunity_samples", []) or []:
                    if isinstance(sample, dict):
                        opportunity_samples.append({"seed": seed, "tick": record.get("tick"), **sample})
                for sample in displacement.get("opportunity_samples", []) or []:
                    if isinstance(sample, dict):
                        opportunity_samples.append({"seed": seed, "tick": record.get("tick"), **sample})

                row["discovery_budget_exhausted"] = search.get("budget_exhausted")
                row["displacement_discovery_work_remaining"] = _counter(
                    work, "displacement_discovery_work_remaining"
                )
                row["displacement_discovery_budget_exhausted"] = (
                    row["displacement_discovery_work_remaining"] == 0
                    if row["displacement_discovery_work_remaining"] is not None else None
                )
                row["target_cell_windows_capped"] = _counter(search, "target_cell_windows_capped")
                row["local_resonance_samples"] = _counter(search, "local_resonance_samples")
                row["coverage"] = {
                    "combine": _combine_coverage(search),
                    "break": _break_coverage(breaking),
                    "displacement": _displacement_coverage(displacement, work),
                }
                for reaction in coverage_ticks:
                    coverage_ticks[reaction].append(row["coverage"][reaction])
                ticks += 1
                all_ticks.append(row)

        seed_totals = {}
        for field, values in seed_values.items():
            subtotal, missing = _sum_known(values)
            seed_totals[field] = subtotal if missing == 0 else None
        per_seed.append({"seed": seed, "diagnostic_ticks": ticks, "totals": seed_totals})

    pooled_totals: dict[str, int | None] = {}
    known_subtotals: dict[str, int] = {}
    missing_counts: dict[str, int] = {}
    for field, values in observed.items():
        subtotal, missing = _sum_known(values)
        known_subtotals[field] = subtotal
        missing_counts[field] = missing
        pooled_totals[field] = subtotal if missing == 0 else None

    inspected = pooled_totals["constituent_pairs_inspected"]
    valid_pairs = pooled_totals["constituent_pairs_with_valid_interfaces"]
    interfaces = pooled_totals["physically_evaluable_interfaces"]
    submitted = pooled_totals["interfaces_sent_to_chemistry"]
    commits = pooled_totals["combine_committed"]
    displacement_searches = pooled_totals["displacement_replacement_contact_searches"]
    displacement_contacts = pooled_totals["displacement_replacement_contacts_found"]

    conversion = {
        "broad_phase_constituent_pairs": pooled_totals["constituent_pairs_available"],
        "inspected_constituent_pairs": inspected,
        "inspected_pairs_with_valid_interfaces": valid_pairs,
        "physically_evaluable_interfaces": interfaces,
        "interfaces_sent_to_chemistry": submitted,
        "valid_interface_fraction_of_inspected_pairs": (
            valid_pairs / inspected if valid_pairs is not None and inspected not in (None, 0) else None
        ),
        "interfaces_per_inspected_pair": (
            interfaces / inspected if interfaces is not None and inspected not in (None, 0) else None
        ),
        "combine_commits_per_interface_submitted": (
            commits / submitted if commits is not None and submitted not in (None, 0) else None
        ),
        "displacement_replacement_contact_searches": displacement_searches,
        "displacement_replacement_contacts_found": displacement_contacts,
        "displacement_contacts_found_per_search": (
            displacement_contacts / displacement_searches
            if displacement_contacts is not None and displacement_searches not in (None, 0) else None
        ),
        "displacement_activation_threshold_met": pooled_totals["displacement_activation_threshold_met"],
        "displacement_activation_below_threshold": pooled_totals["displacement_activation_below_threshold"],
        "displacement_trial_rejections_after_activation": pooled_totals["displacement_trial_build_rejections_after_activation"],
    }
    coverage_summary = {}
    for reaction, rows in coverage_ticks.items():
        states = [row["state"] for row in rows]
        if not states:
            state = "unknown"
        elif all(item == "complete" for item in states):
            state = "complete"
        elif any(item == "partial" for item in states):
            state = "partial"
        else:
            state = "unknown"
        coverage_summary[reaction] = {
            "state": state,
            "ticks": len(states),
            "complete_ticks": states.count("complete"),
            "partial_ticks": states.count("partial"),
            "unknown_ticks": states.count("unknown"),
            "not_applicable_ticks": states.count("not_applicable"),
        }

    return {
        "schema_version": 2,
        "experiment": {"seeds": seeds, "duration_seconds_per_seed": duration},
        "per_seed": per_seed,
        "pooled_totals": pooled_totals,
        "pooled_known_subtotals": known_subtotals,
        "missing_tick_counts": missing_counts,
        "coverage": coverage_summary,
        "conversion": conversion,
        "interpretation_limits": [
            "Per-tick counters are repeated search observations, not unique opportunities across a run.",
            "COMBINE broad-phase constituent pairs are not proof of endpoint contact.",
            "BREAK's denominator is the maintained environmental bond index, not all material pairs.",
            "DISPLACEMENT replacement-contact searches are not unique incoming-molecule opportunities; materials may be revisited.",
            "A bounded search with unvisited work is partial, not a set of rejected opportunities.",
            "Missing legacy fields remain unknown; known subtotals are shown separately and are not complete pooled totals.",
            "Rates are emitted only when every required counter is present and its measured denominator is nonzero.",
            "DISPLACEMENT currently has no authoritative total-unvisited-work denominator, so complete coverage cannot be claimed.",
        ],
        "tick_diagnostics": all_ticks,
        "opportunity_samples": opportunity_samples,
        "opportunity_sample_outcomes": {
            "physically_valid": sum(bool(row.get("physically_valid")) for row in opportunity_samples),
            "physically_rejected": sum(not bool(row.get("physically_valid")) for row in opportunity_samples),
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input_dir", type=Path)
    parser.add_argument("--seeds", required=True)
    parser.add_argument("--duration-seconds", type=int, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    seeds = [int(part.strip()) for part in args.seeds.split(",") if part.strip()]
    if not seeds or len(seeds) != len(set(seeds)):
        parser.error("--seeds must be a nonempty list of unique integers")
    report = analyze(args.input_dir, seeds, args.duration_seconds)
    args.output_dir.mkdir(parents=True, exist_ok=True)
    (args.output_dir / "contact-opportunity-analysis.json").write_text(
        json.dumps(report, indent=2) + "\n", encoding="utf-8"
    )
    lines = [
        "# Environmental contact-opportunity diagnostics", "",
        f"- Seeds: {', '.join(map(str, seeds))}",
        f"- Duration per seed: {args.duration_seconds} seconds",
        f"- Diagnostic ticks: {sum(row['diagnostic_ticks'] for row in report['per_seed'])}",
        "", "## Coverage", "",
    ]
    for reaction, coverage in report["coverage"].items():
        lines.append(f"- {reaction}: {coverage['state']} "
                     f"({coverage['complete_ticks']} complete, {coverage['partial_ticks']} partial, "
                     f"{coverage['unknown_ticks']} unknown ticks)")
    lines.extend(["", "## Pooled discovery funnel", ""])
    for key, value in report["conversion"].items():
        lines.append(f"- {key}: {value}")
    lines.extend(["", "## Interpretation limits", ""])
    lines.extend(f"- {item}" for item in report["interpretation_limits"])
    (args.output_dir / "contact-opportunity-analysis.md").write_text(
        "\n".join(lines) + "\n", encoding="utf-8"
    )
    sample_path = args.output_dir / "contact-opportunity-samples.json"
    sample_path.write_text(json.dumps(report["opportunity_samples"], indent=2) + "\n", encoding="utf-8")
    if report["opportunity_samples"]:
        with (args.output_dir / "contact-opportunity-samples.csv").open(
            "w", newline="", encoding="utf-8"
        ) as sample_stream:
            sample_writer = csv.DictWriter(sample_stream, fieldnames=list(report["opportunity_samples"][0]))
            sample_writer.writeheader()
            sample_writer.writerows(report["opportunity_samples"])
    with (args.output_dir / "contact-opportunity-ticks.csv").open(
        "w", newline="", encoding="utf-8"
    ) as stream:
        writer = csv.DictWriter(
            stream,
            fieldnames=list(report["tick_diagnostics"][0]) if report["tick_diagnostics"] else ["seed", "tick"],
        )
        writer.writeheader()
        writer.writerows(report["tick_diagnostics"])
    print("\n".join(lines))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
