import importlib.util
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).resolve().parents[1] / "tools" / "chemistry_organization_analyzer.py"
SPEC = importlib.util.spec_from_file_location("chemistry_organization_analyzer", MODULE_PATH)
analyzer = importlib.util.module_from_spec(SPEC)
assert SPEC and SPEC.loader
SPEC.loader.exec_module(analyzer)


def lifecycle(
    action, molecule_id, tick, molecule=None, parents=None, reaction="INITIAL", reaction_id=None
):
    return {
        "event": "molecular_lifecycle",
        "action": action,
        "molecule_id": molecule_id,
        "tick": tick,
        "molecule": molecule or {},
        "parent_ids": parents or [],
        "reaction_id": reaction_id or f"reaction-{molecule_id}",
        "reaction": reaction,
    }


def molecule(parts, connections=None):
    connections = connections or []
    bonds = [
        {"part_a": edge["part_a"], "part_b": edge["part_b"]}
        for edge in connections
        if isinstance(edge, dict) and "part_a" in edge and "part_b" in edge
    ]
    return {
        "material": {"parts": parts, "internal_bonds": bonds},
        "internal_connections": connections,
    }


class ChemicalOrganizationAnalyzerTests(unittest.TestCase):
    def test_groups_recurrent_structures_and_reports_active_family(self):
        records = [
            lifecycle("created", 1, 0, molecule([["Carbon", 1.0], ["Oxygen", 1.0]]), reaction="COMBINE"),
            lifecycle("destroyed", 1, 10),
            lifecycle("created", 2, 5, molecule([["Carbon", 1.0], ["Oxygen", 1.0]]), reaction="COMBINE"),
            {"event": "environmental_chemistry_tick", "tick": 20},
        ]
        report = analyzer.analyze(records)
        organization = report["organization"]
        self.assertEqual(organization["unique_exact_structural_families"], 1)
        self.assertEqual(organization["recurrent_families"], 1)
        self.assertEqual(organization["families_active_at_end"], 1)
        self.assertEqual(organization["top_families"][0]["occurrences"], 2)
        self.assertEqual(organization["top_families"][0]["active_at_end"], 1)

    def test_initial_seed_recurrence_is_not_mistaken_for_emergent_recurrence(self):
        compound = molecule([["Hydrogen", 1.0], ["Sulfur", 1.0]])
        records = [
            lifecycle("created", 1, 0, compound, reaction="INITIAL"),
            lifecycle("created", 2, 0, compound, reaction="INITIAL"),
            lifecycle("created", 3, 5, compound, [1], "COMBINE"),
            {"event": "environmental_chemistry_tick", "tick": 10},
        ]
        report = analyzer.analyze(records)
        family = report["organization"]["top_families"][0]
        self.assertEqual(family["initial_seed_occurrences"], 2)
        self.assertEqual(family["reaction_created_occurrences"], 1)
        self.assertFalse(family["recurrent_from_reactions"])
        self.assertEqual(report["organization"]["recurrent_families"], 0)

    def test_sibling_products_from_one_reaction_do_not_count_as_recurrence(self):
        compound = molecule(
            [["Carbon", 1.0], ["Oxygen", 1.0]],
            [{"part_a": 0, "part_b": 1}],
        )
        records = [
            lifecycle("created", 1, 0, molecule([["Carbon", 1.0]])),
            lifecycle("created", 2, 5, compound, [1], "COMBINE", "tick-5-reaction-0"),
            lifecycle("created", 3, 5, compound, [1], "COMBINE", "tick-5-reaction-0"),
            {"event": "environmental_chemistry_tick", "tick": 10},
        ]
        report = analyzer.analyze(records)
        family = report["organization"]["top_families"][0]
        self.assertEqual(family["reaction_created_occurrences"], 2)
        self.assertEqual(family["reaction_created_reaction_events"], 1)
        self.assertFalse(family["recurrent_from_reactions"])
        self.assertEqual(report["organization"]["recurrent_families"], 0)

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

    def test_family_signature_ignores_geometry_but_preserves_bond_connectivity(self):
        first = molecule(
            [["Hydrogen", 1.0], ["Sulfur", 1.0]],
            [{"part_a": 0, "part_b": 1, "bond_energy": 0.15,
              "endpoint_a": {"angle": 0.1}}],
        )
        second = molecule(
            [["Sulfur", 1.0], ["Hydrogen", 1.0]],
            [{"part_a": 1, "part_b": 0, "bond_energy": 0.25,
              "endpoint_a": {"angle": 2.4}}],
        )
        records = [
            lifecycle("created", 1, 0, first, reaction="COMBINE"),
            lifecycle("created", 2, 1, second, reaction="COMBINE"),
            {"event": "environmental_chemistry_tick", "tick": 3},
        ]
        report = analyzer.analyze(records)
        self.assertEqual(report["organization"]["unique_exact_structural_families"], 1)
        self.assertEqual(report["organization"]["recurrent_families"], 1)

    def test_family_signature_distinguishes_different_bond_connectivity(self):
        first = molecule(
            [["Hydrogen", 1.0], ["Sulfur", 1.0], ["Carbon", 1.0]],
            [{"part_a": 0, "part_b": 1}],
        )
        second = molecule(
            [["Hydrogen", 1.0], ["Sulfur", 1.0], ["Carbon", 1.0]],
            [{"part_a": 0, "part_b": 2}],
        )
        report = analyzer.analyze([
            lifecycle("created", 1, 0, first),
            lifecycle("created", 2, 1, second),
            {"event": "environmental_chemistry_tick", "tick": 2},
        ])
        self.assertEqual(report["organization"]["unique_exact_structural_families"], 2)

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
        compound_amounts = report["material_distribution"]["compound_constituent_amount_by_resource"]
        self.assertEqual(compound_amounts, {"Carbon": 1.0, "Oxygen": 2.0})
        reaction_compounds = report["material_distribution"][
            "reaction_created_compound_constituent_amount_by_resource"
        ]
        self.assertEqual(reaction_compounds, {})

    def test_reports_resource_units_in_reaction_created_compounds(self):
        records = [
            lifecycle("created", 1, 0, molecule([["Carbon", 1.0]])),
            lifecycle(
                "created", 2, 4,
                molecule(
                    [["Carbon", 1.0], ["Oxygen", 1.0]],
                    [{"part_a": 0, "part_b": 1}],
                ),
                [1],
                "COMBINE",
            ),
            {"event": "environmental_chemistry_tick", "tick": 8},
        ]
        report = analyzer.analyze(records)
        self.assertEqual(
            report["material_distribution"]["reaction_created_compound_constituent_amount_by_resource"],
            {"Carbon": 1.0, "Oxygen": 1.0},
        )

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
