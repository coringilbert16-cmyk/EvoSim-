#!/usr/bin/env python3
"""Build compact, durable experiment records from specialist analyzer outputs."""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

SCHEMA = "evosim.permanent_experiment_record"
SCHEMA_VERSION = 1
REACTIONS = ("BREAK", "COMBINE", "DISPLACEMENT")


def read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected a JSON object")
    return value


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def nested(value: dict[str, Any], *keys: str, default: Any = None) -> Any:
    current: Any = value
    for key in keys:
        if not isinstance(current, dict) or key not in current:
            return default
        current = current[key]
    return current


def scalar_tree(value: Any) -> Any:
    """Keep numeric/string/bool/null aggregates; drop event arrays and object lists."""
    if isinstance(value, dict):
        return {str(key): scalar_tree(item) for key, item in value.items()
                if isinstance(item, (dict, int, float, str, bool)) or item is None}
    if isinstance(value, list):
        return None
    if isinstance(value, (int, float, str, bool)) or value is None:
        return value
    return None


def build_record(seed: int, chemistry_path: Path, organization_path: Path,
                 manifest_path: Path | None = None) -> dict[str, Any]:
    chemistry = read_json(chemistry_path)
    organization = read_json(organization_path)
    source = chemistry.get("input", {})
    if source.get("seed") != seed:
        raise ValueError(f"seed mismatch: requested {seed}, chemistry report says {source.get('seed')}")
    if not chemistry.get("sampling", {}).get("summary_present"):
        raise ValueError(f"seed {seed}: final chemistry summary is missing")
    lifecycle_errors = nested(chemistry, "molecular_history", "validation_errors", default=[])
    organization_errors = nested(organization, "scope", "lifecycle_validation_errors", default=[])
    if not isinstance(lifecycle_errors, list) or not isinstance(organization_errors, list):
        raise ValueError(f"seed {seed}: lifecycle validation fields must be arrays")

    manifest = read_json(manifest_path) if manifest_path else {}
    experiment = manifest.get("experiment", {})
    run = manifest.get("run", {})
    conditions = manifest.get("conditions", {})
    config = conditions.get("configuration", {})
    reactions = chemistry.get("reactions", {})
    totals = reactions.get("diagnostic_totals", {})
    event_counts = reactions.get("event_counts", {})
    outcome_counts = reactions.get("outcome_counts", {})
    history = chemistry.get("molecular_history", {})
    org = organization.get("organization", {})
    energy = chemistry.get("energy", {})
    bond_pairs = reactions.get("elemental_bond_pairs", {})

    reaction_rows: dict[str, Any] = {}
    for name in REACTIONS:
        reaction_rows[name] = {
            "event_count": event_counts.get(name),
            "outcomes": scalar_tree(outcome_counts.get(name, {})),
            "diagnostics": scalar_tree(totals.get(name, {})),
        }

    record = {
        "schema": SCHEMA,
        "schema_version": SCHEMA_VERSION,
        "record_status": "complete" if not lifecycle_errors and not organization_errors else "partial",
        "identity": {
            "experiment_id": experiment.get("experiment_id"),
            "research_plan": experiment.get("research_plan"),
            "condition_id": experiment.get("condition_id"),
            "seed": seed,
            "independent_replicate_count": None,
        },
        "provenance": {
            "source_commit": nested(manifest, "provenance", "commit_sha"),
            "workflow_run_id": nested(manifest, "provenance", "workflow_run_id"),
            "workflow_run_url": nested(manifest, "provenance", "workflow_run_url"),
            "manifest_schema_version": manifest.get("schema_version"),
            "configuration_sha256": conditions.get("configuration_sha256"),
            "raw_diagnostics_sha256": next(
                (item.get("sha256") for item in manifest.get("artifacts", [])
                 if item.get("role") == "raw_observation"), None),
            "raw_diagnostics_size_bytes": next(
                (item.get("size_bytes") for item in manifest.get("artifacts", [])
                 if item.get("role") == "raw_observation"), None),
            "chemistry_analyzer_sha256": sha256_file(chemistry_path),
            "organization_analyzer_sha256": sha256_file(organization_path),
        },
        "conditions": {
            "requested_duration_seconds": source.get("requested_duration_seconds"),
            "actual_elapsed_wall_seconds": run.get("actual_elapsed_wall_seconds"),
            "start_tick": run.get("start_tick"),
            "end_tick": run.get("end_tick"),
            "tick_count": (
                run.get("end_tick") - run.get("start_tick")
                if isinstance(run.get("end_tick"), int) and isinstance(run.get("start_tick"), int)
                else None
            ),
            "diagnostic_interval": source.get("diagnostic_interval"),
            "configuration": scalar_tree(config),
            "initial_physical_material_count": source.get("initial_physical_material_count"),
            "initial_independent_resource_count": source.get("initial_independent_resource_count"),
            "initial_molecule_count": source.get("initial_molecule_count"),
            "initial_constituent_count": source.get("initial_constituent_count"),
            "active_resources": nested(chemistry, "resource_coverage", "required_resources"),
        },
        "inventory": {
            "initial_by_resource": scalar_tree(nested(chemistry, "resource_coverage", "initial_inventory", default={})),
            "final_by_resource": scalar_tree(nested(chemistry, "resource_coverage", "final_inventory", default=None)),
            "initial_physical_material_count": source.get("initial_physical_material_count"),
            "initial_constituent_count": source.get("initial_constituent_count"),
        },
        "reactions": reaction_rows,
        "opportunity_coverage": scalar_tree(nested(reactions, "opportunity_coverage", default=None)),
        "settlement": scalar_tree(nested(reactions, "settlement", default=None)),
        "bond_accounting": {
            "formed_total": bond_pairs.get("formed_total"),
            "broken_total": bond_pairs.get("broken_total"),
            "committed_events_with_graph_deltas": bond_pairs.get("committed_events_with_graph_deltas"),
            "committed_events_missing_graph_deltas": bond_pairs.get("committed_events_missing_graph_deltas"),
            "inventory_reconciliation": scalar_tree(bond_pairs.get("inventory_reconciliation")),
            "formed_by_element_pair": scalar_tree(bond_pairs.get("formed")),
            "broken_by_element_pair": scalar_tree(bond_pairs.get("broken")),
        },
        "molecular_development": {
            "creation_count": history.get("molecule_creation_count"),
            "destruction_count": history.get("molecule_destruction_count"),
            "active_at_end_count": history.get("active_at_end_count"),
            "lineage_edge_count": len(history.get("lineage_edges", []))
                if isinstance(history.get("lineage_edges"), list) else None,
            "lineage_reaction_counts": scalar_tree(history.get("lineage_reaction_counts")),
            "mean_observed_lifetime_ticks": history.get("mean_observed_lifetime_ticks"),
            "unique_structural_families": org.get("unique_exact_structural_families"),
            "recurrent_structural_families": org.get("recurrent_families"),
            "families_active_at_end": org.get("families_active_at_end"),
            "compound_identity_count": org.get("molecules_with_multi_resource_or_bonded_structure"),
            "reaction_created_compound_material": scalar_tree(
                nested(organization, "material_distribution",
                       "reaction_created_compound_constituent_amount_by_resource", default={})
            ),
            "family_transition_count": len(nested(
                organization, "reaction_network", "family_to_family_transitions", default=[]
            )),
        },
        "energy": {
            "first_balance_residual": energy.get("first_balance_residual"),
            "last_balance_residual": energy.get("last_balance_residual"),
            "maximum_absolute_snapshot_residual": energy.get("maximum_absolute_snapshot_residual"),
        },
        "quality": {
            "chemistry_lifecycle_validation_errors": lifecycle_errors,
            "organization_lifecycle_validation_errors": organization_errors,
            "missing_manifest": manifest_path is None,
            "missing_fields": [
                field for field, value in {
                    "configuration_sha256": conditions.get("configuration_sha256"),
                    "actual_elapsed_wall_seconds": run.get("actual_elapsed_wall_seconds"),
                    "opportunity_coverage": nested(reactions, "opportunity_coverage", default=None),
                    "settlement": nested(reactions, "settlement", default=None),
                }.items() if value is None
            ],
            "warnings": [],
            "interpretation_limits": [
                "Seeds are independent replicates; reaction events within a seed are not independent samples.",
                "Missing or partial opportunity denominators do not support conversion-rate claims.",
                "Structural-family recurrence is descriptive and does not establish catalysis or self-sustaining chemistry.",
                "Raw diagnostics and intermediate analyzer outputs are temporary evidence, not part of this permanent record.",
            ],
        },
        "evidence": [],
    }
    if manifest_path is None:
        record["quality"]["warnings"].append("No run manifest supplied; provenance is incomplete.")
    if lifecycle_errors or organization_errors:
        record["quality"]["warnings"].append("Molecular lifecycle validation reported errors.")
    return record


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--seed", type=int, required=True)
    parser.add_argument("--chemistry", type=Path, required=True)
    parser.add_argument("--organization", type=Path, required=True)
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    record = build_record(args.seed, args.chemistry, args.organization, args.manifest)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(record, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    print(f"Compact permanent record: {args.output} ({args.output.stat().st_size} bytes)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
