#!/usr/bin/env python3
"""Export an entity-type oracle from the pinned original 1.21.8 report.

No download or Java process occurs here. CAT-02.1 owns the original data export.
This helper verifies that its unchanged entity-type fixture equals the complete
entity-type subset of the independently pinned full report before exporting.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import sys
import tempfile
import unittest


CRATE = Path(__file__).resolve().parents[2]
ROOT = CRATE.parents[2]
ORIGINAL = ROOT / "docs/fixtures/minecraft-1.21.8-original"
OUTPUT = CRATE / "tests/fixtures"
REGISTRY = "minecraft:entity_type"
REPORT_PIN = (466543, "177a6ad99d7b6d28f27e49231d15a5ccdb262bed9ac07f7e66dab5d32cc20c27")
FIXTURE_PIN = (9319, "b65a61ac9f4346ad74ff159cb3f8be78ec96229ccfc3abe29a14a7e0063d4d6b")
ORIGINAL_MANIFEST_PIN = (7061, "d5b4506f5749d114d7b54c50231ee00f6f15d14b2826f8c39489df06932ed9ee")
VERSION_PIN = (43719, "3bf5cdd444dfa89199fe4f8a92b6c79f4e4fa8977ff52029c3acd99378506fad")
MINECRAFT = {"version": "1.21.8", "protocol_version": 772, "data_version": 4440}
SERVER_SHA256 = "2349d9a8f0d4be2c40e7692890ef46a4b07015e7955b075460d02793be7fbbe7"
INNER_SHA256 = "2bee159e4ce87ae857b4323f9f32c7a3d7d448f7f8c3f62fa6f295a72e228250"
MAX_BYTES = 1024 * 1024
RESOURCE = re.compile(r"minecraft:[a-z0-9_./-]+\Z")


class ExportError(ValueError):
    """A classified provenance, schema or integrity refusal."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ExportError(message)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def pinned_read(path: Path, pin: tuple[int, str]) -> bytes:
    require(path.is_file() and not path.is_symlink(), f"missing/unsafe source: {path}")
    require(pin[0] <= MAX_BYTES and path.stat().st_size == pin[0], f"pinned length mismatch: {path}")
    with path.open("rb") as source:
        data = source.read(pin[0] + 1)
    require((len(data), digest(data)) == pin, f"pinned SHA/length mismatch: {path}")
    return data


def unique_object(pairs: list[tuple[str, object]]) -> dict:
    result = {}
    for key, value in pairs:
        require(key not in result, "duplicate JSON key")
        result[key] = value
    return result


def reject_constant(value: str) -> None:
    raise ExportError(f"non-finite JSON constant: {value}")


def parse(data: bytes) -> object:
    require(len(data) <= MAX_BYTES, "JSON byte bound")
    try:
        return json.loads(data, object_pairs_hook=unique_object, parse_constant=reject_constant)
    except (UnicodeError, json.JSONDecodeError, RecursionError) as error:
        raise ExportError(f"JSON schema: {error}") from error


def entries(fixture: object) -> list[tuple[int, str]]:
    require(isinstance(fixture, dict), "registry object absent")
    require(set(fixture) == {"registry", "protocol_id", "default", "entries"}, "registry schema differs")
    require(fixture["registry"] == REGISTRY and type(fixture["protocol_id"]) is int
            and fixture["protocol_id"] == 5 and fixture["default"] == "minecraft:pig", "registry pin differs")
    values = fixture["entries"]
    require(isinstance(values, dict) and len(values) == 151, "registry count differs")
    rows = []
    for name, value in values.items():
        require(isinstance(name, str) and RESOURCE.fullmatch(name) is not None
                and all(part not in ("", ".", "..") for part in name.split(":", 1)[1].split("/")),
                "resource location differs")
        require(isinstance(value, dict) and set(value) == {"protocol_id"}
                and type(value["protocol_id"]) is int and 0 <= value["protocol_id"] < 151,
                "entry protocol id differs")
        rows.append((value["protocol_id"], name))
    rows.sort()
    require([row[0] for row in rows] == list(range(151)), "ids duplicate or non-contiguous")
    require("minecraft:player" in values and "minecraft:pig" in values, "player/default absent")
    return rows


