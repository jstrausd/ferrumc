"""Generate explicit input probes; expected values come only from original calls."""
from pathlib import Path
import hashlib
import itertools
import json
import struct

DIRECTORY = Path(__file__).resolve().parent
ROOT = next(p for p in DIRECTORY.parents if (p / "AGENTS.md").is_file())
SEEDS = [0, 1, -1, -(1 << 63), (1 << 63) - 1, 0x123456789ABCDEF, -0x123456789ABCDEF]


def wrap32(value):
    return ((value + (1 << 31)) % (1 << 32)) - (1 << 31)


def zoom_seed(seed):
    return int.from_bytes(hashlib.sha256(seed.to_bytes(8, "little", signed=True)).digest()[:8], "little", signed=True)


def bits(value):
    return struct.pack(">d", value).hex()


def main():
    rows = []
    groups = {}

    def add(kind, label, *fields):
        rows.append("\t".join(map(str, [kind, label, *fields])))

    for i, seed in enumerate(SEEDS + [-(1 << 63) + 1, (1 << 63) - 2, -2, 2]):
        add("H", f"hash-{i}", seed)
    for sign in ["positive", "negative"]:
        for bucket in range(1024):
            value = bucket << 24
            if sign == "negative":
                value -= 1 << 34
            add("F", f"bucket-{sign}-{bucket}", value)
    for i, (left, right) in enumerate(itertools.product(SEEDS, repeat=2)):
        add("L", f"mix-{i}", left, right)

    def selection(group, kind, seed, position):
        index = groups.get(group, 0)
        groups[group] = index + 1
        label = f"{group}-{index}"
        add("G", label, kind, seed, *position)
        actual_seed = zoom_seed(seed) if kind == "world" else seed
        shifted = [wrap32(axis - 2) for axis in position]
        base = [axis >> 2 for axis in shifted]
        fractions = [(axis & 3) / 4.0 for axis in shifted]
        for corner in range(8):
            upper = [bool(corner & mask) for mask in [4, 2, 1]]
            quart = [base[i] + int(upper[i]) for i in range(3)]
            offset = [fractions[i] - float(upper[i]) for i in range(3)]
            add("D", f"{label}.corner-{corner}", actual_seed, *quart, *map(bits, offset))

    anchors = [(0, 0, 0), (-4, -4, -4), (-16, -64, 16), (16, 64, -16),
               (29999996, 0, -30000000), (-30000000, 316, 29999996),
               (2147483644, -2147483648, 2147483644), (-2147483648, 2147483644, -2147483648)]
    residues = list(itertools.product(range(4), repeat=3))
    for seed in SEEDS:
        for anchor in anchors:
            for residue in residues:
                selection("world-residue", "world", seed, [a + r for a, r in zip(anchor, residue, strict=True)])
        for anchor in anchors[:2]:
            for residue in residues:
                selection("zoom-residue", "zoom", seed, [a + r for a, r in zip(anchor, residue, strict=True)])
    seams = [-2147483648, -2147483647, -2147483646, -2147483645, -2147483644,
             -30000001, -30000000, -29999999, -65, -64, -63, -17, -16, -15,
             -5, -4, -3, -2, -1, 0, 1, 2, 3, 4, 5, 15, 16, 17, 63, 64, 65,
             29999999, 30000000, 30000001, 2147483643, 2147483644, 2147483645, 2147483646, 2147483647]
    for seed in SEEDS:
        for axis in range(3):
            for edge in seams:
                point = [5, -7, 11]
                point[axis] = edge
                selection("axis-seam", "world", seed, point)
    for seed in SEEDS:
        for point in itertools.product([-2147483648, -2147483647, -2147483646,
                                         2147483645, 2147483646, 2147483647], repeat=3):
            selection("wrap-corner", "world", seed, point)
    # One bounded search candidate. These derived high-bit seeds are explicit
    # new probes, not claimed as equal-minimum fixtures until original export.
    for seed in [268435456, 268435456 + (1 << 34), 268435456 - (1 << 34), -(1 << 63) + 268435456]:
        selection("tie-candidate", "zoom", seed, [134217733, -134217723, 134217733])

    request = ROOT / "server/crates/ferrumc-world/tests/fixtures/biome-zoom-original-1.21.8/requests.tsv"
    request.write_text("\n".join(rows) + "\n")
    labels = [r.split("\t")[1] for r in rows]
    if len(labels) != len(set(labels)) or len(rows) >= 100000:
        raise ValueError("invalid bounded case corpus")
    counts = {kind: sum(row.startswith(kind + "\t") for row in rows) for kind in "HFLGD"}
    meta = {"scope": "Explicit probe inputs only; no original expected output computed here",
            "counts": counts, "selection_groups": groups, "world_seeds": SEEDS,
            "request_sha256": hashlib.sha256(request.read_bytes()).hexdigest(),
            "rows": len(rows), "all_residue_combinations": len(residues),
            "independent_hashlib_used_only_for_distance_probe_seed_inputs": True,
            "exploration": {"initial_candidates": 513216, "random_candidates": 23887872,
                            "structured_candidates": 8192000, "structured_equal_minimum_candidates": 1,
                            "original_getBiome_invoked_in_exploration": False}}
    (DIRECTORY / "case-contract.json").write_text(json.dumps(meta, sort_keys=True, indent=2) + "\n")
    print(json.dumps(meta, sort_keys=True))


if __name__ == "__main__":
    main()
