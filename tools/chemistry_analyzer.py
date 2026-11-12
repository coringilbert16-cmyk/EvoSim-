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


def _structural_signature(molecule: Any) -> str:
    if not isinstance(molecule, dict):
        return "{}"
    material = molecule.get("material", {})
    if not isinstance(material, dict):
        material = {}
    parts = material.get("parts", [])
    normalized_parts = sorted(
        (str(part[0]), float(part[1]))
        for part in parts
        if isinstance(part, (list, tuple)) and len(part) == 2
    ) if isinstance(parts, list) else []
    raw_edges = molecule.get("internal_connections", [])
    edges = (
        [edge for edge in raw_edges if isinstance(edge, dict)]
        if isinstance(raw_edges, list)
        else []
    )
    normalized_edges = sorted(
        json.dumps(edge, sort_keys=True, separators=(",", ":")) for edge in edges
    )
    return json.dumps(
        {"composition": normalized_parts, "bonds": normalized_edges},
        sort_keys=True,
        separators=(",", ":"),
    )



ACTIVE_ELEMENTS = ("Carbon", "Hydrogen", "Nitrogen", "Oxygen", "Phosphorus", "Sulfur")
ELEMENTAL_BOND_PAIRS = tuple(
    f"{left}-{right}"
    for index, left in enumerate(ACTIVE_ELEMENTS)
    for right in ACTIVE_ELEMENTS[index:]
)


def _has_complete_molecule_graph(molecule: Any) -> bool:
    if not isinstance(molecule, dict):
        return False
    material = molecule.get("material")
    return (
        isinstance(material, dict)
        and isinstance(material.get("parts"), list)
        and isinstance(molecule.get("internal_connections"), list)
    )


def _molecule_bond_pairs(molecule: Any) -> tuple[Counter[str], int]:
    """Count authoritative internal-connection edges by their endpoint elements."""
    if not isinstance(molecule, dict):
        return Counter(), 0
    material = molecule.get("material")
    if not isinstance(material, dict):
        return Counter(), 0
    parts = material.get("parts")
    connections = molecule.get("internal_connections")
    if not isinstance(parts, list) or not isinstance(connections, list):
        return Counter(), 0

    counts: Counter[str] = Counter()
    malformed = 0
    for bond in connections:
        if not isinstance(bond, dict):
            malformed += 1
            continue
        a = bond.get("part_a")
        b = bond.get("part_b")
        if (
            not isinstance(a, int) or isinstance(a, bool)
            or not isinstance(b, int) or isinstance(b, bool)
            or a < 0 or b < 0 or a >= len(parts) or b >= len(parts) or a == b
        ):
            malformed += 1
            continue
        part_a, part_b = parts[a], parts[b]
        if not (
            isinstance(part_a, (list, tuple)) and len(part_a) == 2
            and isinstance(part_b, (list, tuple)) and len(part_b) == 2
        ):
            malformed += 1
            continue
        element_a, element_b = str(part_a[0]), str(part_b[0])
        if element_a not in ACTIVE_ELEMENTS or element_b not in ACTIVE_ELEMENTS:
            continue
        left, right = sorted((element_a, element_b))
        counts[f"{left}-{right}"] += 1
    return counts, malformed


def _bond_pair_at_index(molecule: Any, bond_index: int) -> str | None:
    if not isinstance(molecule, dict):
        return None
    connections = molecule.get("internal_connections")
    if not isinstance(connections, list) or not 0 <= bond_index < len(connections):
        return None
    single_bond_molecule = dict(molecule)
    single_bond_molecule["internal_connections"] = [connections[bond_index]]
    counts, malformed = _molecule_bond_pairs(single_bond_molecule)
    if malformed or len(counts) != 1:
        return None
    return next(iter(counts))


