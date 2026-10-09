#!/usr/bin/env python3
"""Focused safety tests for compact_geometry_practice.py."""
from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))
import compact_geometry_practice as migration  # noqa: E402


SIGNATURE = "v1|Carbon@0,0,0;|"


def write_jsonl(path: Path, records: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        "".join(json.dumps(record, separators=(",", ":")) + "\n" for record in records),
        encoding="utf-8",
    )


class CompactGeometryPracticeTests(unittest.TestCase):
    def make_source(self, root: Path, family_signature: str = SIGNATURE) -> Path:
        source = root / "source"
        source.mkdir()
        write_jsonl(source / "formations.jsonl", [{"signature": SIGNATURE, "schema_version": 1}])
        write_jsonl(
            source / "rigid_contact_families.jsonl",
            [{
                "schema_version": 1,
                "formation_signature": family_signature,
                "candidate_resource": "Nitrogen",
                "anchor_constituent": 0,
                "candidate_rotation_radians": 0.0,
            }],
        )
        (source / "manifest.json").write_text('{"schema_version":1}\n', encoding="utf-8")
        (source / "frontier.json").write_text('{"records":{}}\n', encoding="utf-8")
        return source

    def test_round_trip_preserves_all_fields_and_source(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = self.make_source(root)
            original = (source / "rigid_contact_families.jsonl").read_bytes()
            destination = root / "compact"
            self.assertEqual(migration.run(source, destination), 0)
            compact = json.loads((destination / "rigid_contact_families.jsonl").read_text())
            self.assertNotIn("storage_version", compact)
            self.assertEqual(len(compact["formation_id"]), 22)
            manifest = json.loads((destination / "storage_manifest.json").read_text())
            self.assertEqual(manifest["storage_format_version"], 2)
            self.assertEqual(manifest["formation_id_algorithm"], "sha256-128-base64url")
            self.assertEqual(
                manifest["family_files"]["rigid_contact_families.jsonl"]["rows"],
                1,
            )
            self.assertNotIn("formation_signature", compact)
            self.assertEqual((source / "rigid_contact_families.jsonl").read_bytes(), original)
            restored = dict(compact)
            restored.pop("storage_version")
            restored.pop("formation_id")
            restored["formation_signature"] = SIGNATURE
            self.assertEqual(
                restored,
                {
                    "schema_version": 1,
                    "formation_signature": SIGNATURE,
                    "candidate_resource": "Nitrogen",
                    "anchor_constituent": 0,
                    "candidate_rotation_radians": 0.0,
                },
            )
            self.assertEqual(
                (destination / "frontier.json").read_bytes(),
                (source / "frontier.json").read_bytes(),
            )

    def test_unresolved_reference_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = self.make_source(root, family_signature="not-in-formations")
            with self.assertRaisesRegex(migration.MigrationError, "unresolved formation reference"):
                migration.run(source, root / "compact")

    def test_duplicate_canonical_signature_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "formations.jsonl"
            write_jsonl(path, [{"signature": SIGNATURE}, {"signature": SIGNATURE}])
            with self.assertRaisesRegex(migration.MigrationError, "duplicate canonical"):
                migration.read_formations(path)

    def test_id_collision_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "formations.jsonl"
            write_jsonl(path, [{"signature": "first"}, {"signature": "second"}])
            with mock.patch.object(migration, "formation_id", return_value="A" * 22):
                with self.assertRaisesRegex(migration.MigrationError, "ID collision"):
                    migration.read_formations(path)

    def test_malformed_json_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "formations.jsonl"
            path.write_text('{"signature":"ok"}\n{broken\n', encoding="utf-8")
            with self.assertRaisesRegex(migration.MigrationError, "invalid JSON"):
                migration.read_formations(path)

    def test_nonempty_destination_is_never_overwritten(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = self.make_source(root)
            destination = root / "compact"
            destination.mkdir()
            sentinel = destination / "keep.txt"
            sentinel.write_text("preserve", encoding="utf-8")
            with self.assertRaisesRegex(migration.MigrationError, "destination must be absent or empty"):
                migration.run(source, destination)
            self.assertEqual(sentinel.read_text(encoding="utf-8"), "preserve")


if __name__ == "__main__":
    unittest.main(verbosity=2)
