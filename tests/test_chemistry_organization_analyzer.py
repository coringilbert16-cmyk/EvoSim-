import importlib.util
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).resolve().parents[1] / "tools" / "chemistry_organization_analyzer.py"
SPEC = importlib.util.spec_from_file_location("chemistry_organization_analyzer", MODULE_PATH)
analyzer = importlib.util.module_from_spec(SPEC)
assert SPEC and SPEC.loader
SPEC.loader.exec_module(analyzer)


def lifecycle(action, molecule_id, tick, molecule=None, parents=None, reaction="INITIAL"):
    return {
        "event": "molecular_lifecycle",
        "action": action,
        "molecule_id": molecule_id,
        "tick": tick,
        "molecule": molecule or {},
        "parent_ids": parents or [],
        "reaction": reaction,
    }


def molecule(parts, connections=None):
    return {
        "material": {"parts": parts},
        "internal_connections": connections or [],
    }


class ChemicalOrganizationAnalyzerTests(unittest.TestCase):
    def test_groups_recurrent_structures_and_reports_active_family(self):
        records = [
            lifecycle("created", 1, 0, molecule([["Carbon", 1.0], ["Oxygen", 1.0]])),
            lifecycle("destroyed", 1, 10),
            lifecycle("created", 2, 5, molecule([["Oxygen", 1.0], ["Carbon", 1.0]])),
            {"event": "environmental_chemistry_tick", "tick": 20},
        ]
        report = analyzer.analyze(records)
        organization = report["organization"]
        self.assertEqual(organization["unique_exact_structural_families"], 1)
        self.assertEqual(organization["recurrent_families"], 1)
        self.assertEqual(organization["families_active_at_end"], 1)
        self.assertEqual(organization["top_families"][0]["occurrences"], 2)
        self.assertEqual(organization["top_families"][0]["active_at_end"], 1)

    def test_single_resource_unbonded_materials_do_not_count_as_organized_families(self):
        records = [
            lifecycle("created", 1, 0, molecule([["Carbon", 1.0]])),
            lifecycle("created", 2, 0, molecule([["Carbon", 1.0]])),
            {"event": "environmental_chemistry_tick", "tick": 5},
        ]
        report = analyzer.analyze(records)
        self.assertEqual(report["organization"]["unique_exact_structural_families"], 0)
        self.assertEqual(report["organization"]["recurrent_families"], 0)
        self.assertEqual(
            report["organization"]["single_resource_unbonded_identities_excluded"], 2
        )

    def test_lineage_records_reaction_family_transition(self):
        parent = molecule([["Carbon", 1.0]])
        child = molecule([["Carbon", 1.0], ["Oxygen", 1.0]], [{"from": 0, "to": 1}])
        records = [
            lifecycle("created", 1, 0, parent),
            lifecycle("created", 2, 8, child, [1], "COMBINE"),
            {"event": "environmental_chemistry_tick", "tick": 12},
        ]
        report = analyzer.analyze(records)
        transitions = report["reaction_network"]["family_to_family_transitions"]
        self.assertEqual(len(transitions), 1)
        self.assertEqual(transitions[0]["reaction"], "COMBINE")
        self.assertEqual(transitions[0]["count"], 1)

    def test_lifetimes_separate_completed_and_right_censored(self):
        records = [
            lifecycle("created", 1, 2, molecule([["Hydrogen", 1.0]])),
            lifecycle("destroyed", 1, 9),
            lifecycle("created", 2, 4, molecule([["Sulfur", 1.0]])),
            {"event": "environmental_chemistry_tick", "tick": 20},
        ]
        report = analyzer.analyze(records)
        self.assertEqual(report["persistence"]["completed_lifetime_count"], 1)
        self.assertEqual(report["persistence"]["right_censored_count"], 1)
        self.assertEqual(report["persistence"]["mean_observed_lifetime_ticks"], 11.5)

    def test_material_distribution_counts_each_identity_once(self):
        records = [
            lifecycle("created", 1, 0, molecule([["Carbon", 1.0], ["Oxygen", 2.0]])),
            lifecycle("created", 2, 1, molecule([["Carbon", 1.0]]), [1], "BREAK"),
            {"event": "environmental_chemistry_tick", "tick": 2},
        ]
        report = analyzer.analyze(records)
        amounts = report["material_distribution"]["constituent_amount_by_resource"]
        self.assertEqual(amounts, {"Carbon": 2.0, "Oxygen": 2.0})

    def test_rejects_duplicate_creation_and_invalid_destruction(self):
        records = [
            lifecycle("created", 1, 0, molecule([["Carbon", 1.0]])),
            lifecycle("created", 1, 1, molecule([["Carbon", 1.0]])),
            lifecycle("destroyed", 99, 2),
            {"event": "environmental_chemistry_tick", "tick": 3},
        ]
        report = analyzer.analyze(records)
        self.assertEqual(len(report["scope"]["lifecycle_validation_errors"]), 2)

    def test_rejects_lineage_to_missing_or_already_destroyed_parent(self):
        records = [
            lifecycle("created", 1, 0, molecule([["Carbon", 1.0]])),
            lifecycle("destroyed", 1, 2),
            lifecycle("created", 2, 3, molecule([["Carbon", 1.0], ["Oxygen", 1.0]]), [1], "COMBINE"),
            lifecycle("created", 3, 4, molecule([["Sulfur", 1.0]]), [99], "BREAK"),
            {"event": "environmental_chemistry_tick", "tick": 5},
        ]
        report = analyzer.analyze(records)
        errors = report["scope"]["lifecycle_validation_errors"]
        self.assertTrue(any("already destroyed" in error for error in errors))
        self.assertTrue(any("no prior creation" in error for error in errors))

    def test_requires_molecular_history(self):
        with self.assertRaisesRegex(ValueError, "no molecular_lifecycle"):
            analyzer.analyze([{"event": "environmental_chemistry_tick", "tick": 1}])


if __name__ == "__main__":
    unittest.main()
