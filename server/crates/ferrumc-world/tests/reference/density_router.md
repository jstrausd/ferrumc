# Minecraft 1.21.8 seeded density-router oracle

This corpus calls the actual hash-pinned Vanilla server implementation. It
contains no copied noise, RNG, density, spline, visitor, bound or quantization
formula. Reflection supplies original objects and records their answers; child
proxies supply literal values, literal diagnostic ranges and explicit failures.
No Minecraft server, client, network, terrain generator or gameplay test runs.

## Sources and actual calls

Minecraft 1.21.8, protocol772, Java21.0.12.1+1. Source inspection uses the
complete Vanilla base and applied Paper1.21.8 Build60 at commit
`29c8822d90899c89d2689338e81a98f690bcba12`; the numerical executable here is
the official **Vanilla** inner JAR. See the manifest for all43 library digests
and [the pinned server reference](../../../../../docs/PAPER_REFERENCE.md).

The source root is
`.codex-tmp/paper-1.21.8-reference/paper-server/.gradle/caches/paperweight/mache/base/sources/`.
The applied source root is the same server checkout's `src/minecraft/java/`.
The following complete methods and their relevant callees were read, including
the real factory creation and `mapAll` paths rather than only API declarations.

| Source under that root | Symbols and concrete behavior |
| --- | --- |
| `net/minecraft/world/level/levelgen/RandomState.java:37` | Constructor, local `NoiseWiringHelper.visitNoise/wrapNew/apply`, anonymous climate visitor, retained aquifer/ore factories, `getOrCreateNoise`. The real local and anonymous classes are reflected as `eko$a(eko,long,boolean)` and `eko$1(eko)`; no visitor implementation is rewritten. |
| `net/minecraft/world/level/levelgen/NoiseRouter.java:49` | `mapAll` rebuilds each field in declared order. Actual `RandomState.router()` and its real six climate fields are projected independently. |
| `net/minecraft/world/level/levelgen/NoiseRouterData.java:66` | Complete `bootstrap`, terrain registration, cave helpers and native factories. A resource reference resolves to these native products; its target is not a fresh JSON-inline-holder decode. |
| `net/minecraft/world/level/levelgen/DensityFunction.java:15` | `HOLDER_HELPER_CODEC`, `NoiseHolder`, `SinglePointContext`, visitor contracts. The spline coordinate's raw holder is distinct from a density `HolderHolder`. |
| `net/minecraft/world/level/levelgen/DensityFunctions.java:161` | Real add/mul/min/max/range/mapped factories; `Ap2.compute/mapAll`, `TwoArgumentSimpleFunction.create`, `MulOrAdd.transform/mapAll`, `Mapped.create/transform/mapAll`, `RangeChoice.compute/mapAll`, and `Spline/Coordinate.mapAll`. Bounds and branch traces come from the real products. |
| `net/minecraft/util/CubicSpline.java:138` | `Constant.minValue/maxValue`, full `Multipoint.create/apply/mapAll`, extension and interval-search callees. Every projected Constant/Multipoint retains its actual f32 diagnostics. |
| `net/minecraft/world/level/biome/Climate.java:69` | Actual `quantizeCoord(float)`, `target`, and complete `Sampler.sample` at line475. Actual quart sampling performs the original quart-to-block conversion. |
| `net/minecraft/world/level/levelgen/synth/NormalNoise.java:38` | Constructor and complete `getValue/maxValue`; actual first/second Perlin stacks are reflected without recreating either sampler. The actual retained input-factor constant supplies boundary probe inputs. |
| `net/minecraft/world/level/levelgen/synth/PerlinNoise.java:81` | Complete constructor, `getValue`, `wrap`, `edgeValue`, octave and skip callees. Direct first/second samples receive exactly the saved input bits; per-octave wrapping is original code. |
| `net/minecraft/world/level/levelgen/Noises.java:76` | `instantiate` derives the original canonical resource name from its real holder. |
| `net/minecraft/world/level/levelgen/PositionalRandomFactory.java:20` | Actual `at` and name-derived factories; actual `RandomSource.nextLong` produces all positional draws. |
| `net/minecraft/world/level/levelgen/LegacyRandomSource.java:17` | Constructor, fork, next, and full positional factory. `BitRandomSource.nextLong` retains the signed second summand. |
| `net/minecraft/world/level/levelgen/XoroshiroRandomSource.java:16` | Constructor, fork, nextLong, and full positional factory; `RandomSupport` and `Xoroshiro128PlusPlus` callees were read. |
| `net/minecraft/util/Mth.java:70` | Complete `lfloor` and coordinate `getSeed` at line310, retaining the original integer arithmetic domains. |

