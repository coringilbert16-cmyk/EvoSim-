import json
import tempfile
import unittest
from pathlib import Path

from tools.geometry_aware_reaction_network import analyze, geometry_descriptor


def molecule(parts, positions, rotations=None, bonds=None):
    rotations = rotations or [0.0] * len(parts)
    bonds = bonds or []
    return {
        "environmental_identity": 1,
        "material": {
            "parts": [[name, 1.0] for name in parts],
            "internal_bonds": [
                {"part_a": a, "part_b": b} for a, b in bonds
            ],
        },
        "placements": [
            {"x": x, "y": y, "rotation_radians": rotations[index]}
            for index, (x, y) in enumerate(positions)
        ],
        "realized_shapes": [{"form": {"test_shape": index}} for index in range(len(parts))],
        "internal_connections": [
            {
                "part_a": a,
                "part_b": b,
                "endpoint_a": {"Boundary": {"angle_radians": 0.0}},
                "endpoint_b": {"Boundary": {"angle_radians": 1.0}},
                "bond_energy": 0.25,
            }
            for a, b in bonds
        ],
    }


def write_seed(root):
    first = molecule(["Carbon", "Hydrogen"], [(0.0, 0.0), (1.0, 0.0)], bonds=[(0, 1)])
    second = molecule(["Oxygen", "Sulfur"], [(3.0, 0.0), (4.0, 0.0)], bonds=[(0, 1)])
    product = molecule(
        ["Carbon", "Hydrogen", "Oxygen", "Sulfur"],
        [(0.0, 0.0), (1.0, 0.0), (2.0, 0.0), (3.0, 0.0)],
        bonds=[(0, 1), (1, 2), (2, 3)],
    )
    chemistry = {
        "input": {
            "seed": 42,
            "requested_duration_seconds": 300,
            "initial_physical_material_count": 1280,
            "initial_independent_resource_count": 0,
            "initial_molecule_count": 1280,
            "initial_constituent_count": 2560,
        },
        "molecular_history": {
            "validation_errors": [],
            "molecules": [
                {
                    "molecule_id": 1, "created_tick": 0, "destroyed_tick": 1,
                    "observed_lifetime_ticks": 1, "right_censored": False,
                    "parent_ids": [], "reaction_id": "initial", "reaction": "INITIAL",
                    "molecule": first,
                },
                {
                    "molecule_id": 2, "created_tick": 0, "destroyed_tick": 1,
                    "observed_lifetime_ticks": 1, "right_censored": False,
                    "parent_ids": [], "reaction_id": "initial", "reaction": "INITIAL",
                    "molecule": second,
                },
                {
                    "molecule_id": 3, "created_tick": 1, "destroyed_tick": None,
                    "observed_lifetime_ticks": 2, "right_censored": True,
                    "parent_ids": [1, 2], "reaction_id": "1-0", "reaction": "COMBINE",
                    "molecule": product,
                },
            ],
        },
        "reactions": {
            "elemental_bond_pairs": {
                "event_attribution": [
                    {
                        "reaction_id": "1-0", "reaction": "COMBINE", "tick": 1,
                        "formed": {"Carbon-Oxygen": 1}, "broken": {},
                    }
                ]
            }
        },
    }
    (root / "seed-42-chemistry.json").write_text(json.dumps(chemistry), encoding="utf-8")
    event = {
        "event": "environmental_reaction",
        "reaction": {
            "reaction": "COMBINE",
            "outcome": "committed",
            "tick": 1,
            "molecular_lifecycle": [
                {
                    "action": "destroyed", "molecule_id": 1, "reaction_id": "1-0",
                    "molecule": first,
                },
                {
                    "action": "destroyed", "molecule_id": 2, "reaction_id": "1-0",
                    "molecule": second,
                },
                {
                    "action": "created", "molecule_id": 3, "reaction_id": "1-0",
                    "molecule": product,
                },
            ],
        },
    }
    (root / "seed-42.jsonl").write_text(json.dumps(event) + "\n", encoding="utf-8")


class GeometryAwareNetworkTests(unittest.TestCase):
    def test_geometry_is_invariant_to_world_translation_and_rotation(self):
        original = molecule(
            ["Carbon", "Hydrogen"],
            [(0.0, 0.0), (1.0, 0.0)],
            rotations=[0.2, 0.4],
            bonds=[(0, 1)],
        )
        angle = 1.2
        cosine, sine = __import__("math").cos(angle), __import__("math").sin(angle)
        transformed_positions = []
        for x, y in [(0.0, 0.0), (1.0, 0.0)]:
            transformed_positions.append((
                8.0 + x * cosine - y * sine,
                -3.0 + x * sine + y * cosine,
            ))
        transformed = molecule(
            ["Carbon", "Hydrogen"],
            transformed_positions,
            rotations=[0.2 + angle, 0.4 + angle],
            bonds=[(0, 1)],
        )
        first = geometry_descriptor(original)
        second = geometry_descriptor(transformed)
        self.assertEqual(first["constituents"], second["constituents"])
        self.assertEqual(first["pairwise_constituent_distances"], second["pairwise_constituent_distances"])
        self.assertNotEqual(first["world_centroid"], second["world_centroid"])

    def test_vertical_wrap_seam_preserves_compact_intrinsic_geometry(self):
        across_seam = molecule(
            ["Carbon", "Hydrogen"],
            [(10.0, 249.8), (10.0, 0.2)],
            bonds=[(0, 1)],
        )
        descriptor = geometry_descriptor(across_seam, world_height=250.0)
        self.assertIsNotNone(descriptor)
        self.assertEqual(
            descriptor["pairwise_constituent_distances"][0]["distance"], 0.4
        )
        self.assertAlmostEqual(
            descriptor["constituents"][0]["y"], -0.2, places=6
        )
        self.assertAlmostEqual(
            descriptor["constituents"][1]["y"], 0.2, places=6
        )

    def test_missing_physical_geometry_is_reported_not_fabricated(self):
        incomplete = molecule(["Carbon"], [(0.0, 0.0)])
        incomplete["placements"] = None
        self.assertIsNone(geometry_descriptor(incomplete))

    def test_reconstructs_instance_nodes_and_reaction_time_geometry(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_seed(root)
            report = analyze(root, [42], 300)
        summary = report["summary"]
        self.assertEqual(summary["molecule_instances"], 3)
        self.assertEqual(summary["instances_with_creation_geometry"], 3)
        self.assertEqual(summary["instances_with_reaction_time_geometry"], 2)
        self.assertEqual(summary["committed_reaction_events"], 1)
        self.assertEqual(summary["reaction_events_with_complete_participant_geometry"], 1)
        event = report["reaction_events"][0]
        self.assertEqual(event["parent_ids"], [1, 2])
        self.assertEqual(event["child_ids"], [3])
        self.assertEqual(event["formed_bond_pairs"]["Carbon-Oxygen"], 1)


if __name__ == "__main__":
    unittest.main()
