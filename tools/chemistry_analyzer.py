#!/usr/bin/env python3
"""Offline analyzer for EvoSim environmental-chemistry JSONL diagnostics.

This tool is observational only. It never imports or changes simulation code.
"""
from __future__ import annotations

import argparse
import json
import sys
from collections import Counter
from pathlib import Path
from typing import Any

CANONICAL_RESOURCES = (
    "Water", "Nitrogen", "Phosphorus", "Carbon", "Sulfur", "Hydrogen", "Oxygen"
)


def load_records(path: Path) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    with path.open(encoding="utf-8") as stream:
        for line_number, line in enumerate(stream, 1):
            if not line.strip():
                continue
            try:
                value = json.loads(line)
            except json.JSONDecodeError as exc:
                raise ValueError(f"{path}:{line_number}: invalid JSON: {exc}") from exc
            if not isinstance(value, dict):
                raise ValueError(f"{path}:{line_number}: expected a JSON object")
            records.append(value)
    if not records:
        raise ValueError(f"{path}: no diagnostic records found")
    return records


def _number(value: Any) -> float | int | None:
    return value if isinstance(value, (int, float)) and not isinstance(value, bool) else None


def analyze(records: list[dict[str, Any]]) -> dict[str, Any]:
    headers = [r for r in records if r.get("event") == "environmental_chemistry_header"]
    snapshots = [r for r in records if r.get("event") == "environmental_chemistry_snapshot"]
    ticks = [r for r for r in []]  # replaced below to keep parsing explicit
    ticks = [r for r in records if r.get("event") == "environmental_chemistry_tick"]
    reactions = [r.get("reaction", {}) for r in records if r.get("event") == "environmental_reaction"]
    summaries = [r for r in records if r.get("event") == "environmental_chemistry_summary"]

    if not headers:
        raise ValueError("missing environmental_chemistry_header")
    if not snapshots:
        raise ValueError("missing environmental_chemistry_snapshot")

    header = headers[0]
    initial_inventory = header.get("initial_material_inventory", {})
    if not isinstance(initial_inventory, dict):
        raise ValueError("header initial_material_inventory must be an object")
    missing_resources = [name for name in CANONICAL_RESOURCES
                         if _number(initial_inventory.get(name)) is None
                         or float(initial_inventory.get(name, 0)) <= 0.0]

    snapshot_rows = []
    for snap in snapshots:
        inventory = snap.get("material_inventory", {})
        if not isinstance(inventory, dict):
            inventory = {}
        amounts = {name: _number(inventory.get(name)) for name in CANONICAL_RESOURCES}
        snapshot_rows.append({
            "label": snap.get("label"),
            "tick": snap.get("tick"),
            "physical_material_count": snap.get("physical_material_count"),
            "constituent_count": snap.get("constituent_count"),
            "internal_bond_count": snap.get("internal_bond_count"),
            "resource_inventory": amounts,
            "missing_resources": [name for name, amount in amounts.items()
                                  if amount is None or float(amount) <= 0.0],
            "energy": snap.get("energy", {}),
        })

    reaction_counts = Counter()
    outcome_counts = Counter()
    failure_reasons = Counter()
    for event in reactions:
        kind = str(event.get("reaction", "UNKNOWN"))
        outcome = str(event.get("outcome", "UNKNOWN"))
        reaction_counts[kind] += 1
        outcome_counts[f"{kind}:{outcome}"] += 1
        reason = event.get("failure_reason")
        if reason:
            failure_reasons[f"{kind}:{reason}"] += 1

    # Prefer the recorder's accumulated totals; tick records are a fallback.
    if summaries:
        summary = summaries[-1]
        reaction_totals = {
            "BREAK": {
                "ready": summary.get("break_ready_candidates", 0),
                "scheduled": summary.get("break_scheduled", 0),
                "committed": summary.get("break_committed", 0),
                "rejected": summary.get("break_rejected", 0),
                "failure_reasons": summary.get("break_failure_reasons", {}),
            },
            "COMBINE": {
                "ready": summary.get("combine_ready_candidates", 0),
                "scheduled": summary.get("combine_scheduled", 0),
                "committed": summary.get("combine_committed", 0),
                "rejected": summary.get("combine_rejected", 0),
            },
            "DISPLACEMENT": {
                "ready": summary.get("displacement_ready_candidates", 0),
                "scheduled": summary.get("displacement_scheduled", 0),
                "committed": summary.get("displacement_committed", 0),
                "rejected": summary.get("displacement_rejected", 0),
                "failure_reasons": summary.get("displacement_failure_reasons", {}),
                "participant_sets": summary.get("displacement_participant_sets", {}),
                "discovery": summary.get("displacement_discovery", {}),
                "discovery_budget_exhausted_ticks":
                    summary.get("displacement_discovery_budget_exhausted_ticks", 0),
            },
        }
    else:
        reaction_totals = {}
        for kind, prefix in (("BREAK", "break"), ("COMBINE", "combine"),
                             ("DISPLACEMENT", "displacement")):
            keys = ("ready_candidates", "scheduled", "committed", "rejected")
            reaction_totals[kind] = {
                key.removesuffix("_candidates"): sum(
                    int((r.get("reaction_work") or {}).get(f"{prefix}_{key}", 0) or 0)
                    for r in ticks
                ) for key in keys
            }

    energy_residuals = [
        _number((snap.get("energy") or {}).get("balance_residual"))
        for snap in snapshots
    ]
    energy_residuals = [v for v in energy_residuals if v is not None]

    return {
        "schema_version": 1,
        "input": {
            "seed": header.get("seed"),
            "requested_duration_seconds": header.get("requested_duration_seconds"),
            "planned_ticks": header.get("planned_ticks"),
            "initial_physical_material_count": header.get("initial_physical_material_count"),
            "initial_independent_resource_count": header.get("initial_independent_resource_count"),
            "initial_molecule_count": header.get("initial_molecule_count"),
            "diagnostic_interval": header.get("diagnostic_interval"),
        },
        "resource_coverage": {
            "canonical_resources": list(CANONICAL_RESOURCES),
            "initial_inventory": {name: initial_inventory.get(name) for name in CANONICAL_RESOURCES},
            "missing_at_start": missing_resources,
            "all_seven_present_at_start": not missing_resources,
            "snapshots": snapshot_rows,
            "snapshots_missing_resources": [
                {"label": row["label"], "tick": row["tick"], "missing": row["missing_resources"]}
                for row in snapshot_rows if row["missing_resources"]
            ],
        },
        "sampling": {
            "snapshot_count": len(snapshots),
            "tick_diagnostic_count": len(ticks),
            "reaction_event_count": len(reactions),
            "summary_present": bool(summaries),
        },
        "reactions": {
            "event_counts": dict(sorted(reaction_counts.items())),
            "outcome_counts": dict(sorted(outcome_counts.items())),
            "failure_reasons": dict(sorted(failure_reasons.items())),
            "diagnostic_totals": reaction_totals,
        },
        "energy": {
            "first_balance_residual": energy_residuals[0] if energy_residuals else None,
            "last_balance_residual": energy_residuals[-1] if energy_residuals else None,
            "maximum_absolute_snapshot_residual":
                max((abs(float(v)) for v in energy_residuals), default=None),
        },
    }