def _bond_pair_attribution(
    reactions: list[dict[str, Any]],
    lifecycle_records: list[dict[str, Any]],
    expected_initial_material_count: Any,
    expected_final_material_count: Any,
) -> dict[str, Any]:
    """Attribute committed reaction bond changes from full reactant/product graphs."""
    formed_by_reaction: dict[str, Counter[str]] = {}
    broken_by_reaction: dict[str, Counter[str]] = {}
    formed_total: Counter[str] = Counter()
    broken_total: Counter[str] = Counter()
    event_attribution: list[dict[str, Any]] = []
    events_missing_reaction_id = 0
    observed = missing = malformed = 0

    def complete(counter: Counter[str]) -> dict[str, int]:
        return {pair: int(counter.get(pair, 0)) for pair in ELEMENTAL_BOND_PAIRS}

    for event in reactions:
        if event.get("outcome") != "committed":
            continue
        lifecycle = event.get("molecular_lifecycle")
        if not isinstance(lifecycle, list) or not lifecycle:
            missing += 1
            continue
        reactants: Counter[str] = Counter()
        products: Counter[str] = Counter()
        reactant_molecules: list[Any] = []
        has_reactant = has_product = graph_complete = False
        graph_complete = True
        for record in lifecycle:
            if not isinstance(record, dict):
                malformed += 1
                continue
            molecule = record.get("molecule")
            graph_complete &= _has_complete_molecule_graph(molecule)
            counts, invalid = _molecule_bond_pairs(molecule)
            malformed += invalid
            if record.get("action") == "destroyed":
                reactants.update(counts)
                reactant_molecules.append(molecule)
                has_reactant |= isinstance(molecule, dict)
            elif record.get("action") == "created":
                products.update(counts)
                has_product |= isinstance(molecule, dict)
        if not (has_reactant and has_product and graph_complete):
            missing += 1
            continue
        reaction = str(event.get("reaction", "UNKNOWN"))
        remaining_reactants = reactants.copy()
        explicitly_broken: Counter[str] = Counter()
        broken_index = event.get("broken_bond_index")
        if reaction in ("BREAK", "DISPLACEMENT"):
            if (
                not isinstance(broken_index, int)
                or isinstance(broken_index, bool)
                or not reactant_molecules
            ):
                missing += 1
                continue
            selected_pair = _bond_pair_at_index(reactant_molecules[0], broken_index)
            if not selected_pair or remaining_reactants[selected_pair] <= 0:
                malformed += 1
                missing += 1
                continue
            remaining_reactants[selected_pair] -= 1
            if remaining_reactants[selected_pair] == 0:
                del remaining_reactants[selected_pair]
            explicitly_broken[selected_pair] += 1
        formed = products - remaining_reactants
        broken = (remaining_reactants - products) + explicitly_broken
        formed_by_reaction.setdefault(reaction, Counter()).update(formed)
        broken_by_reaction.setdefault(reaction, Counter()).update(broken)
        formed_total.update(formed)
        broken_total.update(broken)
        reaction_ids = sorted({
            str(record.get("reaction_id"))
            for record in lifecycle
            if record.get("reaction_id") is not None
        })
        if len(reaction_ids) != 1:
            events_missing_reaction_id += 1
        event_attribution.append({
            "reaction_id": reaction_ids[0] if len(reaction_ids) == 1 else None,
            "tick": int(event.get("tick", 0) or 0),
            "reaction": reaction,
            "parent_ids": sorted(
                int(record["molecule_id"])
                for record in lifecycle
                if record.get("action") == "destroyed"
                and isinstance(record.get("molecule_id"), int)
                and not isinstance(record.get("molecule_id"), bool)
            ),
            "child_ids": sorted(
                int(record["molecule_id"])
                for record in lifecycle
                if record.get("action") == "created"
                and isinstance(record.get("molecule_id"), int)
                and not isinstance(record.get("molecule_id"), bool)
            ),
            "formed": complete(formed),
            "broken": complete(broken),
            "formed_total": sum(formed.values()),
            "broken_total": sum(broken.values()),
        })
        observed += 1

    initial_inventory: Counter[str] = Counter()
    active_molecules: dict[int, Any] = {}
    inventory_malformed = 0
    initial_lifecycle_material_count = 0
    missing_inventory_graph_snapshots = 0
    for record in lifecycle_records:
        molecule_id = record.get("molecule_id")
        action = record.get("action")
        if not isinstance(molecule_id, int) or isinstance(molecule_id, bool):
            continue
        if action == "created":
            molecule = record.get("molecule")
            active_molecules[molecule_id] = molecule
            if record.get("reaction") == "INITIAL":
                initial_lifecycle_material_count += 1
                if not _has_complete_molecule_graph(molecule):
                    missing_inventory_graph_snapshots += 1
                counts, invalid = _molecule_bond_pairs(molecule)
                initial_inventory.update(counts)
                inventory_malformed += invalid
        elif action == "destroyed":
            active_molecules.pop(molecule_id, None)

    final_inventory: Counter[str] = Counter()
    for molecule in active_molecules.values():
        if not _has_complete_molecule_graph(molecule):
            missing_inventory_graph_snapshots += 1
        counts, invalid = _molecule_bond_pairs(molecule)
        final_inventory.update(counts)
        inventory_malformed += invalid
    inventory_delta = {
        pair: final_inventory.get(pair, 0) - initial_inventory.get(pair, 0)
        for pair in ELEMENTAL_BOND_PAIRS
    }
    reaction_delta = {
        pair: formed_total.get(pair, 0) - broken_total.get(pair, 0)
        for pair in ELEMENTAL_BOND_PAIRS
    }
    inventory_mismatches = {
        pair: {
            "inventory_delta": inventory_delta[pair],
            "reaction_delta": reaction_delta[pair],
        }
        for pair in ELEMENTAL_BOND_PAIRS
        if inventory_delta[pair] != reaction_delta[pair]
    }

    by_reaction = {}
    for reaction in sorted(set(formed_by_reaction) | set(broken_by_reaction)):
        formed = formed_by_reaction.get(reaction, Counter())
        broken = broken_by_reaction.get(reaction, Counter())
        by_reaction[reaction] = {
            "formed": complete(formed),
            "broken": complete(broken),
            "formed_total": sum(formed.values()),
            "broken_total": sum(broken.values()),
        }
    return {
        "element_pairs": list(ELEMENTAL_BOND_PAIRS),
        "committed_events_with_graph_deltas": observed,
        "committed_events_missing_graph_deltas": missing,
        "malformed_bond_records": malformed,
        "formed_total": sum(formed_total.values()),
        "broken_total": sum(broken_total.values()),
        "formed": complete(formed_total),
        "broken": complete(broken_total),
        "initial_bond_inventory": complete(initial_inventory),
        "final_bond_inventory": complete(final_inventory),
        "inventory_delta": inventory_delta,
        "reaction_delta": reaction_delta,
        "inventory_reconciliation": {
            "matches": (
                not inventory_mismatches
                and missing == 0
                and inventory_malformed == 0
                and missing_inventory_graph_snapshots == 0
                and initial_lifecycle_material_count == expected_initial_material_count
                and len(active_molecules) == expected_final_material_count
            ),
            "mismatches": inventory_mismatches,
            "initial_lifecycle_material_count": initial_lifecycle_material_count,
            "expected_initial_material_count": expected_initial_material_count,
            "final_lifecycle_material_count": len(active_molecules),
            "expected_final_material_count": expected_final_material_count,
            "missing_inventory_graph_snapshots": missing_inventory_graph_snapshots,
            "malformed_inventory_bond_records": inventory_malformed,
        },
        "by_reaction": by_reaction,
        "event_attribution": event_attribution,
        "events_missing_reaction_id": events_missing_reaction_id,
        "method": (
            "For each committed reaction, compare the multiset of endpoint-element "
            "pairs in destroyed reactant PhysicalMaterial.internal_connections "
            "against created product connections. Product-minus-reactant is formed; "
            "reactant-minus-product is broken. The resulting pair deltas are reconciled against the initial and final live-molecule inventories."
        ),
    }


