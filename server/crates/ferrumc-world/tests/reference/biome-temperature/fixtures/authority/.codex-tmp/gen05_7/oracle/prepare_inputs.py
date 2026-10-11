"""Pin the exact reviewed cases and official symbol mappings; no outputs."""
from pathlib import Path
import hashlib
import json
import re
import zipfile

ROOT = Path.cwd()
HERE = ROOT / ".codex-tmp/gen05_7/oracle"
DESIGN = ROOT / ".codex-tmp/gen05_7/design"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path):
    h = hashlib.sha256()
    with Path(path).open("rb") as source:
        for block in iter(lambda: source.read(1048576), b""):
            h.update(block)
    return h.hexdigest()


def write(path, text):
    with path.open("x", encoding="utf-8", newline="\n") as target:
        target.write(text)


require(digest(ROOT / ".codex-tmp/gen05_7/design.freeze.json") == "f195f5c080335b9118083b60c3325f721b03123cd1f89f0d87f764b2f03de013", "Reviewed design pin changed")
require(digest(DESIGN / "cases.json") == "1323fbf0f7318c3db1dfe3e0e68cd445ecf5d6bd944c227ae047888b510d7b6c", "Reviewed contract changed")
require(digest(DESIGN / "scalar-inputs.tsv") == "b0a3fdb404468eb2c01764c4c249c9c9ba78279efa330739478492aee254fcdb", "Reviewed scalars changed")
contract = json.loads((DESIGN / "cases.json").read_text())
classes = {
    "SharedConstants": "net.minecraft.SharedConstants", "Bootstrap": "net.minecraft.server.Bootstrap",
    "VanillaRegistries": "net.minecraft.data.registries.VanillaRegistries", "Provider": "net.minecraft.core.HolderLookup$Provider",
    "HolderLookup": "net.minecraft.core.HolderLookup", "RegistryLookup": "net.minecraft.core.HolderLookup$RegistryLookup",
    "Reference": "net.minecraft.core.Holder$Reference", "ResourceKey": "net.minecraft.resources.ResourceKey",
    "ResourceLocation": "net.minecraft.resources.ResourceLocation", "Registries": "net.minecraft.core.registries.Registries",
    "Biome": "net.minecraft.world.level.biome.Biome", "Climate": "net.minecraft.world.level.biome.Biome$ClimateSettings",
    "Modifier": "net.minecraft.world.level.biome.Biome$TemperatureModifier", "Precipitation": "net.minecraft.world.level.biome.Biome$Precipitation",
    "Effects": "net.minecraft.world.level.biome.BiomeSpecialEffects", "Generation": "net.minecraft.world.level.biome.BiomeGenerationSettings",
    "Mobs": "net.minecraft.world.level.biome.MobSpawnSettings", "BlockPos": "net.minecraft.core.BlockPos",
    "Mutable": "net.minecraft.core.BlockPos$MutableBlockPos", "Random": "net.minecraft.util.RandomSource",
    "Legacy": "net.minecraft.world.level.levelgen.LegacyRandomSource", "Worldgen": "net.minecraft.world.level.levelgen.WorldgenRandom",
    "Perlin": "net.minecraft.world.level.levelgen.synth.PerlinSimplexNoise", "Simplex": "net.minecraft.world.level.levelgen.synth.SimplexNoise",
}
fields = {
    "SharedConstants": ["debugGenerateSquareTerrainWithoutNoise"], "Registries": ["BIOME"],
    "Biome": ["climateSettings", "specialEffects", "generationSettings", "mobSettings", "temperatureCache", "TEMPERATURE_NOISE", "FROZEN_TEMPERATURE_NOISE", "BIOME_INFO_NOISE"],
    "Legacy": ["seed"], "Perlin": ["noiseLevels", "highestFreqInputFactor", "highestFreqValueFactor"],
    "Simplex": ["p", "xo", "yo", "zo"],
}
pos = "net.minecraft.core.BlockPos"
methods = {
    "SharedConstants": [("tryDetectVersion", "void", "")], "Bootstrap": [("bootStrap", "void", "")],
    "VanillaRegistries": [("createLookup", classes["Provider"], "")],
    "Provider": [("lookupOrThrow", classes["RegistryLookup"], classes["ResourceKey"])],
    "HolderLookup": [("listElements", "java.util.stream.Stream", "")],
    "Reference": [("key", classes["ResourceKey"], ""), ("value", "java.lang.Object", "")],
    "ResourceKey": [("location", classes["ResourceLocation"], "")],
    "Biome": [("getHeightAdjustedTemperature", "float", pos + ",int"), ("getTemperature", "float", pos + ",int"),
              ("warmEnoughToRain", "boolean", pos + ",int"), ("coldEnoughToSnow", "boolean", pos + ",int"),
              ("shouldMeltFrozenOceanIcebergSlightly", "boolean", pos + ",int"),
              ("getPrecipitationAt", classes["Precipitation"], pos + ",int"),
              ("getBaseTemperature", "float", ""), ("hasPrecipitation", "boolean", "")],
    "Climate": [("temperatureModifier", classes["Modifier"], ""), ("temperature", "float", "")],
    "Modifier": [("modifyTemperature", "float", pos + ",float")],
    "Mutable": [("set", classes["Mutable"], "int,int,int")], "BlockPos": [("asLong", "long", "")],
    "Worldgen": [("getCount", "int", "")], "Perlin": [("getValue", "double", "double,double,boolean")],
}
mapping = DESIGN / "input-sources/server-mappings.txt"
class_names, members = {}, {}
owner = None
for line in mapping.read_text().splitlines():
    if line and not line[0].isspace() and not line.startswith("#"):
        require(" -> " in line and line.endswith(":"), "Unexpected mapping class grammar")
        owner, obfuscated = line[:-1].split(" -> ")
        class_names[owner] = obfuscated
        members[owner] = {}
    elif line.startswith("    "):
        definition, obfuscated = line.strip().rsplit(" -> ", 1)
        definition = re.sub(r"^(?:\d+:\d+:)+", "", definition)
        require(definition not in members[owner] or members[owner][definition] == obfuscated, "Ambiguous official mapping")
        members[owner][definition] = obfuscated
