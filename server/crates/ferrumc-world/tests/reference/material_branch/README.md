# GEN-05.4 actual-JAR primitive branch oracle

This independently produced reference invokes the original methods from Paper
1.21.8 Build60, commit `29c8822d90899c89d2689338e81a98f690bcba12`.
The Paper inner JAR SHA256 is
`c64fb7dbc95cd15aa0b7460a5b1e117eef5ee91c4e6fa7a8f1fe6487b6c31507`.
Declared constant density leaves and at most three raw float RNG inputs are
test inputs. No Aquifer, pressure, surface, fluid or Ore formula is reproduced
as an expected-value implementation.

The final normal and Python `-O` exporters yield identical branch JSONL,
original class origins, input/result manifests and all five helper class files.
Invocation files record the distinct output paths. The exporter supplies only
PATH/LANG/LC_ALL/TMPDIR to Java, refuses Java option injection, scans all120
original JARs for helper shadowing, checks29 original class CodeSources and five
fresh helper origins, and rechecks all artifact/source/input hashes afterward.
The reference fixture and all26 approved original source pins are unchanged.

## Frozen matrix

| Endpoint group | Rows | Original invoked method |
| --- | ---: | --- |
| pressure |69| Aquifer.NoiseBasedAquifer.calculatePressure |
| surface_level |151| Aquifer.NoiseBasedAquifer.computeSurfaceLevel |
| random_surface |70| Aquifer.NoiseBasedAquifer.computeRandomizedFluidSurfaceLevel |
| fluid_type |66| Aquifer.NoiseBasedAquifer.computeFluidType |
| ore |107| OreVeinifier.create and returned original filler |
| disabled_substance |36| Aquifer.createDisabled and original computeSubstance |
| **Total** |**499**| |

Coverage includes four pressure shapes on either side of the internal +/-2
guard, water/lava contact, fluid-height ties, NaN and prefilled sentinels and
shared sentinels over two calls. Internal exact +/-2 pressure is unreachable
with integral context Y/fluid heights because the shape lies on a half-step
lattice; the rows use nearest integral Y on either side. Water/lava contact
provides an actual result2. The matrix does not claim otherwise.

Surface coverage includes original widened float DeepDark thresholds with f64
and f32 neighbours, short circuit, Floodedness, MAX-surface Java wrapping;
spread quantization around signed zero and +/-3 with scaled addresses; and
strict lava abs0.3 / WAY_BELOW=-32512 boundaries. Ore rows cover all actual
float thresholds, Y/sign/range, short circuit, density/RNG interleaving and21
actual RandomState.oreRandom streams. Disabled Aquifer includes Substance+0/-0
and fluid-height equality. Original own NoiseChunk contexts are constructed
and positioned using their actual initialization/select/update methods;
foreign contexts are genuine original SinglePointContext instances.

## JSONL contract

Each row contains `kind`, `case`, `inputs`, `result`, `requests`, `rng_events`
and `events`. All input f64 values are raw16-digit hex, f32 values raw8-digit
hex. `inputs.leaves` declares barrier/floodedness/spread/lava/erosion/depth/
toggle/ridged/gap. `context` is NoiseChunk or SinglePoint. All coordinates and
endpoint-specific input fields remain unchanged in the actual output.

Pressure results contain arrays `value_f64_bits` and
`sentinel_after_calls_f64_bits`; surface endpoints return `surface_level`;
fluid/ore/disabled return `raw_state_id` with null encoded as-1; disabled also
returns `schedule`. Density requests include kind/label/x/y/z/own_context and
value_f64_bits. RNG events include factory_at coordinates/mode and next_float
f32_bits; the helper can trace actual bounded next_int bound/value calls,
but these499 primitive rows do not exercise bounded-int draws.

`events` preserves complete interleaving. `requests` and `rng_events` are exact
filtered projections. For `upper_inside_positive2_nan_calls2`, the two result
bits are4011555555555556; the two postcall sentinel bits are3fe0000000000000;
only one barrier request occurs at1/113/2 in own NoiseChunk. For
`lower_outside_positive2_nan_calls2`, the result bits are4011555555555555;
the NaN sentinel stays7ff8000000000000 and no requests occur. `water_lava`
returns4000000000000000 twice without barrier requests or sentinel mutation.

## Limits and remaining acceptance work

This freeze proves an actual Java reference export only. It does not establish
Rust parity, a completed Material producer, active generation, gameplay,
Survival, a launched server, drop-in compatibility or performance.

Enabled Aquifer Substance+0/-0, four-center equality/tie order, location/status
cache miss/hit/order, RNG bounds10/9/10, full13 Preliminary surface-offset scan
and enabled status flags are outside this499-row matrix. Those require a
separate frozen addendum or the independent World review. They must not be
inferred from this primitive oracle or silently added to this freeze.

Controlled guard probes run both normally and under Python `-O`: three actual
Java option environment injections refuse before compilation; a simulated
worldgen digest mismatch refuses the real exporter before compilation; an
actual Java process with the helper directory wrongly declared as the original
JAR refuses at SharedConstants CodeSource before bootstrap or output. The hash
probe injects the digest result, not a mutation of a pinned reference artifact.

Only final-normal/final-optimized, source/scripts/cases, this report and guard
probe evidence belong to the freeze. The earlier normal export is provisional.
The freeze manifest includes exact file sizes/SHA256 and records excluded scope.
