from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).resolve().parents[1] / "tools" / "chemistry_starting_mixture_batch_analyzer.py"
SPEC = importlib.util.spec_from_file_location("chemistry_starting_mixture_batch_analyzer", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class StartingMixtureBatchAnalyzerTests(unittest.TestCase):
    def write_reports(self, root: Path) -> None:
        for mixture in MODULE.MIXTURES:
            for seed in MODULE.SEEDS:
                stem = f"seed-{seed}-{mixture}"
                chemistry = {
                    "input": {
                        "seed": seed,
                        "dispersion": "randomized",
                        "starting_mixture": mixture,
                        "requested_duration_seconds": 300,
                        "initial_physical_material_count": 1280,
                        "initial_independent_resource_count": 1200,
                        "initial_molecule_count": 80,
                        "initial_material_inventory": {
                            "Carbon": 200.0, "Hydrogen": 250.0, "Nitrogen": 150.0,
                            "Oxygen": 150.0, "Phosphorus": 150.0, "Sulfur": 150.0,
                            "Water": 150.0,
                        },
                    },
                    "resource_coverage": {"all_seven_present_at_start": True},
                    "sampling": {"summary_present": True},
                    "molecular_history": {"validation_errors": []},
                    "reactions": {
                        "diagnostic_totals": {
                            name: {"ready": 10, "scheduled": 4, "committed": 3, "rejected": 1}
                            for name in ("BREAK", "COMBINE", "DISPLACEMENT")
                        }
                    },
                }
                organization = {
                    "scope": {
                        "lifecycle_validation_errors": [],
                        "molecule_creation_records": 12,
                    },
                    "organization": {
                        "molecules_with_multi_resource_or_bonded_structure": 5,
                        "unique_exact_structural_families": 3,
                        "recurrent_families": 2,
                        "families_active_at_end": 1,
                    },
                    "persistence": {"mean_observed_lifetime_ticks": 120.0},
                }
                (root / f"{stem}-chemistry.json").write_text(json.dumps(chemistry), encoding="utf-8")
                (root / f"{stem}-organization.json").write_text(json.dumps(organization), encoding="utf-8")

    def test_aggregates_fifteen_runs_and_checks_inventory_matching(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.write_reports(root)
            report = MODULE.build_report(root)
            self.assertEqual(report["experiment"]["total_runs"], 15)
            self.assertTrue(report["experiment"]["initial_inventory"]["total_resource_inventory_matched_per_seed"])
            for mixture in MODULE.MIXTURES:
                row = report["conditions"][mixture]
                self.assertEqual(row["run_count"], 5)
                self.assertEqual(row["simulated_seconds"], 1500)
                self.assertEqual(row["reaction_totals"]["BREAK"]["committed"], 15)
                self.assertEqual(row["organization_mean_per_seed"]["recurrent_product_families"], 2.0)

    def test_rejects_inventory_mismatch_between_mixtures(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.write_reports(root)
            path = root / "seed-42-high-bonded-chemistry.json"
            report = json.loads(path.read_text(encoding="utf-8"))
            report["input"]["initial_material_inventory"]["Sulfur"] += 1.0
            path.write_text(json.dumps(report), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "total resource inventory differs"):
                MODULE.build_report(root)


if __name__ == "__main__":
    unittest.main()
