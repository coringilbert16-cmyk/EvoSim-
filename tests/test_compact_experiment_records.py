from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from tools.compact_experiment_record import build_record
from tools.compare_compact_experiment_records import compare


class CompactExperimentRecordTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def write(self, name: str, value: dict) -> Path:
        path = self.root / name
        path.write_text(json.dumps(value), encoding="utf-8")
        return path

    def seed_inputs(self, seed: int, config_hash: str = "a" * 64):
        chemistry = {
            "input": {
                "seed": seed, "requested_duration_seconds": 60,
                "planned_ticks": None, "diagnostic_interval": 100,
                "initial_physical_material_count": 1280,
                "initial_independent_resource_count": 1200,
                "initial_molecule_count": 80, "initial_constituent_count": 1360,
            },
            "sampling": {"summary_present": True},
            "resource_coverage": {
                "required_resources": ["Carbon", "Hydrogen"],
                "initial_inventory": {"Carbon": 600, "Hydrogen": 600},
            },
            "reactions": {
                "event_counts": {"BREAK": 2},
                "outcome_counts": {"BREAK": {"committed": 1, "rejected": 1}},
                "diagnostic_totals": {"BREAK": {"ready": 3, "committed": 1, "rejected": 1}},
                "elemental_bond_pairs": {
                    "formed_total": 1, "broken_total": 1,
                    "committed_events_with_graph_deltas": 1,
                    "committed_events_missing_graph_deltas": 0,
                    "inventory_reconciliation": {"matches": True},
                    "formed": {"C-H": 1}, "broken": {"C-H": 1},
                },
            },
            "molecular_history": {
                "validation_errors": [], "molecule_creation_count": 2,
                "molecule_destruction_count": 1, "active_at_end_count": 1,
                "lineage_edges": [{"parent_id": 1, "child_id": 2}],
                "lineage_reaction_counts": {"COMBINE": 1},
                "mean_observed_lifetime_ticks": None,
            },
            "energy": {
                "first_balance_residual": 0.0, "last_balance_residual": None,
                "maximum_absolute_snapshot_residual": 0.0,
            },
        }
        organization = {
            "scope": {"lifecycle_validation_errors": []},
            "organization": {
                "unique_exact_structural_families": 2, "recurrent_families": 1,
                "families_active_at_end": 1,
                "molecules_with_multi_resource_or_bonded_structure": 1,
                "families": [{"large": "array that must not be copied"}],
            },
            "material_distribution": {
                "reaction_created_compound_constituent_amount_by_resource": {"Carbon": 1}
            },
            "reaction_network": {"family_to_family_transitions": [{"edge": 1}]},
        }
        manifest = {
            "schema_version": 1,
            "experiment": {"experiment_id": "EXP-2026-999", "research_plan": "OBS-001",
                           "condition_id": "baseline"},
            "provenance": {"commit_sha": "deadbeef"},
            "run": {"status": "completed", "actual_elapsed_wall_seconds": 60.1,
                    "start_tick": 0, "end_tick": 600},
            "conditions": {"configuration_sha256": config_hash,
                           "configuration": {"active_elements": ["Carbon", "Hydrogen"]}},
            "artifacts": [{"role": "raw_observation", "sha256": "b" * 64, "size_bytes": 1234}],
        }
        return (
            self.write(f"chemistry-{seed}.json", chemistry),
            self.write(f"organization-{seed}.json", organization),
            self.write(f"manifest-{seed}.json", manifest),
        )

    def test_builder_emits_compact_record_and_preserves_null(self) -> None:
        chemistry, organization, manifest = self.seed_inputs(42)
        record = build_record(42, chemistry, organization, manifest)
        self.assertEqual(record["schema"], "evosim.permanent_experiment_record")
        self.assertEqual(record["reactions"]["BREAK"]["diagnostics"]["committed"], 1)
        self.assertIsNone(record["molecular_development"]["mean_observed_lifetime_ticks"])
        self.assertEqual(record["molecular_development"]["lineage_edge_count"], 1)
        self.assertNotIn("families", record["molecular_development"])
        self.assertNotIn("lineage_edges", record["molecular_development"])
        self.assertEqual(record["record_status"], "complete")

    def test_comparison_uses_compact_records_and_separates_incompatible_conditions(self) -> None:
        records = []
        for seed, digest in ((42, "a" * 64), (43, "a" * 64), (44, "c" * 64)):
            chemistry, organization, manifest = self.seed_inputs(seed, digest)
            record = build_record(seed, chemistry, organization, manifest)
            path = self.root / f"record-{seed}.json"
            path.write_text(json.dumps(record), encoding="utf-8")
            records.append(path)
        result = compare(records)
        self.assertFalse(result["compatible"])
        self.assertEqual(len(result["compatibility_groups"]), 2)
        self.assertFalse(result["quality"]["raw_diagnostics_required"])
        self.assertFalse(result["quality"]["pooled_rates_emitted"])

    def test_comparison_rejects_duplicate_seeds(self) -> None:
        chemistry, organization, manifest = self.seed_inputs(42)
        record = build_record(42, chemistry, organization, manifest)
        path = self.root / "record.json"
        path.write_text(json.dumps(record), encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "duplicate"):
            compare([path, path])


if __name__ == "__main__":
    unittest.main()