The corresponding applied Paper files are byte-identical to the base for
RandomState, NoiseRouterData, NormalNoise, PerlinNoise, Climate, CubicSpline,
PositionalRandomFactory, LegacyRandomSource and XoroshiroRandomSource. The
`DensityFunctions.java.patch` was read completely: Paper widens the End radial
distance and adds its End cache. This corpus deliberately retains Vanilla's
overflow/NaN behavior. Root's separate actual Paper-JAR End probe establishes
that difference; this fixture does not claim Paper End-terrain parity.

Registry objects come from real `SharedConstants`, `Bootstrap` and
`VanillaRegistries.createLookup`. Settings and the original named definitions
are decoded through their real `RegistryOps` codecs from the pinned source
fixture. `projection.named` therefore means the actual direct-codec product,
whose resource references already resolve to native registered graphs.
`projection.registered_named` separately contains the real registered roots:
`jm$a.b(mn.aS)` obtains the density getter, `amd.a(mn.aS,ame.a(name))` constructs
the actual key, `jm.b(key)` obtains its real holder and `jl.a()` its value.
Both receive the same real wiring visitor. Their structural differences remain
visible; neither view is substituted for the other.

## Census and grammar

The fixture contains **17,213 records /47,371,851 bytes**:

| Record | Count | Evidence |
| --- | ---: | --- |
| `binding` | 49 | Seven settings ×seven signed seeds; field/named/climate bounds, aquifer/ore draws and all60 original bound-noise maxima. |
| `projection` | 7 | Seed0; all15 field roots,35 direct-codec named roots,35 native registered roots and six climate roots. |
| `point` | 1,813 | 90,650 direct f64 density values, plus10,878 climate f32/quantized pairs. Negative coordinates, seams, geometry endpoints, ±30M and i32-Y extrema. |
| `quart` | 294 | 1,764 climate f32/quantized pairs from the actual quart sampler. |
| `wrapped_noise` | 14,700 | 11,760 canonical and2,940 alternate-name actual Normal/first-Perlin/second-Perlin samples. |
| `quantize` | 42 | ±0, both NaN signs/payloads, ±infinity, subnormal/largest-finite values, adjacent f32 truncation/rounding and i64 saturation boundaries. |
| `trace` | 308 | 77 cases ×four views: original factory, identity `mapAll`, real wiring visitor, and real wiring followed by the real climate visitor. |

The projected trees contain53,628 spline occurrences, each with actual8-hex
`min_value_bits` and `max_value_bits`; these are repeated tree occurrences,
not unique runtime nodes. Density diagnostics retain16-hex f64 bits. All
245 registered roots remain recursively projected. Resource identities and
the actual transformed raw spline-coordinate holder views remain intact.

Each binding has `aquifer_draws` and `ore_draws`, each seven objects with
`position:[x,y,z]` and `next_longs:[signed_i64,signed_i64,signed_i64]`:
686 factory positions /2,058 longs in total. Positions cover origin, negative
coordinates, seams, mixed world borders and both i32-Y extrema. The helper
repeats requests in reverse order against the same retained factories and
requires equality. `noise_max_values` maps all60 canonical noise resources to
actual16-hex `NormalNoise.maxValue` bits:2,940 original diagnostic pins. These
pins do not imply that60 Rust gameplay consumers are active.

`quantize` records contain `case`, `input_bits` (8hex) and signed i64
`quantized`; the result is the actual `Climate.quantizeCoord` call.

`wrapped_noise` contains `case`, `settings`, signed `seed`, canonical `noise`,
`stack` (`normal|first|second`), `name_mode` (`canonical|shortname|slot`),
`input_bits` (three16-hex f64 values), `result_bits`, and `max_value_bits`.
Canonical samples use the actual `NoiseWiringHelper.visitNoise`, including
legacy temperature/vegetation and the legacy offset-zero branch. Alternate
controls use the actual retained positional factory's `fromHashOf(String)`
and actual modern `NormalNoise.create` with the registered resource parameters;
the name is the short resource name or `slot_0..4`. In legacy special cases
these controls intentionally also omit the special constructor. For ordinary
modern bindings they differ only in the derived name. Every mixed-coordinate
Normal control differs from its canonical sample:245/245 for each alternate
name mode. All sampler requests are repeated in reverse order and checked.

