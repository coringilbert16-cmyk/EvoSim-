import json
import tempfile
import unittest
from pathlib import Path

from tools.compare_historical_specialist_outputs import compare_directories


class CompareHistoricalSpecialistOutputsTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.original = self.root / "original"
        self.recomputed = self.root / "recomputed"
        self.original.mkdir()
        self.recomputed.mkdir()
        for analyzer in ("chemistry", "organization"):
            payload = {"schema_version": 2, "metrics": {"committed": 12, "families": ["H-O", "C-O"]}}
            for directory in (self.original, self.recomputed):
                (directory / f"seed-42-{analyzer}.json").write_text(
                    json.dumps(payload), encoding="utf-8"
                )

    def tearDown(self):
        self.temporary.cleanup()

    def test_identical_specialist_outputs_match_exactly(self):
        result = compare_directories(self.original, self.recomputed, [42])
        self.assertEqual(result["report_count"], 2)
        self.assertEqual(result["exact_match_count"], 2)
        self.assertEqual(result["reports_with_differences"], 0)

    def test_reports_structured_differences_without_hiding_paths(self):
        path = self.recomputed / "seed-42-chemistry.json"
        payload = json.loads(path.read_text(encoding="utf-8"))
        payload["metrics"]["committed"] = 13
        path.write_text(json.dumps(payload), encoding="utf-8")
        result = compare_directories(self.original, self.recomputed, [42])
        chemistry = next(row for row in result["per_report"] if row["analyzer"] == "chemistry")
        self.assertFalse(chemistry["exact_match"])
        self.assertEqual(chemistry["difference_count"], 1)
        self.assertEqual(chemistry["difference_examples"][0]["path"], "metrics.committed")

    def test_missing_original_or_recomputed_report_fails(self):
        (self.original / "seed-42-organization.json").unlink()
        with self.assertRaisesRegex(ValueError, "missing retained organization"):
            compare_directories(self.original, self.recomputed, [42])


if __name__ == "__main__":
    unittest.main()
