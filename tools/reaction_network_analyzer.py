#!/usr/bin/env python3
"""Build a topology-aware reaction network from environmental chemistry artifacts."""
import argparse
import csv
import itertools
import json
import statistics
from collections import Counter, defaultdict
from pathlib import Path


def family_key(molecule):
    parts = molecule.get("material", {}).get("parts", [])
    labels = [str(part[0]) for part in parts]
    if not labels:
        raise ValueError("molecule has no constituents")
    raw = molecule.get("internal_connections") or molecule.get("material", {}).get("internal_bonds", [])
    edges = set()
    for bond in raw:
        a, b = int(bond["part_a"]), int(bond["part_b"])
        if a < 0 or b < 0 or a >= len(labels) or b >= len(labels) or a == b:
            raise ValueError("invalid bond endpoint")
        edges.add(tuple(sorted((a, b))))
    groups = [[i for i, name in enumerate(labels) if name == element] for element in sorted(set(labels))]
    best = None
    for groups_order in itertools.product(*(itertools.permutations(group) for group in groups)):
        order = [i for group in groups_order for i in group]
        remap = {old: new for new, old in enumerate(order)}
        candidate = (tuple(labels[i] for i in order),
                     tuple(sorted(tuple(sorted((remap[a], remap[b]))) for a, b in edges)))
        if best is None or candidate < best:
            best = candidate
    return json.dumps(best, separators=(",", ":"))


def family_label(key):
    labels, edges = json.loads(key)
    counts = Counter(labels)
    composition = "-".join(
        element if count == 1 else f"{element}{count}"
        for element, count in sorted(counts.items())
    )
    topology = ",".join(f"{a}-{b}" for a, b in edges) or "unbonded"
    return f"{composition} [{topology}]"


def load_seed(root, seed, duration):
    path = root / f"seed-{seed}-chemistry.json"
    if not path.is_file():
        raise ValueError(f"missing {path.name}")
    data = json.loads(path.read_text(encoding="utf-8"))
    source, history = data.get("input", {}), data.get("molecular_history", {})
    expected = {
        "seed": seed, "requested_duration_seconds": duration,
        "initial_physical_material_count": 1280,
        "initial_independent_resource_count": 0,
        "initial_molecule_count": 1280, "initial_constituent_count": 2560,
    }
    for key, value in expected.items():
        if source.get(key) != value:
            raise ValueError(f"seed {seed}: expected {key}={value}, got {source.get(key)}")
    if history.get("validation_errors"):
        raise ValueError(f"seed {seed}: lifecycle validation errors present")
    return data


