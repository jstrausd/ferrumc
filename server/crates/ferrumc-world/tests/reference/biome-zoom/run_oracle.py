"""Pinned real BiomeManager calls in both official original artifacts."""
from pathlib import Path
import hashlib
import json
import os
import struct
import subprocess
import sys
import zipfile

DIRECTORY = Path(__file__).resolve().parent
ROOT = next(p for p in DIRECTORY.parents if (p / "AGENTS.md").is_file())
JAVA_ENV = ("JAVA_TOOL_OPTIONS", "_JAVA_OPTIONS", "JDK_JAVA_OPTIONS", "JDK_JAVAC_OPTIONS", "CLASSPATH")
HELPER_PREFIX = "ferrumc/zoomreference/"


class GuardFailure(ValueError):
    """Fail closed before compilation or accepted sample output."""


def require(condition, message):
    if not condition:
        raise GuardFailure(message)


def digest(path):
    h = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def environment_guard(environment):
    for name in JAVA_ENV:
        require(not environment.get(name), "Java environment injection " + name)


def inventory_guard(inventory):
    for path, expected in inventory.items():
        require(digest(path) == expected, "Pinned input mismatch " + path)


def shadow_guard(names, artifact):
    require(not any(name.startswith(HELPER_PREFIX) for name in names), "Helper class shadow " + artifact)


def canonical_guard(source):
    require(Path(source).resolve() == ROOT / "server/crates/ferrumc-world/tests/reference/biome-zoom/ZoomOracle.java", "Noncanonical helper source")


def originals(pins, mode):
    inventory = pins["runtime_inputs"][mode]
    required_count = 45 if mode == "vanilla" else 120
    require(len(inventory) == required_count, "Original artifact inventory count changed")
    expected = ("2bee159e4ce87ae857b4323f9f32c7a3d7d448f7f8c3f62fa6f295a72e228250" if mode == "vanilla"
                else "c64fb7dbc95cd15aa0b7460a5b1e117eef5ee91c4e6fa7a8f1fe6487b6c31507")
    inners = [path for path, pin in inventory.items() if pin == expected]
    require(len(inners) == 1, "Unique actual original inner required")
    guava = [path for path in inventory if "/guava/guava/" in path]
    dfu = [path for path in inventory if "/datafixerupper/" in path]
    require(len(guava) == len(dfu) == 1, "Exact original Guava/DFU origins required")
    return inners[0], guava[0], dfu[0]


def validate(output, requests):
    parsed = [line.split("\t") for line in requests.read_text().splitlines()]
    expected_labels = [(row[0], row[1]) for row in parsed]
    require(len(set(expected_labels)) == len(parsed), "Duplicate input label")
    modes = {}
    for mode in ("vanilla", "paper"):
        rows = [line.split("\t") for line in (output / (mode + ".tsv")).read_text().splitlines()]
        require([(row[0], row[1]) for row in rows] == expected_labels, "Original output order/count changed")
        results = {row[1]: row[2:] for row in rows}
        counts = {kind: sum(row[0] == kind for row in rows) for kind in "HFLGD"}
        require(counts == {"H": 11, "F": 2048, "L": 49, "G": 6815, "D": 54520}, "Incomplete original corpus")
        ties, nearest = [], []
        for request in parsed:
            kind, label = request[:2]
            result = results[label]
            if kind == "H":
                seed = int(request[2])
                independent = int.from_bytes(hashlib.sha256(seed.to_bytes(8, "little", signed=True)).digest()[:8], "little", signed=True)
                require(int(result[0]) == independent, "Actual Guava LE/hash interpretation mismatch")
            elif kind == "G":
                require(result[4:] == ["1", "1", "1"], "Incomplete genuine source/fresh calls")
                seed = int(request[3])
                if request[2] == "world":
                    seed = int.from_bytes(hashlib.sha256(seed.to_bytes(8, "little", signed=True)).digest()[:8], "little", signed=True)
                require(int(result[0]) == seed, "Wrong actual world/obfuscated seed")
                probes = [results[f"{label}.corner-{corner}"][0] for corner in range(8)]
                values = [struct.unpack(">d", bytes.fromhex(bits))[0] for bits in probes]
                minimum = min(values)
                winners = [corner for corner, value in enumerate(values) if value == minimum]
                corner = winners[0]
                shifted = [((int(axis) - 2 + (1 << 31)) % (1 << 32)) - (1 << 31) for axis in request[4:]]
                expected = [(axis >> 2) + int(bool(corner & mask)) for axis, mask in zip(shifted, (4, 2, 1), strict=True)]
                require(list(map(int, result[1:4])) == expected, "getBiome source differs from first genuine method minimum")
                if len(winners) > 1:
                    ties.append({"label": label, "minimum_corners": winners, "selected_corner": corner,
                                 "selected_quart": expected, "genuine_distance_bits": probes})
                sorted_values = sorted(values)
                if len(winners) == 1:
                    gap = sorted_values[1] - sorted_values[0]
                    nearest.append((gap, label, corner, probes))
        origins = (output / (mode + ".tsv.origins.tsv")).read_text().splitlines()
        require(len(origins) == 19 and len(set(origins)) == 19, "Missing original/helper/library CodeSources")
        modes[mode] = {"counts": counts, "actual_getBiome_cases": counts["G"], "source_replacement_fresh_reverse_verified": True,
                       "exact_minimum_ties": ties, "nearest_unique_minima": [
                           {"gap": gap, "label": label, "selected_corner": corner, "genuine_distance_bits": probes}
                           for gap, label, corner, probes in sorted(nearest)[:16]],
                       "actual_codesources": origins,
                       "fixture_sha256": digest(output / (mode + ".tsv")),
                       "origins_sha256": digest(output / (mode + ".tsv.origins.tsv"))}
    v = [row.split("\t") for row in (output / "vanilla.tsv").read_text().splitlines()]
    p = [row.split("\t") for row in (output / "paper.tsv").read_text().splitlines()]
    differences = {kind: sum(a[0] == kind and a[2:] != b[2:] for a, b in zip(v, p, strict=True)) for kind in "HFLGD"}
    (output / "differences.tsv").write_text("".join("\t".join(a) + "\t" + "\t".join(b[2:]) + "\n"
                                                        for a, b in zip(v, p, strict=True) if a[2:] != b[2:]))
    return modes, differences


