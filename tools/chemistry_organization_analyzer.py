#!/usr/bin/env python3
"""Offline analysis of recurring molecular organization in EvoSim JSONL logs.

This analyzer is observational only. Structural families use exact normalized
serialized composition/topology, not graph-isomorphism or chemical equivalence.
"""
from __future__ import annotations

import argparse
import json
import statistics
import sys
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any


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


def _molecule_signature(molecule: Any) -> str:
    if not isinstance(molecule, dict):
        molecule = {}
    material = molecule.get("material")
    if not isinstance(material, dict):
        material = {}
    raw_parts = material.get("parts", [])
    parts: list[tuple[str, float]] = []
    if isinstance(raw_parts, list):
        for part in raw_parts:
            if not isinstance(part, (list, tuple)) or len(part) != 2:
                continue
            try:
                amount = float(part[1])
            except (TypeError, ValueError):
                continue
            parts.append((str(part[0]), amount))
    raw_edges = molecule.get("internal_connections", [])
    edges = raw_edges if isinstance(raw_edges, list) else []
    normalized_edges = sorted(
        json.dumps(edge, sort_keys=True, separators=(",", ":"))
        for edge in edges if isinstance(edge, dict)
    )
    return json.dumps(
        {"composition": sorted(parts), "connections": normalized_edges},
        sort_keys=True, separators=(",", ":"),
    )


def _composition(molecule: Any) -> dict[str, float]:
    if not isinstance(molecule, dict):
        return {}
    material = molecule.get("material")
    if not isinstance(material, dict):
        return {}
    parts = material.get("parts", [])
    result: dict[str, float] = defaultdict(float)
    if isinstance(parts, list):
        for part in parts:
            if isinstance(part, (list, tuple)) and len(part) == 2:
                try:
                    amount = float(part[1])
                except (TypeError, ValueError):
                    continue
                if amount > 0:
                    result[str(part[0])] += amount
    return dict(result)


