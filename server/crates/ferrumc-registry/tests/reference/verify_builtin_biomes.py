#!/usr/bin/env python3
"""Offline exhaustive original-source check for REG-BIOME-01.

Rust tests separately prove every generated runtime definition equals its export
slice and every ordered quantized entry equals the independently frozen census.
This check proves the export's contents and exact decimal values agree with all 67
verified original biome/preset resources. Export/JAR whitespace and number-token
spelling are not claimed byte-identical; generated datagen/JAR raw bytes are.
"""

from __future__ import annotations

import argparse
import importlib.util
from pathlib import Path
import zipfile


ROOT = Path(__file__).resolve().parents[5]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--original-dir", type=Path,
                        default=ROOT / ".codex-tmp/original-1.21.8")
    args = parser.parse_args()
    specification = importlib.util.spec_from_file_location(
        "original_export", ROOT / "scripts/minecraft-original-export.py")
    if specification is None or specification.loader is None:
        raise RuntimeError("original exporter module unavailable")
    original = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(original)
    require = original.require
    source = args.original_dir
    bundle = original.source_bytes(source / "server.jar", original.MAX_DOWNLOAD)
    require(len(bundle) == original.SERVER_BYTES
            and original.sha256(bundle) == original.SERVER_SHA256,
            "official bundle bytes/SHA256 differ")
    with original.checked_archive(bundle) as archive:
        inner = original.read_entry(archive, original.INNER_PATH, original.MAX_DOWNLOAD)
    original.validate_inner(inner)
    extracted = original.source_bytes(
        source / "versions/1.21.8/server-1.21.8.jar", original.MAX_DOWNLOAD)
    require(extracted == inner, "extracted inner server differs from official bundle")
    fixture_dir = ROOT / "docs/fixtures/minecraft-1.21.8-original"
    for filename in ("worldgen.json", "manifest.json", "version-manifest.json"):
        raw = original.source_bytes(fixture_dir / filename)
        count, digest = original.OUTPUT_PINS[filename]
        require(len(raw) == count and original.sha256(raw) == digest,
                f"immutable fixture pin differs: {filename}")
    fixture = original.parse_json(original.source_bytes(fixture_dir / "worldgen.json"))
    manifest = original.parse_json(original.source_bytes(fixture_dir / "manifest.json"))
    total = 0
    with zipfile.ZipFile(source / "versions/1.21.8/server-1.21.8.jar") as archive:
        for family, count in (("biome", 65), ("multi_noise_biome_source_parameter_list", 2)):
            prefix = f"data/minecraft/worldgen/{family}/"
            paths = sorted(name for name in archive.namelist()
                           if name.startswith(prefix) and name.endswith(".json"))
            require(len(paths) == count, f"original resource count differs: {family}")
            entries = fixture["registries"][f"minecraft:worldgen/{family}"]["entries"]
            require(set(entries) == {"minecraft:" + path[len(prefix):-5] for path in paths},
                    f"resource sets differ: {family}")
            receipts = []
            for path in paths:
                raw = original.read_entry(archive, path)
                require(raw == original.source_bytes(source / "generated" / path),
                        f"datagen/JAR raw bytes differ: {path}")
                value = original.parse_json(raw)
                exported = entries["minecraft:" + path[len(prefix):-5]]
                require(value == exported, f"original content/exact decimal value differs: {path}")
                receipts.append({"path": path, "bytes": len(raw), "sha256": original.sha256(raw)})
            receipt = manifest["source"]["datapack_categories"][f"worldgen/{family}"]
            require(receipt["count"] == count
                    and receipt["bytes"] == sum(row["bytes"] for row in receipts)
                    and receipt["path_bytes_sha256_index"] == original.sha256(original.rendered(receipts)),
                    f"resource receipt differs: {family}")
            print(f"{family}: {count}/{count}, datagen/JAR byte equality, exact Decimal content, receipt verified")
            total += count
    print(f"All {total} original biome/preset resources verified")
    print(f"Official bundle SHA256 {original.sha256(bundle)}")
    print(f"Inner server SHA256 {original.sha256(inner)}")
    print("Resource/data parity only; selection, dynamic/wire registry order, terrain and gameplay remain separate")


if __name__ == "__main__":
    main()
