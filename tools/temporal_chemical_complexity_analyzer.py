#!/usr/bin/env python3
"""Time-resolved chemical complexity and reaction-network analysis.

This is an offline, observational analyzer. Molecular family fingerprints use
iterative Weisfeiler-Lehman graph refinement. They are scalable descriptors,
not proofs of graph isomorphism; reports label them accordingly.
"""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
import statistics
import sys
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any


def _stable(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True)


def molecule_graph(molecule: Any) -> tuple[list[str], set[tuple[int, int]]]:
    if not isinstance(molecule, dict):
        molecule = {}
    material = molecule.get("material")
    if not isinstance(material, dict):
        material = {}
    raw_parts = material.get("parts", [])
    labels: list[str] = []
    if isinstance(raw_parts, list):
        for part in raw_parts:
            if isinstance(part, (list, tuple)) and len(part) == 2:
                try:
                    amount = float(part[1])
                except (TypeError, ValueError):
                    continue
                labels.append(f"{part[0]}:{amount:g}")
    raw_bonds = molecule.get("internal_connections") or material.get("internal_bonds", [])
    edges: set[tuple[int, int]] = set()
    if isinstance(raw_bonds, list):
        for bond in raw_bonds:
            if not isinstance(bond, dict):
                continue
            a, b = bond.get("part_a"), bond.get("part_b")
            if (isinstance(a, int) and not isinstance(a, bool)
                    and isinstance(b, int) and not isinstance(b, bool)
                    and 0 <= a < len(labels) and 0 <= b < len(labels) and a != b):
                edges.add(tuple(sorted((a, b))))
    return labels, edges


def graph_metrics(molecule: Any) -> dict[str, Any]:
    labels, edges = molecule_graph(molecule)
    n = len(labels)
    adjacency = [set() for _ in labels]
    for a, b in edges:
        adjacency[a].add(b)
        adjacency[b].add(a)
    degrees = [len(neighbors) for neighbors in adjacency]
    seen: set[int] = set()
    components = 0
    for root in range(n):
        if root in seen:
            continue
        components += 1
        stack = [root]
        seen.add(root)
        while stack:
            current = stack.pop()
            for neighbor in adjacency[current]:
                if neighbor not in seen:
                    seen.add(neighbor)
                    stack.append(neighbor)
    diameter = 0
    if n and components == 1:
        for root in range(n):
            distances = {root: 0}
            queue = [root]
            for current in queue:
                for neighbor in adjacency[current]:
                    if neighbor not in distances:
                        distances[neighbor] = distances[current] + 1
                        queue.append(neighbor)
            diameter = max(diameter, max(distances.values(), default=0))
    return {
        "constituents": n,
        "bonds": len(edges),
        "max_degree": max(degrees, default=0),
        "branch_points": sum(degree >= 3 for degree in degrees),
        "connected_components": components,
        "graph_diameter": diameter if n and components == 1 else None,
        "independent_cycles": max(0, len(edges) - n + components),
    }


def structural_fingerprint(molecule: Any, rounds: int | None = None) -> str:
    labels, edges = molecule_graph(molecule)
    n = len(labels)
    adjacency = [set() for _ in labels]
    for a, b in edges:
        adjacency[a].add(b)
        adjacency[b].add(a)
    colors = [hashlib.sha256(_stable(label).encode()).hexdigest()[:16] for label in labels]
    iterations = rounds if rounds is not None else max(1, min(n, 32))
    for _ in range(iterations):
        updated = [
            hashlib.sha256(_stable((colors[i], sorted(colors[j] for j in adjacency[i]))).encode()).hexdigest()[:20]
            for i in range(n)
        ]
        if updated == colors:
            break
        colors = updated
    # Sorted refined node colors and degree/edge summary are invariant to part ordering.
    descriptor = {
        "nodes": sorted(colors),
        "degrees": sorted(len(row) for row in adjacency),
        "edges": len(edges),
        "components": graph_metrics(molecule)["connected_components"],
        "composition": sorted(labels),
    }
    return hashlib.sha256(_stable(descriptor).encode()).hexdigest()


