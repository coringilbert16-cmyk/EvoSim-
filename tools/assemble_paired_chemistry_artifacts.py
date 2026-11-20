#!/usr/bin/env python3
"""Normalize paired experiment artifact names for existing offline analyzers."""
from __future__ import annotations

import argparse
import shutil
from pathlib import Path

CONDITIONS = ("no-feed", "resource-feed")
REQUIRED_SUFFIXES = (".jsonl", "-chemistry.json", "-organization.json")


def assemble(root: Path, seeds: list[int]) -> dict[str, int]:
    """Copy each condition/seed's source files into analyzer-standard directories.

    Source artifacts are preserved. This is a deterministic naming/layout
    adapter only; it does not parse or modify scientific observations.
    """
    copied: dict[str, int] = {}
    for condition in CONDITIONS:
        destination = root / condition
        destination.mkdir(parents=True, exist_ok=True)
        count = 0
        for seed in seeds:
            prefix = f"{condition}-seed-{seed}"
            for suffix in REQUIRED_SUFFIXES:
                source = root / f"{prefix}{suffix}"
                if not source.is_file():
                    raise FileNotFoundError(f"missing paired artifact: {source}")
                target = destination / f"seed-{seed}{suffix}"
                if target.exists():
                    raise FileExistsError(f"refusing to overwrite assembled artifact: {target}")
                shutil.copy2(source, target)
                count += 1
            for suffix in ("-chemistry.md", "-organization.md"):
                source = root / f"{prefix}{suffix}"
                if source.is_file():
                    target = destination / f"seed-{seed}{suffix}"
                    if target.exists():
                        raise FileExistsError(f"refusing to overwrite assembled artifact: {target}")
                    shutil.copy2(source, target)
                    count += 1
        copied[condition] = count
    return copied


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("artifact_dir", type=Path)
    parser.add_argument("--seeds", required=True, help="Comma-separated seed IDs")
    args = parser.parse_args()
    seeds = [int(value) for value in args.seeds.split(",") if value]
    if not seeds or len(seeds) != len(set(seeds)) or any(seed < 0 for seed in seeds):
        parser.error("seeds must be unique nonnegative integers")
    result = assemble(args.artifact_dir, seeds)
    for condition, count in result.items():
        print(f"{condition}: normalized {count} source files")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
