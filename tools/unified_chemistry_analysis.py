#!/usr/bin/env python3
"""Offline orchestrator for reproducible environmental chemistry analysis."""
from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from tools.experiment_manifest import validate_manifest  # noqa: E402
from tools.compare_compact_experiment_records import (  # noqa: E402
    compare as compare_compact_records,
    render_markdown as render_compact_comparison_markdown,
)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def manifest_for(raw: Path) -> Path | None:
    candidate = Path(str(raw) + ".manifest.json")
    return candidate if candidate.is_file() else None


def validate_run(input_dir: Path, seed: int) -> dict[str, Any]:
    raw = input_dir / f"seed-{seed}.jsonl"
    if not raw.is_file():
        raise FileNotFoundError(f"missing raw diagnostics for seed {seed}: {raw}")
    manifest_path = manifest_for(raw)
    if manifest_path:
        manifest = validate_manifest(manifest_path)
        exp = manifest.get("experiment", {})
        if exp.get("seed") != seed:
            raise ValueError(f"{manifest_path}: manifest seed does not match requested seed {seed}")
        artifact = next((a for a in manifest.get("artifacts", []) if a.get("role") == "raw_observation"), None)
        if artifact is None:
            raise ValueError(f"{manifest_path}: no raw_observation artifact")
        resolved_artifact = (manifest_path.parent / artifact["path"]).resolve()
        if resolved_artifact != raw.resolve():
            raise ValueError(f"{manifest_path}: raw artifact path does not resolve to {raw}")
        checksum = artifact["sha256"]
        provenance = "manifest_validated"
    else:
        checksum = sha256_file(raw)
        manifest = None
        provenance = "legacy_manifest_missing"
    return {
        "seed": seed,
        "raw_path": raw.name,
        "_raw_resolved_path": str(raw.resolve()),
        "raw_sha256": checksum,
        "manifest_path": manifest_path.name if manifest_path else None,
        "manifest_sha256": sha256_file(manifest_path) if manifest_path else None,
        "manifest_status": provenance,
        "manifest": manifest,
    }


def stage_cache_valid(metadata_path: Path, output_path: Path, input_sha: str, analyzer_sha: str) -> bool:
    if not metadata_path.is_file() or not output_path.is_file():
        return False
    try:
        metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
        return (
            metadata.get("input_sha256") == input_sha
            and metadata.get("analyzer_sha256") == analyzer_sha
            and metadata.get("output_sha256") == sha256_file(output_path)
        )
    except (OSError, ValueError, TypeError):
        return False


