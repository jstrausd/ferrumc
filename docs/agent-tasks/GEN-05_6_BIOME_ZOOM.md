# GEN-05.6 — Original biome zoom coordinates and seed

Status: World implementation, genuine original exports and author checks
complete; independent Root archive review pending. The pre-production design
is preserved separately in `.codex-tmp/gen05_6/design/` with its source freeze.
GEN-05.5's original exports and all older fixtures stay unchanged.

## Scope and public contract

One crate: ferrumc-world. Allocation-free coordinate selection and bounded seed
obfuscation only. This ticket produces a selected quart coordinate; it does not
resolve stored biome arrays, fetch neighbors, sample a biome source, activate
App generation or evaluate surface/carver rules.

Implemented types and methods, subject to independent integration review:

- `BiomeZoomSeed`: private signed64 obfuscated bits, distinct from `WorldSeed`.
  `from_world_seed(WorldSeed)` hashes exactly once; `from_obfuscated_bits(i64)`
  explicitly accepts an already hashed original seed. `bits()` exposes the
  preserved bits. There is no implicit conversion between the two seed types.
- `BiomeZoomSemantics`: explicit `Vanilla1_21_8` or `Paper1_21_8Build60`; no
  silent default or automatic choice based on noise settings.
- `BiomeQuartPos`: private signed32 X/Y/Z and read-only axis methods, distinct
  from density's checked `ClimateQuartPos`. This producer creates it as output;
  conversion to a density request requires the existing explicit checks.
- `BiomeZoom`: immutable seed plus semantics. `new(BiomeZoomSeed, semantics)`
  and `select_quart(BlockPos) -> BiomeQuartPos` are infallible. Every signed32
  input axis is supported, with original wrapping subtraction before shifts.

The result's possible quart axes are within -536870912..=536870912. No implicit
world-border, noise-domain or height-accessor restriction is imposed. Those
belong to the eventual resolver. Pure selection executes exactly eight corners,
with 64 bounded LCG steps total, using stack data and no collection allocation,
mutable global or I/O. Seed hashing handles exactly eight bytes in one SHA-256
message block. Root approved a new justified World sha2 0.10.9 dependency,
already present in the workspace lock. Root alone updates World's lock edge;
this author does not hand-edit Cargo.lock or declare dependencies elsewhere.

## Actual original behavior

Reference: Minecraft Java 1.21.8, protocol772; Paper1.21.8 build60, commit
`29c8822d90899c89d2689338e81a98f690bcba12`. Read full Vanilla Base and Applied
`BiomeManager` and `LinearCongruentialGenerator`, plus the exact Moonrise
`BiomeManager.getFiddle` hunk. Official Mojang mappings identify original
obfuscated Vanilla classes and methods; actual artifact CodeSources are checked.

`BiomeManager.obfuscateSeed` calls Guava SHA-256 `hashLong(seed).asLong()`.
Guava writes the signed64 bits little-endian and reads the first eight digest
bytes little-endian. Actual original calls and an independent Python hashlib
check establish these bytes; the helper does not implement the hash algorithm.

`getBiome` subtracts2 with Java signed32 wrap on each axis, shifts right2
arithmetically, and takes the low two bits divided by4.0 as local fractions.
It visits corner0..7 with X/Y/Z bits4/2/1. Each corner invokes the original
private fiddled-distance method. The strict `best > candidate` comparison
retains the earliest equal minimum. It then calls the selected source exactly
once and returns that source's Holder. This scope returns only its requested
quart coordinates, with no Holder/resource or fallback behavior.

The LCG helper uses signed64 wrapping multiplication/addition. Coordinate
arguments widen with their signed value. Seed/coordinates are mixed in original
X/Y/Z/X/Y/Z order; subsequent seed mixes select the other two fiddles. Distance
evaluation preserves squareZ + squareY, then + squareX, without FMA or algebraic
reassociation.

Vanilla `getFiddle` divides the floorMod bucket by1024.0, subtracts0.5, then
multiplies0.9. Applied Paper masks the bucket, subtracts512 as an integer and
multiplies0.9/1024.0. All1024 buckets have byte-identical f64 results across
these two original methods. Both explicit source contracts remain; a lookup
table or approximate distance cannot silently replace either evaluation.

## Genuine original oracle and reviewed input contract

The canonical helper, runner, generator and guards live under
`server/crates/ferrumc-world/tests/reference/biome-zoom/`. Initial scratch
exports and final normal/Python-O exports remain under `.codex-tmp/gen05_6/`.
A newly compiled JDK-only reflection helper invokes the actual Vanilla/Paper
methods. It never reimplements their selection, fiddle, LCG or SHA algorithms.

Inputs are strict TSV with a unique ASCII label and one of these record kinds:

| Kind | Explicit input | Actual original invocation and output |
| --- | --- | --- |
| H | signed64 worldseed | obfuscateSeed -> signed64 bits |
| F | signed64 mixer value | private getFiddle -> raw f64 bits |
| L | two signed64 values | LinearCongruentialGenerator.next -> signed64 bits |
| G | world/zoom seed kind, signed64 seed, three signed32 block axes | original getBiome with tracing NoiseBiomeSource -> selected quart and one source call |
| D | signed64 zoom seed, three signed32 quart axes, three explicit raw f64 offset bits | private getFiddledDistance -> raw f64 bits |

