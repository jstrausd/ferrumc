# Pinned connection facts

The four JSON files preserve built-in block-tag membership facts from the fully
generated Minecraft 1.21.8 resources of Paper Build 60, commit
`29c8822d90899c89d2689338e81a98f690bcba12`. `manifest.toml` records their original
byte counts, SHA-256 hashes and source directory. Runtime code does not parse
these files; drift tests compare them with the static classifications and the
independent vendored `blocks.json` registry. No Java implementation is included.

The reviewed complete `FenceBlock.connectsTo` separates same-family fences,
oriented fence gates, and non-exception supporting faces. `isSameFence` compares
`FENCES` and `WOODEN_FENCES` membership; crimson and warped count as wooden.
`FenceGateBlock.connectsToDirection` uses the facing axis perpendicular to the
connection direction, independently of `open` or `in_wall`.

The complete `Block.isExceptionForConnection` uses `LeavesBlock` class identity,
five explicit blocks and the `SHULKER_BOXES` tag. For this pin the eleven built-in
`LEAVES` members match the leaf classes registered in `Blocks`: tinted and
untinted particle leaves derive from `LeavesBlock`, and mangrove leaves derive
through tinted particle leaves. This agreement is a built-in fact, not a claim
that changing the leaf tag at runtime changes the Java class check.

SupportType.FULL examines the actual block support shape for a state and face.
The Prismarine snapshot's `boundingBox` field does not contain that shape. These
facts therefore expose no full-face/support predicate and do not prove that a
known non-exception is connectable. Dynamic datapack tags, support shapes, world
updates, plugin events and Paper differential acceptance remain separate work.