def analyze(records: list[dict[str, Any]]) -> dict[str, Any]:
    lifecycle = [r for r in records if r.get("event") == "molecular_lifecycle"]
    if not lifecycle:
        raise ValueError("no molecular_lifecycle records; run with molecular history enabled")

    last_tick = max(
        (int(r["tick"]) for r in records
         if isinstance(r.get("tick"), int) and not isinstance(r.get("tick"), bool)),
        default=0,
    )
    created: dict[int, dict[str, Any]] = {}
    destroyed: dict[int, int] = {}
    errors: list[str] = []

    for index, event in enumerate(lifecycle):
        action = event.get("action")
        molecule_id = event.get("molecule_id")
        tick = event.get("tick")
        if not isinstance(molecule_id, int) or isinstance(molecule_id, bool) or molecule_id <= 0:
            errors.append(f"record {index}: invalid molecule_id")
            continue
        if not isinstance(tick, int) or isinstance(tick, bool) or tick < 0:
            errors.append(f"record {index}: invalid tick")
            continue
        if action == "created":
            if molecule_id in created:
                errors.append(f"record {index}: duplicate creation for {molecule_id}")
                continue
            parents = event.get("parent_ids", [])
            if not isinstance(parents, list) or any(
                not isinstance(parent, int) or isinstance(parent, bool) or parent <= 0
                for parent in parents
            ):
                errors.append(f"record {index}: invalid parent_ids for {molecule_id}")
                continue
            for parent_id in parents:
                if parent_id not in created:
                    errors.append(f"record {index}: lineage parent {parent_id} has no prior creation")
                elif parent_id in destroyed and destroyed[parent_id] <= tick:
                    errors.append(f"record {index}: lineage parent {parent_id} was already destroyed")
            created[molecule_id] = {
                "tick": tick,
                "parents": parents,
                "reaction": str(event.get("reaction", "UNKNOWN")),
                "molecule": event.get("molecule", {}),
            }
        elif action == "destroyed":
            if molecule_id not in created:
                errors.append(f"record {index}: destruction before creation for {molecule_id}")
            elif molecule_id in destroyed:
                errors.append(f"record {index}: duplicate destruction for {molecule_id}")
            elif tick < created[molecule_id]["tick"]:
                errors.append(f"record {index}: destruction precedes creation for {molecule_id}")
            else:
                destroyed[molecule_id] = tick
        else:
            errors.append(f"record {index}: unsupported action {action!r}")

    family_members: dict[str, list[int]] = defaultdict(list)
    molecule_rows: dict[int, dict[str, Any]] = {}
    for molecule_id, data in created.items():
        signature = _molecule_signature(data["molecule"])
        composition = _composition(data["molecule"])
        end_tick = destroyed.get(molecule_id, last_tick)
        molecule_snapshot = data["molecule"] if isinstance(data["molecule"], dict) else {}
        material_snapshot = molecule_snapshot.get("material")
        material_snapshot = material_snapshot if isinstance(material_snapshot, dict) else {}
        connections = molecule_snapshot.get("internal_connections")
        internal_bonds = material_snapshot.get("internal_bonds")
        is_compound = (
            len(composition) > 1
            or (isinstance(connections, list) and bool(connections))
            or (isinstance(internal_bonds, list) and bool(internal_bonds))
        )
        row = {
            "molecule_id": molecule_id,
            "family_signature": signature,
            "is_compound": is_compound,
            "composition": composition,
            "created_tick": data["tick"],
            "destroyed_tick": destroyed.get(molecule_id),
            "observed_lifetime_ticks": max(0, end_tick - data["tick"]),
            "right_censored": molecule_id not in destroyed,
            "reaction": data["reaction"],
            "parent_ids": data["parents"],
        }
        molecule_rows[molecule_id] = row
        if is_compound:
            family_members[signature].append(molecule_id)

    family_by_id = {
        molecule_id: row["family_signature"] for molecule_id, row in molecule_rows.items()
    }
    families = []
    for signature, ids in family_members.items():
        rows = [molecule_rows[molecule_id] for molecule_id in ids]
        lifetimes = [row["observed_lifetime_ticks"] for row in rows]
        active_ids = [molecule_id for molecule_id in ids if molecule_id not in destroyed]
        composition = rows[0]["composition"]
        families.append({
            "signature": signature,
            "occurrences": len(ids),
            "active_at_end": len(active_ids),
            "molecule_ids": sorted(ids),
            "composition": composition,
            "mean_observed_lifetime_ticks": statistics.fmean(lifetimes) if lifetimes else None,
            "median_observed_lifetime_ticks": statistics.median(lifetimes) if lifetimes else None,
            "completed_lifetimes": sum(not row["right_censored"] for row in rows),
            "right_censored_lifetimes": sum(row["right_censored"] for row in rows),
            "recurrent": len(ids) >= 2,
        })
    families.sort(key=lambda row: (-row["occurrences"], -row["active_at_end"], row["signature"]))

    reaction_counts = Counter(row["reaction"] for row in molecule_rows.values())
    family_pathways: Counter[tuple[str, str, str]] = Counter()
    for molecule_id, data in created.items():
        child_family = family_by_id.get(molecule_id)
        for parent_id in data["parents"]:
            parent_family = family_by_id.get(parent_id)
            if parent_family is not None and child_family is not None:
                family_pathways[(data["reaction"], parent_family, child_family)] += 1

    resource_units: Counter[str] = Counter()
    molecules_by_resource: Counter[str] = Counter()
    compound_ids = [
        molecule_id for molecule_id, row in molecule_rows.items() if row["is_compound"]
    ]
        for resource, amount in composition.items():
            resource_units[resource] += amount
            molecules_by_resource[resource] += 1

    recurrent = [family for family in families if family["recurrent"]]
    active_families = [family for family in families if family["active_at_end"] > 0]
    return {
        "schema_version": 1,
        "scope": {
            "molecule_creation_records": len(created),
            "molecule_destruction_records": len(destroyed),
            "active_molecules_at_end": sum(row["right_censored"] for row in molecule_rows.values()),
            "observation_end_tick": last_tick,
            "lifecycle_validation_errors": errors,
        },
        "organization": {
            "unique_exact_structural_families": len(families),
            "recurrent_families": len(recurrent),
            "single_resource_unbonded_identities_excluded": len(molecule_rows) - len(compound_ids),
            "families_active_at_end": len(active_families),
            "molecules_with_multi_resource_or_bonded_structure": len(compound_ids),
            "recurrence_fraction": len(recurrent) / len(families) if families else 0.0,
            "top_families": families[:25],
            "families": families,
        },
        "persistence": {
            "completed_lifetime_count": sum(not row["right_censored"] for row in molecule_rows.values()),
            "right_censored_count": sum(row["right_censored"] for row in molecule_rows.values()),
            "mean_observed_lifetime_ticks": (
                statistics.fmean(row["observed_lifetime_ticks"] for row in molecule_rows.values())
                if molecule_rows else None
            ),
            "median_observed_lifetime_ticks": (
                statistics.median(row["observed_lifetime_ticks"] for row in molecule_rows.values())
                if molecule_rows else None
            ),
        },
        "material_distribution": {
            "constituent_amount_by_resource": dict(sorted(resource_units.items())),
            "molecules_containing_resource": dict(sorted(molecules_by_resource.items())),
            "note": "Amounts are summed once per created molecule identity; lineage generations can therefore count transferred material more than once.",
        },
        "reaction_network": {
            "molecule_creations_by_reaction": dict(sorted(reaction_counts.items())),
            "family_to_family_transitions": [
                {"reaction": reaction, "parent_family": parent, "child_family": child, "count": count}
                for (reaction, parent, child), count in sorted(
                    family_pathways.items(), key=lambda item: (-item[1], item[0])
                )
            ],
        },
        "interpretation_limits": [
            "Structural-family recurrence excludes single-resource unbonded identities so abundant independent atoms/materials do not masquerade as organized compounds.",
            "A structural family is an exact normalized serialized composition and connection signature, not graph-isomorphism or proof of chemical equivalence.",
            "Active molecule lifetimes are right-censored at the final observed tick.",
            "Material totals count each created identity once, so lineage generations may count the same conserved constituents more than once.",
            "Recurrence and persistence are descriptive evidence, not proof of self-maintaining chemical organization.",
        ],
    }