def strongly_connected_components(nodes: set[str], edges: set[tuple[str, str]]) -> list[list[str]]:
    adjacency: dict[str, list[str]] = {node: [] for node in nodes}
    for source, target in edges:
        adjacency.setdefault(source, []).append(target)
    index = 0
    stack: list[str] = []
    on_stack: set[str] = set()
    indices: dict[str, int] = {}
    low: dict[str, int] = {}
    result: list[list[str]] = []

    def visit(node: str) -> None:
        nonlocal index
        indices[node] = low[node] = index
        index += 1
        stack.append(node)
        on_stack.add(node)
        for neighbor in adjacency.get(node, []):
            if neighbor not in indices:
                visit(neighbor)
                low[node] = min(low[node], low[neighbor])
            elif neighbor in on_stack:
                low[node] = min(low[node], indices[neighbor])
        if low[node] == indices[node]:
            component = []
            while True:
                member = stack.pop()
                on_stack.remove(member)
                component.append(member)
                if member == node:
                    break
            result.append(sorted(component))

    for node in sorted(nodes):
        if node not in indices:
            visit(node)
    return result


def _mean(values: list[float]) -> float | None:
    return statistics.fmean(values) if values else None


def analyze_seed(path: Path, seed: int, duration: int, windows: int) -> dict[str, Any]:
    data = json.loads(path.read_text(encoding="utf-8"))
    source = data.get("input", {})
    history = data.get("molecular_history", {})
    if source.get("seed") != seed:
        raise ValueError(f"seed {seed}: chemistry report identifies seed {source.get('seed')}")
    if source.get("requested_duration_seconds") != duration:
        raise ValueError(f"seed {seed}: requested duration does not match {duration}")
    if history.get("validation_errors"):
        raise ValueError(f"seed {seed}: lifecycle validation errors present")
    molecules = history.get("molecules", [])
    if not molecules:
        raise ValueError(f"seed {seed}: no molecular history")
    end_tick = max((int(row.get("created_tick", 0)) + int(row.get("observed_lifetime_ticks", 0))
                    for row in molecules), default=0)
    if end_tick <= 0:
        raise ValueError(f"seed {seed}: invalid observation tick range")

    rows: list[dict[str, Any]] = []
    by_id: dict[int, dict[str, Any]] = {}
    for row in molecules:
        molecule_id = int(row["molecule_id"])
        created = int(row.get("created_tick", 0))
        destroyed = row.get("destroyed_tick")
        destroyed = int(destroyed) if destroyed is not None else None
        molecule = row.get("molecule", {})
        metrics = graph_metrics(molecule)
        fingerprint = structural_fingerprint(molecule)
        item = {
            "molecule_id": molecule_id, "created_tick": created,
            "destroyed_tick": destroyed, "reaction": str(row.get("reaction", "UNKNOWN")),
            "parents": [int(value) for value in row.get("parent_ids", [])],
            "fingerprint": fingerprint, "metrics": metrics,
            "is_product": str(row.get("reaction", "UNKNOWN")) != "INITIAL",
        }
        rows.append(item)
        by_id[molecule_id] = item

    rows.sort(key=lambda item: (item["created_tick"], item["molecule_id"]))
    tick_span = end_tick
    boundaries = [round(tick_span * index / windows) for index in range(windows + 1)]
    family_first_seen: dict[str, int] = {}
    reaction_first_seen: dict[str, int] = {}
    transition_edges: set[tuple[str, str]] = set()
    transition_nodes: set[str] = set()
    transition_events: list[tuple[int, str, str, str]] = []
    events: dict[str, dict[str, Any]] = {}
    for edge in history.get("lineage_edges", []):
        reaction_id = str(edge.get("reaction_id") or f"missing:{edge.get('tick')}:{edge.get('parent_id')}:{edge.get('child_id')}")
        event = events.setdefault(reaction_id, {
            "tick": int(edge.get("tick", 0)), "reaction": str(edge.get("reaction", "UNKNOWN")),
            "parents": set(), "children": set(),
        })
        event["parents"].add(int(edge["parent_id"]))
        event["children"].add(int(edge["child_id"]))
    for event in events.values():
        parent_families = sorted({by_id[i]["fingerprint"] for i in event["parents"] if i in by_id})
        child_families = sorted({by_id[i]["fingerprint"] for i in event["children"] if i in by_id})
        for parent in parent_families:
            transition_nodes.add(parent)
        for child in child_families:
            transition_nodes.add(child)
        for parent in parent_families:
            for child in child_families:
                transition_edges.add((parent, child))
                transition_events.append((event["tick"], parent, child, event["reaction"]))
        signature = _stable((event["reaction"], parent_families, child_families))
        event["signature"] = hashlib.sha256(signature.encode()).hexdigest()

    temporal_rows = []
    for index in range(windows):
        start, finish = boundaries[index], boundaries[index + 1]
        created_here = [row for row in rows if start < row["created_tick"] <= finish or (index == 0 and row["created_tick"] == 0)]
        active = [row for row in rows if row["created_tick"] <= finish and
                  (row["destroyed_tick"] is None or row["destroyed_tick"] > finish)]
        new_families = 0
        repeated_products = 0
        product_rows = [row for row in created_here if row["is_product"]]
        for row in created_here:
            fingerprint = row["fingerprint"]
            if fingerprint not in family_first_seen:
                family_first_seen[fingerprint] = row["created_tick"]
                new_families += 1
            elif row["is_product"]:
                repeated_products += 1
        for event in events.values():
            if start < event["tick"] <= finish:
                signature = event["signature"]
                reaction_first_seen.setdefault(signature, event["tick"])
        active_products = [row for row in active if row["is_product"]]
        active_metrics = [row["metrics"] for row in active_products]
        created_metrics = [row["metrics"] for row in product_rows]
        edges_so_far = {(a, b) for tick, a, b, _ in transition_events if tick <= finish}
        nodes_so_far = {node for edge in edges_so_far for node in edge}
        components = strongly_connected_components(nodes_so_far, edges_so_far) if nodes_so_far else []
        cyclic_components = [component for component in components
                             if len(component) > 1 or (component and (component[0], component[0]) in edges_so_far)]
        event_signatures_so_far = {
            event["signature"] for event in events.values() if event["tick"] <= finish
        }
        temporal_rows.append({
            "window": index + 1, "start_tick_exclusive": start,
            "end_tick_inclusive": finish,
            "created_molecule_identities": len(created_here),
            "created_reaction_products": len(product_rows),
            "active_molecule_identities": len(active),
            "active_reaction_products": len(active_products),
            "new_structural_fingerprints": new_families,
            "cumulative_structural_fingerprints": len(family_first_seen),
            "repeated_product_identities": repeated_products,
            "cumulative_reaction_patterns": len(event_signatures_so_far),
            "new_reaction_patterns_cumulative": len(reaction_first_seen),
            "active_product_mean_constituents": _mean([m["constituents"] for m in active_metrics]),
            "active_product_max_constituents": max((m["constituents"] for m in active_metrics), default=0),
            "active_product_mean_bonds": _mean([m["bonds"] for m in active_metrics]),
            "active_product_max_bonds": max((m["bonds"] for m in active_metrics), default=0),
            "active_product_max_degree": max((m["max_degree"] for m in active_metrics), default=0),
            "active_product_branch_points_total": sum(m["branch_points"] for m in active_metrics),
            "active_product_max_graph_diameter": max((m["graph_diameter"] or 0 for m in active_metrics), default=0),
            "created_product_mean_constituents": _mean([m["constituents"] for m in created_metrics]),
            "created_product_max_constituents": max((m["constituents"] for m in created_metrics), default=0),
            "reaction_network_nodes_cumulative": len(nodes_so_far),
            "reaction_network_edges_cumulative": len(edges_so_far),
            "reaction_network_cyclic_scc_count": len(cyclic_components),
            "largest_cyclic_scc_nodes": max((len(c) for c in cyclic_components), default=0),
        })

    # Family and reaction fingerprint summaries, with explicit approximation caveat.
    family_counts = Counter(row["fingerprint"] for row in rows if row["is_product"])
    reaction_counts = Counter(event["signature"] for event in events.values())
    max_product_size = max((row["metrics"]["constituents"] for row in rows if row["is_product"]), default=0)
    return {
        "seed": seed, "requested_duration_seconds": duration,
        "observation_end_tick": end_tick, "window_count": windows,
        "molecule_identity_count": len(rows),
        "reaction_created_product_count": sum(row["is_product"] for row in rows),
        "unique_product_structural_fingerprints": len(family_counts),
        "recurrent_product_fingerprints": sum(count > 1 for count in family_counts.values()),
        "max_product_constituents": max_product_size,
        "distinct_reaction_pattern_fingerprints": len(reaction_counts),
        "recurrent_reaction_pattern_fingerprints": sum(count > 1 for count in reaction_counts.values()),
        "lineage_reaction_events_reconstructed": len(events),
        "lineage_events_missing_reaction_id": sum(key.startswith("missing:") for key in events),
        "temporal_windows": temporal_rows,
        "interpretation_limits": [
            "Structural fingerprints use iterative Weisfeiler-Lehman refinement plus composition and degree summaries; they are scalable descriptors, not exact graph-isomorphism proofs.",
            "A reaction pattern is reconstructed from lineage parent and child family fingerprints and reaction label; it is not a balanced thermodynamic equation.",
            "A directed cycle in the family-transition graph is a candidate network cycle, not proof of autocatalysis or net self-sustainability.",
            "Only recorded molecular identities and lineage are analyzed; unrecorded transient contacts are not inferred.",
            "Active product measurements are snapshots at window boundaries; molecules destroyed and recreated between boundaries are represented through lifecycle events.",
        ],
    }


