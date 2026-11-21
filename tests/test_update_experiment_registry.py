import tempfile
import unittest
from pathlib import Path

from tools.update_experiment_registry import update_registry


class UpdateExperimentRegistryTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.registry = self.root / "registry.md"
        self.registry.write_text(
            "# Registry\n\n"
            "| ID | Experiment | Conditions | Status | Primary record |\n"
            "|---|---|---|---|---|\n"
            "| EXP-2026-099 | Test | Baseline | Planned | |\n",
            encoding="utf-8",
        )
        self.results = self.root / "results"
        self.results.mkdir()
        (self.results / "setup.json").write_text("{}", encoding="utf-8")
        (self.results / "consolidated-report.json").write_text("{}", encoding="utf-8")

    def tearDown(self):
        self.temporary.cleanup()

    def update(self, condition="baseline"):
        update_registry(
            self.registry,
            "EXP-2026-099",
            condition,
            "results/setup.json",
            "results/consolidated-report.json",
        )

    def test_updates_registered_row_with_two_result_links(self):
        self.update()
        row = next(line for line in self.registry.read_text(encoding="utf-8").splitlines()
                   if line.startswith("| EXP-2026-099 |"))
        self.assertIn("Results recorded: baseline", row)
        self.assertIn("[baseline setup](results/setup.json)", row)
        self.assertIn("[baseline report](results/consolidated-report.json)", row)

    def test_is_idempotent_and_preserves_other_condition_links(self):
        self.update("baseline")
        self.update("baseline")
        self.update("followup")
        row = next(line for line in self.registry.read_text(encoding="utf-8").splitlines()
                   if line.startswith("| EXP-2026-099 |"))
        self.assertEqual(row.count("[baseline setup]"), 1)
        self.assertIn("[followup setup]", row)
        self.assertIn("[followup report]", row)

    def test_rejects_missing_or_duplicate_registry_rows(self):
        self.registry.write_text("| ID | Experiment | Conditions | Status | Primary record |\n", encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "found 0"):
            self.update()
        self.registry.write_text(
            "| EXP-2026-099 | A | A | Planned | |\n"
            "| EXP-2026-099 | B | B | Planned | |\n",
            encoding="utf-8",
        )
        with self.assertRaisesRegex(ValueError, "found 2"):
            self.update()

    def test_rejects_missing_published_files(self):
        (self.results / "consolidated-report.json").unlink()
        with self.assertRaisesRegex(ValueError, "both published result files"):
            self.update()


if __name__ == "__main__":
    unittest.main()
