"""Fresh actual-JAR exports. All provenance guards also run under Python -O."""
from pathlib import Path
import collections
import hashlib
import json
import os
import subprocess
import sys
import zipfile

HERE = Path(__file__).resolve().parent
ROOT = Path(__file__).resolve().parents[5]
JAVA_HOME = Path("/home/jstrauss/.local/share/ferrumc-tools/jdk-21.0.12.1+1")
HELPER = HERE / "NoiseChunkIndependentOracle.java"
MAPS = HERE / "semantic-mappings.json"
SOURCE_PINS = HERE / "source-pins.json"
ROOT_INPUTS = ROOT / ".codex-tmp/gen05_2-root-paper-end/inputs.json"
FIXTURE = ROOT / "docs/fixtures/minecraft-1.21.8-original/worldgen.json"
PIN = {
    "helper": "b19574d4f5d82e1a97b77985cb1cc5b2896e7eaf9f97c34d804b0e8355f79380",
    "mapping": "b4301f100eece874946b3962da3ac0b5efd8e4b3ef9e76a314e6a8d2750687ca",
    "source_pins": "61d9659d4de817e3f114f38f35221d2ee16b6f626fdb4b441dfd28ff8694eda7",
    "root_inputs": "d7eff61da418e8a8883f69dd2b079cc5c5b1a91612f16745f3c07ec90ff58841",
    "fixture": "bc0dbfa46f495865fc98af835c70dee13ddfdcebb263cf40b3a075324f261b30",
    "vanilla_inner": "2bee159e4ce87ae857b4323f9f32c7a3d7d448f7f8c3f62fa6f295a72e228250",
    "paper_inner": "c64fb7dbc95cd15aa0b7460a5b1e117eef5ee91c4e6fa7a8f1fe6487b6c31507",
    "vanilla_bundle": "2349d9a8f0d4be2c40e7692890ef46a4b07015e7955b075460d02793be7fbbe7",
    "paper_bundle": "8de7c52c3b02403503d16fac58003f1efef7dd7a0256786843927fa92ee57f1e",
}
JDK_PINS = {
    "bin/java": "2a207f5e7d075afa01d97f8048389a64432a44c4a5af0f5e77d6e286ec5f401d",
    "bin/javac": "55859b80e7a9c4c4736be19ad3addeb35112ca6d17a30c4e0e116afc0a499bdb",
    "lib/modules": "70c5465dabf8e1bec2860631ea3ffd342f0ec23a5e2efd3a389d2abe32448f98",
}
ENV = ("JAVA_TOOL_OPTIONS", "_JAVA_OPTIONS", "JDK_JAVA_OPTIONS", "JDK_JAVAC_OPTIONS", "CLASSPATH")
LIB_CLASSES = [
    "com.google.gson.JsonParser", "com.google.gson.Gson",
    "com.mojang.serialization.Codec", "com.mojang.serialization.JsonOps",
    "com.google.common.collect.Lists", "it.unimi.dsi.fastutil.longs.Long2IntOpenHashMap",
]


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path):
    with Path(path).open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def verify(path, expected, label):
    require(digest(path) == expected, label + " hash mismatch")


def reject_env(environment):
    for key in ENV:
        require(not environment.get(key), "Java environment injection " + key)


def reject_shadow(paths):
    for path in paths:
        with zipfile.ZipFile(path) as archive:
            require(not any(name.startswith("ferrumc/independent/NoiseChunkIndependentOracle") and name.endswith(".class") for name in archive.namelist()),
                    "helper classpath shadow " + str(path))


def library_origins(paths):
    result = {}
    for cls in LIB_CLASSES:
        member = cls.replace(".", "/") + ".class"
        candidates = []
        for path in paths:
            with zipfile.ZipFile(path) as archive:
                if member in archive.namelist():
                    candidates.append(str(path))
        require(len(candidates) == 1, "ambiguous library class origin " + cls)
        result[cls] = candidates[0]
    return result


def configuration():
    reject_env(os.environ)
    for label, path in [("helper", HELPER), ("mapping", MAPS), ("source_pins", SOURCE_PINS), ("root_inputs", ROOT_INPUTS), ("fixture", FIXTURE)]:
        verify(path, PIN[label], label)
    for relative, expected in JDK_PINS.items():
        verify(JAVA_HOME / relative, expected, "JDK " + relative)
    sources = json.loads(SOURCE_PINS.read_text())
    for path, expected in sources.items():
        verify(path, expected, "source/evidence " + path)
    inputs = json.loads(ROOT_INPUTS.read_text())
    version = subprocess.run([str(JAVA_HOME / "bin/java"), "-version"], capture_output=True, text=True, check=True).stderr.strip()
    require(version == inputs["java"], "JDK version changed")
    verify(inputs["paper_bundle"]["path"], PIN["paper_bundle"], "Paper bundle")
    require(inputs["paper_bundle"]["sha256"] == PIN["paper_bundle"], "Paper bundle declaration changed")
    modes = {}
    for mode, expected_count in [("vanilla", 45), ("paper", 120)]:
        artifacts = inputs[mode]
        require(len(artifacts) == expected_count, mode + " exact artifact count changed")
        for path, expected in artifacts.items():
            verify(path, expected, mode + " artifact " + path)
        candidates = [path for path, sha in artifacts.items() if sha == PIN[mode + "_inner"]]
        require(len(candidates) == 1, "unique pinned " + mode + " inner required")
        inner = candidates[0]
        if mode == "vanilla":
            bundle = ROOT / ".codex-tmp/original-1.21.8/server.jar"
            verify(bundle, PIN["vanilla_bundle"], "Vanilla bundle")
            with zipfile.ZipFile(bundle) as archive:
                libraries = archive.read("META-INF/libraries.list").decode().splitlines()
                require(len(libraries) == 43, "Vanilla bundle inventory changed")
                for line in libraries:
                    expected, coordinate, relative = line.split("\t")
                    path = bundle.parent / "libraries" / relative
                    require(artifacts.get(str(path)) == expected, "Vanilla library inventory pin mismatch " + coordinate)
        paths = [inner, *sorted(path for path in artifacts if path != inner and not path.endswith("/server.jar"))]
        require(len(paths) == (44 if mode == "vanilla" else 120), "classpath count changed")
        reject_shadow(paths)
        modes[mode] = {"inner": inner, "paths": paths, "library_origins": library_origins(paths)}
    pinned = {str(path): PIN[label] for label, path in [("helper", HELPER), ("mapping", MAPS), ("source_pins", SOURCE_PINS), ("root_inputs", ROOT_INPUTS), ("fixture", FIXTURE)]}
    pinned.update(sources)
    pinned.update({str(JAVA_HOME / relative): expected for relative, expected in JDK_PINS.items()})
    pinned.update(inputs["vanilla"])
    pinned.update(inputs["paper"])
    pinned[inputs["paper_bundle"]["path"]] = PIN["paper_bundle"]
    return modes, pinned, version


