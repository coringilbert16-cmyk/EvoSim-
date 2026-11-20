import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from tools.experiment_manifest import validate_manifest


class ExperimentManifestTests(unittest.TestCase):
    def make_manifest(self, root: Path) -> Path:
        artifact = root / "seed-42.jsonl"
        artifact.write_bytes(b'{"event":"environmental_chemistry_header"}\n')
        manifest = {
            "schema": "evosim.experiment_manifest",
            "schema_version": 1,
            "experiment": {"experiment_id": None, "research_plan": None, "condition_id": None, "seed": 42},
            "run": {
                "status": "completed",
                "started_at_unix_ms": 1_800_000_000_000,
                "completed_at_unix_ms": 1_800_000_060_000,
                "actual_elapsed_wall_seconds": 60.0,
                "start_tick": 0,
                "end_tick": 1200,
            },
            "build": {"repository": None, "commit_sha": None},
            "conditions": {
                "configuration_schema_version": 1,
                "configuration_sha256": "0" * 64,
                "configuration": {"seed": 42},
            },
            "diagnostics": {"raw_schema_versions": {"environmental_chemistry_header": 2}},
            "analyzers": {"source_commit_sha": None},
            "artifacts": [{
                "role": "raw_observation",
                "path": artifact.name,
                "sha256": hashlib.sha256(artifact.read_bytes()).hexdigest(),
                "size_bytes": artifact.stat().st_size,
            }],
            "quality": {"observation_complete": True, "warnings": []},
        }
        path = root / "manifest.json"
        path.write_text(json.dumps(manifest), encoding="utf-8")
        return path

    def test_valid_manifest_and_artifact(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = self.make_manifest(Path(temporary))
            manifest = validate_manifest(path)
        self.assertEqual(manifest["run"]["end_tick"], 1200)

    def test_rejects_artifact_checksum_mismatch(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            path = self.make_manifest(root)
            data = json.loads(path.read_text(encoding="utf-8"))
            data["artifacts"][0]["sha256"] = "f" * 64
            path.write_text(json.dumps(data), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "checksum mismatch"):
                validate_manifest(path)

    def test_rejects_unsupported_schema(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = self.make_manifest(Path(temporary))
            data = json.loads(path.read_text(encoding="utf-8"))
            data["schema_version"] = 2
            path.write_text(json.dumps(data), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "schema_version"):
                validate_manifest(path)

    def test_rejects_negative_or_nonfinite_elapsed_time(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = self.make_manifest(Path(temporary))
            data = json.loads(path.read_text(encoding="utf-8"))
            data["run"]["actual_elapsed_wall_seconds"] = float("nan")
            path.write_text(json.dumps(data), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "finite and non-negative"):
                validate_manifest(path)


if __name__ == "__main__":
    unittest.main()
