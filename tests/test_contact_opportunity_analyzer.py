import json
import tempfile
import unittest
from pathlib import Path

from tools.contact_opportunity_analyzer import analyze


class ContactOpportunityAnalyzerTests(unittest.TestCase):
    def test_aggregates_broad_phase_to_reaction_funnel(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            row = {
                "event": "environmental_chemistry_tick",
                "tick": 10,
                "reaction_work": {
                    "combine_search": {
                        "constituent_pairs_available": 12,
                        "constituent_pairs_inspected": 10,
                        "constituent_pairs_uninspected_by_window": 2,
                        "constituent_pairs_with_valid_interfaces": 4,
                        "constituent_pairs_without_valid_interfaces": 6,
                        "physically_evaluable_interfaces": 5,
                        "endpoint_pair_candidates_generated": 7,
                        "endpoint_pair_candidates_rejected_by_evaluation": 2,
                        "interfaces_sent_to_chemistry": 5,
                        "budget_exhausted": True,
                    },
                    "combine_ready_candidates": 1,
                    "combine_scheduled": 1,
                    "combine_committed": 1,
                },
            }
            (root / "seed-42.jsonl").write_text(json.dumps(row) + "\n", encoding="utf-8")
            report = analyze(root, [42], 1500)

        self.assertEqual(report["pooled_totals"]["constituent_pairs_available"], 12)
        self.assertEqual(report["pooled_totals"]["constituent_pairs_without_valid_interfaces"], 6)
        self.assertEqual(report["pooled_totals"]["endpoint_pair_candidates_rejected_by_evaluation"], 2)
        self.assertEqual(report["pooled_totals"]["combine_committed"], 1)
        self.assertEqual(report["conversion"]["inspected_pairs_with_valid_interfaces"], 4)
        self.assertEqual(report["tick_diagnostics"][0]["discovery_budget_exhausted"], True)

    def test_captures_local_neighbors_and_joins_exact_reaction_outcome(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            endpoint_a = {"Boundary": {"angle_radians": 0.25}}
            endpoint_b = {"Boundary": {"angle_radians": 3.0}}
            sample = {
                "source_a_identity": 101,
                "source_b_identity": 202,
                "unit_a": 0,
                "unit_b": 1,
                "endpoint_a": endpoint_a,
                "endpoint_b": endpoint_b,
                "physically_valid": True,
                "environmental_usable_energy": 12.5,
                "environmental_resonant_energy": 7.0,
                "local_neighborhood": {
                    "radius": 50.0,
                    "total_molecule_count": 1,
                    "recorded_molecule_count": 1,
                    "truncated_molecule_count": 0,
                    "missing_geometry_count": 0,
                    "molecules": [{
                        "identity": 303,
                        "elemental_counts": [0, 1, 0, 1, 1, 0],
                        "unclassified_constituents": 0,
                        "constituent_count": 3,
                        "topology_signature": 987,
                        "center_x": 8.0,
                        "center_y": 9.0,
                        "distance_from_contact": 4.5,
                    }],
                },
            }
            reaction = {
                "event": "environmental_reaction",
                "reaction": {
                    "tick": 11,
                    "reaction": "COMBINE",
                    "source_material_ids": [101, 202],
                    "contact_unit_indices": [0, 1],
                    "contact_endpoints": [endpoint_a, endpoint_b],
                    "outcome": "committed",
                    "failure_reason": None,
                },
            }
            tick = {
                "event": "environmental_chemistry_tick",
                "tick": 10,
                "reaction_work": {
                    "combine_search": {"opportunity_samples": [sample]},
                },
            }
            (root / "seed-42.jsonl").write_text(
                json.dumps(reaction) + "\n" + json.dumps(tick) + "\n",
                encoding="utf-8",
            )
            report = analyze(root, [42], 300)

        observed = report["opportunity_samples"][0]
        self.assertEqual(observed["exact_transaction_match_count"], 1)
        self.assertEqual(observed["matching_reaction_outcomes"][0]["outcome"], "committed")
        self.assertEqual(report["local_neighborhood_summary"]["COMBINE"]["neighbors_recorded"], 1)
        self.assertEqual(report["local_neighborhood_summary"]["COMBINE"]["samples_with_exact_transaction_match"], 1)
        self.assertEqual(report["neighborhood_samples"][0]["neighbor_identity"], 303)
        self.assertEqual(report["neighborhood_samples"][0]["phosphorus_count"], 1)

    def test_missing_seed_artifact_fails_clearly(self):
        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaisesRegex(ValueError, "missing raw observation"):
                analyze(Path(temporary), [42], 1500)


if __name__ == "__main__":
    unittest.main()