def compare(vanilla, paper):
    require(len(vanilla) == len(paper), "backend row counts differ")
    differences = collections.Counter()
    examples = []
    for first, second in zip(vanilla, paper, strict=True):
        require((first["kind"], first["case"]) == (second["kind"], second["case"]), "backend request identity/order differ")
        if first["kind"] == "origins":
            continue
        if first != second:
            differences[first["kind"]] += 1
            if len(examples) < 40:
                examples.append({"kind": first["kind"], "case": first["case"]})
    return {"different_rows_by_kind": dict(sorted(differences.items())), "first_different_cases": examples,
            "policy": "Preserve actual Vanilla/Paper numerical drift including float-rounded RNG and End patch; no normalization of numerical results"}


def main():
    require(len(sys.argv) == 2, "one fresh output directory required")
    modes, pinned, version = configuration()
    output = Path(sys.argv[1]).resolve()
    output.mkdir(exist_ok=False)
    classes = output / "classes"
    classes.mkdir()
    compile_cp = os.pathsep.join(modes["vanilla"]["paths"])
    subprocess.run([str(JAVA_HOME / "bin/javac"), "-proc:none", "-encoding", "UTF-8", "-cp", compile_cp, "-d", str(classes), str(HELPER)], check=True)
    rows = {}
    for mode, config in modes.items():
        libraries = output / (mode + "-library-origins.json")
        libraries.write_text(json.dumps(config["library_origins"], sort_keys=True) + "\n")
        cp = os.pathsep.join([*config["paths"], str(classes)])
        with (output / (mode + ".log")).open("w") as log:
            subprocess.run([str(JAVA_HOME / "bin/java"), "-Xmx1G", "-cp", cp, "ferrumc.independent.NoiseChunkIndependentOracle",
                            mode, str(FIXTURE), str(MAPS), str(output / (mode + ".jsonl")), config["inner"], str(classes), str(libraries)],
                           cwd=output, stdout=log, stderr=subprocess.STDOUT, check=True)
        rows[mode] = [json.loads(line) for line in (output / (mode + ".jsonl")).read_text().splitlines()]
        census = collections.Counter(row["kind"] for row in rows[mode])
        require(census["constructor"] == 98, "expected 98 constructor cases")
        require(census["random"] == 14, "expected 14 actual random controls")
        require(census["preliminary"] == census["climate"] == census["diagnostic"] == 49, "49 setting/seed sessions required")
        require(all(len(row["columns"]) == 57 for row in rows[mode] if row["kind"] == "preliminary"), "25+32 preliminary columns required")
        require(all(len(row["roots"]) == 15 for row in rows[mode] if row["kind"] == "constructor"), "15 constructor router roots required")
    for path, expected in pinned.items():
        verify(path, expected, "changed input after export " + path)
    differences = compare(rows["vanilla"], rows["paper"])
    (output / "differences.json").write_text(json.dumps(differences, indent=2, sort_keys=True) + "\n")
    class_hashes = {str(path.relative_to(classes)): digest(path) for path in sorted(classes.rglob("*.class"))}
    manifest = {
        "source_pins": pinned, "jdk_version": version, "generator_sha256": digest(__file__),
        "class_hashes": class_hashes, "actual_code_sources_checked": True, "helper_last_in_classpath": True,
        "rows": {mode: {"count": len(values), "kinds": dict(sorted(collections.Counter(row["kind"] for row in values).items())),
                        "sha256": digest(output / (mode + ".jsonl"))} for mode, values in rows.items()},
        "differences_sha256": digest(output / "differences.json"),
        "existing_full_chunk_goldens_reused": {"freeze_sha256": "80e17003c7229cf769bda08a3b90094f27fc9bded97032d1991a294813f6bb20",
                                               "chunks": 126, "samples": 7077888, "bytes": 56623104},
        "scope": "Actual NoiseChunk/Random/DensityFunction numerical and trace contracts only; no Aquifer/material-block/gameplay/client/plugin acceptance",
    }
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    print(json.dumps({"rows": manifest["rows"], "different_rows_by_kind": differences["different_rows_by_kind"]}))


if __name__ == "__main__":
    main()
