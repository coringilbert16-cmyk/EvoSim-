from __future__ import annotations

import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).resolve().parents[1] / "tools" / "chemistry_dispersion_batch_analyzer.py"
SPEC = importlib.util.spec_from_file_location("chemistry_dispersion_batch_analyzer", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class DispersionBatchAnalyzerTests(unittest.TestCase):
    def write_reports(self, root: Path) -> None:
        for dispersion in MODULE.DISPERSIONS:
            for seed in MODULE.SEEDS:
                stem = f"seed-{seed}-{dispersion}"
                chemistry = {
                    "input": {
                        "seed": seed,
                        "dispersion": dispersion,
                        "requested_duration_seconds": 300,
                        "initial_physical_material_count": 1280,
                        "initial_independent_resource_count": 1200,
                        "initial_molecule_count": 80,
                    },
                    "resource_coverage": {"all_seven_present_at_start": True},
                    "sampling": {"summary_present": True},
                    "molecular_history": {"validation_errors": []},
                    "reactions": {
                        "diagnostic_totals": {
                            name: {"ready": 10, "scheduled": 4, "committed": 3, "rejected": 1}
                            for name in MODULE.REACTIONS
                        },
                        "opportunity_funnels": {
                            name: {"candidate_contacts": 7, "activation_updates": 5}
                            for name in MODULE.REACTIONS
                        },
                    },
                    "energy": {"maximum_absolute_snapshot_residual": 1e-8},
                }
                organization = {
                    "scope": {
                        "lifecycle_validation_errors": [],
                        "molecule_creation_records": 12,
                        "molecule_destruction_records": 4,
                        "active_molecules_at_end": 8,
                    },
                    "organization": {
                        "molecules_with_multi_resource_or_bonded_structure": 5,
                        "unique_exact_structural_families": 3,
                        "recurrent_families": 2,
                        "families_active_at_end": 1,
                    },
                    "persistence": {"mean_observed_lifetime_ticks": 120.0},
                }
                (root / f"{stem}-chemistry.json").write_text(
                    json.dumps(chemistry), encoding="utf-8"
                )
                (root / f"{stem}-organization.json").write_text(
                    json.dumps(organization), encoding="utf-8"
                )

    def test_aggregates_exactly_fifteen_matched_runs(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.write_reports(root)
            report = MODULE.build_report(root)
            self.assertEqual(report["experiment"]["total_runs"], 15)
            for dispersion in MODULE.DISPERSIONS:
                condition = report["conditions"][dispersion]
                self.assertEqual(condition["run_count"], 5)
                self.assertEqual(condition["simulated_seconds"], 1500)
                self.assertEqual(condition["reaction_totals"]["BREAK"]["committed"], 15)
                self.assertEqual(
                    condition["organization_mean_per_seed"]["compound_molecule_identities"],
                    5.0,
                )

    def test_rejects_mismatched_dispersion_metadata(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.write_reports(root)
            path = root / "seed-42-uniform-chemistry.json"
            report = json.loads(path.read_text(encoding="utf-8"))
            report["input"]["dispersion"] = "clustered"
            path.write_text(json.dumps(report), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "dispersion expected"):
                MODULE.build_report(root)


if __name__ == "__main__":
    unittest.main()
