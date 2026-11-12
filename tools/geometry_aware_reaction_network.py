#!/usr/bin/env python3
"""Build an instance-level, geometry-aware reaction network from chemistry artifacts."""
from __future__ import annotations

import argparse
import csv
import json
import math
import statistics
from pathlib import Path
from typing import Any

try:
    from .reaction_network_analyzer import family_key, family_label
except ImportError:  # Direct script execution from the tools directory.
    from reaction_network_analyzer import family_key, family_label


def _wrap_angle(value: float) -> float:
    return (value + math.pi) % (2.0 * math.pi) - math.pi


def _rounded(value: float) -> float:
    return round(float(value), 6)


def geometry_descriptor(
    molecule: dict[str, Any], world_height: float | None = 250.0
) -> dict[str, Any] | None:
    """Return translation/rotation-normalized physical geometry for one molecule."""
    material = molecule.get("material") or {}
    parts = material.get("parts") or []
    placements = molecule.get("placements")
    shapes = molecule.get("realized_shapes")
    connections = molecule.get("internal_connections")
    if not (
        isinstance(parts, list)
        and isinstance(placements, list)
        and isinstance(shapes, list)
        and isinstance(connections, list)
        and len(parts) == len(placements) == len(shapes)
        and parts
    ):
        return None

    try:
        points = [(float(p["x"]), float(p["y"])) for p in placements]
        rotations = [float(p["rotation_radians"]) for p in placements]
        if any(not math.isfinite(v) for point in points for v in point):
            return None
        if any(not math.isfinite(v) for v in rotations):
            return None
        if any(not isinstance(part, (list, tuple)) or len(part) != 2 for part in parts):
            return None
        if world_height is not None and (
            not math.isfinite(world_height) or world_height <= 0.0
        ):
            return None
    except (KeyError, TypeError, ValueError):
        return None

    # The simulation wraps vertically (world y), so unwrap every constituent
    # relative to the first before computing intrinsic geometry. Raw world-y
    # subtraction would turn a molecule crossing the seam into a ~world-height
    # molecule. Keep the wrapped centroid separately for world-space reference.
    anchor_y = points[0][1]
    if world_height is None:
        geometry_points = points
        center_y_unwrapped = statistics.fmean(p[1] for p in points)
        center_y_world = center_y_unwrapped
    else:
        geometry_points = [
            (
                x,
                anchor_y + ((y - anchor_y + world_height * 0.5) % world_height)
                - world_height * 0.5,
            )
            for x, y in points
        ]
        center_y_unwrapped = statistics.fmean(p[1] for p in geometry_points)
        center_y_world = center_y_unwrapped % world_height
    center_x = statistics.fmean(p[0] for p in geometry_points)
    frame_angle = rotations[0]
    for x, y in geometry_points[1:]:
        dx, dy = x - geometry_points[0][0], y - geometry_points[0][1]
        if math.hypot(dx, dy) > 1e-9:
            frame_angle = math.atan2(dy, dx)
            break

    cosine, sine = math.cos(frame_angle), math.sin(frame_angle)
    constituents = []
    for index, ((element, amount), point, rotation, shape) in enumerate(
        zip(parts, geometry_points, rotations, shapes)
    ):
        dx, dy = point[0] - center_x, point[1] - center_y
        local_x = dx * cosine + dy * sine
        local_y = -dx * sine + dy * cosine
        constituents.append({
            "part_index": index,
            "element": str(element),
            "amount": float(amount),
            "x": _rounded(local_x),
            "y": _rounded(local_y),
            "rotation_radians": _rounded(_wrap_angle(rotation - frame_angle)),
            "shape": shape,
        })

    pair_distances = []
    for left in range(len(points)):
        for right in range(left + 1, len(points)):
            pair_distances.append({
                "part_a": left,
                "part_b": right,
                "distance": _rounded(math.dist(geometry_points[left], geometry_points[right])),
            })

    bonds = []
    for connection in connections:
        try:
            a, b = int(connection["part_a"]), int(connection["part_b"])
            if a < 0 or b < 0 or a >= len(parts) or b >= len(parts) or a == b:
                return None
            bonds.append({
                "part_a": a,
                "element_a": str(parts[a][0]),
                "endpoint_a": connection["endpoint_a"],
                "part_b": b,
                "element_b": str(parts[b][0]),
                "endpoint_b": connection["endpoint_b"],
                "bond_energy": _rounded(float(connection.get("bond_energy", 0.0))),
            })
        except (KeyError, TypeError, ValueError):
            return None

    return {
        "coordinate_frame": "constituent centroid; first noncoincident constituent vector defines +x; vertical y is minimum-image unwrapped; rotation only, no reflection",
        "world_centroid": {"x": _rounded(center_x), "y": _rounded(center_y_world)},
        "vertical_wrap_period": _rounded(world_height) if world_height is not None else None,
        "world_frame_rotation_radians": _rounded(frame_angle),
        "constituents": constituents,
        "pairwise_constituent_distances": pair_distances,
        "bonds": bonds,
    }


