#!/usr/bin/env python3
"""Combine five matched 5-minute EvoSim resonance observation pairs.

The report is descriptive: it preserves per-seed pairing and does not treat
individual reaction events as statistically independent observations.
"""
from __future__ import annotations

import argparse
import json
import re
import statistics
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any

EXPECTED_SEEDS = (42, 43, 44, 45, 46)
MODES = ("enabled", "disabled")
REACTIONS = ("BREAK", "COMBINE", "DISPLACEMENT")


def read_json(path: Path) -> dict[str, Any]:
    with path.open(encoding="utf-8") as stream:
        value = json.load(stream)
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected a JSON object")
    return value


def number(value: Any) -> float | int | None:
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        return value
    return None


def load_run(chemistry_path: Path, seed: int, mode: str) -> dict[str, Any]:
    stem = chemistry_path.name.removesuffix("-chemistry.json")
    organization_path = chemistry_path.with_name(stem + "-organization.json")
    if not organization_path.is_file():
        raise ValueError(f"{chemistry_path}: missing {organization_path.name}")

    chemistry = read_json(chemistry_path)
    organization = read_json(organization_path)
    input_data = chemistry.get("input", {})
    if input_data.get("seed") != seed:
        raise ValueError(f"{chemistry_path}: expected seed {seed}, got {input_data.get('seed')}")
    if input_data.get("requested_duration_seconds") != 300:
        raise ValueError(
            f"{chemistry_path}: expected a 300-second run, got "
            f"{input_data.get('requested_duration_seconds')}"
        )
    if not chemistry.get("resource_coverage", {}).get("all_seven_present_at_start"):
        raise ValueError(f"{chemistry_path}: not all seven canonical resources were present")
    if not chemistry.get("sampling", {}).get("summary_present"):
        raise ValueError(f"{chemistry_path}: missing final chemistry summary")

    chemistry_lifecycle_errors = chemistry.get("molecular_history", {}).get(
        "validation_errors", []
    )
    organization_lifecycle_errors = organization.get("scope", {}).get(
        "lifecycle_validation_errors", []
    )
    if chemistry_lifecycle_errors or organization_lifecycle_errors:
        raise ValueError(f"{chemistry_path}: molecular lifecycle validation errors")

    totals = chemistry.get("reactions", {}).get("diagnostic_totals", {})
    event_counts = chemistry.get("reactions", {}).get("event_counts", {})
    event_outcomes = chemistry.get("reactions", {}).get("outcome_counts", {})
    reaction_metrics: dict[str, Any] = {}
    for reaction in REACTIONS:
        row = totals.get(reaction, {})
        reaction_metrics[reaction] = {
            "ready": int(row.get("ready", 0) or 0),
            "scheduled": int(row.get("scheduled", 0) or 0),
            "committed": int(row.get("committed", 0) or 0),
            "rejected": int(row.get("rejected", 0) or 0),
            "events": int(event_counts.get(reaction, 0) or 0),
            "outcomes": {
                outcome: int(count)
                for key, count in event_outcomes.items()
                if key.startswith(reaction + ":")
                for outcome in [key.split(":", 1)[1]]
            },
        }

    org = organization.get("organization", {})
    persistence = organization.get("persistence", {})
    scope = organization.get("scope", {})
    energy = chemistry.get("energy", {})
    return {
        "seed": seed,
        "mode": mode,
        "duration_seconds": input_data.get("requested_duration_seconds"),
        "planned_ticks": input_data.get("planned_ticks"),
        "initial_physical_material_count": input_data.get("initial_physical_material_count"),
        "initial_independent_resource_count": input_data.get(
            "initial_independent_resource_count"
        ),
        "initial_molecule_count": input_data.get("initial_molecule_count"),
        "all_seven_resources_present": True,
        "snapshots_missing_resources": chemistry.get("resource_coverage", {}).get(
            "snapshots_missing_resources", []
        ),
        "reaction_metrics": reaction_metrics,
        "molecules_created": int(scope.get("molecule_creation_records", 0) or 0),
        "molecules_destroyed": int(scope.get("molecule_destruction_records", 0) or 0),
        "active_molecules_at_end": int(scope.get("active_molecules_at_end", 0) or 0),
        "unique_structural_families": int(
            org.get("unique_exact_structural_families", 0) or 0
        ),
        "recurrent_product_families": int(org.get("recurrent_families", 0) or 0),
        "recurrent_families_including_initial": int(
            org.get("recurrent_families_including_initial_seeds", 0) or 0
        ),
        "families_with_reaction_products": int(
            org.get("families_with_reaction_created_products", 0) or 0
        ),
        "families_active_at_end": int(org.get("families_active_at_end", 0) or 0),
        "compound_molecule_identities": int(
            org.get("molecules_with_multi_resource_or_bonded_structure", 0) or 0
        ),
        "mean_observed_lifetime_ticks": number(
            persistence.get("mean_observed_lifetime_ticks")
        ),
        "median_observed_lifetime_ticks": number(
            persistence.get("median_observed_lifetime_ticks")
        ),
        "molecule_creations_by_reaction": organization.get(
            "reaction_network", {}
        ).get("molecule_creations_by_reaction", {}),
        "lineage_reaction_counts": chemistry.get("molecular_history", {}).get(
            "lineage_reaction_counts", {}
        ),
        "energy_last_residual": number(energy.get("last_balance_residual")),
        "energy_max_abs_snapshot_residual": number(
            energy.get("maximum_absolute_snapshot_residual")
        ),
    }


