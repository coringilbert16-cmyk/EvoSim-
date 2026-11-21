from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
from unittest.mock import patch
from pathlib import Path

from tools.compact_experiment_record import build_record
from tools.compare_compact_experiment_records import compare
from tools.chemistry_analyzer import summarize_settlements
from tools.contact_opportunity_analyzer import analyze as analyze_opportunities
from tools.unified_chemistry_analysis import main as unified_analysis_main


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
                "settlement": {"BREAK": {"settled_event_count": 1, "not_settled_event_count": 1,
                                        "usable_energy_delta": {"count": 1, "sum": -2.0}}},
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
                "top_families": [{"signature": "sig-a", "occurrences": 3,
                                  "reaction_created_occurrences": 2,
                                  "reaction_created_reaction_events": 2,
                                  "reaction_created_active_at_end": 1,
                                  "active_at_end": 1, "composition": {"Carbon": 2, "Hydrogen": 1},
                                  "mean_observed_lifetime_ticks": 5.0,
                                  "median_observed_lifetime_ticks": 5.0,
                                  "completed_lifetimes": 2, "right_censored_lifetimes": 1,
                                  "recurrent_from_reactions": True,
                                  "molecule_ids": [1, 2, 3]}],
                "families_active_at_end": 1,
                "molecules_with_multi_resource_or_bonded_structure": 1,
                "families": [{"large": "array that must not be copied"}],
            },
            "material_distribution": {
                "reaction_created_compound_constituent_amount_by_resource": {"Carbon": 1}
            },
            "reaction_network": {"family_to_family_transitions": [{"reaction": "COMBINE", "parent_family": "family-a", "child_family": "family-b", "count": 2}]},
        }
        raw_path = self.root / f"seed-{seed}.jsonl"
        raw_bytes = b'{"event":"environmental_chemistry_header"}\\n'
        raw_path.write_bytes(raw_bytes)
        configuration = {
            "seed": seed,
            "active_elements": ["Carbon", "Hydrogen"],
            "flow_speed": 2.0 if config_hash.startswith("c") else 1.0,
        }
        canonical_configuration = json.dumps(configuration, separators=(",", ":"))
        manifest = {
            "schema": "evosim.experiment_manifest",
            "schema_version": 1,
            "experiment": {"experiment_id": "EXP-2026-999", "research_plan": "OBS-001",
                           "condition_id": "baseline", "seed": seed},
            "build": {"commit_sha": "deadbeef"},
            "run": {"status": "completed", "started_at_unix_ms": 1000,
                    "completed_at_unix_ms": 61000, "actual_elapsed_wall_seconds": 60.1,
                    "start_tick": 0, "end_tick": 600},
            "conditions": {
                "configuration_schema_version": 1,
                "configuration_sha256": hashlib.sha256(canonical_configuration.encode()).hexdigest(),
                "configuration_canonical_json": canonical_configuration,
                "configuration": configuration,
            },
            "artifacts": [{
                "role": "raw_observation", "path": raw_path.name,
                "sha256": hashlib.sha256(raw_bytes).hexdigest(), "size_bytes": len(raw_bytes),
            }],
            "diagnostics": {"enabled_tiers": ["core", "standard"]},
            "quality": {"warnings": []},
        }
        return (
            self.write(f"chemistry-{seed}.json", chemistry),
            self.write(f"organization-{seed}.json", organization),
            self.write(f"manifest-{seed}.json", manifest),
        )

    def test_builder_emits_compact_record_and_preserves_null(self) -> None:
        chemistry, organization, manifest = self.seed_inputs(42)
        coverage = self.write("coverage.json", {
            "schema_version": 2,
            "per_seed": [{
                "seed": 42,
                "diagnostic_ticks": 10,
                "totals": {"break_scan_entries_inspected": 4},
                "coverage": {
                    "break": {"state": "complete", "search_domain": "bond_index",
                              "ticks": 10, "complete_ticks": 10, "partial_ticks": 0,
                              "unknown_ticks": 0, "not_applicable_ticks": 0},
                    "combine": {"state": "partial", "search_domain": "broad_phase_pairs",
                                "ticks": 10, "complete_ticks": 0, "partial_ticks": 10,
                                "unknown_ticks": 0, "not_applicable_ticks": 0},
                    "displacement": {"state": "partial", "search_domain": "bounded_search",
                                     "ticks": 10, "complete_ticks": 0, "partial_ticks": 10,
                                     "unknown_ticks": 0, "not_applicable_ticks": 0},
                },
            }],
        })
        record = build_record(42, chemistry, organization, manifest, coverage)
        self.assertEqual(record["schema"], "evosim.permanent_experiment_record")
        self.assertEqual(record["opportunity_coverage"]["by_reaction"]["break"]["state"], "complete")
        self.assertEqual(record["conditions"]["configuration"]["active_elements"], ["Carbon", "Hydrogen"])
        self.assertEqual(record["reactions"]["BREAK"]["diagnostics"]["committed"], 1)
        self.assertIsNone(record["molecular_development"]["mean_observed_lifetime_ticks"])
        self.assertEqual(record["molecular_development"]["lineage_edge_count"], 1)
        self.assertEqual(record["molecular_development"]["lineage_validation_status"], "valid")
        self.assertEqual(record["organization"]["family_transition_summary"]["top_family_pairs"][0]["count"], 2)
        self.assertEqual(record["organization"]["top_structural_families"][0]["signature"], "sig-a")
        self.assertNotIn("molecule_ids", record["organization"]["top_structural_families"][0])
        self.assertNotIn("families", record["molecular_development"])
        self.assertNotIn("lineage_edges", record["molecular_development"])
        self.assertEqual(record["record_status"], "complete")

    def test_unified_comparison_uses_compact_records_across_conditions(self) -> None:
        records = []
        for seed, digest in ((42, "a" * 64), (43, "a" * 64), (42, "c" * 64), (43, "c" * 64)):
            chemistry, organization, manifest = self.seed_inputs(seed, digest)
            record = build_record(seed, chemistry, organization, manifest)
            path = self.root / f"record-{len(records)}-{seed}.json"
            path.write_text(json.dumps(record), encoding="utf-8")
            # Comparison must remain possible after all source and intermediate files expire.
            for source in (chemistry, organization, manifest, self.root / f"seed-{seed}.jsonl"):
                source.unlink()
            records.append(path)
        output = self.root / "comparison.json"
        markdown = self.root / "comparison.md"
        with patch("sys.argv", [
            "unified_chemistry_analysis.py", "--compare-records",
            *(str(record) for record in records),
            "--comparison-json-out", str(output),
            "--comparison-markdown-out", str(markdown),
        ]):
            self.assertEqual(unified_analysis_main(), 0)
        result = json.loads(output.read_text(encoding="utf-8"))
        self.assertTrue(markdown.is_file())
        self.assertFalse(result["quality"]["raw_diagnostics_required"])
        self.assertTrue(result["measurement_compatible"], result["measurement_definition_missing_records"])
        self.assertEqual(len(result["condition_groups"]), 2)
        self.assertEqual(len(result["condition_comparisons"]), 1)
        self.assertEqual(result["condition_groups"][1]["summary"]["seeds"], [42, 43])
        paired_break = result["condition_comparisons"][0]["metrics"]["BREAK.committed"]["paired_seed_differences"]
        self.assertEqual([row["seed"] for row in paired_break], [42, 43])
        first_group = result["condition_groups"][0]["summary"]
        self.assertEqual(first_group["reaction_totals"]["COMBINE"]["committed_by_seed"], [None, None])
        self.assertFalse(result["quality"]["raw_diagnostics_required"])
        self.assertFalse(result["quality"]["pooled_rates_emitted"])

    def test_comparison_separates_measurement_incompatibilities(self) -> None:
        paths = []
        for seed in (42, 43):
            chemistry, organization, manifest = self.seed_inputs(seed)
            record = build_record(seed, chemistry, organization, manifest)
            if seed == 43:
                record["conditions"]["requested_duration_seconds"] = 30
            path = self.root / f"incompatible-{seed}.json"
            path.write_text(json.dumps(record), encoding="utf-8")
            paths.append(path)
        result = compare(paths)
        self.assertFalse(result["measurement_compatible"])
        self.assertEqual(result["measurement_compatibility_group_count"], 2)
        self.assertEqual(result["condition_comparisons"], [])

    def test_comparison_separates_different_analyzer_definitions(self) -> None:
        paths = []
        for seed in (42, 43):
            chemistry, organization, manifest = self.seed_inputs(seed)
            record = build_record(seed, chemistry, organization, manifest)
            if seed == 43:
                record["provenance"]["chemistry_analyzer_sha256"] = "f" * 64
            path = self.root / f"analyzer-definition-{seed}.json"
            path.write_text(json.dumps(record), encoding="utf-8")
            paths.append(path)
        result = compare(paths)
        self.assertFalse(result["measurement_compatible"])
        self.assertEqual(result["measurement_compatibility_group_count"], 2)
        self.assertEqual(result["condition_comparisons"], [])

    def test_comparison_fails_closed_when_measurement_definition_is_missing(self) -> None:
        chemistry, organization, manifest = self.seed_inputs(42)
        record = build_record(42, chemistry, organization, manifest)
        record["provenance"]["organization_analyzer_sha256"] = None
        path = self.root / "missing-measurement-definition.json"
        path.write_text(json.dumps(record), encoding="utf-8")
        result = compare([path])
        self.assertFalse(result["measurement_compatible"])
        missing = next(
            row for row in result["measurement_definition_missing_records"]
            if row["seed"] == 42
        )
        self.assertIn("organization_analyzer_sha256", missing["missing_fields"])

    def test_summary_only_opportunity_analysis_omits_tick_and_sample_arrays(self) -> None:
        raw = self.root / "seed-42.jsonl"
        raw.write_text(json.dumps({
            "event": "environmental_chemistry_tick",
            "tick": 1,
            "reaction_work": {},
        }) + "\n", encoding="utf-8")
        result = analyze_opportunities(self.root, [42], 60, summary_only=True)
        self.assertTrue(result["summary_only"])
        self.assertEqual(result["tick_diagnostics"], [])
        self.assertEqual(result["opportunity_samples"], [])
        self.assertEqual(result["per_seed"][0]["coverage"]["break"]["state"], "unknown")

    def test_settlement_summary_preserves_unsettled_and_missing_values(self) -> None:
        summary = summarize_settlements([
            {"reaction": "BREAK", "energy_settlement": {
                "status": "settled", "usable_energy_delta": -2.0,
                "resonance_withdrawn": 1.0, "structural_energy_delta": 0.5,
                "physical_work": 0.25, "heat_dissipated": 0.25,
            }},
            {"reaction": "BREAK", "energy_settlement": {
                "status": "not_settled", "usable_energy_delta": None,
                "resonance_withdrawn": None, "structural_energy_delta": None,
                "physical_work": None, "heat_dissipated": None,
            }},
        ])
        self.assertEqual(summary["BREAK"]["settled_event_count"], 1)
        self.assertEqual(summary["BREAK"]["not_settled_event_count"], 1)
        self.assertEqual(summary["BREAK"]["usable_energy_delta"]["sum"], -2.0)
        self.assertEqual(summary["BREAK"]["heat_dissipated"]["count"], 1)
        self.assertEqual(summary["BREAK"]["heat_dissipated"]["mean"], 0.25)
        self.assertEqual(summary["COMBINE"]["event_count"], 0)
        self.assertIsNone(summary["COMBINE"]["physical_work"]["sum"])

    def test_comparison_rejects_duplicate_seeds(self) -> None:
        chemistry, organization, manifest = self.seed_inputs(42)
        record = build_record(42, chemistry, organization, manifest)
        path = self.root / "record.json"
        path.write_text(json.dumps(record), encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "duplicate"):
            compare([path, path])


if __name__ == "__main__":
    unittest.main()
