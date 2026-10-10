# ferrumc-proto

Generated packet enums and codecs for protocol 772 (Minecraft 1.21.8).

## Invariants

See `INVARIANTS.md` in this directory.

## Item-entity packet wire contract

Play/clientbound `SetEntityData` (0x5C), `SetEntityMotion` (0x5E), and `TakeItemEntity`
(0x75) provide metadata transport, signed i16 velocity fields, and the three
signed pickup-animation `VarInts`. Motion takes already quantized wire values;
no physics or Vec3 quantizer is introduced. Take's count stays a wire value;
its consumer must choose the original pickup count separately from the amount
moved into an inventory.

`SetEntityData` stores its **unprefixed opaque tail** in `BoundedBytes<263168>`.
Only the local byte cap is checked before copying the remaining input. The
wrapper's length-prefixed `read`/`write` methods are not used. Empty, shortened,
unknown-serializer, internal-0xFF, missing-EOF, and post-EOF bytes within the
cap remain unchanged. This does not implement the original semantic metadata
parser. The cap reserves 256 KiB plus 1 KiB for a planned item consumer; it is
not a Vanilla limit or proof that arbitrary trusted Slots fit. A later builder
must preflight its entire concrete Slot/metadata budget and EOF framing.

[Reference procedure](tests/reference/item_entity_1_21_8/README.md) pins genuine
Paper 1.21.8 Build 60 and independent Prismarine inputs. The 30 exact packet
fixtures include actual item metadata with components and removed components,
signed/large IDs and counts, constructor velocity boundaries, and full i16
wire extrema. These checks establish the bounded packet envelope, without
activating gameplay, tracking, persistence, plugins, or a server-wide memory
or performance claim.
