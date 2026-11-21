#!/usr/bin/env python3
"""Compare retained specialist JSON outputs with OBS-001 reanalysis outputs."""
from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path
from typing import Any


def flatten(value: Any, prefix: str = "") -> dict[str, Any]:
    if isinstance(value, dict):
        if not value:
            return {prefix: {}}
        flattened: dict[str, Any] = {}
        for key, child in value.items():
            path = f"{prefix}.{key}" if prefix else str(key)
            flattened.update(flatten(child, path))
        return flattened
    if isinstance(value, list):
        if not value:
            return {prefix: []}
        flattened = {}
        for index, child in enumerate(value):
            path = f"{prefix}[{index}]"
            flattened.update(flatten(child, path))
        return flattened
    return {prefix: value}


def render_value(value: Any, limit: int = 180) -> str:
    rendered = json.dumps(value, sort_keys=True, ensure_ascii=False)
    return rendered if len(rendered) <= limit else rendered[: limit - 3] + "..."


def compare_file(original: Path, recomputed: Path, seed: int, analyzer: str) -> dict[str, Any]:
    if not original.is_file():
        raise ValueError(f"missing retained {analyzer} specialist output: {original}")
    if not recomputed.is_file():
        raise ValueError(f"missing OBS-001 {analyzer} output: {recomputed}")
    old = json.loads(original.read_text(encoding="utf-8"))
    new = json.loads(recomputed.read_text(encoding="utf-8"))
    old_flat = flatten(old)
    new_flat = flatten(new)
    differences = []
    roots: Counter[str] = Counter()
    for path in sorted(set(old_flat) | set(new_flat)):
        old_present = path in old_flat
        new_present = path in new_flat
        if old_present and new_present and old_flat[path] == new_flat[path]:
            continue
        roots[path.split(".", 1)[0].split("[", 1)[0]] += 1
        if len(differences) < 20:
            differences.append({
                "path": path,
                "original": render_value(old_flat[path]) if old_present else "<missing>",
                "recomputed": render_value(new_flat[path]) if new_present else "<missing>",
            })
    difference_count = sum(roots.values())
    return {
        "seed": seed,
        "analyzer": analyzer,
        "original_schema_version": old.get("schema_version") if isinstance(old, dict) else None,
        "recomputed_schema_version": new.get("schema_version") if isinstance(new, dict) else None,
        "exact_match": difference_count == 0,
        "difference_count": difference_count,
        "difference_count_by_top_level_field": dict(sorted(roots.items())),
        "difference_examples": differences,
        "original_path": original.name,
        "recomputed_path": recomputed.name,
    }


def compare_directories(original_dir: Path, recomputed_dir: Path, seeds: list[int]) -> dict[str, Any]:
    rows = []
    for seed in seeds:
        for analyzer in ("chemistry", "organization"):
            rows.append(compare_file(
                original_dir / f"seed-{seed}-{analyzer}.json",
                recomputed_dir / f"seed-{seed}-{analyzer}.json",
                seed,
                analyzer,
            ))
    exact_matches = sum(row["exact_match"] for row in rows)
    return {
        "schema": "evosim.historical_specialist_output_comparison",
        "schema_version": 1,
        "comparison_policy": "strict recursive JSON value comparison; no metric fields are silently ignored",
        "independent_seeds": seeds,
        "report_count": len(rows),
        "exact_match_count": exact_matches,
        "reports_with_differences": len(rows) - exact_matches,
        "per_report": rows,
        "interpretation_limits": [
            "Differences identify output changes between the retained specialist report and current OBS-001 reanalysis; they do not by themselves establish which output is scientifically correct.",
            "Historical inputs without canonical manifests remain legacy provenance and are not eligible for strict future-record finalization.",
            "Exact JSON equality is a reproducibility check, not a causal or scientific-validity test.",
            "Only compact differences and up to twenty example paths per report are retained; raw observations are not copied into this comparison.",
        ],
    }


def render_markdown(result: dict[str, Any]) -> str:
    lines = [
        "# Historical specialist-output comparison",
        "",
        f"- Independent seeds: {', '.join(map(str, result['independent_seeds']))}",
        f"- Reports compared: {result['report_count']}",
        f"- Exact matches: {result['exact_match_count']}",
        f"- Reports with differences: {result['reports_with_differences']}",
        "- Policy: strict recursive JSON value comparison; no fields silently ignored.",
        "",
        "| Seed | Analyzer | Exact match | Differing JSON paths |",
        "|---:|---|---|---:|",
    ]
    for row in result["per_report"]:
        lines.append(
            f"| {row['seed']} | {row['analyzer']} | "
            f"{'yes' if row['exact_match'] else 'no'} | {row['difference_count']} |"
        )
    for row in result["per_report"]:
        if row["exact_match"]:
            continue
        lines.extend([
            "",
            f"## Seed {row['seed']} — {row['analyzer']}",
            "",
            "| JSON path | Retained | Recomputed |",
            "|---|---|---|",
        ])
        for difference in row["difference_examples"]:
            lines.append(
                f"| `{difference['path']}` | {difference['original']} | {difference['recomputed']} |"
            )
    lines.extend(["", "## Interpretation limits", ""])
    lines.extend(f"- {item}" for item in result["interpretation_limits"])
    lines.append("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--original-dir", type=Path, required=True)
    parser.add_argument("--recomputed-dir", type=Path, required=True)
    parser.add_argument("--seeds", required=True)
    parser.add_argument("--output-json", type=Path, required=True)
    parser.add_argument("--output-markdown", type=Path, required=True)
    args = parser.parse_args()
    seeds = [int(value.strip()) for value in args.seeds.split(",") if value.strip()]
    if not seeds or len(seeds) != len(set(seeds)) or any(seed < 0 for seed in seeds):
        parser.error("--seeds must be a nonempty list of unique nonnegative integers")
    result = compare_directories(args.original_dir, args.recomputed_dir, seeds)
    args.output_json.parent.mkdir(parents=True, exist_ok=True)
    args.output_markdown.parent.mkdir(parents=True, exist_ok=True)
    args.output_json.write_text(json.dumps(result, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    args.output_markdown.write_text(render_markdown(result), encoding="utf-8")
    print(f"Compared {result['report_count']} retained specialist reports; "
          f"{result['reports_with_differences']} contain differences.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
