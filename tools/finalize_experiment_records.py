#!/usr/bin/env python3
"""Finalize temporary per-seed OBS-001 records into exactly two permanent files."""
from __future__ import annotations

import argparse
import json
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


def finalize(paths: list[Path], expected_seeds: list[int]) -> tuple[dict[str, Any], dict[str, Any]]:
    if not paths:
        raise ValueError("no temporary per-seed records supplied")
    records = [load_record(path) for path in paths]
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
    for row in records[1:]:
        other = row["identity"]
        if (other.get("experiment_id"), other.get("research_plan"), other.get("condition_id")) != (
            identity.get("experiment_id"), identity.get("research_plan"), identity.get("condition_id")
        ):
            raise ValueError("records contain multiple experiment/plan/condition identities")
        if measurement_compatibility_key(row) != measurement_compatibility_key(first):
            raise ValueError("records have incompatible measurement definitions")
        if row.get("conditions", {}).get("configuration") != conditions.get("configuration"):
            raise ValueError("records have different experiment configurations")
    comparison = compare(paths)
    if len(comparison.get("condition_groups", [])) != 1:
        raise ValueError("cross-seed comparison did not produce exactly one condition group")
    if comparison.get("measurement_definition_missing_records"):
        raise ValueError("required measurement definitions are missing; refusing finalization")

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
        "configuration": conditions.get("configuration"),
        "configuration_sha256": provenance.get("configuration_sha256"),
        "comparison_configuration_sha256": provenance.get("comparison_configuration_sha256"),
        "seeds": sorted(seeds),
        "replicate_count": len(seeds),
        "conditions_common": {
            key: value for key, value in conditions.items()
            if key not in ("actual_elapsed_wall_seconds", "start_tick", "end_tick", "tick_count")
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
        "setup_file": "experiment-setup.json",
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
    parser.add_argument("--setup-out", type=Path, required=True)
    parser.add_argument("--report-out", type=Path, required=True)
    args = parser.parse_args()
    seeds = [int(value.strip()) for value in args.expected_seeds.split(",") if value.strip()]
    if not seeds or len(seeds) != len(set(seeds)):
        parser.error("--expected-seeds must contain unique seed IDs")
    setup, report = finalize(args.records, seeds)
    args.setup_out.parent.mkdir(parents=True, exist_ok=True)
    args.report_out.parent.mkdir(parents=True, exist_ok=True)
    args.setup_out.write_text(json.dumps(setup, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    args.report_out.write_text(json.dumps(report, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    print(f"Validated setup: {args.setup_out}")
    print(f"Consolidated report: {args.report_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
