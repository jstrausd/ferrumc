# Item-entity packet reference, Minecraft 1.21.8

Authority: Paper 1.21.8 Build 60, commit
`29c8822d90899c89d2689338e81a98f690bcba12`, protocol 772, data version 4440.
`artifact-pins.json` fixes the official Paperclip outer, patched inner, and all
119 bundled libraries. `source-pins.json` distinguishes Vanilla base, applied
Paper, source patches, and selected complete relevant methods from complete
class review. No Paper implementation is copied into Rust.

## Actual original export

The Java helper is a small fresh plugin in a genuine isolated reference server.
After POSTWORLD initialization it uses the original server registry and item
sanitizer context, the original GameProtocols dispatch codec, and all three
original packet STREAM_CODECs. Original CodeSources must resolve to the pinned
inner. The loaded helper class is checked against the freshly compiled class
hash; the jar has stable entry timestamps. Actual ItemEntity.DATA_ITEM yields
index 8 and actual ITEM_STACK serializer ID 7. No recreated metadata/item
serializer, half-constructed server, or copied velocity quantizer is used.

From the repository root, with Java 21 and the pinned reference prepared as
in `docs/PAPER_REFERENCE.md`:

```sh
python3 server/crates/ferrumc-proto/tests/reference/item_entity_1_21_8/export_paper.py \
  --root . --output .codex-tmp/net04_1/my-fresh-paper-export \
  --java-bin /path/to/jdk21/bin
node server/crates/ferrumc-proto/tests/reference/item_entity_1_21_8/export_prismarine.mjs \
  . .codex-tmp/net04_1/my-fresh-paper-export/paper.tsv \
  .codex-tmp/net04_1/my-fresh-prismarine-export
```

`--runtime` and `--source-root` can locate the exact pinned artifacts and
checkout elsewhere. Outputs must be fresh directories. The runner checks all
121 artifact and 42 source pins before helper compilation and after shutdown.
It binds only `127.0.0.1` on an ephemeral port, creates its own explicitly flat
scratch world, disables Nether/End, sets view/simulation distance 2, and uses
256 MiB initial/1024 MiB maximum JVM heap. EULA acceptance belongs to this
explicitly authorized local reference boot. The plugin requests orderly
shutdown after export; the runner stops a timed-out process. Existing worlds
and the FerrumC server are not involved.

Pre/post config hashes, JVM/compile commands, Java version, helper origins and
hashes, class origins, negative controls, and output hashes are recorded. The
full raw evidence is under `.codex-tmp/net04_1/`; production provenance removes
only machine-specific path prefixes and retains all packet/semantic values.
These are actual packet-codec probes during boot, not gameplay comparisons.

## Independent Prismarine export

The existing blackbox toolchain fixes minecraft-protocol 1.66.2 and
minecraft-data 3.111.0. The helper requires deep equality with the vendored
1.21.8 protocol schema before creating the real ProtoDef compiler. It declares
semantic inputs independently and compares their encoded packets byte-for-byte
with Paper, then decodes the Paper bytes independently and checks all semantic
fields and exact consumption. Motion inputs are original, already quantized
wire values; this helper does not reimplement physics or f64 quantization.

## Fixture contract and completed evidence

Both TSVs contain 30 packets, including packet ID but no outer frame length:
14 metadata, 8 motion, and 8 TakeItemEntity. Their identical SHA-256 is
`66b71e900d5e81fd42874ace58cb60a9ea309fec23158797d95d06e53ad753de`.
Rows cover 0/127/128/255/16384/i32 extrema/-1, empty/primitive metadata,
original ItemStack EMPTY and stone counts 1/64, custom NBT with internal FF,
custom name/repair cost, and a removed max-stack component. Motion uses actual
constructors around +/-3.9 and truncation thresholds, plus original-codec
controls for the full i16 range. Take rotates the eight signed/large values
through all three original fields without changing the count.

Final normal and Python `-O` exports produce byte-identical packet tables,
helper classes, helper jars, and semantic observations after path-prefix
normalization. Config inputs are pinned; emitted default configs are recorded
separately. The final independent Prismarine table also matches exactly.

Original semantic negative controls reject empty tail, missing EOF, unknown
serializer, and truncated ItemStack. Original unpack stops at EOF and leaves
trailing bytes unread. FerrumC's opaque envelope deliberately admits and
preserves all these tails inside its local 263168-byte cap, including trailing
bytes; this is not original metadata-parser parity. N/N+1 cap tests are local
synthetic policy checks, not original server limits. FerrumC also keeps its
existing five-byte malformed VarInt refusal; original Java may read a sixth
byte before refusing. No packet-wide cursor rollback is promised.

The wire contract does not activate Spawn/Metadata/Motion/Pickup/Remove writer
ordering, item accounting, physics, tracking, persistence, Java events, arbitrary
components/sanitizer branches, plugin compatibility, or performance claims.
NET-04.1b and later Items/Session/App consumers have separate acceptance.