def load_runs(root: Path) -> dict[int, dict[str, dict[str, Any]]]:
    files = list(root.rglob("seed-*-enabled-chemistry.json"))
    found: dict[int, dict[str, dict[str, Any]]] = {
        seed: {} for seed in EXPECTED_SEEDS
    }
    for path in files:
        match = re.fullmatch(r"seed-(\d+)-enabled-chemistry\.json", path.name)
        if not match:
            continue
        seed = int(match.group(1))
        if seed not in found:
            continue
        for mode in MODES:
            chem_path = path if mode == "enabled" else path.with_name(
                f"seed-{seed}-disabled-chemistry.json"
            )
            if not chem_path.is_file():
                raise ValueError(f"seed {seed}: missing {mode} chemistry report")
            if mode in found[seed]:
                raise ValueError(f"seed {seed}: duplicate {mode} report")
            found[seed][mode] = load_run(chem_path, seed, mode)

    for seed, modes in found.items():
        missing = [mode for mode in MODES if mode not in modes]
        if missing:
            raise ValueError(f"seed {seed}: missing run(s): {', '.join(missing)}")
    if len(files) < len(EXPECTED_SEEDS):
        raise ValueError("fewer than five enabled-seed reports were found")
    return found


def mean(values: list[float]) -> float | None:
    return statistics.fmean(values) if values else None


def summarize_mode(runs: list[dict[str, Any]], all_families: dict[str, Any]) -> dict[str, Any]:
    reaction_totals = {
        reaction: {
            key: sum(run["reaction_metrics"][reaction][key] for run in runs)
            for key in ("ready", "scheduled", "committed", "rejected", "events")
        }
        for reaction in REACTIONS
    }
    metric_names = (
        "molecules_created", "molecules_destroyed", "active_molecules_at_end",
        "unique_structural_families", "recurrent_product_families",
        "recurrent_families_including_initial", "families_with_reaction_products",
        "families_active_at_end", "compound_molecule_identities",
    )
    organization_means = {
        name: mean([float(run[name]) for run in runs]) for name in metric_names
    }
    lifetime_rows = [
        (run["mean_observed_lifetime_ticks"], run["molecules_created"])
        for run in runs if run["mean_observed_lifetime_ticks"] is not None
    ]
    lifetime_weight = sum(weight for _, weight in lifetime_rows)
    pooled_mean_lifetime = (
        sum(float(value) * weight for value, weight in lifetime_rows) / lifetime_weight
        if lifetime_weight else None
    )
    reaction_product_counts: dict[str, int] = defaultdict(int)
    lineage_counts: dict[str, int] = defaultdict(int)
    for run in runs:
        for reaction, count in run["molecule_creations_by_reaction"].items():
            reaction_product_counts[str(reaction)] += int(count)
        for reaction, count in run["lineage_reaction_counts"].items():
            lineage_counts[str(reaction)] += int(count)

    worst_energy = [
        run["energy_max_abs_snapshot_residual"] for run in runs
        if run["energy_max_abs_snapshot_residual"] is not None
    ]
    final_energy = [
        run["energy_last_residual"] for run in runs
        if run["energy_last_residual"] is not None
    ]
    recurrent = [
        row for row in all_families.values()
        if row["seed_count"] >= 2
    ]
    recurrent.sort(key=lambda row: (
        -row["seed_count"], -row["reaction_created_events"],
        -row["reaction_created_occurrences"], json.dumps(row["composition"], sort_keys=True)
    ))
    return {
        "run_count": len(runs),
        "simulated_seconds": sum(int(run["duration_seconds"]) for run in runs),
        "reaction_totals": reaction_totals,
        "organization_mean_per_seed": organization_means,
        "pooled_mean_observed_lifetime_ticks": pooled_mean_lifetime,
        "molecule_creations_by_reaction": dict(sorted(reaction_product_counts.items())),
        "lineage_reaction_counts": dict(sorted(lineage_counts.items())),
        "worst_max_abs_energy_residual": max(worst_energy, default=None),
        "final_energy_residuals": final_energy,
        "cross_seed_recurrent_product_family_count": len(recurrent),
        "cross_seed_recurrent_product_families": recurrent[:20],
    }


