import unittest

from tools.chemistry_analyzer import CANONICAL_RESOURCES, analyze, render_markdown


def fixture_records():
    inventory = {name: 10.0 for name in CANONICAL_RESOURCES}
    return [
        {
            "event": "environmental_chemistry_header",
            "version": 2,
            "seed": 42,
            "requested_duration_seconds": 60,
            "planned_ticks": 600,
            "initial_physical_material_count": 1280,
            "initial_independent_resource_count": 1200,
            "initial_molecule_count": 80,
            "initial_material_inventory": inventory,
            "diagnostic_interval": 100,
        },
        {
            "event": "environmental_chemistry_snapshot",
            "label": "initial",
            "tick": 0,
            "physical_material_count": 1280,
            "constituent_count": 1360,
            "internal_bond_count": 80,
            "material_inventory": inventory,
            "energy": {"balance_residual": 0.0},
        },
        {
            "event": "environmental_reaction",
            "reaction": {
                "reaction": "BREAK",
                "outcome": "committed",
                "failure_reason": None,
            },
        },
        {
            "event": "environmental_chemistry_summary",
            "break_ready_candidates": 4,
            "break_scheduled": 2,
            "break_committed": 1,
            "break_rejected": 1,
            "combine_ready_candidates": 3,
            "combine_scheduled": 2,
            "combine_committed": 2,
            "combine_rejected": 0,
            "displacement_ready_candidates": 0,
            "displacement_scheduled": 0,
            "displacement_committed": 0,
            "displacement_rejected": 0,
            "displacement_failure_reasons": {},
            "displacement_participant_sets": {},
            "displacement_discovery": {},
            "displacement_discovery_budget_exhausted_ticks": 0,
        },
    ]