symbols = {}
for alias, canonical in classes.items():
    symbols["vanilla.class." + alias] = class_names[canonical]
    symbols["paper.class." + alias] = canonical
for alias, names in fields.items():
    for name in names:
        matches = [value for signature, value in members[classes[alias]].items() if signature.endswith(" " + name) and "(" not in signature]
        require(len(matches) == 1, "Unique mapped field required " + alias + "." + name)
        symbols["vanilla.field." + alias + "." + name] = matches[0]
        symbols["paper.field." + alias + "." + name] = name
for alias, specifications in methods.items():
    for name, result, args in specifications:
        signature = f"{result} {name}({args})"
        require(signature in members[classes[alias]], "Exact official method required " + signature)
        symbols["vanilla.method." + alias + "." + name] = members[classes[alias]][signature]
        symbols["paper.method." + alias + "." + name] = name
write(HERE / "semantic-mappings.properties", "".join(key + "=" + value + "\n" for key,value in sorted(symbols.items())))
controls = ["kind\tid\tbiome\tx\ty\tz\tsea\tsequence\treset\n"]
for row in contract["mutable_rows"]:
    controls.append("\t".join(map(str,["M",row["id"],row["biome"],row["x"],row["y"],row["z"],row["sea"],"mutable","false"])) + "\n")
for row in contract["cache_rows"]:
    controls.append("\t".join(map(str,["C",row["id"],row["biome"],row["x"],row["y"],row["z"],row["sea"],row["sequence"],str(row["reset_retained_instance"]).lower()])) + "\n")
write(HERE / "control-inputs.tsv", "".join(controls))
write(HERE / "noise-inputs.tsv", "id\tx\tz\n" + "".join(f"noise/{i:03d}\t{x}\t{z}\n" for i,(x,z) in enumerate(contract["noise_probe_xz"])))
old_pins = ROOT / "server/crates/ferrumc-world/tests/reference/biome-zoom/input-pins.json"
old = json.loads(old_pins.read_text())
library_classes = ["it.unimi.dsi.fastutil.longs.Long2FloatLinkedOpenHashMap", "it.unimi.dsi.fastutil.longs.LongCollection",
                   "it.unimi.dsi.fastutil.ints.IntRBTreeSet", "com.google.common.collect.ImmutableList",
                   "com.mojang.serialization.Codec", "com.google.gson.JsonElement", "org.slf4j.Logger"]
for mode in ["vanilla", "paper"]:
    libraries = {}
    for artifact in old["runtime_inputs"][mode]:
        with zipfile.ZipFile(artifact) as jar:
            names = set(jar.namelist())
            for name in library_classes:
                if name.replace(".", "/") + ".class" in names:
                    require(name not in libraries, "Duplicate original library class " + name)
                    libraries[name] = artifact
    require(set(libraries) == set(library_classes), "Incomplete library CodeSource inventory")
    write(HERE / (mode + "-library-origins.properties"), "".join(name + "=" + path + "\n" for name,path in sorted(libraries.items())))
inventory = dict(json.loads((DESIGN / "actual-inputs.freeze.json").read_text())["files"])
freeze = json.loads((ROOT / ".codex-tmp/gen05_7/design.freeze.json").read_text())
for relative, record in freeze["files"].items():
    require(digest(ROOT / relative) == record["sha256"], "Reviewed freeze input changed " + relative)
    inventory[str(ROOT / relative)] = record["sha256"]
for path in sorted(HERE.iterdir()):
    if path.is_file():
        inventory[str(path)] = digest(path)
inventory[str(old_pins)] = digest(old_pins)
inventory[str(ROOT / ".codex-tmp/gen05_7/design.freeze.json")] = digest(ROOT / ".codex-tmp/gen05_7/design.freeze.json")
pins = {"scope": "Genuine original oracle inputs, fixed reviewed96820-row contract; no runtime",
        "inventory": inventory, "runtime_inputs": old["runtime_inputs"], "jdk": old["jdk"],
        "case_contract_sha256": digest(DESIGN / "cases.json"), "scalar_inputs_sha256": digest(DESIGN / "scalar-inputs.tsv"),
        "symbols_sha256": digest(HERE / "semantic-mappings.properties"), "case_counts": contract["counts"]}
write(HERE / "input-pins.json", json.dumps(pins, sort_keys=True, indent=2) + "\n")
print(json.dumps({"inputs": len(inventory), "symbols": len(symbols), "pins_sha256": digest(HERE / "input-pins.json")}))
