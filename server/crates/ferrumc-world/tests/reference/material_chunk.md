# Actual Paper material reference

Minecraft Java 1.21.8, protocol 772, Paper Build60 commit
`29c8822d90899c89d2689338e81a98f690bcba12`, Java21.0.12.1+1.
This reference covers the fresh NoiseChunk material array before surface,
carvers, features, structures and light. It does not activate a natural-world
generator, dimensions, gameplay or a Java-plugin bridge.

`ChunkMaterialGoldens.java` invokes the actual original
`NoiseBasedChunkGenerator.createFluidPicker`, `NoiseChunk.getInterpolatedState`
and complete original interpolation driver. The returned aquifer/vein/default
state is read using actual `Block.getId`; the fluid flag uses actual aquifer
schedule, air and fluid-state predicates. The helper contains no aquifer,
pressure, vein, density or random expected-value formula.

The pinned eight-file Root freeze is retained as
`fixtures/material_chunk_1_21_8/root-goldens.freeze.json`. Its original SHA256 is
`bb9f9f888d11e93c17ff099ac8e5452c6d76fc9c4c2a42ba4319fdf6541b0f3f`.
The source helper remains byte-identical to that independent producer, SHA256
`8e8b2c0a2cc965cc17cc8071142fe030a972c697d99dd582cdd544d8d66c68fb`.

| Corpus | Actual coverage and pinned bytes |
| --- | --- |
| Near chunks | Seven builtin settings × two asymmetric seeds × six signed neighboring chunks =84. |
| Far End chunks | Seven signed/extreme seeds × six distant/edge chunks =42. |
| Full output | 126 chunks,7,077,888 records,35,389,440 bytes. Each record is big-endian u32 state id followed by u8 fluid flag. |
| State/flag file | SHA256 `e27a497fbbed2adb8060e3bc9556f6341a0ecb6ab5eb0ef565e70570113ed384`. |
| Ordered ranges | 32,562-byte `chunks.jsonl`, SHA256 `7782ac995e1adaa7c58c23478101521d633764babb4b5ec46e7fa4124fcd3455`. |

Output index is `((y-minY)*16+localX)*16+localZ`. Contiguous record ranges,
per-chunk content hashes, exact identities, byte counts and trailing-data
absence are checked by the exporter. Rust compares every state and every flag,
without numerical or flag normalization. The unchanged older .2/.3 numerical
fixtures remain separate.

`generate_material_goldens.py` requires the original locally extracted Paper
inner and119 libraries recorded in the frozen
`.codex-tmp/gen05_2-root-paper-end/inputs.json`, its pinned Paper bundle, the
original worldgen fixture and full reconstructed reference sources. It verifies
all26 approved Base/Applied source pins from `material-source-pins.json`.
The JDK executables/module bytes and exact version are pinned. None of these
external server/JDK artifacts is redistributed in this repository.

Before export, the runner refuses injected Java/Javac options and CLASSPATH,
changed helper/input/fixture/JAR/JDK/source bytes and any helper classpath
shadow. During export, all25 selected original class CodeSources must resolve
to the pinned Paper inner and the helper must resolve to its fresh classes
directory. Inputs are verified again after export. Each exporter output
directory must be new. Missing prerequisites fail; there is no formula fallback.

```sh
python3 server/crates/ferrumc-world/tests/reference/generate_material_goldens.py NEW_NORMAL_DIRECTORY
python3 -O server/crates/ferrumc-world/tests/reference/generate_material_goldens.py NEW_OPTIMIZED_DIRECTORY
python3 server/crates/ferrumc-world/tests/reference/test_material_negative_guards.py VALID_EXPORT NEW_GUARD_DIRECTORY
python3 -O server/crates/ferrumc-world/tests/reference/test_material_negative_guards.py VALID_EXPORT OTHER_GUARD_DIRECTORY
```

Owned normal/`-O` exports have identical rows, state/flag data, manifests and
compiled helper bytes. Their current provenance is retained in
`fixtures/material_chunk_1_21_8/owned-reexport-manifest.json`.
All19 concrete negative guards pass in both Python modes. Actual JVM checks
deliberately supply a wrong original origin or a wrong fresh helper origin and
verify that neither material file is published. Guard comparisons remove only
their fresh scratch-output directory and the Python optimization diagnostic;
no rejection class, numerical value, state, flag or request is removed.

Rust-only failure tests separately cover bounded resolver parsing, strict
unknown property/fluid refusal, preparation, output/cache admission, lazy
aquifer misses/hits, cache publication, exact shared budgets, rejection-loop
metering, distinct final state/bitmap writes, public poison and atomic reset.
These FerrumC limits/errors are additional checked behavior; Java does not
define or validate FerrumC work units. Nonfinite leaves and arithmetic fail with
raw diagnostic bits instead of continuing through Java NaN branches.

