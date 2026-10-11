"""Predeclare input rows only; no temperature or original-output algorithm."""
from pathlib import Path
import hashlib
import itertools
import json
import sys

BASE = Path(__file__).resolve().parent
CENSUS = BASE.parent / "census/metadata.json"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_new(path, data):
    with path.open("x", encoding="utf-8", newline="\n") as output:
        output.write(data)


require(len(sys.argv) == 1, "No output/case overrides admitted")
metadata = json.loads(CENSUS.read_text())
biomes = sorted(metadata["full_biome_roster"])
require(len(biomes) == 65, "Complete builtin roster required")
MIN_I32, MAX_I32 = -(2**31), 2**31 - 1
main_xz = [(0, 0), (1, -1), (-1, 1), (15, 15), (16, 16), (-16, -16),
           (-17, 17), (17, -17), (30000000, 30000000), (-30000000, -30000000),
           (MIN_I32, 0), (MAX_I32, 0), (0, MIN_I32), (0, MAX_I32),
           (MIN_I32, MAX_I32), (MAX_I32, MIN_I32)]
main_y = [-64, -1, 0, 17, 79, 80, 81, 256, 319, 1024, MIN_I32, MAX_I32]
main_sea = [-64, 0, 32, 63, 128, MIN_I32, MAX_I32]
precision_x = [-16777218, -16777217, -16777216, -16777215,
               16777215, 16777216, 16777217, 16777218]
grid_axes = [-65, -17, -1, 0, 1, 16, 17, 65]
groups = [
    {"label": "main", "biomes": biomes, "xz": main_xz, "y": main_y,
     "sea": main_sea, "count": 87360},
    {"label": "f32-precision", "biomes": biomes,
     "xz": [(x, -x) for x in precision_x], "y": [79, 80, 81],
     "sea": [0, 63], "count": 3120},
    {"label": "frozen-grid", "biomes": ["minecraft:deep_frozen_ocean", "minecraft:frozen_ocean"],
     "xz": list(itertools.product(grid_axes, grid_axes)), "y": [-1, 79, 80, 81],
     "sea": [0, 63], "count": 1024},
    {"label": "sea-overflow", "biomes": biomes,
     "xz": [(0, 0), (30000000, -30000000)],
     "y": [MIN_I32, MIN_I32 + 16, MAX_I32 - 1, MAX_I32],
     "sea": [MAX_I32 - 18, MAX_I32 - 17, MAX_I32 - 16, MAX_I32], "count": 2080},
]

scalar_file = BASE / "scalar-inputs.tsv"
scalar_count = 0
with scalar_file.open("x", encoding="utf-8", newline="\n") as output:
    output.write("id\tbiome\tx\ty\tz\tsea\n")
    for group in groups:
        count = 0
        for biome, (x, z), y, sea in itertools.product(group["biomes"], group["xz"], group["y"], group["sea"]):
            require(all(MIN_I32 <= n <= MAX_I32 for n in [x, y, z, sea]), "i32 input overflow")
            output.write(f'{group["label"]}/{count:05d}\t{biome}\t{x}\t{y}\t{z}\t{sea}\n')
            count += 1
        require(count == group["count"], "Wrong group count " + group["label"])
        scalar_count += count
require(scalar_count == 93584, "Scalar count mismatch")

mutable_points = [
    (0, 79, 0, 63), (0, 80, 0, 63), (0, 81, 0, 63), (0, 81, 0, 0),
    (0, 4177, 0, 63), (67108864, 81, 0, 63), (0, 81, 67108864, 63),
    (-1, -64, -1, 63), (16777217, 81, -16777217, 63),
    (-16777217, 81, 16777217, 63), (MAX_I32, MAX_I32, MIN_I32, MAX_I32),
    (MIN_I32, MIN_I32, MAX_I32, MIN_I32), (17, 256, -17, 0),
    (-17, 319, 17, 128), (0, 81, 0, 63), (0, 79, 0, 63),
]
mutable_biomes = ["minecraft:plains", "minecraft:frozen_ocean", "minecraft:deep_frozen_ocean",
                  "minecraft:desert", "minecraft:snowy_taiga"]
mutable_rows = []
for biome, ordinal in itertools.product(mutable_biomes, range(len(mutable_points))):
    x, y, z, sea = mutable_points[ordinal]
    mutable_rows.append({"id": f"mutable/{biome}/{ordinal:02d}", "biome": biome,
                         "x": x, "y": y, "z": z, "sea": sea})
require(len(mutable_rows) == 80, "Mutable count mismatch")

