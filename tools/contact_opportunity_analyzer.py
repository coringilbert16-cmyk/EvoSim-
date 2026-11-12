#!/usr/bin/env python3
"""Summarize per-tick environmental contact-opportunity diagnostics."""
from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path
from typing import Any

SEARCH_FIELDS = (
    "anchor_cells_inspected",
    "anchor_materials_inspected",
    "target_cells_inspected",
    "target_materials_inspected",
    "forward_ordered_target_material_pair_inspections",
    "target_material_pairs_restored",
    "target_material_pairs_restore_failed",
    "target_material_pairs_rejected_for_overlap",
    "constituent_pairs_available",
    "constituent_pairs_inspected",
    "constituent_pairs_uninspected_by_window",
    "constituent_pair_windows_truncated",
    "constituent_pairs_with_valid_interfaces",
    "constituent_pairs_without_valid_interfaces",
    "physically_evaluable_interfaces",
    "endpoint_pair_candidates_generated",
    "endpoint_pair_candidates_rejected_by_evaluation",
    "interfaces_sent_to_chemistry",
    "accumulation_keys_updated",
    "accumulation_keys_preserved_unsearched",
    "accumulation_keys_invalidated",
)
DISPLACEMENT_FIELDS = (
    "anchor_materials_inspected",
    "bonded_anchor_materials",
    "target_cells_inspected",
    "target_materials_inspected",
    "bonds_inspected",
    "replacement_contact_searches",
    "replacement_contacts_found",
    "activation_updates",
    "activation_threshold_met",
    "activation_below_threshold",
    "activation_status_unavailable",
    "trial_build_rejections_after_activation",
    "trial_rejections_old_bond_energy",
    "trial_rejections_topology",
    "trial_rejections_replacement",
    "trial_rejections_candidate_revalidation",
    "trial_rejections_formation_investment",
    "trial_rejections_physical_insertion",
    "trial_rejections_energy",
    "trial_rejections_products",
    "trial_rejections_other",
)
REACTION_FIELDS = (
    "combine_ready_candidates",
    "combine_scheduled",
    "combine_committed",
    "combine_rejected",
    "displacement_ready_candidates",
    "displacement_scheduled",
    "displacement_committed",
    "displacement_rejected",
)


