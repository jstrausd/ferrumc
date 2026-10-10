#!/usr/bin/env python3
"""Export actual pinned Minecraft density codec products, without Rust replay.

Usage: generate_density_router.py ORIGINAL_CACHE JAVA_HOME OUTPUT_DIR
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
    "baseline_fixture": "70cded38014c5f4b6947c09da9588a578013ecef11fd53493fc52e889db2aae6",
}
MAPPING_SHA1 = "eb1e1eb47cb740012fc82eacc394859463684132"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha256(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def verify_digest(path, expected, label):
    require(sha256(path) == expected, f"{label} SHA256 mismatch")


def reject_helper_shadow(classpath):
    for path in classpath:
        with zipfile.ZipFile(path) as archive:
            require("ferrumc/reference/DensityRouterOracle.class" not in archive.namelist(),
                    f"helper classpath shadow: {path.name}")


def baseline_record(record):
    # Strip only new metadata. The old actual-JAR sample/structure corpus must
    # remain byte-identical, including native resource-holder references.
    if record["kind"] not in {"binding", "projection", "point", "quart"}:
        return None

    def strip(value):
        if isinstance(value, list):
            return [strip(child) for child in value]
        if isinstance(value, dict):
            result = {key: strip(child) for key, child in value.items()}
            if (value.get("kind") in {"constant", "multipoint"}
                    and len(value.get("min_value_bits", "")) == 8):
                result.pop("min_value_bits")
                result.pop("max_value_bits")
            return result
        return value

    result = strip(record)
    if result["kind"] == "binding":
        result.pop("aquifer_draws")
        result.pop("ore_draws")
        result.pop("noise_max_values")
    if result["kind"] == "projection":
        result.pop("registered_named")
    return result


def identity(record):
    if record["kind"] in {"quantize", "trace"}:
        return record["kind"], record["case"]
    if record["kind"] == "wrapped_noise":
        return (record["kind"], record["settings"], record["seed"], record["noise"],
                record["name_mode"], record["stack"], record["case"])
    return record["kind"], record["settings"], record["seed"], tuple(record.get("position", []))


def main():
    require(len(sys.argv) == 4, "expected ORIGINAL_CACHE JAVA_HOME OUTPUT_DIR")
    cache, java_home, output = (pathlib.Path(value).resolve() for value in sys.argv[1:])
    output.mkdir(parents=True, exist_ok=True)
    bundle = cache / "server.jar"
    inner = cache / "versions/1.21.8/server-1.21.8.jar"
    verify_digest(bundle, PINS["bundle"], "official bundle")
    verify_digest(inner, PINS["inner"], "official inner JAR")
    source = pathlib.Path(__file__).with_name("density_router_oracle.java").resolve()
    fixture_source = pathlib.Path(__file__).resolve().parents[5] / "docs/fixtures/minecraft-1.21.8-original/worldgen.json"
    verify_digest(fixture_source, PINS["source_fixture"], "original worldgen fixture")
    mappings = output / "server-mappings.txt"
    if not mappings.exists():
        with urllib.request.urlopen(f"https://piston-data.mojang.com/v1/objects/{MAPPING_SHA1}/server.txt", timeout=30) as response:
            mappings.write_bytes(response.read(8_186_233))
    require(mappings.stat().st_size == 8_186_232, "official mappings size mismatch")
    verify_digest(mappings, PINS["mappings"], "official mappings")
    classpath = [inner]
    libraries = {}
    with zipfile.ZipFile(bundle) as archive:
        for line in archive.read("META-INF/libraries.list").decode().splitlines():
            digest, coordinate, relative = line.split("\t")
            path = cache / "libraries" / relative
            verify_digest(path, digest, f"official library: {coordinate}")
            classpath.append(path)
            libraries[coordinate] = digest
    require(len(libraries) == 43, "official library inventory mismatch")
    require(len(classpath) == 44 and len(set(classpath)) == 44, "official classpath inventory mismatch")
    reject_helper_shadow(classpath)
    for variable in ("JAVA_TOOL_OPTIONS", "_JAVA_OPTIONS", "JDK_JAVA_OPTIONS"):
        require(not os.environ.get(variable), f"unsupported Java classpath/agent injection: {variable}")
    cp = os.pathsep.join(str(path.resolve()) for path in classpath)
    java = java_home / "bin/java"
    version = subprocess.run([str(java), "-version"], capture_output=True, text=True, check=True).stderr
    require(re.search(r'\bversion "21(?:\.|\")', version) is not None, "oracle requires Java 21")
    artifacts = [bundle, inner, mappings, fixture_source, source,
                 pathlib.Path(__file__).resolve(), *classpath[1:],
                 java_home / "bin/java", java_home / "bin/javac"]
    before = {path: sha256(path) for path in artifacts}
    fixture = output / "density_router_1_21_8.jsonl"
    with tempfile.TemporaryDirectory(prefix="density-oracle-classes-", dir=output) as classes:
        subprocess.run([str(java_home / "bin/javac"), "-proc:none", "-encoding", "UTF-8", "-cp", cp, "-d", classes, str(source)], check=True)
        subprocess.run([str(java), "-cp", cp + os.pathsep + classes, "ferrumc.reference.DensityRouterOracle",
                        str(fixture_source), str(fixture), str(inner), classes], cwd=output, check=True)
    require(before == {path: sha256(path) for path in artifacts}, "oracle input artifact changed during execution")
    reject_helper_shadow(classpath)
    records = [json.loads(line) for line in fixture.read_text().splitlines()]
    counts = dict(sorted(collections.Counter(record["kind"] for record in records).items()))
    require(counts.get("binding") == 49 and counts.get("projection") == 7, "actual RandomState inventory mismatch")
    require(counts.get("quart") == 294, "actual Climate quart inventory mismatch")
    require(counts.get("point") == 1813, "actual point inventory mismatch")
    require(counts.get("wrapped_noise") == 14700, "actual wrapped-noise inventory mismatch")
    require(counts.get("quantize") == 42, "actual quantize inventory mismatch")
    require(counts.get("trace") == 308, "actual child trace inventory mismatch")
    identities = [identity(record) for record in records]
    require(len(set(identities)) == len(records), "duplicate oracle record identity")
    baseline = hashlib.sha256()
    for record in records:
        old = baseline_record(record)
        if old is not None:
            baseline.update((json.dumps(old, sort_keys=True, separators=(",", ":")) + "\n").encode())
    require(baseline.hexdigest() == PINS["baseline_fixture"], "existing oracle baseline changed")
    require(sum(len(record["fields"]) + len(record["named"]) for record in records
                if record["kind"] == "point") == 90650, "actual direct density value inventory mismatch")
    draws = sum(len(record["aquifer_draws"]) + len(record["ore_draws"])
                for record in records if record["kind"] == "binding")
    require(draws == 686, "actual positional factory draw inventory mismatch")
    registered = sum(len(record["registered_named"]) for record in records if record["kind"] == "projection")
    require(registered == 245, "actual native registered root inventory mismatch")
    noise_maxima = sum(len(record["noise_max_values"]) for record in records if record["kind"] == "binding")
    require(noise_maxima == 2940, "actual noise diagnostic inventory mismatch")
    manifest = {
        "minecraft": "1.21.8", "protocol": 772,
        "source": "official inner server JAR; actual VanillaRegistries, RandomState and original visitors; original density/scalar/bounds and Climate samples; no copied numerical or wiring formulas",
        "projection_schema": 2, "pins": PINS, "mapping_sha1": MAPPING_SHA1,
        "libraries_sha256": libraries,
        "python": platform.python_version(), "platform": platform.platform(), "java": version.strip(),
        "javac": subprocess.run([str(java_home / "bin/javac"), "-version"], capture_output=True, text=True, check=True).stdout.strip(),
        "oracle_sha256": sha256(source), "generator_sha256": sha256(pathlib.Path(__file__)),
        "record_types": counts, "records": len(records),
        "direct_density_values": 90650, "positional_draw_positions": draws,
        "positional_next_longs": draws * 3, "registered_named_roots": registered,
        "original_noise_max_value_pins": noise_maxima,
        "request_order": "actual factory draws and wrapped-noise samples checked in reverse order",
        "origin_validation": "27 original class CodeSources checked against inner JAR; fresh helper CodeSource checked; helper classpath shadow refused",
        "inputs_rehashed_after_execution": len(before),
        "fixture_bytes": fixture.stat().st_size, "fixture_sha256": sha256(fixture),
    }
    (output / "density_router_1_21_8.manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    print(json.dumps({key: manifest[key] for key in ("record_types", "records", "fixture_bytes", "fixture_sha256")}))


if __name__ == "__main__":
    main()
