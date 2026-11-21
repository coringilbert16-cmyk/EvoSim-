#!/usr/bin/env python3
"""Compare compact permanent EvoSim experiment records without raw diagnostics."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

SCHEMA = "evosim.permanent_experiment_record"


def load_record(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict) or value.get("schema") != SCHEMA:
        raise ValueError(f"{path}: unsupported permanent experiment record")
    if value.get("schema_version") != 1:
        raise ValueError(f"{path}: unsupported record schema version")
    identity = value.get("identity")
    if not isinstance(identity, dict) or not isinstance(identity.get("seed"), int):
        raise ValueError(f"{path}: missing independent seed identity")
    return value


def measurement_compatibility_key(record: dict[str, Any]) -> tuple[Any, ...]:
    conditions = record.get("conditions", {})
    provenance = record.get("provenance", {})
    coverage = record.get("opportunity_coverage") or {}
    active_resources = conditions.get("active_resources")
    tiers = provenance.get("enabled_diagnostic_tiers")
    return (
        record.get("schema_version"),
        tuple(sorted(active_resources)) if isinstance(active_resources, list) else None,
        "requested_wall_clock_seconds"
            if numeric(conditions.get("requested_duration_seconds")) is not None else None,
        conditions.get("requested_duration_seconds"),
        conditions.get("diagnostic_interval"),
        provenance.get("manifest_schema_version"),
        provenance.get("chemistry_analyzer_sha256"),
        provenance.get("organization_analyzer_sha256"),
        tuple(sorted(tiers)) if isinstance(tiers, list) else None,
        coverage.get("source_schema_version"),
    )


def measurement_compatibility_warnings(record: dict[str, Any]) -> list[str]:
    conditions = record.get("conditions", {})
    provenance = record.get("provenance", {})
    missing = []
    if not isinstance(conditions.get("active_resources"), list):
        missing.append("active-resource set")
    if numeric(conditions.get("requested_duration_seconds")) is None:
        missing.append("requested wall-clock duration")
    if numeric(conditions.get("diagnostic_interval")) is None:
        missing.append("diagnostic interval")
    for field in ("manifest_schema_version", "chemistry_analyzer_sha256",
                  "organization_analyzer_sha256"):
        if provenance.get(field) is None:
            missing.append(field)
    return missing


def condition_group_key(record: dict[str, Any]) -> tuple[Any, ...]:
    identity = record.get("identity", {})
    provenance = record.get("provenance", {})
    return (
        identity.get("experiment_id"),
        identity.get("condition_id"),
        provenance.get("comparison_configuration_sha256"),
        *measurement_compatibility_key(record),
    )


def numeric(value: Any) -> int | float | None:
    return value if isinstance(value, (int, float)) and not isinstance(value, bool) else None


def summarize(records: list[dict[str, Any]]) -> dict[str, Any]:
    reaction_totals: dict[str, dict[str, int]] = {}
    for reaction in ("BREAK", "COMBINE", "DISPLACEMENT"):
        committed = []
        events = []
        for record in records:
            row = record.get("reactions", {}).get(reaction, {})
            diagnostics = row.get("diagnostics") or {}
            committed_value = numeric(diagnostics.get("committed"))
            event_value = numeric(row.get("event_count"))
            committed.append(committed_value)
            events.append(event_value)
        reaction_totals[reaction] = {
            "committed_by_seed": committed,
            "event_count_by_seed": events,
            "committed_sum": sum(committed) if all(value is not None for value in committed) else None,
        }
    return {
        "independent_seed_count": len(records),
        "seeds": [row["identity"]["seed"] for row in records],
        "reaction_totals": reaction_totals,
        "molecular_development": {
            key: [numeric(row.get("molecular_development", {}).get(key)) for row in records]
            for key in ("creation_count", "destruction_count", "active_at_end_count",
                        "unique_structural_families", "recurrent_structural_families",
                        "compound_identity_count")
        },
        "energy": {
            key: [numeric(row.get("energy", {}).get(key)) for row in records]
            for key in ("last_balance_residual", "maximum_absolute_snapshot_residual")
        },
        "organization_by_seed": [row.get("organization") for row in records],
        "opportunity_coverage_by_seed": [
            row.get("opportunity_coverage") for row in records
        ],
        "settlement_by_seed": [row.get("settlement") for row in records],
        "settlement_totals_by_seed": {
            reaction: {
                field: [
                    numeric((row.get("settlement") or {}).get(reaction, {}).get(field, {}).get("sum"))
                    for row in records
                ]
                for field in ("usable_energy_delta", "resonance_withdrawn",
                              "structural_energy_delta", "physical_work", "heat_dissipated")
            }
            for reaction in ("BREAK", "COMBINE", "DISPLACEMENT")
        },
        "bond_accounting_by_seed": [row.get("bond_accounting") for row in records],
        "bond_accounting_by_reaction_by_seed": [
            (row.get("bond_accounting") or {}).get("by_reaction") for row in records
        ],
        "bond_deltas_by_seed": {
            field: [numeric(row.get("bond_accounting", {}).get(field)) for row in records]
            for field in ("formed_total", "broken_total")
        },
        "inventory_by_seed": [row.get("inventory") for row in records],
    }


def comparison_vectors(summary: dict[str, Any], records: list[dict[str, Any]]) -> dict[str, list[int | float | None]]:
    vectors: dict[str, list[int | float | None]] = {}
    for reaction in ("BREAK", "COMBINE", "DISPLACEMENT"):
        vectors[f"{reaction}.committed"] = summary["reaction_totals"][reaction]["committed_by_seed"]
        for field, values in summary["settlement_totals_by_seed"][reaction].items():
            vectors[f"{reaction}.settlement.{field}"] = values
    for metric, values in summary["molecular_development"].items():
        vectors[f"molecular_development.{metric}"] = values
    for metric, values in summary["energy"].items():
        vectors[f"energy.{metric}"] = values
    for metric, values in summary["bond_deltas_by_seed"].items():
        vectors[f"bond_accounting.{metric}"] = values
    for reaction in ("BREAK", "COMBINE", "DISPLACEMENT"):
        for field in ("formed_total", "broken_total"):
            vectors[f"bond_accounting.{reaction}.{field}"] = [
                numeric(((row.get("bond_accounting") or {}).get("by_reaction") or {})
                        .get(reaction, {}).get(field))
                for row in records
            ]
    return vectors


def mean_if_complete(values: list[int | float | None]) -> float | None:
    if not values or any(value is None for value in values):
        return None
    return sum(values) / len(values)


def compare(paths: list[Path]) -> dict[str, Any]:
    records = [load_record(path) for path in paths]
    if not records:
        raise ValueError("at least one compact record is required")
    measurement_groups: dict[tuple[Any, ...], list[dict[str, Any]]] = {}
    condition_groups: dict[tuple[Any, ...], list[dict[str, Any]]] = {}
    seeds_by_condition: dict[tuple[Any, ...], set[int]] = {}
    for record in records:
        condition_key = condition_group_key(record)
        seed = record["identity"]["seed"]
        seen = seeds_by_condition.setdefault(condition_key, set())
        if seed in seen:
            raise ValueError("duplicate independent seed within the same condition")
        seen.add(seed)
        measurement_groups.setdefault(measurement_compatibility_key(record), []).append(record)
        condition_groups.setdefault(condition_key, []).append(record)

    rendered_groups = []
    for index, (key, group) in enumerate(condition_groups.items(), 1):
        summary = summarize(group)
        rendered_groups.append({
            "group_id": index,
            "experiment_id": key[0],
            "condition_id": key[1],
            "comparison_configuration_sha256": key[2],
            "measurement_schema_version": key[3],
            "active_resources": list(key[4]) if key[4] is not None else None,
            "duration_interpretation": key[5],
            "requested_duration_seconds": key[6],
            "diagnostic_interval": key[7],
            "manifest_schema_version": key[8],
            "chemistry_analyzer_sha256": key[9],
            "organization_analyzer_sha256": key[10],
            "enabled_diagnostic_tiers": list(key[11]) if key[11] is not None else None,
            "opportunity_coverage_schema_version": key[12],
            "summary": summary,
        })

    comparisons = []
    if len(measurement_groups) == 1 and len(rendered_groups) > 1:
        baseline = rendered_groups[0]
        baseline_records = condition_groups[condition_group_key(records[0])]
        baseline_vectors = comparison_vectors(baseline["summary"], baseline_records)
        baseline_means = {name: mean_if_complete(values) for name, values in baseline_vectors.items()}
        for candidate in rendered_groups[1:]:
            candidate_records = next(
                group for key, group in condition_groups.items()
                if key[0] == candidate["experiment_id"] and key[1] == candidate["condition_id"]
                and key[2] == candidate["comparison_configuration_sha256"]
            )
            candidate_vectors = comparison_vectors(candidate["summary"], candidate_records)
            differences = {}
            baseline_seed_order = [row["identity"]["seed"] for row in baseline_records]
            candidate_seed_order = [row["identity"]["seed"] for row in candidate_records]
            common_seeds = sorted(set(baseline_seed_order) & set(candidate_seed_order))
            for name, values in candidate_vectors.items():
                left_mean = baseline_means.get(name)
                right_mean = mean_if_complete(values)
                baseline_by_seed = dict(zip(baseline_seed_order, baseline_vectors[name]))
                candidate_by_seed = dict(zip(candidate_seed_order, values))
                paired = [
                    {
                        "seed": seed,
                        "difference": (
                            candidate_by_seed[seed] - baseline_by_seed[seed]
                            if candidate_by_seed[seed] is not None and baseline_by_seed[seed] is not None
                            else None
                        ),
                    }
                    for seed in common_seeds
                ]
                paired_mean = mean_if_complete([row["difference"] for row in paired])
                differences[name] = {
                    "baseline_mean": left_mean,
                    "comparison_mean": right_mean,
                    "difference": right_mean - left_mean
                        if left_mean is not None and right_mean is not None else None,
                    "paired_seed_differences": paired,
                    "mean_paired_difference": paired_mean,
                }
            comparisons.append({
                "baseline_group_id": baseline["group_id"],
                "comparison_group_id": candidate["group_id"],
                "descriptive_differences_only": True,
                "metrics": differences,
            })

    missing_definition_fields = {
        record["identity"]["seed"]: measurement_compatibility_warnings(record)
        for record in records
        if measurement_compatibility_warnings(record)
    }
    compatible = len(measurement_groups) == 1 and not missing_definition_fields
    return {
        "schema": "evosim.compact_record_comparison",
        "schema_version": 1,
        "source": "compact_permanent_records_only",
        "measurement_compatible": compatible,
        "measurement_compatibility_group_count": len(measurement_groups),
        "measurement_definition_missing_by_seed": missing_definition_fields,
        "condition_groups": rendered_groups,
        "condition_comparisons": comparisons,
        "separated_records": (
            [] if compatible else [
                {"seed": record["identity"]["seed"],
                 "reason": (
                     "measurement definitions are incomplete: "
                     + ", ".join(measurement_compatibility_warnings(record))
                     if measurement_compatibility_warnings(record)
                     else "measurement conditions differ (active resources, duration interpretation, diagnostic interval, analyzer definitions, diagnostic tier, coverage schema, or record schema)"
                 )}
                for record in records
            ]
        ),
        "quality": {
            "null_values_preserved": True,
            "within_seed_events_treated_as_independent": False,
            "pooled_rates_emitted": False,
            "raw_diagnostics_required": False,
            "between_condition_differences_are_descriptive_only": True,
        },
    }


def render_markdown(result: dict[str, Any]) -> str:
    lines = [
        "# Compact EvoSim experiment comparison",
        "",
        f"- Compatible conditions: {'yes' if result['measurement_compatible'] else 'no'}",
        f"- Compatibility groups: {result['measurement_compatibility_group_count']}",
        "- Source: compact permanent records only",
        f"- Measurement definitions complete: {'yes' if not result['measurement_definition_missing_by_seed'] else 'no'}",
        "",
    ]
    for index, group in enumerate(result["condition_groups"], 1):
        summary = group["summary"]
        lines.extend([
            f"## Condition group {index}",
            "",
            f"- Experiment: {group['experiment_id'] or 'unregistered'}",
            f"- Condition: {group['condition_id'] or 'unspecified'}",
            f"- Independent seeds: {', '.join(map(str, summary['seeds']))}",
            f"- Requested duration: {group['requested_duration_seconds']} seconds",
            f"- Active resources: {', '.join(group['active_resources'] or [])}",
            "",
            "| Reaction | Committed by seed | Complete sum |",
            "|---|---|---:|",
        ])
        for reaction, totals in summary["reaction_totals"].items():
            per_seed = ", ".join("unknown" if value is None else str(value)
                                 for value in totals["committed_by_seed"])
            total = totals["committed_sum"]
            lines.append(f"| {reaction} | {per_seed or 'unknown'} | {total if total is not None else 'unknown'} |")
        lines.extend(["", "### Molecular development", ""])
        for metric, values in summary["molecular_development"].items():
            rendered = ", ".join("unknown" if value is None else str(value) for value in values)
            lines.append(f"- {metric}: {rendered or 'unknown'}")
        lines.append("")
    if result["condition_comparisons"]:
        lines.extend(["", "## Descriptive condition differences", "",
                      "| Metric | Baseline mean | Comparison mean | Difference |",
                      "|---|---:|---:|---:|"])
        for comparison in result["condition_comparisons"]:
            for metric, values in comparison["metrics"].items():
                left = values["baseline_mean"]
                right = values["comparison_mean"]
                delta = values["difference"]
                lines.append(
                    f"| {metric} | {left if left is not None else 'unknown'} | "
                    f"{right if right is not None else 'unknown'} | "
                    f"{delta if delta is not None else 'unknown'} |"
                )
    lines.extend([
        "## Data-quality rules",
        "",
        "- Null values remain unknown; they are not converted to measured zero.",
        "- Incompatible conditions are separated into distinct groups.",
        "- Seeds are independent replicates; reaction events within a seed are not independent samples.",
        "- No pooled reaction rates are emitted by this comparison.",
        "- Raw diagnostics and specialist intermediate files are not required.",
        "",
    ])
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("records", nargs="+", type=Path, help="compact permanent record JSON files")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--markdown-out", type=Path)
    args = parser.parse_args()
    result = compare(args.records)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    if args.markdown_out:
        args.markdown_out.parent.mkdir(parents=True, exist_ok=True)
        args.markdown_out.write_text(render_markdown(result), encoding="utf-8")
    print(f"Compared {len(args.records)} compact records; compatible={result['measurement_compatible']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
