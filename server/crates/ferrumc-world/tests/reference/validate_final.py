"""Independent export consistency and concrete branch/identity evidence checks."""
from pathlib import Path
import json
import sys
import run_oracle as base
import run_seams


def rows(directory, mode):
    return [json.loads(line) for line in (directory / (mode + ".jsonl")).read_text().splitlines()]


def main():
    base.require(len(sys.argv) == 2, "fresh normal/-O export parent directory required")
    root = Path(sys.argv[1]).resolve()
    checks = []
    for prefix, files in [
        ("final", ["vanilla.jsonl", "paper.jsonl", "manifest.json", "differences.json"]),
        ("seams", ["vanilla.jsonl", "paper.jsonl", "manifest.json", "results.json"]),
        ("compact-final", ["classes.json", "paper-constructors.jsonl", "paper-contracts.jsonl", "vanilla-constructors.jsonl", "vanilla-contracts.jsonl", "manifest.json"]),
    ]:
        for name in files:
            first, second = root / (prefix + "-normal") / name, root / (prefix + "-optimized") / name
            base.require(first.read_bytes() == second.read_bytes(), prefix + " normal/-O bytes differ " + name)
            checks.append(prefix + ":byte-identical:" + name)
    for mode in ["vanilla", "paper"]:
        actual = rows(root / "final-normal", mode)
        keyed = {(row["kind"], row["case"]): row for row in actual}
        base.require(len(keyed) == len(actual), "duplicate actual case identity")
        direct = keyed["provider", "cell-direct-fill"]
        base.require([request["context"]["arrayIndex"] for request in direct["trace"]] == list(range(1, 129)), "actual direct-provider indices must be 1..128")
        counters = {request["context"]["interpolationCounter"] for request in direct["trace"]}
        base.require(len(counters) == 1, "cell direct-provider changed scalar counter")
        sentinel = keyed["wrapper", "cache2d|[1875066, 0, 1875066]"]
        base.require(sentinel["result_bits"] == "0000000000000000" and not sentinel["trace"], "actual Cache2D initial sentinel behavior differs")
        once = keyed["wrapper", "once-initial-counter-zero"]
        base.require(once["result_bits"] == "0000000000000000" and not once["trace"], "actual CacheOnce initial counter behavior differs")
        base.require(keyed["array", "cacheonce-own-cell-direct-arrayIndex-contract"]["error"] == "java.lang.ArrayIndexOutOfBoundsException", "actual CacheOnce one-past-end synthetic refusal differs")
        base.require(keyed["wrapper", "once-poisoned-scalar-hit"]["trace"] == [], "actual scalar failure timing differs")
        base.require(keyed["wrapper", "cache2d-poisoned-key-hit"]["trace"] == [], "actual cache2d failure timing differs")
        base.require(all(row["fresh_reordered_checked"] and len(row["columns"]) == 57 for row in actual if row["kind"] == "preliminary"), "preliminary matrix incomplete")
        base.require(sum(len(row["samples"]) for row in actual if row["kind"] == "climate") == 1078, "294 original plus784 added Climate cases required")
        base.require(len(keyed["origins", "after-all-actual-calls"]["code_sources"]) == 77, "actual CodeSource census changed")
        checks.append(mode + ":actual-provider-sentinel-counter-failure-matrix")
    v = {(row["kind"], row["case"]): row for row in rows(root / "final-normal", "vanilla")}
    p = {(row["kind"], row["case"]): row for row in rows(root / "final-normal", "paper")}
    for key in v:
        if key[0] == "random":
            base.require(v[key]["permutation"] == p[key]["permutation"], "RNG drift unexpectedly changes Improved permutation")
            base.require(v[key]["post_improved_next_long"] == p[key]["post_improved_next_long"], "RNG drift unexpectedly changes consume order")
            base.require(v[key]["offset_bits"] != p[key]["offset_bits"], "missing actual Improved offset drift")
    checks.append("actual-random-permutation-and-consumption-stable-offset-drift-preserved")
    negatives = []
    for mode in ["normal", "optimized"]:
        result = json.loads((root / ("negative-" + mode) / "results.json").read_text())
        base.require(result["count"] == 27 and len(result["cases"]) == 27, "27 negative guards required")
        # Only the location of our deliberately-created shadow JAR differs.
        test_directory = str(root / ("negative-" + mode))
        negatives.append(json.loads(json.dumps(result["cases"]).replace(test_directory, "$fresh-negative-dir")))
    base.require(negatives[0] == negatives[1], "normal/-O guard behavior differs")
    checks.append("27-negative-guards-normal-and-optimized")
    run_seams.verify_freeze(base.ROOT / ".codex-tmp/gen05_3-root-chunk-goldens.freeze.json", "80e17003c7229cf769bda08a3b90094f27fc9bded97032d1991a294813f6bb20")
    checks.append("existing-126-paper-chunk-freeze-unchanged")
    result = {
        "checks": checks, "passed": len(checks), "core_rows_per_backend": 474,
        "full_vanilla_paper_comparison": json.loads((root / "seams-normal/results.json").read_text()),
        "normal_optimized_all_fixture_and_manifest_bytes_equal": True,
        "compact_classes": json.loads((root / "compact-final-normal/manifest.json").read_text())["classes"],
    }
    (root / "validation.json").write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    print(json.dumps({"checks_passed": len(checks), "compact_classes": result["compact_classes"], "complete": True}))


if __name__ == "__main__":
    main()
