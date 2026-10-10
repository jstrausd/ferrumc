# GEN-05.4 actual-JAR Aquifer addendum

Separate immutable addition to the original499-row freeze. The original matrix,
its scripts and its freeze are unchanged. This helper invokes original
`Aquifer.NoiseBasedAquifer.computeSubstance`, `computeFluid`, `getAquiferStatus`,
`calculatePressure` and original `NoiseChunk.preliminarySurfaceLevel` against
the same Paper1.21.8 Build60 JAR, source pins and120-artifact closure.
No center selection, pressure, fluid, Preliminary scan, indexing, packing,
surface or RNG formula is reproduced as an expected-value implementation.

| Group | Rows | Policy |
| --- | ---: | --- |
| enabled_substance |27| finite_parity |
| enabled_ties |6| finite_parity |
| fluid_scan |6| finite_parity |
| fluid_cache |3| finite_parity |
| nonfinite_scope_probe |15| nonfinite_scope_probe |
| **Total** |**57**| **42 finite parity cases,15 explicit scope probes** |

## Declared seams

Nine constant raw f64 density leaves remain the same input convention as499.
Preliminary uses `preliminary={default_f64_bits,overrides:[{x,y,z,value_f64_bits}]}`
with at most128 exact point overrides, injected into the genuine own
NoiseChunk's initialDensityNoJaggedness field. The original Preliminary method,
its threshold, vertical loop, Quart alignment and cache remain untouched.
There is no piecewise density rule or test-side surface-height formula.

Global fluid uses `picker={default:{height,fluid},overrides:[{x,y,z,status}]}` with
at most16 exact positions. `global_fluid` events capture original returned
FluidStatus before the original caller performs `at(y)`.
Ties declare exactly twelve `{grid:[x,y,z],point:[x,y,z]}` location inputs,
installed using the original private getIndex and BlockPos.asLong. The
original computeSubstance alone chooses their order; there is no test-side
distance calculation, sorting or expected-rank implementation.
RNG delegates to the original RandomState.aquiferRandom positional factory and
original RandomSource nextInt. NoiseChunk is genuinely initialized and advanced
for each fresh case. Own and foreign SinglePoint contexts stay distinguished.

## Output contract

Rows retain `kind`, `case`, `policy`, `inputs`, `result`, `requests`, `rng_events`
and `events`, plus `global_requests`. Inputs remain byte-value equivalent to
the declared cases. The three projections filter the full interleaving by
density, factory/nextInt, and global_fluid respectively.

`result` has `cache_before`, `calls` and `cache_after_calls`. Each call records
events_start/events_end and raw_state_id/schedule for enabled Substance,
height/fluid_raw_state_id for FluidStatus endpoints, or raw result/sentinel f64
bits for Pressure. Each actual reflected cache snapshot records both array
capacities, occupied locations `{index,packed_i64,x,y,z}`, occupied statuses
`{index,height,fluid_raw_state_id}`, and Preliminary cache entries
`{packed_i64,x,z,surface_level}`. Packed i64 values are decimal strings, arrays
are ordered by original array index, Preliminary entries by signed packed key.

## Actual observed branch evidence

The first finite actual-random case performs12 original factory-at calls and
36 original bounded draws in10/9/10 order. Repeated Substance calls reuse
those locations without new draws. The twelve declared tie candidates contain
four equal-distance points about8/24/8; the actual miss chronology selects
16/24/16,16/24/0,0/24/16,0/24/0. The same-status case reports false flags;
changing only the fourth selected status reports true flags; the subsequent
strictly positive subnormal Substance call resets the flag to false and
performs no request. Positivezero and negativezero enter the fluid branch.

The full13 scan records thirteen Preliminary columns including the negative3
chunk X offset. Its second invocation performs no Preliminary density query.
The submerged-center and above-surface cases exercise the original early
returns; the negative3-neighbor case stops after six scanned columns. The
constantzero Preliminary input records Integer.MAX_VALUE and the original
wrapped +8 early return. Direct getAquiferStatus cache probes perform no
event at all on their second call. These facts are observed from actual traces,
not encoded as expected values in the helper.

## Nonfinite scope boundary

The15 nonfinite rows deliberately lie outside the finite Rust density/
Material contract. Original Java does not reject these values: positive
infinite Substance returns null before any request; NaN and negative infinity
can return fluid. NaN Floodedness returns WAY_BELOW, NaN spread returns a
quantized finite height, and infinite spread can produce wrapped i32 heights.
NaN or negative-infinite Preliminary produces MAX, whereas positive infinity
passes its threshold. Pressure preserves raw NaN/infinity results and sentinels.

Therefore finite-only Rust refusal must be tested and documented as admission
policy. It must not be described as original-Java parity on these15 rows or
silently compared to a synthetic finite expected result. Finite42 rows are the
parity contract; nonfinite15 are explicit policy controls. No exception was
observed from the original methods in these15 cases.

## Provenance and limits

Fresh normal and Python -O exports have byte-identical JSONL, original origins,
manifest and all five helper class files. The exporter checks32 original
CodeSources, five fresh helper origins, all120 artifact hashes, all26 approved
source hashes, fixed registry input hash and compiled-helper hashes before and
after execution. Java receives only PATH/LANG/LC_ALL/TMPDIR, with no global
environment inheritance. Helper-package shadowing in all JARs is refused.

Eight controlled guard probes pass normally and under Python-O: three actual
Java-option environment injections; worldgen/JAR/approved-source digest fault
injections; actual Java processes with wrong original and wrong fresh-helper
CodeSources. Digest faults simulate changed hash results without mutating any
pinned original input. Refusal occurs before compilation or original bootstrap
and no branch output is published.

Only final-normal/final-optimized, helper/source scripts/cases, this report and
guard evidence belong to this freeze; pilot output is provisional. This is a
Java reference export only, not Rust/Material/gameplay/Survival/serverboot/
drop-in or performance acceptance. Full material arrays and Rust budgets,
failure lifecycle, reset and publication are separate World/root checks.

## Owned World reproduction

The adopted Java helper, cases and case builder are byte-identical to the
independent freeze. Only scaffold paths in `build_helper.py` and repository/
output paths in the exporter/guard runner were relocated. The owned exporter
also pins helper/cases/both builders/scaffold inputs/root manifest/source pins
and JDK bytes/version, and refuses five Java/Javac/classpath environment
injections. Original JSONL, origins and all five helper classes are unchanged.
Normal and Python-O owned manifests are byte-identical; owned provenance and
ten passing guards are retained in `material_addendum_1_21_8`.

Run the owned runner with one new repository `.codex-tmp` output directory.
Run `probe_guards.py VALID_EXPORT NEW_GUARD_DIRECTORY` in normal or Python-O
mode. The ten guards include the five environment injections, three controlled
digest faults, and two actual JVM CodeSource failures. The independent eight
guards described above remain original producer evidence. These commands need
the same locally pinned Paper120-artifact closure, full Base/Applied sources,
worldgen fixture and Java21 installation as the complete-array reference.
