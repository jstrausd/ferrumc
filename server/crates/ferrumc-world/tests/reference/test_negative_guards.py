"""Exercise concrete refusal paths, with no mutation of shared reference inputs."""
from pathlib import Path
import json
import os
import subprocess
import sys
import zipfile
import run_oracle as runner


def refuses(label, action, expected, results):
    try:
        action()
    except ValueError as error:
        runner.require(expected in str(error), "wrong refusal for " + label + ": " + str(error))
        results.append({"case": label, "refused": str(error)})
    else:
        raise ValueError("guard accepted " + label)


def main():
    runner.require(len(sys.argv) == 3, "VALID_EXPORT FRESH_TEST_DIR required")
    export, output = map(lambda value: Path(value).resolve(), sys.argv[1:])
    output.mkdir(exist_ok=False)
    modes, _, _ = runner.configuration()
    results = []
    for key in runner.ENV:
        environment = {key: "-javaagent:untrusted.jar"}
        refuses("env-" + key, lambda: runner.reject_env(environment), key, results)
    altered = output / "altered.bin"
    altered.write_bytes(b"intentionally different reference bytes\n")
    for label in ("helper", "mapping", "source_pins", "root_inputs", "fixture", "vanilla_inner", "paper_inner", "vanilla_bundle", "paper_bundle"):
        refuses("hash-" + label, lambda: runner.verify(altered, runner.PIN[label], label), label + " hash mismatch", results)
    for mode, config in modes.items():
        library = config["paths"][1]
        expected = runner.digest(library)
        refuses("hash-library-" + mode, lambda: runner.verify(altered, expected, "library"), "library hash mismatch", results)
    for relative, expected in runner.JDK_PINS.items():
        refuses("hash-jdk-" + relative, lambda: runner.verify(altered, expected, relative), relative + " hash mismatch", results)
    for member in ("NoiseChunkIndependentOracle.class", "NoiseChunkIndependentOracle$Probe.class"):
        shadow = output / member.replace("$", "-")
        with zipfile.ZipFile(shadow, "w") as archive:
            archive.writestr("ferrumc/independent/" + member, b"shadow bytes")
        refuses("shadow-" + member, lambda: runner.reject_shadow([shadow]), "helper classpath shadow", results)
    classes = export / "classes"
    runner.require(classes.is_dir(), "fresh actual helper classfiles missing")
    wrong_classes = output / "wrong-classes"
    wrong_classes.mkdir()
    for mode, config in modes.items():
        cp = os.pathsep.join([*config["paths"], str(classes)])
        for label in ("inner-origin", "helper-origin", "library-origin"):
            libraries = output / (mode + "-" + label + "-libraries.json")
            origins = dict(config["library_origins"])
            if label == "library-origin":
                origins["com.google.gson.JsonParser"] = config["inner"]
            libraries.write_text(json.dumps(origins, sort_keys=True) + "\n")
            expected_inner = str(wrong_classes) if label == "inner-origin" else config["inner"]
            expected_classes = str(wrong_classes) if label == "helper-origin" else str(classes)
            target = output / (mode + "-" + label + ".jsonl")
            process = subprocess.run([
                str(runner.JAVA_HOME / "bin/java"), "-Xmx256M", "-cp", cp,
                "ferrumc.independent.NoiseChunkIndependentOracle", mode, str(runner.FIXTURE), str(runner.MAPS),
                str(target), expected_inner, expected_classes, str(libraries),
            ], capture_output=True, text=True, cwd=output)
            log = process.stdout + process.stderr
            runner.require(process.returncode != 0 and "wrong actual CodeSource" in log, "actual JVM origin guard accepted " + mode + "/" + label)
            runner.require(not target.exists(), "actual origin refusal published a fixture")
            (output / (mode + "-" + label + ".log")).write_text(log)
            results.append({"case": mode + "-" + label, "refused": "wrong actual CodeSource", "output_absent": True})
    (output / "results.json").write_text(json.dumps({"cases": results, "count": len(results), "python_optimized": not __debug__}, indent=2, sort_keys=True) + "\n")
    print(json.dumps({"guards_passed": len(results), "python_optimized": not __debug__}))


if __name__ == "__main__":
    main()