def analyze(root: Path, seeds: list[int], duration: int) -> dict[str, Any]:
    per_seed: list[dict[str, Any]] = []
    all_ticks: list[dict[str, Any]] = []
    opportunity_samples: list[dict[str, Any]] = []
    totals = {field: 0 for field in (*SEARCH_FIELDS, *DISPLACEMENT_FIELDS, *REACTION_FIELDS)}
    for seed in seeds:
        path = root / f"seed-{seed}.jsonl"
        if not path.is_file():
            raise ValueError(f"seed {seed}: missing raw observation {path}")
        seed_totals = {field: 0 for field in (*SEARCH_FIELDS, *DISPLACEMENT_FIELDS, *REACTION_FIELDS)}
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
                row: dict[str, Any] = {"seed": seed, "tick": record.get("tick")}
                for field in SEARCH_FIELDS:
                    value = int(search.get(field, 0) or 0)
                    row[field] = value
                    seed_totals[field] += value
                for sample in search.get("opportunity_samples", []) or []:
                    if isinstance(sample, dict):
                        opportunity_samples.append({"seed": seed, "tick": record.get("tick"), **sample})
                displacement = work.get("displacement_discovery") or {}
                for field in DISPLACEMENT_FIELDS:
                    value = int(displacement.get(field, 0) or 0)
                    row[f"displacement_{field}"] = value
                    seed_totals[field] += value
                for field in REACTION_FIELDS:
                    value = int(work.get(field, 0) or 0)
                    row[field] = value
                    seed_totals[field] += value
                row["discovery_budget_exhausted"] = bool(search.get("budget_exhausted", False))
                row["target_cell_windows_capped"] = int(search.get("target_cell_windows_capped", 0) or 0)
                row["local_resonance_samples"] = int(search.get("local_resonance_samples", 0) or 0)
                ticks += 1
                all_ticks.append(row)
        per_seed.append({"seed": seed, "diagnostic_ticks": ticks, "totals": seed_totals})
        for field, value in seed_totals.items():
            totals[field] += value

    inspected = totals["constituent_pairs_inspected"]
    valid_pairs = totals["constituent_pairs_with_valid_interfaces"]
    interfaces = totals["physically_evaluable_interfaces"]
    report = {
        "schema_version": 1,
        "experiment": {"seeds": seeds, "duration_seconds_per_seed": duration},
        "per_seed": per_seed,
        "pooled_totals": totals,
        "conversion": {
            "broad_phase_constituent_pairs": totals["constituent_pairs_available"],
            "inspected_constituent_pairs": inspected,
            "inspected_pairs_with_valid_interfaces": valid_pairs,
            "physically_evaluable_interfaces": interfaces,
            "interfaces_sent_to_chemistry": totals["interfaces_sent_to_chemistry"],
            "valid_interface_fraction_of_inspected_pairs": valid_pairs / inspected if inspected else None,
            "interfaces_per_inspected_pair": interfaces / inspected if inspected else None,
            "combine_commits_per_interface_submitted": totals["combine_committed"] / totals["interfaces_sent_to_chemistry"] if totals["interfaces_sent_to_chemistry"] else None,
            "displacement_replacement_contact_searches": totals["replacement_contact_searches"],
            "displacement_replacement_contacts_found": totals["replacement_contacts_found"],
            "displacement_contacts_found_per_search": totals["replacement_contacts_found"] / totals["replacement_contact_searches"] if totals["replacement_contact_searches"] else None,
            "displacement_activation_threshold_met": totals["activation_threshold_met"],
            "displacement_activation_below_threshold": totals["activation_below_threshold"],
            "displacement_trial_rejections_after_activation": totals["trial_build_rejections_after_activation"],
        },
        "interpretation_limits": [
            "These are per-tick aggregate discovery counters, not a complete list of every molecule-pair encounter.",
            "A constituent-pair broad-phase overlap is a candidate opportunity, not proof of endpoint contact.",
            "A physically evaluable interface is passed to the chemistry accumulation step; it is not necessarily reaction-ready or committed.",
            "Bounded search windows leave some available pairs uninspected; uninspected counts must not be treated as rejected contacts.",
            "The counters do not yet attribute failed opportunities to individual molecule identities or preserve their full geometry.",
        ],
        "tick_diagnostics": all_ticks,
        "opportunity_samples": opportunity_samples,
        "opportunity_sample_outcomes": {
            "physically_valid": sum(bool(row.get("physically_valid")) for row in opportunity_samples),
            "physically_rejected": sum(not bool(row.get("physically_valid")) for row in opportunity_samples),
        },
    }
    return report


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
        "# Environmental contact-opportunity diagnostics",
        "",
        f"- Seeds: {', '.join(map(str, seeds))}",
        f"- Duration per seed: {args.duration_seconds} seconds",
        f"- Diagnostic ticks: {sum(row['diagnostic_ticks'] for row in report['per_seed'])}",
        "",
        "## Pooled discovery funnel",
        "",
    ]
    for key, value in report["conversion"].items():
        lines.append(f"- {key}: {value}")
    lines.extend(["", "## Interpretation limits", ""])
    lines.extend(f"- {item}" for item in report["interpretation_limits"])
    (args.output_dir / "contact-opportunity-analysis.md").write_text(
        "\n".join(lines) + "\n", encoding="utf-8"
    )
    sample_path = args.output_dir / "contact-opportunity-samples.json"
    sample_path.write_text(json.dumps(report["opportunity_samples"], indent=2) + "\\n", encoding="utf-8")
    if report["opportunity_samples"]:
        with (args.output_dir / "contact-opportunity-samples.csv").open("w", newline="", encoding="utf-8") as sample_stream:
            sample_writer = csv.DictWriter(sample_stream, fieldnames=list(report["opportunity_samples"][0]))
            sample_writer.writeheader()
            sample_writer.writerows(report["opportunity_samples"])
    with (args.output_dir / "contact-opportunity-ticks.csv").open("w", newline="", encoding="utf-8") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(report["tick_diagnostics"][0]) if report["tick_diagnostics"] else ["seed", "tick"])
        writer.writeheader()
        writer.writerows(report["tick_diagnostics"])
    print("\n".join(lines))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