The separate independent [endpoint helper](material_branch/README.md) invokes
the original private pressure/surface/spread/fluid methods, OreVeinifier and
disabled Substance dispatch. It covers499 finite cases:69 pressure,
151 surface-level,70 random-surface,66 fluid-type,107 ore (21 actual seeded
streams) and36 disabled Substance cases. Every result/raw bit, ordered density
request/context, factory request and float draw is compared directly. The
475,091-byte frozen `branches.jsonl` has SHA256
`4ae88195e5fb1008794f0f4ac2b4ee69b6fc1a63f0902b52f00baf9ad21ff4ab`.
Original independent freeze SHA256:
`ad582f936bb33dfb507504f38b86249f979ea695ca8ab91e1d638ef038aee714`.

The independently frozen [Aquifer addendum](material_addendum/README.md)
adds42 finite cases (27 enabled Substance,6 exact center ties,6 complete
fluid scans,3 direct status-cache cases). Original `computeSubstance`,
`computeFluid`, `getAquiferStatus` and `NoiseChunk.preliminarySurfaceLevel`
alone choose branches and requests. Expected traces include all13 surface
offsets, lazy miss/hit draws10/9/10, >= ties, fourth-status flag differences,
positive-subnormal flag reset, MAX wrapping, exact point inputs and cached
repeats. Every occupied location/status array and signed-packed Preliminary
cache is compared before and after each call, alongside the full interleaving
and its density/RNG/global-picker projections. No distance ranking or
surface-height formula is supplied as an expected-value implementation.

The873,181-byte `addendum.jsonl` has SHA256
`850ccb051798c73317bd26bc22d40ee60b8c29ef6eb1c1403fd40feeeb022d87`;
original37-file independent freeze SHA256:
`3e7529d90389b90332729b67bbb45d287622ce9e22787186ba23ae3ff2c4b41c`.
Its15 further original-Java rows contain NaN/+infinity/-infinity in Substance,
Floodedness, Spread, Preliminary or Barrier. Java returns successful values;
Rust refuses the requested nonfinite input with the typed original raw bits.
These15 controls establish additional finite-only admission policy, not Java
result parity. Failed Preliminary/status values are never cached.

All541 finite endpoint cases independently pass with the exact used work
budget and refuse at N−1, retaining only an ordered request prefix. Declared
input seams are bounded (128 point overrides,16 picker overrides,12 centers,
3 ore floats) and compiled only under `cfg(test)`. Production uses concrete
global-fluid logic and positional factories without callback or trace storage.
The126-array comparison remains a separate complete-driver check. Further
seven-settings checks retain outputs after dropping routers, reset the same
session with a neighboring run in between, and reverse requests using the
same retained router.

Owned endpoint and addendum normal/`-O` exports reproduce original JSONL,
origins and all five helper classes exactly; their own manifests are identical
across Python modes and retained next to each corpus. Exporters pin helpers,
declared cases/builders, input manifests, all26 sources and the actual JDK,
refuse helper shadowing and five option/classpath injections, and verify actual
original and fresh helper CodeSources. All8 endpoint and10 addendum negative
guards pass in both modes. Only fresh output paths and explicit Python mode
diagnostics may be normalized when comparing guard evidence.

```sh
python3 server/crates/ferrumc-world/tests/reference/material_branch/run_oracle.py NEW_NORMAL_DIRECTORY
python3 -O server/crates/ferrumc-world/tests/reference/material_branch/run_oracle.py NEW_OPTIMIZED_DIRECTORY
python3 server/crates/ferrumc-world/tests/reference/material_branch/probe_guards.py VALID_EXPORT NEW_GUARD_DIRECTORY
python3 server/crates/ferrumc-world/tests/reference/material_addendum/run_addendum.py NEW_NORMAL_DIRECTORY
python3 -O server/crates/ferrumc-world/tests/reference/material_addendum/run_addendum.py NEW_OPTIMIZED_DIRECTORY
python3 server/crates/ferrumc-world/tests/reference/material_addendum/probe_guards.py VALID_EXPORT NEW_GUARD_DIRECTORY
```

Every output directory must be new and under repository `.codex-tmp`; external
reference prerequisites are the same as the complete-array runner. The source
and exact scope review still require an independent CLEAN decision. These
tested arrays and endpoints are not active-server, Survival, drop-in,
cross-platform or RAM/TPS acceptance.

The final author World checks pass in Debug and Release (205 unit plus46
integration tests in each profile), strict all-target Clippy, format checking
and Rustdoc with warnings denied. The immutable older compiler/router/chunk
fixtures and reference producers retain their prior exact bytes. Independent
source/contract review is the remaining integration gate.
