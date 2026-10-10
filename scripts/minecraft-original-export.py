#!/usr/bin/env python3
"""Export pinned Mojang 1.21.8 data, or check committed fixtures offline.

The Python tooling is MIT. Original Minecraft data retains Mojang/Microsoft
rights; provenance in manifest.json does not relicense it or imply gameplay.
"""

from __future__ import annotations

import argparse
from contextlib import contextmanager
from decimal import Decimal
import hashlib
import io
import json
import math
import os
from pathlib import Path, PurePosixPath
import re
import stat
import struct
import subprocess
import sys
import tempfile
import time
import unittest
import urllib.request
import zipfile


ROOT = Path(__file__).resolve().parent.parent
OUTPUT = Path("docs/fixtures/minecraft-1.21.8-original")
VERSION_INDEX = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json"
VERSION_SHA1 = "b1bb44ad769609925328838694503a8dcac7f529"
VERSION_SHA256 = "3bf5cdd444dfa89199fe4f8a92b6c79f4e4fa8977ff52029c3acd99378506fad"
VERSION_BYTES = 43719
VERSION_URL = f"https://piston-meta.mojang.com/v1/packages/{VERSION_SHA1}/1.21.8.json"
SERVER_SHA1 = "6bce4ef400e4efaa63a13d5e6f6b500be969ef81"
SERVER_SHA256 = "2349d9a8f0d4be2c40e7692890ef46a4b07015e7955b075460d02793be7fbbe7"
SERVER_BYTES = 57555044
SERVER_URL = f"https://piston-data.mojang.com/v1/objects/{SERVER_SHA1}/server.jar"
INNER_PATH = "META-INF/versions/1.21.8/server-1.21.8.jar"
INNER_SHA256 = "2bee159e4ce87ae857b4323f9f32c7a3d7d448f7f8c3f62fa6f295a72e228250"
INNER_BYTES = 18900451
MINECRAFT = {"version": "1.21.8", "protocol_version": 772, "data_version": 4440}
MAX_DOWNLOAD = 64 * 1024 * 1024
MAX_FILE = 16 * 1024 * 1024
MAX_FIXTURES = 32 * 1024 * 1024
MAX_GENERATED = 64 * 1024 * 1024
MAX_ARCHIVE_BYTES = 384 * 1024 * 1024
MAX_ENTRIES = 32768
MAX_JSON_ENTRIES = 65536
MAX_JSON_NODES = 1_000_000
MAX_DEPTH = 96
MAX_STRING = 1024 * 1024
MAX_RUN_SECONDS = 180
MAX_DOWNLOAD_SECONDS = 120
RESOURCE = re.compile(r"minecraft:[a-z0-9_./-]+\Z")
ARCHIVE_NAME = re.compile(r"[A-Za-z0-9_./+\-$]+\Z")
WORLDGEN_COUNTS = {
    "biome": 65, "configured_carver": 4, "configured_feature": 224,
    "density_function": 35, "flat_level_generator_preset": 9,
    "multi_noise_biome_source_parameter_list": 2, "noise": 60,
    "noise_settings": 7, "placed_feature": 258, "processor_list": 40,
    "structure": 34, "structure_set": 20, "template_pool": 188,
    "world_preset": 6,
}
FILES = (
    "entity-types.json", "recipe-serializers.json", "recipes.json",
    "loot-tables.json", "worldgen.json", "dimensions.json",
    "version-manifest.json", "manifest.json",
)
# These pins were measured from the verified official export. They do not
# come from the writable output manifest, so changing both data and manifest
# cannot make an offline integrity check succeed.
OUTPUT_PINS = {
    "dimensions.json": (8752, "37fde55f5e010babea5dcdab9a0c773c211f8d4d43a4f5d826e53035dad188d1"),
    "entity-types.json": (9319, "b65a61ac9f4346ad74ff159cb3f8be78ec96229ccfc3abe29a14a7e0063d4d6b"),
    "loot-tables.json": (1366578, "ea338e10f1ad9618171085cb77be4ddc686e47b8515c24b757d5d47ea8f538ec"),
    "manifest.json": (7061, "d5b4506f5749d114d7b54c50231ee00f6f15d14b2826f8c39489df06932ed9ee"),
    "recipe-serializers.json": (1681, "f8d4bd892ddd3cf238ce92a164d2272f87637596f3455ff8c360eae3fc392bd9"),
    "recipes.json": (501688, "023cc012b9e01b138831c302bdfb956e4179203b447a9e1826fc0939f285a232"),
    "version-manifest.json": (VERSION_BYTES, VERSION_SHA256),
    "worldgen.json": (2530249, "bc0dbfa46f495865fc98af835c70dee13ddfdcebb263cf40b3a075324f261b30"),
}


