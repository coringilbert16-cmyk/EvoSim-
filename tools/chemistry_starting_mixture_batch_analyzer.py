#!/usr/bin/env python3
"""Aggregate the matched five-seed starting-molecule composition experiment."""
from __future__ import annotations

import argparse
import json
import statistics
from pathlib import Path
from typing import Any

SEEDS = (42, 43, 44, 45, 46)
MIXTURES = ("baseline", "high-bonded", "mixed-stability")


def read_json(path: Path) -> dict[str, Any]:
    with path.open(encoding="utf-8") as stream:
        value = json.load(stream)
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected JSON object")
    return value


def mean(values: list[float]) -> float | None:
    return statistics.fmean(values) if values else None


def load_run(root: Path, seed: int, mixture: str) -> dict[str, Any]:
    stem = f"seed-{seed}-{mixture}"
    chemistry_path = root / f"{stem}-chemistry.json"
    organization_path = root / f"{stem}-organization.json"
    if not chemistry_path.is_file() or not organization_path.is_file():
        raise ValueError(f"{mixture}: missing chemistry or organization report for seed {seed}")
    chemistry = read_json(chemistry_path)
    organization = read_json(organization_path)
    source = chemistry.get("input", {})
    expected = {
        "seed": seed,
        "dispersion": "randomized",
        "starting_mixture": mixture,
        "requested_duration_seconds": 300,
        "initial_physical_material_count": 1280,
        "initial_independent_resource_count": 1200,
        "initial_molecule_count": 80,
    }
    for key, value in expected.items():
        if source.get(key) != value:
            raise ValueError(
                f"{chemistry_path}: {key} expected {value!r}, got {source.get(key)!r}"
            )
    if not chemistry.get("resource_coverage", {}).get("all_seven_present_at_start"):
        raise ValueError(f"{chemistry_path}: missing canonical starting resources")
    if not chemistry.get("sampling", {}).get("summary_present"):
        raise ValueError(f"{chemistry_path}: missing final summary")
    if chemistry.get("molecular_history", {}).get("validation_errors"):
        raise ValueError(f"{chemistry_path}: lifecycle validation errors")
    if organization.get("scope", {}).get("lifecycle_validation_errors"):
        raise ValueError(f"{organization_path}: lifecycle validation errors")
    inventory = source.get("initial_material_inventory")
    if not isinstance(inventory, dict) or len(inventory) != 7:
        raise ValueError(f"{chemistry_path}: missing complete seven-resource inventory")
    return {"seed": seed, "chemistry": chemistry, "organization": organization}


def summarize(runs: list[dict[str, Any]]) -> dict[str, Any]:
    def org_value(run: dict[str, Any], section: str, key: str) -> Any:
        return run["organization"].get(section, {}).get(key)

    metrics = {
        "molecules_created": [org_value(run, "scope", "molecule_creation_records") for run in runs],
        "compound_molecule_identities": [
            org_value(run, "organization", "molecules_with_multi_resource_or_bonded_structure")
            for run in runs
        ],
        "unique_structural_families": [
            org_value(run, "organization", "unique_exact_structural_families") for run in runs
        ],
        "recurrent_product_families": [
            org_value(run, "organization", "recurrent_families") for run in runs
        ],
        "families_active_at_end": [
            org_value(run, "organization", "families_active_at_end") for run in runs
        ],
        "mean_observed_lifetime_ticks": [
            org_value(run, "persistence", "mean_observed_lifetime_ticks") for run in runs
        ],
    }
    means = {
        name: mean([float(value) for value in values if isinstance(value, (int, float))])
        for name, values in metrics.items()
    }
    reaction_totals: dict[str, dict[str, int]] = {}
    for reaction in ("BREAK", "COMBINE", "DISPLACEMENT"):
        reaction_totals[reaction] = {
            key: sum(
                int(run["chemistry"].get("reactions", {}).get("diagnostic_totals", {})
                    .get(reaction, {}).get(key, 0) or 0)
                for run in runs
            )
            for key in ("ready", "scheduled", "committed", "rejected")
        }
    return {
        "run_count": len(runs),
        "simulated_seconds": 300 * len(runs),
        "seeds": [run["seed"] for run in runs],
        "reaction_totals": reaction_totals,
        "organization_mean_per_seed": means,
        "seed_metrics": {
            str(run["seed"]): {
                name: org_value(run, section, key)
                for name, section, key in (
                    ("molecules_created", "scope", "molecule_creation_records"),
                    ("compound_molecule_identities", "organization", "molecules_with_multi_resource_or_bonded_structure"),
                    ("recurrent_product_families", "organization", "recurrent_families"),
                    ("unique_structural_families", "organization", "unique_exact_structural_families"),
                )
            }
            for run in runs
        },
    }


