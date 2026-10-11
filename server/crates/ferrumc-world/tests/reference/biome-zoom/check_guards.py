"""Nine independent refusals; actual pinned original artifacts remain read-only."""
from pathlib import Path
import importlib.util
import json
import os
import subprocess
import sys
import zipfile

DIRECTORY = Path(__file__).resolve().parent
ROOT = next(p for p in DIRECTORY.parents if (p / "AGENTS.md").is_file())
SPEC = importlib.util.spec_from_file_location("zoom_original_runner", DIRECTORY / "run_oracle.py")
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("No canonical runner module")
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)


def rejected(action, reason):
    try:
        action()
    except runner.GuardFailure as error:
        runner.require(reason in str(error), "Wrong refusal: " + str(error))
        return str(error)
    raise RuntimeError("Guard unexpectedly accepted")


def main():
    runner.require(len(sys.argv) == 2, "One fresh guard directory required")
    output = Path(sys.argv[1]).resolve()
    runner.require(not output.exists(), "Guard directory must be fresh")
    output.mkdir()
    pins = json.loads((DIRECTORY / "input-pins.json").read_text())
    receipts = []
    environment = dict(os.environ)
    for name in runner.JAVA_ENV:
        environment.pop(name, None)
    options = ["-O"] if sys.flags.optimize else []
    for name in runner.JAVA_ENV:
        env = dict(environment)
        env[name] = "-Dzoom.guard.refuse=true" if name != "CLASSPATH" else str(output / "untrusted")
        target = output / name
        result = subprocess.run([sys.executable, *options, str(DIRECTORY / "run_oracle.py"), str(target)],
                                env=env, capture_output=True, text=True)
        runner.require(result.returncode != 0 and "Java environment injection " + name in result.stderr,
                       "Environment guard accepted " + name)
        runner.require(not target.exists(), "Environment guard wrote output")
        receipts.append({"case": name, "kind": "actual subprocess environment rejection", "refused": True,
                         "output_created": False})
    refusal = rejected(lambda: runner.canonical_guard(output / "ZoomOracle.java"), "Noncanonical")
    receipts.append({"case": "helper-source-path", "kind": "canonical-source guard input rejection",
                     "refused": True, "reason": refusal})
    inner, guava, dfu = runner.originals(pins, "paper")
    before = runner.digest(inner)
    refusal = rejected(lambda: runner.inventory_guard({inner: "0" * 64}), "Pinned input mismatch")
    runner.require(runner.digest(inner) == before, "Declared fault changed actual original")
    receipts.append({"case": "declared-digest", "kind": "declared inventory digest fault injection; original unmodified",
                     "refused": True, "reason": refusal})
    with zipfile.ZipFile(inner) as archive:
        names = archive.namelist()
    refusal = rejected(lambda: runner.shadow_guard(names + [runner.HELPER_PREFIX + "ZoomOracle$Injected.class"], inner),
                       "Helper class shadow")
    receipts.append({"case": "declared-class-shadow", "kind": "declared ZIP-name fault injection; original ZIP unmodified",
                     "refused": True, "reason": refusal})
    classes = ROOT / ".codex-tmp/gen05_6/final-normal/classes"
    runner.require(classes.is_dir(), "Fresh accepted helper classes required")
    paths = [p for p in pins["runtime_inputs"]["paper"] if p != inner]
    cp = os.pathsep.join([inner, *sorted(paths), str(classes)])
    target = output / "wrong-origin.tsv"
    with (output / "wrong-origin.log").open("w") as log:
        # Only the declared expected-origin argument changes. The classpath
        # still invokes the actual original class and the pinned helper.
        result = subprocess.run([str(Path(pins["jdk"]) / "bin/java"), "-Xmx128M", "-cp", cp,
                                 "ferrumc.zoomreference.ZoomOracle", "paper", str(ROOT / "server/crates/ferrumc-world/tests/fixtures/biome-zoom-original-1.21.8/requests.tsv"), str(target),
                                 guava, guava, dfu, str(classes), str(DIRECTORY / "semantic-mappings.properties")],
                                env=environment, stdout=log, stderr=subprocess.STDOUT)
    runner.require(result.returncode != 0 and "Wrong CodeSource net.minecraft.world.level.biome.BiomeManager" in
                   (output / "wrong-origin.log").read_text(), "Actual class-origin guard accepted")
    runner.require(not target.exists(), "Class-origin rejection wrote samples")
    receipts.append({"case": "actual-original-codesource", "kind": "actual original helper refusal of declared wrong expected origin",
                     "refused": True, "output_created": False})
    runner.require(len(receipts) == 9, "Missing guard cases")
    inventory = dict(pins["repo_inputs"])
    inventory.update(pins["jdk_inputs"])
    inventory.update(pins["bundle_inputs"])
    for mode in ("vanilla", "paper"):
        inventory.update(pins["runtime_inputs"][mode])
    runner.inventory_guard(inventory)
    receipt = {"scope": "Independent guard receipts; no original artifacts modified", "python_optimize": sys.flags.optimize,
               "guard_script_sha256": runner.digest(__file__), "input_pins_sha256": runner.digest(DIRECTORY / "input-pins.json"),
               "cases": receipts, "all_pinned_inputs_unchanged": True}
    (output / "guards.json").write_text(json.dumps(receipt, sort_keys=True, indent=2) + "\n")
    print(json.dumps({"guards": len(receipts), "optimization": sys.flags.optimize, "pins_unchanged": True}))


if __name__ == "__main__":
    main()
