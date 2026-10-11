# GEN-05.6 — Actual original biome zoom oracle

The `biome_zoom` module implements original seed obfuscation and block-to-quart
coordinate selection. The expected results come from actual methods in the
official Vanilla Java1.21.8 and Paper1.21.8-build60 inner JARs. This is a genuine
helper-method comparison, not a started server or gameplay acceptance.
Independent Root archive review remains pending.

## Sources and numerical contract

Paper commit `29c8822d90899c89d2689338e81a98f690bcba12`, protocol772. Full Base
and Applied `BiomeManager.java` and `LinearCongruentialGenerator.java`, the
Moonrise `BiomeManager.getFiddle` feature hunk and full official Mojang mappings
were reviewed and frozen before production edits. The seven-file source/design
freeze is `.codex-tmp/gen05_6/design/source-contract.freeze.json`, SHA256
`83ade59a4ebeddb6dc8cce565a339465948d090dee36e0fc547679a58e23e0c6`.

`obfuscateSeed` calls original Guava SHA-256 `hashLong(seed).asLong()`. Guava
writes eight little-endian seed bytes and reads the first eight digest bytes
little-endian. Original outputs and independent Python hashlib checks establish
this interpretation. Rust's new justified `sha2 = "=0.10.9"` dependency uses
bounded stack SHA state. Root generated the lock edge using actual offline
Cargo; only World's dependency list changed, with no package/version changes.

`getBiome` wraps signed32 subtraction by2 before arithmetic right shifts and
local low-bit fractions. It visits eight corners in X/Y/Z bit4/2/1 order, uses
signed64 wrapping LCG mixing in X/Y/Z/X/Y/Z/seed/seed order, evaluates
squareZ + squareY + squareX, and retains the first equal minimum. Rust preserves
that order without FMA or reassociation. Every signed32 block axis is supported;
the resulting quart may be outside density's separately checked domain.

Vanilla divides its floorMod bucket by1024, subtracts0.5 and multiplies0.9.
Applied Paper subtracts integer512 from its masked bucket before multiplying
0.9/1024. Both expressions produced identical bits in all1024 possible buckets.
The two explicit semantics variants preserve their source provenance.

## Genuine calls and complete retained values

Canonical sources live in [biome-zoom](biome-zoom/). Fresh Java compilation
produces three helper classes in package `ferrumc.zoomreference`; reflection
invokes actual original hash, fiddle, LCG, distance and selection methods.
The helper contains no rewritten version of these algorithms.

The committed [requests](../fixtures/biome-zoom-original-1.21.8/requests.tsv)
and [values](../fixtures/biome-zoom-original-1.21.8/original.tsv) retain every
input and actual method output, including full raw f64 bits. Vanilla and Paper
value files are byte-identical and stored once; their distinct provenance
manifests remain in the review evidence. No lossy value projection is used.

| Kind | Genuine invocation | Rows per original mode |
| --- | --- | ---: |
| H | `BiomeManager.obfuscateSeed` | 11 |
| F | private `BiomeManager.getFiddle` | 2048 |
| L | `LinearCongruentialGenerator.next` | 49 |
| G | `BiomeManager.getBiome` with a tracing original source interface | 6815 |
| D | separately reflected private `getFiddledDistance` | 54520 |
| Total | complete accepted output rows | 63443 |

The2048 F rows include every1024 positive/negative bucket. G groups are3584
world-seed residue cases,896 already-obfuscated residue cases,819 axis-seams,
1512 all-axis wrapping cases and4 exact tie cases. They cover all64 block
residues, positive/negative quart/section/chunk seams, +/-30M, signed32
extremes and all selected signed64 seed boundaries. For each G the actual
retained manager, original `withDifferentSource`, fresh manager and reverse
replay are called: four genuine getBiome calls per labelled case.

The tracing source permits only the original `getNoiseBiome(int,int,int)`
callback. It records quart coordinates, requires exactly one call, returns
null, and checks the original method returns null. This observes genuine final
source delegation without implementing biome resolution or registry bootstrap.
Rust publishes coordinates only and never invokes that callback.

Each G has eight explicitly generated D inputs. Those D values are independently
invoked genuine distance methods, not interception of getBiome's private calls.
Their first minimum is independently compared with the actual source coordinate.
The Java helper itself never constructs corners or chooses D minima. Python's
independent seed hash is used only to construct explicit D inputs and validate
actual H/G outputs, not to substitute an original algorithm result.

At raw zoom seed268435456, BlockPos(134217733,-134217723,134217733), genuine
minima at corners2 and4 have bits `3fe07d54eb851eb8`. Actual getBiome selects
corner2, Quart(33554432,-33554431,33554432), in both modes. Three explicit
variants (+2^34, -2^34 and i64MIN+268435456) confirm the same behavior. Local
comparison controls for equal and adjacent finite values are labelled separately
from these genuine ties. Exploratory candidate searches invoked private
distances only; their logs are not claimed as independent frozen earlier source
versions or getBiome acceptance. No further search is required.

## Pins, origins and guards

[input-pins.json](biome-zoom/input-pins.json) pins every canonical helper,
runner, generator and guard source, semantic mappings, requests, case contract,
frozen design, complete Base/Applied sources, feature patch and official mapping.
It additionally pins Java21.0.12.1+1 `java`, `javac` and `lib/modules`, the
Vanilla bundle/inner/43 libraries (45 entries), and Paper inner/119 libraries
(120 entries) plus its bundle. Actual artifact inventories were adopted from
the already independently checked GEN-05.2 Root receipt, not reconstructed
from convenient classpaths.

