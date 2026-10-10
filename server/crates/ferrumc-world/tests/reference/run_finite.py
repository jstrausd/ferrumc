"""Actual-JAR finite array addendum; immutable 117-file oracle stays untouched."""
from pathlib import Path
import hashlib
import importlib.util
import json
import os
import subprocess
import sys
import zipfile

HERE = Path(__file__).resolve().parent
ROOT = Path(__file__).resolve().parents[5]
BASE = ROOT / ".codex-tmp/gen05_3-independent-oracle"
HELPER = HERE / "FiniteNoiseChunkArrays.java"
JAVA_SHA = "0d466e3b74bcf6e0f4324eff66b36492588376716dfb574eb2e13a03368fa0c6"
BASE_RUN_SHA = "cd5d4f39d1770d2d61ce83104f925ca22d825a2f5716c543268fe26cf63f1502"
CORE_SHA = "892f86c29b5202f0e6cafd57a4d4c4b5fbe049609fd80222f1c39ea8a6ab2cc4"
REUSE = {"add|left", "mul|left", "min|left", "max|left",
         "mulOrAdd-zero|0000000000000000|true",
         "mulOrAdd-zero|8000000000000000|true", "rangeChoice|input"}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def verify(path, expected, label):
    require(digest(path) == expected, label + " hash mismatch")


def load_core():
    verify(BASE / "run_oracle.py", BASE_RUN_SHA, "frozen base runner")
    spec = importlib.util.spec_from_file_location("frozen_noise_chunk_runner", BASE / "run_oracle.py")
    module = importlib.util.module_from_spec(spec)
    sys.dont_write_bytecode = True
    spec.loader.exec_module(module)
    return module


def reject_shadow(paths):
    for path in paths:
        with zipfile.ZipFile(path) as jar:
            require(not any(n.startswith("ferrumc/independent/FiniteNoiseChunkArrays") and n.endswith(".class")
                            for n in jar.namelist()), "finite helper classpath shadow")


def main():
    require(len(sys.argv) == 2, "one fresh output directory required")
    core = load_core()
    core.reject_env(os.environ)
    verify(HELPER, JAVA_SHA, "finite helper")
    verify(BASE / "final-normal/paper.jsonl", CORE_SHA, "frozen actual Paper corpus")
    modes, pins, version = core.configuration()
    pins.update({str(HELPER): JAVA_SHA, str(BASE / "run_oracle.py"): BASE_RUN_SHA,
                 str(BASE / "final-normal/paper.jsonl"): CORE_SHA})
    for mode in modes.values():
        reject_shadow(mode["paths"])
    output = Path(sys.argv[1]).resolve()
    output.mkdir(exist_ok=False)
    classes = output / "classes"
    classes.mkdir()
    subprocess.run([str(core.JAVA_HOME / "bin/javac"), "-proc:none", "-encoding", "UTF-8",
                    "-cp", os.pathsep.join(modes["vanilla"]["paths"]), "-d", str(classes),
                    str(core.HELPER), str(HELPER)], check=True)
    original = {r["case"]: r for r in (json.loads(s) for s in (BASE / "final-normal/paper.jsonl").read_text().splitlines())
                if r["kind"] == "array"}
    rows = {}
    for name, mode in modes.items():
        libraries = output / (name + "-library-origins.json")
        libraries.write_text(json.dumps(mode["library_origins"], sort_keys=True) + "\n")
        with (output / (name + ".log")).open("w") as log:
            subprocess.run([str(core.JAVA_HOME / "bin/java"), "-Xmx512M", "-cp",
                            os.pathsep.join([*mode["paths"], str(classes)]),
                            "ferrumc.independent.FiniteNoiseChunkArrays", name, str(core.MAPS),
                            str(output / (name + "-full.jsonl")), mode["inner"], str(classes), str(libraries)],
                           cwd=output, stdout=log, stderr=subprocess.STDOUT, check=True)
        exported = [json.loads(s) for s in (output / (name + "-full.jsonl")).read_text().splitlines()]
        actual = [r for r in exported if r["kind"] == "array"]
        require(len(actual) == 23, "all 23 declared finite branch/failure controls required")
        require({r["case"] for r in actual if r["case"] in REUSE} == REUSE, "reused original finite prefix cases missing")
        for row in actual:
            if row["case"] in REUSE:
                require(row == original[row["case"]], "existing finite error prefix changed " + row["case"])
            for bits in row.get("array_bits", []):
                require(int(bits, 16) & 0x7ff0000000000000 != 0x7ff0000000000000, "unexpected nonfinite array result")
            for trace in row["trace"]:
                for bits in trace.get("bits", []):
                    require(int(bits, 16) & 0x7ff0000000000000 != 0x7ff0000000000000, "unexpected nonfinite failure prefix")
        additions = [r for r in actual if r["case"] not in REUSE]
        require(len(additions) == 16, "only the 16 missing finite variants are additions")
        (output / (name + "-finite-addendum.jsonl")).write_text(
            "".join(json.dumps(row, separators=(",", ":")) + "\n" for row in additions))
        rows[name] = additions
        sources = next(r["code_sources"] for r in exported if r["kind"] == "origins")
        require(sources.get("ferrumc.independent.FiniteNoiseChunkArrays") == "$fresh-helper-classes", "fresh addendum origin missing")
        require(sources.get("ferrumc.independent.NoiseChunkIndependentOracle") == "$fresh-helper-classes", "fresh frozen helper origin missing")
        (output / (name + "-code-sources.json")).write_text(json.dumps(sources, indent=2, sort_keys=True) + "\n")
    require(rows["vanilla"] == rows["paper"], "finite Vanilla/Paper array contracts differ")
    for path, expected in pins.items():
        verify(path, expected, "changed input after finite export")
    manifest = {
        "source_pins": pins, "generator_sha256": digest(__file__), "jdk_version": version,
        "class_hashes": {str(p.relative_to(classes)): digest(p) for p in sorted(classes.rglob("*.class"))},
        "helper_last_in_classpath": True, "actual_code_sources_checked": True,
        "declared_child_inputs": ["0000000000000000", "8000000000000000", "c008000000000000",
                                  "3fffffffffffffff", "4000000000000000", "4000000000000001",
                                  "4014000000000000", "4018000000000000"],
        "only_input_change": "last declared NaN replaced by finite 6.0; actual original methods decide every branch",
        "raw_controls_per_backend": 23, "new_rows_per_backend": 16,
        "existing_finite_failure_rows_reused": sorted(REUSE),
        "rows": {name: {"sha256": digest(output / (name + "-finite-addendum.jsonl")),
                        "full_sha256": digest(output / (name + "-full.jsonl")),
                        "bytes": (output / (name + "-finite-addendum.jsonl")).stat().st_size}
                 for name in rows},
        "scope": "Actual finite fillArray values/request/failure-prefix controls; no server/terrain/gameplay acceptance",
    }
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    print(json.dumps({"new_rows_per_backend": 16, "reused": len(REUSE), "rows": manifest["rows"]}))


if __name__ == "__main__":
    main()