def analyze(records: list[dict[str, Any]]) -> dict[str, Any]:
    headers = [r for r in records if r.get("event") == "environmental_chemistry_header"]
    snapshots = [r for r in records if r.get("event") == "environmental_chemistry_snapshot"]
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
    excluded_resources = header.get("excluded_resources", [])
    if not isinstance(excluded_resources, list) or any(
        not isinstance(name, str) or name not in CANONICAL_RESOURCES
        for name in excluded_resources
    ):
        raise ValueError("header excluded_resources must contain canonical resource names")
    required_resources = [
        name for name in CANONICAL_RESOURCES if name not in excluded_resources
    ]
    missing_resources = [
        name for name in required_resources
        if _number(initial_inventory.get(name)) is None
        or float(initial_inventory.get(name, 0)) <= 0.0
    ]

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
            "missing_resources": [
                name for name in required_resources
                if amounts[name] is None or float(amounts[name]) <= 0.0
            ],
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
    lifecycle = [r for r in records if r.get("event") == "molecular_lifecycle"]
    bond_pair_attribution = _bond_pair_attribution(
        reactions,
        lifecycle,
        header.get("initial_physical_material_count"),
        snapshots[-1].get("physical_material_count"),
    )


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

    def local_resonance_totals(path: str) -> dict[str, Any]:
        attempts = samples = unavailable = 0
        similarity_sum = 0.0
        similarity_max = 0.0
        for tick_record in ticks:
            work = tick_record.get("reaction_work") or {}
            section = work.get(path) or {}
            attempts += int(section.get("local_resonance_sample_attempts", 0) or 0)
            samples += int(section.get("local_resonance_samples", 0) or 0)
            unavailable += int(section.get("local_resonance_unavailable", 0) or 0)
            value_sum = _number(section.get("local_resonance_similarity_sum"))
            value_max = _number(section.get("local_resonance_similarity_max"))
            if value_sum is not None:
                similarity_sum += float(value_sum)
            if value_max is not None:
                similarity_max = max(similarity_max, float(value_max))
        mean_similarity = similarity_sum / samples if samples else None
        return {
            "sample_attempts": attempts,
            "successful_samples": samples,
            "unavailable_samples": unavailable,
            "similarity_sum": similarity_sum,
            "mean_similarity": mean_similarity,
            "maximum_similarity": similarity_max if samples else None,
            "mean_activation_multiplier": 1.0 + mean_similarity
                if mean_similarity is not None else None,
        }

    local_resonance = {
        "BREAK": local_resonance_totals("break_discovery"),
        "COMBINE": local_resonance_totals("combine_search"),
        "DISPLACEMENT": local_resonance_totals("displacement_discovery"),
    }
    for kind in ("BREAK", "COMBINE", "DISPLACEMENT"):
        reaction_totals.setdefault(kind, {})["local_resonance"] = local_resonance[kind]

    # Optional persistent molecule lifecycle records are emitted by Layer B.
    # Older Layer A logs remain valid and simply report no lifecycle coverage.
    created: dict[int, dict[str, Any]] = {}
    destroyed: dict[int, int] = {}
    lineage_edges: list[dict[str, Any]] = []
    lifecycle_errors: list[str] = []
    last_tick = max(
        (int(r.get("tick", 0)) for r in records
         if isinstance(r.get("tick"), (int, float))),
        default=0,
    )
    for index, event in enumerate(lifecycle):
        action = event.get("action")
        molecule_id = event.get("molecule_id")
        tick = event.get("tick")
        if not isinstance(molecule_id, int) or isinstance(molecule_id, bool) or molecule_id <= 0:
            lifecycle_errors.append(f"record {index}: molecule_id must be a positive integer")
            continue
        if not isinstance(tick, int) or isinstance(tick, bool) or tick < 0:
            lifecycle_errors.append(f"record {index}: tick must be a nonnegative integer")
            continue
        if action == "created":
            if molecule_id in created:
                lifecycle_errors.append(f"record {index}: duplicate creation for molecule {molecule_id}")
                continue
            parents = event.get("parent_ids", [])
            if not isinstance(parents, list) or any(
                not isinstance(parent, int) or isinstance(parent, bool) or parent <= 0
                for parent in parents
            ):
                lifecycle_errors.append(f"record {index}: parent_ids must contain positive integers")
                continue
            for parent in parents:
                if parent not in created:
                    lifecycle_errors.append(
                        f"record {index}: lineage parent {parent} has no prior creation"
                    )
                    continue
                if parent in destroyed and destroyed[parent] <= tick:
                    lifecycle_errors.append(
                        f"record {index}: lineage parent {parent} was already destroyed"
                    )
                    continue
                lineage_edges.append({
                    "parent_id": parent,
                    "child_id": molecule_id,
                    "reaction_id": event.get("reaction_id"),
                    "reaction": event.get("reaction"),
                    "tick": tick,
                })
            created[molecule_id] = {
                "created_tick": tick,
                "parent_ids": parents,
                "reaction_id": event.get("reaction_id"),
                "reaction": event.get("reaction"),
                "molecule": event.get("molecule", {}),
            }
        elif action == "destroyed":
            if molecule_id not in created:
                lifecycle_errors.append(f"record {index}: destruction before creation for molecule {molecule_id}")
                continue
            if molecule_id in destroyed:
                lifecycle_errors.append(f"record {index}: duplicate destruction for molecule {molecule_id}")
                continue
            if tick < created[molecule_id]["created_tick"]:
                lifecycle_errors.append(f"record {index}: destruction precedes creation for molecule {molecule_id}")
                continue
            destroyed[molecule_id] = tick
        else:
            lifecycle_errors.append(f"record {index}: unsupported lifecycle action {action!r}")

    molecule_lifetimes = []
    for molecule_id, data in sorted(created.items()):
        end_tick = destroyed.get(molecule_id, last_tick)
        molecule_lifetimes.append({
            "molecule_id": molecule_id,
            "created_tick": data["created_tick"],
            "destroyed_tick": destroyed.get(molecule_id),
            "observed_lifetime_ticks": end_tick - data["created_tick"],
            "right_censored": molecule_id not in destroyed,
            "parent_ids": data["parent_ids"],
            "reaction_id": data["reaction_id"],
            "reaction": data["reaction"],
            "molecule": data["molecule"],
        })
    lifetime_values = [row["observed_lifetime_ticks"] for row in molecule_lifetimes]
    structural_families: dict[str, list[int]] = {}
    for molecule in molecule_lifetimes:
        signature = _structural_signature(molecule["molecule"])
        molecule["structural_signature"] = signature
        structural_families.setdefault(signature, []).append(molecule["molecule_id"])
    reaction_pathways = Counter(
        str(edge["reaction"]) for edge in lineage_edges if edge["reaction"] is not None
    )

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
            "initial_constituent_count": header.get("initial_constituent_count"),
            "diagnostic_interval": header.get("diagnostic_interval"),
        },
        "resource_coverage": {
            "canonical_resources": list(CANONICAL_RESOURCES),
            "required_resources": required_resources,
            "excluded_resources": excluded_resources,
            "initial_inventory": {name: initial_inventory.get(name) for name in CANONICAL_RESOURCES},
            "missing_at_start": missing_resources,
            "all_required_present_at_start": not missing_resources,
            "all_seven_present_at_start": not missing_resources and not excluded_resources,
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
            "elemental_bond_pairs": bond_pair_attribution,
        },
        "molecular_history": {
            "lifecycle_record_count": len(lifecycle),
            "molecule_creation_count": len(created),
            "molecule_destruction_count": len(destroyed),
            "active_at_end_count": sum(molecule_id not in destroyed for molecule_id in created),
            "mean_observed_lifetime_ticks": (
                sum(lifetime_values) / len(lifetime_values) if lifetime_values else None
            ),
            "maximum_observed_lifetime_ticks": max(lifetime_values, default=None),
            "molecules": molecule_lifetimes,
            "structural_families": [
                {"signature": signature, "molecule_ids": ids, "count": len(ids)}
                for signature, ids in sorted(
                    structural_families.items(), key=lambda item: (-len(item[1]), item[0])
                )
            ],
            "lineage_edges": lineage_edges,
            "lineage_reaction_counts": dict(sorted(reaction_pathways.items())),
            "validation_errors": lifecycle_errors,
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
        "## Resource coverage",
        "",
        "| Resource | Initial amount | Role in this run |",
        "|---|---:|---|",
    ]
    for name in CANONICAL_RESOURCES:
        role = "excluded" if name in coverage["excluded_resources"] else "required"
        lines.append(
            f"| {name} | {coverage['initial_inventory'].get(name, 'missing')} | {role} |"
        )
    lines += [
        "",
        f"All required resources present initially: **{'yes' if coverage['all_required_present_at_start'] else 'NO'}**",
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
        "## Local resonance activation sampling",
        "",
        "| Reaction | Attempts | Successful | Unavailable | Mean similarity | Max similarity | Mean multiplier |",
        "|---|---:|---:|---:|---:|---:|---:|",
    ]
    for name in ("BREAK", "COMBINE", "DISPLACEMENT"):
        sample = report["reactions"]["diagnostic_totals"][name]["local_resonance"]
        lines.append(
            f"| {name} | {sample['sample_attempts']} | {sample['successful_samples']} | "
            f"{sample['unavailable_samples']} | {sample['mean_similarity']} | "
            f"{sample['maximum_similarity']} | {sample['mean_activation_multiplier']} |"
        )
    bond_pairs = report["reactions"]["elemental_bond_pairs"]
    lines += [
        "",
        "## Exact elemental bond-pair changes",
        "",
        f"- Committed reaction events with complete graph deltas: {bond_pairs['committed_events_with_graph_deltas']}",
        f"- Committed events missing graph deltas: {bond_pairs['committed_events_missing_graph_deltas']}",
        f"- Formed bonds: {bond_pairs['formed_total']}",
        f"- Broken bonds: {bond_pairs['broken_total']}",
        "",
        f"- Initial-to-final bond inventory reconciles with reaction deltas: {bond_pairs['inventory_reconciliation']['matches']}",
        "| Element pair | Initial | Formed | Broken | Final |",
        "|---|---:|---:|---:|---:|",
    ]
    for pair in bond_pairs["element_pairs"]:
        lines.append(
            f"| {pair} | {bond_pairs['initial_bond_inventory'][pair]} | "
            f"{bond_pairs['formed'][pair]} | {bond_pairs['broken'][pair]} | "
            f"{bond_pairs['final_bond_inventory'][pair]} |"
        )
    lines += [
        "",
        "## Energy accounting",
        "",
        f"- First snapshot residual: {report['energy']['first_balance_residual']}",
        f"- Final snapshot residual: {report['energy']['last_balance_residual']}",
        f"- Maximum absolute snapshot residual: {report['energy']['maximum_absolute_snapshot_residual']}",
        "",
        "## Molecular history",
        "",
        f"- Lifecycle records: {report['molecular_history']['lifecycle_record_count']}",
        f"- Molecules created: {report['molecular_history']['molecule_creation_count']}",
        f"- Molecules destroyed: {report['molecular_history']['molecule_destruction_count']}",
        f"- Molecules active at end: {report['molecular_history']['active_at_end_count']}",
        f"- Mean observed lifetime (ticks): {report['molecular_history']['mean_observed_lifetime_ticks']}",
        f"- Lineage edges: {len(report['molecular_history']['lineage_edges'])}",
        f"- Structural families: {len(report['molecular_history']['structural_families'])}",
        f"- Lifecycle validation errors: {len(report['molecular_history']['validation_errors'])}",
        "",
        "## Interpretation limits",
        "",
        "- Lifetime values for molecules still active at the final record are right-censored observations, not completed lifetimes.",
        "- Lineage is reported only when explicit molecular_lifecycle records are present; it is never inferred from cell indices or compositions.",
        "- Canonical resource history is retained; explicit experiment exclusions are honored when validating active inventory coverage.",
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
    if not report["resource_coverage"]["all_required_present_at_start"]:
        print("chemistry analyzer: initial inventory does not contain all required experiment resources",
              file=sys.stderr)
        return 1
    lifecycle_errors = report["molecular_history"]["validation_errors"]
    if lifecycle_errors:
        print(
            "chemistry analyzer: invalid molecular lifecycle: "
            + "; ".join(lifecycle_errors[:10]),
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
