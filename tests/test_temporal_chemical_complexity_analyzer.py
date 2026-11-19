import unittest

from tools.temporal_chemical_complexity_analyzer import (
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

if __name__ == "__main__":
    unittest.main()
