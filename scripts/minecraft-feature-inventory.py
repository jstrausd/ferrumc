#!/usr/bin/env python3
"""Inventory pinned Minecraft data; data presence is not implemented behavior."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import sys
import tempfile
import tomllib
import unittest


ROOT = Path(__file__).resolve().parent.parent
OUTPUT = Path("docs/fixtures/minecraft-1.21.8-feature-inventory.json")
MAX_FILE_BYTES = 16 * 1024 * 1024
MAX_ENTRIES = 65536
NAME = re.compile(r"[a-z0-9_./-]+\Z")
ITEMS_SHA256 = "a790e2ea77f54cb3d1640577f6c86c94f7f66a3168fe38928fc55066162579bb"


class InventoryError(ValueError):
    """The source data is missing, changed, malformed, or exceeds its limit."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise InventoryError(message)


def source_bytes(path: Path, limit: int = MAX_FILE_BYTES) -> bytes:
    require(path.stat().st_size <= limit, f"source exceeds {limit} bytes: {path}")
    with path.open("rb") as source:
        data = source.read(limit + 1)
    require(len(data) <= limit, f"source grew past {limit} bytes: {path}")
    return data


def unique_object(pairs: list[tuple[str, object]]) -> dict:
    result = {}
    for key, value in pairs:
        require(key not in result, f"duplicate JSON key: {key}")
        result[key] = value
    return result


def parse_json(data: bytes) -> object:
    return json.loads(data, object_pairs_hook=unique_object)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def fingerprint(value: object) -> str:
    return digest(json.dumps(value, sort_keys=True, separators=(",", ":")).encode())


def registry_rows(value: object, kind: str) -> list[dict]:
    if kind == "items":
        require(isinstance(value, dict), "items must be an object")
        rows = []
        for name, entry in value.items():
            require(isinstance(entry, dict), "item entry must be an object")
            rows.append({"name": name, "id": entry.get("id")})
    else:
        require(isinstance(value, list), f"{kind} must be an array")
        rows = value
    require(0 < len(rows) <= MAX_ENTRIES, f"{kind} entry count out of bounds")
    ids, names = set(), set()
    for row in rows:
        require(isinstance(row, dict), f"{kind} entry must be an object")
        name, numeric_id = row.get("name"), row.get("id")
        require(isinstance(name, str) and NAME.fullmatch(name) is not None,
                f"{kind} invalid resource name")
        require(type(numeric_id) is int and 0 <= numeric_id < MAX_ENTRIES,
                f"{kind} invalid numeric id")
        require(name not in names and numeric_id not in ids,
                f"{kind} duplicate name or id")
        names.add(name)
        ids.add(numeric_id)
        if kind == "blocks":
            low, high, default = (row.get(key) for key in
                                  ("minStateId", "maxStateId", "defaultState"))
            require(all(type(n) is int for n in (low, high, default)) and
                    0 <= low <= default <= high < MAX_ENTRIES,
                    "invalid block state range/default")
    require(ids == set(range(len(rows))), f"{kind} numeric ids have gaps")
    rows = sorted(rows, key=lambda row: row["id"])
    if kind == "blocks":
        end = 0
        for row in rows:
            require(row["minStateId"] == end, "block state ranges overlap or have gaps")
            end = row["maxStateId"] + 1
    return rows


def packet_rows(value: object) -> list[str]:
    require(isinstance(value, dict), "protocol must be an object")
    rows = []
    for phase in ("handshaking", "status", "login", "configuration", "play"):
        require(isinstance(value.get(phase), dict), f"missing protocol phase: {phase}")
        for direction in ("toClient", "toServer"):
            try:
                packet = value[phase][direction]["types"]["packet"]
                mappings = packet[1][0]["type"][1]["mappings"]
            except (KeyError, IndexError, TypeError) as error:
                raise InventoryError(f"missing packet mapper: {phase}/{direction}") from error
            require(isinstance(mappings, dict) and len(mappings) <= MAX_ENTRIES,
                    "invalid packet mappings")
            names, ids = set(), set()
            for wire_id, name in mappings.items():
                require(isinstance(wire_id, str) and
                        re.fullmatch(r"0x[0-9a-fA-F]+", wire_id) is not None,
                        "invalid packet wire id")
                numeric_id = int(wire_id, 16)
                require(numeric_id < MAX_ENTRIES and numeric_id not in ids,
                        "duplicate or oversized packet id")
                require(isinstance(name, str) and NAME.fullmatch(name) is not None and
                        name not in names, "invalid or duplicate packet name")
                ids.add(numeric_id)
                names.add(name)
                rows.append(f"{phase}/{direction}/{numeric_id:02x}/{name}")
    return sorted(rows)


