#!/usr/bin/env python3
"""Aggregate independent, enabled-resonance environmental chemistry observations."""
from __future__ import annotations

import argparse
import json
import statistics
import sys
from collections import Counter
from pathlib import Path
from typing import Any

ACTIVE_ELEMENTS = ("Carbon", "Hydrogen", "Nitrogen", "Oxygen", "Phosphorus", "Sulfur")
REACTIONS = ("BREAK", "COMBINE", "DISPLACEMENT")
ELEMENTAL_BOND_PAIRS = tuple(
    f"{left}-{right}"
    for index, left in enumerate(ACTIVE_ELEMENTS)
    for right in ACTIVE_ELEMENTS[index:]
)


def read_json(path: Path) -> dict[str, Any]:
    with path.open(encoding="utf-8") as stream:
        value = json.load(stream)
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected a JSON object")
    return value


def locate(root: Path, filename: str) -> Path:
    matches = list(root.rglob(filename))
    if len(matches) != 1:
        raise ValueError(f"expected exactly one {filename}, found {len(matches)}")
    return matches[0]


def load_seed(root: Path, seed: int, duration: int) -> dict[str, Any]:
    chemistry_path = locate(root, f"seed-{seed}-chemistry.json")
    organization_path = locate(root, f"seed-{seed}-organization.json")
    chemistry = read_json(chemistry_path)
    organization = read_json(organization_path)
    source = chemistry.get("input", {})
    coverage = chemistry.get("resource_coverage", {})
    if source.get("seed") != seed:
        raise ValueError(f"seed {seed}: chemistry report identifies seed {source.get('seed')}")
    if source.get("requested_duration_seconds") != duration:
        raise ValueError(f"seed {seed}: expected {duration}s, got {source.get('requested_duration_seconds')}")
    if source.get("initial_physical_material_count") != 1280:
        raise ValueError(f"seed {seed}: expected 1,280 starting physical materials")
    if source.get("initial_independent_resource_count") != 0 or source.get("initial_molecule_count") != 1280:
        raise ValueError(f"seed {seed}: expected 1,280 starting two-constituent molecules and no free resources")
    if source.get("initial_constituent_count") != 2560:
        raise ValueError(f"seed {seed}: expected 2,560 starting constituents")
    if coverage.get("excluded_resources") != ["Water"]:
        raise ValueError(f"seed {seed}: active chemistry must exclude Water")
    if set(coverage.get("required_resources", [])) != set(ACTIVE_ELEMENTS):
        raise ValueError(f"seed {seed}: expected all six active elements")
    if not coverage.get("all_required_present_at_start"):
        raise ValueError(f"seed {seed}: one or more active elements missing at start")
    if coverage.get("snapshots_missing_resources"):
        raise ValueError(f"seed {seed}: active element missing from a later inventory snapshot")
    if not chemistry.get("sampling", {}).get("summary_present"):
        raise ValueError(f"seed {seed}: missing final chemistry summary")
    history = chemistry.get("molecular_history", {})
    scope = organization.get("scope", {})
    if history.get("validation_errors") or scope.get("lifecycle_validation_errors"):
        raise ValueError(f"seed {seed}: molecular lifecycle validation errors")
    totals = chemistry.get("reactions", {}).get("diagnostic_totals", {})
    reactions = {
        name: {
            key: int(totals.get(name, {}).get(key, 0) or 0)
            for key in ("ready", "scheduled", "committed", "rejected")
        }
        for name in REACTIONS
    }
    org = organization.get("organization", {})
    persistence = organization.get("persistence", {})
    energy = chemistry.get("energy", {})
    return {
        "seed": seed,
        "duration_seconds": duration,
        "planned_ticks": source.get("planned_ticks"),
        "reactions": reactions,
        "reaction_events": chemistry.get("reactions", {}).get("event_counts", {}),
        "outcomes": chemistry.get("reactions", {}).get("outcome_counts", {}),
        "failure_reasons": chemistry.get("reactions", {}).get("failure_reasons", {}),
        "elemental_bond_pairs": chemistry.get("reactions", {}).get("elemental_bond_pairs", {}),
        "displacement_discovery": totals.get("DISPLACEMENT", {}).get("discovery", {}),
        "displacement_budget_exhausted_ticks": totals.get("DISPLACEMENT", {}).get("discovery_budget_exhausted_ticks", 0),
        "molecules_created": history.get("molecule_creation_count", 0),
        "molecules_destroyed": history.get("molecule_destruction_count", 0),
        "active_molecules_at_end": history.get("active_at_end_count", 0),
        "lineage_edges": history.get("lineage_edges", []),
        "molecules": history.get("molecules", []),
        "mean_observed_lifetime_ticks": history.get("mean_observed_lifetime_ticks"),
        "structural_families": org.get("families", []),
        "unique_structural_families": org.get("unique_exact_structural_families", 0),
        "recurrent_product_families": org.get("recurrent_families", 0),
        "families_active_at_end": org.get("families_active_at_end", 0),
        "compound_identities": org.get("molecules_with_multi_resource_or_bonded_structure", 0),
        "family_transitions": organization.get("reaction_network", {}).get("family_to_family_transitions", []),
        "reaction_product_material": organization.get("material_distribution", {}).get("reaction_created_compound_constituent_amount_by_resource", {}),
        "energy_last_residual": energy.get("last_balance_residual"),
        "energy_max_abs_snapshot_residual": energy.get("maximum_absolute_snapshot_residual"),
    }


