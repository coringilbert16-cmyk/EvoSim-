#!/usr/bin/env python3
"""Finalize temporary per-seed OBS-001 records into exactly two permanent files."""
from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path
from typing import Any

from tools.compare_compact_experiment_records import (
    compare,
    measurement_compatibility_key,
)

SETUP_SCHEMA = "evosim.experiment_setup"
REPORT_SCHEMA = "evosim.consolidated_experiment_report"
REPORT_VERSION = 1
PERMANENT_RECORD_SCHEMA = "evosim.permanent_experiment_record"


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def validate_stage_outputs(stage: dict[str, Any], stage_id: str, index_path: Path) -> None:
    outputs = stage.get("outputs")
    hashes = stage.get("output_sha256")
    if not isinstance(outputs, list) or not outputs or not isinstance(hashes, dict):
        raise ValueError(f"required analysis stage {stage_id} has no checksummed outputs")
    root = index_path.parent.resolve()
    for relative in outputs:
        if not isinstance(relative, str) or not relative:
            raise ValueError(f"required analysis stage {stage_id} has an invalid output path")
        output_path = (root / relative).resolve()
        try:
            output_path.relative_to(root)
        except ValueError as exc:
            raise ValueError(f"required analysis stage {stage_id} output escapes the analysis directory") from exc
        expected = hashes.get(relative)
        if not isinstance(expected, str) or len(expected) != 64:
            raise ValueError(f"required analysis stage {stage_id} output lacks a SHA-256 digest: {relative}")
        if not output_path.is_file():
            raise ValueError(f"required analysis stage {stage_id} output is missing: {relative}")
        if sha256_file(output_path) != expected:
            raise ValueError(f"required analysis stage {stage_id} output checksum mismatch: {relative}")


def load_record(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict) or value.get("schema") != PERMANENT_RECORD_SCHEMA:
        raise ValueError(f"{path}: unsupported temporary per-seed record")
    if value.get("schema_version") != 1:
        raise ValueError(f"{path}: unsupported temporary record version")
    if value.get("record_status") != "complete":
        raise ValueError(f"{path}: per-seed analysis is not complete")
    identity = value.get("identity")
    if not isinstance(identity, dict) or not isinstance(identity.get("seed"), int):
        raise ValueError(f"{path}: missing seed identity")
    return value