def build_report(root: Path) -> dict[str, Any]:
    runs = {
        mixture: [load_run(root, seed, mixture) for seed in SEEDS]
        for mixture in MIXTURES
    }
    # Match each seed's total elemental inventory across all three mixtures.
    for seed in SEEDS:
        inventories = {
            mixture: runs[mixture][index]["chemistry"]["input"]["initial_material_inventory"]
            for index, mixture in enumerate(MIXTURES)
        }
        reference = inventories["baseline"]
        for mixture, inventory in inventories.items():
            if inventory != reference:
                raise ValueError(
                    f"seed {seed}: total resource inventory differs for {mixture}: {inventory}"
                )
    return {
        "schema_version": 1,
        "experiment": {
            "seeds": list(SEEDS),
            "starting_mixtures": list(MIXTURES),
            "dispersion": "randomized",
            "seconds_per_run": 300,
            "runs_per_mixture": len(SEEDS),
            "total_runs": len(SEEDS) * len(MIXTURES),
            "initial_inventory": {
                "physical_objects": 1280,
                "independent_resources": 1200,
                "molecules": 80,
                "total_resource_inventory_matched_per_seed": True,
            },
        },
        "conditions": {
            mixture: summarize(condition_runs)
            for mixture, condition_runs in runs.items()
        },
    }


def render_markdown(report: dict[str, Any]) -> str:
    lines = [
        "# Starting-molecule composition experiment",
        "",
        "Five matched seeds per condition; 300 seconds per run; randomized dispersion.",
        "Total per-resource inventory is held constant across mixtures for each seed.",
        "",
        "| Starting mixture | Runs | BREAK commits | COMBINE commits | DISPLACEMENT commits | Molecules created / seed | Compound identities / seed | Recurrent product families / seed |",
        "|---|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for name, row in report["conditions"].items():
        means = row["organization_mean_per_seed"]
        reactions = row["reaction_totals"]
        lines.append(
            f"| {name} | {row['run_count']} | "
            f"{reactions['BREAK']['committed']} | {reactions['COMBINE']['committed']} | "
            f"{reactions['DISPLACEMENT']['committed']} | "
            f"{means['molecules_created']} | {means['compound_molecule_identities']} | "
            f"{means['recurrent_product_families']} |"
        )
    lines += ["", "## Seed-level organization", ""]
    for name, row in report["conditions"].items():
        lines += [f"### {name}", "", "| Seed | Molecules created | Compound identities | Recurrent families | Structural families |", "|---:|---:|---:|---:|---:|"]
        for seed, metrics in row["seed_metrics"].items():
            lines.append(
                f"| {seed} | {metrics['molecules_created']} | "
                f"{metrics['compound_molecule_identities']} | "
                f"{metrics['recurrent_product_families']} | "
                f"{metrics['unique_structural_families']} |"
            )
        lines.append("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input_dir", type=Path)
    parser.add_argument("--json-out", type=Path, default=Path("starting-mixture-five-seed-report.json"))
    parser.add_argument("--markdown-out", type=Path, default=Path("starting-mixture-five-seed-report.md"))
    args = parser.parse_args()
    report = build_report(args.input_dir)
    args.json_out.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    args.markdown_out.write_text(render_markdown(report) + "\n", encoding="utf-8")
    print(render_markdown(report))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
