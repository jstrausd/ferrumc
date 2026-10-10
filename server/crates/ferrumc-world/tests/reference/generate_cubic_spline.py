#!/usr/bin/env python3
"""Export actual pinned original-JAR float32 splines; no copied interpolation.

Usage: generate_cubic_spline.py ORIGINAL_CACHE JAVA_HOME OUTPUT_DIR
Fresh helper bytecode lives only in OUTPUT_DIR; checked-in fixtures are untouched.
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
    source = pathlib.Path(__file__).with_name("CubicSplineOracle.java")
    cp = os.pathsep.join(str(path.resolve()) for path in classpath)
    java = java_home / "bin/java"
    version = subprocess.run([str(java), "-version"], capture_output=True, text=True, check=True).stderr
    require(re.search(r'\bversion "21(?:\.|\")', version) is not None, "oracle requires Java 21")
    fixture = output / "cubic_spline_1_21_8.tsv"
    with tempfile.TemporaryDirectory(prefix="spline-oracle-classes-", dir=output) as classes:
        subprocess.run([str(java_home / "bin/javac"), "-proc:none", "-encoding", "UTF-8", "-cp", cp, "-d", classes, str(source)], check=True)
        subprocess.run([str(java), "-cp", cp + os.pathsep + classes, "ferrumc.reference.CubicSplineOracle", str(fixture)], cwd=output, check=True)
    lines = [line for line in fixture.read_text().splitlines() if not line.startswith("#")]
    manifest = {
        "minecraft": "1.21.8", "protocol": 772,
        "source": "official inner server JAR; direct CubicSpline reflection; no reimplemented interpolation",
        "pins": PINS, "mapping_sha1": MAPPING_SHA1, "libraries_sha256": libraries,
        "python": platform.python_version(), "platform": platform.platform(), "java": version.strip(),
        "javac": subprocess.run([str(java_home / "bin/javac"), "-version"], capture_output=True, text=True, check=True).stdout.strip(),
        "oracle_sha256": sha256(source), "generator_sha256": sha256(pathlib.Path(__file__)),
        "record_types": dict(sorted(collections.Counter(line.split("|")[0] for line in lines).items())),
        "cases": len({line.split("|")[1] for line in lines}),
        "records": len(lines), "fixture_bytes": fixture.stat().st_size, "fixture_sha256": sha256(fixture),
    }
    (output / "cubic_spline_1_21_8.manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    print(json.dumps({key: manifest[key] for key in ("cases", "records", "fixture_bytes", "fixture_sha256")}))


if __name__ == "__main__":
    main()