def render_markdown(report: dict[str, Any]) -> str:
    org = report["organization"]
    persistence = report["persistence"]
    lines = [
        "# EvoSim chemical organization analysis", "",
        f"- Molecule identities created: {report['scope']['molecule_creation_records']}",
        f"- Molecules active at end: {report['scope']['active_molecules_at_end']}",
        f"- Exact structural families: {org['unique_exact_structural_families']}",
        f"- Recurrent families (2+ identities): {org['recurrent_families']}",
        f"- Families still active at end: {org['families_active_at_end']}",
        f"- Multi-resource or bonded identities: {org['molecules_with_multi_resource_or_bonded_structure']}",
        f"- Mean observed lifetime: {persistence['mean_observed_lifetime_ticks']} ticks",
        f"- Median observed lifetime: {persistence['median_observed_lifetime_ticks']} ticks",
        f"- Lifecycle validation errors: {len(report['scope']['lifecycle_validation_errors'])}",
        "", "## Most recurrent structural families", "",
        "| Occurrences | Active | Mean observed lifetime (ticks) | Composition |",
        "|---:|---:|---:|---|",
    ]
    for family in org["top_families"][:15]:
        composition = ", ".join(
            f"{name}:{amount:g}" for name, amount in sorted(family["composition"].items())
        )
        lines.append(
            f"| {family['occurrences']} | {family['active_at_end']} | "
            f"{family['mean_observed_lifetime_ticks']:.2f} | {composition or '(empty)'} |"
        )
    lines += ["", "## Molecule creations by reaction", "",
              "| Reaction | Created identities |", "|---|---:|"]
    for reaction, count in report["reaction_network"]["molecule_creations_by_reaction"].items():
        lines.append(f"| {reaction} | {count} |")
    lines += ["", "## Interpretation limits", ""]
    lines.extend(f"- {item}" for item in report["interpretation_limits"])
    lines.append("")
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
        print(f"chemistry organization analyzer: {exc}", file=sys.stderr)
        return 2
    if args.json_out:
        args.json_out.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    if args.markdown_out:
        args.markdown_out.write_text(render_markdown(report), encoding="utf-8")
    if not args.json_out and not args.markdown_out:
        print(render_markdown(report), end="")
    if report["scope"]["lifecycle_validation_errors"]:
        print("chemistry organization analyzer: lifecycle validation failed", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