The16 canonical inputs include signed zero, mixed coordinates, ±45-billion
jagged inputs, ±wrap-halfperiod and adjacent f64 values, multiple periods and
the Normal second-stack scale boundary with adjacent inputs. A direct `second`
record samples its Perlin stack at exactly the recorded coordinates; only the
`normal` record performs Normal's internal second-stack scaling.

`trace` contains a unique `case` suffix identifying `view`, `operation`
(`add|mul|min|max|range_choice|mapped`), actual `factory`
(`binary|mul_or_add|range_choice|mapped`), actual `factory_before/after`
obfuscated class names, and actual16-hex `min_value_bits/max_value_bits`.
Applicable operands retain `left_bits/right_bits/left_bounds/right_bounds`,
`input_bits/input_bounds`, `argument_bits`, `min_bits/max_bits`,
`inside_bits/outside_bits/inside_bounds/outside_bounds` and, for mapped
factories, `mapped_operation`. `trace` lists only requested children.
Successful records have `result_bits`. A configured `throw_child` can be
skipped successfully; only an actual throw adds
`error:"original_probe_failure"` and omits the result. Each view contains54
successful and23 throwing cases. MIN/MAX equality requests the right child;
only the strict adjacent side skips it. Ap2 ±0 skips right and produces +0,
while zero-factor MulOrAdd still requests its input. Source failures stop
before later children. Literal NaN/infinity/reversed diagnostics remain visible.

Spline endpoint/equality/right-interval and low-before-high request traces are
also established by the unchanged actual-JAR CubicSpline corpus and helper
`CubicSplineOracle.java`; this oracle adds the real density-wrapper/coordinate
and nested-spline diagnostic views, without copying a second spline algorithm.

Java has no FerrumC sample-work budget. Rust admission, shared work counters,
finite-result policy, cancellation and failure timing require separate Rust
tests. Literal NaN probes intentionally retain the original Java answer and
request trace; a Rust nonfinite refusal is a distinct checked consumer policy.
For example, a NaN RangeChoice input can select Java's finite outside value;
FerrumC can refuse that completed input before selecting a branch. The private
Rust trace harness represents Java `ProbeFailure` with a selected-child
nonfinite sentinel and checks the typed refusal prefix. That demonstrates
request order and checked failure stopping, not arbitrary Java exceptions in
the Rust runtime or plugin exception compatibility.

## Reproduction and provenance

```sh
python3 server/crates/ferrumc-world/tests/reference/generate_density_router.py \
  .codex-tmp/original-1.21.8 /path/to/jdk21 /fresh/output
python3 -O server/crates/ferrumc-world/tests/reference/generate_density_router.py \
  .codex-tmp/original-1.21.8 /path/to/jdk21 /another/fresh/output
```

The helper is freshly compiled and placed last after the pinned inner JAR
and43 pinned libraries. The generator rejects a helper class entry in those
JARs and Java option-based agent/classpath injection. Before bootstrap, the
helper loads27 original classes without initialization, verifies each actual
`Class.CodeSource` against the inner JAR, and separately verifies its own
source against the fresh classes directory. Every successful class check is
printed individually in the run log. Missing or wrong origins refuse before
bootstrap and output. All51 input artifacts, including helper/generator and
Java/javac binaries, are hashed before and after execution.

Fresh normal and `-O` exports have byte-identical fixture **and manifest**.
Eight separate negative guards passed in both modes: bundle, inner, mappings,
source fixture and a bundled library digest; helper classpath shadow; wrong
original class CodeSource; wrong fresh-helper CodeSource. Digest negatives
exercise the real generator entry point with one injected wrong hash and
require refusal before helper/output. Class-origin negatives freshly compile
the final helper and invoke it against the actual pinned classpath with the
wrong expected origin. No input artifact changes during these guards.

The initial2,163-record corpus is independently reconstructed by stripping
only added draw/maxima/native-root metadata and new spline diagnostics, then
requiring its original SHA256
`70cded38014c5f4b6947c09da9588a578013ecef11fd53493fc52e889db2aae6`.
Existing sample bits, diagnostics, ordering and structures are preserved.
The GEN05.1 compiler and GEN04.3 CubicSpline reference artifacts remain read-only.

Final fixture SHA256:
`2012acc2b8256a9adbf10b54a3534e8d9cc5d0a79d5e481d5bc3ff55cd73430e`.
The manifest supplies the exact final helper, generator, library and toolchain
hashes. Local proof logs/outputs are `.codex-tmp/gen05_2-child-oracle-final-*`
and `.codex-tmp/gen05_2-child-oracle-negative-*`; the exact five-file final
freeze is `.codex-tmp/gen05_2-child-oracle-final.freeze.json`.
