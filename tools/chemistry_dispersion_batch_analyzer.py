#!/usr/bin/env python3
"""Aggregate the 15 matched five-minute resource-dispersion experiments."""
from __future__ import annotations

import argparse
import json
import statistics
from pathlib import Path
from typing import Any

SEEDS = (42, 43, 44, 45, 46)
DISPERSIONS = ("randomized", "uniform", "clustered")
REACTIONS = ("BREAK", "COMBINE", "DISPLACEMENT")


def read_json(path: Path) -> dict[str, Any]:
    with path.open(encoding="utf-8") as stream:
        value = json.load(stream)
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected JSON object")
    return value


def mean(values: list[float]) -> float | None:
    return statistics.fmean(values) if values else None


def load_condition(root: Path, dispersion: str) -> list[dict[str, Any]]:
    runs = []
    for seed in SEEDS:
        stem = f"seed-{seed}-{dispersion}"
        chemistry_path = root / f"{stem}-chemistry.json"
        organization_path = root / f"{stem}-organization.json"
        if not chemistry_path.is_file() or not organization_path.is_file():
            raise ValueError(f"{dispersion}: missing chemistry or organization report for seed {seed}")
        chemistry = read_json(chemistry_path)
        organization = read_json(organization_path)
        source = chemistry.get("input", {})
        expected = {
            "seed": seed,
            "dispersion": dispersion,
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
        runs.append({"seed": seed, "chemistry": chemistry, "organization": organization})
    return runs


def summarize(runs: list[dict[str, Any]]) -> dict[str, Any]:
    reaction_totals: dict[str, dict[str, int]] = {}
    funnels: dict[str, dict[str, int]] = {}
    for reaction in REACTIONS:
        reaction_totals[reaction] = {
            key: sum(int(run["chemistry"].get("reactions", {}).get("diagnostic_totals", {})
                         .get(reaction, {}).get(key, 0) or 0) for run in runs)
            for key in ("ready", "scheduled", "committed", "rejected")
        }
        stage_names = sorted({
            stage
            for run in runs
            for stage in run["chemistry"].get("reactions", {}).get("opportunity_funnels", {})
                .get(reaction, {})
        })
        funnels[reaction] = {
            stage: sum(int(run["chemistry"].get("reactions", {}).get("opportunity_funnels", {})
                           .get(reaction, {}).get(stage, 0) or 0) for run in runs)
            for stage in stage_names
        }

    def organization_value(run: dict[str, Any], section: str, key: str) -> Any:
        return run["organization"].get(section, {}).get(key)

    organization_metrics = {
        "molecules_created": [organization_value(run, "scope", "molecule_creation_records") for run in runs],
        "molecules_destroyed": [organization_value(run, "scope", "molecule_destruction_records") for run in runs],
        "active_molecules_at_end": [organization_value(run, "scope", "active_molecules_at_end") for run in runs],
        "compound_molecule_identities": [organization_value(run, "organization", "molecules_with_multi_resource_or_bonded_structure") for run in runs],
        "unique_structural_families": [organization_value(run, "organization", "unique_exact_structural_families") for run in runs],
        "recurrent_product_families": [organization_value(run, "organization", "recurrent_families") for run in runs],
        "families_active_at_end": [organization_value(run, "organization", "families_active_at_end") for run in runs],
        "mean_observed_lifetime_ticks": [organization_value(run, "persistence", "mean_observed_lifetime_ticks") for run in runs],
    }
    means = {
        name: mean([float(value) for value in values if isinstance(value, (int, float))])
        for name, values in organization_metrics.items()
    }
    energy_residuals = [
        abs(float(run["chemistry"].get("energy", {}).get("maximum_absolute_snapshot_residual")))
        for run in runs
        if isinstance(run["chemistry"].get("energy", {}).get("maximum_absolute_snapshot_residual"), (int, float))
    ]
    return {
        "run_count": len(runs),
        "simulated_seconds": 300 * len(runs),
        "reaction_totals": reaction_totals,
        "opportunity_funnels": funnels,
        "organization_mean_per_seed": means,
        "worst_max_abs_energy_residual": max(energy_residuals, default=None),
        "seeds": [run["seed"] for run in runs],
    }


def build_report(root: Path) -> dict[str, Any]:
    conditions = {
        dispersion: summarize(load_condition(root, dispersion))
        for dispersion in DISPERSIONS
    }
    return {
        "schema_version": 1,
        "experiment": {
            "seeds": list(SEEDS),
            "dispersions": list(DISPERSIONS),
            "seconds_per_run": 300,
            "runs_per_dispersion": len(SEEDS),
            "total_runs": len(SEEDS) * len(DISPERSIONS),
            "initial_inventory": {
                "physical_objects": 1280,
                "independent_resources": 1200,
                "molecules": 80,
            },
        },
        "conditions": conditions,
    }


def render_markdown(report: dict[str, Any]) -> str:
    lines = [
        "# Resource-dispersion chemistry experiment",
        "",
        "Five independent seeds per condition; 300 seconds per run; identical starting inventory.",
        "",
        "| Dispersion | Runs | BREAK commits | COMBINE commits | DISPLACEMENT commits | Molecules created (mean/seed) | Compound identities (mean/seed) | Recurrent families (mean/seed) |",
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
    lines += ["", "## Opportunity funnels", ""]
    for name, row in report["conditions"].items():
        lines += [f"### {name}", "", "| Reaction | Stage | Total across five seeds |", "|---|---|---:|"]
        for reaction, stages in row["opportunity_funnels"].items():
            for stage, count in stages.items():
                lines.append(f"| {reaction} | {stage} | {count} |")
        lines.append("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input_dir", type=Path)
    parser.add_argument("--json-out", type=Path, default=Path("dispersion-five-seed-report.json"))
    parser.add_argument("--markdown-out", type=Path, default=Path("dispersion-five-seed-report.md"))
    args = parser.parse_args()
    report = build_report(args.input_dir)
    args.json_out.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    args.markdown_out.write_text(render_markdown(report) + "\n", encoding="utf-8")
    print(render_markdown(report))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
