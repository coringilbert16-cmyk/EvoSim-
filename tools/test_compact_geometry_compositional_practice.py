#!/usr/bin/env python3
"""Focused round-trip tests for the isolated compositional geometry encoder."""
from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import compact_geometry_practice as compact  # noqa: E402
import compact_geometry_compositional_practice as composition  # noqa: E402


def formation(constituents: list[dict], bonds: list[dict]) -> dict:
    row = {"schema_version": 1, "constituents": constituents, "bonds": bonds}
    row["signature"] = compact.canonical_signature_from_record(row)
    return row


def carbon(x: float) -> dict:
    return {
        "resource": "Carbon",
        "placement": {"x": x, "y": 0.0, "rotation_radians": 0.0},
    }


def write_jsonl(path: Path, rows: list[dict]) -> None:
    path.write_text("".join(json.dumps(row, separators=(",", ":")) + "\n" for row in rows), encoding="utf-8")


class CompositionalPracticeTests(unittest.TestCase):
    def test_recursive_round_trip_and_source_preservation(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source"
            source.mkdir()
            one = formation([carbon(0.0)], [])
            two = formation(
                [carbon(0.0), carbon(1.0)],
                [{"constituent_a": 0, "constituent_b": 1}],
            )
            write_jsonl(source / "formations.jsonl", [one, two])
            write_jsonl(source / "rigid_contact_families.jsonl", [{
                "schema_version": 1,
                "formation_signature": two["signature"],
                "candidate_resource": "Nitrogen",
                "anchor_constituent": 0,
                "candidate_rotation_radians": 0.0,
            }])
            original = {p.name: p.read_bytes() for p in source.iterdir()}
            destination = root / "compositional"
            composition.build_compositional_copy(source, destination)

            encoded = compact.read_jsonl(destination / "formations.jsonl")
            self.assertEqual(len(encoded), 2)
            self.assertTrue(any("base_id" in row for row in encoded))
            decoded = composition.decode_all(encoded)
            for row in [one, two]:
                row_id = compact.formation_id(row["signature"])
                expected = dict(row)
                expected.pop("signature")
                expected["formation_id"] = row_id
                self.assertEqual(decoded[row_id], expected)
            manifest = json.loads((destination / "storage_manifest.json").read_text(encoding="utf-8"))
            self.assertEqual(manifest["storage_format_version"], 3)
            self.assertEqual(manifest["formation_encoding"], "exact-one-constituent-delta-v1")
            self.assertEqual({p.name: p.read_bytes() for p in source.iterdir()}, original)


if __name__ == "__main__":
    unittest.main(verbosity=2)
