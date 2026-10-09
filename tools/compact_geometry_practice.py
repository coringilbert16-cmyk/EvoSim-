#!/usr/bin/env python3
"""Build and verify a compact-v2 practice copy of Bob's geometry library.

This tool never edits the source catalogue. It copies the catalogue to a new
output directory, replacing repeated family formation_signature fields with
deterministic 128-bit IDs. formations.jsonl remains the authoritative source
of canonical signatures. Every transformed row is round-trip checked.
"""
from __future__ import annotations

import argparse
import base64
import hashlib
import json
import shutil
import sys
from pathlib import Path
from typing import Any

STORAGE_VERSION = 2
ID_HEX_LENGTH = 32
FAMILY_SUFFIX = "_families.jsonl"


class MigrationError(RuntimeError):
    pass


def formation_id(signature: str) -> str:
    """Encode a 128-bit SHA-256 prefix compactly as unpadded base64url."""
    digest = hashlib.sha256(signature.encode("utf-8")).digest()[:16]
    return base64.urlsafe_b64encode(digest).decode("ascii").rstrip("=")


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    with path.open("r", encoding="utf-8", newline="") as stream:
        for line_number, line in enumerate(stream, start=1):
            if not line.strip():
                continue
            try:
                record = json.loads(line)
            except json.JSONDecodeError as exc:
                raise MigrationError(f"{path}:{line_number}: invalid JSON: {exc}") from exc
            if not isinstance(record, dict):
                raise MigrationError(f"{path}:{line_number}: expected a JSON object")
            records.append(record)
    return records


def read_formations(path: Path) -> tuple[dict[str, str], dict[str, str]]:
    if not path.is_file():
        raise MigrationError(f"required formations file not found: {path}")
    id_to_signature: dict[str, str] = {}
    signature_to_id: dict[str, str] = {}
    seen_signatures: set[str] = set()
    for row_number, formation in enumerate(read_jsonl(path), start=1):
        signature = formation.get("signature")
        if not isinstance(signature, str) or not signature:
            raise MigrationError(f"{path}: formation row {row_number} has no non-empty signature")
        if signature in seen_signatures:
            raise MigrationError(f"{path}: duplicate canonical formation signature: {signature[:100]!r}")
        seen_signatures.add(signature)
        compact_id = formation_id(signature)
        previous = id_to_signature.get(compact_id)
        if previous is not None and previous != signature:
            raise MigrationError(
                f"128-bit formation ID collision {compact_id}: "
                f"{previous[:100]!r} versus {signature[:100]!r}"
            )
        id_to_signature[compact_id] = signature
        signature_to_id[signature] = compact_id
    return id_to_signature, signature_to_id


def compact_record(
    record: dict[str, Any],
    *,
    path: Path,
    row_number: int,
    signature_to_id: dict[str, str],
) -> dict[str, Any]:
    if "formation_id" in record:
        raise MigrationError(
            f"{path}:{row_number}: source already contains formation_id; refusing to guess its format"
        )
    signature = record.get("formation_signature")
    if not isinstance(signature, str) or not signature:
        raise MigrationError(f"{path}:{row_number}: missing formation_signature")
    compact_id = signature_to_id.get(signature)
    if compact_id is None:
        raise MigrationError(
            f"{path}:{row_number}: unresolved formation reference {signature[:120]!r}"
        )
    compact = dict(record)
    del compact["formation_signature"]
    compact["formation_id"] = compact_id
    return compact


def write_jsonl(path: Path, records: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="\n") as stream:
        for record in records:
            stream.write(json.dumps(record, separators=(",", ":"), ensure_ascii=False))
            stream.write("\n")


