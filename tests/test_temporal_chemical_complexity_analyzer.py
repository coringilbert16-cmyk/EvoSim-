import json
import tempfile
import unittest
from pathlib import Path

from tools.temporal_chemical_complexity_analyzer import (
    analyze_seed,
    graph_metrics,
    strongly_connected_components,
    structural_fingerprint,
)


class TemporalChemicalComplexityTests(unittest.TestCase):
    def test_fingerprint_is_invariant_to_part_order(self):
        first = {
            "material": {
                "parts": [["C", 1], ["H", 1], ["O", 1]],
                "internal_bonds": [{"part_a": 0, "part_b": 1}, {"part_a": 1, "part_b": 2}],
            }
        }
        reordered = {
            "material": {
                "parts": [["O", 1], ["C", 1], ["H", 1]],
                "internal_bonds": [{"part_a": 1, "part_b": 2}, {"part_a": 2, "part_b": 0}],
            }
        }
        self.assertEqual(structural_fingerprint(first), structural_fingerprint(reordered))

    def test_graph_metrics_report_chain_structure(self):
        molecule = {
            "material": {
                "parts": [["C", 1], ["H", 1], ["O", 1], ["S", 1]],
                "internal_bonds": [
                    {"part_a": 0, "part_b": 1},
                    {"part_a": 1, "part_b": 2},
                    {"part_a": 2, "part_b": 3},
                ],
            }
        }
        metrics = graph_metrics(molecule)
        self.assertEqual(metrics["constituents"], 4)
        self.assertEqual(metrics["bonds"], 3)
        self.assertEqual(metrics["graph_diameter"], 3)
        self.assertEqual(metrics["branch_points"], 0)
        self.assertEqual(metrics["independent_cycles"], 0)

    def test_graph_metrics_detect_branch_and_cycle(self):
        molecule = {
            "material": {
                "parts": [["C", 1], ["H", 1], ["O", 1], ["S", 1]],
                "internal_bonds": [
                    {"part_a": 0, "part_b": 1},
                    {"part_a": 0, "part_b": 2},
                    {"part_a": 0, "part_b": 3},
                    {"part_a": 1, "part_b": 2},
                ],
            }
        }
        metrics = graph_metrics(molecule)
        self.assertEqual(metrics["branch_points"], 1)
        self.assertEqual(metrics["independent_cycles"], 1)

    def test_strongly_connected_components_find_cycle(self):
        nodes = {"A", "B", "C", "D"}
        edges = {("A", "B"), ("B", "C"), ("C", "A"), ("C", "D")}
        components = strongly_connected_components(nodes, edges)
        self.assertIn(["A", "B", "C"], components)
        self.assertIn(["D"], components)


    def test_large_network_scc_does_not_depend_on_recursion(self):
        nodes = {str(index) for index in range(1500)}
        edges = {(str(index), str(index + 1)) for index in range(1499)}
        edges.add(("1499", "0"))
        components = strongly_connected_components(nodes, edges)
        self.assertEqual(len(components), 1)
        self.assertEqual(len(components[0]), 1500)

    def test_large_molecule_uses_bounded_diameter_estimate(self):
        parts = [["C", 1] for _ in range(129)]
        bonds = [{"part_a": index, "part_b": index + 1} for index in range(128)]
        metrics = graph_metrics({"material": {"parts": parts, "internal_bonds": bonds}})
        self.assertEqual(metrics["graph_diameter"], 128)
        self.assertFalse(metrics["graph_diameter_exact"])

    def test_time_windows_track_compounds_not_single_atom_fragments(self):
        def molecule(parts, bonds):
            return {
                "material": {
                    "parts": [[element, 1.0] for element in parts],
                    "internal_bonds": [
                        {"part_a": a, "part_b": b} for a, b in bonds
                    ],
                }
            }

        data = {
            "input": {"seed": 42, "requested_duration_seconds": 120},
            "molecular_history": {
                "validation_errors": [],
                "molecules": [
                    {
                        "molecule_id": 1, "created_tick": 0, "destroyed_tick": None,
                        "observed_lifetime_ticks": 120, "reaction": "INITIAL",
                        "parent_ids": [], "molecule": molecule(["C"], []),
                    },
                    {
                        "molecule_id": 2, "created_tick": 0, "destroyed_tick": None,
                        "observed_lifetime_ticks": 120, "reaction": "INITIAL",
                        "parent_ids": [], "molecule": molecule(["C", "H"], [(0, 1)]),
                    },
                    {
                        "molecule_id": 3, "created_tick": 30, "destroyed_tick": None,
                        "observed_lifetime_ticks": 90, "reaction": "COMBINE",
                        "parent_ids": [2], "molecule": molecule(["C", "H", "O"], [(0, 1), (1, 2)]),
                    },
                    {
                        "molecule_id": 4, "created_tick": 90, "destroyed_tick": None,
                        "observed_lifetime_ticks": 30, "reaction": "BREAK",
                        "parent_ids": [3], "molecule": molecule(["C", "H"], [(0, 1)]),
                    },
                    {
                        "molecule_id": 5, "created_tick": 30, "destroyed_tick": None,
                        "observed_lifetime_ticks": 90, "reaction": "BREAK",
                        "parent_ids": [2], "molecule": molecule(["O"], []),
                    },
                ],
                "lineage_edges": [
                    {"parent_id": 2, "child_id": 3, "reaction_id": "30-1", "reaction": "COMBINE", "tick": 30},
                    {"parent_id": 2, "child_id": 5, "reaction_id": "30-2", "reaction": "BREAK", "tick": 30},
                    {"parent_id": 3, "child_id": 4, "reaction_id": "90-1", "reaction": "BREAK", "tick": 90},
                ],
            },
        }
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "seed-42-chemistry.json"
            path.write_text(json.dumps(data), encoding="utf-8")
            report = analyze_seed(path, seed=42, duration=120, windows=12)

        window_three = report["temporal_windows"][2]
        window_nine = report["temporal_windows"][8]
        self.assertEqual(window_three["created_reaction_product_identities"], 2)
        self.assertEqual(window_three["created_compound_products"], 1)
        self.assertEqual(window_three["new_structural_fingerprints"], 1)
        self.assertEqual(window_nine["created_compound_products"], 1)
        self.assertEqual(window_nine["repeated_product_identities"], 1)
        self.assertEqual(window_nine["reaction_network_cyclic_scc_count"], 1)

if __name__ == "__main__":
    unittest.main()
