# Fresh Paper NoiseChunk numerical contract

Minecraft Java 1.21.8; Paper Build 60, inner SHA256
`c64fb7dbc95cd15aa0b7460a5b1e117eef5ee91c4e6fa7a8f1fe6487b6c31507`;
Temurin 21.0.12.1+1. The owned `tests/fixtures/noise_chunk_1_21_8/` corpus
contains one complete Paper-density file, compact constructor visits, separate
Paper/Vanilla contracts and End seams, and the small finite-array addendum.
`manifest.json` records their exact bytes/hashes and original export provenance.
Existing public Vanilla point/noise/spline fixtures remain unchanged.

The immutable independent export freezes are
`0ece7c737a023744d6124aa5fb3cf5b7d6bf9c53f9af094cf9466ba21dcf4582`
(core, 117 files),
`80e17003c7229cf769bda08a3b90094f27fc9bded97032d1991a294813f6bb20`
(126 complete Paper chunks), and
`a111bf5647885c643e8cbdde4b950f6342430ef5191daeca6f02b25a014f163f`
(finite addendum, 42 files). These exports call actual original methods; declared
proxy leaves only supply values/arrays/errors and record calls.

Rust directly compares 126 complete chunks / 7,077,888 f64 bits; 49 decoded
constructor visitation/equality partitions, all fifteen roots, mutable wrapper
sharing and registration, and six shared Climate roots; 49 initial/bare-final/
slice/cell/corner/update/swap diagnostics; 49×57 preliminary columns; 49×22
Climate samples; 14 RNG/Improved controls; reachable bound named-noise offsets;
658 End foreign-interpolator fallbacks in both orders; all 55 wrapper controls
and 12 provider controls. The 23 array-branch family controls use seven original
finite failure prefixes and sixteen actual-JAR finite variants; nine further
original unit/gradient/weird/shared-Once/owned-direct controls remain separate.
The immutable source corpus still records Java's NaN continuation. Rust's
additional early finite refusal is tested separately; it is not trace parity
with Java's nonfinite continuation. Bound legacy temperature/vegetation/zero
special samplers are not members of RandomState's named-noise inventory; their
samples are covered by the complete and diagnostic numerical comparisons.

The compact tuple is `[keyclassid,equal_partition,key_identity,value_identity,
valueclassid]` indexed by `classes.json`. Rust compares the complete visitation
order and Java equality partition; mutable wrapper identity and registration
must agree. Immutable Java record identity IDs and all 23 independent equality
controls are retained as original diagnostics, without promising identical
Rust allocation identity. Numerical comparisons use actual Paper's widened
End radius and float-rounded nextDouble offsets. Public Vanilla consumers keep
their existing algorithms.

The runners require the locally hash-pinned original bundles, extracted
libraries, full reconstructed source/patch evidence, mappings and JDK recorded
in `.codex-tmp/gen05_2-root-paper-end/inputs.json`. These external artifacts are
not copied into the repository. The finite runner also checks its frozen base
runner/raw corpus under `.codex-tmp/gen05_3-independent-oracle/`. A missing or
changed prerequisite rejects export; no fallback numerical implementation is
provided. New output directories must not exist.

```sh
python3 server/crates/ferrumc-world/tests/reference/run_oracle.py NEW_CORE_DIRECTORY
python3 -O server/crates/ferrumc-world/tests/reference/run_oracle.py OTHER_CORE_DIRECTORY
python3 server/crates/ferrumc-world/tests/reference/run_seams.py NEW_SEAMS_DIRECTORY
python3 server/crates/ferrumc-world/tests/reference/run_finite.py NEW_FINITE_DIRECTORY
python3 server/crates/ferrumc-world/tests/reference/generate_noise_chunk_goldens.py NEW_CHUNK_DIRECTORY
python3 server/crates/ferrumc-world/tests/reference/test_negative_guards.py CORE_EXPORT NEW_GUARD_DIRECTORY
```

`compact_corpus.py RAW_CORE NEW_COMPACT_DIRECTORY` checks the frozen raw hashes
and verifies lossless visit packing. `validate_final.py EXPORT_PARENT` expects
`final-normal`, `final-optimized`, `seams-normal`, `seams-optimized`,
`compact-final-normal`, `compact-final-optimized`, `negative-normal`, and
`negative-optimized` under that parent. Normal/-O manifests are compared only
when exported from the same runner location. Owned runner relocation changes
its path/hash provenance, never original helper or fixture bytes.

Source review and actual numerical helper acceptance do not establish an
active natural generator, aquifer/material/biome/surface/carver/features,
Survival, Java-plugin compatibility, or RAM/TPS gains.
