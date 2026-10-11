"""Checked original methods only; exact reviewed input corpus, fail-closed guards."""
from pathlib import Path
import hashlib
import json
import math
import os
import struct
import subprocess
import sys
import zipfile

HERE = Path(__file__).resolve().parent
ROOT = next(path for path in HERE.parents if (path / "AGENTS.md").is_file())
DESIGN = ROOT / ".codex-tmp/gen05_7/design"
JAVA_ENV = ("JAVA_TOOL_OPTIONS", "_JAVA_OPTIONS", "JDK_JAVA_OPTIONS", "JDK_JAVAC_OPTIONS", "CLASSPATH")
HELPER_PREFIX = "ferrumc/temperatureoracle/"
FILES = ("metadata.tsv", "constructors.tsv", "noise.tsv", "scalar.tsv", "mutable.tsv", "cache.tsv", "origins.tsv", "frozen-extension.tsv")


class GuardFailure(ValueError):
    """No accepted output on failed admission."""


def require(condition, message):
    if not condition:
        raise GuardFailure(message)


def digest(path):
    h = hashlib.sha256()
    with Path(path).open("rb") as source:
        for block in iter(lambda: source.read(1048576), b""):
            h.update(block)
    return h.hexdigest()


def environment_guard(env):
    for name in JAVA_ENV:
        require(not env.get(name), "Java environment injection " + name)


def inventory_guard(inventory):
    for path, expected in inventory.items():
        require(digest(path) == expected, "Pinned input mismatch " + path)


def canonical_guard(source):
    require(Path(source).resolve() == ROOT / ".codex-tmp/gen05_7/oracle/TemperatureOracle.java", "Noncanonical helper source")


def shadow_guard(names, artifact):
    require(not any(name.startswith(HELPER_PREFIX) for name in names), "Helper package shadow " + artifact)


def fresh_guard(path):
    require(not path.exists(), "Output must be fresh")


def original(pins, mode):
    inventory = pins["runtime_inputs"][mode]
    require(len(inventory) == (45 if mode == "vanilla" else 120), "Original artifact count drift")
    expected = "2bee159e4ce87ae857b4323f9f32c7a3d7d448f7f8c3f62fa6f295a72e228250" if mode == "vanilla" else "c64fb7dbc95cd15aa0b7460a5b1e117eef5ee91c4e6fa7a8f1fe6487b6c31507"
    paths = [path for path, value in inventory.items() if value == expected]
    require(len(paths) == 1, "Unique pinned inner required")
    return paths[0]


def classpath(pins, mode, classes):
    inner = original(pins, mode)
    paths = [path for path in pins["runtime_inputs"][mode] if path != inner and not path.endswith("/server.jar")]
    return os.pathsep.join([inner, *sorted(paths), str(classes)])


def command(pins, mode, classes, directory, expected_inner=None):
    return [str(Path(pins["jdk"]) / "bin/java"), "-Xmx384M", "-cp", classpath(pins, mode, classes),
            "ferrumc.temperatureoracle.TemperatureOracle", mode, str(directory), expected_inner or original(pins, mode),
            str(classes), str(HERE / "semantic-mappings.properties"), str(DESIGN / "scalar-inputs.tsv"),
            str(HERE / "control-inputs.tsv"), str(HERE / "noise-inputs.tsv"), str(HERE / (mode + "-library-origins.properties")),
            str(HERE / "frozen-extension-inputs.tsv")]


def rows(path):
    return [line.split("\t") for line in path.read_text().splitlines()]


def f32(bits):
    return struct.unpack(">f", bytes.fromhex(bits))[0]


def f64(bits):
    return struct.unpack(">d", bytes.fromhex(bits))[0]


