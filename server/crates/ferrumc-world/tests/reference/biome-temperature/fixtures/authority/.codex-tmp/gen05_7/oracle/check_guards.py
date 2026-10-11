"""Explicit require-based refusal series; genuine originals stay unmodified."""
from pathlib import Path
import importlib.util
import json
import os
import subprocess
import sys
import zipfile

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("temperature_original_runner", HERE / "run_oracle.py")
if spec is None or spec.loader is None:
    raise RuntimeError("Canonical runner module missing")
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


def rejected(action, fragment):
    try:
        action()
    except runner.GuardFailure as error:
        runner.require(fragment in str(error), "Wrong refusal reason " + str(error))
        return str(error)
    raise RuntimeError("Guard unexpectedly accepted")


def main():
    runner.require(len(sys.argv) == 3, "Fresh guard output and accepted original output required")
    output, accepted = Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve()
    runner.fresh_guard(output)
    output.mkdir()
    pins = json.loads((HERE / "input-pins.json").read_text())
    runner.inventory_guard(pins["inventory"])
    environment = dict(os.environ)
    for name in runner.JAVA_ENV:
        environment.pop(name, None)
    options = ["-O"] if sys.flags.optimize else []
    receipts = []
    for name in runner.JAVA_ENV:
        env = dict(environment)
        env[name] = "-Dtemperature.guard.refuse=true" if name != "CLASSPATH" else str(output / "untrusted")
        target = output / name
        result = subprocess.run([sys.executable, *options, str(HERE / "run_oracle.py"), str(target)], env=env, capture_output=True, text=True)
        runner.require(result.returncode != 0 and "Java environment injection " + name in result.stderr and not target.exists(), "Environment guard accepted " + name)
        receipts.append({"case": name, "kind": "actual subprocess environment rejection", "refused": True, "output_created": False})
    receipts.append({"case": "helper-source", "kind": "canonical path rejection", "reason": rejected(lambda: runner.canonical_guard(output / "TemperatureOracle.java"), "Noncanonical"), "refused": True})
    receipts.append({"case": "nonfresh-output", "kind": "actual existing directory rejection", "reason": rejected(lambda: runner.fresh_guard(output), "fresh"), "refused": True})
    inner = runner.original(pins, "paper")
    receipts.append({"case": "declared-digest", "kind": "declared inventory digest fault injection; actual original unchanged", "reason": rejected(lambda: runner.inventory_guard({inner:"0" * 64}), "Pinned input mismatch"), "refused": True})
    with zipfile.ZipFile(inner) as jar:
        names = jar.namelist()
    receipts.append({"case": "declared-package-shadow", "kind": "declared ZIP-name fault injection; original ZIP unchanged", "reason": rejected(lambda: runner.shadow_guard(names + [runner.HELPER_PREFIX + "Injected.class"], inner), "Helper package shadow"), "refused": True})
    classes = accepted / "classes"
    runner.require(classes.is_dir(), "Accepted helper classes missing")
    guava = next(path for path in pins["runtime_inputs"]["paper"] if "/guava/guava/" in path)
    target = output / "wrong-origin"
    target.mkdir()
    with (output / "wrong-origin.log").open("w") as log:
        result = subprocess.run(runner.command(pins, "paper", classes, target, expected_inner=guava), env=environment, stdout=log, stderr=subprocess.STDOUT)
    runner.require(result.returncode != 0 and "Wrong CodeSource" in (output / "wrong-origin.log").read_text(), "Actual original origin guard accepted")
    runner.require(not list(target.iterdir()), "Origin rejection emitted samples")
    receipts.append({"case": "actual-original-codesource", "kind": "genuine helper rejects wrong declared expected inner; original unchanged", "refused": True, "samples_emitted": False})
    for label in ["helper", "library"]:
        target = output / ("wrong-" + label + "-origin")
        target.mkdir()
        call = runner.command(pins, "paper", classes, target)
        if label == "helper":
            call[8] = guava
        else:
            declared = (HERE / "paper-library-origins.properties").read_text().splitlines()
            declared = ["com.mojang.serialization.Codec=" + inner if r.startswith("com.mojang.serialization.Codec=") else r for r in declared]
            property_file = output / "wrong-library-origins.properties"
            property_file.write_text("\n".join(declared) + "\n")
            call[13] = str(property_file)
        log_path = output / ("wrong-" + label + "-origin.log")
        with log_path.open("w") as log:
            result = subprocess.run(call, env=environment, stdout=log, stderr=subprocess.STDOUT)
        runner.require(result.returncode != 0 and "Wrong CodeSource" in log_path.read_text() and not list(target.iterdir()), "Actual " + label + " origin guard accepted")
        receipts.append({"case": "actual-" + label + "-codesource", "kind": "genuine helper refuses wrong declared expected origin; actual inputs unchanged", "refused": True, "samples_emitted": False})
    runner.require(len(receipts) == 12, "Incomplete guard series")
    runner.inventory_guard(pins["inventory"])
    receipt = {"scope": "Explicit independent guard receipts, original artifacts read-only", "python_optimize": sys.flags.optimize,
               "pins_sha256": runner.digest(HERE / "input-pins.json"), "cases": receipts, "all_actual_inputs_unchanged": True}
    (output / "guards.json").write_text(json.dumps(receipt, sort_keys=True, indent=2) + "\n")
    print(json.dumps({"guards": len(receipts), "python_optimize": sys.flags.optimize}))


if __name__ == "__main__":
    main()
