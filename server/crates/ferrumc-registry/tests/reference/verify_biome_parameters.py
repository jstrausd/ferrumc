#!/usr/bin/env python3
"""Offline exhaustive verification of the adopted original biome census.

This verifies immutable evidence and resource data. It does not rerun Java or
admit a dynamic registry, network payload, selector, terrain or gameplay.
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[5]
CRATE = ROOT / "server/crates/ferrumc-registry"
PINS = {
    "data/biome-parameters-1.21.8.json": (
        1072932, "4e9ee584071de6822b9e0e8030eb0666814a3c61b77f8e0ce611f9757508f971"),
    "data/biome-parameters-1.21.8.provenance.json": (
        1389, "f24ff1017170e1623904e3570dfa4371328edd895b5f94b09ea717a22fe1464f"),
    "tests/reference/biome_parameters/BiomeParameterCensus.java": (
        4175, "7c128dbb203565c9cfd1728bb753870c3037219cae283f759cf1f18df15122c9"),
    "tests/reference/biome_parameters/run_census.py": (
        4358, "821327f9770821a0431a52385434cfcecf996457fab08a1727562affb54381f9"),
    "tests/reference/biome_parameters/normal.manifest.json": (
        31027, "5a37d47e69f8f0cf1866a22dcdd6827c187b8f0fb932703324fe50e6a944854f"),
    "tests/reference/biome_parameters/optimized.manifest.json": (
        31027, "5a37d47e69f8f0cf1866a22dcdd6827c187b8f0fb932703324fe50e6a944854f"),
    "tests/reference/biome_parameters/root-census.freeze.json": (
        2001, "95deae4a95856a4a80cb3ec71c1c251e1852b6ecc0a1506d6aa811433e28729b"),
}


def require(condition: bool, message: str) -> None:
    # Explicit guards remain active under Python -O.
    if not condition:
        raise RuntimeError(message)


def read_pinned(path: Path, count: int, digest: str) -> bytes:
    require(path.stat().st_size == count, f"byte count differs: {path.name}")
    with path.open("rb") as stream:
        raw = stream.read(count + 1)
    require(len(raw) == count and hashlib.sha256(raw).hexdigest() == digest,
            f"SHA256 differs: {path.name}")
    return raw


def unique_object(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result = {}
    for key, value in pairs:
        require(key not in result, f"duplicate decoded JSON key: {key}")
        result[key] = value
    return result


def parse(raw: bytes) -> object:
    def refuse_constant(token: str) -> None:
        raise RuntimeError(f"nonfinite JSON token: {token}")
    return json.loads(raw, object_pairs_hook=unique_object,
                      parse_constant=refuse_constant)


def verify_rows(parameters: object, keys: set[str]) -> int:
    require(type(parameters) is dict
            and set(parameters) == {"minecraft:nether", "minecraft:overworld"},
            "preset keys differ")
    total = 0
    for preset, count, unique in (("minecraft:nether", 5, 5),
                                  ("minecraft:overworld", 7593, 54)):
        rows = parameters[preset]
        require(type(rows) is list and len(rows) == count, f"count differs: {preset}")
        biome_keys = set()
        for index, row in enumerate(rows):
            require(type(row) is dict and set(row) == {"index", "biome", "axes", "offset"},
                    "entry fields differ")
            require(type(row["index"]) is int and row["index"] == index,
                    "entry order differs")
            require(type(row["biome"]) is str and row["biome"] in keys,
                    "unknown/noncanonical biome reference")
            require(type(row["offset"]) is int and 0 <= row["offset"] <= 10000,
                    "offset outside quantized codec domain")
            require(type(row["axes"]) is list and len(row["axes"]) == 6,
                    "axes count differs")
            for span in row["axes"]:
                require(type(span) is list and len(span) == 2
                        and all(type(value) is int for value in span)
                        and -20000 <= span[0] <= span[1] <= 20000,
                        "span outside quantized codec domain")
            biome_keys.add(row["biome"])
            total += 1
        require(len(biome_keys) == unique, f"biome key census differs: {preset}")
    return total


def verify(crate: Path = CRATE, original: Path | None = None) -> int:
    originals = original or ROOT / "docs/fixtures/minecraft-1.21.8-original"
    raw_files = {name: read_pinned(crate / name, *pin) for name, pin in PINS.items()}
    worldgen = read_pinned(originals / "worldgen.json", 2530249,
                          "bc0dbfa46f495865fc98af835c70dee13ddfdcebb263cf40b3a075324f261b30")
    keys = set(parse(worldgen)["registries"]["minecraft:worldgen/biome"]["entries"])
    require(len(keys) == 65, "biome definition count differs")
    parameters = parse(raw_files["data/biome-parameters-1.21.8.json"])
    count = verify_rows(parameters, keys)
    provenance = parse(raw_files["data/biome-parameters-1.21.8.provenance.json"])
    require(provenance["minecraft"] == {"version": "1.21.8", "protocol_version": 772,
                                      "data_version": 4440}, "release pins differ")
    require(provenance["parameters"]["sha256"]
            == PINS["data/biome-parameters-1.21.8.json"][1], "data receipt differs")
    manifests = [raw_files[f"tests/reference/biome_parameters/{mode}.manifest.json"]
                 for mode in ("normal", "optimized")]
    require(manifests[0] == manifests[1], "normal/optimized census manifests differ")
    manifest = parse(manifests[0])
    require(manifest["fresh_helper_origin_checked"] is True
            and manifest["original_class_origins_checked"] == 19,
            "original method origin receipts differ")
    require(manifest["parameters_sha256"]
            == PINS["data/biome-parameters-1.21.8.json"][1], "census digest differs")
    require(manifest["presets"] == {
        "minecraft:nether": {"parameters": 5, "unique_biomes": 5},
        "minecraft:overworld": {"parameters": 7593, "unique_biomes": 54}},
        "census counts differ")
    freeze = parse(raw_files["tests/reference/biome_parameters/root-census.freeze.json"])
    require(freeze["file_count"] == len(freeze["files"]) == 10,
            "independent root freeze differs")
    for mode in ("normal", "optimized"):
        frozen = freeze["files"][f".codex-tmp/gen05_5-root-biome-census-{mode}/parameters.json"]
        require((frozen["bytes"], frozen["sha256"])
                == PINS["data/biome-parameters-1.21.8.json"],
                "normal/optimized frozen data pins differ")
    return count


if __name__ == "__main__":
    print(f"Verified all {verify()} immutable original ordered parameter entries,")
    print("65 original biome keys, unchanged helper and normal/optimized origin receipts")
    print("Static resource data only; no dynamic/wire or gameplay acceptance")
