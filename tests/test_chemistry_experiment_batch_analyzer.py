import json
import tempfile
import unittest
from pathlib import Path

from tools.chemistry_experiment_batch_analyzer import ACTIVE_ELEMENTS, build_report


def write_run(root: Path, seed: int, water_excluded: bool = True) -> None:
    chemistry = {
        "input": {
            "seed": seed,
            "requested_duration_seconds": 300,
            "planned_ticks": 3000,
            "initial_physical_material_count": 1280,
            "initial_independent_resource_count": 1200,
            "initial_molecule_count": 80,
        },
        "resource_coverage": {
            "excluded_resources": ["Water"] if water_excluded else [],
            "required_resources": list(ACTIVE_ELEMENTS) if water_excluded else [*ACTIVE_ELEMENTS, "Water"],
            "all_required_present_at_start": True,
            "snapshots_missing_resources": [],
        },
        "sampling": {"summary_present": True},
        "molecular_history": {
            "validation_errors": [],
            "molecule_creation_count": 2,
            "molecule_destruction_count": 1,
            "active_at_end_count": 1,
            "mean_observed_lifetime_ticks": 12.0,
            "lineage_edges": [
                {"parent_id": 1, "child_id": 2, "reaction": "COMBINE"}
            ],
            "molecules": [
                {"molecule_id": 1, "reaction": "BREAK"},
                {"molecule_id": 2, "reaction": "COMBINE"},
            ],
        },
        "reactions": {
            "diagnostic_totals": {
                name: {"ready": 2, "scheduled": 1, "committed": 1, "rejected": 0}
                for name in ("BREAK", "COMBINE", "DISPLACEMENT")
            },
            "event_counts": {"COMBINE": 1},
            "outcome_counts": {"COMBINE:committed": 1},
            "failure_reasons": {},
        },
        "energy": {
            "last_balance_residual": 0.0,
            "maximum_absolute_snapshot_residual": 0.000001,
        },
    }
    organization = {
        "scope": {"lifecycle_validation_errors": []},
        "organization": {
            "unique_exact_structural_families": 1,
            "recurrent_families": 0,
            "families_active_at_end": 1,
            "molecules_with_multi_resource_or_bonded_structure": 1,
            "families": [{
                "signature": "C-H",
                "composition": {"Carbon": 1, "Hydrogen": 1},
                "reaction_created_occurrences": 1,
                "reaction_created_reaction_events": 1,
                "reaction_created_active_at_end": 1,
            }],
        },
        "reaction_network": {
            "family_to_family_transitions": [],
        },
        "material_distribution": {
            "reaction_created_compound_constituent_amount_by_resource": {
                "Carbon": 1, "Hydrogen": 1
            },
        },
    }
    (root / f"seed-{seed}-chemistry.json").write_text(json.dumps(chemistry), encoding="utf-8")
    (root / f"seed-{seed}-organization.json").write_text(json.dumps(organization), encoding="utf-8")


class ChemistryExperimentBatchAnalyzerTests(unittest.TestCase):
    def test_aggregates_enabled_only_seeds_and_tracks_reuse(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_run(root, 42)
            write_run(root, 43)
            report = build_report(root, [42, 43], 300)
        self.assertEqual(report["experiment"]["total_simulated_seconds"], 600)
        self.assertEqual(report["reaction_totals"]["COMBINE"]["committed"], 2)
        self.assertEqual(report["molecular_development"]["reaction_created_products_reused_as_parents"], 2)
        self.assertEqual(report["organization"]["cross_seed_recurrent_product_families"][0]["seed_count"], 2)
        self.assertTrue(report["validation"]["all_runs_pass_inventory_and_lifecycle_checks"])

    def test_rejects_water_in_the_active_chemistry_inventory(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_run(root, 42, water_excluded=False)
            with self.assertRaisesRegex(ValueError, "exclude Water"):
                build_report(root, [42], 300)


if __name__ == "__main__":
    unittest.main()
