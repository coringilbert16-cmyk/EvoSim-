#!/usr/bin/env python3
"""Build an isolated, losslessly round-tripped compositional-v3 practice copy."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import compact_geometry_practice as compact


GEOMETRY_FIELDS = {"signature", "formation_id", "schema_version", "constituents", "bonds"}


def full_compact_row(source_row: dict[str, Any], row_id: str) -> dict[str, Any]:
    row = dict(source_row)
    row.pop("signature")
    row["formation_id"] = row_id
    return row


def make_delta(
    row: dict[str, Any],
    row_id: str,
    removed_index: int,
    base_id: str,
    constituent: dict[str, Any],
    incident_bonds: list[dict[str, Any]],
) -> dict[str, Any]:
    delta: dict[str, Any] = {
        "formation_id": row_id,
        "schema_version": row["schema_version"],
        "base_id": base_id,
        "insert_at": removed_index,
        "constituent": constituent,
        "incident_bonds": incident_bonds,
    }
    extras = {key: value for key, value in row.items() if key not in GEOMETRY_FIELDS}
    if extras:
        delta["extra_fields"] = extras
    return delta


def decode_all(encoded_rows: list[dict[str, Any]]) -> dict[str, dict[str, Any]]:
    by_id = {row["formation_id"]: row for row in encoded_rows}
    if len(by_id) != len(encoded_rows):
        raise ValueError("duplicate formation_id in compositional output")
    cache: dict[str, dict[str, Any]] = {}
    active: set[str] = set()

    def decode(row_id: str) -> dict[str, Any]:
        if row_id in cache:
            return cache[row_id]
        if row_id in active:
            raise ValueError(f"cyclic compositional reference at {row_id}")
        row = by_id.get(row_id)
        if row is None:
            raise ValueError(f"unresolved compositional base ID: {row_id}")
        if "base_id" not in row:
            cache[row_id] = dict(row)
            return cache[row_id]

        active.add(row_id)
        base_id = row["base_id"]
        base_row = decode(base_id)
        insert_at = int(row["insert_at"])
        constituents = list(base_row["constituents"])
        if insert_at < 0 or insert_at > len(constituents):
            raise ValueError(f"invalid insertion index for {row_id}")
        constituents.insert(insert_at, row["constituent"])

        incident = {int(item["at"]): dict(item["bond"]) for item in row["incident_bonds"]}
        if len(incident) != len(row["incident_bonds"]):
            raise ValueError(f"duplicate incident-bond position for {row_id}")
        base_bonds = []
        for source_bond in base_row["bonds"]:
            bond = dict(source_bond)
            for endpoint in ("constituent_a", "constituent_b"):
                value = int(bond[endpoint])
                bond[endpoint] = value + 1 if value >= insert_at else value
            base_bonds.append(bond)

        bonds = []
        base_iter = iter(base_bonds)
        total_bonds = len(base_bonds) + len(incident)
        if any(position < 0 or position >= total_bonds for position in incident):
            raise ValueError(f"invalid incident-bond position for {row_id}")
        for position in range(total_bonds):
            if position in incident:
                bonds.append(incident[position])
            else:
                try:
                    bonds.append(next(base_iter))
                except StopIteration as exc:
                    raise ValueError(f"invalid bond reconstruction for {row_id}") from exc
        try:
            next(base_iter)
            raise ValueError(f"unconsumed base bonds for {row_id}")
        except StopIteration:
            pass

        restored: dict[str, Any] = {
            "formation_id": row_id,
            "schema_version": row["schema_version"],
            "constituents": constituents,
            "bonds": bonds,
        }
        restored.update(row.get("extra_fields", {}))
        active.remove(row_id)
        cache[row_id] = restored
        return restored

    for row_id in by_id:
        decode(row_id)
    return cache


def reference_depths(encoded_rows: list[dict[str, Any]]) -> dict[str, float | int]:
    """Report reference-chain depth, where a full row has depth zero."""
    by_id = {row["formation_id"]: row for row in encoded_rows}
    cache: dict[str, int] = {}
    active: set[str] = set()

    def depth(row_id: str) -> int:
        if row_id in cache:
            return cache[row_id]
        if row_id in active:
            raise ValueError(f"cyclic compositional reference at {row_id}")
        row = by_id.get(row_id)
        if row is None:
            raise ValueError(f"unresolved compositional base ID: {row_id}")
        if "base_id" not in row:
            cache[row_id] = 0
            return 0
        active.add(row_id)
        result = 1 + depth(row["base_id"])
        active.remove(row_id)
        cache[row_id] = result
        return result

    values = [depth(row_id) for row_id in by_id]
    ordered = sorted(values)
    count = len(ordered)
    return {
        "max_reference_depth": max(ordered, default=0),
        "mean_reference_depth": round(sum(ordered) / count, 3) if count else 0,
        "p95_reference_depth": ordered[min(count - 1, int((count - 1) * 0.95))] if count else 0,
        "rows_with_depth_over_8": sum(value > 8 for value in ordered),
    }


def build_compositional_copy(source: Path, destination: Path) -> dict[str, int]:
    source = source.resolve()
    destination = destination.resolve()
    # Reuse the previously validated family-reference migration. It creates only a
    # separate destination and leaves every source file untouched.
    compact.run(source, destination)

    source_rows = compact.read_jsonl(source / "formations.jsonl")
    signature_to_id: dict[str, str] = {}
    source_by_id: dict[str, dict[str, Any]] = {}
    full_rows: dict[str, dict[str, Any]] = {}
    for row in source_rows:
        signature = row.get("signature")
        if not isinstance(signature, str) or not signature:
            raise ValueError("formation row lacks canonical signature")
        if compact.canonical_signature_from_record(row) != signature:
            raise ValueError(f"canonical signature mismatch: {signature[:80]!r}")
        row_id = compact.formation_id(signature)
        if row_id in source_by_id and source_by_id[row_id]["signature"] != signature:
            raise ValueError(f"128-bit ID collision: {row_id}")
        signature_to_id[signature] = row_id
        source_by_id[row_id] = row
        full_rows[row_id] = full_compact_row(row, row_id)

    encoded_by_id: dict[str, dict[str, Any]] = {}
    for row_id, row in source_by_id.items():
        full = full_rows[row_id]
        best = full
        best_size = len(compact.compact_json(full).encode("utf-8")) if hasattr(compact, "compact_json") else len(
            json.dumps(full, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
        )
        constituents = row["constituents"]
        bonds = row["bonds"]
        if len(constituents) > 1:
            for removed_index, unit in enumerate(constituents):
                kept = [item for index, item in enumerate(constituents) if index != removed_index]
                index_map = {
                    old: old if old < removed_index else old - 1
                    for old in range(len(constituents)) if old != removed_index
                }
                kept_bonds: list[dict[str, Any]] = []
                incident_bonds: list[dict[str, Any]] = []
                for bond_index, bond in enumerate(bonds):
                    a, b = int(bond["constituent_a"]), int(bond["constituent_b"])
                    if a == removed_index or b == removed_index:
                        incident_bonds.append({"at": bond_index, "bond": dict(bond)})
                    else:
                        kept_bonds.append({
                            **bond,
                            "constituent_a": index_map[a],
                            "constituent_b": index_map[b],
                        })
                base_signature = compact.canonical_signature_from_record({
                    "schema_version": row["schema_version"],
                    "constituents": kept,
                    "bonds": kept_bonds,
                })
                base_id = signature_to_id.get(base_signature)
                base = source_by_id.get(base_id) if base_id else None
                # A signature match alone is insufficient: every persisted field in
                # the subset must match before it can serve as the base record.
                if (base is None
                        or base["schema_version"] != row["schema_version"]
                        or base["constituents"] != kept
                        or base["bonds"] != kept_bonds):
                    continue
                candidate = make_delta(row, row_id, removed_index, base_id, unit, incident_bonds)
                candidate_size = len(json.dumps(candidate, separators=(",", ":"), ensure_ascii=False).encode("utf-8"))
                if candidate_size >= best_size:
                    continue
                # Full recursive decoding and exact comparison are performed for
                # every chosen row after the complete encoded table has been built.
                best, best_size = candidate, candidate_size
        encoded_by_id[row_id] = best

    encoded_rows = [encoded_by_id[compact.formation_id(row["signature"])] for row in source_rows]
    restored_by_id = decode_all(encoded_rows)
    for row in source_rows:
        row_id = compact.formation_id(row["signature"])
        expected = full_rows[row_id]
        if restored_by_id[row_id] != expected:
            raise ValueError(f"recursive round-trip mismatch for {row_id}")
        restored_signature = compact.canonical_signature_from_record(restored_by_id[row_id])
        if restored_signature != row["signature"]:
            raise ValueError(f"recursive signature mismatch for {row_id}")

    output_path = destination / "formations.jsonl"
    compact.write_jsonl(output_path, encoded_rows)
    reloaded = compact.read_jsonl(output_path)
    if reloaded != encoded_rows:
        raise ValueError("serialized compositional formations changed after reload")

    manifest_path = destination / "storage_manifest.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["storage_format_version"] = 3
    manifest["formation_encoding"] = "exact-one-constituent-delta-v1"
    manifest["formation_id_algorithm"] = "sha256-128-base64url"
    manifest["formation_rows"] = len(encoded_rows)
    manifest["formation_rows_delta_encoded"] = sum("base_id" in row for row in encoded_rows)
    manifest["formation_rows_full"] = len(encoded_rows) - manifest["formation_rows_delta_encoded"]
    manifest_path.write_text(json.dumps(manifest, separators=(",", ":"), ensure_ascii=False) + "\n", encoding="utf-8")

    baseline_bytes = sum(len(json.dumps(row, separators=(",", ":"), ensure_ascii=False).encode("utf-8")) + 1 for row in full_rows.values())
    encoded_bytes = output_path.stat().st_size
    depth_stats = reference_depths(encoded_rows)
    print(json.dumps({
        "formation_rows": len(encoded_rows),
        "full_compact_formation_bytes": baseline_bytes,
        "compositional_formation_file_bytes": encoded_bytes,
        "saved_bytes": baseline_bytes - encoded_bytes,
        "delta_encoded_rows": manifest["formation_rows_delta_encoded"],
        "full_encoded_rows": manifest["formation_rows_full"],
        **depth_stats,
        "recursive_round_trip": "passed for every formation",
        "source_modified": False,
    }, indent=2, sort_keys=True))
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    args = parser.parse_args()
    build_compositional_copy(args.source, args.destination)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