def analyze(root, seeds, duration):
    families = defaultdict(lambda: {
        "seeds": set(), "created_seeds": set(), "identities": 0, "created": 0,
        "active_created": 0, "lifetimes": [], "created_lifetimes": [],
        "reused_ids": set(), "created_by_reaction": Counter(),
    })
    pathways = defaultdict(lambda: {
        "seeds": set(), "events": 0, "child_lifetimes": [], "reused_children": 0,
        "formed_pairs": Counter(), "broken_pairs": Counter(),
    })
    transitions = defaultdict(lambda: {
        "seeds": set(), "events": 0, "child_lifetimes": [], "reused_children": 0,
    })
    seed_rows = []
    for seed in seeds:
        data = load_seed(root, seed, duration)
        history = data["molecular_history"]
        molecules = {int(m["molecule_id"]): m for m in history.get("molecules", [])}
        keys = {identity: family_key(row["molecule"]) for identity, row in molecules.items()}
        pair_events = {
            str(row["reaction_id"]): row
            for row in data.get("reactions", {}).get("elemental_bond_pairs", {}).get("event_attribution", [])
            if row.get("reaction_id") is not None
        }
        events = defaultdict(lambda: {"parents": set(), "children": set(), "reaction": "", "tick": 0})
        for edge in history.get("lineage_edges", []):
            event = events[str(edge["reaction_id"])]
            event["parents"].add(int(edge["parent_id"]))
            event["children"].add(int(edge["child_id"]))
            event["reaction"], event["tick"] = str(edge["reaction"]), int(edge["tick"])

        reused_ids, reused_edges, reused_combine_edges = set(), 0, 0
        for edge in history.get("lineage_edges", []):
            parent, child = int(edge["parent_id"]), int(edge["child_id"])
            if parent in molecules and molecules[parent].get("reaction") != "INITIAL":
                reused_ids.add(parent)
                reused_edges += 1
                if child in molecules and molecules[child].get("reaction") == "COMBINE":
                    reused_combine_edges += 1

        for identity, molecule in molecules.items():
            key = keys[identity]
            row = families[key]
            row["seeds"].add(seed)
            row["identities"] += 1
            lifetime = int(molecule.get("observed_lifetime_ticks", 0) or 0)
            row["lifetimes"].append(lifetime)
            if molecule.get("reaction") != "INITIAL":
                row["created"] += 1
                row["created_seeds"].add(seed)
                row["created_lifetimes"].append(lifetime)
                row["created_by_reaction"][str(molecule.get("reaction", "UNKNOWN"))] += 1
                row["active_created"] += int(bool(molecule.get("right_censored")))
            if identity in reused_ids and molecule.get("reaction") != "INITIAL":
                row["reused_ids"].add((seed, identity))

        for reaction_id, event in events.items():
            parents, children = sorted(event["parents"]), sorted(event["children"])
            parent_keys = tuple(sorted(keys[i] for i in parents if i in keys))
            child_keys = tuple(sorted(keys[i] for i in children if i in keys))
            reaction = event["reaction"]
            path = pathways[(reaction, parent_keys, child_keys)]
            path["seeds"].add(seed)
            path["events"] += 1
            path["child_lifetimes"].extend(
                int(molecules[i].get("observed_lifetime_ticks", 0) or 0)
                for i in children if i in molecules
            )
            path["reused_children"] += sum(i in reused_ids for i in children)
            pair_event = pair_events.get(reaction_id)
            if pair_event is not None:
                path["formed_pairs"].update({
                    pair: int(count) for pair, count in pair_event.get("formed", {}).items()
                    if int(count) > 0
                })
                path["broken_pairs"].update({
                    pair: int(count) for pair, count in pair_event.get("broken", {}).items()
                    if int(count) > 0
                })
            for parent_key in set(parent_keys):
                for child_key in set(child_keys):
                    edge_row = transitions[(reaction, parent_key, child_key)]
                    edge_row["seeds"].add(seed)
                    edge_row["events"] += 1
                    edge_row["child_lifetimes"].extend(
                        int(molecules[i].get("observed_lifetime_ticks", 0) or 0)
                        for i in children if i in molecules and keys[i] == child_key
                    )
                    edge_row["reused_children"] += sum(
                        i in reused_ids and keys.get(i) == child_key for i in children
                    )

        seed_rows.append({
            "seed": seed, "molecule_identities": len(molecules),
            "reaction_events": len(events), "lineage_edges": len(history.get("lineage_edges", [])),
            "reaction_created_products_reused_as_parents": len(reused_ids),
            "reaction_created_parent_lineage_edges": reused_edges,
            "reaction_created_parent_edges_followed_by_combine": reused_combine_edges,
            "lineage_events_with_bond_pair_attribution": sum(
                str(event_id) in pair_events for event_id in events
            ),
            "lineage_events_missing_bond_pair_attribution": sum(
                str(event_id) not in pair_events for event_id in events
            ),
            "bond_pair_events_missing_reaction_id": data.get("reactions", {}).get("elemental_bond_pairs", {}).get("events_missing_reaction_id", 0),
            "active_molecules_at_end": history.get("active_at_end_count", 0),
            "mean_observed_lifetime_ticks": history.get("mean_observed_lifetime_ticks"),
        })

    family_rows = []
    for key, row in families.items():
        family_rows.append({
            "family_key": key, "family": family_label(key),
            "seed_count": len(row["seeds"]), "seeds": sorted(row["seeds"]),
            "reaction_created_seed_count": len(row["created_seeds"]),
            "reaction_created_seeds": sorted(row["created_seeds"]),
            "molecule_identities": row["identities"],
            "reaction_created_identities": row["created"],
            "reaction_created_active_at_end": row["active_created"],
            "reaction_created_mean_lifetime_ticks": statistics.fmean(row["created_lifetimes"]) if row["created_lifetimes"] else None,
            "reaction_created_median_lifetime_ticks": statistics.median(row["created_lifetimes"]) if row["created_lifetimes"] else None,
            "reused_product_identity_count": len(row["reused_ids"]),
            "created_by_reaction": dict(row["created_by_reaction"]),
        })
    family_rows.sort(key=lambda r: (
        -r["reaction_created_seed_count"], -r["reused_product_identity_count"],
        -r["reaction_created_identities"], r["family"],
    ))

    pathway_rows = []
    for (reaction, parents, children), row in pathways.items():
        pathway_rows.append({
            "reaction": reaction,
            "parent_families": [family_label(k) for k in parents],
            "child_families": [family_label(k) for k in children],
            "event_count": row["events"], "seed_count": len(row["seeds"]),
            "seeds": sorted(row["seeds"]),
            "mean_child_lifetime_ticks": statistics.fmean(row["child_lifetimes"]) if row["child_lifetimes"] else None,
            "reused_child_observations": row["reused_children"],
            "formed_bond_pairs": dict(sorted(row["formed_pairs"].items())),
            "broken_bond_pairs": dict(sorted(row["broken_pairs"].items())),
            "formed_bond_count": sum(row["formed_pairs"].values()),
            "broken_bond_count": sum(row["broken_pairs"].values()),
        })
    pathway_rows.sort(key=lambda r: (
        -r["seed_count"], -r["reused_child_observations"], -r["event_count"],
        r["reaction"], r["parent_families"], r["child_families"],
    ))

    transition_rows = []
    for (reaction, parent, child), row in transitions.items():
        transition_rows.append({
            "reaction": reaction, "parent_family": family_label(parent),
            "child_family": family_label(child), "event_count": row["events"],
            "seed_count": len(row["seeds"]), "seeds": sorted(row["seeds"]),
            "mean_child_lifetime_ticks": statistics.fmean(row["child_lifetimes"]) if row["child_lifetimes"] else None,
            "reused_child_observations": row["reused_children"],
        })
    transition_rows.sort(key=lambda r: (
        -r["seed_count"], -r["reused_child_observations"], -r["event_count"],
        r["reaction"], r["parent_family"], r["child_family"],
    ))

    summary = {
        "total_molecule_identities": sum(r["molecule_identities"] for r in seed_rows),
        "total_reaction_events": sum(r["reaction_events"] for r in seed_rows),
        "total_lineage_edges": sum(r["lineage_edges"] for r in seed_rows),
        "reaction_created_products_reused_as_parents": sum(r["reaction_created_products_reused_as_parents"] for r in seed_rows),
        "reaction_created_parent_lineage_edges": sum(r["reaction_created_parent_lineage_edges"] for r in seed_rows),
        "reaction_created_parent_edges_followed_by_combine": sum(r["reaction_created_parent_edges_followed_by_combine"] for r in seed_rows),
        "lineage_events_with_bond_pair_attribution": sum(r["lineage_events_with_bond_pair_attribution"] for r in seed_rows),
        "lineage_events_missing_bond_pair_attribution": sum(r["lineage_events_missing_bond_pair_attribution"] for r in seed_rows),
        "bond_pair_events_missing_reaction_id": sum(r["bond_pair_events_missing_reaction_id"] for r in seed_rows),
        "unique_composition_topology_families": len(family_rows),
        "reaction_created_families_recurrent_in_2_or_more_seeds": sum(r["reaction_created_seed_count"] >= 2 for r in family_rows),
        "distinct_pathway_patterns": len(pathway_rows),
        "pathways_recurrent_in_2_or_more_seeds": sum(r["seed_count"] >= 2 for r in pathway_rows),
    }
    return {
        "schema_version": 1,
        "experiment": {
            "seeds": seeds, "duration_seconds_per_seed": duration,
            "starting_inventory": "1,280 established two-constituent molecules; zero independent resources; 2,560 constituents; six active elements; Water excluded",
            "family_definition": "exact constituent composition plus undirected bond topology; excludes bond energy, endpoint geometry, placement, and environmental identity",
        },
        "validation": {"all_seeds_present": len(seed_rows) == len(seeds), "all_runs_match_1280_molecule_start": True, "all_lifecycle_checks_pass": True},
        "summary": summary, "seed_summaries": seed_rows,
        "compound_families": family_rows,
        "reaction_created_families": [r for r in family_rows if r["reaction_created_identities"] > 0],
        "reaction_pathways": pathway_rows, "family_transitions": transition_rows,
        "interpretation_limits": [
            "Reaction event patterns are grouped by recorded reaction_id; they are not thermodynamic reaction equations.",
            "Family identity uses composition and bond topology only; bond energy and endpoint geometry remain separate properties.",
            "Observed lifetimes are right-censored for molecules still active at the final snapshot.",
            "The five seed runs, not individual reaction events, are the independent experimental replicates.",
            "A lineage edge records participation, not proof of catalysis or net energy yield.",
        ],
    }