For each G record, the input generator also creates eight explicit D records.
They are isolated genuine method probes, not traced interceptions of getBiome's
private calls. The expected selected coordinate comes only from actual
getBiome. Independent validation compares it with the first minimum of the
eight genuine D results and retains exact minimum ties separately. The Java
helper does no corner construction or minimum calculation for accepted G/D
results. Exploratory tie searches may enumerate candidate inputs and compare
genuine distances; only verified final getBiome requests enter acceptance.

Every G request is checked with the retained original manager, original
withDifferentSource replacement, fresh manager and reverse replay. The tracing
source permits only the precise original getNoiseBiome(int,int,int) callback,
counts it, records its signed coordinates and returns null. A null return is
legal for observing this method's final delegation; no biome resolver or registry
bootstrap is claimed. Unexpected callbacks, extra source calls and origin
changes abort without a completed fixture.

The corpus covers every64 block-residue combination; positive/negative
quart, section and chunk seams; +/-30M positions; all signed32 boundary wrap
cases; complete signed64 seed extremes; pre-obfuscated seeds; all1024 positive
and negative getFiddle buckets; genuine LCG wrap controls; near ties and an
original strict-first source contract. Exact minimum ties have been verified
through genuine getBiome calls in both modes.

Exploration is finished: 513216 and23887872 candidate positions without equal
minima, followed by8192000 structured full-domain candidates with one equal
minimum. Each candidate compared eight actual private getFiddledDistance calls;
these are not claimed as getBiome calls or tracing-source acceptance. The one
candidate is raw zoom seed268435456 at BlockPos(134217733,-134217723,134217733),
with genuine distance minima at corners2 and4. The final helper verifies this
case and three explicit high-bit seed variants with actual getBiome in both
original modes. Corner2 is selected in every variant. No additional exploration
search is required or authorized. Exploration logs do not constitute frozen
reproducible earlier source versions; only final accepted inputs and original
exports establish tie acceptance.

Per original mode the final corpus has11 H,2048 F,49 L,6815 G and54520 D rows
(63443 total). The G groups are3584 world-residue,896 pre-obfuscated residue,
819 axis-seam,1512 all-axis wrapping and4 genuine tie cases. Each G has four
actual calls across retained/replacement/fresh/reverse checks. Its eight D
rows remain separately invoked original probes. Vanilla/Paper differences
are0 in every record kind; normal/Python-O and scratch/final exports have
identical value bytes, SHA256
`987e0beaea65cd564acc7e9f4a743e2212105f37430a36fe710f64d45c2f30ce`.
Final canonical input pins SHA256 is
`6ff3b436cec31abb010ce5fe0fb051a2bd2358ceecb11380e58d25541bd610e6`.
See the [oracle report](../../server/crates/ferrumc-world/tests/reference/biome_zoom.md)
for complete provenance, recipes and acceptance limits.

The runner pins helper/runner/case source, requests, mappings, Base/Applied
sources, feature patch, JDK java/javac/modules, original Vanilla bundle/inner
and all libraries, original Paper bundle/inner and119 libraries. It rejects five
Java environment injection variables and any original artifact shadowing the
entire helper package before compilation. Exact canonical helper placement,
fresh outputs, actual original/library/helper CodeSources and pre/post hashes
are mandatory. Guard tests run with Python and Python-O using explicit require
checks, not assertions. Declared inventory/ZIP-name fault injections are labelled
as such and never mutate actual pinned original artifacts.

## Acceptance and limits

The approved design and pinned genuine exports are implemented. Final
acceptance requires independent Root archive review and exact Debug/Release
comparison of seed bits, every fiddle bucket, genuine distances and
getBiome-selected coordinates.
Type-separation compile-fail checks, explicit i32/i64 wrap boundaries and exact
tie cases prevent the density domain or an accidental seed rehash from
entering this API. Author Debug and Release each pass222 unit,46 integration
and2 compile-fail rustdoc tests. These are shared-tree World scope checks,
not an isolated full-checkout claim. Strict all-target Clippy, Rustdoc with
`-D warnings`, World Fmt and workspace Fmt pass in isolated baseline65064bc7
plus only World/brief files and the Root-produced lock edge. The initial
Clippy refusal from parallel foreign Items WIP is retained separately without
lint exceptions or foreign fixes. Root's independent fresh full-archive build
is still required. All44 prior World fixture files remain byte-identical.

This pure bounded producer does not need a cumulative sampler budget or poison
state: no user-sized loop, recursion, candidate array or admitted execution
failure exists. Native layout checks observe8 bytes for the seed,12 for the
quart,1 for semantics and16 for the immutable selector on this target. Selection
and seed hashing own no retained heap; stack SHA state and stack arrays are
separate from allocator overhead/RSS, and there is no TPS claim. This supplies one dependency
for future surface/carver consumers; no complete Minecraft/Survival or drop-in
server acceptance follows from this ticket.