class ExportError(ValueError):
    """Pinned content, provenance, bounds or a subprocess contract failed."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ExportError(message)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def source_bytes(path: Path, limit: int = MAX_FILE) -> bytes:
    require(not path.is_symlink() and path.is_file(), f"not a regular input: {path}")
    require(path.stat().st_size <= limit, f"file exceeds {limit} bytes: {path}")
    with path.open("rb") as source:
        data = source.read(limit + 1)
    require(len(data) <= limit, f"file grew past limit: {path}")
    return data


def unique_object(pairs: list[tuple[str, object]]) -> dict:
    result = {}
    require(len(pairs) <= MAX_JSON_ENTRIES, "JSON object exceeds entry limit")
    for key, value in pairs:
        require(key not in result, f"duplicate JSON key: {key}")
        result[key] = value
    return result


def number(token: str, integer: bool) -> object:
    require(len(token) <= 128, "JSON number token exceeds limit")
    value = int(token) if integer else Decimal(token)
    if not integer:
        require(value.is_finite() and abs(value.adjusted()) <= 308,
                "non-finite or oversized JSON exponent")
    return value


def validate_tree(value: object, depth: int = 0, budget: list | None = None) -> None:
    if budget is None:
        budget = [MAX_JSON_NODES]
    budget[0] -= 1
    require(budget[0] >= 0 and depth <= MAX_DEPTH, "JSON node/depth limit exceeded")
    if isinstance(value, dict):
        require(len(value) <= MAX_JSON_ENTRIES, "JSON object exceeds entry limit")
        for key, child in value.items():
            require(isinstance(key, str), "JSON key must be text")
            validate_tree(key, depth + 1, budget)
            validate_tree(child, depth + 1, budget)
    elif isinstance(value, list):
        require(len(value) <= MAX_JSON_ENTRIES, "JSON array exceeds entry limit")
        for child in value:
            validate_tree(child, depth + 1, budget)
    elif isinstance(value, str):
        require(len(value) <= MAX_STRING, "JSON string exceeds limit")
        try:
            text_bytes = value.encode("utf-8")
        except UnicodeEncodeError as error:
            raise ExportError("unpaired JSON surrogate") from error
        require(len(text_bytes) <= MAX_STRING, "JSON string UTF-8 byte limit exceeded")
    else:
        require(value is None or type(value) in (bool, int, Decimal), "unsupported JSON value")


def parse_json(data: bytes) -> object:
    require(len(data) <= MAX_FILE, "JSON byte limit exceeded")
    try:
        value = json.loads(data.decode("utf-8"), object_pairs_hook=unique_object,
                           parse_int=lambda token: number(token, True),
                           parse_float=lambda token: number(token, False),
                           parse_constant=lambda token: require(False, f"invalid JSON number: {token}"))
        validate_tree(value)
        return value
    except (UnicodeError, json.JSONDecodeError, RecursionError) as error:
        raise ExportError(f"malformed JSON: {error}") from error


def encoded(value: object, level: int = 0) -> str:
    """Canonical keys/whitespace; Decimal preserves exact original numbers."""
    if isinstance(value, dict):
        if not value:
            return "{}"
        items = ["  " * (level + 1) + json.dumps(key, ensure_ascii=False) + ": " +
                 encoded(value[key], level + 1) for key in sorted(value)]
        return "{\n" + ",\n".join(items) + "\n" + "  " * level + "}"
    if isinstance(value, list):
        if not value:
            return "[]"
        items = ["  " * (level + 1) + encoded(child, level + 1) for child in value]
        return "[\n" + ",\n".join(items) + "\n" + "  " * level + "]"
    if isinstance(value, Decimal):
        return str(value)
    return json.dumps(value, ensure_ascii=False, allow_nan=False)


def rendered(value: object) -> bytes:
    validate_tree(value)
    data = (encoded(value) + "\n").encode("utf-8")
    require(len(data) <= MAX_FILE, "rendered fixture exceeds byte limit")
    return data


def safe_path(name: str) -> PurePosixPath:
    require(isinstance(name, str) and 0 < len(name) <= 512 and
            ARCHIVE_NAME.fullmatch(name) is not None and not name.startswith("/"),
            "invalid archive/resource path")
    parts = name.rstrip("/").split("/")
    require(all(part not in ("", ".", "..") for part in parts), "unsafe path segment")
    require(len(parts) <= 32, "path depth exceeds limit")
    return PurePosixPath(name)


def resource_id(name: str) -> str:
    require(isinstance(name, str) and RESOURCE.fullmatch(name) is not None,
            "invalid Minecraft resource id")
    safe_path(name.split(":", 1)[1])
    return name


@contextmanager
def checked_archive(data: bytes):
    require(len(data) <= MAX_DOWNLOAD, "archive byte limit exceeded")
    # Bound central-directory allocation before ZipFile parses it. ZIP64 and
    # multidisk archives are unnecessary for the pinned official artifacts.
    offset = data.rfind(b"PK\x05\x06", max(0, len(data) - 65557))
    require(offset >= 0 and len(data) - offset >= 22, "missing ZIP end record")
    # ZipFile follows a ZIP64 locator even when the legacy EOCD fields do not
    # contain sentinel values. Reject it before ZipFile can replace our bounds.
    require(offset < 20 or data[offset - 20:offset - 16] != b"PK\x06\x07",
            "ZIP64 locator rejected before directory parsing")
    fields = struct.unpack_from("<4s4H2IH", data, offset)
    _, disk, start_disk, disk_count, count, directory_size, directory_offset, comment = fields
    require(disk == start_disk == 0 and disk_count == count and count <= MAX_ENTRIES,
            "ZIP64/multidisk/entry limit rejected")
    require(directory_size <= 8 * 1024 * 1024 and directory_offset + directory_size <= offset
            and offset + 22 + comment == len(data), "invalid/oversized ZIP directory")
    try:
        with zipfile.ZipFile(io.BytesIO(data)) as archive:
            entries = archive.infolist()
            require(len(entries) == count, "ZIP entry count mismatch")
            names = set()
            total = 0
            for info in entries:
                safe_path(info.filename)
                require(info.filename not in names, "duplicate archive path")
                names.add(info.filename)
                require(not stat.S_ISLNK(info.external_attr >> 16), "archive symlink rejected")
                require(not info.flag_bits & 1 and info.compress_type in
                        (zipfile.ZIP_STORED, zipfile.ZIP_DEFLATED), "unsupported ZIP encryption/compression")
                require(info.file_size <= MAX_DOWNLOAD and
                        info.file_size <= max(1, info.compress_size) * 512,
                        "archive entry size/compression ratio exceeds limit")
                total += info.file_size
                require(total <= MAX_ARCHIVE_BYTES, "archive expanded-size limit exceeded")
            yield archive
    except (zipfile.BadZipFile, RuntimeError) as error:
        raise ExportError(f"malformed ZIP: {error}") from error


def read_entry(archive: zipfile.ZipFile, name: str, limit: int = MAX_FILE) -> bytes:
    safe_path(name)
    try:
        info = archive.getinfo(name)
        require(info.file_size <= limit and not info.is_dir(), "archive member byte/type limit")
        with archive.open(info) as source:
            data = source.read(limit + 1)
        require(len(data) == info.file_size and len(data) <= limit, "archive member size mismatch")
        return data
    except KeyError as error:
        raise ExportError(f"missing archive member: {name}") from error


def validate_version(data: bytes) -> dict:
    require(len(data) == VERSION_BYTES and hashlib.sha1(data).hexdigest() == VERSION_SHA1
            and sha256(data) == VERSION_SHA256, "version manifest integrity mismatch")
    value = parse_json(data)
    require(isinstance(value, dict) and value.get("id") == "1.21.8" and
            value.get("type") == "release" and value.get("javaVersion", {}).get("majorVersion") == 21,
            "version/Java pin differs")
    require(value.get("downloads", {}).get("server") ==
            {"sha1": SERVER_SHA1, "size": SERVER_BYTES, "url": SERVER_URL},
            "version manifest server provenance differs")
    return value


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, fp, code, msg, headers, newurl):
        raise ExportError("download redirects are not accepted for pinned URLs")


def copy_download(source, target, expected_size: int, expected_sha256: str,
                  deadline: float) -> None:
    require(0 < expected_size <= MAX_DOWNLOAD, "download size bound")
    count = 0
    hasher = hashlib.sha256()
    while True:
        require(time.monotonic() < deadline, "download absolute deadline exceeded")
        # HTTPResponse.read1 performs at most one buffered/socket read. read(n)
        # can wait for n bytes while a slow sender repeatedly resets IO timeout.
        reader = getattr(source, "read1", source.read)
        data = reader(min(64 * 1024, expected_size + 1 - count))
        if not data:
            break
        count += len(data)
        require(count <= expected_size, "download exceeds pinned byte length")
        hasher.update(data)
        target.write(data)
    require(count == expected_size and hasher.hexdigest() == expected_sha256,
            "download truncated or SHA-256 mismatch")


def fetch(url: str, path: Path, size: int, fingerprint: str, offline: bool) -> bytes:
    require(url in (VERSION_URL, SERVER_URL), "unpinned download URL")
    if path.exists() or path.is_symlink():
        data = source_bytes(path, MAX_DOWNLOAD)
        require(len(data) == size and sha256(data) == fingerprint, "cached source integrity mismatch")
        return data
    require(not offline, "pinned cache missing in --offline mode")
    opener = urllib.request.build_opener(NoRedirect())
    request = urllib.request.Request(url, headers={"User-Agent": "FerrumC-original-data-export/1"})
    deadline = time.monotonic() + MAX_DOWNLOAD_SECONDS
    temporary = None
    try:
        with opener.open(request, timeout=20) as source:
            require(source.geturl() == url and source.status == 200, "download origin/status mismatch")
            length = source.headers.get("Content-Length")
            require(length is None or length == str(size), "download Content-Length differs")
            with tempfile.NamedTemporaryFile(dir=path.parent, delete=False) as output:
                temporary = Path(output.name)
                copy_download(source, output, size, fingerprint, deadline)
        os.replace(temporary, path)
        temporary = None
        return source_bytes(path, MAX_DOWNLOAD)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def registry(value: object, key: str, expected_count: int) -> dict:
    require(isinstance(value, dict) and isinstance(value.get(key), dict), "missing registry report")
    result = value[key]
    require(set(result) <= {"entries", "protocol_id", "default"} and
            type(result.get("protocol_id")) is int and 0 <= result["protocol_id"] < MAX_JSON_ENTRIES,
            "invalid registry metadata")
    entries = result.get("entries")
    require(isinstance(entries, dict) and len(entries) == expected_count, "registry entry count differs")
    ids = set()
    for name, row in entries.items():
        resource_id(name)
        require(isinstance(row, dict) and set(row) == {"protocol_id"} and
                type(row["protocol_id"]) is int and row["protocol_id"] >= 0,
                "invalid registry protocol id")
        require(row["protocol_id"] not in ids, "duplicate registry protocol id")
        ids.add(row["protocol_id"])
    require(ids == set(range(expected_count)), "registry protocol ids have gaps")
    require("default" not in result or result["default"] in entries, "registry default is absent")
    return {"registry": key, **result}


def family(path: str) -> str | None:
    parts = safe_path(path).parts
    if len(parts) < 4 or parts[:2] != ("data", "minecraft") or not path.endswith(".json"):
        return None
    if parts[2] in ("recipe", "loot_table", "dimension_type"):
        return parts[2]
    if parts[2] == "worldgen":
        require(len(parts) >= 5 and parts[3] in WORLDGEN_COUNTS, "unexpected worldgen family")
        return "worldgen/" + parts[3]
    return None


def generated_files(root: Path) -> dict[str, Path]:
    require(root.is_dir() and not root.is_symlink(), "generated root missing/unsafe")
    found = {}
    total = 0
    visited = 0
    for directory, dirs, files in os.walk(root, followlinks=False):
        for name in dirs + files:
            path = Path(directory) / name
            relative = path.relative_to(root).as_posix()
            safe_path(relative)
            require(not path.is_symlink(), "generated symlink rejected")
            visited += 1
            require(visited <= MAX_ENTRIES, "generated entry count exceeds limit")
        for name in files:
            path = Path(directory) / name
            require(path.is_file() and path.stat().st_size <= MAX_FILE, "generated file size/type limit")
            total += path.stat().st_size
            require(total <= MAX_GENERATED, "generated aggregate byte limit")
            relative = path.relative_to(root).as_posix()
            if family(relative) is not None:
                found[relative] = path
    return found


def validate_inner(inner: bytes) -> None:
    require(len(inner) == INNER_BYTES and sha256(inner) == INNER_SHA256,
            "inner-server provenance differs")
    with checked_archive(inner) as archive:
        version = parse_json(read_entry(archive, "version.json"))
        require(isinstance(version, dict) and version.get("id") == MINECRAFT["version"] and
                version.get("protocol_version") == MINECRAFT["protocol_version"] and
                version.get("world_version") == MINECRAFT["data_version"] and
                version.get("java_version") == 21 and version.get("stable") is True,
                "inner game/protocol/DataVersion/Java pin differs")


def assemble(generated: Path, inner: bytes, version: bytes) -> dict[str, bytes]:
    validate_version(version)
    validate_inner(inner)
    paths = generated_files(generated)
    require(len(paths) == 3609, "selected generated resource count differs")
    categories = {}
    receipts = {}
    selected_total = 0
    with checked_archive(inner) as archive:
        original = {name for name in archive.namelist() if family(name) is not None}
        require(set(paths) == original, "generated resource set differs from original server resources")
        for name in sorted(paths):
            data = source_bytes(paths[name])
            selected_total += len(data)
            require(selected_total <= MAX_FILE, "selected original aggregate byte limit")
            # Byte equality is stronger than a comparison of decoded JSON. It
            # independently catches missing output and DataProvider's logged IO errors.
            require(data == read_entry(archive, name), f"generated bytes differ from embedded original: {name}")
            kind = family(name)
            identifier = resource_id("minecraft:" + name[len("data/minecraft/" + kind + "/"):-5])
            value = parse_json(data)
            # Density-function codecs also emit literal numbers or registry
            # references; an object-only parser would silently lose valid data.
            require(kind.startswith("worldgen/") or isinstance(value, dict),
                    "recipe/loot/dimension-type record must be an object")
            bucket = categories.setdefault(kind, {})
            require(identifier not in bucket, "duplicate datapack resource id")
            bucket[identifier] = value
            receipts.setdefault(kind, []).append({"path": name, "bytes": len(data), "sha256": sha256(data)})
    counts = {"recipe": 1407, "loot_table": 1246, "dimension_type": 4,
              **{"worldgen/" + key: count for key, count in WORLDGEN_COUNTS.items()}}
    require({key: len(value) for key, value in categories.items()} == counts,
            "datapack category counts differ")
    registry_raw = source_bytes(generated / "reports/registries.json")
    report = parse_json(registry_raw)
    entities = registry(report, "minecraft:entity_type", 151)
    serializers = registry(report, "minecraft:recipe_serializer", 22)
    recipes = categories["recipe"]
    for recipe in recipes.values():
        require(recipe.get("type") in serializers["entries"], "recipe serializer absent from original registry")
    presets = categories["worldgen/world_preset"]
    stems = {}
    for name, preset in presets.items():
        dimensions = preset.get("dimensions")
        require(isinstance(dimensions, dict) and set(dimensions) ==
                {"minecraft:overworld", "minecraft:the_nether", "minecraft:the_end"},
                "world preset dimension set differs")
        for key, stem in dimensions.items():
            resource_id(key)
            require(isinstance(stem, dict) and stem.get("type") in categories["dimension_type"]
                    and isinstance(stem.get("generator"), dict), "unresolved world preset dimension type")
        stems[name] = dimensions
    values = {
        "entity-types.json": entities,
        "recipe-serializers.json": serializers,
        "recipes.json": {"registry": "minecraft:recipe", "entries": recipes},
        "loot-tables.json": {"registry": "minecraft:loot_table", "entries": categories["loot_table"]},
        "worldgen.json": {"registries": {"minecraft:" + key: {"entries": value}
                                        for key, value in categories.items() if key.startswith("worldgen/")}},
        "dimensions.json": {"dimension_types": categories["dimension_type"],
                            "dimension_stems_by_world_preset": stems,
                            "standalone_dimension_resources": "not emitted by vanilla datagen; stems are nested in world presets"},
    }
    outputs = {name: rendered(value) for name, value in values.items()}
    outputs["version-manifest.json"] = version
    manifest = {
        "schema_version": 1, "minecraft": MINECRAFT,
        "scope": "default vanilla only; data capture, no gameplay or API implementation status",
        "source": {
            "publisher": "Mojang / Microsoft", "version_index_url": VERSION_INDEX,
            "version_manifest": {"url": VERSION_URL, "sha1": VERSION_SHA1,
                                 "sha256": VERSION_SHA256, "bytes": VERSION_BYTES},
            "server_bundle": {"url": SERVER_URL, "sha1": SERVER_SHA1,
                              "sha256": SERVER_SHA256, "bytes": SERVER_BYTES},
            "inner_server": {"path": INNER_PATH, "sha256": INNER_SHA256, "bytes": INNER_BYTES},
            "registry_report": {"path": "reports/registries.json", "sha256": sha256(registry_raw),
                                "bytes": len(registry_raw)},
            "datapack_categories": {key: {"count": len(rows), "bytes": sum(row["bytes"] for row in rows),
                                         "path_bytes_sha256_index": sha256(rendered(rows))}
                                    for key, rows in receipts.items()},
            "selected_datapack_files": len(paths), "selected_datapack_bytes": selected_total,
            "verification": "every selected generated path and raw byte equals its embedded official inner-JAR resource",
        },
        "exporter": {"path": "scripts/minecraft-original-export.py", "java_major": 21,
                     "main_class": "net.minecraft.data.Main", "bundler_property": "bundlerMainClass",
                     "arguments": ["--reports", "--server", "--output", "generated"],
                     "normalization": "sorted keys/resource IDs and whitespace; arrays and exact decimal number values retained; version manifest retained byte-for-byte"},
        "licensing": {"tooling": "MIT (FerrumC)", "original_data": "Mojang/Microsoft Minecraft game data; not relicensed under the repository MIT license",
                      "eula": "https://www.minecraft.net/en-us/eula", "usage_guidelines": "https://www.minecraft.net/en-us/usage-guidelines",
                      "distribution": "reference JSON data only; no server JAR, classes, textures, sounds or NBT structure templates vendored"},
        "not_captured": ["experimental builtin datapacks", "tags and their resolution", "NBT structure templates",
                         "commands/arguments", "advancements/statistics", "other static/dynamic registries",
                         "loot predicate/function behavior and registries", "worldgen algorithms and seed parity",
                         "Bukkit/Paper APIs/events/NMS", "behavior and consumer acceptance matrix"],
        "fixtures": {name: {"bytes": len(data), "sha256": sha256(data)} for name, data in outputs.items()},
    }
    outputs["manifest.json"] = rendered(manifest)
    require(sum(map(len, outputs.values())) <= MAX_FIXTURES, "fixture aggregate byte limit")
    return outputs


def verify_outputs(outputs: dict[str, bytes]) -> None:
    require(set(outputs) == set(FILES) == set(OUTPUT_PINS), "fixture set/pins differ")
    require(sum(map(len, outputs.values())) <= MAX_FIXTURES, "fixture aggregate byte limit")
    for name, data in outputs.items():
        require(len(data) <= MAX_FILE and (len(data), sha256(data)) == OUTPUT_PINS[name],
                f"pinned fixture integrity mismatch: {name}")
    validate_version(outputs["version-manifest.json"])
    manifest = parse_json(outputs["manifest.json"])
    require(manifest.get("minecraft") == MINECRAFT and manifest.get("schema_version") == 1,
            "export manifest version/schema differs")
    for name in FILES:
        if name != "manifest.json":
            require(manifest["fixtures"].get(name) ==
                    {"bytes": len(outputs[name]), "sha256": sha256(outputs[name])},
                    "manifest fixture receipt differs")


def load_exports(root: Path = ROOT) -> dict:
    directory = root / OUTPUT
    require(directory.is_dir() and not directory.is_symlink(), "original fixture directory missing/unsafe")
    require({path.name for path in directory.iterdir()} == set(FILES), "unexpected/missing original fixture file")
    # Enforce the independent exact per-fixture lengths before allocating data;
    # a tampered set cannot first allocate eight MAX_FILE-sized inputs.
    outputs = {name: source_bytes(directory / name, OUTPUT_PINS[name][0]) for name in FILES}
    verify_outputs(outputs)
    return {name: parse_json(data) for name, data in outputs.items()}


def run_export(java: str, cache: Path, offline: bool) -> dict[str, bytes]:
    cache.mkdir(parents=True, exist_ok=True)
    cache = cache.resolve()
    version = fetch(VERSION_URL, cache / "version.json", VERSION_BYTES, VERSION_SHA256, offline)
    validate_version(version)
    bundle_path = cache / "server.jar"
    bundle = fetch(SERVER_URL, bundle_path, SERVER_BYTES, SERVER_SHA256, offline)
    require(hashlib.sha1(bundle).hexdigest() == SERVER_SHA1, "Mojang server SHA-1 differs")
    with checked_archive(bundle) as archive:
        require(read_entry(archive, "META-INF/versions.list") ==
                f"{INNER_SHA256}\t1.21.8\t1.21.8/server-1.21.8.jar".encode(), "bundle version list differs")
        inner = read_entry(archive, INNER_PATH, MAX_DOWNLOAD)
    validate_inner(inner)
    # Version probe is bounded and observes a deadline independently of datagen.
    probe = subprocess.run([java, "-version"], capture_output=True, timeout=10, check=False)
    require(probe.returncode == 0 and re.search(rb'version "21(?:[.+-]|\")', probe.stderr + probe.stdout),
            "export requires Java 21")
    with tempfile.TemporaryDirectory(prefix="datagen-", dir=cache) as temporary:
        run = Path(temporary)
        log = cache / "last-export.log"
        require(not log.is_symlink(), "export log symlink rejected")
        with log.open("wb") as stream:
            result = subprocess.run([java, "-Xmx1G", "-Djava.util.concurrent.ForkJoinPool.common.parallelism=2",
                                     "-DbundlerMainClass=net.minecraft.data.Main", "-DbundlerRepoDir=.",
                                     "-jar", str(bundle_path), "--reports", "--server", "--output", "generated"],
                                    cwd=run, stdout=stream, stderr=subprocess.STDOUT,
                                    timeout=MAX_RUN_SECONDS, check=False)
        require(result.returncode == 0, f"official data generator failed; see {log}")
        require(log.stat().st_size <= MAX_FILE, "datagen log exceeds bound")
        return assemble(run / "generated", inner, version)


def write_outputs(outputs: dict[str, bytes], root: Path = ROOT) -> None:
    verify_outputs(outputs)
    directory = root / OUTPUT
    directory.mkdir(parents=True, exist_ok=True)
    require(not directory.is_symlink(), "output directory symlink rejected")
    require({path.name for path in directory.iterdir()} <= set(FILES), "unknown output file would be overwritten")
    for name, data in outputs.items():
        path = directory / name
        require(not path.is_symlink(), "output fixture symlink rejected")
        temporary = None
        try:
            with tempfile.NamedTemporaryFile(dir=directory, delete=False) as output:
                temporary = Path(output.name)
                output.write(data)
            os.replace(temporary, path)
            temporary = None
        finally:
            if temporary is not None:
                temporary.unlink(missing_ok=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="verify committed data offline; with --export compare a fresh official export")
    parser.add_argument("--export", action="store_true", help="run pinned official datagen and regenerate fixtures")
    parser.add_argument("--offline", action="store_true", help="with --export require already verified cached downloads")
    parser.add_argument("--java", default="java", help="Java 21 executable for --export")
    parser.add_argument("--cache", type=Path, default=ROOT / ".codex-tmp/original-1.21.8", help="ignored local download/run cache")
    parser.add_argument("--self-test", action="store_true", help="exercise malformed data, bounds, provenance and integrity offline")
    args = parser.parse_args()
    if args.self_test:
        result = unittest.TextTestRunner().run(unittest.defaultTestLoader.loadTestsFromTestCase(SelfTests))
        if not result.wasSuccessful():
            return 1
    try:
        if args.export:
            outputs = run_export(args.java, args.cache, args.offline)
            verify_outputs(outputs)
            if args.check:
                load_exports()
                require(all(source_bytes(ROOT / OUTPUT / name) == data for name, data in outputs.items()),
                        "official regenerated fixtures differ")
                print("fresh official export matches pinned committed fixtures")
            else:
                write_outputs(outputs)
                print(f"official fixtures generated: {OUTPUT}")
        else:
            load_exports()
            print(f"original fixtures match pins (offline): {OUTPUT}")
    except (ExportError, OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
        print(f"original export error: {error}", file=sys.stderr)
        return 1
    return 0


class SelfTests(unittest.TestCase):
    def test_json_malformed(self):
        for data in (b"", b"{", b'{"a":1,"a":2}', b"NaN", b"Infinity", b"-Infinity",
                     b"1e309", b"1e-309", b'"\\ud800"', b'"\xff"', b"[1,]",
                     b"1" * 129, b"null null"):
            with self.subTest(data=data), self.assertRaises(ExportError):
                parse_json(data)

    def test_json_depth_and_size(self):
        parse_json(b"[" * MAX_DEPTH + b"0" + b"]" * MAX_DEPTH)
        with self.assertRaises(ExportError):
            parse_json(b"[" * (MAX_DEPTH + 1) + b"0" + b"]" * (MAX_DEPTH + 1))
        with self.assertRaises(ExportError):
            parse_json(b" " * (MAX_FILE + 1))
        with self.assertRaises(ExportError):
            validate_tree([0] * (MAX_JSON_ENTRIES + 1))
        with self.assertRaises(ExportError):
            validate_tree("x" * (MAX_STRING + 1))
        with self.assertRaises(ExportError):
            validate_tree([0, 1], budget=[2])

    def test_exact_numbers_array_order_and_canonical_keys(self):
        first = parse_json(b'{"b":[2,1,-0.0],"a":0.123456789012345678901}')
        second = parse_json(b'{"a":0.123456789012345678901,"b":[2,1,-0.0]}')
        self.assertEqual(rendered(first), rendered(second))
        self.assertIn(b"0.123456789012345678901", rendered(first))
        self.assertIn(b"-0.0", rendered(first))
        self.assertEqual(parse_json(rendered(first))["b"], first["b"])

    def test_utf8_string_exact_byte_boundary(self):
        boundary = "\U0001f600" * (MAX_STRING // 4)
        self.assertEqual(parse_json(json.dumps(boundary, ensure_ascii=False).encode()), boundary)
        with self.assertRaisesRegex(ExportError, "UTF-8 byte"):
            parse_json(json.dumps(boundary + "\U0001f600", ensure_ascii=False).encode())
        with self.assertRaisesRegex(ExportError, "UTF-8 byte"):
            validate_tree({boundary + "\U0001f600": None})

    def test_paths_and_resource_ids(self):
        for name in ("../a", "a/../../b", "/a", "a//b", "a\\b", "C:/a", "./a", "a/./b", "a/..", "", "\x00"):
            with self.subTest(name=name), self.assertRaises(ExportError):
                safe_path(name)
        for name in ("minecraft:a/../b", "minecraft:a//b", "other:stone", "minecraft:Stone"):
            with self.assertRaises(ExportError):
                resource_id(name)
        self.assertEqual(resource_id("minecraft:blocks/stone"), "minecraft:blocks/stone")

    @staticmethod
    def zip_data(rows: list[tuple[object, bytes]]) -> bytes:
        target = io.BytesIO()
        with zipfile.ZipFile(target, "w") as archive:
            for name, data in rows:
                archive.writestr(name, data)
        return target.getvalue()

    def test_archive_paths_duplicate_symlinks_and_truncation(self):
        import warnings
        symlink = zipfile.ZipInfo("link")
        symlink.create_system = 3
        symlink.external_attr = (stat.S_IFLNK | 0o777) << 16
        with warnings.catch_warnings():
            warnings.simplefilter("ignore", UserWarning)
            invalid = [self.zip_data([("../escape", b"x")]), self.zip_data([("a", b"x"), ("a", b"y")]),
                       self.zip_data([(symlink, b"target")]), b"PK", self.zip_data([("a", b"x")])[:-1]]
        for data in invalid:
            with self.assertRaises(ExportError), checked_archive(data):
                pass
        with checked_archive(self.zip_data([("data/minecraft/recipe/a.json", b"{}")])) as archive:
            self.assertEqual(read_entry(archive, "data/minecraft/recipe/a.json"), b"{}")
            with self.assertRaises(ExportError):
                read_entry(archive, "missing")

    def test_archive_preallocation_bound(self):
        data = bytearray(self.zip_data([]))
        struct.pack_into("<HH", data, 8, MAX_ENTRIES + 1, MAX_ENTRIES + 1)
        with self.assertRaises(ExportError), checked_archive(bytes(data)):
            pass

    def test_zip64_locator_cannot_override_bounded_legacy_eocd(self):
        from unittest.mock import patch
        data = self.zip_data([("a", b"x")])
        offset = data.rfind(b"PK\x05\x06")
        _, _, _, _, count, directory_size, directory_offset, _ = struct.unpack_from("<4s4H2IH", data, offset)
        record = struct.pack("<4sQ2H2L4Q", b"PK\x06\x06", 44, 45, 45, 0, 0,
                             count, count, directory_size, directory_offset)
        locator = struct.pack("<4sLQL", b"PK\x06\x07", 0, offset, 1)
        # This archive used to pass our legacy precheck; Python then follows
        # ZIP64 and replaces the already checked directory sizes/counts.
        invalid = data[:offset] + record + locator + data[offset:]
        with patch.object(zipfile, "ZipFile", side_effect=AssertionError("directory parsed too early")):
            with self.assertRaisesRegex(ExportError, "ZIP64 locator"), checked_archive(invalid):
                pass

    def test_archive_expansion_compression_and_member_limits(self):
        for uncompressed, compressed in ((MAX_DOWNLOAD + 1, 1), (513, 1)):
            data = bytearray(self.zip_data([("a", b"x")]))
            central = data.index(b"PK\x01\x02")
            struct.pack_into("<II", data, central + 20, compressed, uncompressed)
            with self.assertRaises(ExportError), checked_archive(bytes(data)):
                pass
        data = bytearray(self.zip_data([(str(i), b"x") for i in range(7)]))
        position = 0
        for _ in range(7):
            position = data.index(b"PK\x01\x02", position)
            struct.pack_into("<II", data, position + 20, MAX_DOWNLOAD, MAX_DOWNLOAD)
            position += 4
        with self.assertRaises(ExportError), checked_archive(bytes(data)):
            pass
        with checked_archive(self.zip_data([("a", b"abcd")])) as archive:
            with self.assertRaises(ExportError):
                read_entry(archive, "a", 3)
        data = bytearray(self.zip_data([]))
        struct.pack_into("<I", data, 12, 8 * 1024 * 1024 + 1)
        with self.assertRaises(ExportError), checked_archive(bytes(data)):
            pass

    def test_download_short_long_hash_and_deadline(self):
        data = b"abc"
        copy_download(io.BytesIO(data), io.BytesIO(), 3, sha256(data), math.inf)
        for source, size, fingerprint, deadline in ((b"ab", 3, sha256(data), math.inf),
                                                   (b"abcd", 3, sha256(data), math.inf),
                                                   (data, 3, "0" * 64, math.inf),
                                                   (data, 3, sha256(data), 0)):
            with self.assertRaises(ExportError):
                copy_download(io.BytesIO(source), io.BytesIO(), size, fingerprint, deadline)
        with self.assertRaises(ExportError):
            NoRedirect().redirect_request(None, None, 302, "", {}, "https://example.com")

    def test_cache_integrity_offline_and_provenance(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "cache"
            with self.assertRaisesRegex(ExportError, "cache missing"):
                fetch(SERVER_URL, path, 3, sha256(b"abc"), True)
            path.write_bytes(b"abc")
            self.assertEqual(fetch(SERVER_URL, path, 3, sha256(b"abc"), True), b"abc")
            for url, size, fingerprint in (("https://example.com", 3, sha256(b"abc")),
                                           (SERVER_URL, 4, sha256(b"abc")),
                                           (SERVER_URL, 3, "0" * 64)):
                with self.assertRaises(ExportError):
                    fetch(url, path, size, fingerprint, True)

    def test_registry_bad_ids_and_default(self):
        good = {"minecraft:entity_type": {"protocol_id": 5, "default": "minecraft:pig",
                                          "entries": {"minecraft:pig": {"protocol_id": 0}}}}
        self.assertEqual(registry(good, "minecraft:entity_type", 1)["registry"], "minecraft:entity_type")
        for row in ({"protocol_id": True}, {"protocol_id": -1}, {"protocol_id": 1}, {"protocol_id": 0, "x": 1}):
            invalid = {"minecraft:entity_type": {"protocol_id": 5, "entries": {"minecraft:pig": row}}}
            with self.assertRaises(ExportError):
                registry(invalid, "minecraft:entity_type", 1)
        good["minecraft:entity_type"]["default"] = "minecraft:missing"
        with self.assertRaises(ExportError):
            registry(good, "minecraft:entity_type", 1)

    def test_unknown_worldgen_family_fails(self):
        self.assertIsNone(family("data/minecraft/datapacks/experimental/data/minecraft/recipe/a.json"))
        with self.assertRaises(ExportError):
            family("data/minecraft/worldgen/new_kind/a.json")

    def test_inner_server_provenance_required(self):
        with self.assertRaisesRegex(ExportError, "provenance"):
            validate_inner(b"not an original server")

    def test_generated_symlink_and_aggregate_bound(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "link").symlink_to(ROOT / "AGENTS.md")
            with self.assertRaises(ExportError):
                generated_files(root)
            (root / "link").unlink()
            with (root / "large").open("wb") as source:
                source.truncate(MAX_FILE + 1)
            with self.assertRaises(ExportError):
                generated_files(root)
            (root / "large").unlink()
            for i in range(4):
                with (root / f"total_{i}").open("wb") as source:
                    source.truncate(MAX_FILE)
            self.assertEqual(generated_files(root), {})
            (root / "overflow").write_bytes(b"x")
            with self.assertRaisesRegex(ExportError, "aggregate"):
                generated_files(root)

    def test_fixture_extra_missing_and_symlink(self):
        outputs = {name: source_bytes(ROOT / OUTPUT / name) for name in FILES}
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_outputs(outputs, root)
            directory = root / OUTPUT
            (directory / "extra.json").write_bytes(b"{}")
            with self.assertRaisesRegex(ExportError, "fixture file"):
                load_exports(root)
            (directory / "extra.json").unlink()
            (directory / "entity-types.json").unlink()
            with self.assertRaisesRegex(ExportError, "fixture file"):
                load_exports(root)
            (directory / "entity-types.json").symlink_to(ROOT / OUTPUT / "entity-types.json")
            with self.assertRaisesRegex(ExportError, "regular input"):
                load_exports(root)

    def test_version_manifest_and_committed_fixture_integrity(self):
        exports = load_exports()
        self.assertEqual(len(exports["recipes.json"]["entries"]), 1407)
        self.assertEqual(len(exports["entity-types.json"]["entries"]), 151)
        raw = source_bytes(ROOT / OUTPUT / "version-manifest.json")
        validate_version(raw)
        with self.assertRaises(ExportError):
            validate_version(raw + b" ")
        outputs = {name: source_bytes(ROOT / OUTPUT / name) for name in FILES}
        outputs["recipes.json"] += b" "
        with self.assertRaises(ExportError):
            verify_outputs(outputs)
        outputs = {name: source_bytes(ROOT / OUTPUT / name) for name in FILES}
        outputs["manifest.json"] = b"{}"
        with self.assertRaises(ExportError):
            verify_outputs(outputs)


if __name__ == "__main__":
    raise SystemExit(main())