def build_report(root: Path, seeds: list[int], duration: int, windows: int) -> dict[str, Any]:
    per_seed = [analyze_seed(root / f"seed-{seed}-chemistry.json", seed, duration, windows)
                for seed in seeds]
    pooled_windows = []
    for index in range(windows):
        rows = [run["temporal_windows"][index] for run in per_seed]
        numeric = [key for key, value in rows[0].items()
                   if key not in ("window", "start_tick_exclusive", "end_tick_inclusive")
                   and isinstance(value, (int, float))]
        pooled = {"window": index + 1}
        for key in numeric:
            values = [row[key] for row in rows]
            pooled[f"{key}_mean_across_seeds"] = statistics.fmean(values)
            pooled[f"{key}_min_across_seeds"] = min(values)
            pooled[f"{key}_max_across_seeds"] = max(values)
        pooled_windows.append(pooled)
    return {
        "schema_version": 1,
        "experiment": {"seeds": seeds, "duration_seconds_per_seed": duration,
                       "windows": windows, "window_duration_seconds": duration / windows,
                       "total_simulated_seconds": duration * len(seeds)},
        "per_seed": per_seed, "pooled_temporal_windows": pooled_windows,
        "interpretation": [
            "Compare novelty, active molecular size, reaction-pattern recurrence, and network cycles across successive windows.",
            "Increasing molecule identities alone is not evidence of increasing chemical complexity.",
            "A stable fingerprint count with high repeated-product counts suggests recurrence or stagnation; it does not by itself identify the cause.",
            "Network cycles are candidates for follow-up analysis, not proof of autocatalysis.",
        ],
    }


