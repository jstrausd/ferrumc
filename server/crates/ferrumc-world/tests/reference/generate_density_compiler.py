#!/usr/bin/env python3
"""Export actual pinned Minecraft density codec products, without Rust replay.

Usage: generate_density_compiler.py ORIGINAL_CACHE JAVA_HOME OUTPUT_DIR
The checked source fixture is read only; fresh helper classes live in OUTPUT_DIR.
"""
import collections
import hashlib
import json
import os
import pathlib
import platform
import re
import subprocess
import sys
import tempfile
import urllib.request
import zipfile

PINS = {
    "bundle": "2349d9a8f0d4be2c40e7692890ef46a4b07015e7955b075460d02793be7fbbe7",
    "inner": "2bee159e4ce87ae857b4323f9f32c7a3d7d448f7f8c3f62fa6f295a72e228250",
    "mappings": "e373ba569839e6f3c7bd4880247f3e85577d92cde2c605adfa2024eb9cae503e",
    "source_fixture": "bc0dbfa46f495865fc98af835c70dee13ddfdcebb263cf40b3a075324f261b30",
}
MAPPING_SHA1 = "eb1e1eb47cb740012fc82eacc394859463684132"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha256(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def main():
    require(len(sys.argv) == 4, "expected ORIGINAL_CACHE JAVA_HOME OUTPUT_DIR")
    cache, java_home, output = (pathlib.Path(value).resolve() for value in sys.argv[1:])
    output.mkdir(parents=True, exist_ok=True)
    bundle = cache / "server.jar"
    inner = cache / "versions/1.21.8/server-1.21.8.jar"
    require(sha256(bundle) == PINS["bundle"], "official bundle SHA256 mismatch")
    require(sha256(inner) == PINS["inner"], "official inner JAR SHA256 mismatch")
    source = pathlib.Path(__file__).with_name("density_compiler_oracle.java").resolve()
    fixture_source = pathlib.Path(__file__).resolve().parents[5] / "docs/fixtures/minecraft-1.21.8-original/worldgen.json"
    require(sha256(fixture_source) == PINS["source_fixture"], "original worldgen fixture SHA256 mismatch")
    mappings = output / "server-mappings.txt"
    if not mappings.exists():
        with urllib.request.urlopen(f"https://piston-data.mojang.com/v1/objects/{MAPPING_SHA1}/server.txt", timeout=30) as response:
            mappings.write_bytes(response.read(8_186_233))
    require(mappings.stat().st_size == 8_186_232, "official mappings size mismatch")
    require(sha256(mappings) == PINS["mappings"], "official mappings SHA256 mismatch")
    classpath = [inner]
    libraries = {}
    with zipfile.ZipFile(bundle) as archive:
        for line in archive.read("META-INF/libraries.list").decode().splitlines():
            digest, coordinate, relative = line.split("\t")
            path = cache / "libraries" / relative
            require(sha256(path) == digest, f"official library SHA256 mismatch: {coordinate}")
            classpath.append(path)
            libraries[coordinate] = digest
    require(len(libraries) == 43, "official library inventory mismatch")
    cp = os.pathsep.join(str(path.resolve()) for path in classpath)
    java = java_home / "bin/java"
    version = subprocess.run([str(java), "-version"], capture_output=True, text=True, check=True).stderr
    require(re.search(r'\bversion "21(?:\.|\")', version) is not None, "oracle requires Java 21")
    fixture = output / "density_compiler_1_21_8.jsonl"
    with tempfile.TemporaryDirectory(prefix="density-oracle-classes-", dir=output) as classes:
        subprocess.run([str(java_home / "bin/javac"), "-proc:none", "-encoding", "UTF-8", "-cp", cp, "-d", classes, str(source)], check=True)
        subprocess.run([str(java), "-cp", cp + os.pathsep + classes, "ferrumc.reference.DensityCompilerOracle", str(fixture_source), str(fixture)], cwd=output, check=True)
    records = [json.loads(line) for line in fixture.read_text().splitlines()]
    counts = dict(sorted(collections.Counter(record["kind"] for record in records).items()))
    require(counts.get("density") == 35 and counts.get("router") == 7, "decoded original registry inventory mismatch")
    require(len({(record["kind"], record["id"]) for record in records}) == len(records), "duplicate oracle record identity")
    probe_inputs = [json.loads(record["input_json"]) for record in records
                    if record["kind"] == "probe" and record["status"] == "ok"]
    forms = sorted({value["type"].removeprefix("minecraft:") for value in probe_inputs
                    if isinstance(value, dict) and "type" in value})
    require(len(forms) == 32, "successful original density codec form coverage mismatch")
    manifest = {
        "minecraft": "1.21.8", "protocol": 772,
        "source": "official inner server JAR; actual VanillaRegistries, RegistryOps and density/settings codecs; reflection-only semantic projection; no density sampling",
        "projection_schema": 1, "pins": PINS, "mapping_sha1": MAPPING_SHA1,
        "libraries_sha256": libraries,
        "python": platform.python_version(), "platform": platform.platform(), "java": version.strip(),
        "javac": subprocess.run([str(java_home / "bin/javac"), "-version"], capture_output=True, text=True, check=True).stdout.strip(),
        "oracle_sha256": sha256(source), "generator_sha256": sha256(pathlib.Path(__file__)),
        "record_types": counts, "records": len(records),
        "successful_density_codec_forms": forms,
        "probe_statuses": dict(sorted(collections.Counter(record["status"] for record in records if record["kind"] in ("probe", "settings_probe")).items())),
        "fixture_bytes": fixture.stat().st_size, "fixture_sha256": sha256(fixture),
    }
    (output / "density_compiler_1_21_8.manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    print(json.dumps({key: manifest[key] for key in ("record_types", "probe_statuses", "records", "fixture_bytes", "fixture_sha256")}))


if __name__ == "__main__":
    main()