def main():
    require(len(sys.argv) == 2, "One fresh output directory required")
    source = DIRECTORY / "ZoomOracle.java"
    canonical_guard(source)
    output = Path(sys.argv[1]).resolve()
    require(not output.exists(), "Output must be fresh")
    pins = json.loads((DIRECTORY / "input-pins.json").read_text())
    environment_guard(os.environ)
    environment = dict(os.environ)
    for name in JAVA_ENV:
        environment.pop(name, None)
    inventory = dict(pins["repo_inputs"])
    inventory.update(pins["jdk_inputs"])
    for mode in ("vanilla", "paper"):
        inventory.update(pins["runtime_inputs"][mode])
    inventory.update(pins["bundle_inputs"])
    inventory_guard(inventory)
    for mode in ("vanilla", "paper"):
        for artifact in pins["runtime_inputs"][mode]:
            with zipfile.ZipFile(artifact) as jar:
                shadow_guard(jar.namelist(), artifact)
    output.mkdir()
    classes = output / "classes"
    classes.mkdir()
    jdk = Path(pins["jdk"])
    subprocess.run([str(jdk / "bin/javac"), "-proc:none", "-encoding", "UTF-8", "-d", str(classes), str(source)], env=environment, check=True)
    require(sorted(p.name for p in (classes / "ferrumc/zoomreference").iterdir()) ==
            ["ZoomOracle$Instance.class", "ZoomOracle$Trace.class", "ZoomOracle.class"], "Unexpected fresh helper class inventory")
    for mode in ("vanilla", "paper"):
        inner, guava, dfu = originals(pins, mode)
        paths = [p for p in pins["runtime_inputs"][mode] if p != inner and not p.endswith("/server.jar")]
        cp = os.pathsep.join([inner, *sorted(paths), str(classes)])
        with (output / (mode + ".log")).open("w") as log:
            subprocess.run([str(jdk / "bin/java"), "-Xmx384M", "-cp", cp, "ferrumc.zoomreference.ZoomOracle", mode,
                            str(ROOT / "server/crates/ferrumc-world/tests/fixtures/biome-zoom-original-1.21.8/requests.tsv"), str(output / (mode + ".tsv")), inner, guava, dfu,
                            str(classes), str(DIRECTORY / "semantic-mappings.properties")],
                           cwd=output, env=environment, stdout=log, stderr=subprocess.STDOUT, check=True)
    inventory_guard(inventory)
    modes, differences = validate(output, ROOT / "server/crates/ferrumc-world/tests/fixtures/biome-zoom-original-1.21.8/requests.tsv")
    manifest = {"scope": "Actual original helper computation only, no server/biome resolver/surface activation",
                "inputs": inventory, "input_pins_sha256": digest(DIRECTORY / "input-pins.json"),
                "request_sha256": digest(ROOT / "server/crates/ferrumc-world/tests/fixtures/biome-zoom-original-1.21.8/requests.tsv"), "modes": modes,
                "vanilla_paper_differences": differences, "difference_sha256": digest(output / "differences.tsv"),
                "pre_post_input_hashes_unchanged": True, "genuine_getBiome_and_isolated_distance_probes_distinct": True}
    (output / "manifest.json").write_text(json.dumps(manifest, sort_keys=True, indent=2) + "\n")
    print(json.dumps({"differences": differences, "counts": modes["paper"]["counts"],
                      "ties": {m: len(v["exact_minimum_ties"]) for m, v in modes.items()},
                      "fixtures": {m: v["fixture_sha256"] for m, v in modes.items()}}, sort_keys=True))


if __name__ == "__main__":
    main()
