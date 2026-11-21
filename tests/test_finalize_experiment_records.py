import hashlib
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
                       "organization_analyzer_sha256": "org", "enabled_diagnostic_tiers": [],
                       "raw_diagnostics_sha256": hashlib.sha256(f"seed-{seed}".encode()).hexdigest()},
        "conditions": {"configuration": {
                           "duration": 60,
                           "deep_diagnostics_enabled": True,
                           "profile_movement_enabled": True,
                           "profile_tick_enabled": False,
                           "biology_enabled": False,
                           "initial_material_inventory": {"Carbon": 1360.0},
                           "initial_physical_material_count": 1280,
                           "initial_independent_resource_count": 1200,
                           "initial_molecule_count": 80,
                           "initial_constituent_count": 1360,
                           "pile_sizes": [1280],
                           "hotspot": {"center_x": 125.0, "center_y": 125.0},
                           "world_geometry": {"width": 250.0, "height": 250.0,
                                              "width_cells": 10, "height_cells": 10,
                                              "cell_size": 25.0},
                           "flow_field": {"direction_degrees": 0.0, "speed": 1.0,
                                          "strength": 1.0, "vortex_strength": 1.0},
                       },
                       "requested_duration_seconds": 60,
                       "actual_elapsed_wall_seconds": 61.0, "start_tick": 0, "end_tick": 600,
                       "tick_count": 600, "diagnostic_interval": 100, "active_resources": ["C", "H"]},
    }


def sample_index_run(seed):
    return {
        "seed": seed,
        "raw_sha256": sample_record(seed)["provenance"]["raw_diagnostics_sha256"],
        "manifest_status": "manifest_validated",
    }


def sample_stage(root, stage_id, status="completed"):
    output = root / f"{stage_id}.json"
    output.write_text(f"{stage_id} output\\n", encoding="utf-8")
    digest = hashlib.sha256(output.read_bytes()).hexdigest()
    return {"stage": stage_id, "status": status, "outputs": [output.name],
            "output_sha256": {output.name: digest}}


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
            index_path = root / "index.json"
            index_path.write_text(json.dumps({
                "schema": "evosim.unified_chemistry_analysis",
                "experiment_id": "EXP-2026-099",
                "research_plan": "OBS-001",
                "condition_id": "baseline",
                "runs": [sample_index_run(42), sample_index_run(43)],
                "quality": {"all_runs_have_validated_manifests": True},
                "stages": [
                    sample_stage(root, "chemistry"),
                    sample_stage(root, "organization"),
                    sample_stage(root, "experiment_batch")]}), encoding="utf-8")
            with patch.object(finalizer, "compare", return_value=comparison):
                setup, report = finalizer.finalize(paths, [42, 43], index_path)
            self.assertEqual(setup["seeds"], [42, 43])
            self.assertEqual(
                setup["simulator_invocation"]["arguments_by_seed"][0]["command"],
                ["cargo", "run", "--locked", "--", "--headless",
                 "--environment-chemistry-experiment", "--seconds", "60", "--seed", "42",
                 "--diagnostics", "observations/seed-42.jsonl", "--diagnostic-interval", "100",
                 "--deep-diagnostics"],
            )
            self.assertTrue(setup["simulator_invocation"]["environment_by_seed"][0]["EVOSIM_PROFILE_MOVEMENT"])
            self.assertFalse(setup["simulator_invocation"]["environment_by_seed"][0]["EVOSIM_PROFILE_TICK"])
            self.assertEqual(report["status"], "validated")
            self.assertEqual(len(report["analysis"]["per_seed_results"]), 2)


    def test_rejects_incomplete_required_pipeline_stage(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            records = []
            for seed in (42, 43):
                path = root / f"seed-{seed}.json"
                path.write_text(json.dumps(sample_record(seed)), encoding="utf-8")
                records.append(path)
            index_path = root / "index.json"
            index_path.write_text(json.dumps({
                "schema": "evosim.unified_chemistry_analysis",
                "experiment_id": "EXP-2026-099",
                "research_plan": "OBS-001",
                "condition_id": "baseline",
                "runs": [{"seed": 42}, {"seed": 43}],
                "quality": {"all_runs_have_validated_manifests": True},
                "stages": [
                    {"stage": "chemistry", "status": "completed"},
                    {"stage": "organization", "status": "failed"},
                    {"stage": "experiment_batch", "status": "completed"}]}), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "required unified analysis stages"):
                finalizer.finalize(records, [42, 43], index_path)

    def test_rejects_analysis_output_checksum_mismatch(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            records = []
            for seed in (42, 43):
                path = root / f"seed-{seed}.json"
                path.write_text(json.dumps(sample_record(seed)), encoding="utf-8")
                records.append(path)
            stages = [sample_stage(root, name) for name in ("chemistry", "organization", "experiment_batch")]
            (root / "organization.json").write_text("tampered\\n", encoding="utf-8")
            index_path = root / "index.json"
            index_path.write_text(json.dumps({
                "schema": "evosim.unified_chemistry_analysis",
                "experiment_id": "EXP-2026-099",
                "research_plan": "OBS-001",
                "condition_id": "baseline",
                "runs": [{"seed": 42}, {"seed": 43}],
                "quality": {"all_runs_have_validated_manifests": True},
                "stages": stages}), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "output checksum mismatch"):
                finalizer.finalize(records, [42, 43], index_path)

    def test_rejects_records_from_different_workflow_runs(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            records = []
            for seed in (42, 43):
                record = sample_record(seed)
                if seed == 43:
                    record["provenance"]["workflow_run_id"] = "different-run"
                    record["provenance"]["workflow_run_url"] = "https://example.test/run/different"
                path = root / f"seed-{seed}.json"
                path.write_text(json.dumps(record), encoding="utf-8")
                records.append(path)
            index_path = root / "index.json"
            index_path.write_text("{}", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "different workflow runs"):
                finalizer.finalize(records, [42, 43], index_path)

    def test_rejects_missing_seed_before_comparison(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "seed-42.json"
            path.write_text(json.dumps(sample_record(42)), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "seed coverage mismatch"):
                finalizer.finalize([path], [42, 43], Path(directory) / "unused-index.json")

    def test_rejects_partial_seed_record(self):
        with tempfile.TemporaryDirectory() as directory:
            record = sample_record(42)
            record["record_status"] = "partial"
            path = Path(directory) / "seed-42.json"
            path.write_text(json.dumps(record), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "not complete"):
                finalizer.finalize([path], [42], Path(directory) / "unused-index.json")


if __name__ == "__main__":
    unittest.main()