def build_report(root: Path, seeds: list[int], duration: int) -> dict[str, Any]:
    runs = [load_seed(root, seed, duration) for seed in seeds]
    totals = {
        reaction: {
            key: sum(run["reactions"][reaction][key] for run in runs)
            for key in ("ready", "scheduled", "committed", "rejected")
        }
        for reaction in REACTIONS
    }
    failures: Counter[str] = Counter()
    event_counts: Counter[str] = Counter()
    outcomes: Counter[str] = Counter()
    products: Counter[str] = Counter()
    families: dict[str, dict[str, Any]] = {}
    pathway_counts: Counter[tuple[str, str, str]] = Counter()
    lineage_edges = 0
    reused_reaction_products = 0
    multistep_combine_edges = 0
    formed_bonds: Counter[str] = Counter()
    broken_bonds: Counter[str] = Counter()
    initial_bond_inventory: Counter[str] = Counter()
    final_bond_inventory: Counter[str] = Counter()
    inventory_delta: dict[str, int] = {}
    inventory_reconciliation_by_seed: dict[str, dict[str, Any]] = {}
    formed_by_reaction: dict[str, Counter[str]] = {}
    broken_by_reaction: dict[str, Counter[str]] = {}
    bond_pair_coverage_by_seed: dict[str, dict[str, Any]] = {}
    for run in runs:
        failures.update(run["failure_reasons"])
        event_counts.update(run["reaction_events"])
        outcomes.update(run["outcomes"])
        products.update({key: float(value) for key, value in run["reaction_product_material"].items()})
        attribution = run["elemental_bond_pairs"]
        formed = attribution.get("formed", {})
        broken = attribution.get("broken", {})
        formed_bonds.update({key: int(value) for key, value in formed.items()})
        broken_bonds.update({key: int(value) for key, value in broken.items()})
        initial_bond_inventory.update({
            key: int(value)
            for key, value in attribution.get("initial_bond_inventory", {}).items()
        })
        final_bond_inventory.update({
            key: int(value)
            for key, value in attribution.get("final_bond_inventory", {}).items()
        })
        for key, value in attribution.get("inventory_delta", {}).items():
            inventory_delta[key] = inventory_delta.get(key, 0) + int(value)
        inventory_reconciliation_by_seed[str(run["seed"])] = attribution.get(
            "inventory_reconciliation", {"matches": False, "mismatches": {}}
        )
        bond_pair_coverage_by_seed[str(run["seed"])] = {
            "committed_events_with_graph_deltas": attribution.get("committed_events_with_graph_deltas", 0),
            "committed_events_missing_graph_deltas": attribution.get("committed_events_missing_graph_deltas", 0),
            "malformed_bond_records": attribution.get("malformed_bond_records", 0),
        }
        for reaction, values in attribution.get("by_reaction", {}).items():
            formed_by_reaction.setdefault(reaction, Counter()).update(values.get("formed", {}))
            broken_by_reaction.setdefault(reaction, Counter()).update(values.get("broken", {}))
        molecules = {row["molecule_id"]: row for row in run["molecules"]}
        for edge in run["lineage_edges"]:
            lineage_edges += 1
            parent = molecules.get(edge.get("parent_id"))
            child = molecules.get(edge.get("child_id"))
            if parent and parent.get("reaction") != "INITIAL":
                reused_reaction_products += 1
                if child and child.get("reaction") == "COMBINE":
                    multistep_combine_edges += 1
        for transition in run["family_transitions"]:
            pathway_counts[(
                str(transition.get("reaction", "UNKNOWN")),
                str(transition.get("parent_family", "")),
                str(transition.get("child_family", "")),
            )] += int(transition.get("count", 0) or 0)
        for family in run["structural_families"]:
            if int(family.get("reaction_created_occurrences", 0) or 0) <= 0:
                continue
            signature = str(family.get("signature", ""))
            if not signature:
                continue
            row = families.setdefault(signature, {
                "signature": signature,
                "composition": family.get("composition", {}),
                "seeds": [],
                "reaction_created_occurrences": 0,
                "reaction_created_events": 0,
                "active_reaction_created_occurrences": 0,
            })
            row["seeds"].append(run["seed"])
            row["reaction_created_occurrences"] += int(family.get("reaction_created_occurrences", 0) or 0)
            row["reaction_created_events"] += int(family.get("reaction_created_reaction_events", 0) or 0)
            row["active_reaction_created_occurrences"] += int(family.get("reaction_created_active_at_end", 0) or 0)
    for row in families.values():
        row["seed_count"] = len(set(row["seeds"]))
        row["seeds"] = sorted(set(row["seeds"]))
    recurrent = sorted(
        (row for row in families.values() if row["seed_count"] >= 2),
        key=lambda row: (-row["seed_count"], -row["reaction_created_events"], row["signature"]),
    )
    element_pairs = list(ELEMENTAL_BOND_PAIRS)
    elemental_bond_pairs = {
        "element_pairs": element_pairs,
        "committed_events_with_graph_deltas": sum(
            int(row["committed_events_with_graph_deltas"] or 0)
            for row in bond_pair_coverage_by_seed.values()
        ),
        "committed_events_missing_graph_deltas": sum(
            int(row["committed_events_missing_graph_deltas"] or 0)
            for row in bond_pair_coverage_by_seed.values()
        ),
        "malformed_bond_records": sum(
            int(row["malformed_bond_records"] or 0)
            for row in bond_pair_coverage_by_seed.values()
        ),
        "formed_total": sum(formed_bonds.values()),
        "broken_total": sum(broken_bonds.values()),
        "formed": dict(sorted(formed_bonds.items())),
        "broken": dict(sorted(broken_bonds.items())),
        "initial_bond_inventory": {
            pair: int(initial_bond_inventory.get(pair, 0)) for pair in ELEMENTAL_BOND_PAIRS
        },
        "final_bond_inventory": {
            pair: int(final_bond_inventory.get(pair, 0)) for pair in ELEMENTAL_BOND_PAIRS
        },
        "inventory_delta": {
            pair: int(inventory_delta.get(pair, 0)) for pair in ELEMENTAL_BOND_PAIRS
        },
        "inventory_reconciliation_by_seed": inventory_reconciliation_by_seed,
        "inventory_reconciliation_all_seeds_match": all(
            bool(row.get("matches")) for row in inventory_reconciliation_by_seed.values()
        ) and len(inventory_reconciliation_by_seed) == len(runs),
        "by_reaction": {
            reaction: {
                "formed": dict(sorted(formed_by_reaction.get(reaction, Counter()).items())),
                "broken": dict(sorted(broken_by_reaction.get(reaction, Counter()).items())),
                "formed_total": sum(formed_by_reaction.get(reaction, Counter()).values()),
                "broken_total": sum(broken_by_reaction.get(reaction, Counter()).values()),
            }
            for reaction in sorted(set(formed_by_reaction) | set(broken_by_reaction))
        },
        "coverage_by_seed": bond_pair_coverage_by_seed,
    }
    energy_residuals = [
        abs(float(run["energy_max_abs_snapshot_residual"]))
        for run in runs if isinstance(run["energy_max_abs_snapshot_residual"], (int, float))
    ]
    return {
        "schema_version": 1,
        "experiment": {
            "seeds": seeds,
            "duration_seconds_per_seed": duration,
            "total_simulated_seconds": duration * len(seeds),
            "condition": "ordinary environmental chemistry; local resonance enabled; no control or forced reactions",
            "initial_environment": "1,280 physical materials: 1,200 independent resources plus 80 molecules; six active elements; Water excluded",
            "simulation_rules_changed": False,
        },
        "validation": {
            "all_requested_seeds_present": len(runs) == len(seeds),
            "all_runs_match_requested_duration": all(run["duration_seconds"] == duration for run in runs),
            "all_runs_pass_inventory_and_lifecycle_checks": True,
        },
        "reaction_totals": totals,
        "reaction_event_counts": dict(event_counts),
        "reaction_outcomes": dict(outcomes),
        "failure_reasons": dict(failures),
        "elemental_bond_pairs": elemental_bond_pairs,
        "displacement": {
            "budget_exhausted_ticks_total": sum(int(run["displacement_budget_exhausted_ticks"] or 0) for run in runs),
            "discovery_by_seed": {str(run["seed"]): run["displacement_discovery"] for run in runs},
        },
        "molecular_development": {
            "molecules_created_total": sum(int(run["molecules_created"] or 0) for run in runs),
            "molecules_destroyed_total": sum(int(run["molecules_destroyed"] or 0) for run in runs),
            "active_molecules_mean_per_seed": statistics.fmean(float(run["active_molecules_at_end"] or 0) for run in runs),
            "mean_observed_lifetime_ticks_by_seed": {str(run["seed"]): run["mean_observed_lifetime_ticks"] for run in runs},
            "lineage_edges_total": lineage_edges,
            "reaction_created_products_reused_as_parents": reused_reaction_products,
            "reused_products_followed_by_combine": multistep_combine_edges,
        },
        "organization": {
            "unique_structural_families_mean_per_seed": statistics.fmean(float(run["unique_structural_families"] or 0) for run in runs),
            "recurrent_product_families_mean_per_seed": statistics.fmean(float(run["recurrent_product_families"] or 0) for run in runs),
            "families_active_at_end_mean_per_seed": statistics.fmean(float(run["families_active_at_end"] or 0) for run in runs),
            "compound_identities_mean_per_seed": statistics.fmean(float(run["compound_identities"] or 0) for run in runs),
            "cross_seed_recurrent_product_families": recurrent[:30],
            "family_pathway_transitions": [
                {"reaction": reaction, "parent_family": parent, "child_family": child, "count": count}
                for (reaction, parent, child), count in sorted(pathway_counts.items(), key=lambda item: (-item[1], item[0]))
            ][:50],
            "reaction_created_compound_material_by_element": dict(sorted(products.items())),
        },
        "energy": {
            "maximum_absolute_snapshot_residual_across_runs": max(energy_residuals, default=None),
            "final_residual_by_seed": {str(run["seed"]): run["energy_last_residual"] for run in runs},
        },
        "runs": runs,
        "interpretation_limits": [
            "Seeds are the independent observations; reactions within a run are not independent samples.",
            "Cross-seed structural recurrence compares composition and bond topology, not bond energy or endpoint geometry.",
            "A lineage edge shows recorded molecular reuse, not necessarily a productive synthesis pathway.",
            "Active lifetimes are right-censored at the final snapshot.",
            "Material totals across lineage generations can count the same conserved constituents more than once.",
        ],
    }