def _load_seed(root: Path, seed: int, duration: int) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    chemistry_path = root / f"seed-{seed}-chemistry.json"
    raw_path = root / f"seed-{seed}.jsonl"
    if not chemistry_path.is_file() or not raw_path.is_file():
        raise ValueError(f"seed {seed}: missing chemistry JSON or raw JSONL artifact")
    chemistry = json.loads(chemistry_path.read_text(encoding="utf-8"))
    source = chemistry.get("input", {})
    expected = {
        "seed": seed,
        "requested_duration_seconds": duration,
        "initial_physical_material_count": 1280,
        "initial_independent_resource_count": 0,
        "initial_molecule_count": 1280,
        "initial_constituent_count": 2560,
    }
    for key, value in expected.items():
        if source.get(key) != value:
            raise ValueError(f"seed {seed}: expected {key}={value}, got {source.get(key)}")
    history = chemistry.get("molecular_history", {})
    if history.get("validation_errors"):
        raise ValueError(f"seed {seed}: molecular lifecycle validation errors")
    records = []
    with raw_path.open(encoding="utf-8") as stream:
        for line_number, line in enumerate(stream, 1):
            if not line.strip():
                continue
            try:
                record = json.loads(line)
            except json.JSONDecodeError as exc:
                raise ValueError(f"{raw_path}:{line_number}: invalid JSON: {exc}") from exc
            if isinstance(record, dict):
                records.append(record)
    return chemistry, records


