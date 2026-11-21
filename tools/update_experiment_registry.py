#!/usr/bin/env python3
"""Update the shared experiment registry after publishing a condition's two-file record."""
from __future__ import annotations

import argparse
from pathlib import Path


def update_registry(
    registry_path: Path,
    experiment_id: str,
    condition_id: str,
    setup_path: str,
    report_path: str,
    repo_root: Path | None = None,
) -> None:
    setup_file = Path(setup_path)
    report_file = Path(report_path)
    if setup_file.is_absolute() or report_file.is_absolute():
        raise ValueError("registry links must be repository-relative paths")
    root = (repo_root or Path.cwd()).resolve()
    if not (root / setup_file).is_file() or not (root / report_file).is_file():
        raise ValueError("both published result files must exist before registry update")

    lines = registry_path.read_text(encoding="utf-8").splitlines()
    matches: list[tuple[int, list[str]]] = []
    for index, line in enumerate(lines):
        if not line.startswith("|"):
            continue
        columns = [column.strip() for column in line.strip().strip("|").split("|")]
        if columns and columns[0] == experiment_id:
            matches.append((index, columns))
    if len(matches) != 1:
        raise ValueError(
            f"expected exactly one registry row for {experiment_id}; found {len(matches)}"
        )

    index, columns = matches[0]
    if len(columns) != 5:
        raise ValueError(f"unexpected registry schema for {experiment_id}; expected five columns")

    setup_link = f"[{condition_id} setup]({setup_file.as_posix()})"
    report_link = f"[{condition_id} report]({report_file.as_posix()})"
    links = f"{setup_link} · {report_link}"
    if links not in columns[4]:
        columns[4] = f"{columns[4]}; {links}" if columns[4] else links
    columns[3] = f"Results recorded: {condition_id}"
    lines[index] = "| " + " | ".join(columns) + " |"
    registry_path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--registry", type=Path, required=True)
    parser.add_argument("--experiment-id", required=True)
    parser.add_argument("--condition-id", required=True)
    parser.add_argument("--setup-path", required=True)
    parser.add_argument("--report-path", required=True)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    args = parser.parse_args()
    update_registry(
        args.registry,
        args.experiment_id,
        args.condition_id,
        args.setup_path,
        args.report_path,
        args.repo_root,
    )
    print(f"Updated registry entry for {args.experiment_id}/{args.condition_id}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
