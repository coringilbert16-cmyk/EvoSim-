import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from tools import unified_chemistry_analysis as unified


class UnifiedChemistryAnalysisTests(unittest.TestCase):
    def test_legacy_input_is_checksum_tracked_without_invented_manifest(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            raw = root / "seed-42.jsonl"
            raw.write_bytes(b'{"record":"fixture"}\n')
            expected_sha = unified.sha256_file(raw)
            run = unified.validate_run(root, 42)
            self.assertEqual(run["manifest_status"], "legacy_manifest_missing")
            self.assertEqual(run["raw_sha256"], expected_sha)
            self.assertIsNone(run["manifest_path"])

    def test_cache_requires_matching_input_analyzer_and_output_hash(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            output = root / "stage.json"
            metadata = root / "stage.json.meta"
            output.write_text('{"ok":true}', encoding="utf-8")
            metadata.write_text(json.dumps({
                "input_sha256": "input-hash",
                "analyzer_sha256": "script-hash",
                "output_sha256": unified.sha256_file(output),
            }), encoding="utf-8")
            self.assertTrue(unified.stage_cache_valid(metadata, output, "input-hash", "script-hash"))
            self.assertFalse(unified.stage_cache_valid(metadata, output, "changed-input", "script-hash"))
            output.write_text('{"changed":true}', encoding="utf-8")
            self.assertFalse(unified.stage_cache_valid(metadata, output, "input-hash", "script-hash"))

    def test_pipeline_writes_index_report_and_defers_heavy_contact_stage(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "input"
            output = root / "analysis"
            source.mkdir()
            raw = source / "seed-42.jsonl"
            raw.write_text('{"fixture":true}\n', encoding="utf-8")

            def fake_seed_stage(stage_id, script, raw_path, seed, outdir, input_sha):
                outdir.mkdir(parents=True, exist_ok=True)
                result = outdir / f"seed-{seed}-{stage_id}.json"
                result.write_text('{"seed":42}', encoding="utf-8")
                (outdir / f"seed-{seed}-{stage_id}.md").write_text("fixture", encoding="utf-8")
                return {"stage": stage_id, "status": "completed", "outputs": [str(result)]}

            def fake_group_stage(stage_id, script_name, workspace, seeds, duration, outdir, output_files, extra=None):
                outdir.mkdir(parents=True, exist_ok=True)
                if stage_id == "experiment_batch":
                    output_files[0].write_text(json.dumps({"reaction_totals": {
                        "COMBINE": {"ready": 1, "scheduled": 1, "committed": 1, "rejected": 0}
                    }}), encoding="utf-8")
                    output_files[1].write_text("batch", encoding="utf-8")
                else:
                    (outdir / "summary.md").write_text("stage", encoding="utf-8")
                return {"stage": stage_id, "status": "completed", "outputs": []}

            with patch.object(unified, "run_cached_stage", side_effect=fake_seed_stage), \
                 patch.object(unified, "run_group_stage", side_effect=fake_group_stage):
                index = unified.analyze(source, [42], 60, output)

            self.assertEqual(index["runs"][0]["manifest_status"], "legacy_manifest_missing")
            self.assertIn("contact_opportunities", index["quality"]["deferred_stages"])
            self.assertIn("reaction_network", index["quality"]["deferred_stages"])
            self.assertTrue((output / "unified-analysis-index.json").is_file())
            self.assertTrue((output / "unified-chemistry-report.md").is_file())
            self.assertTrue((output / "normalized-input/seed-42.jsonl").is_symlink())
            self.assertTrue(raw.is_file())

    def test_failed_stage_does_not_leave_stale_json_as_valid_result(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            out = root / "per-seed"
            out.mkdir()
            output = out / "seed-42-chemistry.json"
            output.write_text('{"stale":true}', encoding="utf-8")
            raw = root / "seed-42.jsonl"
            raw.write_text("{}\n", encoding="utf-8")
            failed = subprocess.CompletedProcess([], 1, "", "analyzer failed")
            with patch.object(unified.subprocess, "run", return_value=failed):
                row = unified.run_cached_stage("chemistry", unified.ROOT / "tools/chemistry_analyzer.py",
                                              raw, 42, out, unified.sha256_file(raw))
            self.assertEqual(row["status"], "failed")
            self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