def write_outputs(report: dict[str, Any], output_dir: Path) -> None:
    output_dir.mkdir(parents=True, exist_ok=True)
    (output_dir / "temporal-chemical-complexity.json").write_text(
        json.dumps(report, indent=2) + "\n", encoding="utf-8")
    rows = report["pooled_temporal_windows"]
    if rows:
        with (output_dir / "temporal-chemical-complexity.csv").open("w", newline="", encoding="utf-8") as stream:
            writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
            writer.writeheader()
            writer.writerows(rows)
    lines = [
        "# Time-resolved chemical complexity",
        "",
        f"- Seeds: {', '.join(map(str, report['experiment']['seeds']))}",
        f"- Duration per seed: {report['experiment']['duration_seconds_per_seed']} seconds",
        f"- Windows: {report['experiment']['windows']} ({report['experiment']['window_duration_seconds']:.1f} seconds each)",
        f"- Total simulated time: {report['experiment']['total_simulated_seconds']} seconds",
        "",
        "## Temporal development (mean across seeds)",
        "",
        "| Window | New structural fingerprints | Cumulative fingerprints | Active product mean size | Active product max size | Cumulative reaction patterns | Network edges | Cyclic SCCs |",
        "|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for row in rows:
        lines.append(
            f"| {row['window']} | {row['new_structural_fingerprints_mean_across_seeds']:.2f} | "
            f"{row['cumulative_structural_fingerprints_mean_across_seeds']:.2f} | "
            f"{row['active_product_mean_constituents_mean_across_seeds']:.2f} | "
            f"{row['active_product_max_constituents_mean_across_seeds']:.2f} | "
            f"{row['cumulative_reaction_patterns_mean_across_seeds']:.2f} | "
            f"{row['reaction_network_edges_cumulative_mean_across_seeds']:.2f} | "
            f"{row['reaction_network_cyclic_scc_count_mean_across_seeds']:.2f} |"
        )
    lines += ["", "## Per-seed summary", "",
              "| Seed | Products | Unique product fingerprints | Recurrent fingerprints | Max constituents | Reaction patterns | Cyclic SCCs at end |",
              "|---:|---:|---:|---:|---:|---:|---:|"]
    for run in report["per_seed"]:
        last = run["temporal_windows"][-1]
        lines.append(
            f"| {run['seed']} | {run['reaction_created_product_count']} | "
            f"{run['unique_product_structural_fingerprints']} | "
            f"{run['recurrent_product_fingerprints']} | {run['max_product_constituents']} | "
            f"{run['distinct_reaction_pattern_fingerprints']} | "
            f"{last['reaction_network_cyclic_scc_count']} |"
        )
    lines += ["", "## Interpretation limits", ""]
    lines.extend(f"- {item}" for item in report["interpretation"])
    lines += ["", "Per-seed structural and reaction fingerprint limitations:", ""]
    lines.extend(f"- {item}" for item in report["per_seed"][0]["interpretation_limits"])
    (output_dir / "temporal-chemical-complexity.md").write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input_dir", type=Path)
    parser.add_argument("--seeds", required=True)
    parser.add_argument("--duration-seconds", type=int, required=True)
    parser.add_argument("--windows", type=int, default=12)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    try:
        seeds = [int(value.strip()) for value in args.seeds.split(",") if value.strip()]
        if not seeds or len(seeds) != len(set(seeds)):
            raise ValueError("seeds must be a nonempty list of unique integers")
        if args.windows < 1 or args.duration_seconds < args.windows:
            raise ValueError("windows must be positive and duration must cover every window")
        report = build_report(args.input_dir, seeds, args.duration_seconds, args.windows)
        write_outputs(report, args.output_dir)
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as exc:
        print(f"temporal chemistry analyzer: {exc}", file=sys.stderr)
        return 2
    print((args.output_dir / "temporal-chemical-complexity.md").read_text(encoding="utf-8"))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
