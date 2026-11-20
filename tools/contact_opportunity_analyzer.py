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


def _contact_key(
    reaction_kind: str,
    source_ids: Any,
    unit_indices: Any,
    endpoints: Any,
) -> tuple[Any, ...] | None:
    if not isinstance(source_ids, list) or len(source_ids) != 2:
        return None
    if not isinstance(unit_indices, (list, tuple)) or len(unit_indices) != 2:
        return None
    if not isinstance(endpoints, (list, tuple)) or len(endpoints) != 2:
        return None
    return (
        reaction_kind,
        int(source_ids[0]),
        int(source_ids[1]),
        int(unit_indices[0]),
        int(unit_indices[1]),
        json.dumps(endpoints[0], sort_keys=True, separators=(",", ":")),
        json.dumps(endpoints[1], sort_keys=True, separators=(",", ":")),
    )


def analyze(root: Path, seeds: list[int], duration: int) -> dict[str, Any]:
    per_seed: list[dict[str, Any]] = []
    all_ticks: list[dict[str, Any]] = []
    opportunity_samples: list[dict[str, Any]] = []
    reaction_outcomes: dict[tuple[Any, ...], list[dict[str, Any]]] = {}
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

                if record.get("event") == "environmental_reaction":
                    event = record.get("reaction") or {}
                    reaction_kind = event.get("reaction")
                    key = _contact_key(
                        reaction_kind,
                        event.get("source_material_ids"),
                        event.get("contact_unit_indices"),
                        event.get("contact_endpoints"),
                    )
                    if key is not None:
                        reaction_outcomes.setdefault(key, []).append({
                            "tick": event.get("tick"),
                            "outcome": event.get("outcome"),
                            "failure_reason": event.get("failure_reason"),
                        })
                    continue

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
                        opportunity_samples.append({
                            "seed": seed,
                            "tick": record.get("tick"),
                            "reaction_kind_name": "COMBINE",
                            **sample,
                        })
                displacement = work.get("displacement_discovery") or {}
                for sample in displacement.get("opportunity_samples", []) or []:
                    if isinstance(sample, dict):
                        opportunity_samples.append({
                            "seed": seed,
                            "tick": record.get("tick"),
                            "reaction_kind_name": "DISPLACEMENT",
                            **sample,
                        })
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

    neighborhood_rows: list[dict[str, Any]] = []
    neighborhood_summary: dict[str, dict[str, int]] = {
        "COMBINE": {
            "opportunity_samples": 0,
            "samples_with_neighbors": 0,
            "neighbors_recorded": 0,
            "neighbors_truncated": 0,
            "neighbors_missing_geometry": 0,
            "samples_with_exact_transaction_match": 0,
            "samples_with_multiple_transaction_matches": 0,
        },
        "DISPLACEMENT": {
            "opportunity_samples": 0,
            "samples_with_neighbors": 0,
            "neighbors_recorded": 0,
            "neighbors_truncated": 0,
            "neighbors_missing_geometry": 0,
            "samples_with_exact_transaction_match": 0,
            "samples_with_multiple_transaction_matches": 0,
        },
    }
    for sample in opportunity_samples:
        reaction_kind = sample.get("reaction_kind_name", "UNKNOWN")
        neighborhood = sample.get("local_neighborhood") or {}
        neighbors = neighborhood.get("molecules") or []
        recorded_neighbors = [neighbor for neighbor in neighbors if isinstance(neighbor, dict)]
        summary = neighborhood_summary.get(reaction_kind)
        if summary is not None:
            summary["opportunity_samples"] += 1
            summary["samples_with_neighbors"] += int(bool(recorded_neighbors))
            summary["neighbors_recorded"] += len(recorded_neighbors)
            summary["neighbors_truncated"] += int(neighborhood.get("truncated_molecule_count", 0) or 0)
            summary["neighbors_missing_geometry"] += int(neighborhood.get("missing_geometry_count", 0) or 0)

        key = _contact_key(
            reaction_kind,
            [sample.get("source_a_identity"), sample.get("source_b_identity")],
            [sample.get("unit_a"), sample.get("unit_b")],
            [sample.get("endpoint_a"), sample.get("endpoint_b")],
        )
        matches = reaction_outcomes.get(key, []) if key is not None else []
        sample["matching_reaction_outcomes"] = matches
        sample["exact_transaction_match_count"] = len(matches)
        if summary is not None:
            summary["samples_with_exact_transaction_match"] += int(bool(matches))
            summary["samples_with_multiple_transaction_matches"] += int(len(matches) > 1)

        for neighbor in recorded_neighbors:
            composition = neighbor.get("elemental_counts") or []
            neighborhood_rows.append({
                "seed": sample.get("seed"),
                "tick": sample.get("tick"),
                "reaction_kind": reaction_kind,
                "source_a_identity": sample.get("source_a_identity"),
                "source_b_identity": sample.get("source_b_identity"),
                "unit_a": sample.get("unit_a"),
                "unit_b": sample.get("unit_b"),
                "neighbor_identity": neighbor.get("identity"),
                "neighbor_distance": neighbor.get("distance_from_contact"),
                "neighbor_center_x": neighbor.get("center_x"),
                "neighbor_center_y": neighbor.get("center_y"),
                "neighbor_constituent_count": neighbor.get("constituent_count"),
                "neighbor_topology_signature": neighbor.get("topology_signature"),
                "carbon_count": composition[0] if len(composition) > 0 else 0,
                "hydrogen_count": composition[1] if len(composition) > 1 else 0,
                "nitrogen_count": composition[2] if len(composition) > 2 else 0,
                "oxygen_count": composition[3] if len(composition) > 3 else 0,
                "phosphorus_count": composition[4] if len(composition) > 4 else 0,
                "sulfur_count": composition[5] if len(composition) > 5 else 0,
                "unclassified_constituents": neighbor.get("unclassified_constituents", 0),
                "environmental_usable_energy": sample.get("environmental_usable_energy"),
                "environmental_resonant_energy": sample.get("environmental_resonant_energy"),
                "matched_transaction_count": len(matches),
            })

    inspected = totals["constituent_pairs_inspected"]
    valid_pairs = totals["constituent_pairs_with_valid_interfaces"]
    interfaces = totals["physically_evaluable_interfaces"]
    report = {
        "schema_version": 2,
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
        "local_neighborhood_summary": neighborhood_summary,
        "interpretation_limits": [
            "The local neighborhood is a diagnostic observation window extending two spatial cells around the sampled contact midpoint; it is not a chemistry contact cutoff or proof of facilitation.",
            "At most 16 neighboring molecules are retained per sample; truncation and missing-geometry counts are reported.",
            "Neighbor composition is recorded as C, H, N, O, P, S counts plus an unclassified count; topology signatures are descriptors, not exact graph-isomorphism proofs.",
            "Environmental usable and resonant energy values are global reservoir context, not local field energy density or energy owned by neighboring molecules.",
            "Opportunity samples remain bounded and deterministic; they are not a complete unbiased census of every eligible encounter.",
            "An exact transaction match links the sampled source pair, constituent indices, and endpoints to a later reaction event. A missing match does not prove that no reaction occurred.",
            "A nearby molecule is a candidate facilitator only. Causal facilitation still requires paired present/absent controls and evidence that the candidate remains chemically available.",
            "A physically evaluable interface is passed to chemistry accumulation; it is not necessarily reaction-ready or committed.",
        ],
        "tick_diagnostics": all_ticks,
        "opportunity_samples": opportunity_samples,
        "opportunity_sample_outcomes": {
            "physically_valid": sum(bool(row.get("physically_valid")) for row in opportunity_samples),
            "physically_rejected": sum(not bool(row.get("physically_valid")) for row in opportunity_samples),
            "exact_transaction_matches": sum(bool(row.get("exact_transaction_match_count")) for row in opportunity_samples),
        },
        "neighborhood_samples": neighborhood_rows,
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
    sample_path.write_text(json.dumps(report["opportunity_samples"], indent=2) + "\n", encoding="utf-8")
    neighborhood_path = args.output_dir / "contact-opportunity-neighborhoods.csv"
    neighborhood_rows = report["neighborhood_samples"]
    if neighborhood_rows:
        with neighborhood_path.open("w", newline="", encoding="utf-8") as neighborhood_stream:
            neighborhood_writer = csv.DictWriter(
                neighborhood_stream, fieldnames=list(neighborhood_rows[0])
            )
            neighborhood_writer.writeheader()
            neighborhood_writer.writerows(neighborhood_rows)
    else:
        neighborhood_path.write_text("", encoding="utf-8")
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