def validate(output):
    scalar_inputs = rows(DESIGN / "scalar-inputs.tsv")[1:]
    extension_inputs = rows(HERE / "frozen-extension-inputs.tsv")[1:]
    controls = rows(HERE / "control-inputs.tsv")[1:]
    contract = json.loads((DESIGN / "cases.json").read_text())
    census = json.loads((ROOT / ".codex-tmp/gen05_7/census/metadata.json").read_text())["full_biome_roster"]
    summaries = {}
    for mode in ("vanilla", "paper"):
        directory = output / mode
        metadata = rows(directory / "metadata.tsv")
        require([row[0] for row in metadata] == sorted(census), "Actual metadata roster/order drift")
        for biome, bits, precipitation, modifier, climate_bits in metadata:
            require(bits == climate_bits, "Original base getter and climate disagree")
            require(precipitation == str(census[biome]["has_precipitation"]).lower(), "Original precipitation flag differs")
            require(modifier.lower() == census[biome]["modifier"], "Original modifier differs")
            require(f32(bits) == f32(struct.pack(">f", float(census[biome]["temperature_token"])).hex()), "Builtin base climate differs")
        constructors = rows(directory / "constructors.tsv")
        nrows, prows = [r for r in constructors if r[0] == "N"], [r for r in constructors if r[0] == "P"]
        require(len(nrows) == 3 and len(prows) == 5, "Missing constructor factors/permutations")
        require([int(r[3]) for r in nrows] == [262, 786, 262], "Original draw census differs")
        for row in prows:
            permutation = list(map(int, row[6].split(",")))
            require(len(permutation) == 512 and sorted(permutation[:256]) == list(range(256)), "Malformed actual original permutation")
            require(permutation[256:] == [0] * 256, "Unexpected original unused permutation half")
        noise = rows(directory / "noise.tsv")
        require(len(noise) == 352, "Missing direct actual noise samples")
        require([r[0] for r in noise] == [f"noise/{i:03d}/{j}" for i in range(88) for j in range(4)], "Noise order drift")
        frozen_branches = {"outer_not_below_0.3": 0, "inner_not_below_0.8": 0, "modified_to_0.2": 0}
        branch_records = []
        for i in range(88):
            block = noise[i * 4:i * 4 + 4]
            first = f64(block[1][5]) * 7.0 + f64(block[2][5])
            second = f64(block[3][5])
            branch = "outer_not_below_0.3" if not first < 0.3 else "inner_not_below_0.8" if not second < 0.8 else "modified_to_0.2"
            frozen_branches[branch] += 1
            branch_records.append({"id": f"noise/{i:03d}", "original_noise_bits": [r[5] for r in block], "observed_predicate_branch": branch})
        scalar = rows(directory / "scalar.tsv")
        require(len(scalar) == 93584 and all(len(r) == 13 for r in scalar), "Missing scalar corpus or fields")
        require([r[:6] for r in scalar] == scalar_inputs, "Scalar identity/input chronology drift")
        finite = sum(math.isfinite(f32(row[6])) for row in scalar)
        extension = rows(directory / "frozen-extension.tsv")
        require(len(extension) == 32768 and all(len(r) == 16 for r in extension) and [r[:6] for r in extension] == extension_inputs, "Frozen extension shape/chronology drift")
        extension_branches = {"outer_not_below_0.3": 0, "inner_not_below_0.8": 0, "modified_to_0.2": 0}
        for r in extension:
            first = f64(r[13]) * 7.0 + f64(r[14])
            second = f64(r[15])
            branch = "outer_not_below_0.3" if not first < 0.3 else "inner_not_below_0.8" if not second < 0.8 else "modified_to_0.2"
            extension_branches[branch] += 1
        require(all(row[8:13] == ["-"] * 5 for row in extension) if mode == "vanilla" else all(row[8] != "-" for row in extension), "Wrong extension public labels")
        require(all(row[8:] == ["-"] * 5 for row in scalar) if mode == "vanilla" else all(row[8] != "-" for row in scalar), "Wrong stateless/public contract labels")
        mutable = rows(directory / "mutable.tsv")
        expected_mutable = [r[1:7] for r in controls if r[0] == "M"]
        require(len(mutable) == 80 and [r[:6] for r in mutable] == expected_mutable, "Missing real MutableBlockPos reuse")
        require(all(r[6] == r[7] for r in mutable), "Actual reused mutable differs from immutable private baseline")
        cache = rows(directory / "cache.tsv")
        expected_cache = [r[1:7] for r in controls if r[0] == "C"]
        require(len(cache) == 3156 and all(len(r) == 11 for r in cache) and [r[:6] for r in cache] == expected_cache, "Cache chronology/input drift")
        require(all(r[7] == r[9] for r in cache), "Genuine cold-public and private baseline differ")
        cache_sizes = [0 if not r[10] else len(r[10].split(",")) for r in cache]
        origins = rows(directory / "origins.tsv")
        require(len(origins) >= 40 and len({r[0] for r in origins}) == len(origins), "Missing actual CodeSource evidence")
        (directory / "frozen-observations.json").write_text(json.dumps(branch_records, sort_keys=True, indent=2) + "\n")
        summaries[mode] = {"metadata": len(metadata), "simplex_permutations": len(prows), "original_core_draws": [int(r[3]) for r in nrows],
                           "direct_noise_samples": len(noise), "scalar_rows": len(scalar), "finite_original_scalar_rows": finite,
                           "mutable_rows": len(mutable), "cache_rows": len(cache), "total_rows": len(scalar) + len(mutable) + len(cache),
                           "frozen_noise_predicate_observations": frozen_branches,
                           "extension_scalar_rows": len(extension), "extension_original_noise_samples": len(extension) * 3,
                           "extension_frozen_noise_predicate_observations": extension_branches,
                           "combined_temperature_rows": len(scalar) + len(mutable) + len(cache) + len(extension),
                           "retained_public_different_from_private": sum(r[7] != r[8] for r in cache),
                           "cache_max_size": max(cache_sizes), "codesources": len(origins),
                           "outputs": {name: digest(directory / name) for name in FILES}}
    v, p = rows(output / "vanilla/scalar.tsv"), rows(output / "paper/scalar.tsv")
    differences = [{"id": a[0], "private_vanilla": a[6], "private_paper": b[6], "public_paper": b[8],
                    "modified_vanilla": a[7], "modified_paper": b[7]}
                   for a,b in zip(v,p,strict=True) if a[6] != b[6] or a[7] != b[7] or a[6] != b[8]]
    (output / "stateless-differences.json").write_text(json.dumps(differences, sort_keys=True, indent=2) + "\n")
    ve, pe = rows(output / "vanilla/frozen-extension.tsv"), rows(output / "paper/frozen-extension.tsv")
    extension_differences = [{"id": a[0], "vanilla": a[6:], "paper": b[6:]}
                             for a,b in zip(ve,pe,strict=True) if a[6:8] != b[6:8] or a[13:] != b[13:] or a[6] != b[8]]
    (output / "extension-stateless-differences.json").write_text(json.dumps(extension_differences, sort_keys=True, indent=2) + "\n")
    require(summaries["paper"]["combined_temperature_rows"] == 129588 and summaries["paper"]["combined_temperature_rows"] < 150000, "Extension admission drift")
    require(summaries["paper"]["total_rows"] == contract["counts"]["total_temperature_rows_per_mode"], "Reviewed count drift")
    return summaries, differences


