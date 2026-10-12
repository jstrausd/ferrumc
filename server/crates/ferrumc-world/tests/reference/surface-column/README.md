# GEN-05.10 surface column driver reference

Reference material for `surface_rules::column`. Everything here except this
README and `measure_allocations.py` is a byte copy of the frozen GEN-05.10
evidence (`.codex-tmp/gen05_10`, freeze manifest SHA-256
`ce9703069995f4dd947a22e61fa5d606918e9634b22478e9e0f19c824bd4887d`). The
scripts resolve their pinned inputs relative to that area; they are kept here
for review and reproduction, not run by `cargo test`.

| File | Origin |
| --- | --- |
| `SurfaceColumnOracle.java` | frozen original helper (`oracle/`), SHA-256 `f0e04e4c…` |
| `run_oracle.py`, `check_guards.py`, `project_originals.py`, `prepare.py` | frozen runner, 16 guards (normal and `-O`), contract model check, input pins (`input-pins.json` `51b52a58…`, not copied) |
| `aliases.txt`, `paper-aliases.txt`, `*-library-origins.properties`, `semantic-mappings.properties` | pinned runner inputs (`oracle/`) |
| `surface_column_fixture.py` | fixture generator/validator (`generate RAW OUT`, `validate RAW FIX`), SHA-256 `4239c968…` |
| `measure_allocations.py` | external allocator runner for the ignored allocation test (new) |

## Fixture

`tests/fixtures/surface-column-original-1.21.8/` is the unchanged projection
(8 files; `manifest.json` SHA-256 `98fcf62a…`, pinned in the tests together with
every file digest, review condition A6). Paper cells are the expectations;
Vanilla cells are discriminating evidence only. The tests classify states by
name and never read the fixture's flag columns.

Replay rule (`manifest.json` `replay`): every `full` group (70) is driven from
the genuine producers — GEN-05.4 material equal to `s0_columns_rle` and the
NOISE post-processing, GEN-05.3 corners equal to `preliminary_corners`, GEN-05.5
arrays equal to `paper_array_digests` — and compared exactly on the per-column
trace (biome, `height_plus_1`, eroded writes, `first_call_height_plus_1`,
surface depth, every call input and outcome, steep call/result/heights,
minimum surface, frozen writes), `final_states_sha256`,
`final_world_surface_wg` and the fluid flag set (= `final_post_processing`
positions). `refused:CentreOutsideDomain` expects the GEN-05.5 halo refusal
`ChunkOutOfRange` and the GEN-05.8 `CoordinateDomain`;
`refused:InconsistentResolver(Source)` expects only
`Rules(InconsistentResolver(Source))` and an `Empty` buffer. No refused
group's original value is compared.

## Allocation measurement

```text
cargo test --locked -p ferrumc-world --lib --no-run [--release]
python3 tests/reference/surface-column/measure_allocations.py --binary <unit-test binary> --out <fresh dir>
```

It preloads the unchanged GEN-05.7 probe (`../biome-temperature/allocator_probe.c`)
and requires a positive 4096-byte control and zero allocations, bytes and frees
for every `build_chunk` call (70 published and 70 refused chunks). Bind and the
buffer are counted, not claimed free. No RSS, TPS or latency claim follows.