def render_markdown(report: dict[str, Any]) -> str:
    coverage = report["resource_coverage"]
    lines = [
        "# EvoSim chemistry analysis",
        "",
        f"- Seed: {report['input']['seed']}",
        f"- Requested duration: {report['input']['requested_duration_seconds']} seconds",
        f"- Snapshots: {report['sampling']['snapshot_count']}",
        f"- Reaction events: {report['sampling']['reaction_event_count']}",
        "",
        "## Seven-resource coverage",
        "",
        "| Resource | Initial amount |",
        "|---|---:|",
    ]
    for name in CANONICAL_RESOURCES:
        lines.append(f"| {name} | {coverage['initial_inventory'].get(name, 'missing')} |")
    lines += [
        "",
        f"All seven present initially: **{'yes' if coverage['all_seven_present_at_start'] else 'NO'}**",
        "",
        "## Reaction event counts",
        "",
        "| Reaction | Events |",
        "|---|---:|",
    ]
    for name, count in report["reactions"]["event_counts"].items():
        lines.append(f"| {name} | {count} |")
    lines += ["", "## Reaction diagnostic totals", ""]
    for name, values in report["reactions"]["diagnostic_totals"].items():
        lines.append(f"### {name}")
        lines.append("")
        for key, value in values.items():
            if isinstance(value, (dict, list)):
                continue
            lines.append(f"- {key}: {value}")
        lines.append("")
    lines += [
        "## Energy accounting",
        "",
        f"- First snapshot residual: {report['energy']['first_balance_residual']}",
        f"- Final snapshot residual: {report['energy']['last_balance_residual']}",
        f"- Maximum absolute snapshot residual: {report['energy']['maximum_absolute_snapshot_residual']}",
        "",
        "## Interpretation limits",
        "",
        "- This report describes recorded inventory and reaction diagnostics; it does not infer molecular lifetimes or reaction pathways.",
        "- The current recorder does not provide a complete persistent molecular identity/lineage history.",
        "- Resource coverage is checked against all seven canonical resource names; none is excluded based on reaction likelihood.",
        "",
    ]
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path, help="environmental chemistry JSONL diagnostics")
    parser.add_argument("--json-out", type=Path, help="write machine-readable analysis JSON")
    parser.add_argument("--markdown-out", type=Path, help="write Markdown analysis report")
    args = parser.parse_args()
    try:
        report = analyze(load_records(args.input))
    except (OSError, ValueError) as exc:
        print(f"chemistry analyzer: {exc}", file=sys.stderr)
        return 2

    if args.json_out:
        args.json_out.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    if args.markdown_out:
        args.markdown_out.write_text(render_markdown(report), encoding="utf-8")
    if not args.json_out and not args.markdown_out:
        print(render_markdown(report), end="")
    if not report["resource_coverage"]["all_seven_present_at_start"]:
        print("chemistry analyzer: initial inventory does not contain all seven resources",
              file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
