import unittest
from pathlib import Path


class OBS001HistoricalPilotWorkflowTests(unittest.TestCase):
    def test_historical_pilot_is_analysis_only_and_uses_retained_sources(self):
        root = Path(__file__).resolve().parents[1]
        workflow = (root / ".github/workflows/obs-001-historical-pilot.yml").read_text(
            encoding="utf-8"
        )
        self.assertIn("workflow_dispatch:", workflow)
        self.assertIn("actions/download-artifact@v4", workflow)
        self.assertIn("44582565701", workflow)
        self.assertIn("44776501166", workflow)
        self.assertIn("8e7cc7d624e3131b2d99889fef02d9d5c3095318", workflow)
        self.assertIn("191069deb65a9f40638e65fe93134eeadde2219c", workflow)
        self.assertNotIn("cargo run", workflow)
        self.assertNotIn("--environment-chemistry-experiment", workflow)
        self.assertIn("condition_id", workflow)
        self.assertIn("legacy_manifest_missing", workflow)
        self.assertIn("retention-days: 1", workflow)

    def test_uploaded_artifact_contains_only_compact_reports_not_raw_inputs(self):
        root = Path(__file__).resolve().parents[1]
        workflow = (root / ".github/workflows/obs-001-historical-pilot.yml").read_text(
            encoding="utf-8"
        )
        upload_section = workflow.split("      - name: Upload compact historical pilot report only", 1)[1]
        upload_section = upload_section.split("      - name: Remove downloaded raw", 1)[0]
        self.assertIn("historical-pilot-summary.md", upload_section)
        self.assertIn("unified-analysis-index.json", upload_section)
        self.assertNotIn("observations/", upload_section)
        self.assertNotIn("historical-artifacts/", upload_section)
        self.assertNotIn("original-specialist/", upload_section)


if __name__ == "__main__":
    unittest.main()
