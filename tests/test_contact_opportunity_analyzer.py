import json
import tempfile
import unittest
from pathlib import Path

from tools.contact_opportunity_analyzer import analyze


def write_tick(root: Path, reaction_work: dict) -> dict:
    row = {
        "event": "environmental_chemistry_tick",
        "tick": 10,
        "reaction_work": reaction_work,
    }
    (root / "seed-42.jsonl").write_text(json.dumps(row) + "\n", encoding="utf-8")
    return row


class ContactOpportunityAnalyzerTests(unittest.TestCase):
    def test_aggregates_bounded_combine_search_as_partial(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_tick(root, {
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
            })
            report = analyze(root, [42], 60)

        self.assertEqual(report["pooled_totals"]["constituent_pairs_available"], 12)
        self.assertEqual(report["pooled_totals"]["constituent_pairs_without_valid_interfaces"], 6)
        self.assertEqual(report["pooled_totals"]["endpoint_pair_candidates_rejected_by_evaluation"], 2)
        self.assertEqual(report["pooled_totals"]["combine_committed"], 1)
        self.assertEqual(report["conversion"]["inspected_pairs_with_valid_interfaces"], 4)
        self.assertEqual(report["tick_diagnostics"][0]["coverage"]["combine"]["state"], "partial")

    def test_complete_combine_search_requires_reconciled_counters_and_budget_status(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_tick(root, {"combine_search": {
                "constituent_pairs_available": 0,
                "constituent_pairs_inspected": 0,
                "constituent_pairs_uninspected_by_window": 0,
                "budget_exhausted": False,
            }})
            report = analyze(root, [42], 60)

        self.assertEqual(report["tick_diagnostics"][0]["coverage"]["combine"]["state"], "complete")
        self.assertEqual(report["pooled_totals"]["constituent_pairs_available"], 0)
        self.assertIsNone(report["conversion"]["valid_interface_fraction_of_inspected_pairs"])

    def test_missing_legacy_fields_remain_unknown_not_zero(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_tick(root, {"combine_search": {"budget_exhausted": False}})
            report = analyze(root, [42], 60)

        self.assertIsNone(report["tick_diagnostics"][0]["constituent_pairs_available"])
        self.assertIsNone(report["pooled_totals"]["constituent_pairs_available"])
        self.assertEqual(report["pooled_known_subtotals"]["constituent_pairs_available"], 0)
        self.assertEqual(report["missing_tick_counts"]["constituent_pairs_available"], 1)
        self.assertIsNone(report["conversion"]["valid_interface_fraction_of_inspected_pairs"])
        self.assertEqual(report["tick_diagnostics"][0]["coverage"]["combine"]["state"], "unknown")

    def test_break_coverage_uses_complete_bond_index_domain(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_tick(root, {"break_discovery": {
                "scan_entries_inspected": 300,
                "bond_candidates_inspected": 300,
                "bond_candidates_checked": 298,
            }})
            report = analyze(root, [42], 60)

        coverage = report["tick_diagnostics"][0]["coverage"]["break"]
        self.assertEqual(coverage["state"], "complete")
        self.assertEqual(coverage["domain_count"], 300)

    def test_break_mismatch_cannot_claim_complete_coverage(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_tick(root, {"break_discovery": {
                "scan_entries_inspected": 300,
                "bond_candidates_inspected": 299,
            }})
            report = analyze(root, [42], 60)

        self.assertEqual(report["tick_diagnostics"][0]["coverage"]["break"]["state"], "unknown")

    def test_displacement_budget_exhaustion_is_partial_not_complete(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_tick(root, {
                "displacement_discovery": {
                    "anchor_cells_available": 100,
                    "anchor_cells_inspected": 64,
                    "anchor_cells_unvisited_by_window": 36,
                    "anchor_cells_not_reached_in_selected_window": 0,
                    "target_cell_windows_inspected": 12,
                    "target_cell_window_limit_reached": 0,
                    "replacement_contact_searches": 4,
                    "replacement_contacts_found": 0,
                },
                "displacement_discovery_work_remaining": 0,
            })
            report = analyze(root, [42], 60)

        self.assertEqual(report["tick_diagnostics"][0]["coverage"]["displacement"]["state"], "partial")
        self.assertEqual(report["pooled_totals"]["displacement_replacement_contacts_found"], 0)

    def test_displacement_with_remaining_budget_is_still_unknown(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_tick(root, {"displacement_discovery_work_remaining": 7})
            report = analyze(root, [42], 60)

        self.assertEqual(report["tick_diagnostics"][0]["coverage"]["displacement"]["state"], "unknown")


    def test_displacement_anchor_window_reports_partial_coverage(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_tick(root, {"displacement_discovery": {
                "anchor_cells_available": 100,
                "anchor_cells_inspected": 64,
                "anchor_cells_unvisited_by_window": 36,
                "anchor_cells_not_reached_in_selected_window": 0,
                "target_cell_windows_inspected": 10,
                "target_cell_window_limit_reached": 0,
            }, "displacement_discovery_work_remaining": 5})
            report = analyze(root, [42], 60)

        self.assertEqual(report["tick_diagnostics"][0]["coverage"]["displacement"]["state"], "partial")
        self.assertEqual(report["pooled_totals"]["displacement_anchor_cells_unvisited_by_window"], 36)

    def test_displacement_target_window_limit_is_not_misreported_as_complete(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_tick(root, {"displacement_discovery": {
                "anchor_cells_available": 10,
                "anchor_cells_inspected": 10,
                "anchor_cells_unvisited_by_window": 0,
                "anchor_cells_not_reached_in_selected_window": 0,
                "target_cell_windows_inspected": 10,
                "target_cell_window_limit_reached": 2,
            }, "displacement_discovery_work_remaining": 5})
            report = analyze(root, [42], 60)

        self.assertEqual(report["tick_diagnostics"][0]["coverage"]["displacement"]["state"], "unknown")


    def test_legacy_displacement_zero_remaining_is_partial(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_tick(root, {"displacement_discovery_work_remaining": 0})
            report = analyze(root, [42], 60)

        self.assertEqual(report["tick_diagnostics"][0]["coverage"]["displacement"]["state"], "partial")


    def test_no_diagnostic_ticks_does_not_become_zero_totals(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "seed-42.jsonl").write_text(
                json.dumps({"event": "environmental_chemistry_summary"}) + "\n",
                encoding="utf-8",
            )
            report = analyze(root, [42], 60)

        self.assertIsNone(report["pooled_totals"]["constituent_pairs_available"])
        self.assertEqual(report["pooled_known_subtotals"]["constituent_pairs_available"], 0)
        self.assertEqual(report["coverage"]["combine"]["state"], "unknown")


    def test_empty_break_bond_index_is_complete(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_tick(root, {"break_discovery": {
                "scan_entries_inspected": 0,
                "bond_candidates_inspected": 0,
            }})
            report = analyze(root, [42], 60)

        coverage = report["tick_diagnostics"][0]["coverage"]["break"]
        self.assertEqual(coverage["state"], "complete")
        self.assertEqual(coverage["domain_count"], 0)

    def test_pooled_coverage_preserves_mixed_tick_states(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            records = [
                {
                    "event": "environmental_chemistry_tick",
                    "tick": 1,
                    "reaction_work": {
                        "combine_search": {
                            "constituent_pairs_available": 0,
                            "constituent_pairs_inspected": 0,
                            "constituent_pairs_uninspected_by_window": 0,
                            "budget_exhausted": False,
                        },
                        "break_discovery": {
                            "scan_entries_inspected": 0,
                            "bond_candidates_inspected": 0,
                        },
                        "displacement_discovery_work_remaining": 0,
                    },
                },
                {
                    "event": "environmental_chemistry_tick",
                    "tick": 2,
                    "reaction_work": {
                        "combine_search": {
                            "constituent_pairs_available": 2,
                            "constituent_pairs_inspected": 1,
                            "constituent_pairs_uninspected_by_window": 1,
                            "budget_exhausted": True,
                        },
                    },
                },
            ]
            (root / "seed-42.jsonl").write_text(
                "".join(json.dumps(record) + "\n" for record in records),
                encoding="utf-8",
            )
            report = analyze(root, [42], 60)

        self.assertEqual(report["coverage"]["combine"]["complete_ticks"], 1)
        self.assertEqual(report["coverage"]["combine"]["partial_ticks"], 1)
        self.assertEqual(report["coverage"]["combine"]["state"], "partial")
        self.assertEqual(report["coverage"]["combine"]["search_domain"], "broad_phase_constituent_pairs")
        self.assertEqual(report["coverage"]["break"]["complete_ticks"], 1)
        self.assertEqual(report["coverage"]["break"]["unknown_ticks"], 1)
        self.assertEqual(report["coverage"]["break"]["state"], "unknown")
        self.assertEqual(report["coverage"]["displacement"]["partial_ticks"], 1)
        self.assertEqual(report["coverage"]["displacement"]["unknown_ticks"], 1)

    def test_missing_seed_artifact_fails_clearly(self):
        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaisesRegex(ValueError, "missing raw observation"):
                analyze(Path(temporary), [42], 60)


if __name__ == "__main__":
    unittest.main()
