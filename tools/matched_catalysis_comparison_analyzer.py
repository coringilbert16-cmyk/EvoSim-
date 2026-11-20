#!/usr/bin/env python3
"""Compare paired candidate-near and candidate-remote chemistry observations.

The output is a screening comparison only. Bounded deterministic opportunity
samples are not a complete, unbiased exposure denominator and cannot establish
catalytic causation by themselves.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


def _parts_key(raw: Any) -> tuple[str, ...] | None:
    if not isinstance(raw, list) or not raw:
        return None
    names: list[str] = []
    for item in raw:
        if not isinstance(item, (list, tuple)) or len(item) != 2:
            return None
        name, amount = item
        try:
            quantity = float(amount)
        except (TypeError, ValueError):
            return None
        if not isinstance(name, str) or not name or not quantity.is_integer() or quantity <= 0:
            return None
        names.extend([name] * int(quantity))
    return tuple(sorted(names))


def _target_pair_key(raw: Any) -> tuple[tuple[str, ...], tuple[str, ...]] | None:
    if not isinstance(raw, list) or len(raw) != 2:
        return None
    first, second = _parts_key(raw[0]), _parts_key(raw[1])
    if first is None or second is None:
        return None
    return tuple(sorted((first, second)))  # type: ignore[return-value]


def _load_seed(path: Path, expected_seed: int, expected_condition: str,
               target_pair: tuple[tuple[str, ...], tuple[str, ...]]) -> dict[str, Any]:
    if not path.is_file():
        raise ValueError(f"seed {expected_seed}: missing observation {path}")

    records: list[dict[str, Any]] = []
    with path.open(encoding="utf-8") as stream:
        for line_number, line in enumerate(stream, 1):
            if not line.strip():
                continue
            try:
                record = json.loads(line)
            except json.JSONDecodeError as exc:
                raise ValueError(f"{path}:{line_number}: invalid JSON: {exc}") from exc
            if isinstance(record, dict):
                records.append(record)

    header = next((row for row in records if row.get("event") == "environmental_chemistry_header"), None)
    setup_record = next((row for row in records if row.get("event") == "environmental_chemistry_experiment_setup"), None)
    summary = next((row for row in reversed(records) if row.get("event") == "environmental_chemistry_summary"), {})
    if header is None or setup_record is None:
        raise ValueError(f"seed {expected_seed}: missing chemistry header or matched setup metadata")
    metadata = setup_record.get("metadata") or {}
    if int(header.get("seed", -1)) != expected_seed or int(metadata.get("seed", -1)) != expected_seed:
        raise ValueError(f"seed {expected_seed}: run seed does not match requested seed")
    if metadata.get("condition") != expected_condition:
        raise ValueError(f"seed {expected_seed}: expected {expected_condition}, found {metadata.get('condition')}")

    # Build the full identity-to-composition lookup before classifying samples.
    # Initial identities and all created products remain in this lookup even
    # after destruction, so same-tick diagnostic records can still be traced.
    identity_parts: dict[int, tuple[str, ...]] = {}
    destroyed_ids: set[int] = set()
    for row in records:
        if row.get("event") != "molecular_lifecycle":
            continue
        try:
            identity = int(row.get("molecule_id"))
        except (TypeError, ValueError):
            continue
        molecule = row.get("molecule") or {}
        material = molecule.get("material") or {}
        parts = _parts_key(material.get("parts"))
        if row.get("action") == "created" and parts is not None:
            identity_parts[identity] = parts
        elif row.get("action") == "destroyed":
            destroyed_ids.add(identity)

    candidate_id = int(metadata["candidate_identity"])
    target_a_id = int(metadata["target_reactant_a_identity"])
    target_b_id = int(metadata["target_reactant_b_identity"])
    if candidate_id in (target_a_id, target_b_id):
        raise ValueError(f"seed {expected_seed}: candidate is one of the target reactants")
    if not metadata.get("initial_elemental_inventory_unchanged"):
        raise ValueError(f"seed {expected_seed}: setup did not confirm unchanged elemental inventory")
    if not metadata.get("initial_energy_inventory_unchanged"):
        raise ValueError(f"seed {expected_seed}: setup did not confirm unchanged energy inventory")
    if not metadata.get("all_non_candidate_molecule_geometry_unchanged"):
        raise ValueError(f"seed {expected_seed}: setup moved molecules other than the candidate")

    target_ids = tuple(sorted((target_a_id, target_b_id)))
    target_committed_events = []
    target_rejected_events = []
    target_kind_counts = {"COMBINE": 0, "DISPLACEMENT": 0}
    for row in records:
        if row.get("event") != "environmental_reaction":
            continue
        event = row.get("reaction") or {}
        reaction_kind = event.get("reaction")
        if reaction_kind not in target_kind_counts:
            continue
        source_ids = event.get("source_material_ids") or []
        if tuple(sorted(int(identity) for identity in source_ids)) != target_ids:
            continue
        if event.get("outcome") == "committed":
            target_committed_events.append(event)
            target_kind_counts[reaction_kind] += 1
        elif event.get("outcome") == "rejected":
            target_rejected_events.append(event)

    target_opportunity_samples = []
    for row in records:
        if row.get("event") != "environmental_chemistry_tick":
            continue
        work = row.get("reaction_work") or {}
        for kind, search_key in (("COMBINE", "combine_search"), ("DISPLACEMENT", "displacement_discovery")):
            search = work.get(search_key) or {}
            for sample in search.get("opportunity_samples", []) or []:
                if not isinstance(sample, dict):
                    continue
                source_ids = [sample.get("source_a_identity"), sample.get("source_b_identity")]
                try:
                    exact_source_ids = tuple(sorted(int(identity) for identity in source_ids))
                except (TypeError, ValueError):
                    continue
                if exact_source_ids != target_ids:
                    continue
                try:
                    composition_pair = tuple(sorted((identity_parts[int(source_ids[0])], identity_parts[int(source_ids[1])])))
                except (KeyError, TypeError, ValueError, IndexError):
                    continue
                if composition_pair != target_pair:
                    continue
                neighborhood = sample.get("local_neighborhood") or {}
                neighbors = [entry for entry in (neighborhood.get("molecules") or []) if isinstance(entry, dict)]
                target_opportunity_samples.append({
                    "reaction_kind": kind,
                    "tick": row.get("tick"),
                    "physically_valid": bool(sample.get("physically_valid")),
                    "candidate_observed_nearby": any(int(entry.get("identity", -1)) == candidate_id for entry in neighbors),
                    "neighbor_truncated_count": int(neighborhood.get("truncated_molecule_count", 0) or 0),
                    "neighbor_missing_geometry_count": int(neighborhood.get("missing_geometry_count", 0) or 0),
                    "exact_transaction_match_count": int(sample.get("exact_transaction_match_count", 0) or 0),
                })

    duration = int(header.get("requested_duration_seconds") or 0)
    if duration <= 0:
        raise ValueError(f"seed {expected_seed}: missing or invalid requested duration")
    initial_inventory = header.get("initial_material_inventory")
    return {
        "seed": expected_seed,
        "condition": expected_condition,
        "duration_seconds": duration,
        "setup": metadata,
        "initial_material_inventory": initial_inventory,
        "target_committed_reactions": len(target_committed_events),
        "target_rejected_reactions": len(target_rejected_events),
        "target_committed_by_kind": target_kind_counts,
        "target_opportunity_samples": len(target_opportunity_samples),
        "target_physically_valid_samples": sum(row["physically_valid"] for row in target_opportunity_samples),
        "target_samples_with_candidate_nearby": sum(row["candidate_observed_nearby"] for row in target_opportunity_samples),
        "target_samples_with_exact_transaction_match": sum(row["exact_transaction_match_count"] > 0 for row in target_opportunity_samples),
        "target_samples_with_truncated_neighborhood": sum(row["neighbor_truncated_count"] > 0 for row in target_opportunity_samples),
        "candidate_identity_destroyed": candidate_id in destroyed_ids,
        "candidate_destruction_events": sum(
            1 for row in records
            if row.get("event") == "molecular_lifecycle"
            and row.get("action") == "destroyed"
            and int(row.get("molecule_id", -1)) == candidate_id
        ),
        "energy_balance_residual": summary.get("energy_balance_residual"),
    }


def analyze(near_dir: Path, remote_dir: Path, seeds: list[int],
            duration_seconds: int, config: dict[str, Any]) -> dict[str, Any]:
    if not seeds or len(seeds) != len(set(seeds)):
        raise ValueError("seeds must be a non-empty list of unique values")
    target_a = _parts_key([[name, 1] for name in config["target_reactant_a_parts"]])
    target_b = _parts_key([[name, 1] for name in config["target_reactant_b_parts"]])
    if target_a is None or target_b is None:
        raise ValueError("target reactant part lists must contain valid element names")
    target_pair = tuple(sorted((target_a, target_b)))
    if target_a == target_b:
        raise ValueError("target reactant compositions must be different")

    paired_rows = []
    for seed in seeds:
        near = _load_seed(near_dir / f"seed-{seed}.jsonl", seed, "candidate_near", target_pair)
        remote = _load_seed(remote_dir / f"seed-{seed}.jsonl", seed, "candidate_remote", target_pair)
        if near["duration_seconds"] != duration_seconds or remote["duration_seconds"] != duration_seconds:
            raise ValueError(f"seed {seed}: observed duration differs from requested duration")
        for key in ("candidate_identity", "target_reactant_a_identity", "target_reactant_b_identity"):
            if near["setup"].get(key) != remote["setup"].get(key):
                raise ValueError(f"seed {seed}: paired setup differs for {key}")
        if near["initial_material_inventory"] != remote["initial_material_inventory"]:
            raise ValueError(f"seed {seed}: paired arms have different initial elemental inventories")
        paired_rows.append({
            "seed": seed,
            "candidate_identity": near["setup"]["candidate_identity"],
            "target_reactant_a_identity": near["setup"]["target_reactant_a_identity"],
            "target_reactant_b_identity": near["setup"]["target_reactant_b_identity"],
            "near": near,
            "remote": remote,
            "delta_target_committed_reactions_near_minus_remote":
                near["target_committed_reactions"] - remote["target_committed_reactions"],
        })

    pooled = {}
    for condition in ("near", "remote"):
        rows = [pair[condition] for pair in paired_rows]
        total_committed = sum(row["target_committed_reactions"] for row in rows)
        total_samples = sum(row["target_opportunity_samples"] for row in rows)
        pooled[condition] = {
            "target_committed_reactions": total_committed,
            "target_rejected_reactions": sum(row["target_rejected_reactions"] for row in rows),
            "target_committed_per_wall_clock_minute": total_committed / (duration_seconds * len(seeds) / 60.0),
            "target_opportunity_samples": total_samples,
            "target_physically_valid_samples": sum(row["target_physically_valid_samples"] for row in rows),
            "target_samples_with_candidate_nearby": sum(row["target_samples_with_candidate_nearby"] for row in rows),
            "target_samples_with_exact_transaction_match": sum(row["target_samples_with_exact_transaction_match"] for row in rows),
            "candidate_identity_destroyed_seeds": sum(row["candidate_identity_destroyed"] for row in rows),
            "candidate_destruction_events": sum(row["candidate_destruction_events"] for row in rows),
            "mean_energy_balance_residual": (
                sum(float(row["energy_balance_residual"]) for row in rows if isinstance(row["energy_balance_residual"], (int, float)))
                / max(1, sum(isinstance(row["energy_balance_residual"], (int, float)) for row in rows))
            ),
        }
        pooled[condition]["commits_per_sampled_target_opportunity_descriptive_only"] = (
            total_committed / total_samples if total_samples else None
        )

    report = {
        "schema_version": 1,
        "experiment": {
            "seeds": seeds,
            "duration_seconds_per_seed": duration_seconds,
            "candidate_parts": config["candidate_parts"],
            "target_reactant_a_parts": config["target_reactant_a_parts"],
            "target_reactant_b_parts": config["target_reactant_b_parts"],
            "near_distance": config["near_distance"],
            "minimum_remote_distance": config["minimum_remote_distance"],
            "max_target_pair_distance": config["max_target_pair_distance"],
        },
        "paired_seeds": paired_rows,
        "pooled": pooled,
        "interpretation_limits": [
            "This is a paired placement screen, not proof that the selected candidate is a catalyst.",
            "The local-neighborhood opportunity samples are bounded and deterministic, not a complete unbiased count of eligible target-reactant contacts.",
            "The descriptive commits-per-sampled-opportunity ratio must not be interpreted as a reaction probability or exposure-normalized catalytic rate.",
            "Candidate identity destruction shows consumption of the configured instance; lack of destruction only shows that this identity was not retired in the recorded lifecycle stream.",
            "Global usable/resonant reservoir values are not local field energy density. Full energy-balance residuals must be reviewed alongside event counts.",
            "A causal claim still requires a justified operational facilitator definition, adequate eligible-opportunity denominators, repeated matched seeds, uncertainty intervals, and confirmation that the candidate remains chemically available while target reactions are promoted.",
        ],
    }
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--near-dir", type=Path, required=True)
    parser.add_argument("--remote-dir", type=Path, required=True)
    parser.add_argument("--seeds", required=True)
    parser.add_argument("--duration-seconds", type=int, required=True)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    seeds = [int(item.strip()) for item in args.seeds.split(",") if item.strip()]
    config = json.loads(args.config.read_text(encoding="utf-8"))
    report = analyze(args.near_dir, args.remote_dir, seeds, args.duration_seconds, config)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    markdown_path = args.output.with_suffix(".md")
    lines = [
        "# Matched candidate-placement chemistry screen",
        "",
        f"- Seeds: {', '.join(map(str, seeds))}",
        f"- Duration per seed: {args.duration_seconds} seconds",
        "",
        "## Paired pooled observations",
        "",
        "| Metric | Candidate near | Candidate remote |",
        "|---|---:|---:|",
    ]
    metric_labels = (
        ("target_committed_reactions", "Target reactions committed"),
        ("target_rejected_reactions", "Target reactions rejected"),
        ("target_committed_per_wall_clock_minute", "Target commits per wall-clock minute"),
        ("target_opportunity_samples", "Sampled target opportunities"),
        ("target_physically_valid_samples", "Physically valid sampled opportunities"),
        ("target_samples_with_candidate_nearby", "Samples with candidate in neighborhood"),
        ("candidate_identity_destroyed_seeds", "Seeds where candidate identity was consumed"),
        ("candidate_destruction_events", "Candidate identity destruction events"),
    )
    for key, label in metric_labels:
        lines.append(f"| {label} | {report['pooled']['near'][key]} | {report['pooled']['remote'][key]} |")
    lines.extend(["", "## Interpretation limits", ""])
    lines.extend(f"- {item}" for item in report["interpretation_limits"])
    markdown_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print("\n".join(lines))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
