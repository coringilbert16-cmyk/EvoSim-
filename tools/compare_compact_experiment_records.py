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


def compatibility_key(record: dict[str, Any]) -> tuple[Any, ...]:
    conditions = record.get("conditions", {})
    provenance = record.get("provenance", {})
    return (
        provenance.get("comparison_configuration_sha256"),
        tuple(conditions.get("active_resources") or []),
        conditions.get("requested_duration_seconds"),
        conditions.get("diagnostic_interval"),
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
            if committed_value is not None:
                committed.append(committed_value)
            if event_value is not None:
                events.append(event_value)
        reaction_totals[reaction] = {
            "committed_by_seed": committed,
            "event_count_by_seed": events,
            "committed_sum": sum(committed) if len(committed) == len(records) else None,
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
    }


def compare(paths: list[Path]) -> dict[str, Any]:
    records = [load_record(path) for path in paths]
    if not records:
        raise ValueError("at least one compact record is required")
    seeds = [record["identity"]["seed"] for record in records]
    if len(seeds) != len(set(seeds)):
        raise ValueError("duplicate independent seed records supplied")
    groups: dict[tuple[Any, ...], list[dict[str, Any]]] = {}
    for record in records:
        groups.setdefault(compatibility_key(record), []).append(record)
    compatible = len(groups) == 1
    return {
        "schema": "evosim.compact_record_comparison",
        "schema_version": 1,
        "source": "compact_permanent_records_only",
        "compatible": compatible,
        "compatibility_groups": [
            {
                "comparison_configuration_sha256": key[0],
                "active_resources": list(key[1]),
                "requested_duration_seconds": key[2],
                "diagnostic_interval": key[3],
                "summary": summarize(group),
            }
            for key, group in groups.items()
        ],
        "excluded_or_separated_records": (
            [] if compatible else [
                {"seed": record["identity"]["seed"],
                 "reason": "record conditions/configuration differ; retained in a separate compatibility group"}
                for record in records
            ]
        ),
        "quality": {
            "null_values_preserved": True,
            "within_seed_events_treated_as_independent": False,
            "pooled_rates_emitted": False,
            "raw_diagnostics_required": False,
        },
    }


def render_markdown(result: dict[str, Any]) -> str:
    lines = [
        "# Compact EvoSim experiment comparison",
        "",
        f"- Compatible conditions: {'yes' if result['compatible'] else 'no'}",
        f"- Compatibility groups: {len(result['compatibility_groups'])}",
        "- Source: compact permanent records only",
        "",
    ]
    for index, group in enumerate(result["compatibility_groups"], 1):
        summary = group["summary"]
        lines.extend([
            f"## Condition group {index}",
            "",
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
    print(f"Compared {len(args.records)} compact records; compatible={result['compatible']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