def analyze(
    root: Path, seeds: list[int], duration: int, world_height: float | None = 250.0
) -> dict[str, Any]:
    instances: list[dict[str, Any]] = []
    reactions: list[dict[str, Any]] = []
    geometry_missing_at_creation = 0
    geometry_missing_at_reaction = 0
    lineage_events_missing_reaction_geometry = 0

    for seed in seeds:
        chemistry, raw_records = _load_seed(root, seed, duration)
        history = chemistry["molecular_history"]
        rows = {int(row["molecule_id"]): row for row in history.get("molecules", [])}
        instance_by_id: dict[int, dict[str, Any]] = {}

        for identity, row in rows.items():
            molecule = row.get("molecule") or {}
            descriptor = geometry_descriptor(molecule, world_height)
            if descriptor is None:
                geometry_missing_at_creation += 1
            try:
                key = family_key(molecule)
                label = family_label(key)
            except (ValueError, TypeError, KeyError):
                key, label = None, "unclassified"
            instance = {
                "seed": seed,
                "molecule_id": identity,
                "family_key": key,
                "family": label,
                "reaction_created": row.get("reaction", "UNKNOWN"),
                "reaction_id_created": row.get("reaction_id"),
                "created_tick": row.get("created_tick"),
                "destroyed_tick": row.get("destroyed_tick"),
                "observed_lifetime_ticks": row.get("observed_lifetime_ticks"),
                "right_censored": bool(row.get("right_censored")),
                "parent_ids": list(row.get("parent_ids") or []),
                "creation_geometry": descriptor,
                "reaction_time_geometry": None,
                "consumed_by_reaction_id": None,
            }
            instances.append(instance)
            instance_by_id[identity] = instance

        pair_events = {
            str(event["reaction_id"]): event
            for event in chemistry.get("reactions", {}).get("elemental_bond_pairs", {}).get("event_attribution", [])
            if event.get("reaction_id") is not None
        }
        for record in raw_records:
            if record.get("event") != "environmental_reaction":
                continue
            event = record.get("reaction") or {}
            if event.get("outcome") != "committed":
                continue
            lifecycle = event.get("molecular_lifecycle") or []
            parents, children = [], []
            parent_geometry_count = 0
            child_geometry_count = 0
            for lifecycle_row in lifecycle:
                identity = lifecycle_row.get("molecule_id")
                if not isinstance(identity, int):
                    continue
                descriptor = geometry_descriptor(
                    lifecycle_row.get("molecule") or {}, world_height
                )
                if lifecycle_row.get("action") == "destroyed":
                    parents.append(identity)
                    if descriptor is not None:
                        parent_geometry_count += 1
                    instance = instance_by_id.get(identity)
                    if instance is not None:
                        instance["reaction_time_geometry"] = descriptor
                        instance["consumed_by_reaction_id"] = lifecycle_row.get("reaction_id")
                    if descriptor is None:
                        geometry_missing_at_reaction += 1
                elif lifecycle_row.get("action") == "created":
                    children.append(identity)
                    if descriptor is not None:
                        child_geometry_count += 1
                    instance = instance_by_id.get(identity)
                    if instance is not None and instance["creation_geometry"] is None:
                        instance["creation_geometry"] = descriptor

            reaction_id = next(
                (str(item.get("reaction_id")) for item in lifecycle if item.get("reaction_id") is not None),
                None,
            )
            attribution = pair_events.get(reaction_id, {})
            all_participants = len(parents) + len(children)
            geometry_participants = parent_geometry_count + child_geometry_count
            if geometry_participants != all_participants:
                lineage_events_missing_reaction_geometry += 1
            reactions.append({
                "seed": seed,
                "reaction_id": reaction_id,
                "tick": event.get("tick"),
                "reaction": event.get("reaction", "UNKNOWN"),
                "parent_ids": sorted(parents),
                "child_ids": sorted(children),
                "parent_geometry_count": parent_geometry_count,
                "child_geometry_count": child_geometry_count,
                "all_participants_have_geometry": geometry_participants == all_participants,
                "formed_bond_pairs": attribution.get("formed", {}),
                "broken_bond_pairs": attribution.get("broken", {}),
            })

    total_instances = len(instances)
    creation_complete = sum(row["creation_geometry"] is not None for row in instances)
    reaction_complete = sum(row["reaction_time_geometry"] is not None for row in instances)
    summary = {
        "seed_count": len(seeds),
        "molecule_instances": total_instances,
        "instances_with_creation_geometry": creation_complete,
        "instances_missing_creation_geometry": geometry_missing_at_creation,
        "instances_with_reaction_time_geometry": reaction_complete,
        "destroyed_instances_missing_reaction_time_geometry": geometry_missing_at_reaction,
        "committed_reaction_events": len(reactions),
        "reaction_events_with_complete_participant_geometry": sum(
            row["all_participants_have_geometry"] for row in reactions
        ),
        "reaction_events_missing_participant_geometry": lineage_events_missing_reaction_geometry,
        "unique_structural_families": len({row["family_key"] for row in instances if row["family_key"] is not None}),
    }
    return {
        "schema_version": 1,
        "experiment": {
            "seeds": seeds,
            "duration_seconds_per_seed": duration,
            "world_height": world_height,
            "vertical_boundary": "periodic wrap" if world_height is not None else "not normalized",
            "starting_inventory": "1,280 established two-constituent molecules; zero independent resources; 2,560 constituents",
        },
        "summary": summary,
        "molecule_instances": instances,
        "reaction_events": reactions,
        "interpretation_limits": [
            "Geometry is normalized for translation and global rotation but not reflection; constituent indices remain explicit.",
            "Creation geometry is the molecule snapshot at creation; reaction-time geometry comes from the committed event participant snapshot.",
            "The artifacts do not record every eligible per-instance contact opportunity, so geometry-conditioned reaction probabilities cannot yet be estimated.",
            "World centroid is retained separately and must not be confused with intrinsic molecular conformation.",
            "Right-censored molecules have no observed destruction reaction; their final fate is unknown.",
        ],
    }