def build_report(root: Path) -> dict[str, Any]:
    paired = load_runs(root)
    all_by_mode: dict[str, list[dict[str, Any]]] = {
        mode: [paired[seed][mode] for seed in EXPECTED_SEEDS] for mode in MODES
    }
    family_maps: dict[str, dict[str, Any]] = {mode: {} for mode in MODES}
    for mode in MODES:
        for run in all_by_mode[mode]:
            seed = run["seed"]
            org_path = next(root.rglob(f"seed-{seed}-{mode}-organization.json"), None)
            if org_path is None:
                raise ValueError(f"seed {seed} {mode}: missing organization detail")
            organization = read_json(org_path)
            for family in organization.get("organization", {}).get("families", []):
                if int(family.get("reaction_created_occurrences", 0) or 0) <= 0:
                    continue
                signature = str(family.get("signature", ""))
                if not signature:
                    continue
                target = family_maps[mode].setdefault(signature, {
                    "signature": signature,
                    "composition": family.get("composition", {}),
                    "seed_ids": [],
                    "seed_count": 0,
                    "reaction_created_occurrences": 0,
                    "reaction_created_events": 0,
                    "active_reaction_created_occurrences": 0,
                })
                target["seed_ids"].append(seed)
                target["reaction_created_occurrences"] += int(
                    family.get("reaction_created_occurrences", 0) or 0
                )
                target["reaction_created_events"] += int(
                    family.get("reaction_created_reaction_events", 0) or 0
                )
                target["active_reaction_created_occurrences"] += int(
                    family.get("reaction_created_active_at_end", 0) or 0
                )
        for row in family_maps[mode].values():
            row["seed_ids"] = sorted(set(row["seed_ids"]))
            row["seed_count"] = len(row["seed_ids"])

    seed_rows = []
    for seed in EXPECTED_SEEDS:
        enabled = paired[seed]["enabled"]
        disabled = paired[seed]["disabled"]
        deltas = {
            "BREAK_committed": enabled["reaction_metrics"]["BREAK"]["committed"]
                - disabled["reaction_metrics"]["BREAK"]["committed"],
            "COMBINE_committed": enabled["reaction_metrics"]["COMBINE"]["committed"]
                - disabled["reaction_metrics"]["COMBINE"]["committed"],
            "DISPLACEMENT_committed": enabled["reaction_metrics"]["DISPLACEMENT"]["committed"]
                - disabled["reaction_metrics"]["DISPLACEMENT"]["committed"],
            "compound_molecule_identities": enabled["compound_molecule_identities"]
                - disabled["compound_molecule_identities"],
            "unique_structural_families": enabled["unique_structural_families"]
                - disabled["unique_structural_families"],
            "recurrent_product_families": enabled["recurrent_product_families"]
                - disabled["recurrent_product_families"],
            "families_active_at_end": enabled["families_active_at_end"]
                - disabled["families_active_at_end"],
        }
        seed_rows.append({
            "seed": seed,
            "enabled": enabled,
            "disabled": disabled,
            "enabled_minus_disabled": deltas,
        })

    delta_keys = list(seed_rows[0]["enabled_minus_disabled"])
    paired_deltas: dict[str, Any] = {}
    for key in delta_keys:
        values = [float(row["enabled_minus_disabled"][key]) for row in seed_rows]
        paired_deltas[key] = {
            "values_by_seed": {
                str(row["seed"]): row["enabled_minus_disabled"][key] for row in seed_rows
            },
            "mean": mean(values),
            "median": statistics.median(values),
            "positive_seeds": sum(value > 0 for value in values),
            "negative_seeds": sum(value < 0 for value in values),
            "equal_seeds": sum(value == 0 for value in values),
        }

    return {
        "schema_version": 1,
        "experiment": {
            "seeds": list(EXPECTED_SEEDS),
            "duration_seconds_per_run": 300,
            "condition_pairing": "same seed and environment; local resonance activation enabled versus disabled",
            "simulated_minutes_per_condition": 25,
            "total_simulated_minutes": 50,
            "initial_environment": "standard environmental chemistry experiment; 1,280 physical materials, including 1,200 independent resources and 80 molecules",
            "discovery_and_reaction_rules": "unchanged; no forced reactions",
            "delta_convention": "enabled minus disabled",
        },
        "runs": seed_rows,
        "conditions": {
            mode: summarize_mode(
                all_by_mode[mode], family_maps[mode]
            ) for mode in MODES
        },
        "paired_deltas": paired_deltas,
        "validation": {
            "all_ten_runs_present": True,
            "all_runs_300_seconds": all(
                row[mode]["duration_seconds"] == 300
                for row in seed_rows for mode in MODES
            ),
            "all_runs_have_seven_initial_resources": all(
                row[mode]["all_seven_resources_present"]
                for row in seed_rows for mode in MODES
            ),
            "no_snapshot_resource_gaps": all(
                not row[mode]["snapshots_missing_resources"]
                for row in seed_rows for mode in MODES
            ),
        },
        "interpretation_limits": [
            "The five seeds are the independent paired observations; individual reactions within a run are not independent samples.",
            "Differences are descriptive paired effects, not a claim of statistical significance.",
            "Structural families compare composition and bond topology, not bond energy, endpoint geometry, or chemical equivalence.",
            "Active molecule lifetimes at the final snapshot are right-censored.",
            "Material totals across lineage generations may count the same conserved constituents more than once.",
        ],
    }


