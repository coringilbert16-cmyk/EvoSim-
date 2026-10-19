#!/usr/bin/env python3
"""Read-only size audit for Bob's geometry-library data directory.

Usage:
    python3 tools/audit_geometry_storage.py [geometry_library/data]

The compact-size estimate models Bob's current family encoding: replace
"formation_signature" with the runtime's 22-character unpadded base64url
ID derived from the first 128 bits of SHA-256. It estimates family rows only; it does not model compositional-v3 formation
deltas or prove reference coverage. It checks proposed ID collisions across
the scanned JSONL files. It is read-only and never migrates data.
"""
from __future__ import annotations

import base64
import hashlib
import json
import sys
from pathlib import Path


COMPACT_ID_PLACEHOLDER = "A" * 22


def formation_id(signature: str) -> str:
    """Match Bob's stable 128-bit SHA-256 prefix, base64url encoded without padding."""
    digest = hashlib.sha256(signature.encode("utf-8")).digest()[:16]
    return base64.urlsafe_b64encode(digest).decode("ascii").rstrip("=")


def compact_estimate(record: dict) -> int | None:
    if not isinstance(record.get("formation_signature"), str):
        return None
    compact = dict(record)
    compact.pop("formation_signature", None)
    compact["formation_id"] = COMPACT_ID_PLACEHOLDER
    return len(json.dumps(compact, separators=(",", ":"), ensure_ascii=False).encode("utf-8"))


def audit_jsonl(path: Path) -> dict:
    total_bytes = path.stat().st_size
    rows = 0
    invalid_rows = 0
    signature_bytes = 0
    signature_rows = 0
    max_row_bytes = 0
    estimated_compact_bytes = 0
    distinct_signatures: set[str] = set()
    id_to_signature: dict[str, str] = {}
    id_collisions: list[dict[str, str]] = []

    with path.open("rb") as stream:
        for raw_line in stream:
            row_bytes = len(raw_line)
            max_row_bytes = max(max_row_bytes, row_bytes)
            if not raw_line.strip():
                estimated_compact_bytes += row_bytes
                continue
            rows += 1
            try:
                record = json.loads(raw_line)
            except (UnicodeDecodeError, json.JSONDecodeError):
                invalid_rows += 1
                estimated_compact_bytes += row_bytes
                continue
            if not isinstance(record, dict):
                estimated_compact_bytes += row_bytes
                continue
            signature = record.get("formation_signature")
            if isinstance(signature, str):
                signature_rows += 1
                signature_bytes += len(signature.encode("utf-8"))
                distinct_signatures.add(signature)
                compact_id = formation_id(signature)
                previous = id_to_signature.setdefault(compact_id, signature)
                if previous != signature:
                    id_collisions.append({
                        "formation_id": compact_id,
                        "first_signature": previous,
                        "colliding_signature": signature,
                    })
                estimate = compact_estimate(record)
                if estimate is None:
                    estimated_compact_bytes += row_bytes
                else:
                    # Preserve the JSONL newline in the estimate.
                    estimated_compact_bytes += estimate + (1 if raw_line.endswith(b"\n") else 0)
            else:
                estimated_compact_bytes += row_bytes

    return {
        "file": path.name,
        "bytes": total_bytes,
        "rows": rows,
        "invalid_rows": invalid_rows,
        "rows_with_formation_signature": signature_rows,
        "distinct_formation_signatures": len(distinct_signatures),
        "duplicate_signature_rows": signature_rows - len(distinct_signatures),
        "formation_id_collisions": id_collisions,
        "formation_signature_utf8_bytes": signature_bytes,
        "average_row_bytes": round(total_bytes / rows, 1) if rows else 0,
        "max_row_bytes": max_row_bytes,
        "estimated_compact_bytes": estimated_compact_bytes,
        "estimated_savings_bytes": total_bytes - estimated_compact_bytes,
    }


def audit_id_collisions(paths: list[Path]) -> list[dict[str, str]]:
    """Check proposed IDs across the entire directory, not just individual files."""
    id_to_signature: dict[str, str] = {}
    collisions: list[dict[str, str]] = []
    for path in paths:
        with path.open("rb") as stream:
            for raw_line in stream:
                if not raw_line.strip():
                    continue
                try:
                    record = json.loads(raw_line)
                except (UnicodeDecodeError, json.JSONDecodeError):
                    continue
                if not isinstance(record, dict):
                    continue
                signature = record.get("formation_signature")
                if not isinstance(signature, str):
                    continue
                compact_id = formation_id(signature)
                previous = id_to_signature.setdefault(compact_id, signature)
                if previous != signature:
                    collisions.append({
                        "file": path.name,
                        "formation_id": compact_id,
                        "first_signature": previous,
                        "colliding_signature": signature,
                    })
    return collisions


def human_bytes(value: int) -> str:
    amount = float(value)
    for unit in ("B", "KiB", "MiB", "GiB", "TiB"):
        if abs(amount) < 1024 or unit == "TiB":
            return f"{amount:.2f} {unit}"
        amount /= 1024
    return f"{value} B"


def main() -> int:
    root = Path(sys.argv[1] if len(sys.argv) > 1 else "geometry_library/data")
    if not root.is_dir():
        print(f"error: data directory not found: {root}", file=sys.stderr)
        return 2

    paths = sorted(root.glob("*.jsonl"))
    if not paths:
        print(f"error: no JSONL files found in {root}", file=sys.stderr)
        return 2

    results = [audit_jsonl(path) for path in paths]
    total_bytes = sum(item["bytes"] for item in results)
    estimated_bytes = sum(item["estimated_compact_bytes"] for item in results)
    collisions = audit_id_collisions(paths)
    print(f"Geometry library storage audit: {root}")
    print("Read-only; compact sizes are estimates, not measured migration output.\n")
    print(
        f"{'file':38} {'disk':>11} {'rows':>10} {'avg row':>10} "
        f"{'sig bytes':>12} {'est. saved':>12}"
    )
    print("-" * 100)
    for item in results:
        print(
            f"{item['file'][:38]:38} "
            f"{human_bytes(item['bytes']):>11} "
            f"{item['rows']:>10,} "
            f"{human_bytes(int(item['average_row_bytes'])):>10} "
            f"{human_bytes(item['formation_signature_utf8_bytes']):>12} "
            f"{human_bytes(item['estimated_savings_bytes']):>12}"
        )
    print("-" * 100)
    print(f"{'TOTAL JSONL':38} {human_bytes(total_bytes):>11}")
    print(f"Estimated compact JSONL total: {human_bytes(estimated_bytes)}")
    print(f"Estimated savings: {human_bytes(total_bytes - estimated_bytes)}")
    print(f"Proposed 128-bit formation-ID collisions across all files: {len(collisions)}")
    if collisions:
        print("ERROR: collisions must be resolved before migration.")
        print(json.dumps(collisions, indent=2))
    print("\nPer-file details:")
    print(json.dumps(results, indent=2))
    return 1 if collisions else 0


if __name__ == "__main__":
    raise SystemExit(main())
