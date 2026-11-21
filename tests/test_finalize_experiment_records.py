import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from tools import finalize_experiment_records as finalizer


def sample_record(seed):
    return {
        "schema": "evosim.permanent_experiment_record",
        "schema_version": 1,
        "record_status": "complete",
        "identity": {"experiment_id": "EXP-2026-099", "research_plan": "OBS-001",
                     "condition_id": "baseline", "seed": seed},
        "provenance": {"source_commit": "abc123", "simulator_version": "1.0",
                       "workflow_run_id": "123", "workflow_run_url": "https://example.test/run/123",
                       "configuration_sha256": "cfg", "comparison_configuration_sha256": "cmp",
                       "manifest_schema_version": 1, "chemistry_analyzer_sha256": "chem",
                       "organization_analyzer_sha256": "org", "enabled_diagnostic_tiers": []},
        "conditions": {"configuration": {"duration": 60}, "requested_duration_seconds": 60,
                       "actual_elapsed_wall_seconds": 61.0, "start_tick": 0, "end_tick": 600,
                       "tick_count": 600, "diagnostic_interval": 100, "active_resources": ["C", "H"]},
    }


class FinalizeExperimentRecordsTests(unittest.TestCase):
    def test_emits_one_setup_and_one_consolidated_report(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            paths = []
            for seed in (42, 43):
                path = root / f"seed-{seed}.json"
                path.write_text(json.dumps(sample_record(seed)), encoding="utf-8")
                paths.append(path)
            comparison = {"condition_groups": [{"summary": {"independent_seed_count": 2}}],
                          "measurement_definition_missing_records": []}
            with patch.object(finalizer, "compare", return_value=comparison):
                setup, report = finalizer.finalize(paths, [42, 43])
            self.assertEqual(setup["seeds"], [42, 43])
            self.assertEqual(report["status"], "validated")
            self.assertEqual(len(report["analysis"]["per_seed_results"]), 2)

    def test_rejects_missing_seed_before_comparison(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "seed-42.json"
            path.write_text(json.dumps(sample_record(42)), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "seed coverage mismatch"):
                finalizer.finalize([path], [42, 43])

    def test_rejects_partial_seed_record(self):
        with tempfile.TemporaryDirectory() as directory:
            record = sample_record(42)
            record["record_status"] = "partial"
            path = Path(directory) / "seed-42.json"
            path.write_text(json.dumps(record), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "not complete"):
                finalizer.finalize([path], [42])


if __name__ == "__main__":
    unittest.main()