def original_fixture() -> tuple[dict, dict]:
    fixture = parse(pinned_read(ORIGINAL / "entity-types.json", FIXTURE_PIN))
    entries(fixture)
    manifest = parse(pinned_read(ORIGINAL / "manifest.json", ORIGINAL_MANIFEST_PIN))
    version = parse(pinned_read(ORIGINAL / "version-manifest.json", VERSION_PIN))
    require(isinstance(manifest, dict) and manifest.get("schema_version") == 1
            and manifest.get("minecraft") == MINECRAFT, "original manifest release differs")
    require(isinstance(version, dict) and version.get("id") == "1.21.8"
            and version.get("javaVersion", {}).get("majorVersion") == 21, "original Java/release differs")
    source = manifest["source"]
    require(source["registry_report"] == {"bytes": REPORT_PIN[0], "path": "reports/registries.json",
                                         "sha256": REPORT_PIN[1]}, "report provenance differs")
    require(source["server_bundle"]["sha256"] == SERVER_SHA256
            and source["inner_server"]["sha256"] == INNER_SHA256, "server provenance differs")
    require(manifest["fixtures"]["entity-types.json"] == {"bytes": FIXTURE_PIN[0], "sha256": FIXTURE_PIN[1]},
            "subset provenance differs")
    return fixture, source


def render(fixture: dict, source: dict) -> dict[str, bytes]:
    rows = entries(fixture)
    oracle = ("protocol_id\tresource_location\tis_player\n" + "".join(
        f"{wire_id}\t{name}\t{str(name == 'minecraft:player').lower()}\n" for wire_id, name in rows
    )).encode()
    manifest = {
        "schema_version": 1,
        "minecraft": MINECRAFT,
        "registry": REGISTRY,
        "entry_count": len(rows),
        "scope": "complete static entity type identity only; no metadata, spawn rules or simulation",
        "exporter": "server/crates/ferrumc-registry/tests/reference/export_entity_types.py",
        "source": {
            "original_exporter": "scripts/minecraft-original-export.py",
            "original_manifest": {"bytes": ORIGINAL_MANIFEST_PIN[0], "sha256": ORIGINAL_MANIFEST_PIN[1]},
            "entity_subset": {"path": "docs/fixtures/minecraft-1.21.8-original/entity-types.json",
                              "bytes": FIXTURE_PIN[0], "sha256": FIXTURE_PIN[1]},
            "registry_report": source["registry_report"],
            "server_bundle": source["server_bundle"],
            "inner_server": source["inner_server"],
            "version_manifest": source["version_manifest"],
        },
        "verification": "all 151 names and protocol ids equal the entity_type subset of the pinned full original report",
        "files": {"entity_types_1_21_8.tsv": {"bytes": len(oracle), "sha256": digest(oracle)}},
        "licensing": {"tooling": "MIT (FerrumC)",
                      "original_data": "Mojang/Microsoft data; not relicensed under repository MIT",
                      "eula": "https://www.minecraft.net/en-us/eula",
                      "usage_guidelines": "https://www.minecraft.net/en-us/usage-guidelines"},
    }
    return {"entity_types_1_21_8.tsv": oracle,
            "entity_types_1_21_8.manifest.json": (json.dumps(manifest, indent=2, sort_keys=True) + "\n").encode()}