def render_markdown(report: dict[str, Any]) -> str:
    exp = report["experiment"]
    lines = [
        "# EvoSim environmental chemistry experiment",
        "",
        f"- Seeds: {', '.join(map(str, exp['seeds']))}",
        f"- Duration: {exp['duration_seconds_per_seed']} seconds per seed",
        f"- Total simulated time: {exp['total_simulated_seconds']} seconds",
        f"- Condition: {exp['condition']}",
        f"- Initial environment: {exp['initial_environment']}",
        "",
        "## Reaction totals across seeds",
        "",
        "| Reaction | Ready | Scheduled | Committed | Rejected |",
        "|---|---:|---:|---:|---:|",
    ]
    for name, row in report["reaction_totals"].items():
        lines.append(f"| {name} | {row['ready']} | {row['scheduled']} | {row['committed']} | {row['rejected']} |")
    lines += [
        "",
        "## Molecular development",
        "",
        f"- Molecule identities created: {report['molecular_development']['molecules_created_total']}",
        f"- Molecule identities destroyed: {report['molecular_development']['molecules_destroyed_total']}",
        f"- Mean active molecules per seed at end: {report['molecular_development']['active_molecules_mean_per_seed']:.2f}",
        f"- Recorded lineage edges: {report['molecular_development']['lineage_edges_total']}",
        f"- Reaction-created products reused as parents: {report['molecular_development']['reaction_created_products_reused_as_parents']}",
        f"- Reused products followed by COMBINE: {report['molecular_development']['reused_products_followed_by_combine']}",
        "",
        "## Molecular organization",
        "",
        f"- Mean exact structural families per seed: {report['organization']['unique_structural_families_mean_per_seed']:.2f}",
        f"- Mean recurrent reaction-product families per seed: {report['organization']['recurrent_product_families_mean_per_seed']:.2f}",
        f"- Mean compound identities per seed: {report['organization']['compound_identities_mean_per_seed']:.2f}",
        "",
        "### Product families recurring across seeds",
        "",
        "| Seeds | Reaction-created products | Product reaction events | Active at end | Composition |",
        "|---:|---:|---:|---:|---|",
    ]
    for row in report["organization"]["cross_seed_recurrent_product_families"]:
        composition = ", ".join(f"{key}:{value:g}" for key, value in sorted(row["composition"].items()))
        lines.append(f"| {row['seed_count']} | {row['reaction_created_occurrences']} | {row['reaction_created_events']} | {row['active_reaction_created_occurrences']} | {composition} |")
    if not report["organization"]["cross_seed_recurrent_product_families"]:
        lines.append("| — | — | — | — | None observed in multiple seeds |")
    lines += [
        "",
        "## Exact elemental bond-pair changes",
        "",
        f"- Committed events with graph deltas: {report['elemental_bond_pairs']['committed_events_with_graph_deltas']}",
        f"- Committed events missing graph deltas: {report['elemental_bond_pairs']['committed_events_missing_graph_deltas']}",
        f"- Malformed bond records: {report['elemental_bond_pairs']['malformed_bond_records']}",
        f"- Formed bonds: {report['elemental_bond_pairs']['formed_total']}",
        f"- Broken bonds: {report['elemental_bond_pairs']['broken_total']}",
        f"- Initial-to-final bond inventory reconciles in every seed: {report['elemental_bond_pairs']['inventory_reconciliation_all_seeds_match']}",
        "",
        "| Element pair | Initial | Formed | Broken | Final |",
        "|---|---:|---:|",
    ]
    for pair in report["elemental_bond_pairs"]["element_pairs"]:
        initial = report["elemental_bond_pairs"]["initial_bond_inventory"].get(pair, 0)
        formed = report["elemental_bond_pairs"]["formed"].get(pair, 0)
        broken = report["elemental_bond_pairs"]["broken"].get(pair, 0)
        final = report["elemental_bond_pairs"]["final_bond_inventory"].get(pair, 0)
        lines.append(f"| {pair} | {initial} | {formed} | {broken} | {final} |")
    for reaction, values in report["elemental_bond_pairs"]["by_reaction"].items():
        lines += [
            "",
            f"### {reaction} bond-pair changes",
            "",
            "| Element pair | Formed | Broken |",
            "|---|---:|---:|",
        ]
        for pair in report["elemental_bond_pairs"]["element_pairs"]:
            formed = values["formed"].get(pair, 0)
            broken = values["broken"].get(pair, 0)
            lines.append(f"| {pair} | {formed} | {broken} |")
    lines += [
        "",
        "## Energy and displacement",
        "",
        f"- Worst absolute snapshot energy residual: {report['energy']['maximum_absolute_snapshot_residual_across_runs']}",
        f"- Total displacement discovery-budget-exhausted ticks: {report['displacement']['budget_exhausted_ticks_total']}",
        "",
        "## Validation",
        "",
        f"- All requested seeds present: {report['validation']['all_requested_seeds_present']}",
        f"- All runs match requested duration: {report['validation']['all_runs_match_requested_duration']}",
        f"- All runs pass inventory and lifecycle checks: {report['validation']['all_runs_pass_inventory_and_lifecycle_checks']}",
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
    parser.add_argument("--seeds", required=True, help="comma-separated seed values")
    parser.add_argument("--duration-seconds", required=True, type=int)
    parser.add_argument("--json-out", type=Path, required=True)
    parser.add_argument("--markdown-out", type=Path, required=True)
    args = parser.parse_args()
    try:
        seeds = [int(item.strip()) for item in args.seeds.split(",") if item.strip()]
        if not seeds or len(seeds) != len(set(seeds)):
            raise ValueError("seeds must be a nonempty list of unique integers")
        report = build_report(args.input_dir, seeds, args.duration_seconds)
    except (OSError, ValueError, KeyError, TypeError) as exc:
        print(f"chemistry experiment batch analyzer: {exc}", file=sys.stderr)
        return 2
    args.json_out.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    args.markdown_out.write_text(render_markdown(report), encoding="utf-8")
    print(render_markdown(report), end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
