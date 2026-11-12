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

    def test_missing_seed_artifact_fails_clearly(self):
        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaisesRegex(ValueError, "missing raw observation"):
                analyze(Path(temporary), [42], 1500)


if __name__ == "__main__":
    unittest.main()