def inventory(root: Path) -> dict:
    fixture = Path("fixtures/protocol/1_21_8")
    manifest_data = source_bytes(root / fixture / "manifest.toml")
    manifest = tomllib.loads(manifest_data.decode())
    require(manifest.get("minecraft") == {
        "version": "1.21.8", "protocol_version": 772, "data_version": 4440},
        "manifest Minecraft pin differs")
    require(manifest["source"].get("commit") ==
            "a8cf733fe44f3069f87f63e7ec3d74b521840ded", "manifest source pin differs")
    categories = {}
    for kind in ("blocks", "biomes", "items", "packets"):
        if kind == "items":
            path = Path("server/crates/ferrumc-registry/data/items.json")
            expected_hash = ITEMS_SHA256
            provenance = {"kind": "vendored item-component extract",
                          "minecraft_version": "1.21.8",
                          "original_extraction_record": "not recorded; CAT-02 remains open"}
            expected_size = None
        else:
            filename = "protocol.json" if kind == "packets" else f"{kind}.json"
            path = fixture / filename
            expected_hash = manifest["files"][filename]["sha256"]
            expected_size = manifest["files"][filename]["bytes"]
            provenance = manifest["source"]
        data = source_bytes(root / path)
        require(digest(data) == expected_hash, f"SHA-256 mismatch: {path}")
        require(expected_size is None or len(data) == expected_size,
                f"byte length mismatch: {path}")
        value = parse_json(data)
        parsed = packet_rows(value) if kind == "packets" else registry_rows(value, kind)
        ids = parsed if kind == "packets" else ["minecraft:" + row["name"] for row in parsed]
        category = {"capture_status": "captured_data_only", "implementation_status":
                    "not inferred; assign behavior and acceptance via CAT-03/CAT-04",
                    "count": len(ids), "source": {"path": path.as_posix(),
                    "sha256": digest(data), "bytes": len(data), "provenance": provenance},
                    "resource_ids": ids, "resource_id_order":
                    "phase/direction/hex-id/name" if kind == "packets" else "numeric source id, starting at zero"}
        if kind == "blocks":
            category["block_state_count"] = parsed[-1]["maxStateId"] + 1
            category["block_state_ranges_sha256"] = fingerprint([
                [row["name"], row["minStateId"], row["maxStateId"], row["defaultState"]]
                for row in parsed])
            category["state_properties_source"] = "see source blocks.json; not duplicated here"
        if kind == "biomes":
            category["numeric_id_note"] = "source indices; runtime dynamic registries assign their own IDs"
        categories[kind] = category
    for kind, ticket in (("entities", "CAT-02"), ("worldgen_and_structures", "CAT-02"),
                         ("recipes_and_serializers", "CAT-02"), ("loot_and_predicates", "CAT-02"),
                         ("commands_and_arguments", "CAT-02/CAT-04"),
                         ("gamerules_effects_enchantments_attributes_damage_types", "CAT-02"),
                         ("advancements_statistics_tags_datapacks", "CAT-02"),
                         ("bukkit_paper_api_events", "CAT-05"), ("craftbukkit_nms_plugins", "CAT-06")):
        categories[kind] = {"capture_status": "not_captured", "count": None,
                            "resource_ids": None, "required_ticket": ticket,
                            "reason": "no complete pinned vendored export in this inventory"}
    return {"schema_version": 1, "minecraft": manifest["minecraft"],
            "generator": "scripts/minecraft-feature-inventory.py",
            "meaning": "version-pinned source inventory; gameplay and Java plugin behavior require separate acceptance tests",
            "manifest": {"path": (fixture / "manifest.toml").as_posix(),
                         "sha256": digest(manifest_data)}, "categories": categories}


def rendered(value: object) -> bytes:
    return (json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True) + "\n").encode()


class SelfTests(unittest.TestCase):
    def test_json_rejects_duplicate_keys_and_malformed(self):
        with self.assertRaises(InventoryError):
            parse_json(b'{"a":1,"a":2}')
        with self.assertRaises(json.JSONDecodeError):
            parse_json(b'{')

    def test_registry_boundaries(self):
        rows = [{"id": 0, "name": "air", "minStateId": 0, "maxStateId": 0, "defaultState": 0}]
        self.assertEqual(len(registry_rows(rows, "blocks")), 1)
        for invalid in ([], [{"id": True, "name": "air"}], [{"id": 1, "name": "air"}],
                        [{"id": 0, "name": "../../ Bad"}], [rows[0], rows[0]],
                        [dict(rows[0], defaultState=1)],
                        [rows[0], dict(rows[0], id=1, name="stone", minStateId=0)]):
            with self.assertRaises(InventoryError):
                registry_rows(invalid, "blocks")
        self.assertEqual(registry_rows({"stone": {"id": 1}, "air": {"id": 0}}, "items")[0]["name"], "air")

    def test_packet_missing_mapper(self):
        with self.assertRaises(InventoryError):
            packet_rows({})

    def test_file_limit(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "fixture.json"
            path.write_bytes(b"12345")
            self.assertEqual(source_bytes(path, 5), b"12345")
            with self.assertRaises(InventoryError):
                source_bytes(path, 4)

    def test_generation_deterministic_and_source_hash_enforced(self):
        first = inventory(ROOT)
        self.assertEqual(rendered(first), rendered(inventory(ROOT)))
        self.assertEqual(first["categories"]["entities"]["capture_status"], "not_captured")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / "fixtures/protocol/1_21_8"
            path.mkdir(parents=True)
            (path / "manifest.toml").write_bytes(source_bytes(ROOT / "fixtures/protocol/1_21_8/manifest.toml"))
            (path / "blocks.json").write_bytes(b"[]")
            with self.assertRaisesRegex(InventoryError, "SHA-256 mismatch"):
                inventory(root)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail if committed inventory differs")
    parser.add_argument("--self-test", action="store_true", help="validate parsing, integrity, bounds and determinism")
    args = parser.parse_args()
    if args.self_test:
        result = unittest.TextTestRunner().run(unittest.defaultTestLoader.loadTestsFromTestCase(SelfTests))
        if not result.wasSuccessful():
            return 1
    try:
        data = rendered(inventory(ROOT))
        path = ROOT / OUTPUT
        if args.check:
            require(path.exists() and source_bytes(path) == data,
                    f"inventory differs; run python3 scripts/minecraft-feature-inventory.py to regenerate {OUTPUT}")
            print(f"inventory matches: {OUTPUT}")
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
            print(f"inventory generated: {OUTPUT}")
    except (InventoryError, OSError, ValueError, KeyError, RecursionError) as error:
        print(f"inventory error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