def compact_family_file(
    source: Path,
    destination: Path,
    *,
    id_to_signature: dict[str, str],
    signature_to_id: dict[str, str],
) -> dict[str, int]:
    original_records = read_jsonl(source)
    compact_records: list[dict[str, Any]] = []
    reconstructed_records: list[dict[str, Any]] = []
    seen_logical_rows: set[tuple[str, str]] = set()

    for row_number, original in enumerate(original_records, start=1):
        compact = compact_record(
            original,
            path=source,
            row_number=row_number,
            signature_to_id=signature_to_id,
        )
        compact_id = compact["formation_id"]
        resolved_signature = id_to_signature.get(compact_id)
        if resolved_signature != original["formation_signature"]:
            raise MigrationError(
                f"{source}:{row_number}: compact ID does not resolve to the original signature"
            )

        reconstructed = dict(compact)
        reconstructed.pop("formation_id")
        reconstructed["formation_signature"] = resolved_signature
        if reconstructed != original:
            raise MigrationError(
                f"{source}:{row_number}: round-trip reconstruction changed the source record"
            )

        logical_key = (
            resolved_signature,
            json.dumps(
                {key: value for key, value in original.items() if key != "formation_signature"},
                sort_keys=True,
                separators=(",", ":"),
                ensure_ascii=False,
            ),
        )
        seen_logical_rows.add(logical_key)
        compact_records.append(compact)
        reconstructed_records.append(reconstructed)

    write_jsonl(destination, compact_records)
    reloaded = read_jsonl(destination)
    if len(reloaded) != len(original_records):
        raise MigrationError(f"{destination}: row count changed during write")
    for row_number, (actual, expected) in enumerate(zip(reloaded, compact_records), start=1):
        if actual != expected:
            raise MigrationError(f"{destination}:{row_number}: serialized record changed on reload")
    for row_number, (actual, expected) in enumerate(zip(reconstructed_records, original_records), start=1):
        if actual != expected:
            raise MigrationError(f"{source}:{row_number}: final equivalence check failed")

    source_bytes = source.stat().st_size
    destination_bytes = destination.stat().st_size
    return {
        "rows": len(original_records),
        "source_bytes": source_bytes,
        "compact_bytes": destination_bytes,
        "saved_bytes": source_bytes - destination_bytes,
        "duplicate_rows_preserved": len(original_records) - len(seen_logical_rows),
    }


def run(source: Path, destination: Path) -> int:
    source = source.resolve()
    destination = destination.resolve()
    if not source.is_dir():
        raise MigrationError(f"source directory does not exist: {source}")
    if source == destination or source in destination.parents or destination in source.parents:
        raise MigrationError("source and destination must be separate, non-nested directories")
    if destination.exists() and any(destination.iterdir()):
        raise MigrationError(f"destination must be absent or empty: {destination}")

    formations_path = source / "formations.jsonl"
    id_to_signature, signature_to_id = read_formations(formations_path)

    destination.mkdir(parents=True, exist_ok=True)
    family_paths = sorted(source.glob(f"*{FAMILY_SUFFIX}"))
    if not family_paths:
        raise MigrationError(f"no *{FAMILY_SUFFIX} files found in {source}")

    reports: list[tuple[str, dict[str, int]]] = []
    family_names = {path.name for path in family_paths}
    storage_manifest: dict[str, Any] = {
        "storage_format_version": STORAGE_VERSION,
        "formation_id_algorithm": "sha256-128-base64url",
        "formation_file": "formations.jsonl",
        "family_files": {},
    }
    for path in sorted(source.iterdir()):
        if not path.is_file():
            continue
        target = destination / path.name
        if path.name in family_names:
            report = compact_family_file(
                path,
                target,
                id_to_signature=id_to_signature,
                signature_to_id=signature_to_id,
            )
            reports.append((path.name, report))
            storage_manifest["family_files"][path.name] = {
                "rows": report["rows"],
                "formation_id_field": "formation_id",
            }
        else:
            shutil.copy2(path, target)

    manifest_path = destination / "storage_manifest.json"
    with manifest_path.open("w", encoding="utf-8", newline="\\n") as stream:
        json.dump(storage_manifest, stream, separators=(",", ":"), ensure_ascii=False)
        stream.write("\\n")

    source_total = sum(item["source_bytes"] for _, item in reports)
    compact_total = sum(item["compact_bytes"] for _, item in reports)
    print("Bob geometry library compact-v2 practice migration")
    print(f"Source:      {source}")
    print(f"Destination: {destination}")
    print(f"Formations indexed: {len(signature_to_id):,}")
    print("Family file results (source files remain untouched):")
    for name, report in reports:
        print(
            f"  {name}: rows={report['rows']:,}, "
            f"source={report['source_bytes']:,} B, "
            f"compact={report['compact_bytes']:,} B, "
            f"saved={report['saved_bytes']:,} B, "
            f"duplicate rows preserved={report['duplicate_rows_preserved']:,}"
        )
    print(
        f"TOTAL family files: source={source_total:,} B, "
        f"compact={compact_total:,} B, saved={source_total - compact_total:,} B"
    )
    print(f"Storage manifest: {manifest_path}")
    print("PASS: every compact ID resolved; every family row round-tripped exactly.")
    print("NOTE: this validates the data transformation, not the Rust reader/writer integration.")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path, help="source geometry_library/data directory")
    parser.add_argument("destination", type=Path, help="new practice-copy output directory")
    args = parser.parse_args()
    try:
        return run(args.source, args.destination)
    except (MigrationError, OSError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