def run_cached_stage(
    stage_id: str,
    script: Path,
    raw: Path,
    seed: int,
    output_dir: Path,
    input_sha: str,
) -> dict[str, Any]:
    output_dir.mkdir(parents=True, exist_ok=True)
    output_json = output_dir / f"seed-{seed}-{stage_id}.json"
    output_md = output_dir / f"seed-{seed}-{stage_id}.md"
    metadata_path = output_dir / f"seed-{seed}-{stage_id}.stage.json"
    analyzer_sha = sha256_file(script)
    if output_md.is_file() and stage_cache_valid(metadata_path, output_json, input_sha, analyzer_sha):
        return {
            "stage": stage_id, "status": "cached", "analyzer": str(script.relative_to(ROOT)),
            "analyzer_sha256": analyzer_sha, "outputs": [str(output_json), str(output_md)],
        }
    # A stale prior result must never survive a failed recomputation.
    output_json.unlink(missing_ok=True)
    output_md.unlink(missing_ok=True)
    metadata_path.unlink(missing_ok=True)
    command = [
        sys.executable, str(script), str(raw),
        "--json-out", str(output_json), "--markdown-out", str(output_md),
    ]
    result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
    if result.returncode != 0:
        return {
            "stage": stage_id, "status": "failed", "analyzer": str(script.relative_to(ROOT)),
            "analyzer_sha256": analyzer_sha, "error": (result.stderr or result.stdout)[-4000:],
        }
    metadata = {
        "schema": "evosim.analysis_stage_cache", "schema_version": 1,
        "stage": stage_id, "input_sha256": input_sha, "analyzer_sha256": analyzer_sha,
        "output_sha256": sha256_file(output_json),
    }
    metadata_path.write_text(json.dumps(metadata, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    return {
        "stage": stage_id, "status": "completed", "analyzer": str(script.relative_to(ROOT)),
        "analyzer_sha256": analyzer_sha, "outputs": [str(output_json), str(output_md)],
    }


def run_group_stage(stage_id: str, script_name: str, workspace: Path, seeds: list[int],
                    duration: int, output_dir: Path, output_files: list[Path],
                    extra: list[str] | None = None) -> dict[str, Any]:
    script = ROOT / "tools" / script_name
    output_dir.mkdir(parents=True, exist_ok=True)
    command = [sys.executable, str(script), str(workspace), "--seeds",
               ",".join(map(str, seeds)), "--duration-seconds", str(duration)]
    if stage_id == "experiment_batch":
        command += ["--json-out", str(output_files[0]), "--markdown-out", str(output_files[1])]
    else:
        command += ["--output-dir", str(output_dir)]
        if extra:
            command += extra
    result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
    status = "completed" if result.returncode == 0 else "failed"
    outputs = [str(path) for path in output_files if path.is_file()]
    if output_dir.is_dir():
        outputs.extend(str(path) for path in sorted(output_dir.rglob("*")) if path.is_file())
    row: dict[str, Any] = {
        "stage": stage_id, "status": status, "analyzer": str(script.relative_to(ROOT)),
        "analyzer_sha256": sha256_file(script), "outputs": outputs,
    }
    if result.returncode != 0:
        row["error"] = (result.stderr or result.stdout)[-4000:]
    return row


def compose_report(index: dict[str, Any], batch_path: Path | None) -> str:
    lines = [
        "# Unified environmental chemistry analysis", "",
        f"- Experiment: {index['experiment_id'] or 'unregistered/legacy'}",
        f"- Plan: {index['research_plan'] or 'not specified'}",
        f"- Condition: {index['condition_id'] or 'not specified'}",
        f"- Seeds: {', '.join(str(row['seed']) for row in index['runs'])}",
        f"- Requested duration per seed: {index['duration_seconds']} seconds",
        "",
        "## Source validation", "",
        "| Seed | Manifest | Raw SHA-256 |",
        "|---:|---|---|",
    ]
    for run in index["runs"]:
        lines.append(f"| {run['seed']} | {run['manifest_status']} | {run['raw_sha256']} |")
    lines += ["", "## Pipeline stages", "", "| Stage | Status |", "|---|---|"]
    for stage in index["stages"]:
        lines.append(f"| {stage['stage']} | {stage['status']} |")
    if batch_path and batch_path.is_file():
        try:
            batch = json.loads(batch_path.read_text(encoding="utf-8"))
            totals = batch.get("reaction_totals", {})
            lines += ["", "## Reaction totals (seed-level aggregate)", "",
                      "| Reaction | Ready | Scheduled | Committed | Rejected |",
                      "|---|---:|---:|---:|---:|"]
            for reaction, values in totals.items():
                lines.append(f"| {reaction} | {values.get('ready')} | {values.get('scheduled')} | "
                             f"{values.get('committed')} | {values.get('rejected')} |")
        except (OSError, ValueError):
            lines += ["", "Batch report exists but could not be parsed; inspect the linked JSON output."]
    lines += [
        "", "## Interpretation and data-quality limits", "",
        "- Seed-level results are retained; independent seeds are the experimental replicates.",
        "- Missing manifest metadata is labeled legacy and is not inferred.",
        "- Opportunity rates are only valid where the source analyzer reports a complete, semantically appropriate denominator.",
        "- Contact-opportunity detailed CSV generation is opt-in because retained diagnostic datasets can be very large.",
        "- Network cycles, family recurrence, and neighborhood associations are descriptive; they do not establish catalysis or self-sustaining chemistry.",
        "- This pipeline is offline analysis only and does not start or modify simulations.",
    ]
    return "\n".join(lines) + "\n"


def analyze(input_dir: Path, seeds: list[int], duration: int, output_dir: Path,
            include_heavy_contact: bool = False, include_deep_analysis: bool = False) -> dict[str, Any]:
    input_dir = input_dir.resolve()
    output_dir = output_dir.resolve()
    output_dir.mkdir(parents=True, exist_ok=True)
    runs = [validate_run(input_dir, seed) for seed in seeds]
    experiment_id = research_plan = condition_id = None
    manifests = [row["manifest"] for row in runs if row["manifest"]]
    if manifests:
        identities = []
        for manifest in manifests:
            experiment = manifest.get("experiment", {})
            identities.append((experiment.get("experiment_id"), experiment.get("research_plan"),
                               experiment.get("condition_id")))
        if len(set(identities)) != 1:
            raise ValueError("one invocation must contain a single experiment/plan/condition group")
        experiment_id, research_plan, condition_id = identities[0]
    workspace = output_dir / "normalized-input"
    workspace.mkdir(parents=True, exist_ok=True)
    per_seed_dir = output_dir / "per-seed"
    stages: list[dict[str, Any]] = []
    for row in runs:
        seed = row["seed"]
        raw = Path(row["_raw_resolved_path"])
        link = workspace / f"seed-{seed}.jsonl"
        if link.exists() or link.is_symlink():
            link.unlink()
        link.symlink_to(raw)
        for stage_id, script in (
            ("chemistry", ROOT / "tools/chemistry_analyzer.py"),
            ("organization", ROOT / "tools/chemistry_organization_analyzer.py"),
        ):
            stages.append(run_cached_stage(stage_id, script, raw, seed, per_seed_dir, row["raw_sha256"]))
            source_json = per_seed_dir / f"seed-{seed}-{stage_id}.json"
            expected = workspace / f"seed-{seed}-{stage_id}.json"
            if source_json.is_file():
                if expected.exists() or expected.is_symlink():
                    expected.unlink()
                expected.symlink_to(source_json.resolve())
        if any(stage.get("status") == "failed" for stage in stages[-2:]):
            continue
    required_json = [workspace / f"seed-{seed}-{suffix}.json"
                     for seed in seeds for suffix in ("chemistry", "organization")]
    if any(not path.is_file() for path in required_json):
        raise RuntimeError("per-seed chemistry/organization analysis failed; inspect stage records")
    batch_json = output_dir / "experiment-batch.json"
    batch_md = output_dir / "experiment-batch.md"
    stages.append(run_group_stage("experiment_batch", "chemistry_experiment_batch_analyzer.py",
                                  workspace, seeds, duration, output_dir,
                                  [batch_json, batch_md]))
    if include_deep_analysis:
        stages.append(run_group_stage("reaction_network", "reaction_network_analyzer.py",
                                      workspace, seeds, duration, output_dir / "reaction-network", []))
        stages.append(run_group_stage("geometry_network", "geometry_aware_reaction_network.py",
                                      workspace, seeds, duration, output_dir / "geometry-aware-network", []))
        stages.append(run_group_stage("temporal_complexity", "temporal_chemical_complexity_analyzer.py",
                                      workspace, seeds, duration, output_dir / "temporal-complexity", [],
                                      ["--windows", "12"]))
    else:
        for stage_id in ("reaction_network", "geometry_network", "temporal_complexity"):
            stages.append({"stage": stage_id, "status": "deferred_deep",
                           "reason": "deep analysis not requested"})
    if include_heavy_contact:
        stages.append(run_group_stage("contact_opportunities", "contact_opportunity_analyzer.py",
                                      workspace, seeds, duration, output_dir / "contact-opportunities", []))
    else:
        stages.append({
            "stage": "contact_opportunities", "status": "deferred_heavy",
            "reason": "Detailed sample CSV is opt-in due to potentially multi-gigabyte output; coverage/funnel counters remain in core chemistry reports.",
        })
    # Store derived artifact paths relative to the index, with checksums for each
    # completed output. This makes the index portable when the analysis folder moves.
    for stage in stages:
        portable_outputs = []
        output_hashes = {}
        for value in stage.get("outputs", []):
            path = Path(value).resolve()
            if path.is_file():
                relative = str(path.relative_to(output_dir.resolve()))
                portable_outputs.append(relative)
                output_hashes[relative] = sha256_file(path)
        stage["outputs"] = portable_outputs
        if output_hashes:
            stage["output_sha256"] = output_hashes
    index = {
        "schema": "evosim.unified_chemistry_analysis",
        "schema_version": 1,
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "experiment_id": experiment_id, "research_plan": research_plan, "condition_id": condition_id,
        "duration_seconds": duration,
        "runs": [{key: value for key, value in row.items()
                  if key not in ("manifest", "_raw_resolved_path")} for row in runs],
        "stages": stages,
        "quality": {
            "all_runs_have_validated_manifests": all(row["manifest_status"] == "manifest_validated" for row in runs),
            "failed_stages": [row["stage"] for row in stages if row["status"] == "failed"],
            "deferred_stages": [row["stage"] for row in stages if row["status"] in ("deferred_heavy", "deferred_deep")],
            "warnings": ["Historical runs without manifests are checksum-verified but lack full run provenance."]
            if any(row["manifest_status"] != "manifest_validated" for row in runs) else [],
        },
    }
    index_path = output_dir / "unified-analysis-index.json"
    index_path.write_text(json.dumps(index, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    report_path = output_dir / "unified-chemistry-report.md"
    report_path.write_text(compose_report(index, batch_json), encoding="utf-8")
    index["report_path"] = str(report_path)
    index_path.write_text(json.dumps(index, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    return index


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input_dir", nargs="?", type=Path, help="directory containing seed-N.jsonl and optional manifests")
    parser.add_argument("--seeds", help="comma-separated independent seed IDs")
    parser.add_argument("--duration-seconds", type=int)
    parser.add_argument("--output-dir", type=Path)
    parser.add_argument("--compare-records", nargs="+", type=Path,
                        help="compare compact permanent records without raw diagnostics")
    parser.add_argument("--comparison-json-out", type=Path,
                        help="JSON output path for compact-record-only comparison")
    parser.add_argument("--comparison-markdown-out", type=Path,
                        help="optional Markdown output path for compact-record-only comparison")
    parser.add_argument("--include-heavy-contact", action="store_true",
                        help="run detailed contact CSV analysis; may produce very large files")
    parser.add_argument("--deep-analysis", action="store_true",
                        help="run expanded reaction-network, geometry, and temporal analyzers")
    args = parser.parse_args()
    if args.compare_records:
        if args.input_dir or args.seeds or args.duration_seconds is not None or args.output_dir:
            parser.error("--compare-records cannot be combined with raw-input analysis arguments")
        if not args.comparison_json_out:
            parser.error("--comparison-json-out is required with --compare-records")
        result = compare_compact_records(args.compare_records)
        args.comparison_json_out.parent.mkdir(parents=True, exist_ok=True)
        args.comparison_json_out.write_text(
            json.dumps(result, sort_keys=True, indent=2) + "\n", encoding="utf-8"
        )
        if args.comparison_markdown_out:
            args.comparison_markdown_out.parent.mkdir(parents=True, exist_ok=True)
            args.comparison_markdown_out.write_text(
                render_compact_comparison_markdown(result), encoding="utf-8"
            )
        print(f"Compact-only comparison: {args.comparison_json_out}")
        return 0
    if not args.input_dir or not args.seeds or args.duration_seconds is None or not args.output_dir:
        parser.error("raw analysis requires input_dir, --seeds, --duration-seconds, and --output-dir")
    seeds = [int(value) for value in args.seeds.split(",") if value]
    if not seeds or len(seeds) != len(set(seeds)) or any(seed < 0 for seed in seeds):
        parser.error("seeds must be unique nonnegative integers")
    if args.duration_seconds <= 0:
        parser.error("duration-seconds must be positive")
    try:
        index = analyze(args.input_dir, seeds, args.duration_seconds, args.output_dir,
                        args.include_heavy_contact, args.deep_analysis)
    except (OSError, ValueError, RuntimeError) as exc:
        print(f"unified analysis failed: {exc}", file=sys.stderr)
        return 2
    print(f"Unified analysis index: {args.output_dir / 'unified-analysis-index.json'}")
    print(f"Stages: {len(index['stages'])}; failures: {len(index['quality']['failed_stages'])}")
    return 1 if index["quality"]["failed_stages"] else 0


if __name__ == "__main__":
    raise SystemExit(main())
