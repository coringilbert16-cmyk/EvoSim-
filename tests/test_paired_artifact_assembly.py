import tempfile
import unittest
from pathlib import Path

from tools.assemble_paired_chemistry_artifacts import assemble


class PairedArtifactAssemblyTests(unittest.TestCase):
    def _write_pair(self, root: Path, seed: int, condition: str) -> None:
        prefix = root / f"{condition}-seed-{seed}"
        for suffix in (".jsonl", "-chemistry.json", "-organization.json"):
            prefix.with_name(prefix.name + suffix).write_text("{}", encoding="utf-8")

    def test_normalizes_condition_prefixed_filenames_without_touching_sources(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for condition in ("no-feed", "resource-feed"):
                for seed in (42, 43):
                    self._write_pair(root, seed, condition)
            counts = assemble(root, [42, 43])
            self.assertEqual(counts, {"no-feed": 6, "resource-feed": 6})
            self.assertTrue((root / "no-feed/seed-42-chemistry.json").is_file())
            self.assertTrue((root / "resource-feed/seed-43-organization.json").is_file())
            self.assertTrue((root / "no-feed-seed-42-chemistry.json").is_file())

    def test_fails_closed_when_a_required_source_is_missing(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self._write_pair(root, 42, "no-feed")
            with self.assertRaises(FileNotFoundError):
                assemble(root, [42])


if __name__ == "__main__":
    unittest.main()
