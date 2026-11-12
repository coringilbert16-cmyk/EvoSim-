import json
import tempfile
import unittest
from pathlib import Path

from tools.reaction_network_analyzer import analyze, family_key


def molecule(parts, edges, identity):
    return {
        "environmental_identity": identity,
        "material": {
            "parts": [[name, 1.0] for name in parts],
            "internal_bonds": [{"part_a": a, "part_b": b} for a, b in edges],
        },
        "internal_connections": [
            {"part_a": a, "part_b": b, "bond_energy": 0.01} for a, b in edges
        ],
    }


def write_seed(root: Path, seed: int) -> None:
    rows = [
        (1, "INITIAL", 0, 2, 2, False, molecule(["Carbon", "Hydrogen"], [(0, 1)], 1)),
        (2, "INITIAL", 0, 2, 2, False, molecule(["Oxygen", "Sulfur"], [(0, 1)], 2)),
        (3, "COMBINE", 1, 3, 2, False, molecule(
            ["Carbon", "Hydrogen", "Oxygen", "Sulfur"], [(0, 1), (1, 2), (2, 3)], 3)),
        (4, "COMBINE", 2, None, 3, True, molecule(
            ["Carbon", "Hydrogen", "Oxygen", "Sulfur"], [(0, 1), (1, 2), (2, 3)], 4)),
        (5, "INITIAL", 0, 3, 3, False, molecule(["Phosphorus", "Sulfur"], [(0, 1)], 5)),
    ]
    molecules = [{
        "molecule_id": identity, "reaction": reaction, "reaction_id": "initial" if reaction == "INITIAL" else str(tick),
        "created_tick": tick, "destroyed_tick": destroyed, "observed_lifetime_ticks": lifetime,
        "right_censored": censored, "molecule": value,
    } for identity, reaction, tick, destroyed, lifetime, censored, value in rows]
    chemistry = {
        "input": {
            "seed": seed, "requested_duration_seconds": 300,
            "initial_physical_material_count": 1280,
            "initial_independent_resource_count": 0,
            "initial_molecule_count": 1280, "initial_constituent_count": 2560,
        },
        "reactions": {
            "elemental_bond_pairs": {
                "events_missing_reaction_id": 0,
                "event_attribution": [
                    {"reaction_id": "1-1", "reaction": "COMBINE", "tick": 1,
                     "formed": {"Carbon-Oxygen": 1}, "broken": {}},
                    {"reaction_id": "2-1", "reaction": "COMBINE", "tick": 2,
                     "formed": {"Hydrogen-Sulfur": 1}, "broken": {}},
                ],
            },
        },
        "molecular_history": {
            "validation_errors": [], "molecules": molecules, "active_at_end_count": 1,
            "mean_observed_lifetime_ticks": 2.4,
            "lineage_edges": [
                {"parent_id": 1, "child_id": 3, "reaction_id": "1-1", "reaction": "COMBINE", "tick": 1},
                {"parent_id": 2, "child_id": 3, "reaction_id": "1-1", "reaction": "COMBINE", "tick": 1},
                {"parent_id": 3, "child_id": 4, "reaction_id": "2-1", "reaction": "COMBINE", "tick": 2},
                {"parent_id": 5, "child_id": 4, "reaction_id": "2-1", "reaction": "COMBINE", "tick": 2},
            ],
        },
    }
    (root / f"seed-{seed}-chemistry.json").write_text(json.dumps(chemistry), encoding="utf-8")


class ReactionNetworkAnalyzerTests(unittest.TestCase):
    def test_family_key_ignores_part_order_but_preserves_topology(self):
        first = molecule(["Carbon", "Hydrogen", "Oxygen", "Sulfur"],
                         [(0, 1), (1, 2), (2, 3)], 1)
        reordered = molecule(["Sulfur", "Oxygen", "Hydrogen", "Carbon"],
                             [(3, 2), (2, 1), (1, 0)], 2)
        star = molecule(["Carbon", "Hydrogen", "Oxygen", "Sulfur"],
                        [(0, 1), (0, 2), (0, 3)], 3)
        self.assertEqual(family_key(first), family_key(reordered))
        self.assertNotEqual(family_key(first), family_key(star))

    def test_reconstructs_multistep_reuse_and_cross_seed_pathways(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_seed(root, 42)
            write_seed(root, 43)
            report = analyze(root, [42, 43], 300)
        summary = report["summary"]
        self.assertEqual(summary["total_reaction_events"], 4)
        self.assertEqual(summary["total_lineage_edges"], 8)
        self.assertEqual(summary["reaction_created_products_reused_as_parents"], 2)
        self.assertEqual(summary["reaction_created_parent_lineage_edges"], 2)
        self.assertEqual(summary["reaction_created_parent_edges_followed_by_combine"], 2)
        self.assertEqual(summary["reaction_created_families_recurrent_in_2_or_more_seeds"], 1)
        self.assertEqual(summary["lineage_events_with_bond_pair_attribution"], 4)
        self.assertEqual(summary["lineage_events_missing_bond_pair_attribution"], 0)
        pathway = next(
            row for row in report["reaction_pathways"]
            if "Carbon-Hydrogen-Oxygen-Sulfur [0-1,1-2,2-3]" in row["parent_families"]
        )
        self.assertEqual(pathway["event_count"], 2)
        self.assertEqual(pathway["seed_count"], 2)
        self.assertEqual(pathway["formed_bond_pairs"], {"Hydrogen-Sulfur": 2})
        self.assertEqual(pathway["formed_bond_count"], 2)

    def test_rejects_incorrect_starting_inventory(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_seed(root, 42)
            path = root / "seed-42-chemistry.json"
            data = json.loads(path.read_text(encoding="utf-8"))
            data["input"]["initial_independent_resource_count"] = 1200
            path.write_text(json.dumps(data), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "initial_independent_resource_count"):
                analyze(root, [42], 300)


if __name__ == "__main__":
    unittest.main()
