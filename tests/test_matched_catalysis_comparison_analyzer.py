import json
import tempfile
import unittest
from pathlib import Path

from tools.matched_catalysis_comparison_analyzer import analyze


CONFIG = {
    "candidate_parts": ["Hydrogen", "Sulfur"],
    "target_reactant_a_parts": ["Oxygen", "Phosphorus"],
    "target_reactant_b_parts": ["Carbon", "Hydrogen"],
    "near_distance": 8.0,
    "minimum_remote_distance": 60.0,
    "max_target_pair_distance": 30.0,
}


def records(seed, condition, committed):
    candidate_id, target_a_id, target_b_id = 30, 10, 20
    inventory = {"Carbon": 2, "Hydrogen": 2, "Oxygen": 1, "Phosphorus": 1, "Sulfur": 1}
    metadata = {
        "seed": seed,
        "condition": condition,
        "candidate_identity": candidate_id,
        "target_reactant_a_identity": target_a_id,
        "target_reactant_b_identity": target_b_id,
        "initial_elemental_inventory_unchanged": True,
        "initial_energy_inventory_unchanged": True,
        "all_non_candidate_molecule_geometry_unchanged": True,
    }
    rows = [
        {"event": "environmental_chemistry_header", "seed": seed,
         "requested_duration_seconds": 60, "initial_material_inventory": inventory},
        {"event": "environmental_chemistry_experiment_setup", "metadata": metadata},
    ]
    for identity, parts in (
        (candidate_id, [["Hydrogen", 1], ["Sulfur", 1]]),
        (target_a_id, [["Oxygen", 1], ["Phosphorus", 1]]),
        (target_b_id, [["Carbon", 1], ["Hydrogen", 1]]),
    ):
        rows.append({
            "event": "molecular_lifecycle", "action": "created", "molecule_id": identity,
            "molecule": {"material": {"parts": parts}},
        })
    rows.append({
        "event": "environmental_chemistry_tick", "tick": 12,
        "reaction_work": {"combine_search": {"opportunity_samples": [{
            "source_a_identity": target_a_id,
            "source_b_identity": target_b_id,
            "physically_valid": True,
            "exact_transaction_match_count": int(committed),
            "local_neighborhood": {
                "molecules": [{"identity": candidate_id}] if condition == "candidate_near" else [],
                "truncated_molecule_count": 0,
                "missing_geometry_count": 0,
            },
        }]}}
    })
    if committed:
        rows.append({
            "event": "environmental_reaction",
            "reaction": {
                "reaction": "COMBINE", "outcome": "committed",
                "source_material_ids": [target_a_id, target_b_id],
            },
        })
    rows.append({
        "event": "environmental_chemistry_summary",
        "energy_balance_residual": 0.0,
    })
    return rows


class MatchedCatalysisComparisonAnalyzerTests(unittest.TestCase):
    def _write_arm(self, directory, seed, condition, committed):
        directory.mkdir(parents=True, exist_ok=True)
        path = directory / f"seed-{seed}.jsonl"
        path.write_text(
            "".join(json.dumps(row) + "\\n" for row in records(seed, condition, committed)),
            encoding="utf-8",
        )

    def test_compares_paired_arms_and_reports_candidate_neighborhood(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            near, remote = root / "near", root / "remote"
            self._write_arm(near, 42, "candidate_near", True)
            self._write_arm(remote, 42, "candidate_remote", False)

            report = analyze(near, remote, [42], 60, CONFIG)

        pair = report["paired_seeds"][0]
        self.assertEqual(pair["delta_target_committed_reactions_near_minus_remote"], 1)
        self.assertEqual(pair["near"]["target_samples_with_candidate_nearby"], 1)
        self.assertEqual(pair["remote"]["target_samples_with_candidate_nearby"], 0)
        self.assertEqual(report["pooled"]["near"]["target_committed_reactions"], 1)
        self.assertIn("not proof", " ".join(report["interpretation_limits"]))

    def test_rejects_paired_arms_with_different_initial_inventory(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            near, remote = root / "near", root / "remote"
            self._write_arm(near, 42, "candidate_near", True)
            self._write_arm(remote, 42, "candidate_remote", False)
            path = remote / "seed-42.jsonl"
            rows = [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines()]
            rows[0]["initial_material_inventory"]["Carbon"] = 999
            path.write_text("".join(json.dumps(row) + "\\n" for row in rows), encoding="utf-8")

            with self.assertRaisesRegex(ValueError, "different initial elemental inventories"):
                analyze(near, remote, [42], 60, CONFIG)


if __name__ == "__main__":
    unittest.main()
