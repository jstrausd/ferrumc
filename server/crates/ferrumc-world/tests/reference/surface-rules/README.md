# GEN-05.9 surface-rule reference

Reference material for `surface_rules`. Everything here is a byte copy of the
frozen GEN-05.9 evidence (`.codex-tmp/gen05_9`, freeze manifest SHA-256
`9f09cbd5bd191f3e173bf1843ab45fc5b5b9411de350fe45ff6a4429a8fa0e11`) or of the
runtime A5 evidence (`.codex-tmp/gen05_9-runtime/clay-oracle`). The scripts
resolve their pinned inputs relative to those areas; they are kept here for
review and reproduction, not run by `cargo test`.

| File | Origin |
| --- | --- |
| `SurfaceRulesOracle.java` | frozen original helper (`oracle/`), SHA-256 `eff09a3d…` |
| `run_oracle.py`, `check_guards.py`, `project_originals.py`, `prepare.py` | frozen runner, 16 guards (normal and `-O`), projection, input pins |
| `surface_fixture.py` | round-3 fixture generator/validator (`generate RAW OUT`, `validate RAW FIX`) |
| `a5-clay-extension/` | the Root-authorized extension (contract §13 A5, review R3-H2) |
| `measure_allocations.py` | external allocator runner for the ignored allocation test |

## Fixture

`tests/fixtures/surface-rules-original-1.21.8/` is the unchanged round-3
projection (14 files, `manifest.json` digests). It is the mechanical
projection of genuine Paper outcomes for `PaperFlatBedrock::Disabled`;
Vanilla cells are discriminating evidence only and the invalid round-1
flat-bedrock cells are not projected. `replay.tsv` declares, per row, the run
start, the replay class (`full`, `partial`, `refused`), the heights view and
the expected P4 refusal.

## A5: genuine `clay_bands_offset`

The frozen oracle never sampled `clay_bands_offset`. The extension is a copy of
the frozen helper with one added side-effect-free read
(`helper.diff`): in every observed genuine column it records
`SurfaceSystem.clayBandsOffsetNoise.getValue(x, 0.0, z)` of the group's genuine
`SurfaceSystem` into `vanilla/clay.tsv` and `paper/clay.tsv`. The runner copy
(`runner.diff`) only moves the output area, pins the extended helper and the
runner copies (`extension-pins.json`), reads every other input from the frozen
area and fixes the repository depth. Results (`compare_frozen.py`):

- normal and Python `-O` runs are byte-identical;
- all 23 data outputs equal the frozen round-2 raw outputs byte for byte; only
  the freshly compiled helper classes differ, plus the two new `clay.tsv`;
- the 16 guards refuse in both modes, and both `guards.json` equal the frozen
  round-3 ones (they differ only in `entrypoint_python_optimize`).

`clay_fixture.py generate EXT RAW OUT` projects
`tests/fixtures/surface-rules-clay-1.21.8/clay.tsv`; `validate` regenerates,
compares bytes and re-derives the column set on a second route (normal = `-O`).

- **Column set (mechanical):** the distinct `(group, ctx_x, ctx_z)` of the
  Paper G/C rows of the frozen raw `paper/rows.tsv` whose `biome_direct` is
  `minecraft:badlands`, `minecraft:eroded_badlands` or
  `minecraft:wooded_badlands`: 3393 columns.
- **Pass criterion:** in every listed column the runtime Paper-basis
  `clay_bands_offset.sample(x, 0, z)` has exactly the genuine Paper f64 bits,
  and the Vanilla basis has exactly the genuine Vanilla bits (control). Any
  deviation means A5 is not met. Genuine Paper and Vanilla bits differ in
  3393/3393 columns, so the comparison discriminates the bases.

## Allocation measurement

```text
python3 tests/reference/surface-rules/measure_allocations.py --binary <world lib test binary> --out <fresh dir>
```

The ignored test `surface_rules::tests::allocation::measured_build_and_every_fixture_evaluation`
counts compile and bind (`BUILD`), requires zero allocation calls, bytes and
frees for session construction, `begin_column`, `evaluate` and
`min_surface_level` over all 38,889 fixture rows (`EVALUATE`), and requires a
positive 4096-byte `CONTROL`. These are allocator observations for one binary;
they are not RSS, TPS or latency figures.
