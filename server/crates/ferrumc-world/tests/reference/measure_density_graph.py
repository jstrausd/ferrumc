#!/usr/bin/env python3
"""Measure the private compiler layout from actual original codec projections.

One vertex per occurrence; named definitions shared; both HolderHolder origins
and raw spline-coordinate holders explicit. No factory/structural deduplication,
no numerical sampling, and no generated answer from Rust implementation data.
"""
import collections
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
FIXTURE = Path(__file__).resolve().parent.parent / "fixtures/density_compiler_1_21_8.jsonl"
MANIFEST = FIXTURE.with_suffix(".manifest.json")
raw = FIXTURE.read_bytes()
manifest = json.loads(MANIFEST.read_text())
if hashlib.sha256(raw).hexdigest() != manifest["fixture_sha256"]:
    raise ValueError("codec fixture mismatch")
records = [json.loads(line) for line in raw.splitlines()]
definitions = {row["id"]: row["projection"] for row in records if row["kind"] == "density"}
routers = {row["id"]: row["projection"]["noise_router"] for row in records if row["kind"] == "router"}
if len(definitions) != 35 or len(routers) != 7:
    raise ValueError("original registry census mismatch")
nodes, resource_roots, active_resources = [], {}, set()

def add(kind, children=()):
    index = len(nodes)
    nodes.append((kind, tuple(children)))
    return index

def resource(name):
    if name in active_resources:
        raise ValueError("joint resource/spline coordinate cycle")
    if name not in resource_roots:
        active_resources.add(name)
        resource_roots[name] = density(definitions[name])
        active_resources.remove(name)
    return resource_roots[name]

def raw_holder(value):
    target = resource(value["resource"]) if "resource" in value else density(value["inline"])
    return add("coordinate_holder", [target])

def spline(value):
    if value["kind"] == "constant":
        return add("spline_constant")
    if value["kind"] != "multipoint":
        raise ValueError("unknown original spline class")
    return add("spline_multipoint", [raw_holder(value["coordinate"])] + [spline(p["value"]) for p in value["points"]])

def density(value):
    kind = value["kind"]
    if kind == "holder":
        target = resource(value["resource"]) if "resource" in value else density(value["inline"])
        return add("holder", [target])
    if kind == "binary":
        return add(value["operation"], [density(value["argument1"]), density(value["argument2"])])
    if kind in {"mapped", "marker", "blend_density"}:
        return add(value.get("operation", kind), [density(value["argument"])])
    if kind in {"clamp", "weird_scaled_sampler"}:
        return add(kind, [density(value["input"])])
    if kind == "range_choice":
        return add(kind, [density(value[key]) for key in ("input", "when_in_range", "when_out_of_range")])
    if kind == "shifted_noise":
        return add(kind, [density(value[key]) for key in ("shift_x", "shift_y", "shift_z")])
    if kind == "spline":
        return add(kind, [spline(value["spline"])])
    if kind not in {"constant", "noise", "shift", "shift_a", "shift_b", "old_blended_noise", "end_islands", "y_clamped_gradient", "blend_alpha", "blend_offset", "beardifier"}:
        raise ValueError("unknown original density class: " + kind)
    return add(kind)

for name in sorted(definitions):
    resource(name)
router_roots = {name: [density(value) for value in fields.values()] for name, fields in sorted(routers.items())}
depths = [0] * len(nodes)
for index, (_, children) in enumerate(nodes):
    depths[index] = 1 + max((depths[child] for child in children), default=0)

def measure(roots):
    reached, pending = set(), list(roots)
    while pending:
        node = pending.pop()
        if node in reached:
            continue
        reached.add(node)
        pending.extend(nodes[node][1])
    kinds = collections.Counter(nodes[node][0] for node in reached)
    return {"vertices": len(reached), "edges": sum(len(nodes[n][1]) for n in reached),
            "combined_depth": max(depths[n] for n in reached),
            "spline_vertices": kinds["spline_constant"] + kinds["spline_multipoint"],
            "spline_points": sum(len(nodes[n][1]) - 1 for n in reached if nodes[n][0] == "spline_multipoint"),
            "markers": sum(kinds[k] for k in ("interpolated", "flat_cache", "cache_2d", "cache_once", "cache_all_in_cell"))}

output = {"codec_fixture_sha256": manifest["fixture_sha256"], "method": __doc__,
          "union": measure(range(len(nodes))), "routers": {name: measure(roots) for name, roots in router_roots.items()}}
print(json.dumps(output, indent=2, sort_keys=True))
