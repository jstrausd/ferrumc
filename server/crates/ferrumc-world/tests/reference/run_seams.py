"""Reuse 126 frozen Paper chunks; compute Vanilla comparisons without duplicate bits."""
from pathlib import Path
import collections
import json
import os
import subprocess
import sys
import zipfile
import run_oracle as base

HELPER = base.HERE / "EndAndChunkSeams.java"
HELPER_SHA = "1808cf856980d13a3da336ea7114fb987411e219ce6b11d5eef4668fa6773147"
END = base.ROOT / ".codex-tmp/gen05_2-root-paper-end"
CHUNKS = base.ROOT / ".codex-tmp/gen05_3-root-chunk-goldens-normal"
PINS = {
    str(HELPER): HELPER_SHA,
    str(END / "final.freeze.json"): "d27d4bac9ec5870eba58eabf4919cc6382a1846c928d1790b720f7d714bb10ad",
    str(END / "vanilla.jsonl"): "903e8ebd571e11da575c7b308ede8799b1dee08141db4782290717489c7c54ce",
    str(END / "paper.jsonl"): "9510370af48ac8c8bcd5ecb098360afb210ef40b72ee7f60f03ee20634667c76",
    str(CHUNKS / "chunks.jsonl"): "dc7f1a3a824f7ab39c67a86cd50811a5bda38d99d38661f11831acf383a95be2",
    str(CHUNKS / "material_density.bits"): "9dd6827609a0a6b65968b239a588fd60e21121097520501ff5828eb19655a9db",
    str(CHUNKS / "manifest.json"): "a4b29a5750ea768d5faa61fb70464d622dd708cfb164114f873eec525c2f49b6",
}


def verify_freeze(path, expected):
    base.verify(path, expected, "existing frozen reference")
    frozen = json.loads(path.read_text())
    for relative, metadata in frozen["files"].items():
        target = base.ROOT / relative
        base.verify(target, metadata["sha256"], "existing frozen file " + relative)
        base.require(target.stat().st_size == metadata["bytes"], "existing frozen file size " + relative)


def main():
    base.require(len(sys.argv) == 2, "fresh output directory required")
    modes, artifacts, version = base.configuration()
    for path, sha in PINS.items():
        base.verify(path, sha, "seam reference " + path)
    verify_freeze(base.ROOT / ".codex-tmp/gen05_3-root-chunk-goldens.freeze.json", "80e17003c7229cf769bda08a3b90094f27fc9bded97032d1991a294813f6bb20")
    output = Path(sys.argv[1]).resolve()
    output.mkdir(exist_ok=False)
    classes = output / "classes"
    classes.mkdir()
    for config in modes.values():
        for path in config["paths"]:
            with zipfile.ZipFile(path) as archive:
                base.require(not any(name.startswith("ferrumc/independent/EndAndChunkSeams") and name.endswith(".class") for name in archive.namelist()), "seam helper shadow " + path)
    artifacts.update(PINS)
    artifacts[str(Path(__file__).resolve())] = base.digest(__file__)
    artifacts[str(base.HERE / "run_oracle.py")] = base.digest(base.HERE / "run_oracle.py")
    subprocess.run([str(base.JAVA_HOME / "bin/javac"), "-proc:none", "-encoding", "UTF-8", "-cp", os.pathsep.join(modes["vanilla"]["paths"]),
                    "-d", str(classes), str(base.HELPER), str(HELPER)], check=True)
    rows = {}
    for mode, config in modes.items():
        libraries = output / (mode + "-library-origins.json")
        libraries.write_text(json.dumps(config["library_origins"], sort_keys=True) + "\n")
        with (output / (mode + ".log")).open("w") as log:
            subprocess.run([str(base.JAVA_HOME / "bin/java"), "-Xmx1G", "-cp", os.pathsep.join([*config["paths"], str(classes)]),
                            "ferrumc.independent.EndAndChunkSeams", mode, str(base.FIXTURE), str(base.MAPS), str(output / (mode + ".jsonl")),
                            config["inner"], str(classes), str(libraries), str(END / (mode + ".jsonl")),
                            str(CHUNKS / "chunks.jsonl"), str(CHUNKS / "material_density.bits")], cwd=output, stdout=log, stderr=subprocess.STDOUT, check=True)
        rows[mode] = [json.loads(line) for line in (output / (mode + ".jsonl")).read_text().splitlines()]
        census = collections.Counter(row["kind"] for row in rows[mode])
        base.require(census["end-foreign-fallback"] == 658 and census["end-far-corners"] == 42 and census["origins"] == 1, "complete End seam corpus required")
        base.require(census["full-vanilla-paper-comparison"] == (84 if mode == "vanilla" else 0), "84 Vanilla chunks and zero duplicate Paper chunks required")
    for path, sha in artifacts.items():
        base.verify(path, sha, "changed input after seam export " + path)
    paired = {}
    for mode, values in rows.items():
        paired[mode] = {(row["kind"], row["case"]): row for row in values if row["kind"] not in {"origins", "full-vanilla-paper-comparison"}}
    base.require(paired["vanilla"].keys() == paired["paper"].keys(), "End seam requests differ")
    differences = collections.Counter(key[0] for key in paired["vanilla"] if paired["vanilla"][key] != paired["paper"][key])
    chunks = [row for row in rows["vanilla"] if row["kind"] == "full-vanilla-paper-comparison"]
    result = {
        "end_seam_different_rows": dict(sorted(differences.items())),
        "complete_vanilla_chunks": len(chunks),
        "vanilla_paper_chunks_equal": sum(row["different_bits"] == 0 for row in chunks),
        "vanilla_paper_chunks_different": sum(row["different_bits"] != 0 for row in chunks),
        "compared_samples": sum(row["samples"] for row in chunks),
        "different_sample_bits": sum(row["different_bits"] for row in chunks),
        "vanilla_nonfinite": sum(row["vanilla_nonfinite"] for row in chunks),
        "full_paper_bits_reexported": False,
    }
    (output / "results.json").write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    manifest = {
        "inputs": artifacts, "jdk_version": version, "helper_sha256": HELPER_SHA,
        "classes": {str(path.relative_to(classes)): base.digest(path) for path in sorted(classes.rglob("*.class"))},
        "outputs": {mode: {"rows": len(values), "sha256": base.digest(output / (mode + ".jsonl"))} for mode, values in rows.items()},
        "results_sha256": base.digest(output / "results.json"),
        "scope": "Actual End foreign-wrapper/corner methods and 84 complete Vanilla numerical chunks compared against frozen actual-Paper bits; no Aquifer/block/biome/survival/plugin acceptance",
    }
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    print(json.dumps(result))


if __name__ == "__main__":
    main()
