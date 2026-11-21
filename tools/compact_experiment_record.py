#!/usr/bin/env python3
"""Build compact, durable experiment records from specialist analyzer outputs."""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from tools.experiment_manifest import validate_manifest  # noqa: E402

SCHEMA = "evosim.permanent_experiment_record"
SCHEMA_VERSION = 1
DROP_KEYS = {
    "participant_sets", "opportunity_samples", "tick_diagnostics", "molecules",
    "lineage_edges", "families", "top_families", "family_to_family_transitions",
    "event_attribution",
}
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
                if key not in DROP_KEYS and (isinstance(item, (dict, list, int, float, str, bool)) or item is None)}
    if isinstance(value, list):
        if len(value) <= 128 and all(item is None or isinstance(item, (int, float, str, bool)) for item in value):
            return value
        return None
    if isinstance(value, (int, float, str, bool)) or value is None:
        return value
    return None


def build_record(seed: int, chemistry_path: Path, organization_path: Path,
                 manifest_path: Path | None = None,
                 opportunity_coverage_path: Path | None = None) -> dict[str, Any]:
    chemistry = read_json(chemistry_path)
    organization = read_json(organization_path)
    source = chemistry.get("input", {})
    if source.get("seed") != seed:
        raise ValueError(f"seed mismatch: requested {seed}, chemistry report says {source.get('seed')}")
    if not chemistry.get("sampling", {}).get("summary_present"):
        raise ValueError(f"seed {seed}: final chemistry summary is missing")
    lifecycle_errors = nested(chemistry, "molecular_history", "validation_errors", default=None)
    organization_errors = nested(organization, "scope", "lifecycle_validation_errors", default=None)
    if lifecycle_errors is not None and not isinstance(lifecycle_errors, list):
        raise ValueError(f"seed {seed}: chemistry lifecycle validation errors must be an array")
    if organization_errors is not None and not isinstance(organization_errors, list):
        raise ValueError(f"seed {seed}: organization lifecycle validation errors must be an array")
    lineage_status = (
        "unknown" if lifecycle_errors is None or organization_errors is None
        else "invalid" if lifecycle_errors or organization_errors
        else "valid"
    )
    org = organization.get("organization", {})
    transition_rows = nested(organization, "reaction_network", "family_to_family_transitions", default=None)
    transition_summary = None
    top_structural_families = [
        {
            "signature": row.get("signature"),
            "occurrences": row.get("occurrences"),
            "reaction_created_occurrences": row.get("reaction_created_occurrences"),
            "reaction_created_reaction_events": row.get("reaction_created_reaction_events"),
            "reaction_created_active_at_end": row.get("reaction_created_active_at_end"),
            "active_at_end": row.get("active_at_end"),
            "composition": scalar_tree(row.get("composition", {})),
            "mean_observed_lifetime_ticks": row.get("mean_observed_lifetime_ticks"),
            "median_observed_lifetime_ticks": row.get("median_observed_lifetime_ticks"),
            "completed_lifetimes": row.get("completed_lifetimes"),
            "right_censored_lifetimes": row.get("right_censored_lifetimes"),
            "recurrent_from_reactions": row.get("recurrent_from_reactions"),
        }
        for row in (org.get("top_families", []) or [])[:25]
        if isinstance(row, dict)
    ]
    if isinstance(transition_rows, list):
        ordered_transitions = sorted(
            (row for row in transition_rows if isinstance(row, dict)),
            key=lambda row: (-int(row.get("count", 0) or 0), str(row.get("reaction")),
                             str(row.get("parent_family")), str(row.get("child_family"))),
        )
        transition_summary = {
            "transition_type_count": len(transition_rows),
            "transition_event_count": sum(
                int(row.get("count", 0) or 0) for row in ordered_transitions
            ),
            "top_family_pairs": [
                {
                    "reaction": row.get("reaction"),
                    "parent_family": row.get("parent_family"),
                    "child_family": row.get("child_family"),
                    "count": row.get("count"),
                }
                for row in ordered_transitions[:25]
            ],
            "truncated": len(ordered_transitions) > 25,
        }

    manifest = validate_manifest(manifest_path) if manifest_path else {}
    coverage_report = read_json(opportunity_coverage_path) if opportunity_coverage_path else {}
    seed_coverage = next(
        (row for row in coverage_report.get("per_seed", [])
         if row.get("seed") == seed), None
    )
    experiment = manifest.get("experiment", {})
    run = manifest.get("run", {})
    conditions = manifest.get("conditions", {})
    config = conditions.get("configuration", {})
    comparison_config = dict(config) if isinstance(config, dict) else {}
    # Seed-specific stochastic realizations belong in per-seed setup, not in
    # the shared measurement-definition digest.
    comparison_config.pop("seed", None)
    comparison_config.pop("pile_sizes", None)
    comparison_config.pop("initial_material_inventory", None)
    comparison_configuration_sha256 = hashlib.sha256(
        json.dumps(comparison_config, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest() if comparison_config else None
    reactions = chemistry.get("reactions", {})
    totals = reactions.get("diagnostic_totals", {})
    event_counts = reactions.get("event_counts", {})
    outcome_counts = reactions.get("outcome_counts", {})
    history = chemistry.get("molecular_history", {})
    energy = chemistry.get("energy", {})
    bond_pairs = reactions.get("elemental_bond_pairs", {})

    reaction_rows: dict[str, Any] = {}
    for name in REACTIONS:
        reaction_rows[name] = {
            "event_count": event_counts.get(name),
            "outcomes": scalar_tree(outcome_counts.get(name, {})),
            "diagnostics": scalar_tree(totals.get(name, {})),
        }

    coverage_states = (
        seed_coverage.get("coverage", {}) if isinstance(seed_coverage, dict) else {}
    )
    partial_or_unknown_coverage = [
        {"reaction": reaction, "state": details.get("state")}
        for reaction, details in coverage_states.items()
        if isinstance(details, dict) and details.get("state") in ("partial", "unknown")
    ]

    record = {
        "schema": SCHEMA,
        "schema_version": SCHEMA_VERSION,
        "record_status": "complete" if manifest_path is not None and lifecycle_errors is not None and organization_errors is not None and not lifecycle_errors and not organization_errors and seed_coverage is not None and nested(reactions, "settlement", default=None) is not None else "partial",
        "identity": {
            "experiment_id": experiment.get("experiment_id"),
            "research_plan": experiment.get("research_plan"),
            "condition_id": experiment.get("condition_id"),
            "seed": seed,
            "independent_replicate_count": None,
        },
        "provenance": {
            "source_commit": nested(manifest, "build", "commit_sha"),
            "simulator_version": nested(manifest, "build", "executable_version"),
            "workflow_run_id": nested(manifest, "run", "run_id"),
            "workflow_run_url": nested(manifest, "run", "workflow_url"),
            "manifest_schema_version": manifest.get("schema_version"),
            "enabled_diagnostic_tiers": nested(manifest, "diagnostics", "enabled_tiers"),
            "configuration_sha256": conditions.get("configuration_sha256"),
            "comparison_configuration_sha256": comparison_configuration_sha256,
            "raw_diagnostics_sha256": next(
                (item.get("sha256") for item in manifest.get("artifacts", [])
                 if item.get("role") == "raw_observation"), None),
            "raw_diagnostics_size_bytes": next(
                (item.get("size_bytes") for item in manifest.get("artifacts", [])
                 if item.get("role") == "raw_observation"), None),
            "chemistry_analyzer_sha256": sha256_file(ROOT / "tools/chemistry_analyzer.py"),
            "organization_analyzer_sha256": sha256_file(ROOT / "tools/chemistry_organization_analyzer.py"),
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
            "final_by_resource": scalar_tree((nested(chemistry, "resource_coverage", "snapshots", default=[]) or [{}])[-1].get("resource_inventory")),
            "initial_physical_material_count": source.get("initial_physical_material_count"),
            "initial_constituent_count": source.get("initial_constituent_count"),
            "final_physical_material_count": (nested(chemistry, "resource_coverage", "snapshots", default=[{}])[-1].get("physical_material_count") if nested(chemistry, "resource_coverage", "snapshots", default=[]) else None),
            "final_constituent_count": (nested(chemistry, "resource_coverage", "snapshots", default=[{}])[-1].get("constituent_count") if nested(chemistry, "resource_coverage", "snapshots", default=[]) else None),
        },
        "reactions": reaction_rows,
        "opportunity_coverage": (
            {
                "by_reaction": scalar_tree(seed_coverage.get("coverage", {})),
                "diagnostic_totals": scalar_tree(seed_coverage.get("totals", {})),
                "diagnostic_ticks": seed_coverage.get("diagnostic_ticks"),
                "source_schema_version": coverage_report.get("schema_version"),
            } if seed_coverage is not None else None
        ),
        "settlement": scalar_tree(nested(reactions, "settlement", default=None)),
        "bond_accounting": {
            "formed_total": bond_pairs.get("formed_total"),
            "broken_total": bond_pairs.get("broken_total"),
            "committed_events_with_graph_deltas": bond_pairs.get("committed_events_with_graph_deltas"),
            "committed_events_missing_graph_deltas": bond_pairs.get("committed_events_missing_graph_deltas"),
            "inventory_reconciliation": scalar_tree(bond_pairs.get("inventory_reconciliation")),
            "formed_by_element_pair": scalar_tree(bond_pairs.get("formed")),
            "broken_by_element_pair": scalar_tree(bond_pairs.get("broken")),
            "by_reaction": scalar_tree(bond_pairs.get("by_reaction")),
            "initial_bond_inventory": scalar_tree(bond_pairs.get("initial_bond_inventory")),
            "final_bond_inventory": scalar_tree(bond_pairs.get("final_bond_inventory")),
            "inventory_delta": scalar_tree(bond_pairs.get("inventory_delta")),
            "reaction_delta": scalar_tree(bond_pairs.get("reaction_delta")),
        },
        "molecular_development": {
            "creation_count": history.get("molecule_creation_count"),
            "destruction_count": history.get("molecule_destruction_count"),
            "active_at_end_count": history.get("active_at_end_count"),
            "lineage_edge_count": len(history.get("lineage_edges", []))
                if isinstance(history.get("lineage_edges"), list) else None,
            "lineage_validation_status": lineage_status,
            "lineage_reaction_counts": scalar_tree(history.get("lineage_reaction_counts")),
            "mean_observed_lifetime_ticks": history.get("mean_observed_lifetime_ticks"),
            "unique_structural_families": org.get("unique_exact_structural_families"),
            "top_structural_families": top_structural_families,
            "recurrent_structural_families": org.get("recurrent_families"),
            "families_active_at_end": org.get("families_active_at_end"),
            "compound_identity_count": org.get("molecules_with_multi_resource_or_bonded_structure"),
            "reaction_created_compound_material": scalar_tree(
                nested(organization, "material_distribution",
                       "reaction_created_compound_constituent_amount_by_resource", default={})
            ),
            "family_transition_summary": transition_summary,
        },
        "organization": {
            "unique_structural_families": org.get("unique_exact_structural_families"),
            "recurrent_product_families": org.get("recurrent_families"),
            "recurrent_fraction": org.get("recurrence_fraction"),
            "active_families_at_end": org.get("families_active_at_end"),
            "compound_identity_count": org.get("molecules_with_multi_resource_or_bonded_structure"),
            "top_structural_families": top_structural_families,
            "mean_observed_lifetime_ticks": nested(organization, "persistence", "mean_observed_lifetime_ticks"),
            "median_observed_lifetime_ticks": nested(organization, "persistence", "median_observed_lifetime_ticks"),
            "compound_material_by_resource": scalar_tree(nested(
                organization, "material_distribution", "compound_constituent_amount_by_resource", default={}
            )),
            "reaction_created_compound_material_by_resource": scalar_tree(nested(
                organization, "material_distribution",
                "reaction_created_compound_constituent_amount_by_resource", default={}
            )),
            "family_transition_summary": transition_summary,
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
            "partial_or_unknown_coverage": partial_or_unknown_coverage,
            "missing_fields": [
                field for field, value in {
                    "configuration_sha256": conditions.get("configuration_sha256"),
                    "actual_elapsed_wall_seconds": run.get("actual_elapsed_wall_seconds"),
                    "opportunity_coverage": seed_coverage,
                    "chemistry_lifecycle_validation": lifecycle_errors,
                    "organization_lifecycle_validation": organization_errors,
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
    if partial_or_unknown_coverage:
        record["quality"]["warnings"].append(
            "One or more reaction opportunity searches have partial or unknown coverage."
        )
    return record


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--seed", type=int, required=True)
    parser.add_argument("--chemistry", type=Path, required=True)
    parser.add_argument("--organization", type=Path, required=True)
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--opportunity-coverage", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    record = build_record(args.seed, args.chemistry, args.organization, args.manifest, args.opportunity_coverage)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(record, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    print(f"Compact permanent record: {args.output} ({args.output.stat().st_size} bytes)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