def fmt(value: Any, digits: int = 3) -> str:
    if value is None:
        return "n/a"
    if isinstance(value, float):
        return f"{value:.{digits}f}"
    return str(value)


def render_markdown(report: dict[str, Any]) -> str:
    conditions = report["conditions"]
    enabled = conditions["enabled"]
    disabled = conditions["disabled"]
    lines = [
        "# EvoSim five-seed resonance chemistry observation",
        "",
        "- Seeds: 42, 43, 44, 45, 46",
        "- Duration: 5 minutes per run",
        "- Conditions: matched local-resonance enabled and disabled",
        "- Total simulation time: 25 minutes per condition; 50 minutes combined",
        "- Environment: standard 1,280-material setup",
        "- Reaction discovery and reaction rules: unchanged; no forced reactions",
        "",
        "## Paired results by seed",
        "",
        "| Seed | BREAK Δ commits | COMBINE Δ commits | DISPLACEMENT Δ commits | Δ compound identities | Δ structural families | Δ recurrent product families |",
        "|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for row in report["runs"]:
        delta = row["enabled_minus_disabled"]
        lines.append(
            f"| {row['seed']} | {delta['BREAK_committed']} | {delta['COMBINE_committed']} | "
            f"{delta['DISPLACEMENT_committed']} | {delta['compound_molecule_identities']} | "
            f"{delta['unique_structural_families']} | {delta['recurrent_product_families']} |"
        )

    lines += [
        "",
        "## Aggregate reaction totals",
        "",
        "| Condition | Reaction | Ready candidates | Scheduled | Committed | Rejected | Recorded events |",
        "|---|---|---:|---:|---:|---:|---:|",
    ]
    for mode in MODES:
        for reaction in REACTIONS:
            values = conditions[mode]["reaction_totals"][reaction]
            lines.append(
                f"| {mode} | {reaction} | {values['ready']} | {values['scheduled']} | "
                f"{values['committed']} | {values['rejected']} | {values['events']} |"
            )

    lines += [
        "",
        "## Organization and persistence by condition",
        "",
        "| Metric (mean per seed unless noted) | Enabled | Disabled |",
        "|---|---:|---:|",
    ]
    metric_labels = [
        ("molecules_created", "Molecule identities created"),
        ("compound_molecule_identities", "Multi-resource or bonded molecule identities"),
        ("unique_structural_families", "Exact structural families"),
        ("recurrent_product_families", "Families recurring across reaction-created products"),
        ("families_with_reaction_products", "Families with reaction-created products"),
        ("families_active_at_end", "Structural families active at end"),
        ("active_molecules_at_end", "Molecules active at end"),
    ]
    for key, label in metric_labels:
        lines.append(
            f"| {label} | {fmt(enabled['organization_mean_per_seed'][key])} | "
            f"{fmt(disabled['organization_mean_per_seed'][key])} |"
        )
    lines.append(
        f"| Pooled mean observed lifetime (ticks) | "
        f"{fmt(enabled['pooled_mean_observed_lifetime_ticks'])} | "
        f"{fmt(disabled['pooled_mean_observed_lifetime_ticks'])} |"
    )

    lines += [
        "",
        "## Paired deltas (enabled minus disabled)",
        "",
        "| Metric | Mean difference per seed | Median difference | Seeds higher | Seeds lower | Tied |",
        "|---|---:|---:|---:|---:|---:|",
    ]
    labels = {
        "BREAK_committed": "BREAK commits",
        "COMBINE_committed": "COMBINE commits",
        "DISPLACEMENT_committed": "DISPLACEMENT commits",
        "compound_molecule_identities": "Compound identities",
        "unique_structural_families": "Structural families",
        "recurrent_product_families": "Recurrent product families",
        "families_active_at_end": "Active structural families",
    }
    for key, label in labels.items():
        row = report["paired_deltas"][key]
        lines.append(
            f"| {label} | {fmt(row['mean'])} | {fmt(row['median'])} | "
            f"{row['positive_seeds']} | {row['negative_seeds']} | {row['equal_seeds']} |"
        )

    lines += [
        "",
        "## Recurrent product structures appearing in multiple seeds",
        "",
    ]
    for mode in MODES:
        families = conditions[mode]["cross_seed_recurrent_product_families"]
        lines.append(f"### {mode.capitalize()}")
        lines.append("")
        lines.append("| Seeds containing family | Reaction-created occurrences | Distinct product reactions | Active at end | Composition |")
        lines.append("|---:|---:|---:|---:|---|")
        if not families:
            lines.append("| — | — | — | — | None observed in 2+ seeds |")
        for family in families[:15]:
            composition = ", ".join(
                f"{name}:{amount:g}" for name, amount in sorted(
                    (family.get("composition") or {}).items()
                )
            )
            lines.append(
                f"| {family['seed_count']} | {family['reaction_created_occurrences']} | "
                f"{family['reaction_created_events']} | "
                f"{family['active_reaction_created_occurrences']} | {composition} |"
            )
        lines.append("")

    lines += [
        "## Energy accounting",
        "",
        "| Condition | Worst absolute snapshot residual across five seeds | Final residuals by seed |",
        "|---|---:|---|",
    ]
    for mode in MODES:
        condition = conditions[mode]
        final_values = ", ".join(fmt(value, 8) for value in condition["final_energy_residuals"])
        lines.append(
            f"| {mode} | {fmt(condition['worst_max_abs_energy_residual'], 8)} | {final_values} |"
        )

    lines += [
        "",
        "## Validation",
        "",
        f"- Ten matched runs present: {report['validation']['all_ten_runs_present']}",
        f"- Every run completed 300 seconds: {report['validation']['all_runs_300_seconds']}",
        f"- All seven canonical resources present initially in every run: {report['validation']['all_runs_have_seven_initial_resources']}",
        f"- No snapshot resource gaps: {report['validation']['no_snapshot_resource_gaps']}",
        "",
        "## Interpretation limits",
        "",
    ]
    lines.extend(f"- {item}" for item in report["interpretation_limits"])
    lines.append("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input_dir", type=Path)
    parser.add_argument("--json-out", type=Path, required=True)
    parser.add_argument("--markdown-out", type=Path, required=True)
    args = parser.parse_args()
    try:
        report = build_report(args.input_dir)
    except (OSError, ValueError, KeyError, TypeError) as exc:
        print(f"chemistry batch analyzer: {exc}", file=sys.stderr)
        return 2
    args.json_out.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    args.markdown_out.write_text(render_markdown(report), encoding="utf-8")
    print(render_markdown(report), end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