Actual original inner hashes:

- Vanilla: `2bee159e4ce87ae857b4323f9f32c7a3d7d448f7f8c3f62fa6f295a72e228250`.
- Paper: `c64fb7dbc95cd15aa0b7460a5b1e117eef5ee91c4e6fa7a8f1fe6487b6c31507`.

The full official mapping is8186232 bytes, SHA256
`e373ba569839e6f3c7bd4880247f3e85577d92cde2c605adfa2024eb9cae503e`.
Vanilla names are derived and checked against it: Manager`dnz`, source`dnz$a`,
BlockPos`jb`, LCG`bbx`, Vec3i`kg`, QuartPos`jw`, Holder`jl`, Mth`bcb`.
Per-mode actual CodeSources include3 helper,8 original,6 Guava and2 DFU
classes (19 total), all checked before accepted output. The runner hashes every
actual input before and after; no actual original input was modified.

The runner refuses five Java environment variables, noncanonical helper
placement, nonfresh output, pinned digest changes and any artifact shadowing
the entire helper package. Original JARs precede fresh helper classes in the
classpath; unexpected fresh class inventory also fails. Python normal and -O
use explicit `require` checks, with no assertion-dependent guard.

Both canonical guard series pass nine refusals: five actual subprocess
environment injections, a canonical-source path rejection, a declared digest
fault, a declared ZIP-name shadow fault, and an actual helper invocation with
the wrong expected original CodeSource. Declared faults modify expected inputs
only; originals remain untouched. The actual CodeSource refusal occurs before
sample files are created. These fault kinds are not misrepresented as mutated
real artifacts.

## Reproduction and results

From the repository root, with pinned original inputs at their recorded paths
and all five Java environment injection variables unset, use fresh output dirs:

```sh
python3 server/crates/ferrumc-world/tests/reference/biome-zoom/run_oracle.py .codex-tmp/gen05_6/final-normal
python3 -O server/crates/ferrumc-world/tests/reference/biome-zoom/run_oracle.py .codex-tmp/gen05_6/final-optimized
python3 server/crates/ferrumc-world/tests/reference/biome-zoom/check_guards.py .codex-tmp/gen05_6/final-guards-normal
python3 -O server/crates/ferrumc-world/tests/reference/biome-zoom/check_guards.py .codex-tmp/gen05_6/final-guards-optimized
```

The final directories already contain author evidence; a fresh reexport needs
new output names. The guard recipe requires the accepted classes in final-normal.
The case generator deterministically reproduces the committed requests and
case contract; expected values always require original method calls. Relocating
pinned originals or a checkout requires reviewed path/pin updates, never silent
input substitution. Canonical normal/-O exports were rerun after promotion from
scratch; scratch and final original values all remain byte-identical.

| Artifact | SHA256 |
| --- | --- |
| final canonical input pins | `6ff3b436cec31abb010ce5fe0fb051a2bd2358ceecb11380e58d25541bd610e6` |
| requests | `9f044930afd9ad3ab4f2d4091c109483d8e9ab6ba184e4897b86d80ff6ea5cfa` |
| original values in both modes/variants | `987e0beaea65cd564acc7e9f4a743e2212105f37430a36fe710f64d45c2f30ce` |
| final normal manifest | `66031f764ca13912260b50cd820a470dc0db77c9cc9d9226d4249d8d68eee21b` |
| final optimized manifest | `accf7b2686009c5e6a095e7be0ed87590a627e2e14b4d29bcadeb6bfd2c6cf18` |

Every H/F/L/G/D Vanilla/Paper difference count is0. Normal/-O manifests have
different class/output origins because they use separate fresh directories;
value bytes and all accepted cases are identical. Complete manifests include
actual inventories, origins, exact tie distances and nearest unique minima.

Rust's complete corpus test runs every retained value in both semantics.
Separate contracts verify genuine ties, local strict comparison, distinct seed
types/no rehash, the full signed32 domain independently of density admission,
and native value layouts. Two compile-fail rustdocs prevent implicit seed and
quart type conversion. Author Debug and Release each pass222 unit,46 integration
and2 compile-fail rustdoc tests. These are shared-tree World scope checks, not
an isolated full-checkout claim. Strict all-target Clippy, Rustdoc with
`-D warnings`, World Fmt and workspace Fmt pass in isolated baseline65064bc7
plus only World/brief files and the actual Root-generated lock edge. Initial
Clippy errors from parallel foreign Items WIP remain as failed evidence;
no lint exceptions or foreign fixes were applied. Logs and receipts are under
`.codex-tmp/gen05_6/checks/`. All44 prior fixture files remain byte-identical.
Root's independent fresh archive build and full test run remain pending.

## Memory and activation limits

Observed native layouts on this target are8-byte seed,12-byte quart,1-byte
semantics and16-byte immutable selector. Selection retains0 heap bytes and
uses only fixed stack arrays and eight fixed corners/64 LCG steps; SHA hashing
uses fixed stack state for an8-byte message. There is no mutable cache,
cumulative budget, poison state, global or I/O in this producer. These are
native layout/code-review facts, not allocator-overhead, RSS, latency or TPS
measurements.

This ticket adds no source resolver, implicit Plains fallback, active chunk
adoption, height clamp, biome wire/storage ID, surface rules, bedrock, carvers,
features, structures, mobs, Survival or Java-plugin compatibility. It supplies
one required numerical dependency for later original surface/biome consumers.
The executable server still uses its existing generator.
