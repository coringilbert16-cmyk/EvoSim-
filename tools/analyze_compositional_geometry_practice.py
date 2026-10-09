#!/usr/bin/env python3
"""Measure exact one-constituent compositional encoding; never writes catalogue data."""
from __future__ import annotations

import argparse
import base64
import hashlib
import json
import math
from pathlib import Path
from typing import Any


def compact_json(value: Any) -> str:
    return json.dumps(value, separators=(",", ":"), ensure_ascii=False)


def formation_id(signature: str) -> str:
    digest = hashlib.sha256(signature.encode("utf-8")).digest()[:16]
    return base64.urlsafe_b64encode(digest).decode("ascii").rstrip("=")


def quantize(value: float) -> int:
    scaled = value / 1e-9
    return math.floor(scaled + 0.5) if scaled >= 0 else math.ceil(scaled - 0.5)


def signature_from_parts(schema_version: int, constituents: list[dict], bonds: list[dict]) -> str:
    out = f"v{schema_version}|"
    for item in constituents:
        placement = item["placement"]
        angle = (float(placement["rotation_radians"]) + math.pi) % math.tau - math.pi
        out += (
            item["resource"] + "@" + str(quantize(float(placement["x"]))) + ","
            + str(quantize(float(placement["y"]))) + ","
            + str(quantize(angle)) + ";"
        )
    out += "|"
    for bond in bonds:
        out += f'{int(bond["constituent_a"])}-{int(bond["constituent_b"])};'
    return out


def read_rows(path: Path) -> list[dict]:
    rows = []
    with path.open(encoding="utf-8") as stream:
        for line_no, line in enumerate(stream, 1):
            try:
                row = json.loads(line)
            except json.JSONDecodeError as exc:
                raise ValueError(f"{path}:{line_no}: invalid JSON: {exc}") from exc
            if isinstance(row, dict):
                rows.append(row)
            else:
                raise ValueError(f"{path}:{line_no}: expected object")
    return rows


def analyze(source: Path) -> dict:
    rows = read_rows(source / "formations.jsonl")
    by_signature: dict[str, dict] = {}
    by_id: dict[str, dict] = {}
    for row in rows:
        signature = row.get("signature")
        if not isinstance(signature, str) or not signature:
            raise ValueError("formation without signature")
        reconstructed = signature_from_parts(row["schema_version"], row["constituents"], row["bonds"])
        if reconstructed != signature:
            raise ValueError(f"canonical signature mismatch: {signature[:80]!r}")
        if signature in by_signature:
            raise ValueError(f"duplicate canonical signature: {signature[:80]!r}")
        row_id = formation_id(signature)
        if row_id in by_id and by_id[row_id]["signature"] != signature:
            raise ValueError(f"128-bit ID collision: {row_id}")
        by_signature[signature] = row
        by_id[row_id] = row

    baseline_bytes = 0
    encoded_bytes = 0
    composable = 0
    savings = 0
    chain_counts: dict[int, int] = {}
    for row in rows:
        sig = row["signature"]
        row_id = formation_id(sig)
        compact_full = dict(row)
        compact_full.pop("signature")
        compact_full["formation_id"] = row_id
        full_size = len(compact_json(compact_full).encode("utf-8"))
        baseline_bytes += full_size
        best_size = full_size
        best_candidate = None
        constituents = row["constituents"]
        bonds = row["bonds"]
        if len(constituents) > 1:
            for removed_index, unit in enumerate(constituents):
                kept = [c for i, c in enumerate(constituents) if i != removed_index]
                index_map = {old: (old if old < removed_index else old - 1)
                             for old in range(len(constituents)) if old != removed_index}
                kept_bonds = []
                removed_bonds = []
                for bond_index, bond in enumerate(bonds):
                    a, b = int(bond["constituent_a"]), int(bond["constituent_b"])
                    if a == removed_index or b == removed_index:
                        removed_bonds.append({"at": bond_index, "bond": dict(bond)})
                    else:
                        kept_bonds.append({
                            **bond,
                            "constituent_a": index_map[a],
                            "constituent_b": index_map[b],
                        })
                base_sig = signature_from_parts(row["schema_version"], kept, kept_bonds)
                base = by_signature.get(base_sig)
                # Canonical signatures omit any extra persisted fields; require the
                # actual subset record to match exactly before referencing it.
                if (base is None
                        or base["schema_version"] != row["schema_version"]
                        or base["constituents"] != kept
                        or base["bonds"] != kept_bonds):
                    continue
                # Encoding is lossless: base ID, insertion index, full removed constituent,
                # and original-index bond records incident to the removed constituent.
                candidate = {
                    "formation_id": row_id,
                    "schema_version": row["schema_version"],
                    "base_id": formation_id(base_sig),
                    "insert_at": removed_index,
                    "constituent": unit,
                    "incident_bonds": removed_bonds,
                }
                candidate_size = len(compact_json(candidate).encode("utf-8"))
                # Decode the proposed delta immediately and compare exact geometry/bond fields.
                base_row = by_id[candidate["base_id"]]
                restored_constituents = list(base_row["constituents"])
                restored_constituents.insert(removed_index, unit)
                restored_bonds = []
                incident_by_position = {item["at"]: dict(item["bond"]) for item in removed_bonds}
                base_bond_iter = iter(base_row["bonds"])
                for original_position in range(len(base_row["bonds"]) + len(removed_bonds)):
                    if original_position in incident_by_position:
                        restored_bonds.append(incident_by_position[original_position])
                    else:
                        base_bond = next(base_bond_iter)
                        restored = dict(base_bond)
                        for endpoint in ("constituent_a", "constituent_b"):
                            if int(restored[endpoint]) >= removed_index:
                                restored[endpoint] = int(restored[endpoint]) + 1
                        restored_bonds.append(restored)
                if (restored_constituents != constituents
                        or restored_bonds != bonds
                        or signature_from_parts(row["schema_version"], restored_constituents, restored_bonds) != sig):
                    raise ValueError(f"delta round-trip mismatch for formation {row_id}")
                if candidate_size < best_size:
                    best_size, best_candidate = candidate_size, candidate
        if best_candidate is not None:
            composable += 1
            savings += full_size - best_size
            chain_counts[len(constituents)] = chain_counts.get(len(constituents), 0) + 1
        encoded_bytes += best_size

    return {
        "formation_rows": len(rows),
        "baseline_compact_formation_bytes": baseline_bytes,
        "compositional_formation_bytes": encoded_bytes,
        "net_saved_bytes": baseline_bytes - encoded_bytes,
        "rows_with_profitable_one_unit_delta": composable,
        "rows_by_constituent_count": dict(sorted(chain_counts.items())),
        "note": "Analytical size estimate only; no source or compact files are written.",
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path, help="geometry_library/data directory")
    args = parser.parse_args()
    result = analyze(args.source)
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
