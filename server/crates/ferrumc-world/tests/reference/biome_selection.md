# GEN-05.5 original biome selection and full arrays

Reference: Minecraft1.21.8/772/4440, PaperBuild60 at
`29c8822d90899c89d2689338e81a98f690bcba12`. This is a pure compute consumer.
The active App still uses FerrumC flat and Plains; no natural-world, wire,
storage, survival or plugin activation follows from these tests.

The immutable tree/scalar evidence is adopted unchanged from the independent
Root freeze: complete Overworld9112nodes/7593leaves/depth6 and Nether6nodes/
5leaves/depth2,3279 builtin cold/forward/reverse calls plus8 declared two-point
tie controls. Rust exhaustively checks every node box, child order and leaf
ordinal. All distance longs wrap exactly as Java; strict retained-history ties
are not replaced by linear search. History belongs to explicitly owned sessions.

`BiomeArrayOracle.java` freshly invokes actual original version detection,
registry bootstrap, VanillaRegistries.createLookup, actual WorldPreset/LevelStem
bindings, original ProtoChunk, RandomState, NoiseBasedChunkGenerator.createBiomes
and LevelChunkSection getters. The original StructureManager and Beardifier
execute their genuine methods. A fail-closed LevelAccessor permits only the
exact candidate chunk at STRUCTURE_REFERENCES, with its actual empty reference
map; every other method throws. No copied fill loop, selector or noise formula
supplies expected results. ProtoChunk.getNoiseBiome requires a separately
advanced BIOMES status, so the helper reads genuine section getters after join.

The tracing source delegates once to the independently checked original source;
root wrappers delegate each actual compute once and retain coordinates/f64 bits.
Original tree leaf identity is mapped to the independently frozen whole-tree
ordinal after checking every actual box/child/value against that evidence.
Twenty-four full unwrapped actual-generator baselines agree with instrumented
output. Original same-thread ThreadLocal history is reflected at every query and
array boundary; cold uses remove. This does not invent a FerrumC scheduler policy.

The156-array corpus contains88 multinoise arrays (64 cold from four true builtin
bindings,4 full signed seeds,4 chunks, plus16 forward/reverse warm arrays and8 warm arrays sharing original parameter-list/RTree objects across normal/large_biomes/amplified settings),64 End
arrays at center/radius/outside/negative/distant/±30M admitted edge chunks, and4
single_biome_surface fixed-Plains arrays. Original full accessor heights are
-64/384 for Overworld and0/256 for Nether/End, independent of0/128 noise geometry.
All five End resources occur.27 additional actual End source calls use original
constant density factories at the exact three f64 thresholds and their adjacent
representable values, at radius64 and positive/negative outside coordinates.

The complete six-root original trace export is45486680bytes and remains hashed in the
review archive. The checked compact9.73MB projection retains every section
resource, every original traversal coordinate, every multinoise ordinal, every
End transformed f64 call, and first/final density-call traces. It is generated
only after the complete trace is checked against actual output and order;
it contains no substituted selection result. Whole-array Rust instrumentation
checks all coordinates, stored resources and actual selection ordinals. Separate
End checks compare every transformed center and erosion f64 bit. Fixtures:

- `arrays-compact.jsonl`: SHA256
  `33c3cec9c0937b659cddd026fe89cbbabe4e91abc6095c48f4a9b6adfd8caa78`.
- `end-probes.jsonl`: SHA256
  `2c108a34f32c723cd8023f8aa8771b9dc4d622311352333400d29936f85f2167`.
- Complete original export: SHA256
  `81a50ffc2fb92ebdc695e671ecf493c72564e4cc09c16e259f99e895f19fe1ef`.

Run from the repository root with two fresh output paths:

```sh
python3 server/crates/ferrumc-world/tests/reference/biome-selection/run_arrays.py .codex-tmp/fresh-biome-arrays-normal
python3 -O server/crates/ferrumc-world/tests/reference/biome-selection/run_arrays.py .codex-tmp/fresh-biome-arrays-optimized
```

The runner uses fixed50 repo source/evidence/helper pins,3 exact JDK pins,
original inner and119 library pins, pre/post hash checks, canonical helper-source
admission, class-shadow refusal and five Java-option/classpath environment guards.
Original45 class origins,4 helper origins and4 library origins are verified by
actual CodeSources. `require` remains active under Python-O. A separate guard
receipt distinguishes real environment rejection from declared hash/namelist
fault injection; no original artifact was modified to test rejection.

Runtime construction plans exact bucket counts before fallible allocation.
Bounds are8192entries/10000nodes/9999edges/depth8/2MiB retained/1MiB simultaneous
scratch; build allowance1..67108864. Every original search and array operation
shares one cumulative source allowance1..268435456. Quart output is<=2048 values
and<=8192 simultaneous native staged/final bytes. Source fixed/End construction
and all source resets allocate nothing; sampling allocates no per-quart data.
Native capacity statistics exclude allocator metadata, graph sharing and RSS.

Admission failures preserve live work and authority. Executed allocation/work/
noise/selection failure drops partial arrays, retains pre-array history and
poisons source plus noise. Reset must be explicit for both. Successful biome
execution retains material Fresh authority. All-source N/N-1, shared cumulative
meter, transfer of retained history, poison/reset isolation, full-height/clamping,
maximum output, negative floors and transformed bounds have meaningful tests.
Unknown/custom parameter or datapack source construction, clamped noise settings,
surface/carvers/features/structures and full source scheduling remain outside
this closed builtin consumer.