def outputs(report: Path | None) -> dict[str, bytes]:
    fixture, source = original_fixture()
    if report is not None:
        full = parse(pinned_read(report, REPORT_PIN))
        require(isinstance(full, dict) and REGISTRY in full, "entity registry report absent")
        require(fixture == {"registry": REGISTRY, **full[REGISTRY]}, "full-report/subset mismatch")
    return render(fixture, source)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", type=Path, help="also compare the SHA-pinned full original registry report")
    parser.add_argument("--export", action="store_true", help="write the oracle and manifest; requires --report")
    parser.add_argument("--check", action="store_true", help="compare the committed oracle and manifest offline")
    parser.add_argument("--self-test", action="store_true", help="run malformed/provenance tests, also valid with python -O")
    args = parser.parse_args()
    if args.self_test:
        result = unittest.TextTestRunner().run(unittest.defaultTestLoader.loadTestsFromTestCase(SelfTests))
        if not result.wasSuccessful():
            return 1
    try:
        require(not args.export or args.report is not None, "--export requires pinned --report")
        expected = outputs(args.report)
        if args.export:
            OUTPUT.mkdir(parents=True, exist_ok=True)
            require(not OUTPUT.is_symlink(), "unsafe output directory")
            for name, data in expected.items():
                path = OUTPUT / name
                require(not path.is_symlink(), "unsafe output file")
                path.write_bytes(data)
        if args.check or not args.export:
            for name, data in expected.items():
                pinned_read(OUTPUT / name, (len(data), digest(data)))
        print("entity-type oracle: all 151 types verified" + (" against full original report" if args.report else " (offline)"))
    except (ExportError, OSError, KeyError, TypeError, ValueError) as error:
        print(f"entity-type export error: {error}", file=sys.stderr)
        return 1
    return 0


class SelfTests(unittest.TestCase):
    def test_require_cannot_be_disabled(self):
        with self.assertRaises(ExportError):
            require(False, "refusal")

    def test_malformed_json(self):
        for data in (b"", b"{", b'{"a":1,"a":2}', b"NaN", b"Infinity", b'"\xff"', b"{} {}"):
            with self.subTest(data=data), self.assertRaises(ExportError):
                parse(data)

    def test_byte_bound(self):
        with self.assertRaises(ExportError):
            parse(b" " * (MAX_BYTES + 1))

    def test_source_pins_reject_tampering_missing_and_symlink(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "report.json"
            for data in (b"abc", b"ab", b"abcd"):
                path.write_bytes(data)
                with self.assertRaises(ExportError):
                    pinned_read(path, (3, digest(b"xyz")))
            path.unlink()
            with self.assertRaises(ExportError):
                pinned_read(path, REPORT_PIN)
            path.symlink_to(ORIGINAL / "entity-types.json")
            with self.assertRaises(ExportError):
                pinned_read(path, FIXTURE_PIN)

    def test_invalid_schema_and_ids(self):
        fixture, _ = original_fixture()
        for change in ("registry", "default", "negative", "duplicate", "bool", "gap", "player", "resource", "unknown"):
            value = json.loads(json.dumps(fixture))
            if change in ("registry", "default"):
                value[change] = "minecraft:missing"
            elif change == "player":
                value["entries"]["minecraft:unknown"] = value["entries"].pop("minecraft:player")
            elif change == "resource":
                value["entries"]["minecraft:a/../b"] = value["entries"].pop("minecraft:cow")
            elif change == "unknown":
                value["extra"] = 1
            else:
                value["entries"]["minecraft:cow"]["protocol_id"] = {
                    "negative": -1, "duplicate": 0, "bool": True, "gap": 151
                }[change]
            with self.subTest(change=change), self.assertRaises(ExportError):
                entries(value)

    def test_full_report_pin_and_complete_subset(self):
        fixture, source = original_fixture()
        self.assertEqual(len(entries(fixture)), 151)
        self.assertEqual(fixture["entries"]["minecraft:player"]["protocol_id"], 149)
        rendered = render(fixture, source)
        self.assertEqual(len(rendered["entity_types_1_21_8.tsv"].splitlines()), 152)


if __name__ == "__main__":
    raise SystemExit(main())
