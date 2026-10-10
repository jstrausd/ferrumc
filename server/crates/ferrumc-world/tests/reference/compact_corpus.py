"""Lossless visit packing for the author; raw actual-JAR exports stay authoritative."""
from pathlib import Path
import hashlib
import json
import sys


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path):
    with Path(path).open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def main():
    require(len(sys.argv) == 3, "RAW_EXPORT FRESH_OUTPUT_DIR required")
    source, output = map(lambda value: Path(value).resolve(), sys.argv[1:])
    for mode, expected in [
        ("vanilla", "074a41f57fc2f0a8e1e73f2882225f9710d422a543e52ace0d0f6a3dfec460c3"),
        ("paper", "892f86c29b5202f0e6cafd57a4d4c4b5fbe049609fd80222f1c39ea8a6ab2cc4"),
    ]:
        require(digest(source / (mode + ".jsonl")) == expected, "raw actual-JAR corpus hash mismatch " + mode)
    output.mkdir(exist_ok=False)
    classes, ids = [], {}

    def class_id(name):
        if name not in ids:
            ids[name] = len(classes)
            classes.append(name)
        return ids[name]

    for mode in ["paper", "vanilla"]:
        for line in (source / (mode + ".jsonl")).open():
            row = json.loads(line)
            if row["kind"] == "constructor":
                for visit in row["second_wrap_visits"]:
                    class_id(visit["case"])
                    class_id(visit["value_class"])
    counts = {}
    for mode in ["paper", "vanilla"]:
        count = 0
        with (output / (mode + "-constructors.jsonl")).open("w") as constructors, (output / (mode + "-contracts.jsonl")).open("w") as contracts:
            for line in (source / (mode + ".jsonl")).open():
                row = json.loads(line)
                if row["kind"] != "constructor":
                    contracts.write(line)
                    continue
                original = row.pop("second_wrap_visits")
                packed = [[class_id(v["case"]), v["equal_partition"], v["key_identity"], v["value_identity"], class_id(v["value_class"])] for v in original]
                restored = [
                    {"kind": "visit", "case": classes[v[0]], "equal_partition": v[1], "key_identity": v[2], "value_identity": v[3], "value_class": classes[v[4]]}
                    for v in packed
                ]
                require(restored == original, "lossy constructor visit packing")
                row["visits"] = packed
                constructors.write(json.dumps(row, separators=(",", ":")) + "\n")
                count += 1
        require(count == 98, "98 constructors per backend required")
        counts[mode] = count
    (output / "classes.json").write_text(json.dumps(classes, separators=(",", ":")) + "\n")
    files = {path.name: {"bytes": path.stat().st_size, "sha256": digest(path)} for path in sorted(output.iterdir()) if path.is_file()}
    manifest = {
        "format": "visit tuple [keyclassid,equal_partition,key_identity,value_identity,valueclassid]; classes.json indexes",
        "counts": counts, "classes": len(classes), "files": files, "lossless_roundtrip_verified": True,
        "raw": {mode: digest(source / (mode + ".jsonl")) for mode in ["paper", "vanilla"]},
        "generator_sha256": digest(__file__),
        "identity_policy": "Java identity IDs are diagnostics; Rust must preserve mutable wrapper sharing/registration. Immutable Java record identities are not a Rust identity claim.",
    }
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    print(json.dumps(files))


if __name__ == "__main__":
    main()