def _write_csv(path: Path, rows: list[dict[str, Any]]) -> None:
    if not rows:
        path.write_text("", encoding="utf-8")
        return
    with path.open("w", newline="", encoding="utf-8") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
        writer.writeheader()
        for row in rows:
            writer.writerow({
                key: json.dumps(value, separators=(",", ":")) if isinstance(value, (dict, list)) else value
                for key, value in row.items()
            })


def render_markdown(report: dict[str, Any]) -> str:
    summary = report["summary"]
    lines = [
        "# EvoSim instance-level geometry-aware reaction network",
        "",
        f"- Seeds: {', '.join(map(str, report['experiment']['seeds']))}",
        f"- Duration per seed: {report['experiment']['duration_seconds_per_seed']} seconds",
        f"- Molecule instances: {summary['molecule_instances']}",
        f"- Instances with creation geometry: {summary['instances_with_creation_geometry']}",
        f"- Instances with reaction-time geometry: {summary['instances_with_reaction_time_geometry']}",
        f"- Committed reaction events: {summary['committed_reaction_events']}",
        f"- Reaction events with complete participant geometry: {summary['reaction_events_with_complete_participant_geometry']}",
        f"- Unique structural families: {summary['unique_structural_families']}",
        "",
        "## Geometry coverage",
        "",
        f"- Missing creation geometry: {summary['instances_missing_creation_geometry']}",
        f"- Destroyed instances missing reaction-time geometry: {summary['destroyed_instances_missing_reaction_time_geometry']}",
        f"- Reaction events missing participant geometry: {summary['reaction_events_missing_participant_geometry']}",
        "",
        "## What the records preserve",
        "",
        "- Per-instance constituent positions and rotations, normalized to remove world translation and global rotation.",
        "- Realized constituent shapes and pairwise constituent-center distances.",
        "- Bond endpoint classes and local endpoint descriptors.",
        "- Exact per-bond energies and parent/product reaction lineage.",
        "- Separate world centroid and reaction-time geometry snapshots.",
        "",
        "## Important limitation",
        "",
        "The existing artifacts record committed participant geometry, but not every eligible instance-level contact opportunity. This supports descriptive geometry/pathway comparisons, not unbiased geometry-conditioned reaction probabilities. Candidate-opportunity instrumentation is a separate future diagnostic step.",
        "",
    ]
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input_dir", type=Path)
    parser.add_argument("--seeds", required=True)
    parser.add_argument("--duration-seconds", type=int, required=True)
    parser.add_argument(
        "--world-height", type=float, default=250.0,
        help="Vertical periodic-wrap period used to unwrap constituent geometry (default: 250).",
    )
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    seeds = [int(value.strip()) for value in args.seeds.split(",") if value.strip()]
    if not seeds or len(seeds) != len(set(seeds)):
        parser.error("--seeds must be a nonempty list of unique integers")
    report = analyze(args.input_dir, seeds, args.duration_seconds, args.world_height)
    args.output_dir.mkdir(parents=True, exist_ok=True)
    (args.output_dir / "geometry-aware-network.json").write_text(
        json.dumps(report, indent=2) + "\n", encoding="utf-8"
    )
    (args.output_dir / "geometry-aware-network.md").write_text(
        render_markdown(report), encoding="utf-8"
    )
    _write_csv(args.output_dir / "molecule-instances.csv", report["molecule_instances"])
    _write_csv(args.output_dir / "instance-reaction-events.csv", report["reaction_events"])
    print(render_markdown(report), end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
