#!/usr/bin/env python3
"""Validate an EvoSim experiment manifest and its raw artifact references."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
from pathlib import Path
from typing import Any


SHA256_PATTERN = re.compile(r"^[0-9a-f]{64}$")


def validate_manifest(path: Path, verify_artifacts: bool = True) -> dict[str, Any]:
    manifest = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(manifest, dict):
        raise ValueError("manifest root must be an object")
    if manifest.get("schema") != "evosim.experiment_manifest":
        raise ValueError("unsupported manifest schema")
    if manifest.get("schema_version") != 1:
        raise ValueError("unsupported manifest schema_version")

    run = manifest.get("run")
    if not isinstance(run, dict):
        raise ValueError("run must be an object")
    if run.get("status") not in {"completed", "running", "failed", "interrupted"}:
        raise ValueError("run.status is missing or invalid")
    started = run.get("started_at_unix_ms")
    completed = run.get("completed_at_unix_ms")
    elapsed = run.get("actual_elapsed_wall_seconds")
    if not isinstance(started, int) or started < 0:
        raise ValueError("run.started_at_unix_ms must be a non-negative integer")
    if completed is not None and (not isinstance(completed, int) or completed < started):
        raise ValueError("run.completed_at_unix_ms must be >= start time")
    if elapsed is not None and (
        not isinstance(elapsed, (int, float)) or not math.isfinite(elapsed) or elapsed < 0
    ):
        raise ValueError("run.actual_elapsed_wall_seconds must be finite and non-negative")
    start_tick, end_tick = run.get("start_tick"), run.get("end_tick")
    if not isinstance(start_tick, int) or not isinstance(end_tick, int) or end_tick < start_tick:
        raise ValueError("run tick range is missing or invalid")

    conditions = manifest.get("conditions")
    if not isinstance(conditions, dict) or conditions.get("configuration_schema_version") != 1:
        raise ValueError("conditions.configuration_schema_version must be 1")
    if not SHA256_PATTERN.fullmatch(str(conditions.get("configuration_sha256", ""))):
        raise ValueError("conditions.configuration_sha256 must be a lowercase SHA-256 digest")
    if not isinstance(conditions.get("configuration"), dict):
        raise ValueError("conditions.configuration must be an object")

    artifacts = manifest.get("artifacts")
    if not isinstance(artifacts, list) or not artifacts:
        raise ValueError("manifest must reference at least one artifact")
    for artifact in artifacts:
        if not isinstance(artifact, dict):
            raise ValueError("artifact entries must be objects")
        artifact_path = artifact.get("path")
        expected_size = artifact.get("size_bytes")
        expected_hash = artifact.get("sha256")
        if not isinstance(artifact_path, str) or not artifact_path:
            raise ValueError("artifact.path is missing")
        if not isinstance(expected_size, int) or expected_size < 0:
            raise ValueError(f"artifact {artifact_path}: invalid size_bytes")
        if not SHA256_PATTERN.fullmatch(str(expected_hash or "")):
            raise ValueError(f"artifact {artifact_path}: invalid sha256")
        if verify_artifacts:
            resolved = Path(artifact_path)
            if not resolved.is_absolute():
                resolved = path.parent / resolved
            if not resolved.is_file():
                raise ValueError(f"artifact does not exist: {resolved}")
            if resolved.stat().st_size != expected_size:
                raise ValueError(f"artifact size mismatch: {resolved}")
            digest = hashlib.sha256()
            with resolved.open("rb") as stream:
                for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                    digest.update(chunk)
            if digest.hexdigest() != expected_hash:
                raise ValueError(f"artifact checksum mismatch: {resolved}")

    quality = manifest.get("quality")
    if not isinstance(quality, dict) or not isinstance(quality.get("warnings"), list):
        raise ValueError("quality.warnings must be an array")
    return manifest


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("--skip-artifact-check", action="store_true")
    args = parser.parse_args()
    manifest = validate_manifest(args.manifest, verify_artifacts=not args.skip_artifact_check)
    run = manifest["run"]
    print(
        f"VALID manifest schema=v{manifest['schema_version']} "
        f"status={run['status']} ticks={run['start_tick']}..{run['end_tick']} "
        f"artifacts={len(manifest['artifacts'])}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