class ChemistryAnalyzerTests(unittest.TestCase):
    def test_checks_all_seven_resources_without_reaction_bias(self):
        report = analyze(fixture_records())
        self.assertEqual(report["resource_coverage"]["canonical_resources"],
                         list(CANONICAL_RESOURCES))
        self.assertTrue(report["resource_coverage"]["all_seven_present_at_start"])
        self.assertEqual(len(report["resource_coverage"]["initial_inventory"]), 7)

    def test_reports_missing_resource_instead_of_silently_ignoring_it(self):
        records = fixture_records()
        del records[0]["initial_material_inventory"]["Oxygen"]
        report = analyze(records)
        self.assertFalse(report["resource_coverage"]["all_seven_present_at_start"])
        self.assertEqual(report["resource_coverage"]["missing_at_start"], ["Oxygen"])

    def test_accepts_explicit_water_exclusion_for_active_experiments(self):
        records = fixture_records()
        records[0]["excluded_resources"] = ["Water"]
        del records[0]["initial_material_inventory"]["Water"]

        report = analyze(records)
        coverage = report["resource_coverage"]
        self.assertEqual(coverage["canonical_resources"], list(CANONICAL_RESOURCES))
        self.assertEqual(coverage["excluded_resources"], ["Water"])
        self.assertEqual(len(coverage["required_resources"]), 6)
        self.assertTrue(coverage["all_required_present_at_start"])
        self.assertFalse(coverage["all_seven_present_at_start"])
        self.assertEqual(coverage["missing_at_start"], [])
        self.assertEqual(report["resource_coverage"]["snapshots"][0]["missing_resources"], [])

    def test_uses_summary_totals_and_reaction_events(self):
        report = analyze(fixture_records())
        self.assertEqual(report["reactions"]["diagnostic_totals"]["BREAK"]["committed"], 1)
        self.assertEqual(report["reactions"]["event_counts"]["BREAK"], 1)

    def test_reports_local_resonance_sampling_for_all_three_reactions(self):
        records = fixture_records()
        records.append({
            "event": "environmental_chemistry_tick",
            "tick": 10,
            "reaction_work": {
                "break_discovery": {
                    "local_resonance_sample_attempts": 2,
                    "local_resonance_samples": 2,
                    "local_resonance_unavailable": 0,
                    "local_resonance_similarity_sum": 0.5,
                    "local_resonance_similarity_max": 0.4,
                },
                "combine_search": {
                    "local_resonance_sample_attempts": 1,
                    "local_resonance_samples": 1,
                    "local_resonance_unavailable": 0,
                    "local_resonance_similarity_sum": 0.25,
                    "local_resonance_similarity_max": 0.25,
                },
                "displacement_discovery": {
                    "local_resonance_sample_attempts": 1,
                    "local_resonance_samples": 0,
                    "local_resonance_unavailable": 1,
                    "local_resonance_similarity_sum": 0.0,
                    "local_resonance_similarity_max": 0.0,
                },
            },
        })
        totals = analyze(records)["reactions"]["diagnostic_totals"]
        self.assertEqual(totals["BREAK"]["local_resonance"]["successful_samples"], 2)
        self.assertEqual(totals["BREAK"]["local_resonance"]["mean_similarity"], 0.25)
        self.assertEqual(totals["COMBINE"]["local_resonance"]["mean_activation_multiplier"], 1.25)
        self.assertEqual(totals["DISPLACEMENT"]["local_resonance"]["sample_attempts"], 1)
        self.assertEqual(totals["DISPLACEMENT"]["local_resonance"]["successful_samples"], 0)
        self.assertEqual(totals["DISPLACEMENT"]["local_resonance"]["unavailable_samples"], 1)
        markdown = render_markdown(analyze(records))
        self.assertIn("Local resonance activation sampling", markdown)

    def test_markdown_explicitly_lists_all_resources(self):
        markdown = render_markdown(analyze(fixture_records()))
        for name in CANONICAL_RESOURCES:
            self.assertIn(f"| {name} |", markdown)

    def test_molecular_lifecycle_reports_lifetimes_and_lineage(self):
        records = fixture_records()
        records.extend([
            {"event": "molecular_lifecycle", "action": "created", "tick": 10,
             "molecule_id": 101, "parent_ids": [], "reaction_id": 1,
             "reaction": "SEED",
             "molecule": {"composition": [["Carbon", 1.0]], "bonds": []}},
            {"event": "molecular_lifecycle", "action": "created", "tick": 20,
             "molecule_id": 102, "parent_ids": [101], "reaction_id": 2,
             "reaction": "BREAK",
             "molecule": {"composition": [["Carbon", 1.0]], "bonds": []}},
            {"event": "molecular_lifecycle", "action": "destroyed", "tick": 20,
             "molecule_id": 101, "reaction_id": 2, "reaction": "BREAK"},
            {"event": "molecular_lifecycle", "action": "destroyed", "tick": 30,
             "molecule_id": 102, "reaction_id": 3, "reaction": "COMBINE"},
        ])
        history = analyze(records)["molecular_history"]
        self.assertEqual(history["molecule_creation_count"], 2)
        self.assertEqual(history["molecule_destruction_count"], 2)
        self.assertEqual(history["mean_observed_lifetime_ticks"], 10.0)
        self.assertEqual(history["lineage_edges"][0]["parent_id"], 101)
        self.assertEqual(history["lineage_edges"][0]["child_id"], 102)
        self.assertEqual(len(history["structural_families"]), 1)
        self.assertEqual(history["structural_families"][0]["molecule_ids"], [101, 102])
        self.assertEqual(history["validation_errors"], [])

    def test_active_molecules_are_right_censored_not_treated_as_destroyed(self):
        records = fixture_records()
        records.append({
            "event": "molecular_lifecycle", "action": "created",
            "tick": 5, "molecule_id": 201, "parent_ids": [],
        })
        history = analyze(records)["molecular_history"]
        self.assertEqual(history["active_at_end_count"], 1)
        self.assertTrue(history["molecules"][0]["right_censored"])
        self.assertIsNone(history["molecules"][0]["destroyed_tick"])

    def test_molecular_lifecycle_rejects_duplicate_or_unknown_destruction(self):
        records = fixture_records()
        records.extend([
            {"event": "molecular_lifecycle", "action": "created",
             "tick": 4, "molecule_id": 301, "parent_ids": []},
            {"event": "molecular_lifecycle", "action": "destroyed",
             "tick": 5, "molecule_id": 301},
            {"event": "molecular_lifecycle", "action": "destroyed",
             "tick": 6, "molecule_id": 301},
            {"event": "molecular_lifecycle", "action": "destroyed",
             "tick": 7, "molecule_id": 999},
            {"event": "molecular_lifecycle", "action": "created",
             "tick": 8, "molecule_id": 302, "parent_ids": [999]},
        ])
        errors = analyze(records)["molecular_history"]["validation_errors"]
        self.assertEqual(len(errors), 3)


if __name__ == "__main__":
    unittest.main()
