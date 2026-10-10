# Original density codec structural oracle

`generate_density_compiler.py` compiles fresh `density_compiler_oracle.java`
helper bytecode and reflects into the SHA-pinned Minecraft 1.21.8 server JAR.
It calls the actual `VanillaRegistries.createLookup`, `RegistryOps.create`,
`DensityFunction.DIRECT_CODEC`, `DensityFunction.HOLDER_HELPER_CODEC`, and
`NoiseGeneratorSettings.DIRECT_CODEC`. It does not calculate density samples,
copy density formulas, or generate answers from the Rust implementation.

Reproduction, from the repository root:

```sh
python3 server/crates/ferrumc-world/tests/reference/generate_density_compiler.py \
  .codex-tmp/original-1.21.8 \
  /home/jstrauss/.local/share/ferrumc-tools/jdk-21.0.12.1+1 \
  .codex-tmp/density-codec-reproduction
```

The generator checks the bundle, inner server JAR, official mappings, exact
source fixture, and all 43 bundled library SHA256 hashes before executing the
helper. It requires Java 21 and places the freshly compiled helper last in the
classpath. Pin checks use explicit exceptions and remain active under Python
`-O`. No checked-in golden is modified by reproduction.

## Projection schema 1

The JSONL file contains one canonical key-sorted record per line. Array order
is preserved. Built-in records have `kind`, `id`, and `projection`; `density`
records cover all 35 definitions and `router` records cover all seven settings.
References are terminal canonical names. The matching separate density record
provides the definition rather than expanding a resource recursively.

- `constant` contains `value_bits`: 16 lowercase hexadecimal digits for f64.
  Spline constants and point `location_bits`/`derivative_bits` use eight digits
  for f32. These are actual raw Java bits, including both signed zeros.
- `holder` contains either `resource` or `inline`. The real density holder
  helper creates a `HolderHolder` for **both** reference and inline inputs.
- A spline's `coordinate` is a raw holder: `{resource: ...}` or
  `{inline: <density>}`. It does not add the density holder helper's wrapper.
- `binary` has the original `operation` string and ordered `argument1` and
  `argument2`. `mul_or_add` has `operation`, the remaining `argument`, and
  `mapped_argument_bits`, reflecting the actual original factory class.
- `mapped` and `marker` retain the original serialized `operation` string and
  `argument`. Marker descriptors are retained rather than removed.
- `noise` fields keep raw parameter-holder identity. Named parameters contain
  `resource`; inline parameters contain `first_octave` and `amplitudes_bits`.
  `shift`, `shift_a`, and `shift_b` also hold **noise**, not density, arguments.
- `multipoint` splines retain ordered points and nested spline values. Each
  point has its location, value, and required derivative; no derivative default
  is invented.
- Routers contain exactly 15 `noise_router` fields, the four original noise
  geometry fields plus actual `cell_width`/`cell_height`, `sea_level`, and the
  four original boolean settings flags. The whole settings codec actually
  decodes default materials, surface rules, and spawn targets, but these
  deferred payloads are not projected or claimed as compiled by this ticket.

`probe` and `settings_probe` records additionally contain `input_json` and
`status` (`ok` or `error`). This string retains the exact source tokens passed
to the codec's JSON parser; geometry probes retain the injected number token.
A redundant decoded input object is omitted so a consumer does not first round
`-0` or try to store the valid INT token `1e1000` in f64.
Successful probes contain an actual class
projection. Error records do not include unstable object strings or identity
hashes. Synthetic geometry probes use a minimal valid settings definition and
an all-zero router; they are distinct from the seven real built-in records.
`factory` records directly call original `DensityFunctions.add`/`mul` to
distinguish factory lowering from codec holder behavior.

## Confirmed codec details and explicit Rust policies

The corpus covers all 32 serialized density forms successfully, plus bare
constants, namespace normalization, holder/direct shape errors, unknown
resources/types, missing fields, and bounds. Six factory cases compare left
and right inline Constants and a named holder. Codec binary arguments remain
holders, so they do not automatically trigger the factory's Constant lowering.