def main():
    require(len(sys.argv) == 2, "One fresh output directory required")
    source = HERE / "TemperatureOracle.java"
    canonical_guard(source)
    environment_guard(os.environ)
    output = Path(sys.argv[1]).resolve()
    fresh_guard(output)
    pins = json.loads((HERE / "input-pins.json").read_text())
    inventory_guard(pins["inventory"])
    for mode in ("vanilla", "paper"):
        for artifact in pins["runtime_inputs"][mode]:
            with zipfile.ZipFile(artifact) as jar:
                shadow_guard(jar.namelist(), artifact)
    output.mkdir()
    classes = output / "classes"
    classes.mkdir()
    subprocess.run([str(Path(pins["jdk"]) / "bin/javac"), "-proc:none", "-encoding", "UTF-8", "-d", str(classes), str(source)], check=True)
    require(sorted(p.name for p in (classes / HELPER_PREFIX).iterdir()) == ["TemperatureOracle.class"], "Unexpected compiled helper classes")
    for mode in ("vanilla", "paper"):
        directory = output / mode
        directory.mkdir()
        with (output / (mode + ".log")).open("w") as log:
            subprocess.run(command(pins, mode, classes, directory), cwd=directory, stdout=log, stderr=subprocess.STDOUT, check=True)
    inventory_guard(pins["inventory"])
    summaries, differences = validate(output)
    manifest = {"scope": "Genuine original biome/noise/temperature/predicates/mutable/cache computation; no Rust/runtime/Survival acceptance",
                "input_pins_sha256": digest(HERE / "input-pins.json"), "inputs": pins["inventory"],
                "modes": summaries, "stateless_difference_count": len(differences),
                "stateless_differences_sha256": digest(output / "stateless-differences.json"),
                "extension_stateless_differences_sha256": digest(output / "extension-stateless-differences.json"), "pre_post_actual_inputs_unchanged": True}
    (output / "manifest.json").write_text(json.dumps(manifest, sort_keys=True, indent=2) + "\n")
    print(json.dumps({"modes": summaries, "stateless_difference_count": len(differences)}, sort_keys=True))


if __name__ == "__main__":
    main()
