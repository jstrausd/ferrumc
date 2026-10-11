"""Root-approved fixed complete grid; frozen before any original grid export."""
from pathlib import Path
import hashlib
import json
import shutil
import subprocess

ROOT = Path.cwd()
HERE = ROOT / ".codex-tmp/gen05_7/oracle"
BASE = HERE.parent


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path):
    h = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1048576), b""):
            h.update(block)
    return h.hexdigest()


def write_new(path, value):
    with path.open("x", encoding="utf-8", newline="\n") as output:
        output.write(value)


contract = {"schema": 1, "scope": "Separately Root-approved fixed complete Frozen grid; no output-selected coordinates; frozen before original grid invocation",
            "original_case_contract_sha256": "1323fbf0f7318c3db1dfe3e0e68cd445ecf5d6bd944c227ae047888b510d7b6c",
            "biomes": ["minecraft:deep_frozen_ocean", "minecraft:frozen_ocean"], "x_min": -64, "x_max": 63, "z_min": -64, "z_max": 63,
            "y": 79, "sea": 63, "order": "listed biome, ascending X, ascending Z; Z changes fastest", "scalar_rows": 32768,
            "direct_noise_observations_per_row": 3, "original_noise_observations": 98304,
            "combined_temperature_rows_per_mode": 129588, "combined_cap": 150000,
            "original_methods": ["private getHeightAdjustedTemperature", "modifier.modifyTemperature", "Paper public temperature/rain/cold/melt/precipitation",
                                 "actual fixed PerlinSimplex.getValue falseOffsets at X/Z*0.05, *0.2 and *0.09"],
            "extension_policy": "Keep every grid row; if inner-false still absent report gap, no further search"}
inputs = ["id\tbiome\tx\ty\tz\tsea\n"]
ordinal = 0
for biome in contract["biomes"]:
    for x in range(-64, 64):
        for z in range(-64, 64):
            inputs.append(f"frozen-full-grid/{ordinal:05d}\t{biome}\t{x}\t79\t{z}\t63\n")
            ordinal += 1
require(ordinal == 32768, "Extension count drift")
write_new(HERE / "frozen-extension-inputs.tsv", "".join(inputs))
contract["inputs_sha256"] = digest(HERE / "frozen-extension-inputs.tsv")
contract["generator_sha256"] = digest(Path(__file__))
write_new(HERE / "extension-contract.json", json.dumps(contract, sort_keys=True, indent=2) + "\n")
props = HERE / "semantic-mappings.properties"
mapping = (BASE / "design/input-sources/server-mappings.txt").read_text().splitlines()
names = [r.split(" -> ")[1][:-1] for r in mapping if r.startswith("net.minecraft.world.level.levelgen.BitRandomSource -> ")]
require(len(names) == 1, "Unique official BitRandomSource mapping required")
props.write_text(props.read_text() + "paper.class.BitRandom=net.minecraft.world.level.levelgen.BitRandomSource\nvanilla.class.BitRandom=" + names[0] + "\n")
old = json.loads((HERE / "input-pins.json").read_text())
inner = next(path for path in old["runtime_inputs"]["vanilla"] if "/versions/1.21.8/" in path)
bytecode = BASE / "bytecode-evidence"
with (bytecode / "vanilla-BitRandomSource.txt").open("x") as output:
    subprocess.run([str(Path(old["jdk"]) / "bin/javap"), "-p", "-c", "-classpath", inner, names[0]], check=True, stdout=output)
source_inputs = {}
for label, relative in [("Applied", "src/minecraft/java"), ("Base", ".gradle/caches/paperweight/mache/base/sources")]:
    source = ROOT / ".codex-tmp/paper-1.21.8-reference/paper-server" / relative / "net/minecraft/world/level/levelgen/BitRandomSource.java"
    shutil.copyfile(source, bytecode / (label + "-BitRandomSource.java"))
    source_inputs[str(source)] = digest(source)
inventory = dict(json.loads((BASE / "design/actual-inputs.freeze.json").read_text())["files"])
freeze = json.loads((BASE / "design.freeze.json").read_text())
for relative, record in freeze["files"].items():
    require(digest(ROOT / relative) == record["sha256"], "Immutable reviewed design changed")
    inventory[str(ROOT / relative)] = record["sha256"]
for directory in [HERE, bytecode, BASE / "first-oracle-source-snapshot"]:
    for path in sorted(directory.rglob("*")):
        if path.is_file() and path != HERE / "input-pins.json" and "__pycache__" not in path.parts:
            inventory[str(path)] = digest(path)
inventory.update(source_inputs)
old_registry_pins = ROOT / "server/crates/ferrumc-world/tests/reference/biome-zoom/input-pins.json"
inventory[str(old_registry_pins)] = digest(old_registry_pins)
inventory[str(BASE / "design.freeze.json")] = digest(BASE / "design.freeze.json")
pins = {"scope": "Genuine original immutable base96820 rows plus separately declared full Frozen32768-row extension; runtime readonly",
        "inventory": inventory, "runtime_inputs": old["runtime_inputs"], "jdk": old["jdk"],
        "case_contract_sha256": old["case_contract_sha256"], "scalar_inputs_sha256": old["scalar_inputs_sha256"],
        "symbols_sha256": digest(props), "case_counts": old["case_counts"], "extension_contract_sha256": digest(HERE / "extension-contract.json"),
        "extension_input_sha256": digest(HERE / "frozen-extension-inputs.tsv"), "extension_counts": contract["scalar_rows"]}
for path, expected in inventory.items():
    require(digest(Path(path)) == expected, "Actual input drift while freezing extension " + path)
(HERE / "input-pins.json").write_text(json.dumps(pins, sort_keys=True, indent=2) + "\n")
print(json.dumps({"inputs": len(inventory), "pins_sha256": digest(HERE / "input-pins.json"),
                  "extension_contract_sha256": pins["extension_contract_sha256"], "extension_inputs_sha256": pins["extension_input_sha256"]}))