alias_points = [
    (0, 128, 0, 63), (0, 128, 0, 0), (0, 4224, 0, 63),
    (67108864, 128, 0, 63), (0, 128, 67108864, 63), (0, 128, 0, 128),
    (-1, 128, -1, 63), (67108863, 128, -1, 63), (-1, 128, 67108863, 63),
    (-1, -3968, -1, 128), (0, 128, 0, 0), (0, 128, 0, MAX_I32),
]
fifo_points = [(x, 128, 0, 63) for x in range(1024)] + [
    (0, 128, 0, 0), (512, 128, 0, 128), (1024, 128, 0, 63),
    (512, 128, 0, 0), (0, 128, 0, 0), (1, 128, 0, 0),
    (1024, 128, 0, 128), (2, 128, 0, 128), (1025, 128, 0, 63),
    (3, 128, 0, 0), (512, 128, 0, MIN_I32), (1023, 128, 0, MAX_I32),
    (1025, 128, 0, 0), (4, 128, 0, 0), (1024, 128, 0, 0), (0, 128, 0, 63),
]
require(len(fifo_points) == 1040, "FIFO count mismatch")
cache_biomes = mutable_biomes[:3]
cache_rows = []
for biome in cache_biomes:
    for label, points in [("aliases", alias_points), ("fifo", fifo_points)]:
        for ordinal, (x, y, z, sea) in enumerate(points):
            cache_rows.append({"id": f"cache/{biome}/{label}/{ordinal:04d}", "biome": biome,
                               "sequence": label, "reset_retained_instance": ordinal == 0,
                               "x": x, "y": y, "z": z, "sea": sea})
require(len(cache_rows) == 3156, "Cache count mismatch")

contract = {
    "schema": 1,
    "scope": "Predeclared inputs/order only; no genuine temperature output or runtime acceptance yet",
    "inputs": {"metadata_sha256": digest(CENSUS), "generator_sha256": digest(Path(__file__)),
               "scalar_inputs_sha256": digest(scalar_file)},
    "scalar_groups": groups,
    "scalar_order": "group, listed biome, listed XZ, listed Y, listed sea; final axis changes fastest",
    "scalar_original_calls": ["Vanilla private getHeightAdjustedTemperature", "Paper private getHeightAdjustedTemperature",
                              "Paper public getTemperature", "Paper warmEnoughToRain", "Paper coldEnoughToSnow",
                              "Paper getPrecipitationAt", "Paper shouldMeltFrozenOceanIcebergSlightly"],
    "mutable_rows": mutable_rows,
    "mutable_policy": "One real MutableBlockPos object reused across all80 rows; mutation via original set method; immutable-original BlockPos baseline at each row",
    "cache_rows": cache_rows,
    "cache_policy": "Fresh genuine original Biome retained at each reset boundary using actual builtin immutable constructor fields; each row also uses its own cold genuine original instance; scalar/private calls never populate retained cache",
    "cache_observations": ["actual BlockPos.asLong key", "private raw f32", "retained public raw f32",
                           "cold public raw f32", "actual original FIFO key order after public invocation"],
    "noise_probe_xz": main_xz + [(x, -x) for x in precision_x] + list(itertools.product(grid_axes, grid_axes)),
    "noise_probe_calls": ["TEMPERATURE_NOISE at original f32(X)/8.0F,f32(Z)/8.0F",
                          "FROZEN_TEMPERATURE_NOISE at X*0.05,Z*0.05",
                          "BIOME_INFO_NOISE at X*0.2,Z*0.2", "BIOME_INFO_NOISE at X*0.09,Z*0.09"],
    "noise_policy": "352 direct actual getValue calls with useNoiseOffsets=false; raw f64; constructor factors, five offsets/permutations and counted RNG captured independently",
    "counts": {"scalar": scalar_count, "mutable": len(mutable_rows), "cache": len(cache_rows),
               "total_temperature_rows_per_mode": scalar_count + len(mutable_rows) + len(cache_rows),
               "direct_original_noise_samples_per_mode": 352, "builtin_metadata_records_per_mode": 65},
}
require(contract["counts"]["total_temperature_rows_per_mode"] == 96820, "Total row mismatch")
require(contract["counts"]["total_temperature_rows_per_mode"] <= 100000, "Case cap exceeded")
write_new(BASE / "cases.json", json.dumps(contract, sort_keys=True, indent=2) + "\n")
print(json.dumps({"counts": contract["counts"], "scalar_sha256": digest(scalar_file),
                  "contract_sha256": digest(BASE / "cases.json")}, sort_keys=True))
