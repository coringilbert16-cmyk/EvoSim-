#!/usr/bin/env python3
"""Read-only size audit for Bob's geometry-library data directory.

Usage:
    python3 tools/audit_geometry_storage.py [geometry_library/data]

The compact-size estimate replaces each JSONL record's repeated
"formation_signature" field with a 32-hex-character formation_id plus
storage_version=2. It is an estimate, not a migration or a guarantee of
exact serialized Rust output. No files are modified.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path


COMPACT_ID_PLACEHOLDER = "0" * 32


def compact_estimate(record: dict) -> int | None:
    if not isinstance(record.get("formation_signature"), str):
        return None
    compact = dict(record)
    compact.pop("formation_signature", None)
    compact["formation_id"] = COMPACT_ID_PLACEHOLDER
    compact["storage_version"] = 2
    return len(json.dumps(compact, separators=(",", ":"), ensure_ascii=False).encode("utf-8"))


def audit_jsonl(path: Path) -> dict:
    total_bytes = path.stat().st_size
    rows = 0
    invalid_rows = 0
    signature_bytes = 0
    signature_rows = 0
    max_row_bytes = 0
    estimated_compact_bytes = 0

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
                estimate = compact_estimate(record)
                if estimate is None:
                    estimated_compact_bytes += row_bytes
                else:
                    # Preserve the JSONL newline in the estimate.
                    estimated_compact_bytes += estimate + (1 if raw_line.endswith(b"\\n") else 0)
            else:
                estimated_compact_bytes += row_bytes

    return {
        "file": path.name,
        "bytes": total_bytes,
        "rows": rows,
        "invalid_rows": invalid_rows,
        "rows_with_formation_signature": signature_rows,
        "formation_signature_utf8_bytes": signature_bytes,
        "average_row_bytes": round(total_bytes / rows, 1) if rows else 0,
        "max_row_bytes": max_row_bytes,
        "estimated_compact_bytes": estimated_compact_bytes,
        "estimated_savings_bytes": total_bytes - estimated_compact_bytes,
    }


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
    print("\nPer-file details:")
    print(json.dumps(results, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