def finalize(paths: list[Path], expected_seeds: list[int], analysis_index_path: Path) -> tuple[dict[str, Any], dict[str, Any]]:
    if not paths:
        raise ValueError("no temporary per-seed records supplied")
    loaded = [(load_record(path), path) for path in paths]
    loaded.sort(key=lambda item: item[0]["identity"]["seed"])
    records = [row for row, _ in loaded]
    paths = [path for _, path in loaded]
    seeds = [row["identity"]["seed"] for row in records]
    if len(seeds) != len(set(seeds)):
        raise ValueError("duplicate seed record")
    if set(seeds) != set(expected_seeds):
        raise ValueError(
            f"seed coverage mismatch: expected {sorted(expected_seeds)}, received {sorted(seeds)}"
        )
    records.sort(key=lambda row: row["identity"]["seed"])
    first = records[0]
    identity = first["identity"]
    provenance = first["provenance"]
    conditions = first["conditions"]
    requested_duration = conditions.get("requested_duration_seconds")
    diagnostic_interval = conditions.get("diagnostic_interval")
    if not provenance.get("workflow_run_id") or not provenance.get("workflow_run_url"):
        raise ValueError("validated experiment records must identify their source workflow run")
    canonical_configuration = dict(conditions.get("configuration") or {})
    canonical_configuration.pop("seed", None)
    required_configuration_fields = (
        "initial_material_inventory", "initial_physical_material_count",
        "initial_independent_resource_count", "initial_molecule_count",
        "initial_constituent_count", "pile_sizes", "hotspot", "world_geometry",
        "flow_field", "deep_diagnostics_enabled", "profile_movement_enabled",
        "profile_tick_enabled", "biology_enabled",
    )
    missing_configuration = [
        field for field in required_configuration_fields
        if canonical_configuration.get(field) is None
    ]
    if missing_configuration:
        raise ValueError(
            "reproducibility setup is missing required environment/configuration fields: "
            + ", ".join(missing_configuration)
        )
    if not isinstance(canonical_configuration["initial_material_inventory"], dict):
        raise ValueError("initial material inventory must be a structured mapping")
    inventory_values = list(canonical_configuration["initial_material_inventory"].values())
    if (not inventory_values or any(not isinstance(value, (int, float)) or isinstance(value, bool)
                                    or not math.isfinite(value) or value < 0
                                    for value in inventory_values)):
        raise ValueError("initial material inventory must contain finite nonnegative quantities")
    if not isinstance(canonical_configuration["pile_sizes"], list):
        raise ValueError("environment pile sizes must be recorded as a list")
    if not isinstance(canonical_configuration["world_geometry"], dict):
        raise ValueError("world geometry must be recorded as a structured mapping")
    if not isinstance(canonical_configuration["flow_field"], dict):
        raise ValueError("ambient flow configuration must be recorded as a structured mapping")
    if not isinstance(canonical_configuration["hotspot"], dict):
        raise ValueError("hotspot parameters must be recorded as a structured mapping")
    count_fields = (
        "initial_physical_material_count", "initial_independent_resource_count",
        "initial_molecule_count", "initial_constituent_count",
    )
    if any(not isinstance(canonical_configuration[field], int)
           or isinstance(canonical_configuration[field], bool)
           or canonical_configuration[field] < 0 for field in count_fields):
        raise ValueError("initial inventory counts must be nonnegative integers")
    if (canonical_configuration["initial_independent_resource_count"]
            + canonical_configuration["initial_molecule_count"]
            != canonical_configuration["initial_physical_material_count"]):
        raise ValueError("initial resource and molecule counts do not reconcile to physical inventory")
    if not math.isclose(sum(inventory_values), canonical_configuration["initial_constituent_count"],
                        rel_tol=0.0, abs_tol=1e-6):
        raise ValueError("initial elemental inventory does not reconcile to constituent count")
    if (any(not isinstance(size, int) or isinstance(size, bool) or size <= 0
            for size in canonical_configuration["pile_sizes"])
            or sum(canonical_configuration["pile_sizes"])
            != canonical_configuration["initial_physical_material_count"]):
        raise ValueError("initial pile sizes do not reconcile to physical inventory")
    geometry = canonical_configuration["world_geometry"]
    for field in ("width", "height", "cell_size"):
        value = geometry.get(field)
        if not isinstance(value, (int, float)) or isinstance(value, bool) or value <= 0:
            raise ValueError(f"world geometry requires a positive numeric {field}")
    for field in ("width_cells", "height_cells"):
        value = geometry.get(field)
        if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
            raise ValueError(f"world geometry requires a positive integer {field}")
    flow = canonical_configuration["flow_field"]
    for field in ("direction_degrees", "speed", "strength", "vortex_strength"):
        value = flow.get(field)
        if not isinstance(value, (int, float)) or isinstance(value, bool) or not math.isfinite(value):
            raise ValueError(f"ambient flow configuration requires a finite numeric {field}")
    for field in ("deep_diagnostics_enabled", "profile_movement_enabled", "profile_tick_enabled"):
        if not isinstance(canonical_configuration[field], bool):
            raise ValueError(f"{field} must be recorded as a boolean")
    if canonical_configuration["biology_enabled"] is not False:
        raise ValueError("environmental chemistry setup must explicitly record biology as disabled")
    if requested_duration is None or diagnostic_interval is None:
        raise ValueError("requested duration and diagnostic interval are required for reproduction")
    if (not isinstance(requested_duration, int) or isinstance(requested_duration, bool)
            or requested_duration <= 0 or not isinstance(diagnostic_interval, int)
            or isinstance(diagnostic_interval, bool) or diagnostic_interval <= 0):
        raise ValueError("requested duration and diagnostic interval must be positive integers")
    for row in records[1:]:
        other = row["identity"]
        if (other.get("experiment_id"), other.get("research_plan"), other.get("condition_id")) != (
            identity.get("experiment_id"), identity.get("research_plan"), identity.get("condition_id")
        ):
            raise ValueError("records contain multiple experiment/plan/condition identities")
        if measurement_compatibility_key(row) != measurement_compatibility_key(first):
            raise ValueError("records have incompatible measurement definitions")
        if (row.get("provenance", {}).get("workflow_run_id"),
                row.get("provenance", {}).get("workflow_run_url")) != (
                provenance.get("workflow_run_id"), provenance.get("workflow_run_url")):
            raise ValueError("records originate from different workflow runs")
        other_configuration = dict(row.get("conditions", {}).get("configuration") or {})
        other_configuration.pop("seed", None)
        if other_configuration != canonical_configuration:
            raise ValueError("records have different experiment configurations")
    index = json.loads(analysis_index_path.read_text(encoding="utf-8"))
    if not isinstance(index, dict) or index.get("schema") != "evosim.unified_chemistry_analysis":
        raise ValueError("missing or invalid unified analysis index")
    if (index.get("experiment_id"), index.get("research_plan"), index.get("condition_id")) != (
        identity.get("experiment_id"), identity.get("research_plan"), identity.get("condition_id")
    ):
        raise ValueError("unified analysis index identity does not match the seed records")
    index_runs = {
        row.get("seed"): row for row in index.get("runs", []) if isinstance(row, dict)
    }
    if len(index_runs) != len(index.get("runs", [])) or set(index_runs) != set(expected_seeds):
        raise ValueError("unified analysis index seed coverage does not match the requested seeds")
    for record in records:
        seed = record["identity"]["seed"]
        indexed_run = index_runs[seed]
        if indexed_run.get("manifest_status") != "manifest_validated":
            raise ValueError(f"unified analysis did not validate the manifest for seed {seed}")
        expected_raw_hash = record.get("provenance", {}).get("raw_diagnostics_sha256")
        if not expected_raw_hash or indexed_run.get("raw_sha256") != expected_raw_hash:
            raise ValueError(f"unified analysis raw-input checksum does not match seed {seed} record")
    if not index.get("quality", {}).get("all_runs_have_validated_manifests", False):
        raise ValueError("one or more run manifests were not validated")
    stages = index.get("stages", [])
    stage_rows = {row.get("stage"): row for row in stages if isinstance(row, dict)}
    stage_status = {stage_id: row.get("status") for stage_id, row in stage_rows.items()}
    required_stages = ("chemistry", "organization", "experiment_batch")
    failed_required = [stage for stage in required_stages if stage_status.get(stage) != "completed"]
    if failed_required:
        raise ValueError("required unified analysis stages did not complete: " + ", ".join(failed_required))
    for stage_id in required_stages:
        validate_stage_outputs(stage_rows[stage_id], stage_id, analysis_index_path)
    comparison = compare(paths)
    if len(comparison.get("condition_groups", [])) != 1:
        raise ValueError("cross-seed comparison did not produce exactly one condition group")
    if comparison.get("measurement_definition_missing_records"):
        raise ValueError("required measurement definitions are missing; refusing finalization")

    deep_diagnostics = canonical_configuration.get("deep_diagnostics_enabled") is True
    profile_movement = canonical_configuration.get("profile_movement_enabled") is True
    profile_tick = canonical_configuration.get("profile_tick_enabled") is True
    simulator_arguments_by_seed = []
    for row in records:
        seed = row["identity"]["seed"]
        arguments = [
            "--headless",
            "--environment-chemistry-experiment",
            "--seconds", str(requested_duration),
            "--seed", str(seed),
            "--diagnostics", f"observations/seed-{seed}.jsonl",
            "--diagnostic-interval", str(diagnostic_interval),
        ]
        if deep_diagnostics:
            arguments.append("--deep-diagnostics")
        simulator_arguments_by_seed.append({
            "seed": seed,
            "command": ["cargo", "run", "--locked", "--", *arguments],
        })

    setup = {
        "schema": SETUP_SCHEMA,
        "schema_version": 1,
        "experiment": {
            "experiment_id": identity.get("experiment_id"),
            "research_plan": identity.get("research_plan"),
            "condition_id": identity.get("condition_id"),
        },
        "source": {
            "repository_commit": provenance.get("source_commit"),
            "simulator_version": provenance.get("simulator_version"),
            "workflow_run_id": provenance.get("workflow_run_id"),
            "workflow_run_url": provenance.get("workflow_run_url"),
        },
        "configuration": canonical_configuration,
        "simulator_invocation": {
            "workflow_file": ".github/workflows/chemistry-experiments.yml",
            "toolchain_selector": "stable",
            "arguments_by_seed": simulator_arguments_by_seed,
            "environment_by_seed": [
                {
                    "seed": row["identity"]["seed"],
                    "EVOSIM_PROFILE_MOVEMENT": profile_movement,
                    "EVOSIM_PROFILE_TICK": profile_tick,
                    "EVOSIM_EXPERIMENT_ID": identity.get("experiment_id"),
                    "EVOSIM_RESEARCH_PLAN": identity.get("research_plan"),
                    "EVOSIM_CONDITION_ID": identity.get("condition_id"),
                }
                for row in records
            ],
        },
        "configuration_sha256": provenance.get("comparison_configuration_sha256"),
        "comparison_configuration_sha256": provenance.get("comparison_configuration_sha256"),
        "manifest_schema_version": provenance.get("manifest_schema_version"),
        "enabled_diagnostic_tiers": provenance.get("enabled_diagnostic_tiers"),
        "seeds": sorted(seeds),
        "replicate_count": len(seeds),
        "conditions_common": {
            **{
                key: value for key, value in conditions.items()
                if key not in ("actual_elapsed_wall_seconds", "start_tick", "end_tick", "tick_count", "configuration")
            },
            "configuration": canonical_configuration,
        },
        "execution_by_seed": [
            {
                "seed": row["identity"]["seed"],
                "requested_duration_seconds": row["conditions"].get("requested_duration_seconds"),
                "actual_elapsed_wall_seconds": row["conditions"].get("actual_elapsed_wall_seconds"),
                "start_tick": row["conditions"].get("start_tick"),
                "end_tick": row["conditions"].get("end_tick"),
                "tick_count": row["conditions"].get("tick_count"),
                "diagnostic_interval": row["conditions"].get("diagnostic_interval"),
                "configuration_sha256": row["provenance"].get("configuration_sha256"),
            }
            for row in records
        ],
        "setup_validation": "passed",
    }
    report = {
        "schema": REPORT_SCHEMA,
        "schema_version": REPORT_VERSION,
        "status": "validated",
        "experiment": setup["experiment"],
        "setup_file": "setup.json",
        "analysis": {
            "analyzer_fingerprints": {
                "chemistry": provenance.get("chemistry_analyzer_sha256"),
                "organization": provenance.get("organization_analyzer_sha256"),
            },
            "per_seed_results": records,
            "cross_seed_comparison": comparison,
        },
        "quality": {
            "validated_seed_count": len(records),
            "expected_seed_count": len(expected_seeds),
            "all_seed_records_complete": True,
            "measurement_definitions_complete": True,
            "pipeline_stages": [
                {"stage": row.get("stage"), "status": row.get("status"),
                 "analyzer": row.get("analyzer"), "analyzer_sha256": row.get("analyzer_sha256"),
                 "output_sha256": row.get("output_sha256")}
                for row in stages if isinstance(row, dict)
            ],
            "optional_stage_warnings": [f"{stage}: {status}" for stage, status in stage_status.items() if stage not in required_stages and status not in ("completed", "deferred_heavy", "deferred_deep")],
            "limitations": [
                "Seeds are independent replicates; reaction events within a seed are not independent samples.",
                "Rates require complete, semantically compatible denominators.",
                "Unknown and partial opportunity coverage do not support complete conversion-rate claims.",
            ],
        },
    }
    return setup, report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--records", nargs="+", type=Path, required=True)
    parser.add_argument("--expected-seeds", required=True, help="comma-separated requested seed IDs")
    parser.add_argument("--analysis-index", type=Path, required=True)
    parser.add_argument("--setup-out", type=Path, required=True)
    parser.add_argument("--report-out", type=Path, required=True)
    args = parser.parse_args()
    seeds = [int(value.strip()) for value in args.expected_seeds.split(",") if value.strip()]
    if not seeds or len(seeds) != len(set(seeds)):
        parser.error("--expected-seeds must contain unique seed IDs")
    setup, report = finalize(args.records, seeds, args.analysis_index)
    args.setup_out.parent.mkdir(parents=True, exist_ok=True)
    args.report_out.parent.mkdir(parents=True, exist_ok=True)
    args.setup_out.write_text(json.dumps(setup, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    args.report_out.write_text(json.dumps(report, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    print(f"Validated setup: {args.setup_out}")
    print(f"Consolidated report: {args.report_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