Several probes deliberately expose original permissiveness. They are evidence
for documenting checked FerrumC restrictions, not instructions to loosen
bounded production admission:

| Probe | Actual original result |
| --- | --- |
| Spline missing `derivative` | Codec error; the field is required. |
| Decimal `1.0000000596046448` in all three spline float fields | `3f800001`, from direct decimal-to-f32 conversion; an f64 intermediate can produce a different result. |
| Reversed clamp/range endpoints; equal/reversed gradient endpoints | Accepted by their original codecs. |
| Spline duplicate signed-zero/reversed locations | Codec accepts them; the separate Java builder enforces ascending order. |
| Spline float input `1e39`; noise scale `1e309` | Original accepts infinity; checked Rust finite policy is additional. |
| Extra object field | Ignored by the original codec; strict unknown-field rejection is additional. |
| Inline noise parameter object | Accepted by the original holder codec; support restrictions of a closed built-in compiler must be explicit. |
| Gradient integer fields `1.5`, `2.9` | Original `Codec.INT` with Gson converts to `1`, `2`. |
| Gradient `4294967297`, `4294967298` | Converts to `1`, `2`; integer wrap is not a schema error in the original. |
| Gradient `9223372036854775808`, `9223372036854775809` | Converts to `0`, `1` through the actual Gson number conversion. |
| Gradient `1e309`, `1e1000`, `1e-1000` | INT admits zero after decimal truncation/wrapping, even when DOUBLE decoding would overflow. |
| Geometry `min_y=-64.5`, `height=128.9` | Converts to `-64`, `128`, then passes geometry guards. |
| Geometry size `1.9`/`2.9` and oversized wrapping integers | Truncates/wraps before original range and alignment guards. |

Rust preserves the tested INT truncation/wrapping. Bundled Gson 2.11
`NumberLimits.parseBigDecimal` bytecode was also inspected: token length is at
most 10000 characters and absolute decimal scale must be less than 10000.
Rust checks those guards before its bounded digit loop and reports a typed
failure where the original may throw `NumberFormatException`. These synthetic
cases do not occur in the pinned built-in fixture. No sampling, seed binding,
interpolation, aquifer, surface,
biome, full natural chunk, Survival, or Paper runtime parity is established by
this structural corpus.

## Verified generation

The final normal and Python `-O` runs produced byte-identical JSONL: 128 records
(35 densities, seven routers, 72 density codec probes, eight settings probes,
and six factories), 278,269 bytes, SHA256
`856bd9f9d25642864945a11bd2ea907fa3c90b4fda8bd7b1153017b74da65a73`.
All 32 serialized forms succeeded. Across the 80 codec probes, 71 succeeded and
nine were rejected by the actual original codecs. Negative bundle, inner-JAR,
mappings, source-fixture, and altered-library hash checks all failed before
helper execution under both Python modes.

## Reference inspection

Official mappings identify all reflected fields/classes and codec/factory
methods. The original JAR and its libraries supply every codec product.
Relevant inspected source paths are under the pinned Paper Build 60 applied
source tree; its behavioral differences remain separate from JAR evidence:

| Source | SHA256 |
| --- | --- |
| `VanillaRegistries.java` | `69965842728b8a5ff4a829cabf16ffde977513fef6cef5fdb30854a84189344c` |
| `RegistryFileCodec.java` | `030a5ad3d560f6f1e2d52b4698a8cdb0bd832ae11be4cff9e644f91820f90f3a` |
| `CubicSpline.java` | `834d1b5f4696161ca974fcf2b4ef8caa3ebec5e6a0b5bfcccb561ba4a6a63f97` |
| `DensityFunction.java` | `3a7c65bcefe09274d50a69e971858679273d2090a190202200cc2d6b8c7459f9` |

Full density/settings/factory source review belongs to the owning GEN-05 task
matrix. The fixture manifest pins this helper, generator, source JSON, JAR,
mappings, libraries, and Java/Python/platform information. Reproduction logs
can contain original warnings about disjoint `min`/`max` inputs; they do not
change deterministic projection records.