def write_csv(path, rows):
    if not rows:
        path.write_text("", encoding="utf-8")
        return
    with path.open("w", newline="", encoding="utf-8") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
        writer.writeheader()
        for row in rows:
            writer.writerow({
                k: json.dumps(v, separators=(",", ":")) if isinstance(v, (dict, list)) else v
                for k, v in row.items()
            })


def markdown(report):
    s = report["summary"]
    lines = [
        "# EvoSim reaction-network analysis", "",
        f"- Seeds: {', '.join(map(str, report['experiment']['seeds']))}",
        f"- Duration: {report['experiment']['duration_seconds_per_seed']} seconds per seed",
        f"- Starting inventory: {report['experiment']['starting_inventory']}",
        f"- Family definition: {report['experiment']['family_definition']}", "",
        "## Network coverage", "",
        f"- Molecule identities: {s['total_molecule_identities']}",
        f"- Reconstructed reaction events: {s['total_reaction_events']}",
        f"- Parent-child lineage edges: {s['total_lineage_edges']}",
        f"- Distinct reaction-created products reused as parents: {s['reaction_created_products_reused_as_parents']}",
        f"- Parent-lineage edges from reaction-created products: {s['reaction_created_parent_lineage_edges']}",
        f"- Those edges leading to COMBINE products: {s['reaction_created_parent_edges_followed_by_combine']}",
        f"- Lineage events matched to exact bond-pair attribution: {s['lineage_events_with_bond_pair_attribution']}",
        f"- Lineage events missing bond-pair attribution: {s['lineage_events_missing_bond_pair_attribution']}",
        f"- Bond-pair events missing reaction IDs: {s['bond_pair_events_missing_reaction_id']}",
        f"- Composition/topology families: {s['unique_composition_topology_families']}",
        f"- Reaction-created families recurring in 2+ seeds: {s['reaction_created_families_recurrent_in_2_or_more_seeds']}",
        f"- Distinct pathway patterns: {s['distinct_pathway_patterns']}",
        f"- Pathways recurring in 2+ seeds: {s['pathways_recurrent_in_2_or_more_seeds']}", "",
        "## Highest-priority reaction-created families", "",
        "| Family | Seeds | Created identities | Reused identities | Active created | Mean lifetime (ticks) |",
        "|---|---:|---:|---:|---:|---:|",
    ]
    for r in report["reaction_created_families"][:30]:
        mean = r["reaction_created_mean_lifetime_ticks"]
        mean_text = f"{mean:.1f}" if mean is not None else "—"
        lines.append(f"| {r['family']} | {r['reaction_created_seed_count']} | {r['reaction_created_identities']} | {r['reused_product_identity_count']} | {r['reaction_created_active_at_end']} | {mean_text} |")
    lines += ["", "## Recurrent reaction pathways", "",
              "| Reaction | Parent families | Child families | Events | Seeds | Reused child identities | Mean child lifetime (ticks) |",
              "|---|---|---|---:|---:|---:|---:|"]
    for r in report["reaction_pathways"][:40]:
        mean = r["mean_child_lifetime_ticks"]
        mean_text = f"{mean:.1f}" if mean is not None else "—"
        lines.append(f"| {r['reaction']} | {' + '.join(r['parent_families'])} | {' + '.join(r['child_families'])} | {r['event_count']} | {r['seed_count']} | {r['reused_child_observations']} | {mean_text} |")
    lines += ["", "## Interpretation limits", ""]
    lines.extend(f"- {item}" for item in report["interpretation_limits"])
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input_dir", type=Path)
    parser.add_argument("--seeds", required=True)
    parser.add_argument("--duration-seconds", type=int, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    seeds = [int(x.strip()) for x in args.seeds.split(",") if x.strip()]
    if not seeds or len(seeds) != len(set(seeds)):
        parser.error("--seeds must be a nonempty list of unique integers")
    report = analyze(args.input_dir, seeds, args.duration_seconds)
    args.output_dir.mkdir(parents=True, exist_ok=True)
    (args.output_dir / "reaction-network.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    (args.output_dir / "reaction-network.md").write_text(markdown(report), encoding="utf-8")
    write_csv(args.output_dir / "compound-families.csv", report["compound_families"])
    write_csv(args.output_dir / "reaction-pathways.csv", report["reaction_pathways"])
    write_csv(args.output_dir / "family-transitions.csv", report["family_transitions"])
    print(markdown(report), end="")


if __name__ == "__main__":
    main()
