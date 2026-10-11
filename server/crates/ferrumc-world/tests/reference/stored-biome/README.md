# Stored biome resolver reference (GEN-05.8)

The approved oracle lives unchanged in `.codex-tmp/gen05_8/`. Its whole-file
freeze manifest is `delivery/freeze-manifest.json` with SHA-256
`d838de6c3af6d512d9b47741d8ed2be8b476337885bb0487d1a51435ea296cee`. Root
reproduced it and independently returned CLEAN_CONTRACT. This crate does not
re-execute Java and does not invent a new receipt.

## Original provenance

The original Vanilla 1.21.8 obfuscated JAR and the Paper 1.21.8 build 60 JAR
ran the frozen helper `StoredBiomeOracle.java`. Vanilla ran with
`-Dmax.bg.threads=1`, so its `createBiomes` executed on a single background
worker. The shared forward and reverse Vanilla histories are therefore
single-worker sequences, not production multi-worker scheduling. Paper ran
`createBiomes` on the calling thread.

Vanilla and Paper produced byte-identical query, array and history
projections in both normal and Python `-O` mode. Only 21 diagnostic exception
rows differ, and all of them lie outside the admitted `BiomeQuartGeometry`.

## Compact fixture (`tests/fixtures/stored-biome-original-1.21.8`)

`stored_biome_fixture.py generate` reads only freeze-pinned files and writes:

| File | Content |
| --- | --- |
| `queries.tsv` | All 220960 query rows: group, kind G/B/Q, seed index, input, original delegated quart, requested chunk, and the outcome `V` (resource index) or `H` (controlled hook) |
| `arrays.tsv` | All 243 original arrays (300096 values) in the original listing order |
| `histories.tsv` | All 180 genuine `createBiomes` identities with initial and final leaf ordinals |
| `manifest.json` | Roster, groups, seeds with zoom bits, source hashes, counts and the excluded exception evidence |

Selection rule: every row is included. Vanilla and Paper rows are stored once,
and Rust evaluates each row under both explicit zoom semantics. No
selected-quart, clamp, mask, section or storage-index formula supplies a
value; names are only replaced by table positions. The generator checks, as
ordering evidence, that each original listing is the lexicographic
(quart Y, X, Z) sort of the original's own quarts.

`stored_biome_fixture.py validate` checks the pins, regenerates the fixture
into a fresh directory and compares every byte. It then expands every
committed row back into the original tokens and compares them field by field
with the frozen source rows. Run both commands from the repository holding the
frozen inputs:

```sh
python3 server/crates/ferrumc-world/tests/reference/stored-biome/stored_biome_fixture.py \
  validate --repo . --fixture server/crates/ferrumc-world/tests/fixtures/stored-biome-original-1.21.8
```

## Rows that are not original-equality rows

| Rows | Treatment |
| --- | --- |
| 5568 `controlled_null_hook` rows (`H`) | FerrumC policy, never original equality. Each one observed the original's empty `LevelReader` hook outside the centre's 3×3 window. Rust checks that the original delegated quart is refused with `OutsideNeighborhood` for the original requested chunk. Public block methods apply the documented block X/Z ±30,000,000 policy first; for the 1008 `fulli32` G rows that refusal is `BlockCoordinateDomain`. |
| 32 exception rows | Excluded-geometry evidence only (before BIOMES, injected null section, unaligned and zero-height accessors). The 30 accessor rows are checked to be refused by `BiomeQuartGeometry`. |

## Allocation measurement

`stored_biome::tests::allocation::measured_build_clone_and_every_original_query`
is an ignored unit test. `measure_allocations.py` runs it in a fresh process
with the unchanged GEN-05.7 test-only preload probe
(`../biome-temperature/allocator_probe.c`). A positive 4096-byte control must
allocate before zero calls, bytes and frees are accepted for three phases:

- BUILD: the process's first admitted builds and refused builds;
- CLONE: 100 copy rounds;
- QUERY: 532576 corpus queries.

Fixture parsing and the actual GEN-05.5 array production happen before
measurement. The probe adds no unsafe Rust, production global or dependency.

```sh
python3 server/crates/ferrumc-world/tests/reference/stored-biome/measure_allocations.py \
  --binary /exact/ferrumc_world-unit-test-binary --out /fresh/report
```

Observed stack-write high water is reported separately from the typed native
scratch census (`construction_scratch_bytes`). These are observations for one
binary and toolchain. They are not RSS, TPS or latency claims.
