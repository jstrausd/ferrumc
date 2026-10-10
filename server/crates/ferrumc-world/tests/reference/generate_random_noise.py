#!/usr/bin/env python3
"""Reproduce goldens by invoking the official 1.21.8 classes, not copied algorithms.

Usage: python3 generate_random_noise.py ORIGINAL_CACHE JAVA_HOME OUTPUT_DIR
ORIGINAL_CACHE is the verified cache produced by minecraft-original-export.py.
All probe/class/output work occurs in OUTPUT_DIR. No checked-in fixture is overwritten.
"""
import hashlib
import json
import pathlib
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

def sha256(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()

def require(condition, message):
    if not condition:
        raise ValueError(message)

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
    source = pathlib.Path(__file__).with_name("RandomNoiseOracle.java")
    cp = ":".join(str(path.resolve()) for path in classpath)
    java = java_home / "bin/java"
    version = subprocess.run([str(java), "-version"], capture_output=True, text=True, check=True).stderr
    require(re.search(r'\bversion "21(?:\.|\")', version) is not None, "oracle requires Java 21")
    fixture = output / "random_noise_1_21_8.tsv"
    # Only this run's oracle bytecode is retained. Original pinned classes and
    # pinned libraries precede it, so stale classes cannot shadow the reference.
    with tempfile.TemporaryDirectory(prefix="oracle-classes-", dir=output) as classes:
        subprocess.run([str(java_home / "bin/javac"), "-proc:none", "-encoding", "UTF-8", "-cp", cp, "-d", classes, str(source)], check=True)
        subprocess.run([str(java), "-cp", cp + ":" + classes, "ferrumc.reference.RandomNoiseOracle", str(fixture)], cwd=output, check=True)
    manifest = {"minecraft": "1.21.8", "protocol": 772, "source": "official inner server JAR; direct reflection calls; no reimplemented oracle algorithms", "java": version.strip(), "pins": PINS, "mapping_sha1": MAPPING_SHA1, "libraries_sha256": libraries, "oracle_sha256": sha256(source), "generator_sha256": sha256(pathlib.Path(__file__)), "fixture_sha256": sha256(fixture), "fixture_bytes": fixture.stat().st_size, "records": sum(not line.startswith("#") for line in fixture.read_text().splitlines())}
    (output / "random_noise_1_21_8.manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    print(json.dumps({key: manifest[key] for key in ("fixture_sha256", "fixture_bytes", "records")}))

if __name__ == "__main__":
    main()
